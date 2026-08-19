// ============================================================================
// The rule table and its guards (DRIVE-02, T-20-17, T-20-21)
//
// One rule governs every assertion below: **the table and the test table are
// derived from one another, and the guard fails in BOTH directions.** A rule
// with no row is a violation; a row that produces nothing is equally a
// violation. That shape is `tests/spawn_seam_guard.rs`'s, and its justification
// is the same one TESTING.md records for `SPAWN_ALLOWLIST` — an allowlist wider
// than the truth it describes has stopped being an audit and become a list of
// things somebody once had to add.
//
// It is an integration test rather than an in-source one because one of its
// guards reads the source tree, and a test that walks `src/` has no business
// living inside it. The value-property guards live here beside it so that
// "everything that proves the TABLE is a table" is one file.
//
// **The source walk covers `src/` only, never `tests/`.** That is what lets this
// file's own prose name the commands it forbids without invalidating its own
// gate.
// ============================================================================

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use gsd_meta_manager::driver::router::{
    decide, status_token, Decision, RouterAction, RouterReason, RuleRow, OBSERVED_NO_INFERENCE,
    RULE_TABLE, SAFE_COMMAND_ALPHABET,
};
use gsd_meta_manager::state_reader::disk_status::{DiskInference, DiskStatus};
use gsd_meta_manager::state_reader::roadmap_md::RoadmapPhase;
use gsd_meta_manager::state_reader::ProjectState;

/// The tree under audit. Resolved at compile time, so the test is
/// cwd-independent — the idiom `tests/spawn_seam_guard.rs:23` already uses.
const SRC_ROOT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src");

/// The module whose emitted-command alphabet is under audit.
const ROUTER_SOURCE: &str = "src/driver/router.rs";

/// The floor below which the alphabet scan is examining too little to mean
/// anything.
///
/// Non-vacuity in the register `tests/spawn_seam_guard.rs` and
/// `tests/async_blocking_guard.rs` both use: a scan that matched nothing passes
/// for the wrong reason, and "no violations" and "nothing examined" are the two
/// outcomes an assertion on emptiness alone cannot tell apart. Three, because
/// the alphabet has three members and each must appear at least once for the
/// set comparison below to be doing any work.
const MIN_COMMAND_LITERALS: usize = 3;

/// Commands GSD's runtime can emit, or a reader might reach for, that this
/// router must never auto-select.
///
/// Taken from the research document's never-auto-select table verbatim. The
/// enforcement is **structural** — `RouterAction` has three arms and every
/// emitted command is `format!("{verb} {phase}")` over one of them, so nothing
/// here is constructible — and this list exists to make the prohibition
/// greppable and to give the matcher a control arm with real inputs rather than
/// an invented one.
const NEVER_AUTO_SELECT: &[&str] = &[
    // Ends in an interactive merge-strategy prompt and rewrites ROADMAP.md and
    // PROJECT.md by judgement.
    "/gsd-complete-milestone",
    // Archives and prunes branches behind a confirmation.
    "/gsd-cleanup",
    // Its own outcomes are gates.
    "/gsd-audit-milestone",
    // Recursive self-invocation: the driver driving a driver.
    "/gsd-autonomous",
    // Routers. Selecting a router from a router is how a run re-selects the same
    // command forever, and it is what CONTEXT.md forbids as a default fallback.
    "/gsd-progress",
    "/gsd-next",
    // Unbounded scope; no deterministic state predicate selects them.
    "/gsd-undo",
    "/gsd-quick",
    "/gsd-fast",
    "/gsd-debug",
    // Push and PR side effects belong to the envelope's explicit paths, never to
    // a router branch.
    "/gsd-ship",
    "/gsd-pr-branch",
    // The unpark command a HUMAN runs. Auto-selecting it would be the router
    // answering the very question the verification gate exists to ask (DRIVE-05).
    "/gsd-verify-work",
];

/// One source file: its path relative to the crate root, and its lines.
type SourceFile = (String, Vec<String>);

/// Every `*.rs` file under `src/`, recursively, sorted by path.
fn source_files() -> Vec<SourceFile> {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut out = Vec::new();
    collect(Path::new(SRC_ROOT), &base, &mut out);
    assert!(
        !out.is_empty(),
        "the audit walked {SRC_ROOT} and found no Rust source at all, which means it is \
         auditing nothing"
    );
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
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
        out.push((relative, text.lines().map(str::to_string).collect()));
    }
}

/// Every command-shaped token on the non-comment lines of `lines`.
///
/// A "command-shaped token" is `/gsd-` followed by the identifier characters
/// that continue it, stopped at the first character that cannot be part of a
/// command verb. That is what turns the literal `"/gsd-plan-phase 20"` into the
/// verb `/gsd-plan-phase`, so the audit compares verbs against a verb alphabet
/// rather than comparing rendered commands against templates.
///
/// **Comment lines are dropped, and that filter is load-bearing rather than
/// tidy.** `src/driver/router.rs` documents its own alphabet by naming, in
/// prose, every command it refuses to emit. Without this filter the module's
/// honesty about what it will not do would be the thing that failed the audit —
/// the exact perverse incentive `tests/spawn_seam_guard.rs:262-271` records.
fn command_tokens(lines: &[String]) -> Vec<String> {
    const MARKER: &str = "/gsd-";
    let mut found = Vec::new();

    for line in lines {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            continue;
        }
        let mut from = 0;
        while let Some(offset) = line[from..].find(MARKER) {
            let at = from + offset;
            let token: String = line[at..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '/' || *c == '-')
                .collect();
            from = at + MARKER.len();
            found.push(token);
        }
    }

    found
}

// ---------------------------------------------------------------------------
// The alphabet guard: the declared list is EXACTLY what the router can emit
// ---------------------------------------------------------------------------

#[test]
fn the_declared_alphabet_is_exactly_the_set_of_commands_the_router_source_names() {
    let files = source_files();
    let router = files
        .iter()
        .find(|(path, _)| path == ROUTER_SOURCE)
        .unwrap_or_else(|| panic!("{ROUTER_SOURCE} must exist"));

    let tokens = command_tokens(&router.1);
    assert!(
        tokens.len() >= MIN_COMMAND_LITERALS,
        "the alphabet scan found only {} command-shaped tokens in {ROUTER_SOURCE}, below the \
         floor of {MIN_COMMAND_LITERALS}. A scan that matches nothing reports 'no violations' \
         for the wrong reason",
        tokens.len()
    );

    let observed: BTreeSet<&str> = tokens.iter().map(String::as_str).collect();
    let declared: BTreeSet<&str> = SAFE_COMMAND_ALPHABET.iter().copied().collect();

    let undeclared: Vec<&&str> = observed.difference(&declared).collect();
    assert!(
        undeclared.is_empty(),
        "{ROUTER_SOURCE} names command(s) outside the declared alphabet: {undeclared:?}. A \
         command whose workflow ends in an interactive prompt, rewrites milestone state, \
         recursively invokes the driver, or is itself a router must not be reachable from any \
         rule arm (T-20-17)"
    );

    let unemitted: Vec<&&str> = declared.difference(&observed).collect();
    assert!(
        unemitted.is_empty(),
        "the alphabet declares command(s) {unemitted:?} that {ROUTER_SOURCE} never names. The \
         guard fails in this direction too, because an allowlist wider than the truth it \
         describes has stopped being an audit"
    );
}

#[test]
fn the_command_matcher_fires_on_an_offending_snippet_and_spares_a_clean_one() {
    let offending: Vec<String> = vec![
        r#"    let command = "/gsd-complete-milestone 1".to_string();"#.to_string(),
        r#"    let router = format!("/gsd-progress");"#.to_string(),
    ];
    let hits = command_tokens(&offending);
    assert!(
        hits.iter().any(|t| t == "/gsd-complete-milestone"),
        "the matcher must find a forbidden command in executable code; found {hits:?}"
    );
    assert!(
        hits.iter().any(|t| t == "/gsd-progress"),
        "the matcher must find a router selected from a router; found {hits:?}"
    );

    let clean: Vec<String> = vec![
        "    // Never `/gsd-complete-milestone`: it ends in an interactive prompt."
            .to_string(),
        "    /// `/gsd-autonomous` would be the driver driving a driver.".to_string(),
        "    let command = row.action.command_for(target_phase);".to_string(),
    ];
    assert!(
        command_tokens(&clean).is_empty(),
        "a module that documents the commands it refuses to emit must not fail its own audit \
         for saying so; only non-comment lines are scanned"
    );
}

#[test]
fn no_never_auto_select_command_appears_in_the_router_source() {
    let files = source_files();
    let router = files
        .iter()
        .find(|(path, _)| path == ROUTER_SOURCE)
        .unwrap_or_else(|| panic!("{ROUTER_SOURCE} must exist"));
    let tokens = command_tokens(&router.1);

    for forbidden in NEVER_AUTO_SELECT {
        assert!(
            !tokens.iter().any(|token| token == forbidden),
            "{forbidden} is on the never-auto-select list and appears on an executable line \
             of {ROUTER_SOURCE}. Whatever put it there, an unattended run must not be able to \
             reach it (T-20-17)"
        );
    }

    // The two lists must also be disjoint as declarations, not merely as
    // observations — a forbidden command that had crept into the alphabet would
    // otherwise pass the scan above by being declared.
    for forbidden in NEVER_AUTO_SELECT {
        assert!(
            !SAFE_COMMAND_ALPHABET.contains(forbidden),
            "{forbidden} is declared both safe and never-auto-select; the two lists must be \
             disjoint or neither means anything"
        );
    }
}

// ---------------------------------------------------------------------------
// The both-directions rule/row guard
// ---------------------------------------------------------------------------

/// The actions no row in `rows` produces.
///
/// Extracted as a function over an arbitrary slice, rather than written inline
/// against [`RULE_TABLE`], so the control arm below can feed it a deliberately
/// truncated table and prove the check fires. A guard nobody has watched fail is
/// a guard nobody knows works.
fn actions_no_row_produces(rows: &[RuleRow]) -> Vec<RouterAction> {
    RouterAction::ALL
        .iter()
        .copied()
        .filter(|action| !rows.iter().any(|row| row.action == *action))
        .collect()
}

/// The rows in `rows` that [`decide`] does not turn into a command.
fn rows_that_produce_no_command(rows: &[RuleRow]) -> Vec<DiskStatus> {
    rows.iter()
        .filter(|row| {
            !matches!(
                decide(&state_with_status("20", row.observed), "20"),
                Decision::Run { .. }
            )
        })
        .map(|row| row.observed)
        .collect()
}

#[test]
fn every_row_produces_a_command_and_every_action_is_produced_by_a_row() {
    let barren = rows_that_produce_no_command(RULE_TABLE);
    assert!(
        barren.is_empty(),
        "these rows are declared in the rule table and produce no command: {barren:?}. A row \
         that routes to nothing is a table wider than the truth it describes"
    );

    let unreachable = actions_no_row_produces(RULE_TABLE);
    assert!(
        unreachable.is_empty(),
        "these actions are declared and no row produces them: {unreachable:?}. The guard \
         fails in BOTH directions on purpose — a rule with no row and a row with no rule are \
         the same class of drift"
    );
}

#[test]
fn the_both_directions_guard_fails_when_a_row_is_removed_from_the_table() {
    // Drop the last row (the `planned` → execute row) while leaving the action
    // declared, which is precisely the drift the guard exists to catch.
    let truncated = &RULE_TABLE[..RULE_TABLE.len() - 1];
    let unreachable = actions_no_row_produces(truncated);
    assert!(
        !unreachable.is_empty(),
        "the control arm: with a row removed and its action still declared, the guard must \
         report a violation. If it does not, the passing run above proved nothing"
    );
    assert!(
        unreachable.contains(&RouterAction::Execute),
        "the control arm must name the action the removed row produced; got {unreachable:?}"
    );
}

#[test]
fn no_two_rows_accept_the_same_observed_state() {
    let observed: Vec<DiskStatus> = RULE_TABLE.iter().map(|row| row.observed).collect();
    for (index, status) in observed.iter().enumerate() {
        for other in observed.iter().skip(index + 1) {
            assert_ne!(
                status, other,
                "two rows accepting one observed state makes 'the same state always yields \
                 the same choice' a property of arm ORDER rather than of the table. The \
                 disjointness is what lets `decide` use `find` and be correct"
            );
        }
    }

    // And the disjointness is a property of the DECIDED answer, not only of the
    // declaration: every covered state resolves to exactly one command.
    for row in RULE_TABLE {
        let matching: Vec<&RuleRow> = RULE_TABLE
            .iter()
            .filter(|candidate| candidate.observed == row.observed)
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "state {:?} is accepted by {} rows",
            row.observed,
            matching.len()
        );
    }
}

// ---------------------------------------------------------------------------
// The empty-input guard
// ---------------------------------------------------------------------------

fn roadmap_phase(number: &str) -> RoadmapPhase {
    RoadmapPhase {
        number: number.to_string(),
        name: String::new(),
        description: String::new(),
        completed: false,
        total_plans: 0,
        completed_plans: 0,
        depends_on: Vec::new(),
    }
}

fn state_with_status(number: &str, status: DiskStatus) -> ProjectState {
    let mut state = ProjectState {
        phases: vec![roadmap_phase(number)],
        ..Default::default()
    };
    state.phase_disk_statuses.insert(
        number.to_string(),
        DiskInference {
            status,
            ..Default::default()
        },
    );
    state
}

#[test]
fn every_empty_input_shape_parks_naming_what_was_observed_and_never_commands() {
    // 1. An empty phases vector: the roadmap declares nothing.
    let no_phases = ProjectState::default();

    // 2. A declared phase whose key is absent from the inference map.
    let mut absent_key = ProjectState {
        phases: vec![roadmap_phase("20")],
        ..Default::default()
    };
    absent_key
        .phase_disk_statuses
        .insert("19".to_string(), DiskInference::default());

    // 3. A declared phase and an entirely empty inference map.
    let empty_map = ProjectState {
        phases: vec![roadmap_phase("20")],
        ..Default::default()
    };

    for (label, state, expected_detail) in [
        ("empty phases vector", no_phases, "20"),
        ("phase map lacks the target key", absent_key, OBSERVED_NO_INFERENCE),
        ("empty phase map", empty_map, OBSERVED_NO_INFERENCE),
    ] {
        let decision = decide(&state, "20");
        match &decision {
            Decision::Park { detail, .. } | Decision::NoRule { observed: detail } => {
                assert_eq!(
                    detail, expected_detail,
                    "{label}: the park must NAME what was observed, or the gap it reports is \
                     unactionable"
                );
            }
            other => panic!(
                "{label}: must park rather than defaulting to a command. It must also never \
                 panic — the router runs inside a detached process whose whole value is \
                 surviving its parent. Got {other:?}"
            ),
        }
        assert!(
            !matches!(decision, Decision::Run { .. }),
            "{label}: an empty input must never produce a command, and least of all a \
             fallback to a router command"
        );
    }
}

// ---------------------------------------------------------------------------
// The determinism property
// ---------------------------------------------------------------------------

#[test]
fn a_hundred_decisions_over_two_insertion_orders_of_the_same_map_are_all_equal() {
    let entries = [
        ("18", DiskStatus::Complete),
        ("19", DiskStatus::Executed),
        ("20", DiskStatus::Discussed),
        ("21", DiskStatus::Empty),
    ];

    let build = |order: &[(&str, DiskStatus)]| {
        let mut state = ProjectState {
            phases: entries
                .iter()
                .map(|(number, _)| roadmap_phase(number))
                .collect(),
            ..Default::default()
        };
        for (number, status) in order {
            state.phase_disk_statuses.insert(
                (*number).to_string(),
                DiskInference {
                    status: *status,
                    ..Default::default()
                },
            );
        }
        state
    };

    let forward = build(&entries);
    let mut reversed_entries = entries;
    reversed_entries.reverse();
    let reversed = build(&reversed_entries);

    let expected = decide(&forward, "20");
    assert!(
        matches!(expected, Decision::Run { .. }),
        "the fixture must exercise a real routing decision, not a park"
    );

    for iteration in 0..100 {
        assert_eq!(
            decide(&forward, "20"),
            expected,
            "iteration {iteration}: `phase_disk_statuses` is a `HashMap` with undefined \
             iteration order. A routing decision that read it in map order would falsify \
             DRIVE-02's determinism claim in a way no single test run reveals — the same \
             binary over the same bytes would choose differently on different days"
        );
        assert_eq!(
            decide(&reversed, "20"),
            expected,
            "iteration {iteration}: two maps holding the same entries in different insertion \
             orders must route identically; if they do not, the router is reading the map in \
             iteration order"
        );
    }
}

#[test]
fn every_observed_status_has_a_stable_token_and_no_two_share_one() {
    let tokens: Vec<&str> = [
        DiskStatus::NoDirectory,
        DiskStatus::Empty,
        DiskStatus::Discussed,
        DiskStatus::Researched,
        DiskStatus::Planned,
        DiskStatus::Partial,
        DiskStatus::Executed,
        DiskStatus::Complete,
    ]
    .into_iter()
    .map(status_token)
    .collect();

    let unique: BTreeSet<&str> = tokens.iter().copied().collect();
    assert_eq!(
        unique.len(),
        tokens.len(),
        "two statuses sharing one token makes a park record ambiguous about what was seen: \
         {tokens:?}"
    );
    assert!(
        !unique.contains(OBSERVED_NO_INFERENCE),
        "`{OBSERVED_NO_INFERENCE}` must not collide with a real observed status — the whole \
         point of the token is that it says NOTHING was observed"
    );
}

#[test]
fn every_router_reason_is_greppable_by_its_producer_prefix() {
    for reason in RouterReason::ALL {
        let identifier = reason.as_str();
        assert!(
            identifier.starts_with("router_") || identifier.starts_with("gate_"),
            "`{identifier}` belongs to neither prefix. `router_` marks a refusal about the \
             rule table and `gate_` a human-judgement gate the run refused to answer; the \
             split is what makes 'how often does always-park stop a run, and where?' a grep \
             rather than a memory"
        );
    }
}
