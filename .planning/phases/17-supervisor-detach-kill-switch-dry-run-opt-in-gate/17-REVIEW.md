---
phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
reviewed: 2026-07-29T19:45:00Z
depth: standard
files_reviewed: 36
files_reviewed_list:
  - src/action.rs
  - src/app.rs
  - src/cli.rs
  - src/config.rs
  - src/driver/dry_run.rs
  - src/driver/kill.rs
  - src/driver/liveness.rs
  - src/driver/lock.rs
  - src/driver/mod.rs
  - src/driver/reconcile.rs
  - src/driver/run.rs
  - src/driver/spawn.rs
  - src/error.rs
  - src/executor/claude.rs
  - src/executor/mod.rs
  - src/journal/mod.rs
  - src/journal/reader.rs
  - src/journal/writer.rs
  - src/lib.rs
  - src/main.rs
  - src/registry.rs
  - src/state_reader/git_ops.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/help.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/normal.rs
  - tests/driver_dry_run.rs
  - tests/driver_kill.rs
  - tests/driver_lock.rs
  - tests/driver_optin.rs
  - tests/driver_reattach.rs
  - tests/driver_tracer.rs
  - tests/spawn_seam_guard.rs
  - tests/fixtures/fake-claude-cwd.sh
  - tests/fixtures/fake-claude-tripwire.sh
findings:
  critical: 6
  warning: 17
  info: 0
  total: 23
status: issues_found
---

# Phase 17: Code Review Report

**Reviewed:** 2026-07-29T19:45:00Z
**Depth:** standard
**Files Reviewed:** 36
**Status:** issues_found

## Summary

This phase is the milestone's safety contract, so the review weighted the five
properties the phase brief names: the kill switch, the opt-in gate, dry-run
inertness, the `flock`, and the zero-write reconciliation. The dry-run, the lock
and the reconciliation scan hold up under adversarial reading — the `flock` is
held for the run's duration through a guard whose field order releases it after
the last write, the loser path never truncates, the acquire is genuinely
non-blocking, and the scan performs no write and is proven so twice over
(fingerprint plus verb grep). `cargo test` passes end to end and
`cargo clippy -- -D warnings` is clean on the production target.

The **kill switch does not hold up.** Three independent, reachable paths leave a
live agent process group running while the TUI reports the run stopped, gone, or
crashed. Two were reproduced against the built binary rather than argued from
the source:

* **CR-01** — a stop issued while the driver is still inside `Executor::start()`
  is *swallowed*: `tokio::signal::unix::signal` replaces SIGTERM's default
  disposition the moment it is constructed, but nothing polls `term.recv()`
  until the drain loop, ~200 lines later. Reproduced: the driver survived a
  group SIGTERM, and the subsequent SIGKILL (which `kill.rs` sends after its 12s
  grace) left the agent's process group orphaned. This is precisely the CTRL-01
  failure the two-layer design exists to prevent, and it is reachable in the
  single most likely case — the documented CLI startup hang.
* **CR-04** — a run whose `--run-id` is not on the driver's argv (the
  `run_id: None` path that `DriveArgs`, `cli.rs` and `driver/mod.rs` all
  document as supported) is invisible to `liveness::is_run_alive`. Reproduced: a
  live run reads as dead, so `reconcile` reports it crashed and `stop_run`
  returns `AlreadyGone` **without sending a signal**, while the agent runs on.
* **CR-02** — `stop_run` validates `pid` and then signals `pgid`, a separate
  value read out of a `run.json` that lives inside the driven project and is
  therefore writable by the very agent being driven. D-04's `pid == pgid`
  invariant is asserted in docs and tests but never checked at the one call site
  where being wrong means `kill(-pgid, SIGKILL)` against a stranger's group.

Two further blockers are scope/plumbing rather than teardown: the detached spawn
drops the TUI's `--config` (CR-03), and the whole liveness/kill layer is
Linux-only while the driver is gated on `unix`, so on the project's own second
release target the stop reports "already finished; nothing was signalled"
(CR-05).

The tests in this phase are unusually strong on vacuity — nearly every one pins
its own preconditions — but they exercise exactly one shape of run (agent emits
`system/init` immediately, `--run-id` always present, `ReapArm::Adopted`), which
is why every blocker above sits outside their coverage. `ReapArm::Parent` has no
behavioural coverage at all, and none is possible, because the enum has no
behavioural effect (WR-05).

## Structural Findings (fallow)

No `<structural_findings>` block was supplied with this review.

## Narrative Findings (AI reviewer)

### Critical

#### CR-01: SIGTERM is caught but never acted on until the drain loop — a stop during agent startup orphans the agent's process group

**File:** `src/driver/run.rs:258-334` (handler install through `executor.start`), `src/driver/kill.rs:231-266`
**Severity:** BLOCKER

**Issue:** `execute_run` constructs the terminate stream first, "before the group
is established, before the lock, and before a single byte lands on disk". Tokio
registers the OS handler when `signal()` returns, so from that instant SIGTERM
no longer terminates the process. But the first `term.recv()` is inside the
`select!` loop, after `establish_own_group`, `lock::acquire`,
`JournalRun::start`, `executor.start(...).await` and `handle.close_input()`. Any
SIGTERM delivered in that window is buffered and never acted on.

`executor.start()` awaits the capability gate — the first `system/init` from
`claude`. That wait is bounded only by the coordinator's 15-minute idle cap, and
`src/executor/mod.rs:225-239` documents a *reproduced* hook hang that parks the
CLI at exactly this point. So the window is not microseconds; it is up to 15
minutes, in the exact scenario a user is most likely to press stop.

Layer 2 of D-06 (`Executor::cancel`) therefore never runs. `kill.rs` waits out
`DRIVER_TEARDOWN_GRACE`, escalates to SIGKILL against the *driver's* group, and
the `claude` group — a different group, never signalled — survives with its
grandchildren.

Reproduced against `target/debug/gsd-meta-manager` with a stand-in that emits no
`system/init`:

```
driver pid=75516  agent stand-in pid=75538
driver pgid=75516  agent pgid=75538
RESULT-A: driver STILL ALIVE 4s after group SIGTERM -> swallowed
RESULT-B: agent 75538 SURVIVED -> orphaned agent group
```

`tests/driver_kill.rs` cannot see this: `fake-claude-spawner.sh` emits `init`
immediately, so every kill test lands in the drain loop.

**Fix:** Race the terminate signal against everything that can block, not just
the event stream. Minimally, wrap the startup section:

```rust
let handle = tokio::select! {
    biased;
    _ = term.recv() => {
        // Nothing has been spawned yet, or the spawn is in flight; refuse
        // and let the journal record the stop rather than dying to SIGKILL.
        if let Err(err) = run.journal.finish("killed") { /* warn */ }
        return Ok(());
    }
    started = executor.start(&project, args.command.clone(), options) => started,
};
```

Note the spawn-in-flight case needs care: if `start` is cancelled after the child
exists, the group must still be torn down. The safer shape is to run
`executor.start` on a task and keep a teardown handle, or to have `start` itself
take a cancellation token. A stop request arriving before the drain loop must not
be silently dropped in any case.

---

#### CR-02: the kill switch signals a process group id read from a file the driven agent can rewrite

**File:** `src/driver/kill.rs:231-241`, `src/driver/reconcile.rs:147-207`
**Severity:** BLOCKER

**Issue:** `stop_run` does its pid-reuse double-check against `pid`
(`liveness::is_run_alive(pid, run_id)`) and then signals a *different* value,
`pgid`, with no validation of any kind:

```rust
if !liveness::is_run_alive(pid, run_id) { return StopOutcome::AlreadyGone; }
if let Err(err) = signal_group(pgid, Signal::TERM) { ... }
```

`pgid` reaches this function from `ObservedRun.pgid`, which
`reconcile::read_run_facts` reads out of
`<project>/.planning/meta-manager/runs/<id>/run.json` with
`value.get("pgid").and_then(as_u64).unwrap_or(0) as u32`. That file lives inside
the project the autonomous agent is running in, with ordinary write access. So
the agent chooses the process group the user's TUI signals.

`signal_group` refuses only `0` and values that do not fit a `pid_t`. Everything
else is signalled, first SIGTERM and then SIGKILL. The `active` pointer is
equally agent-writable (see WR-02), so the run id and the entire record can be
fabricated; the only remaining requirement is a live process whose
`/proc/<pid>/cmdline` contains `gsd-meta-manager` and `--run-id <id>`, which any
agent can arrange with `exec -a`.

D-04's invariant (`pgid == pid`, established by `setpgid(0,0)`) is asserted in
`tests/driver_tracer.rs` and in the module docs but is never enforced at the one
call site where being wrong is catastrophic — which is exactly what
`kill.rs:168-174` says the danger is.

**Fix:** Do not trust the recorded `pgid`. Either enforce the documented
invariant, or read the group from `/proc` beside the cmdline check:

```rust
// In kill::stop_run, before any signal:
if pgid != pid {
    return StopOutcome::SignalFailed {
        detail: "the recorded process group does not match the recorded pid (D-04)".into(),
    };
}
// Better still, add to liveness.rs and require agreement:
pub fn process_group(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let tail = stat.get(stat.rfind(')')?.checked_add(2)?..)?;
    tail.split_whitespace().nth(2)?.parse().ok()
}
```

Also clamp `read_run_facts`: a `pid`/`pgid` that is absent, `0`, or exceeds
`u32::MAX` should yield `None` (no observable run) rather than `unwrap_or(0) as
u32`, which silently truncates.

---

#### CR-03: the detached spawn drops the TUI's `--config`, so the driver reads a different registry than the user saw

**File:** `src/driver/spawn.rs:36-50` (`drive_argv`), `src/app.rs:903-969`
**Severity:** BLOCKER

**Issue:** `--config` is declared `global = true` in `src/cli.rs:16-17` and
`App` carries `ctx.config_path`, but `drive_argv` emits only
`drive <alias> --command <c> --run-id <id> [--goal <g>]`. The spawned driver
therefore always loads `Config::default_path()`.

Two consequences, both silent because the child's stdio is `/dev/null` and its
`eprintln!` + `exit(1)` goes nowhere:

1. TUI run with `--config /custom.json`: the alias is usually absent from the
   default config, so `drive` refuses with `UnknownAlias`. The TUI has already
   inserted an optimistic `ObservedRun { live: true }` and shown
   "Driving {alias} — run {id}"; the run silently never exists, and the entry
   disappears at the next scan with no error.
2. Worse: if the *default* config happens to carry the same alias pointing at a
   different path with an opt-in record, the agent runs against **a different
   project than the one the user selected** — an autonomous agent with git rights
   in the wrong repository.

`tests/driver_reattach.rs` and `tests/driver_kill.rs` both pass `--config`
explicitly in their hand-built argv, so no test covers the production argv on
this axis; `the_drive_argv_carries_no_development_flag` pins the exact argv and
would need updating with the fix.

**Fix:**

```rust
pub fn drive_argv(
    config_path: &Path,
    alias: &str,
    command: &str,
    run_id: &str,
    goal: Option<&str>,
) -> Vec<OsString> {
    let mut argv = vec![
        OsString::from("--config"),
        config_path.as_os_str().to_owned(),
        OsString::from("drive"),
        // …unchanged…
    ];
    // …
}
```

and pass `self.ctx.config_path` at the call site in `App::start_driver_run`.

---

#### CR-04: a run started without `--run-id` on argv is reported crashed and cannot be stopped

**File:** `src/driver/liveness.rs:63-80`, `src/driver/run.rs:271-276`
**Severity:** BLOCKER

**Issue:** `is_run_alive` requires the *command line* to contain `--run-id`
immediately followed by the run id:

```rust
args.windows(2).any(|w| w[0] == b"--run-id" && w[1] == wanted)
```

But `DriveArgs.run_id` is `Option<String>` and `execute_run` generates one when
it is absent (`driver/mod.rs:92-97` and `cli.rs:53-56` both document this as a
supported mode). The generated id lands in `run.json` and in the `active`
pointer, and never on argv. Every consumer of `is_run_alive` therefore reads the
run as dead:

* `reconcile::reconcile_one` returns `live: false` → the dashboard reports a
  perfectly healthy run as `CrashedWithoutEnding`;
* `kill::stop_run` returns `AlreadyGone` **before sending any signal** — the
  user is told "already finished; nothing was signalled" while an autonomous
  agent keeps running;
* `admit()` does not count it, so the concurrency cap can be exceeded.

Reproduced against the built binary:

```
run_id=2026-07-29T19-38-31Z-f37c  recorded pid=73694
--- driver cmdline:
… drive demo --command /gsd-progress --claude-program … (no --run-id)
CMDLINE HAS NO --run-id  => liveness::is_run_alive() will report this LIVE run as DEAD
```

The same defect bites the documented `--run-id=VALUE` spelling that clap accepts,
because that arrives as one argv element and the `windows(2)` pair never matches.

**Fix:** Either make the run id mandatory on argv, or stop depending on argv for
identity. The cheapest correct fix is both:

```rust
// src/driver/run.rs — re-exec is not needed; just refuse the ambiguous mode.
let run_id = args.run_id.clone().ok_or_else(|| DriveError::…)?;
```

or, preferably, keep the generated id and make `liveness` tolerate both
spellings while adding a second identity source (e.g. compare
`/proc/<pid>/cwd` against the project root, or have the driver write its own
`/proc`-verifiable marker):

```rust
let wanted = run_id.as_bytes();
let flag_eq = |a: &[u8]| a == b"--run-id";
let inline = format!("--run-id={run_id}");
args.iter().any(|a| a == inline.as_bytes())
    || args.windows(2).any(|w| flag_eq(&w[0]) && w[1] == wanted)
```

---

#### CR-05: the kill switch is Linux-only while the driver is gated on `unix` — on macOS a stop reports success and signals nothing

**File:** `src/driver/liveness.rs:14-25`, `src/driver/mod.rs:53-68`
**Severity:** BLOCKER

**Issue:** `liveness` is deliberately not `cfg`-gated, on the rationale that its
`/proc` reads "simply fail off Linux and the functions yield `None`/`false`" —
the "honest failure" posture `session_detector` uses. That posture is honest for
session *detection* (nothing is claimed). It is not honest here, because the same
`false` is consumed as a safety decision:

* `stop_run` → `AlreadyGone`, **no signal sent**, rendered to the user as
  "already finished; nothing was signalled";
* `reconcile_one` → every live run reported `CrashedWithoutEnding`;
* `is_zombie` → always `false`, so criterion #1's zombie half silently passes.

Meanwhile `driver::{run, spawn, kill, lock}` are gated on `unix`, which includes
macOS — and `CLAUDE.md`'s release section names `aarch64-apple-darwin` as a build
target. So on a shipped target the tool will happily spawn a detached autonomous
agent and then be structurally unable to stop it, while telling the user it did.

**Fix:** Make the platform boundary match the implementation. Either gate the
driver on `target_os = "linux"` and return `DriveError::UnsupportedPlatform`
elsewhere (consistent with D-05's stated posture of naming the missing facility),
or give `liveness` an explicit unsupported-platform answer that `stop_run` and
`reconcile` propagate:

```rust
pub enum Liveness { Alive, Dead, Unknown }   // Unknown off Linux

// kill::stop_run
match liveness::probe(pid, run_id) {
    Liveness::Dead => return StopOutcome::AlreadyGone,
    Liveness::Unknown => return StopOutcome::SignalFailed {
        detail: "liveness cannot be determined on this platform".into(),
    },
    Liveness::Alive => {}
}
```

At minimum, `#[cfg(not(target_os = "linux"))] compile_error!` in `driver/mod.rs`
would make the limitation loud instead of silent.

---

#### CR-06: unregistering a project with a live run abandons a running autonomous agent with no stop path

**File:** `src/ui/screens/delete_confirm.rs:69-124`, `src/registry.rs:182-187`
**Severity:** BLOCKER

**Issue:** `do_remove_project` removes the registry entry and then explicitly
drops `ctx.observed_runs.remove(alias)` and the sibling maps (D-27). It never
checks whether that alias has a live run. `reconcile_all` iterates only
`config.projects`, so from the next tick onward the run is invisible; the stop
key (`x`) refuses with "No run is being observed for '{alias}'"; and the driver
keeps its `flock` and keeps driving an autonomous agent with git and push rights
in a project the tool no longer knows about.

The confirmation text actively reassures the user: *"This only unregisters it —
project files are not deleted."* It says nothing about the run it is about to
abandon. There is no recovery path in the TUI — the user must find the pid by
hand. For the phase whose stated contract is "makes an autonomous agent run
stoppable", a one-keystroke path to permanently unstoppable is a defect, not a
scope boundary.

**Fix:** Refuse, or make the consequence explicit and offer the stop:

```rust
fn do_remove_project(ctx: &mut AppContext, alias: &str) {
    if ctx.observed_runs.get(alias).is_some_and(|run| run.live) {
        ctx.error_message = Some(format!(
            "\"{alias}\" has a live driver run. Stop it with `x` first — \
             unregistering it would leave the agent running with no way to stop it"
        ));
        ctx.needs_redraw = true;
        return;
    }
    // …existing body…
}
```

---

### Warnings

#### WR-01: a failed `setpgid` is downgraded to a warning but `pgid` is still recorded as `pid`

**File:** `src/driver/run.rs:118-125`, `src/driver/run.rs:96-97`
**Severity:** WARNING

**Issue:** `establish_own_group` logs and continues on error, and its own doc
says *"`establish_own_group` is what makes the equality honest rather than
assumed"*. But `make_run_record` writes `pgid: std::process::id()`
unconditionally, and `lock::acquire` is handed the same value. If `setpgid` ever
fails, the record asserts a group leadership the process does not have, and the
recorded `pgid` names either nothing (`ESRCH` on stop) or an unrelated group. The
comment claims the invariant is established; the code assumes it.

**Fix:** Read the truth rather than assuming it.

```rust
fn establish_own_group() -> u32 {
    if let Err(err) = rustix::process::setpgid(None, None) {
        tracing::warn!(kind = ?err.kind(), "could not become process group leader");
    }
    rustix::process::getpgrp().as_raw_nonzero().get() as u32
}
```

and thread the returned value into `make_run_record` and `lock::acquire`.

---

#### WR-02: `run_id` is joined into a path with no validation — the run directory can escape the project entirely

**File:** `src/journal/mod.rs:168-178`, `src/journal/writer.rs:449-472`, `src/driver/reconcile.rs:187-193`
**Severity:** WARNING

**Issue:** `run_paths` does `runs_root(planning).join(run_id)` with no check that
`run_id` is a single normal path component. Two routes reach it with untrusted
data:

* **Write side (user):** `--run-id '../../../../escaped'`. Reproduced — the run
  directory and `run.json`/`journal.jsonl` were created outside the project, in a
  directory with no `.gitignore`, violating D-08/SAFE-04's "the protection lands
  before the protected byte":

  ```
  exit=0
  …/scratchpad/escaped/run.json
  …/scratchpad/escaped/journal.jsonl
  ```

* **Read side (agent):** `writer::read_active_run` returns whatever the `active`
  file contains, guarded only by `root.join(run_id).is_dir()` — which a
  traversing path satisfies. `reconcile_one` then reads `run.json` from an
  arbitrary location and `App::schedule_journal_tail` tails an arbitrary file.
  The `active` file is inside the driven project, so an agent controls it.

`journal::classify_change` already gets this right (it rejects any non-`Normal`
component); `run_paths` should too.

**Fix:**

```rust
/// A run id that is safe to use as a directory name.
fn is_plain_component(run_id: &str) -> bool {
    !run_id.is_empty()
        && Path::new(run_id).components().count() == 1
        && matches!(Path::new(run_id).components().next(), Some(Component::Normal(_)))
}
```

Reject in `run_paths`' callers (`JournalRun::start`, `read_active_run`,
`reconcile_one`, `schedule_journal_tail`) — an id that is not a plain component
is a corrupt/hostile record and should yield `None`, not a filesystem operation.

---

#### WR-03: the 12s teardown grace does not budget for `capture_snapshot`'s three git shell-outs, so a stop can be recorded as a crash

**File:** `src/driver/kill.rs:57`, `src/executor/claude.rs:1188-1202`, `src/executor/outcome.rs:60-66`
**Severity:** WARNING

**Issue:** `DRIVER_TEARDOWN_GRACE` is 12s, justified as the `claude` group's 10s
grace "plus teardown slack". The slack budget is wrong: after the grace expires
and `finish_teardown` returns, the coordinator runs
`capture_snapshot(project_root)` — `git rev-parse HEAD`, `git status --porcelain`
and a full `parse_project_state` (which itself shells out to `git log`) — and
only then sends the outcome, after which the driver journals its diagnostic and
its terminal record. On a large repository `git status --porcelain` alone can take
seconds.

If the total exceeds 12s the TUI SIGKILLs the driver mid-teardown. The agent
group is already dead by then (so no orphan), but the terminal record is never
written — and an absent `ended_at` beside a dead pid *is* Phase 16's crash
signal. The user's deliberate stop is then permanently recorded as a crash, and
`ExitedAfterKill`'s message tells them their agent group may be orphaned when it
is not.

**Fix:** Either raise the grace (criterion #1's 15s ceiling leaves room for ~13s
with the 2s `KILL_REAP_BOUND`), or — better — make the escalation conditional on
the *agent* group still being alive rather than on the driver, since the driver
finishing its journal is not what criterion #1 measures. At minimum, assert the
budget explicitly:

```rust
// kill.rs tests
assert!(DRIVER_TEARDOWN_GRACE >= CLAUDE_GROUP_GRACE + SNAPSHOT_BUDGET + JOURNAL_BUDGET);
```

with `SNAPSHOT_BUDGET` mirrored from the executor, the way `CLAUDE_GROUP_GRACE`
already is.

---

#### WR-04: `run.json.claude_code_version` is always the empty string

**File:** `src/driver/run.rs:97-100`, `src/journal/mod.rs:646-648`
**Severity:** WARNING

**Issue:** `make_run_record` writes `claude_code_version: String::new()` with the
comment *"Empty until the first `system/init`. Record what is known; nothing
overwrites it, because `run.json` is written exactly twice."* But write two
happens in `finish`, long after `system/init`, and `ExecutionHandle` carries
`claude_code_version` (`executor/mod.rs:400-402`, added "so Phase 16 can journal
it without a signature change"). `run.rs` reads `handle.pgid` and never
`handle.claude_code_version`, and `JournalRun` has no setter for it.

Verified on a real run:

```json
"claude_code_version": "",
```

The field is dead in every record this phase produces, and the comment
rationalises the omission as if it were a constraint.

**Fix:** Mirror `set_claude_pgid`:

```rust
// src/journal/mod.rs
pub fn set_claude_version(&mut self, version: &str) {
    self.record.claude_code_version = version.to_string();
}
// src/driver/run.rs, beside set_claude_pgid
run.journal.set_claude_version(&handle.claude_code_version);
```

---

#### WR-05: `ReapArm` has no behavioural effect; its only test asserts two strings differ

**File:** `src/driver/kill.rs:78-109`, `src/driver/kill.rs:367-374`
**Severity:** WARNING

**Issue:** `ReapArm` is documented as D-07's two reaping arms, and
`app.rs::stop_driver_run` computes it carefully from `session_spawned_runs`. But
`stop_run` uses it for exactly one thing: a `tracing::warn!` field. Both arms
poll `/proc` identically; nothing branches. The type therefore promises a
behavioural distinction the code does not make, and a reader will reasonably
assume the `Parent` arm reaps.

The only unit test is `both_reaping_arms_name_who_performs_the_wait`, which
asserts `Parent.reaper() != Adopted.reaper()` and
`Adopted.reaper() == "init"` — i.e. it pins two string literals. Every
integration test passes `ReapArm::Adopted`. `ReapArm::Parent` has zero
behavioural coverage, and none is achievable while the arms are identical.

**Fix:** Either delete the parameter (and the `session_spawned_runs` bookkeeping
that feeds it) as the honest expression of "both arms poll `/proc`", or give
`Parent` the property that justifies it — e.g. have the spawn-side reaping task
publish completion through a channel and have `stop_run` await that instead of
polling, with the `Adopted` arm falling back to `/proc`. Whichever is chosen,
the test should assert observed behaviour, not the strings.

---

#### WR-06: `--no-optional-locks` covers only the new dry-run helpers; the live-run snapshot still writes `.git/index`

**File:** `src/state_reader/git_ops.rs:11-32`, `108-153`, `189-203`
**Severity:** WARNING

**Issue:** `git_read_raw` documents `--no-optional-locks` as load-bearing because
`git diff` opportunistically rewrites `.git/index`. The same is true of
`git status --porcelain`, and `is_dirty` does **not** pass the flag. `is_dirty`
and `head_sha` are called from `RunSnapshot::capture`, twice per run (before and
after), inside the repository an autonomous agent is concurrently running git
commands in.

Two consequences: the tool writes to the driven repository during a live run
(the same class of write D-23 forbids in preview mode, just outside its stated
scope), and `git status` takes `.git/index.lock` — so it can fail because the
agent is mid-`git add`, or make the agent's own git command fail. `is_dirty`
returning `None` degrades outcome derivation silently to "inconclusive".

**Fix:** Add the flag to the three older helpers. It costs nothing and makes the
"our reads never write" property uniform:

```rust
std::process::Command::new("git")
    .arg("--no-optional-locks")
    .arg("-C")
    .arg(project_root)
    .args(["status", "--porcelain"])
```

---

#### WR-07: the reconcile "zero writes" verb guard is bypassable by the very helper names used in the same file

**File:** `src/driver/reconcile.rs:434-482`
**Severity:** WARNING

**Issue:** `WRITE_VERB_HALVES` forbids `fs::write`, `File::create`,
`OpenOptions`, `remove_file`, `remove_dir`, `create_dir`, `rename`, `persist`,
`clear_active_pointer`, `prune_runs`. None of those substrings appears in
`writer::create_run_dir`, `writer::write_run_record`,
`writer::write_active_pointer`, `File::options()`, `set_len`, `std::fs::copy` or
`std::fs::set_permissions` — and the test module two dozen lines above calls
three of those writers happily. So a future edit that adds
`writer::write_run_record(&paths, &record)` to production code in this module
passes the guard cleanly.

The guard reads like proof and is not; the byte-identical fingerprint test
(`the_scan_leaves_the_planning_tree_byte_identical`) is the actual protection.
Presenting the weaker check as one of "three ways" it is enforced overstates the
assurance.

**Fix:** Invert the check — forbid the *module paths* that can write rather than
enumerating verbs:

```rust
const FORBIDDEN_PREFIXES: &[&str] = &["writer::", "std::fs::", "fs::", "File::", "tempfile"];
```

with an allowlist for the two read helpers actually used
(`writer::read_active_run`, `std::fs::read_to_string`). Or drop the verb test and
say plainly that the fingerprint test is the guard.

---

#### WR-08: the opt-in toggle's rollback does not restore the original record

**File:** `src/ui/screens/driver_confirm.rs:261-298`
**Severity:** WARNING

**Issue:** When `save_config` fails after *withdrawing* an opt-in, the rollback
calls `registry::record_opt_in`, which mints a **new** record: a fresh
`opted_in_at` timestamp and a freshly-read `claude_md_digest`. The disk still
holds the original record. The doc claims *"Reverting keeps the two in
agreement"*; it does not — they now differ in both fields.

Today the gate only asks "is there a record?", so the divergence is invisible.
Phase 21 will compare `claude_md_digest` for drift, at which point the in-memory
digest is one that was never persisted and the recorded opt-in time is a lie
about when the user consented.

**Fix:** Snapshot and restore the value rather than re-deriving it:

```rust
let previous = ctx.config.projects.get(alias).and_then(|e| e.driver_opt_in.clone());
// …on save failure…
if let Some(entry) = ctx.config.projects.get_mut(alias) {
    entry.driver_opt_in = previous;
}
```

---

#### WR-09: `claude_md_digest` uses FNV-1a, which the phase that consumes it will need to be tamper-evident

**File:** `src/registry.rs:123-140`, `src/journal/mod.rs:211`
**Severity:** WARNING

**Issue:** `claude_md_digest` reuses `journal::argv_digest`, whose own doc says
it is *"explicitly not a security control"*. The stated consumer is Phase 21,
which "re-confirms the opt-in when `CLAUDE.md` drifts" — i.e. it will use this
value to decide whether the instructions an autonomous agent is about to follow
are the ones the user consented to. FNV-1a is trivially collidable, so the agent
that edits `CLAUDE.md` can preserve the digest and defeat the re-confirmation.

Recording a knowingly-unsuitable digest now is what makes it expensive later:
the field is on a user-owned file, and changing the algorithm is the second
migration this phase exists to avoid.

**Fix:** Store a cryptographic digest (or at least a length-prefixed
`blake3`/`sha2` value) and a tag so the algorithm can evolve:

```rust
pub claude_md_digest: Option<String>, // e.g. "sha256:…"
```

If adding a hash dependency is unacceptable in this phase, record the file's
byte length alongside the FNV value and document that Phase 21 must upgrade the
algorithm before relying on it.

---

#### WR-10: `drive`/`execute_run` are `async fn` but perform blocking `flock`, blocking fs, and blocking `git` shell-outs

**File:** `src/driver/mod.rs:124-156`, `src/driver/run.rs:296-316`, `src/driver/dry_run.rs:97-103`
**Severity:** WARNING

**Issue:** `drive` is a public async library function. Inside it:
`dry_run::build_report` runs two synchronous `std::process::Command` git calls;
`lock::acquire` performs a synchronous `flock(2)` plus file writes;
`JournalRun::start` prunes directories and writes files synchronously. None is on
a `spawn_blocking` boundary, and only comments say "the dry-run path is a
foreground CLI invocation".

This repository has already been bitten by exactly this: the header of
`tests/driver_lock.rs:201-215` records a *deadlock* caused by a blocking
`flock` inside an async fn defeating `tokio::time::timeout` on a current-thread
runtime — "observed, not theorised". The comments protecting the current callers
are not a mechanism; any future TUI-side caller (Phase 18 is the obvious one)
reintroduces the hang, and the failure mode is a frozen frame rather than an
error.

**Fix:** Put the blocking sections behind the boundary the rest of the codebase
already uses for exactly this (`App::schedule_reparse`,
`claude.rs::capture_snapshot`):

```rust
let report = tokio::task::spawn_blocking({
    let project = project.clone();
    let command = args.command.clone();
    move || dry_run::build_report(&project, &command)
}).await.expect("the preview task did not panic");
```

and likewise for `lock::acquire` + `JournalRun::start`.

---

#### WR-11: `App::stop_driver_run` returns silently when there is no event channel

**File:** `src/app.rs:1016-1018`
**Severity:** WARNING

**Issue:**

```rust
let Some(tx) = &self.ctx.event_tx else { return; };
```

The comment directly above says *"a stop whose result cannot be reported is a
stop the user cannot tell happened"* — and then returns without setting
`error_message`, without `needs_redraw`, and without the "Stopping {alias}"
status. The sibling path in `driver_confirm::dispatch` does exactly the right
thing for the same condition. A user who presses `x` gets no feedback of any
kind.

**Fix:**

```rust
let Some(tx) = &self.ctx.event_tx else {
    self.ctx.error_message = Some(format!(
        "Cannot dispatch a stop for '{alias}' — the event channel is closed"
    ));
    self.needs_redraw = true;
    return;
};
```

---

#### WR-12: the default drive command is spelled two different ways across the phase and is never validated

**File:** `src/ui/screens/driver_confirm.rs:46`, `tests/driver_dry_run.rs:32`, `CLAUDE.md`
**Severity:** WARNING

**Issue:** `DEFAULT_DRIVE_COMMAND` is `/gsd-progress` (hyphen). The dry-run tests
use `/gsd:progress` (colon), and the project's own `CLAUDE.md` refers throughout
to `/gsd:quick`, `/gsd:execute-phase`, `/gsd:progress`. Nothing in the driver
validates the command string — it is passed verbatim to the agent as a user
message.

If the hyphen form is wrong for the target project, every start this phase ships
burns a real agent turn (and real quota) to produce a no-op, and the only signal
is the run's outcome after the fact. If the colon form is wrong, the dry-run
tests are asserting on a string no run will ever issue. One of the two is a
defect; both cannot be right.

**Fix:** Settle the spelling, use one constant everywhere (including the tests),
and reject an obviously malformed command up front:

```rust
if !args.command.starts_with('/') {
    return Err(DriveError::…);  // a GSD command is a slash command
}
```

---

#### WR-13: nothing prevents an alias or goal that clap will parse as a flag in the child

**File:** `src/registry.rs:13-31`, `src/driver/spawn.rs:36-50`
**Severity:** WARNING

**Issue:** `add_project` validates only non-empty and no-whitespace, so `-x` or
`--config` are accepted aliases. `drive_argv` emits the alias as a bare
positional and the goal as `--goal <value>`. In the child, clap parses
`drive -x --command …` as an unknown flag and exits non-zero — with stdio nulled,
so the user sees the optimistic "Driving -x" entry appear and silently vanish.
A goal beginning with `-` fails the same way once Phase 18 adds the goal input.

**Fix:** Validate at registration (reject a leading `-`), and terminate option
parsing at the spawn seam:

```rust
let mut argv = vec![OsString::from("drive"), OsString::from("--"), OsString::from(alias), …];
```

plus `#[arg(long, allow_hyphen_values = true)]` on `goal` in `cli.rs`.

---

#### WR-14: `RunRecord.target` is persisted as a `Debug` rendering of an enum

**File:** `src/driver/run.rs:84`
**Severity:** WARNING

**Issue:** `target: format!("{:?}", options.target)` writes `"Host"` into a
committed document. `ExecutionTarget` gains a `Container` variant in Phase 22
(`executor/mod.rs:212-223`), and renaming any variant silently changes the
on-disk schema of a file the project doc says is committed and read later. The
in-source test fixture in `reconcile.rs:251` uses `"local"`, so two different
vocabularies already exist for this field.

**Fix:** Give the enum an explicit stable label, the way `SettingSources` and
`PermissionMode` already do:

```rust
impl ExecutionTarget {
    pub fn as_record_label(&self) -> &'static str {
        match self { Self::Host => "host" }
    }
}
```

and assert it in a test so a rename is a build failure, not a data change.

---

#### WR-15: `DriverStopped` drops the observed run even when nothing was stopped

**File:** `src/app.rs:797-807`
**Severity:** WARNING

**Issue:** The handler unconditionally does `observed_runs.remove(&alias)` and
`session_spawned_runs.remove(&run_id)`. But `StopOutcome::SignalFailed` means the
signal was never delivered and `AlreadyGone` means nothing was signalled — in
both cases the run may still be live. The dashboard then shows the project as
having no run for up to five seconds, and the `session_spawned_runs` entry is
gone permanently, so a subsequent stop takes the `Adopted` arm for a run this
session did spawn.

The status message does carry the outcome text, but the map mutation contradicts
it.

**Fix:** Carry the outcome as a value (the `#[cfg(unix)]` obstacle is solvable
with a small portable enum in `action.rs`) and only drop the entry when the run
is actually gone:

```rust
Action::DriverStopped { alias, run_id, outcome, stopped } => {
    if stopped {
        self.ctx.observed_runs.remove(&alias);
        self.ctx.session_spawned_runs.remove(&run_id);
    }
    // …status message either way…
}
```

---

#### WR-16: the hidden `--claude-program` / `--claude-args` development flags ship in release builds

**File:** `src/cli.rs:64-83`, `src/driver/run.rs:313-316`
**Severity:** WARNING

**Issue:** `--claude-program` lets any caller of the released binary make the
"driver" exec an arbitrary program in an opted-in project root, while the journal
records it as a normal GSD run (`run_started`, `exec_started`, `run_ended`,
`outcome`) with no marker distinguishing it from a real agent. `hide = true`
removes it from `--help` but not from the parser.

The rationale given (an env var would be inherited by children) argues correctly
against an env var but does not argue for shipping the flag in release. The spawn
seam is guarded against *emitting* it; nothing guards against a human or a script
passing it.

**Fix:** Gate it, and journal it when used:

```rust
#[cfg(debug_assertions)]
#[arg(long, hide = true)]
claude_program: Option<PathBuf>,
```

or keep it in release but record a `Diagnostic { code: "agent_program_overridden" }`
so a run driven by a stand-in is never mistakable for a real one on disk.

---

#### WR-17: the spawn-seam allowlist matches three spellings and misses several

**File:** `tests/spawn_seam_guard.rs:73`
**Severity:** WARNING

**Issue:** `SPAWN_MARKERS` is `["Command::new(", "CommandWrap::with_new(",
"process_group("]`. A spawn added as `Command::from(...)`, through an aliased
import (`use std::process::Command as Cmd; Cmd::new(...)`), via
`CommandExt::exec`, or through a helper that wraps `Command::new` in another
crate would not be seen. The test's own failure message calls the allowlist the
thing that prevents "an agent … launched against a directory the user never
opted in", which is a stronger claim than the matcher supports.

`calls_marker`'s word-boundary fix is good and its control arm is genuinely
non-vacuous — this is about the marker set, not the matching.

**Fix:** Broaden the markers and add a control arm for each new spelling in
`a_signal_to_a_process_group_is_not_mistaken_for_a_spawn`:

```rust
const SPAWN_MARKERS: &[&str] = &[
    "Command::new(", "Command::from(", "CommandWrap::with_new(",
    "process_group(", ".exec(", "posix_spawn",
];
```

and forbid `use std::process::Command as` / `use tokio::process::Command as`
outright, since an alias is what defeats a literal matcher.

---

_Reviewed: 2026-07-29T19:45:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
