# Phase 17: Supervisor — Detach, Kill Switch, Dry-Run, Opt-In Gate - Context

**Gathered:** 2026-07-29
**Status:** Ready for planning
**Mode:** Autonomous — grey areas proposed and auto-accepted by the orchestrator per user
direction ("best well-reasoned guess is good enough; don't ask questions"). Every decision
below is Claude's Discretion unless tagged otherwise, and is listed explicitly so it can be
corrected at the end. Decisions marked **[LOCKED-BY-RESEARCH]** were settled by the committed
research pass (`.planning/research/{SUMMARY,ARCHITECTURE,PITFALLS,FEATURES}.md`) and are
carried in as given — do not relitigate them during planning. Decisions marked
**[USER-HARD-REQUIREMENT]** were made hard requirements by the user personally when they
chose FULL autonomy including `git push`; they may not degrade into partial implementations.

<domain>
## Phase Boundary

A run is stoppable, survivable, single-instance, previewable, and impossible to start against
a project that did not opt in.

In scope: CTRL-01 (stop any run from the TUI, terminating the whole process tree including
`claude`'s Bash grandchildren), CTRL-02 (dry-run reporting the GSD commands *plus* the
diffstat and push refspecs, executing nothing), CTRL-03 (opt-in enforced at the process-spawn
seam, non-opted-in projects never spawned against), CTRL-04 (a run survives the TUI closing;
the TUI reconciles live runs on restart), CTRL-05 (one driver per project, OS-level lock).

Also in scope, because this phase is where they become reachable and nowhere earlier:

- The `Commands::Drive` subcommand and its dispatch **before** `tui::init()` — the driver
  must never touch ratatui (ARCHITECTURE M8/M9).
- **First production wiring of Phase 15's `ClaudeExecutor` and Phase 16's `JournalRun`.**
  Both are complete and both are currently dead code with no production caller
  (`src/journal/mod.rs:548` says so in a comment). Phase 17 is their first customer. Without
  this, CTRL-04's "reopening the TUI shows that run still live with its current step" has no
  journal to read and is untestable.
- `driver_opt_in` on the registry schema (`RegisteredProject`) plus its migration, in the
  style of the v1.6 QUEUE relocation.
- The `DrivableProject` production constructor — Phase 15 shipped the type with a
  `for_testing` escape hatch only and explicitly deferred the record here.
- Resolution of the three carried-forward open questions: OQ4 (`--worktree`), OQ8 (per-project
  vs per-fleet driver), OQ11 (Unix-only detachment).
- Two cheap Phase 16 carry-forwards whose natural home is this phase's retention/reattach
  work: unbounded `journal_cursors`, and cross-batch sequence-gap blindness.

Out of scope — each has a named later owner:

- **The Driver tab, any rendered driver surface, live stream widget, dashboard badges, the
  injection UI** → Phase 18. This phase adds *one* key binding and *one* confirm affordance at
  most (D-25); it ships no new screen, tab, or widget.
- **`decide()`, the D-R-P-E-V router, multi-command sequences, run bounds, quota park,
  no-progress detection** → Phase 20. Phase 17's driver executes **exactly one** GSD command
  supplied on the command line. There is no loop yet, and no acceptance criterion may assume
  one.
- **Git blast-radius enforcement: push allowlists, `--disallowedTools`, pre-push hooks, secret
  scanning, scoped credentials, branch-prefix rules** → Phase 19. Phase 17's dry-run *reports*
  refspecs; it does not *police* them.
- **LLM goal decomposition, prompt-injection hardening, CLAUDE.md re-confirmation on change**
  → Phase 21. Phase 17 records the CLAUDE.md digest at opt-in (D-14) but takes no action on
  drift.
- **`ExecutionTarget::Container`** → Phase 22.
- **Worktree isolation of the driver's working directory** → Phase 19 (D-21). Phase 17
  resolves OQ4 and hands forward five hard constraints; it does not adopt `--worktree`.
- **Orphan sweeping by `pgrep`/`/proc` cwd scan** (finding `claude` processes no journal
  recorded) → Phase 18. Phase 17 records the precise handle (D-09) that makes a sweep
  unnecessary in every non-SIGKILL case.
- **Fleet-level driving — one goal spanning multiple projects** → deferred past v2.0 by
  REQUIREMENTS. Phase 17 ships the per-project model plus the counting seam a global cap needs
  (D-18).

</domain>

<decisions>
## Implementation Decisions

### Detachment and process shape

- **D-01:** **[LOCKED-BY-RESEARCH]** The driver is a **detached child of the same binary**:
  `gsd-meta-manager drive <alias> --run-id <id> …`, spawned into a **new process group** via
  `std::os::unix::process::CommandExt::process_group(0)`, with all three stdio handles set to
  `Stdio::null()`. Not an in-process tokio task (ARCHITECTURE §5.1 option A — fails "survive
  TUI close" outright), not `systemd-run` (option C — Linux+systemd only, fails portability),
  not tmux (option D — requires tmux running). One binary, one build, one version; the driver
  reuses `state_reader::parse_project_state` verbatim so the dashboard and the driver can
  never drift in how they read a project.

- **D-02:** The TUI spawns the driver through **`tokio::process::Command`** with
  `.kill_on_drop(false)` set **explicitly**, `process_group(0)` applied via `as_std_mut()`, and
  a detached `tokio::spawn`ed task that awaits `child.wait()` purely to reap it. Three reasons
  this shape and not `std::process::Command` fire-and-forget: (a) `kill_on_drop` defaults to
  false but the default is **load-bearing here** — a clean TUI shutdown drops the `Child`, and
  an accidental `kill_on_drop(true)` would silently destroy the entire point of the phase, so
  it is written out and asserted in a test; (b) the reaping task is what stops the TUI
  accumulating a zombie across a multi-hour run while it is still the parent; (c) it gives the
  TUI an in-session completion signal without polling. If the TUI exits first, the driver is
  reparented to init and reaped there — both arms are covered.

- **D-03:** **The run-id is generated by the TUI and passed in via `--run-id`; `run.json` is
  written by the driver.** ARCHITECTURE §5.3 says "write `run.json` before spawning" so a
  spawn failure still leaves a record — that reasoning does not survive the detached design,
  because `run.json` carries the **driver's own** `pid`/`pgid` (Phase 16 `RunRecord`), which
  only the driver knows. Resolution: the TUI owns the id (so it knows what to look for), the
  driver owns the record. A spawn failure is synchronous and reportable in the TUI, and leaves
  genuinely nothing on disk — which is the correct state, not a lost run.

- **D-04:** After `process_group(0)` the driver **is its own group leader, so pgid == pid**.
  No `getpgid` call, no extra dependency. This is the same property Phase 15 already relies on
  at `src/executor/claude.rs:365`. `RunRecord.pid` and `RunRecord.pgid` are both
  `std::process::id()` and a plan that computes them differently is wrong.

- **D-05:** **[LOCKED-BY-RESEARCH — OQ11]** Detachment is **Unix-only, deliberately and
  explicitly**. `process_group` is `std::os::unix`. The whole driver path — the `Drive`
  dispatch arm, the lock, the signal handling — sits behind `#[cfg(unix)]`, matching the
  existing `#[cfg(unix)] pub mod claude;` at `src/executor/mod.rs:27-28`. On non-Unix the
  `Drive` subcommand **still exists and still parses**, and returns a typed
  "driving is not supported on this platform" error naming the reason. It must not be a
  missing subcommand, a `todo!()`, or a silent no-op: REQUIREMENTS already records this as an
  accepted limitation, and an accepted limitation that produces "unknown subcommand" is
  indistinguishable from a bug. The TUI itself stays cross-platform.

### Kill switch (CTRL-01)

- **D-06:** **[USER-HARD-REQUIREMENT]** Stopping is **two-layer**, because there are two
  process groups, not one. Phase 15 spawns `claude` with `ProcessGroup::leader()`, so `claude`
  has its **own** pgid distinct from the driver's. `kill(-driver_pgid, SIGTERM)` therefore does
  **not** reach `claude`. Required sequence:
  1. TUI → `SIGTERM` to the **driver's** process group (by `RunRecord.pgid`).
  2. Driver has a `tokio::signal::unix::signal(SignalKind::terminate())` handler that calls
     the **existing** `Executor::cancel()` — which is Phase 15's already-built
     SIGTERM → 10s grace → SIGKILL → unconditional `wait()` on the `claude` group
     (`src/executor/claude.rs:1491-1549`). **Reuse it. Do not build a second teardown.**
  3. Driver finishes the journal (`JournalRun::finish`), releases the lock, exits.
  4. TUI: grace period → `SIGKILL` to the driver's group → reap.

- **D-07:** Reaping has **two arms and both must be implemented**, because the TUI is only
  sometimes the parent. If the TUI spawned this driver in the current session it is the parent
  and **must** `wait()` after signalling or it leaves a zombie. If the TUI restarted and
  *adopted* the run (D-11), the driver was reparented to init and `wait()` returns `ECHILD`;
  liveness is then confirmed by **re-probing `/proc/<pid>` until it disappears**, not by
  `wait()`. A plan with one arm fails success criterion #1 ("no zombie behind — verified 15s
  later") in exactly the case the phase exists for.

- **D-08:** Signals to a process **group** are sent with
  `rustix::process::kill_process_group`. **Never** shell out to `/bin/kill`, and never write
  `libc::kill(-pgid, …)` by hand. Add `rustix = { version = "1.1", features = ["process",
  "fs"] }` as a **single new direct dependency** covering both this and the `flock` in D-15 —
  verified to resolve to **1.1.4, the exact version already in `Cargo.lock` transitively** via
  ratatui/crossterm, so it adds zero new compilation units to the graph. It is pure Rust with
  no libc linkage, which matches the project's existing "no `libc`, no `nix`" posture (Phase 15
  deliberately avoided both — see the note at `src/executor/claude.rs:102-103`). If the planner
  finds `rustix`'s `flock` API awkward, the fallback is `fs4 = "1.1"` for locking only, keeping
  `rustix` for signalling — but a single crate is preferred and the dry-run add already
  succeeded.

- **D-09:** The driver **journals the `claude` process group id** so a driver killed with
  SIGKILL (skipping D-06 layer 2) does not orphan an untraceable tree. `ExecutionHandle.pgid`
  is already populated by Phase 15 (`src/executor/mod.rs:324-357`); carry it into the
  `exec_started` journal event. This is a **field addition** to a Phase 16 `JournalEvent`
  variant, which is safe by construction: `src/journal/` has no `deny_unknown_fields` anywhere
  (a grepped guard at `src/journal/mod.rs:34-38`), and `JournalRecord` carries unknown fields
  in a flattened `Map` (`src/journal/reader.rs:198-209`). Do not widen `RunRecord` for this —
  that would disturb the exactly-twice write contract.

### Reattachment (CTRL-04)

- **D-10:** **[LOCKED-BY-RESEARCH]** Liveness is a **PID + cmdline double-check**, reusing the
  `/proc` technique already proven in `src/session_detector.rs:83-98`: read
  `/proc/<pid>/cmdline`, split on NUL, and require **both** `gsd-meta-manager` **and** the
  matching `--run-id <run_id>`. The cmdline half is the standard defence against PID reuse and
  is not optional. This is a second consumer of an existing technique, not a new capability.
  Note `session_detector.rs` has **no `#[cfg]` guards at all** — the `/proc` reads simply fail
  and yield empty on non-Linux; the new probe should carry the same honest failure mode and be
  Linux-documented rather than Linux-`cfg`'d, so behaviour matches the module it copies.

- **D-11:** **Reattachment is READ-ONLY by construction, and the UX must say so.** Once the TUI
  exits, the child's stdout pipe is gone — live re-streaming is physically impossible, and
  REQUIREMENTS lists it as out of scope for exactly that reason. The adopted state is
  **"observed"**: journal-tail for history and current step, plus a kill-by-pgid affordance.
  It is **not** "reattached", not "streaming", and the wording in code, docs, and any status
  text must not promise otherwise. Do not promise what physics forbids.

- **D-12:** **Crash reconciliation performs ZERO disk writes.** Absent `ended_at` with a dead
  pid *is* the crash signal — that is Phase 16's stated contract and it is already correct on
  disk. The TUI must not "repair" `run.json` (a third write would break the
  `debug_assert_eq!(self.record_writes, 2)` contract at `src/journal/mod.rs:636-639` and
  destroy the evidence), must not clear the stale `active` pointer, and must not prune. The
  verdict lives in memory and is surfaced; the disk stays as the driver left it. This also
  keeps startup reconciliation trivially safe against non-opted-in projects — it is a read,
  and reads are what v1.x already does.

- **D-13:** The reconciliation scan runs at **TUI startup** next to the existing session scan
  (`src/main.rs:132-135`), and re-probes on the **existing 20-tick block** in the `Action::Tick`
  arm (`src/app.rs:445`, ~5s). ARCHITECTURE §4.4(c) is explicit: reuse the existing counter,
  do not add a second timer. Both go through `spawn_blocking` — `/proc` reads and `run.json`
  reads are sync fs work and the codebase idiom for that is unambiguous.

### Opt-in gate (CTRL-03)

- **D-14:** **[USER-HARD-REQUIREMENT] [LOCKED-BY-RESEARCH]** `driver_opt_in` is a **record,
  not a `bool`**. `RegisteredProject` gains
  `#[serde(default)] pub driver_opt_in: Option<DriverOptIn>` where `DriverOptIn` carries at
  minimum `opted_in_at: String` (RFC3339) and `claude_md_digest: Option<String>`. Three
  reasons it is not a bool: (a) PITFALLS:446 says a bool on the registry entry is the wrong
  answer outright and a capability type costs ~30 lines; (b) a record cannot be accidentally
  `true` — `Some(record)` requires deliberate construction; (c) Phase 21 needs the CLAUDE.md
  digest to re-confirm on drift, and adding it later is a second migration. Phase 17 **records**
  the digest and takes **no action** on drift.

- **D-15 (registry migration):** `#[serde(default)]` on the new field makes every existing
  `config.json` load unchanged — but note `Config.version: u32` has **no** `#[serde(default)]`
  today (`src/config.rs:8-14`), so the schema-version discipline already exists and this phase
  should bump it and add the forward-compatible read path in the style of the v1.6 QUEUE
  relocation at `src/state_reader/queue_md.rs:177-230` (read-side fallback, write-side one-shot
  normalisation, no destructive rewrite of a file the user might still be reading with an older
  binary). The migration must be **idempotent** and must be covered by a test that loads a
  pre-Phase-17 `config.json` byte-for-byte and asserts every project comes back
  not-opted-in — silent enrollment is the single failure FEATURES:236 names as
  "invisible until the agent has already committed".

- **D-16:** **[USER-HARD-REQUIREMENT]** `DrivableProject::from_registry(alias, &RegisteredProject)
  -> Result<DrivableProject, OptInError>` becomes the **only production constructor**, and the
  **`Drive` subcommand handler is the only production caller.** Because the gate lives in the
  *driver*, not the TUI, it holds identically for both entry points — a user typing
  `gsd-meta-manager drive foo` by hand is refused by the same code path as the TUI. That
  property is what makes CTRL-03's "never" literally true and it must be stated in the type's
  doc comment.

- **D-17:** Phase 15's `DrivableProject::for_testing` (`src/executor/mod.rs:135`) is an
  unconditional `pub` constructor that bypasses the gate. It cannot simply be deleted —
  integration tests in `tests/` are separate crates and cannot see `#[cfg(test)]` items — and a
  cargo feature would break bare `cargo test`. Resolution, in two parts: **(a)** rename it to
  something that reads as an alarm at any call site (`for_testing_bypassing_opt_in` or
  equivalent) and mark it `#[doc(hidden)]`; **(b)** add a **mechanical grep-guard test** that
  walks `src/**` and asserts the only occurrences are the definition and its own doc — the same
  technique `src/journal/` already uses to keep `deny_unknown_fields` out
  (`src/journal/mod.rs:34-38`, `reader.rs:7-13`). PITFALLS:521 prescribes exactly this audit:
  "grep for every process-spawn site and confirm each takes a capability type". A comment is
  not a guard; the test is.

- **D-18 (OQ8 — RESOLVED):** **One driver process per project.** Rationale, decided not
  deferred: per-project is what makes the kill switch simple (one pgid, one project, one
  `RunRecord`), it is what `flock` on a per-project path naturally expresses, and REQUIREMENTS
  already defers fleet-level driving past v2.0. The fleet dimension Phase 20 will want is a
  **global concurrency cap**, and this phase ships its seam almost free: the startup/tick
  reconciliation scan (D-13) already enumerates every live run across every registered project,
  so add `Preferences.driver_max_concurrent: usize` (default **1**, per PITFALLS:329) and
  enforce it **at the spawn seam** by counting live runs from that same scan. Phase 20 inherits
  a working cap instead of building one. This is a genuine addition, not scope creep: without
  it, "one driver per project" multiplied across N opted-in projects burns the *shared* 5h/7d
  quota N× — the failure PITFALLS Pitfall 4 names.

### Single-execution lock (CTRL-05)

- **D-19:** **[LOCKED-BY-RESEARCH]** The lock is **`flock(2)`**, on
  `.planning/meta-manager/run.lock`, **never a PID file and never an in-memory flag**
  (PITFALLS Pitfall 9: "A PID-or-flag lock is not a lock"). Advisory locks are
  process-death-safe; a PID file is not.

- **D-20:** Four mechanical properties the lock design must get right, each of which is a way
  the naive version silently fails:
  1. **The driver acquires it, in its own process, after exec** — not the TUI. The TUI can
     exit; the lock must not.
  2. **The `File` handle is held for the run duration.** `flock` is per-open-file-description;
     dropping the `File` releases the lock. Store it in the driver's run state, not a local
     that falls out of scope.
  3. **The holder writes `{pgid, run_id, started_at}` into the file *after* acquiring**, so a
     loser can report **who** holds it (that is success criterion #5's whole content). The
     loser opens **read-only** and must **never** truncate — an `O_TRUNC` on the loser path
     destroys the very information it came to read. A partial or empty read is reported as
     "held by an unknown run", never as "not held".
  4. **The authoritative check is the driver's**, and it is `try_lock` (non-blocking) — a
     blocking acquire turns a duplicate-start into a hang. The TUI may do a cheap advisory
     pre-check for UX, but it is TOCTOU by nature and the driver's refusal is the real gate.
     The driver exits with a typed error the TUI surfaces; "a second run was not started" is
     satisfied because no *run* began, even though a short-lived process did.

### Worktree isolation (OQ4)

- **D-21 (OQ4 — RESOLVED EMPIRICALLY, and DECLINED for this phase):** `-w, --worktree [name]`
  **does exist** on the installed CLI 2.1.220 and **does work under `-p`** — verified by
  execution in a disposable scratch repo during this context pass, exit 0, `result`/`success`.
  Observed semantics (full transcript in Specific Ideas below): the worktree is created at
  **`<repo>/.claude/worktrees/<name>`** on branch **`worktree-<name>`**, `git worktree list`
  reports it **`locked`**, and `system/init.cwd` becomes the worktree path.
  **Phase 17 does not adopt it**, for five concrete reasons found by the probe:
  1. It writes **inside the repo working directory**. `watcher.rs::extract_project_root` walks
     *up* to the nearest `.planning` at arbitrary depth, so a worktree copy of `.planning/`
     resolves the project root to the **worktree**, manufacturing a phantom project and
     mis-routing `FileChanged` and journal classification.
  2. The worktree gets its **own `.planning/`**, so the journal the driver writes (main
     checkout) and the artifacts the agent writes (worktree) diverge — and
     `RunSnapshot::capture(project_root)` (`src/executor/outcome.rs:60`) would fingerprint the
     wrong tree, breaking Phase 15's disk-corroborated outcome derivation.
  3. The branch name `worktree-<name>` is chosen by the CLI and is not configurable, colliding
     head-on with Phase 19's `gsd-auto/**` push-prefix allowlist.
  4. `.claude/` is left **untracked and unignored** in the main checkout (observed
     `?? .claude/` with no `.gitignore` and an empty `.git/info/exclude`) — a driven agent's
     own `git add -A` sweeps the entire worktree into a commit.
  5. The worktree is `locked`, so `git worktree prune` will not clean it; abandoned worktrees
     accumulate and need explicit `git worktree remove --force`.
  Also worth recording: there is **no `worktree` key on the `-p` `system/init` envelope**
  (observed key set enumerated below) — the statusline field ARCHITECTURE inferred the flag
  from is not exposed on this protocol, so a driver could not read the worktree path back even
  if it wanted to. **Owner: Phase 19**, which owns blast radius and branch policy; the safe
  fallback (`git worktree add` at a path **outside** the repo, driver cwd pointed at it) is
  strictly better on all five counts and remains available.

### Dry-run (CTRL-02)

- **D-22:** **[USER-HARD-REQUIREMENT] [LOCKED-BY-RESEARCH]** Dry-run must be **load-bearing**:
  PITFALLS:63 — *"A dry-run that prints 'would run: /gsd:execute-phase' tells the user nothing
  about blast radius"*, and PITFALLS:69 lists "the dry-run output does not include a refspec
  list" as the warning sign. `gsd-meta-manager drive <alias> --dry-run` emits **three** things
  and a plan that emits fewer has not delivered CTRL-02:
  1. **The GSD command sequence it would issue.** In Phase 17 the router does not exist
     (Phase 20), so the sequence is the single `--command` argument. That is the *complete and
     honest* sequence for this phase, and the output should say so rather than implying a
     sequence exists that it cannot compute.
  2. **The diffstat.** You cannot know the diff of a command you have not run — so what is
     reported is the **working-tree state the run would inherit and could commit**:
     `git diff --stat HEAD` plus the untracked-file list. That is the real, honest, computable
     blast radius, and it answers the question the user actually has ("if this run does
     `git add -A && git commit`, what goes in?").
  3. **The push refspec list**, computed **locally** from `branch.<b>.remote`,
     `branch.<b>.merge`, `remote.<r>.push`, and `push.default`, plus the resolved remote URL —
     rendered as the exact `refs/heads/<src>:refs/heads/<dst>` lines a `git push` from this
     state would produce. Local computation only: **never** `git push --dry-run`, which
     contacts the network and needs credentials, making the preview non-deterministic and
     untestable in CI.

- **D-23:** Dry-run performs **zero git writes and zero `claude` spawns**, and that is proved
  mechanically, not asserted. PITFALLS:520 names the test: *"check `git reflog` before/after"*.
  The acceptance test snapshots `git reflog`, every ref, and the `.git` directory listing
  before and after a dry-run and asserts equality. It must also assert the executor was never
  invoked — cheapest reachable form is asserting no `run.json`/journal was created and no
  `claude` process appeared, both of which are observable without a real subscription.

- **D-24:** Dry-run is a **CLI mode, not a UI mode** (ARCHITECTURE §5.2): `gsd-meta-manager
  drive <alias> --dry-run` is a plain invocation, unit-testable and CI-runnable, which
  satisfies the hard requirement **by construction** rather than as a rendering path. Its
  output goes to **stdout as human-readable text**, not to the journal and not to the TUI —
  the driver's stdio is `Stdio::null()` when detached (D-01), so a dry-run is by definition a
  foreground invocation and stdout is available. The TUI surfacing dry-run output is Phase 18's.

### UI surface (deliberately minimal)

- **D-25:** This phase's UI surface is **one key binding and one confirmation**, and nothing
  else. Phase 18 owns the Driver tab and every rendered driver surface, and the ROADMAP says so
  twice. What Phase 17 needs to reach its own success criteria from the TUI is: a key that
  starts a run on the selected project (routed through the opt-in gate, refusing visibly when
  not opted in), a key that stops a live run, and an opt-in toggle. Prefer reusing the existing
  status-message and confirmation idioms over introducing any new widget. **[LOCKED-BY-RESEARCH]**
  Driver state lives in a **sibling map on `AppContext`, never on `ProjectState`** — that type
  derives `PartialEq` and `app.rs` uses the equality to suppress "Updated: {alias}" status spam,
  a deliberate v1.4 feature that driver state changing every few seconds would defeat for an
  entire multi-hour run (ARCHITECTURE AP1; Phase 15 D-19; Phase 16 already established
  `run_states` and `journal_cursors` as the precedent).

- **D-26:** The opt-in **disclosure** UX that PITFALLS:511 describes (list the files that will
  enter prompts, require typing the project name) is **Phase 18's**, not this phase's. Phase 17
  ships the *record*, the *type*, the *migration*, and the *enforcement*; a bare toggle with a
  clear confirmation is sufficient here and must not block on a richer flow that has no screen
  to live in yet.

### Phase 16 carry-forwards folded in

- **D-27:** **`journal_cursors` is never pruned** (`src/ui/screens/mod.rs:170`, keyed
  `(alias, run_id)`; inserted at `src/app.rs:686`; zero removals anywhere in `src/`). It grows
  unbounded for the process lifetime. Retention is this phase's territory, so fix it here:
  drop cursors whose alias is no longer registered (note `registry::remove_project`,
  `src/registry.rs:76`, touches neither `journal_cursors` nor `run_states` today — both leak),
  and retain at most the newest few run-ids per alias, aligned with `RETAIN_RUNS = 10`
  (`src/journal/mod.rs:115`). Prune on the same 20-tick block as D-13; do not add a timer.

- **D-28:** **Sequence-gap detection is blind across tail reads.** `seq_gaps`
  (`src/journal/reader.rs:263`) only compares within one batch, and `ReadDiagnostics.last_seq`
  is returned but never stored — so a gap that straddles two tail reads is invisible. Fix by
  storing the last observed `seq` **alongside** the byte cursor: widen the `journal_cursors`
  value from a bare `TailCursor` to a small struct carrying `{cursor, last_seq}`, and seed the
  next batch's gap check from it. Note `src/app.rs:673-684` **re-implements** the same
  `windows(2)` filter inline instead of calling `seq_gaps`, and logs only a count — collapse
  the duplicate onto the shared function while making this change.

### Claude's Discretion

- Plan granularity and wave structure.
- Module layout for the new code — `src/driver/supervisor.rs` per ARCHITECTURE N11 vs a
  flatter placement. ARCHITECTURE's N6-N11 table assumes a full `src/driver/` tree that mostly
  belongs to Phases 20/21; do not create empty modules for later phases.
- Exact `DriverOptIn` field set beyond the two required in D-14.
- Whether the lock file lives at `.planning/meta-manager/run.lock` (PITFALLS' literal
  suggestion) or under `meta-manager/runs/` next to `active` — note `RUNS_SUBDIR` is
  `"meta-manager/runs"` (`src/journal/mod.rs:89`) and the runs dir is gitignored by
  `RUNS_GITIGNORE_BODY`, which is an argument for putting the lock there so it is never
  committed. Decide and record.
- Concrete grace-period value for the TUI→driver SIGTERM→SIGKILL step (D-06 step 4). Phase 15
  uses 10s for the `claude` group; the driver needs *at least* that plus teardown slack, so a
  larger value is likely right. Pick a defensible number, make it a named constant, and note
  that success criterion #1 verifies at 15s — the two must be consistent.
- Exact test names and file placement, following the existing `#[cfg(test)] mod tests`
  convention and the `tests/*.rs` integration-test precedent
  (`executor_lifecycle.rs`, `journal_crash.rs` are the closest analogues).
- Whether `rustix` covers both jobs or splits with `fs4` (D-08).

</decisions>

<code_context>
## Existing Code Insights

Every anchor below was read directly during this context pass against the current tree.

**The capability token — Phase 15 left the seam, this phase supplies the record**
- `src/executor/mod.rs:123-151` — `pub struct DrivableProject { alias: String, root: PathBuf }`,
  private fields, `Debug + Clone + PartialEq + Eq`. Accessors `alias()` `:143`, `root()` `:148`.
- `src/executor/mod.rs:135` — `pub fn for_testing(alias, root) -> Self` is the **only**
  constructor. Doc at `:117-122` states Phase 17 supplies the production one.
- **All 14 uses are in `tests/executor_lifecycle.rs` and `tests/executor_transport.rs`.
  Production call sites: zero.** `grep driver_opt_in src/` returns only that doc comment.
- Functions taking it: `Executor::start` (`mod.rs:74-79`), `ClaudeExecutor::start_run`
  (`claude.rs:308-313`), `<ClaudeExecutor as Executor>::start` (`claude.rs:456-463`).

**Phase 15's teardown — REUSE, do not rebuild (D-06)**
- `src/executor/claude.rs:104` `const SIGTERM: i32 = 15;` — `:107` `TEARDOWN_GRACE = 10s`.
- `claude.rs:1491` `fn terminate_group(child: &dyn ChildWrapper)` → `child.signal(SIGTERM)`.
- `claude.rs:1522` `tear_down_group` → `terminate_group` then `finish_teardown`.
- `claude.rs:1532` `finish_teardown(child, grace)` → bounded `wait()` → `start_kill()`
  (SIGKILL) → **unconditional unbounded `wait()`** at `:1541`. That final `wait()` is what
  prevents zombies and must not be raced.
- `claude.rs:351` `wrap.wrap(ProcessGroup::leader());` + `:353` `KillOnDrop` as backstop only.
  `claude.rs:365` records pgid from `child.id()` — child is group leader so pgid == pid.
- `claude.rs:102-103` documents the deliberate no-`nix`/no-`libc` posture: `ChildWrapper::signal`
  takes an `i32`. D-08's `rustix` choice is consistent with this, not a reversal.
- **`process-wrap`'s `start_kill()`/`kill()` send SIGKILL**, so any SIGTERM path must call
  `signal(15)` explicitly — Phase 15 already does. `signal(&self)` and `wait(&mut self)` cannot
  be used concurrently.

**Phase 16's journal — the reattach inputs are already on disk**
- `src/journal/mod.rs:461-497` `pub struct RunRecord` — `run_id, goal, gsd_command, target,
  opt_in: Option<String>, started_at, session_id, pid: u32, pgid: u32, claude_code_version,
  argv_digest, ended_at: Option<String>, outcome: Option<String>`. **`opt_in` already exists and
  is `None` today — Phase 16 wrote the field specifically to save Phase 17 a migration.**
  **There is no alias or project-path field**; the project is implied by which `.planning/` tree
  the run dir sits in.
- Layout: `RUNS_SUBDIR = "meta-manager/runs"` (`mod.rs:89`) — so
  `<planning>/meta-manager/runs/{.gitignore, active, <run-id>/{run.json, journal.jsonl}}`.
  `pub struct RunPaths` at `mod.rs:131-143`.
- Write points: `JournalRun::start` → `mod.rs:591`; `JournalRun::finish` → `mod.rs:634`, with
  `debug_assert_eq!(self.record_writes, 2)` at `:636-639`. D-12 depends on this.
- `writer.rs:427` `read_active_run(planning_dir) -> Option<String>`; `writer.rs:529`
  `has_end_timestamp` reads `ended_at` schema-agnostically as a raw `Value`.
- `writer.rs:565` `prune_runs(planning_dir, retain, active_run_id)` — never prunes a run whose
  `ended_at` is absent. Called only from `JournalRun::start` (`mod.rs:588`).
- `RUNS_GITIGNORE_BODY` at `writer.rs:237-244` — write-only-if-absent, ignores everything except
  `.gitignore` and `*/run.json`.
- **Nothing in `src/` outside `src/journal/` calls `JournalRun::start`, `create_run_dir`, or
  `new_run_id`.** `src/journal/mod.rs:548`: *"no code in this repository spawns a
  `ClaudeExecutor` yet; wiring a real one is Phase 17's"*. This phase is the first producer.

**The `/proc` technique to reuse (D-10)**
- `src/session_detector.rs:83` `read_session_id(pid)` — reads `/proc/<pid>/cmdline`, splits on
  NUL, scans `windows(2)` for `--resume <value>`. The exact shape D-10 needs for `--run-id`.
- `session_detector.rs:50` — `read_link(/proc/<pid>/cwd)` is the only *required* read; a failed
  read drops the pid via `filter_map` at `:27`. That is the liveness idiom.
- `session_detector.rs` has **no `cfg` guards** except `#[cfg(test)]`; callers are
  `src/main.rs:132` (startup) and `src/app.rs:445` (20-tick block, ~5s, via `spawn_blocking`).

**CLI and dispatch (D-01, D-05)**
- `src/cli.rs` is 35 lines: `Cli { command: Option<Commands>, config: Option<PathBuf> }`,
  `Commands { Add, Remove, List }`. No driver subcommand.
- `src/main.rs:33` — a single `match cli.command`; `None =>` is the TUI arm at `:81-156`,
  `tui::init()` at `:83`. **`Drive` must dispatch in this match, before `:83`.**
- `src/main.rs:103-104` — the bounded exec channel (`EXEC_CHANNEL_CAPACITY = 8192`) already
  exists; `app.ctx.exec_tx` set at `:108`. **No production code sends into it yet** — the only
  senders are `src/main_loop.rs` tests. Phase 17's driver is not the sender either (it is a
  different process); the TUI-side sender, if any, is Phase 18's.
- `src/main_loop.rs:130` `pub async fn pump(...)` with `biased` select and `EXEC_BATCH = 64`;
  `App::apply_exec_event` at `src/app.rs:198-224` writes `ctx.run_states`.

**Registry and config (D-14, D-15)**
- **There is no `Registry` struct.** `src/registry.rs` is free functions over `&mut Config`:
  `add_project` `:13`, `add_project_unchecked` `:50`, `remove_project` `:76`, `list_projects`
  `:84`, `auto_register_from_sessions` `:132`. None persist; callers call `save_config`.
- `src/config.rs:8-14` `pub struct Config { version: u32, projects: HashMap<String,
  RegisteredProject>, #[serde(default)] preferences: Preferences }`. `version` has **no**
  `#[serde(default)]`, so a config lacking it fails to parse. `Config::new()` hardcodes `1`
  (`:50-56`) and **no code branches on it**.
- `src/config.rs:16-20` `pub struct RegisteredProject { path: PathBuf, added: String }` — **no
  serde attributes on either field.** This is D-14's target.
- `src/config.rs:72-87` `save_config` — `NamedTempFile::new_in(dir)` → `to_string_pretty` →
  `persist`. The atomic idiom.
- Migration precedent (v1.6 QUEUE relocation): `src/state_reader/queue_md.rs:177-181`
  `queue_paths` (canonical `meta-manager/QUEUE.md`, legacy root), `:189-198` `load_queue`
  read-side fallback, `:207-230` `save_queue` write-side one-shot with
  `let _ = std::fs::remove_file(&legacy)` at `:228`. Note it uses a **fixed** `QUEUE.md.tmp`
  name, which `src/journal/writer.rs:474-478` explicitly documents as an idiom **not** to copy
  (collision risk under a race) — follow `config.rs`/`writer.rs`, not `queue_md.rs`, for the
  atomic write.
- `auto_register_from_sessions` (`registry.rs:132`) is the silent-enrollment hazard
  FEATURES:236 names: discovery may **register**, but driving requires a separate explicit
  persisted opt-in. Two different flags — verify the new field is never set by this path.

**Git helpers for dry-run (D-22)**
- `src/state_reader/git_ops.rs:108` `head_sha(project_root) -> Option<String>`, `:139`
  `is_dirty(project_root) -> Option<bool>`. Both shell out to `git`. This is where the
  diffstat/refspec helpers belong — extend this module, do not start a second git wrapper.
- `src/executor/outcome.rs:60` `RunSnapshot::capture(project_root)` documents that it
  *"shells out to git twice"* and **must run on a blocking thread**. Same discipline applies to
  every new git helper.
- Other existing `git` shell-outs: `src/journal/writer.rs:323` (`git check-ignore -v --`,
  inspects the **reported pattern**, not the exit status), `src/project_creator.rs:47`.

**Dependencies and toolchain**
- `Cargo.toml`: version `1.6.0`, edition 2021, **`rust-version = "1.87"`**. Deps include
  `tokio = { version = "1", features = ["full"] }` — **`signal` is already available**, so
  D-06's SIGTERM handler adds no dependency. `process-wrap 9.1.0` (`tokio1`), `uuid 1.24`,
  `tempfile 3`, `chrono 0.4`. `[dev-dependencies]` is **`assert_fs = "1"` only**.
- **No `nix`, `libc`, `fs4`, `fs2`, or `flock` anywhere in `src/` or `Cargo.toml`. No file
  locking of any kind exists in the repo.** `rustix 1.1.4` is already in `Cargo.lock`
  transitively; `cargo add --dry-run rustix --features process,fs` resolves to exactly
  **1.1.4** — verified during this context pass.
- `Cargo.toml:39-40` records a deliberate no-`tokio-util` decision; keep new deps to the one in
  D-08 and record the reasoning the same way.

**Project gates**
- `cargo build && cargo test && cargo clippy -- -D warnings` (lib target) must pass clean.
- `cargo clippy --all-targets -- -D warnings` has **exactly 5 pre-existing lints** —
  3× `browser.rs`, 1× `project_creator.rs`, 1× `state_reader/mod.rs`. Not this phase's to fix;
  **the count must not grow.**
- **Baseline at phase start: 427 tests passing** (366 lib + 11 `executor_lifecycle` +
  7 `executor_transport` + 3 `journal_crash` + 3 `journal_gitignore` + 12 `registry_test` +
  25 `state_reader_test`), measured this pass.
- **`rtk` filters cargo output**: raw `warning:` and `test result:` lines are absent from normal
  `cargo` invocations, so any acceptance criterion that greps for them passes **vacuously**.
  Use `rtk proxy cargo …` wherever a criterion needs raw output. This bit Phase 15.

</code_context>

<specifics>
## Specific Ideas

### OQ4 spike result, executed during this context pass

Run in a disposable scratch git repo (never this repo, never a registered project), against
the installed CLI **2.1.220**:

```
$ claude -p --worktree oq4probe --output-format stream-json --verbose \
         --setting-sources project 'Reply with exactly: OK'
exit=0     (stderr empty)
envelopes: system/init → rate_limit_event → assistant → result/success
```

```
$ git worktree list
<repo>                                    f33c92f [main]
<repo>/.claude/worktrees/oq4probe         f33c92f [worktree-oq4probe]  locked

$ git status --porcelain
?? .claude/          ← untracked AND unignored; no .gitignore, .git/info/exclude empty
```

`system/init.cwd` = `<repo>/.claude/worktrees/oq4probe`. Full observed init key set:

```
agents, analytics_disabled, apiKeySource, capabilities, claude_code_version, cwd,
fast_mode_disabled_reason, fast_mode_state, mcp_servers, memory_paths, model, output_style,
permissionMode, plugins, product_feedback_disabled, session_id, skills, slash_commands,
tools, type, uuid
```

— **no `worktree` key**. `capabilities` were the same three Phase 15 recorded
(`interrupt_receipt_v1`, `interrupt_cancel_queued_v1`, `msg_lifecycle_v1`);
`apiKeySource: "none"` (subscription path alive, D-08's Phase-15 guard still valid);
`claude_code_version: "2.1.220"`.

Also confirmed present in `--help`: `--tmux  Create a tmux session for the worktree (requires
--worktree)`. Not relevant to this phase but it confirms `--worktree` is a first-class,
documented flag rather than an artifact.

**Verdict:** OQ4 answered — the flag exists and works, but its placement inside the repo and
its CLI-chosen branch name make it wrong for this codebase. See D-21. Phase 19 inherits the
decision with the fallback (`git worktree add` outside the repo) already identified as
strictly better.

### Success criteria, and how each is reachable without breaking the scope fence

Two Phase 15 criteria were written so they could only be satisfied by doing out-of-scope work
and had to be rewritten mid-phase. Each of this phase's five was checked against its own fence:

1. *"Pressing stop … leaves no `claude` process, no grandchild build or server process, and no
   zombie behind — verified 15s later."* Reachable: Phase 15's fake-`claude` fixture already
   exists (`tests/fixtures/`, used by `executor_lifecycle.rs`) and can spawn a real grandchild.
   No subscription needed. Verification is `pgrep`/`/proc` plus a `Z`-state check, both already
   in the codebase. **Watch:** the 15s figure must exceed the D-06 step-4 grace constant.
2. *"Closing the TUI mid-run leaves the run going; reopening the TUI shows that run still live
   with its current step."* Reachable **only because** this phase wires `JournalRun` into
   production (in scope, stated in the boundary). "Current step" = the last journal record, not
   a live stream (D-11). If a plan leaves the journal unwired, this criterion is unreachable.
3. *"A dry-run reports the GSD commands it would issue plus the diffstat and push refspecs …
   and performs zero git writes."* Reachable and fully local: D-22's three outputs are all
   computable from `git` reads, and D-23's reflog-equality check is the proof. **Watch:** does
   not require the router (Phase 20) — the "sequence" is the one `--command` argument.
4. *"A run cannot be started against a project that has not opted in, and a live run against
   one project provably performs no write, spawn, or git operation against any other registered
   project."* Reachable: register two projects, opt in only A, drive A with the fake fixture,
   hash B's whole tree before and after and assert byte-identical, and assert no process has a
   cwd under B. **Watch:** the "no other project" half is a *test*, not a mechanism — do not
   let it grow into a sandbox feature.
5. *"A second attempt to drive the same project reports which run holds the lock instead of
   starting a second one."* Reachable: D-20's holder-metadata-in-the-lock-file is what makes
   "which run" answerable at all. **Watch:** a design where the loser truncates the file, or
   where the TUI is the lock holder, cannot satisfy this.

### Concrete shapes worth fixing early

The `Drive` argument surface (ARCHITECTURE M8, narrowed to this phase's scope):

```
gsd-meta-manager drive <alias>
    --command <gsd-command>     # exactly one; the router is Phase 20
    --run-id <id>               # supplied by the TUI (D-03); generated if absent
    --dry-run                   # D-22/D-23
    [--goal <text>]             # recorded into RunRecord.goal; not interpreted (Phase 21)
```

The dry-run output is a **contract**, so pin its shape in a test with a golden fixture rather
than eyeballing it — a dry-run whose refspec section silently disappears is exactly the
regression PITFALLS:69 warns about, and it is invisible to a smoke test.

### Things that will look done but are not

Adapted from PITFALLS' own "Looks Done But Isn't" checklist, filtered to this phase:

- **Kill switch:** stops the driver but not `claude`. The tell is `pgrep -x claude` returning
  processes after a stop. D-06 layer 2 is the fix and it is easy to omit because layer 1 alone
  *looks* like it works.
- **Dry-run:** logs commands only. The tell is no refspec section in the output.
- **Opt-in:** a bool checked at one call site. The tell is `grep` finding a spawn site that
  does not take `DrivableProject`. D-17's grep-guard is the mechanical answer.
- **Lock:** a PID file, or an in-memory flag, or the TUI holding it.
- **Reattach:** promising live output after a restart. Physically impossible (D-11).
- **`kill_on_drop(true)`** appearing anywhere near the detached spawn — silently defeats the
  entire phase on a clean TUI shutdown.

</specifics>

<deferred>
## Deferred Ideas

Each of these was considered during this pass and deliberately pushed out, with its owner
named:

- **The Driver tab, live stream render, ring-buffered output, three-state injection UI,
  dashboard driver badges, surfacing dry-run output in the TUI** → Phase 18. Phase 17 ships at
  most one key binding and one confirmation (D-25).
- **The rich opt-in disclosure flow** (list every file that will enter prompts, require typing
  the project name, PITFALLS:511) → Phase 18, once there is a screen to host it. Phase 17 ships
  the record and the enforcement.
- **Worktree isolation of the driver's cwd** → Phase 19 (D-21), with the five constraints the
  OQ4 probe found attached.
- **Push allowlists, `--disallowedTools` git denylist, pre-push hooks, secret scanning,
  per-run scoped credentials, forbidding `git stash`** → Phase 19. Phase 17's dry-run reports
  refspecs; it does not enforce a policy over them.
- **`decide()`, the D-R-P-E-V router, lifting `derive_all_stage_statuses` out of
  `ui/screens/detail.rs`, no-progress and repeat-command detectors, run-level step and
  wall-clock caps, quota-floor park** → Phase 20. Phase 17's driver runs exactly one command.
- **Enforcing the global concurrency cap beyond a simple count at the spawn seam** → Phase 20.
  Phase 17 ships the preference and the count (D-18); Phase 20 owns the policy around it.
- **Human-collision detection before each iteration** (dirty tree the driver did not create,
  `index.lock` present, a live `claude` session in that directory — PITFALLS Pitfall 9) →
  Phase 20, which is where "before each iteration" first means something. `session_detector.rs`
  already provides the input.
- **CLAUDE.md drift re-confirmation** → Phase 21. Phase 17 records the digest (D-14) and acts on
  nothing.
- **Orphan sweeping via `pgrep -x claude` + `/proc/<pid>/cwd` under the project root** →
  Phase 18. D-09's journaled `claude` pgid covers the precise case; the sweep is a
  belt-and-braces for a SIGKILL'd driver.
- **The optional `control.sock` fast path and `inbox.jsonl` durable injection** → Phase 18
  (inbox) and a later milestone (socket), per REQUIREMENTS' "Future Requirements".
- **`ExecutionTarget::Container`, `PathMap`, bollard, podman (OQ5)** → Phase 22.
- **Replacing `session_detector.rs`'s pgrep+`/proc` with `claude agents --json`** → deferred
  past v2.0 by REQUIREMENTS as a v1.x subsystem refactor.
- **Fixing the 5 pre-existing `--all-targets` clippy lints** — unrelated; the count must not
  grow but is not this phase's to shrink.
- **`session_detector.rs`'s `pts/1` vs `pts/11` TTY substring match** — a real bug carried
  forward from Phase 15's deferred list; belongs to the tmux attach path, not the supervisor.
  Still a quick task.
- **Fixing the `pending_editor` blocking shell-out on the render thread** — Phase 15 D-18's
  known wart. Still not this phase's.

</deferred>
