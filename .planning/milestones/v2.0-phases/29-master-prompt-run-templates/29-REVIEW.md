---
phase: 29-master-prompt-run-templates
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
  warning: 7
  info: 8
  total: 15
new_findings:
  critical: 0
  warning: 2
  info: 5
status: issues_found
---

# Phase 29: Code Review Report (incremental re-review at HEAD)

**Reviewed:** 2026-10-29
**Depth:** standard
**Files Reviewed:** 22
**Status:** issues_found

## Summary

This review is the union of two passes. The first (`e6c0d9e..9eb4dff`) is preserved verbatim
below under "Prior review (9eb4dff)". The second is an incremental pass over
`git diff 9eb4dff HEAD` for quick task 261029-1dp: `f4cfa01` (the SAFE-04 redacting capture pump,
`src/launcher/process.rs` and `src/launcher/run.rs`) and `ce86206` (removal of the dead
`AgentRuntime` config key, `src/executor/runtime.rs` deleted, doc-comment corrections).

**Delta 9eb4dff..HEAD, what was traced.**

* The capture pump (`process.rs:96-209`, `run.rs:1629-1700`). Every byte the master writes to
  stdout or stderr goes through one pipe (`process.rs:105-115`), a single pump thread, and one
  `File`; `launch-N.jsonl` has no other writer (`grep` over `src/` finds the path only at
  `run.rs:1629` and the reader at `run.rs:1698`). The parent's copies of the write end are
  released by `drop(cmd)` (`:119`) so EOF is reachable; the pipe fds are `O_CLOEXEC`, so a
  concurrent spawn in another launcher thread cannot inherit them. A write error switches to
  drain-and-discard (`:168-170`), so the child cannot block on a full pipe because of the file.
  An over-long line is replaced by a fixed note with no fragment (`:161-164`). A JSON line is
  redacted by tree walk, a non-JSON or unparseable line as text, with an unterminated final line
  handled at EOF. The launcher waits at most `CAPTURE_FINISH_BUDGET` for EOF and a pump panic
  reads as a disconnect, so the launcher cannot hang on the pump. `summarize_stream_file` reads
  the redacted file, so the outcome now rests on redacted text (the OAuth-revoked marker and
  `type`/`result` survive; pinned by a unit test).
* The AgentRuntime removal: `RegisteredProject::runtime()`, `Preferences::default_runtime()` and
  `executor/runtime.rs` are gone and nothing in `src/`, `tests/`, `docs/` or `Cargo.toml` refers
  to them. Both keys stay in the `#[serde(flatten)] extra` maps (`config.rs:67-68`, `:515-516`),
  so an old config still loads and round-trips; `default_runtime` is pinned by
  `old_preference_keys_still_load_from_a_config_file` (`ui/screens/preferences.rs:218-241`).
  The docs now say "kept and written back unchanged", which is accurate. The claim in
  `registry.rs:958-961` that no launch is gated on the opt-in record was checked:
  `driver_opt_in` is read only for `policy_for` at `driver_cli.rs:291-298`, `:523-525`,
  `app.rs:1053-1058` and the Driver-tab form, never to refuse a launch.

No critical defect in the delta. Two new warnings: a gap in the pump's redaction (a multi-line
credential printed as plain text is written to disk unredacted) and a child left running when the
pump thread cannot be started. Five new info items: a stale assertion message, a dangling doc
reference, an unpinned per-project round trip, a silently ignored capture timeout, and an
over-cap output line the reader would skip. All carried findings from the prior review are
unchanged; their files are byte-identical since 9eb4dff (`git diff --stat 9eb4dff HEAD` over
them is empty).

### Prior review (9eb4dff)

Incremental re-review of the diff `e6c0d9e..HEAD` (quick tasks 261028-t3d/t3e/t3f and the
261028-v2z WR-13 fix). The changes were traced end to end:

* `src/driver/lock.rs`: one guarded `open_lock_file` now serves `acquire_at`, `ensure_lock_file`,
  `probe_at` and `read_holder_at`. The refusal mapping (ELOOP, non-regular, link count) and the
  `InvalidData` to `Unavailable` / `Unknown(INVAL)` / `None` translations are consistent; the
  4096-byte holder cap is applied correctly (`take(cap + 1)` then a length test).
* `src/driver/direction.rs`: `exchange_or_restore` is shared by `give_back_newest` and the
  `run_loop` exchange back. The `EEXIST` path (exchange with the file created in between, then the
  verify loop puts the newest save back) and the `Restored => continue` retry were walked through
  and are sound; the loop is bounded by `GIVE_BACK_ROUNDS` and `SPLICE_TRIES`, and nothing unlinks a
  temp holding a human's bytes.
* `src/driverctl/store.rs`, `src/launcher/run.rs`, `src/launcher/stop.rs`, `src/driverctl/serve.rs`:
  `state.json` and `DIRECTION.lock` now go through the guarded reads and opens. `NotFound` still
  creates a fresh state; `InvalidData` is `CorruptState`; the wall-clock pre-check treating a refused
  `state.json` as "no recorded time" is backstopped by the store's own refusal at open.
* `src/launcher/config_dir.rs`: WR-02 and the nested WR-13 shapes are handled by
  `well_formed_hooks` / `well_formed_permissions` and the `("hooks" | "permissions", _) => {}` arm.

No critical defects. Three new warnings concern the read-side link-count refusal, an
inconsistent precedence rule in the managed `permissions` merge, and the silent dropping of host
policy; one new info item concerns an uninformative refusal message. Prior findings are
re-checked below under their original IDs.

## Critical Issues

None.

## Warnings

### WR-14: the capture pump redacts line by line, so a multi-line PEM private key printed as plain text reaches `launch-N.jsonl` unredacted (new, 2026-10-29)

**File:** `src/launcher/process.rs:161-167,179-187` (rule: `src/journal/redact.rs:107-110`)
**Issue:** SAFE-04 is the stated purpose of the pump ("a credential-shaped token the master prints
never reaches the file", `process.rs:83-86`). The pump hands `redact` exactly one physical line at
a time (`:165-166`), and `redact_line` falls back to text redaction for any line that is not one
JSON document (`:185`). The `pem` rule is the one rule in the table that is inherently multi-line:
`-----BEGIN [A-Z ]*PRIVATE KEY-----(?s:.)*?-----END [A-Z ]*PRIVATE KEY-----` needs both fences in
one haystack. Fed line by line, the BEGIN fence, every base64 body line and the END fence each fail
to match, so the whole key is written to disk verbatim. Checked with the same pattern on
`BEGIN`, body, body and `END` lines: no line matches (all `False`), while the joined text matches.
Inside stream-json this does not happen (a JSON string escapes its newlines, so the key is one
leaf and the rule fires, which is why the unit tests are green), but the same pipe also carries
stderr and any non-JSON output of the master or of a leftover descendant (`cat id_rsa` in a shell
that writes to the inherited stderr, a Node stack trace printing a key). The module doc and the
pump doc ("Any other line is redacted as text") present the text path as equivalent; it is not,
and no test covers a multi-line block.
**Fix:** Make the text path block-aware for the one multi-line rule: in `pump_redacted`, when a
non-JSON line contains `-----BEGIN ` and `PRIVATE KEY-----` without a matching END, switch to a
"inside key block" state that writes `[REDACTED:private-key]` once and discards lines until the
`-----END ... PRIVATE KEY-----` line (or EOF, or a bounded line count), then resumes. Add a test
that feeds a four-line PEM through `pump_chunks` and asserts no body line survives, and one where
the END fence never arrives.

### WR-15: a failure to start the pump thread leaves the already-running sandboxed child unsupervised (new, 2026-10-29)

**File:** `src/launcher/process.rs:116-129`; caller `src/launcher/run.rs:1638-1646`
**Issue:** `cmd.spawn()` succeeds at `:116` and the `Child` is held in a local. The next fallible
step, `std::thread::Builder::new()...spawn(...)?` at `:123-128`, returns `Err` on thread or
resource exhaustion, dropping the `Child` without `kill`/`wait` (dropping a `std::process::Child`
does neither). `supervise` maps the error to `LauncherError::Spawn` (`run.rs:1639-1646`) and
returns, before `containment` and `CapMonitor` exist (`:1653-1661`), so no cap, user stop or
teardown ever applies to a live `systemd-run`/`claude` that has full write access to the project.
The thread closure owns `reader`, so it is dropped on the failure and the child's next write raises
`SIGPIPE`/`EPIPE`; a master in a long tool call or a quiet phase keeps running until then. The old
code had no step between spawn and the return of the `Child`, so this window is new.
**Fix:** Start the pump thread so its failure can be handled with the child in hand, and stop the
child on that path:
```rust
let handle = std::thread::Builder::new().name("launch-capture".to_owned()).spawn(...);
if let Err(e) = handle {
    let _ = signal_group(child.id(), Signal::KILL); // it is a group leader (process_group(0))
    let _ = child.kill();
    let _ = child.wait();
    return Err(e);
}
```
For the `BwrapScope` posture the scope must be stopped as well, or the pump thread should be
created before the child (the reader and `out` are available first), so a thread failure costs
nothing.

### WR-01: lint and store still disagree on which line is `- control:` (WR-01 only half closed)

**File:** `src/driver_templates/lint.rs:52-54,287-295` vs `src/driver/direction.rs:156-162`
**Issue:** The store's `control_re` is deliberately loose (`(?mi)^[ \t]*-[ \t]+control[ \t]*:`):
an indented line, extra spaces after the dash or any letter case is honoured, and a `pause`/`stop`
on any such line holds the run. The lint's `KEY_VALUE` is `^- ([A-Za-z][A-Za-z-]*):` and the
`control` arms match the lowercase key only. So `  - control: pause`, `-  control: stop` and
`- Control: pause` are honoured by the driver (the dispatch gate and `dispatched` record refuse)
while `driver check` reports a clean file; the new "more than one `- control:`" and "outside
`## Direction`" checks are blind to the same lines. The module doc claims the two readers
"cannot disagree about what is an item or a control line", which is false for control lines.
Direction of failure is safe (the run holds), but the human is told the plan is valid and the
driver then refuses to dispatch with no lint finding to explain it.
**Fix:** In `lint_direction`, feed control detection from the store's matcher
(`crate::driver::direction::control_re()`), per line, independent of `KEY_VALUE`, and apply the
section/duplicate checks to every match:
```rust
if let Some(caps) = control_re().captures(line) {
    let value = caps[1].to_owned();
    if section == Section::Direction { controls.push((n, value)); }
    else { problems.push(/* outside ## Direction */); }
    // a non-canonical spelling (indent, case) should also be reported as a problem
}
```
**Re-check (2026-10-29):** Still present. `src/driver_templates/lint.rs` is unchanged since the
diff base; `KEY_VALUE` is still `^- ([A-Za-z][A-Za-z-]*):` at `lint.rs:51-53` and the `control` arms
at `lint.rs:286-295` still key on it, while `control_re()` at `src/driver/direction.rs:157-163`
is still the loose `(?mi)` form.
_Carried unchanged — source byte-identical since 9eb4dff._

### WR-03: `hook_is_kept` treats any script under `~/.claude/hooks/` or `~/.claude/scripts/` as a GSD hook

**File:** `src/launcher/config_dir.rs:350-355,444-447`
**Issue:** D-19 says only GSD and rtk hooks reach the driver, but `names_gsd` is a substring test
for the directories `.claude/hooks/`, `.claude/gsd-core/`, `.claude/get-shit-done/` and
`.claude/scripts/`. Those are shared directories: a third-party or personal hook script placed in
`~/.claude/hooks/` or `~/.claude/scripts/` (and `scripts` is symlinked into every per-run config
dir by `LINK_FROM_HOST`) passes the filter, as does any command that merely contains one of the
strings (for example in a trailing comment or argument). The only remaining guards are the
`localhost`/`127.0.0.1` substring test and the keep-roots path check, neither of which stops a
hook that talks to a LAN or cloud endpoint. The stated purpose (keep the operator's phone-bridge
hooks out of the sandbox) therefore depends on where the operator happened to install them.
**Fix:** Require the file name, not just the directory: for the `hooks/` and `scripts/` markers
also require a path component starting with `gsd-` (every shipped GSD hook is `gsd-*.js|sh`
under `~/.claude/hooks`, per the installer), or an explicit allowlist of basenames.
**Re-check (2026-10-29):** Still present. The marker list is unchanged at
`src/launcher/config_dir.rs:429-434` and `hook_is_kept` still accepts on the directory substring
alone at `config_dir.rs:523-533` (the cited line numbers above are from the earlier revision; the
code moved down by the WR-13 helpers).
_Carried unchanged — source byte-identical since 9eb4dff._

### WR-08: the read-side link-count refusal turns a held run lock into `Unknown` after any hard link is made (new, 2026-10-29)

**File:** `src/driver/lock.rs:197-206` (via `probe_at` at `:437` and `read_holder_at` at `:366`); consumers `src/launcher/stop.rs:103-106`, `src/launcher/push.rs:939-945`, `src/state_reader/driver_frontmatter.rs:805`
**Issue:** t3d routed `probe_at` and `read_holder_at` through `open_lock_file`, which refuses any
file whose link count is not exactly one. For the write side (`acquire_at`) that protects a user's
file from `set_len(0)` through a pre-planted link. For the read side it protects nothing: a probe
only takes `LOCK_SH | LOCK_NB` and the holder read is capped, and anyone able to plant a hard link
in the run directory can equally unlink `launcher.lock` and put their own regular, single-link
file there, so the check does not stop a forged holder record. What it does do is misreport a
perfectly good lock. A hard link made AFTER the launcher took the lock (`cp -al` or `rsync -H`
snapshots, hard-link backup tools, an editor "backup by link") gives the live lock `nlink == 2`,
and from then on `probe_at` answers `Unknown(EINVAL)` for a run that is `Held`:
`driver stop` fails with `could not probe the run lock: Invalid argument` and never signals a
live launcher, `driver push` errors, and the TUI shows the run as `LivenessUnknown` instead of
running. The module doc itself notes that a link added after the open "only aliases the inode";
the same reasoning makes the check pointless on the read path.
**Fix:** Keep `O_NOFOLLOW | O_NONBLOCK` and the regular-file check for the read-only opens, and
apply the `nlink` test only when the options carry `create`/`write` (acquire and
`ensure_lock_file`):
```rust
fn open_lock_file(path: &Path, options: &mut OpenOptions, writer: bool) -> io::Result<File> {
    // ...open and the is_file() check as now...
    if writer && meta.nlink() != 1 { /* InvalidData "has N hard links" */ }
    Ok(file)
}
```
Update `probe_at_reads_a_hard_linked_lock_file_as_unknown` and the
`read_holder_at_refuses_a_symlink_or_hard_link...` test accordingly.
_Carried unchanged — source byte-identical since 9eb4dff._

### WR-09: managed `permissions` scalars resolve first-wins while every other managed key resolves last-wins (new, 2026-10-29)

**File:** `src/launcher/config_dir.rs:702-710` vs `:719-721`
**Issue:** `managed_settings` documents `host` as "in precedence order: the file, then the
drop-ins" (`:650`) and the catch-all arm (`settings.insert`) lets a later document override an
earlier one, which is Claude Code's behaviour for `managed-settings.json` plus
`managed-settings.d/*.json` (drop-ins are layered on top of the base file). The new WR-13 code
merges the surviving string/bool `permissions` settings with `ours.entry(key).or_insert(other)`,
"the first doc that sets it wins (I-6)". With `managed-settings.json` setting
`permissions.defaultMode = "default"` and `managed-settings.d/50-local.json` setting `"plan"`,
Claude Code resolves `plan` but the launcher's managed file pins `default`; for
`disableBypassPermissionsMode` the same inversion applies. Before the change this branch only
handled non-array values the same way, so it is not new behaviour, but WR-13 now codifies it as
intended and routes the real settings (`defaultMode`, `disableBypassPermissionsMode`,
`blockReadsOutsideWorkingDirectories`) through it, while the doc comment promises the opposite
order. The test `well_formed_host_values_still_merge_beside_wrong_shape_ones` only uses one
document, so the order is not pinned.
**Fix:** Let a later document override, matching the catch-all and Claude Code, and pin it:
```rust
other => { ours.insert(key, other); }   // later doc wins; `deny` only ever arrives as an array
```
Add a two-document test (`defaultMode` in the file, a different value in a drop-in) asserting the
drop-in's value.
_Carried unchanged — source byte-identical since 9eb4dff._

### WR-10: wrong-shape and unrecognised host policy is dropped with no diagnostic, unlike the top-level refusal (new, 2026-10-29)

**File:** `src/launcher/config_dir.rs:359-425,681-723`; contrast `:728-735`
**Issue:** `read_host_managed` deliberately refuses the launch when a managed file is not a JSON
object, with the rationale that silently running without the administrator's policy is worse. The
new nested filters take the opposite stance: `well_formed_hooks` and `well_formed_permissions`
discard anything outside a fixed shape and nothing reports it (there is no `eprintln!`/tracing
call anywhere in the module, and `seed` returns nothing about what was dropped). Two consequences
beyond the already documented "wrong shape is dropped": (a) `well_formed_permissions` keeps a key
outside `PERMISSION_LISTS` only when its value is a string or bool, so any list- or object-valued
`permissions` key the administrator uses that this list does not know (a rule list added by a
later Claude Code release, a structured setting) is silently removed from the managed tier and the
sandboxed driver runs without that restriction; (b) a deny rule that Claude Code would accept but
that is not a plain string is dropped from an otherwise merged `deny` list. In both cases the
operator sees a normal launch.
**Fix:** Return or log the dropped paths and surface them where `seed` is called (a launch
warning naming the file and JSON pointer), and either treat an unknown list/object-valued
`permissions` key as pass-through (append lists, keep objects) or refuse the launch for it the way
a non-object file is refused. At minimum emit one line per dropped value so the weakening is
visible.
_Carried unchanged — source byte-identical since 9eb4dff._

## Info

### IN-07: the pump can emit a line the stream reader will skip, contradicting the "one constant for both sides" claim (new, 2026-10-29)

**File:** `src/launcher/process.rs:40-43,161-168`; reader `src/launcher/run.rs:1891-1893`
**Issue:** `MAX_STREAM_LINE` is documented as the single cap so "the pump never writes a line the
reader would have to skip". The cap is applied to the INPUT line (`:151-154`); the written line is
the redacted one, which can be longer. Redaction replacements are longer than many matches
(`sk-ant-` plus 8 characters, 15 bytes, becomes the 24-byte `[REDACTED:anthropic-key]`), and the
JSON path re-serialises the tree. A line a little under 4 MiB that carries many short
credential-shaped strings can therefore be written at over 4 MiB, and `summarize_stream_file`
then drops it unread (`skip_line`). If that line is the `result` event, the run is summarised as
having no result. Needs a hostile or pathological output, so it is cosmetic in practice, but the
documented invariant is not what the code enforces.
**Fix:** Check the rendered length before writing; if `rendered.len() as u64 >= MAX_STREAM_LINE`
write `OVERLONG_STREAM_NOTE` instead (or lower the pump's input cap to leave headroom), and reword
the constant's doc comment.

### IN-08: a capture-finish timeout is ignored and leaves no trace, so a truncated `launch-N.jsonl` looks like a normal end (new, 2026-10-29)

**File:** `src/launcher/run.rs:1690`; `src/launcher/process.rs:65-71`
**Issue:** `CaptureHandle::finish` returns `false` when the pump did not reach EOF within
`CAPTURE_FINISH_BUDGET` (a descendant that escaped the group or scope still holds the pipe, or a
very large backlog). `supervise` discards the value and reads the file at once
(`run.rs:1690-1698`), so the summary can lack the last events, including the `result`, and the
launch is judged on a partial file with no log line or `launch-end` field saying the capture was
cut. The `[inferred]` rationale only covers "must not hang the launcher", not "must not mislead".
**Fix:** Keep the boolean and record it where the outcome is recorded (a `tracing::warn!` with the
run id, and a note on the `launch-end` event / `Exited`), e.g.
`let capture_complete = capture.finish(CAPTURE_FINISH_BUDGET);`.

### IN-09: the redaction-corpus assertion message still describes five subjects including the removed `UnrecognizedRuntime` (new, 2026-10-29)

**File:** `src/registry.rs:2362-2372`
**Issue:** The count was re-measured to `corpus.len() * 4` and the comment explains why, but the
failure message in the same `assert!` still says "Each member must reach all FIVE subjects —
`UnrecognizedRuntime`, both halves of `remove`, and the two opt-in functions". Anyone who sees
the assertion fire will be told to look for a subject that no longer exists and that there are
five, which sends them hunting for a missing one. The `messages_for` doc (`:2131-2133`) was
updated; this string was not.
**Fix:** "Each member must reach all FOUR subjects — both halves of `remove` and the two opt-in
functions".

### IN-10: `RegisteredProject::driver_egress_allowlist` doc still refers to the removed `runtime()` accessor (new, 2026-10-29)

**File:** `src/config.rs:89`
**Issue:** "The key lives in the flattened `extra` map like `runtime()`, so a typo is a refusal at
drive time". `RegisteredProject::runtime()` was deleted by ce86206 (`grep -rn "runtime()" src docs`
finds only this line), so the comparison points at nothing, and the comment block above the `extra`
field (`:63-66`) now describes `runtime` as unvalidated and selecting nothing, which makes the
"typo is a refusal" half of the sentence contradict its own analogy.
**Fix:** Reword to "The key lives in the flattened `extra` map, so a typo is a refusal at
drive time and the key round-trips untouched `[inferred — audit]`."

### IN-11: no test pins the per-project legacy `runtime` key surviving a load and save, and `RUNTIME_KEY` is now referenced only from comments (new, 2026-10-29)

**File:** `src/config.rs:71-72`; tests `src/ui/screens/preferences.rs:218-241`, `src/config.rs:1110-1138`
**Issue:** The docs promise per-project `runtime` is "kept and written back unchanged"
(`docs/CONFIGURATION.md:84`). The only round-trip test of a legacy runtime key is the
`preferences.default_runtime` one; the per-project key relies on the generic `FUTURE_CONFIG` test,
which carries `driver_quarantine_until` and `driver_nice_level`, not `runtime`. The deleted
`executor/runtime.rs` tests and the removed `RegisteredProject::runtime()` were the only code that
named the key (`grep -n '"runtime"'` over `tests/` and `src/` finds only GSD's own `runtime` in
`config_json.rs`), and `RUNTIME_KEY` now has no code reference at all (doc comments only). The
behaviour is structurally guaranteed by `#[serde(flatten)]`, so this is a regression guard, not a
live bug.
**Fix:** Add `old_per_project_runtime_key_survives_load_and_save` next to the preferences test
(a project entry with `RUNTIME_KEY: "codex"`, load, save, reload, assert), which also gives
`RUNTIME_KEY` a user; or drop the constant and use the string in the test.

### IN-01: store comment says "every other event still records" when DIRECTION.md is unreadable, but item events are rejected

**File:** `src/driverctl/store.rs:1116-1132`
**Issue:** With `DIRECTION.md` unreadable (oversized, symlink, FIFO) only `dispatched` is refused
with the "unreadable" reason. `report`, `verified`, `status`, `needs-human` and `recovered` are
validated against `direction.items == []` (via `unwrap_or_default`), so `known_item` rejects them
with "`--item 'I-01' is not an item in DIRECTION.md`", which names the wrong cause and sends the
driver (and a human reading LOG) looking at the item id rather than the file. The comment
promises they record.
**Fix:** When `read` is `Err` and the event takes an item, reject with the same "DIRECTION.md is
unreadable (kind)" message, and correct the comment.
**Re-check (2026-10-29):** Still present. `handle_log` at `src/driverctl/store.rs:1124-1137` is
unchanged in behaviour: only `event == "dispatched"` returns early on `Err(kind)`, and
`read.unwrap_or_default()` feeds an empty id list to `protocol::validate_log` for every other
event; the comment "Every other event still records" is still at `:1127-1128`.
_Carried unchanged — source byte-identical since 9eb4dff._

### IN-02: `filter_user_hooks` keeps any event entry that has no `hooks` array

**File:** `src/launcher/config_dir.rs:476-480`
**Issue:** `let Some(Value::Array(inner)) = entry.get_mut("hooks") else { return true; }` retains
a flat or malformed entry (for example `{"type":"command","command":"..."}` directly in the event
list) unfiltered, so the D-19 filter is bypassed for that shape. Claude Code probably ignores such
an entry, but the filter should not rely on that.
**Fix:** Retain only entries that are not objects with a `hooks` key at all if they are provably
empty; otherwise drop entries whose shape the filter cannot judge (`return false`).
**Re-check (2026-10-29):** Status: resolved in effect, verified at HEAD. `settings_transform` now
runs `well_formed_hooks` before the filter (`src/launcher/config_dir.rs:617-619`), and
`is_well_formed_hook_entry` (`:365-378`) rejects every entry that is not an object with an array
`hooks`, so a flat entry never reaches `filter_user_hooks` (sole caller, `:625`). The
`return true` at `:556-558` is now an unreachable branch (private function, one caller); it could
become `return false` for defence in depth but no longer lets a hook bypass D-19. Not counted as
open.
_Carried unchanged — source byte-identical since 9eb4dff._

### IN-03: `next` refusal escapes the target twice

**File:** `src/next_cli.rs:150-156`
**Issue:** The value is passed through `render_for_terminal` and then formatted with `{shown:?}`.
`Debug` re-escapes the backslashes the first pass produced, so a hostile target prints as
`\\x1b...` style text instead of the single escape the sibling code paths show. Cosmetic only; no
control character reaches the terminal either way.
**Fix:** Print with `"{shown}"` inside explicit quotes, or drop `render_for_terminal` and rely on
`{:?}`.
**Re-check (2026-10-29):** Still present. `src/next_cli.rs` is unchanged since the diff base;
`let shown = render_for_terminal(target).to_string();` (`:152`) is still formatted with
`{shown:?}` (`:154`).
_Carried unchanged — source byte-identical since 9eb4dff._

### IN-06: a refused or link-flipped run lock surfaces as "Invalid argument" with the cause discarded (new, 2026-10-29)

**File:** `src/driver/lock.rs:437-452`; `src/launcher/stop.rs:106`; `src/launcher/push.rs:941-945`
**Issue:** `probe_at` collapses every `InvalidData` refusal (symlink, FIFO, directory, hard link)
into `LockProbe::Unknown(Errno::INVAL)`. The two callers that report it print
`could not probe the run lock: Invalid argument`, which does not tell the operator that
`launcher.lock` is a link or a FIFO (the `open_lock_file` message that does say so is thrown away).
Together with WR-08 an operator cannot tell why `driver stop` refuses a live run.
**Fix:** Add the refusal text to the `Unknown` payload (or a `LockProbe::Refused(String)` variant)
and print it in `stop_run` and the push preflight; keep `Unknown(EINVAL)` only for the genuine
`EINVAL` case.
_Carried unchanged — source byte-identical since 9eb4dff._

---

## Re-check of prior finding WR-02

### WR-02: `managed_settings` lets a non-object host `hooks`/`permissions` replace the launcher's gate and deny list

**File:** `src/launcher/config_dir.rs:583-616`
**Issue:** (as recorded) a host managed document whose `hooks` or `permissions` is a non-object
value fell through to the catch-all arm and overwrote the launcher's gate hooks or `DENY_TOOLS`.
**Fix:** Never let a launcher-owned key be replaced by the catch-all.
**Re-check (2026-10-29):** Status: fixed, verified at HEAD (quick 261028-t3f, commit 6a55713;
extended by 261028-v2z). `managed_settings` now has `("hooks" | "permissions", _) => {}` at
`src/launcher/config_dir.rs:718`, placed after the two object arms (`:682`, `:694`) and before the
catch-all (`:719`), so a non-object `hooks`/`permissions` can never reach `settings.insert`. The
degraded path is covered by `settings_transform` (`:608-613` replaces a non-object `hooks` with
`{}`; `:637-639` replaces a non-object `permissions`). Tests
`a_non_object_host_hooks_or_permissions_never_replaces_the_managed_gate` and
`..._the_degraded_gate` (`config_dir.rs:984-1042`) plus the seed test in
`tests/launcher_config_dir.rs:964-1000` pin every shape in both postures. Not counted as open.
_Carried unchanged — source byte-identical since 9eb4dff._

---

_Reviewed: 2026-10-29_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
