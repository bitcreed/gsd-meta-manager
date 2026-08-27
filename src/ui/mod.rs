pub mod roadmap_widget;
pub mod screens;

use crate::app::App;
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    if let Some(screen) = app.screen_stack.last() {
        screen.render(frame, area, &app.ctx);
    }
}

/// The census that makes `crate::text::render_for_terminal`'s ONE-composition
/// claim checkable over `src/ui/`, plus the two-direction pin that proves the
/// conversion is neither a no-op nor a behaviour change (WR-05, T-21-29-02).
#[cfg(test)]
mod tests {
    use crate::test_support::LOOK_ALIKE_PAIRS;
    // **Imported under an alias on purpose.** The census below matches the
    // literal un-composed call construction in every `.rs` file under `src/ui/`,
    // and `src/ui/mod.rs` is one of those files — a direct call here would make
    // the census report itself. The alias is also this census's first named
    // residual; see `every_render_site_under_ui_composes_both_classes`.
    use crate::text::display_identity as invisible_class_half;
    use crate::text::{render_for_terminal, strip_terminal_controls};

    // -----------------------------------------------------------------------
    // The needle, assembled at runtime from halves that are meaningless apart
    // -----------------------------------------------------------------------

    /// The head of the un-composed call construction the census looks for.
    ///
    /// Spelled as two halves for exactly the reason `text.rs`'s
    /// `ALPHABET_CLAUSE_HEAD`/`ALPHABET_CLAUSE_TAIL` are, and
    /// `render_escape_guard`'s `IMPL_HEAD`/`IMPL_TAIL` are: the census walks
    /// `src/ui/`, and this module is under `src/ui/`. Written as one literal,
    /// this const's own line would be a hit and the census would count itself.
    /// [`the_census_cannot_report_itself`] asserts that property rather than
    /// trusting this paragraph.
    const CALL_HEAD: &str = "display_";
    /// The tail of [`CALL_HEAD`].
    const CALL_TAIL: &str = "identity(";

    /// The head of the marker that makes a call a COMPOSITION rather than a
    /// violation. Split for the same anti-self-match reason as [`CALL_HEAD`].
    const COMPOSED_HEAD: &str = "sanitize_";
    /// The tail of [`COMPOSED_HEAD`].
    const COMPOSED_TAIL: &str = "render_line";

    /// How many physical lines a wrapped call may span before the join gives up.
    const CALL_JOIN_LINES: usize = 4;

    /// A path the census does not report, and the reason it does not.
    ///
    /// `inside_walk` is what makes the list self-cleaning: an exemption that is
    /// under `src/ui/` and no longer contains a matching un-composed call is
    /// STALE and is reported by [`stale_exemptions`], so the list cannot quietly
    /// grow into a place where a real violation hides. An exemption that is
    /// outside the walk is not subject to that check — being outside the walk is
    /// its whole reason — and is listed only so a reader does not have to
    /// rediscover why it is absent.
    struct Exemption {
        path: &'static str,
        reason: &'static str,
        inside_walk: bool,
    }

    /// **Exactly two exemptions, each named with its reason (D-21-44).**
    ///
    /// The list is deliberately short and deliberately not a list of files that
    /// happen to be inconvenient. Widening it at execution time to make a
    /// violation pass is the failure this whole census exists to prevent.
    const EXEMPTIONS: [Exemption; 2] = [
        Exemption {
            path: "src/ui/screens/render_escape_guard.rs",
            // Worded WITHOUT the literal call construction: a reason string on
            // an executable line that spelled it would itself be a census hit.
            reason: "the probe module. It computes the EXPECTED escaped form of a \
                     hostile fixture and asserts a rendered screen buffer against \
                     it, so the bare escape there is an oracle, not a render \
                     site. Converting it would make the probe assert the \
                     implementation against itself.",
            inside_walk: true,
        },
        Exemption {
            path: "src/text.rs",
            reason: "the definition itself, and the ONE composition that wraps \
                     it. Outside the `src/ui/` walk, so it is never reached; \
                     listed here so its absence is a stated exemption rather \
                     than something a reader has to rediscover.",
            inside_walk: false,
        },
    ];

    /// A violation this wave's OTHER plans own, recorded rather than absorbed.
    ///
    /// **Why this is not an exemption.** `21-29` may not edit
    /// `src/ui/screens/driver_confirm.rs` — it is fenced to `21-28`, which runs
    /// in a PARALLEL worktree in this same wave and carries the matching
    /// acceptance criterion for that exact site. Subtracting it here is what
    /// makes the equality below merge-safe in both directions: it is zero in
    /// this worktree, where the site is still un-composed, and it is still zero
    /// after `21-28` merges, where the site is gone.
    ///
    /// **Its failure direction, stated rather than relied on.** An entry is
    /// pinned to one exact `path:line` pair, so it hides at most that one line
    /// and cannot absorb a new violation elsewhere in the file. When the owning
    /// plan merges, the entry becomes STALE — [`satisfied_pending`] REPORTS it
    /// (printed by the census, quoted in the phase's post-merge findings) rather
    /// than failing, because a red here would break the merged tree for a
    /// bookkeeping reason. **Direction: under-detection, one line wide,
    /// reported-not-red.** Removing a satisfied entry is a post-merge step.
    ///
    /// **Currently empty, and that is the post-merge steady state.** The one
    /// entry this list ever held — `src/ui/screens/driver_confirm.rs:275`,
    /// owned by `21-28` — was reported `SATISFIED WAVE-PENDING` by the census
    /// once `21-28` merged, and removed here by the wave-1 post-merge step. The
    /// census now hides no line at all: nothing is subtracted from the equality
    /// below, so every un-composed site under `src/ui/` is counted.
    const WAVE_PENDING: [(&str, &str); 0] = [];

    // -----------------------------------------------------------------------
    // The walk
    // -----------------------------------------------------------------------

    /// Every `.rs` file under `dir`, recursively, as `(relative path, lines)`.
    ///
    /// The recursive `read_dir` shape follows `text::tests::collect_rs` and
    /// `screens::render_escape_guard::collect`: an unreadable entry is skipped
    /// rather than panicked on, and paths are relative to `CARGO_MANIFEST_DIR`.
    fn collect_rs(
        dir: &std::path::Path,
        base: &std::path::Path,
        out: &mut Vec<(String, Vec<(usize, String)>)>,
    ) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                collect_rs(&path, base, out);
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

    fn ui_sources() -> Vec<(String, Vec<(usize, String)>)> {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut files = Vec::new();
        collect_rs(&base.join("src").join("ui"), &base, &mut files);
        files.sort_by(|a, b| a.0.cmp(&b.0));
        files
    }

    /// Whether `logical` is syntactically unfinished, so the next physical line
    /// belongs to the same call.
    ///
    /// A call is joined forward only while its parentheses are unbalanced, which
    /// is what makes a wrapped COMPOSITION —
    /// `..(\n    &..(value),\n)` — read as one unit and not be
    /// mis-reported. It is also why this module's own `CALL_TAIL` line, whose
    /// string literal carries a lone `(`, never starts a join: the join is only
    /// ever attempted from a line that already carries the WHOLE call needle,
    /// and that const's line carries half of it.
    fn continues_onto_the_next_line(logical: &str) -> bool {
        logical.matches('(').count() > logical.matches(')').count()
    }

    /// The un-composed call sites under `src/ui/`, as `path:line`, paired with
    /// the logical line that produced each.
    ///
    /// A line is a violation when it carries the call needle and the logical
    /// unit starting at it does NOT also name the composition marker. Comment
    /// lines are dropped before anything else, so the doc mentions in
    /// `screens/mod.rs` and `screens/detail.rs` are not hits — the rule is about
    /// what EXECUTES.
    fn census(files: &[(String, Vec<(usize, String)>)]) -> Vec<(String, String)> {
        let call = format!("{CALL_HEAD}{CALL_TAIL}");
        let composed = format!("{COMPOSED_HEAD}{COMPOSED_TAIL}");
        let mut sites = Vec::new();
        for (path, lines) in files {
            for (index, (number, line)) in lines.iter().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                if !line.contains(&call) {
                    continue;
                }
                let mut logical = line.clone();
                let mut taken = 1;
                let mut ahead = index;
                while taken < CALL_JOIN_LINES
                    && ahead + 1 < lines.len()
                    && continues_onto_the_next_line(&logical)
                {
                    ahead += 1;
                    taken += 1;
                    let next = lines[ahead].1.trim();
                    if next.starts_with("//") {
                        continue;
                    }
                    logical.push(' ');
                    logical.push_str(next);
                }
                if logical.contains(&composed) {
                    continue;
                }
                sites.push((format!("{path}:{number}"), logical));
            }
        }
        sites
    }

    /// Exemptions that are inside the walk and no longer shield anything.
    ///
    /// Extracted so the live assertion and its control drive the SAME function:
    /// a report that is only ever exercised by the tree it certifies is a report
    /// nobody has seen work.
    fn stale_exemptions(
        files: &[(String, Vec<(usize, String)>)],
        exemptions: &[Exemption],
    ) -> Vec<String> {
        let raw = census(files);
        exemptions
            .iter()
            .filter(|exemption| exemption.inside_walk)
            .filter(|exemption| {
                !raw.iter()
                    .any(|(site, _)| site.starts_with(&format!("{}:", exemption.path)))
            })
            .map(|exemption| {
                format!(
                    "STALE EXEMPTION: {} no longer contains an un-composed call, so the \
                     exemption shields nothing and must be removed. Its recorded reason was: {}",
                    exemption.path, exemption.reason
                )
            })
            .collect()
    }

    /// Wave-pending entries whose site is no longer a violation.
    fn satisfied_pending(raw: &[(String, String)]) -> Vec<String> {
        WAVE_PENDING
            .iter()
            .filter(|(site, _)| !raw.iter().any(|(found, _)| found == site))
            .map(|(site, owner)| {
                format!(
                    "SATISFIED WAVE-PENDING: {site} is no longer un-composed — {owner} has \
                     landed. Remove the entry; keeping it hides that one line."
                )
            })
            .collect()
    }

    // -----------------------------------------------------------------------
    // The claim, as a control
    // -----------------------------------------------------------------------

    /// `crate::text::render_for_terminal`'s doc calls itself **the ONE
    /// composition of the two classes, resolved once so that no consumer
    /// re-decides which halves apply**. This makes that claim checkable over
    /// `src/ui/` rather than believed.
    ///
    /// # Why an EQUALITY on a count and not an `is_empty()`
    ///
    /// The same reason `text::tests::exactly_one_executable_spelling_..` gives:
    /// an equality's failure message can name the offending sites and the count,
    /// and a count that drifts by one is as much a finding as a count that
    /// drifts by nine. The claim was measured FALSE of this tree before this
    /// plan: nine files under `src/ui/` drew `.planning/`-derived and
    /// registry-derived values through the invisible-formatting half alone.
    ///
    /// # Committed RED, verbatim
    ///
    /// Against the tree at `80bc4c1`, before any conversion,
    /// `cargo test --lib -- ui::tests::every_render_site --nocapture`:
    ///
    /// ```text
    /// thread 'ui::tests::every_render_site_under_ui_composes_both_classes' (1996514) panicked at src/ui/mod.rs:374:9:
    /// assertion `left == right` failed: 23 executable call sites under src/ui/ apply the invisible-formatting half alone, and `crate::text::render_for_terminal`'s doc claims to be THE one composition of the two classes. Sites: ["src/ui/roadmap_widget.rs:131", "src/ui/roadmap_widget.rs:136", "src/ui/screens/add_project.rs:243", "src/ui/screens/add_project.rs:256", "src/ui/screens/create_project.rs:88", "src/ui/screens/create_project.rs:89", "src/ui/screens/create_project.rs:265", "src/ui/screens/create_project.rs:274", "src/ui/screens/delete_confirm.rs:90", "src/ui/screens/delete_confirm.rs:129", "src/ui/screens/delete_confirm.rs:183", "src/ui/screens/driver_inject.rs:197", "src/ui/screens/driver_start.rs:340", "src/ui/screens/driver_start.rs:352", "src/ui/screens/driver_start.rs:356", "src/ui/screens/enqueue.rs:124", "src/ui/screens/normal.rs:697", "src/ui/screens/normal.rs:709", "src/ui/screens/normal.rs:749", "src/ui/screens/normal.rs:763", "src/ui/screens/normal.rs:779", "src/ui/screens/normal.rs:1003", "src/ui/screens/queue_delete_confirm.rs:104"]. ...
    ///   left: 23
    ///  right: 0
    /// ```
    ///
    /// Twenty-three sites, in exactly the nine files this plan converted — the
    /// claim measured false of the tree rather than argued to be.
    ///
    /// And again, AFTER the conversions, by PLANTING one bare call in
    /// `src/ui/screens/help.rs` — a file this plan does not otherwise touch:
    ///
    /// ```text
    /// CENSUS raw un-composed sites under src/ui/ (3): ["src/ui/screens/driver_confirm.rs:275", "src/ui/screens/help.rs:182", "src/ui/screens/render_escape_guard.rs:2019"]
    ///
    /// thread 'ui::tests::every_render_site_under_ui_composes_both_classes' (2088034) panicked at src/ui/mod.rs:377:9:
    /// assertion `left == right` failed: 1 executable call sites under src/ui/ apply the invisible-formatting half alone ... Sites: ["src/ui/screens/help.rs:182"]
    ///   left: 1
    ///  right: 0
    /// ```
    ///
    /// The plant was then REWRITTEN as a WRAPPED composition at the same site —
    /// the call on one physical line and `sanitize_render_line` on the next —
    /// and the census reported it no longer, which is the join and the non-ban
    /// direction measured at the same time. Both plants were removed and
    /// `git status --porcelain` showed a clean tree. All three reds are quoted
    /// in full in `21-29-SUMMARY.md`.
    ///
    /// # What this census does NOT claim (T-21-29-01, prohibition 2)
    ///
    /// **An un-composed site here was a false CLAIM, not a live leak, and the
    /// reason is DEPENDENCY BEHAVIOUR that this tree asserts nowhere.** ratatui
    /// 0.30 filters `char::is_control` graphemes in both
    /// `Span::styled_graphemes` and `Buffer::set_stringn`, so the control class
    /// cannot reach a terminal cell today through any widget family. That is why
    /// the round-10 review rated the nine half-escaped files a WARNING rather
    /// than a BLOCKER, and inflating it into a live leak would be the same
    /// overclaim this phase exists to end.
    ///
    /// **It does not travel, and that is the residual.** `ctx.error_message` and
    /// `ctx.status_message` are also produced by non-TUI paths, where no ratatui
    /// filter exists. **Direction: under-detection if the dependency changes,
    /// silent** — nothing in this tree goes red if a future ratatui stops
    /// filtering, because nothing in this tree asserts that it does. The
    /// standing obligation to track ratatui's grapheme filtering is already
    /// recorded in this phase's `deferred-items.md`; it is pointed at here
    /// rather than duplicated, so there is one entry to keep current.
    ///
    /// # The census's own residuals, with their directions
    ///
    /// 1. **A call made through a local alias or a re-export is invisible.**
    ///    This module's own test imports the invisible-formatting half under an
    ///    alias, which is a live demonstration of the gap rather than a
    ///    hypothetical. **Under-detection, silent.** What bounds it is the same
    ///    thing that bounds `text.rs`'s census: one composition every render
    ///    site calls, not a textual scan.
    /// 2. **A composition assembled across more than [`CALL_JOIN_LINES`]
    ///    physical lines** reads as un-composed. **Over-detection, loud** — it
    ///    fails the build and names the site, which is the safe direction.
    #[test]
    fn every_render_site_under_ui_composes_both_classes() {
        let files = ui_sources();
        assert!(
            !files.is_empty(),
            "the census walked src/ui/ and found no Rust source at all, so a \
             clean result here would be a walk that never looked"
        );

        let raw = census(&files);

        // Printed, not just asserted: the RAW list is what a reader needs to see
        // what the census says about the files OTHER plans in this wave own, and
        // a number nobody can reproduce is the shape this phase exists to end.
        println!(
            "CENSUS raw un-composed sites under src/ui/ ({}): {:?}",
            raw.len(),
            raw.iter().map(|(site, _)| site).collect::<Vec<_>>()
        );

        for report in satisfied_pending(&raw) {
            // Reported, never red: see WAVE_PENDING's doc for why, and for the
            // direction this costs.
            println!("{report}");
        }

        let outstanding: Vec<String> = raw
            .iter()
            .map(|(site, _)| site.clone())
            .filter(|site| {
                !EXEMPTIONS
                    .iter()
                    .any(|exemption| site.starts_with(&format!("{}:", exemption.path)))
            })
            .filter(|site| !WAVE_PENDING.iter().any(|(pending, _)| pending == site))
            .collect();

        assert_eq!(
            outstanding.len(),
            0,
            "{} executable call sites under src/ui/ apply the \
             invisible-formatting half alone, and `crate::text::render_for_terminal`'s \
             doc claims to be THE one composition of the two classes. Sites: {outstanding:?}. \
             A site that applies one half is open in the other direction: \
             `General_Category=Cf` union `Default_Ignorable_Code_Point` is not `Cc`, and \
             `ESC` is `Cc` and in neither of the first two. The repair is to call \
             `crate::text::render_for_terminal`, or to compose on the same logical line \
             with `{COMPOSED_HEAD}{COMPOSED_TAIL}` where the display cap is also wanted — \
             not to soften the claim.",
            outstanding.len()
        );
    }

    /// **The non-ban arm (prohibition 1, D-21-44).**
    ///
    /// The rule is about the un-composed CONSTRUCTION, not about an identifier.
    /// A census that forbade the identifier would be satisfied by the next
    /// author with a rename, which is strictly worse than the defect it
    /// replaced. So a real in-tree COMPOSITION must not be reported — and the
    /// fixture is read off disk rather than spelled here, so it is the tree's
    /// composition and not a copy of one that could drift from it.
    #[test]
    fn the_census_does_not_report_a_composed_call() {
        let files = ui_sources();
        let raw = census(&files);
        let call = format!("{CALL_HEAD}{CALL_TAIL}");
        let composed = format!("{COMPOSED_HEAD}{COMPOSED_TAIL}");

        let mut compositions: Vec<String> = Vec::new();
        for (path, lines) in &files {
            for (number, line) in lines {
                if !line.trim_start().starts_with("//")
                    && line.contains(&call)
                    && line.contains(&composed)
                {
                    compositions.push(format!("{path}:{number}"));
                }
            }
        }

        assert!(
            !compositions.is_empty(),
            "no in-tree composition was found to drive this arm, so a green \
             result would prove nothing. The deliberate second composition lives \
             in `screens/driver.rs` and `screens/driver_confirm.rs`, which cap \
             agent prose as well as escaping it."
        );

        for site in &compositions {
            assert!(
                !raw.iter().any(|(found, _)| found == site),
                "the census reported {site}, which composes both classes on one \
                 logical line. That would make the rule a ban on an identifier \
                 rather than on the un-composed construction, and the next author \
                 would satisfy it with a rename. Reported: {:?}",
                raw.iter().map(|(site, _)| site).collect::<Vec<_>>()
            );
        }
    }

    /// The anti-self-match property, ASSERTED rather than argued.
    ///
    /// The needle halves above are meaningless apart, which is what stops this
    /// module — walked like every other file under `src/ui/` — from becoming a
    /// hit and certifying itself. `text.rs`'s census learned by measurement that
    /// this property can be broken by JOINING two adjacent halves, so the join
    /// is only ever attempted from a line already carrying the WHOLE needle.
    #[test]
    fn the_census_cannot_report_itself() {
        let files = ui_sources();
        let raw = census(&files);
        assert!(
            !raw.iter().any(|(site, _)| site.starts_with("src/ui/mod.rs:")),
            "the census reported its own module. Its needle halves have been \
             respelled into something that matches this file, so every future \
             green is the census certifying itself. Reported: {:?}",
            raw.iter().map(|(site, _)| site).collect::<Vec<_>>()
        );
    }

    /// **The exemption list is self-cleaning, and the report is exercised.**
    ///
    /// An exemption that stops shielding anything is a hole with a reason
    /// attached to it — the shape a violation hides in three rounds later. This
    /// drives the same [`stale_exemptions`] the live census consumes: once with
    /// a SYNTHETIC exemption on a file under the walk that contains no matching
    /// call, which must be reported, and once with the real list, which must be
    /// clean.
    #[test]
    fn a_stale_exemption_is_reported() {
        let files = ui_sources();

        let synthetic = [Exemption {
            path: "src/ui/screens/help.rs",
            reason: "a synthetic exemption on a file with nothing to exempt",
            inside_walk: true,
        }];
        let reported = stale_exemptions(&files, &synthetic);
        assert_eq!(
            reported.len(),
            1,
            "a synthetic exemption on a file that contains no un-composed call \
             must be reported as stale; got {reported:?}"
        );
        assert!(
            reported[0].contains("src/ui/screens/help.rs"),
            "the stale report must name the path; got {reported:?}"
        );
        println!("{}", reported[0]);

        assert!(
            stale_exemptions(&files, &EXEMPTIONS).is_empty(),
            "an exemption in the live list no longer shields anything: {:?}",
            stale_exemptions(&files, &EXEMPTIONS)
        );
    }

    // -----------------------------------------------------------------------
    // The two-direction conversion pin
    // -----------------------------------------------------------------------

    /// A control-class fixture: `ESC`, `TAB`, `DEL` and a C1 byte, none of which
    /// the invisible-formatting half answers for.
    const CONTROL_FIXTURES: [&str; 4] = [
        "demo\u{1b}[31mred",
        "a\tb",
        "x\u{7f}y",
        "gsd\u{9b}run",
    ];

    /// **Both directions, because one of them alone proves nothing.**
    ///
    /// The conversion this plan performed replaced
    /// `<invisible-formatting half>(v)` with `render_for_terminal(v)`, which is
    /// that half composed over `strip_terminal_controls`. Without the FIRST
    /// direction the conversion could have changed what clean values render as,
    /// silently; without the SECOND it could have been a no-op at all nine files
    /// and nothing in the tree would have said so — which is exactly how a
    /// half-applied promote survives.
    #[test]
    fn the_conversion_is_a_no_op_on_clean_values_and_is_not_on_control_values() {
        let mut clean: Vec<&str> = Vec::new();
        for (visible, look_alike) in LOOK_ALIKE_PAIRS {
            clean.push(visible);
            clean.push(look_alike);
        }
        clean.extend([
            "demo",
            "2.1",
            "20",
            "/gsd:progress",
            "2026-08-19T12-00-00Z-aaaa",
            "clean /tmp/project",
        ]);

        for value in &clean {
            assert_eq!(
                render_for_terminal(value).as_ref(),
                invisible_class_half(value),
                "{value:?} carries no control character, so the added half is \
                 the identity on it and the conversion must be \
                 behaviour-preserving. If this fails, converting nine render \
                 sites changed what the dashboard shows for ordinary values."
            );
            assert_eq!(
                strip_terminal_controls(value),
                *value,
                "{value:?} was chosen as a CLEAN fixture; if the control pass \
                 changes it, the equality above is vacuous"
            );
        }

        for value in CONTROL_FIXTURES {
            assert_ne!(
                render_for_terminal(value).as_ref(),
                invisible_class_half(value),
                "{value:?} carries a control character, and the conversion must \
                 render it DIFFERENTLY. If this passes as an equality, the \
                 conversion is a no-op at every one of the nine files and the \
                 census would be certifying a rename."
            );
        }
    }

    /// **The two-direction property at a REAL render site, in a real terminal
    /// buffer**, spot-checked on `roadmap_widget.rs` — one of the eight files
    /// converted for the same reason `normal.rs` was.
    ///
    /// The generic pin above compares two function results. This renders the
    /// widget the operator actually looks at into a `ratatui::buffer::Buffer`
    /// and compares CELLS, which is the only level at which the claim "the
    /// operator can see what is there" means anything.
    ///
    /// **It also measures the dependency behaviour this census refuses to rely
    /// on silently.** ratatui filters `char::is_control` graphemes before a cell
    /// exists, so an un-composed site DROPS a raw `ESC`/`DEL`/C1 and renders as
    /// though it were not there — which is precisely why the un-composed sites
    /// were a false claim rather than a live leak. After the conversion those
    /// same bytes arrive as `strip_terminal_controls`' visible replacement, so
    /// the cells differ. Both directions are asserted; either alone would be
    /// satisfiable by a no-op.
    #[test]
    fn the_roadmap_widget_renders_a_clean_phase_name_unchanged_and_a_control_one_differently() {
        use crate::state_reader::roadmap_md::RoadmapPhase;
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;
        use ratatui::widgets::Widget;

        fn phase(name: &str, number: &str) -> RoadmapPhase {
            RoadmapPhase {
                number: number.to_string(),
                name: name.to_string(),
                description: String::new(),
                completed: false,
                total_plans: 3,
                completed_plans: 1,
                depends_on: Vec::new(),
            }
        }

        fn rendered(name: &str, number: &str) -> Buffer {
            let area = Rect::new(0, 0, 60, 8);
            let mut buffer = Buffer::empty(area);
            let phases = [phase(name, number)];
            super::roadmap_widget::RoadmapWidget {
                phases: &phases,
                current_phase_num: 21,
                scroll_offset: 0,
            }
            .render(area, &mut buffer);
            buffer
        }

        fn text_of(buffer: &Buffer) -> String {
            buffer
                .content()
                .iter()
                .map(ratatui::buffer::Cell::symbol)
                .collect()
        }

        // **The dependency behaviour, MEASURED here rather than quoted from a
        // changelog.** ratatui drops `char::is_control` graphemes before a cell
        // exists, so BEFORE the conversion a phase number carrying `U+009B` was
        // indistinguishable in the cells from one that did not. That is what
        // made an un-composed render site a false CLAIM rather than a live leak
        // — and it is a property of the dependency, not of this tree.
        let mut with_c1 = Buffer::empty(Rect::new(0, 0, 20, 1));
        with_c1.set_string(0, 0, "2\u{9b}1", ratatui::style::Style::default());
        let mut without_c1 = Buffer::empty(Rect::new(0, 0, 20, 1));
        without_c1.set_string(0, 0, "21", ratatui::style::Style::default());
        assert_eq!(
            with_c1, without_c1,
            "ratatui is expected to drop the C1 grapheme outright, which is the \
             stated basis for calling an un-composed site a claim defect rather \
             than a live leak. If this ever fails the dependency has changed and \
             the residual named in this module's census doc has moved — that is \
             the whole point of measuring it here."
        );

        // Direction 1 — behaviour-preserving. A clean phase name and number
        // reach the cells exactly as they did before the conversion, because
        // `strip_terminal_controls` is the identity on both.
        let clean_name = "Injection Hardening";
        let clean_number = "21";
        assert_eq!(
            strip_terminal_controls(clean_name),
            clean_name,
            "the clean fixture must be a fixed point of the added half, or \
             direction 1 is vacuous"
        );
        let clean = rendered(clean_name, clean_number);
        let clean_text = text_of(&clean);
        assert!(
            clean_text.contains(&invisible_class_half(clean_name)),
            "a clean phase name must reach the cells exactly as the \
             pre-conversion half rendered it; got {clean_text:?}"
        );
        assert!(
            clean_text.contains("P21:"),
            "a clean phase number must reach the cells unchanged; got \
             {clean_text:?}"
        );

        // Direction 2 — NOT a no-op. `DEL` in the name and `U+009B` in the
        // number now reach the cells as the visible `CONTROL_REPLACEMENT`,
        // where the measurement above shows they previously reached them as
        // NOTHING AT ALL. Without this direction the conversion could have been
        // a no-op at all nine files and no test would have said so.
        let hostile = rendered("Injection\u{7f} Hardening", "2\u{9b}1");
        assert_ne!(
            hostile, clean,
            "a phase name carrying DEL and a number carrying U+009B must reach \
             the cells DIFFERENTLY from their control-free twins once both \
             classes are composed. Equality here means the conversion is a \
             no-op at this render site."
        );
        let hostile_text = text_of(&hostile);
        assert!(
            hostile_text.contains(crate::text::CONTROL_REPLACEMENT),
            "the control characters must arrive as the visible replacement, so \
             the operator can see that something was there; got {hostile_text:?}"
        );
    }

    /// The same two directions at `queue_delete_confirm.rs`'s prompt — the
    /// second of the eight files spot-checked, and a `.planning/queue.md`-derived
    /// value rather than a `ROADMAP.md`-derived one.
    ///
    /// This drives [`super::screens::queue_delete_confirm::prompt_text`], the
    /// function that produces the exact string the confirm footer draws, so the
    /// escape, the `char`-wise truncation and the cap are all exercised together
    /// rather than around.
    #[test]
    fn the_queue_delete_prompt_is_unchanged_for_a_clean_command_and_changed_for_a_control_one() {
        use super::screens::queue_delete_confirm::prompt_text;

        /// The prompt exactly as it was built BEFORE this plan converted the
        /// site — the invisible-formatting half alone, same cap, same
        /// `char`-wise truncation. It is the oracle both directions compare
        /// against; without it the test would compare the new code to itself.
        fn prompt_text_before_the_conversion(command_text: &str) -> String {
            const CAP: usize = 50;
            const KEEP: usize = 47;
            let escaped = invisible_class_half(command_text);
            let display_text = if escaped.chars().count() > CAP {
                format!("{}...", escaped.chars().take(KEEP).collect::<String>())
            } else {
                escaped
            };
            format!("  Remove \"{}\" from queue? [y/n]", display_text)
        }

        let clean = "/gsd:execute-phase 21";
        assert_eq!(
            prompt_text(clean),
            prompt_text_before_the_conversion(clean),
            "a clean queued command must produce a byte-identical prompt before \
             and after the added control-stripping half"
        );

        // `\u{9b}` is the C1 CSI — a one-codepoint equivalent of an `ESC`-led
        // introducer, which `strip_terminal_controls` replaces with the visible
        // `CONTROL_REPLACEMENT` and the old half passed through untouched.
        let hostile = "/gsd:execute-phase\u{9b}2K 21";
        assert_ne!(
            prompt_text(hostile),
            prompt_text_before_the_conversion(hostile),
            "a queued command carrying a C1 control must produce a DIFFERENT \
             prompt once both classes are composed; equality here would mean the \
             conversion is a no-op at this render site"
        );
        assert!(
            prompt_text_before_the_conversion(hostile).contains('\u{9b}'),
            "the oracle must actually carry the control character through, or \
             the inequality above proves nothing"
        );
        assert!(
            !prompt_text(hostile).contains('\u{9b}'),
            "no C1 control may survive into the prompt a human confirms"
        );
    }
}
