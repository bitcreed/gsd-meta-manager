# Phase 28: Sandboxed Driver Launcher - Context

**Gathered:** 2026-10-19
**Status:** Ready for planning
**Mode:** unattended (operator unavailable; equivalent to `--auto`). Every decision not taken verbatim from
ROADMAP / REQUIREMENTS / Phase 27 hand-offs / spike evidence is tagged `[inferred — audit]`.

<domain>
## Phase Boundary

Port spike 002/003's `launcher.py` / `launcher_s003.py` to Rust as `gsd-meta-manager driver launch` (plus the
`stop` and push verbs it needs). The launcher is the host-side owner of everything the model must not be trusted
with:

- the run lock (`<run-dir>/launcher.lock`, held for the whole run including fresh-exit relaunches);
- the embedded Phase 27 `RunStore` (driverctl single writer), on a dedicated thread;
- the per-run `CLAUDE_CONFIG_DIR` and the env scrub;
- auth (stored `claude setup-token` token as `CLAUDE_CODE_OAUTH_TOKEN`, host-token fallback);
- `bwrap` under `systemd-run --user --scope`, with `RuntimeMaxSec` as backstop;
- the hard caps (dispatch / wall clock / gap rounds) and the rate-limit park;
- exit classification (`classify_exit` → `set_terminal`);
- the push step outside the sandbox (fixed refspec, SAFE-01/03/06 reuse);
- the explicit opt-in degraded (unsandboxed) mode for hosts without bwrap or systemd-run.

In scope: LNCH-01..08, plus the launcher half of DRIVE-01 (re-mapped; Phase 29 closes the prompt half).

Out of scope, owned by other phases:
- the embedded, versioned MASTER-PROMPT, `driver new` run minting and DIRECTION/LOG templates, stop-rule
  precedence text: Phase 29;
- the full Driver tab on DIRECTION.md (launch/steer UI, caps/audit display): Phase 30;
- deleting/rewiring the Phase 19/21 code that Phase 28 does not reuse: Phase 31;
- gap-round counting itself (owned by Phase 27's store, D-26) — Phase 28 owns only gate/trip/kill.
</domain>

<decisions>
## Implementation Decisions

### Musk pass (discuss:pre)
- No scope cuts. LNCH-01..08 each trace to the spike-002/003 user requirements and the 2026-10-14 re-plan; SC2's
  real-driver gap-cap case is an operator-accepted carried risk (26.1 sign-off 2026-10-18), so it has an owner.
  Lean lane not taken: multi-subsystem (process supervision, sandbox argv, auth, git push, CLI/TUI), well over
  3 tasks. `musk milestone-check` reported `run: false`.

### Process model & CLI surface (LNCH-01, LNCH-02, LNCH-08)
- **D-01:** New clap subcommand group `gsd-meta-manager driver launch|stop|push|auth` (same binary, one build,
  Phase 17 D-01 rule). Phase 29 adds `driver new` to the same group. The legacy Phase 17 `drive` subcommand is left
  untouched (removal is Phase 31). `[inferred — audit]`
- **D-02:** `driver launch <project> <run-id>` addresses an **existing** run dir
  `.planning/meta-manager/drivers/<run-id>/` (validated with Phase 27's `validate_run_dir` / `valid_run_id`).
  Minting run dirs is Phase 29; Phase 28 tests build run dirs from the spike-002/003 templates. `[inferred — audit]`
- **D-03:** Until Phase 29 embeds the prompt, `launch` takes the master prompt from `--prompt-file <path>`
  (required in Phase 28); Phase 29 replaces it with the embedded versioned default and keeps `--prompt-file` as a
  hidden override. Default prompt for the Phase 28 acceptance runs: the spike-003 prompt v0.4 and its Run-2
  variant (`MASTER-PROMPT.run2.md`, gap-round rules removed). `[inferred — audit]`
- **D-04:** The launcher is a **long-lived host process that daemonizes itself** (setsid, stdio to the run's
  launch log under the data dir) unless `--foreground` is given; it is the lock holder for the run's whole life.
  Headless only: `claude -p --output-format stream-json`, no tmux, no TTY on the control path (LNCH-08). An
  interactive/tmux attach is not built in this phase. `[inferred — audit]`
- **D-05:** Fresh-per-item / fresh-exit relaunch loop (spike `--loop-mode`) runs **inside one launcher process
  holding the run lock across relaunches** (Phase 27 hand-off 3, R3), acquiring the lock before writing `running`
  and writing the terminal status before releasing it (R4). `begin_launch` is called per launch so `launches` and
  `crash-detected` evidence stay correct.
- **D-06:** `RunStore` is embedded on a dedicated `std::thread` with the socket served from it — never by spawning
  `driverctl serve`, which the launcher's own lock would refuse (hand-off 4). The launcher drives `scan_tick` and
  `flush_heartbeat` every ~2 s (hand-off 9) and passes `serve::socket_path(run_id, run_dir)` as `DRIVERCTL_SOCK`
  (hand-off 5).
- **D-07:** Second launcher on a live run is refused by the run lock (rc 3 as in spike L5, typed `HeldBy` error);
  a relaunch after SIGKILL logs `crash-detected previous_launch=N` via the store. Never a pid file (LNCH-02).

### Stop & kill (LNCH-01, SC1)
- **D-08:** Each launch runs in a transient scope with a deterministic unit name
  `gsdmm-driver-<run-id>-<sha8(canonical run dir)>-<launch-n>.scope` (same hash as the Phase 27 socket name),
  recorded in state via the store. `[inferred — audit]`
- **D-09:** `driver stop <project> <run-id>` (and the TUI) signals the launcher (SIGTERM; the launcher's pid is
  read from the lock-holder record, used only as a signal target, never as liveness). The launcher then stops the
  scope (`systemctl --user stop <scope>` → SIGTERM to every process in the cgroup, systemd escalates to SIGKILL
  after its timeout), writes `stopped` / `stop:user`, and releases the lock last. If the launcher is already dead
  (lock free) but the scope still exists, `stop` stops the scope by name directly and records nothing beyond what
  the reader derives. `[inferred — audit]`
- **D-10:** "Stop kills the whole scope, including descendants" is proven by a test that spawns a grandchild
  (`sleep` inside a bash inside the sandboxed child) and asserts no process of the cgroup survives
  (`systemctl --user show -p ControlGroup` empty / unit inactive, and `pgrep` over the recorded pids empty).
  Die-with-parent (`bwrap --die-with-parent`) stays on as defence in depth (spike L5).
- **D-11:** TUI stop (SC1 "from the TUI") is a thin call into the same library stop function from the existing
  Driver screen for a selected driver run; no new tab or layout. The full launch/steer UI is Phase 30. `[inferred — audit]`

### Hard caps (LNCH-03, SC2)
- **D-12:** Dispatch cap: the gate installed via `RunStore::set_gate` denies the driver-level `PreToolUse Agent|Task`
  dispatch that would exceed the limit (spike L1 semantics: with limit 3, dispatches 1-3 pass and the 4th is
  denied with `LAUNCHER CAP: dispatch cap reached (3/3)`). The PreToolUse hook is seeded with
  `"onFailure": "block"` (hand-off 1), so a dead socket fails closed.
- **D-13:** Wall clock: **active** minutes = summed scope-alive time across launches of the run, persisted in
  state.json (`caps.wall_clock_min.used`). The monitor kills the scope at the limit; each launch's
  `RuntimeMaxSec` = remaining minutes × 60 + 300 s grace (spike value) as backstop. Status after trip:
  `stopped`, reason `launcher-cap:wall_clock_min` (spike L2). `[inferred — audit]` for "active = scope-alive".
- **D-14:** Gap cap: gate reads Phase 27's `caps.gap_rounds.per_phase`. With limit L, a dispatch whose prompt
  names phase P (spike `mentions_phase` rule, ported) is **denied once `per_phase[P] ≥ L+1`**, and the scope is
  **killed when `per_phase[P] ≥ L+2`** (`cap-tripped cap=gap_rounds … action=kill-scope`), status `stopped`,
  reason `launcher-cap:gap_rounds`. Phase 28 never counts gaps itself (Phase 27 D-26). — **Reversibility:**
  costly — the replay fixtures and Phase 29's stop-rule tests pin these numbers.
- **D-15:** Rate limit: the stream's `rate_limit_event` / `result` is classified with the existing
  `src/driver/rate_limit.rs` classifier; an exhausted-quota result parks the run **without retry**
  (`parked`, reason names the quota window and its reset time), and the launcher does not relaunch.
- **D-16:** Exit classification at every launch end calls `counters::classify_exit` then `RunStore::set_terminal`
  (hand-off 7): a driver `RUN end` or a tripped cap takes precedence; clean exit with an open dispatch is
  `stalled`, never `crashed` (27 D-17).
- **D-17:** The spike-003 Stop guard (`stop_guard.py`: at most 3 blocks per run, keeps a driver alive while a
  dispatched worker is unreported) is ported as a second `Stop` hook entry served by the same binary
  (`driverctl hook`, using the shared `outstanding_dispatch` definition). `[inferred — audit]` GSD's
  `planner.stall_detection_enabled` is a project config the launcher must not edit (non-intrusive constraint);
  it stays a Phase 29 prompt/template carry.

### Sandbox & per-run config (LNCH-04, SC3)
- **D-18:** Per-run `CLAUDE_CONFIG_DIR` lives **outside the driven repo**, under
  `dirs::data_local_dir()/gsd-meta-manager/driver/<sha8(canonical project)>/<run-id>/claude-config` (same
  base-dir rule and no-`/tmp`-fallback rule as `src/envelope/mod.rs` D-02). Seeding follows spike `seed_config`:
  symlinks to host `agents`, `commands`, `skills`, `gsd-core`, `get-shit-done`, `hooks` (read-only inside);
  a copy of the user's `settings.json` keeping GSD and rtk hooks, dropping `statusLine` and plugins, appending the
  driverctl hooks; a minimal `.claude.json` (`hasCompletedOnboarding`); a `bin/driverctl` shim that execs
  `gsd-meta-manager driverctl "$@"` on the sandbox PATH (hand-off 8). Global `CLAUDE.md` is **not** linked
  (OQ-R2 default `[inferred]`, carried from REQUIREMENTS). `[inferred — audit]` for the path.
- **D-19:** bwrap profile (port of the spike's, validated in L1 step 0): host root read-only; project checkout
  read-write (the driver commits); run dir read-only; host `~/.claude` read-only; `.credentials.json` and
  `~/.claude.json` masked with `--ro-bind /dev/null`; `<run-dir>/launcher.lock` and `<run-dir>/DIRECTION.lock`
  masked the same way (hand-off 2); `~/.ssh` replaced by an empty tmpfs; per-run config dir read-write;
  `--die-with-parent`, `--unshare-pid`; network kept (claude needs it). Env: the spike `STRIP_ENV` list
  (incl. `SSH_AUTH_SOCK`, `GH_TOKEN`, `GITHUB_TOKEN`, `ANTHROPIC_API_KEY`, `CLAUDECODE*`, `CLAUDE_CONFIG_DIR`)
  is stripped, then `CLAUDE_CONFIG_DIR`, `DRIVERCTL_SOCK`, `CLAUDE_CODE_OAUTH_TOKEN` are set. The token is passed
  in the environment, never argv.
- **D-20:** The bwrap argv and the seeded `settings.json` are built by **pure functions** with golden tests that
  run everywhere (CI included); the live isolation probes of SC3 (EROFS on `~/.claude`, EACCES on the masked
  files, empty `~/.ssh`, absent env vars, EROFS on the run dir, push failing for lack of credentials) run in an
  opt-in live test (`#[ignore]`, skip-with-reason when bwrap/systemd-run absent) executed on this Linux host with
  captured evidence. CI must stay green on runners without bwrap or a user systemd (the router-conformance /
  git-witness lesson from CLAUDE.md). `[inferred — audit]`

### Auth (LNCH-05, SC4)
- **D-21:** `driver auth setup` is the guided onboarding: it explains and runs `claude setup-token` interactively
  on the host, then asks the user to paste the printed long-lived token; gsd-mm stores it at
  `dirs::config_dir()/gsd-meta-manager/driver-oauth-token`, mode 0600, owner-checked on read, never inside a repo.
  `driver auth status` reports presence (never the value). No new keyring dependency. `[inferred — audit]`
- **D-22:** Token resolution at launch: stored setup-token → else host fallback (the host's current
  `claudeAiOauth.accessToken`, read-only, refused if < 5 min to expiry as in the spike). Only the access token is
  ever passed; the refresh token never leaves the host file (spike 002 A2.4 measured that a copied refresh token
  would log the host out). — **Reversibility:** one-way in effect if violated (host logout), so this is a hard
  invariant with a test asserting the sandbox env/config contains no refresh-token material.
- **D-23:** Revocation fallback: a launch whose result contains `OAuth token revoked` (or the equivalent auth
  failure) is classified as an auth failure, not a crash; when on the host-token path the launcher re-reads the
  host token and relaunches, at most **2** times per run, logging `auth-relaunch`; on the stored-token path it
  parks `needs-human` with reason `auth: stored token rejected — rerun driver auth setup`. `[inferred — audit]`

### Push step (LNCH-06, SC5)
- **D-24:** The push runs only on the host, outside the sandbox, via `driver push <project> <run-id>` and
  automatically once at run end for terminal `done` / `parked` / `stopped` (never mid-run, never from inside the
  sandbox). `[inferred — audit]` for the auto-at-end trigger.
- **D-25:** Fixed refspec: `<recorded-branch-tip>:refs/heads/gsd-auto/<alias>/<run-id>`, using
  `envelope::policy::DEFAULT_NAMESPACE_ROOT` (`refs/heads/gsd-auto/`) — the existing reserved namespace, no new
  one. The launch records the branch and start sha; the push sends only that branch's tip, never `--force`,
  never a protected branch (SAFE-01 namespace refusal reused), after the SAFE-03 full-worktree credential scan
  (`src/envelope/scan.rs`) passes. A scan hit parks with the existing SAFE-03 reason. — **Reversibility:**
  costly — the namespace is shared with Phase 19's ledger and pushed refs are visible remotely.
- **D-26:** PR opening is **off by default** (`driver.open_pr = false` in gsd-mm config); when enabled, the push
  step opens at most one PR per run with `gh` on the host, gated by the SAFE-06 rolling-window ledger cap
  (`src/envelope/ledger.rs`). `[inferred — audit]`
- **D-27:** "A push attempted inside the sandbox fails for lack of credentials" is part of the live SC3/SC5 probe
  (D-20): `git push` from inside bwrap to an ssh and an https remote both fail (no `~/.ssh` keys, no agent socket,
  no `GH_TOKEN`, host git credential helpers unreachable). The launcher additionally sets
  `GIT_TERMINAL_PROMPT=0` inside so a push cannot hang on a prompt.

### Degraded mode (LNCH-07, SC6)
- **D-28:** Detection at launch: `bwrap` and `systemd-run` on PATH and a working user manager
  (`systemctl --user is-system-running` not `offline`/unreachable). Missing any → refuse with a typed error naming
  what is missing and the opt-in flag, rc ≠ 0. This applies on Linux too, not only macOS.
- **D-29:** Opt-in is `driver launch --unsandboxed` (no config default, so it is always an explicit per-launch
  act). Degraded runs: no bwrap, no scope; claude spawned as a process-group leader and supervised by the launcher
  (reusing the Phase 17 kill escalation in `src/driver/kill.rs` — SIGTERM group, grace, SIGKILL, reap); caps,
  gate, Stop guard, auth and push step unchanged; wall clock enforced by the monitor alone (no `RuntimeMaxSec`).
  `[inferred — audit]` for the flag name.
- **D-30:** Posture is recorded on the run as an **additive** frontmatter key `sandbox: bwrap-scope | degraded`
  (plus `sandbox_missing: [...]` when degraded) in the Phase 27 schema v1 emitter and typed reader
  (`#[serde(default)]`, no version bump — 27 D-19 additive rule), and shown in the TUI as a one-line badge on the
  existing Driver screen for that run (`UNSANDBOXED` in the warning style). Phase 30 re-hosts it in the new tab.
  — **Reversibility:** costly — schema key read by Phase 30 and persisted in run dirs. `[inferred — audit]`
- **D-31:** macOS: the launcher builds and refuses by default; `--unsandboxed` works. The Phase 27 splice is
  Linux-only (27 D-12), so on macOS degraded mode returns the typed unsupported-platform error from driverctl
  until a `renamex_np` path exists — i.e. degraded mode is functional on Linux-without-bwrap and documented as
  refused on macOS in this phase. Flagged for audit because it narrows SC6's "(macOS)" example. `[inferred — audit]`
  — **Re-assessed 2026-10-21 (quick 261021-0jg):** macOS `--unsandboxed` is supported at the code level (splice portable via renameatx_np RENAME_SWAP, socket falls back to `$TMPDIR`, MAX_SUN_PATH 103); runtime unverified without a Mac; see 28-04-SUMMARY.md 'D-31 re-assessment'. `[inferred — audit]`
  — **Socket length (2026-10-21, quick 261021-1if):** over-limit default socket names fall back to a fixed-length 16-hex name in the same private dir, so a stock macOS $TMPDIR takes any valid run id; D-21 names unchanged where they fit; see 28-04-SUMMARY.md 'Socket name length'. `[inferred — audit]`

### Acceptance cases & the carried risk (SC2, ROADMAP carried risk from 26.1)
- **D-32:** The spike L1-L5 cases are re-run against the Rust launcher on Haiku (cents) with the spike-002
  misbehaving prompts (`experiment/cap-tests/L1..L5`) and `gap-writer.sh`, plus the 26.1 L7 deny/kill tracer
  (`results/r261-l7`) and L8 stop-guard case. Each has pass rules taken from the spike LOG lines (e.g. L1
  `dispatch-denied reason=dispatch cap reached (3/3)`, L4 `cap-tripped cap=gap_rounds used=4 limit=2
  action=kill-scope`, L5 `crash-detected previous_launch=1`). A committed harness script runs them; they are
  opt-in live tests, not default `cargo test`.
- **D-33:** **Carried risk — real-driver gap-cap case.** SC2's gap-cap case must be exercised under a real Opus
  driver with the prompt divergence/gap-round rules disabled (`MASTER-PROMPT.run2.md`), on the spike-003
  **seed v2** scratch project (moving target advances on `gsd-verifier` spawn). Because the 26.1 run `r261-run2b`
  showed Opus self-parking at gap count 2 even with the rules removed, the case uses **gap cap limit L = 0** so the
  deny at L+1 = 1 lands on the round-1 closure dispatch Opus did attempt on `r261a`. Pass requires the **organic
  deny** (`dispatch-denied … phase N has failed verification` from a real Opus dispatch). The kill at L+2 = 2 is
  then required under the same live Opus driver: if the driver does not produce a second organic `gaps_found`
  within the run, the harness's host-side `gap-writer` injects one (L4 shape) and the kill of the real driver's
  scope must be observed (`cap-tripped … action=kill-scope`, no surviving process). The verdict records which half
  was organic vs injected. `[inferred — audit]` (limit 0 and the hybrid kill are inferred; ROADMAP only says
  "under the real (Opus) driver … not only by a scripted stand-in").
- **D-34:** Paid-run discipline (26.1 pattern): the rubric for D-33 is pre-registered in a committed file (by sha)
  **before** any spend; one paid Opus run plus at most one retry; spend ceiling **$20** for the phase
  (`--max-budget-usd` per launch plus a campaign check), cost measured with `cost_split.py`-equivalent transcript
  accounting. If both attempts end without the organic deny, the phase records ADJUST with evidence and the
  carried risk moves forward to Phase 29 (whose SC3 must test the same pairing) rather than looping.
  `[inferred — audit]` for the $20 ceiling and the one-retry cap.

### Claude's Discretion
- Module layout: `src/driver/launcher/` submodules vs a new top-level `src/launcher/` (follow the envelope
  placement reasoning: the launcher is invoked from CLI and TUI).
- Sync std threads vs tokio for the monitor loop (Phase 27 used std threads + one tiny tokio runtime for signals).
- Exact scope-stop mechanism (`systemctl --user stop` vs `kill --signal` on the unit) as long as D-10's test holds.
- Plan split and waves: argv/config builders, auth, push step and degraded detection are largely independent;
  the live launch loop and the acceptance harness depend on them.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & roadmap
- `.planning/ROADMAP.md` §"Phase 28: Sandboxed Driver Launcher" — goal, SC1-6, and the **carried risk** paragraph
- `.planning/ROADMAP.md` §"Phase 29" carried risk, §"Phase 30", §"Phase 31" — consumers and the SAFE-01/03/06 hand-over
- `.planning/REQUIREMENTS.md` §"Launcher (LNCH): Phase 28" (LNCH-01..08), DRIVE-01, SAFE-01/03/06 traceability rows
- `.planning/spikes/MANIFEST.md` — consolidated requirements and the 26.1 carries (stop guard, stall-watch, seed v2)

### Phase 27 (the store this launcher embeds)
- `.planning/phases/27-driverctl-driver-frontmatter/27-CONTEXT.md` — D-20 lock files and ordering, D-21 socket
  path, D-22 liveness, D-25 hook fail-closed + bwrap masks, D-26 gate hook point
- `.planning/phases/27-driverctl-driver-frontmatter/27-06-SUMMARY.md` §"Phase 28 hand-off" — the 9 hand-off items
- `src/driverctl/mod.rs` — header hand-off sections (authoritative copy)
- `.planning/phases/27-driverctl-driver-frontmatter/27-REVIEW.md` — CR-01 (gap scan file-type/size guard under the
  store mutex) is being fixed in parallel; assume it lands before Phase 28 execution

### Spike 002 / 003 (the prototype being ported)
- `.planning/spikes/002-launcher-caps-and-tool-frontmatter/README.md` — A1 caps L1-L5, A2 per-run config and the
  OAuth refresh-rotation measurement, "What a production phase needs" (a)/(d)
- `.planning/spikes/002-launcher-caps-and-tool-frontmatter/tools/launcher.py` — seed_config, bwrap argv, gate, trip
- `.planning/spikes/003-divergent-gap-loop-validation/tools/launcher_s003.py` — STRIP_ENV, host_access_token,
  mentions_phase, Stop guard wiring, RuntimeMaxSec, classify_run_exit
- `.planning/spikes/003-divergent-gap-loop-validation/tools/stop_guard.py`, `tools/exitclass.py`
- `.planning/spikes/002-launcher-caps-and-tool-frontmatter/experiment/cap-tests/` — L1-L6 prompts, gap-writer.sh
- `.planning/spikes/003-divergent-gap-loop-validation/experiment/` — run.sh, seed v2, `MASTER-PROMPT.run2.md`,
  `results/r261-l7`, `results/r261-run2b` (Opus self-park at gap count 2)
- `.planning/phases/26.1-driver-validation-spike-003-re-run/26.1-VERIFICATION.md` — VALID-02 ADJUST, carried risk

### Reused gsd-mm code
- `src/envelope/mod.rs`, `src/envelope/policy.rs` (`DEFAULT_NAMESPACE_ROOT`), `src/envelope/scan.rs` (SAFE-03),
  `src/envelope/ledger.rs` (SAFE-06), `src/envelope/hooks.rs` (namespace refusal)
- `src/driver/lock.rs` (`acquire_at_with_retry`, `probe_at`), `src/driver/kill.rs` (group kill escalation),
  `src/driver/rate_limit.rs` (quota classifier)
</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `crate::driverctl` (`RunStore`, `set_gate`, `begin_launch`, `scan_tick`, `flush_heartbeat`, `set_terminal`,
  `counters::classify_exit`, `serve::socket_path`, `validate_run_dir`): the whole single-writer side is done.
- `src/driver/lock.rs`: run-lock acquire/probe primitives (27-01).
- `src/driver/kill.rs`: two-process-group lesson and SIGTERM→grace→SIGKILL→reap — basis for degraded supervision.
- `src/driver/rate_limit.rs`: pure, clock-injected `rate_limit_event` classifier.
- `src/executor/claude.rs` / `stream_json.rs`: claude spawn + stream-json parsing (process-group leader spawn).
- `src/envelope/{scan,ledger,policy,hooks}.rs`: SAFE-03 scan, SAFE-06 PR ledger, namespace root and refusal.

### Established Patterns
- Typed refusals; Linux-only submodules `#[cfg]`-gated while the clap subcommand stays ungated and returns a typed
  `UnsupportedPlatform` (src/driver/mod.rs).
- Artifacts outside the driven repo under `dirs::data_local_dir()/gsd-meta-manager/…`, no `/tmp` fallback.
- Pure builders + golden tests; live/paid checks opt-in (`#[ignore]`) with captured evidence; mutation checks for
  non-vacuity (27-06).
- `cargo test --no-fail-fast` (fail-fast hides envelope suites); the local git-version witness failure is expected.

### Integration Points
- `src/cli.rs` `Commands`: add `Driver { Launch | Stop | Push | Auth }` (Phase 29 adds `New`).
- `src/main.rs`: headless dispatch, no terminal ownership; daemonize path.
- `src/state_reader` `DriverFrontmatter`: additive `sandbox` / `sandbox_missing` keys.
- Existing Driver screen (`src/ui/screens/driver.rs`): stop action + posture badge only.
</code_context>

<specifics>
## Specific Ideas

- Spike LOG lines are the oracle for L1-L5/L7/L8 pass rules; the Rust launcher must emit the same event grammar
  (the Phase 27 store already does).
- Measured facts to preserve: a host refresh rotates the refresh token and revokes the old access token early;
  `/dev/null` ro-bind masks are `nodev` so even reads fail; `--ro-bind / /` alone left `~/.ssh` readable (spike 001
  bug) — the tmpfs over `~/.ssh` is required.
</specifics>

<deferred>
## Deferred Ideas

- Interactive / tmux attach to a running driver (LNCH-08 says optional; not built here).
- macOS functional sandbox (sandbox-exec or containers) and the `renamex_np` splice — future phase.
- Making GSD's stall-watch (`planner.stall_detection_enabled`) part of the run template — Phase 29.
- Codex runtime under the launcher — out of scope (Claude only).

### Reviewed Todos (not folded)
- "Driver tab renders blank pipeline row at terminal heights 8-13" (0.9, ui) — Phase 30 Driver tab.
- "The verify-work gate policy must be configurable" (0.9, driver) — Phase 29 prompt/router policy.
- "Codex runtime: remainder after the MVP slice" (0.5) — not the Claude launcher.
- "Dashboard renders quoted VERIFICATION status as Unknown…" (0.4) — dashboard reader, filed by 27 D-27.
- "Make queue input truly multi-line" (0.3) — UI, unrelated.
</deferred>

---

*Phase: 28-sandboxed-driver-launcher*
*Context gathered: 2026-10-19*
