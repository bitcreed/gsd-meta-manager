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
//! # CORRECTED 2026-08-27 (21-26): the adjudication is a COMPILE-TIME obligation now
//!
//! **The sentence this doc used to carry, verbatim, and it was measured false:**
//! *"[`screen_implementors_from_source`] walks `src/` recursively with
//! `std::fs::read_dir` and collects every implementation of the
//! [`Screen`](super::Screen) trait. It is a **filesystem walk, never a path
//! list**: a twelfth screen added tomorrow in a file this module has never heard
//! of is discovered without anybody editing anything here."*
//!
//! CR-05 measured that claim short in TWO source spellings at once. A
//! `macro_rules!`-generated implementor has no `impl` line whose type name any
//! line-oriented scan can extract, and an `impl` header wrapped across two
//! physical lines is not one line to match. With both in the tree the walk
//! reported ELEVEN implementors while thirteen existed, and reported it green.
//! A scan's completeness is bounded by source FORMATTING — one level below where
//! anybody was looking, and neither spelling appeared in the LIMITS block below.
//!
//! **What is true now, and what each mechanism's job is:**
//!
//! * [`RenderAdjudicated`](super::RenderAdjudicated) is a **sealed supertrait of
//!   [`Screen`](super::Screen)**, so an `impl Screen for X` where `X` carries no
//!   adjudication is `error[E0277]` and does not build. That is what makes
//!   adjudication mandatory, it is blind to formatting entirely because it is a
//!   property of the TYPE rather than of the text, and there is no spelling for
//!   it to be short of. The E0277 is quoted verbatim in that trait's doc,
//!   observed by planting an unadjudicated implementor in
//!   `src/driver/liveness.rs`.
//! * [`screen_implementors_from_source`] is a **second, weaker mechanism with a
//!   narrowed job**: it walks `src/` and derives the implementors it can name, so
//!   that an adjudicated screen with **no probe fixture** is reported. It no
//!   longer makes adjudication mandatory — the compiler does — and it no longer
//!   records what a screen draws — the screen does. Its own residual is stated in
//!   LIMIT 6.
//! * [`SCREEN_IDENTITY_DISPOSITIONS`] is now a **`(type name, path)` fixture
//!   map** and nothing more. Its disposition and reason columns are gone; those
//!   moved onto the screens, beside the `render` they describe.
//!   `the_screen_census_matches_the_tree` asserts the derived set and this map
//!   equal in BOTH directions, so a derived member with no row is a screen with
//!   no fixture and a row with no derived member is a stale row.
//! * `the_screen_renders_identity_escaped` then CHECKS each disposition rather
//!   than trusting it, by rendering the screen through the real
//!   [`Screen::render`](super::Screen::render) into a ratatui `Buffer` and
//!   inspecting the resulting cells. It reads the disposition off the
//!   CONSTRUCTED INSTANCE — `screen.disposition()` through `&dyn Screen` — so
//!   the disposition it checks and the disposition the screen declares cannot
//!   drift. The both-ways set equality could only ever catch that for
//!   *membership*; content was never checked until the disposition became one
//!   statement instead of two.
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
//!    **REWRITTEN AGAIN 2026-08-27 (21-28), strictly narrower, with the wording
//!    it replaces quoted verbatim so the narrowing is checkable.** The
//!    "What REMAINS" paragraph used to read:
//!
//!    > *"The residual is now `states no fixture constructs` rather than `tabs
//!    > no fixture populates`. Concretely: the Defaults tab's string-EDIT
//!    > overlay — `defaults_editing = Some(idx)` on a `ConfigValueKind::String`
//!    > entry — draws `defaults_text_buffer` and `entry.key` into a `Clear`ed
//!    > popup through a code path no probe state reaches, and the Driver tab's
//!    > `driver_dry_run` preview is another. Both are reachable only by driving
//!    > the key handler into a mode, which is the shape `DriverStartScreen`'s
//!    > "goal step" fixture uses and which is not done for these.
//!    > **Under-detection, silent.**"*
//!
//!    That wording named `driver_dry_run` as an unprobed STATE and named NEITHER
//!    the output pane, the journal nor the inbox as unescaped SITES — which is
//!    why round 9 read as green over the Driver tab's largest render surface.
//!    `driver_dry_run` is now a probed state (`Driver tab, dry-run preview`),
//!    and `probe_ctx` populates `ctx.driver_output`, `cache.driver_journal` and
//!    `cache.driver_inbox`. So the Defaults string-EDIT overlay remains the
//!    residual STATE, and the sentence below replaces the rest of it.
//!
//!    **What REMAINS after 21-28, and it is a different kind of residual.** The
//!    four Driver-tab sites are now all composed, but they are NOT all held the
//!    same way, and a reader who assumes the whole path is compiler-held is
//!    wrong about three quarters of it:
//!
//!    | Site | Value | Held by |
//!    |---|---|---|
//!    | output pane | `DriverOutputLine::text` | **the TYPE** — `crate::text::Untrusted`; a new render is a compile error |
//!    | injection rows | `journal::inbox::InboxMessage::text` | a CALL + this probe |
//!    | dry-run preview | `ui::screens::DryRunPreview::report` | a CALL + this probe |
//!    | opt-in disclosure | `PromptInput::path`, `digest` | a CALL + this probe |
//!
//!    The last three carriers are still bare `String`s, because they reach
//!    `src/journal/inbox.rs` and `src/app.rs`, which no plan in this wave owns —
//!    retyping them would have been a wave conflict, not a closure (D-21-39).
//!    **The failure direction is under-protection, and it is SILENT**: a NEW
//!    render of any of those three values compiles, draws, and is caught only if
//!    a probe state happens to reach it. Nothing goes red at the moment the new
//!    site is written.
//!
//!    **What would force the promote to a type:** a third render of either
//!    value, or any change to `src/journal/inbox.rs` or `src/app.rs` already
//!    open for another reason — at which point retyping the carrier costs almost
//!    nothing and removes the last call-held sites on this path.
//!
//!    Two smaller residuals, named rather than left implicit. The injection
//!    fixture leaves its message in the `Queued` state, so the `Delivered`,
//!    `ActedOn` and `Missed` transitions are not probed — the `message.text` row
//!    is drawn in every state, so the escaped SITE is covered, but the elapsed
//!    counter and the missed-reason gloss are not. And the dry-run fixture sets
//!    `report: Some(..)`, so the `None` loading branch is unprobed; it draws an
//!    authored constant and no identity. **Under-detection, silent**, both.
//!
//!    **REWRITTEN AGAIN 2026-08-27 (21-30 T1), strictly narrower, with the
//!    wording it replaces quoted verbatim so the narrowing is checkable.** The
//!    sentence above closing the 21-28 rewrite used to read:
//!
//!    > *"So the Defaults string-EDIT overlay remains the residual STATE, and
//!    > the sentence below replaces the rest of it."*
//!
//!    and the 21-25 wording it inherited framed that overlay purely as a
//!    COVERAGE gap — a state "reachable only by driving the key handler into a
//!    mode". **It was also a CORRECTNESS gap, and naming it only as coverage is
//!    what let it stand for two more rounds.** The overlay drew
//!    `defaults_text_buffer` raw: a plain `String` copy of the same
//!    `entry.value` the list one render above already escaped, laundering the
//!    escape through an untyped round trip on the surface where the operator
//!    decides what to write to disk.
//!
//!    **Of that wording's two concrete residual examples, BOTH are now closed
//!    and by what:** `driver_dry_run` by **21-28** (the `Driver tab, dry-run
//!    preview` state), and the Defaults string-EDIT overlay by **21-30** — a
//!    `Defaults tab, string edit` state whose index and seed are derived from
//!    the populated `defaults_config`, plus `ProjectViewCache::defaults_text_buffer`
//!    retyped to [`super::EditBuffer`] so the popup's `Span` is a compile error
//!    until it goes through `shown()`.
//!
//!    **A residual with no example is a residual nobody can check, so here is a
//!    NEW concrete one.** `DetailScreen`'s Defaults tab has a THIRD overlay this
//!    fixture still does not construct: the **dropdown** branch, taken when
//!    `defaults_editing` is `Some(idx)` at a row whose `dropdown_options` are
//!    non-empty (`ConfigValueKind::Bool` and the enum-valued keys). It draws
//!    `entry.key` in its title and its option strings in the list.
//!    `first_string_entry` deliberately skips those rows, so no probe state
//!    reaches that branch. **Under-detection, silent** — though narrower than
//!    the overlay it replaces: the dropdown's options come from
//!    `dropdown_options`, which returns authored `&'static str` variants rather
//!    than anything read off disk, so what is unprobed there is the KEY in the
//!    title, which the tab already draws escaped one render below.
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
//! 6. **The WALK's residual, now that its job is only fixture coverage
//!    (21-26).** What makes adjudication mandatory is the sealed supertrait, and
//!    that has no residual of this shape at all — it is a property of the type.
//!    What the walk still answers is "is this adjudicated screen rendered by any
//!    committed control", and THAT answer is still bounded by source text: a
//!    screen the walk cannot NAME is a screen whose fixture coverage nobody
//!    checked. Three of that residual's known shapes are closed or made loud,
//!    each against a planted defect:
//!
//!    * A **wrapped `impl` header** is no longer a residual: physical lines are
//!      joined into logical ones by [`join_logical_impl_header`] before the
//!      needle is looked for. Controlled by
//!      `a_wrapped_impl_header_is_one_logical_unit`, which drives the same
//!      helper with the header split at each of the three places it can wrap and
//!      also pins the join's bound.
//!    * An **`impl` whose type name the scan cannot extract** — macro-generated,
//!      or generic — is now an `UNNAMEABLE IMPLEMENTATION` offence naming file
//!      and line, where it used to be a silent `continue`. It fails the same
//!      assertion the other offences do.
//!    * **Two same-named implementors in two files** no longer collapse: the
//!      derived set and the table are both sets of `(type name, path)` PAIRS
//!      (IN-01). Controlled by the fourth synthetic direction in
//!      `the_census_reports_an_unadjudicated_screen_and_a_stale_row`.
//!
//!    **What REMAINS, with its direction.** An implementation whose source
//!    carries no `impl` token the walk recognises at all — one emitted entirely
//!    by a procedural macro, say — is still invisible to this walk, and so is one
//!    whose header wraps across more than [`IMPL_HEADER_JOIN_LINES`] physical
//!    lines. Such a screen still **cannot ship unadjudicated** (the supertrait),
//!    and if any fixture renders it the probe's assertions still apply — but
//!    nothing here reports that it HAS no fixture. **Under-detection, silent**,
//!    and one step narrower than CR-05 found it.
//!
//! **Every bound claimed above names a committed control; every residual names
//! its direction.** Limits 1, 2, 3 and the `Paragraph` half of 4 are residuals
//! and are marked under-detection. Limits 4 (for the preserving families) and 5
//! are bounds, and the controls are `the_screen_renders_identity_escaped`'s
//! assertions 0 and 4 plus
//! `the_teeth_precondition_answers_false_when_the_class_cannot_reach_a_cell`,
//! each observed red before it was observed green.

use super::{
    AppContext, RenderDisposition, Screen, RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    RENDERS_NO_ATTACKER_INFLUENCED_IDENTITY,
};
use crate::test_support::LOOK_ALIKE_PAIRS;
use crate::text::{display_identity, is_invisible_formatting_char};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// The fixture map — NARROWED 2026-08-27 (21-26)
// ---------------------------------------------------------------------------

/// `(type name, file relative to the crate root)`.
///
/// # This table's job is NARROWED, and the wording it replaces is quoted
///
/// **What this type's doc used to say, verbatim:** *"`(type name, file relative
/// to the crate root, disposition, reason)`. The reason column is the
/// adjudication, and a future reader inherits it: it states **which values the
/// screen draws and where they come from**, never 'escaped' or 'safe'. Rows are
/// added here because the walk found an implementor, never the other way
/// round."*
///
/// Two of those four columns are gone, and neither vanished — both were
/// PROMOTED onto the screen itself by the sealed
/// [`RenderAdjudicated`](super::RenderAdjudicated) supertrait:
///
/// * The **disposition** is now [`RenderAdjudicated::disposition`], read off the
///   constructed instance by the probe. Two authoritative statements of one fact
///   is the drift the probe exists to detect, one level up.
/// * The **reason** is now [`RenderAdjudicated::adjudication_reason`], carried
///   verbatim beside the `Screen::render` it describes rather than in this file.
///
/// **What this table still answers, and it is a different question from what
/// makes adjudication mandatory:** WHICH ADJUDICATED SCREENS HAVE A PROBE
/// FIXTURE. The compiler makes adjudication mandatory; the walk plus this table
/// catch a screen that compiles, is adjudicated, and yet is never rendered by
/// any committed control. Deleting it once the compiler took over the first job
/// would have traded a bounded residual for an unbounded one, so it is kept with
/// its job stated (prohibition 4 of 21-26).
type FixtureRow = (&'static str, &'static str);

const SCREEN_IDENTITY_DISPOSITIONS: &[FixtureRow] = &[
    ("AddProjectScreen", "src/ui/screens/add_project.rs"),
    ("CreateProjectScreen", "src/ui/screens/create_project.rs"),
    ("DeleteConfirmScreen", "src/ui/screens/delete_confirm.rs"),
    ("DetailScreen", "src/ui/screens/detail.rs"),
    ("DriverConfirmScreen", "src/ui/screens/driver_confirm.rs"),
    ("DriverInjectScreen", "src/ui/screens/driver_inject.rs"),
    ("DriverStartScreen", "src/ui/screens/driver_start.rs"),
    ("EnqueueScreen", "src/ui/screens/enqueue.rs"),
    ("HelpScreen", "src/ui/screens/help.rs"),
    ("NormalScreen", "src/ui/screens/normal.rs"),
    (
        "QueueDeleteConfirmScreen",
        "src/ui/screens/queue_delete_confirm.rs",
    ),
];

// ---------------------------------------------------------------------------
// The walk — the second, weaker mechanism, with a narrowed job
// ---------------------------------------------------------------------------

const SRC_ROOT: &str = "src";

/// The needle is assembled at RUNTIME from two halves that are meaningless
/// apart, following `tests/spawn_seam_guard.rs:216`. Spelled as one literal,
/// this file's own source would match the walk and the census would start
/// reporting itself.
const IMPL_HEAD: &str = "Screen";
const IMPL_TAIL: &str = " for ";

/// The second census's needle halves (WR-02, 21-30), assembled at runtime for
/// the same reason [`IMPL_HEAD`]'s are: spelled as one literal, the line
/// spelling it would itself be an `impl`-shaped match and the census would
/// report its own definition.
const ADJ_HEAD: &str = "RenderAdj";
const ADJ_MID: &str = "udicated";

/// **The adjudication reasons that DO use a forbidden verdict word, pinned by
/// screen and by token** (WR-03, 21-30) — a DISCLOSURE, not a pardon.
///
/// `(screen type name, the token it uses, why it is not fixed here)`.
///
/// WR-03's step is to MEASURE before asserting, and the measurement found one:
/// `NormalScreen`'s reason ends *"`row_badge`'s lookup keys off the RAW alias
/// while the cell beside it is escaped — the worked example of the split."*
/// That is the finding WR-03 predicts, and it is recorded here rather than
/// smoothed away by narrowing the check.
///
/// **It is not fixed in this plan because `src/ui/screens/normal.rs` is not this
/// plan's file** — it is 21-29's, merged in wave 1, and this plan's prohibitions
/// fence the screen files. Rewriting a reason there would be a wave-fence
/// violation to make a number look right.
///
/// **Failure direction: under-detection, ONE screen and ONE token wide, and
/// LOUD in both other directions.** The entry pins the exact token, so a
/// SECOND forbidden word in the same reason still goes red; every other screen
/// is unexempted; and if the reason is ever rewritten the entry reports itself
/// STALE on every run, so it cannot quietly outlive its subject.
///
/// **What would remove it:** one clause rewritten in `normal.rs`'s
/// `adjudicate_screen!` reason to name the split by provenance — *"`row_badge`'s
/// lookup keys off the RAW alias while the cell beside it goes through
/// `render_for_terminal`"* — owned by whoever next edits that file.
const REASON_VERDICT_EXEMPTIONS: &[(&str, &str, &str)] = &[(
    "NormalScreen",
    "escaped",
    "the status-footer split's worked example: \"`row_badge`'s lookup keys off \
     the RAW alias while the cell beside it is escaped\". src/ui/screens/normal.rs \
     is 21-29's file and outside 21-30's fence; the repair is to name the split \
     by provenance (`render_for_terminal`) rather than by verdict.",
)];

/// How the macro's OWN definition site is recognised — **structurally, not by
/// path and line**.
///
/// `adjudicate_screen!`'s body contains `impl $crate::ui::screens::RenderAdjudicated
/// for $type`, which is a legitimate match and the only one. Exempting it by
/// `src/ui/screens/mod.rs:NNN` would go stale the first time a line moved above
/// it, and exempting the whole file would blind the census to a hand-written
/// impl added there. `$crate` is valid ONLY inside a macro body, so it
/// identifies the one legitimate site and cannot be spelled by a hand-written
/// impl anywhere.
const MACRO_BODY_MARKER: &str = "$crate";

/// One source file: its path relative to the crate root, and its numbered lines.
type SourceFile = (String, Vec<(usize, String)>);

/// One implementation site the walk could NAME: `(type name, path)`.
///
/// **The key is the pair and not the bare type name** (IN-01). Keyed by name
/// alone, two `Screen`s with the same type name in two different files collapse
/// to one entry — on BOTH sides, since the table was keyed the same way — and one
/// implementor goes silently unchecked while the census reports clean. Observed
/// by planting exactly that: see
/// [`the_census_reports_an_unadjudicated_screen_and_a_stale_row`](tests::the_census_reports_an_unadjudicated_screen_and_a_stale_row).
type ScreenSite = (String, String);

/// What the walk found: the sites it could name, and the ones it could not.
///
/// **The second field is the point of this struct.** The walk used to
/// `continue` past an `impl` line whose type name it could not extract, which
/// is a census being silently short — the exact harm it exists to prevent, one
/// level down. Those sites are now carried out and reported as their own
/// offence.
struct SourceCensus {
    implementors: std::collections::BTreeSet<ScreenSite>,
    /// `path:line` for every `impl` whose type name the scan could not extract.
    unnameable: Vec<String>,
}

/// How many physical lines a wrapped `impl` header may span before the join
/// gives up.
///
/// Four is generous for a header — the longest in this tree is one line — and
/// bounded so a file with an unclosed brace cannot make the join swallow the
/// rest of the file.
const IMPL_HEADER_JOIN_LINES: usize = 4;

/// Join physical lines from `start` into ONE logical `impl` header.
///
/// **A wrapped header is not two lines to the compiler and must not be two
/// lines here** (CR-05). `impl Screen for\n    NormalScreen {` was invisible to
/// the old single-line scan, and that invisibility was one of the two spellings
/// that let the census report ELEVEN while thirteen implementors existed.
///
/// The accumulation stops at the body opener `{`, at a blank line, or after
/// [`IMPL_HEADER_JOIN_LINES`] physical lines. Comment-only continuation lines
/// are dropped, preserving the property that a doc comment naming the trait
/// cannot forge a member. The caller keeps the FIRST physical line's number, so
/// an offence names where a reader should look.
fn join_logical_impl_header(lines: &[(usize, String)], start: usize) -> String {
    let mut logical = lines[start].1.trim().to_string();
    let mut taken = 1;
    let mut index = start;
    while !logical.contains('{') && taken < IMPL_HEADER_JOIN_LINES && index + 1 < lines.len() {
        index += 1;
        let next = lines[index].1.trim();
        if next.is_empty() {
            break;
        }
        taken += 1;
        if next.starts_with("//") {
            continue;
        }
        logical.push(' ');
        logical.push_str(next);
    }
    logical
}

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
/// **NARROWED 2026-08-27 (21-26). This is NO LONGER what makes adjudication
/// mandatory** — the sealed [`RenderAdjudicated`](super::RenderAdjudicated)
/// supertrait is, and it is blind to source formatting because it is a property
/// of the type. What this walk answers is the different question the compiler
/// does not: which adjudicated screens have a probe FIXTURE.
///
/// The recursive shape follows `tests/spawn_seam_guard.rs:227-294`: an
/// unreadable entry is skipped rather than panicked on, paths are relative to
/// `CARGO_MANIFEST_DIR`, and lines whose trimmed form opens a line comment are
/// dropped so a doc comment naming the trait cannot forge a member.
///
/// Its floor was raised in the same commit that narrowed its job, because a
/// narrowed job can still miss: physical lines are joined by
/// [`join_logical_impl_header`] so a wrapped header is one unit, and an `impl`
/// whose type name cannot be extracted is carried out in
/// [`SourceCensus::unnameable`] and REPORTED rather than skipped. LIMIT 6 of the
/// module doc states what still gets past it and in which direction.
///
/// It asserts that it found production source at all — the non-vacuity floor
/// `source_files` already carries — so a walk that looked at nothing cannot
/// report clean.
/// Every Rust source file under `src/`, numbered, sorted, with the non-vacuity
/// floor asserted.
///
/// Extracted in 21-30 so the two censuses that walk `src/` — the screen-
/// implementor walk and the hand-written-adjudication census — enumerate the
/// SAME set by construction rather than by two copies of the same six lines
/// that can drift apart.
fn walked_source_files() -> Vec<SourceFile> {
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
    files
}

/// **Every hand-written `impl .. RenderAdjudicated for` outside the macro's own
/// definition site** (WR-02, 21-30).
///
/// This is the control for the half of WR-02 that is NOT a type property. The
/// `sealed` doc used to claim the macro is the only in-crate route to an
/// adjudication; it is not, because `mod sealed` is `pub(crate)` so any module
/// of this crate can satisfy the supertrait by hand. That half is CONVENTION,
/// and this turns the convention into something that can go red.
///
/// It judges the same logical unit [`screen_implementors_from_source`] does —
/// physical lines joined by [`join_logical_impl_header`], comment lines dropped
/// — so a wrapped header is one unit here exactly as it is to the compiler. The
/// one legitimate match is recognised by [`MACRO_BODY_MARKER`].
///
/// Pure over its input so the live assertion and its self-match control drive
/// THIS function rather than two spellings of it.
fn adjudication_impl_offences(files: &[SourceFile]) -> Vec<String> {
    let needle = format!("{ADJ_HEAD}{ADJ_MID}{IMPL_TAIL}");
    let mut out = Vec::new();
    for (path, lines) in files {
        for (index, (number, line)) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || !trimmed.starts_with("impl") {
                continue;
            }
            let logical = join_logical_impl_header(lines, index);
            if !logical.contains(&needle) {
                continue;
            }
            if logical.contains(MACRO_BODY_MARKER) {
                continue;
            }
            out.push(format!("{path}:{number}"));
        }
    }
    out
}

fn screen_implementors_from_source() -> SourceCensus {
    let files = walked_source_files();

    let needle = format!("{IMPL_HEAD}{IMPL_TAIL}");
    let mut implementors: std::collections::BTreeSet<ScreenSite> =
        std::collections::BTreeSet::new();
    let mut unnameable: Vec<String> = Vec::new();
    for (path, lines) in &files {
        for (index, (number, line)) in lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || !trimmed.starts_with("impl") {
                continue;
            }
            // JOINED, not the physical line (CR-05). A header wrapped across two
            // lines is one logical unit here exactly as it is to the compiler.
            let logical = join_logical_impl_header(lines, index);
            let Some(at) = logical.find(&needle) else {
                continue;
            };
            let name: String = logical[at + needle.len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if name.is_empty() {
                // REPORTED, never skipped. `continue` here is a census going
                // quietly short, which is the harm this module exists to
                // prevent.
                unnameable.push(format!("{path}:{number}"));
                continue;
            }
            implementors.insert((name, path.clone()));
        }
    }
    assert!(
        !implementors.is_empty(),
        "the census walked {} source files and found no trait implementors at \
         all. Either the trait was renamed or the needle stopped matching; \
         either way this census is now enumerating nothing.",
        files.len()
    );
    SourceCensus {
        implementors,
        unnameable,
    }
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
    derived: &std::collections::BTreeSet<ScreenSite>,
    table: &std::collections::BTreeSet<ScreenSite>,
    unnameable: &[String],
) -> Vec<String> {
    let mut offences = Vec::new();

    let paths_recorded_for = |name: &str, set: &std::collections::BTreeSet<ScreenSite>| {
        set.iter()
            .filter(|(candidate, _)| candidate == name)
            .map(|(_, path)| path.clone())
            .collect::<Vec<_>>()
    };

    for site in derived {
        if table.contains(site) {
            continue;
        }
        let (name, path) = site;
        let recorded = paths_recorded_for(name, table);
        if recorded.is_empty() {
            offences.push(format!(
                "UNFIXTURED IMPLEMENTOR: `{name}` (in {path}) implements the \
                 trait but no row gives it a probe fixture, so nothing renders \
                 it and its adjudication is never CHECKED against a real render. \
                 Add a row to SCREEN_IDENTITY_DISPOSITIONS and an arm to \
                 `fixture_for`. (Renamed 2026-08-27 from `UNADJUDICATED \
                 IMPLEMENTOR`: since 21-26 an unadjudicated screen does not \
                 compile at all, so this message can no longer be about \
                 adjudication without lying about which mechanism failed.)"
            ));
        } else {
            offences.push(format!(
                "RELOCATED IMPLEMENTOR: `{name}` has a fixture row at {recorded:?} \
                 but the walk found an implementation at {path}. Either the \
                 screen moved and the row's path outlived it, or there are TWO \
                 same-named implementors in different files and only one of them \
                 is fixtured. The census keys on (type name, path) precisely so \
                 the second case cannot collapse into the first and go unchecked \
                 (IN-01); re-read the render and re-state the row."
            ));
        }
    }

    for site in table {
        if derived.contains(site) {
            continue;
        }
        let (name, path) = site;
        if paths_recorded_for(name, derived).is_empty() {
            offences.push(format!(
                "STALE ROW: `{name}` has a fixture row at {path} but the walk found \
                 no such implementor. Either the screen was deleted and the row \
                 outlived it, or the walk stopped reaching it — and a table that \
                 outlives its subject is how a reader is told a surface is \
                 covered when it is not."
            ));
        }
    }

    for site in unnameable {
        offences.push(format!(
            "UNNAMEABLE IMPLEMENTATION: an implementation of the trait at {site} \
             has a type name this scan cannot extract — it is macro-generated, or \
             carries a generic parameter, or is spelled in some way the extractor \
             does not handle. It COMPILES, so it is adjudicated: the sealed \
             supertrait saw to that. What nobody has checked is whether it has a \
             probe fixture, which is this walk's whole remaining job. Give it a \
             row in SCREEN_IDENTITY_DISPOSITIONS and an arm in `fixture_for` by \
             hand, or this census is silently short by one."
        ));
    }

    offences
}

fn disposition_table() -> std::collections::BTreeSet<ScreenSite> {
    SCREEN_IDENTITY_DISPOSITIONS
        .iter()
        .map(|(name, path)| ((*name).to_string(), (*path).to_string()))
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

    // The Driver tab's LIVE OUTPUT PANE — the pane that displays the agent's own
    // prose, and the one this whole phase is named for (21-28 T1).
    //
    // `output_for_run` takes the live ring only when its run id equals the
    // selected run's AND it is non-empty, so both of those are properties of
    // this fixture rather than accidents of it. Without this the pane fell
    // through to `NO_JOURNAL_ENTRIES` — an authored `&'static str` — and every
    // assertion about the pane passed by drawing a string this build wrote.
    //
    // The text goes in through `push_record`, not by constructing a
    // `DriverOutputLine` literal: `push_record` is the one place the buffer's
    // sanitisation and its ring accounting happen, and a fixture that bypassed
    // it would probe a value the production path cannot produce.
    let mut live = super::DriverOutput::for_run(identity);
    live.push_record(super::DriverLineKind::Output, identity);
    live.push_record(super::DriverLineKind::Stderr, identity);
    live.push_record(super::DriverLineKind::Diagnostic, identity);
    live.push_record(super::DriverLineKind::Terminal, identity);
    ctx.driver_output.insert(identity.to_string(), live);

    let cache = ctx.view_cache.entry(identity.to_string()).or_default();

    // The INJECTION ROWS (21-28 T2). `derive_injection_states` maps over the
    // inbox, so one message here is one rendered injection block, and
    // `injection_rows` draws `message.text` at `INJECTION_INDENT`.
    //
    // `InboxMessage::text` is stored VERBATIM and deliberately un-redacted (its
    // own field doc says so), the file is `inbox.jsonl` on disk, and this build
    // does not exclusively own that file.
    //
    // The message is left in its `Queued` state: `derive_injection_states`
    // returns `Queued` when no journal record carries the id, and the
    // `message.text` row is drawn in EVERY state, so the site under test is
    // reached without inventing a transition. The Delivered and Missed
    // transitions are NOT probed here — see LIMIT 1.
    cache.driver_inbox = vec![crate::journal::inbox::InboxMessage {
        id: identity.to_string(),
        ts: "2026-08-27T00:00:00Z".to_string(),
        text: identity.to_string(),
    }];

    // The journal the scan read off disk, with the run id the selected run
    // carries — `render_output_section` only consults `driver_journal` when the
    // ids match, so a mismatched id would make this populated and unread.
    cache.driver_journal = Some(Box::new(super::DriverRunJournal {
        run_id: identity.to_string(),
        output: {
            let mut journal = super::DriverOutput::for_run(identity);
            journal.push_record(super::DriverLineKind::Output, identity);
            journal
        },
        injections: Vec::new(),
    }));

    ctx.recompute_filtered_aliases();
    ctx.table_state.select(Some(0));
    ctx
}

/// The authored half of the status footer's message, spelled exactly as
/// `App::start_driver_run` builds it at `src/app.rs:1870`.
///
/// **A token only the status branch can produce.** `render_footer` dispatches
/// on `searching` first and `status_message` second; the alternative branch,
/// `render_normal_footer`, draws counts and keybinding hints and nothing
/// containing this. Asserting on it is what tells a reached branch apart from a
/// state that set a field the render never looks at — the failure the
/// `searching: true` note in this module records, where an assertion passed by
/// silence until somebody noticed.
const STATUS_BRANCH_TOKEN: &str = "Driving ";

/// A status message shaped like the ones `src/app.rs` actually builds.
///
/// **The whole point of WR-03 is visible in this one line**: the untrusted
/// value is INSIDE a `format!`, interleaved with a sentence this build wrote,
/// so there is no field to give a carrier. Measured at HEAD, six
/// `status_message = Some(..)` sites exist in `src/app.rs`; four of them
/// interpolate a registry key or a run id exactly like this, one is a literal,
/// and the sixth forwards whatever any screen handed to
/// `ScreenAction::SetStatusMessage`.
fn status_message_like_app_builds_it(identity: &str) -> String {
    format!("{STATUS_BRANCH_TOKEN}{identity} \u{2014} run {identity}")
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
         four half-composed ones below it were exercised by nothing. \
         ALSO draws, since 21-28 T1, the LIVE OUTPUT PANE: `DriverOutputLine::text` for \
         each line of `ctx.driver_output[alias]`, whose bytes are the `exec_event` text \
         a run's journal holds on disk — the agent's own prose, written by the model and \
         read back by `crate::app::driver_line_for_record`. Four lines are pushed through \
         `DriverOutput::push_record`, one per `DriverLineKind` the pane styles \
         differently, so the marker arms are drawn rather than assumed.",
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
    (
        "Driver tab, dry-run preview",
        true,
        "Draws `DryRunPreview::report` line by line through `render_dry_run_preview`. \
         The report is the already-rendered text of `dry_run::render`, whose own doc \
         says it interpolates paths and branch names read from the project — a \
         repository the operator cloned. `report` is `Some` here deliberately: `None` \
         is the loading state and paints the authored `DRY_RUN_LOADING` idiom, which \
         draws no identity at all. The preview REPLACES the run detail, which is why \
         this is a state of its own rather than a field set in `probe_ctx`.",
    ),
    (
        "Defaults tab, string edit",
        true,
        "Draws `defaults_text_buffer` and `entry.key` into a `Clear`ed `Paragraph` \
         popup that overlays the list. The buffer's bytes are a copy of \
         `entry.value` for a `ConfigValueKind::String` row of the project's \
         `.planning/config.json` — free-form text supplied by whoever wrote that \
         file, taken at the moment the operator pressed Enter on the row. The list \
         underneath keeps drawing `entry.value` for every row, so this state draws \
         the same bytes from two different sources through two different widget \
         families. The index and the seed are derived from `defaults_config` by \
         `detail::first_string_entry`, never spelled.",
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
    // The DRY-RUN PREVIEW (21-28 T2). It is a separate state and not part of
    // `probe_ctx` for a load-bearing reason: `render_driver_tab` dispatches on
    // `driver_dry_run` being `Some` and the preview REPLACES the run detail, so
    // setting it in `probe_ctx` would have hidden the output pane and the
    // injection rows that Task 1 and this task's first half just made visible.
    //
    // `report` is the already-rendered text of `dry_run::render`, whose own doc
    // says it interpolates paths and branch names read from the project — a
    // repository the operator cloned. `Some`, not `None`: `None` is the loading
    // state and paints the authored `DRY_RUN_LOADING` idiom, which is the empty
    // branch this fixture exists to leave.
    // **The report is DERIVED from the run the fixture populated, never spelled
    // from `identity` directly, and that is load-bearing.** Every arrange here
    // runs against `chrome_ctx` too, to measure the baseline arrival is
    // compared against. An arrange that spells the untrusted value itself puts
    // that value in BOTH sides, the difference is zero, and the state reports
    // "did not arrive" — which is exactly what happened when this was written
    // the obvious way, and is why it is written this way instead.
    //
    // `chrome_ctx` has no runs, so the `else` below leaves `driver_dry_run`
    // unset there and the baseline draws chrome only. It also mirrors the real
    // report, which interpolates values read from the project rather than
    // authored ones.
    ("Driver tab, dry-run preview", |identity, ctx| {
        ctx.detail_sub_view_per_project
            .insert(identity.to_string(), crate::app::DetailSubView::Driver);
        let cache = ctx.view_cache.entry(identity.to_string()).or_default();
        let Some(run) = cache.driver_runs.first() else {
            return;
        };
        let (goal, command) = (run.goal.clone(), run.gsd_command.clone());
        cache.driver_dry_run = Some(super::DryRunPreview {
            command: command.clone(),
            report: Some(format!(
                "Would run: {command}\nWorktree: {goal}\nPush: refs/heads/{goal}\n"
            )),
        });
    }),
    ("Browse tab, file view", |identity, ctx| {
        ctx.detail_sub_view_per_project
            .insert(identity.to_string(), crate::app::DetailSubView::Browse);
        let cache = ctx.view_cache.entry(identity.to_string()).or_default();
        cache.browser_depth = crate::browser::BrowserDepth::View;
        cache.browser_file_content = Some(format!("# {identity}\n"));
    }),
    // The DEFAULTS STRING-EDIT OVERLAY (21-30 T1). LIMIT 1 named this state as
    // the residual no fixture reached, and framed it purely as a coverage gap.
    // It was also a CORRECTNESS gap: `render_defaults_tab` dispatches on
    // `defaults_editing` being `Some(idx)` at a `ConfigValueKind::String` row
    // and draws `defaults_text_buffer` — a raw copy of the very `entry.value`
    // the list one render above already escapes — into a `Clear`ed
    // `Paragraph` popup. That is the LAUNDERING this task closes at the type.
    //
    // **Both the index and the seed are DERIVED from the config the fixture
    // populated, never spelled from `identity`** — see `detail::first_string_entry`
    // and 21-28's measured near-miss. `chrome_ctx` has no `defaults_config`, so
    // the helper returns `None` there, the arrange returns early, and the
    // baseline draws chrome only.
    ("Defaults tab, string edit", |identity, ctx| {
        ctx.detail_sub_view_per_project
            .insert(identity.to_string(), crate::app::DetailSubView::Defaults);
        let cache = ctx.view_cache.entry(identity.to_string()).or_default();
        let Some((idx, value)) = super::detail::first_string_entry(cache) else {
            return;
        };
        cache.defaults_editing = Some(idx);
        cache.defaults_text_buffer = super::EditBuffer::seed_from_untrusted_source(value);
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
            let mut with_status = probe_ctx(identity);
            with_status.status_message = Some((
                status_message_like_app_builds_it(identity),
                std::time::Instant::now(),
            ));
            vec![
                one_state(
                    "dashboard",
                    plain,
                    Box::new(super::normal::NormalScreen::new()),
                ),
                // WR-03. `render_footer` dispatches on `searching` FIRST and on
                // `status_message` second, so this state must set the message
                // AND leave `searching` false — `NormalScreen::new()` does, and
                // the `searching: true` note below records what happens when a
                // state sets a field without the field the render dispatches
                // on. That the branch is actually reached is asserted by
                // `the_status_footer_state_reaches_the_status_branch` rather
                // than assumed here.
                one_state(
                    "dashboard with a status message",
                    with_status,
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
        use std::collections::BTreeSet;

        let site = |name: &str, path: &str| (name.to_string(), path.to_string());

        let derived: BTreeSet<ScreenSite> = [
            site("Fixtured", "src/a.rs"),
            site("NobodyFixturedMe", "src/b.rs"),
            // IN-01: the SAME type name in a SECOND file. Keyed on the bare name
            // this pair collapsed into the row below and the census reported
            // clean; keyed on the pair it is reported.
            site("Fixtured", "src/twin.rs"),
        ]
        .into_iter()
        .collect();
        let table: BTreeSet<ScreenSite> = [
            site("Fixtured", "src/a.rs"),
            site("IOutlivedMySubject", "src/c.rs"),
        ]
        .into_iter()
        .collect();
        let unnameable = vec!["src/macro_generated.rs:42".to_string()];

        let offences = census_offences(&derived, &table, &unnameable);

        assert_eq!(
            offences.len(),
            4,
            "the comparison must report all four harms and only those four, \
             got: {offences:#?}"
        );
        assert!(
            offences
                .iter()
                .any(|o| o.starts_with("UNFIXTURED IMPLEMENTOR")
                    && o.contains("NobodyFixturedMe")),
            "a derived pair whose NAME the table does not carry at all is a \
             screen with no probe fixture and must be reported as that, got: \
             {offences:#?}"
        );
        assert!(
            offences
                .iter()
                .any(|o| o.starts_with("RELOCATED IMPLEMENTOR") && o.contains("src/twin.rs")),
            "a SECOND implementor with the same type name in a different file \
             must be reported. Under the old bare-name key it collapsed into the \
             first and went silently unchecked — that is IN-01, and this arm is \
             the control for it. Got: {offences:#?}"
        );
        assert!(
            offences
                .iter()
                .any(|o| o.starts_with("STALE ROW") && o.contains("IOutlivedMySubject")),
            "a table member absent from the tree is a stale row and must be \
             reported as that — the harms are different and a message that \
             conflates them tells the reader to fix the wrong end, got: \
             {offences:#?}"
        );
        assert!(
            offences
                .iter()
                .any(|o| o.starts_with("UNNAMEABLE IMPLEMENTATION")
                    && o.contains("src/macro_generated.rs:42")),
            "an `impl` whose type name the scan cannot extract must be REPORTED \
             with its file and line, never `continue`d. A census that skips what \
             it cannot name is a census that is silently short, which is the harm \
             this module exists to prevent one level down. Got: {offences:#?}"
        );

        // The clean direction, so "reports something" is not mistaken for
        // "reports everything". Note the empty `unnameable`: a walk that named
        // everything it found must report nothing on that axis either.
        assert!(
            census_offences(&derived, &derived, &[]).is_empty(),
            "a set compared against itself, with nothing unnameable, must report \
             nothing"
        );
    }

    /// A wrapped `impl` header is ONE logical unit (CR-05).
    ///
    /// This drives the same [`join_logical_impl_header`] the live walk consumes,
    /// with the header split at each of the three places it can wrap, and
    /// asserts the needle is found in the joined form and absent from the first
    /// physical line. The `false` direction is the point: without it the test
    /// would pass against a join that simply concatenated the whole file.
    #[test]
    fn a_wrapped_impl_header_is_one_logical_unit() {
        let needle = format!("{IMPL_HEAD}{IMPL_TAIL}");

        let numbered = |source: &[&str]| -> Vec<(usize, String)> {
            source
                .iter()
                .enumerate()
                .map(|(index, line)| (index + 1, (*line).to_string()))
                .collect()
        };

        for wrapped in [
            vec!["impl Screen for", "    WrappedScreen {", "}"],
            vec!["impl", "    Screen for WrappedScreen {", "}"],
            vec!["impl Screen", "    for WrappedScreen {", "}"],
        ] {
            let lines = numbered(&wrapped);
            assert!(
                !lines[0].1.contains(&needle) || !lines[0].1.contains("WrappedScreen"),
                "the first physical line of {wrapped:?} must not carry the whole \
                 header, or this case is not testing a wrap at all"
            );
            let logical = join_logical_impl_header(&lines, 0);
            let at = logical.find(&needle).unwrap_or_else(|| {
                panic!("the joined header {logical:?} from {wrapped:?} does not carry the needle")
            });
            let name: String = logical[at + needle.len()..]
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            assert_eq!(
                name, "WrappedScreen",
                "the joined header {logical:?} from {wrapped:?} must yield the \
                 implementor's name"
            );
        }

        // The bound, so the join cannot swallow a file. A header that never
        // opens a body stops after IMPL_HEADER_JOIN_LINES physical lines.
        let runaway: Vec<&str> = std::iter::once("impl")
            .chain(std::iter::repeat_n("    filler", 20))
            .collect();
        let joined = join_logical_impl_header(&numbered(&runaway), 0);
        assert_eq!(
            joined.matches("filler").count(),
            IMPL_HEADER_JOIN_LINES - 1,
            "the join must stop after {IMPL_HEADER_JOIN_LINES} physical lines; \
             an unbounded join would let one unclosed brace make the census \
             read the rest of the file as one header. Joined: {joined:?}"
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
        let census = screen_implementors_from_source();
        let table = disposition_table();

        let offences = census_offences(&census.implementors, &table, &census.unnameable);
        assert!(
            offences.is_empty(),
            "the derived render surface and the fixture map disagree:\n\n{}",
            offences.join("\n\n")
        );

        assert_eq!(
            census.implementors, table,
            "the walk's set of (type name, path) pairs and the fixture map's set \
             must be equal in both directions"
        );
    }

    /// **WR-02's convention half, given a control instead of a claim** (21-30).
    ///
    /// `RenderDisposition` makes the VOCABULARY a type property. The other half
    /// of `sealed`'s corrected doc — that an in-crate screen goes through
    /// `adjudicate_screen!` rather than hand-writing `impl Sealed` and `impl
    /// RenderAdjudicated` — is genuinely convention, because `mod sealed` is
    /// `pub(crate)` by design. This is what makes that convention able to go
    /// red.
    ///
    /// Observed RED by planting a compiling hand-written impl in
    /// `src/driver/liveness.rs`, a file this plan does not otherwise touch; the
    /// panic named it by path and line, and the plant was removed.
    #[test]
    fn no_hand_written_render_adjudicated_impl_skips_the_macro() {
        let files = walked_source_files();
        let offences = adjudication_impl_offences(&files);
        assert_eq!(
            offences.len(),
            0,
            "these sites implement `RenderAdjudicated` by hand rather than \
             through `crate::ui::screens::adjudicate_screen!`: {offences:?}. \
             That is possible — `mod sealed` is `pub(crate)` so any module of \
             this crate can satisfy the supertrait — which is exactly why this \
             is a census and not a claim in a doc. The macro is the route \
             because it emits the seal and the adjudication together and cannot \
             be given a partial one. If a site here is legitimate, it belongs in \
             the macro; if the macro cannot express it, say so at the site and \
             widen this census deliberately."
        );

        // NON-VACUITY, two ways. The walk must have looked at something, and
        // the needle must match the one legitimate site — otherwise a needle
        // that matches nothing at all would report zero forever.
        assert!(!files.is_empty());
        let needle = format!("{ADJ_HEAD}{ADJ_MID}{IMPL_TAIL}");
        let macro_sites = files
            .iter()
            .flat_map(|(path, lines)| lines.iter().map(move |(n, l)| (path, n, l)))
            .filter(|(_, _, line)| {
                let t = line.trim_start();
                !t.starts_with("//") && t.contains(&needle) && t.contains(MACRO_BODY_MARKER)
            })
            .count();
        assert_eq!(
            macro_sites, 1,
            "the needle {needle:?} matched {macro_sites} macro-body sites, not \
             the one `adjudicate_screen!` definition. Zero would mean the needle \
             matches nothing and the equality above is vacuous; more than one \
             would mean a second macro emits adjudications and this census's \
             structural exemption is now hiding it."
        );
    }

    /// The census cannot report its OWN definition, and does not report the
    /// macro's.
    #[test]
    fn the_adjudication_census_cannot_report_itself() {
        let files = walked_source_files();
        let this_file: Vec<SourceFile> = files
            .iter()
            .filter(|(path, _)| path.ends_with("render_escape_guard.rs"))
            .cloned()
            .collect();
        assert_eq!(
            this_file.len(),
            1,
            "the walk did not find this module's own source, so the self-match \
             control below is checking nothing"
        );
        assert!(
            adjudication_impl_offences(&this_file).is_empty(),
            "the census reports its own definition site. The needle is assembled \
             at runtime from halves meaningless apart for exactly this reason; \
             something in this file now spells it whole on a line starting with \
             `impl`."
        );
    }

    /// **`Screen` is still object-safe after `disposition()`'s return type
    /// changed** (21-30, WR-02), proven at RUNTIME rather than by argument.
    ///
    /// Round 9's design note says methods returning `&'static str` keep the
    /// vtable and an associated const would not. Changing a return type is
    /// exactly the change that could break that, so it is re-proven rather than
    /// assumed: two implementors stored as `Box<dyn Screen>` in a `Vec`, both
    /// supertrait methods called through `&dyn Screen`.
    #[test]
    fn the_screen_stays_object_safe_after_the_return_type_change() {
        let alias = clean_identity();
        let screens: Vec<Box<dyn Screen>> = vec![
            Box::new(crate::ui::screens::detail::DetailScreen::new(alias.clone())),
            Box::new(crate::ui::screens::help::HelpScreen::new()),
        ];
        assert_eq!(screens.len(), 2);

        for boxed in &screens {
            // THE CALL SITE, through the trait object's vtable.
            let as_dyn: &dyn Screen = boxed.as_ref();
            let disposition: RenderDisposition = as_dyn.disposition();
            let reason: &'static str = as_dyn.adjudication_reason();
            assert!(
                matches!(
                    disposition,
                    RENDERS_ATTACKER_INFLUENCED_IDENTITY | RENDERS_NO_ATTACKER_INFLUENCED_IDENTITY
                ),
                "{} reported a disposition outside the two-variant vocabulary, \
                 which the enum makes unrepresentable",
                as_dyn.name()
            );
            assert!(
                !reason.is_empty(),
                "{} adjudicated itself with an empty reason",
                as_dyn.name()
            );
            // And the snake_case mapping is reachable through the object too.
            assert!(disposition.as_str().starts_with("renders_"));
        }
    }

    /// **WR-03: `adjudication_reason` gets a CONTROL, so a trait method promoted
    /// as what a future reader inherits stops being a claim nothing consumes**
    /// (21-30).
    ///
    /// Before this, `adjudication_reason()` had ZERO readers in the tree: it was
    /// asserted by nothing and quoted by nothing, while its own doc told the
    /// next author what to write in it. A rule with no reader is a rule that
    /// drifts silently.
    ///
    /// Two things are checked, both from that doc's own words — *"never
    /// 'escaped' or 'safe'"*. A reason naming what a value IS sends a reader to
    /// the render; a reason asserting the value is safe tells them not to look.
    ///
    /// **Measured before it was asserted.** Every reason in the tree was
    /// collected and checked first; the result is recorded in this plan's
    /// SUMMARY. Had one contained a forbidden word, that would itself have been
    /// the finding WR-03 predicts and it would have been reported by screen name
    /// and rewritten to name values and sources — not quietly reworded into
    /// compliance.
    #[test]
    fn every_adjudication_reason_is_non_empty_and_names_values_not_verdicts() {
        // From `RenderAdjudicated::adjudication_reason`'s own doc.
        const FORBIDDEN: [&str; 2] = ["escaped", "safe"];

        // WHOLE TOKENS, never substrings — and this is not a nicety, it is a
        // defect this control had and shed.
        //
        // The first formulation asked `reason.to_lowercase().contains("safe")`
        // and went RED against `DetailScreen`, whose reason ends *"All of it is
        // third-party text under SAFE-07"*. `SAFE-07` is a REQUIREMENT ID. The
        // implementation was right and the control was wrong — the exact shape
        // WR-08 reports one file over, and the shape 21-29's first backlog
        // assertion had. A control that goes red for a correct implementation is
        // worse than no control, so the check was fixed rather than the reason
        // reworded into compliance.
        //
        // A token is a maximal run of alphanumerics, `-` and `_`, so `safe-07`
        // is one token and does not equal `safe`, while a bare `safe` or a
        // `safe,` still does.
        fn forbidden_tokens(reason: &str) -> Vec<String> {
            reason
                .split(|c: char| !(c.is_alphanumeric() || c == '-' || c == '_'))
                .map(|token| token.to_ascii_lowercase())
                .filter(|token| FORBIDDEN.contains(&token.as_str()))
                .collect()
        }

        // The control's own control: the check must still FIRE for a bare
        // verdict word, or the pass below would be a pass by construction.
        assert_eq!(
            forbidden_tokens("every value here is escaped before it reaches a cell"),
            vec!["escaped".to_string()],
            "the token check no longer fires for a bare verdict word, so every \
             assertion below passes vacuously"
        );
        assert!(
            forbidden_tokens("third-party text under SAFE-07 and DRIVE-01").is_empty(),
            "the token check still reports a requirement ID as a verdict word — \
             the substring defect this formulation exists to fix"
        );

        let clean = clean_identity();
        let mut checked = 0usize;
        for (name, path) in SCREEN_IDENTITY_DISPOSITIONS {
            let Some(build) = fixture_for(name) else {
                panic!("{name} ({path}) has no probe fixture");
            };
            let states = build(&clean);
            let Some(first) = states.first() else {
                panic!("{name} ({path}) built no states");
            };
            let as_dyn: &dyn Screen = first.screen.as_ref();
            let reason = as_dyn.adjudication_reason();
            checked += 1;

            assert!(
                !reason.trim().is_empty(),
                "{name} ({path}) adjudicated itself with an EMPTY reason. The \
                 reason is what a future reader inherits about which values this \
                 screen draws and where their bytes come from; empty, the \
                 adjudication is a disposition with no argument behind it."
            );
            let found = forbidden_tokens(reason);
            let exempted = REASON_VERDICT_EXEMPTIONS
                .iter()
                .find(|(screen, _, _)| screen == name);
            match exempted {
                Some((_, token, why)) => {
                    // A SATISFIED exemption is REPORTED, not made red — the
                    // `WAVE_PENDING` idiom 21-29 established. A red here would
                    // break the tree for a bookkeeping reason the moment the
                    // reason is finally rewritten.
                    if found.is_empty() {
                        println!(
                            "STALE REASON EXEMPTION: {name} ({path}) no longer \
                             uses a verdict word, so the exemption shields \
                             nothing and must be removed. Its recorded reason \
                             was: {why}"
                        );
                    } else {
                        assert_eq!(
                            found,
                            vec![token.to_string()],
                            "{name} ({path}) is exempted for the token \
                             {token:?} only, and it now uses {found:?}. An \
                             exemption widened by drift is an exemption nobody \
                             decided. Reason: {reason:?}"
                        );
                        println!(
                            "KNOWN REASON VIOLATION (exempt, disclosed): {name} \
                             ({path}) uses {token:?} — {why}"
                        );
                    }
                }
                None => assert!(
                    found.is_empty(),
                    "{name} ({path})'s adjudication reason uses {found:?} as a \
                     whole word, which its own doc forbids: the reason must say \
                     WHICH values the screen draws and WHERE THEIR BYTES COME \
                     FROM, never that they are escaped or safe. A verdict tells \
                     the next reader not to look; a provenance tells them where \
                     to look. Reason: {reason:?}"
                ),
            }

            // The MEASUREMENT this control was written to make, printed rather
            // than only asserted, so a `--nocapture` run is the record.
            println!(
                "ADJUDICATION REASON: {name} ({path}) — {} chars, forbidden \
                 tokens: {found:?}",
                reason.chars().count()
            );
        }
        assert_eq!(
            checked,
            SCREEN_IDENTITY_DISPOSITIONS.len(),
            "not every adjudicated screen's reason was read, so this control is \
             narrower than it claims"
        );
        assert!(
            checked >= 11,
            "only {checked} screens were checked; the tree has eleven adjudicated \
             screens, so a smaller number means the fixture map shrank and this \
             control silently narrowed with it"
        );
    }

    /// Every adjudicated implementor can actually be built and rendered.
    #[test]
    fn every_adjudicated_screen_has_a_probe_fixture() {
        let missing: Vec<&str> = SCREEN_IDENTITY_DISPOSITIONS
            .iter()
            .filter(|(name, _)| fixture_for(name).is_none())
            .map(|(name, _)| *name)
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

        for (name, path) in SCREEN_IDENTITY_DISPOSITIONS {
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

            // THE DISPOSITION, READ OFF THE CONSTRUCTED INSTANCE (21-26, CR-05).
            // Not off a row in this file that restates what the screen does: one
            // statement of a fact cannot disagree with itself, and the both-ways
            // set equality below could only ever check MEMBERSHIP, never
            // CONTENT. `clean_states[0].screen` is a `Box<dyn Screen>`, so this
            // is a virtual call through the supertrait's vtable — which is also
            // the runtime proof that `Screen` stayed object-safe.
            let screen_disposition: RenderDisposition = {
                let as_dyn: &dyn Screen = clean_states[0].screen.as_ref();
                as_dyn.disposition()
            };

            // THE REASON, READ OFF THE SAME INSTANCE (WR-03, 21-30). It is
            // quoted in assertions 2, 3 and 4's failure messages below, so a
            // red names what the screen CLAIMS to draw beside what it actually
            // drew — which is the difference between "this screen leaked
            // something" and "this screen leaked something it told you it
            // draws". Before this, `adjudication_reason` had no readers at all:
            // a trait method promoted as what a future reader inherits,
            // consumed by nothing and asserted by nothing.
            let screen_reason: &'static str = {
                let as_dyn: &dyn Screen = clean_states[0].screen.as_ref();
                as_dyn.adjudication_reason()
            };

            for (clean_state, hostile_state) in clean_states.iter().zip(hostile_states.iter()) {
                let where_ = format!("{name} ({path}) [{}]", clean_state.label);
                let disposition: &dyn Screen = clean_state.screen.as_ref();
                let disposition = disposition.disposition();
                assert_eq!(
                    disposition, screen_disposition,
                    "{where_}: two instances of the same screen reported \
                     different dispositions, which the sealed macro makes \
                     impossible — so the fixture is building a different type \
                     for this state than for the first one"
                );
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

                match disposition {
                    RENDERS_ATTACKER_INFLUENCED_IDENTITY => {
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
                                 comparisons and persistence stays RAW.\n\
                                 \n\
                                 What this screen CLAIMS to draw \
                                 (adjudication_reason): {screen_reason}"
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
                                 segments, subprocess arguments and persistence.\n\
                                 \n\
                                 What this screen CLAIMS to draw \
                                 (adjudication_reason): {screen_reason}"
                            );
                        }
                    }
                    RENDERS_NO_ATTACKER_INFLUENCED_IDENTITY => {
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
                    // NO WILDCARD ARM, and its removal is a STRENGTHENING.
                    //
                    // Round 9's prohibition on this file required, verbatim:
                    // "the probe must `panic!` on any third value" — a runtime
                    // backstop for a hole `&'static str` left open. Since 21-30
                    // (WR-02) `disposition()` returns `RenderDisposition`, an
                    // enum with exactly two variants declared in a `pub(crate)`
                    // module. A third value is not EXPRESSIBLE, so there is
                    // nothing left for the arm to catch, and the compiler now
                    // enforces exhaustiveness here: adding a third variant
                    // would make THIS match fail to compile, which is a louder
                    // and earlier signal than a panic in one test run.
                    //
                    // A reader who finds the arm gone must not read it as a
                    // weakening. It was removed because its job moved into the
                    // type, not because the check was dropped.
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
                     not what the value is.\n\
                     \n\
                     What this screen CLAIMS to draw (adjudication_reason): \
                     {screen_reason}",
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
            match screen_disposition {
                RENDERS_ATTACKER_INFLUENCED_IDENTITY => assert!(
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

    /// **The `NormalScreen` status-message state is proven to REACH the status
    /// branch** (WR-03).
    ///
    /// This module already records what happens when a probe state sets a field
    /// without setting the field the render dispatches on: the
    /// `dashboard with filter footer` state set `filter_text` and left
    /// `searching` false, `render_footer` took the NORMAL footer, and the
    /// assertion about the filter passed by silence while the search footer
    /// drew `ctx.filter_text` raw the whole time. The new
    /// `dashboard with a status message` state has the same hazard one branch
    /// along — `render_footer` dispatches on `searching` FIRST and only then on
    /// `status_message` — so the branch is asserted rather than assumed.
    ///
    /// [`STATUS_BRANCH_TOKEN`] is drawn by nothing else on this screen: the
    /// alternative branch, `render_normal_footer`, paints counts, a sort
    /// indicator and keybinding hints.
    ///
    /// **The `searching: true` direction is asserted too**, because a token that
    /// arrives under BOTH branches would prove nothing. With `searching` set,
    /// the same context renders the search footer and the token is absent —
    /// which is exactly what the probe would report if the fixture state were
    /// written that way, and is the failure this test exists to make loud.
    #[test]
    fn the_status_footer_state_reaches_the_status_branch() {
        let clean = clean_identity();
        let message = status_message_like_app_builds_it(&clean);

        let mut reached = probe_ctx(&clean);
        reached.status_message = Some((message.clone(), std::time::Instant::now()));
        let reached_text = render_to_text(&crate::ui::screens::normal::NormalScreen::new(), &reached);

        assert!(
            reached_text.contains(STATUS_BRANCH_TOKEN),
            "the `dashboard with a status message` state did not reach \
             `render_footer`'s status branch: {STATUS_BRANCH_TOKEN:?} is absent \
             from the rendered buffer. Nothing else on this screen draws that \
             token, so what rendered is `render_normal_footer` and every \
             assertion the probe makes about the status footer is passing by \
             silence. Check that the state leaves `searching` false. \
             Rendered:\n{reached_text}"
        );

        let mut shadowed = probe_ctx(&clean);
        shadowed.status_message = Some((message, std::time::Instant::now()));
        let shadowed_text = render_to_text(
            &crate::ui::screens::normal::NormalScreen { searching: true },
            &shadowed,
        );

        assert!(
            !shadowed_text.contains(STATUS_BRANCH_TOKEN),
            "with `searching: true` the same context still drew \
             {STATUS_BRANCH_TOKEN:?}, so the token is not specific to the status \
             branch and the assertion above proves nothing about which branch \
             ran. Find a token only `render_footer`'s status arm can produce."
        );
    }

    /// **The two new Driver states REACH their branches** (21-28 T2 step (c)).
    ///
    /// Same discipline as [`the_status_footer_state_reaches_the_status_branch`],
    /// applied to the two sites this task escaped. A state that sets a field the
    /// render never dispatches on proves nothing, and this module already
    /// records one such near-miss — so each branch is asserted with a token only
    /// it can produce, and each token is checked in BOTH directions.
    ///
    /// * **Injection rows** — `\u{25CB} queued`, the glyph-and-label pair
    ///   `InjectionState::Queued.cell()` returns. It is drawn only by
    ///   `injection_rows`; the run detail's other panes draw no such pair. The
    ///   `false` direction empties `driver_inbox`, which makes
    ///   `derive_injection_states` return no entries and the block disappear.
    /// * **Dry-run preview** — the run detail's `steps` section rule, asserted
    ///   ABSENT. That is the stronger form of the token: the preview REPLACES
    ///   the run detail, so its rule vanishing is evidence the dispatch actually
    ///   swapped, not merely that a string appeared somewhere. The `true`
    ///   direction is the same rule present in the ordinary Driver tab state.
    #[test]
    fn the_two_new_driver_states_reach_their_branches() {
        let clean = clean_identity();
        use crate::ui::screens::driver::{GLYPH_QUEUED, LABEL_QUEUED};
        let queued_cell = format!("{GLYPH_QUEUED} {LABEL_QUEUED}");

        let build = |identity: &str, _ctx: &mut AppContext| -> Box<dyn Screen> {
            Box::new(crate::ui::screens::detail::DetailScreen::new(identity.to_string()))
        };
        let rendered = |label: &str| -> String {
            let state = states_over_sub_views(&clean, &build)
                .into_iter()
                .find(|s| s.label == label)
                .unwrap_or_else(|| panic!("no probe state labelled {label:?}"));
            render_to_text(state.screen.as_ref(), &state.ctx)
        };

        // --- Injection rows -------------------------------------------------
        let driver_tab = rendered("Driver tab");
        assert!(
            driver_tab.contains(&queued_cell),
            "the `Driver tab` state did not reach `injection_rows`: {queued_cell:?} \
             is absent, so `derive_injection_states` returned no entries and the \
             assertion that the injection row's `message.text` is composed is \
             passing by silence. Check that `probe_ctx` populates \
             `cache.driver_inbox`. Rendered:\n{driver_tab}"
        );

        let mut no_inbox = probe_ctx(&clean);
        no_inbox
            .detail_sub_view_per_project
            .insert(clean.clone(), crate::app::DetailSubView::Driver);
        no_inbox
            .view_cache
            .entry(clean.clone())
            .or_default()
            .driver_inbox
            .clear();
        let no_inbox_text = render_to_text(
            &crate::ui::screens::detail::DetailScreen::new(clean.clone()),
            &no_inbox,
        );
        assert!(
            !no_inbox_text.contains(&queued_cell),
            "with `driver_inbox` emptied the same state still drew \
             {queued_cell:?}, so the token is not specific to the injection-row \
             branch and the assertion above proves nothing about which branch ran."
        );

        // --- Dry-run preview ------------------------------------------------
        // `section_rule("steps", ..)` renders as `\u{2500}\u{2500} steps `.
        let steps_rule = "\u{2500}\u{2500} steps ";
        assert!(
            driver_tab.contains(steps_rule),
            "the ordinary `Driver tab` state does not draw the run detail's \
             `steps` section rule, so its ABSENCE below cannot be evidence that \
             the dry-run preview replaced it. Rendered:\n{driver_tab}"
        );

        let preview = rendered("Driver tab, dry-run preview");
        assert!(
            !preview.contains(steps_rule),
            "the `Driver tab, dry-run preview` state still drew the run detail's \
             `steps` section rule, so `render_driver_tab` did NOT dispatch into \
             `render_dry_run_preview` and every assertion about the preview is \
             passing by silence. Check that the state leaves `driver_dry_run` \
             `Some` — and note the arrange returns early when `driver_runs` is \
             empty. Rendered:\n{preview}"
        );
        assert!(
            preview.contains("Would run:"),
            "the `Driver tab, dry-run preview` state drew no report body. \
             `DryRunPreview::report` is drawn by `render_dry_run_preview` and by \
             nothing else, so its absence means the preview rendered its \
             `None` loading branch instead. Rendered:\n{preview}"
        );
    }

    /// **The Defaults string-edit state reaches the POPUP BRANCH, not merely
    /// the field** (21-30 T1).
    ///
    /// This module records a near-miss where a state set a field the render
    /// never dispatched on and every assertion about that state passed by
    /// silence. `defaults_editing = Some(idx)` is exactly that shape: the popup
    /// is drawn only if `entries.get(idx)` resolves AND that entry's kind is
    /// `ConfigValueKind::String`, so an index pointing at a `Bool` row would set
    /// the field, render the dropdown instead, and leave the escaping assertion
    /// asserting nothing.
    ///
    /// `detail::DEFAULTS_EDIT_BRANCH_TOKEN` is the popup's own title
    /// suffix and is drawn by no other branch of this screen — which the second
    /// half measures rather than asserts, by rendering the SAME tab with
    /// `defaults_editing` left `None`.
    #[test]
    fn the_defaults_string_edit_state_reaches_the_popup_branch() {
        let clean = clean_identity();
        let token = crate::ui::screens::detail::DEFAULTS_EDIT_BRANCH_TOKEN;

        let build = |identity: &str, _ctx: &mut AppContext| -> Box<dyn Screen> {
            Box::new(crate::ui::screens::detail::DetailScreen::new(identity.to_string()))
        };
        let edit_state = states_over_sub_views(&clean, &build)
            .into_iter()
            .find(|s| s.label == "Defaults tab, string edit")
            .expect("no probe state labelled \"Defaults tab, string edit\"");
        let edited = render_to_text(edit_state.screen.as_ref(), &edit_state.ctx);

        assert!(
            edited.contains(token),
            "the `Defaults tab, string edit` state did not reach the popup \
             branch: {token:?} is absent, so `render_defaults_tab` did NOT \
             dispatch into the string-input overlay and every assertion about \
             what the popup renders is passing by silence. Check that \
             `detail::first_string_entry` found a `ConfigValueKind::String` row \
             — it returns `None` for a cache with no `defaults_config`. \
             Rendered:\n{edited}"
        );

        // THE OTHER DIRECTION: the same tab with the overlay closed. If the
        // token appeared here too it would say nothing about which branch ran.
        let mut closed = probe_ctx(&clean);
        closed
            .detail_sub_view_per_project
            .insert(clean.clone(), crate::app::DetailSubView::Defaults);
        closed
            .view_cache
            .entry(clean.clone())
            .or_default()
            .defaults_editing = None;
        let closed_text = render_to_text(
            &crate::ui::screens::detail::DetailScreen::new(clean.clone()),
            &closed,
        );
        assert!(
            !closed_text.contains(token),
            "with `defaults_editing` left `None` the same tab still drew \
             {token:?}, so the token is not specific to the string-edit overlay \
             and the assertion above proves nothing about which branch ran."
        );
    }

    /// The alias the S1 spot-check registers its project under.
    ///
    /// **Deliberately shares no substring with either member of the fixture
    /// pair.** The arrival assertion below looks for the clean value in the
    /// rendered buffer; if the alias contained it, the alias's own render would
    /// satisfy that assertion and the echo could stop arriving entirely without
    /// anything going red.
    const ECHO_SPOT_CHECK_ALIAS: &str = "s1-spot-check-project";

    /// Type `typed` into the Enqueue screen and return the rendered buffer's
    /// cell symbols, through the real [`super::super::Screen::render`].
    fn enqueue_echo_text(typed: &str) -> String {
        // `super::super::tests` — this nested module's grandparent is
        // `ui::screens`, so the same `pub(super) fn ctx_with_aliases` the
        // module level reaches at :982 and :1191 is reachable here too, with
        // no visibility change anywhere (F3, D-21-60).
        let mut ctx = super::super::tests::ctx_with_aliases(&[ECHO_SPOT_CHECK_ALIAS]);
        ctx.input_buffer = typed.to_string();
        let screen = crate::ui::screens::enqueue::EnqueueScreen::new(
            ECHO_SPOT_CHECK_ALIAS.to_string(),
        );
        render_to_text(&screen, &ctx)
    }

    /// **21-29's S1, delivered: two directions at an INPUT-ECHO screen.**
    ///
    /// # Why this screen, and why S1 needed it
    ///
    /// `21-29` asked for a two-direction spot-check on a screen whose rendered
    /// value is **what the operator TYPED**, and both spot-checks it delivered
    /// landed on `.planning/`-derived values instead (`roadmap_widget.rs`'s
    /// phase name, `queue_delete_confirm.rs`'s queued command). `21-29`
    /// self-disclosed that as not met; `21-30` deferred it. The subject here is
    /// [`EnqueueScreen`](crate::ui::screens::enqueue::EnqueueScreen), whose
    /// footer at `src/ui/screens/enqueue.rs:126-133` echoes `ctx.input_buffer`
    /// — the characters the operator pressed, arriving through no file at all.
    ///
    /// # The widget family, NAMED — and why it decides the fixture
    ///
    /// That footer is a **`Paragraph`**. This module has already measured
    /// (see [`ProbeSink`]) that against ratatui 0.30.2 a `Paragraph` **DROPS**
    /// `U+200B`, `U+FEFF`, `U+00AD`, `U+202E`, `U+2062` and `U+2065` before a
    /// cell exists, while both families preserve the **tag block**.
    ///
    /// That measurement is not a footnote here, it is what makes this control
    /// able to fail at all. Had this test used a zero-width pair, the hostile
    /// direction would assert the absence of characters the widget discards on
    /// its own, and it would pass identically **whether or not the escape ran**
    /// — a control that cannot go red, which is precisely this round's own
    /// subject. So the fixture is [`TAG_PAIR`], whose hostile member carries
    /// `U+E0041`: a character this family PRESERVES, so its absence from the
    /// cells is evidence that the escape acted rather than that ratatui
    /// swallowed it.
    ///
    /// # Arrival before property
    ///
    /// The clean direction runs first and does double duty: it proves the
    /// fixture reaches the buffer at all, and it proves the escape did not
    /// rewrite a value that needed no rewriting. A fixture that never arrives
    /// would otherwise satisfy the hostile assertion trivially and be counted
    /// as coverage — the failure mode this whole module exists to remove.
    ///
    /// # The committed RED, verbatim
    ///
    /// Captured by `21-33`'s executor by temporarily reverting the escape at
    /// the echo — `Span::raw(ctx.input_buffer.clone())` in place of
    /// `Span::raw(crate::text::render_for_terminal(&ctx.input_buffer))` at
    /// `src/ui/screens/enqueue.rs:128` — then restoring it, with
    /// `git status --porcelain` clean afterwards:
    ///
    /// ```text
    /// thread 'ui::screens::render_escape_guard::tests::an_input_echo_screen_escapes_a_hostile_value_and_leaves_a_clean_one_alone' (3019460) panicked at src/ui/screens/render_escape_guard.rs:3252:9:
    /// EnqueueScreen rendered 1 invisible-formatting character(s) into the buffer: ['\u{e0041}']. What the operator typed reached a terminal cell unescaped, through the Paragraph footer at src/ui/screens/enqueue.rs:126-133.
    /// ```
    ///
    /// Note what that red proves beyond "the escape ran": the surviving
    /// character is `U+E0041`, a **tag** character, which is exactly the class
    /// the `Paragraph` family preserves. Had the fixture been a zero-width
    /// pair, the reverted escape would have produced no red at all — the
    /// widget would have dropped the character itself and the control would
    /// have been green against broken code.
    ///
    /// # What this does NOT cover, with its direction
    ///
    /// **One screen and one fixture pair.** `EnqueueScreen` was chosen because
    /// S1 named the input-echo CLASS, not because one screen stands for five:
    /// `AddProjectScreen`, `CreateProjectScreen`, `DriverInjectScreen` and
    /// `DriverStartScreen` are covered only by this module's per-screen
    /// behavioural probes, which assert arrival and escaping but not this
    /// test's clean-value no-over-escaping direction. **Direction:
    /// under-detection, silent, bounded by those probes and not by this test.**
    /// A single spot-check must not acquire a class-wide claim — that is this
    /// round's own subject, one level down.
    #[test]
    fn an_input_echo_screen_escapes_a_hostile_value_and_leaves_a_clean_one_alone() {
        // Imported BY INDEX, never respelled (D-21-6): a hand copy can drift
        // from the const, and this module's DEGENERATE uniqueness scan would
        // collide with a new invisible literal in this file.
        let (clean, hostile) = LOOK_ALIKE_PAIRS[TAG_PAIR];

        // The fixture is only meaningful if the hostile member actually carries
        // a member of the class. Asserted, not assumed.
        assert!(
            !invisible_chars(hostile).is_empty(),
            "the hostile member of LOOK_ALIKE_PAIRS[{TAG_PAIR}] ({hostile:?}) \
             carries no character satisfying `text::is_invisible_formatting_char`, \
             so the hostile direction below would pass on an empty property. \
             LOOK_ALIKE_PAIRS is addressed positionally and a reorder is enough \
             to cause this."
        );

        // ---- Direction 1: ARRIVAL, and no over-escaping. ----
        //
        // Asserted FIRST. A clean value must reach the cells byte-identical:
        // the escape is for hostile input, and a version that rewrote ordinary
        // typing would be a regression the hostile direction alone cannot see.
        let clean_text = enqueue_echo_text(clean);
        assert!(
            clean_text.contains(clean),
            "the clean value {clean:?} did not reach the rendered buffer at \
             all, so nothing below is evidence of anything. Either the fixture \
             never arrived — the echo stopped rendering `ctx.input_buffer`, or \
             the footer moved — or the escape REWROTE a clean value that \
             needed no rewriting. Those are different defects and both are \
             failures; check the buffer to tell them apart."
        );

        // ---- Direction 2: the hostile property. ----
        let hostile_text = enqueue_echo_text(hostile);
        let survivors = invisible_chars(&hostile_text);
        assert!(
            survivors.is_empty(),
            "EnqueueScreen rendered {} invisible-formatting character(s) into \
             the buffer: {survivors:?}. What the operator typed reached a \
             terminal cell unescaped, through the Paragraph footer at \
             src/ui/screens/enqueue.rs:126-133.",
            survivors.len()
        );

        // The escape must have ACTED, not merely produced a clean buffer. The
        // expected escaped form is computed with the same `display_identity`
        // this module already uses as its oracle, rather than by a second
        // spelling that could disagree with it.
        let expected = display_identity(hostile);
        assert!(
            hostile_text.contains(&expected),
            "the buffer carries no invisible-formatting character, but it does \
             not carry the expected escaped form {expected:?} either. That \
             combination means the hostile value was DROPPED rather than \
             escaped, and an absence assertion over a dropped value is vacuous. \
             Buffer excerpt: {:?}",
            hostile_text
                .lines()
                .find(|line| line.contains("Enqueue>"))
                .unwrap_or("<no footer line found>")
        );
    }
}
