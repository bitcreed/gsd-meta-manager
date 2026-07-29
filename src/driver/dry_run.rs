//! The three-section preview a `drive --dry-run` prints, and nothing else.
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
//! 1. **The GSD command sequence.** One command, because the decision router is
//!    Phase 20. That single `--command` argument is the *complete and honest*
//!    sequence for this build, and the output says so rather than implying a
//!    sequence it cannot compute.
//! 2. **The diffstat.** Of the **working tree**, not of the run: you cannot know
//!    the diff of a command you have not run. What is computable — and what the
//!    user actually wants — is the state the run would inherit and could commit,
//!    which answers *"if this run does `git add -A && git commit`, what goes
//!    in?"*
//! 3. **The push refspecs**, computed **locally** from git config. Never
//!    `git push --dry-run`: it contacts the network, needs credentials, and
//!    makes the preview non-deterministic and untestable in CI.
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

use crate::executor::DrivableProject;
use crate::state_reader::git_ops::{self, PushPreview, WorkingTreeStat};

/// The commands header (**pinned contract** — see [`SECTION_REFSPECS`]).
pub const SECTION_COMMANDS: &str = "== GSD commands this run would issue ==\n\
    The decision router is Phase 20, so the single --command argument below is\n\
    the complete and honest sequence for this build — not a truncated one.";

/// The diffstat header (**pinned contract** — see [`SECTION_REFSPECS`]).
pub const SECTION_DIFFSTAT: &str = "== Working tree a commit would capture ==\n\
    `git diff --stat HEAD` plus untracked files: the state this run would\n\
    inherit and could sweep into a `git add -A && git commit`. It is not the\n\
    diff of the command — that cannot be known without running it.";

/// The refspecs header, and the pinned-contract rule for all three.
///
/// **These three constants are a contract, not decoration.**
/// `tests/driver_dry_run.rs::the_dry_run_output_names_the_command_the_diffstat_and_the_refspecs`
/// asserts all three appear, in this order, so a section cannot silently
/// disappear the way PITFALLS:69 warns about. Changing their text is a
/// user-visible output change and breaks anyone scripting against the preview.
pub const SECTION_REFSPECS: &str = "== Push refspecs this state would produce ==\n\
    Computed locally from git config (branch.<b>.remote, remote.pushDefault,\n\
    remote.<r>.push, push.default). No network was contacted, no credential was\n\
    used, and git's push subcommand was never invoked in any form.";

/// The banner every preview opens with, so the mode is unmistakable.
const BANNER: &str = "DRY RUN — nothing below was executed. No agent was spawned, no run was\n\
    journaled, and no git write was performed.";

/// Everything a preview reports, gathered before anything is rendered.
///
/// Three fields for D-22's three outputs, and the shape is deliberately flat:
/// building the report and rendering it are separate so a test can assert on the
/// rendered string without capturing stdout.
#[derive(Debug, Clone)]
pub struct DryRunReport {
    /// The GSD commands the run would issue, in order.
    ///
    /// **One element, because this phase runs exactly one command** — not
    /// because the builder is a stub. Phase 20's decision router is what makes
    /// this longer; until it exists, a single element is the truth.
    pub commands: Vec<String>,
    /// What a commit from the current working tree would capture.
    pub diffstat: WorkingTreeStat,
    /// What a push from the current state would send.
    pub push: PushPreview,
}

/// Gather the three sections for `project` and `command`.
///
/// Read-only throughout: the two git helpers it calls shell out with
/// `--no-optional-locks` and touch nothing (D-23). Synchronous, because the
/// dry-run path is a foreground CLI invocation (D-24); a TUI-side caller would
/// put this on a blocking thread.
pub fn build_report(project: &DrivableProject, command: &str) -> DryRunReport {
    DryRunReport {
        commands: vec![command.to_string()],
        diffstat: git_ops::working_tree_stat(project.root()),
        push: git_ops::push_refspecs(project.root()),
    }
}

/// Render a report as the human-readable text that goes to stdout (D-24).
///
/// **Every section prints something**, always — a blank section reads as a
/// missing section, and that invariant is what makes the pinned-header test
/// meaningful. An empty working tree says so explicitly; a state that would push
/// nothing says that explicitly too.
pub fn render(report: &DryRunReport) -> String {
    let mut lines: Vec<String> = Vec::new();

    lines.push(BANNER.to_string());
    lines.push(String::new());

    // 1. The command sequence.
    lines.push(SECTION_COMMANDS.to_string());
    let count = report.commands.len();
    lines.push(format!(
        "  {count} command{} in the sequence:",
        if count == 1 { "" } else { "s" }
    ));
    for (index, command) in report.commands.iter().enumerate() {
        lines.push(format!("    {}. {command}", index + 1));
    }
    lines.push(String::new());

    // 2. The working-tree diffstat.
    lines.push(SECTION_DIFFSTAT.to_string());
    if report.diffstat.stat_lines.is_empty() && report.diffstat.untracked.is_empty() {
        lines.push("  Clean working tree — a commit from this state would capture nothing.".to_string());
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
            None => lines.push(
                "  No push would occur from this state.".to_string(),
            ),
        }
    } else {
        for refspec in &report.push.refspecs {
            lines.push(format!("  {refspec}"));
        }
        if let Some(note) = &report.push.note {
            lines.push(format!("  Note: {note}"));
        }
    }

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
        }
    }

    #[test]
    fn the_rendered_report_carries_all_three_section_headers_in_order() {
        let rendered = render(&report());

        let commands = rendered
            .find(SECTION_COMMANDS)
            .expect("the commands section must appear");
        let diffstat = rendered
            .find(SECTION_DIFFSTAT)
            .expect("the diffstat section must appear");
        let refspecs = rendered
            .find(SECTION_REFSPECS)
            .expect("the refspec section must appear — PITFALLS:69 names its absence as THE warning sign");

        assert!(
            commands < diffstat && diffstat < refspecs,
            "the three sections have a fixed order; got offsets {commands}, {diffstat}, {refspecs}"
        );
    }

    #[test]
    fn the_command_section_states_that_one_command_is_the_complete_sequence_for_this_phase() {
        let rendered = render(&report());

        assert!(
            rendered.contains("1 command in the sequence:"),
            "the count is stated rather than left to be counted, got:\n{rendered}"
        );
        assert!(
            rendered.contains("complete and honest sequence"),
            "one command is the honest answer for this build, not a shortfall to \
             paper over; got:\n{rendered}"
        );
        assert!(
            rendered.contains("/gsd:progress"),
            "the command itself has to appear, got:\n{rendered}"
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
        };
        let rendered = render(&empty);

        assert!(rendered.contains("Clean working tree"));
        assert!(rendered.contains("No push would occur"));
        assert!(rendered.contains(SECTION_REFSPECS));
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
