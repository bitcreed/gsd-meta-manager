//! The TUI's render surface, closed by **derivation** rather than by a list.
//!
//! Seven rounds of this phase each closed one enumeration level by naming the
//! next set of sites by hand, and each time the set that was named turned out to
//! be a subset of the set that existed. CR-01 is that failure one more time and
//! at its most embarrassing: the escaping mechanism ([`crate::text::display_identity`])
//! was correct, but the single UI call to it lived in `src/ui/project_list.rs` —
//! a file the module tree does not contain and the build never compiled — and
//! that dead call convinced both a reviewer and a verifier that the TUI was
//! escaped.
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

const SCREEN_IDENTITY_DISPOSITIONS: &[DispositionRow] = &[(
    "DeleteConfirmScreen",
    "src/ui/screens/delete_confirm.rs",
    RENDERS_IDENTITY,
    "Draws the registry key of the project about to be unregistered into a \
     destructive [y/n] prompt, and again into the removal toast. This is the \
     highest-consequence identity render in the tree: the operator confirms the \
     name they READ, so a rendered name that is not the key is a confirmation \
     of a different thing than was asked (T-21-21-01).",
)];

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

/// Build the context and the screen that a probe run renders.
type Fixture = fn(&str) -> (AppContext, Box<dyn Screen>);

/// The constructor for each adjudicated implementor.
///
/// This is a mapping from a derived name to a way of building the thing — it is
/// **not** an enumeration, and it cannot become one: every disposition row must
/// have an arm here (`every_adjudicated_screen_has_a_probe_fixture`), and every
/// disposition row must have come from the walk. A screen with no arm is a red
/// test, not a silent skip.
fn fixture_for(type_name: &str) -> Option<Fixture> {
    match type_name {
        "DeleteConfirmScreen" => Some(|identity| {
            let ctx = super::tests::ctx_with_aliases(&[identity]);
            let screen: Box<dyn Screen> = Box::new(
                super::delete_confirm::DeleteConfirmScreen::new(identity.to_string()),
            );
            (ctx, screen)
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
    let mut terminal =
        Terminal::new(TestBackend::new(PROBE_WIDTH, PROBE_HEIGHT)).expect("TestBackend terminal");
    terminal
        .draw(|frame| {
            let area = frame.area();
            screen.render(frame, area, ctx);
        })
        .expect("draw the screen under probe");
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

    /// The live census: the walk's derived set equals the disposition table,
    /// both ways.
    #[test]
    #[ignore = "red until Task 2 adjudicates the remaining ten implementors — \
                deny-by-default working as designed"]
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
    /// 1. **ratatui 0.30's `Buffer` DROPS zero-width graphemes before a cell
    ///    exists.** `U+00AD` is simply gone from the rendered row. So the TUI
    ///    does not *reorder* a hostile key — it silently *deletes* bytes, and
    ///    the legacy key renders as a DIFFERENT string that can collide with a
    ///    real project of that name. `CONTAINS RAW: false` is the direct
    ///    consequence: **an assertion that the raw form is absent passes
    ///    vacuously against an unescaped site, and would go on passing
    ///    forever.**
    /// 2. **The tag block SURVIVES.** `U+E0041` — the LLM ASCII-smuggling
    ///    carrier — reached a terminal cell intact. That is the one class that
    ///    arrives whole, and it is what assertion 3 catches.
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
    #[ignore = "red: CR-01 probe; un-ignored in the fix commit"]
    fn the_screen_renders_identity_escaped() {
        let clean = clean_identity();
        let hostile = hostile_identity();
        let escaped = display_identity(&hostile);

        for (name, path, disposition, _reason) in SCREEN_IDENTITY_DISPOSITIONS {
            let Some(build) = fixture_for(name) else {
                panic!("{name} ({path}) has no probe fixture");
            };

            let (clean_ctx, clean_screen) = build(&clean);
            let clean_text = render_to_text(clean_screen.as_ref(), &clean_ctx);
            let (hostile_ctx, hostile_screen) = build(&hostile);
            let hostile_text = render_to_text(hostile_screen.as_ref(), &hostile_ctx);

            match *disposition {
                RENDERS_IDENTITY => {
                    // 1. ARRIVAL, before any property. A screen that rendered
                    //    nothing, or that was built in a state showing no
                    //    identity, fails HERE rather than passing by silence.
                    assert!(
                        clean_text.contains(clean.as_str()),
                        "{name} ({path}) is adjudicated as rendering identity, \
                         but a clean identity handed to its fixture never \
                         reached the buffer. Either the disposition is wrong or \
                         the fixture does not put the screen in a state that \
                         renders it — and until this passes, every assertion \
                         below it would pass by silence."
                    );
                    // 2. The escaped form of the hostile twin is what a human
                    //    reads.
                    assert!(
                        hostile_text.contains(escaped.as_str()),
                        "{name} ({path}) did not render {escaped:?}. Route what \
                         a human READS through crate::text::display_identity; \
                         the value used for lookups, map keys, path segments, \
                         comparisons and persistence stays RAW."
                    );
                }
                RENDERS_NO_IDENTITY => {
                    // The disposition is CHECKED, not claimed: if the screen
                    // really draws no identity, the clean stem cannot be in its
                    // buffer.
                    assert!(
                        !clean_text.contains(clean.as_str()),
                        "{name} ({path}) is adjudicated as rendering no \
                         identity, but it drew one. The row is wrong: re-read \
                         the render, re-state the adjudication, and escape what \
                         a human reads."
                    );
                    assert!(
                        clean_text.chars().any(|c| !c.is_whitespace()),
                        "{name} ({path}) rendered a blank buffer, so its \
                         absence assertion above proved nothing"
                    );
                }
                other => panic!("{name} ({path}) carries an unknown disposition {other:?}"),
            }

            // 3. Applies to EVERY implementor whatever its disposition: the tag
            //    block U+E0000-U+E007F reaches a terminal cell INTACT (measured
            //    — see this test's doc), and a cell holding it is a cell the
            //    operator cannot see (T-21-21-02).
            assert!(
                invisible_chars(&hostile_text).is_empty(),
                "{name} ({path}) rendered {:?} into the terminal buffer. Those \
                 characters render as nothing, so what the operator reads is not \
                 what the value is.",
                invisible_chars(&hostile_text)
            );
        }
    }
}
