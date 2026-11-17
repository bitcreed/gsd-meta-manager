---
phase: 39-check-actions-through-gsd
plan: 03
subsystem: owner-inbox-tui
tags: [rust, ratatui, settle, clipboard, osc52, oracle, source-scan, flow-matrix, docs]
requires:
  - phase: 39-01
    provides: owner_settle core, settle_popup screens, launch seam
  - phase: 39-02
    provides: check rows, Owner block x, Move
provides:
  - "live-session Yes: focus the session and put the prompt on the clipboard (native tool or OSC 52), nothing typed into it (OWN-30, D-11)"
  - "ScreenAction::PopWriteOsc52, AppContext::start_copy_led(_with), owner_actions::copy_led"
  - "tests/owner_settle_guard.rs: the settle modules contain no write or spawn API (OWN-31, D-13a)"
  - "tests/owner_settle_oracle.rs: gsd-core accepts the product-built argument and the requested deferral encoding (OWN-31, D-13)"
  - "12-cell outcome x host x target flow matrix with tree snapshots (D-13b)"
  - "x in the help overlay, the widest inbox footer rung and the README (OWN-29, D-05)"
affects: []
status: complete
commits: 3
plan_head_before: 38c9a2f2e8c4fb10fb00e432836fc7ab81f419df
plan_head_after: 59aa92d371eba202d5f6f1b26134a3a82e5d2020
actuals:
  tokens: 40000
  tasks: 3
  commits: 3
key-files:
  created:
    - tests/owner_settle_guard.rs
    - tests/owner_settle_oracle.rs
  modified:
    - src/ui/screens/mod.rs
    - src/app.rs
    - src/ui/screens/owner_actions.rs
    - src/ui/screens/settle_popup.rs
    - src/ui/screens/owner_block.rs
    - src/ui/screens/help.rs
    - src/ui/screens/inbox.rs
    - README.md
key-decisions:
  - "Pitfall 8 solved with one narrow ScreenAction::PopWriteOsc52 variant, not an AppContext mailbox (App already owns pending_osc52; one action per key)"
  - "Oracle policy copied from owner_acknowledge_roundtrip: fail loud without gsd-core unless GSD_META_MANAGER_ALLOW_MISSING_ORACLE is non-blank; scripts/install-conformance-oracle.sh already provisions the npm gsd-core (with workflows/)"
requirements-completed: [OWN-29, OWN-30, OWN-31]
---

# Phase 39 Plan 03: Live-session copy, read-only proof, discoverability Summary

Yes with a live Claude session now focuses it and puts the exact prompt (slash command plus instruction, never
the `claude` argv) on the clipboard; the read-only claim is proven by a source scan, a gsd-core oracle round trip
on temp copies and a 12-cell flow matrix; `x` is in the help overlay, the widest footer rung and the README.

## Tasks

1. Tracer: `PopWriteOsc52`, `start_copy_led(_with)`, `copy_led`, the live-session branch of
   `SettleConfirmScreen::confirm` (5180ffb0)
2. Read-only proof: `owner_settle_guard`, `owner_settle_oracle`, the flow matrix, `GroupKey::of` public (e897edef)
3. Help rows, footer key, README (59aa92d3)

## Verification

- `rtk bypass cargo test --no-fail-fast` (raw `test result:` lines summed over 69 suites): 4568 passed, 0 failed,
  19 ignored (lib 3704).
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt` applied. No file under `tests/fixtures/` changed
  relative to the plan base (38c9a2f2).
- Oracle (gsd-core 1.21.0, installed at `~/.claude/gsd-core`): 4 tests pass, no SKIP. `init.verify-work` finds the
  phase for all 6 cases (3 outcomes x phase/check) on `heuristic-labels` 01, the `workstreams` alpha phase 03 and
  the `workstreams` root phase 01, resolving the workstream to the command's own; verify-work's two argument-parse
  pipelines are pinned verbatim and asserted against the installed `verify-work.md`; no `xargs:` warning, also
  with a hostile name and reason (`x --ws evil won't a "b" \ end`, after the product's `clean`); the requested
  encoding is `deferred` for `audit-uat`, passing and `deferred: true` for `phase uat-passed --uat-only`, and test
  2 leaves the product's TEST lane.
- Guard: 1261 executable non-test lines scanned over the two settle files (floor 400, per-file floor 50); 15
  markers each caught when planted and ignored behind `//`; `#[cfg(test)]` mod/fn items are skipped.
- Flow matrix: all 12 cells launch the expected prompt and leave the copy byte-identical.
- Live-session: OSC 52 payload equals `osc52(prompt)` and the prompt equals the 39-01 tracer prompt; with a forced
  wl-copy the seam holds `[(WlCopy, prompt, 1)]`; Owner block check flow copies that check's prompt.

## Deviations from Plan

**1. [Rule 1 - Bug in plan] Footer rung 2 would push the 100-column footer down**
- **Found during:** Task 3 full lib run (`the_inbox_renders_sections_in_order_at_100x30` failed: `[A]cknowledge`
  became `[A]ck`).
- **Issue:** the plan adds `[x]settle` to rungs 1 and 2, making rung 2 105 cells, which no longer fits at 100
  columns (the UI-SPEC golden width); the plan's bound was only the 80-column rung.
- **Fix:** `[x]settle` is on the widest rung only (137 cells; new `FooterKey::rung2` flag). Rungs 2-4 are exactly
  as before, so 100 and 80 columns are unchanged. The ladder test pins 137/136 for rung 1/2 and the old 94/93
  boundary. Discoverability of `x` on narrower screens is by the help overlay and the README.
- **Commit:** 59aa92d3

**2. [Rule 1 - Bug] Popup consequence row clipped**
- **Issue:** the new live-session consequence text (96 chars) exceeded the popup's inner width and was cut off.
- **Fix:** `settle_body` wraps the consequence row at spaces. The live-session popup is one row taller (the 80x24
  body budget had 2 rows of slack).
- **Commit:** 5180ffb0

**3. [Test shape] Decoded OSC 52 payload**
- The test compares the sequence to `clipboard::osc52(&prompt)` (the base64 encoder is injective, no decoder
  exists in the crate) and checks the framing, instead of decoding the base64.

**4. [Rule 3] `Stack` in the test helpers**
- `settle_popup::tests::Stack` gained a held-OSC-52 slot and applies `PopWriteOsc52` as App does (pop + hold).

## Inferred decisions

- [inferred, plan] One narrow `ScreenAction::PopWriteOsc52` instead of an `AppContext` mailbox.
- [inferred, plan, D-11] The copy is made even when focusing fails (the status then leads with the focus failure);
  the clipboard holds the slash command plus instruction.
- [inferred, plan] The oracle's model round trip is human UAT; only what gsd-core decides deterministically is automated.
- [inferred] Oracle policy: same as `owner_acknowledge_roundtrip` (fail loud without gsd-core, opt-out env var).
  `scripts/install-conformance-oracle.sh` installs the gsd-core npm package with its `workflows/`, so the
  publish job has the oracle and `verify-work.md`; a `node` install lacking `verify-work.md` fails the pin test
  loudly, only a bare `gsd-tools` on PATH skips it. The script header lists consumers by name and was left alone.
- [inferred] `GroupKey::of` is `pub` (plan); the oracle builds keys through the product.
- [inferred] `phase_number` is compared with leading zeros trimmed (`1` and `01` name the same phase for gsd-core).
- [inferred] The source scan skips any `#[cfg(test)]` item (module, fn, thread_local), not only modules, because
  both settle files carry cfg(test) seams outside their test modules.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register (T-39-15..20 mitigated by the tests above).

## Human UAT (carried forward unchanged for /gsd-verify-work; not blocking)

On a throwaway project (a copy of a fixture with a git repo) in the owner's real multiplexer (herdr, else tmux), with the watcher running:
1. Won't do on a phase row (inbox) and on one check (Owner block): the launched session resumes the UAT session without asking resume/restart, shows no `xargs:` warning, records `result: skipped` with a `Deferred follow-up: will not do.` reason, offers no 999.x promotion; after the watcher refresh the row (or check) leaves the TEST lane.
2. Deferred permanently, same checks.
3. Move to phase M: the check is recorded as `Deferred follow-up: moved to phase M.`, one todo appears under `.planning/todos/pending/` whose title names phase M's number and name, and `gsd-tools query todo.match-phase M` lists it; the source phase can close without it.
4. A project with a live Claude session: Yes switches to it, the status says to paste, pasting runs the same instruction; no second session appears. (Also: the status line text reads right; OSC 52 reaches the real terminal clipboard.)
5. A VERIFICATION-only phase entry (RESEARCH Pitfall 11): whether its human checks leave the list; if not, record it (A remains the file-level answer).
6. The instruction wording of all three outcomes reads right to the owner; the popup is readable at 80x24 and 100x30 (the live-session popup is one row taller than the others).
7. Footer: `[x]settle` shows only at 137+ columns; confirm that is acceptable for discoverability, or widen the ladder.
- Edge probe OWN-29 (unclassified) stays flagged for human review (from 39-01).

## Self-Check: PASSED

Commits 5180ffb0, e897edef, 59aa92d3 are ancestors of HEAD; tests/owner_settle_guard.rs and
tests/owner_settle_oracle.rs exist.
