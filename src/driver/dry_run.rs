//! The four-section preview a `drive --dry-run` prints, and nothing else.
//!
//! This is the only place in v2.0 where a user sees blast radius **before** an
//! autonomous agent with push rights starts. PITFALLS:63 is blunt about the
//! failure mode this module exists to avoid — *"a dry-run that prints 'would
//! run: /gsd:execute-phase' tells the user nothing about blast radius"* — and
//! PITFALLS:69 names *"the dry-run output does not include a refspec list"* as
//! the warning sign. A command log is not a dry-run.
//!
//! D-22 therefore requires **three** outputs, and each is shaped by an honesty
//! constraint rather than by convenience:
//!
//! 1. **The GSD command sequence** — and what it can honestly claim now depends
//!    on which of the two execution models the run is (Phase 20). A supplied
//!    `--command` still yields exactly one command, and that one command is the
//!    *complete and honest* sequence. A **routed** run does not: the router
//!    chooses each iteration's command from what the previous iteration left on
//!    disk, so only the **first** command is computable here and every command
//!    after it depends on state this preview deliberately does not produce. The
//!    output says which case it is in, rather than presenting one command as a
//!    whole sequence or padding the list with guesses — a preview that showed
//!    one command for a run that issues many is PITFALLS:63's failure in a more
//!    convincing form.
//! 2. **The diffstat.** Of the **working tree**, not of the run: you cannot know
//!    the diff of a command you have not run. What is computable — and what the
//!    user actually wants — is the state the run would inherit and could commit,
//!    which answers *"if this run does `git add -A && git commit`, what goes
//!    in?"*
//! 3. **The push refspecs**, computed **locally** from git config. Never
//!    `git push --dry-run`: it contacts the network, needs credentials, and
//!    makes the preview non-deterministic and untestable in CI.
//!
//! D-26 and D-27 add a fourth, and it is the section that keeps the other three
//! honest:
//!
//! 4. **What the envelope guarantees and what it does not**, plus the remote's
//!    own protection state. The statement is
//!    [`crate::envelope::advisory::SECTION_ENVELOPE`], pinned by its own test.
//!    The protection state is **not probed here** — a dry run contacts no
//!    network, which section 3 says in this same output — so a preview reports
//!    `unknown` **with that as the reason**. Reporting anything more comfortable
//!    would be the overstated safety claim D-27 exists to refuse; a caller that
//!    has probed hands the real state to [`DryRunReport::with_protection`].
//!
//! **D-24: this is a CLI mode, not a UI mode.** The rendered string goes to
//! stdout as human-readable text — never to the journal and never to the TUI. A
//! detached driver's stdio is null (D-01), so a dry-run is by definition a
//! foreground invocation and stdout is available. Phase 18 owns TUI surfacing.
//!
//! **D-23: zero git writes and zero agent spawns, proved mechanically.** Every
//! read below is read-only, and nothing here constructs a journal, a lock or an
//! executor. `tests/driver_dry_run.rs` proves it rather than asserting it:
//! `git reflog`, every ref and the whole `.git` directory listing are captured
//! before and after and compared, and a tripwire program supplied as the agent
//! leaves an evidence file if it is ever executed.
//!
//! **No `cfg` gate.** Dry-run is git reads and string building, so it compiles
//! and is testable on every platform; only the *running* of an agent is
//! Unix-only.

use crate::envelope::advisory::{self, ProtectionState};
use crate::executor::DrivableProject;
use crate::state_reader::git_ops::{self, PushPreview, WorkingTreeStat};

/// The envelope section's header and honesty statement (**pinned contract** —
/// see [`SECTION_REFSPECS`]).
///
/// Re-exported rather than referenced through its own module so the four
/// constants that make up the preview's output contract are visible in one
/// place. It is defined in [`crate::envelope::advisory`] because the run journal
/// renders the same text at run start, and a constant owned by the dry-run
/// module would imply the preview owns the claim. It does not — it is one of two
/// consumers.
pub use crate::envelope::advisory::SECTION_ENVELOPE;

/// The commands header (**pinned contract** — see [`SECTION_REFSPECS`]).
///
/// **Replaced in Phase 20, in the same commit as the code that falsified it.**
/// The previous text read *"the single --command argument below is the complete
/// and honest sequence for this build"*, which stopped being true the moment
/// `--target-phase` put a run under the decision router. Under CONVENTIONS.md:75
/// a change to this text is a breaking, user-visible output change, so the
/// paired byte-offset ordering assertion moves with it.
pub const SECTION_COMMANDS: &str = "== GSD commands this run would issue ==\n\
    A run is either one supplied --command or a routed sequence the decision\n\
    router chooses per iteration. For a supplied command, the line below is the\n\
    complete and honest sequence. For a routed run only the FIRST command can be\n\
    shown: every command after it is chosen from state this preview does not\n\
    produce, so the rest are not withheld — they do not exist yet.";

/// The diffstat header (**pinned contract** — see [`SECTION_REFSPECS`]).
pub const SECTION_DIFFSTAT: &str = "== Working tree a commit would capture ==\n\
    `git diff --stat HEAD` plus untracked files: the state this run would\n\
    inherit and could sweep into a `git add -A && git commit`. It is not the\n\
    diff of the command — that cannot be known without running it.";

/// The refspecs header, and the pinned-contract rule for all four.
///
/// **These four constants are a contract, not decoration.**
/// `dry_run::tests::the_rendered_report_carries_all_three_section_headers_in_order`
/// asserts all four appear, in this order, by comparing byte offsets — so a
/// section cannot silently disappear the way PITFALLS:69 warns about, and there
/// is exactly **one** place that pins the order rather than two that can
/// disagree. (`tests/driver_dry_run.rs` additionally pins the first three
/// against a real repository.) Changing their text is a user-visible output
/// change and breaks anyone scripting against the preview.
pub const SECTION_REFSPECS: &str = "== Push refspecs this state would produce ==\n\
    Computed locally from git config (branch.<b>.remote, remote.pushDefault,\n\
    remote.<r>.push, push.default). No network was contacted, no credential was\n\
    used, and git's push subcommand was never invoked in any form.";

/// The banner every preview opens with, so the mode is unmistakable.
const BANNER: &str = "DRY RUN — nothing below was executed. No agent was spawned, no run was\n\
    journaled, and no git write was performed.";

/// Everything a preview reports, gathered before anything is rendered.
///
/// Three fields for D-22's three outputs plus D-26's protection state, and the
/// shape is deliberately flat: building the report and rendering it are separate
/// so a test can assert on the rendered string without capturing stdout.
#[derive(Debug, Clone)]
pub struct DryRunReport {
    /// The GSD commands the run would issue, in order — **and only the ones
    /// that are actually knowable here.**
    ///
    /// One element in command mode, because one command is the whole run. One
    /// element in routed mode too, but for an entirely different reason: it is
    /// the router's **first** selection and the sequence continues past it.
    /// Empty when the router would park instead of choosing.
    ///
    /// **The vector alone therefore cannot say whether it is complete**, and a
    /// reader who assumed it could would draw exactly the wrong conclusion in
    /// routed mode. [`PreviewScope`] is what carries that, and it rides
    /// [`RoutedPreview`] rather than this struct — see that type for why the
    /// scope is not a field here.
    pub commands: Vec<String>,
    /// What a commit from the current working tree would capture.
    pub diffstat: WorkingTreeStat,
    /// What a push from the current state would send.
    pub push: PushPreview,
    /// What the remote itself enforces (D-26).
    ///
    /// `Unknown` with a reason for a plain preview — see the module doc for why
    /// a dry run does not probe. A caller that has probed supplies the real
    /// state through [`DryRunReport::with_protection`].
    pub protection: ProtectionState,
}

impl DryRunReport {
    /// This report, carrying a protection state somebody actually probed.
    ///
    /// A separate step rather than an argument to [`build_report`], because the
    /// probe and the preview have different costs and different callers: the
    /// preview is three local git reads, and the probe is a bounded network
    /// query that belongs at run start. Wiring them together would put a network
    /// call behind a command whose own output promises none.
    pub fn with_protection(mut self, protection: ProtectionState) -> Self {
        self.protection = protection;
        self
    }
}

/// A routed run's preview: the report, plus what its command list is worth.
///
/// **A wrapper rather than a field on [`DryRunReport`], and the reason is that
/// command mode has nothing to gain from the field.** A supplied `--command` is
/// always [`PreviewScope::Complete`]; carrying a scope on every report would put
/// a field on the common path whose value is constant there, and would oblige
/// every existing constructor and caller to name it. Routed mode is the case
/// with something to say, so routed mode is the type that says it.
#[derive(Debug, Clone)]
pub struct RoutedPreview {
    /// The report proper — same three outputs and same protection state.
    pub report: DryRunReport,
    /// What [`DryRunReport::commands`] is a complete answer to.
    pub scope: PreviewScope,
}

/// What a preview's command list is a complete answer to.
///
/// **Three arms and no unclassified one**, for the same reason
/// `driver::run::Terminal` has three: the failure this replaces was a preview
/// that could not say whether its list was the whole story, so "we did not say"
/// must not be representable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreviewScope {
    /// The listed commands are the **complete** sequence the run would issue.
    Complete,
    /// Only the **first** command is listed, and the run would continue past it.
    FirstOfMany,
    /// The router would choose **no** command from this state; it would park.
    WouldPark {
        /// The taxonomy member's stable identifier, from
        /// [`router::RouterReason::as_str`](super::router::RouterReason::as_str)
        /// — never a fresh string minted here, so the same grep finds the
        /// preview and the run record it predicts.
        reason: String,
        /// One token naming what was observed. Never prose, never artifact
        /// content (SAFE-04, T-20-05).
        detail: String,
    },
}

/// Gather the report for `project` and one supplied `command`.
///
/// Read-only throughout: the two git helpers it calls shell out with
/// `--no-optional-locks` and touch nothing (D-23). Synchronous, because the
/// dry-run path is a foreground CLI invocation (D-24); a TUI-side caller would
/// put this on a blocking thread.
///
/// **Command mode, and unchanged from Phase 17 in both signature and meaning:**
/// one supplied command is the whole of what the run would issue, and the
/// rendered section says so. A routed run goes through [`build_routed_report`].
pub fn build_report(project: &DrivableProject, command: &str) -> DryRunReport {
    DryRunReport {
        commands: vec![command.to_string()],
        diffstat: git_ops::working_tree_stat(project.root()),
        push: git_ops::push_refspecs(project.root()),
        // Not probed, and the rendered section says so with the reason. The
        // alternative — omitting the state, or defaulting it to something
        // reassuring — is exactly the failure D-26 names.
        protection: advisory::not_probed(),
    }
}

/// Gather the preview for a **routed** run driving toward `target_phase`.
///
/// **This reads `.planning/` and calls the router, and neither widens the blast
/// radius this module promises.** The read goes through the one shared reader
/// every other caller uses and opens nothing outside the project;
/// [`decide`](super::router::decide) is pure — it performs no I/O, spawns
/// nothing and contacts no model. The preview therefore still makes zero git
/// writes and zero agent spawns, which `tests/driver_dry_run.rs` proves with a
/// before/after capture of the whole `.git` directory and a tripwire program
/// rather than asserting it.
///
/// The commands vector gets the router's **first selection and nothing else**.
/// Showing the real first command is honest precisely because
/// [`RoutedPreview::scope`] beside it refuses to call it a sequence; showing
/// nothing would withhold the one fact that *is* knowable, and padding the list
/// with guesses is the failure the whole module exists to avoid.
pub fn build_routed_report(project: &DrivableProject, target_phase: &str) -> RoutedPreview {
    let state = crate::state_reader::parse_project_state(&project.root().join(".planning"));

    let (commands, scope) = match super::router::decide(&state, target_phase) {
        super::router::Decision::Run { command, .. } => (vec![command], PreviewScope::FirstOfMany),
        super::router::Decision::Park { reason, detail } => (
            Vec::new(),
            PreviewScope::WouldPark {
                reason: reason.as_str().to_string(),
                detail,
            },
        ),
        super::router::Decision::NoRule { observed } => (
            Vec::new(),
            PreviewScope::WouldPark {
                reason: super::router::RouterReason::NoRule.as_str().to_string(),
                detail: observed,
            },
        ),
        // The target phase's verification already passed, so the run would
        // issue nothing at all — an empty command list whose scope says
        // `Complete` rather than one that leaves a reader guessing whether the
        // preview was truncated. Spelled out rather than wildcarded so that a
        // change to the goal-met predicate is a decision here rather than a
        // silent fall-through to "would park".
        super::router::Decision::GoalMet => (Vec::new(), PreviewScope::Complete),
    };

    RoutedPreview {
        report: DryRunReport {
            commands,
            diffstat: git_ops::working_tree_stat(project.root()),
            push: git_ops::push_refspecs(project.root()),
            protection: advisory::not_probed(),
        },
        scope,
    }
}

/// Render a report as the human-readable text that goes to stdout (D-24).
///
/// **Every section prints something**, always — a blank section reads as a
/// missing section, and that invariant is what makes the pinned-header test
/// meaningful. An empty working tree says so explicitly; a state that would push
/// nothing says that explicitly too.
pub fn render(report: &DryRunReport) -> String {
    // Command mode is `Complete` by construction: `build_report` is reached only
    // with one supplied command, and one supplied command is the whole run.
    render_with_scope(report, &PreviewScope::Complete)
}

/// Render a routed preview, scope and all.
///
/// A separate entry point rather than an argument on [`render`], so the common
/// path keeps its one-argument shape and no existing caller has to name a scope
/// that is constant for it.
pub fn render_routed(preview: &RoutedPreview) -> String {
    render_with_scope(&preview.report, &preview.scope)
}

/// The one renderer both entry points share.
///
/// Sharing it is what stops the two previews drifting into two different
/// four-section layouts, which would make the pinned ordering assertion true of
/// one and vacuous for the other.
fn render_with_scope(report: &DryRunReport, scope: &PreviewScope) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push(BANNER.to_string());
    lines.push(String::new());

    // 1. The command sequence.
    lines.push(SECTION_COMMANDS.to_string());
    match scope {
        PreviewScope::Complete => {
            let count = report.commands.len();
            lines.push(format!(
                "  {count} command{} in the sequence:",
                if count == 1 { "" } else { "s" }
            ));
        }
        PreviewScope::FirstOfMany => {
            // The count is stated as what it is — a first, not a total — because
            // "1 command in the sequence" would be a false total for a routed
            // run, and a false total is worse than an absent one.
            lines.push(
                "  Routed run: the FIRST command only. The run continues past it, and\n  \
                 how far cannot be computed here — each later command is chosen from\n  \
                 what the previous one leaves on disk, which a preview does not produce."
                    .to_string(),
            );
        }
        PreviewScope::WouldPark { reason, detail } => {
            lines.push(format!(
                "  Routed run: no command would be issued. From the state on disk now,\n  \
                 the router would park ({reason}: {detail}) and the run would stop for\n  \
                 a human rather than choose."
            ));
        }
    }
    for (index, command) in report.commands.iter().enumerate() {
        lines.push(format!("    {}. {command}", index + 1));
    }
    lines.push(String::new());

    // 2. The working-tree diffstat.
    lines.push(SECTION_DIFFSTAT.to_string());
    if report.diffstat.stat_lines.is_empty() && report.diffstat.untracked.is_empty() {
        lines.push(
            "  Clean working tree — a commit from this state would capture nothing.".to_string(),
        );
    } else {
        if report.diffstat.stat_lines.is_empty() {
            lines.push("  No tracked file is modified.".to_string());
        } else {
            for stat in &report.diffstat.stat_lines {
                lines.push(format!("  {stat}"));
            }
        }
        if !report.diffstat.untracked.is_empty() {
            lines.push("  Untracked:".to_string());
            for path in &report.diffstat.untracked {
                lines.push(format!("    {path}"));
            }
        }
    }
    lines.push(String::new());

    // 3. The push refspecs.
    lines.push(SECTION_REFSPECS.to_string());
    match (&report.push.remote, &report.push.url) {
        (Some(remote), Some(url)) => {
            lines.push(format!("  Remote: {remote}"));
            lines.push(format!("  URL:    {url}"));
        }
        (Some(remote), None) => {
            lines.push(format!("  Remote: {remote}"));
            lines.push("  URL:    (no URL configured for that remote)".to_string());
        }
        _ => {}
    }
    if report.push.refspecs.is_empty() {
        match &report.push.note {
            Some(note) => lines.push(format!("  {note}")),
            None => lines.push("  No push would occur from this state.".to_string()),
        }
    } else {
        for refspec in &report.push.refspecs {
            lines.push(format!("  {refspec}"));
        }
        if let Some(note) = &report.push.note {
            lines.push(format!("  Note: {note}"));
        }
    }
    lines.push(String::new());

    // 4. What the envelope does and does not guarantee, and what the remote
    //    itself enforces. Rendered through `envelope_notice` rather than
    //    assembled here, so the preview and the run journal cannot come to
    //    disagree about what was claimed (T-19-42).
    lines.push(advisory::envelope_notice(&report.protection));

    let mut rendered = lines.join("\n");
    rendered.push('\n');
    rendered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{DriverOptIn, RegisteredProject};

    fn report() -> DryRunReport {
        DryRunReport {
            commands: vec!["/gsd:progress".to_string()],
            diffstat: WorkingTreeStat {
                stat_lines: vec![" src/lib.rs | 2 +-".to_string()],
                untracked: vec!["notes.md".to_string()],
            },
            push: PushPreview {
                remote: Some("origin".to_string()),
                url: Some("https://example.invalid/demo.git".to_string()),
                refspecs: vec!["refs/heads/main:refs/heads/main".to_string()],
                note: None,
            },
            protection: advisory::not_probed(),
        }
    }

    // The name is unchanged on purpose although this now covers four sections:
    // extending the assertion that exists, rather than adding a second one
    // beside it, is what keeps exactly ONE place pinning the order. Two ordering
    // tests are two things that can disagree, and the one that gets updated is
    // not necessarily the one somebody reads.
    #[test]
    fn the_rendered_report_carries_all_three_section_headers_in_order() {
        let rendered = render(&report());

        let commands = rendered
            .find(SECTION_COMMANDS)
            .expect("the commands section must appear");
        let diffstat = rendered
            .find(SECTION_DIFFSTAT)
            .expect("the diffstat section must appear");
        let refspecs = rendered.find(SECTION_REFSPECS).expect(
            "the refspec section must appear — PITFALLS:69 names its absence as THE warning sign",
        );
        let envelope = rendered.find(SECTION_ENVELOPE).expect(
            "the envelope section must appear — a preview that shows blast radius \
             without saying what is NOT guaranteed is the overstated safety claim \
             D-27 exists to refuse",
        );

        assert!(
            commands < diffstat && diffstat < refspecs && refspecs < envelope,
            "the four sections have a fixed order; got offsets {commands}, \
             {diffstat}, {refspecs}, {envelope}"
        );
    }

    #[test]
    fn a_supplied_command_is_still_rendered_as_the_complete_sequence() {
        let rendered = render(&report());

        assert!(
            rendered.contains("1 command in the sequence:"),
            "the count is stated rather than left to be counted, got:\n{rendered}"
        );
        assert!(
            rendered.contains("complete and honest sequence"),
            "one supplied command IS the whole run, and saying so is not a \
             shortfall to paper over; got:\n{rendered}"
        );
        assert!(
            rendered.contains("/gsd:progress"),
            "the command itself has to appear, got:\n{rendered}"
        );
    }

    #[test]
    fn a_routed_preview_shows_one_command_and_says_the_run_continues_past_it() {
        let preview = RoutedPreview {
            report: report(),
            scope: PreviewScope::FirstOfMany,
        };
        let rendered = render_routed(&preview);

        assert_eq!(
            preview.report.commands.len(),
            1,
            "exactly one entry — the router's first selection, never a padded list"
        );
        assert!(
            rendered.contains("FIRST command"),
            "a routed preview must say the entry is a first rather than a total; \
             got:\n{rendered}"
        );
        assert!(
            rendered.contains("does not produce"),
            "and must say WHY there is no more: every later command is chosen \
             from state this preview deliberately does not produce. Without the \
             reason, a reader takes the single entry for a short run; got:\n{rendered}"
        );
        assert!(
            !rendered.contains("1 command in the sequence:"),
            "the command-mode total must NOT appear for a routed run — `1 command \
             in the sequence` would be a false total, which is worse than an \
             absent one; got:\n{rendered}"
        );
    }

    #[test]
    fn a_routed_preview_that_would_park_names_the_reason_and_lists_no_command() {
        let preview = RoutedPreview {
            report: DryRunReport {
                commands: Vec::new(),
                ..report()
            },
            scope: PreviewScope::WouldPark {
                reason: super::super::router::RouterReason::NoRule.as_str().to_string(),
                detail: "Complete".to_string(),
            },
        };
        let rendered = render_routed(&preview);

        assert!(
            rendered.contains("no command would be issued"),
            "the section still prints something — a blank section reads as a \
             missing one; got:\n{rendered}"
        );
        assert!(
            rendered.contains("router_no_rule") && rendered.contains("Complete"),
            "and it names the taxonomy member and what was observed, so the same \
             grep finds the preview and the run record it predicts; got:\n{rendered}"
        );
    }

    #[test]
    fn every_scope_renders_the_four_pinned_sections_in_the_same_order() {
        // The ordering contract must hold for BOTH entry points. Pinning it only
        // against `render` would leave `render_routed` free to grow a different
        // layout while the assertion still passed.
        let scopes = [
            PreviewScope::Complete,
            PreviewScope::FirstOfMany,
            PreviewScope::WouldPark {
                reason: "router_no_rule".to_string(),
                detail: "Complete".to_string(),
            },
        ];

        for scope in &scopes {
            let rendered = render_routed(&RoutedPreview {
                report: report(),
                scope: scope.clone(),
            });

            let commands = rendered
                .find(SECTION_COMMANDS)
                .unwrap_or_else(|| panic!("commands section missing for {scope:?}"));
            let diffstat = rendered
                .find(SECTION_DIFFSTAT)
                .unwrap_or_else(|| panic!("diffstat section missing for {scope:?}"));
            let refspecs = rendered
                .find(SECTION_REFSPECS)
                .unwrap_or_else(|| panic!("refspec section missing for {scope:?}"));
            let envelope = rendered
                .find(SECTION_ENVELOPE)
                .unwrap_or_else(|| panic!("envelope section missing for {scope:?}"));

            assert!(
                commands < diffstat && diffstat < refspecs && refspecs < envelope,
                "the four sections have a fixed order under every scope; {scope:?} \
                 gave offsets {commands}, {diffstat}, {refspecs}, {envelope}"
            );
        }
    }

    #[test]
    fn the_pinned_commands_text_no_longer_claims_one_command_is_always_the_whole_run() {
        // The stale claim, verbatim from before Phase 20. It is spelled out here
        // rather than referenced, because a test that compared the constant with
        // itself could not detect its return.
        assert!(
            !SECTION_COMMANDS.contains("the single --command argument below is"),
            "the routed loop made that sentence false; a user-facing statement \
             the code has falsified is worse than having said nothing (DRIVE-06)"
        );
        assert!(
            SECTION_COMMANDS.contains("FIRST"),
            "and the replacement must state the routed limit rather than being \
             merely vaguer than what it replaced"
        );
    }

    #[test]
    fn every_section_prints_something_even_for_a_clean_tree_with_no_remote() {
        // The empty case is where a section would silently vanish, so it is the
        // case worth pinning.
        let empty = DryRunReport {
            commands: vec!["/gsd:progress".to_string()],
            diffstat: WorkingTreeStat::default(),
            push: PushPreview::default(),
            protection: advisory::not_probed(),
        };
        let rendered = render(&empty);

        assert!(rendered.contains("Clean working tree"));
        assert!(rendered.contains("No push would occur"));
        assert!(rendered.contains(SECTION_REFSPECS));
        assert!(rendered.contains(SECTION_ENVELOPE));
    }

    #[test]
    fn a_preview_reports_the_protection_state_as_unknown_with_its_reason_rather_than_omitting_it() {
        let rendered = render(&report());

        assert!(
            rendered.contains("Remote protection: unknown"),
            "a preview that did not probe says so; got:\n{rendered}"
        );
        assert!(
            rendered.contains("a dry run contacts no network"),
            "and says WHY, because a bare `unknown` reads as an oversight rather \
             than as a constraint; got:\n{rendered}"
        );
    }

    #[test]
    fn a_probed_state_replaces_the_not_probed_one_without_touching_the_other_sections() {
        let probed = report().with_protection(ProtectionState::Unprotected);
        let rendered = render(&probed);

        assert!(
            rendered.contains("Remote protection: unprotected"),
            "a caller that probed gets its own state rendered; got:\n{rendered}"
        );
        assert!(
            rendered.contains(SECTION_COMMANDS) && rendered.contains(SECTION_REFSPECS),
            "and the other three sections are untouched; got:\n{rendered}"
        );
    }

    #[test]
    fn build_report_reads_a_real_project_root_and_carries_the_single_command() {
        let root = tempfile::TempDir::new().expect("temp dir");
        let entry = RegisteredProject {
            path: root.path().to_path_buf(),
            added: "2026-07-29T12:00:00Z".to_string(),
            driver_opt_in: Some(DriverOptIn {
                opted_in_at: "2026-07-29T11:59:00Z".to_string(),
                claude_md_digest: None,
                branch_namespace: None,
                credential: None,
                pr_cap_per_24h: None,
                pr_cap_per_run: None,
            }),
            extra: Default::default(),
        };
        // The production constructor, deliberately: the escape hatch is fenced
        // out of `src/` by `tests/spawn_seam_guard.rs`.
        let project =
            DrivableProject::from_registry("preview", &entry).expect("an opted-in real directory");

        let built = build_report(&project, "/gsd:progress");

        assert_eq!(
            built.commands,
            vec!["/gsd:progress".to_string()],
            "exactly one command, because this phase issues exactly one"
        );
        // A plain directory is not a repository: the preview degrades to empty
        // sections rather than erroring.
        assert!(built.diffstat.stat_lines.is_empty());
        assert!(built.push.refspecs.is_empty());
    }
}
