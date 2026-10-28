---
phase: 31
review: 31-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "`DRIVER_TEARDOWN_GRACE` lower-bound guard now compares against a mirror of a deleted constant"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "Comments and assertion messages in `session_detector.rs` still say this build's own executor emits `--session-id` and `codex exec`"
  - id: IN-01
    severity: info
    disposition: fixed
    title: "Comments still cite deleted files and line numbers"
  - id: IN-02
    severity: info
    disposition: fixed
    title: "Broken intra-doc link to a deleted constant"
  - id: IN-03
    severity: info
    disposition: fixed
    title: "Spawn-seam guard still names the process-wrap marker and a capability type that no longer exist"
  - id: IN-04
    severity: info
    disposition: deferred
    title: "Production-dead code is retained, and the module doc overstates its use"
  - id: IN-05
    severity: info
    disposition: fixed
    title: "Retargeted app tests keep \"driver maps\" names but now assert a non-driver map"
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
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "Unregistering a project is no longer guarded against a live launcher run"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "PR-ledger refusal message cites a \"5-second deadline\" the push step no longer has"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "SAFE-04 redaction can be bypassed when the captured stderr line exceeds the 4 KiB tail window"
open: 0
total: 15
recorded: 2026-10-28T05:11:07.620Z
---

# Phase 31: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | fixed | fixed by quick 261028-n12 fc9d536 (the mirror of the retired executor's 10 s grace and the lower-bound test are deleted; the DRIVER_TEARDOWN_GRACE doc gives the surviving below-15 s bound and names launcher::run::stop_group) |
| WR-02 | warning | fixed | fixed by quick 261028-n12 52b4364 (session_detector docs and assertion messages cite the documented claude --session-id option and external launchers, not a retired in-process producer) |
| IN-01 | info | fixed | fixed by quick 261028-n12 715cebe (citations of the deleted executor files, lifecycle test and events reworded in past tense by concept; kill.rs part with 31 WR-01 fc9d536) |
| IN-02 | info | fixed | fixed by quick 261028-n12 715cebe (the intra-doc link to the deleted EXEC_BATCH constant is a plain sentence) |
| IN-03 | info | fixed | fixed by quick 261028-n12 715cebe (the process-wrap marker is out of SPAWN_MARKERS; failure message and guard-of-the-guard comment reworded) |
| IN-04 | info | deferred | still present (stream_json has only a cfg(test) user, AgentRuntime::resolve has no caller, head_sha is test-only, and executor/mod.rs says every submodule has a live user); not small (a roughly 1000-line module move or cfg(test) plus API retirements); also covers registry::check_prompt_input_drift, which has no production caller; todo 2026-10-28-retire-test-only-executor-remnants.md [inferred — cleanup 2026-10-28] |
| IN-05 | info | fixed | fixed by quick 261028-n12 0321761 (tests renamed removing_a_project_interactively_clears_its_per_alias_maps and every_per_alias_map_is_pruned) |
| IN-06 | info | skipped | coverage note only. The MCP posture is pinned at the boundary that matters. ENABLE_CLAUDEAI_MCP_SERVERS=false is in the child env (sandbox.rs). The 28.1 egress body policy refuses mcp_servers and MCP tool blocks, with tests in egress/policy.rs. A local MCP server would run inside the same bwrap sandbox and egress allowlist with no capability beyond the agent's own Bash, so a --mcp-config scan would add no protection [inferred — cleanup 2026-10-28] |
| IN-07 | info | fixed | fixed by quick 261028-n12 715cebe (DriverOptIn docs say the digests are recorded at opt-in and nothing on the launcher path compares them; re-confirm sentences qualified as the retired executor gate only) (not in the current review) |
| IN-08 | info | deferred | still present (delete_confirm.rs restates detail::driver_live_for); the IN-10 fix changes the guard's predicate (drops the ctx.experimental clause and re-reads the locks at confirm time), so sharing the helper now would be undone by that fix; done together with IN-10 in todo 2026-10-28-unregister-guard-cached-live-check.md [inferred — cleanup 2026-10-28] (not in the current review) |
| IN-09 | info | fixed | fixed by quick 261028-n12 715cebe (OVERLONG_STDERR_NOTE no longer hard-codes the byte count) (not in the current review) |
| IN-10 | info | deferred | still present (the guard reads only the cached marker and is gated on ctx.experimental); needs a synchronous lock re-read in a UI key handler, to be checked against tests/async_blocking_guard.rs, plus disclosure in docs/CONFIGURATION.md; same todo as IN-08, 2026-10-28-unregister-guard-cached-live-check.md [inferred — cleanup 2026-10-28] (not in the current review) |
| WR-03 | warning | fixed | 36e0805 (31-08) (not in the current review) |
| WR-04 | warning | fixed | 21b94df, a51b39a (31-09) (not in the current review) |
| WR-05 | warning | fixed | 21b94df (31-09) (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
