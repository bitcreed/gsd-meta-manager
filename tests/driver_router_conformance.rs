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
// **A missing oracle FAILS; skipping is opt-in.** This file used to return
// early when `gsd-tools` could not be resolved, printing its reason to a stream
// libtest captures — so on any machine or CI runner without it, the test passed
// while nothing checked that the transcription still holds. Its own comment
// claimed a companion test prevented that, and the named companion never
// resolves or consults the oracle at all (WR-07). The default is now to fail,
// and an environment that genuinely cannot run Node says so explicitly through
// `ALLOW_MISSING_ORACLE`. When the oracle IS available every fixture must
// produce a real comparison, and a run in which any fixture silently produced
// none fails. A test that passes because it checked nothing is worse than no
// test, which is the whole reason the counters below exist.
// ============================================================================

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

use gsd_meta_manager::driver::router::{decide, Decision, SAFE_COMMAND_ALPHABET};
use gsd_meta_manager::state_reader::gsd_install::{install_candidates, InstallRoots};
use gsd_meta_manager::state_reader::parse_project_state;
use tempfile::TempDir;

/// The default target phase: what a fixture's `target` is unless it names another.
const TARGET: &str = "01";

/// The oracle verb. Named once, because the whole point of the source scan in
/// `tests/driver_router_table.rs` is that this string appears in `tests/` and
/// nowhere under `src/`.
const ORACLE_VERB: &str = "init.manager";

/// The variable an environment sets to say that it genuinely cannot run the
/// oracle, and accepts a conformance run that checked nothing.
///
/// **Opt-out rather than opt-in, so the default is fail-loud.** An absent
/// oracle is not a neutral condition: this file is the only thing standing
/// between the rule table and the runtime it transcribes, and the research
/// behind it self-expires. A skip nobody has to ask for is a skip nobody
/// notices.
const ALLOW_MISSING_ORACLE: &str = "GSD_META_MANAGER_ALLOW_MISSING_ORACLE";

/// Whether a run with no resolvable oracle may pass.
///
/// Pure over the variable's *value*, so the default can be pinned by a test
/// without unsetting anything in a process other tests share. An empty or
/// whitespace-only value is not an opt-out: `FOO= cargo test` is how a variable
/// is unset by accident, and reading it as consent is the same silent pass by a
/// different route.
fn skip_permitted(opt_out: Option<&str>) -> bool {
    matches!(opt_out, Some(value) if !value.trim().is_empty())
}

/// Where the oracle looks for GSD's `gsd-tools.cjs`, in order.
///
/// The global arm of the production list: `install_candidates` with no project
/// root, the same list `gsd_tools_candidates` in `src/state_reader/queue_md.rs`
/// projects onto the same `bin/gsd-tools.cjs` shim. The order is therefore
/// `${CLAUDE_CONFIG_DIR:-~/.claude}`, then `${CODEX_HOME:-~/.codex}` — since
/// gsd-core 1.15.0 (#4667) a Codex install is self-contained there, so a
/// Codex-only machine has no Claude-home `gsd-core`. An override replaces its
/// default rather than adding to it, and counts only when non-blank
/// (whitespace-only is unset) with a leading `~` expanded against home. The
/// project-local candidates of the production resolver are omitted: the
/// fixtures are temp trees that never carry an install of their own.
fn oracle_candidates(roots: &InstallRoots) -> Vec<PathBuf> {
    install_candidates(None, roots)
        .into_iter()
        .map(|candidate| candidate.gsd_core_dir.join("bin/gsd-tools.cjs"))
        .collect()
}

/// A resolved way to run GSD's own router.
struct Oracle {
    program: PathBuf,
    script: Option<PathBuf>,
}

impl Oracle {
    /// Resolve a runnable `gsd-tools`, or `None` with the reason printed.
    ///
    /// The search order is the list `src/state_reader/queue_md.rs`'s resolver
    /// reads, which already had to answer this question for the TUI: a `.cjs`
    /// shim is run through `node`, and a `gsd-tools` binary on `PATH` is run
    /// directly. Reading the same list rather than inventing one keeps "which
    /// GSD is this project talking to?" a single answer.
    fn resolve() -> Option<Self> {
        let candidates = oracle_candidates(&InstallRoots::from_env());

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
            "SKIP: neither ${{CLAUDE_CONFIG_DIR:-~/.claude}}/gsd-core/bin/gsd-tools.cjs, \
             ${{CODEX_HOME:-~/.codex}}/gsd-core/bin/gsd-tools.cjs nor a `gsd-tools` on PATH \
             is available, so the conformance oracle cannot run. This is a SKIP and not a pass: \
             the Rust rule table is a transcription of GSD's routing table and nothing in this \
             run checked that the transcription still holds."
        );
        None
    }

    /// The recommended action GSD's router emits for phase `target` in `root`, as
    /// `(action, command)`.
    ///
    /// `None` means the oracle emitted no action for that phase — which is
    /// itself an answer the assertions below check, not a reason to skip.
    /// The `gsd-tools query <verb>` command, pinned to `GSD_RUNTIME=claude`.
    ///
    /// **Why the pin.** The oracle's `init.manager` spells every command via
    /// `formatGsdSlash(resolveRuntime(cwd))` (upstream 1.15.0 `src/init.cts`
    /// 2865, 3230-3252). A Codex install's `.gsd-runtime` marker (`codex`), or a
    /// `GSD_RUNTIME` inherited from the test process, makes it answer
    /// `$gsd-discuss-phase 01` — measured, and it failed the safe-alphabet
    /// assert. The rule table stores the canonical `/gsd-…` spelling (a
    /// runtime-specific translation used to happen at the retired executor's
    /// edge), so the comparison has to be made in canonical spelling. For this verb the
    /// runtime feeds only the spelling, never the routing, so pinning it cannot
    /// hide a routing change. (The retired driven codex child was the opposite case: it
    /// had `GSD_RUNTIME` scrubbed, never set.)
    fn query_command(&self, verb: &str) -> Command {
        let mut command = Command::new(&self.program);
        if let Some(script) = &self.script {
            command.arg(script);
        }
        command.args(["query", verb]).env("GSD_RUNTIME", "claude");
        command
    }

    fn recommended_action(
        &self,
        root: &Path,
        target: &str,
    ) -> Result<Option<(String, String)>, String> {
        let output = self
            .query_command(ORACLE_VERB)
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
            .find(|action| action.get("phase").and_then(serde_json::Value::as_str) == Some(target))
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
    /// The phase both routers are asked about ([`TARGET`] unless the fixture's
    /// roadmap numbers its phases differently).
    target: &'static str,
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

/// A project on gsd-core's `milestone-prefixed` phase-id convention: the
/// config opt-in, a roadmap of `3`, `3-01` and `3-02`, and one plan under each
/// of the two `M-NN` directories. Written as files, like [`skeleton`].
fn milestone_prefixed_skeleton(root: &Path) {
    write(
        &root.join(".planning/config.json"),
        "{\n  \"phase_id_convention\": \"milestone-prefixed\"\n}\n",
    );
    write(
        &root.join(".planning/ROADMAP.md"),
        "# Roadmap\n\n## Phases\n\n\
         - [ ] **Phase 3: Gamma** - the bare milestone phase\n\
         - [ ] **Phase 3-01: Alpha** - the first phase of milestone 3\n\
         - [ ] **Phase 3-02: Beta** - the second phase of milestone 3\n\n\
         ### Phase 3: Gamma\n\n\
         **Goal**: exercise the bare id\n\
         **Depends on**: Nothing\n\n\
         ### Phase 3-01: Alpha\n\n\
         **Goal**: exercise the M-NN id\n\
         **Depends on**: Nothing\n\n\
         ### Phase 3-02: Beta\n\n\
         **Goal**: a sibling that must not lend its plans\n\
         **Depends on**: Nothing\n",
    );
    write(
        &root.join(".planning/STATE.md"),
        "---\nstatus: in_progress\n---\n\n# State\n",
    );
    write(
        &root.join(".planning/phases/03-01-alpha/03-01-01-PLAN.md"),
        "---\nphase: 03-01\n---\n\n# Plan\n",
    );
    write(
        &root.join(".planning/phases/03-02-beta/03-02-01-PLAN.md"),
        "---\nphase: 03-02\n---\n\n# Plan\n",
    );
}

/// A `milestone-prefixed` project built from a list of `(phase id, name)`
/// roadmap entries and the files to write under `.planning/phases/`. Written as
/// files, like [`skeleton`]. A file ending `-SUMMARY.md` is a completed summary;
/// anything else is a plan. `state_extra` is appended to the skeleton STATE.md.
fn milestone_prefixed_tree(
    root: &Path,
    phases: &[(&str, &str)],
    files: &[&str],
    state_extra: &str,
) {
    write(
        &root.join(".planning/config.json"),
        "{\n  \"phase_id_convention\": \"milestone-prefixed\"\n}\n",
    );
    let mut roadmap = String::from("# Roadmap\n\n## Phases\n\n");
    for (id, name) in phases {
        roadmap.push_str(&format!("- [ ] **Phase {id}: {name}** - d\n"));
    }
    for (id, name) in phases {
        roadmap.push_str(&format!(
            "\n### Phase {id}: {name}\n\n**Goal**: g\n**Depends on**: Nothing\n"
        ));
    }
    write(&root.join(".planning/ROADMAP.md"), &roadmap);
    write(
        &root.join(".planning/STATE.md"),
        &format!("---\nstatus: in_progress\n---\n\n# State\n{state_extra}"),
    );
    for file in files {
        let body = if file.ends_with("-SUMMARY.md") {
            "---\nphase: x\nstatus: complete\n---\n\n# Summary\n"
        } else {
            "---\nphase: x\n---\n\n# Plan\n"
        };
        write(&root.join(".planning/phases").join(file), body);
    }
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
        target: TARGET,
        upstream: Upstream::SameCommand,
        build: |root| skeleton(root),
    },
    Fixture {
        name: "empty",
        target: TARGET,
        upstream: Upstream::SameCommand,
        build: |root| {
            skeleton(root);
            std::fs::create_dir_all(phase_dir(root)).expect("the phase dir is creatable");
        },
    },
    Fixture {
        name: "discussed",
        target: TARGET,
        upstream: Upstream::SameCommand,
        build: |root| {
            skeleton(root);
            write(&phase_dir(root).join("01-CONTEXT.md"), "# Context\n");
        },
    },
    Fixture {
        name: "researched",
        target: TARGET,
        upstream: Upstream::SameCommand,
        build: |root| {
            skeleton(root);
            write(&phase_dir(root).join("01-RESEARCH.md"), "# Research\n");
        },
    },
    Fixture {
        name: "planned",
        target: TARGET,
        upstream: Upstream::SameCommand,
        build: |root| {
            skeleton(root);
            plan(root, "01");
        },
    },
    Fixture {
        name: "partial",
        target: TARGET,
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
        target: TARGET,
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
        target: TARGET,
        upstream: Upstream::VerifyGate,
        build: |root| {
            skeleton(root);
            plan(root, "01");
            summary(root, "01");
            verification(root, "gaps_found");
        },
    },
    // gsd-core 1.15.0's `unparseable` (#4806): a closed leading block that is not
    // YAML. The shape is the real one from this repository's own
    // v1.1-phases/06-read-only-views/06-VERIFICATION.md — a scalar key followed
    // by indented children. Both oracles (1.14.0: `missing`; 1.15.0:
    // `unparseable`) read it `executed` / `verify`, never `complete`, while a
    // line scan finds `status: passed` and would call it GoalMet.
    Fixture {
        name: "executed_unparseable",
        target: TARGET,
        upstream: Upstream::VerifyGate,
        build: |root| {
            skeleton(root);
            plan(root, "01");
            summary(root, "01");
            write(
                &phase_dir(root).join("01-VERIFICATION.md"),
                "---\nphase: 01\nstatus: passed\nre_verification: true\n  previous_status: gaps_found\n  previous_score: 3/5\n---\n\n# Verification\n",
            );
        },
    },
    // A leading `---` that is never closed. Upstream's extractFrontmatter
    // returns `{}` for it, i.e. `missing`: a body `status:` line is not
    // frontmatter.
    Fixture {
        name: "executed_unterminated",
        target: TARGET,
        upstream: Upstream::VerifyGate,
        build: |root| {
            skeleton(root);
            plan(root, "01");
            summary(root, "01");
            write(
                &phase_dir(root).join("01-VERIFICATION.md"),
                "---\nphase: 01\nstatus: passed\n\n# Verification\n",
            );
        },
    },
    // gsd-core >= 1.20.1's `milestone-prefixed` convention (#5675 / #5676). The
    // first two fixtures share one tree; the oracle is measured to emit
    // `/gsd-execute-phase 3-01` for the M-NN phase and `/gsd-discuss-phase 3`
    // for the bare one when two `03-*` siblings exist (a bare `3` takes a lone
    // `3`-leading sibling directory, #2528, and two siblings are ambiguous,
    // #2237 — the next fixtures pin the lone case). They need an oracle >=
    // 1.20.1, which quick-261105-e7u pins for CI and the pre-tag container.
    Fixture {
        name: "milestone_prefixed_m_nn_phase",
        target: "3-01",
        upstream: Upstream::SameCommand,
        build: milestone_prefixed_skeleton,
    },
    Fixture {
        name: "milestone_prefixed_bare_phase",
        target: "3",
        upstream: Upstream::SameCommand,
        build: milestone_prefixed_skeleton,
    },
    Fixture {
        name: "milestone_prefixed_bare_phase_lone_sibling",
        target: "3",
        upstream: Upstream::SameCommand,
        build: |root| {
            milestone_prefixed_tree(
                root,
                &[("3", "Gamma"), ("3-02", "Beta")],
                &["03-02-beta/03-02-01-PLAN.md"],
                "",
            )
        },
    },
    Fixture {
        name: "milestone_prefixed_year_slug_dir",
        target: "14",
        upstream: Upstream::SameCommand,
        build: |root| {
            milestone_prefixed_tree(
                root,
                &[("14", "Photos")],
                &["14-2026-photos/14-01-PLAN.md"],
                "",
            )
        },
    },
    Fixture {
        name: "milestone_prefixed_unpadded_dir",
        target: "3-01",
        upstream: Upstream::SameCommand,
        build: |root| {
            milestone_prefixed_tree(
                root,
                &[("3-01", "Alpha")],
                &["3-01-alpha/3-01-01-PLAN.md"],
                "",
            )
        },
    },
    Fixture {
        name: "milestone_prefixed_slugged_plan",
        target: "3-01",
        upstream: Upstream::SameCommand,
        build: |root| {
            milestone_prefixed_tree(
                root,
                &[("3-01", "Alpha")],
                &[
                    "03-01-alpha/03-01-01-setup-PLAN.md",
                    "03-01-alpha/03-01-01-SUMMARY.md",
                ],
                "",
            )
        },
    },
    Fixture {
        name: "milestone_prefixed_sibling_deferred_verification",
        target: "3-01",
        upstream: Upstream::SameCommand,
        build: |root| {
            milestone_prefixed_tree(
                root,
                &[("3-01", "Alpha"), ("3-02", "Beta")],
                &[
                    "03-01-alpha/03-01-01-PLAN.md",
                    "03-02-beta/03-02-01-PLAN.md",
                ],
                "\n## Deferred Verification\n\n\
                 | Phase | Reason | Resume |\n\
                 |-------|--------|--------|\n\
                 | 3-02 | verification_deferred_human | /gsd:verify-work 3-02 |\n",
            )
        },
    },
    Fixture {
        name: "complete",
        target: TARGET,
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
    let oracle = match Oracle::resolve() {
        Some(oracle) => oracle,
        None => {
            // The reason was printed by `resolve` — to a stream libtest
            // captures, which is precisely why printing it cannot be the whole
            // response. Failing is (WR-07).
            let opt_out = std::env::var(ALLOW_MISSING_ORACLE).ok();
            assert!(
                skip_permitted(opt_out.as_deref()),
                "the conformance oracle could not be resolved, and this run \
                 therefore checked NOTHING about whether the Rust rule table is \
                 still a faithful transcription of GSD's router. That is a \
                 failure by default: the table is a transcription of a runtime \
                 this project neither owns nor versions, GSD shipped 1.8.0 → \
                 1.10.0 in about a month, and the research behind the table \
                 self-expires. Install GSD \
                 (${{CLAUDE_CONFIG_DIR:-~/.claude}}/gsd-core/bin/gsd-tools.cjs \
                 or ${{CODEX_HOME:-~/.codex}}/gsd-core/bin/gsd-tools.cjs; \
                 ./scripts/install-conformance-oracle.sh provisions the first) \
                 and Node, or set {ALLOW_MISSING_ORACLE}=1 to accept a run that \
                 verified none of it"
            );
            eprintln!(
                "SKIP: {ALLOW_MISSING_ORACLE} is set, so a run with no oracle is \
                 accepted. Nothing in it checked the rule table against GSD's own \
                 router."
            );
            return;
        }
    };

    let mut compared = 0usize;
    let mut diverged = 0usize;
    let mut silent = 0usize;

    for fixture in FIXTURES {
        let tree = build(fixture);
        let upstream = oracle
            .recommended_action(&tree.root, fixture.target)
            .unwrap_or_else(|err| panic!("fixture `{}`: {err}", fixture.name));

        let state = parse_project_state(&tree.root.join(".planning"));
        let ours = decide(&state, fixture.target);

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
fn a_missing_oracle_is_a_failure_unless_the_environment_says_otherwise() {
    // **The genuine companion to the skip path** (WR-07). The one this file used
    // to name — `every_fixture_reaches_the_state_it_is_named_for` — never
    // resolves or consults the oracle, so on a machine without `gsd-tools` both
    // tests passed while nothing checked the transcription. This pins the
    // decision that made the skip visible: it is off unless asked for.
    assert!(
        !skip_permitted(None),
        "an unset variable must NOT permit a skip. The default has to be \
         fail-loud, because an absent oracle is invisible otherwise: `eprintln!` \
         from a passing test is captured by libtest and never seen without \
         --nocapture"
    );
    assert!(
        !skip_permitted(Some("")),
        "and neither may an empty value — `FOO= cargo test` is how a variable is \
         unset by accident, and reading that as consent is the same silent pass \
         by another route"
    );
    assert!(!skip_permitted(Some("   ")), "nor a whitespace-only one");
    assert!(
        skip_permitted(Some("1")),
        "an environment that genuinely cannot run Node must have a way to say so, \
         or the only remaining option is deleting the test"
    );
}

/// Injected roots: home `/h`, with the two overrides as the process would see them.
fn roots(claude_config_dir: Option<&str>, codex_home: Option<&str>) -> InstallRoots {
    InstallRoots {
        home: Some(PathBuf::from("/h")),
        claude_config_dir: claude_config_dir.map(OsString::from),
        codex_home: codex_home.map(OsString::from),
    }
}

#[test]
fn the_oracle_follows_claude_config_dir() {
    let home = PathBuf::from("/h");
    let explicit = oracle_candidates(&roots(Some("/cc"), None));
    assert_eq!(
        explicit.first(),
        Some(&PathBuf::from("/cc/gsd-core/bin/gsd-tools.cjs")),
        "an absolute CLAUDE_CONFIG_DIR is where Claude, GSD and the TUI load gsd-core from"
    );
    assert_eq!(
        explicit.len(),
        2,
        "CLAUDE_CONFIG_DIR replaces ~/.claude, it does not add"
    );
    assert!(
        !explicit.iter().any(|c| c.starts_with(home.join(".claude"))),
        "~/.claude must not be consulted as an extra candidate: {explicit:?}"
    );
    for unset in ["", "   "] {
        assert_eq!(
            oracle_candidates(&roots(Some(unset), None)).first(),
            Some(&home.join(".claude/gsd-core/bin/gsd-tools.cjs")),
            "a blank CLAUDE_CONFIG_DIR ({unset:?}) counts as unset"
        );
    }
    assert_eq!(
        oracle_candidates(&roots(Some("~/cc"), None)).first(),
        Some(&home.join("cc/gsd-core/bin/gsd-tools.cjs")),
        "a leading `~` expands against home"
    );
}

#[test]
fn a_relative_claude_config_dir_counts_as_unset_for_the_oracle() {
    // Claude Code refuses a relative CLAUDE_CONFIG_DIR, and the oracle checks a
    // candidate in the test's cwd but runs it with `current_dir(fixture)`, so a
    // relative candidate could never be the one that runs.
    let candidates = oracle_candidates(&roots(Some("rel/cfg"), None));
    assert_eq!(
        candidates.first(),
        Some(&PathBuf::from("/h/.claude/gsd-core/bin/gsd-tools.cjs")),
        "a relative CLAUDE_CONFIG_DIR falls back to the home default"
    );
    assert!(
        candidates.iter().all(|c| c.is_absolute()),
        "no relative candidate may be probed: {candidates:?}"
    );
}

#[test]
fn the_oracle_looks_for_a_claude_install_then_a_codex_install() {
    let home = PathBuf::from("/h");
    assert_eq!(
        oracle_candidates(&roots(None, None)),
        vec![
            home.join(".claude/gsd-core/bin/gsd-tools.cjs"),
            home.join(".codex/gsd-core/bin/gsd-tools.cjs"),
        ],
        "a dual install must keep answering from the Claude install, and a \
         Codex-only one (gsd-core 1.15.0, #4667) must still be found"
    );
}

#[test]
fn the_oracle_honours_a_non_empty_codex_home() {
    let home = PathBuf::from("/h");
    let explicit = oracle_candidates(&roots(None, Some("/x/codex")));
    assert_eq!(
        explicit.last(),
        Some(&PathBuf::from("/x/codex/gsd-core/bin/gsd-tools.cjs"))
    );
    assert_eq!(
        explicit.len(),
        2,
        "CODEX_HOME replaces ~/.codex, it does not add"
    );
    for unset in ["", "   "] {
        let blank = oracle_candidates(&roots(None, Some(unset)));
        assert_eq!(
            blank.last(),
            Some(&home.join(".codex/gsd-core/bin/gsd-tools.cjs")),
            "a blank CODEX_HOME ({unset:?}) falls back like the shell's \
             `${{CODEX_HOME:-$HOME/.codex}}`"
        );
    }
}

#[test]
fn the_oracle_is_asked_for_the_canonical_claude_spelling() {
    let oracle = Oracle {
        program: PathBuf::from("node"),
        script: Some(PathBuf::from("/h/.codex/gsd-core/bin/gsd-tools.cjs")),
    };
    let command = oracle.query_command(ORACLE_VERB);
    let runtime = command
        .get_envs()
        .find(|(key, _)| *key == OsStr::new("GSD_RUNTIME"))
        .and_then(|(_, value)| value);
    assert_eq!(
        runtime,
        Some(OsStr::new("claude")),
        "the rule table stores `/gsd-…`; a Codex-marked oracle or an inherited \
         GSD_RUNTIME would otherwise answer `$gsd-…`"
    );
}

#[test]
fn every_fixture_reaches_the_state_it_is_named_for() {
    // A guard on the fixtures themselves, independent of the oracle. A fixture
    // that stopped producing its state would make the conformance run above
    // compare two routers about a state neither is being asked about — and it
    // would do so while passing. It is NOT the guard on the skip path: that one
    // is `a_missing_oracle_is_a_failure_unless_the_environment_says_otherwise`,
    // and this comment used to claim a duty this test never discharged (WR-07).
    for fixture in FIXTURES {
        let tree = build(fixture);
        let state = parse_project_state(&tree.root.join(".planning"));
        let inference = state
            .phase_disk_statuses
            .get(fixture.target)
            .unwrap_or_else(|| panic!("fixture `{}` produced no inference", fixture.name));

        let observed = gsd_meta_manager::driver::router::status_token(inference.status);
        let expected = fixture
            .name
            .strip_prefix("milestone_prefixed_")
            .map(|shape| match shape {
                // `03-01-alpha` holds a plan.
                "m_nn_phase" => "planned",
                // Two `03-*` siblings are ambiguous (#2237), so a bare `3`
                // has no directory of its own and borrows no plans.
                "bare_phase" => "no_directory",
                // A lone `3`-leading sibling is taken (#2528).
                "bare_phase_lone_sibling" => "planned",
                // `14-2026-photos` is phase 14, not a malformed M-NN id.
                "year_slug_dir" => "planned",
                // `3-01-alpha` is not phase `3-01` (gsd-core wants `03-01`).
                "unpadded_dir" => "no_directory",
                // `03-01-01-setup-PLAN.md` does not pair with
                // `03-01-01-SUMMARY.md`, as upstream reads it.
                "slugged_plan" => "planned",
                "sibling_deferred_verification" => "planned",
                other => panic!(
                    "milestone_prefixed fixture shape `{other}` has no expected state mapped"
                ),
            })
            .or_else(|| {
                fixture
                    .name
                    .split("_gaps")
                    .next()
                    .and_then(|n| n.split("_human").next())
                    .and_then(|n| n.split("_unparseable").next())
                    .and_then(|n| n.split("_unterminated").next())
            })
            .unwrap_or(fixture.name);
        assert_eq!(
            observed, expected,
            "fixture `{}` is named for the `{expected}` state and this repository's reader \
             sees `{observed}`",
            fixture.name
        );
    }
}
