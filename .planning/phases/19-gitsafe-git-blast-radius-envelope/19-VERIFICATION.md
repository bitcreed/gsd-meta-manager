---
phase: 19-gitsafe-git-blast-radius-envelope
verified: 2026-08-18T23:27:41Z
status: human_needed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Read `SECTION_ENVELOPE` (src/envelope/advisory.rs:192-214, rendered by the dry-run preview and the run journal) and confirm it reads as an honest account of the safety ceiling rather than as a hedge or pre-excuse."
    expected: "The statement should plainly say what IS mechanically guaranteed (no ambient credentials/SSH agent reachable, pre-push hook sees ground truth, PR cap enforced from an out-of-repo ledger), what is NOT (client-side hooks/tool denies/env-injected config are all defeatable by an agent that spawns an unsupervised shell and chooses to; the PR cap specifically has no git-hook second carrier and degrades to unenforced if the settings file is ignored), and conclude with the server-side branch-protection recommendation as the phase's honest conclusion rather than a footnote."
    why_human: "19-06 coverage D5 and 19-08 coverage D2 both record this as `human_judgment: true` — a test can only assert the three required parts are present and ordered; whether the prose reads as candid rather than as marketing is a reading judgement, and the phase's own transparency prohibitions verify this class of claim by judgment, not by grep."
  - test: "Confirm the composed-proof judgement calls in 19-08's traceability table are adequate readings of the ROADMAP criteria as literally written — in particular criterion 2's 'and the attempt parks the run' clause (proved once per park reason, not once per force-push spelling) and criterion 5's 'instead of opening another PR' clause (proved at the PreToolUse guard's deny, never by observing a forge, per D-35's fence)."
    expected: "Each composed clause ([C] rows in 19-08-SUMMARY.md's traceability table for criteria 1, 2, 4 and 5) should read as a faithful decomposition of the criterion sentence, not as a substitution of a weaker claim for the one actually written."
    why_human: "19-08 coverage D6 records this as `human_judgment: true` explicitly: 'whether a composed proof is an adequate proof of the sentence as written is a reader's judgement.'"
  - test: "Confirm the residual-exposure paragraphs in `src/envelope/mod.rs`, `src/envelope/cred.rs` and `src/envelope/scan.rs` (the unset-GIT_CONFIG_COUNT escape, the askpass-token-readable-by-the-agent-inside-the-run admission, and the gitleaks Blocked/Failed arms being unexercised on this machine) read as honest disclosures rather than as rationalizations."
    expected: "Each doc comment states the residual gap plainly, in the same paragraph as the mechanism it qualifies, without softening language."
    why_human: "19-01 D9, 19-02 D8, 19-03's pre-commit-hook rationale, 19-04 D11, 19-05 D11 and 19-07's D25 items are each recorded `human_judgment: true` for the same reason — this is a systemic, deliberate pattern across the phase, not an isolated item, and it is the exact axis PITFALLS names as most dangerous to get wrong (an overstated safety claim is worse than a stated limitation because it gets trusted)."
  - test: "Re-run the full workspace suite 2-3 times under load (`rtk proxy cargo test`) and watch `tests/driver_lock.rs::the_lock_is_released_when_the_holding_process_dies`."
    expected: "Decide whether the observed one-off failure (`the child driver never took the lock within 30s`) is acceptable background flakiness or needs a fix/deferred-items.md entry before Phase 20 builds on this envelope."
    why_human: "Not previously documented in `deferred-items.md` or in the orchestrator's pre-cleared list (only `driver_reattach.rs` and `envelope_tracer.rs`'s `ExecutableFileBusy` case were pre-cleared). Reproduced once in this verification pass under full-parallel load; passed cleanly in isolation and in a second isolated run. Plausible mechanism: 19-07's `establish_envelope()` now runs synchronously (hook stub writes, settings-file generation + round-trip readback, `.git/info/exclude` write, cred config generation) inside one `spawn_blocking`, *before* `lock::acquire` — this is new I/O on the run-startup critical path that did not exist before Phase 19, and under heavy contention it can plausibly eat into the test's 30s budget. This is a test-timing observation, not a mechanism defect: no SAFE-0x success criterion depends on lock-acquisition latency, and the fixture's own project root carries no `.git`, so `probe_protection` short-circuits immediately without a network call in this specific test."
---

# Phase 19: GITSAFE — Git & Blast-Radius Envelope Verification Report

**Phase Goal:** Autonomous git operations are bounded by mechanisms the agent cannot argue its way past
**Verified:** 2026-08-18T23:27:41Z
**Status:** human_needed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A driven run configured to push to `main` is rejected by the envelope, with the model's cooperation removed from the equation | ✓ VERIFIED | `tests/envelope_tracer.rs::a_driven_push_to_main_is_refused_and_the_remote_ref_never_appears` — reproduced directly: `cargo test --test envelope_tracer` → 6/6 pass. Drives a real `git push` against a `file://` bare remote under the envelope's environment with no agent spawned (D-31). Paired allow test (`a_driven_push_inside_the_reserved_namespace_reaches_the_remote`) confirms the envelope is a boundary, not a wall. `classify_push_ref` at `src/envelope/policy.rs:125` is the enforcing pure function; `DEFAULT_NAMESPACE_ROOT = "refs/heads/gsd-auto/"` at `policy.rs:23`. |
| 2 | `git push --force`, `+refs/…`, `--no-verify`, and `core.hooksPath` rewrites from a driven run are all blocked, and the attempt parks the run | ✓ VERIFIED | `classify_git` at `src/envelope/policy.rs:257` — 47 passing unit tests in `envelope::policy` cover every spelling (`-f`, `--force`, `--force-with-lease[=v]`, `--force-if-includes`, `--mirror`, `--delete`/`-d`, `+refspec`, `--no-verify`, every `git config` scope touching `core.hooksPath`, and the `-c core.hooksPath=` form). End-to-end park proof reproduced directly: `cargo test --test envelope_wiring` → 14/14 pass, including `a_force_push_is_refused_and_lands_a_force_push_blocked_park` and `a_hooks_path_rewrite_is_refused_and_lands_a_hook_bypass_blocked_park`, both asserting an on-disk `JournalEvent::Parked` read back by a separate process. `core.hooksPath` delivered via `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` (confirmed present in `src/envelope/cred.rs`), never by mutating `.git/config` or `.git/hooks/`. |
| 3 | A push carrying a detectable secret is blocked before it leaves the machine, including a secret written to a gitignored path | ✓ VERIFIED | `src/envelope/scan.rs` hand-rolled walk explicitly does not consult `.gitignore` (confirmed by reading the file and its test `the_git_directory_is_not_walked` plus the gitignored-path fixture). Reproduced directly: `cargo test --test envelope_hook_refusals` → 7/7 pass, including `a_credential_on_a_path_the_ignore_rules_cover_still_blocks_the_push`, which first asserts via `git check-ignore` that the planted `secrets/prod.pem` really is ignored, then asserts the push refusal and an unchanged remote. Scanner reuses `journal::redact`'s `PARTS` table split by `SecretClass::{Credential, PathHygiene}` — only `Credential` blocks. |
| 4 | A driven run pushes using a per-run scoped credential and still works with the user's ambient credentials and SSH agent unavailable to it | ✓ VERIFIED | `src/envelope/cred.rs` confirmed removes `SSH_AUTH_SOCK`/`SSH_AGENT_PID` (not overwrites), sets `GIT_SSH_COMMAND`, redirects `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` to a generated config with no `credential.helper`, sets `GIT_TERMINAL_PROMPT=0`, redirects `GH_CONFIG_DIR`, and delivers the token only through `GIT_ASKPASS`. Reproduced directly: `cargo test --test envelope_credential` → 6/6 pass, including `a_credential_helper_the_user_really_has_stops_being_resolvable_under_the_envelope` (a planted control proving plain git *would* resolve it and the envelope doesn't) and `a_push_inside_the_reserved_namespace_succeeds_with_home_emptied` (exit 0, ref present, `HOME` pointed at an empty directory). |
| 5 | Exceeding the per-project 24-hour PR cap parks the run instead of opening another PR | ✓ VERIFIED | `src/envelope/ledger.rs` append-only NDJSON ledger outside the repo. Reproduced directly: `cargo test --test envelope_pr_cap` → 11/11 pass, including `the_third_pull_request_in_the_window_succeeds_and_the_fourth_is_refused`, the rolling-window boundary tests, and the per-run cap test. `cargo test --test envelope_wiring` confirms `a_pull_request_beyond_the_cap_is_refused_and_lands_a_pr_cap_exceeded_park` — an on-disk `Parked` event carrying `pr_cap_exceeded`. **Caveat, honestly disclosed by the phase itself (not discovered by this verifier):** the cap has only one enforcement layer (the `PreToolUse` settings-file guard) — no git hook observes `gh pr create` because it is not a git operation, so a settings file the agent's own CLI silently ignores (the documented `-p` mode risk) leaves this specific criterion's cap unenforced while every push boundary stays standing. This is stated in `SECTION_ENVELOPE` and in 19-08-SUMMARY.md's "What is NOT proved" item 3, not hidden. |

**Score:** 5/5 truths verified (0 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/envelope/mod.rs` | Module root, D-06 three-layer doc contract, envelope dir resolver | ✓ VERIFIED | Present, substantive, imported and used across the crate |
| `src/envelope/policy.rs` | Pure `classify_git`/`classify_push_ref` decision functions | ✓ VERIFIED | 47 passing unit tests; genuine logic, not a stub |
| `src/envelope/hooks.rs` | Hook stub generation + provenance check | ✓ VERIFIED | 33 passing lib tests; generated stub re-enters the binary, no policy logic in the stub (confirmed by `both_hooks_are_generated_and_neither_carries_policy_logic`) |
| `src/envelope/scan.rs` | Full-worktree secret scanner, reported skip list | ✓ VERIFIED | Confirmed does not consult `.gitignore`; skip-list reporting present |
| `src/envelope/cred.rs` | Scoped-credential env builder, askpass responder | ✓ VERIFIED | 20 passing lib tests; all D-16/D-17 environment variables confirmed present in source |
| `src/envelope/ledger.rs` | Append-only PR-cap ledger | ✓ VERIFIED | 16 passing lib tests; `record_and_check` present |
| `src/envelope/advisory.rs` | Read-only protection probe, pinned honesty statement | ✓ VERIFIED | `SECTION_ENVELOPE` present, wired into both the dry-run preview (`src/driver/dry_run.rs:67,235`) and the run journal (`src/driver/run.rs:1338`) — genuinely reachable from production code paths, not test-only |
| `tests/envelope_tracer.rs` | Namespace refusal/allow fixture | ✓ VERIFIED | 6/6 passing |
| `tests/envelope_hook_refusals.rs` | Secret-scan + worktree-sweep fixtures | ✓ VERIFIED | 7/7 passing |
| `tests/envelope_credential.rs` | Scoped-credential environment fixtures | ✓ VERIFIED | 6/6 passing |
| `tests/envelope_pr_cap.rs` | PR-cap ledger fixtures | ✓ VERIFIED | 11/11 passing |
| `tests/envelope_wiring.rs` | End-to-end refusal→park fixtures | ✓ VERIFIED | 14/14 passing (each `Parked` row also re-run individually, per plan requirement) |
| `tests/envelope_advisory.rs` | Protection-probe fixtures | ✓ VERIFIED | 9/9 passing |
| `tests/async_blocking_guard.rs` | D-29 blocking-call-inside-async lint | ✓ VERIFIED | 917 lines (≥120 required), 7/7 passing, fail-first proof independently plausible from source (planted call reported by name) |
| `.planning/REQUIREMENTS.md` | SAFE-01/02/03/05/06 marked Complete | ✓ VERIFIED | Confirmed directly: all five `[x]` in the checklist and "Complete" in the traceability table |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `src/driver/run.rs` (`establish_envelope`, single `ExecutionOptions` site) | `src/envelope/*` | `envelope::hooks::install`, `cred::build_env`, `advisory::probe_protection` inside one `spawn_blocking` at the single production construction site | ✓ WIRED | Confirmed at `src/driver/run.rs:110-127,1225-1226`; runs before `lock::acquire` (line 1295) and before the journal is created, so a refused envelope creates nothing (matches D-24) |
| `src/executor/claude.rs` (spawn closure) | envelope environment | `envelope_disallowed_tools` rendered onto `--disallowedTools` argv | ✓ WIRED | Confirmed at `src/executor/claude.rs:286-288`; pinned argv test present |
| `src/driver/mod.rs::drive` (dry-run branch) | `src/driver/dry_run.rs` | `dry_run::build_report` → `dry_run::render` → `println!` | ✓ WIRED | Confirmed at `src/driver/mod.rs:241,261` — real CLI output path, not test-only |
| `src/journal/mod.rs::JournalEvent::Parked` | envelope refusal points | `park`/`park_at` appender, `ParkReason::as_str` taxonomy | ✓ WIRED | Confirmed `parked` moved from `RESERVED_KINDS` into `EMITTED_KINDS` at `src/journal/mod.rs:921-934`, matching the D-24 comment correction |

### Behavioral Spot-Checks / Independent Reproduction

All checks below were run directly by this verifier, not taken from SUMMARY.md claims.

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Clean build | `rtk proxy cargo build` | exit 0 | ✓ PASS |
| Lint gate | `rtk proxy cargo clippy -- -D warnings` | exit 0 | ✓ PASS |
| All-targets lint delta | `rtk proxy cargo clippy --all-targets -- -D warnings` | exactly 5 errors, at `browser.rs:131/132/133`, `project_creator.rs:146`, `state_reader/mod.rs:258` — matches claimed unchanged locations | ✓ PASS |
| `envelope_wiring` suite | `cargo test --test envelope_wiring -- --test-threads=1` | 14 passed, 0 failed | ✓ PASS |
| `envelope_tracer`, `envelope_hook_refusals`, `envelope_credential`, `envelope_pr_cap`, `envelope_advisory`, `async_blocking_guard` | `cargo test --test <name>` | 6, 7, 6, 11, 9, 7 passed respectively, 0 failed | ✓ PASS |
| Full workspace suite (run once) | `rtk proxy cargo test` | Stopped early on one failure — see below | ⚠️ see note |

**Full-suite note:** the first full run hit one failure, `tests/driver_lock.rs::the_lock_is_released_when_the_holding_process_dies` ("the child driver never took the lock within 30s"). Re-run of that file alone (`cargo test --test driver_lock -- --test-threads=1`) passed 5/5 cleanly. This is a timing failure under full-parallel contention, not a deterministic one, and it is **not** one of the two pre-cleared `driver_reattach.rs` flakes or the `envelope_tracer.rs` `ExecutableFileBusy` case documented in `deferred-items.md`. Traced to a plausible new cause: 19-07 wired `establish_envelope()` (hook stub writes, settings-file generation + round-trip read-back, `.git/info/exclude` write, cred config generation) to run synchronously inside one `spawn_blocking`, before `lock::acquire` — new I/O on the run-startup critical path that did not exist pre-Phase-19. Routed to human verification below rather than treated as a gap, because no SAFE-0x criterion depends on lock-acquisition latency and the mechanism logic itself is unaffected.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| SAFE-01 | 19-01, 19-06, 19-07 | Reserved push namespace, model's cooperation removed | ✓ SATISFIED | `classify_push_ref`, `envelope_tracer.rs`, `SECTION_ENVELOPE` |
| SAFE-02 | 19-02, 19-03, 19-07 | Force-push/hook-bypass blocked | ✓ SATISFIED | `classify_git`, `envelope_wiring.rs` park fixtures |
| SAFE-03 | 19-03 | Secret scan over full worktree, gitignore-blind | ✓ SATISFIED | `scan.rs`, `envelope_hook_refusals.rs` |
| SAFE-05 | 19-04 | Scoped credential, ambient credentials unreachable | ✓ SATISFIED | `cred.rs`, `envelope_credential.rs` |
| SAFE-06 | 19-05 | PR cap, rolling 24h window | ✓ SATISFIED (single-layer caveat disclosed) | `ledger.rs`, `envelope_pr_cap.rs` |

No orphaned requirements: SAFE-04 belongs to Phase 16 (already complete), SAFE-07/08 belong to Phase 21 and are correctly `Pending`.

### Anti-Patterns Found

None. `grep -rn -E "TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER"` over `src/envelope/*.rs`, `tests/envelope*.rs` and `tests/async_blocking_guard.rs` returns no matches. No debt markers.

### Deviations / Honesty Disclosures Cross-Checked

The phase's own SUMMARY documents (principally 19-08-SUMMARY.md's "What is NOT proved" section) disclose eight residual gaps. This verifier confirmed each is genuinely disclosed (not silently absorbed into a passing claim) and none is misrepresented:

1. Honesty statement's tone — not test-provable, routed to human verification above.
2. Protection probe's 10s-budget stall path — no network fixture exists (D-35 forbids one); reasoned, not observed. Confirmed the code path exists (`PROBE_BUDGET_SECS = 10`, kill-at-deadline loop in `advisory.rs`).
3. PR cap has no git-hook second carrier — confirmed by reading `policy::disallowed_tools()` (only covers `Bash(git push:*)` and `.claude/**` writes, not `gh pr create`) — genuinely a single-layer control, honestly stated in `SECTION_ENVELOPE` itself.
4. Askpass token readable by an agent inside the run — confirmed as an inherent, stated limitation (`D-17`'s own doc).
5. No test reads the driven child's actual environment via `/proc` — confirmed absent; reasoned from `Command`'s contract instead.
6. `gitleaks` Blocked/Failed arms unexercised (binary not installed) — confirmed absent from `$PATH` in this environment; Absent arm (must-never-fail-open) is exercised on every run.
7. The async-blocking guard is a lint, not a proof — its own doc says so (`grep -c 'a lint, not a proof'` → 1, confirmed).
8. SAFE-01 is true of the model's cooperation being removed, not of "no escape exists" — an agent that unsets `GIT_CONFIG_COUNT` in a subshell is past the last client-side layer, stated in `SECTION_ENVELOPE` and in the module docs.

None of these were found to be overstated or hidden. This matches the phase's stated governing principle (D-27: "an overstated safety claim is worse than a stated limitation").

### Human Verification Required

See frontmatter `human_verification`. Summary:

1. **Honesty statement tone** (`SECTION_ENVELOPE`) — reads as candid to this verifier on direct reading, but the phase's own transparency prohibitions require human judgment, not an automated verifier's read, to close this item.
2. **Composed-proof adequacy** — whether the `[C]`-marked clauses in 19-08's traceability table are faithful decompositions of the ROADMAP criteria as literally written.
3. **Residual-exposure disclosures** (unset-`GIT_CONFIG_COUNT` escape, askpass-token-readable-by-agent, gitleaks-arms-unexercised) — same class of judgment, recorded across nearly every plan in the phase.
4. **`tests/driver_lock.rs` new intermittent failure** — a previously undocumented flake this verifier observed once under full-suite load; needs a human decision on whether to accept as background flakiness (à la the two already-documented ones) or open a follow-up.

### Gaps Summary

No must-have truth failed, no artifact is missing or stubbed, and no key link is unwired. All five ROADMAP success criteria have genuine, independently-reproduced enforcing code and passing named tests, not merely SUMMARY claims. Status is `human_needed` rather than `passed` solely because of (a) the large number of `human_judgment: true` honesty/tone items the phase's own plans explicitly deferred to a human reader — most centrally whether the pinned honesty statement reads as candor rather than hedge — and (b) one previously undocumented intermittent test failure (`driver_lock.rs`) observed during this verification pass that was not part of the orchestrator's pre-cleared flake list and plausibly traces to new I/O this phase added to the run-startup critical path.

---

_Verified: 2026-08-18T23:27:41Z_
_Verifier: Claude (gsd-verifier)_
