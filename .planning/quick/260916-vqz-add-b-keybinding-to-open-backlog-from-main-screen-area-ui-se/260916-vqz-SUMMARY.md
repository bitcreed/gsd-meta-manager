---
quick_id: 260916-vqz
phase: quick-260916-vqz
plan: 01
subsystem: ui
status: complete
tags: [keybinding, tui, backlog, dashboard, help]
requires:
  - "260916-vr0 — the one-rule backlog matcher in src/state_reader/backlog.rs"
provides:
  - "DetailScreen::opened_on(alias, sub_view, ctx) — a detail screen already parked on a tab, with that tab's arrival work done"
  - "`b` on the dashboard: open the selected project's Backlog tab, populated"
affects:
  - src/ui/screens/normal.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/help.rs
tech-stack:
  added: []
  patterns:
    - "A caller that wants to LAND on a detail tab goes through switch_to_tab, never by setting detail_sub_view_per_project itself — the arrival work lives in exactly one place"
    - "Tab indices are derived through tab_index(), never written as literals at a call site"
    - "Key documentation is asserted as WHOLE ROWS, never by contains(key)"
key-files:
  created: []
  modified:
    - src/ui/screens/normal.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/help.rs
decisions:
  - "opened_on runs switch_to_tab rather than pre-parking the sub-view, so the new entry path inherits 260916-vr0's repair instead of carrying a copy of the load rule"
  - "switch_to_tab stays private — both callers are inside detail.rs and the field-level borrow split works from there"
  - "Reaching Backlog by `b` is STICKY per project, like every other tab switch in the app"
metrics:
  duration: ~35min
  completed: 2026-09-17
commits: 4
plan_head_before: 2c29351a1aa93fbc2d38b7f4dfb02b0f05a4a7d3
actuals:
  tokens: 4268
  tasks: 2
  commits: 4
---

# Quick Task 260916-vqz: Add `b` keybinding to open backlog from the main screen — Summary

`b` on the dashboard now opens the selected project's detail view already on the
Backlog tab with items loaded, by routing through the same `switch_to_tab` the
`3` key uses — proved by an equivalence test rather than by inspection.

## What shipped

**`DetailScreen::opened_on(alias, sub_view, ctx)`** (`src/ui/screens/detail.rs`)
— a `pub(crate)` constructor that builds `Self::new(alias)` and then runs the
target tab's arrival work by calling `switch_to_tab` with `tab_index(&sub_view)`.
It does **not** write `detail_sub_view_per_project` itself: arrival work for every
tab (backlog parse, git-log spawn, defaults load, browser lazy-init, run scan)
lives in `switch_to_tab` and nowhere else, so a hand-parked screen would paint a
blank tab on first frame *and* would be a second copy of a rule free to drift.
The index comes from `tab_index`, never from the literal `2`.

**The `b` arm** (`src/ui/screens/normal.rs`) — placed beside `KeyCode::Enter`,
not beside the three driver keys, because it is a second way to open the *same*
screen. Guards on `ctx.selected_alias()` exactly as `Enter` does; sets
`detail_scroll_offset = 0` and `needs_redraw = true`. Both collision-check
comment blocks now list `b`, with a note recording that the lists are kept
current deliberately.

**The help row** (`src/ui/screens/help.rs`) — `row("b", "Open project detail on
the backlog tab")`, directly under `Enter`, asserted as a whole row built through
the same `row()` helper the body uses.

## Verification

Measured on this tree, after the change:

| Gate | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo test --no-fail-fast` | **2087 passed / 1 failed / 15 ignored** |
| `cargo clippy -- -D warnings` | exit 0 |

The single failure is the documented environmental one:
`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
(constants derived against git 2.43, installed git is 2.53). It is in the
pre-change baseline and touches nothing this task changed.

Per-suite counts against the baselines measured on this same tree before any
edit:

| Suite | Before | After |
|---|---|---|
| `ui::screens::normal::tests::` | 32 | 35 (+3) |
| `ui::screens::detail::tests::` | 103 | 104 (+1) |
| `ui::screens::help::tests::` | 9 | 9 |
| lib total | 1310 | 1314 (+4) |

The `+4` is exactly the four tests added, so the failing set did not grow.

Note on the plan's `<done>` numbers: it predicted 74 detail tests and 32 normal
tests. 32 was right; the detail baseline was **103**, not 74 — siblings
260916-vqy and 260916-vr1 merged tests into `detail.rs` after this plan was
written. The measured baseline is the one used above.

## TDD Gate Compliance

**RED** (`e3b92d1`) — tests committed alone, observed failing, verbatim:

```
---- b_on_the_dashboard_opens_the_detail_screen_on_a_populated_backlog_tab
panicked at src/ui/screens/normal.rs:2215:18:
`b` with a project selected must push the detail screen

---- b_and_the_3_key_reach_the_backlog_through_one_load_path
assertion `left == right` failed: both entry paths must record the same sub-view
  left: None
 right: Some(Backlog)

test result: FAILED. 33 passed; 2 failed
```

The second failure's right-hand side is load-bearing: `Some(Backlog)` was
MEASURED from the `3` key against the same fixture in the same run, which proves
the fixture reaches real files on disk before it is used to judge `b`.

Test 3 (`b_with_no_project_selected_does_nothing`) passed at RED, correctly — an
unbound key already falls through to the catch-all arm. It is a guard against the
binding landing *without* its `selected_alias()` guard, not a RED row, and is
recorded as such in the RED commit message.

Test 4 (`switching_to_the_backlog_tab_returns_no_screen_action`, `detail.rs`)
also passed at RED by construction. It exists because `opened_on` **discards**
`switch_to_tab`'s return value: the pin goes red the day that function learns to
return a `Push`, which is the day the discard stops being safe.

**GREEN** (`1e4ed00`) — implementation only; normal 35/0, detail 104/0, clippy 0.

**Task 2 fail-first proof** — `help_lines_documents_every_key_this_phase_binds`
was proved to fail before commit by commenting the new `row("b", …)` out of
`help_lines()`:

```
failures:
    ui::screens::help::tests::help_lines_documents_every_key_this_phase_binds
test result: FAILED. 0 passed; 1 failed
```

Restored, re-run green. An assertion that cannot fail documents nothing.

## Tracer feedback gate

Task 1 was `type="tracer"`. Auto mode (human unavailable): `<verify>` re-run
end-to-end after the task commit — `cargo test --lib --no-fail-fast
ui::screens::normal::tests:: ui::screens::detail::tests::` green — so execution
continued into Task 2 rather than expanding onto an unproven slice.

## Precondition

The plan's precondition was that **260916-vr0 has merged into this tree's base**.
Verified, and the shape of its repair matters: vr0 fixed the matching rule in
`src/state_reader/backlog.rs` (`backlog_dirs` is now the sole definition, and the
`.md` file inside a `999.N-slug` directory became optional), *not* the Backlog arm
of `switch_to_tab`. Since `switch_to_tab` calls `backlog::parse_backlog_items`,
the new `b` entry path inherits that repair automatically — it did not need to
carry a copy, and does not.

## Threat model

| Threat ID | Disposition | Outcome |
|---|---|---|
| T-vqz-01 | mitigate | **Mitigated.** The `b` path reaches `backlog_items` only through `switch_to_tab` → `backlog::parse_backlog_items`, the one site that wraps `dir_name`/`number`/`description` in `Untrusted`. No load code was written in `normal.rs`. `b_and_the_3_key_reach_the_backlog_through_one_load_path` is the enforcement: it compares the two entry paths' items by `dir_name` and asserts non-emptiness, so a second, hand-rolled load path would have to reproduce the producer exactly to pass. |
| T-vqz-02 | accept | Unchanged. `opened_on` is `pub(crate)` and its only caller passes `ctx.selected_alias()`. |
| T-vqz-03 | accept | Unchanged. `b` adds a second trigger for the synchronous `read_dir` the `3` key already performs — not heavier work. |

No `Cargo.toml` change, so no package-install checkpoint applied.

## Inferred decisions (for audit)

1. **Backlog becomes the project's sticky sub-view after `b`** (carried forward
   from the plan's own recorded inference). `switch_to_tab` records the tab for
   every switch in the app, so a later `Enter` on that project lands on Backlog.
   A non-sticky variant would need a second, differently-shaped state field, and
   the source todo asks to land on Backlog "instead of whatever tab was last
   active" — it does not ask for the previous tab to be restored afterwards.
2. **The help description is "Open project detail on the backlog tab"**, not the
   plan's suggested "Open the backlog tab (dashboard)". Reason: it reads as a
   sibling of the `Enter` row directly above it ("Open project detail"), which is
   what the two rows actually are, and it is not byte-identical to any other row
   in the body — the constraint the whole-row assertions impose.
3. **The help assertion was placed beside the `Toggle sort` assertion**, not
   appended to the array above it. That array is the *Driver tab* section's list;
   `b` is a dashboard key, so widening it would misfile the row. The plan
   explicitly permitted this choice.
4. **The plan's stated detail-test baseline (74) was treated as stale** and the
   measured value (103) used instead, rather than treating the mismatch as a
   regression. Siblings vqy and vr1 merged detail.rs tests after this plan was
   written.

## Known Stubs

None.

## Deviations from Plan

**None.** Both tasks executed as written. The only numeric departure is the
detail-suite baseline (103 measured vs. 74 predicted), recorded above as an
inferred decision rather than a deviation — no plan instruction was changed.

## Commits

| Commit | Message |
|---|---|
| `e3b92d1c4bba8d2db97a1d3f7a8914148ff8c2e7` | test(quick-260916-vqz): pin `b` -> populated Backlog tab, RED |
| `1e4ed00987c66220a2a07e02dcb985018bf3de98` | feat(quick-260916-vqz): bind `b` to open the Backlog tab, already loaded |
| `021bda5a539c47066723b0008117b47c4d81f52e` | feat(quick-260916-vqz): document `b` on its own help row |
| `d256191bece458e9ff53bd5982e7a3746d7d7698` | chore(quick-260916-vqz): retire the backlog-keybinding todo |

## Self-Check: PASSED

- `src/ui/screens/normal.rs` — FOUND (modified)
- `src/ui/screens/detail.rs` — FOUND (modified)
- `src/ui/screens/help.rs` — FOUND (modified)
- `.planning/todos/completed/2026-09-15-add-b-keybinding-to-open-backlog-from-main-screen.md` — FOUND
- All four commit hashes — FOUND in `git log`
- `git rev-list --count 2c29351..HEAD` — 4, matching the `commits:` field above

## Coordinator inferred decisions (for audit) — quick-batch 260916-vqv resume

The human operator was unavailable for this resume run. The batch coordinator made these
calls from the planning artifacts alone; none was confirmed by a human.

1. **Base-revision divergence reconciled rather than refused.** `quick-batch resume` refused
   closed with `batch 260916-vqv base revision diverged: created against be80701e…, current is
   89fdea87…` (ADR-1239 § Base divergence). All 20 commits in that range belong to this batch
   itself and `be80701e` is a strict ancestor of `89fdea87`, so the coordinator advanced
   `BATCH.json.base_revision` to `89fdea873f1e14433da70165798588c7e44fa211` and resumed — the
   reconciliation `resumeBatch`'s own contract delegates to the caller.
2. **This item ran SECOND and sequentially, not in parallel with 260916-vqy.** Both items declare
   `src/ui/screens/detail.rs` in `files_modified`; the coordinator forced the mutating wave to
   concurrency 1 and dispatched in the batch's own wave order (vqy is wave 3, this item is wave 4)
   so this item's executor read `detail.rs` with vqy's changes already merged.
3. **This item ran UNISOLATED, inline on the primary checkout — no worktree.** Immediately before
   this dispatch `worktree.base-check --mode harness-worktree` returned
   `shouldDegrade: true, reason: "baseref-head-ignored-by-harness"` — after vqy's merge, local HEAD
   was `2c293512` while the harness still forks isolated worktrees from `origin/HEAD` (`89fdea87`,
   unpushed). Per the workflow's #1941/#48 auto-degrade the coordinator set `ISOLATION=none`,
   `USE_WORKTREES=false`, re-recorded the run-scoped isolation sentinel as `none` (so the
   PreToolUse isolation guard would not deny a flagless dispatch), and dispatched the executor
   inline with an explicit dirty-tree constraint (stage only your own paths; never `git add -A`).
   Consequently Step 7 (deterministic merge) was skipped for this item — there was nothing to
   merge; the commits landed directly on `dev`.
4. **Verification wave skipped.** `init.quick-batch` reports `section_manifest.excluded` =
   [research-phase, verification-wave] and the run carried no `--validate`, so no `gsd-verifier`
   was dispatched and no `VERIFICATION.md` exists for this item — unlike the four items merged in
   the batch's earlier, validated run. The coordinator substituted its own build gate instead
   (see 5).
5. **Build gate run by the coordinator on the final tree:** `cargo build` pass; `cargo clippy --
   -D warnings` pass; `cargo test --no-fail-fast` → 48 suites, 2087 passed, 1 failed — only the
   known-environmental `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
   (installed git 2.53 vs constants derived against 2.43).
6. **One flaky failure investigated and dismissed.** A first full run also failed
   `driver::run::tests::the_current_group_agrees_with_the_proc_parse` (`getpgrp()` 300296 vs the
   /proc pgrp field 300318). It passed 3/3 in isolation, passed on the immediately following full
   run, and this item touched no file under `src/driver/`. Classified as a process-group flake in
   the shared test binary, not a regression.
7. **Pre-existing lint debt left alone.** `cargo clippy --all-targets -- -D warnings` fails with 4
   errors in `tests/envelope_carrier_reach.rs`, `tests/envelope_config_resolution.rs`,
   `src/browser.rs` and `src/project_creator.rs` — all files untouched by this batch. The gate this
   run enforced is the plain `cargo clippy -- -D warnings`, which passes. Flagged, not fixed.
