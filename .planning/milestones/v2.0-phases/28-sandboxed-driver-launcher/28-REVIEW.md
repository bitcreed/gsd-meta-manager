---
phase: 28-sandboxed-driver-launcher
reviewed: 2026-10-29T00:00:00Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - docs/CONFIGURATION.md
  - docs/GSD-CORE-SYNC.md
  - src/agents/adapters/mod.rs
  - src/config.rs
  - src/executor/mod.rs
  - src/launcher/process.rs
  - src/launcher/run.rs
  - src/registry.rs
  - src/state_reader/config_json.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/mod.rs
  - src/journal/redact.rs
  - src/launcher/sandbox.rs
  - src/driverctl/store.rs
  - src/launcher/config_dir.rs
findings:
  critical: 0
  warning: 8
  info: 6
  total: 14
status: issues_found
---

# Phase 28: Code Review Report

**Reviewed:** 2026-10-29
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Summary

Incremental re-review at HEAD. The scope is the delta `git diff 415ba4a HEAD` over the eleven requested files, which
is quick task 261029-1dp. The last four files in the list are not in that delta. I re-read them because they hold the
carried findings (`sandbox.rs`, `store.rs`, `config_dir.rs`) or the redaction table the new pump depends on
(`journal/redact.rs`). `git log 415ba4a..HEAD` is empty for the three carried files, so they are byte-identical to
the prior review.

- **f4cfa01 (SAFE-04 pump).** The master's stdout and stderr now share one pipe. A `launch-capture` thread reads it
  a whole line at a time, redacts each line, and writes the result to `launch-N.jsonl`. I traced the cases the task
  named. Partial lines and tokens split across reads are reassembled by `read_until` before the redactor sees them.
  The final unterminated line is redacted at EOF. Non-UTF-8 bytes go through `from_utf8_lossy`. A line over the
  4 MiB cap becomes a fixed note. After a write error the pump keeps draining, so the child cannot block on a full
  pipe. If the pump thread panics or reaches a read error, the reader is dropped and the child gets `SIGPIPE`, which
  fails closed. Child exit before the drain is handled by `capture.finish(5 s)`, and `finish` treats a disconnect as
  done. I found no path that writes unredacted bytes to disk. The bridged child inherits the same pipe, so the
  in-sandbox driver is covered too. Four weaknesses remain (WR-11 to WR-14), all conditional: a mid-line interleave
  between the two streams, a text fallback that is weaker than the JSON tree walk, no multi-line match for PEM keys
  in plain text, and a leaked child if the pump thread cannot start.
- **ce86206 (`AgentRuntime` removal).** `src/executor/runtime.rs` is deleted and the `RegisteredProject::runtime()`
  and `Preferences::default_runtime()` accessors are gone. A grep of `src`, `tests` and `docs` finds no dangling
  reference to `AgentRuntime`, `UnrecognizedRuntime`, `from_config_value` or `executor::runtime`. Both
  config structs flatten unknown keys into an `extra` map and have no `deny_unknown_fields`. A `config.json` that
  still carries `runtime`, `default_runtime` or `driver_max_concurrent` therefore loads and round-trips unchanged,
  whatever the value. Before, a bad value was a refusal at drive time. Now it is silently ignored, which is
  the intended outcome because the key selects nothing. There is no backwards-compatibility break. The only
  leftovers are stale comments and one dead constant (IN-10).

I read the whole diff and the surrounding code in `supervise` and `summarize_stream_file`. I did not run cargo. The
disk is tight, and the findings below rest on code reading plus one regex-semantics check done outside the crate
(Python's `\b` matches Rust's `regex` for ASCII, and it confirmed WR-12). Judgments marked
`[inferred — cleanup 2026-10-28]` are mine and unverified in a live run.

No critical issues. The ledger continues: CR-01, WR-01..WR-10 and IN-01..IN-07 were taken, so the new IDs are
WR-11..WR-14 and IN-08..IN-10. There is no CR-02.

### Prior findings re-checked at HEAD

| Prior | Disposition | Status at HEAD |
|---|---|---|
| CR-01 `bind_private` changes the process-wide umask | fixed | Holds. Not in the delta. |
| WR-01 bwrap started without `--new-session` | fixed | Holds. `sandbox.rs:626-628` still sets it. |
| WR-02 sibling-run hiding tmpfs skipped for a symlinked drivers dir | deferred | **Still present.** `sandbox.rs:582-587`, unchanged. Restated below. |
| WR-03 master prompt positional parsed as an option | fixed | Holds. Not in the delta. |
| WR-04 D-17 recovery block with no parsed items | deferred | **Still present.** `store.rs:1085`, unchanged. Restated below. |
| WR-05 tests require `.planning/` content the crate excludes | fixed | Holds. Not in the delta. |
| WR-06 float and budget flags unvalidated | fixed | Holds. Not in the delta. |
| WR-07 `install_signal_handlers` one-shot | fixed | Holds. Not in the delta. |
| WR-08 malformed host managed `hooks` or `permissions` replaces the gate | fixed | Holds. `config_dir.rs:718` still has the `("hooks" \| "permissions", _) => {}` arm. No heading emitted. |
| WR-09 launcher binary under `/tmp` or `/run` unreachable | open | **Still present.** `config_dir.rs:304`, unchanged. Restated below. |
| WR-10 wrong-shape host managed policy value dropped silently | open | **Still present.** `config_dir.rs` has no diagnostic path (`grep eprintln\|tracing` empty). Restated below. |
| IN-01 `DRIVER_LOCK_HELD=1` exported but unread | fixed | Holds. The variable is still documented as read by the master prompt (`run.rs:1610-1613`). |
| IN-02 module documentation drift in `process.rs`, `auth.rs` | fixed | Holds for the original items. The new pump doc is accurate except as noted in IN-08. |
| IN-03 harness-tests pass vacuously | fixed | Holds. Not in the delta. |
| IN-04 campaign ledger read-modify-append without a lock | skipped | Unchanged. |
| IN-05 UTS namespace does not hide the host's name | open | **Still present.** `sandbox.rs:51,78,625-626`, unchanged. Restated below. |
| IN-06 failed `record_launch_posture` leaves the launch half recorded | open | **Still present.** `run.rs` moved: the `?` is now at line 934. Restated below. |
| IN-07 host managed `permissions` scalars first-wins | open | **Still present.** `config_dir.rs:707-709` (`ours.entry(key).or_insert(other)`), unchanged. Restated below. |

## Warnings

### WR-02: The sibling-run hiding tmpfs is silently skipped when the drivers dir is reached through a symlink

**Status:** prior, still present at HEAD (deferred;
`todos/pending/2026-10-28-sandbox-drivers-tmpfs-skipped-for-symlinked-planning.md`).

**File:** `src/launcher/sandbox.rs:580-587`

**Issue:** `bwrap_argv` emits `--tmpfs <project>/.planning/meta-manager/drivers` only when
`paths.run_dir.parent() == Some(project.join(DRIVER_RUNS_REL))`. In `launch()` the run dir comes from
`validate_run_dir`, which returns the canonical path, so a symlinked `.planning` or drivers directory never matches.
The golden test `no_drivers_tmpfs_unless_the_run_dir_sits_directly_under_the_projects_drivers_dir` pins the silent
skip. The comment at lines 580-581 still says `validate_run_dir` "guarantees" the shape, which is false for symlinked
layouts. When the symlink target lies inside the project bind, sibling runs stay visible and writable to the
sandboxed driver. The state.json and lock-file hardening narrows what that buys (a planted FIFO or symlink is refused
instead of hanging a reader), but a driver can still rewrite a sibling's `state.json` or lock files.

**Fix:** carry the canonical drivers directory in `LaunchPaths` and compare against that. If the run dir is not
directly under it, refuse the launch rather than skipping the hiding.

```rust
// LaunchPaths gains: pub drivers_dir: PathBuf  // canonical parent of run_dir
if paths.run_dir.parent() == Some(paths.drivers_dir.as_path()) { /* emit the tmpfs */ }
```

### WR-04: The D-17 recovery block cannot be satisfied when DIRECTION has no parsed items

**Status:** prior, still present at HEAD (deferred;
`todos/pending/2026-10-28-d17-recovery-block-with-no-direction-items.md`).

**File:** `src/driverctl/store.rs:1081-1086` (`blocking_recovery`)

**Issue:** `.find(|id| direction.items.is_empty() || direction.items.iter().any(..))` is unchanged. The doc comment at
`store.rs:1076-1080` still says an awaiting id the human removed "no longer blocks", which is false when
`direction.items` is empty. With zero parsed items every awaiting id blocks, while `validate_log("recovered", ..)`
rejects any id DIRECTION does not list. The driver is told to log `recovered` for an item the protocol refuses. The
deferral reasoning stands: dropping the clause would loosen a fail-closed gate. The defect is the unsatisfiable
message, not the block.

**Fix:** keep the block. When `direction.items.is_empty()`, reword both refusals to say "DIRECTION.md has no
parseable `### I-NN` items; repair it, then the run can continue", or accept `recovered` for an awaiting id while
items are empty. Correct the doc comment for the empty case.

### WR-09: A launcher binary under `/tmp` or `/run` is unreachable inside the sandbox (NEW)

**Status:** prior, still present at HEAD (open).

**File:** `src/launcher/config_dir.rs:302-304` (`snapshot_exe`), with `src/launcher/sandbox.rs` (`--tmpfs /tmp`,
`--tmpfs /run`) and `src/launcher/run.rs:1596` (`bridged_command(&ctx.binary, ..)`)

**Issue:** `snapshot_exe` copies the binary out only when it lies under the driven project
(`!real.starts_with(project) && !exe.starts_with(project)` returns the original path). The sandbox replaces `/tmp`
and `/run` with empty tmpfs mounts, and `$HOME` with a tmpfs holding only the allowlist. A binary under `$HOME` is
handled by `HomeSetup::for_host`. Nothing handles one under `/tmp` or `/run` (`CARGO_TARGET_DIR=/tmp/...`, an
extracted release under `/tmp`, NixOS `/run/current-system`). bwrap then cannot exec the bridge. The visible
failure is a "No such file" (exit 127) after the run was already recorded, not a clear refusal. It is fail-closed,
so this is a robustness defect, not an exposure. `[inferred — cleanup 2026-10-28]`: argv order and call sites read,
no live sandbox run with such a binary.

**Fix:** widen `snapshot_exe`'s condition to "somewhere the sandbox hides": under the project, or under `/tmp`,
`/run`, `/var/run`, `/dev` or `/proc`. Copy it to `<cfg>/bin` as for the project case.

```rust
const SANDBOX_HIDDEN: &[&str] = &["/tmp", "/run", "/var/run", "/dev", "/proc"];
let hidden = |p: &Path| p.starts_with(project) || SANDBOX_HIDDEN.iter().any(|d| p.starts_with(d));
if !hidden(&real) && !hidden(exe) { return Ok(exe.to_owned()); }
```

### WR-10: A wrong-shape host managed policy value is dropped with no diagnostic, though a non-object file is refused (NEW)

**Status:** prior, still present at HEAD (open).

**File:** `src/launcher/config_dir.rs:359-424` (`well_formed_hooks`, `well_formed_permissions`),
`src/launcher/config_dir.rs:663-725` (their use in `managed_settings`), against `read_host_managed`
(`config_dir.rs:736-767`)

**Issue:** `read_host_managed` refuses a bad host managed file "instead of silently running without the
administrator's policy". The nested merge does the opposite: `{"permissions": {"deny": "Bash(rm *)"}}` loses the
deny rule, `{"hooks": {"PreToolUse": {...}}}` loses an audit hook, and one malformed hook item discards its whole
matcher entry. The path contains no `eprintln!`, `tracing` call or returned note, so neither the operator nor the
administrator learns the driver ran without the policy they wrote. WR-08's choice (ignore a non-object value) is
right for the gate. The defect is the silence. `[inferred — cleanup 2026-10-29]`: I can prove the absence of any
diagnostic. I did not verify how Claude Code treats such a value itself.

**Fix:** make the drop observable, and ideally keep the refuse-on-bad-policy rule.

```rust
// in read_host_managed, after the object check, for each doc:
if let Some(Value::Object(h)) = doc.get("hooks") && &well_formed_hooks(h) != h {
    return Err(invalid("a malformed hooks entry"));
}
// likewise for "permissions" with well_formed_permissions.
```

If refusal is too strict, return the dropped key paths from `managed_settings` and have `launch` print one
`driver launch: ignored malformed host managed policy: <file>: <path>` line, as it does for the host-proxy and gateway
warnings. Add a test asserting the notice or the refusal.

### WR-11: The master's stdout and stderr share one pipe, so a large write can interleave mid-line and split a token across the redactor's lines

**File:** `src/launcher/process.rs:105-110` (`std::io::pipe()`, `writer.try_clone()` for stderr), consumed by
`pump_redacted` at `process.rs:146-175`

**Issue:** Before f4cfa01 both streams were dup'd descriptors of one regular-file open description. A `write(2)` to
a regular file is serialized by the inode, so a stdout line written in one call could not be split by a stderr
write. A pipe gives no such guarantee beyond `PIPE_BUF` (4096 bytes): a write larger than that can be split when the
64 KiB pipe fills, and another writer's bytes can land in the gap. Tool results of tens of KiB are normal in
stream-json. When a stderr line lands inside a large stdout line, the pump sees (a) a stdout line cut in two by the
stderr text, and (b) the remainder of the JSON as a separate line. Both then fail the JSON parse and take the
text path (see WR-12). A token that straddles the cut is no longer one match: `sk-ant-` plus 8 or more characters
before the cut is redacted, but the remaining 80-odd characters sit at the start of the next line with no prefix,
so nothing matches them. This also breaks the result line for the outcome reader.
`[inferred — cleanup 2026-10-28]`: needs the pipe full during a large write and a stderr write at that moment.
Probability is low and I did not reproduce it. The prior file-based capture did not have this failure mode, and
SAFE-04's own premise is that a split must never expose a token.

**Fix:** give each stream its own pipe and its own pump thread, and have both threads write whole redacted lines
through one `Arc<Mutex<File>>`, so a line is never interleaved with another and each stream's lines are intact.

```rust
let (out_r, out_w) = std::io::pipe()?;
let (err_r, err_w) = std::io::pipe()?;
let sink = Arc::new(Mutex::new(out));
// two threads: pump_redacted(BufReader::new(out_r), LockedSink(sink.clone())) and the same for err_r;
// CaptureHandle waits for both completion signals.
```

### WR-12: A line that does not parse as JSON is redacted as raw text, where `\b`-anchored rules miss tokens that follow a JSON escape

**File:** `src/launcher/process.rs:179-186` (`redact_line`, the `Err(_)` arm), against the anchors in
`src/journal/redact.rs:143,149,155,161,167,173`

**Issue:** `redact_line` uses the tree walk only when `serde_json::from_slice` succeeds. Otherwise it runs `redact`
on the raw bytes. For a line that is JSON but fails to parse, the raw text still carries JSON escapes. The rules
`sk`, `ghpat`, `gh`, `aws`, `slack` and `jwt` are anchored with `\b`. In raw text a token that follows an escape
such as `\n` or `\t` is preceded by the letter `n` or `t`, so there is no word boundary and the rule does not fire.
`{"text":"KEY=\nghp_<36 chars>"}` is the usual shape of tool output. I checked the boundary semantics with a Python
regex (`\bgh[pousr]_...` against `x\nghp_AAAA...` returns no match); Python's `\b` and the Rust `regex` crate agree
for ASCII. A line fails to parse when it is cut by WR-11, nests deeper than serde_json's 128-level recursion limit,
contains a lone surrogate escape (`\ud800`), or has trailing garbage. In each case the credential reaches
`launch-N.jsonl` unredacted. The pump's own doc says "Any other line is redacted as text" without noting that this
text may be JSON-escaped.

**Fix:** for a body that starts with `{` or `[` and does not parse, redact a de-escaped view before falling back.
Replace `\n`, `\r`, `\t` with a space, and `\"` and `\\` with their characters. Keep the line on one line, so NDJSON
framing survives.

```rust
Err(_) => {
    let text = String::from_utf8_lossy(body);
    let view = if text.trim_start().starts_with(['{', '[']) { deescape(&text) } else { text.into_owned() };
    redact(&view)
}
```

Add a test with an unterminated JSON line containing `\nghp_...` and one with a lone surrogate.

### WR-13: A multi-line PEM private key printed as plain text is not redacted, because the text path sees one line at a time

**File:** `src/launcher/process.rs:146-175,185` (`pump_redacted`, `redact_line`), against the `pem` rule at
`src/journal/redact.rs:108`

**Issue:** The `pem` rule needs `-----BEGIN ... PRIVATE KEY-----` and `-----END ... PRIVATE KEY-----` in the same
string. Inside a JSON event the whole block is one string (newlines decoded by the tree walk), so it is caught.
On a plain-text line (the master's stderr, a non-JSON stdout line, or a block that lost its JSON framing under WR-11),
the block arrives as separate lines. The `BEGIN` line, every base64 body line and the `END` line each fail the
pattern, so the key body is written to disk as-is. The pump's doc presents the reassembly of split reads as the
guarantee, but it does not reassemble across lines. `[inferred — cleanup 2026-10-28]`: whether the master prints a
PEM on stderr is unlikely, so the likelihood is low and the impact is a private key.

**Fix:** keep a small state in `pump_redacted`: after a plain-text line that matches `-----BEGIN [A-Z ]*PRIVATE KEY-----`
with no `END` on the same line, replace every following line with `[REDACTED:private-key]` until a line contains
`-----END`, with a line limit as a backstop. Add a test with a three-line key on stderr.

### WR-14: A failure to start the capture thread leaves the driver child running with no one watching it

**File:** `src/launcher/process.rs:116-129` (`spawn_launch_child`)

**Issue:** The child is spawned at line 116 and `let child = spawned?` succeeds. The pump thread is created after
that with `Builder::spawn(..)?` at lines 123-128. If thread creation fails (`EAGAIN` under a thread or memory limit),
the function returns `Err`, the `Child` is dropped (which neither kills nor reaps it), and the closure holding the
pipe reader is dropped. `supervise` maps the error to `LauncherError::Spawn` and returns. The driver, already running
in its own process group or scope, continues until its first write hits the closed pipe (`SIGPIPE`). Until then no
cap monitor or egress guard is running for it. The window is small, and the failure is rare, but it is a path that
leaves a sandboxed model process alive after the launcher reports failure.

**Fix:** start the pump thread before the child and give it the reader first. If the child then fails to spawn,
the thread sees EOF once the write ends are dropped. Alternatively kill and reap on the error path:

```rust
let child = spawned?;
let handle = match std::thread::Builder::new().name("launch-capture".into()).spawn(/* .. */) {
    Ok(h) => h,
    Err(e) => { let mut c = child; let _ = c.kill(); let _ = c.wait(); return Err(e); }
};
```

## Info

### IN-05: The UTS namespace does not hide the host's name (NEW)

**Status:** prior, still present at HEAD (open).

**File:** `src/launcher/sandbox.rs:51` (module doc), `src/launcher/sandbox.rs:625-626` (`--hostname`),
`src/launcher/sandbox.rs:383-404` (`etc_entries_in`)

**Issue:** The module doc says `--unshare-uts` fixes the hostname to `gsdmm-sandbox` so the host's name is not
leaked. The sandbox then re-binds every host `/etc` entry read-only except `claude-code`, which includes
`/etc/hostname`, usually `/etc/hosts` and `/etc/machine-id`. The kernel hostname is hidden, but the host name and a
stable host identifier are still readable. The driver can send them out only through the inference proxy or an
allowlisted CONNECT host, so the exposure is small. The claim is stronger than the profile.

**Fix:** soften the doc to "the kernel hostname is not leaked", or skip `hostname` and `machine-id` in
`etc_entries_in` and bind per-run stand-ins at those paths.

### IN-06: A failed `record_launch_posture` leaves the launch half recorded (NEW)

**Status:** prior, still present at HEAD (open).

**File:** `src/launcher/run.rs:934` (the `?`), compare the `supervise` error arm at `run.rs:947-955`

**Issue:** `begin_launch(..)?` (line 920) has already bumped `launches` and written the launcher connection. The next
statement, `record_launch_posture(..)?`, can fail on a store write error and returns without the `stop_serving()` plus
`set_terminal(Stopped, "launcher:startup-failed")` that the `supervise` error arm does for the same state. The caller
closes the session and drops the lock, leaving a run whose status says it started a launch that never ran. The state
is recoverable (liveness derives the run as dead), and a store that fails here would likely fail the terminal write
too. This is a consistency gap, not a data-loss path.

**Fix:** route both early `?` returns through the same cleanup as the `Err(e)` arm, with a small helper that records
the failed start before returning the error.

### IN-07: Host managed `permissions` scalar settings are first-wins while every other host key is last-wins (NEW)

**Status:** prior, still present at HEAD (open).

**File:** `src/launcher/config_dir.rs:698-711` (the `other => ours.entry(key).or_insert(other)` arm), against the
catch-all `settings.insert(key.clone(), value.clone())` at `config_dir.rs:719-721`

**Issue:** `managed_settings` merges `host` in precedence order (the file, then the drop-ins sorted by name). For a
top-level key the catch-all `insert` makes the last doc win. For the string and bool `permissions` settings
(`defaultMode`, `disableBypassPermissionsMode`, `blockReadsOutsideWorkingDirectories`) the arm uses `or_insert`,
so the FIRST doc wins. A base `defaultMode: "default"` beats a drop-in's `"plan"`, while the same pair under `model`
would merge as the drop-in's. The comment calls this "(I-6)", a recorded decision, but the function doc does not say
it and no test pins the winner. `[inferred — cleanup 2026-10-29]`: whether Claude Code's drop-in precedence is
last-wins is not verified here, so this is an internal inconsistency, not a confirmed mismatch.

**Fix:** pick one rule. Use `ours.insert(key, other)` so these settings are last-wins like every other key, or state
first-wins in the `managed_settings` doc comment and add a test with the same scalar in the file and in a drop-in.

### IN-08: The cap comment claims the pump never writes a line the reader would skip, but redaction can make a line longer than its input

**File:** `src/launcher/process.rs:41-43` (`MAX_STREAM_LINE` doc), `process.rs:161-167`, reader at
`src/launcher/run.rs:1891-1894`

**Issue:** The pump limits its *input* line to 4 MiB, and `summarize_stream_file` skips an *output* line longer
than 4 MiB. The redacted line can be longer than the input: `sk-ant-` plus 8 characters (15 bytes) becomes
`[REDACTED:anthropic-key]` (24 bytes), and `/home/x` grows to `/home/[REDACTED:user]`. A line just under the cap with many
such matches is written longer than the cap, and the reader skips it. If that line is the `result`, the outcome reader
sees no result. This needs a near-4 MiB `result` line, which is unrealistic, so the impact is small. The comment's
"never" is still wrong.

**Fix:** soften the comment, or make the check post-redaction: if the rendered line exceeds `MAX_STREAM_LINE`,
write `OVERLONG_STREAM_NOTE` instead.

### IN-09: The pump hides write and read failures, and the launcher cannot tell an empty capture from a lost one

**File:** `src/launcher/process.rs:151-157,168-174`, `CaptureHandle::finish` at `process.rs:63-69`,
`src/launcher/run.rs:1690`

**Issue:** On a write error (disk full, quota) the pump sets `writable = false` and silently discards every later
line, with no log and no signal. On a read error it breaks out of the loop, which drops the reader and `SIGPIPE`s the
child, again with no signal. `finish` returns only "done or disconnected" and `supervise` discards the `bool`.
`summarize_stream_file` then sees a truncated or empty file and reports no result, so a run that succeeded is
classified as failed with no explanation. The loss is fail-closed for secrets and not a security issue.

**Fix:** have the pump send a small status (`Ok`, `WriteFailed`, `ReadFailed`) instead of `()`, and print one
`driver launch: master output capture failed: ...` line from `supervise` when it is not `Ok` or when `finish` times out.

### IN-10: Leftovers from the `AgentRuntime` removal: a stale `runtime()` reference, a stale assertion message, a dead constant, and an untested per-project `runtime` round trip

**File:** `src/config.rs:89` (comment "like `runtime()`"), `src/registry.rs:2367-2371` (assertion message), `src/config.rs:72`
(`RUNTIME_KEY`)

**Issue:** (1) The doc for `driver_egress_allowlist` still says the key "lives in the flattened `extra` map like
`runtime()`, so a typo is a refusal at drive time". `runtime()` no longer exists and `drive` is gone. (2) The
registry census assertion now counts four subjects (`corpus.len() * 4`), but its failure message still says "reach
all FIVE subjects — `UnrecognizedRuntime`, both halves of `remove`, and the two opt-in functions". If it ever fires,
it sends the reader to a subject that no longer exists. (3) `pub const RUNTIME_KEY` has no remaining use in code,
only doc links. (4) `old_preference_keys_still_load_from_a_config_file` pins the `default_runtime` round trip, and
the generic `extra` flatten test covers unknown keys, but nothing pins the per-project `runtime` key the docs say
"is kept and written back unchanged". `[inferred — cleanup 2026-10-29]`: the behavior is right because the flatten
carries it, so this is a doc and test gap only.

**Fix:** reword (1) to "like `driver_open_pr`", change the message in (2) to "all FOUR subjects — both halves of
`remove` and the two opt-in functions", keep `RUNTIME_KEY` only if a test uses it (add one asserting that
`{"projects":{"a":{"path":"..","runtime":"codex"}}}` survives load and save), otherwise delete it.

---

_Reviewed: 2026-10-29_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
