# Phase 31: Retire Superseded Envelope & Goal-Layer Code - Research

**Researched:** 2026-10-24
**Domain:** Rust deletion/refactor of a 220k-line crate (envelope guard, legacy `drive`, goal layer, injection), threat-register closure, release tooling
**Confidence:** HIGH for the code inventory (compiler trial + source reads); MEDIUM for expected test counts (estimated, not measured post-deletion)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

### Musk pass (discuss:pre)
- No scope cuts. RETIRE-01..03 trace to the 2026-10-14 re-plan and to the hand-overs in Phase 28 D-01
  and Phase 30 D-01/D-02. Phase 31 is the only owner of the open Phase 19/21 threats. Lean lane not
  taken: the phase spans the envelope, driver, executor, journal, TUI, CLI, tests, scripts and
  planning docs, which is well over 3 tasks. `musk milestone-check` reported `run: false`.
  `musk pass --phase 31` is recorded.

### What survives Phase 28.1 (the user's carry-in)
- **D-01:** Phase 28.1 counts as landed for this phase. It verified 7/7 automated, and only the
  human-run D-11 live probe is pending. Nothing that `src/launcher/**`, `src/driverctl/**`, egress,
  `home.rs` or `config_dir.rs` (`DENY_TOOLS`) imports is deleted or changed in behaviour.

  The code inventory found these live imports from the envelope, which must keep compiling with
  unchanged semantics:
  - `launcher/push.rs:52-58` uses `ledger::{record_and_check, CapVerdict, LedgerEntry}`, the
    `policy` namespace items (`DEFAULT_NAMESPACE_ROOT`, `validate_namespace`, `classify_push_ref`,
    `PushVerdict`, the `REASON_*` constants), `EnvelopePolicy` and the PR caps, and
    `scan::{scan_with_external, ScanLimits}`;
  - `launcher/run.rs:101` uses `EnvelopePolicy`;
  - `launcher/outcome.rs` uses `driver::rate_limit`.

  `[inferred — audit]`
- **D-02:** Open Phase 19 threats that rely on network or `$HOME` confinement are re-pointed at
  Phase 28.1. These are exfiltration, credential use and network reach: credential helpers, the
  envelope dir and ledger as files the agent can reach, and the gitconfig carriers. Their pointers
  name the 28.1 mechanism:
  - LNCH-09: `--unshare-net` plus the egress/CONNECT proxy and the token-injecting reverse proxy
    (28.1 D-02..D-06);
  - LNCH-10: the `$HOME` allowlist;
  - LNCH-12: the token stays outside the sandbox.

  They close now on the strength of 28.1's automated verification. Each such row carries a
  conditional: "reopens against Phase 28.1 if the D-11 live probe fails". `[inferred — audit]`

### Scope of deletion (RETIRE-01)
- **D-03: The legacy Phase 17 `drive` path is deleted together with what only it uses.** This covers:
  - the hidden `drive` subcommand (`cli.rs:156`, `main.rs:377`);
  - `driver::spawn`, `driver/run.rs`, `dry_run.rs` and the `drive`-only parts of `bounds.rs`;
  - the TUI legacy start/stop (`app.rs` `start_driver_run`/`stop_driver_run`, the dashboard r/x/o
    keys in `normal.rs:768-796`, `driver_start.rs`, `driver_confirm.rs`);
  - the "legacy runs" read-only section that Phase 30 D-01 added.

  Rationale: `run.rs` is the main consumer of the Phase 19 guard wiring, the Phase 21 goal layer and
  the inbox injection. Phase 28 D-01 and Phase 30 D-01/D-02 and its Deferred list already gave
  "delete `drive`, inbox injection and journal-run code" to Phase 31. Keeping `drive` would mean
  rewiring a dead path just to delete its safety seams. Journal run dirs already on disk are left
  in place, just no longer shown. `[inferred — audit]` — **Reversibility:** costly. Restoring it
  means reverting a large multi-module deletion. Git keeps everything, but conflicts grow with every
  later commit.
- **D-04: Codex driving goes with `drive`. The parts with other users stay.** Quick 260922-hdj's
  `CodexExecutor` only ever ran under `drive`.
  - Delete the executor spawn and stream paths that only `run.rs` uses (`ExecutionHandle::write_line`,
    `stdin_tx`, the `envelope_env` plumbing at `executor/mod.rs:538`, `claude.rs:483,563`,
    `codex.rs:203,221`).
  - Keep `executor::runtime` (`AgentRuntime`), because `config.rs:89,538` and `registry.rs:2225` use
    it.
  - Keep whatever of `executor::stream_json` the surviving journal/agents code parses with
    (`journal/mod.rs:83`).
  - Keep the session-detector code.
  - The researcher must list exactly which `executor/*` items lose their last caller, and the
    planner deletes only those.

  This is the decision most worth auditing: it removes a shipped (experimental) Codex drive
  capability. It is not escalated because the Phase 30 and Phase 28 hand-overs already put the
  legacy driver on the deletion path, and the new launcher is Claude-only by design. `[inferred —
  audit]` — **Reversibility:** costly. Same as D-03.
- **D-05: Phase 19 guard deletion.** Delete:
  - `envelope/hooks.rs`: the `PreToolUse` guard, the settings writers, the pre-push/pre-commit
    stubs, `install*` and `write_exclude_block`;
  - `envelope/cred.rs` (askpass, gitconfig, `EnvelopeEnv`) and `envelope/advisory.rs`;
  - the hidden `Commands::Envelope` CLI (`cli.rs:398,738-810`, `main.rs:482-620`);
  - the classifier and tokenizer in `policy.rs` (`classify_git`, `GitVerdict`, `resolve_*`,
    `disallowed_tools`, roughly 193-635 and 2365-7094, plus their tests);
  - `CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION` and its witness test
    (`policy.rs:1102,10729`).

  `policy.rs` keeps the namespace items (23-150) and `EnvelopePolicy` plus the cap constants
  (2424-2490). Cut `ledger.rs:442`'s dependency on `hooks::GUARD_TIMEOUT_SECS` by inlining a
  ledger-owned constant before `hooks.rs` goes. `[inferred — audit]`
- **D-06: The kept envelope pieces stay at `src/envelope/`.** That is `mod.rs` (the
  `envelope_root`/`envelope_dir*` helpers), `scan.rs`, `ledger.rs` and the reduced `policy.rs`.
  RETIRE-01's "rewired into the push step" is already true by import (`launcher/push.rs`), so a
  rename or move is churn with no behaviour change. The planner may rename the module (for example
  to `push_safety`) only if the reduced `policy.rs` would otherwise mislead readers. `[inferred —
  audit]`
- **D-07: Delete the Phase 21 goal layer.** That is `driver/goal.rs`, `driver/escalate.rs`,
  `driver/untrusted.rs` (the SAFE-07 boundary) and the goal/approved_plan/max_escalations fields of
  `RawDriveArgs`. Also remove the `GoalRefused`/`EscalationRefused` errors (`error.rs:769-802,
  1131-1140`) and the journal approval-digest path (`journal/mod.rs:3060`), along with their tests.

  The SAFE-08 fixed-action enum (`RouterAction`, `SAFE_COMMAND_ALPHABET`, `router.rs:339,352`)
  belongs to the router and stays. `tests/driver_router_conformance.rs` must keep passing unchanged.
  Phase 21's `Untrusted` display hardening (`text.rs:568`, the `DriverOutputLine`/`EditBuffer` over
  `Untrusted`, and the session-detector resume fixes) is UI/display code, not goal layer, and stays.
  `[inferred — audit]`
- **D-08: Delete stream-json durable injection.** That is:
  - `journal/inbox.rs`;
  - `ui/screens/driver_inject.rs`;
  - `Action::DriverInjectRequested`/`DriverInjectWritten` and their handlers in `app.rs`;
  - `injection_rows`/`injection_block` in `ui/screens/driver.rs`;
  - the injection fixtures in `render_escape_guard.rs`;
  - the `driver_inbox` test and the inbox part of `journal_gitignore`;
  - the transcripts `05-queued-injection-two-turns` and `08-tooluse-queued-two-turns`.

  Phase 30's `driver::steer` (DIRECTION.md `control`) is the steering path and is untouched.
  `[inferred — audit]`
- **D-09: SAFE-04 redaction stays in `journal/redact.rs`, and whatever is left of `journal/` is
  trimmed to its live users.** Live users include `scan.rs`, `main.rs` and `journal/writer.rs`, if
  the writer survives. The researcher must check whether the Phase 28 launcher writes any captured
  child output, such as push stderr or `gh` output, to disk without `journal::redact`. Today it uses
  only `push.rs:238 redact_urls`. If it does, route that capture through `journal::redact`. That is
  the "SAFE-04 rewired into the launcher" half of RETIRE-01. If nothing is captured, record that
  finding rather than adding a call for its own sake. `[inferred — audit]`

### Tests and corpora
- **D-10: Delete the carrier corpora.** That is every `tests/envelope_*.rs` that drives the guard
  or tokenizer: argv_deletion, callee_grammar, carrier_reach, command_position, config_resolution,
  control_carrier, expansion_slots, hook_refusals, interior_path, literal_decision, reparsed_value,
  word_set, wrapper_bypass, wrapper_class, tracer, credential, wiring and advisory.

  Two suites are rewritten or kept against the surviving modules, not deleted, because they guard
  behaviour that remains:
  - `envelope_pr_cap.rs` tests the ledger directly and no longer goes through
    `hooks::write_settings_in`;
  - the namespace, scan and ledger unit tests.

  Delete the legacy driver tests listed by the inventory: `driver_{tracer, iteration_loop, kill,
  kill_startup, lock, optin, reattach, rate_limit, dry_run, codex_runtime}`, `driver_goal_seam`,
  `driver_model_seam`, `driver_injection_corpus` plus `tests/fixtures/injection-corpus/`,
  `driver_escalation_cap`, `driver_refusal_record`, and the goal arms of `spawn_seam_guard`. The
  exceptions are any of these that test `driver::{lock, kill, rate_limit}` as modules the launcher
  still uses. Those are kept or retargeted, never dropped. The researcher classifies each test file
  as delete, keep or retarget. `[inferred — audit]`
- **D-11: The pass-count baseline is measured.** Record `cargo test --no-fail-fast` pass/fail/ignored
  counts before the deletion, and the expected count after it, in the plan. A large drop is
  expected. The check that matters is that every surviving suite still runs: the memory note says
  fail-fast hides the envelope suites, and `rtk` truncates piped output. Use `rtk proxy cargo test
  --no-fail-fast` with no downstream grep for the count. `[inferred — audit]`

### Git-version witness and release tooling (follows from D-05)
- **D-12:** Deleting `CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION` removes the git-version
  witness test. That has these consequences:
  - Delete the git reconciliation banner in `scripts/pre-tag-check.sh` (around line 573).
  - Delete the PPA git pin comment and its rationale in `scripts/pre-tag-check.Dockerfile` (lines
    7-24). The PPA pin itself may be dropped only if nothing else needs that git version. That is
    the researcher's call, with evidence.
  - Update `.github/workflows/release.yml`, `scripts/launcher-acceptance.sh` and
    `scripts/prompt-acceptance.sh` wherever they reference the envelope or the constant.
  - Update the Release Process paragraphs in the project `CLAUDE.md` that describe the witness, the
    banner and its MATCH/MISMATCH reading.
  - The memory note `git-version-constants-test-env-failure.md` becomes obsolete. Report it to the
    coordinator; do not edit memory from the plan.

  `[inferred — audit]` — **Reversibility:** reversible.

### Experimental gate (RETIRE-02's `GSDMM_EXPERIMENTAL_FEATURES` clause)
- **D-13: The flag stays, and still gates the Phase 28/30 launcher surfaces.** Only the gate sites
  whose surface is deleted are removed (`start_driver_run`/`stop_driver_run` and the r/x/o keys).
  Un-gating the launcher is a product decision outside this phase. After the change, every remaining
  `ctx.experimental` read must gate a surface that still exists. Update `README.md`,
  `docs/CONFIGURATION.md`, `help.rs:215` and the `cli.rs:140` comment so they describe only those
  surfaces. `[inferred — audit]`

### Phase 22 archive (RETIRE-02)
- **D-14: The Phase 22 directory stays in place.** Add `status: superseded` and a pointer to
  Phase 28 to the frontmatter of `22-01..04-PLAN.md`, and add a short `22-SUPERSEDED.md` naming
  bwrap + `systemd-run --user --scope` (Phase 28) as the replacement. Moving the directory would break
  ROADMAP links and `gsd-tools` phase lookup. Physical archival belongs to `/gsd-complete-milestone`.
  `bollard` is absent from `Cargo.toml` and `Cargo.lock` today. A cheap test or pre-tag gate that
  fails if `bollard` appears in `Cargo.lock` makes "never added" enforceable. Whether to add it is
  the planner's call. `[inferred — audit]`

### Threat closure (RETIRE-03)
- **D-15: Close every open Phase 19 threat, not just the four the ROADMAP names.** 19-SECURITY.md
  lists 139 total, 27 open and 6 blocking at `high`: T-19-86, T-19-91, T-19-111, T-19-112, T-19-116,
  T-19-121. The ROADMAP's list (86, 91, 111, 116) omits T-19-112 (the PR-ledger file) and T-19-121
  (`git -c credential.helper`). Both are closed here too, because leaving them open would keep
  `threats_open` above 0 after the guard they belong to is gone. Each open id gets one pointer class:
  - **(G) guard-internal:** parser, tokenizer or corpus defects of the deleted guard. Closed as moot,
    because the mechanism is deleted. Superseded by capability removal: no push credentials in the
    sandbox (LNCH-04), and the push leaves only through gsd-mm's fixed-refspec push step (LNCH-06).
  - **(N) network or `$HOME` dependent:** exfiltration, credential use, network reach, and agent
    reach to envelope files. Points at Phase 28.1 LNCH-09/10/12, conditional on D-11 per D-02 of this
    phase. T-19-112 and T-19-121 are expected here: the ledger lives outside the `$HOME` allowlist,
    and no credential helper has anything to reach.
  - **(K) kept-code:** threats against the surviving ledger, scan or namespace code. Re-assessed
    against the push step, which runs outside the sandbox. Closed only if the push-step context
    actually removes the attack. Otherwise it stays open with a new owner and is not falsely marked
    superseded.

  Write the closure as a dated "Phase 31 closure" section in
  `.planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md`, one row per id, as the
  ROADMAP requires. Set 19-SECURITY.md's frontmatter to `status: superseded` with `threats_open` at
  the post-closure count, and add a closure table. Leave the earlier audit text as written; the
  closure goes beside it. `[inferred — audit]`
- **D-16: Phase 21 criterion 4** (injected `.planning/`/`CLAUDE.md` instructions do not change the
  command) is closed as superseded. Its pointer: injection is contained by the sandbox, not
  prevented (LNCH-04 plus the Phase 28.1 egress/`$HOME` controls), and the planning-doc channel is
  documented accepted residual risk (28.1 SC6 / LNCH-14). The closure goes in
  `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md`.

  Any Phase 21 open item that sits in code that survives is not closed as superseded. The known case
  is 21-VERIFICATION's CR-01, the `-r`/`--resume` optional-value arity in `session_detector.rs`. The
  researcher re-checks whether it is still live. If it is, it gets a backlog todo as its owner.
  `[inferred — audit]`
- **D-17:** Mark SAFE-02 and SAFE-05/07/08 in the REQUIREMENTS.md traceability "guard retired in
  Phase 31" (or the matching text), and mark RETIRE-01..03 complete at verification. Do not touch
  the 28.1 rows. `[inferred — audit]`

### Done gate
- **D-18:** SC4's `./scripts/pre-tag-check.sh --container` runs with the tag that would be cut next.
  If none is planned, use the current `Cargo.toml` version as a dry-run tag. This is a dry run that
  never tags or publishes. Gate 6 (`cargo package --locked`) must still pass, because deleted modules
  can strand `include_str!` targets or `[package] include/exclude` entries. Clippy must also stay
  clean: dead-code warnings from half-deleted modules count as failures. `[inferred — audit]`

### Claude's Discretion
- Plan and wave split. Suggested order: (1) baseline counts and the ledger timeout decoupling;
  (2) delete the goal layer and injection; (3) delete `drive`, the executor remnants and the legacy
  tests; (4) delete the guard, CLI, cred, advisory and classifier, with their corpora and the
  scripts/CLAUDE.md witness updates; (5) threat closure docs, Phase 22 archive and gate docs;
  (6) the container gate. Deletion plans touch shared files (`app.rs`, `main.rs`, `cli.rs`,
  `error.rs`, `driver/mod.rs`), so mostly sequential waves are expected. That is the shared-file
  reason; the docs-only plan can run in parallel.
- Whether `ParkReason` variants are pruned to the ones the ledger, scan and push step still
  produce.
- Exact wording of the closure rows.

### Deferred Ideas (OUT OF SCOPE)

- Un-gating `GSDMM_EXPERIMENTAL_FEATURES` for the launcher and Driver tab: a product decision for
  later.
- Renaming `src/envelope/` to a push-step name: optional, see D-06.
- Re-adding Codex driving on the Phase 28 launcher: a future phase if wanted. D-04 removes the
  legacy Codex drive.
- Physically moving `.planning/phases/22-*` into a milestone archive: `/gsd-complete-milestone`.

#### Reviewed Todos (not folded)
`--auto` folds todos scoring ≥ 0.4. These were overridden as out of scope, because the phase
boundary is fixed and folding them would be scope creep `[inferred — audit]`:
- `2026-08-19-verify-work-gate-policy-configurable.md` (0.6): a router policy feature, not a
  retirement. It is probably superseded by Phase 29's DIRECTION.md standing rules, but that is for
  the todo's owner to decide.
- `2026-07-29-driver-tab-layout-at-medium-terminal-heights.md` (0.5): a UI layout bug.
- `2026-10-21-ambiguous-wide-typography-borders-and-darkgray.md` (0.5): a UI rendering bug.
- `2026-09-22-codex-runtime-remainder-after-mvp.md` (0.4): its legacy-driver half becomes moot under
  D-04. Re-scope the todo after Phase 31 lands; do not fold it.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| RETIRE-01 | Remove the Phase 19 argv/bash-classifying `PreToolUse` guard + carrier corpora, the Phase 21 goal-layer seams, and stream-json durable-injection steering; rewire the survivors (namespace refusal, secret scan, PR ledger, SAFE-04 redaction, `Untrusted`, router) into the launcher/push step | §Deletion/Keep Inventory, §Call-graph edges, §Corrections C1-C12, §D-09 finding (push step DOES persist child stderr without `journal::redact`) |
| RETIRE-02 | Phase 22 plans archived as superseded; `bollard` never added; `GSDMM_EXPERIMENTAL_FEATURES` gates only surfaces that still exist | §Experimental gate sites, §Phase 22 archive, §bollard gate example |
| RETIRE-03 | Open Phase 19/21 threat ids + criterion 4 closed as superseded with pointers; `./scripts/pre-tag-check.sh --container` passes on the reduced tree | §Threat closure inventory, §Phase 21 CR-01 re-check, §Release tooling (pre-tag-check HARD-ERRORS if the constant vanishes first) |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **State reading / non-intrusive / portability** (project constraints): nothing here may require running Claude/GSD to read status; deletion must not change launcher, egress or sandbox behaviour (also D-01).
- **Release Process step 3:** the container dry run `./scripts/pre-tag-check.sh --container vX.Y.Z` is the certifying gate; a bare run is "unproven". Gate 6 is `cargo package --locked`. Phase 31 edits the step-3 text itself (D-12).
- **Clippy `-D warnings`** in gate 5 and in CI `cross-target-check` (`cargo clippy --locked --target <apple|windows> --all-targets -- -D warnings`) [VERIFIED: .github/workflows/release.yml:54-98, read this session]. Dead code in a **binary** target (`src/main.rs`, e.g. `judged_alias_or_exit`) and unused imports fail it; `pub` lib items do not warn.
- **Version/tag discipline:** `Cargo.toml` `version = "1.15.0"` [VERIFIED: Cargo.toml:3]. D-18 dry-run tag = `v1.15.0` unless a release is planned.
- **GSD workflow enforcement:** edits only via a GSD command (this research edits no src).
- **User global CLAUDE.md:** test runs via `rtk proxy cargo test --no-fail-fast`; never pipe the count through grep/awk (rtk filters downstream). Upstream is a **public** GitHub repo (memory `upstream-is-public.md`), so pushing to unblock parallel worktrees is NOT pre-authorized — parallel waves must not depend on a push.
- **Memory notes in force:** fail-fast hides envelope suites; the git-version witness fails locally by design (`git version 2.53.0` here vs constant `2.55.0`); router-conformance oracle test needs the oracle (present on this machine); worktree base-check false negative (force `ISOLATION=none` if it returns nulls); never rebase `dev`.

## Summary

The phase is a large, mostly mechanical deletion, but the CONTEXT inventory was written from greps and **twelve of its premises are wrong or incomplete** (see §Corrections). The most consequential: `driver_confirm.rs` is shared with Phase 28/30 (opt-in toggle and launcher-stop confirm), `ui/screens/driver.rs` exports ~14 layout helpers the Phase 30 `driver_run.rs` view imports, `envelope scan` is the evidence gate of both acceptance scripts, transcripts 05/08 are `include_str!` targets of kept modules (`stream_json`, `rate_limit`), `tests/common/mod.rs` (used by eight kept `agents_*` suites) calls `hooks::install_in`, the reconcile scan is the carrier of Phase 30's launcher-marker refresh, `next --json` uses `DriveError::TargetPhaseInvalid`, and `pre-tag-check.sh` **hard-errors (exit 1)** when the git-version constant disappears. I confirmed the first-order coupling by trial-deleting the `drive` path, goal layer, injection and reconcile in a scratch copy and reading the compiler's 116 errors.

D-09's conditional fires: the push step persists the last stderr line of `git push` and `gh pr create` into the run's LOG.md (`log_event(store, "push-failed"/"pr-failed", detail)`) after only `redact_urls` (URL userinfo). `journal::redact::redact` (token-pattern table, includes a `gh` rule) is not applied. Routing those two captures through `journal::redact::redact` is the concrete "SAFE-04 rewired into the launcher" change.

Measured baseline (clean HEAD copy `e6053d7`, debug-info off, no fail-fast): **4411 passed / 2 failed / 30 ignored over 94 `test result:` lines**. The 2 failures are the git-version witness (expected locally) and `render_escape_guard::tests::the_selected_run_view_sites_are_reached`, which is the concurrent 28.1 WR-09 posture-row change, not Phase 31. Expected after the full phase: roughly **3,250-3,400 passed over ~57-59 result lines** (estimate; must be re-measured).

**Primary recommendation:** Order the code plans so each deletion leaves the tree compiling and green: (1) decouple + retarget, (2) remove the legacy TUI surfaces, (3) delete the `drive` core + goal layer + injection, (4) [droppable] Codex/executor remnants, (5) delete the guard/classifier/witness together with the `pre-tag-check.sh`/Dockerfile/CLAUDE.md edits in one commit, keeping `envelope scan`, (6) container gate. The docs-only threat/Phase-22/REQUIREMENTS plan runs in parallel.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Namespace refusal (SAFE-01) | Launcher push step (outside sandbox, `launcher/push.rs::check_destination`) | `envelope::policy` (pure fns) | Push leaves only via fixed refspec `tip:dest` (LNCH-06) |
| Secret scan (SAFE-03) | Launcher push step (`scan_with_external` on a scan checkout) | `envelope::scan` + `journal::redact` rule table | Runs before push, outside sandbox |
| PR cap ledger (SAFE-06) | Launcher push step (`record_and_check`) | `envelope::ledger` file under `~/.local/share/gsd-meta-manager/envelope/<alias>/` | File is outside the sandbox `$HOME` allowlist |
| Capture-time redaction (SAFE-04) | `journal::redact` | Launcher push step log events (to be routed) | Redaction must happen before bytes reach disk |
| `Untrusted` display type | TUI (`text.rs`) | CLI echo sites | Display/escape concern, survives intact |
| Router (`next --json`, SAFE-08 alphabet) | `driver::router` + `next_cli.rs` | `rate_limit` tests reuse constants | Kept by D-07; conformance test unchanged |
| Steering | `driver::steer` (DIRECTION.md `control`) | Driver tab (Phase 30) | Replaces stream-json injection |
| Legacy drive / goal / injection / guard | — (deleted) | — | Superseded by sandbox + capability removal |

## Corrections to CONTEXT premises (the planner MUST read these)

Each was confirmed against source this session (compiler trial, `Read`, or `sed` print of the file).

| # | CONTEXT says | Actually | Consequence for the plan |
|---|---|---|---|
| C1 | D-03: delete `driver_confirm.rs` | Shared: `DriverAction::{ToggleOptIn, StopLauncher}`, `DriverConfirmScreen::new_launcher_stop`, `dispatch` and the test helpers `tests::{ctx_with_project, ALIAS}` are imported by `detail.rs`, `normal.rs`, `driver_launch.rs`, `driver_steer.rs`, `driver_run.rs`, `help.rs`, `render_escape_guard.rs` (trial compile: 27 errors) | **Trim, don't delete.** Remove `DriverAction::{Start, Stop}`, `new_start`, `do_start_run`, `do_stop_run`, `goal_row`, `DEFAULT_DRIVE_COMMAND` (also used by `action.rs`, legacy), `render_disclosure`/`SECTION_*` (only the Start prompt reads them) |
| C2 | D-08: delete `injection_rows`/`injection_block` in `ui/screens/driver.rs` (implies the file is legacy) | `driver_run.rs:35-39` (Phase 30) imports `driver_panes, follow_indicator, label_style, muted_style, pane_header_line, render_dry_run_preview, render_run_detail, ring_overflow_notice, run_list_row, run_list_title, shown_capped, DriverTabRegions, DRIVER_PANE_TITLE, DRY_RUN_LOADING`; `detail.rs`/`driver_launch.rs` import `shown_capped` | **Trim `driver.rs` to the shared helpers.** `render_run_detail`, `run_list_title`, `run_list_row`, `render_dry_run_preview` are reached only from the legacy-row/preview branches of `render_launcher_tab` and the `legacy_*` helpers, so they go with the legacy section. `DRY_RUN_LOADING` is used as a generic "Loading..." in live code; keep it (rename optional) |
| C3 | D-05: delete the whole hidden `Commands::Envelope` CLI | `envelope scan` is the evidence gate in `scripts/launcher-acceptance.sh:940` and `scripts/prompt-acceptance.sh:1134` (mitigations T-28-27 / T-29-24) | **Keep `EnvelopeAction::Scan` only**; delete `PrePush`, `PreCommit`, `Askpass`, `Guard` and `judged_alias_or_exit` (only those four arms use it; the Scan arm uses `Alias::new`). Alternative: rename to a `driver` subcommand and edit both scripts. `[inferred — audit]` |
| C4 | D-08: delete transcripts 05 and 08 | `08-tooluse-queued-two-turns.ndjson` is `include_str!`'d by `driver/rate_limit.rs:397` (kept; it is the `allowed_warning` rate-limit fixture), `executor/stream_json.rs:534`, `executor/outcome.rs:391`. `05-queued-injection-two-turns.ndjson` by `stream_json.rs:529`, `outcome.rs:387` | **Keep both transcripts.** They are Phase 15 parser captures, not injection code. Deleting them breaks compilation of kept test modules |
| C5 | D-10: corpora only drive the guard/tokenizer | The ledger's T-19-117 size bound and T-19-120 kind bound (FIFO/dir/char-device/symlink, fresh root, forbidden link-non-following-stat slice pin) are pinned **only** in to-be-deleted corpora: `envelope_carrier_reach.rs:1884,2082`, `envelope_interior_path.rs:1985,2029,2078`, `envelope_word_set.rs:2154,2201,2276`. `ledger.rs`'s own 20 unit tests cover none of them | **Retarget these 8 tests** into `ledger.rs` unit tests (or `envelope_pr_cap.rs`) driving `record_and_check_in` directly before the corpora go |
| C6 | `tests/common/mod.rs` not mentioned | `tests/common/mod.rs:20,33` uses `hooks::install_in`; `common::fixture` is used by `agents_claude/fixers/process/scan/waves.rs` and `registry_worktree_guard.rs`; `common::git` by all 8 `agents_*` suites | **Rewrite `common::fixture` without hooks** in the same commit that deletes `hooks.rs`, or 9 kept suites stop compiling |
| C7 | D-03: delete the "legacy runs" section | The 20-tick `RunsReconciled` task in `app.rs:2383-2400` carries Phase 30's `launcher_markers` (dashboard launcher badge) beside the legacy `reconcile_all`/`last_ended_outcomes` | **Rewire, not just delete:** keep the 20-tick task, send only launcher markers (a renamed action or the existing `LauncherMarkerScanned`), drop `runs`/`last_outcomes`. Removes `driver::reconcile`, `ctx.observed_runs`, `ctx.last_outcomes`, the dashboard legacy "driven" badge (`normal.rs:356`) and the `delete_confirm.rs:140` legacy-run guard `[inferred — audit]` |
| C8 | `DriveError` goes with the goal layer | `src/next_cli.rs:151` (router, kept) builds `DriveError::TargetPhaseInvalid` | Give `next_cli` its own refusal (same message text) or keep a minimal error; then delete `DriveError` and its `registry.rs:2121` census arms |
| C9 | D-03: "drive-only parts of `bounds.rs`" | **All** of `bounds.rs` is drive-only in production: callers are `driver/mod.rs`, `run.rs`, `goal.rs`, `escalate.rs`, `error.rs::BoundsRefused`. `rate_limit.rs:863-866` names its reason constants only inside its test module | Delete `bounds.rs` whole; edit the `rate_limit.rs` test that checks the reason ids are distinct |
| C10 | D-04: keep `executor::runtime` because `config.rs`/`registry.rs` use it | Its only consumer is `driver::drive` (`driver/mod.rs:858-864`). After D-03 the per-project `runtime` key and the Preferences `default_runtime` row select nothing; `driver_max_concurrent` (Preferences row, `admit` in `driver::spawn`) also governs nothing | Keep the parse (config compatibility) but treat the two Preferences rows as dead knobs (see Open Question 2) |
| C11 | D-09 conditional ("if it does...") | It does: `push.rs` `git_with_env` stores `redact_urls(&line)` as `last_error`, which reaches `log_event(store,"push-failed",detail)`; the `gh pr create` stderr line reaches `log_event(store,"pr-failed",detail)` — both written via `RunStore::log_launcher_event` | Route both through `crate::journal::redact::redact(...)` (after `redact_urls`). Schedule after the 28.1 code-review agent finishes with `push.rs` |
| C12 | Opt-in is the legacy spawn gate | The launcher does **not** require opt-in: `policy_for(alias, Option<&DriverOptIn>)` falls back to defaults [VERIFIED: src/launcher/push.rs:162-172, read]. After D-03 the kept opt-in toggle's prompts "No NEW run can start afterwards" and "This is the record the spawn gate checks" (`driver_confirm.rs` `prompt_text`) are false | Reword the two strings to what the record now does (carries the PR caps/namespace). Open Question 1 |

Also noted, not a correction: `driver::kill` and `driver::lock` are **partially** legacy. The launcher uses `kill::{signal_group, DRIVER_TEARDOWN_GRACE}` and `lock::{acquire_at, acquire_at_with_retry, probe_at, read_holder_at, run_lock_path, LockProbe, LockHolder, RunLock, ACQUIRE_RETRY_BUDGET}`. Legacy-only: `kill::{stop_run, ReapArm, StopOutcome}` (only `app.rs` `stop_driver_run`) and `lock::{lock_path, acquire, read_holder}` (`meta-manager/runs/run.lock`; these are the only reason `lock.rs` imports `journal::{runs_root, writer}`).

## Deletion / Keep Inventory

LOC from `wc -l` at HEAD. "Prod callers after" means non-test callers once the earlier rows are gone.

### D-03 legacy `drive` (+ reconcile per C7)

| Item | Action | Notes |
|---|---|---|
| `src/driver/run.rs` (5484), `spawn.rs` (381), `dry_run.rs` (869), `bounds.rs` (863), `reconcile.rs` (752) | delete files | `reconcile.rs:722` `include_str!("reconcile.rs")` goes with it |
| `src/driver/mod.rs` lines ~121-1474 + tests 1476-2909 (`DriveArgs`, `RawDriveArgs`, `payload::NonBlank`, `CommandSource`, `command_source`, `ROUTED_RECORD_MARKER`, `preview_text`, `drive`, `approve_plan`, `envelope_refusal`, `platform_refusal`) | delete | Keep the module decls for `direction, steer, router, rate_limit, kill, liveness, lock` and rewrite the module doc (it describes `drive`) |
| `src/cli.rs` `Commands::Drive` (~126-381) + tests at ~620-720 (`drive` parse, `the_project_target_leaves_the_drive_respawn_argv_parsing`) + comments at 126-153 | delete | Also the Driverctl/Bridge comments that point at "the `Commands::Envelope` block above" (fix the wording if Envelope keeps only Scan) |
| `src/main.rs` `Commands::Drive` arm (~370-438) + `use ...driver::{drive, DriveArgs, RawDriveArgs}` + startup `reconcile_all`/`last_ended_outcomes` (~743-765) | delete | |
| `src/error.rs` `DriveError` (632-1160, after C8), `OptInError` variants only `drive` raised (re-check: `driver_confirm`/`detail.rs:21416` tests reference `NotOptedIn`), `SpawnError` if only executor (see D-04) | delete/trim | Update the `registry.rs:2106-2200` untrusted-field census arms that enumerate these variants |
| `src/driver/kill.rs` `stop_run`, `ReapArm`, `StopOutcome` (+ their unit tests) | delete | Keep `signal_group`, `DRIVER_TEARDOWN_GRACE` |
| `src/driver/lock.rs` `lock_path`, `acquire`, `read_holder` (+ tests 396-480 using `runs_root`) | delete | Drops `use crate::journal::{runs_root, writer}` |
| `src/executor/mod.rs` `DrivableProject` (+ `from_registry`, `for_testing_bypassing_opt_in`) | delete | Prod callers after: none (driver mod, dry_run, `schedule_dry_run_report`). `detail.rs:21412,21450` tests: retarget to `registry::is_opted_in` |
| `src/app.rs`: `start_driver_run`, `stop_driver_run`, `schedule_journal_tail`, `report_stop`-family, legacy `update` arms for `RunsReconciled` (rewire, C7), `DriverJournalAppended`, `DriverStartRequested`, `DriverStopRequested`, `DriverStopped`, `DriverRunsListed`, `DriverDryRunLoaded`; `driver_elapsed_redraw_wanted` (re-check); `ChangeKind::DriverJournal` arm at ~2507 | delete | ~30 legacy unit tests (names in the trial: `a_crashed_run_does_not_consume_a_concurrency_slot`, `a_start_request_carries_its_goal_to_the_spawn_seam`, `a_journal_batch_*`, `a_tail_result_advances_only_its_own_cursor`, ...). **Retarget, not delete:** `launcher_stop_stopped_sets_the_status_line_and_relists_the_driver_runs`, `launcher_marker_*`, `the_driver_tab_rescans_on_the_existing_twenty_tick_block` (they currently also expect a `DriverRunsListed`) |
| `src/action.rs` variants `DriverJournalAppended, RunsReconciled (rewire), DriverStartRequested, DriverStopRequested, DriverStopped (+ StopDisposition if unused), DriverRunsListed, DriverDryRunLoaded` + `action.rs` tests at ~372-424 | delete | |
| `src/ui/screens/mod.rs`: `schedule_run_list_scan` (2008), `schedule_dry_run_report` (2315), `AppContext::{observed_runs, last_outcomes, journal_cursors, session_spawned_runs}`, `ProjectViewCache::{driver_runs, legacy_expanded, legacy_row_selected, driver_selected_run, dry_run preview}`, `DriverRunJournal` | delete | Paired call sites: `app.rs:1377,3155,3855`, `detail.rs:1577,3723,6019`. Delete the `schedule_run_list_scan` line and keep the adjacent `schedule_launcher_refresh` |
| `src/ui/screens/normal.rs` `r`/`x` keys (768-795), legacy `driven_and_live` badge input (356), legacy tests | delete | **`o` (796):** see Open Question 1 — the Driver-tab `o` (`detail.rs:6048`) still toggles opt-in |
| `src/ui/screens/driver_start.rs` (877) | delete file | |
| `src/ui/screens/driver_confirm.rs` | trim (C1) | Its own census (`ui/screens/driver.rs` `CENSUS_FILES`) and `ui/mod.rs:358-360` `MEASURED_REACH` counts change |
| `src/ui/screens/driver_run.rs` legacy section: `has_legacy`, `legacy_rows_shown`, `toggle_legacy`, `legacy_header_row`, `legacy_row`, `Selection::Legacy`, the `RunVerdict` import, legacy branches of `render_launcher_tab` + tests (`mixed_cache` etc.) | delete | Phase 30 file — coordinate; it is also touched by 28.1 WR-09 |
| `src/ui/screens/driver.rs` | trim (C2) | Delete the legacy run renderer, injection states, goal/steps/cost rows, `TerminalState::from_outcome` if `RunOutcome` goes (check `ui/screens/mod.rs:37` use) |
| `src/journal/mod.rs`: `RunSummary`, `list_runs`, `last_ended_outcome`, `sort_run_summaries_newest_first`, `new_run_id`, `run_paths`/`RunPaths` (if `park_at` goes), `JournalEvent`, `RunRecord`, `RecordedBounds`, `EMITTED_KINDS`, `JournalRun`, `from_exec_event`, `stream_label`, `RETAIN_RUNS`, `MISSED_*`, `ChangeKind::DriverJournal` | delete | **Keep:** `is_plain_path_component` (next_cli, registry, text, envelope), `classify_change` + `ChangeKind::{DriverRun, Ignored, Planning}` (watcher, app, steer), `sha256_digest` (registry opt-in), `argv_digest` (check), `runs_root` only while lock legacy fns live, `redact`, `reader::{tail_file, TailCursor}` (driver_run LOG.md tail). Map `.planning/meta-manager/runs/**` to `ChangeKind::Ignored` (code example below) |
| `src/journal/writer.rs` | trim | `JournalWriter`, `create_run_dir`, `write_run_record`, `prune_runs`, active pointer, `crash_test_writer_loop` and its consumer `tests/journal_crash.rs` lose their prod callers. `ensure_runs_root`/`RUNS_GITIGNORE_BODY` only if lock legacy stays. Decide per D-09 ("if the writer survives") |
| `src/experimental.rs:120-186` test `the_flag_neither_weakens_opt_in_nor_the_concurrency_cap` | delete/retarget | Uses `driver::spawn::admit` |

### D-07 goal layer
`src/driver/goal.rs` (1497), `escalate.rs` (688), `untrusted.rs` (462) — delete. Their `pub mod` lines; `error.rs` `GoalRefused`, `EscalationRefused`, `GoalSeamUnusable`, `PlanApproval*`, and the `From` impls at 1131-1140; `journal/mod.rs` `approval_digest`, `render_approval_token`, `parse_approval_token`, `ApprovalTokenError`, `ApprovedPlan`, `ApprovalRefusal`, `recheck_approval`, `ABSENT_INPUT_DIGEST`, `APPROVAL_TOKEN_SEPARATOR` and the ~3060 tests. **Keep:** `router.rs` `RouterAction`, `SAFE_COMMAND_ALPHABET`, `is_goal_met`; `text.rs` `Untrusted`; `session_detector` resume code. Many doc comments link `[`crate::driver::untrusted`]`, `goal::`, `escalate::` (`app.rs:491`, `registry.rs:784`, `state_reader/mod.rs:232`, `state_md.rs:541,647,1331`, `router.rs:388`, `text.rs:21,49`, `cli.rs:217,293`, `journal/mod.rs:698,748,921,1413,1581`). They break only `cargo doc`, which no gate runs, but they rot. Sweep with `rtk proxy grep -rn "driver::\(goal\|escalate\|untrusted\|run\|spawn\|dry_run\|bounds\|reconcile\)" src`.

### D-08 injection
`src/journal/inbox.rs` (495), `src/ui/screens/driver_inject.rs` (380) — delete. `Action::DriverInjectRequested/Written`; `app.rs` `schedule_inbox_append` (2128) and handlers; `RunPaths::inbox` (`journal/mod.rs:392`); `ui/screens/mod.rs:1179,2037` inbox uses; `driver.rs` `InjectionState`, `MissedReason`, `InjectionEntry`, `derive_injection_states`, `injection_rows`, `injection_block`, `INJECTION_KINDS` (also `app.rs`), `GLYPH_QUEUED`… `LABEL_*`; `render_escape_guard.rs:1346,2269,2534,2559` fixtures; `tests/driver_inbox.rs`; the inbox half of `tests/journal_gitignore.rs`; fixtures `fake-claude-paced.sh`, `fake-claude-turns.sh` (only `driver_inbox`). Executor stdin write path: `ExecutionHandle::write_line`/`stdin_tx`/`WriterCommand` (see D-04b). **Transcripts 05/08 stay (C4).**

### D-05 guard
| Item | Action |
|---|---|
| `src/envelope/hooks.rs` (2377), `cred.rs` (1586), `advisory.rs` (873) | delete files |
| `src/envelope/policy.rs` 191-2416 (GitVerdict, GitContext, classify_git ... `resolve_push_context`, `disallowed_tools`) and 2492-7093 (Token, tokenizer, programs, `ENVELOPE_ENV_KEYS`, `ProtectedPath`, carriers) + ~96 of 106 tests, including `the_config_section_constants_record_the_git_version_they_were_derived_against` (10729) and the `include_str!("ledger.rs"/"cred.rs"/"policy.rs")` source-slice tests (9071, 11022-11024, 11147, 11542, 12568) | delete |
| `policy.rs` keep: 15-189 (`DEFAULT_NAMESPACE_ROOT`, `REFS_HEADS`, `RESERVED_FIRST_SEGMENTS`, `REASON_*`, `default_namespace`, `validate_namespace`, `PushVerdict`, `classify_push_ref`, `ParkReason`) + 2418-2490 (`DEFAULT_PR_CAP_PER_24H`, `DEFAULT_PR_CAP_PER_RUN`, `EnvelopePolicy::resolve`) + ~10 tests (7116, 7134, 7156, 7178, 7441, 7519, 7531, 7553, 7564, 7593) | keep; rewrite the module doc (lines 1-13 cite `super::hooks` and `resolve_push_context`) |
| `src/envelope/mod.rs` `park`, `park_at`, `ParkOutcome` (no caller outside envelope), `pub mod hooks/cred/advisory`; keep `ENVELOPE_ROOT_ENV`, `envelope_root`, `envelope_dir`, `envelope_dir_in`; rewrite the module doc (three-layer model) | trim |
| `src/envelope/ledger.rs:442` `super::hooks::GUARD_TIMEOUT_SECS` + doc refs at 6, 60, 171, 258, 345 | decouple first (code example) |
| `src/cli.rs` `EnvelopeAction::{PrePush, PreCommit, Askpass, Guard}` (keep `Scan`, C3); `src/main.rs` the four arms + `judged_alias_or_exit` (main.rs:34) | delete |
| `src/executor/mod.rs:538,571` `envelope_env` + `claude.rs:483,563` + `codex.rs:203,221` | delete (mandatory in D-05 even if D-04 is dropped) |
| `ParkReason` (discretion) | Recommend pruning to `{PushOutsideNamespace, SecretDetected, PrCapExceeded}` + their constants. The others have no producer left: the remaining `REASON_*` producers outside the deleted files are only `envelope/mod.rs` docs and `journal/mod.rs`/`error.rs` text. `launcher_push.rs` asserts `pr_cap_exceeded` |
| `EnvelopePolicy.credential` / `config::CredentialSource` | **Leave as is** to avoid touching `push.rs` again (`policy_for` builds `credential: None`). Dead but harmless; optional cleanup `[inferred — audit]` |
| `src/driver/mod.rs` `use crate::envelope::{self, policy::ParkReason}` | gone with D-03 |
| `tests/envelope_*` corpora (18 files, 471 passing tests) | delete (C5 retargets first) |
| `tests/envelope_pr_cap.rs` | retarget tests 173-271 (5) from `hooks::guard_in` to `ledger::record_and_check_in`; delete 312-475 (settings/argv tests, 5); split `a_hostile_alias_gets_no_settings_file_and_no_ledger` to its ledger half |
| `tests/driver_lock.rs:785,832` (settings path tests) | deleted with the file (D-03) |
| Witness and release tooling | see §Release tooling |

### D-09 SAFE-04
Keep `journal/redact.rs` whole (`redact`, `redact_value`, `credential_alternation`, `credential_rule_names` — `scan.rs` consumes the alternation). Add the two push-step routing calls (C11). `main.rs` keeps `journal::redact` only if a kept arm uses it; today only the deleted Askpass/Guard arms do.

## D-04 isolation map (the droppable plan)

**Coupling verdict: D-04 can be dropped cleanly with one exception.** Once D-03 deletes `run.rs`, every Codex/transport item has zero production callers, but they are `pub`, so they keep compiling and clippy stays quiet. The exception: `ExecutionOptions::envelope_env: Option<crate::envelope::cred::EnvelopeEnv>` (`executor/mod.rs:538`) and its reads in `claude.rs:483,563` and `codex.rs:203,221` **must** be removed by the D-05 plan when `cred.rs` goes. If D-04 runs before D-05, `codex.rs` is already gone. If D-04 is dropped, D-05 edits `codex.rs` (2 lines). `tests/driver_codex_runtime.rs` belongs to **D-03**, not D-04: it calls `drive()` and cannot compile once `drive` is gone.

### D-04a — Codex only (smallest droppable unit)
| Item | Prod callers after D-03 | Test-only callers |
|---|---|---|
| `src/executor/codex.rs` (743: `CodexExecutor`, `build_codex_argv`, `scrubbed_from_codex_child`) | none | own tests (9); `session_detector.rs:1205` `the_executors_own_codex_argv_is_never_an_interactive_session` (delete it) |
| `src/executor/codex_json.rs` (389) | `codex.rs` only | own tests (7) incl. `include_str!` of `tests/fixtures/codex/01-exec-success.jsonl`, `02-exec-failure.jsonl` |
| `executor/runtime.rs` `AgentExecutor` (enum over Claude/Codex executors, 204-330), `codex_command` (161) | none (`run.rs` only; `queue_md.rs:145` is a doc mention) | runtime tests |
| `tests/fixtures/codex/`, `tests/fixtures/fake-codex.sh` | — | only `driver_codex_runtime.rs` (D-03) and `codex_json` tests |
| `spawn_seam_guard.rs` `the_codex_spawn_seam_takes_the_capability_type` (4511), `no_executable_line_in_src_carries_a_codex_bypass_flag_or_the_full_access_sandbox` (4545), `the_codex_bypass_scan_fires_on_code_and_not_on_a_comment` (4557); `SPAWN_ALLOWLIST` entry `src/executor/codex.rs` | — | edit |
| `async_blocking_guard.rs:226` `("src/executor/codex.rs", ".spawn()")` | — | edit |
| README 37-44, 62, 113; `docs/GSD-CORE-SYNC.md:343` | — | doc edit |
| **Keep:** `AgentRuntime::{Claude, Codex}`, `UnrecognizedRuntime`, `from_config_value`, `resolve` (config parse; `RegisteredProject::runtime`, `Preferences::default_runtime`); session-detector `pgrep -x codex` interactive-session detection | | |

### D-04b — Claude transport remnants (also droppable; recommend deleting for "no parallel model")
These all lose their last production caller with D-03 (verified by grep over `src/` excluding the deleted files):
`ClaudeExecutor` and `build_argv`, `interrupt_stopped_a_turn`, `MAX_LINE_BYTES`, `EVENT_FORWARD_TIMEOUT`, `POST_EXIT_DRAIN_CAP`, `ABORTED_STREAMING` (claude.rs, 2591); `Executor` trait, `BoxFuture`, `PendingControl`, `ExecutionHandle` (+ `write_line`, `stdin_tx`, `close_input`, `wait_outcome`), `WriterCommand`, `ExecutionOptions`, `ExecutionId`, `SpawnProfile`, `SettingSources`, `PermissionMode`, `ExecutionTarget`, `TurnOutcome`, `InterruptAck`, `encode_interrupt` (mod.rs); `gate.rs` (595, whole); `outcome.rs` (1187, whole: `RunSnapshot`/`DiskDelta` only fed `bounds.rs`); `SpawnError`/`SendError`/`CapabilityError` (error.rs).
- `ExecutionEvent` + `RunState` + `RunOutcome`: production users are `app.rs::apply_exec_event`, `ctx.run_states` and `main_loop::ExecEvent`. **That channel already has no sender at HEAD:** `exec_tx` is created in `main.rs:713-718` and nothing sends on it. Removing it touches `main.rs`, `main_loop.rs` (`pump` signature + its 3 tests), `app.rs`, and the `exec_tx: None` literals in 6 test contexts.
- `stream_json.rs`: **keep** (CONTEXT). After D-03/D-09 trimming its only consumer is the `rate_limit.rs` test module (`parse_line`, `Envelope`, `StreamMessage`). Keeping it also keeps transcripts 01-08 meaningful.
- Tests removed: `executor_lifecycle.rs` (14), `executor_transport.rs` (10), unit tests in claude (19), gate (17), outcome (32), executor mod (10); `session_detector` tests that use `claude::build_argv` (`the_executors_own_argv_is_an_argv_this_build_can_read_back` ~2035 and the `build_argv` round-trip ~1932) — delete with the producer; `spawn_seam_guard` claude/escape-hatch/`DrivableProject` tests; fixtures `fake-claude-{deaf,echo,late-init,orphan,silent,slow,spawner}.sh`, `fake-claude.sh`.
- **Dependencies:** `process-wrap` loses its only users (`claude.rs:71`, `codex.rs:35`) → remove from `Cargo.toml` **and commit the regenerated `Cargo.lock`** (gate 6 runs `--locked`; a stale lock fails). `uuid` stays (journal/app; re-check after the journal trim). `tui-textarea` is **already unused at HEAD** (0 `tui_textarea` refs in `src/`): pre-existing, out of scope, note it only.

## Call-graph edges from kept pieces into deleted code (and where each lands)

| Kept piece | Edge into deleted code | Resolution |
|---|---|---|
| `envelope/ledger.rs` | `super::hooks::GUARD_TIMEOUT_SECS` (:442 message) + docs | Ledger-owned constant or reworded message; the bound is about a count the push step can establish, not a hook deadline |
| `envelope/ledger.rs` | bound tests only in corpora (C5) | move into `ledger.rs` tests |
| `envelope/policy.rs` kept head | `use crate::config::{CredentialSource, DriverOptIn}` (:15) | keep both (EnvelopePolicy unchanged) |
| `envelope/scan.rs` | `crate::journal::redact` (:48) | unchanged (journal::redact kept) |
| `envelope/mod.rs` `envelope_dir_in` | `journal::is_plain_path_component` | unchanged |
| `launcher/push.rs` | `ledger`, `policy` namespace/caps, `scan` | unchanged; plus the D-09 redact routing (C11) |
| `launcher/run.rs` | `EnvelopePolicy`, `driver::kill::{signal_group, DRIVER_TEARDOWN_GRACE}`, `driver::lock::RunLock` | unchanged |
| `launcher/outcome.rs` | `driver::rate_limit` | unchanged; `rate_limit.rs` tests keep transcripts 01/08 and `stream_json::parse_line` |
| `next_cli.rs` (router) | `error::DriveError::TargetPhaseInvalid` (C8) | local refusal |
| `ui/screens/driver_run.rs` (Phase 30) | `ui::screens::driver::*` helpers (C2), `journal::reader::{tail_file, TailCursor}`, `driver::reconcile::RunVerdict` (legacy rows only) | keep the helpers + reader; drop the RunVerdict use with the legacy rows |
| Driver tab / dashboard / help / steer screens | `driver_confirm::{DriverAction, DriverConfirmScreen, dispatch, tests::*}` (C1) | keep the trimmed `driver_confirm.rs` |
| 20-tick marker scan (`app.rs` ~2383) | `reconcile::{reconcile_all, last_ended_outcomes}`, `Action::RunsReconciled` (C7) | send markers only |
| `watcher.rs`/`app.rs`/`steer.rs` | `journal::ChangeKind::DriverJournal` | variant removed; `runs/**` → `Ignored` |
| `tests/common/mod.rs` → 8 `agents_*` + `registry_worktree_guard` | `hooks::install_in` (C6) | fixture without hooks |
| `scripts/{launcher,prompt}-acceptance.sh` | `envelope scan` CLI (C3) | keep `EnvelopeAction::Scan` |
| `scripts/pre-tag-check.sh` | `src/envelope/policy.rs` + the constant (HARD ERROR if missing) | edit in the **same commit** as the constant deletion |
| `Untrusted` (`text.rs`) | none into deleted code (only doc links) | unchanged |
| Router (`router.rs`) | doc link to `driver::goal::parse_action` (:388) | doc fix only; `tests/driver_router_conformance.rs` imports only `router` + `state_reader` and is unchanged |

## Test classification (integration test files)

Baseline per-binary passed counts are from the measured run.

| File | Passed | Class | Reason |
|---|---|---|---|
| `envelope_{advisory, argv_deletion, callee_grammar, carrier_reach, command_position, config_resolution, control_carrier, credential, expansion_slots, hook_refusals, interior_path, literal_decision, reparsed_value, tracer, wiring, word_set, wrapper_bypass, wrapper_class}` | 471 | **delete** (after C5 retargets) | guard/tokenizer/hook/cred |
| `envelope_pr_cap` | 11 | **retarget** (5 kept via `record_and_check_in`, 5 deleted, 1 split) | ledger survives |
| `driver_{codex_runtime, dry_run, escalation_cap, goal_seam, inbox, injection_corpus(+10 ign), iteration_loop, model_seam(+3 ign), optin, rate_limit, reattach, refusal_record, tracer}` | 117 | **delete** | every test enters through `drive()` or the goal seam |
| `driver_kill`, `driver_kill_startup` | 4 | **delete** | spawn a real `drive` and exercise `kill::stop_run`, which is legacy-only. `signal_group` stays covered by `kill.rs` unit tests + launcher stop tests |
| `driver_lock` | 8 | **delete** | 4 via `drive()`, 2 envelope-settings tests; `the_lock_is_released_when_the_holding_process_dies` uses the legacy `lock::acquire`. The launcher lock API is covered by `driverctl_lock_race`, `launcher_*`. Optionally retarget that one test to `acquire_at` |
| `driver_rate_limit` | 14 | **delete** except `the_synthesised_fixtures_quota_event_classifies_as_a_rejection` (790), which tests `rate_limit::classify` directly → **move into `rate_limit.rs` tests** |
| `journal_run_paths` | 2 | **delete** | via `drive()` |
| `journal_gitignore` | 3 | **trim** | drop the inbox half |
| `journal_crash` | 3 | **delete or keep with the writer** | only if `JournalWriter`/`crash_test_writer_loop` go (D-09) |
| `executor_lifecycle`, `executor_transport` | 24 | **D-04b** | Claude transport |
| `spawn_seam_guard` | 41 | **retarget** | delete drive/goal/CommandSource/DriveArgs/terminal-write/codex tests (~30); keep and edit `every_process_spawn_site_in_src_is_on_the_allowlist` (bidirectional allowlist; `vanished` fails on stale entries; hard-expects `src/executor/claude.rs` + `DrivableProject`), `no_production_item_follows_a_test_module_marker`, `no_executable_line_in_src_passes_the_hook_disabling_flag`, the MCP-set test if still applicable |
| `async_blocking_guard` | 9 | **retarget** | `REQUIRED_ASYNC_FILES = ["src/driver/mod.rs","src/driver/run.rs"]` (:276) and marker rows at 217-261, 850 name deleted files. Repoint at surviving async files (`src/app.rs`, `src/main.rs`, `src/main_loop.rs`) |
| `agents_*` (8), `registry_worktree_guard` | 84 | **keep** (C6 fixture rewrite) | |
| `driver_router_conformance`, `driver_router_table`, `next_cli` | 30 | **keep unchanged** | router |
| All `launcher_*`, `driverctl_*`, `driver_new`, `driver_prompt`, `driver_tab_launch_e2e`, `state_reader_*`, `registry_test`, `editor_command`, `bounded_output`, `launch_target_cli`, `launcher_acceptance`, `prompt_acceptance` | — | **keep** | |

### In-source census / source-reading tests that go red on a deletion (bidirectional pins)

Update each **in the same commit** as the file it names. These are runtime failures, not compile errors:
- `tests/spawn_seam_guard.rs` `SPAWN_ALLOWLIST` (:48-120; entries for `executor/claude.rs`, `codex.rs`, `outcome.rs`, `driver/kill.rs`, `liveness.rs`, `envelope/policy.rs`, `driver/spawn.rs`, `envelope/advisory.rs`, ...) and `SEAM_SITES` (:1521), `COMMAND_SOURCE_*` (:2524).
- `tests/async_blocking_guard.rs` (see above).
- `src/ui/mod.rs:358-360` `MEASURED_REACH = [("src/ui/screens/driver.rs",1),("src/ui/screens/driver_confirm.rs",4),("src/ui/screens/render_escape_guard.rs",3)]` (Vec equality, fails both directions); `UI_SOURCE_FLOOR = 16` (24 files today, 21-22 after; still above).
- `src/ui/screens/driver.rs:2015` `CENSUS_FILES = ["src/ui/screens/driver.rs","src/ui/screens/driver_confirm.rs"]`. If the census test moves or is dropped with the trim, keep a `driver_confirm.rs` composition check somewhere.
- `src/ui/screens/render_escape_guard.rs`: per-file path list (`driver_inject.rs`, `driver_start.rs`, `driver_confirm.rs` entries) and the legacy-row/injection fixtures (its Driver-tab fixture renders `legacy runs (1), read-only`).
- `src/ui/glyphs.rs:367-382` walks `src/ui` (OK, but drops glyph users).
- `src/session_detector.rs` tests naming `src/executor/claude.rs` (×3) and `build_argv`/`build_codex_argv` (D-04).
- `src/driver/rate_limit.rs:863-866` (bounds reason ids).
- Self-`include_str!`: `envelope/hooks.rs:2350`, `envelope/mod.rs:449`, `journal/mod.rs:3283`, `driver/reconcile.rs:722`, `envelope/policy.rs:9071,11022-11024,11147,11542,12568`. `policy.rs:11023-11024` read `ledger.rs` and `cred.rs`, so a policy test fails if `cred.rs` is deleted before that test is.

## Recommended plan / wave structure

Shared files (`app.rs`, `main.rs`, `cli.rs`, `error.rs`, `driver/mod.rs`, `ui/screens/{mod,normal,detail,driver,driver_run,driver_confirm}.rs`, `spawn_seam_guard.rs`) force the code plans into **sequential waves**. Every plan must end compiling, clippy-clean and green, census tests included.

1. **31-01 Baseline + decouple + retarget** (no deletion yet): record baseline; ledger constant decoupling; move the 8 ledger-bound tests (C5) and the `rate_limit::classify` test; rewrite `envelope_pr_cap.rs` against `record_and_check_in`; rewrite `tests/common/mod.rs` without hooks (C6); `next_cli` local refusal (C8); `ChangeKind` runs→`Ignored` prep.
2. **31-02 Legacy TUI surfaces** (D-03 UI half + D-08 UI): remove `r`/`x` keys, the start/stop/inject handlers and Actions, `driver_start.rs`, `driver_inject.rs`, the legacy runs section, the `driver.rs`/`driver_confirm.rs` trims; rewire the 20-tick marker scan off reconcile (C7). Leaves `driver::{spawn, dry_run, reconcile}` and `journal::inbox` callerless but compiling.
3. **31-03 Drive core + goal layer + injection core** (D-03 core, D-07, D-08 core): CLI `Drive`, `driver/mod.rs` drive, `run/spawn/dry_run/bounds/reconcile/goal/escalate/untrusted`, `journal/inbox.rs`, legacy `kill`/`lock` fns, `DriveError`/goal errors, journal trim, legacy driver tests, `spawn_seam_guard`/`async_blocking_guard` edits, `DrivableProject`.
4. **31-04 [droppable, D-04] Codex + Claude transport remnants**: D-04a, then D-04b (process-wrap removal + `Cargo.lock`). Put it before 31-05 so D-05 does not edit `codex.rs`.
5. **31-05 Guard** (D-05 + D-12, **one atomic commit for the constant + `pre-tag-check.sh` + Dockerfile comment + CLAUDE.md**): delete hooks/cred/advisory, classifier and tests, the four Envelope arms (keep Scan), `envelope_env`, corpora; `ParkReason` prune; module docs; `release.yml:12` comment; `install-conformance-oracle.sh:231` comment; `Cargo.toml:27-29` exclude comment (mentions `tests/fixtures/injection-corpus/`, removed in 31-03); bollard test (optional).
6. **31-06 Docs (parallel with 31-01..05, docs-only files):** Phase 19 closure table in `deferred-items.md` + `19-SECURITY.md` frontmatter; Phase 21 criterion-4 closure + CR-01 backlog todo; Phase 22 frontmatter + `22-SUPERSEDED.md`; REQUIREMENTS traceability (SAFE-02/05/07/08 text). Keep README/CONFIGURATION/help.rs edits **in the plan that changes the surface** (help.rs is code).
7. **31-07 Gate:** full `rtk proxy cargo test --no-fail-fast > log` count vs the estimate; `cargo clippy --locked --target {x86_64-pc-windows-gnu, aarch64-apple-darwin, x86_64-apple-darwin} --all-targets -- -D warnings` (pre-tag-check does **not** mirror CI's cross-target job, per `release.yml:63`); `./scripts/pre-tag-check.sh --container v1.15.0`.

**Precondition:** start execution only after `/gsd-code-review 28.1 --fix` has committed. At research time it was editing `push.rs`, `egress/*`, `home.rs`, `sandbox.rs`, `driver_run.rs`, and its WR-09 change had a red `render_escape_guard` test at `e6053d7`. Re-measure the baseline then. Parallel worktrees for 31-06 are fine locally; do not rely on a push (public upstream).

## Runtime State Inventory

| Category | Items Found | Action Required |
|---|---|---|
| Stored data | (a) Legacy run dirs `.planning/meta-manager/runs/<id>/` (run.json, journal.jsonl, inbox) + `runs/.gitignore` + `runs/run.lock` in any project once driven by `drive`. (b) Envelope dirs `~/.local/share/gsd-meta-manager/envelope/<alias>/` holding hook stubs, askpass stub, generated gitconfig, settings file, **and `pr-ledger.ndjson`**. (c) An envelope marker block in each driven repo's `.git/info/exclude` (`hooks::write_exclude_block`, "the single persistent mutation the envelope makes to a driven repository") | (a) none, left in place (D-03); watcher maps them to `Ignored`. (b) Ledger stays live (push step). Hook/askpass/gitconfig/settings files become inert: they exec `gsd-meta-manager envelope pre-push` etc., which now fails closed with "unrecognized subcommand" (exit 2); git only ran them via `GIT_CONFIG_COUNT` inside drive runs (T-19-02: nothing persisted in repo config). No migration; optionally mention in the summary. (c) none (inert ignore block; still useful for the leftover `runs/`) |
| Live service config | None — no external service holds these names (verified: no CI secret, no server-side ruleset is written by the envelope; module doc "No server-side protection is ever applied") | none |
| OS-registered state | A legacy `drive` process detached in its own process group could still be running at upgrade. After `reconcile` is deleted the TUI cannot see or stop it | Note in the plan summary; operator kills it manually. No systemd/launchd units existed for `drive` |
| Secrets/env vars | `GSD_MM_ENVELOPE_ROOT` (kept: ledger location), `GSD_MM_PROJECT_ROOT` (`cred::PROJECT_ROOT_ENV`, deleted), `GSDMM_EXPERIMENTAL_FEATURES` (kept). Config keys left in users' `config.json`: `driver_opt_in.{credential, prompt_inputs, claude_md_digest, branch_namespace, pr_cap_*}`, `preferences.{default_runtime, driver_max_concurrent}`, per-project `runtime` | Keep every serde field/`extra` flatten so old configs still load. `branch_namespace`/`pr_cap_*` remain live via `EnvelopePolicy::resolve` |
| Build artifacts | Installed binaries (`cargo install`) whose stubs reference `current_exe()`: the new binary lacks the hook subcommands → fail closed. `Cargo.lock` must be regenerated if `process-wrap` is removed | Commit `Cargo.lock` with the `Cargo.toml` edit |

## Threat closure inventory (RETIRE-03)

**The 27 open ids.** Audit 13's terminal statement names 6 blocking ids (T-19-86, 91, 111, 112, 116, 121) and, "beside them, not counted in `threats_open`": T-19-115, 96, 110, 113, 127, 61-73, 84, 85. That is **26** [VERIFIED: 19-SECURITY.md:16040-16052, read]. The frontmatter says `27 open` (`register_totals: 139 total / 112 closed / 27 open / 6 at or above high`). A table parse also leaves T-19-105 reading "ADDRESSED for the config-resolution axis, and OPEN for the tenth…" (:8151), and `T-19-17r` is an unregistered "accepted" item. **The executor must reconcile the 27th id** (most likely T-19-105) by reading the register, and must not guess.

Proposed classes (wording is the planner's discretion):

| Id | Subject (19-SECURITY.md) | Class | Pointer |
|---|---|---|---|
| T-19-86 (high) | `classify_git` denylist default arm, `!`-bodied alias | G (+N) | Guard deleted; push only via launcher fixed refspec (LNCH-06), no push credential in sandbox (LNCH-04), no network (LNCH-09). Conditional on D-11 |
| T-19-91 (high) | three `classify_git` arms answering `Allow` outside the decision region | G | LNCH-04/06 |
| T-19-111 (high) | `.git/config` alias written by a permitted `printf >>` | G + 28.1 | Guard deleted; driver-written `.git/config` is ro/detected and the push step refuses on git-relevant planted findings and runs host git with global/system config off (LNCH-11, 28.1 WR-06) |
| T-19-112 (high) | PR-cap ledger as an agent-reachable FILE | N | Ledger at `~/.local/share/gsd-meta-manager/envelope/<alias>/pr-ledger.ndjson`; `.local/share` is a refused `$HOME` root and only `.local/share/rtk` is bound (LNCH-10) [VERIFIED: src/launcher/home.rs:85-99 quotes `const RTK_REL: &str = ".local/share/rtk";` and `REFUSED_ROOT_REL` incl. `".local/share"`]. Caveat: a user-set `GSD_MM_ENVELOPE_ROOT` inside the project would re-expose it. Conditional on D-11 |
| T-19-116 (high) | guard's own binary replaceable (exact path, tilde, PATH-symlink, parent) | N | Nothing execs a hook any more; the gsd-mm binary/dir is bound ro (28.1 WR-04 area) and `$HOME` is allowlisted (LNCH-10) |
| T-19-121 (high) | `git -c credential.helper=` returns ambient credentials | N | No credential file reachable (LNCH-10), token outside the sandbox (LNCH-12), `~/.ssh` empty / `SSH_AUTH_SOCK` `GH_TOKEN` stripped (LNCH-04) |
| T-19-113 (med) | hook/askpass stubs as files | G | files no longer executed |
| T-19-115 (med) | rule (a) LITERAL condition / carrier alphabets | G | |
| T-19-96, T-19-110 (med) | `git push --forc?` glob; `git config core.hooksPath -c` | G | push step's hardened git args, fixed refspec |
| T-19-61, 62 (med) | `hooks::deny`, park coverage | G | |
| T-19-63, 73 (med/low) | `CredentialSource::Env` disclosure / drift | G | askpass deleted; `CredentialSource` is now inert config |
| T-19-64-72, 84, 85, 127 (low) | second-carrier table, `askpass_into`, `hooks::git_dir`, `protection_line`, `ProtectionState::Unknown`, source-grep pins, honesty pins, doc/park-reason repudiation, `protected_carrier_named` message | G | |
| T-19-105 (if open) | generative alphabets / class axes | G | |
| K-class candidates (re-check if any is still open): T-19-117 (ledger unbounded work vs 5 s guard deadline), T-19-120 (ledger KIND bound) | | K→closed | The work now runs in the push step outside the sandbox with no hook deadline; the size/kind bounds stay in `ledger.rs` and C5 keeps their tests. Close only once the retargeted tests are green |

**Phase 21:** criterion 4 → superseded (D-16; LNCH-04 + 28.1 egress/`$HOME`, residual = LNCH-14 / `docs/DRIVER-SANDBOX.md:144`). Note: REQUIREMENTS still shows LNCH-13/14 `[ ]`/"Gaps Found", while 28.1-VERIFICATION says both VERIFIED and that the REQUIREMENTS rows were reverted by `0afa314` and never restored. D-17 says not to touch 28.1 rows, so the closure row should cite 28.1-VERIFICATION.md SC6, not the REQUIREMENTS checkbox.

**CR-01 is still live** [VERIFIED: src/session_detector.rs:728-734, read this session — `} else if element == RESUME_OPTION_NAME || element == RESUME_OPTION_SHORT_NAME {` … `let next = args.get(index + 1).copied(); index += 1; next.map(|value| (value, SessionIdRank::Resume))`, with `RESUME_OPTION_NAME: &[u8] = b"--resume"` (:425) and `RESUME_OPTION_SHORT_NAME: &[u8] = b"-r"` (:469). The successor is bound unconditionally, which is exactly the optional-value arity error 21-VERIFICATION measured against `claude` 2.1.250]. Per D-16 it gets a **backlog todo** (owner: session detector), not a superseded closure. 21-VERIFICATION also records **CR-02** (shell injection at `detail.rs:1692-1707`, `sh -c` interpolating `/proc`-sourced values) as a confirmed blocker in a **surviving** surface. Re-check whether it was fixed later. If not, it also needs a todo, not a superseded closure [ASSUMED: not re-checked this session]. The memory note "Phase 21 stopped at round 13" says not to resume Phase 21 gap-closure without asking — a todo is not a resumption.

**28.1 landed?** Automated: 28.1-VERIFICATION `status: human_needed`, `score: 7/7` (2026-10-25). 28.1-SECURITY `status: verified`, `threats_open: 0`. 28.1-UAT: D-11 setup-token probe `pending`. STATE.md: `current_phase: "28.1"`, `status: verifying`. The `/gsd-code-review 28.1 --fix` loop is running now (WR-02/05/06/08/09/10 commits observed during this session). N-class rows therefore carry D-02's conditional "reopens against Phase 28.1 if the D-11 live probe fails".

## Experimental gate sites (RETIRE-02 / D-13)

`ctx.experimental` production reads [grep-located, HEAD]:
- **Remove with their surface:** `app.rs` `start_driver_run` (3769), `stop_driver_run` (3897); `normal.rs` `r` (768), `x` (782); `normal.rs:356` (legacy `driven_and_live` badge input, C7); `o` (796) only if Open Question 1 resolves to removal.
- **Keep (live Phase 28/30 surfaces):** `app.rs` `stop_launcher_run` (1418), `launch_target` (1470), `request_new_run` (1598), `steering_target` (1841), marker scan (2383; it stays after the C7 rewire), `LauncherMarkerScanned` (3551); `normal.rs:390` (launcher badge); `ui/screens/mod.rs` `needs_attention_for` (1932), `refresh_launcher_marker_from_cache` (1945), `schedule_launcher_marker` (2254), `attention_rank_for` (2372); `detail.rs` Driver tab key `D` (4751), tab stepping/titles/footer (~21 reads); `help.rs:516` `help_lines(experimental)`.
- Text to update: `help.rs:207-215` doc + Driver section rows (drop r/x/inject legend glyphs), `cli.rs:126-153` comment (drive exemption), README.md:246 ("the dashboard's `r` / `x` / `o` start a…"), `docs/CONFIGURATION.md:117-150` (lines 139-146 describe `driver_max_concurrent` and the `drive` respawn exemption).

## Release tooling (D-12)

- `scripts/pre-tag-check.sh`: lines 572-574 define `POLICY_FILE="src/envelope/policy.rs"`, `CONST_NAME="CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION"`, `WITNESS_TEST="the_config_section_constants_record_the_git_version_they_were_derived_against"` [VERIFIED: scripts/pre-tag-check.sh:572-574, sed-read]. The extraction block (~664-707) **exits 1** with "HARD ERROR: could not extract …" if the declaration is missing. Delete together: state vars `INSTALLED_GIT`/`DERIVED_GIT_VERSION`/`GIT_ADVISORY`/`WITNESS_IN_FAILURES` (599-601), the summary advisory (630-646), the line-650 sentence, the extraction + banner (657-738), the DEC-5 witness detection (~886-891), and header comments (lines 20-25 name "the git-version witness"). Keep everything container-related. CLAUDE.md step 3's bare-vs-container paragraph must drop "and the git-version witness" and its "git reconciliation banner" paragraph.
- `scripts/pre-tag-check.Dockerfile`: **keep the `ppa:git-core/ppa` install; rewrite its comment.** Evidence: the GitHub ubuntu-24.04 image (version 20261012.334.1) lists "Git 2.55.0" [CITED: github.com/actions/runner-images/blob/main/images/ubuntu/Ubuntu2404-Readme.md]. The image exists to mirror the runner, and surviving suites shell out to real git with version-sensitive flags (`launcher/push.rs` hardened args, `launcher_push.rs`, `launcher_e2e.rs`, worktree/mirror operations). No surviving test pins a git version (grep for version literals outside `envelope` returned nothing), so this is a fidelity reason, not a test dependency. The `gh` layer's comment cites the `gh_grammar_probe` tests in `policy.rs`, which are deleted. No surviving test needs real `gh` (`launcher_push.rs` installs a shim), so keep the layer as runner-mirroring and rewrite the comment. Runner gh is 2.102.0 vs the Dockerfile's stated 2.101.0, cosmetic.
- `.github/workflows/release.yml:11-16` comment names the witness — reword ("environment-sensitive" still true for the oracle). `scripts/install-conformance-oracle.sh:231` comment "Mirrors the git-version advisory at scripts/pre-tag-check.sh lines 207-235" — reword.
- `scripts/{launcher,prompt}-acceptance.sh`: only `envelope scan` (kept, C3) → no change if Scan is kept.
- Memory note `git-version-constants-test-env-failure.md` becomes obsolete → report to the coordinator (D-12). Also `cargo-test-failfast-hides-envelope-suites.md` loses its envelope subject (the fail-fast lesson still holds).

## Common Pitfalls

### Pitfall 1: Bidirectional census tests go red on a correct deletion
**What goes wrong:** `spawn_seam_guard`, `async_blocking_guard`, `ui/mod.rs MEASURED_REACH`, `render_escape_guard` path lists and `driver.rs CENSUS_FILES` fail when a listed file vanishes or a count changes.
**How to avoid:** edit the census in the same commit as the deletion; grep `rtk proxy grep -rn '"src/<file>' src tests` before deleting any file.
**Warning signs:** "an allowlisted file no longer spawns anything", "a new render site is not probed".

### Pitfall 2: Deleting the constant before the script
`pre-tag-check.sh` exits 1 at constant extraction, so SC4 cannot pass. Do it in one commit (31-05).

### Pitfall 3: Fail-fast + rtk hide whole suites
Use `rtk proxy cargo test --no-fail-fast > "$LOG" 2>&1` and count `test result:` lines with a script (python/awk run through `rtk proxy`), never `| grep`. Baseline: **94** result lines. A drop to ~57-59 is expected; a drop larger than the deleted-binary count means a suite stopped compiling or running.

### Pitfall 4: `pub` items hide orphans; binaries don't
Clippy won't flag orphaned `pub` lib items, so the D-04 lists must be applied deliberately. Conversely, `src/main.rs`/`next_cli.rs`/`driver_cli.rs` are a bin crate, and orphaned private fns there (`judged_alias_or_exit`) **do** fail `-D warnings`.

### Pitfall 5: Cross-target build not covered by pre-tag-check
`driver/mod.rs` and others carry `#[cfg(unix)]`/`#[cfg(not(unix))]` pairs. Removing one side can break the Windows/macOS clippy job, which `pre-tag-check` does not run (`release.yml:63`). Run the three target clippies locally (targets are installed).

### Pitfall 6: `Cargo.lock` drift with `--locked`
Removing `process-wrap` without committing the regenerated lock fails gate 6 and CI `--locked` steps.

### Pitfall 7: Kept UI text that now lies
The opt-in prompts (C12), the Preferences `default_runtime`/`driver_max_concurrent` rows (C10), the help legends for injection glyphs, and the `ToggleOptIn` "spawn gate" sentence. Honesty conventions in this repo treat these as defects.

### Pitfall 8: Concurrent edits
`push.rs`, `driver_run.rs`, `render_escape_guard.rs` are being changed by the 28.1 review. Rebase plan line numbers at execution and merge, never rebase `dev`.

## Code Examples

### Ledger decoupling (D-05 prerequisite)
```rust
// src/envelope/ledger.rs — replaces super::hooks::GUARD_TIMEOUT_SECS in the size-bound message.
// Name/value are the planner's; the bound itself (MAX_LEDGER_BYTES) is unchanged.
/// How long the push step may spend establishing the count before the bound refuses (T-19-117).
const COUNT_DEADLINE_SECS: u64 = 5; // was hooks::GUARD_TIMEOUT_SECS [ASSUMED value carried over]
```

### Watcher: legacy run dirs become inert
```rust
// src/journal/mod.rs classify_change — the DriverJournal arm becomes Ignored, so leftover
// .planning/meta-manager/runs/** files trigger neither a tail nor a planning reparse.
if parts.len() >= 3 && parts[0] == ".planning" && parts[1] == "meta-manager" && parts[2] == "runs" {
    return ChangeKind::Ignored;
}
```

### SAFE-04 routing in the push step (D-09)
```rust
// src/launcher/push.rs — both persisted child-stderr captures pass the capture-path redactor.
last_error: crate::journal::redact::redact(&redact_urls(&line)),          // git_with_env
Some((_, line)) => (None, crate::journal::redact::redact(&redact_urls(&line))), // gh pr create
```

### bollard never added (RETIRE-02, optional)
```rust
// tests/no_container_crate.rs
#[test]
fn bollard_is_not_in_the_lockfile() {
    let lock = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.lock")).unwrap();
    assert!(!lock.contains("name = \"bollard\""), "Phase 22 is superseded; bollard must not be added");
}
```
`bollard` count today: 0 in `Cargo.toml`, 0 in `Cargo.lock` (grep -c). A runtime read (not `include_str!`) keeps `cargo package` independent of the lock's presence.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|---|---|---|---|
| In-sandbox git argv/bash classifier + hooks + askpass (Phase 19) | Capability removal: no credentials/network in the sandbox; push only via the launcher's fixed refspec (Phase 28/28.1) | 2026-10-14 re-plan | Guard and corpora retire; namespace/scan/ledger move to the push step |
| Model goal decomposition + untrusted boundary (Phase 21) | Sandbox contains injection; DIRECTION.md items are the reviewable plan (Phase 28-29) | 2026-10-14 | goal/escalate/untrusted retire |
| stream-json stdin inbox injection (Phase 18) | DIRECTION.md `control` steering (Phase 30) | Phase 30 | inbox/driver_inject retire |
| Docker container target via bollard (Phase 22) | bwrap + `systemd-run --user --scope` (Phase 28) | 2026-10-14 | archive in place |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|---|---|---|
| A1 | The 27th open Phase 19 id is T-19-105 (or an unregistered T-19-17r) | Threat inventory | Closure table count off by one; the executor must enumerate from the register |
| A2 | Keeping `envelope scan` (not deleting all of `Commands::Envelope`) is acceptable under D-05 | C3 | If the user wants the CLI gone, both acceptance scripts need a replacement scan entry point |
| A3 | Deleting `driver::reconcile` and the legacy dashboard badge/delete guard is within D-03's "journal-run code" | C7 | If not, reconcile must stay and still drive `RunsReconciled` |
| A4 | Expected post-phase counts ~3,250-3,400 passed / ~57-59 result lines | Summary | Estimate only; the plan must record the measured number |
| A5 | Phase 21 CR-02 (`detail.rs` `sh -c`) not re-checked | Threat inventory | A live injection bug could be mis-closed as superseded |
| A6 | `COUNT_DEADLINE_SECS = 5` mirrors the old `GUARD_TIMEOUT_SECS` value | Code example | Message text only |
| A7 | ubuntu-24.04 archive git is far older than 2.55 (Dockerfile comment says "years behind") | Release tooling | Only affects whether dropping the PPA would change the container's git |

## Open Questions

1. **Opt-in toggle after `drive` is gone** (C12). The launcher doesn't require opt-in, so "No NEW run can start afterwards" becomes false. Recommendation: keep the toggle (it is how the PR-cap/namespace record gets created) and reword both prompts. Keep the dashboard `o` or drop it as a duplicate of the Driver-tab `o`; recommend keeping it, since removing it is a UX change, not a retirement. `[inferred — audit]`. Making the launcher honour opt-in would be a new capability, so it is out of scope.
2. **Dead Preferences rows** (`default_runtime`, `driver_max_concurrent`, C10). Recommendation: remove both rows from the Preferences screen (the screen keeps its `g` → GSD template route), and keep the config fields so old `config.json` files still load. If the planner prefers zero UI change, add "(no effect since Phase 31)" labels. Not escalated: both options are reversible.
3. **D-04b scope.** CONTEXT says delete "items that lose their last caller". That reaches the whole Claude transport (`ClaudeExecutor`, `gate`, `outcome`, the exec channel), not only the injection write path. Recommendation: delete them in the droppable 31-04 so a user objection drops the whole plan.
4. **Writer/journal_crash** (D-09 "if the writer survives"). After the trim no production code writes a journal. Recommendation: delete `JournalWriter`, its crash-test binary path and `tests/journal_crash.rs`, but keep `redact` and `reader::{tail_file, TailCursor}`.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|---|---|---|---|---|
| rustc/cargo | all | ✓ | 1.99.1 | — |
| MSRV toolchain 1.89 | gate 2 | in the container image | 1.89.0 | container |
| Targets `x86_64-pc-windows-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin` | cross-target clippy | ✓ installed | — | — |
| docker / podman | `pre-tag-check.sh --container` | ✓ / ✓ | 29.1.3 / 5.7.0 | podman via `PRE_TAG_CONTAINER_RUNTIME` |
| git | tests, push step | ✓ | 2.53.0 (witness mismatch → expected local failure until 31-05) | container has 2.55.0 |
| bwrap, systemd-run, gh | launcher tests | ✓ | — | — |
| gitleaks | optional external scanner | ✗ | — | built-in scan rules |
| GSD conformance oracle | `driver_router_conformance` | ✓ locally; provisioned in container | — | — |

## Security Domain

| ASVS Category | Applies | Control |
|---|---|---|
| V1 Architecture | yes | Deletion must not remove a live control: the namespace refusal, scan, ledger and redaction stay with their tests (C5) |
| V5 Input validation | yes | `is_plain_path_component`/`Alias::new` stay on CLI paths (`envelope scan`, `next`) |
| V7 Error handling / logging | yes | SAFE-04: route push-step captures through `journal::redact` (C11) |
| V14 Configuration | yes | Release gate integrity (pre-tag-check edit is atomic with the constant), `Cargo.lock` consistency |
| V2/V3/V6 | no | No auth/session/crypto change (sha2 users stay in launcher/driverctl) |

| Threat | STRIDE | Mitigation |
|---|---|---|
| A kept safety pin silently deleted with a corpus | Tampering (evidence) | C5 retarget list; compare per-binary counts |
| Kept UI text claims a gate that no longer exists | Repudiation | C12 / Open Questions 1-2 |
| Acceptance evidence gate broken (T-28-27, T-29-24) | Information disclosure | keep `envelope scan` (C3) |
| Orphaned envelope hook stubs executed by an old config | EoP | Fail closed (subcommand gone); nothing persisted in repo config |

## Sources

### Primary (HIGH)
- Compiler trial in a scratch copy of HEAD (stage 1: 74 errors/warnings; stage 2 with Action variants + reconcile cut: 116), `cargo check --all-targets`.
- Measured baseline `rtk proxy cargo test --no-fail-fast` on a clean copy of `e6053d7`: 4411/2/30, 94 result lines, per-binary table.
- Source reads: `src/envelope/{mod,policy,ledger}.rs`, `src/driver/mod.rs`, `src/main.rs:340-640,735-790`, `src/cli.rs`, `src/ui/screens/{driver_confirm,driver,driver_run,normal,preferences}.rs`, `src/launcher/{push,home}.rs`, `src/journal/mod.rs`, `src/config.rs`, `src/executor/*`, `tests/{common/mod,spawn_seam_guard,async_blocking_guard,envelope_pr_cap}.rs`, `scripts/pre-tag-check.sh`, `scripts/pre-tag-check.Dockerfile`, `.github/workflows/release.yml`, `Cargo.toml`.
- Planning: 31-CONTEXT.md, REQUIREMENTS.md, STATE.md, ROADMAP.md §28.1/§31, 19-SECURITY.md (frontmatter, audit trail, terminal set), 21-VERIFICATION.md, 28.1-{VERIFICATION,SECURITY,UAT,REVIEW-DISPOSITION}.md, 30-CONTEXT.md, 28-CONTEXT.md.

### Secondary (MEDIUM)
- GitHub runner-images Ubuntu 24.04 readme (image 20261012.334.1: Git 2.55.0, gh 2.102.0).

## Metadata

**Confidence breakdown:**
- Inventory/coupling: HIGH (compiler-confirmed first-order edges, source-read second-order)
- Test classification: HIGH for whole-file classes, MEDIUM for per-test retarget lists inside `app.rs`/`detail.rs`
- Threat classes: MEDIUM (the register is 16k lines; the 27th id is unresolved)
- Expected counts: MEDIUM-LOW (estimate)

**Research date:** 2026-10-24
**Valid until:** until the 28.1 code-review loop lands (line numbers drift); then 30 days.
