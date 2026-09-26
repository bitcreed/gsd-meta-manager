use super::phase_num::{phase_key, PhaseNum};
use regex::Regex;
use std::collections::HashMap;
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq)]
pub struct RoadmapPhase {
    pub number: String,
    pub name: String,
    pub description: String,
    pub completed: bool,
    pub total_plans: u32,
    pub completed_plans: u32,
    /// The phase identifiers this phase's `**Depends on**:` line declares, in
    /// the order written. Empty when the entry has no dependency line. The
    /// entry's own phase is never listed (gsd-core #4764); see
    /// [`parse_depends_on`] for the grammar.
    ///
    /// **Declared, never inferred.** GSD's own router gates two of its three
    /// forward-motion actions on `deps_satisfied` (`init.cjs:2037-2064`), so a
    /// dependency condition has to read what the roadmap says. Deriving one from
    /// phase numbering — "20 depends on 19" — is exactly the kind of guess that
    /// drifts from the runtime the driver is driving, and an absent line means
    /// *no declared dependencies*, not *unknown*.
    pub depends_on: Vec<String>,
}

/// Aggregate phase/plan counts parsed from a ROADMAP.md `## Progress` table.
///
/// The `## Progress` table is the authoritative source of progress counts for a
/// GSD 1.8.0 roadmap. plan 6 (mod.rs) prefers this over STATE.md frontmatter
/// when present; when the section is absent `roadmap_progress` returns `None` and
/// the caller falls back to STATE.md.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RoadmapProgress {
    pub total_phases: u32,
    pub completed_phases: u32,
    pub total_plans: u32,
    pub completed_plans: u32,
}

/// Phase-ID token used inside the phase recognizers.
///
/// Accepts:
/// - bare numeric / decimal IDs (`4`, `0.3`, `14`, `26`)
/// - project-code / milestone-prefixed IDs (`M-2`, `AB-29`)
/// - a trailing letter (covers backlog sentinels like `999.x`)
/// - letter-suffixed ids with dotted sub-phases (`12A`, `12A.1`, `23A.1.2`) —
///   gsd-core's canonical `PHASE_NUMBER_TOKEN_SOURCE` `\d+[A-Z]?(?:\.\d+)*`
///   (`src/phase-id.cts:65`, #2128 / #4830)
///
/// The optional leading `[A-Za-z]{1,4}-` is the project-code/milestone prefix;
/// the numeric body is `[0-9][0-9.]*` with an optional trailing `[A-Za-z]`,
/// then any number of `.N` segments. A strict superset of the upstream token:
/// for letter-free input the `.N` tail adds nothing the greedy `[0-9.]*` body
/// has not already consumed, so every decimal, padded, prefixed and sentinel
/// match is unchanged; only a letter followed by `.N` segments newly matches.
const PHASE_ID: &str = r"(?:[A-Za-z]{1,4}-)?[0-9][0-9.]*[A-Za-z]?(?:\.[0-9]+)*";

/// The phase identifier a written phase reference names, in the forms this file
/// already recognises.
///
/// **One normaliser, because a gate keyed on one spelling is a gate that fails
/// open on every other** (WR-06). GSD writes a phase reference several ways —
/// `19`, `Phase 19`, `**19**`, `#19`, `19-gitsafe-git-blast-radius-envelope` —
/// and [`parse_depends_on`] already accepts the keyword form for exactly this
/// reason. A consumer comparing a table cell to an argv token by raw string
/// equality matches this repository's own spelling and silently never fires on
/// any other, which is worse than an absent gate: the journal grep that asks
/// "how often did this gate stop a run?" answers zero for a reason unrelated to
/// the gate.
///
/// Accepts an optional leading `Phase` keyword and `#`, surrounding `*`
/// emphasis, and a trailing `-<slug>`. Returns `None` when nothing identifier
/// shaped is there, so a caller can keep the raw text rather than substituting a
/// guess.
pub fn extract_phase_id(text: &str) -> Option<String> {
    static ID_AT_START: OnceLock<Regex> = OnceLock::new();
    let id_at_start =
        ID_AT_START.get_or_init(|| Regex::new(&format!(r"^({id})", id = PHASE_ID)).unwrap());

    let trimmed = text
        .trim()
        .trim_matches('*')
        .trim()
        .trim_start_matches('#')
        .trim();
    // `get(..5)` rather than a slice: the text is arbitrary and a byte index
    // inside a multi-byte character would panic.
    let body = match trimmed.get(..5) {
        Some(head) if head.eq_ignore_ascii_case("phase") => trimmed[5..].trim_start(),
        _ => trimmed,
    };

    id_at_start
        .captures(body)
        // The same trailing-punctuation trim `parse_depends_on` applies, so
        // `Phase 19.` and `Phase 19,` yield what they name.
        .map(|caps| caps[1].trim_end_matches(['.', ',']).to_string())
        .filter(|id| !id.is_empty())
}

/// Returns true when a phase number is a backlog sentinel that must be excluded
/// from the returned phase list (and every count).
///
/// Sentinels: `Phase 0` (pre-milestone) and `Phase 999` / `999.x` (backlog).
/// A leading alphabetic project-code prefix (`M-`, `AB-`) is stripped first so
/// only the numeric body is inspected. Ordinary decimals like `0.3` are kept.
/// Numeric, so GSD's padded spellings (`00`, `0999.1`) are sentinels too.
fn is_sentinel_phase(number: &str) -> bool {
    let n = match number.split_once('-') {
        Some((prefix, rest)) if !prefix.is_empty() && prefix.chars().all(|c| c.is_ascii_alphabetic()) => {
            rest
        }
        _ => number,
    };
    match super::phase_num::PhaseNum::parse(n) {
        Some(num) => num == 0 || num.major() == 999,
        None => n == "0" || n == "999" || n.starts_with("999."),
    }
}

/// Extract the phase identifiers declared by a `**Depends on**:` line's text,
/// for the entry whose own recognised number is `own_number`.
///
/// Two rules:
///
/// 1. **Parenthetical groups are stripped first** (Plan 20-03, threat
///    mitigation T-20-16). A qualifier is prose about ordering, not a
///    dependency — this repository's phase 20 declares `Phase 16, Phase 17,
///    Phase 19 (and Phase 22 must land before this phase closes)`, and
///    promoting `22` out of that aside would state a dependency the roadmap
///    does not. It also disarms `Nothing (no v2.0 dependencies…)`, where a
///    bare-number scan would invent a phase `2.0` from a version string.
///    **This is the one recorded divergence from gsd-core 1.15.0** (which
///    reads parentheticals): measured over this repository's ROADMAP, 1.15.0
///    gives phase 20 → 16, 17, 19, 22 while phase 22 → 15, 17, 21, 20 — a
///    20↔22 cycle that leaves both unsatisfiable — and phases 24/25 gain 15, 23,
///    14 (and 24) out of an "independent of … phases 15-23 … like Phase 14"
///    aside. Reversing a threat-register mitigation is a decision of its own,
///    so the strip stays.
/// 2. **gsd-core 1.15.0's `PHASE_DEP_REF` grammar** (`src/phase-id.cts:80`,
///    #4764, the grammar `init manager` computes `dep_phases` with): a
///    case-insensitive `phase` / `phases` keyword followed by a list of ids
///    joined by `,` / `, and` / `and` / `&`, or ranges joined by `-` / `to` /
///    `through` (hyphen only — en/em dashes are prose, as upstream). **A range
///    contributes its ENDPOINTS only** (`Phases 17-21` → 17, 21), upstream's
///    deliberate choice. The keyword is still required, because a bare number
///    in prose is indistinguishable from an identifier — dates, shas and
///    version strings stay prose. The id token is [`PHASE_ID`], so beyond
///    upstream's `\d+[A-Z]?(?:\.\d+)*` it also reads project-code-prefixed
///    ids (`Phase M-2`), which upstream skips. Negation prose (`blocks on
///    nothing in Phases 9-11`) is read, as upstream reads it: dropping a real
///    dependency is the dangerous direction.
///
/// The entry's own phase is never listed (#4764 self-reference), and a
/// repeated identifier appears once — both compared by [`dep_ref_key`], so
/// `Phase 8, phase 08` and `phase 12a` under `### Phase 12A:` resolve as
/// upstream resolves them. Order written is preserved and the first-written
/// spelling is kept.
fn parse_depends_on(text: &str, own_number: &str) -> Vec<String> {
    static QUALIFIER: OnceLock<Regex> = OnceLock::new();
    static REFERENCE: OnceLock<Regex> = OnceLock::new();
    static TOKEN: OnceLock<Regex> = OnceLock::new();
    let qualifier = QUALIFIER.get_or_init(|| Regex::new(r"\([^)]*\)").unwrap());
    let reference = REFERENCE.get_or_init(|| {
        Regex::new(&format!(
            r"(?i)\bphases?\s+({id}(?:(?:\s*,\s*(?:and\s+)?|\s+and\s+|\s*&\s*|\s+(?:to|through)\s+|\s*-\s*){id})*)",
            id = PHASE_ID
        ))
        .unwrap()
    });
    let token = TOKEN.get_or_init(|| Regex::new(PHASE_ID).unwrap());

    let without_qualifiers = qualifier.replace_all(text, " ");
    let own = dep_ref_key(own_number);
    let mut seen: Vec<String> = Vec::new();
    let mut out: Vec<String> = Vec::new();
    for caps in reference.captures_iter(&without_qualifiers) {
        for tok in token.find_iter(&caps[1]) {
            let id = tok.as_str().trim_end_matches(['.', ',']);
            if id.is_empty() {
                continue;
            }
            let key = dep_ref_key(id);
            if key == own || seen.contains(&key) {
                continue;
            }
            seen.push(key);
            out.push(id.to_string());
        }
    }
    out
}

/// The comparison key [`parse_depends_on`] uses for the self-reference skip
/// and the dedupe — a port of gsd-core's `init.cts` `normalizePhaseNumber`.
/// Each `.`-separated segment of the form `^(\d+)([A-Za-z]?)$` becomes its
/// digits with leading zeros stripped (an all-zero run becomes `0`) followed by
/// the letter uppercased; any other segment (`M-2`) is kept verbatim.
///
/// Leading zeros are **stripped, not parsed**, so an absurdly long digit run
/// cannot overflow. Private on purpose: [`phase_key`] keeps letter ids raw and
/// case-sensitive, and changing it would move every other caller.
fn dep_ref_key(id: &str) -> String {
    id.trim()
        .split('.')
        .map(|segment| {
            let digits_end = segment
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(segment.len());
            let (digits, rest) = segment.split_at(digits_end);
            let letter_ok = rest.is_empty()
                || (rest.len() == 1 && rest.as_bytes()[0].is_ascii_alphabetic());
            if digits.is_empty() || !letter_ok {
                return segment.to_string();
            }
            let stripped = digits.trim_start_matches('0');
            let number = if stripped.is_empty() { "0" } else { stripped };
            format!("{number}{}", rest.to_ascii_uppercase())
        })
        .collect::<Vec<_>>()
        .join(".")
}

/// Parse ROADMAP.md content and extract phase checklist items with per-phase plan counts.
///
/// Recognizes several phase-line shapes used by GSD roadmaps, one grammar
/// shared with [`parse_shipped_phases`] (see [`recognize_phase_line`]):
/// - bold checklist with a dash tail: `- [ ] **Phase N: Title** - desc`
///   (the original grammar, tried first so its captures never change)
/// - bold checklist with any other tail, or none: `- [ ] **Phase N: Title**`,
///   `- [x] **Phase N: Title** (INSERTED) - desc`, `… (3/3 plans) — done`
/// - plain checklist: `- [x] Phase N: Title — desc`
/// - parenthetical cluster tags: `- [ ] **Phase 26 (Cluster B): Title** - desc`
/// - project-code / milestone-prefixed IDs: `Phase M-2`, `Phase AB-29`
/// - markdown headings, colon or spaced-dash separated: `### Phase 4:
///   Visualization`, `### Phase 13 — Title` (no checkbox → `completed: false`)
/// - `<details>` / `</details>` / `<summary>` wrapper lines are transparent, so
///   phases and plans nested inside an active or unlabelled `<details>` block
///   are still counted — but **every line of a closed-milestone collapse is
///   skipped** ([`closed_milestone_lines`], GSD's own rule): those phases are
///   shipped history and go to [`parse_shipped_phases`] instead.
pub fn parse_roadmap_phases(content: &str) -> Vec<RoadmapPhase> {
    parse_phases_in_region(content, Region::Current)
}

/// Which part of a roadmap a phase parser reads: everything outside
/// closed-milestone collapses (the GSD-facing list), or only inside them
/// (the shipped history).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Region {
    Current,
    Closed,
}

/// Which recognizer matched a phase line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntryShape {
    Checklist,
    Heading,
    Bare,
}

/// A recognized phase line. `phase` is `None` for a retired (`~~`) or
/// sentinel entry: it is not returned, yet it still ends the entry above.
struct RecognizedLine {
    phase: Option<RoadmapPhase>,
    tally: Option<PlanTally>,
    shape: EntryShape,
}

/// R1, the original bold checklist grammar. Groups: 1=checkbox,
/// 2=strike-open (`~~`), 3=id, 4=name, 5=strike-close (`~~`), 6=description.
/// Unanchored and byte-identical to the grammar this parser always had.
fn checklist_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(
            r"- \[([ xX])\] (~~)?\*\*Phase ({id})(?:\s*\([^)]*\))?:\s*(.+?)\*\*(~~)?\s*[-\x{{2014}}]\s*(.*)",
            id = PHASE_ID
        ))
        .unwrap()
    })
}

/// R2, a bold checklist line whose tail is not a dash description — nothing,
/// a `(TAG)`, a plan tally. Anchored. Groups as [`checklist_re`], 6=the tail.
fn bold_checkbox_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(
            r"^\s*[-*] \[([ xX])\] (~~)?\*\*Phase ({id})(?:\s*\([^)]*\))?:\s*(.+?)\*\*(~~)?(.*)$",
            id = PHASE_ID
        ))
        .unwrap()
    })
}

/// R5, a bare `Phase 01: Name (3 plans, complete)` line — this repository's
/// own v1.0-v1.2 shape. Consulted ONLY inside a closed collapse: anywhere
/// else a bare line is prose. Groups: 1=id, 2=the text after the colon.
fn bare_phase_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(
            r"^\s*Phase ({id})(?:\s*\([^)]*\))?:\s+(.+?)\s*$",
            id = PHASE_ID
        ))
        .unwrap()
    })
}

/// Whether `line` opens (or, retired, still bounds) a phase entry in `region`.
fn is_entry_line(line: &str, region: Region) -> bool {
    checklist_re().is_match(line)
        || bold_checkbox_re().is_match(line)
        || plain_checkbox_re().is_match(line)
        || phase_heading_re().is_match(line)
        || (region == Region::Closed && bare_phase_re().is_match(line))
}

/// The one phase-line grammar both channels share. Recognizers are tried in
/// order — R1 [`checklist_re`], R2 [`bold_checkbox_re`], R3
/// [`plain_checkbox_re`], headings [`phase_heading_re`], and in the closed
/// region only R5 [`bare_phase_re`] — so an original-grammar line keeps its
/// captures exactly. Only R2, R3 and R5 read a plan tally.
fn recognize_phase_line(line: &str, region: Region) -> Option<RecognizedLine> {
    let entry = |number: &str,
                 name: String,
                 description: String,
                 completed: bool,
                 retired: bool,
                 tally: Option<PlanTally>,
                 shape: EntryShape| {
        let number = number.to_string();
        let phase = (!retired && !is_sentinel_phase(&number)).then(|| RoadmapPhase {
            number,
            name,
            description,
            completed,
            total_plans: 0,
            completed_plans: 0,
            depends_on: Vec::new(),
        });
        RecognizedLine { phase, tally, shape }
    };

    if let Some(caps) = checklist_re().captures(line) {
        // Strikethrough (`~~...~~`) marks a retired phase → exclude entirely.
        let retired = caps.get(2).is_some() || caps.get(5).is_some() || line.contains("~~");
        return Some(entry(
            &caps[3],
            caps[4].trim().to_string(),
            caps[6].trim().to_string(),
            &caps[1] != " ",
            retired,
            None,
            EntryShape::Checklist,
        ));
    }
    if let Some(caps) = bold_checkbox_re().captures(line) {
        let retired = caps.get(2).is_some() || caps.get(5).is_some() || line.contains("~~");
        let (tags, description, tally) = split_bold_tail(&caps[6]);
        let mut name = caps[4].trim().to_string();
        for tag in tags {
            name.push(' ');
            name.push_str(&tag);
        }
        return Some(entry(
            &caps[3],
            name,
            description,
            &caps[1] != " ",
            retired,
            tally,
            EntryShape::Checklist,
        ));
    }
    if let Some(caps) = plain_checkbox_re().captures(line) {
        let retired = caps.get(2).is_some() || line.contains("~~");
        let (name, description, tally) = split_summary_rest(&caps[4]);
        return Some(entry(
            &caps[3],
            name,
            description,
            &caps[1] != " ",
            retired,
            tally,
            EntryShape::Checklist,
        ));
    }
    if let Some(caps) = phase_heading_re().captures(line) {
        // `###`-style headings carry no checkbox and no inline description.
        return Some(entry(
            &caps[1],
            caps[2].trim().to_string(),
            String::new(),
            false,
            line.contains("~~"),
            None,
            EntryShape::Heading,
        ));
    }
    if region == Region::Closed {
        if let Some(caps) = bare_phase_re().captures(line) {
            let (name, description, tally) = split_summary_rest(&caps[2]);
            let completed = tally.is_some_and(|t| t.says_complete);
            return Some(entry(
                &caps[1],
                name,
                description,
                completed,
                line.contains("~~"),
                tally,
                EntryShape::Bare,
            ));
        }
    }
    None
}

/// The phase entries of one region of a roadmap: every recognized line in
/// the region opens an entry that runs to the next entry line or the edge of
/// the region. Inside it, plan-checklist lines are counted and the first
/// `**Depends on**:` line is read. Plan counts are the per-field max of the
/// line's tally and the scanned items. A heading is complete only in the
/// closed region, and only when it lists plans and every one is checked.
fn parse_phases_in_region(content: &str, region: Region) -> Vec<RoadmapPhase> {
    static DEPENDS: OnceLock<Regex> = OnceLock::new();
    // The phase part of a plan filename may be decimal: an inserted phase's
    // plans are `07.1-01-PLAN.md`, and `\d+-` alone never counted them. It
    // may carry a letter too (`12A-01-PLAN.md`, `23A.1.2-01-PLAN.md`), the
    // same token [`PHASE_ID`] recognises in the phase line itself.
    let plan_re = plan_checklist_re();
    // `**Depends on**: …`, with the emphasis markers optional.
    let depends_re =
        DEPENDS.get_or_init(|| Regex::new(r"(?i)^\s*\*{0,2}Depends on\*{0,2}\s*:\s*(.*)$").unwrap());

    let lines: Vec<&str> = content.lines().collect();
    let closed = closed_milestone_lines(&lines);
    let in_region = |i: usize| closed[i] == (region == Region::Closed);
    let mut phases: Vec<RoadmapPhase> = Vec::new();

    for (i, line) in lines.iter().enumerate() {
        if !in_region(i) {
            continue;
        }
        let Some(recognized) = recognize_phase_line(line, region) else {
            continue;
        };
        let Some(mut phase) = recognized.phase else {
            continue;
        };

        // Scan subsequent lines for plan items. `<details>`/`</details>`/
        // `<summary>` lines match no recognizer, so inside one region they
        // are transparent; the region's edge ends the entry, so a plan item
        // in a closed collapse is never credited to a live phase above it.
        let (mut scanned_total, mut scanned_done) = (0u32, 0u32);
        for (j, l) in lines.iter().enumerate().skip(i + 1) {
            if !in_region(j) || is_entry_line(l, region) {
                break;
            }
            if let Some(plan_caps) = plan_re.captures(l) {
                scanned_total += 1;
                if &plan_caps[1] != " " {
                    scanned_done += 1;
                }
            }
            // First dependency line inside the entry wins. Only the
            // `## Phase Details` copy of a phase carries one; the summary
            // checklist copy stops at the next header, so its list stays
            // empty and `merge_duplicate_phases` takes the detail copy's.
            if phase.depends_on.is_empty() {
                if let Some(dep_caps) = depends_re.captures(l) {
                    phase.depends_on = parse_depends_on(&dep_caps[1], &phase.number);
                }
            }
        }
        let (tally_total, tally_done) = recognized.tally.map_or((0, 0), |t| (t.total, t.completed));
        phase.total_plans = scanned_total.max(tally_total);
        phase.completed_plans = scanned_done.max(tally_done);
        if region == Region::Closed && recognized.shape == EntryShape::Heading {
            phase.completed = phase.total_plans > 0 && phase.completed_plans == phase.total_plans;
        }
        phases.push(phase);
    }

    merge_duplicate_phases(phases)
}

/// A plan-checklist item, `- [x] 07.1-01-PLAN.md …` or a bare `- [ ] PLAN.md`.
/// Group 1 is the checkbox. The phase part of the optional `NN-NN-` prefix is
/// `\d+[A-Za-z]?(?:\.\d+)*`, so a letter-suffixed phase's `12A-01-PLAN.md` and
/// `23A.1.2-01-PLAN.md` items count.
fn plan_checklist_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^\s*- \[([ xX])\] (?:\d+[A-Za-z]?(?:\.\d+)*-\d+-)?PLAN\.md").unwrap()
    })
}

/// Collapse repeated sightings of the same phase number into one entry.
///
/// A standard GSD 1.8.0 ROADMAP.md describes every phase twice: once in the
/// summary checklist near the top (`- [x] **Phase 4: Title** - desc`) and once
/// under `## Phase Details` (`### Phase 4: Title`, with the plan items beneath
/// it). Without this step the Phases pane lists each phase twice, and the two
/// copies disagree — the checklist copy reports `total_plans: 0` because the
/// next line is another header, while the detail copy carries the real counts.
///
/// Phases are keyed by `number` alone; that is already the identity key the
/// caller uses when building `phase_disk_statuses`. Merge rule per field:
/// - `completed` — logical OR (any checked copy marks the phase complete)
/// - `name`, `description` — first non-empty value wins (the heading form
///   carries no description, so the checklist text survives)
/// - `total_plans`, `completed_plans` — max (the copy that actually scanned the
///   plan list wins over the one that stopped at the next header)
/// - `depends_on` — first non-empty value wins (only the detail copy sees the
///   `**Depends on**:` line)
///
/// First-seen order is preserved, and the pass is O(n) — no nested scan.
///
/// **"Prefer the richer entry" is honoured field by field, never wholesale**
/// (quick 260926-fi9): the detail heading's plans (max) and dependencies (only
/// it has them) win, the checkbox fills completion (OR), and the checklist
/// description fills the heading's empty one. The name stays first-non-empty
/// rather than switching to the heading's, because this repository's phases 15
/// and 17 title their checklist line and their detail heading differently, and
/// switching would rename them.
///
/// **Keyed pad-insensitively** ([`super::phase_num::phase_key`]): GSD writes an
/// inserted phase as `Phase 7.1` in the checklist and `### Phase 07.1:` in its
/// details heading, and keying on the raw text listed it twice — one row with
/// the checklist's name and no directory, one with the detail's plans. The
/// first-seen spelling is kept as the row's `number`.
fn merge_duplicate_phases(phases: Vec<RoadmapPhase>) -> Vec<RoadmapPhase> {
    let mut merged: Vec<RoadmapPhase> = Vec::with_capacity(phases.len());
    let mut index: HashMap<String, usize> = HashMap::new();

    for phase in phases {
        let key = super::phase_num::phase_key(&phase.number);
        match index.get(&key) {
            Some(&at) => {
                let existing = &mut merged[at];
                existing.completed |= phase.completed;
                if existing.name.is_empty() {
                    existing.name = phase.name;
                }
                if existing.description.is_empty() {
                    existing.description = phase.description;
                }
                existing.total_plans = existing.total_plans.max(phase.total_plans);
                existing.completed_plans = existing.completed_plans.max(phase.completed_plans);
                // Same rule as `name`/`description`: first non-empty wins. The
                // checklist copy never carries a dependency line, so this is
                // what lets the `## Phase Details` copy's declaration survive.
                if existing.depends_on.is_empty() {
                    existing.depends_on = phase.depends_on;
                }
            }
            None => {
                index.insert(key, merged.len());
                merged.push(phase);
            }
        }
    }

    merged
}

/// One flag per line: `true` when the line lies inside a **closed-milestone**
/// `<details>` block, its `<details>` and `</details>` lines included.
///
/// **A port of GSD's own rule** (`gsd-core/bin/lib/roadmap-parser.cjs`,
/// `stripClosedMilestoneDetails` / `isClosedMilestoneHeading`): a block is
/// closed when its first `<summary>…</summary>` text carries a closed marker
/// (`CLOSED`, `ARCHIVED`, `ABANDONED`, `SHIPPED`, `FAILED`, `✅`, `🗄`) and no
/// active marker (`STARTED`, `ACTIVE`, `WIP`, `in progress`, `🚧`, `🔄`). GSD
/// strips exactly these blocks before it enumerates phases, which is why
/// [`parse_roadmap_phases`] must not see their lines and
/// [`parse_shipped_phases`] reads only them.
///
/// A block without a summary, and a block with no terminating `</details>`,
/// is not closed — GSD's pattern matches neither. Nesting is tracked with a
/// stack, so a closed outer block covers everything inside it.
fn closed_milestone_lines(lines: &[&str]) -> Vec<bool> {
    static OPEN: OnceLock<Regex> = OnceLock::new();
    static CLOSE: OnceLock<Regex> = OnceLock::new();
    static SUMMARY: OnceLock<Regex> = OnceLock::new();
    static CLOSED_MARKER: OnceLock<Regex> = OnceLock::new();
    static ACTIVE_MARKER: OnceLock<Regex> = OnceLock::new();
    let open = OPEN.get_or_init(|| Regex::new(r"(?i)<details\b").unwrap());
    let close = CLOSE.get_or_init(|| Regex::new(r"(?i)</details>").unwrap());
    let summary = SUMMARY.get_or_init(|| Regex::new(r"(?i)<summary[^>]*>([^<]*)</summary>").unwrap());
    let closed_marker = CLOSED_MARKER.get_or_init(|| {
        Regex::new(r"(?i)\b(?:CLOSED|ARCHIVED|ABANDONED|SHIPPED|FAILED)\b|\x{2705}|\x{1F5C4}").unwrap()
    });
    let active_marker = ACTIVE_MARKER.get_or_init(|| {
        Regex::new(r"(?i)\b(?:STARTED|ACTIVE|WIP)\b|in\s+progress|\x{1F6A7}|\x{1F504}").unwrap()
    });

    let mut flags = vec![false; lines.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        for _ in open.find_iter(line) {
            stack.push(i);
        }
        for _ in close.find_iter(line) {
            let Some(start) = stack.pop() else {
                continue;
            };
            let block = lines[start..=i].join("\n");
            let is_closed = summary.captures(&block).is_some_and(|caps| {
                closed_marker.is_match(&caps[1]) && !active_marker.is_match(&caps[1])
            });
            if is_closed {
                flags[start..=i].fill(true);
            }
        }
    }
    flags
}

/// A plan tally written after a phase title: `(6/6 plans)`, `(3 plans,
/// complete)`, `(1 plan)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlanTally {
    completed: u32,
    total: u32,
    /// The tally itself says the phase is finished (`complete`, `completed`,
    /// `done`) — only the `N plans, …` form can.
    says_complete: bool,
}

fn plan_tally_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?i)\((\d+)/(\d+) plans?\)|\((\d+) plans?(?:,\s*(complete|completed|done))?\)").unwrap()
    })
}

fn read_tally(caps: &regex::Captures<'_>) -> PlanTally {
    let num = |i: usize| caps.get(i).and_then(|m| m.as_str().parse::<u32>().ok()).unwrap_or(0);
    if caps.get(1).is_some() {
        PlanTally {
            completed: num(1),
            total: num(2),
            says_complete: false,
        }
    } else {
        let total = num(3);
        let says_complete = caps.get(4).is_some();
        PlanTally {
            completed: if says_complete { total } else { 0 },
            total,
            says_complete,
        }
    }
}

/// A spaced title/description separator: ` -- `, ` - `, ` – `, ` — `.
fn spaced_separator_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\s+(?:--|[-\x{2013}\x{2014}])\s+").unwrap())
}

/// Strip one leading separator (`--`, `-`, `–`, `—`, `:`) and collapse
/// whitespace runs.
fn tidy_description(text: &str) -> String {
    collapse_whitespace(strip_separator(text))
}

/// `text` without leading whitespace and one leading separator (`--`, `-`,
/// `–`, `—`, `:`).
fn strip_separator(text: &str) -> &str {
    let t = text.trim_start();
    t.strip_prefix("--")
        .or_else(|| t.strip_prefix(['-', '\u{2013}', '\u{2014}', ':']))
        .unwrap_or(t)
}

/// Whitespace runs collapsed to one space, trimmed.
fn collapse_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Split the text after a non-bold `Phase N:` into `(name, description,
/// tally)`. The name ends at the earliest plan tally or spaced separator;
/// the description is the rest with the first tally removed and one leading
/// separator stripped. With neither, the whole text is the name.
fn split_summary_rest(rest: &str) -> (String, String, Option<PlanTally>) {
    let tally = plan_tally_re().captures(rest);
    let tally_span = tally.as_ref().and_then(|c| c.get(0)).map(|m| (m.start(), m.end()));
    let sep_start = spaced_separator_re().find(rest).map(|m| m.start());
    let cut = match (tally_span.map(|(s, _)| s), sep_start) {
        (Some(a), Some(b)) => a.min(b),
        (Some(a), None) => a,
        (None, Some(b)) => b,
        (None, None) => return (rest.trim().to_string(), String::new(), None),
    };
    let name = rest[..cut].trim().to_string();
    let remainder = match tally_span {
        Some((s, e)) => format!("{} {}", &rest[cut..s], &rest[e..]),
        None => rest[cut..].to_string(),
    };
    (name, tidy_description(&remainder), tally.as_ref().map(read_tally))
}

/// R3, the plain (non-bold) checkbox phase line GSD's `complete-milestone`
/// workflow writes: `- [x] Phase 1: Name (2/2 plans) — completed D`.
/// Groups: 1=checkbox, 2=strike-open, 3=id, 4=the text after the colon.
fn plain_checkbox_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(
            r"^\s*[-*] \[([ xX])\] (~~)?Phase ({id})(?:\s*\([^)]*\))?:\s+(.+?)\s*$",
            id = PHASE_ID
        ))
        .unwrap()
    })
}

/// Split the tail after a bold `**Phase N: Title**` into `(tags,
/// description, tally)`. Leading parenthetical groups are read first: a plan
/// tally sets the counts, any other `(TAG)` is returned to be appended to the
/// name (so mailbot's `**Phase 03.1: X** (INSERTED) - d` and its `### Phase
/// 03.1: X (INSERTED)` heading spell one name). Then one leading separator is
/// stripped; a tally later in the tail is read and removed as well.
fn split_bold_tail(tail: &str) -> (Vec<String>, String, Option<PlanTally>) {
    let tally_re = plan_tally_re();
    let mut rest = tail.trim_start();
    let mut tags: Vec<String> = Vec::new();
    let mut tally: Option<PlanTally> = None;
    while rest.starts_with('(') {
        let Some(end) = rest.find(')') else {
            break;
        };
        let group = &rest[..=end];
        let whole_tally = tally_re
            .captures(group)
            .filter(|c| c.get(0).is_some_and(|m| m.start() == 0 && m.end() == group.len()));
        match whole_tally {
            Some(caps) if tally.is_none() => tally = Some(read_tally(&caps)),
            _ => tags.push(group.to_string()),
        }
        rest = rest[end + 1..].trim_start();
    }
    let mut description = strip_separator(rest).to_string();
    if tally.is_none() {
        if let Some(caps) = tally_re.captures(&description) {
            let span = caps.get(0).map(|m| m.range());
            tally = Some(read_tally(&caps));
            if let Some(span) = span {
                description.replace_range(span, " ");
            }
        }
    }
    (tags, collapse_whitespace(&description), tally)
}

/// The phases a roadmap lists inside **closed-milestone** `<details>`
/// collapses — the shipped history GSD's `complete-milestone` folds away.
/// Display-only.
///
/// **Deliberately separate from [`parse_roadmap_phases`].** GSD strips these
/// blocks before it counts phases (see [`closed_milestone_lines`]), so they
/// are not GSD phases: in the GSD-facing list an archived phase with no
/// directory would infer `NoDirectory`, take the current-phase cell, and
/// become a legal driver target. The two parsers share one grammar
/// ([`recognize_phase_line`]) and split the roadmap by region, not by shape —
/// this one reads only closed lines, where a bare `Phase 01: Name (3 plans,
/// complete)` line is recognized too.
///
/// `completed` comes from the checkbox; a bare line is complete iff its tally
/// says so, a heading iff it lists plans and all are checked. Plan counts are
/// the per-field max of the line's tally and the plan items listed below it.
/// Retired (`~~`) and sentinel ids are excluded. Duplicates merge as in
/// [`parse_roadmap_phases`].
pub fn parse_shipped_phases(content: &str) -> Vec<RoadmapPhase> {
    parse_phases_in_region(content, Region::Closed)
}

/// The `#### Build phase N (Milestone M): Title` heading a roadmap uses for a
/// placeholder phase of a planned milestone (ttbook's shape). Groups: 1=id,
/// 2=title. Level 2-4, case-insensitive, the parenthetical optional.
///
/// **Not a GSD phase heading.** GSD's recogniser wants `Phase` right after the
/// hashes (an optional `[...]` tag aside), so `Build ` in between keeps these
/// out of every count GSD makes — which is why [`parse_roadmap_phases`] must not
/// match them either and they are returned by [`parse_planned_build_phases`]
/// instead.
fn build_heading_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(
            r"(?i)^\s*#{{2,4}}\s+Build\s+phase\s+({id})(?:\s*\([^)]*\))?:\s+(.+?)\s*$",
            id = PHASE_ID
        ))
        .unwrap()
    })
}

/// Any markdown heading line (level 1-6): the end of a phase entry for the
/// readers that scope "inside the entry" by heading.
fn any_heading_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*#{1,6}[ \t]").unwrap())
}

/// An ordinary GSD phase-entry heading, `## / ### / #### Phase N: Title`, with
/// an optional parenthetical tag before the colon. Group 1 is the id, group 2
/// the title. **One grammar** shared by [`parse_roadmap_phases`],
/// [`parse_phase_goals`] and [`phase_section`], so the phase list, the Roadmap
/// tab's goals and the Backlog tab's content cannot disagree about which lines
/// open an entry.
///
/// The separator may also be a **spaced** dash (`### Phase 13 — Title`, `-`,
/// `–`, `--`). Spaced, so `### Phase 3 Implementation Scope` stays prose.
fn phase_heading_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(&format!(
            r"^\s*#{{2,4}}\s+Phase ({id})(?:\s*\([^)]*\))?(?::\s+|\s+(?:--|[-\x{{2013}}\x{{2014}}])\s+)(.+?)\s*$",
            id = PHASE_ID
        ))
        .unwrap()
    })
}

/// A markdown thematic break (`---`, `***`, `___`, spaced or not).
fn thematic_break_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^ {0,3}(?:(?:-[ \t]*){3,}|(?:\*[ \t]*){3,}|(?:_[ \t]*){3,})$").unwrap()
    })
}

/// The number of leading `#` of a heading line.
fn heading_level(line: &str) -> usize {
    line.trim_start().bytes().take_while(|b| *b == b'#').count()
}

/// A fenced-code-block delimiter line (```` ``` ```` or `~~~`).
fn is_fence_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    trimmed.starts_with("```") || trimmed.starts_with("~~~")
}

/// Extract the phase identifiers a build-phase entry's `**Depends on**:` line
/// declares. Applied ONLY inside a build-phase entry; [`parse_depends_on`]
/// stays the grammar for every GSD phase. The two differ on ranges: the GSD
/// grammar reads a range's ENDPOINTS only (gsd-core 1.15.0 parity, #4764),
/// while this build-phase grammar EXPANDS a range against the known ids.
///
/// Parentheticals are stripped first, as [`parse_depends_on`] does. Then four
/// forms are accepted: `Build phase N`, `Build phases A, B[, C]`,
/// `Build phases A-B` (hyphen, en or em dash) and the ordinary `Phase N`. A
/// range expands to every id in `known_ids` whose [`PhaseNum`] lies inside it
/// inclusively, in numeric order — **bounded by the known ids, never by the
/// numeric span**, so `Build phases 1-999999999` invents nothing and allocates
/// nothing extra (T-24-02). A repeated id (pad-insensitively) appears once, in
/// first-written order.
fn parse_build_depends_on(text: &str, known_ids: &[String]) -> Vec<String> {
    static REF: OnceLock<Regex> = OnceLock::new();
    static RANGE: OnceLock<Regex> = OnceLock::new();
    static SINGLE: OnceLock<Regex> = OnceLock::new();
    let reference = REF.get_or_init(|| {
        let item = format!(r"{id}(?:\s*[-\x{{2013}}\x{{2014}}]\s*{id})?", id = PHASE_ID);
        Regex::new(&format!(
            r"(?:(?i:build\s+phases?)|Phase)\s+({item}(?:\s*,\s*{item})*)",
            item = item
        ))
        .unwrap()
    });
    let range = RANGE.get_or_init(|| {
        Regex::new(&format!(
            r"^({id})\s*[-\x{{2013}}\x{{2014}}]\s*({id})$",
            id = PHASE_ID
        ))
        .unwrap()
    });
    let single = SINGLE.get_or_init(|| Regex::new(&format!(r"^({id})$", id = PHASE_ID)).unwrap());

    let without_qualifiers = Regex::new(r"\([^)]*\)").unwrap().replace_all(text, " ");
    let mut out: Vec<String> = Vec::new();
    fn push(id: String, out: &mut Vec<String>) {
        if !id.is_empty() && !out.iter().any(|seen| phase_key(seen) == phase_key(&id)) {
            out.push(id);
        }
    }
    for caps in reference.captures_iter(&without_qualifiers) {
        for item in caps[1].split(',') {
            let item = item.trim().trim_end_matches(['.', ',']);
            if let Some(r) = range.captures(item) {
                let (Some(lo), Some(hi)) = (PhaseNum::parse(&r[1]), PhaseNum::parse(&r[2])) else {
                    continue;
                };
                let mut inside: Vec<(PhaseNum, &String)> = known_ids
                    .iter()
                    .filter_map(|k| PhaseNum::parse(k).map(|n| (n, k)))
                    .filter(|(n, _)| lo <= *n && *n <= hi)
                    .collect();
                inside.sort_by(|a, b| a.0.cmp(&b.0));
                for (_, k) in inside {
                    push(k.clone(), &mut out);
                }
            } else if let Some(s) = single.captures(item) {
                push(s[1].to_string(), &mut out);
            }
        }
    }
    out
}

/// The placeholder phases a roadmap declares as `#### Build phase N
/// (Milestone M): Title` headings (D-A12), one [`RoadmapPhase`] per heading.
///
/// Each entry runs to the next markdown heading of any level; inside it,
/// plan-checklist lines count toward `total_plans` / `completed_plans` and the
/// first `**Depends on**:` line is read by the build-phase dependency grammar
/// (`Build phases 8-13` expands against the ids [`parse_roadmap_phases`] and
/// this function see). `completed` is always `false` and `description` empty.
///
/// **Display-only, kept apart from [`parse_roadmap_phases`].** GSD does not
/// count these headings, so merging them into the GSD phase list would let the
/// driver router and the frontier target a phase GSD does not know exists.
pub fn parse_planned_build_phases(content: &str) -> Vec<RoadmapPhase> {
    let plan_re = plan_checklist_re();
    let depends_re = Regex::new(r"(?i)^\s*\*{0,2}Depends on\*{0,2}\s*:\s*(.*)$").unwrap();
    let heading = build_heading_re();
    let any_heading = any_heading_re();

    let lines: Vec<&str> = content.lines().collect();
    // (phase, its dependency-line text) — resolved once every id is known.
    let mut entries: Vec<(RoadmapPhase, Option<String>)> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        let Some(caps) = heading.captures(line) else {
            continue;
        };
        let number = caps[1].trim_end_matches(['.', ',']).to_string();
        if line.contains("~~") || is_sentinel_phase(&number) {
            continue;
        }
        let mut phase = RoadmapPhase {
            number,
            name: caps[2].trim().to_string(),
            description: String::new(),
            completed: false,
            total_plans: 0,
            completed_plans: 0,
            depends_on: Vec::new(),
        };
        let mut depends: Option<String> = None;
        for l in lines.iter().skip(i + 1) {
            if any_heading.is_match(l) {
                break;
            }
            if let Some(plan_caps) = plan_re.captures(l) {
                phase.total_plans += 1;
                if &plan_caps[1] != " " {
                    phase.completed_plans += 1;
                }
            }
            if depends.is_none() {
                if let Some(dep_caps) = depends_re.captures(l) {
                    depends = Some(dep_caps[1].to_string());
                }
            }
        }
        entries.push((phase, depends));
    }
    if entries.is_empty() {
        return Vec::new();
    }

    let mut known_ids: Vec<String> = parse_roadmap_phases(content)
        .into_iter()
        .map(|p| p.number)
        .collect();
    known_ids.extend(entries.iter().map(|(p, _)| p.number.clone()));

    let phases = entries
        .into_iter()
        .map(|(mut phase, depends)| {
            if let Some(text) = depends {
                phase.depends_on = parse_build_depends_on(&text, &known_ids);
            }
            phase
        })
        .collect();
    merge_duplicate_phases(phases)
}

/// Every phase's goal, keyed by [`phase_key`] (D-A03). Display-only.
///
/// Walks every ordinary `## Phase N: Title` heading (the shape
/// [`parse_roadmap_phases`] reads) and every `#### Build phase N` heading;
/// inside each entry — up to the next markdown heading of any level — the first
/// line reading `**Goal**: text` or `**Goal:** text` (case-insensitive) yields
/// the trimmed text. An empty goal is skipped; the first entry for a key wins,
/// so `### Phase 07:` and a later `### Phase 7:` do not overwrite each other.
/// Single-line goals only: every roadmap seen writes the goal on one line.
///
/// **`Untrusted` values**: the text is the project's own prose and reaches a
/// cell only through `shown()`.
pub fn parse_phase_goals(content: &str) -> HashMap<String, crate::text::Untrusted> {
    static GOAL: OnceLock<Regex> = OnceLock::new();
    let phase_heading = phase_heading_re();
    let goal = GOAL.get_or_init(|| Regex::new(r"(?i)^\s*\*\*Goal(?:\*\*\s*:|:\*\*)\s*(.*)$").unwrap());
    let build_heading = build_heading_re();
    let any_heading = any_heading_re();

    let mut goals: HashMap<String, crate::text::Untrusted> = HashMap::new();
    let mut open: Option<String> = None;
    for line in content.lines() {
        if any_heading.is_match(line) {
            open = phase_heading
                .captures(line)
                .or_else(|| build_heading.captures(line))
                .map(|caps| phase_key(caps[1].trim_end_matches(['.', ','])));
            continue;
        }
        let Some(key) = &open else {
            continue;
        };
        if let Some(caps) = goal.captures(line) {
            let text = caps[1].trim();
            if !text.is_empty() {
                goals
                    .entry(key.clone())
                    .or_insert_with(|| crate::text::Untrusted::from_untrusted_source(text.to_string()));
            }
            // The first Goal line inside the entry wins, empty or not.
            open = None;
        }
    }
    goals
}

/// The full text of the roadmap entry for `phase_id` — its `Phase N:` heading
/// line and everything under it — or `None` when the roadmap has no such entry.
/// Display-only; the caller wraps the result as untrusted text.
///
/// **Why this exists (debug backlog-content-empty).** GSD's backlog capture
/// (`gsd-core/workflows/add-backlog.md`) writes an item's content ONLY here, as
/// `### Phase 999.N: … (BACKLOG)` under `## Backlog`, and gives its
/// `.planning/phases/999.N-<slug>/` directory nothing but a `.gitkeep`. So the
/// Backlog tab's content pane must read this section, not only the directory.
///
/// # Where the entry ends
///
/// At the first of, outside a fenced code block:
///
/// * a heading of the SAME or a HIGHER level than the entry's own (a deeper
///   `####` sub-heading inside a `###` entry is part of it), or
/// * a thematic break (`---`). Measured 2026-09-24 over every registered
///   project's ROADMAP.md: all six breaks inside a `Phase` entry sit directly
///   before the next entry or the document footer (`*Roadmap created: …*`),
///   none mid-body — so ending there keeps a footer out of the LAST entry
///   without truncating any real one.
///
/// Trailing blank lines are dropped. The id is matched by [`phase_key`], so
/// `999.1` never takes `999.10`'s entry and `0999.1` names `999.1`; the first
/// matching entry wins. The heading suffix is not inspected — a delivered item
/// reads `(PROMOTED AND DELIVERED)`, not `(BACKLOG)`, and is still its entry.
pub fn phase_section(content: &str, phase_id: &str) -> Option<String> {
    locate_phase_section(content, phase_id).map(|(_, text)| text)
}

/// The 1-based line of the heading [`phase_section`] would open on — the line
/// an editor jumps to when the Backlog tab's edit key opens ROADMAP.md
/// (quick-260924-drx). The SAME locator as [`phase_section`], so the line and
/// the displayed section cannot disagree about which entry is meant.
pub fn phase_section_line(content: &str, phase_id: &str) -> Option<usize> {
    locate_phase_section(content, phase_id).map(|(line, _)| line)
}

/// `(1-based heading line, section text)` for [`phase_section`] and
/// [`phase_section_line`].
fn locate_phase_section(content: &str, phase_id: &str) -> Option<(usize, String)> {
    let want = phase_key(phase_id);
    let mut in_fence = false;
    // (the entry's heading level, its 1-based heading line, its lines) once
    // the entry has opened.
    let mut section: Option<(usize, usize, Vec<&str>)> = None;
    for (idx, line) in content.lines().enumerate() {
        if !in_fence {
            if let Some((level, _, _)) = &section {
                let ends_entry = thematic_break_re().is_match(line)
                    || (any_heading_re().is_match(line) && heading_level(line) <= *level);
                if ends_entry {
                    break;
                }
            } else if let Some(caps) = phase_heading_re().captures(line) {
                if phase_key(caps[1].trim_end_matches(['.', ','])) == want {
                    section = Some((heading_level(line), idx + 1, Vec::new()));
                }
            }
        }
        if is_fence_line(line) {
            in_fence = !in_fence;
        }
        if let Some((_, _, lines)) = &mut section {
            lines.push(line);
        }
    }
    let (_, heading_line, mut lines) = section?;
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    Some((heading_line, lines.join("\n")))
}

/// Returns true when a `## Progress` table Phase cell is a backlog sentinel
/// (`Phase 0` / `Phase 999` / `999.x`) that must not be counted. The cell may
/// carry a trailing label (e.g. `999. Backlog`), so only the leading token is
/// inspected.
fn is_progress_sentinel(phase_cell: &str) -> bool {
    let token = phase_cell
        .split_whitespace()
        .next()
        .unwrap_or("")
        .trim_end_matches('.');
    is_sentinel_phase(token)
}

/// Split a markdown table row into trimmed cells, dropping the outer pipes.
fn split_table_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

/// Parse the authoritative `## Progress` table from ROADMAP.md content.
///
/// Columns are matched by NAME (case-insensitive), not position, so both the
/// flat 4-column layout (`Phase | Plans Complete | Status | Completed`) and the
/// milestone-grouped 5-column layout (with an extra `Milestone` column) — as
/// well as any column reordering — are handled. Returns `None` when the
/// `## Progress` section is absent or its table header cannot be interpreted
/// (missing the required `Phase` / `Plans Complete` columns).
///
/// Pure: takes `&str`, returns `Option<RoadmapProgress>`, mutates nothing.
///
/// plan 6 (mod.rs) prefers this over STATE.md frontmatter when present.
pub fn roadmap_progress(content: &str) -> Option<RoadmapProgress> {
    let lines: Vec<&str> = content.lines().collect();

    // Locate the `## Progress` section heading.
    let progress_heading = Regex::new(r"(?i)^##[ \t]+Progress\b").unwrap();
    // A following level-1 or level-2 heading closes the section scope.
    let heading_boundary = Regex::new(r"^#{1,2}[ \t]").unwrap();

    let start = lines.iter().position(|l| progress_heading.is_match(l))?;
    let mut end = lines.len();
    for (offset, l) in lines.iter().enumerate().skip(start + 1) {
        if heading_boundary.is_match(l) {
            end = offset;
            break;
        }
    }
    let scope = &lines[(start + 1)..end];

    // A markdown table header row is a `|`-delimited line immediately followed
    // by a delimiter row (`| --- | ... |`).
    let is_row = |l: &str| l.trim().starts_with('|');
    let is_delimiter = |l: &str| {
        let t = l.trim();
        t.starts_with('|')
            && t.contains('-')
            && t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ' | '\t'))
    };

    let mut header_idx = None;
    for k in 0..scope.len() {
        if is_row(scope[k]) && k + 1 < scope.len() && is_delimiter(scope[k + 1]) {
            header_idx = Some(k);
            break;
        }
    }
    let header_idx = header_idx?;

    let headers = split_table_row(scope[header_idx]);
    let col = |name: &str| headers.iter().position(|h| h.eq_ignore_ascii_case(name));

    // Required columns; without these the header cannot be interpreted.
    let phase_col = col("Phase")?;
    let plans_col = col("Plans Complete")?;
    let status_col = col("Status");

    let plans_re = Regex::new(r"(\d+)\s*/\s*(\d+)").unwrap();

    let mut progress = RoadmapProgress::default();

    for l in scope.iter().skip(header_idx + 2) {
        if !is_row(l) {
            break; // table ended
        }
        let cells = split_table_row(l);
        if cells.len() < headers.len() {
            continue;
        }
        let phase_cell = cells.get(phase_col).map(String::as_str).unwrap_or("").trim();
        if phase_cell.is_empty() || is_progress_sentinel(phase_cell) {
            continue;
        }
        progress.total_phases += 1;

        if let Some(sc) = status_col {
            if let Some(status) = cells.get(sc) {
                let status = status.trim();
                if status.eq_ignore_ascii_case("complete") || status.eq_ignore_ascii_case("done") {
                    progress.completed_phases += 1;
                }
            }
        }

        if let Some(cell) = cells.get(plans_col) {
            if let Some(m) = plans_re.captures(cell.trim()) {
                progress.completed_plans += m[1].parse::<u32>().unwrap_or(0);
                progress.total_plans += m[2].parse::<u32>().unwrap_or(0);
            }
        }
    }

    Some(progress)
}

/// The in-progress milestone named in ROADMAP.md's `## Milestones` list.
///
/// GSD writes that list as `- 🚧 **<name>** - Phases 1-7 (in progress)`, with
/// `✅` / `📋` for shipped and planned ones. The bold text of the first entry
/// marked `🚧` or `(in progress)` is returned; `None` when the section is
/// absent or nothing in it is marked in progress.
///
/// The fallback for a STATE.md without a `milestone:` key — a project whose
/// roadmap names its milestone should not render an empty `Milestone:` field.
pub fn active_milestone(content: &str) -> Option<String> {
    let heading = Regex::new(r"(?i)^##[ \t]+Milestones\b").unwrap();
    let boundary = Regex::new(r"^#{1,2}[ \t]").unwrap();
    let bold = Regex::new(r"\*\*(.+?)\*\*").unwrap();

    let mut in_section = false;
    for line in content.lines() {
        if heading.is_match(line) {
            in_section = true;
            continue;
        }
        if !in_section {
            continue;
        }
        if boundary.is_match(line) {
            break;
        }
        let t = line.trim_start();
        if !(t.starts_with("- ") || t.starts_with("* ")) {
            continue;
        }
        if line.contains('\u{1F6A7}') || line.to_ascii_lowercase().contains("(in progress)") {
            if let Some(caps) = bold.captures(line) {
                let name = caps[1].trim();
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

/// One milestone a ROADMAP.md names, with the phases it covers (quick
/// 260923-md1). Display-only: the Roadmap graph's header band and row-end
/// labels read it; no gate or action does.
#[derive(Debug, Clone, PartialEq)]
pub struct RoadmapMilestone {
    /// The label as written, e.g. `v2.0 Autonomous Orchestration`.
    ///
    /// **`Untrusted`, not `String`**: it is third-party text out of the
    /// project's ROADMAP.md, so it reaches a cell only through `shown()`, and
    /// `ProjectState` gains no free-string field (`THIRD_PARTY_STRINGS`).
    pub label: crate::text::Untrusted,
    /// Inclusive declared range (`Phases A-B`), when numeric.
    pub first: Option<PhaseNum>,
    pub last: Option<PhaseNum>,
    /// `phase_key`s of phases nested under the milestone's heading. Generated
    /// by the reader for logic only; never rendered.
    pub scoped_phases: Vec<String>,
    /// A `🚧` or `in progress` marker was seen on one of its lines.
    pub in_progress: bool,
}

impl RoadmapMilestone {
    /// Whether `phase_id` lies within the declared inclusive range exactly,
    /// without the inserted-decimal extension `range_contains` adds.
    fn strict_range_contains(&self, phase_id: &str) -> bool {
        match (&self.first, &self.last, PhaseNum::parse(phase_id)) {
            (Some(first), Some(last), Some(p)) => *first <= p && p <= *last,
            _ => false,
        }
    }

    /// Whether the declared range covers `phase_id`. Numeric, and an inserted
    /// decimal belongs to its integer's milestone: `Phases 1-7` holds `7.1`.
    fn range_contains(&self, phase_id: &str) -> bool {
        match (&self.first, &self.last, PhaseNum::parse(phase_id)) {
            (Some(first), Some(last), Some(p)) => {
                *first <= p && (p <= *last || p.major() == last.major())
            }
            _ => false,
        }
    }

    /// Whether this milestone holds `phase_id`, by range or heading scope.
    pub fn contains(&self, phase_id: &str) -> bool {
        self.range_contains(phase_id) || self.scoped_phases.contains(&phase_key(phase_id))
    }
}

/// The index of the milestone holding `phase_id`: the first whose declared
/// range covers it exactly, else the first whose range covers it through the
/// inserted-decimal rule, else the first whose heading scope does.
///
/// The exact pass comes first so an overlap resolves to the declared range:
/// with `Phases 1-7` and `Phases 7.1-12`, `7.1` belongs to the second.
pub fn milestone_index_of(ms: &[RoadmapMilestone], phase_id: &str) -> Option<usize> {
    ms.iter()
        .position(|m| m.strict_range_contains(phase_id))
        .or_else(|| ms.iter().position(|m| m.range_contains(phase_id)))
        .or_else(|| {
            let key = phase_key(phase_id);
            ms.iter().position(|m| m.scoped_phases.contains(&key))
        })
}

/// The active milestone: the one STATE.md names (full label, or its first
/// token such as `v2.0`), else the first in-progress one, else `None`.
pub fn active_milestone_index(ms: &[RoadmapMilestone], state_milestone: &str) -> Option<usize> {
    let wanted = state_milestone.trim();
    if !wanted.is_empty() {
        let hit = ms.iter().position(|m| {
            let label = m.label.as_raw_for_logic_only().trim();
            label.eq_ignore_ascii_case(wanted)
                || label
                    .split_whitespace()
                    .next()
                    .is_some_and(|tok| tok.eq_ignore_ascii_case(wanted))
        });
        if hit.is_some() {
            return hit;
        }
    }
    ms.iter().position(|m| m.in_progress)
}

/// Split a milestone label into a short id and a name: `Milestone 1: Foo` →
/// (`M1`, `Foo`), `v7.0 — Foundation` → (`v7.0`, `Foundation`). Pure text:
/// the reader uses it on raw text for logic, the graph on ESCAPED text.
pub fn split_milestone_label(label: &str) -> (String, String) {
    static NUMBERED: OnceLock<Regex> = OnceLock::new();
    let numbered = NUMBERED.get_or_init(|| Regex::new(r"(?i)^Milestone\s+(\d+)").unwrap());
    let t = label.trim();
    let (short, rest) = match numbered.captures(t) {
        Some(caps) => {
            let end = caps.get(0).map_or(0, |m| m.end());
            (format!("M{}", &caps[1]), &t[end..])
        }
        None => match t.split_once(char::is_whitespace) {
            Some((head, tail)) => (head.to_string(), tail),
            None => (t.to_string(), ""),
        },
    };
    let rest = rest
        .trim()
        .trim_start_matches([':', '-', '\u{2013}', '\u{2014}'])
        .trim();
    let unquoted = [('"', '"'), ('\u{201C}', '\u{201D}')]
        .iter()
        .find_map(|&(open, close)| {
            rest.strip_prefix(open)
                .and_then(|r| r.strip_suffix(close))
        })
        .unwrap_or(rest);
    (short, unquoted.to_string())
}

/// The numeric segments of a `v`-prefixed version token (`v0.11` → `[0, 11]`),
/// or `None` when `token` is not one.
fn version_segments(token: &str) -> Option<Vec<u64>> {
    let rest = token.strip_prefix(['v', 'V'])?;
    let segments = rest
        .split('.')
        .map(|seg| {
            if !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_digit()) {
                seg.parse::<u64>().ok()
            } else {
                None
            }
        })
        .collect::<Option<Vec<u64>>>()?;
    (!segments.is_empty()).then_some(segments)
}

/// Whether version `a` is strictly lower than `b`, segment by segment with a
/// missing segment read as 0 (`v1` == `v1.0`, `v0.9 < v0.11 < v0.12`).
fn version_lower(a: &[u64], b: &[u64]) -> bool {
    let len = a.len().max(b.len());
    let at = |v: &[u64], i: usize| v.get(i).copied().unwrap_or(0);
    (0..len)
        .map(|i| at(a, i).cmp(&at(b, i)))
        .find(|o| o.is_ne())
        .is_some_and(|o| o.is_lt())
}

/// Which milestones are shipped, one flag per milestone — the input to the
/// Roadmap's collapsed shipped-milestones row (D-A07). Display-only.
///
/// A milestone qualifies only when it declares something: a numeric range or
/// scoped phases. A spurious heading like `## Requirement Coverage (v1.5)` is
/// never shipped. Then:
///
/// 1. When [`active_milestone_index`] finds the active milestone at `a`, every
///    qualifying milestone listed before it is shipped (roadmaps list in order).
/// 2. Otherwise, when STATE.md's milestone starts with a version token
///    (`v0.12`), a qualifying milestone whose short id is a version compared
///    LOWER by numeric segments is shipped — never lexicographically, so
///    `v0.9 < v0.11 < v0.12` (sentriq: no roadmap milestone is `v0.12`).
/// 3. Otherwise nothing is.
pub fn shipped_milestones(ms: &[RoadmapMilestone], state_milestone: &str) -> Vec<bool> {
    let qualifies = |m: &RoadmapMilestone| {
        (m.first.is_some() && m.last.is_some()) || !m.scoped_phases.is_empty()
    };
    if let Some(active) = active_milestone_index(ms, state_milestone) {
        return ms
            .iter()
            .enumerate()
            .map(|(i, m)| i < active && qualifies(m))
            .collect();
    }
    let Some(current) = state_milestone
        .split_whitespace()
        .next()
        .and_then(version_segments)
    else {
        return vec![false; ms.len()];
    };
    ms.iter()
        .map(|m| {
            let (short, _) = split_milestone_label(m.label.as_raw_for_logic_only());
            qualifies(m)
                && version_segments(&short).is_some_and(|own| version_lower(&own, &current))
        })
        .collect()
}

/// How many phases a milestone declares: with a numeric range, the count of
/// integer majors `last.major() - first.major() + 1` (`Phases 1-7.1` → 7);
/// else the number of scoped phases; else 0.
///
/// A display-only approximation: inserted decimal phases (`7.1`) are not
/// counted on top of their integer, because a range cannot say how many were
/// inserted.
pub fn declared_phase_count(m: &RoadmapMilestone) -> u32 {
    match (&m.first, &m.last) {
        (Some(first), Some(last)) if last.major() >= first.major() => {
            last.major() - first.major() + 1
        }
        _ => u32::try_from(m.scoped_phases.len()).unwrap_or(u32::MAX),
    }
}

/// A short id two spellings of one milestone share: `v2.0`, `M3`.
fn is_version_like(short: &str) -> bool {
    let mut chars = short.chars();
    match chars.next() {
        Some('v' | 'V') => chars.next().is_some_and(|c| c.is_ascii_digit()),
        Some('M' | 'm') => {
            let rest: Vec<char> = chars.collect();
            !rest.is_empty() && rest.iter().all(char::is_ascii_digit)
        }
        _ => false,
    }
}

/// Whether two raw labels name the same milestone (PI-7).
fn same_milestone(a: &str, b: &str) -> bool {
    if a.trim().eq_ignore_ascii_case(b.trim()) {
        return true;
    }
    let (sa, _) = split_milestone_label(a);
    let (sb, _) = split_milestone_label(b);
    is_version_like(&sa) && is_version_like(&sb) && sa.eq_ignore_ascii_case(&sb)
}

/// Every milestone a ROADMAP.md names, in roadmap order, deduped. Never fails;
/// empty when nothing matches.
///
/// Sources (research §1): `## Milestones` bullets (label = the bold text),
/// `<summary>` lines, and `##`-`####` headings that carry a version token or
/// `Milestone <n>`. Each may declare `Phases A-B`; a `## Milestones` bullet
/// may also declare a single `Phase N`. A milestone heading additionally
/// opens a scope that collects the phase headers nested beneath it, closed by
/// the next heading of the same or a higher level.
pub fn roadmap_milestones(content: &str) -> Vec<RoadmapMilestone> {
    let ms_heading = Regex::new(r"(?i)^##[ \t]+Milestones\b").unwrap();
    let ms_boundary = Regex::new(r"^#{1,2}[ \t]").unwrap();
    let bold = Regex::new(r"\*\*(.+?)\*\*").unwrap();
    // The optional `(` reads sentriq's `## v0.11 Phases (4-7) — archived`.
    let range = Regex::new(&format!(
        r"(?i)\bphases?\s+\(?({id})\s*[-\x{{2013}}\x{{2014}}]\s*({id})",
        id = PHASE_ID
    ))
    .unwrap();
    let single = Regex::new(&format!(r"(?i)\bphase\s+({id})", id = PHASE_ID)).unwrap();
    let any_heading = Regex::new(r"^\s*(#{1,6})[ \t]+(.*?)\s*$").unwrap();
    // A build-phase heading (`#### Build phase 14 (Milestone 3): …`) counts as
    // a phase heading here: otherwise its `(Milestone 3)` reads as a milestone
    // of its own, and its id never joins the enclosing milestone's scope.
    // A spaced-dash heading (`### Phase 2 - Migrate to v2 API`) is a phase
    // heading too, never a `v2` milestone (quick 260926-fi9).
    let phase_heading = Regex::new(&format!(
        r"^\s*#{{2,4}}\s+(?:(?i:build)\s+)?[Pp]hase ({id})(?:\s*\([^)]*\))?(?::|\s+(?:--|[-\x{{2013}}\x{{2014}}])\s)",
        id = PHASE_ID
    ))
    .unwrap();
    let phase_checklist = Regex::new(&format!(
        r"^\s*- \[[ xX]\] (?:~~)?\*\*Phase ({id})",
        id = PHASE_ID
    ))
    .unwrap();
    let version = Regex::new(r"\bv\d+(\.\d+)*\b").unwrap();
    let numbered = Regex::new(r"(?i)\bMilestone\s+\d").unwrap();
    let tags = Regex::new(r"<[^>]*>").unwrap();

    let endpoint = |text: &str| extract_phase_id(text).and_then(|id| PhaseNum::parse(&id));
    let cut_label = |text: &str| -> String {
        let t = text.trim().trim_start_matches(|c: char| !c.is_alphanumeric());
        let t = match t.find(" (") {
            Some(at) => &t[..at],
            None => t,
        };
        t.trim().to_string()
    };
    let marks_progress =
        |line: &str| line.contains('\u{1F6A7}') || line.to_ascii_lowercase().contains("in progress");

    let mut raw: Vec<(String, RoadmapMilestone)> = Vec::new();
    let mut push = |label: String, line: &str, singular: bool| -> usize {
        let (mut first, mut last) = match range.captures(line) {
            Some(caps) => (endpoint(&caps[1]), endpoint(&caps[2])),
            None => (None, None),
        };
        if first.is_none() && singular {
            if let Some(caps) = single.captures(line) {
                first = endpoint(&caps[1]);
                last = first.clone();
            }
        }
        if first.is_none() || last.is_none() {
            first = None;
            last = None;
        }
        raw.push((
            label.clone(),
            RoadmapMilestone {
                label: crate::text::Untrusted::from_untrusted_source(label),
                first,
                last,
                scoped_phases: Vec::new(),
                in_progress: marks_progress(line),
            },
        ));
        raw.len() - 1
    };

    let mut in_ms_section = false;
    // The open heading scope: (heading level, index into `raw`).
    let mut scope: Option<(usize, usize)> = None;
    let mut scoped: Vec<(usize, String)> = Vec::new();
    for line in content.lines() {
        if let Some(caps) = any_heading.captures(line) {
            let level = caps[1].len();
            if scope.is_some_and(|(open, _)| level <= open) {
                scope = None;
            }
            if in_ms_section && ms_boundary.is_match(line) {
                in_ms_section = false;
            }
            if ms_heading.is_match(line) {
                in_ms_section = true;
                continue;
            }
            let text = &caps[2];
            let is_phase = phase_heading.is_match(line);
            if !is_phase && (2..=4).contains(&level) && (version.is_match(text) || numbered.is_match(text)) {
                let label = cut_label(text);
                if !label.is_empty() {
                    let at = push(label, line, false);
                    scope = Some((level, at));
                }
                continue;
            }
        }

        let phase_id = phase_heading
            .captures(line)
            .or_else(|| phase_checklist.captures(line))
            .map(|caps| caps[1].trim_end_matches(['.', ',']).to_string());
        if let (Some(id), Some((_, at))) = (&phase_id, scope) {
            scoped.push((at, super::phase_num::phase_key(id)));
        }

        if in_ms_section {
            let t = line.trim_start();
            if t.starts_with("- ") || t.starts_with("* ") {
                if let Some(caps) = bold.captures(line) {
                    let label = caps[1].trim().to_string();
                    if !label.is_empty() {
                        push(label, line, true);
                    }
                }
            }
            continue;
        }
        if line.contains("<summary>") {
            let label = cut_label(&tags.replace_all(line, " "));
            if !label.is_empty() {
                push(label, line, false);
            }
        }
    }
    for (at, key) in scoped {
        if let Some((_, m)) = raw.get_mut(at) {
            if !m.scoped_phases.contains(&key) {
                m.scoped_phases.push(key);
            }
        }
    }

    // Dedupe (PI-7): keep the first entry's label and position.
    let mut out: Vec<(String, RoadmapMilestone)> = Vec::new();
    for (label, m) in raw {
        match out.iter_mut().find(|(seen, _)| same_milestone(seen, &label)) {
            Some((_, kept)) => {
                kept.in_progress |= m.in_progress;
                if kept.first.is_none() {
                    kept.first = m.first;
                    kept.last = m.last;
                }
                for key in m.scoped_phases {
                    if !kept.scoped_phases.contains(&key) {
                        kept.scoped_phases.push(key);
                    }
                }
            }
            None => out.push((label, m)),
        }
    }
    out.into_iter().map(|(_, m)| m).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_milestone_is_the_in_progress_entry_of_the_milestones_list() {
        let roadmap = "# Roadmap\n\n## Milestones\n\n\
            - ✅ **v1.0 MVP** - Phases 1-3 (shipped 2026-01-01)\n\
            - 🚧 **Milestone 1: Reassessment and decision records** - Phases 1-7 (in progress)\n\
            - 📋 **Milestones 2-5: Build milestones** - build phases 8-18\n\n\
            ## Phases\n\n- [ ] **Phase 1: A** - a\n";
        assert_eq!(
            active_milestone(roadmap).as_deref(),
            Some("Milestone 1: Reassessment and decision records")
        );
        // No section, or nothing in progress: no guess.
        assert_eq!(active_milestone("# Roadmap\n\n- 🚧 **Stray** - x\n"), None);
        assert_eq!(
            active_milestone("## Milestones\n\n- ✅ **v1.0** - shipped\n"),
            None
        );
    }

    #[test]
    fn a_zero_padded_detail_heading_merges_with_its_checklist_entry() {
        // ttbook's shape: checklist `Phase 7.1`, details heading `Phase 07.1`.
        let roadmap = "## Phases\n\n\
            - [x] **Phase 7: Consolidation** - c\n\
            - [ ] **Phase 7.1: Apply rulings (INSERTED)** - apply\n\n\
            ## Phase Details\n\n\
            ### Phase 07: Consolidation\n\n\
            - [x] 07-01-PLAN.md — one\n\n\
            ### Phase 07.1: Apply rulings (INSERTED)\n\n\
            **Depends on**: Phase 7\n\n\
            - [ ] 07.1-01-PLAN.md — a\n\
            - [ ] 07.1-02-PLAN.md — b\n";
        let phases = parse_roadmap_phases(roadmap);
        let numbers: Vec<&str> = phases.iter().map(|p| p.number.as_str()).collect();
        assert_eq!(numbers, ["7", "7.1"], "one row per phase, first spelling kept");
        assert_eq!(phases[0].total_plans, 1);
        assert!(phases[0].completed);
        assert_eq!(phases[1].total_plans, 2);
        assert_eq!(phases[1].depends_on, ["7"]);
    }

    #[test]
    fn a_phase_reference_reduces_to_its_identifier_in_every_written_form() {
        for (written, expected) in [
            ("19", "19"),
            ("Phase 19", "19"),
            ("phase 19", "19"),
            ("PHASE 19", "19"),
            ("#19", "19"),
            ("  **19**  ", "19"),
            ("19-gitsafe-git-blast-radius-envelope", "19"),
            ("Phase 19.", "19"),
            ("20.1", "20.1"),
            ("M-2-something", "M-2"),
        ] {
            assert_eq!(
                extract_phase_id(written).as_deref(),
                Some(expected),
                "`{written}` names phase {expected}. A consumer comparing two \
                 written references by raw equality fires on one spelling and \
                 fails open on every other (WR-06)"
            );
        }

        for nothing in ["", "TBD", "Phase", "—"] {
            assert_eq!(
                extract_phase_id(nothing),
                None,
                "text naming no identifier must yield None so a caller keeps the \
                 raw bytes rather than substituting a guess: {nothing:?}"
            );
        }
    }

    // ── quick 260926-gtl: letter-suffixed phase ids (gsd-core #2128 / #4830) ──

    #[test]
    fn letter_suffixed_phase_ids_parse_in_every_recognizer() {
        let roadmap = "## Phases\n\n\
            - [ ] **Phase 12A: Twelve** - d\n\
            - [x] Phase 12A.1: Sub (1/1 plans)\n\n\
            ## Phase Details\n\n\
            ### Phase 23A.1.2: Letter\n\n\
            **Goal**: letter-suffixed goal\n\
            **Depends on**: Phase 12A\n\n\
            Plans:\n\
            - [x] 23A.1.2-01-PLAN.md — one\n";
        let phases = parse_roadmap_phases(roadmap);
        let numbers: Vec<&str> = phases.iter().map(|p| p.number.as_str()).collect();
        assert_eq!(
            numbers,
            ["12A", "12A.1", "23A.1.2"],
            "every recogniser keeps the full letter-suffixed id (upstream \
             PHASE_NUMBER_TOKEN_SOURCE `\\d+[A-Z]?(?:\\.\\d+)*`)"
        );
        assert_eq!(phases[0].name, "Twelve");
        assert_eq!(phases[1].name, "Sub");
        assert!(phases[1].completed);
        assert_eq!((phases[1].total_plans, phases[1].completed_plans), (1, 1));
        let letter = &phases[2];
        assert_eq!(letter.name, "Letter");
        assert_eq!(
            (letter.total_plans, letter.completed_plans),
            (1, 1),
            "a `23A.1.2-01-PLAN.md` checklist item counts toward its phase"
        );
        assert_eq!(letter.depends_on, ["12A"]);

        assert_eq!(extract_phase_id("23A.1.2-some-slug").as_deref(), Some("23A.1.2"));
        assert_eq!(extract_phase_id("Phase 12A").as_deref(), Some("12A"));
        let section = phase_section(roadmap, "23A.1.2").expect("the entry is found");
        assert!(section.starts_with("### Phase 23A.1.2: Letter"), "{section}");
        assert_eq!(
            goal_of(&parse_phase_goals(roadmap), "23A.1.2").as_deref(),
            Some("letter-suffixed goal")
        );
    }

    // ── Plan 20-03 Task 3: declared roadmap dependencies ──

    /// Parse a `## Phase Details` entry carrying `depends` as its dependency
    /// line, and return the phase's declared dependencies.
    fn depends_of(depends: &str) -> Vec<String> {
        let content = format!(
            "### Phase 20: Router\n\n**Goal**: something\n**Depends on**: {depends}\n\
             **Requirements**: DRIVE-02\n"
        );
        let phases = parse_roadmap_phases(&content);
        assert_eq!(phases.len(), 1, "the fixture declares exactly one phase");
        phases[0].depends_on.clone()
    }

    #[test]
    fn test_depends_on_names_several_phases_in_the_order_written() {
        assert_eq!(
            depends_of("Phase 16, Phase 17, Phase 19"),
            vec!["16", "17", "19"],
            "order written is the order returned"
        );
        // Written out of ascending order, to prove nothing sorts them.
        assert_eq!(depends_of("Phase 19, Phase 16"), vec!["19", "16"]);
    }

    #[test]
    fn test_depends_on_ignores_a_parenthetical_qualifier() {
        assert_eq!(
            depends_of("Phase 16, Phase 17, Phase 19 (and Phase 22 must land before this phase closes)"),
            vec!["16", "17", "19"],
            "a parenthetical qualifier is prose about ordering, not a declared \
             dependency. Promoting `22` out of the aside would state a \
             dependency the roadmap does not"
        );
        assert!(
            depends_of("Nothing (no v2.0 dependencies — parallel-safe, can ship any time)")
                .is_empty(),
            "a bare-number scan would invent a phase `2.0` from the version \
             string `v2.0` and leave the dependency condition unsatisfiable \
             forever"
        );
    }

    #[test]
    fn test_depends_on_absent_line_is_an_empty_list_not_a_guess() {
        let content = "### Phase 20: Router\n\n**Goal**: something\n**Requirements**: DRIVE-02\n";
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert!(
            phases[0].depends_on.is_empty(),
            "an absent line means NO declared dependencies. Inferring `19` from \
             the phase number is the numbering heuristic that drifts from the \
             runtime the driver is driving"
        );
    }

    #[test]
    fn test_depends_on_accepts_every_phase_id_form_and_reads_a_plural_range_as_its_endpoints() {
        assert_eq!(
            depends_of("Phase 0.3, Phase M-2, Phase AB-29"),
            vec!["0.3", "M-2", "AB-29"],
            "the same identifier forms the rest of this file accepts"
        );
        assert_eq!(
            depends_of("Phase 15 only — parallel-eligible with Phases 17-21"),
            vec!["15", "17", "21"],
            "gsd-core 1.15.0 PHASE_DEP_REF (#4764) reads `Phases 17-21` as its \
             range ENDPOINTS only — 17 and 21, never the interior 18-20"
        );
        assert!(depends_of("Nothing").is_empty());
    }

    // ── quick 260926-gtl: gsd-core 1.15.0 PHASE_DEP_REF parity (#4764) ──

    /// Parse `### Phase {row}: X` carrying `text` as its dependency line and
    /// return the declared dependencies of the phase numbered `row`.
    fn depends_of_row(row: &str, text: &str) -> Vec<String> {
        let content = format!("### Phase {row}: X\n\n**Depends on**: {text}\n");
        parse_roadmap_phases(&content)
            .into_iter()
            .find(|p| p.number == row)
            .unwrap_or_else(|| panic!("the fixture declares phase {row}"))
            .depends_on
    }

    #[test]
    fn depends_on_matches_gsd_1_15_dep_phases() {
        // Every value MEASURED 2026-09-26 with the built 1.15.0 oracle's
        // `init manager` dep_phases over a roadmap holding exactly this line.
        let rows: &[(&str, &str, &[&str])] = &[
            ("2", "phase 1", &["1"]),
            (
                "3",
                "Phases 1, 2, and 12A; see 2026-09-14 and sha 8bf403100d",
                &["1", "2", "12A"],
            ),
            ("12A", "Phase 1-3 and Phase 12A", &["1", "3"]),
            ("21", "Phase 15 only — parallel-eligible with Phases 17-21", &["15", "17"]),
            ("22", "Phase 1 & 2, phases 3 to 12A through 20", &["1", "2", "3", "12A", "20"]),
            (
                "22",
                "Phase 15 only — parallel-eligible with Phases 17-21, but must land before Phase 20 closes",
                &["15", "17", "21", "20"],
            ),
            ("07", "Phase 7, Phase 8, phase 08, PHASES 9 AND 10", &["8", "9", "10"]),
            ("12A", "phase 12a, Phase 012A, Phase 11", &["11"]),
            (
                "30",
                "Phase 16 and 17; Phases 18-20; Phase 19. then Phase 21,",
                &["16", "17", "18", "20", "19", "21"],
            ),
            ("14", "Nothing (no v2.0 dependencies — parallel-safe, can ship any time)", &[]),
            ("32", "None", &[]),
        ];
        for (row, text, expected) in rows {
            assert_eq!(
                depends_of_row(row, text),
                expected.to_vec(),
                "phase {row} `**Depends on**: {text}` must read as gsd-core 1.15.0's \
                 router reads it (PHASE_DEP_REF, #4764)"
            );
        }
    }

    #[test]
    fn depends_on_parenthetical_qualifier_stays_stripped_unlike_gsd_1_15() {
        assert_eq!(
            depends_of_row(
                "20",
                "Phase 16, Phase 17, Phase 19 (and Phase 22 must land before this phase closes)"
            ),
            vec!["16", "17", "19"],
            "divergence A (T-20-16): 1.15.0 reads 16, 17, 19, 22 here, and with \
             phase 22's line reading 15, 17, 21, 20 that is a measured 20<->22 \
             cycle making both unsatisfiable. A parenthetical qualifier is never \
             promoted into an identifier"
        );
    }

    #[test]
    fn depends_on_keeps_project_code_prefixed_refs_unlike_gsd_1_15() {
        assert_eq!(
            depends_of_row("31", "Subphase 4, Build phases 8-13, Phase M-2, phase 5 through 6"),
            vec!["8", "13", "M-2", "5", "6"],
            "divergence B: 1.15.0 reads 8, 13, 5, 6 — its token has no project-code \
             prefix, so `Phase M-2` is skipped there and kept here"
        );
    }

    #[test]
    fn depends_on_reads_negation_prose_like_gsd_1_15() {
        assert_eq!(
            depends_of_row(
                "12",
                "Nothing in this milestone — opportunistic, and blocks on nothing in Phases 9-11"
            ),
            vec!["9", "11"],
            "upstream: \"Negation prose ... is NOT detected: the issue's own minimum \
             keeps such tokens\" — silently dropping a real dependency is the \
             dangerous direction, so the tokens are kept as 1.15.0 keeps them"
        );
    }

    #[test]
    fn test_depends_on_is_read_from_this_repositorys_own_roadmap() {
        let roadmap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".planning")
            .join("ROADMAP.md");
        // `expect`, not an early return (IN-07). This file is committed and
        // always present, so the escape hatch bought nothing and cost the one
        // thing the rest of this phase's guards are careful about: a test that
        // cannot read its own subject must fail, not pass having checked
        // nothing.
        let content = std::fs::read_to_string(&roadmap)
            .expect("this repository ships its own .planning/ROADMAP.md");
        let phases = parse_roadmap_phases(&content);
        let find = |number: &str| {
            phases
                .iter()
                .find(|p| p.number == number)
                .unwrap_or_else(|| panic!("the roadmap declares phase {number}"))
        };

        // A phase that declares dependencies, qualifier and all.
        assert_eq!(
            find("20").depends_on,
            vec!["16", "17", "19"],
            "phase 20's line is `Phase 16, Phase 17, Phase 19 (and Phase 22 must \
             land before this phase closes)`"
        );
        assert_eq!(find("16").depends_on, vec!["15"]);
        assert_eq!(find("17").depends_on, vec!["15", "16"]);
        // A phase that declares none: `**Depends on**: Nothing (…)`.
        assert!(
            find("15").depends_on.is_empty(),
            "phase 15 declares `Nothing`, which is an empty list and not an \
             identifier scraped out of the parenthetical"
        );
    }

    #[test]
    fn test_parse_roadmap_phases_real() {
        let content = r#"# Roadmap

- [ ] **Phase 1: Core Infrastructure** - Async TUI foundation
- [ ] **Phase 2: Dashboard and Navigation** - Main project list
- [x] **Phase 3: Live State** - File-watcher auto-refresh
- [ ] **Phase 4: Visualization** - ASCII roadmap
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 4);
        assert!(!phases[0].completed);
        assert_eq!(phases[0].number, "1");
        assert_eq!(phases[0].name, "Core Infrastructure");
        assert!(phases[2].completed);
        assert_eq!(phases[1].name, "Dashboard and Navigation");
        // No plan items listed, so counts should be 0
        assert_eq!(phases[0].total_plans, 0);
        assert_eq!(phases[0].completed_plans, 0);
    }

    #[test]
    fn test_parse_roadmap_empty() {
        let phases = parse_roadmap_phases("");
        assert!(phases.is_empty());
    }

    #[test]
    fn test_parse_roadmap_uppercase_x() {
        let content = "- [X] **Phase 1: Done** - Completed phase\n";
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert!(phases[0].completed);
    }

    #[test]
    fn test_parse_roadmap_with_plan_items() {
        let content = r#"# Roadmap

- [x] **Phase 1: Core Infrastructure** - Foundation

Plans:
- [x] 01-01-PLAN.md -- Project scaffold
- [x] 01-02-PLAN.md -- State reader
- [ ] 01-03-PLAN.md -- Event loop

- [ ] **Phase 2: Dashboard** - Main UI

Plans:
- [x] 02-01-PLAN.md -- Dashboard table
- [ ] 02-02-PLAN.md -- Help overlay
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 2);

        assert_eq!(phases[0].total_plans, 3);
        assert_eq!(phases[0].completed_plans, 2);

        assert_eq!(phases[1].total_plans, 2);
        assert_eq!(phases[1].completed_plans, 1);
    }

    #[test]
    fn test_parse_roadmap_standalone_plan_md() {
        let content = r#"# Roadmap

- [x] **Phase 1: Solo Plan** - Single standalone plan

Plans:
- [x] PLAN.md -- solo plan
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].total_plans, 1);
        assert_eq!(phases[0].completed_plans, 1);
    }

    #[test]
    fn test_parse_roadmap_mixed_standalone_and_numbered() {
        let content = r#"# Roadmap

- [ ] **Phase 3: Mixed** - Both formats

Plans:
- [x] 03-01-PLAN.md -- numbered plan
- [ ] PLAN.md -- standalone plan
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].total_plans, 2);
        assert_eq!(phases[0].completed_plans, 1);
    }

    #[test]
    fn test_parse_roadmap_heading_form() {
        // `### Phase N: Title` (no checkbox) parses as a phase with completed=false.
        let content = "### Phase 4: Visualization\n";
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].number, "4");
        assert_eq!(phases[0].name, "Visualization");
        assert!(!phases[0].completed);
        assert_eq!(phases[0].description, "");
    }

    #[test]
    fn test_parse_roadmap_heading_levels_and_plans() {
        // `##` and `####` heading levels both parse; plan items after a heading count.
        let content = r#"## Phase 2: Dashboard

Plans:
- [x] 02-01-PLAN.md -- table
- [ ] 02-02-PLAN.md -- overlay

#### Phase 3: Live State
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0].number, "2");
        assert_eq!(phases[0].name, "Dashboard");
        assert_eq!(phases[0].total_plans, 2);
        assert_eq!(phases[0].completed_plans, 1);
        assert_eq!(phases[1].number, "3");
        assert_eq!(phases[1].name, "Live State");
    }

    #[test]
    fn test_parse_roadmap_parenthetical_tag() {
        // A parenthetical cluster tag is not part of the number or name.
        let content = "- [ ] **Phase 26 (Cluster B): Title** - desc\n";
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].number, "26");
        assert_eq!(phases[0].name, "Title");
        assert_eq!(phases[0].description, "desc");
    }

    #[test]
    fn test_parse_roadmap_prefixed_ids() {
        // Project-code / milestone-prefixed IDs parse with the prefix intact.
        let content = r#"- [ ] **Phase M-2: Milestone Two** - desc
- [x] **Phase AB-29: Big One** - done
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0].number, "M-2");
        assert_eq!(phases[0].name, "Milestone Two");
        assert_eq!(phases[1].number, "AB-29");
        assert_eq!(phases[1].name, "Big One");
        assert!(phases[1].completed);
    }

    #[test]
    fn test_parse_roadmap_details_wrapped() {
        // Phase + plans nested in a <details> block are still parsed; the
        // <details>/<summary>/</details> wrapper lines are transparent.
        let content = r#"<details>
<summary>Completed phases</summary>

- [x] **Phase 1: Core** - Foundation

Plans:
- [x] 01-01-PLAN.md -- scaffold
- [x] 01-02-PLAN.md -- reader

</details>

- [ ] **Phase 2: Dashboard** - Main UI
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0].number, "1");
        assert_eq!(phases[0].total_plans, 2);
        assert_eq!(phases[0].completed_plans, 2);
        assert_eq!(phases[1].number, "2");
    }

    #[test]
    fn test_parse_roadmap_dedupes_summary_and_details() {
        // A standard GSD 1.8.0 roadmap describes each phase twice: once in the
        // summary checklist near the top, once under `## Phase Details`. The two
        // copies must merge into a single entry per phase number.
        let content = r#"# Roadmap

- [x] **Phase 4: Visualization** - ASCII roadmap rendering
- [ ] **Phase 5: Queue** - Batch execution
- [ ] **Phase 6: Sessions** - tmux attach

## Phase Details

### Phase 4: Visualization

Plans:
- [x] 04-01-PLAN.md -- canvas shapes
- [x] 04-02-PLAN.md -- arrow routing

### Phase 5: Queue

Plans:
- [x] 05-01-PLAN.md -- queue model
- [ ] 05-02-PLAN.md -- executor trait

### Phase 6: Sessions
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 3);

        // First-seen order preserved.
        let numbers: Vec<&str> = phases.iter().map(|p| p.number.as_str()).collect();
        assert_eq!(numbers, vec!["4", "5", "6"]);

        // The literal `## Phase Details` heading is not itself a phase.
        assert!(phases.iter().all(|p| p.name != "Details"));

        // Description survives from the checklist form (heading form has none).
        assert_eq!(phases[0].description, "ASCII roadmap rendering");

        // Checkbox survives the merge (heading form always reports false).
        assert!(phases[0].completed);
        assert!(!phases[1].completed);

        // Plan counts come from the detail section, where the plan items live.
        assert_eq!(phases[0].total_plans, 2);
        assert_eq!(phases[0].completed_plans, 2);
        assert_eq!(phases[1].total_plans, 2);
        assert_eq!(phases[1].completed_plans, 1);
        assert_eq!(phases[2].total_plans, 0);
    }

    #[test]
    fn test_parse_roadmap_dedupes_details_before_checklist() {
        // Mirror layout: the `### Phase N:` detail heading (with its plan items)
        // appears BEFORE the summary checklist entry for the same phase.
        let content = r#"### Phase 2: Dashboard

Plans:
- [x] 02-01-PLAN.md -- table
- [ ] 02-02-PLAN.md -- overlay

- [x] **Phase 2: Dashboard** - Main project list
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].number, "2");
        // OR rule picks up the later checkbox.
        assert!(phases[0].completed);
        // First-seen copy had an empty description; the later non-empty one fills it.
        assert_eq!(phases[0].description, "Main project list");
        // Max rule keeps the counts from the earlier copy that scanned the plan list.
        assert_eq!(phases[0].total_plans, 2);
        assert_eq!(phases[0].completed_plans, 1);
    }

    #[test]
    fn test_parse_roadmap_decimal_id_preserved() {
        // Existing decimal IDs still parse.
        let content = "- [ ] **Phase 0.3: Spike** - explore\n";
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].number, "0.3");
        assert_eq!(phases[0].name, "Spike");
    }

    #[test]
    fn test_parse_roadmap_excludes_strikethrough() {
        // A strikethrough (retired) phase is absent from the returned Vec and
        // does not contribute to any count; real phases are unaffected.
        let content = r#"- [ ] **Phase 1: Real One** - live
- [ ] ~~**Phase 7: Old idea**~~ - abandoned
~~Phase 8: Bare strike~~
- [ ] **Phase 2: Real Two** - live
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 2);
        assert_eq!(phases[0].number, "1");
        assert_eq!(phases[1].number, "2");
        assert!(phases.iter().all(|p| p.number != "7" && p.number != "8"));
    }

    #[test]
    fn test_parse_roadmap_excludes_backlog_sentinels() {
        // Phase 0 and Phase 999 / 999.x sentinels are excluded; real phases stay.
        let content = r#"- [ ] **Phase 0: Backlog** - pre-milestone
- [ ] **Phase 1: Alpha** - real
- [ ] **Phase 999: Someday** - backlog
- [ ] **Phase 999.2: Later** - backlog
- [ ] **Phase 0.3: Spike** - real decimal
- [ ] **Phase 2: Beta** - real
"#;
        let phases = parse_roadmap_phases(content);
        let numbers: Vec<&str> = phases.iter().map(|p| p.number.as_str()).collect();
        assert_eq!(numbers, vec!["1", "0.3", "2"]);
        assert!(!numbers.contains(&"0"));
        assert!(!numbers.contains(&"999"));
        assert!(!numbers.contains(&"999.2"));
    }

    #[test]
    fn padded_backlog_sentinels_are_excluded_too() {
        let content = "### Phase 00: Pre-milestone\n### Phase 01: Real\n### Phase 0999.1: Later\n### Phase 00.3: Spike\n";
        let numbers: Vec<String> = parse_roadmap_phases(content)
            .into_iter()
            .map(|p| p.number)
            .collect();
        assert_eq!(numbers, ["01", "00.3"]);
    }

    #[test]
    fn test_parse_roadmap_three_real_phases_unaffected() {
        // A normal roadmap of 3 real phases is unchanged by the new filters.
        let content = r#"- [ ] **Phase 1: One** - a
- [x] **Phase 2: Two** - b
- [ ] **Phase M-2: Three** - c
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 3);
        assert_eq!(phases[0].number, "1");
        assert_eq!(phases[1].number, "2");
        assert_eq!(phases[2].number, "M-2");
    }

    #[test]
    fn test_parse_roadmap_mixed_plan_formats() {
        let content = r#"
- [ ] **Phase 3: Live State** - File watching

Plans:
- [ ] 03-01-PLAN.md -- File watcher
- [ ] 03-02-PLAN.md -- Detail view

**UI hint**: yes
"#;
        let phases = parse_roadmap_phases(content);
        assert_eq!(phases.len(), 1);
        assert_eq!(phases[0].total_plans, 2);
        assert_eq!(phases[0].completed_plans, 0);
    }

    #[test]
    fn test_roadmap_progress_flat_four_column() {
        let content = r#"# Roadmap

## Progress

| Phase | Plans Complete | Status | Completed |
| --- | --- | --- | --- |
| 1. Alpha | 2/2 | Complete | ✅ |
| 2. Beta | 1/2 | In Progress | |
"#;
        let p = roadmap_progress(content).expect("progress table parses");
        assert_eq!(p.total_phases, 2);
        assert_eq!(p.completed_phases, 1);
        assert_eq!(p.total_plans, 4);
        assert_eq!(p.completed_plans, 3);
    }

    #[test]
    fn test_roadmap_progress_milestone_grouped_five_column() {
        let content = r#"## Progress

| Phase | Milestone | Plans Complete | Status | Completed |
|---|---|---|---|---|
| 1. Alpha | v1.0 | 2/2 | Complete | ✅ |
| 2. Beta | v1.1 | 0/3 | Planned | |
"#;
        let p = roadmap_progress(content).expect("5-column table parses");
        assert_eq!(p.total_phases, 2);
        assert_eq!(p.completed_phases, 1);
        assert_eq!(p.total_plans, 5);
        assert_eq!(p.completed_plans, 2);
    }

    #[test]
    fn test_roadmap_progress_column_reordered() {
        // Columns matched by NAME, not position — a reordered header still parses.
        let content = r#"## Progress

| Status | Completed | Plans Complete | Phase |
| --- | --- | --- | --- |
| Complete | ✅ | 3/3 | 1. Alpha |
| Planned | | 0/2 | 2. Beta |
"#;
        let p = roadmap_progress(content).expect("reordered table parses");
        assert_eq!(p.total_phases, 2);
        assert_eq!(p.completed_phases, 1);
        assert_eq!(p.total_plans, 5);
        assert_eq!(p.completed_plans, 3);
    }

    #[test]
    fn test_roadmap_progress_excludes_backlog_row() {
        // A 999.x backlog row is not counted as a phase or plans.
        let content = r#"## Progress

| Phase | Plans Complete | Status | Completed |
| --- | --- | --- | --- |
| 1. Alpha | 2/2 | Complete | ✅ |
| 999. Backlog | 0/9 | Planned | |
"#;
        let p = roadmap_progress(content).expect("table parses");
        assert_eq!(p.total_phases, 1);
        assert_eq!(p.completed_phases, 1);
        assert_eq!(p.total_plans, 2);
        assert_eq!(p.completed_plans, 2);
    }

    #[test]
    fn test_roadmap_progress_absent_section_is_none() {
        let content = r#"# Roadmap

- [ ] **Phase 1: Alpha** - no progress table here
"#;
        assert_eq!(roadmap_progress(content), None);
    }

    #[test]
    fn test_roadmap_progress_uninterpretable_header_is_none() {
        // A `## Progress` section whose table lacks the required columns → None.
        let content = r#"## Progress

| Foo | Bar |
| --- | --- |
| a | b |
"#;
        assert_eq!(roadmap_progress(content), None);
    }

    // ── quick 260923-md1: milestone membership ─────────────────────────────

    fn labels(ms: &[RoadmapMilestone]) -> Vec<&str> {
        ms.iter().map(|m| m.label.as_raw_for_logic_only()).collect()
    }

    const REPO_SHAPED: &str = "# Roadmap\n\n## Milestones\n\n\
        - ✅ **v7.0 MVP** - Phases 1-4 (shipped 2026-01-01)\n\
        - ✅ **v8.0 Config** - 10 quick tasks (shipped 2026-02-01)\n\
        - 🚧 **v9.0 Autonomous Orchestration** - Phases 14-23 (in progress)\n\n\
        ## Phases\n\n<details>\n\
        <summary>v7.0 MVP (Phases 01-04) - SHIPPED 2026-01-01</summary>\n\n\
        - [x] **Phase 1: A** - a\n</details>\n\n\
        ### v9.0 Autonomous Orchestration (Phases 14-23)\n\n\
        - [ ] **Phase 14: X** - x\n\n\
        ## Phase Details\n\n### Phase 14: X\n**Depends on**: Nothing\n";

    #[test]
    fn roadmap_milestones_parses_and_dedupes_a_repo_shaped_roadmap() {
        let ms = roadmap_milestones(REPO_SHAPED);
        assert_eq!(
            labels(&ms),
            vec!["v7.0 MVP", "v8.0 Config", "v9.0 Autonomous Orchestration"]
        );
        assert_eq!(ms[0].first, PhaseNum::parse("1"));
        assert_eq!(ms[0].last, PhaseNum::parse("4"));
        assert_eq!(ms[1].first, None, "a bullet without a range still lists");
        assert_eq!(
            ms.iter().map(|m| m.in_progress).collect::<Vec<_>>(),
            vec![false, false, true]
        );
        assert_eq!(milestone_index_of(&ms, "3"), Some(0));
        assert_eq!(milestone_index_of(&ms, "14"), Some(2));
        assert_eq!(milestone_index_of(&ms, "23.1"), Some(2), "decimal rule");
        assert_eq!(milestone_index_of(&ms, "24"), None);
        assert_eq!(milestone_index_of(&ms, "07.5"), None);
    }

    #[test]
    fn roadmap_milestones_heading_scope_covers_the_gsd_template_shape() {
        let content = "## Phases\n\n### 🚧 v8.1 Core (In Progress)\n\n\
            #### Phase 5: Alpha\n#### Phase 6: Beta\n\n\
            ### 📋 v9.2 Next (Planned)\n\n#### Phase 7: Gamma\n";
        let ms = roadmap_milestones(content);
        assert_eq!(labels(&ms), vec!["v8.1 Core", "v9.2 Next"]);
        assert_eq!(milestone_index_of(&ms, "5"), Some(0));
        assert_eq!(milestone_index_of(&ms, "06"), Some(0));
        assert_eq!(milestone_index_of(&ms, "7"), Some(1));
        assert!(ms[0].in_progress);
        assert!(!ms[1].in_progress);
    }

    #[test]
    fn roadmap_milestones_range_includes_inserted_decimals_and_skips_false_headings() {
        let content = "## Milestones\n\n\
            - 🚧 **Milestone 1: Reassessment** - Phases 1-7 (in progress)\n\n\
            ## Notes\n\n### Milestone mapping (decision D10)\n\n- [ ] **Phase 8: Z** - z\n";
        let ms = roadmap_milestones(content);
        assert_eq!(labels(&ms), vec!["Milestone 1: Reassessment"]);
        assert!(ms[0].contains("7.1"));
        assert!(ms[0].contains("07"));
        assert!(!ms[0].contains("8"));
    }

    #[test]
    fn milestone_index_of_prefers_an_exact_range_over_the_decimal_rule() {
        let content = "## Milestones\n\n\
            - ✅ **v1.0 Base** - Phases 1-7 (shipped)\n\
            - 🚧 **v2.0 Next** - Phases 7.1-12 (in progress)\n";
        let ms = roadmap_milestones(content);
        assert_eq!(labels(&ms), vec!["v1.0 Base", "v2.0 Next"]);
        assert_eq!(milestone_index_of(&ms, "7"), Some(0));
        assert_eq!(milestone_index_of(&ms, "7.1"), Some(1), "exact range wins");
        assert_eq!(milestone_index_of(&ms, "7.2"), Some(1));
        assert_eq!(milestone_index_of(&ms, "12"), Some(1));
        assert_eq!(milestone_index_of(&ms, "12.1"), Some(1), "decimal rule still applies");
        assert_eq!(milestone_index_of(&ms, "13"), None);
    }

    #[test]
    fn roadmap_milestones_singular_phase_only_on_milestone_bullets() {
        let ms = roadmap_milestones("## Milestones\n\n- 📋 **M5 web** - Phase 18 (planned)\n");
        assert_eq!(ms[0].first, PhaseNum::parse("18"));
        assert_eq!(ms[0].last, PhaseNum::parse("18"));
        let ms = roadmap_milestones("## Plan\n\n### v2.0 Launch - see Phase 9\n");
        assert_eq!(labels(&ms), vec!["v2.0 Launch - see Phase 9"]);
        assert_eq!(ms[0].first, None);
    }

    #[test]
    fn split_milestone_label_splits_short_id_from_name() {
        let s = |l: &str| {
            let (a, b) = split_milestone_label(l);
            (a, b)
        };
        assert_eq!(
            s("Milestone 1: Reassessment and decision records"),
            ("M1".into(), "Reassessment and decision records".into())
        );
        assert_eq!(
            s("Milestone 3 \"Supervised live booking\""),
            ("M3".into(), "Supervised live booking".into())
        );
        assert_eq!(s("v7.0 — Foundation"), ("v7.0".into(), "Foundation".into()));
        assert_eq!(s("v8.1"), ("v8.1".into(), String::new()));
        assert_eq!(s("M3 live booking"), ("M3".into(), "live booking".into()));
    }

    #[test]
    fn active_milestone_index_matches_state_then_falls_back_to_in_progress() {
        let ms = roadmap_milestones(REPO_SHAPED);
        assert_eq!(active_milestone_index(&ms, "v9.0"), Some(2));
        assert_eq!(active_milestone_index(&ms, "v7.0 MVP"), Some(0));
        assert_eq!(active_milestone_index(&ms, ""), Some(2));
        assert_eq!(active_milestone_index(&ms, "v1.7.2"), Some(2));
        let shipped = roadmap_milestones("## Milestones\n\n- ✅ **v1.0** - Phases 1-2 (shipped)\n");
        assert_eq!(active_milestone_index(&shipped, ""), None);
    }

    #[test]
    fn roadmap_milestones_never_fails_on_garbage() {
        for junk in [
            "",
            "\u{0}###\n<summary></summary>\n## Milestones\n- **\n",
            "### v\n#### Phase 5:\n- 🚧 **x** - Phases 9-\n",
            "## Milestones\n- **v1** - Phases ab-cd\n### v1 (Phases 3-1)\n",
        ] {
            let _ = roadmap_milestones(junk);
        }
        assert!(roadmap_milestones("").is_empty());
        assert!(roadmap_milestones("# Roadmap\n\nJust prose.\n").is_empty());
    }

    // ── Phase 24-01: vendored real-roadmap fixtures ─────────────────────────

    const TTBOOK_ROADMAP: &str = include_str!("../../tests/fixtures/roadmaps/ttbook-ROADMAP.md");

    /// Characterisation guard (Pitfall 1): GSD's own heading grammar does not
    /// recognise `#### Build phase N` headings, so the GSD-facing phase list
    /// must not either. Pinned against the parser as it stood BEFORE build-phase
    /// support landed; it must stay byte-identical.
    #[test]
    fn ttbook_fixture_gsd_phase_list_is_unchanged_by_build_phase_support() {
        let phases = parse_roadmap_phases(TTBOOK_ROADMAP);
        let numbers: Vec<&str> = phases.iter().map(|p| p.number.as_str()).collect();
        assert_eq!(numbers, ["8", "9", "10", "11", "12", "13"]);
        let deps: Vec<Vec<&str>> = phases
            .iter()
            .map(|p| p.depends_on.iter().map(String::as_str).collect())
            .collect();
        assert_eq!(
            deps,
            vec![
                vec![],
                vec!["8"],
                vec!["8", "9"],
                vec!["8", "9"],
                vec!["9", "10", "11"],
                vec!["12"],
            ]
        );
        let plans: Vec<(u32, u32)> = phases
            .iter()
            .map(|p| (p.total_plans, p.completed_plans))
            .collect();
        assert_eq!(plans, [(9, 0), (0, 0), (0, 0), (0, 0), (0, 0), (0, 0)]);
    }

    #[test]
    fn ttbook_build_phase_headings_parse_as_planned_phases() {
        let planned = parse_planned_build_phases(TTBOOK_ROADMAP);
        let numbers: Vec<&str> = planned.iter().map(|p| p.number.as_str()).collect();
        assert_eq!(numbers, ["14", "15", "16", "17", "18"]);
        let names: Vec<&str> = planned.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Guided first run",
                "Scheduled window runs",
                "Messaging client and transcript store",
                "Autonomous responder, staged hold and remote control",
                "Browser frontend",
            ]
        );
        let deps: Vec<Vec<&str>> = planned
            .iter()
            .map(|p| p.depends_on.iter().map(String::as_str).collect())
            .collect();
        assert_eq!(
            deps,
            vec![
                vec!["8", "9", "10", "11", "12", "13"],
                vec!["14"],
                vec!["12", "13"],
                vec!["12", "13", "16"],
                vec!["12"],
            ],
            "`Build phases 8-13` expands to the known ids inside the range; \
             lists and singles read as written"
        );
        assert!(planned.iter().all(|p| !p.completed && p.description.is_empty()));
        assert!(planned.iter().all(|p| p.total_plans == 0));
    }

    #[test]
    fn build_phase_headings_are_not_milestones() {
        let ms = roadmap_milestones(TTBOOK_ROADMAP);
        let shorts: Vec<String> = ms
            .iter()
            .map(|m| split_milestone_label(m.label.as_raw_for_logic_only()).0)
            .collect();
        assert_eq!(shorts, ["v1", "v2", "M3", "M4", "M5"]);
        assert!(
            ms.iter().all(|m| !m.label.as_raw_for_logic_only().starts_with("Build phase")),
            "`#### Build phase 14 (Milestone 3): …` is a phase heading, not a milestone"
        );
        for (phase, milestone) in [("14", 2), ("15", 2), ("16", 3), ("17", 3), ("18", 4)] {
            assert_eq!(milestone_index_of(&ms, phase), Some(milestone), "phase {phase}");
        }
        // The heading scope collects them too, independent of the bullet ranges.
        assert_eq!(ms[2].scoped_phases, ["14", "15"]);
        assert_eq!(ms[3].scoped_phases, ["16", "17"]);
        assert_eq!(ms[4].scoped_phases, ["18"]);
    }

    #[test]
    fn a_build_dependency_range_expands_only_to_known_ids() {
        let roadmap = "## Phases\n\n\
            - [ ] **Phase 1: One** - a\n\
            - [ ] **Phase 2: Two** - b\n\
            - [ ] **Phase 4: Four** - d\n\n\
            #### Build phase 5 (Milestone 2): Five\n\n\
            **Depends on**: Build phases 1-4\n\n\
            #### Build phase 6: Six\n\n\
            **Depends on**: Build phases 1-999999999 (and Phase 3 as prose)\n";
        let planned = parse_planned_build_phases(roadmap);
        assert_eq!(planned.len(), 2);
        assert_eq!(
            planned[0].depends_on,
            ["1", "2", "4"],
            "the gap id 3 is not invented from the numeric span"
        );
        assert_eq!(
            planned[1].depends_on,
            ["1", "2", "4", "5", "6"],
            "a huge span is bounded by the known ids; the parenthetical is prose"
        );
    }

    #[test]
    fn a_build_heading_with_no_parenthetical_parses() {
        let roadmap = "### Build phase 3: Plain title\n\n\
            **Depends on**: Phase 2\n\n\
            - [x] 03-01-PLAN.md — a\n\
            - [ ] 03-02-PLAN.md — b\n\n\
            ## Next\n\n- [ ] 03-03-PLAN.md — outside the entry\n";
        let planned = parse_planned_build_phases(roadmap);
        assert_eq!(planned.len(), 1);
        assert_eq!(planned[0].number, "3");
        assert_eq!(planned[0].name, "Plain title");
        assert_eq!(planned[0].depends_on, ["2"]);
        assert_eq!((planned[0].total_plans, planned[0].completed_plans), (2, 1));
        assert!(
            parse_roadmap_phases(roadmap).is_empty(),
            "GSD's grammar does not see a build heading"
        );
    }

    const DAILY_VOW_ROADMAP: &str =
        include_str!("../../tests/fixtures/roadmaps/daily-vow-ROADMAP.md");
    const SENTRIQ_ROADMAP: &str = include_str!("../../tests/fixtures/roadmaps/sentriq-ROADMAP.md");

    fn goal_of(goals: &HashMap<String, crate::text::Untrusted>, key: &str) -> Option<String> {
        goals.get(key).map(|g| g.as_raw_for_logic_only().to_string())
    }

    #[test]
    fn a_goal_is_read_in_both_bold_forms() {
        let roadmap = "### Phase 1: Colon outside\n\n**Goal**: first form\n\n\
            ### Phase 2: Colon inside\n\n**Goal:** second form\n\n\
            ### Phase 3: Lower case\n\n**goal**:   third form  \n";
        let goals = parse_phase_goals(roadmap);
        assert_eq!(goal_of(&goals, "1").as_deref(), Some("first form"));
        assert_eq!(goal_of(&goals, "2").as_deref(), Some("second form"));
        assert_eq!(goal_of(&goals, "3").as_deref(), Some("third form"));
    }

    #[test]
    fn the_first_goal_in_an_entry_wins_and_a_padded_heading_keys_unpadded() {
        let roadmap = "- [ ] **Phase 7: Seven** - s\n\n## Phase Details\n\n\
            ### Phase 07: Seven\n\n**Goal**: the first goal\n**Goal**: a second goal\n\n\
            ### Phase 8: Empty goal\n\n**Goal**:   \n";
        let goals = parse_phase_goals(roadmap);
        assert_eq!(
            goal_of(&goals, "7").as_deref(),
            Some("the first goal"),
            "`### Phase 07:` and checklist `Phase 7` share one key"
        );
        assert!(!goals.contains_key("07"));
        assert!(!goals.contains_key("8"), "an empty goal is skipped");
    }

    #[test]
    fn a_goal_after_the_next_heading_is_not_attributed_upward() {
        let roadmap = "### Phase 1: No goal here\n\n**Depends on**: Nothing\n\n\
            ## Notes\n\n**Goal**: belongs to no phase\n\n\
            #### Build phase 2 (Milestone 2): Planned\n\n**Goal**: a build goal\n";
        let goals = parse_phase_goals(roadmap);
        assert!(!goals.contains_key("1"), "the Goal line sits after `## Notes`");
        assert_eq!(goal_of(&goals, "2").as_deref(), Some("a build goal"));
        assert_eq!(goals.len(), 1);
    }

    // ── quick 260926-gtl: gsd-core 1.15.0 extractPhaseFieldMultiline parity
    //    (#4731 / #4837). Every expected value MEASURED 2026-09-26 with the
    //    built 1.15.0 oracle's `roadmap get-phase 1` over the same body. ──

    /// The goal of phase 1 when `body` sits alone under `### Phase 1: T` — one
    /// roadmap string per case, so an unbalanced fence cannot leak across.
    fn goal_under_phase_1(body: &str) -> Option<String> {
        let roadmap = format!("### Phase 1: T\n\n{body}\n");
        goal_of(&parse_phase_goals(&roadmap), "1")
    }

    #[test]
    fn a_hard_wrapped_goal_reads_past_the_line_break() {
        // This repository's own ROADMAP `999.4` goal, verbatim.
        let body = "**Goal:** Decide whether the `COVERAGE.md` sub-stage should be read as a *state* parsed\n\
            from frontmatter rather than as a *presence bit*, and change the Pipeline drill-down\n\
            rendering if so.\n\
            **Requirements:** TBD";
        assert_eq!(
            goal_under_phase_1(body).as_deref(),
            Some(
                "Decide whether the `COVERAGE.md` sub-stage should be read as a *state* parsed \
                 from frontmatter rather than as a *presence bit*, and change the Pipeline \
                 drill-down rendering if so."
            ),
            "a hard-wrapped goal continues to the next `**Label**` line, as 1.15.0 reads it"
        );
    }

    #[test]
    fn a_goal_stops_at_the_gsd_1_15_boundaries() {
        let rows: &[(&str, Option<&str>)] = &[
            ("**Goal**: Ship it\n- a bullet", Some("Ship it")),
            ("**Goal**: Ship it\n* a bullet", Some("Ship it")),
            ("**Goal**: Ship it\n+ a bullet", Some("Ship it")),
            (
                "**Goal**: Ship it\n   indented wrap   \n| a | table |",
                Some("Ship it indented wrap"),
            ),
            ("**Goal**: Do the thing\n```\nnot this\n```", Some("Do the thing")),
            ("**Goal**: Do the thing\n~~~\nnot this\n~~~", Some("Do the thing")),
            ("**Goal:** ```js\nfoo()\n```", None),
            ("**Goal** without colon\nwrapped too", Some("without colon wrapped too")),
            ("**Goal**: one\n**requirements**: lower", Some("one")),
            ("**Goal**: eight\n\ntail after blank", Some("eight")),
            (
                "**Goal**: nine\n1. numbered item\nPlans:",
                Some("nine 1. numbered item Plans:"),
            ),
            ("**Goal:**\nnext line goal\nwrapped", Some("next line goal wrapped")),
        ];
        for (body, expected) in rows {
            assert_eq!(
                goal_under_phase_1(body).as_deref(),
                *expected,
                "gsd-core 1.15.0 extractPhaseFieldMultiline reads {expected:?} from {body:?}"
            );
        }

        let letter = "### Phase 23A.1.2: K\n\n**Goal**: letter-suffixed\n";
        assert_eq!(
            goal_of(&parse_phase_goals(letter), "23A.1.2").as_deref(),
            Some("letter-suffixed"),
            "a letter-suffixed phase keys its goal by the full id"
        );
    }

    #[test]
    fn goal_divergences_from_gsd_1_15_are_pinned() {
        assert_eq!(
            goal_under_phase_1("**Goal**: ten\n##### deep heading\nmore").as_deref(),
            Some("ten"),
            "divergence C: 1.15.0 reads `ten ##### deep heading more` (its stop is \
             `#{{1,4}}`); here every heading level already ends the entry"
        );
        assert_eq!(
            goal_under_phase_1("**Goal**:\n**Depends on**: Phase 1"),
            None,
            "divergence D: 1.15.0 reads `**Depends on**: Phase 1` as the goal — its \
             label regex's `\\s*` crosses the empty label line into the next field"
        );
        assert_eq!(
            goal_under_phase_1("**Goal**:   \n\nafter blank"),
            None,
            "divergence D: 1.15.0 reads `after blank` — its `\\s*` crosses the blank \
             line; here continuation starts on the next line and a blank ends it"
        );
    }

    #[test]
    fn fixture_goals_cover_every_detailed_phase() {
        for (name, content, keys) in [
            ("daily-vow", DAILY_VOW_ROADMAP, (18..=23).collect::<Vec<u32>>()),
            ("sentriq", SENTRIQ_ROADMAP, (9..=12).collect()),
            ("ttbook", TTBOOK_ROADMAP, (8..=18).collect()),
        ] {
            let goals = parse_phase_goals(content);
            assert_eq!(goals.len(), keys.len(), "{name}: one goal per detailed phase");
            for key in keys {
                let goal = goal_of(&goals, &key.to_string())
                    .unwrap_or_else(|| panic!("{name}: phase {key} has a goal"));
                assert!(goal.starts_with("(sanitised)"), "{name} {key}: {goal}");
            }
        }
    }

    // ── Phase 24-01 Task 3: shipped-milestone facts ──

    #[test]
    fn daily_vow_shipped_milestones_are_the_five_before_v1_5() {
        let ms = roadmap_milestones(DAILY_VOW_ROADMAP);
        assert_eq!(
            labels(&ms),
            [
                "v1.0 MVP",
                "v1.1 Hardening & Polish",
                "v1.2 Depth & Intentionality",
                "v1.3 Polish & Follow-through",
                "v1.4 Assessment & Insight",
                "v1.5 Closing the Loop",
                "Requirement Coverage",
            ]
        );
        let shipped = shipped_milestones(&ms, "v1.5");
        assert_eq!(shipped, [true, true, true, true, true, false, false]);
        let declared: u32 = ms
            .iter()
            .zip(&shipped)
            .filter(|(_, s)| **s)
            .map(|(m, _)| declared_phase_count(m))
            .sum();
        assert_eq!(declared, 17, "Mockup B: `5 milestones · 17 phases shipped`");
    }

    #[test]
    fn sentriq_v0_11_is_shipped_by_version_order() {
        let ms = roadmap_milestones(SENTRIQ_ROADMAP);
        assert_eq!(labels(&ms), ["v0.11 Phases", "Scope Explicitly Excluded from v0.12"]);
        assert_eq!(ms[0].first, PhaseNum::parse("4"), "`Phases (4-7)` range is read");
        assert_eq!(ms[0].last, PhaseNum::parse("7"));
        assert_eq!(
            active_milestone_index(&ms, "v0.12"),
            None,
            "no roadmap milestone is v0.12, so shipped-ness comes from version order"
        );
        assert_eq!(shipped_milestones(&ms, "v0.12"), [true, false]);
        assert_eq!(declared_phase_count(&ms[0]), 4);
    }

    #[test]
    fn ttbook_v1_is_shipped() {
        let ms = roadmap_milestones(TTBOOK_ROADMAP);
        assert_eq!(shipped_milestones(&ms, "v2"), [true, false, false, false, false]);
        assert_eq!(
            declared_phase_count(&ms[0]),
            7,
            "`Phases 1-7.1` counts the integer majors 1..7"
        );
    }

    #[test]
    fn a_milestone_without_range_or_members_is_never_shipped() {
        let ms = roadmap_milestones(
            "## Milestones\n\n\
             - ✅ **v1.0 Old** - shipped long ago\n\
             - ✅ **v1.1 Ranged** - Phases 1-2 (shipped)\n\
             - 🚧 **v2.0 Now** - Phases 3-4 (in progress)\n",
        );
        assert_eq!(shipped_milestones(&ms, "v2.0"), [false, true, false]);
        assert_eq!(shipped_milestones(&ms, ""), [false, true, false], "in-progress fallback");

        // Version order is numeric by segment, never lexicographic.
        let ms = roadmap_milestones(
            "## Plan\n\n### v0.9 Early (Phases 1-2)\n\n### v0.11 Later (Phases 3-4)\n\n\
             ### v0.20 Future (Phases 9-10)\n\n### v0.10 Loose\n",
        );
        assert_eq!(labels(&ms), ["v0.9 Early", "v0.11 Later", "v0.20 Future", "v0.10 Loose"]);
        assert_eq!(shipped_milestones(&ms, "v0.12"), [true, true, false, false]);
        assert_eq!(shipped_milestones(&ms, "v0.9 Early"), [false, false, false, false]);
        assert_eq!(
            shipped_milestones(&ms, "v0.10.1"),
            [true, false, false, false],
            "lexicographically `v0.11` < `v0.10.1`; numerically it is not"
        );
        assert_eq!(
            shipped_milestones(&ms, "v0.20"),
            [true, true, false, false],
            "a STATE milestone the roadmap names ships what is listed before it"
        );
        assert_eq!(
            shipped_milestones(&ms, "Closing the loop"),
            [false; 4],
            "a non-version STATE milestone ships nothing"
        );
    }

    #[test]
    fn declared_phase_count_counts_integer_majors() {
        let ms = roadmap_milestones(
            "## Milestones\n\n\
             - **v1 A** - Phases 1-7.1\n\
             - **v2 B** - Phases 08-13\n\
             - **v3 C** - Phase 14\n\
             - **v4 D** - no phases\n\n\
             ## Later\n\n### Milestone 5 \"E\"\n\n#### Build phase 20: X\n#### Build phase 21: Y\n",
        );
        let counts: Vec<u32> = ms.iter().map(declared_phase_count).collect();
        assert_eq!(counts, [7, 6, 1, 0, 2], "range majors, else scoped members, else 0");
    }

    /// T-24-03: the fixtures are excerpts of PRIVATE roadmaps shipped in a
    /// public crate. Every goal body must stay replaced, and no fixture may
    /// carry an absolute home path (see `tests/fixtures/roadmaps/README.md`).
    #[test]
    fn vendored_roadmap_fixtures_are_sanitised() {
        let fixtures = [
            ("daily-vow-ROADMAP.md", DAILY_VOW_ROADMAP),
            (
                "daily-vow-STATE.md",
                include_str!("../../tests/fixtures/roadmaps/daily-vow-STATE.md"),
            ),
            ("sentriq-ROADMAP.md", SENTRIQ_ROADMAP),
            (
                "sentriq-STATE.md",
                include_str!("../../tests/fixtures/roadmaps/sentriq-STATE.md"),
            ),
            ("ttbook-ROADMAP.md", TTBOOK_ROADMAP),
            (
                "ttbook-STATE.md",
                include_str!("../../tests/fixtures/roadmaps/ttbook-STATE.md"),
            ),
            (
                "ttbook-phase13-ROADMAP.md",
                include_str!("../../tests/fixtures/roadmaps/ttbook-phase13-ROADMAP.md"),
            ),
            (
                "ttbook-phase13-STATE.md",
                include_str!("../../tests/fixtures/roadmaps/ttbook-phase13-STATE.md"),
            ),
            (
                "ttbook-phase13-HANDOFF.json",
                include_str!("../../tests/fixtures/roadmaps/ttbook-phase13-HANDOFF.json"),
            ),
        ];
        let mut goal_lines = 0;
        for (name, content) in fixtures {
            assert!(!content.contains("/home/"), "{name} carries an absolute home path");
            for (no, line) in content.lines().enumerate() {
                if line.contains("**Goal") {
                    goal_lines += 1;
                    assert!(
                        line.contains("(sanitised)"),
                        "{name}:{}: a Goal line lost its `(sanitised)` marker: {line}",
                        no + 1
                    );
                }
            }
        }
        assert_eq!(goal_lines, 6 + 4 + 11 + 11, "the guard saw every fixture goal");
    }

    // --- phase_section (debug backlog-content-empty) -----------------------

    #[test]
    fn a_phase_section_keeps_deeper_sub_headings_and_ends_at_a_sibling() {
        let roadmap = "## Backlog\n\n### Phase 999.1: One (BACKLOG)\n\nBody one.\n\n\
                       #### Notes\n\nA sub-heading is part of the entry.\n\n\
                       ### Phase 999.2: Two (BACKLOG)\n\nBody two.\n";
        let one = phase_section(roadmap, "999.1").expect("999.1 has an entry");
        assert!(one.starts_with("### Phase 999.1: One (BACKLOG)"), "{one:?}");
        assert!(
            one.contains("#### Notes") && one.contains("part of the entry"),
            "{one:?}"
        );
        assert!(!one.contains("Body two"), "{one:?}");
        assert!(
            one.ends_with("A sub-heading is part of the entry."),
            "trailing blanks dropped: {one:?}"
        );
    }

    #[test]
    fn a_phase_section_ends_at_a_higher_heading_and_at_a_thematic_break() {
        let roadmap = "### Phase 999.1: One (BACKLOG)\n\nBody one.\n\n## Progress\n\nnot it\n\n\
                       ### Phase 999.2: Two (BACKLOG)\n\nBody two.\n\n* * *\n*footer*\n";
        let one = phase_section(roadmap, "999.1").unwrap();
        assert!(!one.contains("Progress"), "{one:?}");
        let two = phase_section(roadmap, "999.2").unwrap();
        assert!(
            two.contains("Body two.") && !two.contains("footer"),
            "{two:?}"
        );
    }

    #[test]
    fn a_heading_or_break_inside_a_code_fence_does_not_end_the_section() {
        let roadmap = "### Phase 999.1: One (BACKLOG)\n\n```bash\n# a shell comment\n---\n```\n\n\
                       After the fence.\n\n### Phase 999.2: Two (BACKLOG)\n";
        let one = phase_section(roadmap, "999.1").unwrap();
        assert!(
            one.contains("# a shell comment") && one.contains("After the fence."),
            "{one:?}"
        );
        assert!(!one.contains("Phase 999.2"), "{one:?}");
    }

    #[test]
    fn a_phase_section_matches_by_phase_key_and_reports_an_absent_entry_as_none() {
        let roadmap = "### Phase 0999.1: Padded (PROMOTED → v2.0)\n\nPadded body.\n";
        let padded = phase_section(roadmap, "999.1").expect("0999.1 names 999.1");
        assert!(padded.contains("Padded body."), "{padded:?}");
        assert_eq!(phase_section(roadmap, "999.10"), None);
        assert_eq!(phase_section(roadmap, "999.2"), None);
        assert_eq!(phase_section("", "999.1"), None);
    }

    /// quick-260924-drx: the editor jumps to the heading the pane displays —
    /// 1-based, fence-aware, and by phase key like `phase_section`.
    #[test]
    fn a_phase_section_line_is_the_one_based_line_of_its_heading() {
        let roadmap = "# Roadmap\n\n## Backlog\n\n\
                       ```\n### Phase 999.1: inside a fence\n```\n\n\
                       ### Phase 999.10: Ten (BACKLOG)\n\nTen.\n\n\
                       ### Phase 999.1: One (BACKLOG)\n\nOne.\n";
        assert_eq!(phase_section_line(roadmap, "999.1"), Some(13));
        assert_eq!(phase_section_line(roadmap, "999.10"), Some(9));
        assert_eq!(phase_section_line(roadmap, "999.2"), None);
        assert_eq!(
            roadmap.lines().nth(12),
            Some("### Phase 999.1: One (BACKLOG)"),
            "fixture self-check: line 13 is the 999.1 heading"
        );
    }

    // ── quick 260926-fi9: shipped-milestone collapses ───────────────────────

    #[test]
    fn ttbook_shipped_v1_lines_parse_as_shipped_phases() {
        let shipped = parse_shipped_phases(TTBOOK_ROADMAP);
        let numbers: Vec<&str> = shipped.iter().map(|p| p.number.as_str()).collect();
        assert_eq!(numbers, ["1", "2", "3", "4", "5", "6", "7", "7.1"]);
        let names: Vec<&str> = shipped.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "Access and session handling (R1)",
                "Stack and dependency review (R2)",
                "Architecture and data model (R3)",
                "Deployment and operations (R4)",
                "Module redesign (R5)",
                "Domain rules and open facts (R6)",
                "Consolidation",
                "Apply rulings and review findings (INSERTED)",
            ]
        );
        let plans: Vec<(u32, u32)> = shipped
            .iter()
            .map(|p| (p.completed_plans, p.total_plans))
            .collect();
        assert_eq!(
            plans,
            [(6, 6), (13, 13), (8, 8), (9, 9), (9, 9), (8, 8), (12, 12), (16, 16)]
        );
        assert!(shipped.iter().all(|p| p.completed));
        assert_eq!(shipped[0].description, "completed 2026-09-23; (sanitised)");
        assert!(shipped.iter().all(|p| p.depends_on.is_empty()));
    }

    #[test]
    fn closed_collapses_follow_gsds_marker_rule() {
        let block = |summary: &str| {
            format!(
                "<details>\n{summary}\n\n- [x] Phase 1: One (1/1 plans) - completed 2026-01-01\n\n</details>\n"
            )
        };
        for summary in [
            "<summary>v1.0 MVP (Phases 1-4) - SHIPPED 2026-01-01</summary>",
            "<summary>v1.0 MVP - closed early</summary>",
            "<summary>\u{2705} v1.0 MVP (Phases 1-4)</summary>",
            "<summary>v0.9 Archived notes</summary>",
        ] {
            let roadmap = block(summary);
            let lines: Vec<&str> = roadmap.lines().collect();
            assert!(
                closed_milestone_lines(&lines).iter().all(|c| *c),
                "{summary}: every line of the block, wrappers included, is closed"
            );
            let shipped = parse_shipped_phases(&roadmap);
            assert_eq!(shipped.len(), 1, "{summary}");
            assert!(parse_roadmap_phases(&roadmap).is_empty(), "{summary}");
        }
        for (why, roadmap) in [
            ("no marker", block("<summary>Completed phases</summary>")),
            (
                "closed and active markers",
                block("<summary>\u{2705} v1.0 done, \u{1F6A7} v1.1 in flight</summary>"),
            ),
            ("no summary", block("")),
            (
                "unterminated",
                "<details>\n<summary>v1.0 SHIPPED</summary>\n\n- [x] Phase 1: One (1/1 plans)\n"
                    .to_string(),
            ),
        ] {
            let lines: Vec<&str> = roadmap.lines().collect();
            assert!(
                closed_milestone_lines(&lines).iter().all(|c| !c),
                "{why}: not a closed collapse"
            );
            assert!(
                parse_shipped_phases(&roadmap).is_empty(),
                "{why}: a plain line outside a closed collapse is not shipped"
            );
        }

        // Outside the block nothing is flagged, and a plan item inside a
        // closed collapse is never credited to the GSD phase above it.
        let roadmap = "### Phase 5: Live\n\n<details>\n<summary>v1 SHIPPED</summary>\n\n\
                       - [x] 01-01-PLAN.md - archived\n</details>\n\n- [ ] 05-01-PLAN.md - live\n";
        let lines: Vec<&str> = roadmap.lines().collect();
        let flags = closed_milestone_lines(&lines);
        assert_eq!(flags, [false, false, true, true, true, true, true, false, false]);
        let live = parse_roadmap_phases(roadmap);
        assert_eq!(live.len(), 1);
        assert_eq!((live[0].completed_plans, live[0].total_plans), (0, 0));
    }

    // ── quick 260926-fi9 Task 2: every observed shape, both channels ───────

    const V1_ERA_ROADMAP: &str = include_str!("../../tests/fixtures/roadmap-shapes/v1-era-ROADMAP.md");

    fn numbers_of(phases: &[RoadmapPhase]) -> Vec<&str> {
        phases.iter().map(|p| p.number.as_str()).collect()
    }

    fn find<'a>(phases: &'a [RoadmapPhase], number: &str) -> &'a RoadmapPhase {
        phases
            .iter()
            .find(|p| p.number == number)
            .unwrap_or_else(|| panic!("phase {number} in {:?}", numbers_of(phases)))
    }

    #[test]
    fn v1_era_fixture_gsd_facing_phases_are_the_live_shapes() {
        let phases = parse_roadmap_phases(V1_ERA_ROADMAP);
        assert_eq!(numbers_of(&phases), ["12", "13", "13.1", "14", "16"]);

        let p = find(&phases, "12");
        assert_eq!(p.name, "Mu Bold With Dash");
        assert_eq!(p.description, "existing grammar, unchanged");
        assert!(p.completed);
        assert_eq!((p.completed_plans, p.total_plans), (1, 1));
        assert!(p.depends_on.is_empty());

        let p = find(&phases, "13");
        assert_eq!(p.name, "Nu Bold Bare");
        assert_eq!(p.description, "");
        assert!(!p.completed);
        assert_eq!((p.completed_plans, p.total_plans), (0, 1));
        assert_eq!(p.depends_on, ["12"]);

        let p = find(&phases, "13.1");
        assert_eq!(p.name, "Xi Bold Tagged (INSERTED)");
        assert_eq!(p.description, "an inserted phase");
        assert!(!p.completed);
        assert_eq!(p.depends_on, ["13"]);

        let p = find(&phases, "14");
        assert_eq!(p.name, "Omicron Plain Checkbox");
        assert_eq!(p.description, "plain checklist line");
        assert!(p.completed);
        assert_eq!(p.depends_on, ["13.1"]);

        let p = find(&phases, "16");
        assert_eq!(p.name, "Pi Collapsed But Active");
        assert_eq!(p.description, "lives in the active milestone's own block");
        assert!(!p.completed);
    }

    #[test]
    fn v1_era_fixture_shipped_phases_are_every_collapse_shape() {
        let shipped = parse_shipped_phases(V1_ERA_ROADMAP);
        assert_eq!(
            numbers_of(&shipped),
            ["1", "2", "2.1", "3", "4", "5", "6", "07", "08", "9", "10", "11"]
        );
        let facts = |n: &str| {
            let p = find(&shipped, n);
            (
                p.name.as_str(),
                p.description.as_str(),
                p.completed,
                (p.completed_plans, p.total_plans),
            )
        };
        assert_eq!(
            facts("1"),
            ("Alpha Foundation", "completed 2026-01-01", true, (3, 3))
        );
        assert_eq!(
            facts("2"),
            ("Beta Pipeline (R2)", "completed 2026-01-02", true, (13, 13))
        );
        assert_eq!(
            facts("2.1"),
            ("Hotfix Insert (INSERTED)", "completed 2026-01-03", true, (2, 2))
        );
        assert_eq!(
            facts("3"),
            (
                "Gamma Views",
                "first subtitle, second subtitle (2026-01-04)",
                true,
                (0, 0)
            )
        );
        assert_eq!(
            facts("4"),
            ("Delta Listing", "rescoped; carried to v1.1", false, (0, 0))
        );
        assert_eq!(
            facts("5"),
            ("Epsilon Widgets", "completed 2026-02-01", true, (3, 3))
        );
        assert_eq!(
            facts("6"),
            ("Zeta Wiring", "Wire the parts (completed 2026-02-02)", true, (0, 0))
        );
        assert_eq!(facts("07"), ("Eta Accuracy", "", true, (5, 5)));
        assert_eq!(facts("08"), ("Theta Cleanup", "", true, (1, 1)));
        assert_eq!(facts("9"), ("Iota Detail Heading", "", true, (2, 2)));
        assert_eq!(facts("10"), ("Kappa Unplanned", "", false, (0, 0)));
        assert_eq!(
            facts("11"),
            (
                "Lambda Tail",
                "SKIPPED (conditional, accuracy sufficient)",
                true,
                (0, 0)
            )
        );
    }

    #[test]
    fn v1_era_channels_share_no_key_and_negative_shapes_never_parse() {
        let live: std::collections::HashSet<String> = parse_roadmap_phases(V1_ERA_ROADMAP)
            .iter()
            .map(|p| phase_key(&p.number))
            .collect();
        for p in parse_shipped_phases(V1_ERA_ROADMAP) {
            assert!(!live.contains(&phase_key(&p.number)), "{} in both channels", p.number);
        }

        let negatives = [
            "### Phase 3 Implementation Scope",
            "**Phase 1 count:** 21 requirements (v1.0)",
            "**Phase 12** below, rather than being archived unfinished.",
            "1. **Phase 13 contains the one-way door** and must land first.",
            "  - Phase 14's oracle encodes a declared divergence.",
            "- [ ] Phase 12+: TBD — define via a later milestone",
            "**Depends on**: Phase 3",
            "Depends on: Phase 3",
            "| CHAIN-01..08 | 8 | Phase 4 |",
            "| **Gate 1** gate | Phase 1 | x |",
            "| 1. Name | 3/3 | Complete | 2026-01-01 |",
            "<summary>\u{2705} v1.0 MVP (Phases 1-4) — SHIPPED 2026-01-10</summary>",
        ];
        for line in negatives {
            assert!(parse_roadmap_phases(line).is_empty(), "current region: {line}");
            let collapsed = format!("<details>\n<summary>v0 SHIPPED</summary>\n\n{line}\n\n</details>\n");
            assert!(parse_shipped_phases(&collapsed).is_empty(), "closed region: {line}");
            assert!(parse_roadmap_phases(&collapsed).is_empty(), "closed region: {line}");
        }
        // A bare `Phase N:` line is prose outside a closed collapse.
        let bare = "Phase 20: a bare line outside any closed milestone is prose, not a phase";
        assert!(parse_roadmap_phases(bare).is_empty());
        assert!(parse_shipped_phases(bare).is_empty());
    }

    /// This repository's own v1.0-v1.2 history is written as bare
    /// `Phase 01: Name (3 plans, complete)` lines inside SHIPPED collapses.
    #[test]
    fn this_repositorys_own_shipped_history_parses_as_shipped_phases() {
        let roadmap = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(".planning")
            .join("ROADMAP.md");
        let content = std::fs::read_to_string(&roadmap)
            .expect("this repository ships its own .planning/ROADMAP.md");
        let shipped = parse_shipped_phases(&content);
        let expected: Vec<String> = (1..=13).map(|n| format!("{n:02}")).collect();
        let numbers = numbers_of(&shipped);
        assert!(
            numbers.len() >= expected.len() && numbers[..expected.len()] == expected[..],
            "{numbers:?}"
        );
        let first = &shipped[0];
        assert_eq!(first.name, "Project Foundation");
        assert_eq!((first.completed_plans, first.total_plans), (3, 3));
        assert!(first.completed);

        let live: std::collections::HashSet<String> = parse_roadmap_phases(&content)
            .iter()
            .map(|p| phase_key(&p.number))
            .collect();
        for id in &expected {
            assert!(!live.contains(&phase_key(id)), "{id} leaked into the GSD-facing list");
        }
    }

    #[test]
    fn dash_headings_are_phase_entries_with_goals_and_sections_never_milestones() {
        let roadmap = "## Phase Details\n\n\
                       ### Phase 2 — Two\n\n**Goal**: the dash goal\n\n\
                       ### Phase 3 -- Three\n\n**Goal**: double dash\n\n\
                       ### Phase 4 – Four\n\n**Goal**: en dash\n\n\
                       ### Phase 5 - Five\n\n**Goal**: hyphen\n";
        let goals = parse_phase_goals(roadmap);
        assert_eq!(goal_of(&goals, "2").as_deref(), Some("the dash goal"));
        assert_eq!(goal_of(&goals, "3").as_deref(), Some("double dash"));
        assert_eq!(goal_of(&goals, "4").as_deref(), Some("en dash"));
        assert_eq!(goal_of(&goals, "5").as_deref(), Some("hyphen"));
        let two = phase_section(roadmap, "2").expect("the dash heading opens an entry");
        assert!(two.starts_with("### Phase 2 — Two"), "{two:?}");
        assert!(!two.contains("Three"), "{two:?}");
        let phases = parse_roadmap_phases(roadmap);
        assert_eq!(numbers_of(&phases), ["2", "3", "4", "5"]);
        assert_eq!(phases[0].name, "Two");

        let ms = roadmap_milestones(
            "## Plan\n\n### Phase 2 - Migrate to v2 API\n\n### Phase 3 Implementation Scope\n",
        );
        assert!(ms.is_empty(), "{:?}", labels(&ms));
        assert!(parse_roadmap_phases("### Phase 3 Implementation Scope\n").is_empty());
    }

    #[test]
    fn bold_lines_parse_with_any_tail_and_the_existing_grammar_is_unchanged() {
        let bare = parse_roadmap_phases("- [ ] **Phase 2: Name**\n");
        assert_eq!(numbers_of(&bare), ["2"]);
        assert_eq!(bare[0].name, "Name");
        assert_eq!(bare[0].description, "");

        // A `--` tail is the existing grammar's: it keeps the second dash, as
        // it always has.
        let dashed = parse_roadmap_phases("- [x] **Phase 3: Three** -- desc\n");
        assert_eq!(dashed[0].description, "- desc");
        assert!(dashed[0].completed);

        let tagged = parse_roadmap_phases("- [ ] **Phase 03.1: Tagged** (INSERTED) - desc\n");
        assert_eq!(tagged[0].name, "Tagged (INSERTED)");
        assert_eq!(tagged[0].description, "desc");

        let tallied = parse_roadmap_phases("- [x] **Phase 4: Four** (2/3 plans) — completed 2026-01-01\n");
        assert_eq!(tallied[0].description, "completed 2026-01-01");
        assert_eq!((tallied[0].completed_plans, tallied[0].total_plans), (2, 3));

        let retired = parse_roadmap_phases("- [ ] ~~**Phase 5: Gone**~~ (dropped)\n");
        assert!(retired.is_empty());
    }
}
