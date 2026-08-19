// ============================================================================
// The conformance oracle: does the Rust table still agree with the runtime the
// driver is actually driving? (DRIVE-02, D-11, T-20-22)
//
// **What this file is for, stated plainly, because it is not obvious from its
// assertions.** The router's rule table is not invented here — it is a
// transcription of GSD's own routing table, which is computed with no model call
// and which this project neither owns nor versions. GSD shipped 1.8.0 → 1.10.0
// in roughly one month, and that exact staleness is what made an earlier design
// in this project obsolete: it carried a self-dated "valid until" line in a
// document, the date passed, and nothing failed. The research document behind
// this phase carries the same kind of line (its GSD-runtime findings self-expire
// 2026-09-02).
//
// This file is the durable answer to that. It converts an expiry date in a
// document into a failing build. When upstream's table moves, the assertions
// below go red with the fixture named — rather than the router quietly driving a
// runtime that no longer exists.
//
// **The oracle runs at TEST time and must never become reachable from the run
// loop.** Calling `gsd-tools query init.manager` per iteration would be a second
// project-state reader (D-11), a blocking `Command` on the driver's async path
// (Phase 19's lint), and a Node process inside a loop whose whole value is
// surviving its parent. `tests/driver_router_table.rs` carries a source-scanning
// assertion that no non-comment line under `src/` names that verb, so the
// separation is a property of the build rather than of this paragraph.
//
// **Skipping is loud and bounded.** When the oracle is unavailable the test
// prints what was missing and returns; when it IS available every fixture must
// produce a real comparison, and a run in which any fixture silently produced
// none fails. A test that passes because it checked nothing is worse than no
// test, which is the whole reason the counters below exist.
// ============================================================================

use std::path::{Path, PathBuf};
use std::process::Command;

use gsd_meta_manager::driver::router::{decide, Decision, SAFE_COMMAND_ALPHABET};
use gsd_meta_manager::state_reader::parse_project_state;
use tempfile::TempDir;

/// The phase every fixture's roadmap declares and every assertion targets.
const TARGET: &str = "01";

/// The oracle verb. Named once, because the whole point of the source scan in
/// `tests/driver_router_table.rs` is that this string appears in `tests/` and
/// nowhere under `src/`.
const ORACLE_VERB: &str = "init.manager";

/// A resolved way to run GSD's own router.
struct Oracle {
    program: PathBuf,
    script: Option<PathBuf>,
}

impl Oracle {
    /// Resolve a runnable `gsd-tools`, or `None` with the reason printed.
    ///
    /// The search order mirrors `src/state_reader/queue_md.rs`'s resolver,
    /// which already had to answer this question for the TUI: a `.cjs` shim is
    /// run through `node`, and a `gsd-tools` binary on `PATH` is run directly.
    /// Reusing the order rather than inventing one keeps "which GSD is this
    /// project talking to?" a single answer.
    fn resolve() -> Option<Self> {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        let candidates: Vec<PathBuf> = home
            .into_iter()
            .map(|h| h.join(".claude/gsd-core/bin/gsd-tools.cjs"))
            .collect();

        for script in candidates {
            if script.is_file() {
                // `node` has to exist too; a shim with no interpreter is not a
                // resolved oracle.
                if Command::new("node").arg("--version").output().is_ok() {
                    return Some(Oracle {
                        program: PathBuf::from("node"),
                        script: Some(script),
                    });
                }
                eprintln!(
                    "SKIP: found {} but `node` is not runnable, so the oracle cannot be \
                     executed",
                    script.display()
                );
                return None;
            }
        }

        if Command::new("gsd-tools").arg("--version").output().is_ok() {
            return Some(Oracle {
                program: PathBuf::from("gsd-tools"),
                script: None,
            });
        }

        eprintln!(
            "SKIP: neither ~/.claude/gsd-core/bin/gsd-tools.cjs nor a `gsd-tools` on PATH is \
             available, so the conformance oracle cannot run. This is a SKIP and not a pass: \
             the Rust rule table is a transcription of GSD's routing table and nothing in this \
             run checked that the transcription still holds."
        );
        None
    }

    /// The recommended action GSD's router emits for [`TARGET`] in `root`, as
    /// `(action, command)`.
    ///
    /// `None` means the oracle emitted no action for that phase — which is
    /// itself an answer the assertions below check, not a reason to skip.
    fn recommended_action(&self, root: &Path) -> Result<Option<(String, String)>, String> {
        let mut command = Command::new(&self.program);
        if let Some(script) = &self.script {
            command.arg(script);
        }
        let output = command
            .args(["query", ORACLE_VERB])
            .current_dir(root)
            .output()
            .map_err(|err| format!("the oracle did not run: {}", err.kind()))?;

        if !output.status.success() {
            return Err(format!(
                "the oracle exited {:?} for {}",
                output.status.code(),
                root.display()
            ));
        }

        let parsed: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|_| "the oracle's output did not parse as JSON".to_string())?;

        Ok(parsed
            .get("recommended_actions")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .find(|action| action.get("phase").and_then(serde_json::Value::as_str) == Some(TARGET))
            .map(|action| {
                (
                    action
                        .get("action")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    action
                        .get("command")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                )
            }))
    }
}

/// What upstream is expected to do for a fixture.
///
/// **Declared rather than discovered, and that is what makes the file an
/// oracle rather than a snapshot.** A fixture whose upstream behaviour has
/// changed fails here by name — including in the direction that looks harmless,
/// where a declared divergence stops diverging.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Upstream {
    /// Upstream emits a forward command, and this router must emit the same one.
    SameCommand,
    /// Upstream emits a `verify` action that this router **deliberately refuses
    /// to auto-select**.
    ///
    /// The divergence is CONTEXT.md's OQ7 resolution made concrete: the
    /// verification statuses that put a phase in the `executed` state are the
    /// DRIVE-05 gate set, and `/gsd-verify-work` is the command a *human* runs
    /// to answer them. Auto-selecting it would be the router answering the
    /// question the gate exists to ask. It is declared here, rather than left
    /// implicit, because an undeclared divergence is indistinguishable from a
    /// bug — and because the arm fails if the divergence ever stops being one.
    VerifyGate,
    /// Upstream emits no action at all for the target.
    ///
    /// Asserted rather than skipped: an upstream router that emits nothing is
    /// not an upstream router that emits a command this one may invent.
    NoAction,
}

/// One fixture project tree, and what both routers should say about it.
struct Fixture {
    name: &'static str,
    upstream: Upstream,
    build: fn(&Path),
}

/// Write `contents` to `path`, creating parents.
fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("fixture directories are creatable");
    }
    std::fs::write(path, contents).expect("fixture files are writable");
}

/// The roadmap and state every fixture shares.
///
/// Built by **writing the artifact files both readers look for**, never by
/// constructing reader structs. The whole claim under test is that two
/// independent readers agree about one tree; a fixture assembled from Rust
/// values would only prove that this process agrees with itself.
fn skeleton(root: &Path) {
    write(
        &root.join(".planning/ROADMAP.md"),
        "# Roadmap\n\n## Phases\n\n\
         - [ ] **Phase 01: Alpha** - the phase both routers are asked about\n\n\
         ### Phase 01: Alpha\n\n\
         **Goal**: exercise the rule table\n\
         **Depends on**: Nothing\n",
    );
    write(
        &root.join(".planning/STATE.md"),
        "---\nstatus: in_progress\n---\n\n# State\n",
    );
    std::fs::create_dir_all(root.join(".planning/phases")).expect("the phases dir is creatable");
}

fn phase_dir(root: &Path) -> PathBuf {
    root.join(".planning/phases/01-alpha")
}

fn plan(root: &Path, index: &str) {
    write(
        &phase_dir(root).join(format!("01-{index}-PLAN.md")),
        "---\nphase: 01\n---\n\n# Plan\n",
    );
}

fn summary(root: &Path, index: &str) {
    write(
        &phase_dir(root).join(format!("01-{index}-SUMMARY.md")),
        "---\nphase: 01\nstatus: complete\n---\n\n# Summary\n",
    );
}

fn verification(root: &Path, status: &str) {
    write(
        &phase_dir(root).join("01-VERIFICATION.md"),
        &format!("---\nphase: 01\nstatus: {status}\n---\n\n# Verification\n"),
    );
}

/// One fixture per state the rule table covers, plus the two upstream emits
/// nothing for and the two gate states it emits a human's command for.
const FIXTURES: &[Fixture] = &[
    Fixture {
        name: "no_directory",
        upstream: Upstream::SameCommand,
        build: |root| skeleton(root),
    },
    Fixture {
        name: "empty",
        upstream: Upstream::SameCommand,
        build: |root| {
            skeleton(root);
            std::fs::create_dir_all(phase_dir(root)).expect("the phase dir is creatable");
        },
    },
    Fixture {
        name: "discussed",
        upstream: Upstream::SameCommand,
        build: |root| {
            skeleton(root);
            write(&phase_dir(root).join("01-CONTEXT.md"), "# Context\n");
        },
    },
    Fixture {
        name: "researched",
        upstream: Upstream::SameCommand,
        build: |root| {
            skeleton(root);
            write(&phase_dir(root).join("01-RESEARCH.md"), "# Research\n");
        },
    },
    Fixture {
        name: "planned",
        upstream: Upstream::SameCommand,
        build: |root| {
            skeleton(root);
            plan(root, "01");
        },
    },
    Fixture {
        name: "partial",
        upstream: Upstream::NoAction,
        build: |root| {
            skeleton(root);
            plan(root, "01");
            plan(root, "02");
            summary(root, "01");
        },
    },
    Fixture {
        name: "executed_human_needed",
        upstream: Upstream::VerifyGate,
        build: |root| {
            skeleton(root);
            plan(root, "01");
            summary(root, "01");
            verification(root, "human_needed");
        },
    },
    Fixture {
        name: "executed_gaps_found",
        upstream: Upstream::VerifyGate,
        build: |root| {
            skeleton(root);
            plan(root, "01");
            summary(root, "01");
            verification(root, "gaps_found");
        },
    },
    Fixture {
        name: "complete",
        upstream: Upstream::NoAction,
        build: |root| {
            skeleton(root);
            plan(root, "01");
            summary(root, "01");
            verification(root, "passed");
        },
    },
];

/// A built fixture tree. The `TempDir` is held for its `Drop`.
struct Tree {
    _dir: TempDir,
    root: PathBuf,
}

fn build(fixture: &Fixture) -> Tree {
    let dir = TempDir::new().expect("a temp dir");
    let root = dir.path().to_path_buf();
    (fixture.build)(&root);
    Tree { _dir: dir, root }
}

#[test]
fn the_rust_rule_table_agrees_with_gsd_s_own_router_over_a_fixture_per_state() {
    let Some(oracle) = Oracle::resolve() else {
        // The reason was printed by `resolve`. Returning here is a skip, and the
        // companion test below is what stops a skip becoming a silent pass in an
        // environment where the oracle IS present.
        return;
    };

    let mut compared = 0usize;
    let mut diverged = 0usize;
    let mut silent = 0usize;

    for fixture in FIXTURES {
        let tree = build(fixture);
        let upstream = oracle
            .recommended_action(&tree.root)
            .unwrap_or_else(|err| panic!("fixture `{}`: {err}", fixture.name));

        let state = parse_project_state(&tree.root.join(".planning"));
        let ours = decide(&state, TARGET);

        match fixture.upstream {
            Upstream::SameCommand => {
                let (action, command) = upstream.unwrap_or_else(|| {
                    panic!(
                        "fixture `{}` is declared to produce an upstream command and produced \
                         none. Either the fixture no longer puts the phase into the state it \
                         is named for, or upstream's table moved — and the second is exactly \
                         what this file exists to report as a failing build rather than as an \
                         expiry date in a document",
                        fixture.name
                    )
                });
                assert!(
                    SAFE_COMMAND_ALPHABET
                        .iter()
                        .any(|verb| command.starts_with(verb)),
                    "fixture `{}`: upstream now emits `{command}` for action `{action}`, which \
                     is outside this router's declared safe alphabet. Widening the alphabet is \
                     a decision, not a fix",
                    fixture.name
                );
                match &ours {
                    Decision::Run { command: mine, .. } => assert_eq!(
                        mine, &command,
                        "fixture `{}`: this router selected `{mine}` where GSD's own router \
                         selects `{command}`. The rule table is a transcription of that \
                         table; a disagreement means the driver is driving a runtime that \
                         does not exist",
                        fixture.name
                    ),
                    other => panic!(
                        "fixture `{}`: upstream selects `{command}` and this router produced \
                         {other:?}",
                        fixture.name
                    ),
                }
                compared += 1;
            }
            Upstream::VerifyGate => {
                let (action, command) = upstream.unwrap_or_else(|| {
                    panic!(
                        "fixture `{}` declares a divergence from an upstream `verify` action \
                         and upstream emitted nothing. A declared divergence that no longer \
                         diverges is a stale declaration, and this arm fails on it for the \
                         same reason an allowlist entry that suppresses nothing fails",
                        fixture.name
                    )
                });
                assert_eq!(
                    action, "verify",
                    "fixture `{}`: the declared divergence is specifically from upstream's \
                     `verify` action; upstream now emits `{action}` → `{command}`",
                    fixture.name
                );
                match &ours {
                    Decision::Park { reason, .. } => {
                        let identifier = reason.as_str();
                        assert!(
                            identifier.starts_with("gate_"),
                            "fixture `{}`: upstream routes this state to `{command}`, which is \
                             the command a HUMAN runs to answer the gate. This router must \
                             park under a gate reason instead; got `{identifier}`",
                            fixture.name
                        );
                    }
                    other => panic!(
                        "fixture `{}`: upstream routes to `{command}` and this router must \
                         park at the gate rather than auto-selecting it (DRIVE-05). Got \
                         {other:?}",
                        fixture.name
                    ),
                }
                diverged += 1;
            }
            Upstream::NoAction => {
                assert!(
                    upstream.is_none(),
                    "fixture `{}` is declared to produce no upstream action and upstream now \
                     emits {upstream:?}. Upstream's table moved",
                    fixture.name
                );
                assert!(
                    !matches!(ours, Decision::Run { .. }),
                    "fixture `{}`: an upstream router that emits nothing is not an upstream \
                     router that emits a command this one may invent. Got {ours:?}",
                    fixture.name
                );
                silent += 1;
            }
        }
    }

    assert!(
        compared >= 1,
        "not one fixture produced a real command-to-command comparison. A conformance run in \
         which every fixture was skipped passes for the wrong reason, which is the failure \
         this floor exists to catch"
    );
    assert_eq!(
        compared + diverged + silent,
        FIXTURES.len(),
        "every fixture must land in exactly one arm; {compared} compared, {diverged} diverged \
         and {silent} silent out of {} fixtures",
        FIXTURES.len()
    );

    eprintln!(
        "conformance: {compared} command comparisons, {diverged} declared divergences, \
         {silent} upstream-silent states, over {} fixtures",
        FIXTURES.len()
    );
}

#[test]
fn every_fixture_reaches_the_state_it_is_named_for() {
    // A guard on the fixtures themselves, independent of the oracle. A fixture
    // that stopped producing its state would make the conformance run above
    // compare two routers about a state neither is being asked about — and it
    // would do so while passing.
    for fixture in FIXTURES {
        let tree = build(fixture);
        let state = parse_project_state(&tree.root.join(".planning"));
        let inference = state
            .phase_disk_statuses
            .get(TARGET)
            .unwrap_or_else(|| panic!("fixture `{}` produced no inference", fixture.name));

        let observed = gsd_meta_manager::driver::router::status_token(inference.status);
        let expected = fixture
            .name
            .split("_gaps")
            .next()
            .and_then(|n| n.split("_human").next())
            .unwrap_or(fixture.name);
        assert_eq!(
            observed, expected,
            "fixture `{}` is named for the `{expected}` state and this repository's reader \
             sees `{observed}`",
            fixture.name
        );
    }
}
