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
    /// the order written. Empty when the entry has no dependency line.
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
///
/// The optional leading `[A-Za-z]{1,4}-` is the project-code/milestone prefix;
/// the numeric body is `[0-9][0-9.]*` with an optional trailing `[A-Za-z]`.
const PHASE_ID: &str = r"(?:[A-Za-z]{1,4}-)?[0-9][0-9.]*[A-Za-z]?";

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

/// Extract the phase identifiers declared by a `**Depends on**:` line's text.
///
/// Two rules, both narrowing:
///
/// 1. **Parenthetical groups are stripped first.** A qualifier is prose about
///    ordering, not a dependency — this repository's phase 20 declares
///    `Phase 16, Phase 17, Phase 19 (and Phase 22 must land before this phase
///    closes)`, and promoting `22` out of that aside would state a dependency
///    the roadmap does not. It also disarms `Nothing (no v2.0 dependencies…)`,
///    where a bare-number scan would invent a phase `2.0` from a version string
///    and leave the condition unsatisfiable forever.
/// 2. **Only `Phase <id>` occurrences count**, reusing [`PHASE_ID`] so the same
///    identifier forms the rest of this file accepts — bare numeric, decimal,
///    project-code-prefixed and milestone-prefixed — are accepted here too. The
///    keyword is required precisely because a bare number in prose is
///    indistinguishable from an identifier. `Phases 17-21` does not match: the
///    plural leaves no whitespace after `Phase`, so a range stays prose.
///
/// Order written is preserved; a repeated identifier appears once.
fn parse_depends_on(text: &str) -> Vec<String> {
    let without_qualifiers = Regex::new(r"\([^)]*\)").unwrap().replace_all(text, " ");
    let phase_ref = Regex::new(&format!(r"Phase\s+({id})", id = PHASE_ID)).unwrap();

    let mut out: Vec<String> = Vec::new();
    for caps in phase_ref.captures_iter(&without_qualifiers) {
        let id = caps[1].trim_end_matches(['.', ',']).to_string();
        if !id.is_empty() && !out.contains(&id) {
            out.push(id);
        }
    }
    out
}

/// Parse ROADMAP.md content and extract phase checklist items with per-phase plan counts.
///
/// Recognizes several heading shapes used by GSD 1.8.0 roadmaps:
/// - checklist form: `- [ ] **Phase N: Title** - desc`
/// - parenthetical cluster tags: `- [ ] **Phase 26 (Cluster B): Title** - desc`
/// - project-code / milestone-prefixed IDs: `Phase M-2`, `Phase AB-29`
/// - markdown headings: `### Phase 4: Visualization` (no checkbox → `completed: false`)
/// - `<details>` / `</details>` / `<summary>` wrapper lines are transparent, so
///   phases and plans nested inside a `<details>` block are still counted.
pub fn parse_roadmap_phases(content: &str) -> Vec<RoadmapPhase> {
    // Checklist form. Groups: 1=checkbox, 2=strike-open (`~~`), 3=id, 4=name,
    // 5=strike-close (`~~`), 6=description. The `(~~)?` groups let a retired
    // (strikethrough) phase still match so it can be filtered out explicitly.
    let checklist_re = Regex::new(&format!(
        r"- \[([ xX])\] (~~)?\*\*Phase ({id})(?:\s*\([^)]*\))?:\s*(.+?)\*\*(~~)?\s*[-\x{{2014}}]\s*(.*)",
        id = PHASE_ID
    ))
    .unwrap();
    // Markdown-heading form (no checkbox, no `**`). Groups: 1=id, 2=name.
    let heading_re = Regex::new(&format!(
        r"^\s*#{{2,4}}\s+Phase ({id})(?:\s*\([^)]*\))?:\s+(.+?)\s*$",
        id = PHASE_ID
    ))
    .unwrap();
    // The phase part of a plan filename may be decimal: an inserted phase's
    // plans are `07.1-01-PLAN.md`, and `\d+-` alone never counted them.
    let plan_re = Regex::new(r"^\s*- \[([ xX])\] (?:\d+(?:\.\d+)*-\d+-)?PLAN\.md").unwrap();
    // `**Depends on**: …`, with the emphasis markers optional.
    let depends_re = Regex::new(r"(?i)^\s*\*{0,2}Depends on\*{0,2}\s*:\s*(.*)$").unwrap();

    let is_header = |line: &str| checklist_re.is_match(line) || heading_re.is_match(line);

    let lines: Vec<&str> = content.lines().collect();
    let mut phases: Vec<RoadmapPhase> = Vec::new();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let parsed: Option<RoadmapPhase> = if let Some(caps) = checklist_re.captures(line) {
            let number = caps[3].to_string();
            // Strikethrough (`~~...~~`) marks a retired phase → exclude entirely.
            let retired = caps.get(2).is_some() || caps.get(5).is_some() || line.contains("~~");
            if retired || is_sentinel_phase(&number) {
                None
            } else {
                Some(RoadmapPhase {
                    completed: &caps[1] != " ",
                    number,
                    name: caps[4].trim().to_string(),
                    description: caps[6].trim().to_string(),
                    total_plans: 0,
                    completed_plans: 0,
                    depends_on: Vec::new(),
                })
            }
        } else if let Some(caps) = heading_re.captures(line) {
            // `###`-style headings carry no checkbox and no inline description.
            let number = caps[1].to_string();
            if line.contains("~~") || is_sentinel_phase(&number) {
                None
            } else {
                Some(RoadmapPhase {
                    completed: false,
                    number,
                    name: caps[2].trim().to_string(),
                    description: String::new(),
                    total_plans: 0,
                    completed_plans: 0,
                    depends_on: Vec::new(),
                })
            }
        } else {
            None
        };

        if let Some(mut phase) = parsed {
            // Scan subsequent lines for plan items. `<details>`/`</details>`/
            // `<summary>` lines don't match `plan_re` or the phase recognizers,
            // so they are transparent and never break the scan.
            let mut j = i + 1;
            while j < lines.len() {
                let l = lines[j];
                // Stop at the next phase header (either heading shape).
                if is_header(l) {
                    break;
                }
                if let Some(plan_caps) = plan_re.captures(l) {
                    phase.total_plans += 1;
                    if &plan_caps[1] != " " {
                        phase.completed_plans += 1;
                    }
                }
                // First dependency line inside the entry wins. Only the
                // `## Phase Details` copy of a phase carries one; the summary
                // checklist copy stops at the next header, so its list stays
                // empty and `merge_duplicate_phases` takes the detail copy's.
                if phase.depends_on.is_empty() {
                    if let Some(dep_caps) = depends_re.captures(l) {
                        phase.depends_on = parse_depends_on(&dep_caps[1]);
                    }
                }
                j += 1;
            }

            phases.push(phase);
            i += 1;
        } else {
            i += 1;
        }
    }

    merge_duplicate_phases(phases)
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

/// Extract the phase identifiers a build-phase entry's `**Depends on**:` line
/// declares. Applied ONLY inside a build-phase entry; [`parse_depends_on`]
/// stays the grammar for every GSD phase (its plural-range test pins that).
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
    let plan_re = Regex::new(r"^\s*- \[([ xX])\] (?:\d+(?:\.\d+)*-\d+-)?PLAN\.md").unwrap();
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
    static PHASE_HEADING: OnceLock<Regex> = OnceLock::new();
    static GOAL: OnceLock<Regex> = OnceLock::new();
    let phase_heading = PHASE_HEADING.get_or_init(|| {
        Regex::new(&format!(
            r"^\s*#{{2,4}}\s+Phase ({id})(?:\s*\([^)]*\))?:\s+(.+?)\s*$",
            id = PHASE_ID
        ))
        .unwrap()
    });
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

/// Which milestones are shipped, one flag per milestone (D-A07).
pub fn shipped_milestones(ms: &[RoadmapMilestone], _state_milestone: &str) -> Vec<bool> {
    vec![false; ms.len()]
}

/// How many phases a milestone declares.
pub fn declared_phase_count(_m: &RoadmapMilestone) -> u32 {
    0
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
    let range = Regex::new(&format!(
        r"(?i)\bphases?\s+({id})\s*[-\x{{2013}}\x{{2014}}]\s*({id})",
        id = PHASE_ID
    ))
    .unwrap();
    let single = Regex::new(&format!(r"(?i)\bphase\s+({id})", id = PHASE_ID)).unwrap();
    let any_heading = Regex::new(r"^\s*(#{1,6})[ \t]+(.*?)\s*$").unwrap();
    // A build-phase heading (`#### Build phase 14 (Milestone 3): …`) counts as
    // a phase heading here: otherwise its `(Milestone 3)` reads as a milestone
    // of its own, and its id never joins the enclosing milestone's scope.
    let phase_heading = Regex::new(&format!(
        r"^\s*#{{2,4}}\s+(?:(?i:build)\s+)?[Pp]hase ({id})(?:\s*\([^)]*\))?:",
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
    fn test_depends_on_accepts_every_phase_id_form_and_rejects_a_plural_range() {
        assert_eq!(
            depends_of("Phase 0.3, Phase M-2, Phase AB-29"),
            vec!["0.3", "M-2", "AB-29"],
            "the same identifier forms the rest of this file accepts"
        );
        assert_eq!(
            depends_of("Phase 15 only — parallel-eligible with Phases 17-21"),
            vec!["15"],
            "`Phases 17-21` is a prose range: the plural leaves no whitespace \
             after `Phase`, so no identifier is invented from it"
        );
        assert!(depends_of("Nothing").is_empty());
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
        assert_eq!(shipped_milestones(&ms, "v0.10"), [true, false, false, false]);
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
}
