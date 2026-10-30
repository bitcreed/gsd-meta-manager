---
phase: 31
review: 31-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: deferred
    title: "SAFE-04 pump does not redact a multi-line PEM private key in non-JSON output"
  - id: WR-02
    severity: warning
    disposition: deferred
    title: "`spawn_launch_child` leaks a running child when the pump thread cannot be spawned"
  - id: WR-03
    severity: warning
    disposition: deferred
    title: "A dropped host-managed policy value is silent, and the dropped value is a restriction"
  - id: IN-01
    severity: info
    disposition: deferred
    title: "Doc claims a directory at the lock name is `InvalidData`; the write-side open gives `EISDIR`"
  - id: IN-02
    severity: info
    disposition: deferred
    title: "Stale assertion message in the registry hostile-corpus test"
  - id: IN-03
    severity: info
    disposition: deferred
    title: "`RUNTIME_KEY` no longer has a code user"
  - id: WR-201
    severity: warning
    disposition: fixed
    title: "Lock hardening covers `acquire_at` only; `probe_at` and `read_holder_at` still follow symlinks and block on a FIFO"
  - id: WR-202
    severity: warning
    disposition: open
    title: "Socket publish via `hard_link` has no fallback on filesystems without hard-link support"
  - id: WR-203
    severity: warning
    disposition: fixed
    title: "The first exchange-back in `run_loop` still fails on an absent `DIRECTION.md` (WR-10 fix is incomplete)"
  - id: IN-201
    severity: info
    disposition: open
    title: "Two copies of the capped, guarded reader with divergent messages"
  - id: IN-202
    severity: info
    disposition: open
    title: "Stale \"Linux-only\" comments after the macOS widening"
  - id: IN-203
    severity: info
    disposition: open
    title: "A planted FIFO, symlink or oversized `DIRECTION.md` is reported to the operator only as \"invalid data\""
  - id: IN-04
    severity: info
    disposition: open
    title: "Brittle hard-coded census counts in session-detector tests"
  - id: IN-05
    severity: info
    disposition: deferred
    title: "Production code left with no production caller, and garbled doc prose in `config.rs`"
  - id: IN-06
    severity: info
    disposition: skipped
    title: "Src-wide negative guards for Codex bypass flags and MCP config went with their subjects"
  - id: IN-07
    severity: info
    disposition: fixed
    title: "`DriverOptIn` docs in `config.rs` still say the drift check runs at the spawn gate and that drift re-confirms"
  - id: IN-08
    severity: info
    disposition: deferred
    title: "`delete_confirm.rs` re-implements the \"driver is live\" predicate that `detail.rs` already has"
  - id: IN-09
    severity: info
    disposition: fixed
    title: "`OVERLONG_STDERR_NOTE` hard-codes the window size as a literal"
  - id: IN-10
    severity: info
    disposition: deferred
    title: "The unregister guard is advisory: it covers the cached scan only, and its limits are not stated to the user"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "PR-ledger refusal message cites a \"5-second deadline\" the push step no longer has"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "SAFE-04 redaction can be bypassed when the captured stderr line exceeds the 4 KiB tail window"
  - id: WR-101
    severity: warning
    disposition: fixed
    title: "`DRIVER_TEARDOWN_GRACE` lower-bound guard now compares against a mirror of a deleted constant"
  - id: WR-102
    severity: warning
    disposition: fixed
    title: "Comments and assertion messages in `session_detector.rs` still say this build's own executor emits `--session-id` and `codex exec`"
  - id: WR-103
    severity: warning
    disposition: fixed
    title: "Unregistering a project is no longer guarded against a live launcher run"
  - id: IN-101
    severity: info
    disposition: fixed
    title: "Comments still cite deleted files and line numbers"
  - id: IN-102
    severity: info
    disposition: fixed
    title: "Broken intra-doc link to a deleted constant"
  - id: IN-103
    severity: info
    disposition: fixed
    title: "Spawn-seam guard still names the process-wrap marker and a capability type that no longer exist"
  - id: IN-104
    severity: info
    disposition: deferred
    title: "Production-dead code is retained, and the module doc overstates its use"
  - id: IN-105
    severity: info
    disposition: fixed
    title: "Retargeted app tests keep \"driver maps\" names but now assert a non-driver map"
open: 5
total: 29
recorded: 2026-10-30T00:22:33.055Z
---

# Phase 31: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | deferred | duplicate of Phase 28 WR-13 and Phase 29 WR-14 (SAFE-04 pump redacts line by line; a multi-line PEM body in plain-text output reaches launch-N.jsonl); tracked there, not new; not fixed here [inferred — cleanup 2026-10-28] |
| WR-02 | warning | deferred | duplicate of Phase 29 WR-15 (pump thread spawn failure leaves the running sandboxed child unsupervised); related Phase 30 WR-16; tracked there, not new; not fixed here [inferred — cleanup 2026-10-28] |
| WR-03 | warning | deferred | duplicate of Phase 28 WR-10, Phase 29 WR-10 and Phase 30 IN-18 (wrong-shape host managed policy dropped with no diagnostic); tracked there, not new; not fixed here [inferred — cleanup 2026-10-28] |
| IN-01 | info | deferred | duplicate of Phase 30 IN-15 (directory at the lock name documented as InvalidData, actually EISDIR on the read+write open); tracked there, not new [inferred — cleanup 2026-10-28] |
| IN-02 | info | deferred | duplicate of Phase 29 IN-09 (registry redaction-corpus assertion message still says five subjects and names UnrecognizedRuntime); tracked there, not new [inferred — cleanup 2026-10-28] |
| IN-03 | info | deferred | duplicate of Phase 29 IN-11 (RUNTIME_KEY referenced only from comments after the AgentRuntime removal); tracked there, not new [inferred — cleanup 2026-10-28] |
| WR-201 | warning | fixed | fixed by quick 261028-t3d 4977318 (probe_at and read_holder_at open through acquire_at's guard (open_lock_file): a symlink, hard link or FIFO is Unknown(EINVAL) or None without blocking, and the holder read is capped at 4096 bytes; the acquire_at doc claim now holds for every lock-file open in the module) [prior-round id WR-01; renumbered because the 2026-10-29 re-review reuses WR-01 for a different finding [inferred — cleanup 2026-10-28]] (not in the current review) |
| WR-202 | warning | open | - [prior-round id WR-02; renumbered because the 2026-10-29 re-review reuses WR-02 for a different finding [inferred — cleanup 2026-10-28]] (not in the current review) |
| WR-203 | warning | fixed | fixed by quick 261028-t3e 7f4fc49 (same defect as 30 WR-12; run_loop's exchange back and give_back_newest share exchange_or_restore: an ENOENT restores the raced save by no-replace rename, EEXIST exchanges again, and the splice retries on the restored save; guard test b42e57c pins the fail-closed D-17 gate [retry-after-restore inferred — cleanup 2026-10-28]) [prior-round id WR-03; renumbered because the 2026-10-29 re-review reuses WR-03 for a different finding [inferred — cleanup 2026-10-28]] (not in the current review) |
| IN-201 | info | open | - [prior-round id IN-01; renumbered because the 2026-10-29 re-review reuses IN-01 for a different finding [inferred — cleanup 2026-10-28]] (not in the current review) |
| IN-202 | info | open | - [prior-round id IN-02; renumbered because the 2026-10-29 re-review reuses IN-02 for a different finding [inferred — cleanup 2026-10-28]] (not in the current review) |
| IN-203 | info | open | - [prior-round id IN-03; renumbered because the 2026-10-29 re-review reuses IN-03 for a different finding [inferred — cleanup 2026-10-28]] (not in the current review) |
| IN-04 | info | open | - (not in the current review) |
| IN-05 | info | deferred | deferred with prior-round IN-04 (now IN-104): same issue (head_sha and registry::check_prompt_input_drift have no production caller); todo 2026-10-28-retire-test-only-executor-remnants.md. The config.rs DriverOptIn doc-prose sub-point is new this round and goes in the same todo, since that doc is rewritten when check_prompt_input_drift is retired [inferred — cleanup 2026-10-28] (not in the current review) |
| IN-06 | info | skipped | coverage note only. The MCP posture is pinned at the boundary that matters. ENABLE_CLAUDEAI_MCP_SERVERS=false is in the child env (sandbox.rs). The 28.1 egress body policy refuses mcp_servers and MCP tool blocks, with tests in egress/policy.rs. A local MCP server would run inside the same bwrap sandbox and egress allowlist with no capability beyond the agent's own Bash, so a --mcp-config scan would add no protection [inferred — cleanup 2026-10-28] (not in the current review) |
| IN-07 | info | fixed | fixed by quick 261028-n12 715cebe (DriverOptIn docs say the digests are recorded at opt-in and nothing on the launcher path compares them; re-confirm sentences qualified as the retired executor gate only) (not in the current review) |
| IN-08 | info | deferred | still present (delete_confirm.rs restates detail::driver_live_for); the IN-10 fix changes the guard's predicate (drops the ctx.experimental clause and re-reads the locks at confirm time), so sharing the helper now would be undone by that fix; done together with IN-10 in todo 2026-10-28-unregister-guard-cached-live-check.md [inferred — cleanup 2026-10-28] (not in the current review) |
| IN-09 | info | fixed | fixed by quick 261028-n12 715cebe (OVERLONG_STDERR_NOTE no longer hard-codes the byte count) (not in the current review) |
| IN-10 | info | deferred | still present (the guard reads only the cached marker and is gated on ctx.experimental); needs a synchronous lock re-read in a UI key handler, to be checked against tests/async_blocking_guard.rs, plus disclosure in docs/CONFIGURATION.md; same todo as IN-08, 2026-10-28-unregister-guard-cached-live-check.md [inferred — cleanup 2026-10-28] (not in the current review) |
| WR-04 | warning | fixed | 21b94df, a51b39a (31-09) (not in the current review) |
| WR-05 | warning | fixed | 21b94df (31-09) (not in the current review) |
| WR-101 | warning | fixed | fixed by quick 261028-n12 fc9d536 (the mirror of the retired executor's 10 s grace and the lower-bound test are deleted; the DRIVER_TEARDOWN_GRACE doc gives the surviving below-15 s bound and names launcher::run::stop_group) [prior-round id WR-01; renumbered because the 2026-10-28 re-review reuses WR-01 for a different finding [inferred — cleanup 2026-10-28]] (not in the current review) |
| WR-102 | warning | fixed | fixed by quick 261028-n12 52b4364 (session_detector docs and assertion messages cite the documented claude --session-id option and external launchers, not a retired in-process producer) [prior-round id WR-02; renumbered because the 2026-10-28 re-review reuses WR-02 for a different finding [inferred — cleanup 2026-10-28]] (not in the current review) |
| WR-103 | warning | fixed | 36e0805 (31-08) [prior-round id WR-03; renumbered because the 2026-10-28 re-review reuses WR-03 for a different finding [inferred — cleanup 2026-10-28]] (not in the current review) |
| IN-101 | info | fixed | fixed by quick 261028-n12 715cebe (citations of the deleted executor files, lifecycle test and events reworded in past tense by concept; kill.rs part with 31 WR-01 fc9d536) [prior-round id IN-01; renumbered because the 2026-10-28 re-review reuses IN-01 for a different finding [inferred — cleanup 2026-10-28]] (not in the current review) |
| IN-102 | info | fixed | fixed by quick 261028-n12 715cebe (the intra-doc link to the deleted EXEC_BATCH constant is a plain sentence) [prior-round id IN-02; renumbered because the 2026-10-28 re-review reuses IN-02 for a different finding [inferred — cleanup 2026-10-28]] (not in the current review) |
| IN-103 | info | fixed | fixed by quick 261028-n12 715cebe (the process-wrap marker is out of SPAWN_MARKERS; failure message and guard-of-the-guard comment reworded) [prior-round id IN-03; renumbered because the 2026-10-28 re-review reuses IN-03 for a different finding [inferred — cleanup 2026-10-28]] (not in the current review) |
| IN-104 | info | deferred | still present (stream_json has only a cfg(test) user, AgentRuntime::resolve has no caller, head_sha is test-only, and executor/mod.rs says every submodule has a live user); not small (a roughly 1000-line module move or cfg(test) plus API retirements); also covers registry::check_prompt_input_drift, which has no production caller; todo 2026-10-28-retire-test-only-executor-remnants.md [inferred — cleanup 2026-10-28]; same issue re-reported as IN-05 of the current review [prior-round id IN-04; renumbered because the 2026-10-28 re-review reuses IN-04 for a different finding [inferred — cleanup 2026-10-28]] (not in the current review) |
| IN-105 | info | fixed | fixed by quick 261028-n12 0321761 (tests renamed removing_a_project_interactively_clears_its_per_alias_maps and every_per_alias_map_is_pruned) [prior-round id IN-05; renumbered because the 2026-10-28 re-review reuses IN-05 for a different finding [inferred — cleanup 2026-10-28]] (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
