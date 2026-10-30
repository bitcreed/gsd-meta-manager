---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 08
subsystem: infra
tags: [source-scanning-guard, async-blocking, lint-not-proof, project-gate, clippy-delta, traceability, safety-envelope]

# Dependency graph
requires:
  - phase: 19-gitsafe-git-blast-radius-envelope
    provides: "19-01's `tests/envelope_tracer.rs` namespace fixtures and the `src/envelope/` tree; 19-02's `classify_git` denied set and `ParkReason`; 19-03's gitignored-secret fixture and `envelope::scan`; 19-04's credential fixtures and `cred::build_env`; 19-05's PR ledger, `PreToolUse` guard and `write_settings`; 19-06's `probe_protection` and the pinned honesty statement; 19-07's `tests/envelope_wiring.rs` four on-disk `Parked` rows, `establish_envelope` and the two `terminal_label` join-failure fallbacks"
  - phase: 18-driver-tab
    provides: "commit `66c34ae`, which put every blocking call in the driver behind `spawn_blocking` — the discipline this plan turns from a comment into a test"
  - phase: 17-driver-supervisor
    provides: "`tests/spawn_seam_guard.rs` — the source-scanning guard shape, its `calls_marker` word-boundary rule, its declared-allowlist framing and its non-vacuity assertions"
  - phase: 16-run-journal
    provides: "`tests/driver_lock.rs:201-215` — the OBSERVED deadlock this guard exists because of"
provides:
  - "`tests/async_blocking_guard.rs` — the mechanical blocking-call-inside-`async fn` lint (D-29)"
  - "`ASYNC_BLOCKING_ALLOWLIST` — four justified `(file, marker)` exemptions, marker-scoped rather than file-scoped"
  - "`BLOCKING_MARKERS` / `BLOCKING_HELPERS` / `HANDOFF_MARKERS` — the declared marker set, syscalls plus this repository's own synchronous seams"
  - "`calls_marker` — the sibling's word-boundary matcher, generalised for markers that begin with `.`"
  - "`strip_literals` — literal-blanking so a brace inside a string cannot derail the tracker"
  - ".planning/REQUIREMENTS.md — SAFE-01, SAFE-02, SAFE-03, SAFE-05 and SAFE-06 marked complete"
affects: [20 router]

# Actuals (#2632)
actuals:
  tokens: 17400
  tasks: 2
  commits: 3

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Marker-scoped allowlisting: an entry is `(file, marker)` rather than a bare path, because file granularity on this guard would have put `src/driver/run.rs` — the run body — entirely out of scope for its two deliberate join-failure fallbacks. A guard blind to the file D-29 was written about is an off switch wearing an allowlist's costume."
    - "Naming the helpers is the consequence of the honest limit, not a refutation of it: a syscall-only marker set walks every driver `async fn` and finds nothing, because every blocking call this phase added sits behind `build_report` / `establish_envelope` / `terminal_label` / `lock::acquire`. `BLOCKING_HELPERS` makes the scanner able to see through *those* helpers and says outright that a new seam nobody adds is still invisible."
    - "Literal-blanking before brace counting, because the failure mode of a brace tracker is a false NEGATIVE: one `'{'` character literal makes the tracker leave an `async fn` body early and every blocking call below it goes unreported while the suite stays green."
    - "Clause-by-clause traceability: a success criterion joined by `and` has as many claims as it has clauses, and a table row per criterion lets the weaker half ride on the stronger half's test."

key-files:
  created:
    - tests/async_blocking_guard.rs
  modified:
    - .planning/REQUIREMENTS.md

key-decisions:
  - "In-source `#[cfg(test)]` modules are outside the walk, and the alternative was measured rather than assumed. Following the sibling's file-granular allowlist would have put `src/app.rs`, `src/driver/kill.rs` and `src/driver/mod.rs` on it for their `#[tokio::test]` bodies alone — and `src/driver/mod.rs` is where `drive` lives. A `#[tokio::test]` body owns its own runtime, has nothing else scheduled on it and blocks only itself, which is not the boundary D-29 protects."
  - "The allowlist holds `(file, marker)` pairs rather than bare file paths. Scoping the exemption to the marker that is actually deliberate keeps the rest of each file guarded, and the fail-first proof demonstrates it: a planted `.output()` in `drive` is reported even though `src/driver/mod.rs` already carries an allowlist entry for `build_report(`."
  - "`BLOCKING_HELPERS` widens D-29's marker set from syscalls to this repository's own synchronous seams. Measured before deciding: with the syscall set alone, the production tree yields ZERO hits and ZERO allowlist entries, because every blocking call the driver makes is behind a helper. A guard with an empty allowlist over 1697 lines of async body is the vacuous gate this phase argues is worse than no gate."
  - "`calls_marker`'s left word boundary is applied only when the marker begins with an identifier character. The sibling applies it unconditionally, which — applied to `.spawn()` — rejects every real hit, because the character before the `.` is always the end of a receiver name. That is why `src/executor/claude.rs`'s spawn site was invisible to the first draft."
  - "An awaited call is exempt, and that is what tells `tokio::process::Command` apart from `std::process::Command` lexically. `src/state_reader/git_ops.rs`'s async git reads are not violations and reporting them would have made the guard unusable."
  - "`async` blocks are NOT tracked, only `async fn` bodies. D-29 scopes the boundary to `async fn`; widening the tracker to `async move {` means guessing at closure boundaries a brace counter cannot settle. Stated in the file's own doc as an out-of-scope limit rather than left to be discovered."
  - "SAFE-01 is marked complete in the sense its own sentence carries — the model's cooperation is removed from the equation — and NOT in the sense that no escape exists. 19-06's pinned honesty statement already records that an agent which can spawn an unsupervised shell is past the last client-side layer. Marking it complete while pointing at that constant is the honest position; marking it complete silently would be the overstatement D-27 forbids."

patterns-established:
  - "A guard's own doc states what a pass does NOT mean, in the same paragraph as what it does. `tests/async_blocking_guard.rs` says a green run means 'no NAMED blocking call appears lexically inside an `async fn` body without an intervening hand-off' and that it does not mean 'this program never blocks its runtime', because a reader who takes the second reading has been misled by the file."
  - "Non-vacuity as a floor plus a named-file requirement: `MIN_ASYNC_BODY_LINES` catches a walk that stopped walking, and `REQUIRED_ASYNC_FILES` catches a walk that walks but never reaches `src/driver/mod.rs` or `src/driver/run.rs` — the two files D-29 is about."
  - "Traceability rows record the SHAPE of the proof, not only its name: a clause proved by composition (spelling → reason by unit test, reason → park by fixture) says so, and a clause proved at the guard rather than at the forge says that too."

requirements-completed: [SAFE-01, SAFE-02, SAFE-03, SAFE-05, SAFE-06]

coverage:
  - id: D1
    description: "A known-blocking call appearing lexically inside an `async fn` body in `src/`, with no intervening blocking-task hand-off and no allowlist entry, fails a test"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/async_blocking_guard.rs#every_blocking_call_inside_an_async_fn_is_handed_off_or_allowlisted"
        status: pass
      - kind: other
        ref: "fail-first: a planted `std::process::Command::new(\"git\").arg(\"status\").output()` on the first line of `driver::drive` turns it red naming `src/driver/mod.rs:193 [.output()]` (executed, then reverted; `git diff` empty afterwards)"
        status: pass
      - kind: integration
        ref: "tests/async_blocking_guard.rs#the_scanner_reports_a_planted_blocking_call_and_spares_a_handed_off_one (synthetic control arm: multi-line signature, hand-off, fallback-outside-the-closure, awaited call, plain `fn`)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The guard's own doc states that it is a lint and not a proof — a lexical scanner cannot see through a helper function — and cites the observed deadlock that justifies having it anyway"
    requirement: SAFE-01
    verification:
      - kind: other
        ref: "grep -c 'a lint, not a proof' tests/async_blocking_guard.rs returns 1; grep -c 'driver_lock.rs' returns 2"
        status: pass
    human_judgment: true
    rationale: "A grep proves the sentence is present and that the deadlock is cited by location. Whether the paragraph reads as an honest account of what a lexical scanner can and cannot do — rather than as a pre-excuse for every gap the guard will miss — is a judgement only a reader can make. It is the same judgement 19-01 D9, 19-02 D8, 19-04 D11, 19-05 D11 and 19-06 D5 each recorded, and this phase's transparency prohibitions verify by judgment for exactly this reason."
  - id: D3
    description: "The allowlist carries a one-line justification per entry, and an entry that no longer suppresses anything is refused"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/async_blocking_guard.rs#no_allowlist_entry_is_stale (all four entries confirmed to suppress a real, currently-present hit)"
        status: pass
      - kind: other
        ref: "each of the four `ASYNC_BLOCKING_ALLOWLIST` entries is preceded by its own comment block stating why the block is deliberate"
        status: pass
    human_judgment: false
  - id: D4
    description: "Marker matching is word-boundary aware rather than substring-based, so an identifier that merely contains a marker's letters does not trip the guard"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/async_blocking_guard.rs#a_marker_that_merely_shares_letters_with_an_identifier_does_not_trip_the_guard (with the control arm asserting genuine calls are still found)"
        status: pass
      - kind: integration
        ref: "tests/async_blocking_guard.rs#a_literal_carrying_a_brace_does_not_derail_the_brace_tracker"
        status: pass
      - kind: integration
        ref: "tests/async_blocking_guard.rs#every_named_blocking_helper_still_exists_in_the_tree (stale-marker guard: a rename cannot silently empty a marker)"
        status: pass
    human_judgment: false
  - id: D5
    description: "`cargo build`, `cargo test` and `cargo clippy -- -D warnings` all exit 0, and the all-targets lint count is exactly the five pre-existing lints with no growth"
    requirement: SAFE-02
    verification:
      - kind: other
        ref: "cargo build exit 0; cargo clippy -- -D warnings exit 0; rtk proxy cargo test → 993 passing, 0 failed"
        status: pass
      - kind: other
        ref: "rtk proxy cargo clippy --all-targets → exactly 5 lints at browser.rs:131/132/133, project_creator.rs:146, state_reader/mod.rs:258 — count and locations unchanged since 19-01"
        status: pass
    human_judgment: false
  - id: D6
    description: "Every clause of every ROADMAP success criterion maps to a named passing test inside the phase's own scope fence, including the park clauses of criteria 2 and 5"
    requirement: SAFE-01
    verification:
      - kind: integration
        ref: "tests/envelope_wiring.rs#a_force_push_is_refused_and_lands_a_force_push_blocked_park (run individually: 1 passed)"
        status: pass
      - kind: integration
        ref: "tests/envelope_wiring.rs#a_hooks_path_rewrite_is_refused_and_lands_a_hook_bypass_blocked_park (run individually: 1 passed)"
        status: pass
      - kind: integration
        ref: "tests/envelope_wiring.rs#a_worktree_carrying_a_credential_is_refused_and_lands_a_secret_detected_park (run individually: 1 passed)"
        status: pass
      - kind: integration
        ref: "tests/envelope_wiring.rs#a_pull_request_beyond_the_cap_is_refused_and_lands_a_pr_cap_exceeded_park (run individually: 1 passed)"
        status: pass
      - kind: e2e
        ref: "tests/envelope_tracer.rs (6), tests/envelope_hook_refusals.rs (7), tests/envelope_credential.rs (6), tests/envelope_pr_cap.rs (11), src/envelope/policy.rs (47) — every named clause test run and passing"
        status: pass
    human_judgment: true
    rationale: "Every clause row names a test that exists and passes when run alone. Three rows are covered by COMPOSITION rather than end to end and say so in the table — criterion 2's park clause is proved once per park reason and the spelling→reason mapping is a separate unit test; criterion 5's `instead of opening another PR` is proved at the guard's deny rather than at a forge, because D-35 forbids the real pull request; criterion 4's `per-run scoped` is proved as host-scoping and run-delivery, not as the credential's own authorization scope. Whether a composed proof is an adequate proof of the sentence as written is a reader's judgement, and recording it as one is the point of the exercise."
  - id: D7
    description: "SAFE-01, SAFE-02, SAFE-03, SAFE-05 and SAFE-06 are marked complete in REQUIREMENTS.md only after their named tests were shown passing"
    requirement: SAFE-06
    verification:
      - kind: other
        ref: "grep -c '\\[x\\] \\*\\*SAFE-0[12356]\\*\\*' .planning/REQUIREMENTS.md returns 5; grep 'SAFE-06 |' | grep -c 'Pending' returns 0"
        status: pass
      - kind: other
        ref: "requirements.ready-ids reports 5/5 ready — the shared-ID gate released them because 19-01 through 19-07 have all produced a SUMMARY"
        status: pass
    human_judgment: false

# Metrics
duration: 55 min
completed: 2026-08-18
status: complete
---

# Phase 19 Plan 08: The Async-Blocking Lint and the Clause-by-Clause Gate Summary

**The blocking-call-inside-`async fn` boundary stops being four comments and becomes a test that fails when a `std::process::Command::new("git").…output()` is planted on the first line of `drive` — a lint whose own doc says it is a lint, whose allowlist carries four justified `(file, marker)` entries rather than four blinded files, and which reports 993 passing tests, exactly five unchanged pre-existing clippy lints, and a traceability table split clause by clause so that "blocked" and "parks the run" are two claims with two pieces of evidence.**

## Performance

- **Duration:** ~55 min
- **Started:** 2026-08-18 (wave base `aec4e4b`)
- **Completed:** 2026-08-18
- **Tasks:** 2
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments

- **The guard is proved fail-first, and the proof also demonstrates why the allowlist is marker-scoped.** Planting `std::process::Command::new("git").arg("status").output()` on the first line of `driver::drive` turns `every_blocking_call_inside_an_async_fn_is_handed_off_or_allowlisted` red naming `src/driver/mod.rs:193 [.output()]` — *even though that file already carries an allowlist entry* for `build_report(`. Executed, observed, reverted; `git diff` is empty afterwards. Under a file-granular allowlist that plant would have passed silently.
- **The marker set was measured before it was chosen, and the measurement changed it.** With D-29's syscall list alone the production tree yields **zero** hits across 1697 lines of `async fn` body — not because the tree is exemplary but because every blocking call the driver makes sits behind a helper (`build_report` shells out to `git` twice, `establish_envelope` writes four envelope layers, `terminal_label` reads a whole journal, `lock::acquire` calls `flock`). A guard with an empty allowlist over that surface is the vacuous gate this phase argues is worse than no gate. `BLOCKING_HELPERS` names thirteen of this repository's own seams — seven of them Phase 19's — and the file states plainly that naming them does not make the scanner able to see through *a* helper, only through *these* helpers.
- **The honest limit is the file's own doc, not a footnote.** A green run means "no NAMED blocking call appears lexically inside an `async fn` body without an intervening hand-off". It does not mean "this program never blocks its runtime", and the header says so in those words, because anybody who reads a pass as the second statement has been misled by the file. Two further limits are stated rather than left to be discovered: `async` blocks are out of scope, and in-source `#[cfg(test)]` modules are not walked.
- **Two false-negative traps were closed before they could hide anything.** `calls_marker`'s left word boundary is applied only when the marker begins with an identifier character — the sibling applies it unconditionally, and applied to `.spawn()` that rejects every real hit, since the character before the `.` is always the end of a receiver name. That defect made `src/executor/claude.rs`'s agent spawn invisible to the first draft. And `strip_literals` blanks string and character literals before braces are counted, because one `'{'` makes the tracker leave an `async fn` body early and every blocking call below it goes unreported while the suite stays green.
- **Four deliberate blocks, four reasons.** The TUI's `$EDITOR` suspend-and-edit (`ratatui::restore()` has already handed over the terminal, so there is nothing to draw); the agent spawn (long-lived and duplex, and `src/executor/claude.rs`'s module doc already said so); and the three join-failure fallbacks 19-07 predicted would need naming — the dry-run report and both terminal labels, each reachable only if a `spawn_blocking` task failed to join, each existing so a preview or a park reason survives a path no healthy run reaches.
- **The gate is green and the lint delta is zero.** `cargo build` exit 0, `cargo clippy -- -D warnings` exit 0, `cargo test` **993 passing / 0 failed** (986 after 19-07, 773 at the `740e62f` baseline). `cargo clippy --all-targets` reports exactly the five pre-existing lints at unchanged locations. The known `tests/driver_reattach.rs` flake did **not** reproduce on the gate run; that is luck rather than a fix, and the honest reporting of it is below.
- **Five requirements marked complete against named tests, and three clauses marked as composed rather than end-to-end.** The traceability table quotes each ROADMAP criterion verbatim and splits it, so criterion 2's `and the attempt parks the run` and criterion 5's `parks the run instead of opening another PR` each carry their own row naming an on-disk `Parked` event — not the refusal test they sit beside.

## Task Commits

1. **Task 1: `tests/async_blocking_guard.rs` — a lint, and its doc says so** — `4b33f24` (test)
2. **Task 2: SAFE-01/02/03/05/06 complete, against named tests rather than a green build** — `99f7a7f` (docs)

Task 2's other output — the traceability table and the clippy/test measurements — is this document, and its ROADMAP half is deliberately *not* committed here: see **Requested ROADMAP edit** at the end.

## Files Created/Modified

- `tests/async_blocking_guard.rs` (new, 917 lines) — `SRC_ROOT`, `BLOCKING_MARKERS` (17), `BLOCKING_HELPERS` (13), `HANDOFF_MARKERS`, `TEST_MODULE_EXCLUSION`, `ASYNC_BLOCKING_ALLOWLIST` (4 justified `(file, marker)` entries), `MIN_ASYNC_BODY_LINES`, `REQUIRED_ASYNC_FILES`, `Hit`, `calls_marker`, `strip_literals` + `skip_raw`/`skip_quoted`/`skip_char_literal`, `is_awaited`, `opens_async_fn`, `opens_module`, `Position`, `scan`, `source_files`/`collect`, `is_allowlisted`, `render`, `audit`; 7 tests
- `.planning/REQUIREMENTS.md` — SAFE-01, SAFE-02, SAFE-03, SAFE-05, SAFE-06 ticked in the checklist and flipped Pending → Complete in the traceability table

## The gate

| Check | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo clippy -- -D warnings` | exit 0 |
| `cargo test` | **993 passing, 0 failed** (986 after 19-07; 773 at the `740e62f` baseline; +7 this plan) |
| `cargo clippy --all-targets` (raw, via `rtk proxy`) | **exactly 5** pre-existing lints — `browser.rs:131/132/133` (`bool_assert_comparison` ×3), `project_creator.rs:146` (`cmp_owned`), `state_reader/mod.rs:258` (`items_after_test_module`). Count and locations unchanged since 19-01. |
| `cargo clippy --test async_blocking_guard -- -D warnings` | exit 0 |
| `cargo test --test async_blocking_guard` | exit 0 — 7 tests |
| `cargo test --test envelope_wiring` | exit 0 — 14 tests; the four `Parked` rows each re-run **individually**, 1 passed each |
| `cargo test --test envelope_tracer` | exit 0 — 6 tests |
| `cargo test --test envelope_hook_refusals` | exit 0 — 7 tests |
| `cargo test --test envelope_credential` | exit 0 — 6 tests, none skipped |
| `cargo test --test envelope_pr_cap` | exit 0 — 11 tests |
| `cargo test --lib envelope::policy` | exit 0 — 47 tests |
| `cargo test --lib envelope::cred` / `::ledger` / `::hooks` | exit 0 — 20 / 16 / 33 tests |
| `grep -c 'a lint, not a proof' tests/async_blocking_guard.rs` | 1 (≥ 1 required) |
| `grep -c 'driver_lock.rs' tests/async_blocking_guard.rs` | 2 (≥ 1 required) — the observed deadlock cited by location |
| `grep -c 'ASYNC_BLOCKING_ALLOWLIST' tests/async_blocking_guard.rs` | 4 (≥ 2 required); each of the 4 entries preceded by its own comment block |
| `grep -c 'fn calls_marker' tests/async_blocking_guard.rs` | 1 — word-boundary matching, not `contains` |
| `grep -c 'CARGO_MANIFEST_DIR' tests/async_blocking_guard.rs` | 2; the joined subpath is `src`, never `tests` |
| `grep -c '\[x\] \*\*SAFE-0[12356]\*\*' .planning/REQUIREMENTS.md` | **5** |
| `grep 'SAFE-06 \|' .planning/REQUIREMENTS.md \| grep -c 'Pending'` | **0** (reported anchored — see deviation 3) |
| `Cargo.toml` unchanged | **no crate added** — T-19-SC holds phase-wide, for all eight plans |

**On the clippy measurement.** The plan's criterion is `rtk proxy cargo clippy --all-targets -- -D warnings 2>&1 | grep -c '^warning:'` returning 5. As written it returns **0**, and it returns 0 *vacuously*: `-D warnings` promotes every lint to an error, so no line begins with `warning:` at all. The count is reported two ways instead, both yielding 5 — `rtk proxy cargo clippy --all-targets 2>&1 | grep -E '^warning: '` (5 lint lines plus a `generated 5 warnings` summary line), and the `-D warnings` form's `error: could not compile … due to 5 previous errors`. This is precisely the shape of T-19-51 (a vacuous gate over transformed output) arriving through the flag rather than through the `rtk` filter, which is why it is reported rather than quietly restated. See deviation 2.

## Traceability: every clause of every ROADMAP success criterion

Each criterion is quoted **verbatim** from `.planning/ROADMAP.md` and split into its clauses. Every named test was run individually and passed. `[C]` marks a clause proved by composition rather than end to end, and each one says what the composition is.

### Criterion 1

> "A driven run configured to push to `main` is rejected by the envelope, with the model's cooperation removed from the equation"

| Clause | Named test | File | Result |
|---|---|---|---|
| 1a — "A driven run configured to push to `main` is rejected by the envelope" | `a_driven_push_to_main_is_refused_and_the_remote_ref_never_appears` | `tests/envelope_tracer.rs` | pass |
| 1b — "with the model's cooperation removed from the equation" | same test. This clause is a property of the fixture, not a second assertion: it drives a real `git push` against a `file://` bare repository under the envelope's environment and **spawns no agent at all** (D-31). Stated as a fixture property rather than dressed up as an independent test. | `tests/envelope_tracer.rs` | pass |
| 1 — paired allow (D-32), so the envelope is a boundary and not a wall | `a_driven_push_inside_the_reserved_namespace_reaches_the_remote` | `tests/envelope_tracer.rs` | pass |
| 1 — load-bearing | mutation: forcing `classify_push_ref` to `Allow` reds 1a; forcing it to `Refuse` reds the paired allow (19-01, executed and reverted) | — | pass |

### Criterion 2

> "`git push --force`, `+refs/…`, `--no-verify`, and `core.hooksPath` rewrites from a driven run are all blocked, and the attempt parks the run"

| Clause | Named test | File | Result |
|---|---|---|---|
| 2a — "`git push --force` … blocked" | `every_force_push_spelling_is_refused` (`--force`, `-f`, bundled `-fu`, `--force-with-lease` bare and `=value`, `--force-if-includes`, `--mirror`, `--delete`, `-d`) | `src/envelope/policy.rs` | pass |
| 2a — end to end, non-zero exit | `a_force_push_is_refused_and_lands_a_force_push_blocked_park` | `tests/envelope_wiring.rs` | pass (individually) |
| 2b — "`+refs/…` … blocked" | `a_plus_prefixed_refspec_is_refused_because_it_is_a_force_push_by_another_spelling` | `src/envelope/policy.rs` | pass |
| 2c — "`--no-verify` … blocked" | `push_no_verify_is_refused_as_a_hook_bypass` | `src/envelope/policy.rs` | pass |
| 2d — "`core.hooksPath` rewrites … blocked" | `every_writing_form_of_config_touching_core_hookspath_is_refused_at_every_scope` and `the_command_line_config_form_that_outranks_the_envelope_is_refused` | `src/envelope/policy.rs` | pass |
| 2d — end to end, non-zero exit | `a_hooks_path_rewrite_is_refused_and_lands_a_hook_bypass_blocked_park` | `tests/envelope_wiring.rs` | pass (individually) |
| **2e — "and the attempt parks the run"**, `force_push_blocked` | `a_force_push_is_refused_and_lands_a_force_push_blocked_park` — asserts the **on-disk `Parked` event** carrying `force_push_blocked`, read back by a process other than the one that refused, plus the reason on the terminal `run.json` | `tests/envelope_wiring.rs` | pass (individually) |
| **2e — "and the attempt parks the run"**, `hook_bypass_blocked` | `a_hooks_path_rewrite_is_refused_and_lands_a_hook_bypass_blocked_park` — same four facts, reason `hook_bypass_blocked` | `tests/envelope_wiring.rs` | pass (individually) |
| 2 — paired allow | `a_permitted_command_is_permitted_and_parks_nothing` | `tests/envelope_wiring.rs` | pass |

**`[C]` on 2e.** The park is proved **once per park reason**, not once per spelling. Two facts compose it: every spelling in 2a–2d is unit-proved to classify to `force_push_blocked` or `hook_bypass_blocked` (`every_force_push_spelling_is_refused`, `push_no_verify_is_refused_as_a_hook_bypass`, `every_writing_form_of_config_touching_core_hookspath_is_refused_at_every_scope`), and each of those two reasons is fixture-proved to land as a `Parked` event on disk. No test drives `git push +refs/heads/main:refs/heads/main` end to end and then reads the journal. Stated because the difference between "every spelling parks" and "every spelling classifies to a reason that parks" is exactly the kind of gap a per-criterion row would have hidden.

### Criterion 3

> "A push carrying a detectable secret is blocked before it leaves the machine, including a secret written to a gitignored path"

| Clause | Named test | File | Result |
|---|---|---|---|
| 3a — "blocked before it leaves the machine" | `a_credential_on_a_path_the_ignore_rules_cover_still_blocks_the_push` — non-zero exit from the real generated `pre-push` stub **and** the remote ref absent from `git ls-remote` | `tests/envelope_hook_refusals.rs` | pass |
| 3b — "including a secret written to a gitignored path" | same test; the fixture asserts with `git check-ignore` that the planted `secrets/prod.pem` really is covered by the repository's own ignore rules before asserting the refusal | `tests/envelope_hook_refusals.rs` | pass |
| 3b — load-bearing | mutation: teaching `scan_file` to honour `.gitignore` reds it with `findings: none` and `[new branch] HEAD -> gsd-auto/gitignored/sweep` on the remote (19-03, executed and reverted) | — | pass |
| 3 — the report never reproduces the secret | `the_rendered_report_never_contains_the_planted_secret` | `src/envelope/scan.rs` | pass |
| 3 — paired allow | `a_clean_worktree_pushes_inside_the_namespace_and_succeeds` | `tests/envelope_hook_refusals.rs` | pass |
| 3 — park (extra; the sentence has no park clause) | `a_worktree_carrying_a_credential_is_refused_and_lands_a_secret_detected_park` — a **real** `git push` git ran the generated stub for, with the on-disk `Parked` event and the remote ref byte-identical to before | `tests/envelope_wiring.rs` | pass (individually) |

### Criterion 4

> "A driven run pushes using a per-run scoped credential and still works with the user's ambient credentials and SSH agent unavailable to it"

| Clause | Named test | File | Result |
|---|---|---|---|
| 4a — "pushes using a per-run scoped credential" `[C]` | `the_responder_answers_the_configured_host_and_refuses_every_other_one` (end to end, through the real binary); `a_second_remote_gets_an_authentication_failure_rather_than_the_token`; `nothing_the_envelope_wrote_holds_the_credential` | `tests/envelope_credential.rs`, `src/envelope/cred.rs` | pass |
| 4b — "with the user's ambient credentials … unavailable to it" | `a_credential_helper_the_user_really_has_stops_being_resolvable_under_the_envelope` — a **planted control**: it writes `[credential] helper = store` into a fake `HOME`, proves plain git resolves it, and only then asserts the envelope does not | `tests/envelope_credential.rs` | pass |
| 4b — "… and SSH agent unavailable to it" | `the_agent_is_removed_and_every_config_scope_is_redirected_into_the_envelope`; `the_ambient_ssh_agent_is_removed_rather_than_overwritten` | `tests/envelope_credential.rs`, `src/envelope/cred.rs` | pass |
| 4b — "still works" | `a_push_inside_the_reserved_namespace_succeeds_with_home_emptied` — exit zero and the ref present in `git ls-remote`, with `HOME` pointed at an empty directory | `tests/envelope_credential.rs` | pass |
| 4b — load-bearing | mutation: replacing `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` with inert names reds the helper test with `the envelope still resolves a credential helper ("store\n")` (19-04, executed and reverted) | — | pass |

**`[C]` on 4a.** What the tests prove is **host scoping and per-run delivery**: the host is resolved once at run start from `remote.origin.url` and baked into the generated `<envelope>/<alias>/askpass` stub so the driven agent cannot move it, the token crosses exactly one prompt and is never at rest, never on argv and never in `.git/config`, and `RUN_ID_ENV` identifies the run to the guard. What no test proves is that the credential's own **authorization** scope is narrow — that is whatever the user's `CredentialSource` yields, and D-18 deliberately withholds an administration scope rather than asserting one. 19-04 additionally recorded, and this table does not soften, that an agent inside the driven run can execute the responder itself and read the token off its stdout.

### Criterion 5

> "Exceeding the per-project 24-hour PR cap parks the run instead of opening another PR"

| Clause | Named test | File | Result |
|---|---|---|---|
| 5a — "Exceeding the per-project 24-hour PR cap" is detected | `the_third_pull_request_in_the_window_succeeds_and_the_fourth_is_refused` — the allow and the refusal in one fixture | `tests/envelope_pr_cap.rs` | pass |
| 5a — the window's boundary | `an_attempt_exactly_at_the_boundary_still_counts_against_the_window`; `an_attempt_that_has_fallen_out_of_the_window_releases_its_slot` | `tests/envelope_pr_cap.rs` | pass |
| 5a — the per-run bound is a separate bound | `a_second_attempt_in_one_run_is_refused_while_the_window_still_has_capacity` | `tests/envelope_pr_cap.rs` | pass |
| **5b — "parks the run"** | `a_pull_request_beyond_the_cap_is_refused_and_lands_a_pr_cap_exceeded_park` — asserts the **on-disk `Parked` event** carrying `pr_cap_exceeded`, plus the reason on the terminal `run.json` | `tests/envelope_wiring.rs` | pass (individually) |
| **5c — "instead of opening another PR"** `[C]` | same test: the `PreToolUse` guard **denies** the call — protocol JSON on stdout and exit 2 with the reason on stderr — so `gh pr create` is never executed. `the_refused_attempt_is_on_disk_because_the_ledger_records_before_it_permits` additionally shows a read-only `gh pr list` adds no ledger line, so the classifier is not simply refusing everything. | `tests/envelope_wiring.rs`, `tests/envelope_pr_cap.rs` | pass |
| 5 — the ledger is out of the agent's reach | `the_ledger_sits_under_the_envelope_and_carries_no_component_of_the_repository` | `src/envelope/ledger.rs` | pass |

**`[C]` on 5c.** "Instead of opening another PR" is proved **at the guard's deny**, not by observing that no pull request appeared on a forge. D-35 forbids the real pull request, and the fence is applied as a check rather than relaxed: a criterion that could only be met by opening a real PR would be the wrong criterion, not the wrong fence. The gap between "the tool call was denied" and "no PR exists" is the width of a path the guard never saw — which is T-19-35, **accepted and stated** in 19-05, with `CapVerdict::refusal_detail` telling the user in the refusal itself that a recorded-but-failed attempt costs a slot.

### D-35 fence check

No clause in any table above is satisfied by a real pull request, a real network push, an external scanner installation, an agent actually running, or a Phase 20 decision router. Every end-to-end row runs against a `file://` bare repository, a loopback listener, or an in-process fixture. Two clauses were re-stated (`[C]` on 5c and 4a) rather than relaxing the fence to reach them.

## What is NOT proved — carried forward, not ticked

This section exists because an overstated safety claim is worse than a stated limitation: it gets trusted. Everything below is a real gap in Phase 19's evidence, and none of it is marked green anywhere in this document.

1. **The honesty statement's tone is not test-proved.** 19-06 recorded coverage D5 as `human_judgment: true`: a test asserts that `SECTION_ENVELOPE`'s three required parts are present and ordered, and nothing can assert that the paragraph *reads* as an honest account of the real ceiling rather than as a hedge or a pre-excuse. This is the single item in Phase 19 that most needs a human's eye, and it is still open.
2. **The protection probe's budget path has no test driving a real stall.** 19-06 coverage D8, also `human_judgment: true`. The 10-second deadline and kill-at-deadline branch exist and are documented, but no fixture hangs a remote — D-35 forbids the network fixture that would do it. That the bound is correct under a real stall is reasoned, not observed.
3. **The pull-request cap has no git-hook second carrier.** 19-05 recorded this as a degradation rather than a disarming, and it is the honest weak point of criterion 5: no git hook observes `gh pr create`, because it is not a git operation. A settings file the agent's own CLI silently ignores leaves the cap **unenforced**, while every push boundary stays standing. `settings_value`'s doc says so at the generator; this document says so at the criterion.
4. **An agent inside the run can read the token.** 19-04's stated residual: any credential a run can push with is one the run can read, by executing `<envelope>/<alias>/askpass` itself. No envelope can close that. What D-17 buys is that the token is not at rest, not in the process table and not in `.git/config`.
5. **No test reads the driven child's actual environment.** 19-07 coverage D3, `human_judgment: true`. Greps prove there is exactly one applier and that it removes as well as sets; that the applied environment is what the child *sees* is reasoned from `Command`'s contract, because reading `/proc/<pid>/environ` needs the agent CLI D-35 keeps out of the suite.
6. **`gitleaks`'s Blocked and Failed arms are unexercised.** 19-03 coverage D8, `human_judgment: true`. The binary is not installed on this machine and D-35 forbids a criterion that requires installing it. Only the `Absent` arm — the one that must never fail open — runs on every push. `gitleaks` is additive; criterion 3 is carried by the built-in rules, which are proved.
7. **This plan's own guard is a lint.** It cannot see through a helper it does not name, through a trait object, through a macro expansion, or into an `async` block. `BLOCKING_HELPERS` narrows the gap for thirteen named seams; a fourteenth that nobody adds is invisible.
8. **SAFE-01's "cannot talk its way past" is true of the model, not of a shell.** The mechanism does not depend on the agent's cooperation and that is proved with the model removed from the process entirely. It is *not* proved that no escape exists: an agent that can spawn an unsupervised shell and unset `GIT_CONFIG_COUNT` is past the last client-side layer, which `src/envelope/mod.rs`, `src/envelope/cred.rs` and the pinned `SECTION_ENVELOPE` all say outright. Marking SAFE-01 complete against that reading, and pointing at the constant that records the ceiling, is the honest position. The phase's own conclusion remains D-27's: **enable server-side branch protection.**

## Decisions Made

See `key-decisions` in the frontmatter. The three a later reader is most likely to want the reasoning for:

- **In-source `#[cfg(test)]` modules are outside the walk, and the alternative was measured.** With test modules included, the guard reports six hits — two in `src/app.rs`, three in `src/driver/kill.rs`, one in `src/driver/mod.rs` — every one of them inside a `#[tokio::test]` body doing deliberate setup (`src/driver/kill.rs` spawns a real `sleep` child precisely so an "already gone" pid is a pid that really was reaped). Allowlisting them at the sibling's file granularity would have blinded the guard on `src/driver/mod.rs`, where `drive` lives. The boundary D-29 protects is between blocking work and a runtime that has *other* futures to poll; a `#[tokio::test]` body has none and blocks only itself.
- **The allowlist is `(file, marker)`, not bare file paths.** The plan specifies file paths, matching `SPAWN_ALLOWLIST`. The sibling's granularity is adequate for its question ("why does this file spawn?") and is not adequate for this one: `src/driver/run.rs` would have gone entirely out of scope for two deliberate join-failure fallbacks, and it is the run body. The extra column is proved to be doing work by the fail-first plant, which is reported despite `src/driver/mod.rs` already carrying an entry.
- **`BLOCKING_HELPERS` widens D-29's marker set, deliberately and with the reason in the file.** This was the one genuine fork. D-29 names syscalls; with syscalls alone the production tree is clean and the allowlist is empty, which would have shipped an artifact that lints nothing today and has no recorded exemptions to keep honest. Naming the helpers gives the guard the surface 19-07's handoff note assumed it would have, and it catches the regression that is actually likely — someone calling `establish_envelope` or `lock::acquire` from an `async fn` without the hand-off. The widening is recorded in the const's own doc as a consequence of the "lint, not a proof" limit rather than as a refutation of it.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] `calls_marker`'s word boundary made every `.`-prefixed marker unmatchable**

- **Found during:** Task 1
- **Issue:** The sibling's `calls_marker` rejects a match whose preceding character could continue an identifier. Applied to a marker that already begins with `.` — `.output()`, `.status()`, `.spawn()` — the preceding character is always the last letter of the receiver name, so **every** real hit was rejected. `src/executor/claude.rs:502`'s `wrap.spawn()` and `src/main.rs:404`'s `$EDITOR` `.status()` were both invisible to the first draft, which is a false negative: the guard would have shipped reporting green over two genuine blocking calls.
- **Fix:** the boundary is applied only when the marker's first character could itself continue an identifier. A `.` already is a boundary. The reasoning is at the function, and `a_marker_that_merely_shares_letters_with_an_identifier_does_not_trip_the_guard` asserts both directions.
- **Files modified:** `tests/async_blocking_guard.rs`
- **Verification:** the two hits appear and are allowlisted with reasons; `no_allowlist_entry_is_stale` confirms both entries suppress a real hit
- **Committed in:** `4b33f24`

**2. [Process] The clippy-delta criterion counts a prefix that `-D warnings` removes**

- **Found during:** Task 2, running the gate
- **Issue:** `rtk proxy cargo clippy --all-targets -- -D warnings 2>&1 | grep -c '^warning:'` is specified to return 5. It returns **0**, and vacuously: `-D warnings` promotes every lint to an `error:`, so no line begins with `warning:`. Reported as-is, the criterion would have passed nothing and measured nothing — which is T-19-51's own shape (a gate over transformed output), arriving through the flag rather than through the `rtk` filter the criterion was written to defend against.
- **Fix:** reported two ways, both yielding 5. `rtk proxy cargo clippy --all-targets 2>&1 | grep -E '^warning: '` lists the five lints (plus a `generated 5 warnings` summary line, which is why the naive `grep -c` there returns 6); the `-D warnings` form's own tail reads `error: could not compile … due to 5 previous errors`. Locations are unchanged: `browser.rs:131/132/133`, `project_creator.rs:146`, `state_reader/mod.rs:258`. No production code changed.
- **Files modified:** none
- **Committed in:** n/a (criterion interpretation)

**3. [Process] `grep -A 1 'SAFE-06 |'` reads SAFE-07's row, which is correctly Pending**

- **Found during:** Task 2, checking acceptance criteria
- **Issue:** The criterion expects `grep -A 1 'SAFE-06 |' .planning/REQUIREMENTS.md | grep -c 'Pending'` to return 0. The `-A 1` drags in the next table row, which is `| SAFE-07 | Phase 21: LLM Goal Layer & Prompt-Injection Hardening | Pending |` — correctly Pending, since SAFE-07 belongs to Phase 21. The criterion therefore returns 1 after a correct edit and could only return 0 if a requirement belonging to another phase were wrongly marked.
- **Fix:** reported anchored to SAFE-06's own row — `grep 'SAFE-06 |' .planning/REQUIREMENTS.md | grep -c 'Pending'` → **0**, and the row reads `| SAFE-06 | Phase 19: GITSAFE — Git & Blast-Radius Envelope | Complete |`. The criterion's intent holds exactly.
- **Files modified:** none
- **Committed in:** n/a (criterion interpretation)

**4. [Process] `grep -c 'Plans**: 8/8 plans executed'` already returned 1 before this plan**

- **Found during:** Task 2, checking acceptance criteria
- **Issue:** The criterion expects exactly 1. Phase 15's section has carried `**Plans**: 8/8 plans executed` since it shipped (ROADMAP line 160), so the unanchored grep returned 1 *before* any edit here and would return 2 after the requested one. It measures "some phase has eight plans", not "Phase 19 has eight executed".
- **Fix:** reported anchored to the Phase 19 section (ROADMAP line 364), whose current text is `**Plans**: 7/8 plans executed`. The requested replacement is quoted verbatim at the end of this document. No file was edited.
- **Files modified:** none
- **Committed in:** n/a (criterion interpretation)

**5. [Design] `BLOCKING_HELPERS` widens D-29's marker set; the allowlist is `(file, marker)` rather than a file path**

- **Found during:** Task 1
- **Issue:** The plan's action text specifies the syscall marker set (`output`/`status`/`spawn`, the advisory lock, blocking `std::fs`) and an allowlist that is "a const array of file paths". Measured against the tree as 19-07 left it, that combination produces **zero** hits and **zero** allowlist entries across 1697 lines of production `async fn` body — because every blocking call the driver makes is behind a helper. It would also have contradicted 19-07's own handoff note, which predicted three specific entries.
- **Fix:** a second marker family naming thirteen of this repository's synchronous seams, and `(file, marker)` allowlist entries so an exemption does not blind a whole file. Both choices are documented at the constants with the measurement that produced them.
- **Files modified:** `tests/async_blocking_guard.rs`
- **Verification:** four justified entries, each confirmed live by `no_allowlist_entry_is_stale`; the fail-first plant is reported in a file that already carries an entry for a different marker
- **Committed in:** `4b33f24`

**6. [Rule 2 - Missing Critical] Literals are blanked before braces are counted**

- **Found during:** Task 1
- **Issue:** The scanner's brace depth is what tells it whether a line is inside an `async fn` body. A `'{'` character literal or an unbalanced brace inside a string makes the depth drift, the tracker leaves the body early, and every blocking call below goes **unreported while the suite stays green** — a false negative, which is the worst failure a guard can have. Measured: `src/driver/run.rs` accumulates a +1 drift from JSON fixtures in its test module, which happens to be skipped, so the trap was live but not yet sprung.
- **Fix:** `strip_literals` blanks raw strings, ordinary strings and character literals (while leaving lifetimes alone, since eating one would swallow the rest of the line) before either brace counting or marker matching.
- **Files modified:** `tests/async_blocking_guard.rs`
- **Verification:** `a_literal_carrying_a_brace_does_not_derail_the_brace_tracker`, which asserts the unit behaviour and then plants a blocking read below two brace-carrying literals and requires it to be found
- **Committed in:** `4b33f24`

**7. [Rule 1 - Bug] The first tracker exited every `async fn` with a multi-line signature on its own declaration line**

- **Found during:** Task 1
- **Issue:** Treating the `async fn` line as the body's start makes `depth <= base` true immediately, so the body is left on the line it was entered. Every function whose parameters do not fit on one line — which is most of the ones in `src/driver/run.rs` — was exempt, silently. Found by instrumenting the tracker rather than by reading it.
- **Fix:** an explicit `Position::Signature` state that waits for the body's opening brace before becoming `Position::Body`. The reason is recorded on the variant.
- **Files modified:** `tests/async_blocking_guard.rs`
- **Verification:** `the_scanner_reports_a_planted_blocking_call_and_spares_a_handed_off_one` opens with a three-line signature for exactly this reason; `REQUIRED_ASYNC_FILES` additionally fails the audit if no `async fn` body is found in `src/driver/mod.rs` or `src/driver/run.rs`
- **Committed in:** `4b33f24`

---

**Total deviations:** 7 (2 missing-critical, 1 bug, 1 design, 3 process)
**Impact on plan:** No scope creep. Deviations 1, 6 and 7 each close a way this plan could have shipped a guard that passed its own acceptance criteria while reporting green over real violations — the exact "guardrail-as-prompt-text" failure in mechanical form. Deviation 5 is the one genuine design fork and is documented at the code. Deviations 2, 3 and 4 changed no files; each reports a criterion anchored so it measures what it was written to measure.

## Issues Encountered

None outside the deviations above. Every one of deviations 1, 6 and 7 was found by instrumenting the scanner against the real tree before committing it, rather than by trusting that it worked — which is the only way a source-scanning guard's false negatives ever surface.

## Pre-existing Failures (out of scope, and honestly reported)

- **`tests/driver_reattach.rs`** — the two tracked intermittent failures did **not** reproduce on this plan's gate run (993 passing, 0 failed). That is luck, not a fix, and the gate is therefore reported on the bounded-failure-set rule the orchestrator adopted at wave 4 rather than as an unqualified clean pass: **no new failures beyond the known ones, and the passing count advanced from 986 to 993.** The flake is proved to predate Phase 19 entirely — 4/4 red at `0a84023`, the commit before the phase began, with no Phase 19 code present. Tracked at `.planning/todos/pending/2026-08-18-driver-reattach-spawn-artifact-race.md`. Nothing in 19-08 touches `src/`.
- **`tests/envelope_tracer.rs#a_relocated_copy_of_the_stub_refuses_instead_of_acting`** — 19-07's one-off `ExecutableFileBusy` did not reproduce here either (the file passed 6/6 on every run). Logged in `deferred-items.md` with a fix direction; not touched.

Neither was investigated or fixed, per the scope boundary. This plan changed one test file and one planning file; `git diff aec4e4b..HEAD` touches no `src/` path at all.

## Known Stubs

None. Every symbol this plan introduces is exercised by a passing test, and the guard's own control arms exercise the scanner against synthetic sources so the matcher keeps being proved even once the tree is correct.

Two things that look like gaps and are not:

- **Ten of the thirteen `BLOCKING_HELPERS` markers currently match nothing inside an `async fn`.** That is the intended state: they are tripwires for a future call, not an inventory of today's violations. `every_named_blocking_helper_still_exists_in_the_tree` keeps them from going stale by requiring each to appear somewhere under `src/`, so a rename fails loudly instead of silently emptying a marker.
- **`TEST_MODULE_EXCLUSION` is a `&str` constant read only by assertion messages.** It exists so the decision has a name a failure message can cite, in the same register `spawn_seam_guard.rs` uses for `GATE_LOOKBACK`.

## Threat Flags

None new. No network endpoint, no auth path, no schema change, and **no crate was added to `Cargo.toml`** — T-19-SC now holds across all eight plans of the phase.

Against this plan's own register:

| Threat | Disposition | Where it is closed |
|---|---|---|
| T-19-51 a vacuous gate over filtered output | mitigated, and it fired | the clippy delta is measured through `rtk proxy`; the criterion's own `grep '^warning:'` under `-D warnings` was found to return 0 vacuously and is reported both anchored ways instead (deviation 2) |
| T-19-52 a guard that cannot fail | mitigated | proved fail-first by a real plant in `driver::drive`, observed red with the offending line named, then reverted — plus four synthetic control arms so the matcher stays proved once the tree is correct |
| T-19-53 completion claimed on a green build | mitigated | every requirement marked against a named test run individually; the clause table records which test proves which clause |
| T-19-59 half a criterion ticked on the other half's test | mitigated | five criteria quoted verbatim and split into 24 clause rows; the park clauses of criteria 2 and 5 each name an on-disk `Parked` event, and three composed clauses are marked `[C]` with the composition spelled out |
| T-19-54 marker set weakened to clear a hit | mitigated | no marker was removed; every hit is resolved by a justified allowlist entry, and the failure message names weakening the marker set as the response that is **not** correct |
| T-19-55 guard false positives from substring matching | mitigated | word-boundary matching with both control arms, plus literal blanking so a marker's letters inside a string are not a call |
| T-19-SC package-manager installs | mitigated | `Cargo.toml` untouched, phase-wide |

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Phase 19 is closed on its own terms.** All five SAFE requirements are marked complete against named passing tests; every clause of every success criterion carries a row; three clauses are recorded as composed rather than end-to-end; eight items are recorded as **not** proved. Phase 20 should read "What is NOT proved" before treating any of this as a guarantee.
- **The park vocabulary is stable and complete.** `envelope::park`/`park_at` is the single appender, `ParkReason::as_str` the single taxonomy, and the terminal `run.json` carries `parked:<reason>`. A router can branch on the run's outcome without opening the journal and without re-deriving anything.
- **`tests/async_blocking_guard.rs` is a live constraint on Phase 20, not a Phase 19 artifact.** A router that reads project state on the driver's async path will trip it unless the read goes through `spawn_blocking`. If Phase 20 adds a synchronous seam of its own, add it to `BLOCKING_HELPERS` in the same commit — otherwise the guard silently checks one thing less than it appears to.
- **The one item most needing a human is 19-06's honesty statement.** It is a pinned constant whose parts are test-asserted and whose *tone* is not. A reader confirming it reads as candour rather than as a hedge is the last unexercised gate in this phase.
- **`STATE.md` and `ROADMAP.md` were NOT touched** — parallel worktree mode; the orchestrator owns those writes. The ROADMAP change this plan would have made is quoted verbatim at the end of this document.
- **No blockers.**

## Self-Check: PASSED

- Created file present on disk: `tests/async_blocking_guard.rs` (917 lines, ≥ 120 required). Modified file present: `.planning/REQUIREMENTS.md`. `git diff --stat aec4e4b..HEAD` shows exactly these two files, 927 insertions, 10 deletions.
- Both task commits present in `git log`: `4b33f24`, `99f7a7f`.
- Every task `<acceptance_criteria>` re-run and passing, with three reported anchored (deviations 2, 3, 4) and one — the ROADMAP edit — deliberately deferred to the orchestrator per the parallel-execution contract.
- The plan-level `<verification>` re-run and passing: the gate is green, the pre-existing lint count is unchanged at five measured through the proxy form, and every clause of every success criterion maps to a named test that passes individually and sits inside D-35's fence.
- `must_haves.artifacts` confirmed: `tests/async_blocking_guard.rs` is 917 lines and contains `ASYNC_BLOCKING_ALLOWLIST`; `.planning/REQUIREMENTS.md` contains `SAFE-06` marked Complete.
- `must_haves.key_links` confirmed: `tests/async_blocking_guard.rs` reaches `src/envelope/` through the `CARGO_MANIFEST_DIR`-rooted walk and through seven envelope seams named in `BLOCKING_HELPERS`, with the link spelled out in that const's doc so `grep -r envelope tests/async_blocking_guard.rs` finds this end of it.
- `STATE.md` and `ROADMAP.md` deliberately untouched.

---

## Requested ROADMAP edit

`.planning/ROADMAP.md` is excluded from this worktree's commit, so the two edits below are **not** applied here. The orchestrator should apply them verbatim on the main checkout after the merge.

### Edit 1 — the plan count (ROADMAP line 364, inside `### Phase 19: GITSAFE — Git & Blast-Radius Envelope`)

Current text:

```
**Plans**: 7/8 plans executed
```

Replacement text:

```
**Plans**: 8/8 plans executed
```

### Edit 2 — the wave 8 checkbox (ROADMAP line 397)

Current text:

```
- [ ] 19-08-PLAN.md — `tests/async_blocking_guard.rs`, the project gate with the unchanged 5-lint delta, and criterion-by-criterion traceability
```

Replacement text:

```
- [x] 19-08-PLAN.md — `tests/async_blocking_guard.rs`, the project gate with the unchanged 5-lint delta, and criterion-by-criterion traceability
```

After both edits, `grep -c '19-0[1-8]-PLAN.md' .planning/ROADMAP.md` returns 8 with all eight lines ticked (7 are already `[x]`). Note that `grep -c 'Plans\*\*: 8/8 plans executed'` will then return **2**, not 1: Phase 15's section has carried that exact string since it shipped, so the criterion is unanchored — see deviation 4.

### Optional, and deliberately left to the orchestrator — the phase-level checkbox (ROADMAP line 93)

Phases 14-18 are ticked in the milestone phase list, and Phase 19's line still reads:

```
- [ ] **Phase 19: GITSAFE — Git & Blast-Radius Envelope** - Mechanically enforced push boundary
```

This plan's Task 2 scopes its ROADMAP edit to the plan checkboxes and the plan count, so ticking the phase itself is **not** requested here. It is surfaced rather than silently done because `/gsd-verify-work 19` has not run, and eight items in "What is NOT proved" above are exactly the sort of thing a verification pass exists to weigh. The orchestrator's call.

**Nothing else in the Phase 19 section should change.** The goal line, the five success criteria and the four phase-risk bullets were authored before planning and are the thing this phase is measured against; this document quotes them verbatim rather than editing them.

---
*Phase: 19-gitsafe-git-blast-radius-envelope*
*Completed: 2026-08-18*
