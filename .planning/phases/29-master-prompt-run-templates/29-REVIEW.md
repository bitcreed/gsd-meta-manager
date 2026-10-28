---
phase: 29-master-prompt-run-templates
reviewed: 2026-10-28T00:00:00Z
depth: standard
files_reviewed: 26
files_reviewed_list:
  - scripts/launcher-acceptance.sh
  - scripts/prompt-acceptance.sh
  - src/cli.rs
  - src/driver/direction.rs
  - src/driver_cli.rs
  - src/driver_templates/assets/DIRECTION.md
  - src/driver_templates/assets/LOG.md
  - src/driver_templates/lint.rs
  - src/driver_templates/mod.rs
  - src/driverctl/serve.rs
  - src/driverctl/store.rs
  - src/launcher/config_dir.rs
  - src/launcher/gate.rs
  - src/launcher/mod.rs
  - src/launcher/prompt.rs
  - src/launcher/run.rs
  - src/main.rs
  - src/next_cli.rs
  - src/state_reader/driver_frontmatter.rs
  - tests/driverctl_frontmatter.rs
  - tests/driverctl_store.rs
  - tests/launcher_caps.rs
  - tests/launcher_config_dir.rs
  - tests/launcher_e2e.rs
  - tests/launcher_support/mod.rs
  - tests/prompt_acceptance.rs
findings:
  critical: 0
  warning: 3
  info: 3
  total: 6
status: issues_found
---

# Phase 29: Code Review Report (re-review at HEAD)

**Reviewed:** 2026-10-28
**Depth:** standard
**Files Reviewed:** 26
**Status:** issues_found

## Summary

Re-review of Phase 29 at HEAD, focused on the diff since `7b413a0` (Phase 28.1, quick tasks,
Phase 31 deletions, cleanup batch 261028-n0u). The prior WR-01..WR-07 fixes hold: the
lint and the store now share the comment and section readers, `control_hold` is one matcher
used by store and gate, `mentions_phase` no longer confuses decimal phases, the DIRECTION
template no longer promises `isolation`, and the capped `open_run_file` reader is used by the
store, the CLI and the preflight. The direction exchange/give-back loop, the socket path
fallback, the `bind_private` hard-link publish, the egress guard cleanup and the frontmatter
round-trip were traced and found sound. No critical defects. Remaining items are one
lint-vs-store disagreement left over from WR-01, one fail-open merge in the managed-settings
builder that repeats the shape IN-03 fixed one level down, and an over-broad hook filter.

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

### WR-02: `managed_settings` lets a non-object host `hooks`/`permissions` replace the launcher's gate and deny list

**File:** `src/launcher/config_dir.rs:583-616`
**Issue:** The merge matches `("hooks", Value::Object(..))` and `("permissions", Value::Object(..))`.
A host managed document whose `hooks` or `permissions` is any other JSON value (string, array,
`null`) falls through to the catch-all arm at line 613 and does `settings.insert(key, value)`,
overwriting the launcher's `hooks` (the whole dispatch gate, Stop guard and heartbeat) or
`permissions` (the `DENY_TOOLS` list). IN-03 closed exactly this for a non-array list one level
down and its comment states "the safety list must not depend on Claude Code refusing such a
file"; the same dependency remains at the top level. `read_host_managed` only requires each file
to be a JSON object, so such a file is accepted. If Claude Code tolerates the value, the run
proceeds with no gate in the managed tier and, since the per-run `settings.json` carries no
driverctl entry in the bwrap posture, no gate anywhere.
**Fix:** Never let a launcher-owned key be replaced by the catch-all:
```rust
("hooks", _) | ("permissions", _) => continue, // launcher-owned; a non-object host value is ignored
```
placed after the two object arms, plus a test mirroring
`a_non_array_host_deny_list_does_not_replace_the_launchers` for `{"hooks": "x"}` and
`{"permissions": null}`.

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

### IN-02: `filter_user_hooks` keeps any event entry that has no `hooks` array

**File:** `src/launcher/config_dir.rs:476-480`
**Issue:** `let Some(Value::Array(inner)) = entry.get_mut("hooks") else { return true; }` retains
a flat or malformed entry (for example `{"type":"command","command":"..."}` directly in the event
list) unfiltered, so the D-19 filter is bypassed for that shape. Claude Code probably ignores such
an entry, but the filter should not rely on that.
**Fix:** Retain only entries that are not objects with a `hooks` key at all if they are provably
empty; otherwise drop entries whose shape the filter cannot judge (`return false`).

### IN-03: `next` refusal escapes the target twice

**File:** `src/next_cli.rs:150-156`
**Issue:** The value is passed through `render_for_terminal` and then formatted with `{shown:?}`.
`Debug` re-escapes the backslashes the first pass produced, so a hostile target prints as
`\\x1b...` style text instead of the single escape the sibling code paths show. Cosmetic only; no
control character reaches the terminal either way.
**Fix:** Print with `"{shown}"` inside explicit quotes, or drop `render_for_terminal` and rely on
`{:?}`.

---

_Reviewed: 2026-10-28_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
