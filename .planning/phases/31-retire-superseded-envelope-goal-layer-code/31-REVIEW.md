---
phase: 31-retire-superseded-envelope-goal-layer-code
reviewed: 2026-10-27T00:00:00Z
depth: standard
files_reviewed: 20
files_reviewed_list:
  - README.md
  - docs/CONFIGURATION.md
  - docs/GSD-CORE-SYNC.md
  - src/config.rs
  - src/envelope/ledger.rs
  - src/error.rs
  - src/launcher/process.rs
  - src/launcher/push.rs
  - src/launcher/push/planted.rs
  - src/registry.rs
  - src/state_reader/config_json.rs
  - src/text.rs
  - src/ui/mod.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/render_escape_guard.rs
  - tests/launcher_sandbox.rs
findings:
  critical: 0
  warning: 0
  info: 6
  total: 6
status: issues_found
---

# Phase 31: Code Review Report (incremental re-review)

**Reviewed:** 2026-10-27
**Depth:** standard
**Files Reviewed:** 20
**Status:** issues_found

## Summary

Scope: `git diff c204aff HEAD` for the 20 listed files. This is an incremental re-review of the
prior report, whose five warnings were the target of gap-closure plans 31-08 (36e0805, 12b8a1d,
89f4b3b) and 31-09 (6de4572, 21b94df, 832c58f, a51b39a).

**Provenance of the diff.** Four of the files did not change through phase-31 task commits. They changed
through quick task 261027-184 (gsd-core v1.19.0 re-sync, commits 3cda6eb, 6a8cf8f, 2ea7d77):
`README.md`, `docs/GSD-CORE-SYNC.md`, `src/state_reader/config_json.rs` and `src/ui/screens/detail.rs`. I
read them in full. They are internally consistent. The new `git.repo_tagging` key has a typed field, a
Defaults row, set, cycle and clear arms, a built-in default, a `since` marker, and a test that pins its JSON
path. The three baseline constants, the README note and `GSD-CORE-SYNC.md` agree on 1.19.0.

**Prior WR-01 through WR-05 are resolved.** I checked each against the current source, not the commit
messages.

- **WR-01 (retired-seam disclosure on the grant popup): resolved.** `render` now builds a body of the
  consent prompt only. `SECTION_*` and `render_disclosure` are gone, and the test
  `the_grant_popup_shows_no_disclosure_of_retired_seams` asserts "seam", "digest", "CLAUDE.md" and "=="
  are absent. The `PromptInputsDrifted` message no longer promises a re-confirmation. The doc comments in
  `registry.rs` and `error.rs` now say the digests are recorded but not consulted. The `config.rs` docs
  still carry the stale claim (see IN-07).
- **WR-02 (branch namespace claimed as applying): resolved.** Both prompts, `docs/CONFIGURATION.md` and the
  `config.rs` field doc now say the namespace is parsed and validated but not consulted. The code agrees:
  `policy_for` feeds only caps and `dest_ref` hard-codes the default namespace. A test forbids "namespace"
  in either prompt.
- **WR-03 (unregister not guarded against a live launcher run): resolved.** `do_remove_project` refuses when
  `ctx.experimental` holds and the cached `launcher_marker.any_live` is true. It reports through
  `error_message`, writes no config, and leaves the project registered. Two tests cover the refusal and the
  crashed-run case. The guard's coverage limits are IN-10.
- **WR-04 (ledger message cited a deadline nothing enforces): resolved.** `COUNT_DEADLINE_SECS` is removed.
  The refusal states the size bound only, and a test forbids "deadline", "second", "timeout" and
  "time limit" in the text. The sizing figure survives as a test-only `COUNT_BUDGET_SECS`. The module docs no
  longer describe an agent that can inflate the ledger through the retired guard.
- **WR-05 (SAFE-04 redaction bypass on a long stderr line): resolved.** `last_line_of_tail` drops the partial
  first line of a cut window and returns a fixed note when no whole line remains. `redact_captured`
  redacts first and caps at 512 chars afterwards. I traced the boundary cases: a window cut mid-line, a cut
  at a line start, a single overlong line with and without a trailing newline, and `len == 4096` (not cut).
  All behave correctly. The end-to-end test builds a 4207-byte stderr line with a `ghp_` token cut by the
  window and asserts no 8-byte run of the secret survives.

**Other changes in the window, no defects found.**

- `planted.rs`: the budgeted walks list each directory through `list_dir_bounded`, sorted by name, and
  truncate a directory whole when it exceeds the remaining budget. `room().saturating_add(1)` bounds
  buffering. Reversing the subdirectory list before pushing makes the stack pop in ascending order. The
  new order-independence test is non-vacuous: it builds the trees in opposite orders and checks
  name-order prefixes at every limit from 0 to 40.
- `tests/launcher_sandbox.rs`: the test now pins `spec.runtime_dir` instead of reading the host's
  `XDG_RUNTIME_DIR`, which fixes the container-only failure.
- `ui/mod.rs`, `screens/mod.rs`, `render_escape_guard.rs`, `driver.rs`, `text.rs`: comment and census-pin
  updates only. The `MEASURED_REACH` table and the `PRODUCTION_NEEDLE_*` figures agree with the stated
  removal of the two composed calls from `driver_confirm.rs`.

No critical issues and no warnings. The items below are documentation and maintainability residue.

## Info

### IN-03: Opt-in toggle popup dims the dashboard even though it is only reachable from the Driver tab

**Carried over from the prior review, unchanged.**
**File:** `src/ui/screens/driver_confirm.rs:189-196`
**Issue:** `ToggleOptIn` is constructed only at `detail.rs:5991` (Driver tab), so the user is in a detail view.
The popup still renders `NormalScreen` behind it, and the comment still justifies this with "as it did when
the dashboard also opened it". The background changes under the user when they press `o`.
**Fix:** Render `DetailScreen::new(self.alias.clone()).render_main_only(frame, area, ctx)` for both actions,
drop the `matches!` branch, and drop the `NormalScreen` import path from this screen.

### IN-05: Leftover naming and attributes from retired code

**Carried over from the prior review. Only the `driver.rs` part of the original item remains in this scope.**
**File:** `src/ui/screens/driver.rs:98`
**Issue:** `DRY_RUN_LOADING` is still named for the retired dry-run preview. It is now the generic
"Loading..." string used at `driver_run.rs:990, 1724, 2101, 3075`. The other parts of the original item
(`journal/redact.rs`, `cli.rs`) are outside this re-review's file scope and were not re-checked.
**Fix:** Rename it to `LOADING_TEXT` and update the `driver_run.rs` import at line 37.

### IN-07: `DriverOptIn` docs in `config.rs` still say the drift check runs at the spawn gate and that drift re-confirms

**File:** `src/config.rs:274-282, 306-323, 344, 414`
**Issue:** This is residue of the prior WR-01. The 31-08 fix corrected `registry.rs` and `error.rs`, but
the doc comments on the data type still read as live behaviour. Examples:

- "Phase 21 is that phase, and it has landed ... the drift check runs at the spawn gate."
- `prompt_inputs`: "all mean the same thing: re-confirm."
- `claude_md_digest`: "an empty `prompt_inputs` re-confirms."

The only reader is `DrivableProject::from_registry`, which has no production caller (per the new `error.rs`
and `registry.rs` docs). A maintainer reading `config.rs` first will believe a control exists. The new
docs elsewhere in the same change set say it does not.
**Fix:** Add one paragraph to the `DriverOptIn` doc: the digests are recorded at opt-in but nothing on the
launcher path compares them. Then qualify the "re-confirm" sentences with "(legacy executor gate only)".

### IN-08: `delete_confirm.rs` re-implements the "driver is live" predicate that `detail.rs` already has

**File:** `src/ui/screens/delete_confirm.rs:131-135`, duplicating `src/ui/screens/detail.rs:784-791`
**Issue:** `detail::driver_live_for(ctx, alias)` is the same expression: `ctx.experimental && view_cache
-> launcher_marker -> any_live`. The delete guard repeats it inline. If the definition of "live" changes
(for example to include a stale lock), the badge and the unregister guard can diverge, and the guard is the
safety control.
**Fix:** Make `driver_live_for` `pub(super)`, or move it to `screens/mod.rs`, and call it from
`do_remove_project`:

```rust
if super::detail::driver_live_for(ctx, alias) { /* refuse */ }
```

### IN-09: `OVERLONG_STDERR_NOTE` hard-codes the window size as a literal

**File:** `src/launcher/process.rs:147-148`
**Issue:** The note says "longer than 4096 bytes". `STDERR_TAIL_BYTES` is the source of truth. If someone
retunes the window, the note silently becomes false, which is the class of defect the WR-04 fix removed
from the ledger message. The module test also hard-codes 4096 in a comment, which is fine, but the
production string is not.
**Fix:** Drop the number ("the last stderr line was too long to keep and is not kept") or build the string
from the constant with `format!` at the use site.

### IN-10: The unregister guard is advisory: it covers the cached scan only, and its limits are not stated to the user

**File:** `src/ui/screens/delete_confirm.rs:121-142`
**Issue:** The guard reads the cached marker. It does not protect these cases:

- Before the first marker scan lands, the cache entry is `None`.
- A run started within the last slow-tick window is not yet in the cache.
- A user who turns off the experimental flag while a run is live. `ctx.experimental` is part of the
  condition, so removal proceeds and the detached launcher keeps running.

The code comment discloses the first two. Nothing is disclosed to the user, and the refusal message reads
as if the check were authoritative. The consequence is the same hazard as before, in a narrower window: a
removed project with a live run and no TUI handle.
**Fix:** Either re-read the run locks synchronously at confirm time (one `launcher_markers_for` call for
this alias), or document the window beside the guard and in `docs/CONFIGURATION.md`. Dropping the
`ctx.experimental` clause is also reasonable here, since refusing to unregister a project with a live lock
is safe even when the flag is off.

---

_Reviewed: 2026-10-27_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
