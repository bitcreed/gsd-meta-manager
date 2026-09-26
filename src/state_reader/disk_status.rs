use super::plan_waves;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// GSD's eight-state phase vocabulary, in workflow order.
///
/// **Declaration order IS the ordering.** `Ord` is derived, so the position of a
/// variant in this list is its rank, and more than thirty comparisons across the
/// UI, the reader and the router read that rank. A new variant is therefore
/// *inserted at its semantic position*, never appended: an appended variant
/// sorts above [`DiskStatus::Complete`] and silently changes the meaning of
/// every one of those comparisons without failing to compile.
/// `test_disk_status_ordering` pins the relation rather than this comment
/// claiming it.
///
/// The vocabulary matches `init.cjs:1875-1888` one-for-one, which is what lets a
/// router rule keyed on a variant mean what the runtime means.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum DiskStatus {
    #[default]
    NoDirectory,
    Empty,
    Discussed,
    Researched,
    Planned,
    Partial,
    /// **Implementation complete, verification not yet passed** — GSD's own
    /// `executed` (`init.cjs:1876`, predicate at `:181`).
    ///
    /// Inserted between [`DiskStatus::Partial`] and [`DiskStatus::Complete`],
    /// which is the only correct position: it outranks a partially-summarised
    /// phase and is outranked by one whose verification passed. Before this
    /// variant existed, `Complete` carried this meaning, and a phase whose
    /// verification was `human_needed` therefore read as finished — the exact
    /// collapse DRIVE-05 exists to prevent.
    Executed,
    /// **Implementation complete AND verification passed** — GSD's `complete`
    /// (`init.cjs:194-195`: `implementationComplete && verificationPassed`).
    Complete,
}

/// The status read from a phase's `*-VERIFICATION.md` leading frontmatter.
///
/// Seven values, matching gsd-core 1.15.0's `VERIFICATION_ROUTING_TABLE` keys
/// (`src/verification.cts:91-146`) exactly; 1.15.0 added `unparseable`
/// (#4806). Only three are ever *written* by GSD's verifier
/// (`VERIFIER_STATUSES = ['passed', 'gaps_found', 'human_needed']`);
/// `stale`, `unparseable`, `missing` and `unknown` are constructed internally.
/// All seven are modelled here because a driven agent, a hand edit or a future
/// GSD version can put any of them on disk — and 1.15.0 accepts a literal
/// `status: unparseable` as a table key.
///
/// **Matched as a string with an explicit fallback that keeps the value**, in
/// the tolerant-wire-enum posture `executor::outcome` and `executor::stream_json`
/// already use: a typed parse that discarded an unrecognised value would lose the
/// one fact a human needs to see when GSD ships a seventh status.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum VerificationStatus {
    /// No `*-VERIFICATION.md`, no leading frontmatter block, an UNTERMINATED
    /// leading block (upstream's `extractFrontmatter` returns `{}` for one), or
    /// no `status` key.
    ///
    /// The default, and fail-safe by construction: an unreadable artifact yields
    /// "nothing observed", never an error and never a claim that verification
    /// passed.
    #[default]
    Missing,
    /// Verification passed. The **only** value that admits `DiskStatus::Complete`.
    Passed,
    /// The verifier found gaps. Takes precedence over staleness upstream
    /// (`verification.cjs:333-342`).
    GapsFound,
    /// The verifier needs a human judgement. The definitional DRIVE-05 gate.
    HumanNeeded,
    /// The report is stale: upstream's remedy is `/gsd-execute-phase N`, which
    /// re-runs the verifier (1.15.0, #4682) — verify-work cannot refresh it.
    ///
    /// **Read only from a literal `status: stale`.** Upstream also DERIVES
    /// staleness (a `covered_files`/`covered_digest` fingerprint scan with a
    /// SUMMARY-mtime fallback); that scan is not modelled by this reader, so a
    /// report upstream calls stale by fingerprint reads here as whatever its
    /// literal status says.
    Stale,
    /// The report has a closed leading frontmatter block that is not YAML —
    /// gsd-core 1.15.0's `unparseable` (#4806). The verifier ran, but nothing it
    /// concluded can be read, so a `status: passed` line that survives the
    /// breakage must never be believed. Also the reading of a literal
    /// `status: unparseable`, which 1.15.0 accepts as a table key.
    Unparseable,
    /// A value outside the table, carried **verbatim**.
    ///
    /// Never mapped onto a known arm and never dropped: an unrecognised status
    /// is not a passing one, and the observed bytes are what tells a reader
    /// which unknown it was.
    Unknown(String),
}

impl VerificationStatus {
    /// Classify a raw frontmatter value. Case-insensitive on the known arms;
    /// anything else is carried verbatim by [`VerificationStatus::Unknown`].
    pub fn from_raw(raw: &str) -> Self {
        let trimmed = raw.trim();
        match trimmed.to_ascii_lowercase().as_str() {
            "passed" => VerificationStatus::Passed,
            "gaps_found" => VerificationStatus::GapsFound,
            "human_needed" => VerificationStatus::HumanNeeded,
            "stale" => VerificationStatus::Stale,
            "unparseable" => VerificationStatus::Unparseable,
            "missing" => VerificationStatus::Missing,
            _ => VerificationStatus::Unknown(trimmed.to_string()),
        }
    }

    /// The stable identifier a later reader greps for. Exhaustive, no wildcard.
    pub fn as_str(&self) -> &str {
        match self {
            VerificationStatus::Missing => "missing",
            VerificationStatus::Passed => "passed",
            VerificationStatus::GapsFound => "gaps_found",
            VerificationStatus::HumanNeeded => "human_needed",
            VerificationStatus::Stale => "stale",
            VerificationStatus::Unparseable => "unparseable",
            VerificationStatus::Unknown(observed) => observed,
        }
    }

    /// Whether this status is the one that admits `DiskStatus::Complete`.
    ///
    /// Spelled as a predicate rather than an `== Passed` at each call site so
    /// the completion rule has exactly one definition.
    pub fn is_passed(&self) -> bool {
        matches!(self, VerificationStatus::Passed)
    }
}

/// The UAT statuses `uat-predicate.cjs:30-32` treats as outstanding, plus the
/// tolerant arms either side of that set.
///
/// Same posture as [`VerificationStatus`]: a value outside the vocabulary is
/// carried verbatim rather than mapped onto a member of it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum UatStatus {
    /// No `*-UAT.md`, no leading frontmatter block, or no `status` key.
    #[default]
    Missing,
    /// UAT is partially complete — outstanding.
    Partial,
    /// A UAT item was diagnosed but not closed — outstanding.
    Diagnosed,
    /// UAT has not been run — outstanding.
    Pending,
    /// UAT is blocked — outstanding.
    Blocked,
    /// UAT is underway — outstanding.
    InProgress,
    /// UAT failed — outstanding.
    Failed,
    /// Any other value, carried verbatim. **Not outstanding**: this repository's
    /// own phase 19 carries `status: deferred`, which is a human's explicit
    /// decision to proceed and must not read as an unanswered gate.
    Other(String),
}

impl UatStatus {
    /// Classify a raw frontmatter value. Case-insensitive on the known arms.
    pub fn from_raw(raw: &str) -> Self {
        let trimmed = raw.trim();
        match trimmed.to_ascii_lowercase().as_str() {
            "partial" => UatStatus::Partial,
            "diagnosed" => UatStatus::Diagnosed,
            "pending" => UatStatus::Pending,
            "blocked" => UatStatus::Blocked,
            "in_progress" => UatStatus::InProgress,
            "failed" => UatStatus::Failed,
            _ => UatStatus::Other(trimmed.to_string()),
        }
    }

    /// The stable identifier a later reader greps for. Exhaustive, no wildcard.
    pub fn as_str(&self) -> &str {
        match self {
            UatStatus::Missing => "missing",
            UatStatus::Partial => "partial",
            UatStatus::Diagnosed => "diagnosed",
            UatStatus::Pending => "pending",
            UatStatus::Blocked => "blocked",
            UatStatus::InProgress => "in_progress",
            UatStatus::Failed => "failed",
            UatStatus::Other(observed) => observed,
        }
    }

    /// Whether this status sets the outstanding-UAT gate (G7).
    ///
    /// The set is `uat-predicate.cjs:30-32`'s, taken as written. Everything
    /// outside it — including `passed` and `deferred` — is not a gate.
    pub fn is_outstanding(&self) -> bool {
        matches!(
            self,
            UatStatus::Partial
                | UatStatus::Diagnosed
                | UatStatus::Pending
                | UatStatus::Blocked
                | UatStatus::InProgress
                | UatStatus::Failed
        )
    }
}

/// The planned and realised token cost of ONE plan, as GSD records them.
///
/// GSD writes `estimate.tokens` into a `*-PLAN.md`'s frontmatter and
/// `actuals.tokens` into the paired `*-SUMMARY.md`'s. Both are optional and both
/// are absent from projects predating those keys, so each side is an `Option`
/// and a row exists only when at least ONE of them was found — a row carrying
/// neither number says nothing and is never constructed (D-INF-02).
///
/// `id` is the plan's filename stem minus the `-PLAN.md` suffix, i.e. the same
/// identity `plan_ids` carries. It is third-party filename text from another
/// project's directory: every renderer must put [`PlanTokens::label`] through
/// the UI's `shown()` escape before it reaches a terminal cell.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct PlanTokens {
    pub id: String,
    pub estimate: Option<u64>,
    pub actual: Option<u64>,
}

impl PlanTokens {
    /// A short, fixed-width-friendly display name for this plan.
    ///
    /// Zero-padded `NN-MM` when the id carries a plan index, so a column of
    /// labels aligns and `7-1` and `07-01` read as the one plan they are; the
    /// raw id when it does not; and the authored string `PLAN` for the
    /// standalone `PLAN.md` case, whose id is the empty string and whose label
    /// would otherwise be blank.
    pub fn label(&self) -> String {
        if let Some((phase, plan)) = plan_index(&self.id) {
            return format!("{}-{plan:02}", phase.padded());
        }
        if self.id.is_empty() {
            return "PLAN".to_string();
        }
        self.id.clone()
    }
}

/// One surviving plan as the scan read it (quick 260926-2l4, D-01/D-04): its
/// identity, a display title, where its objective starts, and its wave.
///
/// `id` is the plan's filename stem minus `-PLAN.md` — the identity
/// `plan_tokens`, `plan_waves` and `summarized_plans` carry — so the Waves pane
/// joins the four without a second spelling. `title` is text out of another
/// project's file and therefore arrives [`Untrusted`]; a renderer reads it
/// only through `Untrusted::shown`. `objective_line` is the 1-based line of the
/// `<objective>` tag, the convention `editor_args` hands `$EDITOR`.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanMeta {
    pub id: String,
    pub title: Option<crate::text::Untrusted>,
    pub objective_line: Option<usize>,
    pub wave: Option<u32>,
}

/// The longest title [`plan_title`] keeps, in characters (T-2l4-03).
const MAX_PLAN_TITLE_CHARS: usize = 160;

/// A plan's display title, read out of the plan-file content the scan already
/// holds [inferred I-3].
///
/// 1. The leading-frontmatter `title:` scalar, quotes stripped, when non-empty.
/// 2. Otherwise the first non-empty text after the `<objective>` tag — on the
///    tag's own line or the next non-empty one — cut at the first `: ` or `. `
///    sentence boundary, with markdown emphasis (`**`, backticks) removed and a
///    trailing `.`/`:` dropped.
/// 3. Otherwise `None`.
///
/// Capped at [`MAX_PLAN_TITLE_CHARS`]. H1 headings are NOT consulted: no plan
/// in this repository carries one outside a code fence.
fn plan_title(content: &str) -> Option<String> {
    let cap = |s: &str| -> Option<String> {
        let t: String = s.trim().chars().take(MAX_PLAN_TITLE_CHARS).collect();
        let t = t.trim().to_string();
        (!t.is_empty()).then_some(t)
    };
    if let Some(title) = leading_frontmatter_value(content, "title") {
        let title = title.trim().trim_matches(|c| c == '"' || c == '\'');
        if let Some(t) = cap(title) {
            return Some(t);
        }
    }
    let line = objective_line(content)?;
    let mut rest = content.lines().skip(line - 1);
    let tag_line = rest.next()?;
    let after_tag = tag_line.split_once("<objective>").map(|(_, r)| r).unwrap_or("");
    let first = std::iter::once(after_tag)
        .chain(rest)
        .map(str::trim)
        .find(|l| !l.is_empty())?;
    if first.starts_with("</objective>") {
        return None;
    }
    let text = first.replace("**", "").replace('`', "");
    let cut = [": ", ". "]
        .iter()
        .filter_map(|b| text.find(b))
        .min()
        .unwrap_or(text.len());
    let sentence = text[..cut].trim().trim_end_matches(['.', ':']);
    cap(sentence)
}

/// The 1-based line of the plan's `<objective>` tag: the first line whose
/// trimmed form starts with it, outside a fenced code block. `None` without one.
fn objective_line(content: &str) -> Option<usize> {
    let mut in_fence = false;
    for (index, line) in content.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if !in_fence && trimmed.starts_with("<objective>") {
            return Some(index + 1);
        }
    }
    None
}

/// Per-disposition finding counts out of a phase's
/// `{PADDED}-REVIEW-DISPOSITION.md` ledger (gsd-core 1.15.0). See
/// [`DiskInference::review_disposition`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ReviewDisposition {
    pub open: u32,
    pub fixed: u32,
    pub skipped: u32,
    pub deferred: u32,
}

impl ReviewDisposition {
    /// Every counted finding, whatever its disposition.
    pub fn total(&self) -> u32 {
        self.open + self.fixed + self.skipped + self.deferred
    }
}

/// The most of a `*-REVIEW-DISPOSITION.md` the scan reads.
const REVIEW_DISPOSITION_READ_CAP: u64 = 256 * 1024;

/// Count a disposition ledger's table rows.
fn parse_review_disposition(_content: &str) -> Option<ReviewDisposition> {
    None
}

/// The largest `waves.json` the scan reads (T-2l4-03, [inferred I-13]).
const MAX_WAVES_MANIFEST_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct DiskInference {
    pub status: DiskStatus,
    pub plan_count: u32,
    pub summary_count: u32,
    pub has_plans: bool,
    pub has_summaries: bool,
    pub has_context: bool,
    pub has_research: bool,
    pub has_verification: bool,
    /// The `status` read from the phase's `*-VERIFICATION.md` frontmatter.
    ///
    /// **Presence and status answer different questions, and only the status can
    /// express a gate.** `has_verification` says an artifact exists; this says
    /// what it concluded. A phase with `has_verification: true` and
    /// `VerificationStatus::HumanNeeded` is *not* finished, and reading presence
    /// as completion is how an unattended run walks past the one gate DRIVE-05
    /// exists to stop at. `has_verification` is kept because the dashboard reads
    /// it as an artifact-presence badge.
    pub verification_status: VerificationStatus,
    pub has_security: bool,
    pub has_uat: bool,
    /// The `status` read from the phase's `*-UAT.md` frontmatter (G7).
    ///
    /// Presence is `has_uat`; whether anyone still owes an answer is this.
    /// [`UatStatus::is_outstanding`] is the gate predicate.
    pub uat_status: UatStatus,
    /// A phase-directory `.continue-here.md` carrying **at least one
    /// blocking-severity row** (G13).
    ///
    /// **Not a file-existence check, and it must never be simplified into one.**
    /// This repository carries a stale `.continue-here.md` in
    /// `.planning/phases/19-gitsafe-git-blast-radius-envelope/`, left behind by a
    /// completed phase; every one of its severity rows reads `advisory`. An
    /// existence test — or a substring search for the word "blocking", which its
    /// prose contains as part of the filename `tests/async_blocking_guard.rs` —
    /// would park every run against this project forever.
    /// `test_phase_19_stale_continue_here_marker_is_not_blocking` pins that.
    pub continue_here_blocking: bool,
    pub has_spec: bool,
    pub has_eval_review: bool,
    /// Sub-stage artifacts (per /gsd-settings Planning + Execution toggles).
    pub has_patterns: bool,
    pub has_plan_check: bool,
    pub has_validation: bool,
    pub has_ui_spec: bool,
    pub has_ui_check: bool,
    pub has_ai_spec: bool,
    pub has_review: bool,
    /// The phase's code-review disposition ledger counts.
    pub review_disposition: Option<ReviewDisposition>,
    pub has_ui_review: bool,
    /// GSD 1.8.0 informational artifacts — never affect plan/summary counts.
    pub has_coverage: bool,
    pub has_windows: bool,
    pub has_deferred_items: bool,
    pub has_skeleton: bool,
    /// Per-plan token cost: what the plan estimated, what its summary measured.
    ///
    /// One row per SURVIVING plan for which at least one of the two numbers was
    /// found; a plan carrying neither contributes no row, so a project whose
    /// plans predate GSD's `estimate`/`actuals` keys leaves this empty and the
    /// UI renders exactly what it rendered before the field existed.
    ///
    /// **Ordered by numeric plan index, and the order is load-bearing.**
    /// `plan_ids` is a `HashSet`, whose iteration order is not stable across
    /// runs, while `DiskInference` derives `PartialEq` and is compared to
    /// suppress a spurious "Updated" status (quick task 260512-eyv). An
    /// unsorted vector would make every project compare unequal on every
    /// refresh.
    pub plan_tokens: Vec<PlanTokens>,
    /// The phase's surviving plans grouped by the `wave:` number their own
    /// frontmatter declares — which plans ran concurrently, and which were
    /// serialized behind them.
    ///
    /// Empty for a phase whose plans record no wave, which is the graceful
    /// degradation the UI depends on: no wave section is drawn at all, rather
    /// than a heading over a flat plan list. Read out of the SAME plan-file read
    /// the superseded check already performs, so it costs no extra I/O, and the
    /// derivation happens here rather than at render time.
    ///
    /// **Ordered, and the order is load-bearing** for the same reason
    /// [`DiskInference::plan_tokens`] is: directory iteration order is
    /// unspecified while this struct's `PartialEq` drives the dashboard's
    /// unchanged-state suppression. [`plan_waves::group_into_waves`] sorts.
    pub plan_waves: Vec<plan_waves::PlanWave>,
    /// The surviving plans whose summary paired — the plans this phase's own
    /// directory records as done (D-C11).
    ///
    /// Each entry is the plan's id exactly as [`DiskInference::plan_waves`]
    /// carries it (the plan's filename stem minus `-PLAN.md`, slug included),
    /// so the wave model can join the two without a second spelling. Filled out
    /// of the SAME Pass 2 pairing that computes `summary_count`, so it costs no
    /// extra I/O and can never disagree with that count: `summarized_plans.len()`
    /// IS `summary_count`.
    ///
    /// **Ordered, and the order is load-bearing** for the same reason
    /// [`DiskInference::plan_tokens`] is: the pairing collects into a
    /// `HashSet`, whose iteration order is not stable across runs, while this
    /// struct's `PartialEq` drives the dashboard's unchanged-state suppression.
    /// Sorted by numeric plan index, ids without one last.
    pub summarized_plans: Vec<String>,
    /// Every SURVIVING plan with its title, objective line and wave, read out
    /// of the same single plan-file read the superseded check performs.
    ///
    /// **Sorted by numeric plan index, and the order is load-bearing** for the
    /// same reason [`DiskInference::plan_tokens`] is: this struct's `PartialEq`
    /// drives the dashboard's unchanged-state suppression, and directory
    /// iteration order is unspecified.
    pub plans: Vec<PlanMeta>,
    /// The phase directory's `waves.json` manifest, parsed ONCE here (D-04).
    ///
    /// `None` when the file is absent, unparsable, over 1 MiB, or lists no
    /// wave. The Phases-tab Waves pane reads the grouping from this cache and
    /// never from disk; a manifest that exists only on disk and not here is
    /// never drawn.
    pub waves_manifest: Option<plan_waves::WavesManifest>,
}

/// Read a scalar key out of a file's **leading** YAML frontmatter block.
///
/// A cheap line scan, no YAML dependency, returning the first match's trimmed
/// value. Absence of a leading block, or of the key inside it, yields `None` —
/// fail-safe, never an error.
///
/// **The byte-zero anchor is load-bearing, not stylistic.** The block must open
/// on the very first line with a bare `---`, and the scan stops at the closing
/// `---`. GSD's own verification library records the defect this prevents
/// (`verification.cjs`'s `DEFECT.FRONTMATTER-SCALAR-BROAD-GREP`): a broad search
/// for `status:` false-matched the key inside a fenced code block further down
/// the file, so a document *describing* a status was read as *having* one.
/// Widening this to a whole-file search reintroduces that defect verbatim.
///
/// **The key must sit at column zero, and that is the same defect one level in**
/// (WR-05). Trimming the key made an indented mapping key indistinguishable from
/// a top-level one, so
///
/// ```text
/// ---
/// verification:
///   status: passed
/// status: human_needed
/// ---
/// ```
///
/// returned `passed` — the first match wins, and the nested one comes first.
/// This function is the single input to `VerificationStatus`, which is the
/// goal-met predicate and the whole DRIVE-05 gate set, so a nested
/// `status: passed` is a false `Decision::GoalMet` and a nested
/// `status: human_needed` a spurious park. A *nested* key is a different key.
pub(crate) fn leading_frontmatter_value(content: &str, key: &str) -> Option<String> {
    let mut lines = content.lines();
    // Frontmatter must open on the very first line with a bare `---`.
    if lines.next().map(str::trim) != Some("---") {
        return None;
    }
    for line in lines {
        if line.trim() == "---" {
            // End of the leading block. Nothing below it is frontmatter.
            return None;
        }
        // Column zero or it is somebody else's key. (List items were already
        // safe by accident — `- status` does not equal `status` — but a plain
        // indented mapping key was not.)
        if line.starts_with(char::is_whitespace) {
            continue;
        }
        if let Some((found, value)) = line.split_once(':') {
            if found == key {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// The leading `NN-MM` plan index of a plan/summary filename stem, as numbers.
///
/// **The stem is the plan's identity; the index is the identity a plan shares
/// with its summary.** GSD names a plan with a descriptive slug and its summary
/// without one:
///
/// ```text
/// 13-01-verse-pipeline-fourteen-translations-PLAN.md   stem "13-01-verse-…"
/// 13-01-SUMMARY.md                                     stem "13-01"
/// ```
///
/// Pairing on the stem therefore discards every summary a real GSD project
/// emits, which collapses a finished phase to `Planned`. Pairing on this index
/// derives the same key from both sides.
///
/// Parsed as numbers rather than compared as text, because `13-1` and `13-01`
/// are one index and a lexical compare says otherwise — a bug class this
/// codebase has already paid for once in phase-number handling. The phase half
/// is a [`PhaseNum`](super::phase_num::PhaseNum), so an inserted phase's
/// `07.1-01-slug-PLAN.md` pairs with its `07.1-01-SUMMARY.md` (a `u32` parse
/// rejected `07.1` and left the pair to the exact-stem rule, which cannot match
/// a slugged plan).
///
/// Returns `None` for anything that is not `digits-digits[-…]`: a phase-level
/// `13-SUMMARY.md` (stem `13`) has no plan index and so pairs with no plan, a
/// standalone `SUMMARY.md` (stem ``) likewise, and `14-REMEDIATION-SUMMARY.md`
/// likewise. Those fall back to the exact-stem rule, which is what they want.
pub(crate) fn plan_index(stem: &str) -> Option<(super::phase_num::PhaseNum, u32)> {
    let mut parts = stem.splitn(3, '-');
    let phase = super::phase_num::PhaseNum::parse(parts.next()?)?;
    let plan = parts.next()?.parse::<u32>().ok()?;
    Some((phase, plan))
}

/// Detect whether a plan file's YAML frontmatter declares `status: superseded`.
///
/// GSD 1.8.0 (#2349): a plan marked `status: superseded` was deliberately
/// reassigned or never executed — its work moved to a later plan, so it can
/// never gain a matching `*-SUMMARY.md`. Such a plan is excluded from BOTH the
/// plan and summary counts. A plan without the marker is counted exactly as
/// before. Fail-safe: a file with no frontmatter, or a closed block with no
/// `status: superseded`, is treated as a normal plan.
/// Read a scalar key nested ONE LEVEL inside a block of the **leading** YAML
/// frontmatter — `parent:` at column zero, `key:` as its direct child.
///
/// Same byte-zero anchor as [`leading_frontmatter_value`], for the same reason:
/// the block must open on the very first line with a bare `---`, and the scan
/// stops at the closing `---`. See that function's WR-05 paragraph — this is its
/// doctrine INVERTED rather than an exception beside it. There, a nested key had
/// to be refused because a column-zero key was wanted; here a column-zero key is
/// refused because the nested one is wanted. Both readings say the same thing: a
/// key one level in is a DIFFERENT key, and whichever one is asked for, the
/// other must not answer.
///
/// The rules, stated so the implementation can be checked against them:
///
/// * A blank line, and a line whose trimmed form starts with `#`, is SKIPPED and
///   ends nothing. GSD writes a column-zero `# Actuals (#2632)` heading above
///   the parent and indented `# …` comments inside the block before the key; a
///   reader that ends the block on either reads every plan and no summary.
/// * An unindented line is a column-zero key line. It OPENS the block when its
///   key equals `parent` and its value is empty, and CLOSES an already-open
///   block otherwise.
/// * Inside the block, the indentation width of the first non-blank,
///   non-comment line is the block's own level, and only lines at exactly that
///   width are candidate children. A deeper nesting is a different key one level
///   further in, and is skipped rather than read.
///
/// Returns the trimmed value of the first matching child. Every failure mode —
/// no leading block, no such parent, no such child — yields `None`, fail-safe,
/// never an error.
pub(crate) fn leading_frontmatter_nested_value(
    content: &str,
    parent: &str,
    key: &str,
) -> Option<String> {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return None;
    }
    let mut in_block = false;
    let mut block_indent: Option<usize> = None;
    for line in lines {
        let trimmed = line.trim();
        if trimmed == "---" {
            // End of the leading block. Nothing below it is frontmatter.
            return None;
        }
        // Blank lines and comments end nothing, at any indentation.
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if !line.starts_with(char::is_whitespace) {
            if in_block {
                // A column-zero key line closes the open block.
                return None;
            }
            if let Some((found, value)) = line.split_once(':') {
                if found == parent && value.trim().is_empty() {
                    in_block = true;
                }
            }
            continue;
        }
        if !in_block {
            continue;
        }
        let indent = line.len() - line.trim_start().len();
        let level = *block_indent.get_or_insert(indent);
        if indent != level {
            // One level further in (or further out) is a different key.
            continue;
        }
        if let Some((found, value)) = trimmed.split_once(':') {
            if found.trim() == key {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

/// The `tokens` count of a nested frontmatter block, as a number.
///
/// `parent` is `"estimate"` for a `*-PLAN.md` and `"actuals"` for its
/// `*-SUMMARY.md`. The value is truncated at the first whitespace-preceded `#`
/// (GSD's plan template writes `tokens: 60000   # calibrated projection`),
/// stripped of surrounding quotes, and parsed as a `u64`. Anything that does not
/// parse is `None` — fail-safe, exactly as the rest of this file behaves on a
/// malformed input, because a guessed number is worse than an absent one.
///
/// Note `raw_tokens` is a SIBLING of `tokens`, not a spelling of it: the key
/// comparison in [`leading_frontmatter_nested_value`] is an equality after the
/// first `:`, never a substring test, so the uncalibrated figure can never be
/// reported as the calibrated one.
fn frontmatter_token_count(content: &str, parent: &str) -> Option<u64> {
    let raw = leading_frontmatter_nested_value(content, parent, "tokens")?;
    let mut value = raw.as_str();
    let mut prev_was_space = false;
    for (index, ch) in raw.char_indices() {
        if ch == '#' && prev_was_space {
            value = &raw[..index];
            break;
        }
        prev_was_space = ch.is_whitespace();
    }
    value
        .trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .trim()
        .parse::<u64>()
        .ok()
}

fn plan_frontmatter_superseded(content: &str) -> bool {
    leading_frontmatter_value(content, "status")
        .is_some_and(|value| value.eq_ignore_ascii_case("superseded"))
}

/// Read the `status` out of the phase directory's verification artifact.
///
/// `names` is every `*-VERIFICATION.md` seen in the directory. **They are sorted
/// and the first is read**, which is the tie-break GSD's own reader uses
/// (`verification.cjs:302-303`), so a directory holding more than one artifact
/// yields the same status on every read rather than whatever the filesystem
/// happened to hand back first.
///
/// No artifact, an unreadable file, no leading block, an UNTERMINATED leading
/// block, or no `status` key yields [`VerificationStatus::Missing`]. A closed
/// leading block that is not YAML yields [`VerificationStatus::Unparseable`]
/// (gsd-core 1.15.0, #4806) — checked BEFORE the line scan, because a
/// `status: passed` line that survives a broken block is exactly what upstream
/// (1.14.0 and 1.15.0 alike) refuses to believe, and believing it is a false
/// `Complete` and a false `GoalMet`.
///
/// Unterminated is upstream's `extractFrontmatter` returning `{}`, i.e.
/// `missing`: a body `status:` line is never frontmatter (the
/// `DEFECT.FRONTMATTER-SCALAR-BROAD-GREP` class, one step further).
fn read_verification_status(phase_dir: &Path, mut names: Vec<String>) -> VerificationStatus {
    names.sort();
    let Some(content) = names
        .first()
        .and_then(|name| std::fs::read_to_string(phase_dir.join(name)).ok())
    else {
        return VerificationStatus::Missing;
    };
    verification_status_of(&content)
}

/// The verification status one document's content expresses. Split out of
/// [`read_verification_status`] so the classification is testable on strings.
fn verification_status_of(content: &str) -> VerificationStatus {
    match leading_block(content) {
        LeadingBlock::Absent | LeadingBlock::Unterminated => VerificationStatus::Missing,
        LeadingBlock::Closed(yaml) => {
            if !super::state_md::frontmatter_block_parses(&yaml) {
                return VerificationStatus::Unparseable;
            }
            leading_frontmatter_value(content, "status")
                .map(|raw| VerificationStatus::from_raw(&raw))
                .unwrap_or_default()
        }
    }
}

/// How a document's leading frontmatter block is shaped.
#[derive(Debug, PartialEq, Eq)]
enum LeadingBlock {
    /// The first line is not a bare `---`.
    Absent,
    /// The first line opens a block that no later line closes.
    Unterminated,
    /// A closed block, carrying the text of the lines strictly between the
    /// fences joined by `\n`.
    Closed(String),
}

/// Classify a document's leading block by exactly
/// [`leading_frontmatter_value`]'s byte-zero rule: the first line trimmed is
/// `---`, and the block ends at the first later line whose trimmed form is
/// `---`. `str::lines` strips a `\r`, so a CRLF document classifies the same.
fn leading_block(content: &str) -> LeadingBlock {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return LeadingBlock::Absent;
    }
    let mut inner: Vec<&str> = Vec::new();
    for line in lines {
        if line.trim() == "---" {
            return LeadingBlock::Closed(inner.join("\n"));
        }
        inner.push(line);
    }
    LeadingBlock::Unterminated
}

/// Read the `status` out of the phase directory's UAT artifact.
///
/// Same sorted-first tie-break and the same byte-zero-anchored parse
/// [`read_verification_status`] uses, for the same reason: one directory, one
/// answer, on every read. Every failure mode yields [`UatStatus::Missing`].
fn read_uat_status(phase_dir: &Path, mut names: Vec<String>) -> UatStatus {
    names.sort();
    names
        .first()
        .and_then(|name| std::fs::read_to_string(phase_dir.join(name)).ok())
        .and_then(|content| leading_frontmatter_value(&content, "status"))
        .map(|raw| UatStatus::from_raw(&raw))
        .unwrap_or_default()
}

/// Normalize a markdown cell or value for comparison: strip emphasis and code
/// ticks, trim, lowercase.
fn normalize_cell(cell: &str) -> String {
    cell.trim()
        .trim_matches('*')
        .trim_matches('`')
        .trim()
        .to_ascii_lowercase()
}

/// Split a markdown table row into cells, dropping the outer pipes.
fn split_markdown_row(line: &str) -> Vec<String> {
    line.trim()
        .trim_matches('|')
        .split('|')
        .map(|c| c.trim().to_string())
        .collect()
}

/// True when every cell of a table row is a `---` / `:--:` alignment marker.
fn is_alignment_row(cells: &[String]) -> bool {
    !cells.is_empty()
        && cells.iter().all(|c| {
            let t = c.trim();
            !t.is_empty() && t.chars().all(|ch| ch == '-' || ch == ':')
        })
}

/// Whether a `.continue-here.md` carries at least one blocking-severity ROW.
///
/// **A row, never the file.** GSD's `execute-phase.md:217-235` and
/// `discuss-phase.md:162-177` stop on a marker whose rows carry
/// `severity: blocking`; a marker whose rows are all advisory is a note, not a
/// gate. Two row shapes are recognised, both of which locate the severity as a
/// *field* rather than as text anywhere in the document:
///
/// 1. A markdown table with a `Severity` header column — the cell at that
///    column index is compared, so prose in a neighbouring column cannot match.
/// 2. A `severity: blocking` key line (optionally list-prefixed).
///
/// A substring search for "blocking" is deliberately NOT one of them. This
/// repository's own stale marker contains the word inside the filename
/// `tests/async_blocking_guard.rs`, and would fire on it.
fn continue_here_has_blocking_row(content: &str) -> bool {
    let mut severity_column: Option<usize> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('|') {
            let cells = split_markdown_row(trimmed);
            if is_alignment_row(&cells) {
                continue;
            }
            match severity_column {
                // Inside a table whose Severity column is known: compare that
                // cell, and only that cell.
                Some(index) => {
                    if cells.get(index).map(|c| normalize_cell(c)).as_deref() == Some("blocking") {
                        return true;
                    }
                }
                // Not yet in a severity table: is this row its header?
                None => {
                    severity_column = cells
                        .iter()
                        .position(|cell| normalize_cell(cell) == "severity");
                }
            }
            continue;
        }

        // A non-table line ends whatever table we were in, so a later table's
        // rows are never read against an earlier table's column index.
        severity_column = None;

        // Key-line shape: `severity: blocking`, `- severity: blocking`.
        if let Some((key, value)) = trimmed.split_once(':') {
            let key = key.trim_start_matches(['-', '*', ' ']).trim();
            if key.eq_ignore_ascii_case("severity") && normalize_cell(value) == "blocking" {
                return true;
            }
        }
    }

    false
}

/// Whether a phase directory's `.continue-here.md` sets the G13 gate.
///
/// Fail-safe: an absent or unreadable marker is "no gate observed".
fn phase_continue_here_blocking(phase_dir: &Path) -> bool {
    std::fs::read_to_string(phase_dir.join(".continue-here.md"))
        .map(|content| continue_here_has_blocking_row(&content))
        .unwrap_or(false)
}

/// Infer the GSD status of a phase directory by scanning its file artifacts.
///
/// Follows GSD's algorithm (from roadmap.cjs:127-166):
/// 1. If not a directory, return NoDirectory
/// 2. Count files matching *-PLAN.md or PLAN.md
/// 3. Count files matching *-SUMMARY.md or SUMMARY.md
/// 4. Check for *-CONTEXT.md or CONTEXT.md
/// 5. Check for *-RESEARCH.md or RESEARCH.md
/// 6. Check for *-VERIFICATION.md or VERIFICATION.md
/// 7. Derive status from artifact presence
pub fn infer_disk_status(phase_dir: &Path) -> DiskInference {
    if !phase_dir.is_dir() {
        return DiskInference {
            status: DiskStatus::NoDirectory,
            ..Default::default()
        };
    }

    let entries = match std::fs::read_dir(phase_dir) {
        Ok(entries) => entries,
        Err(_) => {
            return DiskInference {
                status: DiskStatus::NoDirectory,
                ..Default::default()
            }
        }
    };

    // GSD 1.8.0 counting is a two-pass scan (#1988, #2349):
    //   Pass 1 collects the IDs of surviving (non-superseded) plans.
    //   Pass 2 counts only summaries whose ID matches a surviving plan.
    // Both passes run inside the single directory iteration below: plan IDs are
    // gathered into `plan_ids` and candidate summary names into `summary_names`,
    // then paired after the loop.
    //
    // A plan's ID is its full filename stem, which is the plan's own identity —
    // but it is NOT the identity it shares with its summary, because GSD gives
    // plans a descriptive slug and summaries none. `plan_indices` therefore
    // carries the second, shared key (see [`plan_index`]) alongside it.
    let mut plan_ids: HashSet<String> = HashSet::new();
    let mut plan_indices: HashMap<(super::phase_num::PhaseNum, u32), Vec<String>> =
        HashMap::new();
    let mut summary_names: Vec<String> = Vec::new();
    // `estimate.tokens` per surviving plan, read out of the SAME plan-file read
    // the superseded check already performs, so it costs no extra I/O. Only a
    // number that was actually found is recorded.
    let mut plan_estimates: HashMap<String, u64> = HashMap::new();
    // `(plan id, wave number)` for every SURVIVING plan, read out of that same
    // in-hand content. A plan that could not be read contributes a `None` wave
    // rather than disappearing, so it still shows up in the unknown bucket.
    let mut plan_wave_entries: Vec<(String, Option<u32>)> = Vec::new();
    // Title/objective/wave per surviving plan, from that same in-hand content.
    let mut plans: Vec<PlanMeta> = Vec::new();
    // Read only when the walk actually meets the file, so no extra syscall is
    // spent on the (common) directory without one.
    let mut waves_manifest: Option<plan_waves::WavesManifest> = None;
    // Verification artifacts are COLLECTED, not flagged: the status lives inside
    // the file, and which file to read is decided after the scan by sorting.
    let mut verification_names: Vec<String> = Vec::new();
    // UAT artifacts are collected for the same reason, and read the same way.
    let mut uat_names: Vec<String> = Vec::new();
    let mut has_context = false;
    let mut has_research = false;
    let mut has_patterns = false;
    let mut has_plan_check = false;
    let mut has_validation = false;
    let mut has_ui_spec = false;
    let mut has_ui_check = false;
    let mut has_ai_spec = false;
    let mut has_review = false;
    let mut has_ui_review = false;
    let mut has_security = false;
    let mut has_spec = false;
    let mut has_eval_review = false;
    let mut has_coverage = false;
    let mut has_windows = false;
    let mut has_deferred_items = false;
    let mut has_skeleton = false;

    for entry in entries.flatten() {
        let name = match entry.file_name().into_string() {
            Ok(n) => n,
            Err(_) => continue,
        };

        // Only consider files (not directories)
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(true) {
            continue;
        }

        // GSD 1.8.0's claude-orchestration manifest, parsed once per refresh
        // (D-04) instead of once per rendered frame. Oversized files are
        // skipped unread; an empty manifest is recorded as none.
        if name == "waves.json" {
            let small = entry
                .metadata()
                .map(|m| m.len() <= MAX_WAVES_MANIFEST_BYTES)
                .unwrap_or(false);
            if small {
                waves_manifest = std::fs::read_to_string(entry.path())
                    .ok()
                    .and_then(|raw| plan_waves::parse_waves_manifest(&raw))
                    .filter(|m| !m.waves.is_empty());
            }
            continue;
        }

        // Skip review (UI-REVIEW vs REVIEW) and validate before generic SUMMARY/PLAN
        // matches so we don't double-count.
        if name == "UI-REVIEW.md" || name.ends_with("-UI-REVIEW.md") {
            has_ui_review = true;
            continue;
        }
        if name == "EVAL-REVIEW.md" || name.ends_with("-EVAL-REVIEW.md") {
            has_eval_review = true;
            continue;
        }
        if name == "REVIEW.md" || name.ends_with("-REVIEW.md") {
            has_review = true;
            continue;
        }
        if name == "UI-SPEC.md" || name.ends_with("-UI-SPEC.md") {
            has_ui_spec = true;
            continue;
        }
        if name == "UI-CHECK.md" || name.ends_with("-UI-CHECK.md") {
            has_ui_check = true;
            continue;
        }
        if name == "AI-SPEC.md" || name.ends_with("-AI-SPEC.md") {
            has_ai_spec = true;
            continue;
        }
        if name == "SPEC.md" || name.ends_with("-SPEC.md") {
            has_spec = true;
            continue;
        }
        if name == "PATTERNS.md" || name.ends_with("-PATTERNS.md") {
            has_patterns = true;
            continue;
        }
        if name == "PLAN-CHECK.md" || name.ends_with("-PLAN-CHECK.md") {
            has_plan_check = true;
            continue;
        }
        // PLAN-REVIEW artifacts are review notes, not plans (#2349): skip before
        // the PLAN match so they never count toward plan_count or set has_plans.
        if name == "PLAN-REVIEW.md" || name.ends_with("-PLAN-REVIEW.md") {
            continue;
        }
        if name == "VALIDATION.md" || name.ends_with("-VALIDATION.md") {
            has_validation = true;
            continue;
        }
        if name == "SECURITY.md" || name.ends_with("-SECURITY.md") {
            has_security = true;
            continue;
        }
        if name == "UAT.md" || name.ends_with("-UAT.md") {
            uat_names.push(name);
            continue;
        }
        // GSD 1.8.0 informational artifacts — flagged only, never counted.
        if name == "COVERAGE.md" || name.ends_with("-COVERAGE.md") {
            has_coverage = true;
            continue;
        }
        if name == "WINDOWS.md" || name.ends_with("-WINDOWS.md") {
            has_windows = true;
            continue;
        }
        if name == "deferred-items.md" || name.ends_with("-deferred-items.md") {
            has_deferred_items = true;
            continue;
        }
        if name == "SKELETON.md" || name.ends_with("-SKELETON.md") {
            has_skeleton = true;
            continue;
        }

        // Pass 1 — Match PLAN.md or *-PLAN.md (after PLAN-CHECK/PLAN-REVIEW are
        // filtered above). Derive the plan ID (filename minus the PLAN suffix;
        // standalone PLAN.md → empty-string ID) and record it, plus its
        // `NN-MM` index if it has one, unless the plan's frontmatter marks it
        // `status: superseded`. Both keys are recorded only for SURVIVING plans,
        // so a superseded plan's summary still pairs with nothing.
        if name == "PLAN.md" || name.ends_with("-PLAN.md") {
            // Read ONCE. The superseded check and `estimate.tokens` are two
            // readings of the same bytes, not two reads of the same file.
            let content = std::fs::read_to_string(entry.path()).ok();
            if let Some(content) = content.as_deref() {
                if plan_frontmatter_superseded(content) {
                    continue;
                }
            }
            let id = name
                .strip_suffix("-PLAN.md")
                .map(str::to_string)
                .unwrap_or_default();
            if let Some(index) = plan_index(&id) {
                plan_indices.entry(index).or_default().push(id.clone());
            }
            if let Some(estimate) = content
                .as_deref()
                .and_then(|c| frontmatter_token_count(c, "estimate"))
            {
                plan_estimates.insert(id.clone(), estimate);
            }
            let wave = content.as_deref().and_then(plan_waves::plan_wave_number);
            plan_wave_entries.push((id.clone(), wave));
            plans.push(PlanMeta {
                id: id.clone(),
                title: content
                    .as_deref()
                    .and_then(plan_title)
                    .map(crate::text::Untrusted::from_untrusted_source),
                objective_line: content.as_deref().and_then(objective_line),
                wave,
            });
            plan_ids.insert(id);
            continue;
        }

        // Pass 2 (collection) — Match SUMMARY.md or *-SUMMARY.md, excluding FIX and
        // GAPCLOSURE summaries which are never plan partners (#1988). The ID→plan
        // pairing happens after the loop once all surviving plan IDs are known.
        if name == "SUMMARY.md" || name.ends_with("-SUMMARY.md") {
            if name.contains("-FIX-") || name.ends_with("-GAPCLOSURE-SUMMARY.md") {
                continue;
            }
            summary_names.push(name);
            continue;
        }

        // Match CONTEXT.md or *-CONTEXT.md
        if name == "CONTEXT.md" || name.ends_with("-CONTEXT.md") {
            has_context = true;
        }

        // Match RESEARCH.md or *-RESEARCH.md
        if name == "RESEARCH.md" || name.ends_with("-RESEARCH.md") {
            has_research = true;
        }

        // Match VERIFICATION.md or *-VERIFICATION.md. Collected the way the
        // summary pass above collects, rather than setting a boolean and moving
        // on: presence cannot express a gate and the status can.
        if name == "VERIFICATION.md" || name.ends_with("-VERIFICATION.md") {
            verification_names.push(name);
        }
    }

    let has_verification = !verification_names.is_empty();
    let verification_status = read_verification_status(phase_dir, verification_names);
    let has_uat = !uat_names.is_empty();
    let uat_status = read_uat_status(phase_dir, uat_names);
    let continue_here_blocking = phase_continue_here_blocking(phase_dir);

    // Pass 2 (pairing) — a summary counts only if it names a surviving
    // (non-superseded) plan (matched-summary rule, #1988). Two tiers, tried in
    // order:
    //
    //   1. Identical stems. Standalone `SUMMARY.md` derives the empty-string ID
    //      and pairs with standalone `PLAN.md`; a project whose summaries repeat
    //      the plan's slug pairs here too. This is the original rule, verbatim.
    //   2. Same leading plan index. GSD writes
    //      `13-01-verse-pipeline-PLAN.md` beside a bare `13-01-SUMMARY.md`, so
    //      the stems differ by construction and tier 1 can never fire for a real
    //      GSD phase. The index is the identity the pair actually shares.
    //      Ambiguity abstains: an index naming more than one surviving plan
    //      pairs with nothing, so one summary can never satisfy two plans.
    //
    // The count is of PLANS matched, not of summary files, so two summaries
    // resolving to the same plan cannot push `summary_count` past `plan_count`
    // and fake a completion.
    let plan_count: u32 = plan_ids.len() as u32;
    let mut matched_plans: HashSet<String> = HashSet::new();
    // `actuals.tokens` per plan, attributed through the SAME pairing that drives
    // `summary_count` and through no second, looser rule: a summary that does
    // not count toward completion does not contribute a number either, so an
    // ambiguous index shows no actual rather than a neighbour's.
    let mut plan_actuals: HashMap<String, u64> = HashMap::new();
    for name in &summary_names {
        let id = name.strip_suffix("-SUMMARY.md").unwrap_or("");
        let matched = if let Some(exact) = plan_ids.get(id) {
            Some(exact.clone())
        } else if let Some(index) = plan_index(id) {
            match plan_indices.get(&index).map(Vec::as_slice) {
                Some([only]) => Some(only.clone()),
                _ => None,
            }
        } else {
            None
        };
        let Some(plan_id) = matched else { continue };
        // A genuine new read, bounded by the phase's plan count, inside the
        // once-per-refresh scan that already reads every `*-PLAN.md`. An
        // unreadable summary records nothing and disturbs no count.
        if let Ok(content) = std::fs::read_to_string(phase_dir.join(name)) {
            if let Some(actual) = frontmatter_token_count(&content, "actuals") {
                plan_actuals.insert(plan_id.clone(), actual);
            }
        }
        matched_plans.insert(plan_id);
    }
    let summary_count: u32 = matched_plans.len() as u32;

    // One row per surviving plan for which at least one number was found, in
    // numeric plan-index order. The sort is what keeps `DiskInference`
    // comparable across refreshes — see the field's doc comment.
    let mut plan_tokens: Vec<PlanTokens> = plan_ids
        .iter()
        .filter_map(|id| {
            let estimate = plan_estimates.get(id).copied();
            let actual = plan_actuals.get(id).copied();
            if estimate.is_none() && actual.is_none() {
                return None;
            }
            Some(PlanTokens {
                id: id.clone(),
                estimate,
                actual,
            })
        })
        .collect();
    plan_tokens.sort_by(|a, b| plan_id_order(&a.id, &b.id));

    // The paired plans themselves, in the same numeric order — see the field's
    // doc comment. Collected from the pairing above, never re-derived.
    let mut summarized_plans: Vec<String> = matched_plans.into_iter().collect();
    summarized_plans.sort_by(|a, b| plan_id_order(a, b));

    // Waves, from the `wave:` key each surviving plan's own frontmatter carries.
    // Sorted inside `group_into_waves` — see the field's doc comment.
    let plan_waves = plan_waves::group_into_waves(plan_wave_entries);
    // Same order as every other per-plan vector — see the field's doc comment.
    plans.sort_by(|a, b| plan_id_order(&a.id, &b.id));

    // Determine status following GSD's priority order (`init.cjs:1875-1888`).
    //
    // `implementation_complete` is GSD's predicate verbatim (`init.cjs:181`).
    // Before this plan it was the FIRST arm and yielded `Complete`, which made
    // this reader's `Complete` mean GSD's `executed` — so a phase whose
    // verification was `human_needed` read as finished everywhere in the tree.
    // It now yields `Executed`, and `Complete` requires the conjunct
    // `init.cjs:194-195` requires: implementation complete AND verification
    // passed.
    let implementation_complete = summary_count >= plan_count && plan_count > 0;
    let status = if implementation_complete && verification_status.is_passed() {
        DiskStatus::Complete
    } else if implementation_complete {
        DiskStatus::Executed
    } else if summary_count > 0 {
        DiskStatus::Partial
    } else if plan_count > 0 {
        DiskStatus::Planned
    } else if has_research {
        DiskStatus::Researched
    } else if has_context {
        DiskStatus::Discussed
    } else {
        DiskStatus::Empty
    };

    DiskInference {
        status,
        plan_count,
        summary_count,
        has_plans: plan_count > 0,
        has_summaries: summary_count > 0,
        has_context,
        has_research,
        has_verification,
        verification_status,
        has_security,
        has_uat,
        uat_status,
        continue_here_blocking,
        has_spec,
        has_eval_review,
        has_patterns,
        has_plan_check,
        has_validation,
        has_ui_spec,
        has_ui_check,
        has_ai_spec,
        has_review,
        review_disposition: None,
        has_ui_review,
        has_coverage,
        has_windows,
        has_deferred_items,
        has_skeleton,
        plan_tokens,
        plan_waves,
        summarized_plans,
        plans,
        waves_manifest,
    }
}

/// The one ordering every per-plan vector in [`DiskInference`] uses: numeric
/// plan index first (`13-2` before `13-10`), an id with no plan index after
/// every indexed one, and the id itself as the tie-break so the order is total.
fn plan_id_order(a: &str, b: &str) -> std::cmp::Ordering {
    let key_a = plan_index(a);
    let key_b = plan_index(b);
    (key_a.is_none(), key_a, a).cmp(&(key_b.is_none(), key_b, b))
}

/// Test whether a phase directory name belongs to the given phase number.
///
/// GSD 1.8.0 phase directories are no longer always `NN-slug`: they may be
/// decimal (`0.3-slug`), milestone-prefixed (`M1-2-slug`), project-code-prefixed
/// (`AB-29-slug`), or year-prefixed multi-segment (`14-2026-foo`). Matching must
/// also be pad-insensitive (`3-foo` and `03-foo` both match phase `3`) while
/// still rejecting bare-number-vs-longer-number collisions (`1` must not match
/// `14-foo` or `1.2-foo`).
///
/// A candidate list is built from `phase_number` — always the raw string, plus
/// (for all-digit numbers) the zero-padded-to-2 and leading-zeros-stripped forms.
/// A directory matches a candidate `C` when it equals `C` or starts with `C-`;
/// the trailing `-` is the boundary guard against longer-number collisions.
fn phase_dir_matches(dir_name: &str, phase_number: &str) -> bool {
    let mut candidates: Vec<String> = vec![phase_number.to_string()];
    if !phase_number.is_empty() && phase_number.chars().all(|c| c.is_ascii_digit()) {
        if phase_number.len() < 2 {
            candidates.push(format!("{:0>2}", phase_number));
        }
        let stripped = phase_number.trim_start_matches('0');
        candidates.push(if stripped.is_empty() {
            "0".to_string()
        } else {
            stripped.to_string()
        });
    } else if let Some(num) = super::phase_num::PhaseNum::parse(phase_number) {
        // A decimal (inserted) phase: ROADMAP's checklist writes `7.1`, GSD
        // names the directory `07.1-slug`. Both the canonical spelling and the
        // one with its integer part padded to 2 are candidates, so `7.1` finds
        // `07.1-foo` and `07.1` finds `7.1-foo`. The `-` boundary still keeps
        // `7.1` off `7.10-foo`.
        let canonical = num.to_string();
        let padded = match canonical.split_once('.') {
            Some((major, rest)) => format!("{major:0>2}.{rest}"),
            None => format!("{canonical:0>2}"),
        };
        candidates.push(canonical);
        candidates.push(padded);
    }
    candidates
        .iter()
        .any(|c| dir_name == c || dir_name.starts_with(&format!("{c}-")))
}

/// Find a phase directory by its number, checking both active phases/ and archived milestones/.
///
/// Search order:
/// 1. planning_dir/phases/ for directories starting with zero-padded phase number (e.g., "05-")
/// 2. planning_dir/milestones/*/ for archived phases
pub fn find_phase_dir(planning_dir: &Path, phase_number: &str) -> Option<PathBuf> {
    // Check phases/ directory first
    let phases_dir = planning_dir.join("phases");
    if let Ok(entries) = std::fs::read_dir(&phases_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    if phase_dir_matches(name, phase_number) {
                        return Some(entry.path());
                    }
                }
            }
        }
    }

    // Check milestones/*/ directories for archived phases
    let milestones_dir = planning_dir.join("milestones");
    if let Ok(milestone_entries) = std::fs::read_dir(&milestones_dir) {
        for milestone_entry in milestone_entries.flatten() {
            if milestone_entry
                .file_type()
                .map(|t| t.is_dir())
                .unwrap_or(false)
            {
                if let Ok(phase_entries) = std::fs::read_dir(milestone_entry.path()) {
                    for phase_entry in phase_entries.flatten() {
                        if phase_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            if let Some(name) = phase_entry.file_name().to_str() {
                                if phase_dir_matches(name, phase_number) {
                                    return Some(phase_entry.path());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

/// Infer a phase's status by locating its directory and scanning artifacts.
/// Archived phases (in milestones/) are always Complete per Pitfall 4.
pub fn infer_phase_status(planning_dir: &Path, phase_number: &str) -> DiskInference {
    // Check if phase is in milestones/ (archived = Complete)
    let milestones_dir = planning_dir.join("milestones");
    if let Ok(milestone_entries) = std::fs::read_dir(&milestones_dir) {
        for milestone_entry in milestone_entries.flatten() {
            if milestone_entry
                .file_type()
                .map(|t| t.is_dir())
                .unwrap_or(false)
            {
                if let Ok(phase_entries) = std::fs::read_dir(milestone_entry.path()) {
                    for phase_entry in phase_entries.flatten() {
                        if phase_entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                            if let Some(name) = phase_entry.file_name().to_str() {
                                if phase_dir_matches(name, phase_number) {
                                    // Archived phase -- always Complete.
                                    //
                                    // Deliberately NOT re-derived through the
                                    // verification conjunct: a phase archived
                                    // into a milestone shipped, and the
                                    // milestone archive is the corroboration.
                                    // Its `verification_status` stays `Missing`
                                    // because nothing was read, which is the
                                    // honest value — not a claim it passed.
                                    return DiskInference {
                                        status: DiskStatus::Complete,
                                        ..Default::default()
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Check phases/ directory
    match find_phase_dir(planning_dir, phase_number) {
        Some(dir) => infer_disk_status(&dir),
        None => DiskInference {
            status: DiskStatus::NoDirectory,
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_empty_directory_returns_empty() {
        let dir = tempdir().unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Empty);
        assert_eq!(result.plan_count, 0);
        assert_eq!(result.summary_count, 0);
    }

    #[test]
    fn test_context_only_returns_discussed() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-CONTEXT.md"), "context").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Discussed);
        assert!(result.has_context);
    }

    #[test]
    fn test_context_and_research_returns_researched() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-CONTEXT.md"), "context").unwrap();
        fs::write(dir.path().join("05-RESEARCH.md"), "research").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Researched);
        assert!(result.has_context);
        assert!(result.has_research);
    }

    #[test]
    fn test_plans_only_returns_planned() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-02-PLAN.md"), "plan2").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Planned);
        assert_eq!(result.plan_count, 2);
        assert_eq!(result.summary_count, 0);
        assert!(result.has_plans);
        assert!(!result.has_summaries);
    }

    #[test]
    fn test_partial_summaries_returns_partial() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-02-PLAN.md"), "plan2").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Partial);
        assert_eq!(result.plan_count, 2);
        assert_eq!(result.summary_count, 1);
    }

    #[test]
    fn test_all_summaries_returns_executed_not_complete() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-02-PLAN.md"), "plan2").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        fs::write(dir.path().join("05-02-SUMMARY.md"), "summary2").unwrap();
        let result = infer_disk_status(dir.path());
        // Every plan has a summary and there is no verification artifact: GSD
        // calls that `executed`, not `complete` (init.cjs:181 vs :194-195).
        assert_eq!(result.status, DiskStatus::Executed);
        assert_eq!(result.plan_count, 2);
        assert_eq!(result.summary_count, 2);
        assert!(result.has_plans);
        assert!(result.has_summaries);
    }

    #[test]
    fn test_standalone_plan_counted() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("PLAN.md"), "plan").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Planned);
        assert_eq!(result.plan_count, 1);
    }

    #[test]
    fn test_nonexistent_directory_returns_no_directory() {
        let result = infer_disk_status(Path::new("/nonexistent/path/does/not/exist"));
        assert_eq!(result.status, DiskStatus::NoDirectory);
    }

    #[test]
    fn test_has_plans_and_has_summaries_booleans() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-02-PLAN.md"), "plan2").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(
            result.has_plans,
            "has_plans should be true when plans exist"
        );
        assert!(
            result.has_summaries,
            "has_summaries should be true when summaries exist"
        );
        assert_eq!(result.plan_count, 2);
        assert_eq!(result.summary_count, 1);
    }

    #[test]
    fn test_empty_dir_has_no_plans_or_summaries() {
        let dir = tempdir().unwrap();
        let result = infer_disk_status(dir.path());
        assert!(!result.has_plans);
        assert!(!result.has_summaries);
    }

    /// The hazard this assertion message names, stated once and reused.
    const APPEND_HAZARD: &str = "DiskStatus derives Ord from DECLARATION ORDER. A variant \
         APPENDED rather than INSERTED at its semantic position still compiles, still passes \
         every equality test, and silently reorders every `<`/`>=` comparison in the tree — \
         including the dashboard pipeline's stage thresholds. Insert; never append.";

    #[test]
    fn test_disk_status_ordering() {
        assert!(DiskStatus::NoDirectory < DiskStatus::Empty, "{APPEND_HAZARD}");
        assert!(DiskStatus::Empty < DiskStatus::Discussed, "{APPEND_HAZARD}");
        assert!(DiskStatus::Discussed < DiskStatus::Researched, "{APPEND_HAZARD}");
        assert!(DiskStatus::Researched < DiskStatus::Planned, "{APPEND_HAZARD}");
        assert!(DiskStatus::Planned < DiskStatus::Partial, "{APPEND_HAZARD}");
        // The two that pin `Executed`'s inserted position. An appended
        // `Executed` would sort ABOVE `Complete` and fail the second.
        assert!(DiskStatus::Partial < DiskStatus::Executed, "{APPEND_HAZARD}");
        assert!(DiskStatus::Executed < DiskStatus::Complete, "{APPEND_HAZARD}");
    }

    // ── Plan 20-03 Task 1: GSD's vocabulary + the verification frontmatter ──

    /// A phase directory with `n` plans and `n` matching summaries — GSD's
    /// `implementation_complete` predicate satisfied, and nothing more.
    fn implementation_complete_dir() -> tempfile::TempDir {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("19-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("19-01-SUMMARY.md"), "summary1").unwrap();
        dir
    }

    #[test]
    fn test_implementation_complete_without_verification_is_executed() {
        let dir = implementation_complete_dir();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.status,
            DiskStatus::Executed,
            "every plan has a summary and no verification artifact exists — that is \
             GSD's `executed`, not its `complete`. Reading it as Complete is the \
             vocabulary collapse DRIVE-05 exists to prevent"
        );
        assert_ne!(result.status, DiskStatus::Complete);
        assert_eq!(result.verification_status, VerificationStatus::Missing);
        assert!(!result.has_verification);
    }

    #[test]
    fn test_passing_verification_makes_it_complete() {
        let dir = implementation_complete_dir();
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "---\nphase: 19\nstatus: passed\n---\nbody\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.status, DiskStatus::Complete);
        assert_eq!(result.verification_status, VerificationStatus::Passed);
        assert!(result.has_verification);
    }

    #[test]
    fn test_gating_verification_statuses_stay_executed_and_carry_the_value() {
        for (raw, expected) in [
            ("human_needed", VerificationStatus::HumanNeeded),
            ("gaps_found", VerificationStatus::GapsFound),
            ("stale", VerificationStatus::Stale),
        ] {
            let dir = implementation_complete_dir();
            fs::write(
                dir.path().join("19-VERIFICATION.md"),
                format!("---\nstatus: {raw}\n---\nbody\n"),
            )
            .unwrap();
            let result = infer_disk_status(dir.path());
            assert_eq!(
                result.status,
                DiskStatus::Executed,
                "a `{raw}` verification is a gate; a phase carrying one must never \
                 read as Complete anywhere in the tree"
            );
            assert_eq!(result.verification_status, expected);
            // Presence and status answer different questions, and both are kept.
            assert!(result.has_verification);
        }
    }

    #[test]
    fn test_unrecognised_verification_status_is_carried_verbatim() {
        let dir = implementation_complete_dir();
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "---\nstatus: reticulating_splines\n---\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.verification_status,
            VerificationStatus::Unknown("reticulating_splines".to_string()),
            "an unrecognised status must be carried verbatim, never mapped onto a \
             known arm and never dropped: it is not a passing status, and the bytes \
             are what tell a reader which unknown it was"
        );
        assert_eq!(result.status, DiskStatus::Executed);
    }

    #[test]
    fn test_status_key_below_the_leading_block_is_never_matched() {
        let dir = implementation_complete_dir();
        // No leading frontmatter at all, and a fenced code block further down
        // that *documents* a passing status. This is GSD's own
        // DEFECT.FRONTMATTER-SCALAR-BROAD-GREP, reproduced as a fixture.
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "# Verification\n\nThe verifier writes:\n\n```yaml\nstatus: passed\n```\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.verification_status,
            VerificationStatus::Missing,
            "the parse is anchored at byte zero and reads only the leading delimited \
             block. A broad search would read the fenced example as a real status and \
             mark an unverified phase Complete"
        );
        assert_eq!(result.status, DiskStatus::Executed);
    }

    #[test]
    fn test_closed_frontmatter_block_does_not_leak_into_the_body() {
        let dir = implementation_complete_dir();
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "---\nphase: 19\n---\n\n```yaml\nstatus: passed\n```\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.verification_status,
            VerificationStatus::Missing,
            "the scan stops at the closing delimiter; a key below it is body text"
        );
    }

    #[test]
    fn test_a_nested_status_key_is_not_the_documents_status() {
        let dir = implementation_complete_dir();
        // A nested mapping key that sorts BEFORE the real one, which is the
        // whole shape: the first match used to win and the trimmed comparison
        // could not tell the two apart (WR-05). The same defect as the fenced
        // code block above, one level in.
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "---\nphase: 19\nverification:\n  status: passed\nstatus: human_needed\n---\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.verification_status,
            VerificationStatus::HumanNeeded,
            "the document's status is the key at column zero. Reading the nested \
             one produces a false `passed`, which is a false Decision::GoalMet \
             and a run routed straight past the human_needed gate DRIVE-05 exists \
             to park at"
        );
        assert_eq!(
            result.status,
            DiskStatus::Executed,
            "and Complete stays out of reach, since it requires a passing \
             verification"
        );
    }

    #[test]
    fn test_a_top_level_status_after_a_nested_one_is_still_found() {
        // The non-vacuity half: skipping indented lines must not turn into
        // skipping the block. A `status` that follows nested keys is still the
        // document's.
        let dir = implementation_complete_dir();
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "---\nprogress:\n  status: draft\n  items: 3\nstatus: passed\n---\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.verification_status, VerificationStatus::Passed);
        assert_eq!(result.status, DiskStatus::Complete);
    }

    #[test]
    fn test_two_verification_artifacts_always_yield_the_sorted_first() {
        let dir = implementation_complete_dir();
        fs::write(
            dir.path().join("19-01-VERIFICATION.md"),
            "---\nstatus: human_needed\n---\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("19-02-VERIFICATION.md"),
            "---\nstatus: passed\n---\n",
        )
        .unwrap();
        // Repeated reads, because the defect this guards is directory-iteration
        // order — which is not stable and need not differ on any single read.
        for read in 0..8 {
            let result = infer_disk_status(dir.path());
            assert_eq!(
                result.verification_status,
                VerificationStatus::HumanNeeded,
                "read {read}: names are sorted and the first is read, the same \
                 tie-break GSD's own reader uses. Taking whichever the filesystem \
                 offered first would make the same directory report two different \
                 statuses on two different days"
            );
            assert_eq!(result.status, DiskStatus::Executed, "read {read}");
        }
    }

    #[test]
    fn test_unreadable_verification_artifact_is_fail_safe() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("19-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("19-01-SUMMARY.md"), "summary1").unwrap();
        // A *directory* named like the artifact: `read_to_string` fails.
        fs::create_dir(dir.path().join("19-VERIFICATION.md")).unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.verification_status,
            VerificationStatus::Missing,
            "an unreadable artifact yields the absent value, never an error and \
             never a panic — the reader runs against other people's repositories"
        );
        assert_eq!(result.status, DiskStatus::Executed);
    }

    #[test]
    fn test_verification_on_an_incomplete_phase_does_not_complete_it() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("19-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("19-02-PLAN.md"), "plan2").unwrap();
        fs::write(dir.path().join("19-01-SUMMARY.md"), "summary1").unwrap();
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "---\nstatus: passed\n---\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.status,
            DiskStatus::Partial,
            "Complete is a CONJUNCTION: a passing verification over an unfinished \
             implementation is still unfinished"
        );
        assert_eq!(result.verification_status, VerificationStatus::Passed);
    }

    // ── quick 260926-gtn: gsd-core 1.15.0 `unparseable` (#4806) ──
    //
    // Shapes and expectations are the probe table in the gtn PLAN: each was run
    // through 1.14.0 and 1.15.0 `init.manager` and `serde_yml` 0.0.13.

    /// Closed blocks that are not YAML, each carrying a surviving
    /// `status: passed` line a line scan would believe.
    const UNPARSEABLE_BLOCKS: &[(&str, &str)] = &[
        ("unclosed flow", "---\nstatus: passed\nscore: [unclosed\n---\nbody\n"),
        ("bad indent", "---\nstatus: passed\n  bad: indent\n---\nbody\n"),
        ("tab indent", "---\nstatus: passed\n\tbad: tab\n---\nbody\n"),
        (
            "re_verification + indented children (v1.1-phases/06)",
            "---\nphase: 06\nstatus: passed\nre_verification: true\n  previous_status: gaps_found\n  previous_score: 3/5\n---\n# V\n",
        ),
    ];

    #[test]
    fn test_broken_yaml_verification_is_unparseable_and_never_complete() {
        for (label, content) in UNPARSEABLE_BLOCKS {
            assert_eq!(
                verification_status_of(content),
                VerificationStatus::Unparseable,
                "{label}: a closed block that is not YAML is upstream 1.15.0's \
                 `unparseable`; the surviving `status: passed` line must not be read"
            );
            let dir = implementation_complete_dir();
            fs::write(dir.path().join("19-VERIFICATION.md"), content).unwrap();
            let result = infer_disk_status(dir.path());
            assert_eq!(result.verification_status, VerificationStatus::Unparseable, "{label}");
            assert_eq!(
                result.status,
                DiskStatus::Executed,
                "{label}: both 1.14.0 and 1.15.0 read this `executed`, never `complete`"
            );
            assert!(result.has_verification, "{label}");
        }
    }

    #[test]
    fn test_crlf_broken_block_is_still_unparseable() {
        assert_eq!(
            verification_status_of("---\r\nstatus: passed\r\n  bad: indent\r\n---\r\n"),
            VerificationStatus::Unparseable
        );
        assert_eq!(
            verification_status_of("---\r\nstatus: passed\r\n---\r\n"),
            VerificationStatus::Passed
        );
    }

    #[test]
    fn test_unterminated_verification_block_is_missing_not_passed() {
        let dir = implementation_complete_dir();
        fs::write(
            dir.path().join("19-VERIFICATION.md"),
            "---\nphase: 19\nstatus: passed\n\n# Verification\n",
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.verification_status,
            VerificationStatus::Missing,
            "upstream's extractFrontmatter returns {{}} for an unterminated block, \
             i.e. `missing`; the `status:` line is not frontmatter"
        );
        assert!(result.has_verification);
        assert_eq!(result.status, DiskStatus::Executed);
    }

    #[test]
    fn test_literal_unparseable_status_is_unparseable() {
        assert_eq!(
            verification_status_of("---\nstatus: unparseable\n---\n"),
            VerificationStatus::Unparseable
        );
        assert_eq!(VerificationStatus::from_raw("UNPARSEABLE"), VerificationStatus::Unparseable);
        assert_eq!(VerificationStatus::Unparseable.as_str(), "unparseable");
        assert!(!VerificationStatus::Unparseable.is_passed());
    }

    #[test]
    fn test_repairable_and_duplicate_key_blocks_stay_passed() {
        // Probe table rows 7-8: both oracles read these `complete`/`passed`.
        // `5/5: all verified` fails serde_yml's raw parse, so this pins that the
        // one-shot ambiguous-colon repair runs (upstream
        // loadWithAmbiguousColonRepair); serde_yml accepts duplicate keys.
        for content in [
            "---\nstatus: passed\nscore: 5/5: all verified\n---\n",
            "---\nstatus: passed\nstatus: passed\n---\n",
        ] {
            assert_eq!(verification_status_of(content), VerificationStatus::Passed, "{content}");
            let dir = implementation_complete_dir();
            fs::write(dir.path().join("19-VERIFICATION.md"), content).unwrap();
            assert_eq!(infer_disk_status(dir.path()).status, DiskStatus::Complete, "{content}");
        }
    }

    #[test]
    fn test_closed_empty_block_is_missing_not_unparseable() {
        assert_eq!(verification_status_of("---\n---\nbody\n"), VerificationStatus::Missing);
        assert_eq!(
            verification_status_of("---\nphase: 19\n---\n\nstatus: passed\n"),
            VerificationStatus::Missing,
            "a body `status:` below a correctly closed block is still body text"
        );
    }

    #[test]
    fn test_leading_block_classification() {
        assert_eq!(leading_block("# no block\n"), LeadingBlock::Absent);
        assert_eq!(leading_block("---\na: 1\n"), LeadingBlock::Unterminated);
        assert_eq!(
            leading_block("---\na: 1\nb: 2\n---\nbody\n---\n"),
            LeadingBlock::Closed("a: 1\nb: 2".to_string())
        );
        assert_eq!(leading_block(""), LeadingBlock::Absent);
    }

    // ── Plan 20-03 Task 2: the rest of the disk-observable gate set ──

    #[test]
    fn test_uat_blocking_statuses_set_the_outstanding_gate() {
        // uat-predicate.cjs:30-32's set, taken as written.
        for raw in [
            "partial",
            "diagnosed",
            "pending",
            "blocked",
            "in_progress",
            "failed",
        ] {
            let dir = tempdir().unwrap();
            fs::write(
                dir.path().join("19-UAT.md"),
                format!("---\nstatus: {raw}\n---\n"),
            )
            .unwrap();
            let result = infer_disk_status(dir.path());
            assert!(
                result.uat_status.is_outstanding(),
                "`{raw}` is in GSD's blocking UAT set and must set the gate"
            );
            assert_eq!(result.uat_status.as_str(), raw);
            assert!(result.has_uat, "presence is still recorded alongside status");
        }
    }

    #[test]
    fn test_uat_statuses_outside_the_blocking_set_do_not_gate() {
        // The negative arm. `deferred` is this repository's own phase-19 value:
        // a human's explicit decision to proceed, not an unanswered question.
        for raw in ["passed", "deferred", "complete"] {
            let dir = tempdir().unwrap();
            fs::write(
                dir.path().join("19-UAT.md"),
                format!("---\nstatus: {raw}\n---\n"),
            )
            .unwrap();
            let result = infer_disk_status(dir.path());
            assert!(
                !result.uat_status.is_outstanding(),
                "`{raw}` is outside GSD's blocking set; a gate that fires on \
                 everything is as useless as one that fires on nothing"
            );
            assert_eq!(result.uat_status, UatStatus::Other(raw.to_string()));
        }
    }

    #[test]
    fn test_absent_uat_artifact_yields_the_missing_status() {
        let dir = tempdir().unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.uat_status, UatStatus::Missing);
        assert!(!result.uat_status.is_outstanding());
        assert!(!result.has_uat);
    }

    #[test]
    fn test_continue_here_with_a_blocking_row_sets_the_phase_gate() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".continue-here.md"),
            "## Anti-Patterns\n\n| Pattern | Severity |\n|---|---|\n| Something | advisory |\n| \
             Something else | blocking |\n",
        )
        .unwrap();
        assert!(infer_disk_status(dir.path()).continue_here_blocking);
    }

    #[test]
    fn test_continue_here_key_line_form_sets_the_phase_gate() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join(".continue-here.md"),
            "---\ncontext: phase\n---\n\n- item: do the thing\n- severity: blocking\n",
        )
        .unwrap();
        assert!(infer_disk_status(dir.path()).continue_here_blocking);
    }

    #[test]
    fn test_continue_here_with_no_blocking_row_does_not_set_the_phase_gate() {
        let dir = tempdir().unwrap();
        // Rows exist; none is blocking. Plus the word "blocking" in prose and in
        // a filename — exactly the shape of this repository's stale marker.
        fs::write(
            dir.path().join(".continue-here.md"),
            "## Anti-Patterns\n\n| Pattern | Severity |\n|---|---|\n| Plan 19-08: \
             `tests/async_blocking_guard.rs` | advisory |\n| Another blocking-sounding note | \
             advisory |\n",
        )
        .unwrap();
        assert!(
            !infer_disk_status(dir.path()).continue_here_blocking,
            "the gate is a blocking-severity ROW, never the file's existence and \
             never the word appearing in prose. A substring match parks every run \
             against a project carrying a stale advisory marker forever"
        );
    }

    #[test]
    fn test_absent_or_unreadable_continue_here_does_not_set_the_phase_gate() {
        let empty = tempdir().unwrap();
        assert!(!infer_disk_status(empty.path()).continue_here_blocking);

        // A directory where the marker should be: read_to_string fails.
        let unreadable = tempdir().unwrap();
        fs::create_dir(unreadable.path().join(".continue-here.md")).unwrap();
        assert!(
            !infer_disk_status(unreadable.path()).continue_here_blocking,
            "an unreadable marker is 'no gate observed', never an error and never \
             a panic"
        );
    }

    #[test]
    fn test_blocking_row_in_a_table_without_a_severity_column_is_not_a_gate() {
        let dir = tempdir().unwrap();
        // "blocking" sits in a Description column of a table that has no
        // Severity header at all. Comparing by column index is what stops it.
        fs::write(
            dir.path().join(".continue-here.md"),
            "| Item | Description |\n|---|---|\n| One | blocking |\n",
        )
        .unwrap();
        assert!(!infer_disk_status(dir.path()).continue_here_blocking);
    }

    #[test]
    fn test_phase_19_stale_continue_here_marker_is_not_blocking() {
        // The concrete regression the stale-marker note describes, read from
        // this repository's own planning directory rather than a fixture.
        let planning = Path::new(env!("CARGO_MANIFEST_DIR")).join(".planning");
        let Some(phase_19) = find_phase_dir(&planning, "19") else {
            // Archived away by a future milestone: the fixtures above still
            // carry the property, so there is nothing left to regress here.
            return;
        };
        if !phase_19.join(".continue-here.md").exists() {
            return;
        }
        assert!(
            !infer_disk_status(&phase_19).continue_here_blocking,
            "phase 19's marker is left over from a completed phase and every one \
             of its severity rows reads `advisory`. If this fires, the gate has \
             been simplified back into an existence check or a substring search, \
             and every run against this project parks forever"
        );
    }

    #[test]
    fn test_verification_status_round_trips_through_its_identifier() {
        for status in [
            VerificationStatus::Missing,
            VerificationStatus::Passed,
            VerificationStatus::GapsFound,
            VerificationStatus::HumanNeeded,
            VerificationStatus::Stale,
        ] {
            assert_eq!(
                VerificationStatus::from_raw(status.as_str()),
                status,
                "as_str and from_raw must name the same value, or a park record \
                 and the state that produced it disagree"
            );
        }
        assert!(VerificationStatus::Passed.is_passed());
        assert!(!VerificationStatus::HumanNeeded.is_passed());
        assert!(!VerificationStatus::Unknown("passed_ish".to_string()).is_passed());
    }

    #[test]
    fn test_archived_phase_returns_complete() {
        let dir = tempdir().unwrap();
        // Create milestone archive structure: milestones/v1.0/05-something/
        let milestone_dir = dir
            .path()
            .join("milestones")
            .join("v1.0")
            .join("05-state-reader");
        fs::create_dir_all(&milestone_dir).unwrap();
        // Put some content in the archived phase
        fs::write(milestone_dir.join("05-01-PLAN.md"), "plan").unwrap();

        // Also create phases/ directory (empty, no phase 05 there)
        fs::create_dir_all(dir.path().join("phases")).unwrap();

        let result = infer_phase_status(dir.path(), "05");
        assert_eq!(result.status, DiskStatus::Complete);
    }

    #[test]
    fn test_find_phase_dir_in_phases() {
        let dir = tempdir().unwrap();
        let phase_dir = dir.path().join("phases").join("05-state-reader");
        fs::create_dir_all(&phase_dir).unwrap();

        let found = find_phase_dir(dir.path(), "05");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), phase_dir);
    }

    #[test]
    fn test_security_md_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-SECURITY.md"), "security").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_security, "has_security should be true for *-SECURITY.md");
        // SECURITY.md is informational — must not affect plan/summary/status counts.
        assert_eq!(result.plan_count, 0);
        assert_eq!(result.summary_count, 0);
        assert_eq!(result.status, DiskStatus::Empty);
    }

    #[test]
    fn test_standalone_security_md_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("SECURITY.md"), "security").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_security, "has_security should be true for bare SECURITY.md");
    }

    #[test]
    fn test_empty_dir_has_no_security() {
        let dir = tempdir().unwrap();
        let result = infer_disk_status(dir.path());
        assert!(!result.has_security);
    }

    #[test]
    fn test_uat_md_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-UAT.md"), "uat").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_uat, "has_uat should be true for *-UAT.md");
        assert_eq!(result.plan_count, 0);
        assert_eq!(result.summary_count, 0);
        assert_eq!(result.status, DiskStatus::Empty);
    }

    #[test]
    fn test_standalone_uat_md_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("UAT.md"), "uat").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_uat, "has_uat should be true for bare UAT.md");
    }

    #[test]
    fn test_empty_dir_has_no_uat() {
        let dir = tempdir().unwrap();
        let result = infer_disk_status(dir.path());
        assert!(!result.has_uat);
    }

    #[test]
    fn test_spec_md_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-SPEC.md"), "spec").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_spec);
        assert!(!result.has_ui_spec);
        assert!(!result.has_ai_spec);
        assert_eq!(result.status, DiskStatus::Empty);
    }

    #[test]
    fn test_ai_spec_does_not_trigger_spec() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-AI-SPEC.md"), "ai-spec").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_ai_spec);
        assert!(!result.has_spec, "AI-SPEC.md must not be misclassified as SPEC.md");
    }

    #[test]
    fn test_eval_review_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-EVAL-REVIEW.md"), "eval").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_eval_review);
        assert!(!result.has_review, "EVAL-REVIEW.md must not be misclassified as REVIEW.md");
    }

    // ── quick 260926-gtn: the 1.15.0 REVIEW-DISPOSITION ledger ──

    /// The shape `code-review-disposition.md:974-999` renders. The frontmatter
    /// `open: 2` is deliberately stale: the table is the human-edit surface.
    const LEDGER_SAMPLE: &str = "---\nphase: 05\nreview: 05-REVIEW.md\ntitles: json\nfindings:\n  - id: CR-01\n    severity: critical\n    disposition: open\n    title: \"Parser: loses data\"\nopen: 2\ntotal: 4\nrecorded: 2026-09-26T10:00:00.000Z\n---\n\n# Phase 05: Code Review Disposition\n\n| Finding | Severity | Disposition | Source |\n|---------|----------|-------------|--------|\n| CR-01 | critical | open | - |\n| WR-02 | warning | fixed | 05-REVIEW-FIX.md |\n| WR-03 | warning | deferred | waiting on team A \\| team B |\n| IN-01 | info | skipped | 05-REVIEW-FIX.md (not in the current review) |\n\nDispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.\n";

    fn disposition(open: u32, fixed: u32, skipped: u32, deferred: u32) -> ReviewDisposition {
        ReviewDisposition {
            open,
            fixed,
            skipped,
            deferred,
        }
    }

    #[test]
    fn test_review_disposition_ledger_alone_is_not_a_review() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-REVIEW-DISPOSITION.md"), LEDGER_SAMPLE).unwrap();
        let result = infer_disk_status(dir.path());
        assert!(
            !result.has_review,
            "the ledger is a sibling of REVIEW.md, never the review itself"
        );
        assert_eq!(result.review_disposition, Some(disposition(1, 1, 1, 1)));
        assert_eq!(result.status, DiskStatus::Empty);
        assert_eq!((result.plan_count, result.summary_count), (0, 0));
        for (flag, set) in [
            ("has_plans", result.has_plans),
            ("has_summaries", result.has_summaries),
            ("has_context", result.has_context),
            ("has_research", result.has_research),
            ("has_verification", result.has_verification),
            ("has_eval_review", result.has_eval_review),
            ("has_ui_review", result.has_ui_review),
            ("has_uat", result.has_uat),
            ("has_security", result.has_security),
        ] {
            assert!(!set, "the ledger must set no other flag: {flag}");
        }
    }

    #[test]
    fn test_review_disposition_counts_come_from_the_table() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-REVIEW.md"), "---\nphase: 05\n---\n").unwrap();
        fs::write(dir.path().join("05-REVIEW-DISPOSITION.md"), LEDGER_SAMPLE).unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_review);
        let counts = result.review_disposition.expect("a ledger with rows");
        assert_eq!(
            counts,
            disposition(1, 1, 1, 1),
            "the TABLE wins over the frontmatter `open: 2`"
        );
        assert_eq!(counts.total(), 4);
    }

    #[test]
    fn test_review_disposition_row_grammar() {
        let table = |rows: &str| {
            format!(
                "# Ledger\n\n| Finding | Severity | Disposition | Source |\n|---|---|---|---|\n{rows}"
            )
        };
        // Outside the case-sensitive enum falls back to `open` (upstream :390).
        assert_eq!(
            parse_review_disposition(&table("| WR-07 | warning | Deferred | x |\n")),
            Some(disposition(1, 0, 0, 0))
        );
        // A duplicate id counts once; the first occurrence wins.
        assert_eq!(
            parse_review_disposition(&table(
                "| CR-01 | critical | fixed | a |\n| CR-01 | critical | open | b |\n"
            )),
            Some(disposition(0, 1, 0, 0))
        );
        // BL ids count; prose, non-finding rows and fenced rows do not.
        assert_eq!(
            parse_review_disposition(&table(
                "| BL-01 | blocker | deferred | later |\n| Note | - | open | - |\n\nSee | CR-09 | x | open |\n```\n| CR-02 | critical | open | - |\n```\n~~~\n| CR-03 | critical | open | - |\n~~~\n"
            )),
            Some(disposition(0, 0, 0, 1))
        );
        assert_eq!(parse_review_disposition(&table("")), None, "zero rows is None");
        assert_eq!(parse_review_disposition(""), None);
    }

    #[test]
    fn test_review_disposition_absent_is_none() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-REVIEW.md"), "review").unwrap();
        assert_eq!(infer_disk_status(dir.path()).review_disposition, None);
        fs::write(dir.path().join("05-REVIEW-DISPOSITION.md"), "# no table\n").unwrap();
        assert_eq!(infer_disk_status(dir.path()).review_disposition, None);
    }

    #[cfg(unix)]
    #[test]
    fn test_unreadable_review_disposition_is_none() {
        let dir = tempdir().unwrap();
        // A dangling symlink: collected by name, and the open fails.
        std::os::unix::fs::symlink(
            dir.path().join("missing-target"),
            dir.path().join("05-REVIEW-DISPOSITION.md"),
        )
        .unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.review_disposition, None);
        assert!(!result.has_review);
    }

    #[test]
    fn test_review_disposition_read_is_capped() {
        let dir = tempdir().unwrap();
        let mut content = String::from("| CR-01 | critical | open | - |\n");
        // Padding past the cap, then a row that must never be read.
        content.push_str(&"x".repeat(REVIEW_DISPOSITION_READ_CAP as usize + 1024));
        content.push_str("\n| WR-01 | warning | fixed | - |\n");
        fs::write(dir.path().join("05-REVIEW-DISPOSITION.md"), content).unwrap();
        assert_eq!(
            infer_disk_status(dir.path()).review_disposition,
            Some(disposition(1, 0, 0, 0)),
            "only the capped prefix is read"
        );
    }

    #[test]
    fn test_review_disposition_sorted_first_ledger_is_read() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("05-REVIEW-DISPOSITION.md"),
            "| CR-01 | critical | deferred | - |\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("REVIEW-DISPOSITION.md"),
            "| CR-01 | critical | fixed | - |\n",
        )
        .unwrap();
        for _ in 0..4 {
            assert_eq!(
                infer_disk_status(dir.path()).review_disposition,
                Some(disposition(0, 0, 0, 1))
            );
        }
    }

    #[test]
    fn test_empty_dir_has_no_spec_or_eval_review() {
        let dir = tempdir().unwrap();
        let result = infer_disk_status(dir.path());
        assert!(!result.has_spec);
        assert!(!result.has_eval_review);
    }

    // ── Task 1: GSD 1.8.0 counting — exclusions, superseded, matched-summary ──

    #[test]
    fn test_fix_summary_not_counted() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        // A FIX summary must not inflate the count or flip status.
        fs::write(dir.path().join("05-01-FIX-01-SUMMARY.md"), "fix").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 1);
        assert_eq!(
            result.summary_count, 1,
            "FIX summary must not be counted in summary_count"
        );
        assert_eq!(result.status, DiskStatus::Executed);
    }

    #[test]
    fn test_gapclosure_summary_not_counted() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-02-PLAN.md"), "plan2").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        // GAPCLOSURE summary must not flip a still-incomplete phase to Complete.
        fs::write(dir.path().join("05-GAPCLOSURE-SUMMARY.md"), "gap").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 2);
        assert_eq!(
            result.summary_count, 1,
            "GAPCLOSURE summary must not be counted"
        );
        assert_eq!(result.status, DiskStatus::Partial);
    }

    #[test]
    fn test_plan_review_not_counted_as_plan() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-01-PLAN-REVIEW.md"), "review").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.plan_count, 1,
            "PLAN-REVIEW.md must not be counted as a plan"
        );
        assert_eq!(result.status, DiskStatus::Planned);
    }

    #[test]
    fn test_superseded_plan_excluded_from_counts() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        // A superseded plan is dropped from the denominator, and its summary
        // (if any) is not counted either.
        fs::write(
            dir.path().join("05-02-PLAN.md"),
            "---\nstatus: superseded\n---\nbody",
        )
        .unwrap();
        fs::write(dir.path().join("05-02-SUMMARY.md"), "summary2").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.plan_count, 1,
            "superseded plan must be excluded from plan_count"
        );
        assert_eq!(
            result.summary_count, 1,
            "summary of a superseded plan must not be counted"
        );
        assert_eq!(result.status, DiskStatus::Executed);
    }

    #[test]
    fn test_unmatched_summary_not_counted() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        // A summary with no matching plan ID must not be counted.
        fs::write(dir.path().join("05-99-SUMMARY.md"), "orphan").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 1);
        assert_eq!(
            result.summary_count, 0,
            "summary without a matching plan must not be counted"
        );
        assert_eq!(result.status, DiskStatus::Planned);
    }

    // ── Real GSD filename shapes ─────────────────────────────────────────
    //
    // Every fixture above writes a SLUGLESS plan (`05-01-PLAN.md`), and so does
    // this repository's own `.planning/phases/`. GSD does not: it gives plans a
    // descriptive slug and summaries none, so the pair's stems never match and
    // no test or dogfood above can reproduce what a real project does. The
    // fixtures below are copied from `wordoclock/.planning/phases/13-…` and
    // `…/16-…` verbatim, filenames included.

    /// wordoclock phase 13 on disk, byte for byte: four slugged plans, four
    /// bare `NN-MM-SUMMARY.md` partners, and a passing verification.
    ///
    /// Before the index pairing this read `plan_count 4, summary_count 0,
    /// Planned` — the phase renders `o` with `[Planned (4 plans)]` while its
    /// four summaries sit right there in the directory.
    #[test]
    fn slugged_plans_pair_with_the_slugless_summaries_gsd_writes() {
        let dir = tempdir().unwrap();
        for (plan, summary) in [
            (
                "13-01-verse-pipeline-fourteen-translations-PLAN.md",
                "13-01-SUMMARY.md",
            ),
            (
                "13-02-translation-registry-native-twins-PLAN.md",
                "13-02-SUMMARY.md",
            ),
            (
                "13-03-book-name-registry-vendoring-PLAN.md",
                "13-03-SUMMARY.md",
            ),
            (
                "13-04-registry-derived-call-sites-PLAN.md",
                "13-04-SUMMARY.md",
            ),
        ] {
            fs::write(dir.path().join(plan), "plan").unwrap();
            fs::write(dir.path().join(summary), "summary").unwrap();
        }
        fs::write(dir.path().join("13-CONTEXT.md"), "ctx").unwrap();
        fs::write(dir.path().join("13-RESEARCH.md"), "res").unwrap();
        fs::write(
            dir.path().join("13-VERIFICATION.md"),
            "---\nstatus: passed\n---\nbody",
        )
        .unwrap();

        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 4);
        assert_eq!(
            result.summary_count, 4,
            "a slugless summary must pair with the slugged plan sharing its index"
        );
        assert_eq!(result.status, DiskStatus::Complete);
    }

    /// An inserted (decimal) phase pairs the same way: `07.1-01-slug-PLAN.md`
    /// with `07.1-01-SUMMARY.md`, and pad-insensitively (`7.1-2`/`07.1-02`).
    /// A `u32` phase parse rejected `07.1`, so these fell to the exact-stem
    /// rule and a fully executed inserted phase read `Planned`.
    #[test]
    fn slugged_decimal_phase_plans_pair_with_their_summaries() {
        let dir = tempdir().unwrap();
        for (plan, summary) in [
            ("07.1-01-apply-owner-rulings-PLAN.md", "07.1-01-SUMMARY.md"),
            ("07.1-02-booking-har-PLAN.md", "7.1-2-SUMMARY.md"),
        ] {
            fs::write(dir.path().join(plan), "---\nestimate:\n  tokens: 5\n---\n").unwrap();
            fs::write(dir.path().join(summary), "summary").unwrap();
        }
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 2);
        assert_eq!(result.summary_count, 2);
        let labels: Vec<String> = result.plan_tokens.iter().map(PlanTokens::label).collect();
        assert_eq!(labels, ["07.1-01", "07.1-02"]);
    }

    /// wordoclock phase 16: same shape, verification found gaps. The phase is
    /// executed, not complete — and it must not read `Planned` either.
    #[test]
    fn a_slugged_phase_with_gaps_found_reads_executed_not_planned() {
        let dir = tempdir().unwrap();
        for (plan, summary) in [
            (
                "16-01-widget-strings-reader-tracer-PLAN.md",
                "16-01-SUMMARY.md",
            ),
            (
                "16-02-fallback-ladder-and-language-gates-PLAN.md",
                "16-02-SUMMARY.md",
            ),
        ] {
            fs::write(dir.path().join(plan), "plan").unwrap();
            fs::write(dir.path().join(summary), "summary").unwrap();
        }
        fs::write(
            dir.path().join("16-VERIFICATION.md"),
            "---\nstatus: gaps_found\n---\nbody",
        )
        .unwrap();

        let result = infer_disk_status(dir.path());
        assert_eq!(result.summary_count, 2);
        assert_eq!(result.status, DiskStatus::Executed);
        assert_eq!(result.verification_status, VerificationStatus::GapsFound);
    }

    /// A phase-level `NN-SUMMARY.md` is not any plan's partner: its stem is
    /// `13`, which carries no `NN-MM` index at all. It must pair with nothing
    /// even though four plans in the directory start with `13-`.
    #[test]
    fn a_phase_level_summary_pairs_with_no_plan() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("13-01-verse-pipeline-PLAN.md"), "plan").unwrap();
        fs::write(dir.path().join("13-SUMMARY.md"), "phase-level").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 1);
        assert_eq!(
            result.summary_count, 0,
            "a phase-level NN-SUMMARY.md has no plan index and must pair with nothing"
        );
        assert_eq!(result.status, DiskStatus::Planned);
    }

    /// Two plans can share an index when one is a re-cut of the other. A single
    /// slugless summary cannot say which it belongs to, so it satisfies
    /// NEITHER — an ambiguous index abstains rather than counting twice or
    /// guessing.
    #[test]
    fn one_summary_cannot_satisfy_two_plans_sharing_an_index() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-alpha-PLAN.md"), "plan a").unwrap();
        fs::write(dir.path().join("05-01-beta-PLAN.md"), "plan b").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 2);
        assert_eq!(
            result.summary_count, 0,
            "an index naming two surviving plans must pair with neither"
        );
        assert_ne!(
            result.status,
            DiskStatus::Executed,
            "an ambiguous pairing must never read as executed"
        );
    }

    /// When the stems DO match, that still wins outright — so two plans sharing
    /// an index are each satisfied by their own slugged summary, and the
    /// abstention above never costs a real pairing.
    #[test]
    fn an_exact_stem_match_pairs_even_when_the_index_is_ambiguous() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-alpha-PLAN.md"), "plan a").unwrap();
        fs::write(dir.path().join("05-01-beta-PLAN.md"), "plan b").unwrap();
        fs::write(dir.path().join("05-01-alpha-SUMMARY.md"), "sum a").unwrap();
        fs::write(dir.path().join("05-01-beta-SUMMARY.md"), "sum b").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 2);
        assert_eq!(result.summary_count, 2);
        assert_eq!(result.status, DiskStatus::Executed);
    }

    /// Two summaries resolving to the same plan count once. The count is of
    /// plans matched, not of summary files, so `summary_count` can never run
    /// past `plan_count` and fake a completion.
    #[test]
    fn two_summaries_for_one_plan_count_once() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("13-01-verse-pipeline-PLAN.md"), "plan").unwrap();
        fs::write(dir.path().join("13-01-SUMMARY.md"), "bare").unwrap();
        fs::write(
            dir.path().join("13-01-verse-pipeline-SUMMARY.md"),
            "slugged",
        )
        .unwrap();
        fs::write(dir.path().join("13-02-second-PLAN.md"), "plan2").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 2);
        assert_eq!(
            result.summary_count, 1,
            "both summaries name plan 13-01; that is one plan executed, not two"
        );
        assert_eq!(result.status, DiskStatus::Partial);
    }

    /// The index is two numbers, not two strings: `5-1` and `05-01` are one
    /// index. A lexical key would split them and drop the pairing.
    #[test]
    fn the_pairing_index_is_numeric_not_lexical() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("5-1-unpadded-slug-PLAN.md"), "plan").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 1);
        assert_eq!(result.summary_count, 1);
        assert_eq!(result.status, DiskStatus::Executed);
    }

    /// A superseded plan is excluded from BOTH keys, so its slugless summary
    /// still pairs with nothing — the #2349 rule survives the slugged shape.
    #[test]
    fn a_superseded_slugged_plan_still_drops_its_slugless_summary() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-alpha-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        fs::write(
            dir.path().join("05-02-beta-PLAN.md"),
            "---\nstatus: superseded\n---\nbody",
        )
        .unwrap();
        fs::write(dir.path().join("05-02-SUMMARY.md"), "summary2").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 1);
        assert_eq!(
            result.summary_count, 1,
            "the superseded plan's index must not be a pairing target"
        );
        assert_eq!(result.status, DiskStatus::Executed);
    }

    /// `14-REMEDIATION-SUMMARY.md` is a real wordoclock artifact. Its stem
    /// parses to no index and matches no plan stem, so it stays uncounted.
    #[test]
    fn a_non_numeric_suffixed_summary_pairs_with_nothing() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("14-01-l10n-infrastructure-PLAN.md"), "plan").unwrap();
        fs::write(dir.path().join("14-01-SUMMARY.md"), "summary").unwrap();
        fs::write(dir.path().join("14-REMEDIATION-SUMMARY.md"), "remediation").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 1);
        assert_eq!(result.summary_count, 1);
    }

    /// The empty-ID path: a bare `PLAN.md` has no index, a bare `SUMMARY.md`
    /// has no index, and they pair on identical (empty) stems exactly as before
    /// — the index tier must not disturb it, in either direction.
    #[test]
    fn the_bare_plan_summary_pair_is_untouched_by_the_index_tier() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("PLAN.md"), "plan").unwrap();
        fs::write(dir.path().join("SUMMARY.md"), "summary").unwrap();
        // An indexed summary alongside must still find no partner: the bare
        // plan carries no index for it to claim.
        fs::write(dir.path().join("07-03-SUMMARY.md"), "orphan").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 1);
        assert_eq!(result.summary_count, 1);
        assert_eq!(result.status, DiskStatus::Executed);
    }

    #[test]
    fn test_normal_two_plan_two_summary_is_executed() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-02-PLAN.md"), "plan2").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        fs::write(dir.path().join("05-02-SUMMARY.md"), "summary2").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 2);
        assert_eq!(result.summary_count, 2);
        assert_eq!(result.status, DiskStatus::Executed);
    }

    #[test]
    fn test_standalone_summary_requires_standalone_plan() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("PLAN.md"), "plan").unwrap();
        fs::write(dir.path().join("SUMMARY.md"), "summary").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 1);
        assert_eq!(result.summary_count, 1);
        assert_eq!(result.status, DiskStatus::Executed);
    }

    #[test]
    fn test_standalone_summary_without_plan_not_counted() {
        let dir = tempdir().unwrap();
        // Standalone SUMMARY.md with no PLAN.md must not be counted.
        fs::write(dir.path().join("SUMMARY.md"), "summary").unwrap();
        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_count, 0);
        assert_eq!(result.summary_count, 0);
        assert_eq!(result.status, DiskStatus::Empty);
    }

    #[test]
    fn test_find_phase_dir_in_milestones() {
        let dir = tempdir().unwrap();
        let milestone_phase = dir
            .path()
            .join("milestones")
            .join("v1.0")
            .join("03-something");
        fs::create_dir_all(&milestone_phase).unwrap();
        fs::create_dir_all(dir.path().join("phases")).unwrap();

        let found = find_phase_dir(dir.path(), "03");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), milestone_phase);
    }

    // ── Task 2: robust phase-token / directory matching ──

    #[test]
    fn test_phase_dir_matches_year_prefixed() {
        assert!(phase_dir_matches("14-2026-foo", "14"));
    }

    #[test]
    fn test_phase_dir_matches_decimal() {
        assert!(phase_dir_matches("0.3-slug", "0.3"));
    }

    #[test]
    fn test_phase_dir_matches_milestone_prefixed() {
        assert!(phase_dir_matches("M1-2-slug", "M1-2"));
    }

    #[test]
    fn test_phase_dir_matches_project_code_prefixed() {
        assert!(phase_dir_matches("AB-29-slug", "AB-29"));
    }

    #[test]
    fn test_phase_dir_matches_pad_insensitive() {
        assert!(phase_dir_matches("3-foo", "3"));
        assert!(phase_dir_matches("03-foo", "3"));
    }

    #[test]
    fn test_phase_dir_matches_rejects_longer_number() {
        assert!(!phase_dir_matches("14-foo", "1"));
    }

    #[test]
    fn test_phase_dir_matches_rejects_decimal_boundary() {
        assert!(!phase_dir_matches("1.2-foo", "1"));
    }

    /// ttbook: ROADMAP's checklist says `7.1`, the directory is `07.1-slug`.
    #[test]
    fn test_phase_dir_matches_decimal_pad_insensitive() {
        assert!(phase_dir_matches("07.1-apply-owner-rulings", "7.1"));
        assert!(phase_dir_matches("07.1-apply-owner-rulings", "07.1"));
        assert!(phase_dir_matches("7.1-apply", "07.1"));
        assert!(!phase_dir_matches("07.10-foo", "7.1"));
        assert!(!phase_dir_matches("07-consolidation", "7.1"));
        assert!(!phase_dir_matches("07.1-apply", "7"));
    }

    #[test]
    fn test_find_phase_dir_year_prefixed() {
        let dir = tempdir().unwrap();
        let phase_dir = dir.path().join("phases").join("14-2026-foo");
        fs::create_dir_all(&phase_dir).unwrap();

        let found = find_phase_dir(dir.path(), "14");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), phase_dir);
    }

    #[test]
    fn test_find_phase_dir_decimal() {
        let dir = tempdir().unwrap();
        let phase_dir = dir.path().join("phases").join("0.3-foo");
        fs::create_dir_all(&phase_dir).unwrap();

        let found = find_phase_dir(dir.path(), "0.3");
        assert!(found.is_some());
        assert_eq!(found.unwrap(), phase_dir);
    }

    // ── Task 3: COVERAGE / WINDOWS / deferred-items / SKELETON artifacts ──

    #[test]
    fn test_coverage_md_detected() {
        let bare = tempdir().unwrap();
        fs::write(bare.path().join("COVERAGE.md"), "coverage").unwrap();
        assert!(infer_disk_status(bare.path()).has_coverage);

        let prefixed = tempdir().unwrap();
        fs::write(prefixed.path().join("05-COVERAGE.md"), "coverage").unwrap();
        assert!(infer_disk_status(prefixed.path()).has_coverage);
    }

    #[test]
    fn test_windows_md_detected() {
        let bare = tempdir().unwrap();
        fs::write(bare.path().join("WINDOWS.md"), "windows").unwrap();
        assert!(infer_disk_status(bare.path()).has_windows);

        let prefixed = tempdir().unwrap();
        fs::write(prefixed.path().join("05-WINDOWS.md"), "windows").unwrap();
        assert!(infer_disk_status(prefixed.path()).has_windows);
    }

    #[test]
    fn test_deferred_items_md_detected() {
        let bare = tempdir().unwrap();
        fs::write(bare.path().join("deferred-items.md"), "deferred").unwrap();
        assert!(infer_disk_status(bare.path()).has_deferred_items);

        let prefixed = tempdir().unwrap();
        fs::write(prefixed.path().join("05-deferred-items.md"), "deferred").unwrap();
        assert!(infer_disk_status(prefixed.path()).has_deferred_items);
    }

    #[test]
    fn test_skeleton_md_detected() {
        let bare = tempdir().unwrap();
        fs::write(bare.path().join("SKELETON.md"), "skeleton").unwrap();
        assert!(infer_disk_status(bare.path()).has_skeleton);

        let prefixed = tempdir().unwrap();
        fs::write(prefixed.path().join("05-01-SKELETON.md"), "skeleton").unwrap();
        assert!(infer_disk_status(prefixed.path()).has_skeleton);
    }

    #[test]
    fn test_new_artifacts_do_not_affect_counts_or_status() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("COVERAGE.md"), "coverage").unwrap();
        fs::write(dir.path().join("WINDOWS.md"), "windows").unwrap();
        fs::write(dir.path().join("deferred-items.md"), "deferred").unwrap();
        fs::write(dir.path().join("SKELETON.md"), "skeleton").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(result.has_coverage);
        assert!(result.has_windows);
        assert!(result.has_deferred_items);
        assert!(result.has_skeleton);
        assert_eq!(result.plan_count, 0);
        assert_eq!(result.summary_count, 0);
        assert_eq!(
            result.status,
            DiskStatus::Empty,
            "informational artifacts must not change status"
        );
    }

    #[test]
    fn test_per_plan_security_still_detected() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-SECURITY.md"), "security").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(
            result.has_security,
            "05-01-SECURITY.md must still set has_security"
        );
        assert_eq!(result.plan_count, 0);
        assert_eq!(result.summary_count, 0);
    }

    #[test]
    fn test_empty_dir_has_no_new_artifacts() {
        let dir = tempdir().unwrap();
        let result = infer_disk_status(dir.path());
        assert!(!result.has_coverage);
        assert!(!result.has_windows);
        assert!(!result.has_deferred_items);
        assert!(!result.has_skeleton);
    }

    // ── Plan token counts (quick task 260916-vqx) ────────────────────────
    //
    // The two fixtures below are the two shapes GSD actually writes, copied
    // verbatim out of this repository's own `.planning/` tree. They are NOT the
    // same shape: the plan's block is clean, and the summary's carries a
    // column-zero `#` heading above the parent plus indented `#` comments
    // inside the block before the key. A reader that ends the block on any
    // non-key line reads the plan correctly and the summary as absent — half
    // the feature, silently.

    /// `.planning/phases/22-container-execution-target/22-01-PLAN.md`, verbatim
    /// block, inside a realistic surrounding frontmatter.
    const PLAN_22_01_SHAPE: &str = "\
---
phase: 22
plan: 01
type: execute
wave: 1
depends_on: []

estimate:
  tokens: 95000
  raw_tokens: 95000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - \"something true\"
---

# Plan body
";

    /// `.planning/phases/19-gitsafe-git-blast-radius-envelope/19-12-SUMMARY.md`,
    /// verbatim block including its comments.
    const SUMMARY_19_12_SHAPE: &str = "\
---
phase: 19
plan: 12
status: complete

# Actuals (#2632)
actuals:
  # chars/4 over the realized diff, which is the whole of the one created file
  # (51 383 chars / 4). The plan estimated 80 000 on the same scale.
  tokens: 12846
  tasks: 3
  commits: 3
---

# Summary body
";

    #[test]
    fn test_clean_nested_estimate_block_is_read() {
        assert_eq!(
            frontmatter_token_count(PLAN_22_01_SHAPE, "estimate"),
            Some(95_000),
            "the clean nested block GSD writes into a *-PLAN.md must read"
        );
    }

    #[test]
    fn test_commented_nested_actuals_block_is_read() {
        assert_eq!(
            frontmatter_token_count(SUMMARY_19_12_SHAPE, "actuals"),
            Some(12_846),
            "a column-zero `#` heading above the parent and indented `#` comments \
             inside the block end nothing; a reader that stops at them reads every \
             plan and no summary"
        );
    }

    #[test]
    fn test_raw_tokens_sibling_is_not_the_tokens_key() {
        // The clean fixture's siblings agree, so a `contains(\"tokens\")` reader
        // passes on it by luck. This fixture makes them disagree.
        let content = "---\nestimate:\n  raw_tokens: 2\n  tokens: 1\n---\n";
        assert_eq!(
            frontmatter_token_count(content, "estimate"),
            Some(1),
            "`raw_tokens` is a sibling of `tokens`, not a spelling of it: the key \
             is compared for EQUALITY after the first `:`, never by substring"
        );
    }

    #[test]
    fn test_column_zero_twin_is_a_different_key() {
        let content = "---\nphase: 22\ntokens: 999\n---\n";
        assert_eq!(
            frontmatter_token_count(content, "estimate"),
            None,
            "a `tokens:` at column zero is a DIFFERENT key from `estimate.tokens` \
             — WR-05's doctrine inverted"
        );
    }

    #[test]
    fn test_key_under_the_wrong_parent_is_a_different_key() {
        let actuals_only = "---\nactuals:\n  tokens: 5\n---\n";
        let estimate_only = "---\nestimate:\n  tokens: 5\n---\n";
        assert_eq!(frontmatter_token_count(actuals_only, "estimate"), None);
        assert_eq!(frontmatter_token_count(estimate_only, "actuals"), None);
        // ...and each still reads under its own parent, so the test is not
        // vacuously passing on a reader that always returns None.
        assert_eq!(frontmatter_token_count(actuals_only, "actuals"), Some(5));
        assert_eq!(frontmatter_token_count(estimate_only, "estimate"), Some(5));
    }

    #[test]
    fn test_two_levels_in_is_a_different_key() {
        let content = "---\nestimate:\n  breakdown:\n    tokens: 7\n---\n";
        assert_eq!(
            frontmatter_token_count(content, "estimate"),
            None,
            "only a DIRECT child at the block's own indent level counts; a deeper \
             nesting is a different key one level further in"
        );
    }

    #[test]
    fn test_nested_block_ends_at_the_next_column_zero_key() {
        let content = "---\nestimate:\n  tasks: 3\nmust_haves:\n  tokens: 9\n---\n";
        assert_eq!(
            frontmatter_token_count(content, "estimate"),
            None,
            "an unindented key line closes the open block; the next block's \
             children are not this block's"
        );
    }

    #[test]
    fn test_nested_block_ends_at_the_closing_fence() {
        let content = "---\nphase: 22\n---\n\nestimate:\n  tokens: 500\n";
        assert_eq!(
            frontmatter_token_count(content, "estimate"),
            None,
            "nothing below the closing `---` is frontmatter, matching \
             test_status_key_below_the_leading_block_is_never_matched"
        );
    }

    #[test]
    fn test_non_frontmatter_file_yields_no_token_count() {
        let content = "# A document\n\nestimate:\n  tokens: 500\n";
        assert_eq!(
            frontmatter_token_count(content, "estimate"),
            None,
            "the byte-zero anchor is load-bearing: a document DESCRIBING an \
             estimate does not HAVE one"
        );
    }

    #[test]
    fn test_inline_comment_is_not_part_of_the_value() {
        // The GSD plan template writes exactly this, even though this repo's own
        // plans do not.
        let content = "---\nestimate:\n  tokens: 60000             # calibrated projection\n---\n";
        assert_eq!(
            frontmatter_token_count(content, "estimate"),
            Some(60_000),
            "the value is truncated at the first whitespace-preceded `#`"
        );
    }

    #[test]
    fn test_junk_token_values_are_none_never_a_panic() {
        // Deliberately NOT a blank-shape character here: `test_support::DEGENERATE`
        // owns that payload set and `spawn_seam_guard`'s
        // `the_degenerate_payload_set_is_spelled_in_exactly_one_place` census
        // reports any hand copy of a witness. The non-ASCII case this list wants
        // is a DIGIT that `u64::from_str` refuses — Arabic-Indic `١٢` — which is
        // the parse property under test and not that guard's business. The
        // escaping property belongs to the renderer and is pinned there, in
        // `ui::screens::detail::tests::a_hostile_plan_label_reaches_no_cell_unescaped`.
        for value in [
            "soon",
            "",
            "-5",
            "1e5",
            "9_000",
            "\u{0661}\u{0662}",
            "99999999999999999999999999",
        ] {
            let content = format!("---\nestimate:\n  tokens: {value}\n---\n");
            assert_eq!(
                frontmatter_token_count(&content, "estimate"),
                None,
                "a malformed `tokens` value is absent, never an error and never a \
                 guess: value was {value:?}"
            );
        }
    }

    #[test]
    fn test_quoted_token_value_is_read() {
        let content = "---\nestimate:\n  tokens: \"4200\"\n---\n";
        assert_eq!(frontmatter_token_count(content, "estimate"), Some(4_200));
    }

    #[test]
    fn test_nested_value_reads_a_non_numeric_sibling_too() {
        // The block reader is general; `frontmatter_token_count` is the one
        // numeric consumer of it today.
        assert_eq!(
            leading_frontmatter_nested_value(PLAN_22_01_SHAPE, "estimate", "confidence"),
            Some("low".to_string())
        );
    }

    #[test]
    fn test_plan_tokens_collected_end_to_end_over_a_phase_directory() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("07-01-slug-PLAN.md"),
            "---\nestimate:\n  tokens: 95000\n---\nbody",
        )
        .unwrap();
        fs::write(
            dir.path().join("07-01-SUMMARY.md"),
            "---\nactuals:\n  tokens: 12846\n---\nbody",
        )
        .unwrap();
        fs::write(
            dir.path().join("07-02-slug-PLAN.md"),
            "---\nestimate:\n  tokens: 40000\n---\nbody",
        )
        .unwrap();
        // No `estimate` block at all, and no summary: contributes NO row.
        fs::write(
            dir.path().join("07-03-slug-PLAN.md"),
            "---\nphase: 7\n---\n",
        )
        .unwrap();

        let result = infer_disk_status(dir.path());

        assert_eq!(
            result.plan_tokens.len(),
            2,
            "a plan carrying neither number contributes no row (D-INF-02)"
        );
        assert_eq!(result.plan_tokens[0].id, "07-01-slug");
        assert_eq!(result.plan_tokens[0].label(), "07-01");
        assert_eq!(result.plan_tokens[0].estimate, Some(95_000));
        assert_eq!(result.plan_tokens[0].actual, Some(12_846));
        assert_eq!(result.plan_tokens[1].id, "07-02-slug");
        assert_eq!(result.plan_tokens[1].estimate, Some(40_000));
        assert_eq!(result.plan_tokens[1].actual, None);

        // This task changes no count.
        assert_eq!(result.plan_count, 3);
        assert_eq!(result.summary_count, 1);
    }

    #[test]
    fn test_summarized_plans_collected_and_sorted_by_pass_2() {
        let dir = tempdir().unwrap();
        let plan = "---\nphase: 13\n---\nbody";
        fs::write(dir.path().join("13-02-PLAN.md"), plan).unwrap();
        fs::write(dir.path().join("13-10-PLAN.md"), plan).unwrap();
        fs::write(dir.path().join("13-01-slug-PLAN.md"), plan).unwrap();
        fs::write(dir.path().join("13-10-SUMMARY.md"), "done").unwrap();
        fs::write(dir.path().join("13-01-SUMMARY.md"), "done").unwrap();
        // A superseded plan's summary pairs with nothing (#2349).
        fs::write(
            dir.path().join("13-03-PLAN.md"),
            "---\nstatus: superseded\n---\n",
        )
        .unwrap();
        fs::write(dir.path().join("13-03-SUMMARY.md"), "done").unwrap();

        let result = infer_disk_status(dir.path());

        // The pairing stores the PLAN's stem (slug included), not the
        // summary's, and orders numerically: 13-01 before 13-10.
        assert_eq!(
            result.summarized_plans,
            vec!["13-01-slug".to_string(), "13-10".to_string()]
        );
        assert_eq!(result.summary_count, 2, "the field IS the count");
        assert_eq!(result.plan_count, 3, "superseded 13-03 excluded");
        assert_eq!(
            infer_disk_status(dir.path()),
            result,
            "two scans of one directory compare equal"
        );
    }

    #[test]
    fn test_plan_tokens_order_is_numeric_and_the_inference_is_reproducible() {
        let dir = tempdir().unwrap();
        // `07-10` sorts BEFORE `07-2` lexically and AFTER it numerically.
        fs::write(
            dir.path().join("07-10-slug-PLAN.md"),
            "---\nestimate:\n  tokens: 10\n---\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("07-2-slug-PLAN.md"),
            "---\nestimate:\n  tokens: 2\n---\n",
        )
        .unwrap();

        let first = infer_disk_status(dir.path());
        let labels: Vec<String> = first.plan_tokens.iter().map(PlanTokens::label).collect();
        assert_eq!(
            labels,
            vec!["07-02".to_string(), "07-10".to_string()],
            "the order is by numeric plan INDEX; a lexical order would put 07-10 first"
        );

        let second = infer_disk_status(dir.path());
        assert_eq!(
            first, second,
            "two scans of an unchanged directory must produce EQUAL inferences — \
             `plan_ids` is a HashSet, and an unsorted plan_tokens would make every \
             refresh compare unequal and defeat the 260512-eyv change suppression"
        );
    }

    #[test]
    fn test_superseded_plan_contributes_no_token_row() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("05-01-PLAN.md"),
            "---\nestimate:\n  tokens: 111\n---\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("05-02-PLAN.md"),
            "---\nstatus: superseded\nestimate:\n  tokens: 222\n---\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("05-02-SUMMARY.md"),
            "---\nactuals:\n  tokens: 333\n---\n",
        )
        .unwrap();

        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_tokens.len(), 1);
        assert_eq!(result.plan_tokens[0].id, "05-01");
        assert_eq!(result.plan_tokens[0].estimate, Some(111));
        assert_eq!(
            result.plan_tokens[0].actual, None,
            "the superseded plan's summary pairs with nothing, so it contributes \
             no actual to anyone"
        );
    }

    #[test]
    fn test_ambiguous_plan_index_abstains_on_the_actual_too() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("07-01-alpha-PLAN.md"),
            "---\nestimate:\n  tokens: 1\n---\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("07-01-beta-PLAN.md"),
            "---\nestimate:\n  tokens: 2\n---\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("07-01-SUMMARY.md"),
            "---\nactuals:\n  tokens: 999\n---\n",
        )
        .unwrap();

        let result = infer_disk_status(dir.path());
        assert_eq!(
            result.summary_count, 0,
            "the existing two-tier pairing abstains on an ambiguous index"
        );
        assert_eq!(result.plan_tokens.len(), 2);
        for row in &result.plan_tokens {
            assert_eq!(
                row.actual, None,
                "the actual is attributed ONLY through that same pairing — a plan \
                 with an unpaired summary shows no actual rather than a neighbour's"
            );
        }
    }

    #[test]
    fn test_standalone_plan_and_summary_pair_and_label_as_plan() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("PLAN.md"),
            "---\nestimate:\n  tokens: 70000\n---\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("SUMMARY.md"),
            "---\nactuals:\n  tokens: 65000\n---\n",
        )
        .unwrap();

        let result = infer_disk_status(dir.path());
        assert_eq!(result.plan_tokens.len(), 1);
        assert_eq!(result.plan_tokens[0].id, "");
        assert_eq!(
            result.plan_tokens[0].label(),
            "PLAN",
            "the standalone case has an empty id and would otherwise render a blank label"
        );
        assert_eq!(result.plan_tokens[0].estimate, Some(70_000));
        assert_eq!(result.plan_tokens[0].actual, Some(65_000));
    }

    #[test]
    fn test_plans_without_the_keys_leave_plan_tokens_empty() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "plan1").unwrap();
        fs::write(dir.path().join("05-01-SUMMARY.md"), "summary1").unwrap();
        let result = infer_disk_status(dir.path());
        assert!(
            result.plan_tokens.is_empty(),
            "a project predating GSD's estimate/actuals keys renders exactly what \
             it rendered before this field existed: absence degrades to silence, \
             never to a column of dashes"
        );
    }

    // ── quick 260926-2l4: PlanMeta and the cached waves.json ────────────────

    #[test]
    fn plan_meta_title_comes_from_the_objective_first_sentence() {
        let content = "---\nphase: 25\nplan: 02\nwave: 1\n---\n\n# not a title source\n\n<objective>\nImplement the adapter (D-A04) against 25-01's seam: find the\nrest of it.\n</objective>\n";
        assert_eq!(
            plan_title(content).as_deref(),
            Some("Implement the adapter (D-A04) against 25-01's seam")
        );
        assert_eq!(objective_line(content), Some(9), "1-based line of the tag");

        // Text on the tag's own line, a `. ` boundary, emphasis stripped.
        assert_eq!(
            plan_title("<objective>Build the **Waves pane** first. Then more.\n").as_deref(),
            Some("Build the Waves pane first")
        );
        // A frontmatter title wins, quotes stripped.
        let titled = "---\ntitle: \"The real title\"\n---\n<objective>\nOther words.\n</objective>\n";
        assert_eq!(plan_title(titled).as_deref(), Some("The real title"));
        assert_eq!(objective_line(titled), Some(4));
        // No title, no objective: None. An empty objective: None.
        assert_eq!(plan_title("---\nphase: 1\n---\nbody\n"), None);
        assert_eq!(objective_line("---\nphase: 1\n---\nbody\n"), None);
        assert_eq!(plan_title("<objective>\n\n</objective>\n"), None);
        // A tag inside a code fence is not the objective.
        assert_eq!(objective_line("```\n<objective>\n```\n"), None);
        // Capped.
        let long = format!("<objective>\n{}\n", "x".repeat(400));
        assert_eq!(plan_title(&long).map(|t| t.chars().count()), Some(MAX_PLAN_TITLE_CHARS));
    }

    #[test]
    fn plan_meta_lists_every_surviving_plan_sorted_with_its_wave() {
        let dir = tempdir().unwrap();
        let hostile = "Evil \u{1b}[31mred\u{202e} title";
        fs::write(
            dir.path().join("05-10-late-PLAN.md"),
            "---\nwave: 2\n---\n<objective>\nTen comes after two.\n</objective>\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("05-02-early-PLAN.md"),
            format!("---\nwave: 1\n---\n<objective>\n{hostile}\n</objective>\n"),
        )
        .unwrap();
        fs::write(dir.path().join("05-03-nowave-PLAN.md"), "no frontmatter\n").unwrap();
        fs::write(
            dir.path().join("05-04-gone-PLAN.md"),
            "---\nstatus: superseded\nwave: 1\n---\n",
        )
        .unwrap();

        let inf = infer_disk_status(dir.path());
        let ids: Vec<&str> = inf.plans.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["05-02-early", "05-03-nowave", "05-10-late"]);
        let waves: Vec<Option<u32>> = inf.plans.iter().map(|p| p.wave).collect();
        assert_eq!(waves, vec![Some(1), None, Some(2)]);
        let title = inf.plans[0].title.as_ref().expect("a title");
        assert_eq!(title.as_raw_for_logic_only(), hostile, "stored raw");
        let shown = title.shown().to_string();
        assert!(!shown.contains('\u{1b}') && !shown.contains('\u{202e}'), "{shown:?}");
        assert_eq!(inf.plans[0].objective_line, Some(4));
        assert_eq!(inf.plans[1].title, None);
        assert_eq!(inf.plans[1].objective_line, None);
        assert_eq!(
            inf.plans[2].title.as_ref().map(|t| t.as_raw_for_logic_only().to_string()),
            Some("Ten comes after two".to_string())
        );
    }

    #[test]
    fn waves_manifest_is_read_in_the_scan() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("05-01-PLAN.md"), "x\n").unwrap();
        assert_eq!(infer_disk_status(dir.path()).waves_manifest, None, "absent");

        fs::write(
            dir.path().join("waves.json"),
            r#"{"waves":[{"id":"w1","plans":[{"id":"05-01","files_modified":["a","b"]}]}]}"#,
        )
        .unwrap();
        assert_eq!(
            infer_disk_status(dir.path()).waves_manifest,
            Some(plan_waves::WavesManifest {
                waves: vec![plan_waves::ManifestWave {
                    label: "w1".to_string(),
                    plans: vec![plan_waves::ManifestPlan {
                        id: "05-01".to_string(),
                        files: 2
                    }],
                }]
            })
        );

        fs::write(dir.path().join("waves.json"), "{ not json").unwrap();
        assert_eq!(infer_disk_status(dir.path()).waves_manifest, None, "unparsable");
        fs::write(dir.path().join("waves.json"), r#"{"waves":[]}"#).unwrap();
        assert_eq!(infer_disk_status(dir.path()).waves_manifest, None, "empty");

        // Over 1 MiB: skipped, even though it would parse.
        let padding = " ".repeat(MAX_WAVES_MANIFEST_BYTES as usize + 1);
        fs::write(
            dir.path().join("waves.json"),
            format!(r#"{{"waves":[{{"id":"w1","plans":[]}}]}}{padding}"#),
        )
        .unwrap();
        assert_eq!(infer_disk_status(dir.path()).waves_manifest, None, "oversized");
    }
}
