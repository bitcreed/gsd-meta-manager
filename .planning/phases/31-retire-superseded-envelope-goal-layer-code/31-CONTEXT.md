# Phase 31: Retire Superseded Envelope & Goal-Layer Code - Context

**Gathered:** 2026-10-24
**Status:** Ready for planning
**Mode:** `--auto`, human unavailable. Every gray-area answer below is inferred from the planning
artifacts and marked `[inferred — audit]`.

<domain>
## Phase Boundary

Delete the code from superseded Phases 19, 21 and 22, or rewire it into the Phase 28/28.1 launcher
and push step, so the tree has no unfinished parallel safety model. Close the open Phase 19/21 threat
ids, each with a pointer to the mechanism that supersedes it. Keep, and leave working, everything
that live code depends on: the namespace refusal, the secret scan, the PR ledger, SAFE-04 redaction,
`Untrusted`, the router (`next --json`), and all Phase 27/28/28.1/29/30 code. The phase ends when
`./scripts/pre-tag-check.sh --container` passes on the reduced tree.

Requirements: RETIRE-01, RETIRE-02, RETIRE-03.

Not in scope:
- New capabilities.
- Changing launcher, egress or sandbox behaviour. 28.1 mechanisms are treated as landed.
- Un-gating the experimental surfaces.
- Any work on the matched todos (see Deferred).

</domain>

<decisions>
## Implementation Decisions

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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase scope and hand-overs
- `.planning/ROADMAP.md` §"Phase 31" (with the Phase 28.1 note), §"Phase 19", §"Phase 21",
  §"Phase 22": goal, success criteria and supersession banners
- `.planning/REQUIREMENTS.md` RETIRE-01..03, SAFE-01..08 traceability rows, LNCH-04/06/09..15
- `.planning/phases/28-sandboxed-driver-launcher/28-CONTEXT.md` D-01 and the push-step decisions
  (namespace, scan, ledger reuse)
- `.planning/phases/30-driver-tab-on-direction-md/30-CONTEXT.md` D-01, D-02, D-03 and Deferred:
  the legacy `drive`, injection and journal-run code handed to Phase 31

### Threats to close
- `.planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md`: register, the 27 open ids,
  the 6 blocking ids (frontmatter line 23)
- `.planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md`: where the closure rows
  are written
- `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md` and
  `21-VERIFICATION.md`: criterion 4 and CR-01

### Superseding mechanisms (pointer targets)
- `.planning/phases/28.1-sandbox-egress-agent-channel-isolation/28.1-CONTEXT.md` D-02..D-10 and
  the residual-risk decision
- `.planning/phases/28.1-sandbox-egress-agent-channel-isolation/28.1-VERIFICATION.md`: 7/7, with
  the D-11 human probe pending
- `.planning/phases/28.1-sandbox-egress-agent-channel-isolation/28.1-SECURITY.md`
- `.planning/phases/28.1-sandbox-egress-agent-channel-isolation/28.1-AUDIT-FINDINGS.md`
- `.planning/spikes/001-queue-driven-master-agent/README.md`,
  `.planning/spikes/002-launcher-caps-and-tool-frontmatter/README.md`: why capability removal
  supersedes the guard

### Phase 22 archive
- `.planning/phases/22-container-execution-target/` (22-01..04-PLAN.md, 22-CONTEXT.md)

### Release tooling touched by D-12
- `CLAUDE.md` §"Release Process" step 3
- `scripts/pre-tag-check.sh`, `scripts/pre-tag-check.Dockerfile`, `.github/workflows/release.yml`,
  `scripts/launcher-acceptance.sh`, `scripts/prompt-acceptance.sh`

</canonical_refs>

<code_context>
## Existing Code Insights

### Keep (live dependents)
- `src/envelope/policy.rs` 23-150 (namespace, `PushVerdict`, `REASON_*`) and 2424-2490
  (`EnvelopePolicy`, PR caps): used by `src/launcher/push.rs:53-57,162-206,550` and
  `src/launcher/run.rs:101`
- `src/envelope/scan.rs` (SAFE-03): used by `launcher/push.rs:58,668`; needs `journal::redact`
- `src/envelope/ledger.rs` (SAFE-06): used by `launcher/push.rs:52,154,792`, `launcher/run.rs:195`;
  it couples to `hooks::GUARD_TIMEOUT_SECS` at :442, which must be decoupled
- `src/envelope/mod.rs:84-136` (`envelope_root`, `envelope_dir*`)
- `src/journal/redact.rs` (SAFE-04); `src/text.rs:568` (`Untrusted`, used widely by the UI)
- `src/driver/router.rs` (`SAFE_COMMAND_ALPHABET`), used by `next_cli.rs` and `rate_limit.rs:867`;
  `tests/driver_router_conformance.rs` imports only the router and `state_reader`
- `src/driver/{lock, direction, kill, rate_limit, steer}`, used by the launcher, driverctl and the
  Phase 30 tab
- `src/executor/runtime.rs` (`config.rs:89,538`, `registry.rs:2225`); `executor::stream_json`
  where `journal/mod.rs:83` still needs it
- 28.1 code (`launcher/config_dir.rs:90 DENY_TOOLS`, egress, `home.rs`) imports nothing that is
  being deleted

### Delete (inventory, approx LOC)
- `src/envelope/{hooks.rs 2.4k, cred.rs 1.6k, advisory.rs 0.9k}` and about 85% of `policy.rs`
  (12.6k, of which about 5.6k is tests)
- `src/driver/{goal.rs 1.5k, escalate.rs 0.7k, untrusted.rs 0.5k, run.rs 5.5k, dry_run.rs}`,
  `drive`-only `bounds.rs`, `driver::spawn`
- `src/journal/inbox.rs`, `src/ui/screens/{driver_inject.rs, driver_start.rs, driver_confirm.rs}`,
  the injection and legacy-runs rendering in `ui/screens/driver.rs`
- CLI: `Commands::Envelope` (`cli.rs:398,738-810`, `main.rs:482-620`) and `drive` (`cli.rs:156`,
  `main.rs:377`)
- Tests: about 40k LOC of `tests/envelope_*.rs` corpora, plus the legacy driver/goal/injection
  suites (D-10)

### Integration points
- Shared files every deletion wave touches: `src/app.rs`, `src/main.rs`, `src/cli.rs`,
  `src/error.rs`, `src/driver/mod.rs`, `src/ui/screens/{mod.rs, normal.rs, detail.rs, help.rs}`
- No Cargo dependency becomes unused. Recheck `process-wrap` once the executor remnants are known.

</code_context>

<specifics>
## Specific Ideas

- The user's carry-in: Phase 19 threat ids that depend on network or `$HOME` confinement point at
  Phase 28.1 (egress proxy plus the `$HOME` allowlist), which now owns them. Nothing 28.1 depends on
  is retired.
- Honest closure: a threat whose attack still applies to surviving code is not marked superseded.

</specifics>

<deferred>
## Deferred Ideas

- Un-gating `GSDMM_EXPERIMENTAL_FEATURES` for the launcher and Driver tab: a product decision for
  later.
- Renaming `src/envelope/` to a push-step name: optional, see D-06.
- Re-adding Codex driving on the Phase 28 launcher: a future phase if wanted. D-04 removes the
  legacy Codex drive.
- Physically moving `.planning/phases/22-*` into a milestone archive: `/gsd-complete-milestone`.

### Reviewed Todos (not folded)
`--auto` folds todos scoring ≥ 0.4. These were overridden as out of scope, because the phase
boundary is fixed and folding them would be scope creep `[inferred — audit]`:
- `2026-08-19-verify-work-gate-policy-configurable.md` (0.6): a router policy feature, not a
  retirement. It is probably superseded by Phase 29's DIRECTION.md standing rules, but that is for
  the todo's owner to decide.
- `2026-07-29-driver-tab-layout-at-medium-terminal-heights.md` (0.5): a UI layout bug.
- `2026-10-21-ambiguous-wide-typography-borders-and-darkgray.md` (0.5): a UI rendering bug.
- `2026-09-22-codex-runtime-remainder-after-mvp.md` (0.4): its legacy-driver half becomes moot under
  D-04. Re-scope the todo after Phase 31 lands; do not fold it.

</deferred>

---

*Phase: 31-retire-superseded-envelope-goal-layer-code*
*Context gathered: 2026-10-24*
