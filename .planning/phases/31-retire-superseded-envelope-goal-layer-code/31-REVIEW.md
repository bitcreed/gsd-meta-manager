---
phase: 31-retire-superseded-envelope-goal-layer-code
reviewed: 2026-10-29T00:00:00Z
depth: standard
files_reviewed: 22
files_reviewed_list:
  - docs/CONFIGURATION.md
  - docs/DRIVER-SANDBOX.md
  - docs/GSD-CORE-SYNC.md
  - src/agents/adapters/mod.rs
  - src/config.rs
  - src/driver/direction.rs
  - src/driver/lock.rs
  - src/driverctl/serve.rs
  - src/driverctl/store.rs
  - src/executor/mod.rs
  - src/launcher/config_dir.rs
  - src/launcher/process.rs
  - src/launcher/run.rs
  - src/launcher/stop.rs
  - src/registry.rs
  - src/state_reader/config_json.rs
  - src/state_reader/driver_frontmatter.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/mod.rs
  - tests/driverctl_serve.rs
  - tests/driverctl_store.rs
  - tests/launcher_config_dir.rs
findings:
  critical: 0
  warning: 3
  info: 3
  total: 6
status: issues_found
---

# Phase 31: Code Review Report (re-review after quick 261028-t3c, 261028-v2z, 261029-1dp)

**Reviewed:** 2026-10-29
**Depth:** standard
**Files Reviewed:** 22
**Status:** issues_found

## Summary

Reviewed the diff `f3e0464..HEAD` for the listed files, reading the full files around each hunk. Areas covered: the shared guarded lock-file open (`open_lock_file`, `ensure_lock_file`, `probe_at`, `read_holder_at`), the shared `exchange_or_restore` put-back helper in `direction.rs`, the config_dir wrong-shape host-policy filter, the guarded `state.json` reads (store, stop, wall-clock pre-check), the SAFE-04 redacting capture pump, and the `AgentRuntime` removal.

No blocker found. I traced the following and found them sound:

- `exchange_or_restore` is bounded (inner `GIVE_BACK_ROUNDS`, outer `SPLICE_TRIES`). The `Restored => continue` path in `run_loop` re-reads a fresh save and cannot spin.
- `open_lock_file`: `O_NOFOLLOW | O_NONBLOCK`, type and link-count check on the handle. The FIFO case never parks the caller. A write-side read+write open reaches the type check for a FIFO.
- The pump reaches EOF correctly: `drop(cmd)` releases the parent's copies of the write end.
- The pump keeps draining after a write error.
- Pump line cap and note are consistent with `summarize_stream_file`'s skip rule.
- The JSON tree walk preserves `type`, `result` and `is_error`, so the outcome reader is unaffected.
- All callers of the removed `AgentRuntime` API are gone. `grep` finds only doc comments.
- The `is_opted_in` doc claim (feeds only `push::policy_for`) is true: no launcher code reads `driver_opt_in` except `policy_for`.

Remaining concerns are one redaction gap in the new pump, one resource-leak edge, one silent policy drop, and some stale documentation.

## Warnings

### WR-01: SAFE-04 pump does not redact a multi-line PEM private key in non-JSON output

**File:** `src/launcher/process.rs:146-181` (`pump_redacted`, `redact_line`)
**Issue:** The pump redacts one line at a time. `src/journal/redact.rs:108` has a PEM rule, `-----BEGIN ... PRIVATE KEY-----(?s:.)*?-----END ...-----`, that only fires when BEGIN and END are in the same string. Inside a stream-json line this holds, because the key is a JSON string with `\n` escapes and `redact_value` sees it whole. But stderr from the child, and any non-JSON stdout line, is handled by the text path, one physical line at a time. A key printed as raw text (for example `cat key.pem` echoed by a wrapper script or a crash dump) spans several physical lines. The BEGIN line, base64 body lines and END line each fail the regex alone, so the key body reaches `launch-N.jsonl` verbatim. This is the exact case the pump claims to close ("a credential-shaped token the master prints never reaches the file"), and stdout and stderr share one pipe.
**Fix:** Give the text path a small state machine in `pump_redacted`. When a text line matches `-----BEGIN [A-Z ]*PRIVATE KEY-----` with no END on the same line, write `[REDACTED:private-key]` once and discard lines until one contains `-----END [A-Z ]*PRIVATE KEY-----`. Cap the discard by the number of lines or bytes, and fail closed (keep discarding) at EOF. Add a test with a three-line PEM block fed through `pump_chunks`.

### WR-02: `spawn_launch_child` leaks a running child when the pump thread cannot be spawned

**File:** `src/launcher/process.rs:116-129`
**Issue:** `cmd.spawn()` succeeds and `child` is live, then `std::thread::Builder::new().spawn(..)?` can fail (EAGAIN or thread limit). The `?` returns `Err` while the driver (a process-group leader, possibly inside a `systemd-run` scope) keeps running. `supervise` receives `LauncherError::Spawn` and never gets a `Child` or scope handle to stop. `reader` is dropped with the closure, so the child dies only on its next write (SIGPIPE), if ever. The launcher then reports "spawn failed" while a sandboxed agent may be running unsupervised, with no caps and no egress guard. Before this change, spawn was a single fallible step.
**Fix:** On pump-spawn failure, kill and reap the child before returning:
```rust
let pump = std::thread::Builder::new().name("launch-capture".to_owned()).spawn(move || { ... });
if let Err(e) = pump {
    let mut child = child;
    let _ = signal_group(child.id(), Signal::Kill); // or child.kill()
    let _ = child.wait();
    return Err(e);
}
```
In the `BwrapScope` posture the process group is `systemd-run`'s. Also stop the named scope there, or have the caller do it when `Err(Spawn)` follows a started child.

### WR-03: A dropped host-managed policy value is silent, and the dropped value is a restriction

**File:** `src/launcher/config_dir.rs:384-420` (`well_formed_hooks`, `well_formed_permissions`) and `:663-740` (`managed_settings` merge)
**Issue:** The WR-13 and WR-08 filters discard wrong-shape host `hooks` and `permissions` values without any diagnostic. The motivating risk, a malformed value displacing the gate, is real and the filter fixes it. But what is discarded is the administrator's policy from `/etc/claude-code`, which `DRIVER-SANDBOX.md` says is "merged in, never hidden". Examples:
- `permissions.deny: "Edit"` (string) is dropped.
- A future object-valued permissions setting is dropped (the allowlist is only string or bool).
- A non-object `hooks` block is dropped.

The admin's intended deny is lost without a log line, so an operator who typo'd a managed file believes a rule is active that the sandboxed driver does not have. Dropping is the safe direction for the gate, but it is fail-open for the admin's own restriction, and nothing tells anyone.
**Fix:** Have the filters report what they drop (count and top-level key), and surface it where the seed already reports problems, for example the launcher's `eprintln!` channel or the launch summary. Optionally refuse the launch (fail closed) when a host `permissions.deny` is present but not a list of strings, since that is an unambiguous admin restriction. Keep the gate-preserving behaviour for the rest.

## Info

### IN-01: Doc claims a directory at the lock name is `InvalidData`; the write-side open gives `EISDIR`

**File:** `src/driver/lock.rs:215` and `:213-222` (`acquire_at` docs, `ensure_lock_file` docs)
**Issue:** The docs say a symlink, hard link, FIFO "or a directory at the name is `InvalidData`". That holds for the read-only open that `probe_at` and `read_holder_at` use. `acquire_at` and `ensure_lock_file` open read+write with `O_CREAT`, and `open(2)` on a directory with `O_RDWR` fails with `EISDIR` before the type check. The result is still a refusal ("could not open ...: Is a directory"), but the error kind and wording differ from the documentation, and no test covers it.
**Fix:** Reword the docs to "a directory is refused (`EISDIR` on the write side, `InvalidData` on the read side)", or add an `EISDIR` to `InvalidData` mapping in `open_lock_file` plus a directory case in `ensure_lock_file_refuses_symlinks_and_hard_links_...`.

### IN-02: Stale assertion message in the registry hostile-corpus test

**File:** `src/registry.rs:2368-2370`
**Issue:** The count was correctly changed to `corpus.len() * 4`, but the failure message still says "Each member must reach all FIVE subjects -- `UnrecognizedRuntime`, both halves of `remove`, and the two opt-in functions". `UnrecognizedRuntime` no longer exists and the total is four. A future failure would send the reader to a subject that is gone.
**Fix:** Change to "all FOUR subjects -- both halves of `remove` and the two opt-in functions".

### IN-03: `RUNTIME_KEY` no longer has a code user

**File:** `src/config.rs:72`
**Issue:** After removing `RegisteredProject::runtime()`, `RUNTIME_KEY` is referenced only from doc comments (`config.rs:64`, `config_json.rs`). `DEFAULT_RUNTIME_KEY` still has a test user. A `pub const` raises no `dead_code` warning, so it will rot unnoticed.
**Fix:** Drop `RUNTIME_KEY`, or pin it with a test that a project's `runtime` key round-trips through `extra` unchanged. That is the only contract the doc comments still promise.

---

_Reviewed: 2026-10-29_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
