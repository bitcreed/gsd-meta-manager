---
phase: 29-master-prompt-run-templates
reviewed: 2026-10-29T00:00:00Z
depth: standard
files_reviewed: 12
files_reviewed_list:
  - docs/DRIVER-SANDBOX.md
  - src/driver/direction.rs
  - src/driver/lock.rs
  - src/driverctl/serve.rs
  - src/driverctl/store.rs
  - src/launcher/config_dir.rs
  - src/launcher/run.rs
  - src/launcher/stop.rs
  - src/state_reader/driver_frontmatter.rs
  - tests/driverctl_serve.rs
  - tests/driverctl_store.rs
  - tests/launcher_config_dir.rs
findings:
  critical: 0
  warning: 5
  info: 3
  total: 8
new_findings:
  critical: 0
  warning: 3
  info: 1
status: issues_found
---

# Phase 29: Code Review Report (incremental re-review at HEAD)

**Reviewed:** 2026-10-29
**Depth:** standard
**Files Reviewed:** 12
**Status:** issues_found

## Summary

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

## Warnings

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

## Info

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

---

_Reviewed: 2026-10-29_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
