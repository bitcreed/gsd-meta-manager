//! The TUI's render surface, closed by **derivation** rather than by a list.
//!
//! Seven rounds of this phase each closed one enumeration level by naming the
//! next set of sites by hand, and each time the set that was named turned out to
//! be a subset of the set that existed. CR-01 is that failure one more time and
//! at its most embarrassing: the escaping mechanism ([`crate::text::display_identity`])
//! was correct, but the single UI call to it lived in `src/ui/project_list.rs` —
//! a file the module tree did not contain and the build never compiled, orphaned
//! since `c297631` and deleted in 21-21 — and that dead call convinced both a
//! reviewer and a verifier that the TUI was escaped.
//!
//! So this module answers exactly one question: **what performs the
//! enumeration?**
//!
//! * [`screen_implementors_from_source`] walks `src/` recursively with
//!   `std::fs::read_dir` and collects every implementation of the
//!   [`Screen`](super::Screen) trait. It is a **filesystem walk, never a path
//!   list**: a twelfth screen added tomorrow in a file this module has never
//!   heard of is discovered without anybody editing anything here.
//! * [`SCREEN_IDENTITY_DISPOSITIONS`] records, per implementor, whether it
//!   renders attacker-influenced identity and why. **The table adjudicates; it
//!   does not enumerate.** `the_screen_census_matches_the_tree` asserts the two
//!   sets equal in BOTH directions, so a derived member with no row is an
//!   unadjudicated screen and a row with no derived member is a stale row, and
//!   each is reported as its own harm.
//! * `the_screen_renders_identity_escaped` then CHECKS each disposition rather
//!   than trusting it, by rendering the screen through the real
//!   [`Screen::render`](super::Screen::render) into a ratatui `Buffer` and
//!   inspecting the resulting cells.
//!
//! **The probe is behavioural on purpose.** It inspects what was rendered, not
//! what the source says, so it is blind to no sink spelling: a new render site
//! added tomorrow inside an existing screen is caught whether it is written
//! `Span::raw`, `Span::styled`, `Line::from`, `Paragraph::new`,
//! `.title(format!(…))` or a constructor nobody has invented yet. The
//! alternative — a syntactic scan — would have to judge every sink call under
//! `src/ui/`, and every one of them is a place for a scan to be one spelling
//! short.
//!
//! # The census was observed catching a real twelfth screen, not argued to
//!
//! A throwaway twelfth implementor of the trait was added to
//! `src/driver/liveness.rs` — a file two directory levels down, with nothing to
//! do with the UI, and one this plan does not otherwise touch — and the census
//! reported it with nobody editing anything:
//!
//! (The sentence above deliberately avoids spelling the implementation header
//! literally. The walk already drops comment lines, so a doc naming the trait
//! cannot forge a member — but a naive `grep -rn` over the tree does NOT drop
//! them, and a reader cross-checking the census's count against such a grep
//! would otherwise get 12 where the census reports 11 and have to work out why.)
//!
//! ```text
//! thread 'ui::screens::render_escape_guard::tests::the_screen_census_matches_the_tree' (1531855) panicked at src/ui/screens/render_escape_guard.rs:863:9:
//! the derived render surface and the disposition table disagree:
//!
//! UNADJUDICATED IMPLEMENTOR: `TwelfthScreenNobodyAdjudicated` (in src/driver/liveness.rs) implements the trait but no row adjudicates whether it renders identity. A screen nobody adjudicated is a screen nobody escaped. Add a row to SCREEN_IDENTITY_DISPOSITIONS stating which values it draws and where they come from.
//! ```
//!
//! The implementor was then removed and the tree confirmed clean
//! (`git status --porcelain` does not name `src/driver/liveness.rs`).
//!
//! # LIMITS — what this module does NOT bound, each with its failure direction
//!
//! Stated here because the failure this phase keeps repeating is a guard whose
//! doc claims a bound no committed control goes red for. Every bound claimed
//! below names the control that certifies it; every residual names the
//! direction it fails in.
//!
//! 1. **The probe sees only what a screen renders under the states its fixture
//!    constructs.** A render path reachable only under state the fixture does
//!    not build is invisible to it. **Under-detection, and silent.**
//!
//!    **REWRITTEN 2026-08-27 (21-25), and the wording it replaces is quoted so
//!    the narrowing is checkable rather than asserted.** This limit used to
//!    read, after the sentence above: *"Not bounded at all within a state: the
//!    Backlog, Sessions, Archive, Browse and Defaults tabs, and the Driver
//!    tab's run list and run detail, draw from `view_cache` / `archive_cache` /
//!    `active_sessions` entries that `probe_ctx` leaves at their defaults, so
//!    those tabs render their empty branch and their populated branches are
//!    **not** exercised by any committed control. That residual is disclosed,
//!    not closed."*
//!
//!    Every one of those caches is now populated by [`probe_ctx`], so all
//!    eleven tabs render a POPULATED branch on every run, and three of them are
//!    additionally probed in the second state their own dispatch field selects
//!    (`backlog_expanded`, `archive_depth` at two further depths,
//!    `browser_depth`). Populating them was not bookkeeping: it produced four
//!    live leaks that no reader had found in nine rounds — the Defaults tab's
//!    `entry.value`, the Browse tab's breadcrumb path, four half-escaped sites
//!    on the Driver tab (control class applied, invisible class not), and the
//!    markdown body both file viewers draw.
//!
//!    **What is bounded now, and by what.** Arrival is recorded PER STATE by
//!    [`DETAIL_TAB_ARRIVAL`] and asserted as a set equality in both directions,
//!    measured against the chrome baseline [`chrome_ctx`] renders rather than
//!    by containment — so a populated cache whose render the tab never reads is
//!    reported by name instead of counted as coverage. That check was observed
//!    red by planting in both directions (an emptied `backlog_items`; a row
//!    flipped to `false`).
//!
//!    **What REMAINS, with a concrete example, because a residual with no
//!    example is a residual nobody can check.** The residual is now *states no
//!    fixture constructs* rather than *tabs no fixture populates*. Concretely:
//!    the Defaults tab's string-EDIT overlay — `defaults_editing = Some(idx)`
//!    on a `ConfigValueKind::String` entry — draws `defaults_text_buffer` and
//!    `entry.key` into a `Clear`ed popup through a code path no probe state
//!    reaches, and the Driver tab's `driver_dry_run` preview is another. Both
//!    are reachable only by driving the key handler into a mode, which is the
//!    shape `DriverStartScreen`'s "goal step" fixture uses and which is not
//!    done for these. **Under-detection, silent.**
//! 2. **The probe judges the invisible class, not homoglyphs.** A Cyrillic `а`
//!    renders like a Latin `a` and is accepted here, exactly as
//!    [`crate::text::carries_invisible_formatting`] records for its own class.
//!    **Under-detection, and by design** — Unicode TR39 confusables are a
//!    separate roadmap item, and at identity SEAMS the harm is already closed
//!    by the finite alphabet ([`crate::text::is_identity_char`]), which admits
//!    no Cyrillic at all. Disclosed, not bounded here.
//! 3. **A render surface not reached through `Screen::render` is invisible to
//!    the probe** — a widget drawn from somewhere else, or a future second
//!    entry point beside `ui::render`. **Under-detection, silent.** What bounds
//!    how that residual can GROW is the census rather than the probe: a new
//!    entry point that is a `Screen` is reported by
//!    `the_screen_census_matches_the_tree`, which is the control observed red
//!    above. A new entry point that is NOT a `Screen` is reported by nothing
//!    here.
//! 4. **The raw-absence assertion is PRESENT, and its power is per widget
//!    family.** This limit used to read: *"The probe cannot assert that the RAW
//!    form is absent. ratatui 0.30 deletes zero-width graphemes before a cell
//!    exists, so that assertion is true of an unescaped site too and would pass
//!    vacuously forever."* That was measured against a `Paragraph` and
//!    generalised to the whole rendering stack, and the generalisation is FALSE
//!    — re-derived per widget family in a scratch crate outside this tree
//!    against ratatui 0.30.2 (the table is quoted in
//!    `the_screen_renders_identity_escaped`'s doc and in `deferred-items.md`):
//!    `Paragraph` and `Paragraph`-in-`Block` drop `U+202E`, `U+200B`, `U+00AD`,
//!    `U+2062`, `U+2065` and `U+FEFF`; `Block::title` and `ListItem` PRESERVE
//!    every one of them. Those two families are exactly where this tree's live
//!    leaks were, so the assertion is NOT vacuous — it is assertion 4, gated on
//!    arrival exactly as assertion 2 is, and it was observed RED for
//!    `DetailScreen [GitHistory tab]` against a tree where only the git-history
//!    `shown()` was reverted.
//!
//!    **Where it still has no power, with its direction.** At a `Paragraph`
//!    site the raw zero-width form never reaches a cell whether or not the site
//!    escapes, so assertion 4 passes there for a reason unrelated to the code.
//!    **Under-detection at `Paragraph` sites, silent.** What bounds THAT is
//!    assertion 2 (the escaped form must be present) and assertion 3 (no
//!    invisible-class character may reach a cell, which the tag block triggers
//!    through every family). The three assertions have different blind spots by
//!    construction, which is why all three are kept.
//! 5. **Assertion 3's teeth are a property of the FIXTURE, and that property is
//!    now checked rather than assumed.** `TAG_PAIR: usize = 4` is a hand-
//!    maintained index into `LOOK_ALIKE_PAIRS`, and a reorder that put a
//!    dropped-before-a-cell pair at that index would leave assertion 3 green
//!    and empty. Assertion 0 renders `hostile_identity()` through a `ListItem`
//!    and requires that at least one invisible-class character arrives, so the
//!    probe REFUSES TO RUN when the teeth are gone and names the fixture rather
//!    than a screen. The index itself is deliberately NOT pinned (D-21-12): an
//!    equality on `TAG_PAIR` would be red on a harmless reorder and green on a
//!    harmful content change, which is wrong in both directions. The
//!    precondition's own `false` direction is certified by
//!    `the_teeth_precondition_answers_false_when_the_class_cannot_reach_a_cell`,
//!    which drives the same helper with an all-ASCII identity and with one whose
//!    class members a `Paragraph` drops.
//!
//! **Every bound claimed above names a committed control; every residual names
//! its direction.** Limits 1, 2, 3 and the `Paragraph` half of 4 are residuals
//! and are marked under-detection. Limits 4 (for the preserving families) and 5
//! are bounds, and the controls are `the_screen_renders_identity_escaped`'s
//! assertions 0 and 4 plus
//! `the_teeth_precondition_answers_false_when_the_class_cannot_reach_a_cell`,
//! each observed red before it was observed green.

use super::{AppContext, Screen};
use crate::test_support::LOOK_ALIKE_PAIRS;
use crate::text::{display_identity, is_invisible_formatting_char};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// The disposition vocabulary
// ---------------------------------------------------------------------------

/// The screen draws at least one string this build did not author — a registry
/// key, or a workspace/phase/file/entry name read from `.planning/`.
const RENDERS_IDENTITY: &str = "renders_attacker_influenced_identity";

/// The screen draws only text this build authored itself.
const RENDERS_NO_IDENTITY: &str = "renders_no_attacker_influenced_identity";

/// `(type name, file relative to the crate root, disposition, reason)`.
///
/// The reason column is the adjudication, and a future reader inherits it: it
/// states **which values the screen draws and where they come from**, never
/// "escaped" or "safe".
///
/// Rows are added here because the walk found an implementor, never the other
/// way round.
type DispositionRow = (&'static str, &'static str, &'static str, &'static str);

const SCREEN_IDENTITY_DISPOSITIONS: &[DispositionRow] = &[
    (
        "AddProjectScreen",
        "src/ui/screens/add_project.rs",
        RENDERS_IDENTITY,
        "Draws the alias the operator is typing (`ctx.input_buffer`) and, beside \
         it, `ctx.error_message` — which on this screen is an `AliasRefusal` \
         whose Display embeds the alias it refused. The background is an empty \
         bordered block and draws nothing. Fixture states: the alias field, and \
         the alias field with a real refusal for the hostile identity echoed \
         beside it.",
    ),
    (
        "CreateProjectScreen",
        "src/ui/screens/create_project.rs",
        RENDERS_IDENTITY,
        "Draws the project name the operator is typing, `ctx.error_message`, and \
         in its Confirm phase the chosen name and path. The name becomes a \
         directory, so it is an identity in the full sense. Fixture states: the \
         name field, and the name field with an error echoed beside it.",
    ),
    (
        "DeleteConfirmScreen",
        "src/ui/screens/delete_confirm.rs",
        RENDERS_IDENTITY,
        "Draws the registry key of the project about to be unregistered into a \
         destructive [y/n] prompt, again into the removal toast, and again into \
         the live-run refusal. This is the highest-consequence identity render \
         in the tree: the operator confirms the name they READ, so a rendered \
         name that is not the key is a confirmation of a different thing than \
         was asked (T-21-21-01). Fixture state: the confirm prompt for a \
         registered hostile key.",
    ),
    (
        "DetailScreen",
        "src/ui/screens/detail.rs",
        RENDERS_IDENTITY,
        "The widest identity surface in the tree. Draws the registry key in its \
         tab-bar title, and in its eleven tabs the values parsed out of the \
         project's `.planning/`. Per tab, the values and where their bytes come \
         from: PhaseList and RoadmapViz draw each `RoadmapPhase`'s number, name \
         and description plus the status and milestone, all parsed from \
         `ROADMAP.md`/`STATE.md`; Pipeline draws the current phase name, status \
         and the HANDOFF pause context; Queue draws each `QueuedAction::command` \
         from `queue.md`; Backlog draws a `999.*` directory's number and \
         description in its collapsed state and that directory's NAME (through \
         `Block::title`) plus the BODY of the first `.md` file inside it when \
         expanded; GitHistory draws a third-party repository's commit hash, \
         date, author and subject; Sessions draws a session id scraped from \
         another process's `--resume` argument via `/proc`; Archive draws \
         milestone version strings, archive file names and phase display names \
         from `.planning/archive/` directory listings, at three different \
         depths that are three different renders of three different names; \
         Defaults draws the value of every key of the project's \
         `.planning/config.json`, of which `mode`, `granularity`, \
         `project_code`, `phase_naming` and `response_language` are free-form \
         strings; Browse draws the browsed directory's path relative to \
         `.planning/`, each listing entry's name, and — in its file view — the \
         file name and the whole markdown body; Driver draws the run id suffix, \
         goal, `gsd_command` and run directory read back out of a run's \
         committed `run.json`. All of it is third-party text under SAFE-07 and \
         none of it was authored by this build. Fixture states: one per \
         sub-view, all eleven, EACH RENDERING ITS POPULATED BRANCH (21-25), plus \
         four within-tab states for the fields that dispatch to a different \
         render — Backlog expanded, Archive at its phase list and file list \
         depths, Browse at its file view. Arrival is recorded per state by \
         DETAIL_TAB_ARRIVAL against the chrome baseline, so a populated cache \
         the render never reads is reported rather than counted.",
    ),
    (
        "DriverConfirmScreen",
        "src/ui/screens/driver_confirm.rs",
        RENDERS_IDENTITY,
        "Draws the registry key into four prompts — start, stop, grant opt-in, \
         withdraw opt-in — each of which precedes an irreversible act, and draws \
         the command and the goal (already `sanitize_render_line`d for C0/ESC, \
         which is a different class from the invisible one). Fixture states: all \
         four prompts.",
    ),
    (
        "DriverInjectScreen",
        "src/ui/screens/driver_inject.rs",
        RENDERS_IDENTITY,
        "Paints its body with `DetailScreen::render_main_only`, so it draws \
         everything the active detail tab draws, and adds a footer echoing the \
         steering message being typed. Fixture states: one per sub-view.",
    ),
    (
        "DriverStartScreen",
        "src/ui/screens/driver_start.rs",
        RENDERS_IDENTITY,
        "Paints its body with `DetailScreen::render_main_only`, and its two \
         wizard rows draw the command being typed (Step A) and the committed \
         command (Step B). Fixture states: one per sub-view at Step A, plus Step \
         B reached by driving the real key handler.",
    ),
    (
        "EnqueueScreen",
        "src/ui/screens/enqueue.rs",
        RENDERS_IDENTITY,
        "Paints its body with `DetailScreen::render_main_only`, and its footer \
         echoes `ctx.input_buffer` — which Tab-completion fills from \
         `queue_md::suggest_next_commands`, a function of the project's parsed \
         `.planning/` state, so the buffer is not always something the operator \
         typed. Fixture states: one per sub-view.",
    ),
    (
        "HelpScreen",
        "src/ui/screens/help.rs",
        RENDERS_NO_IDENTITY,
        "Draws `help_lines()`, which the module doc calls a pure function of \
         nothing: keybindings, the filter grammar and two legends, every byte of \
         it authored in this repository. It `Clear`s its popup area and paints \
         no background, so nothing from `AppContext` reaches a cell. Checked, \
         not claimed: the fixture registers the hostile identity and puts a \
         hostile project state behind it, and the probe asserts the clean stem \
         is absent from the buffer.",
    ),
    (
        "NormalScreen",
        "src/ui/screens/normal.rs",
        RENDERS_IDENTITY,
        "The dashboard. Draws every registered key in the name column together \
         with the phase, status and milestone parsed from each project's \
         `.planning/`, and echoes the filter text in its search footer. \
         `row_badge`'s lookup keys off the RAW alias while the cell beside it is \
         escaped — the worked example of the split. Fixture states: the \
         dashboard, and the dashboard with the filter footer active.",
    ),
    (
        "QueueDeleteConfirmScreen",
        "src/ui/screens/queue_delete_confirm.rs",
        RENDERS_IDENTITY,
        "Paints its body with `DetailScreen::render_main_only`, and its \
         destructive [y/n] footer draws the queued command text read from the \
         project's `.planning/queue.md`. Fixture states: one per sub-view.",
    ),
];

// ---------------------------------------------------------------------------
// The walk — what performs the enumeration
// ---------------------------------------------------------------------------

const SRC_ROOT: &str = "src";

/// The needle is assembled at RUNTIME from two halves that are meaningless
/// apart, following `tests/spawn_seam_guard.rs:216`. Spelled as one literal,
/// this file's own source would match the walk and the census would start
/// reporting itself.
const IMPL_HEAD: &str = "Screen";
const IMPL_TAIL: &str = " for ";

/// One source file: its path relative to the crate root, and its numbered lines.
type SourceFile = (String, Vec<(usize, String)>);

fn collect(dir: &Path, base: &Path, out: &mut Vec<SourceFile>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            collect(&path, base, out);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let relative = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .replace('\\', "/");
        let lines = text
            .lines()
            .enumerate()
            .map(|(index, line)| (index + 1, line.to_string()))
            .collect();
        out.push((relative, lines));
    }
}

/// Every type under `src/` that implements the [`Screen`](super::Screen) trait,
/// mapped to the file it lives in.
///
/// **This is the enumeration, and it is a `read_dir` walk.** The recursive shape
/// follows `tests/spawn_seam_guard.rs:227-294`: an unreadable entry is skipped
/// rather than panicked on, paths are relative to `CARGO_MANIFEST_DIR`, and
/// lines whose trimmed form opens a line comment are dropped so a doc comment
/// naming the trait cannot forge a member.
///
/// It asserts that it found production source at all — the non-vacuity floor
/// `source_files` already carries — so a walk that looked at nothing cannot
/// report clean.
fn screen_implementors_from_source() -> BTreeMap<String, String> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect(&base.join(SRC_ROOT), &base, &mut files);
    assert!(
        !files.is_empty(),
        "the census walked {SRC_ROOT}/ and found no Rust source at all, which \
         means it is enumerating nothing — a clean result here would be a walk \
         that never looked"
    );
    files.sort_by(|a, b| a.0.cmp(&b.0));

    let needle = format!("{IMPL_HEAD}{IMPL_TAIL}");
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for (path, lines) in &files {
        for (_, line) in lines {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || !trimmed.starts_with("impl") {
                continue;
            }
            let Some(at) = trimmed.find(&needle) else {
                continue;
            };
            let name: String = trimmed[at + needle.len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if name.is_empty() {
                continue;
            }
            found.insert(name, path.clone());
        }
    }
    assert!(
        !found.is_empty(),
        "the census walked {} source files and found no trait implementors at \
         all. Either the trait was renamed or the needle stopped matching; \
         either way this census is now enumerating nothing.",
        files.len()
    );
    found
}

// ---------------------------------------------------------------------------
// The comparison — extracted pure, so a synthetic pair can drive it
// ---------------------------------------------------------------------------

/// Every way the derived set and the disposition table can disagree, each named
/// as its own harm.
///
/// Extracted from the live assertion on purpose: a comparison that only ever
/// runs against a clean tree is a comparison whose clean result is
/// indistinguishable from a comparison that stopped comparing.
/// `the_census_reports_an_unadjudicated_screen_and_a_stale_row` drives this same
/// function with synthetic pairs in both directions.
fn census_offences(
    derived: &BTreeMap<String, String>,
    table: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut offences = Vec::new();
    for (name, path) in derived {
        match table.get(name) {
            None => offences.push(format!(
                "UNADJUDICATED IMPLEMENTOR: `{name}` (in {path}) implements the \
                 trait but no row adjudicates whether it renders identity. A \
                 screen nobody adjudicated is a screen nobody escaped. Add a row \
                 to SCREEN_IDENTITY_DISPOSITIONS stating which values it draws \
                 and where they come from."
            )),
            Some(recorded) if recorded != path => offences.push(format!(
                "RELOCATED IMPLEMENTOR: `{name}` is adjudicated at {recorded} but \
                 the walk found it at {path}. The row's reason was written about \
                 a file that no longer holds it; re-read the render and re-state \
                 the adjudication."
            )),
            Some(_) => {}
        }
    }
    for (name, path) in table {
        if !derived.contains_key(name) {
            offences.push(format!(
                "STALE ROW: `{name}` is adjudicated at {path} but the walk found \
                 no such implementor. Either the screen was deleted and the row \
                 outlived it, or the walk stopped reaching it — and a table that \
                 outlives its subject is how a reader is told a surface is \
                 covered when it is not."
            ));
        }
    }
    offences
}

fn disposition_table() -> BTreeMap<String, String> {
    SCREEN_IDENTITY_DISPOSITIONS
        .iter()
        .map(|(name, path, _, _)| ((*name).to_string(), (*path).to_string()))
        .collect()
}

// ---------------------------------------------------------------------------
// The probe fixtures
// ---------------------------------------------------------------------------

/// The identity a probe run hands to a screen, in a form the screen will draw.
///
/// Every fixture is drawn **by import** from
/// [`LOOK_ALIKE_PAIRS`](crate::test_support::LOOK_ALIKE_PAIRS) and never
/// respelled as a literal here (D-21-6): a hand copy can silently disagree with
/// the const, and new witness literals in this file would collide with the
/// `DEGENERATE` uniqueness scan.
///
/// The two members are the concatenation of two imported pairs:
///
/// * `clean` — `LOOK_ALIKE_PAIRS[4].0` + `LOOK_ALIKE_PAIRS[5].0`. All-ASCII, so
///   a screen that draws it draws it verbatim and nothing can drop it.
/// * `hostile` — `LOOK_ALIKE_PAIRS[4].1` + `LOOK_ALIKE_PAIRS[5].1`. Carries one
///   tag character (`U+E0041`, the LLM ASCII-smuggling carrier, which ratatui
///   passes through intact) and one `U+00AD` (SOFT HYPHEN, which ratatui drops).
///   Both are from OUTSIDE the pre-round-7 literal ranges, so a probe cannot
///   pass by re-confirming the old class.
///
/// The concatenation matters: the clean stem is a token no screen's authored
/// text contains, which is what makes both the arrival assertion and the
/// absence assertion mean something.
const TAG_PAIR: usize = 4;
const SOFT_HYPHEN_PAIR: usize = 5;

fn clean_identity() -> String {
    format!(
        "{}{}",
        LOOK_ALIKE_PAIRS[TAG_PAIR].0, LOOK_ALIKE_PAIRS[SOFT_HYPHEN_PAIR].0
    )
}

fn hostile_identity() -> String {
    format!(
        "{}{}",
        LOOK_ALIKE_PAIRS[TAG_PAIR].1, LOOK_ALIKE_PAIRS[SOFT_HYPHEN_PAIR].1
    )
}

/// An identity whose every invisible-class member is one a `Paragraph` DROPS
/// before a cell exists — `U+200B`, `U+FEFF` and `U+00AD`, and no tag character.
///
/// Drawn BY IMPORT from three pairs of [`LOOK_ALIKE_PAIRS`] (D-21-6), never
/// respelled: a hand copy can silently disagree with the const, and a new
/// invisible literal in this file would collide with the `DEGENERATE` uniqueness
/// scan.
///
/// It exists for exactly one purpose — the `false` direction of
/// [`survives_a_rendered_buffer`]. Through [`ProbeSink::Paragraph`] this value's
/// class members never reach a cell, so the helper must answer `false`; through
/// [`ProbeSink::ListItem`] every one of them does, so it must answer `true`. One
/// value, two answers, is what proves the helper is measuring the RENDER rather
/// than the string.
const ZERO_WIDTH_PAIRS: [usize; 3] = [0, 2, SOFT_HYPHEN_PAIR];

fn zero_width_only_identity() -> String {
    ZERO_WIDTH_PAIRS
        .iter()
        .map(|index| LOOK_ALIKE_PAIRS[*index].1)
        .collect()
}

/// One render state a probe run puts a screen in.
///
/// A screen is not one picture. `DetailScreen` has eleven tabs;
/// `DriverStartScreen` has two steps; `DriverConfirmScreen` has four prompts.
/// The probe renders EACH, and the `label` is what a failure names — so a
/// failure says which state leaked rather than only which screen.
struct ProbeState {
    label: String,
    ctx: AppContext,
    screen: Box<dyn Screen>,
}

/// Build every render state for one implementor.
type Fixture = fn(&str) -> Vec<ProbeState>;

/// A `ProjectState` whose every `.planning/`-read field carries `identity`.
///
/// **This is what widens the probe past aliases.** SAFE-07's trust boundary is
/// `.planning/` file content, so a workspace name, a phase name, a milestone, a
/// status, a HANDOFF context line and a queued command are attacker-influenced
/// in exactly the way a registry key is. Putting the identity in all of them at
/// once means the probe's invisible-class assertion goes red for whichever one a
/// screen renders raw, without the probe having to know which.
fn hostile_project_state(identity: &str) -> crate::state_reader::ProjectState {
    use crate::state_reader::queue_md::QueuedAction;
    use crate::state_reader::roadmap_md::RoadmapPhase;
    use crate::state_reader::ProjectState;

    ProjectState {
        status: identity.to_string(),
        current_phase: "1".to_string(),
        current_phase_name: identity.to_string(),
        current_plan: identity.to_string(),
        total_phases: 1,
        completed_phases: 0,
        total_plans: 1,
        completed_plans: 0,
        milestone: identity.to_string(),
        backlog_count: 1,
        phases: vec![RoadmapPhase {
            number: "1".to_string(),
            name: identity.to_string(),
            description: identity.to_string(),
            completed: false,
            total_plans: 1,
            completed_plans: 0,
            depends_on: vec![identity.to_string()],
        }],
        queued_actions: vec![QueuedAction {
            command: identity.to_string(),
        }],
        paused: true,
        pause_context: Some(identity.to_string()),
        deferred_verification_phases: vec![identity.to_string()],
        ..Default::default()
    }
}

/// An `AppContext` with `identity` registered as a key AND as every
/// `.planning`-read value the dashboard and detail views draw.
///
/// Built on `super::tests::ctx_with_aliases` rather than beside it: a second
/// full-field `AppContext` literal is a second thing to keep in step with the
/// struct.
///
/// # The `git_entries` fixture hole, and why closing it makes CR-04 stop being
/// intermittent
///
/// This function used to reach `git_entries` only through
/// `view_cache.entry(..).or_default()`, so the vector was always EMPTY and
/// `DetailScreen::render_git_tab` returned at its `if cache.git_entries.is_empty()`
/// branch after painting the authored string `"  No commits found (or not a git
/// repository)"`. The `List`/`ListItem` build below that branch — which draws a
/// third-party repository's commit hash, date, author and subject — was never
/// exercised by any committed control, and LIMIT 1 of this module's doc named
/// exactly that class of hole without naming this instance of it.
///
/// Verification pass 9 reported `the_screen_renders_identity_escaped` panicking
/// ONCE for `DetailScreen [GitHistory tab]` with
/// `['\u{e0041}', '\u{ad}', '\u{e0041}', '\u{ad}']`, and offered "the commit
/// cache happened to be populated" as a reconciliation. That reconciliation does
/// not survive reading this function: `ctx_with_aliases` performs no I/O and
/// `or_default()` cannot fill a vector. So the MECHANISM of that one sighting is
/// still unexplained and plan 21-23 does not pretend otherwise. What it does
/// instead is decisive — populate the cache here and the defect fires on EVERY
/// run rather than on one run in eighty.
///
/// **The RED, verbatim, before any escape landed** (`cargo test --lib --
/// ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped
/// --exact --nocapture`, against this fixture with `GitLogEntry`'s fields still
/// bare `String` and the `List` render still raw):
///
/// ```text
/// thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (333264) panicked at src/ui/screens/render_escape_guard.rs:1157:17:
/// DetailScreen (src/ui/screens/detail.rs) [GitHistory tab] rendered ['\u{e0041}', '\u{ad}', '\u{e0041}', '\u{ad}', '\u{e0041}', '\u{ad}', '\u{e0041}', '\u{ad}'] into the terminal buffer. Those characters render as nothing, so what the operator reads is not what the value is.
/// ```
///
/// Note `\u{ad}` in that list. `U+00AD` reaching a cell is IMPOSSIBLE through a
/// `Paragraph` and routine through a `ListItem` — measured per widget family, see
/// [`the_screen_renders_identity_escaped`](tests::the_screen_renders_identity_escaped)'s
/// doc — so the character list is itself independent evidence that the leak was a
/// `List` row, arriving from the opposite direction to the measurement.
///
/// **Where this red does NOT match pass 9's sighting, said rather than
/// smoothed.** Pass 9 reported four characters
/// (`['\u{e0041}', '\u{ad}', '\u{e0041}', '\u{ad}']`); this fixture reports
/// EIGHT, which is one hostile pair per `GitLogEntry` field and is what a render
/// of all four fields must produce. Four is two fields' worth. So this fixture
/// reproduces the CLASS of defect pass 9 saw and does not reproduce its exact
/// count, and the mechanism of that one sighting therefore remains unexplained.
/// Reading (a) of pass 9's dichotomy — a real leak in this tab — is settled here
/// by construction; the count mismatch is not evidence for or against reading (b)
/// and is recorded as an open discrepancy rather than absorbed.
///
/// Reproduction rate with this fixture and the escape NOT applied: **20 failures
/// in 20 runs** of the compiled lib test binary. A defect that fires 20 out of 20
/// is not a flake.
/// # The rest of the fixture hole, closed in 21-25 rather than re-disclosed
///
/// 21-23 closed `git_entries`. The same hole was still open for five more of
/// `DetailScreen`'s eleven tabs, and LIMIT 1 named them by name: **Backlog,
/// Sessions, Archive, Browse and Defaults** all reached their caches only
/// through `view_cache.entry(..).or_default()`, so each rendered its EMPTY
/// branch under probe and every render site below that branch was exercised by
/// no committed control. The Driver tab was in the same position through
/// `driver_runs`.
///
/// Everything below populates those caches with the identity the probe handed
/// in, so each tab takes its POPULATED branch on every run. The measured
/// consequence, recorded rather than promised: with these caches populated and
/// the escapes of this plan's Task 1 NOT applied, the probe goes red naming the
/// specific tab. Four such reds are quoted in
/// `21-25-SUMMARY.md`, one per newly covered tab, each produced by reverting
/// exactly one `shown()` and restoring it immediately.
///
/// **What this does NOT do**, said here because a populated cache the render
/// path never reads makes every new assertion pass by silence. Arrival is
/// RECORDED PER TAB by [`DETAIL_TAB_ARRIVAL`] and asserted as a set equality,
/// so a tab whose cache is populated but whose identity does not reach the
/// buffer is reported by name instead of counted as coverage.
fn probe_ctx(identity: &str) -> AppContext {
    use crate::browser::{BrowserDepth, BrowserEntry};
    use crate::state_reader::backlog::BacklogItem;
    use crate::text::Untrusted;
    use std::path::PathBuf;

    let mut ctx = super::tests::ctx_with_aliases(&[identity]);
    ctx.project_states
        .insert(identity.to_string(), hostile_project_state(identity));

    // `ctx_with_aliases` registers the project at this path; the Sessions tab
    // filters on equality with it and the Browse tab's breadcrumb is computed
    // relative to it, so both are derived from it rather than respelled.
    let project_path = ctx
        .config
        .projects
        .get(identity)
        .map(|project| project.path.clone())
        .unwrap_or_else(|| PathBuf::from("/nonexistent").join(identity));
    let planning_root = project_path.join(".planning");

    // The Sessions tab: a session whose `working_dir` matches the registered
    // project, or the tab's filter drops it and it renders "No active Claude
    // sessions" — the empty branch this fixture exists to leave.
    ctx.active_sessions = vec![crate::session_detector::ClaudeSession {
        pid: 4242,
        session_id: Some(Untrusted::from_untrusted_source(identity.to_string())),
        working_dir: project_path.clone(),
        start_time: Some(1),
        tty: None,
    }];

    // The Archive tab at its PhaseList and FileList depths: both read
    // `ctx.archive_cache`, which is keyed by the RAW milestone string because
    // that is what the navigation key is.
    ctx.archive_cache.insert(
        identity.to_string(),
        hostile_milestone_archive(identity),
    );

    let cache = ctx.view_cache.entry(identity.to_string()).or_default();
    cache.git_entries = vec![hostile_git_entry(identity)];
    cache.git_selected = 0;

    // The Backlog tab, in its COLLAPSED state here. The expanded split pane —
    // whose `Block::title` draws `item.dir_name` — is a separate probe state
    // built by `states_over_sub_views`, because `backlog_expanded` selects
    // between two different renders of two different values.
    cache.backlog_items = vec![BacklogItem {
        dir_name: Untrusted::from_untrusted_source(identity.to_string()),
        number: Untrusted::from_untrusted_source(identity.to_string()),
        description: Untrusted::from_untrusted_source(identity.to_string()),
        content: Some(Untrusted::from_untrusted_source(identity.to_string())),
        path: Some(planning_root.join("phases").join(identity)),
    }];
    cache.backlog_selected = 0;

    // The Archive tab at its MilestoneList depth (the default), which reads
    // `archive_milestones` rather than `archive_cache`.
    cache.archive_milestones = vec![Untrusted::from_untrusted_source(identity.to_string())];
    cache.archive_file_name = Some(Untrusted::from_untrusted_source(identity.to_string()));

    // The Browse tab. `browser_root` and `browser_current_dir` are BOTH needed:
    // the breadcrumb draws the path of the second relative to the first, and
    // `browser_entries` is what the list below it draws.
    cache.browser_depth = BrowserDepth::List;
    cache.browser_root = Some(planning_root.clone());
    cache.browser_current_dir = Some(planning_root.join(identity));
    cache.browser_entry_dir = Some(planning_root.clone());
    cache.browser_entries = vec![
        BrowserEntry {
            name: Untrusted::from_untrusted_source(identity.to_string()),
            path: planning_root.join(identity),
            is_dir: true,
        },
        BrowserEntry {
            name: Untrusted::from_untrusted_source(format!("{identity}.md")),
            path: planning_root.join(format!("{identity}.md")),
            is_dir: false,
        },
    ];
    cache.browser_selected = 0;
    cache.browser_file_name = Some(Untrusted::from_untrusted_source(format!("{identity}.md")));

    // The Defaults tab. It renders `entry.value` for every key of a parsed
    // `.planning/config.json`, and the string-valued keys are the ones a
    // fixture can influence — `mode`, `granularity`, `project_code`. If the
    // config is `None` the tab paints "No config loaded" and nothing else,
    // which is the empty branch.
    cache.defaults_config = Some(hostile_gsd_config(identity));

    // The Driver tab's run list. A `RunSummary` is read back out of a run's
    // committed `run.json`, so its `run_id`, `goal` and `gsd_command` are all
    // third-party text. Reaching the list does NOT require a live run — the
    // render only asks whether `driver_runs` is empty.
    cache.driver_runs = vec![crate::journal::RunSummary {
        run_id: identity.to_string(),
        started_at: "2026-08-27T00:00:00Z".to_string(),
        ended_at: None,
        goal: identity.to_string(),
        gsd_command: format!("/{identity}"),
        outcome: None,
    }];
    cache.driver_selected_run = 0;

    ctx.recompute_filtered_aliases();
    ctx.table_state.select(Some(0));
    ctx
}

/// The SAME `AppContext` as [`probe_ctx`] with every tab-body source emptied —
/// the chrome baseline against which per-tab arrival is measured (D-21-23).
///
/// # Why a baseline is needed at all, and what it caught
///
/// `DetailScreen` draws the registry key into its bordered block's title on
/// EVERY tab. So `clean_text.contains(clean)` is true for all fifteen probe
/// states whatever the tab body renders, and a per-tab arrival record built on
/// it would be a table of fifteen `true`s that stays green when a cache is
/// emptied. **That was measured, not reasoned about**: with
/// `probe_ctx`'s `backlog_items` cleared and nothing else changed, the naive
/// containment check still reported the Backlog tab as arriving.
///
/// The alias is deliberately left registered here, because the chrome that
/// draws it is exactly what this baseline is measuring. Everything a TAB reads
/// is emptied: the project state, the sessions, the archive cache and the whole
/// `ProjectViewCache`.
///
/// Measured chrome counts at the time this landed, so a future reader can see
/// the size of the effect rather than take it on trust: 1 occurrence for the
/// Backlog, GitHistory, Pipeline, Queue, Sessions, Archive, Defaults and Browse
/// tabs; 2 for PhaseList, RoadmapViz, Driver and the three states whose
/// breadcrumb draws a depth-derived milestone. Against populated counts of 2 to
/// 7. **These numbers are NOT pinned** — the assertion compares against the
/// baseline it measures on the same run, so a chrome change moves both sides.
fn chrome_ctx(identity: &str) -> AppContext {
    let mut ctx = super::tests::ctx_with_aliases(&[identity]);
    ctx.recompute_filtered_aliases();
    ctx.table_state.select(Some(0));
    ctx
}

/// One milestone archive whose every name carries `identity`.
///
/// A top-level file AND a phase (with its own file), because the Archive tab
/// renders those two through different code paths at different depths: the
/// `PhaseList` depth draws `top_level_files[..].name` and
/// `phases[..].display_name`, the `FileList` depth draws
/// `phases[..].files[..].name`.
fn hostile_milestone_archive(identity: &str) -> crate::archive::MilestoneArchive {
    use crate::archive::{ArchiveFile, MilestoneArchive, PhaseArchive};
    use crate::text::Untrusted;
    use std::path::PathBuf;

    let file = |name: &str| ArchiveFile {
        name: Untrusted::from_untrusted_source(name.to_string()),
        path: PathBuf::from("/nonexistent").join(name),
    };

    MilestoneArchive {
        version: identity.to_string(),
        top_level_files: vec![file(identity)],
        phases: vec![PhaseArchive {
            number: 1,
            name: Untrusted::from_untrusted_source(identity.to_string()),
            display_name: Untrusted::from_untrusted_source(format!("Phase 01: {identity}")),
            files: vec![file(identity)],
        }],
    }
}

/// A parsed `.planning/config.json` whose string-valued keys carry `identity`.
///
/// The Defaults tab draws `entry.value` for every key it knows about. Most of
/// those values are booleans and numbers, which a fixture cannot make hostile;
/// the three below are the free-form strings a project's own config supplies,
/// and they are what makes the tab render identity at all.
fn hostile_gsd_config(identity: &str) -> crate::state_reader::config_json::GsdConfig {
    use crate::state_reader::config_json::GsdConfig;

    GsdConfig {
        mode: identity.to_string(),
        granularity: identity.to_string(),
        project_code: Some(identity.to_string()),
        ..Default::default()
    }
}

/// One `git log` row whose every field carries `identity`.
///
/// All four fields, not just `message`: `%h`, `%ad`, `%an` and `%s` are four
/// `splitn` slices of one line of a third-party repository's `git log` output,
/// and the render draws all four into the same `ListItem`. A fixture that put
/// the identity in only one of them would leave the other three's render
/// unasserted while looking like coverage.
fn hostile_git_entry(identity: &str) -> crate::state_reader::git_ops::GitLogEntry {
    use crate::state_reader::git_ops::GitLogEntry;
    use crate::text::Untrusted;
    let field = || Untrusted::from_untrusted_source(identity.to_string());
    GitLogEntry {
        hash: field(),
        date: field(),
        author: field(),
        message: field(),
    }
}

/// Every detail sub-view, so a tab is a render state rather than a place the
/// probe never looked.
const ALL_SUB_VIEWS: [crate::app::DetailSubView; 11] = {
    use crate::app::DetailSubView::*;
    [
        PhaseList, RoadmapViz, Backlog, GitHistory, Pipeline, Queue, Sessions, Archive, Defaults,
        Browse, Driver,
    ]
};

/// **Per-state arrival for `DetailScreen`, RECORDED rather than assumed**
/// (D-21-23).
///
/// A populated cache the render path never reads makes every assertion about
/// that tab pass by silence — the tab renders nothing of the identity, so
/// assertion 2 is skipped (it is gated on arrival) and assertion 3 is trivially
/// satisfied. That is the sharpest failure shape this phase has, one level up
/// from where round 8 found it, and the only defence is to state per state
/// whether the clean identity ARRIVED and check it.
///
/// The `bool` is the claim; the string is the reason a reader inherits. For a
/// `false` row the reason must say WHICH of the two causes applies — the
/// fixture does not reach that render, or the tab genuinely draws no identity —
/// and how that was determined, because those two are fixed in different places
/// and confusing them sends the reader to the wrong file.
///
/// The set equality runs in both directions: an unexpected arrival is reported
/// too, since a tab that starts drawing identity is a tab whose row must be
/// re-read rather than a happy accident.
const DETAIL_TAB_ARRIVAL: &[(&str, bool, &str)] = &[
    (
        "PhaseList tab",
        true,
        "Draws the phase number, name and description out of `hostile_project_state`'s \
         `phases`, plus the status and milestone.",
    ),
    (
        "RoadmapViz tab",
        true,
        "Draws the same `RoadmapPhase` names through `ui::roadmap_widget`.",
    ),
    (
        "Backlog tab",
        true,
        "Draws `backlog_items[..].number` and `.description` — populated by 21-25 T2; \
         before that this tab rendered `No backlog items found.`",
    ),
    (
        "GitHistory tab",
        true,
        "Draws all four `GitLogEntry` fields — populated by 21-23.",
    ),
    (
        "Pipeline tab",
        true,
        "Draws the current phase name, status and pause context.",
    ),
    (
        "Queue tab",
        true,
        "Draws `queued_actions[..].command` from `.planning/queue.md`.",
    ),
    (
        "Sessions tab",
        true,
        "Draws the session id of a `ClaudeSession` whose `working_dir` matches the \
         registered project — populated by 21-25 T2; before that the tab's filter \
         admitted nothing and it rendered `No active Claude sessions`.",
    ),
    (
        "Archive tab",
        true,
        "At its default `MilestoneList` depth, draws `archive_milestones` — populated \
         by 21-25 T2; before that it rendered `No archived milestones found.`",
    ),
    (
        "Defaults tab",
        true,
        "Draws `entry.value` for every key of `defaults_config` — populated by 21-25 T2; \
         before that it rendered `No config loaded`. Only the string-valued keys can \
         carry identity; the booleans and numbers cannot, which is why the fixture sets \
         `mode`, `granularity` and `project_code`.",
    ),
    (
        "Browse tab",
        true,
        "Draws the breadcrumb path relative to `.planning/` and `browser_entries[..].name` \
         — populated by 21-25 T2; before that it rendered `(empty directory)` and an \
         empty breadcrumb.",
    ),
    (
        "Driver tab",
        true,
        "Draws the run list row and run header built from `driver_runs[0]`'s `run_id`, \
         `goal` and `gsd_command` — populated by 21-25 T2. Before that the tab rendered \
         `no_runs_lines`, which was the ONE already-composed site on this path, and the \
         four half-escaped ones below it were exercised by nothing.",
    ),
    (
        "Backlog tab, expanded",
        true,
        "The split pane: `Block::title` draws `item.dir_name` and the `Paragraph` below \
         draws `item.content`. Neither value is drawn at all in the collapsed state.",
    ),
    (
        "Archive tab, phase list",
        true,
        "Draws `archive_cache[..].top_level_files[..].name` and `phases[..].display_name`, \
         plus the milestone in the breadcrumb.",
    ),
    (
        "Archive tab, file list",
        true,
        "Draws `phases[0].files[..].name`, plus the milestone and phase display name in \
         the breadcrumb.",
    ),
    (
        "Browse tab, file view",
        true,
        "Draws `browser_file_name` in the breadcrumb and `browser_file_content` through \
         `archive::render_markdown_lines`.",
    ),
];

/// How many times `clean` appears in each `DetailScreen` state rendered with
/// every tab-body source emptied — the chrome contribution, MEASURED on this
/// run rather than pinned as a number somebody wrote down.
///
/// `None` for every screen but `DetailScreen`: the per-state arrival equality
/// is scoped to that one, and building fifteen extra renders for screens the
/// equality does not consult would cost time for nothing.
fn detail_chrome_baseline(
    type_name: &str,
    clean: &str,
) -> Option<BTreeMap<String, usize>> {
    if type_name != "DetailScreen" {
        return None;
    }
    let states = states_over_sub_views_with(clean, chrome_ctx, &|identity, _ctx| {
        Box::new(super::detail::DetailScreen::new(identity.to_string()))
    });
    Some(
        states
            .iter()
            .map(|state| {
                let text = render_to_text(state.screen.as_ref(), &state.ctx);
                (state.label.clone(), text.matches(clean).count())
            })
            .collect(),
    )
}

/// Which of [`DETAIL_TAB_ARRIVAL`]'s states claim the identity arrives.
fn detail_tabs_expected_to_arrive() -> std::collections::BTreeSet<String> {
    DETAIL_TAB_ARRIVAL
        .iter()
        .filter(|(_, arrives, _)| *arrives)
        .map(|(label, _, _)| (*label).to_string())
        .collect()
}

fn sub_view_label(view: &crate::app::DetailSubView) -> &'static str {
    use crate::app::DetailSubView::*;
    match view {
        PhaseList => "PhaseList tab",
        RoadmapViz => "RoadmapViz tab",
        Backlog => "Backlog tab",
        GitHistory => "GitHistory tab",
        Pipeline => "Pipeline tab",
        Queue => "Queue tab",
        Sessions => "Sessions tab",
        Archive => "Archive tab",
        Defaults => "Defaults tab",
        Browse => "Browse tab",
        Driver => "Driver tab",
    }
}

/// One state per detail sub-view, for every screen whose body is a detail view.
///
/// `EnqueueScreen`, `DriverInjectScreen`, `DriverStartScreen` and
/// `QueueDeleteConfirmScreen` all paint their body with
/// `DetailScreen::render_main_only`, so each of them renders whatever the active
/// tab renders. Sharing this helper is what stops the probe covering the detail
/// body for one of them and not the others.
fn states_over_sub_views(
    identity: &str,
    build: &dyn Fn(&str, &mut AppContext) -> Box<dyn Screen>,
) -> Vec<ProbeState> {
    states_over_sub_views_with(identity, probe_ctx, build)
}

/// [`states_over_sub_views`] with the context builder made a parameter, so the
/// SAME fifteen states can be built against [`chrome_ctx`] to measure the
/// baseline the per-tab arrival record is compared against.
///
/// Parameterised rather than duplicated: a second copy of this walk could
/// silently drift from the one the live probe uses, and a baseline measured
/// through a different set of states is a baseline that answers a different
/// question.
fn states_over_sub_views_with(
    identity: &str,
    make_ctx: fn(&str) -> AppContext,
    build: &dyn Fn(&str, &mut AppContext) -> Box<dyn Screen>,
) -> Vec<ProbeState> {
    let mut states: Vec<ProbeState> = ALL_SUB_VIEWS
        .iter()
        .map(|view| {
            let mut ctx = make_ctx(identity);
            ctx.detail_sub_view_per_project
                .insert(identity.to_string(), view.clone());
            let screen = build(identity, &mut ctx);
            ProbeState {
                label: sub_view_label(view).to_string(),
                ctx,
                screen,
            }
        })
        .collect();

    // A TAB IS NOT ONE PICTURE EITHER (21-25). Three of the eleven dispatch on
    // a second field to a genuinely different render of a genuinely different
    // value, and probing only the default value of that field is the same hole
    // one level down from the one this fixture just closed.
    for (label, arrange) in DETAIL_SUB_STATES {
        let mut ctx = make_ctx(identity);
        arrange(identity, &mut ctx);
        let screen = build(identity, &mut ctx);
        states.push(ProbeState {
            label: (*label).to_string(),
            ctx,
            screen,
        });
    }

    states
}

/// How one extra render state within a sub-view is arranged.
type SubStateArrange = fn(&str, &mut AppContext);

/// The extra within-tab states, each named after the field it dispatches on.
///
/// * **Backlog, expanded** — `backlog_expanded` selects between a full-height
///   list and a split pane whose `Block::title` draws `item.dir_name` and whose
///   `Paragraph` draws `item.content`. Neither of those two values is drawn at
///   all in the collapsed state.
/// * **Archive, phase list / file list** — `archive_depth` selects between
///   three different renders of three different names:
///   `archive_milestones` (MilestoneList), `top_level_files[..].name` plus
///   `phases[..].display_name` (PhaseList), and `phases[..].files[..].name`
///   (FileList).
/// * **Browse, file view** — `browser_depth` selects between the entry list and
///   the file view, and only the file view draws `browser_file_name`.
const DETAIL_SUB_STATES: &[(&str, SubStateArrange)] = &[
    ("Backlog tab, expanded", |identity, ctx| {
        ctx.detail_sub_view_per_project
            .insert(identity.to_string(), crate::app::DetailSubView::Backlog);
        let cache = ctx.view_cache.entry(identity.to_string()).or_default();
        cache.backlog_expanded = true;
    }),
    ("Archive tab, phase list", |identity, ctx| {
        ctx.detail_sub_view_per_project
            .insert(identity.to_string(), crate::app::DetailSubView::Archive);
        let cache = ctx.view_cache.entry(identity.to_string()).or_default();
        cache.archive_depth = crate::archive::ArchiveDepth::PhaseList {
            milestone: identity.to_string(),
        };
    }),
    ("Archive tab, file list", |identity, ctx| {
        ctx.detail_sub_view_per_project
            .insert(identity.to_string(), crate::app::DetailSubView::Archive);
        let cache = ctx.view_cache.entry(identity.to_string()).or_default();
        cache.archive_depth = crate::archive::ArchiveDepth::FileList {
            milestone: identity.to_string(),
            phase_idx: 0,
        };
    }),
    ("Browse tab, file view", |identity, ctx| {
        ctx.detail_sub_view_per_project
            .insert(identity.to_string(), crate::app::DetailSubView::Browse);
        let cache = ctx.view_cache.entry(identity.to_string()).or_default();
        cache.browser_depth = crate::browser::BrowserDepth::View;
        cache.browser_file_content = Some(format!("# {identity}\n"));
    }),
];

fn one_state(label: &str, ctx: AppContext, screen: Box<dyn Screen>) -> ProbeState {
    ProbeState {
        label: label.to_string(),
        ctx,
        screen,
    }
}

/// The constructor for each adjudicated implementor.
///
/// This is a mapping from a derived name to a way of building the thing — it is
/// **not** an enumeration, and it cannot become one: every disposition row must
/// have an arm here (`every_adjudicated_screen_has_a_probe_fixture`), and every
/// disposition row must have come from the walk. A screen with no arm is a red
/// test, not a silent skip.
fn fixture_for(type_name: &str) -> Option<Fixture> {
    match type_name {
        // The fixture puts the screen in the state its disposition names: the
        // project is registered, and the screen is the destructive confirm for
        // that exact key.
        //
        // The arrival assertion is load-bearing here, measured by emptying this
        // fixture (`ctx_with_aliases(&[])`, `DeleteConfirmScreen::new(String::new())`)
        // and re-running the probe:
        //
        // ```text
        // panicked at src/ui/screens/render_escape_guard.rs:511:21:
        // DeleteConfirmScreen (src/ui/screens/delete_confirm.rs) is adjudicated as rendering identity, but a clean identity handed to its fixture never reached the buffer. Either the disposition is wrong or the fixture does not put the screen in a state that renders it — and until this passes, every assertion below it would pass by silence.
        // ```
        "DeleteConfirmScreen" => Some(|identity| {
            vec![one_state(
                "confirm prompt",
                probe_ctx(identity),
                Box::new(super::delete_confirm::DeleteConfirmScreen::new(
                    identity.to_string(),
                )),
            )]
        }),

        // The dashboard, in both footer modes: the normal footer and the
        // filter/search footer, because the second one echoes the filter text
        // the operator typed.
        "NormalScreen" => Some(|identity| {
            let plain = probe_ctx(identity);
            let mut filtering = probe_ctx(identity);
            filtering.filter_text = identity.to_string();
            filtering.recompute_filtered_aliases();
            vec![
                one_state(
                    "dashboard",
                    plain,
                    Box::new(super::normal::NormalScreen::new()),
                ),
                // `searching: true` is not decoration. `render_footer`
                // dispatches on that field, so a state that sets `filter_text`
                // without it renders the NORMAL footer and never looks at the
                // filter at all — the assertion would have passed by silence.
                // It did, until this line was added; the search footer was
                // drawing `ctx.filter_text` raw the whole time.
                one_state(
                    "dashboard with filter footer",
                    filtering,
                    Box::new(super::normal::NormalScreen { searching: true }),
                ),
            ]
        }),

        // A pure overlay over authored text. Its disposition is CHECKED: the
        // fixture registers the identity and puts a hostile project state behind
        // it, and the probe asserts the clean stem never reaches the buffer.
        "HelpScreen" => Some(|identity| {
            vec![one_state(
                "help overlay",
                probe_ctx(identity),
                Box::new(super::help::HelpScreen::new()),
            )]
        }),

        "DetailScreen" => Some(|identity| {
            states_over_sub_views(identity, &|identity, _ctx| {
                Box::new(super::detail::DetailScreen::new(identity.to_string()))
            })
        }),

        "EnqueueScreen" => Some(|identity| {
            states_over_sub_views(identity, &|identity, ctx| {
                ctx.input_buffer = identity.to_string();
                Box::new(super::enqueue::EnqueueScreen::new(identity.to_string()))
            })
        }),

        "QueueDeleteConfirmScreen" => Some(|identity| {
            states_over_sub_views(identity, &|identity, _ctx| {
                Box::new(
                    super::queue_delete_confirm::QueueDeleteConfirmScreen::new(
                        identity.to_string(),
                        0,
                        identity.to_string(),
                    ),
                )
            })
        }),

        "DriverInjectScreen" => Some(|identity| {
            states_over_sub_views(identity, &|identity, ctx| {
                ctx.input_buffer = identity.to_string();
                Box::new(super::driver_inject::DriverInjectScreen::new(
                    identity.to_string(),
                    identity.to_string(),
                ))
            })
        }),

        // Both wizard steps. Step B is reached by driving the real state
        // machine — `Enter` on a slash-prefixed command — rather than by
        // constructing the field directly, so what the probe renders is a state
        // the key handler can actually produce.
        "DriverStartScreen" => Some(|identity| {
            let mut states = states_over_sub_views(identity, &|identity, ctx| {
                ctx.input_buffer = identity.to_string();
                Box::new(super::driver_start::DriverStartScreen::new(
                    identity.to_string(),
                ))
            });
            let mut goal_ctx = probe_ctx(identity);
            let mut goal_screen =
                super::driver_start::DriverStartScreen::new(identity.to_string());
            goal_ctx.input_buffer = format!("/{identity}");
            goal_screen.handle_key(
                crossterm::event::KeyCode::Enter,
                crossterm::event::KeyModifiers::NONE,
                &mut goal_ctx,
            );
            goal_ctx.input_buffer = identity.to_string();
            states.push(one_state(
                "goal step, showing the committed command",
                goal_ctx,
                Box::new(goal_screen),
            ));
            states
        }),

        // All four prompts: start (with and without a goal), stop, and both
        // directions of the opt-in toggle. Each names the alias.
        "DriverConfirmScreen" => Some(|identity| {
            use super::driver_confirm::{DriverAction, DriverConfirmScreen};
            let opted_in = {
                let mut ctx = probe_ctx(identity);
                crate::registry::record_opt_in(&mut ctx.config, identity).ok();
                ctx
            };
            vec![
                one_state(
                    "start prompt with a goal",
                    probe_ctx(identity),
                    Box::new(DriverConfirmScreen::new_start(
                        identity.to_string(),
                        format!("/{identity}"),
                        Some(identity.to_string()),
                    )),
                ),
                one_state(
                    "stop prompt",
                    probe_ctx(identity),
                    Box::new(DriverConfirmScreen::new(
                        identity.to_string(),
                        DriverAction::Stop,
                    )),
                ),
                one_state(
                    "opt-in grant prompt, with the disclosure",
                    probe_ctx(identity),
                    Box::new(DriverConfirmScreen::new(
                        identity.to_string(),
                        DriverAction::ToggleOptIn,
                    )),
                ),
                one_state(
                    "opt-in withdrawal prompt",
                    opted_in,
                    Box::new(DriverConfirmScreen::new(
                        identity.to_string(),
                        DriverAction::ToggleOptIn,
                    )),
                ),
            ]
        }),

        // The alias field, with and without a refusal beside it. The refusal
        // path is the one that carries an identity this build did not author:
        // `AliasRefusal`'s own `Display` embeds the alias it refused.
        "AddProjectScreen" => Some(|identity| {
            let mut refused = probe_ctx(identity);
            refused.input_buffer = identity.to_string();
            refused.error_message = Some(
                crate::registry::Alias::new(identity)
                    .map(|_| String::new())
                    .unwrap_or_else(|refusal| refusal.to_string()),
            );
            let mut typing = probe_ctx(identity);
            typing.input_buffer = identity.to_string();
            vec![
                one_state(
                    "alias field",
                    typing,
                    Box::new(super::add_project::AddProjectScreen::new_alias()),
                ),
                one_state(
                    "alias field with the refusal echoed beside it",
                    refused,
                    Box::new(super::add_project::AddProjectScreen::new_alias()),
                ),
            ]
        }),

        "CreateProjectScreen" => Some(|identity| {
            let mut typing = probe_ctx(identity);
            typing.input_buffer = identity.to_string();
            let mut errored = probe_ctx(identity);
            errored.input_buffer = identity.to_string();
            errored.error_message = Some(identity.to_string());
            vec![
                one_state(
                    "name field",
                    typing,
                    Box::new(super::create_project::CreateProjectScreen::new_name()),
                ),
                one_state(
                    "name field with an error echoed beside it",
                    errored,
                    Box::new(super::create_project::CreateProjectScreen::new_name()),
                ),
            ]
        }),

        _ => None,
    }
}

// ---------------------------------------------------------------------------
// The render harness
// ---------------------------------------------------------------------------

/// Wide and tall enough that nothing under probe is clipped or wrapped.
const PROBE_WIDTH: u16 = 200;
const PROBE_HEIGHT: u16 = 60;

/// Render `screen` through the real [`Screen::render`](super::Screen::render)
/// and join the resulting buffer's cell symbols, one line per terminal row.
fn render_to_text(screen: &dyn Screen, ctx: &AppContext) -> String {
    render_into_probe_buffer(|frame, area| screen.render(frame, area, ctx))
}

/// Draw through a `TestBackend` at the probe's dimensions and join the resulting
/// buffer's cell symbols, one line per terminal row.
///
/// Extracted so [`survives_a_rendered_buffer`] inspects cells by exactly the
/// same route the live probe does. A precondition that measured survivorship
/// through a *different* harness could disagree with the assertion it is meant
/// to protect, which is the shape of defect this module exists to remove.
fn render_into_probe_buffer(draw: impl FnOnce(&mut ratatui::Frame, ratatui::layout::Rect)) -> String {
    let mut terminal =
        Terminal::new(TestBackend::new(PROBE_WIDTH, PROBE_HEIGHT)).expect("TestBackend terminal");
    terminal
        .draw(|frame| {
            let area = frame.area();
            draw(frame, area);
        })
        .expect("draw the widget under probe");
    let buffer = terminal.backend().buffer().clone();
    (0..PROBE_HEIGHT)
        .map(|y| {
            (0..PROBE_WIDTH)
                .map(|x| {
                    buffer
                        .cell((x, y))
                        .map(|cell| cell.symbol())
                        .unwrap_or(" ")
                        .to_string()
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn invisible_chars(text: &str) -> Vec<char> {
    text.chars().filter(|c| is_invisible_formatting_char(*c)).collect()
}

/// Which widget family a survivorship measurement goes through.
///
/// The two are not interchangeable and that is the whole point: measured against
/// ratatui 0.30.2 in a scratch crate outside this tree, `Paragraph` DROPS
/// `U+202E`, `U+200B`, `U+00AD`, `U+2062`, `U+2065` and `U+FEFF` before a cell
/// exists, while `ListItem` preserves every one of them. Both preserve the tag
/// block. See `the_screen_renders_identity_escaped`'s doc for the full table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProbeSink {
    /// The family the live leak used, and the one that preserves most.
    ListItem,
    /// The family whose behaviour was wrongly generalised to the whole
    /// rendering stack for three rounds.
    Paragraph,
}

/// Whether any character of `value` that is in the invisible class SURVIVES all
/// the way to a rendered cell through `sink`.
///
/// **This is a property of the fixture's rendered BEHAVIOUR, not of an index**
/// (D-21-12). `TAG_PAIR: usize = 4` silently decides whether assertion 3 has
/// teeth: reorder [`LOOK_ALIKE_PAIRS`] so index 4 holds a pair whose hostile
/// member is dropped before a cell exists, and assertion 3 goes on passing while
/// asserting nothing. Pinning `TAG_PAIR == 4` would be the wrong instrument in
/// both directions — red on a harmless reorder, green on a harmful content
/// change. Rendering the fixture and asking whether the class arrives is red
/// exactly when the teeth are gone and green exactly when they are there.
fn survives_a_rendered_buffer(sink: ProbeSink, value: &str) -> bool {
    use ratatui::text::Line;
    use ratatui::widgets::{List, ListItem, Paragraph};

    let owned = value.to_string();
    let rendered = render_into_probe_buffer(|frame, area| match sink {
        ProbeSink::ListItem => {
            frame.render_widget(List::new(vec![ListItem::new(Line::from(owned))]), area)
        }
        ProbeSink::Paragraph => frame.render_widget(Paragraph::new(owned), area),
    });
    !invisible_chars(&rendered).is_empty()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// The census's permanent non-vacuity control.
    ///
    /// It drives the SAME [`census_offences`] the live assertion consumes, with
    /// two synthetic pairs — one derived set carrying a member the table lacks,
    /// one table carrying a member the tree lacks — and asserts each is reported
    /// with its own message.
    ///
    /// Without this, the live census's clean result would be indistinguishable
    /// from a comparison that stopped comparing, which is the exact shape of
    /// every failure this phase has found.
    #[test]
    fn the_census_reports_an_unadjudicated_screen_and_a_stale_row() {
        let derived: BTreeMap<String, String> = [
            ("Adjudicated".to_string(), "src/a.rs".to_string()),
            ("NobodyJudgedMe".to_string(), "src/b.rs".to_string()),
        ]
        .into_iter()
        .collect();
        let table: BTreeMap<String, String> = [
            ("Adjudicated".to_string(), "src/a.rs".to_string()),
            ("IOutlivedMySubject".to_string(), "src/c.rs".to_string()),
        ]
        .into_iter()
        .collect();

        let offences = census_offences(&derived, &table);

        assert_eq!(
            offences.len(),
            2,
            "the comparison must report both harms and only those two, got: {offences:#?}"
        );
        assert!(
            offences
                .iter()
                .any(|o| o.starts_with("UNADJUDICATED IMPLEMENTOR") && o.contains("NobodyJudgedMe")),
            "a derived member absent from the table is a screen nobody \
             adjudicated and must be reported as that, got: {offences:#?}"
        );
        assert!(
            offences
                .iter()
                .any(|o| o.starts_with("STALE ROW") && o.contains("IOutlivedMySubject")),
            "a table member absent from the tree is a stale row and must be \
             reported as that — the two harms are different and a message that \
             conflates them tells the reader to fix the wrong end, got: \
             {offences:#?}"
        );

        // The clean direction, so "reports something" is not mistaken for
        // "reports everything".
        assert!(
            census_offences(&derived, &derived).is_empty(),
            "a set compared against itself must report nothing"
        );
    }

    /// The teeth precondition's permanent non-vacuity control, in **both**
    /// directions and through **both** widget families.
    ///
    /// [`survives_a_rendered_buffer`] is a precondition, and a precondition that
    /// only ever answers `true` is indistinguishable from `fn f() -> bool { true }`.
    /// So this drives the SAME helper the live probe consumes and asserts it
    /// answers `false` where it must.
    ///
    /// **A note on how the `false` direction is built, because the obvious
    /// construction does not exist.** The plan for this task asked for an
    /// identity "made only of characters the measurement shows are dropped by
    /// every family under probe". No such identity exists for the invisible
    /// class: the per-widget measurement shows `ListItem` preserving every one
    /// of `U+202E`, `U+200B`, `U+00AD`, `U+2062`, `U+2065`, `U+FEFF` and
    /// `U+E0041`. The only code points dropped by every family are C0 controls,
    /// and those are `Cc` — outside the class this helper measures — so an
    /// identity built from them would answer `false` for two reasons at once and
    /// certify neither. The `false` direction is therefore delivered by the two
    /// mechanisms that actually make the precondition fire, both asserted here:
    ///
    /// 1. **The fixture carries no class member at all** — the shape a reorder
    ///    of `LOOK_ALIKE_PAIRS` toward all-clean pairs would produce.
    ///    [`clean_identity`] is all-ASCII, so nothing can arrive.
    /// 2. **The fixture's class members are DROPPED by the family under
    ///    measurement** — the shape the original LIMIT 4 was written about.
    ///    [`zero_width_only_identity`] answers `false` through
    ///    [`ProbeSink::Paragraph`] and `true` through [`ProbeSink::ListItem`],
    ///    which is one value giving two answers and is the strongest available
    ///    proof that the helper measures the RENDER rather than the string.
    ///
    /// Arm 2 doubles as this tree's own in-repo re-derivation of the per-widget
    /// correction: if a future ratatui made `ListItem` drop zero-width
    /// graphemes, or `Paragraph` preserve them, this test goes red and the
    /// STANDING obligation in `deferred-items.md` is what it points at.
    #[test]
    fn the_teeth_precondition_answers_false_when_the_class_cannot_reach_a_cell() {
        assert!(
            survives_a_rendered_buffer(ProbeSink::ListItem, &hostile_identity()),
            "the live fixture must have teeth, or the precondition guarding \
             assertion 3 is itself vacuous"
        );

        assert!(
            !survives_a_rendered_buffer(ProbeSink::ListItem, &clean_identity()),
            "an all-ASCII identity carries no invisible-class character, so \
             nothing can arrive and the precondition must answer false — a \
             helper that answered true here would be reporting on something \
             other than the fixture"
        );

        let zero_width = zero_width_only_identity();
        assert!(
            !survives_a_rendered_buffer(ProbeSink::Paragraph, &zero_width),
            "{zero_width:?} carries only class members a `Paragraph` DROPS \
             before a cell exists, so the precondition must answer false \
             through that family. If this went true, `Paragraph` has started \
             preserving zero-width graphemes and the STANDING ratatui entry in \
             deferred-items.md needs re-measuring."
        );
        assert!(
            survives_a_rendered_buffer(ProbeSink::ListItem, &zero_width),
            "{zero_width:?} is the SAME value, and through a `ListItem` every \
             one of its class members reaches a cell. One value, two answers: \
             that is what proves this helper measures the render rather than \
             the string, and it is the per-widget correction re-derived inside \
             this tree. If this went false, `ListItem` has started dropping \
             zero-width graphemes and LIMIT 4 must be re-stated."
        );
    }

    /// The live census: the walk's derived set equals the disposition table,
    /// both ways.
    #[test]
    fn the_screen_census_matches_the_tree() {
        let derived = screen_implementors_from_source();
        let table = disposition_table();

        let offences = census_offences(&derived, &table);
        assert!(
            offences.is_empty(),
            "the derived render surface and the disposition table disagree:\n\n{}",
            offences.join("\n\n")
        );

        assert_eq!(
            derived, table,
            "the walk's set and the table's set must be equal in both directions"
        );
    }

    /// Every adjudicated implementor can actually be built and rendered.
    #[test]
    fn every_adjudicated_screen_has_a_probe_fixture() {
        let missing: Vec<&str> = SCREEN_IDENTITY_DISPOSITIONS
            .iter()
            .filter(|(name, _, _, _)| fixture_for(name).is_none())
            .map(|(name, _, _, _)| *name)
            .collect();
        assert!(
            missing.is_empty(),
            "these adjudicated screens have no probe fixture, so their \
             disposition is a claim rather than a check: {missing:?}"
        );
    }

    /// The behavioural probe: each disposition is CHECKED against a real render.
    ///
    /// # Why assertion 2 is "the escaped form is PRESENT" and not "the raw form is ABSENT"
    ///
    /// **Measured here, in this tree, rather than inherited** — the probe was
    /// run against the UNESCAPED `delete_confirm.rs` at HEAD with a scratch
    /// reporter and it printed:
    ///
    /// ```text
    /// HOSTILE INPUT   : "demo\u{e0041}r\u{ad}un"
    /// ESCAPED FORM    : "demoU+E0041rU+00ADun"
    /// RENDERED ROW    : "Remove \"demo\u{e0041}run\"? This only unregisters it — project files are not deleted. [y/n]"
    /// INVISIBLE CHARS : ['\u{e0041}']
    /// CONTAINS RAW    : false
    /// CONTAINS ESCAPED: false
    /// CONTAINS CLEAN  : false
    /// ```
    ///
    /// Two facts, both load-bearing, and both contradicting what round 7's
    /// review and verification pass 8 each asserted without measuring:
    ///
    /// 1. **A `Paragraph` drops zero-width graphemes before a cell exists.**
    ///    `U+00AD` is simply gone from the rendered row above. So at a
    ///    `Paragraph` site the TUI does not *reorder* a hostile key — it
    ///    silently *deletes* bytes, and the legacy key renders as a DIFFERENT
    ///    string that can collide with a real project of that name.
    /// 2. **The tag block SURVIVES.** `U+E0041` — the LLM ASCII-smuggling
    ///    carrier — reached a terminal cell intact. That is the one class that
    ///    arrives whole through every family, and it is what assertion 3 catches.
    ///
    /// # CORRECTED 2026-08-27 (21-23): fact 1 is a `Paragraph` property, NOT a `Buffer` property
    ///
    /// **The sentence this doc used to carry, verbatim:** *"ratatui 0.30's
    /// `Buffer` DROPS zero-width graphemes before a cell exists."* That is a
    /// generalisation of one sink's behaviour to the whole rendering stack, and
    /// it is false. The drop happens on the `Paragraph` path; `Block::title`
    /// and `ListItem` reach a cell by a different
    /// route and PRESERVE the class. Three artefacts in this phase asserted the
    /// general form without measuring it — `21-21`'s SUMMARY, the round-8
    /// review, and verification pass 9's Judgment 3, which re-derived only the
    /// `Paragraph` column — and each time it hid the two families where this
    /// tree's live leaks were.
    ///
    /// Re-derived for `21-23` in a throwaway crate OUTSIDE this repository
    /// depending only on `ratatui = "0.30"` (resolved 0.30.2, matching this
    /// tree's `Cargo.lock`), rendering `a<CP>b` through four sinks into a
    /// `TestBackend` buffer. Verbatim:
    ///
    /// ```text
    /// cp          width | Paragraph   Block::title  ListItem    Paragraph-in-Block
    /// U+202E     2 | dropped     SURVIVES      SURVIVES    dropped
    /// U+200B     2 | dropped     SURVIVES      SURVIVES    dropped
    /// U+00AD     2 | dropped     SURVIVES      SURVIVES    dropped
    /// U+2062     2 | dropped     SURVIVES      SURVIVES    dropped
    /// U+2065     2 | dropped     SURVIVES      SURVIVES    dropped
    /// U+FEFF     2 | dropped     SURVIVES      SURVIVES    dropped
    /// U+E0041    2 | SURVIVES    SURVIVES      SURVIVES    SURVIVES
    /// ```
    ///
    /// What this costs the probe is stated in LIMIT 4 of the module doc, and
    /// what it BUYS is the raw-absence assertion LIMIT 4 declined: non-vacuous
    /// for the two preserving families, and present below as assertion 4.
    ///
    /// # The RED assertion 4 was observed at, verbatim
    ///
    /// Against the tree with EXACTLY ONE `shown()` reverted — the git-history
    /// `List` row's `message` span — and nothing else changed:
    ///
    /// ```text
    /// thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (432171) panicked at src/ui/screens/render_escape_guard.rs:1399:29:
    /// DetailScreen (src/ui/screens/detail.rs) [GitHistory tab] rendered the RAW hostile identity "demo\u{e0041}r\u{ad}un" into the terminal buffer. What the operator reads is therefore a value the terminal may reorder, hide characters in, or render as a different string entirely — this is Trojan Source (CVE-2021-42574) in a cell. Route what a human READS through crate::text::render_for_terminal; the raw value belongs only in lookups, map keys, path segments, subprocess arguments and persistence.
    /// ```
    ///
    /// The site was restored immediately and `git status --porcelain` confirmed
    /// clean. Note that assertion 4 fires BEFORE assertion 3 here: the raw form
    /// arriving is the more specific finding, and reporting it first tells the
    /// reader which value leaked rather than only which characters did.
    ///
    /// What makes this probe non-vacuous is therefore assertion 1 plus
    /// assertion 2: the clean stem must ARRIVE (proving the screen renders
    /// identity at all — `CONTAINS CLEAN: false` above shows an unescaped
    /// render does not supply it by accident), and the ESCAPED form must then be
    /// what a hostile identity produces.
    ///
    /// # The RED this probe was observed at, verbatim, before any screen was edited
    ///
    /// ```text
    /// thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (1346161) panicked at src/ui/screens/render_escape_guard.rs:516:21:
    /// DeleteConfirmScreen (src/ui/screens/delete_confirm.rs) did not render "demoU+E0041rU+00ADun". Route what a human READS through crate::text::display_identity; the value used for lookups, map keys, path segments, comparisons and persistence stays RAW.
    /// ```
    ///
    /// A guard never observed red is not certified. This one was.
    #[test]
    fn the_screen_renders_identity_escaped() {
        let clean = clean_identity();
        let hostile = hostile_identity();
        let escaped = display_identity(&hostile);

        // 0. THE TEETH, checked before any screen is probed (D-21-12, pass-9
        //    Warning). Assertion 3 asserts that no invisible-class character
        //    reaches a cell. That is only a claim about anything if the hostile
        //    fixture CAN put one there. `TAG_PAIR: usize = 4` is a hand-
        //    maintained index into `LOOK_ALIKE_PAIRS`, and a reorder that made
        //    index 4 a zero-width pair would leave assertion 3 green and empty.
        //    Deciding this by RENDERING rather than by pinning the index is what
        //    makes it red on a harmful content change and quiet on a harmless
        //    reorder.
        assert!(
            survives_a_rendered_buffer(ProbeSink::ListItem, &hostile),
            "assertion 3 has lost its teeth: no character of the hostile \
             fixture {hostile:?} both satisfies `text::is_invisible_formatting_char` \
             AND survives into a rendered cell, so `invisible_chars` can only \
             ever return empty and assertion 3 passes vacuously while staying \
             green. Look at `LOOK_ALIKE_PAIRS` in src/test_support.rs and at \
             `TAG_PAIR`/`SOFT_HYPHEN_PAIR` above: a reorder that put a pair \
             whose hostile member is dropped before a cell exists at index \
             {TAG_PAIR} is enough to cause this."
        );

        for (name, path, disposition, _reason) in SCREEN_IDENTITY_DISPOSITIONS {
            let Some(build) = fixture_for(name) else {
                panic!("{name} ({path}) has no probe fixture");
            };

            let clean_states = build(&clean);
            let hostile_states = build(&hostile);
            assert_eq!(
                clean_states.len(),
                hostile_states.len(),
                "{name} ({path}): the fixture must build the same states for a \
                 clean identity as for a hostile one, or the two renders being \
                 compared are not the same picture"
            );
            assert!(
                !clean_states.is_empty(),
                "{name} ({path}): the fixture built no render states, so nothing \
                 about this screen was checked"
            );

            let mut arrived_anywhere = false;
            let mut drew_anything = false;
            // The per-state arrival record (D-21-23), measured against the
            // CHROME BASELINE rather than by containment — see `chrome_ctx`
            // for why containment cannot tell a tab body from a block title.
            let baseline = detail_chrome_baseline(name, &clean);
            let mut arrived_labels: std::collections::BTreeSet<String> =
                std::collections::BTreeSet::new();
            let mut all_labels: std::collections::BTreeSet<String> =
                std::collections::BTreeSet::new();

            for (clean_state, hostile_state) in clean_states.iter().zip(hostile_states.iter()) {
                let where_ = format!("{name} ({path}) [{}]", clean_state.label);
                let clean_text = render_to_text(clean_state.screen.as_ref(), &clean_state.ctx);
                let hostile_text =
                    render_to_text(hostile_state.screen.as_ref(), &hostile_state.ctx);

                let arrived = clean_text.contains(clean.as_str());
                arrived_anywhere |= arrived;
                drew_anything |= clean_text.chars().any(|c| !c.is_whitespace());
                all_labels.insert(clean_state.label.clone());
                // ARRIVAL IN THE TAB BODY: strictly more occurrences of the
                // clean stem than the same state renders with every tab-body
                // source emptied. Plain containment would count the block
                // title, which draws the registry key on every tab.
                if let Some(baseline) = baseline.as_ref() {
                    let occurrences = clean_text.matches(clean.as_str()).count();
                    let chrome = baseline.get(&clean_state.label).copied().unwrap_or(0);
                    if occurrences > chrome {
                        arrived_labels.insert(clean_state.label.clone());
                    }
                } else if arrived {
                    arrived_labels.insert(clean_state.label.clone());
                }

                match *disposition {
                    RENDERS_IDENTITY => {
                        // 2. PER STATE, and gated on arrival: wherever the clean
                        //    identity DID reach the buffer, the hostile one must
                        //    reach it escaped. A state that draws no identity is
                        //    not asserted about here — it is covered by the
                        //    whole-screen arrival assertion below and by 3.
                        if arrived {
                            assert!(
                                hostile_text.contains(escaped.as_str()),
                                "{where_} rendered a clean identity but did not \
                                 render {escaped:?} for the hostile one. Route \
                                 what a human READS through \
                                 crate::text::display_identity; the value used \
                                 for lookups, map keys, path segments, \
                                 comparisons and persistence stays RAW."
                            );

                            // 4. THE ASSERTION LIMIT 4 DECLINED, reinstated
                            //    (D-21-11). It was declined on the premise that
                            //    ratatui deletes zero-width graphemes before a
                            //    cell exists, which is TRUE of `Paragraph` and
                            //    FALSE of `Block::title` and `ListItem` — the
                            //    two families this tree's live leaks used. Gated
                            //    on `arrived` exactly as assertion 2 is, so a
                            //    state that draws no identity cannot pass by
                            //    silence.
                            //
                            //    Its power is per family and LIMIT 4 says so: at
                            //    a `Paragraph` site it passes for a reason
                            //    unrelated to the code, and assertions 2 and 3
                            //    are what bound that.
                            assert!(
                                !hostile_text.contains(hostile.as_str()),
                                "{where_} rendered the RAW hostile identity \
                                 {hostile:?} into the terminal buffer. What the \
                                 operator reads is therefore a value the \
                                 terminal may reorder, hide characters in, or \
                                 render as a different string entirely — this is \
                                 Trojan Source (CVE-2021-42574) in a cell. Route \
                                 what a human READS through \
                                 crate::text::render_for_terminal; the raw value \
                                 belongs only in lookups, map keys, path \
                                 segments, subprocess arguments and persistence."
                            );
                        }
                    }
                    RENDERS_NO_IDENTITY => {
                        // The disposition is CHECKED, not claimed: if the screen
                        // really draws no identity, the clean stem — which is
                        // all-ASCII and cannot be dropped — cannot be in its
                        // buffer.
                        assert!(
                            !arrived,
                            "{where_} is adjudicated as rendering no identity, \
                             but it drew one. The row is wrong: re-read the \
                             render, re-state the adjudication, and escape what \
                             a human reads."
                        );
                    }
                    other => panic!("{where_} carries an unknown disposition {other:?}"),
                }

                // 3. Applies to EVERY state of EVERY implementor whatever its
                //    disposition: the tag block U+E0000-U+E007F reaches a
                //    terminal cell INTACT (measured — see this test's doc), and
                //    a cell holding it is a cell the operator cannot see
                //    (T-21-21-02). This is the assertion that forces BREADTH:
                //    it goes red for whichever value a state renders raw,
                //    without the probe having to know which value that is.
                assert!(
                    invisible_chars(&hostile_text).is_empty(),
                    "{where_} rendered {:?} into the terminal buffer. Those \
                     characters render as nothing, so what the operator reads is \
                     not what the value is.",
                    invisible_chars(&hostile_text)
                );
            }

            // 1b. ARRIVAL, PER STATE, for `DetailScreen` (D-21-23). The
            //     whole-screen arrival check below passes as soon as ONE of
            //     fifteen states draws the identity, so it cannot tell a
            //     fixture that reaches every tab from one that reaches one. A
            //     populated cache the render path never reads would otherwise
            //     make every assertion about that tab pass by silence, which is
            //     the vacuity hazard the ROADMAP names as this phase's sharpest
            //     risk.
            //
            //     Scoped to `DetailScreen` deliberately: the four screens that
            //     paint their body with `render_main_only` render the same tabs
            //     PLUS their own footer, so the identity arrives in all of their
            //     states for a reason that says nothing about the tab.
            if *name == "DetailScreen" {
                let expected = detail_tabs_expected_to_arrive();
                let missing: Vec<&String> = expected.difference(&arrived_labels).collect();
                let unexpected: Vec<&String> = arrived_labels.difference(&expected).collect();
                let unlisted: Vec<&String> = all_labels
                    .iter()
                    .filter(|label| {
                        !DETAIL_TAB_ARRIVAL
                            .iter()
                            .any(|(recorded, _, _)| *recorded == label.as_str())
                    })
                    .collect();

                assert!(
                    unlisted.is_empty(),
                    "{name} ({path}) was probed in states DETAIL_TAB_ARRIVAL does not \
                     record: {unlisted:?}. A state nobody adjudicated is a state whose \
                     arrival nobody checked; add a row saying whether the identity is \
                     expected to reach that render and why."
                );
                assert!(
                    missing.is_empty(),
                    "{name} ({path}): DETAIL_TAB_ARRIVAL claims the clean identity \
                     reaches {missing:?}, and it did not. TWO CAUSES, in order of \
                     likelihood. (1) The fixture does not reach that render — the cache \
                     `probe_ctx` populates is not the one the tab reads, or a second \
                     field gates the branch (`backlog_expanded`, `archive_depth`, \
                     `browser_depth`, `searching`). Tell this apart by rendering the \
                     state and looking for the tab's EMPTY-branch string in the buffer; \
                     if it is there, the fixture is the problem. (2) The tab genuinely \
                     draws no identity — then the row is wrong and must be flipped to \
                     `false` with that reason. Do NOT delete the row: a populated cache \
                     whose render is never reached is a fixture that proved nothing, and \
                     it must be reported rather than counted as coverage. Arrived: \
                     {arrived_labels:?}"
                );
                assert!(
                    unexpected.is_empty(),
                    "{name} ({path}): the clean identity reached {unexpected:?}, which \
                     DETAIL_TAB_ARRIVAL records as drawing none. The row was written \
                     about a render that has since changed; re-read it, say which value \
                     the tab now draws and where its bytes come from, and flip the row \
                     to `true`."
                );
            }

            // 1. ARRIVAL, for the screen as a whole. A screen that rendered
            //    nothing, or that was built in states showing no identity, fails
            //    HERE rather than passing by silence.
            match *disposition {
                RENDERS_IDENTITY => assert!(
                    arrived_anywhere,
                    "{name} ({path}) is adjudicated as rendering identity, but a \
                     clean identity handed to its fixture never reached the \
                     buffer in ANY of its {} states. Either the disposition is \
                     wrong or the fixture does not put the screen in a state \
                     that renders it — and until this passes, every assertion \
                     above it passed by silence.",
                    clean_states.len()
                ),
                _ => assert!(
                    drew_anything,
                    "{name} ({path}) rendered a blank buffer in every state, so \
                     its absence assertions proved nothing"
                ),
            }
        }
    }
}
