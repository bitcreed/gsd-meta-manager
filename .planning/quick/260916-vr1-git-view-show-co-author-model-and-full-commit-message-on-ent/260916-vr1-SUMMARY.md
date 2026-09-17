---
phase: quick-260916-vr1
plan: 01
subsystem: ui
status: complete
tags: [git-tab, co-authors, commit-message, untrusted, scroll, ratatui]

# Dependency graph
requires:
  - phase: 21-23 / 21-25
    provides: "`crate::text::Untrusted` as the carrier every third-party `git log` field travels in, plus the `render_escape_guard` probe that asserts it reaches no cell raw"
  - phase: 18-09 / 19 (UIFIX-04)
    provides: "`ViewportMetrics` + `clamp_scroll` + the `Cell<ViewportMetrics>` record-at-render protocol the Browse, Archive and Driver panes already share"
provides:
  - "A co-author column on every git log row, dimmed after the human author, sourced from git's own `Co-authored-by` trailer matching."
  - "An Enter pane whose upper half is the full, scrollable commit message and whose lower half is the touched-file list."
  - "A hash-guarded commit-detail action that discards a load which arrived for a commit the user has since scrolled off — closing a latent bug that predates this task."
  - "`git_row_budget`: the Git tab's first explicit width budget, with its char-vs-display-column limit stated rather than implied."
affects: [git-history-tab, action-enum, project-view-cache, render-escape-guard-fixture]

actuals:
  # chars/4 over the realized diff (70 175 chars / 4), the same scale the
  # plan's `estimate` used. The plan estimated 105 000; raw_tokens 70 000.
  tokens: 17544
  tasks: 3
  commits: 3
  plan_head_before: 858a3075d6bf9493f4f7f07d56c876a47cac7a62

tech-stack:
  added: []
  patterns:
    - "`Vec<Untrusted>`, one carrier per LINE, for any multi-line third-party text — because `strip_terminal_controls` replaces `\\n` with the control-replacement glyph, so a whole body in one carrier renders as one glyph-joined line."
    - "A width budget as a pure function returning a struct, called from the row builder, so the truncation policy is testable as arithmetic rather than only through a rendered buffer."

key-files:
  created: []
  modified:
    - src/state_reader/git_ops.rs
    - src/action.rs
    - src/app.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/render_escape_guard.rs

key-decisions:
  - "QD-01..QD-10 held as written. QD-11 held. One arithmetic correction inside QD-04 and one placement refinement inside QD-07 — both recorded below under `Inferred decisions (for audit)`."
  - "The hash guard lives on `ProjectViewCache::apply_loaded_commit_detail` rather than inline in `app.rs`'s action arm, so it is exercisable without constructing an `App` and the pane's three fields keep exactly one writer."
  - "`render_escape_guard.rs`'s fixture was updated in Task 1's commit rather than Task 3's, because adding the `co_authors` field made the fixture a compile error (deviation Rule 3)."

patterns-established:
  - "When a subprocess-backed load can outlive the selection it was started for, the action carries back the key it was ASKED for and the handler compares before applying. Same shape as `DriverRunTally`'s run id and `DryRunPreview`'s command."

requirements-completed:
  - "todo:.planning/todos/pending/2026-08-18-git-view-show-co-author-model-and-full-commit-message-on-ent.md"
---

# Quick Task 260916-vr1: Git view — co-author column and full commit message Summary

The `4: Git` tab now shows each commit's `Co-authored-by` trailer (in this repo, the model that did the work) dimmed after the human author, and Enter opens a pane whose upper half is the full scrollable commit message above the existing file list — the half that carries the reasoning, which the tab previously dropped entirely.

## What shipped

| Task | Commit | What |
|---|---|---|
| 1 (tracer) | `5abbe20` | `GitLogEntry.co_authors`, the five-field log format, `parse_git_log_line` with both arities, `parse_co_authors`, `GitCommitDetail` + `load_commit_detail` |
| 2 | `4e95090` | `Action::GitCommitDetailLoaded { alias, hash, detail }`, the cache's one-Option pane state and its three transition methods, all six close sites rerouted, the QD-06 paging branch |
| 3 | `4a8094e` | `git_row_budget` + `truncate_subject`, the co-author column, the message-over-files pane layout, the footer hint |

## Tracer feedback gate

Task 1 was `type="tracer"` with no `gate` attribute, so the gate resolved to the auto-mode arm: its `<verify>` was re-run end to end after the commit (`cargo test --lib state_reader`, 245 passed / 0 failed, with all five named tests present and the two real-repo arms genuinely running rather than skipping). It passed, so expansion into Tasks 2 and 3 proceeded.

The real-repo arm is worth naming: `a_co_authored_by_trailer_is_parsed_case_insensitively_from_a_real_repo` writes the `Co-Authored-By:` spelling into an actual commit and asserts the value comes back. It PASSED, which is the evidence for QD-02 — git's own trailer matcher supplied the case-insensitivity, and no hand-rolled matcher exists in this codebase. Had the sandbox forbidden `git commit` the arm would have skipped and that claim would have been unproven; it did not.

## QD-01..QD-11: which held, which were revised

| id | verdict | note |
|---|---|---|
| QD-01 | **held** | All co-authors, deduped case-insensitively, first-seen order, `", "`-joined; `None` emits neither column nor separator. Pinned by `a_git_row_without_co_authors_has_no_trailing_parentheses`. |
| QD-02 | **held, and PROVEN** | Delegated to git's `%(trailers:key=...)`. The real-repo arm ran. |
| QD-03 | **held** | No known-model name list. Every co-author renders, model or human. |
| QD-04 | **held, with one arithmetic correction** | See below. |
| QD-05 | **held** | Outer 40/60 (inverted from today), inner 60/40, and below 8 detail rows the message takes the whole area with the file pane skipped. |
| QD-06 | **held** | PageUp/PageDown scroll the pane while open, page the selection while closed; `j`/`k` still move the selection. No new keybinding, no pane-focus mode. |
| QD-07 | **held, with one placement refinement** | See below. |
| QD-08 | **held** | `Vec<Untrusted>`, split at the producer, trailing blanks trimmed. |
| QD-09 | **held** | One `Option`; presence is "open"; every transition through a method. |
| QD-10 | **held** | No `Wrap`; `total_lines` is the real scroll range. |
| QD-11 | **held** | Trailer field before `%s`; `splitn(5, ..)` accepts 5 or 4, drops anything else. |

## Inferred decisions (for audit)

Three judgement calls beyond QD-01..QD-11, none of which changes what any decision decided:

1. **QD-04's co-author frame costs FOUR characters, not the five the plan wrote.** The frame is `"  ("` (3) plus `")"` (1). The plan's `<action>` said five. Five would have been a fabricated extra column: the budget's claim is "this is what the row costs", and a number that is not the cost makes that claim false by one at every boundary. The decision QD-04 *made* — subject truncates first, co-author drops before author, floor of 12 — is untouched.

2. **The QD-07 hash guard lives on the cache, not inline in `app.rs`.** The plan put the comparison in the action arm. Writing it there would have meant the only way to test the guard was to construct an `App`, and it would have added a fourth writer to fields QD-09 says have one. `ProjectViewCache::apply_loaded_commit_detail(hash, detail)` does the whole thing — always clears the loading flag, opens only on a hash match — and `app.rs` calls it. Both of QD-07's properties are unchanged and are now pinned by `a_commit_detail_for_an_unselected_hash_is_discarded`.

3. **The file pane's title is `" Files "`, a constant, rather than a formatted name-the-files string.** The plan said "retitled to name the files rather than the hash". The stat's file list is a bare `Vec<String>` and the count already appears in the summary line inside the pane, so a computed title would have duplicated it. A constant says the same thing with nothing to keep in agreement.

## Deviations from plan

**1. [Rule 3 — Blocking] `render_escape_guard.rs`'s fixture moved from Task 3 into Task 1's commit**

- **Found during:** Task 1, at the first `cargo test` after adding `GitLogEntry.co_authors`.
- **Issue:** `hostile_git_entry` constructs `GitLogEntry` with all fields named, so the new field was an `E0063` compile error in the same commit that introduced it. The tree cannot be committed task-atomically with Task 1 unless the fixture moves with it.
- **Fix:** Applied exactly the edit Task 3 prescribed (`co_authors: Some(field())`) plus its doc rewrite, in Task 1's commit.
- **Files modified:** `src/ui/screens/render_escape_guard.rs`
- **Commit:** `5abbe20`

**2. [Rule 3 — Blocking] `GitDiffStat` import narrowed in `src/ui/screens/mod.rs`**

- **Found during:** Task 2, at `cargo build`.
- **Issue:** Replacing the `git_diff_stat` field left `GitDiffStat` used only by the new tests, producing an `unused_imports` warning — which `cargo clippy -- -D warnings` would have turned into a hard failure.
- **Fix:** Dropped it from the module import; the test helper spells it fully qualified.
- **Commit:** `4e95090`

**3. One RED in Task 3 was a test defect, not an implementation defect**

`a_git_row_without_co_authors_has_no_trailing_parentheses` failed on `alone_row.trim_end().ends_with("Human")`. The render was correct; the scraped buffer line carries the enclosing block's right border `│`, which `trim_end()` does not remove. Recorded rather than quietly fixed because the distinction matters: the assertion was wrong about the instrument, not about the behaviour. The fix strips the border and the padding together.

## Measured verification

| Gate | Result |
|---|---|
| `cargo build` | exit 0, zero warnings |
| `cargo clippy -- -D warnings` | exit 0 (lib gate, per the plan — `--all-targets` carries 5 pre-existing lints and is not a clean signal) |
| `cargo test --no-fail-fast` BEFORE | **2054 passed / 1 failed** |
| `cargo test --no-fail-fast` AFTER | **2068 passed / 1 failed** |

The `+14` is exactly the 14 tests this task adds (5 + 3 + 6). The single failure is identical before and after: `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, the known environmental git-version-constants test (installed git 2.53 vs the pinned constant). The final failing set is therefore a strict subset of the baseline's, as `<verification>` requires.

Note on the baseline: the plan quoted 2005, measured before this batch's three sibling items merged. The re-measured pre-task baseline on this worktree's actual base (`858a3075`) is 2054. `driver_reattach` did not flake in either run.

The counts were read from raw `cargo` output under `rtk proxy` with the pipe inside the proxied command — `rtk` strips `test result:` lines, which would have made the count check succeed vacuously.

## Known gaps

**`probe_ctx` does not populate `git_commit_detail`** — disclosed in the plan's Task 3 and confirmed here rather than silently dropped.

The escape-guard probe reaches the log ROW's new co-author span (the fixture carries `co_authors: Some(field())`, and `the_screen_renders_identity_escaped` passes with it populated), but it does not reach the commit MESSAGE pane, because opening that pane also draws `GitDiffStat::file_stats` — a bare `Vec<String>` this task does not retype. Populating it would have turned a quick task into an unrelated carrier retrofit.

What bounds the gap: the body is `Vec<Untrusted>`, which has no `Display`, no `AsRef<str>` and no `Into<Cow<'_, str>>`, so an unescaped spelling of a body line does not compile. The pane's render is structurally safe for the same reason the row's was made safe — the gap is in the PROBE's coverage, not in the mechanism. `GitDiffStat::file_stats` remains the untyped carrier it already was before this task.

No stubs, no skipped tests, no unrun `<verify>` steps. `.planning/WINDOWS.md` was not written: this run's scope fence limits writes to the declared code files plus this summary, and there is no ledger-eligible defect to record.

## Threat flags

None. The threat model's five registered threats were all `mitigate`/`accept` dispositions already covered by the implementation (co-author and body both `Untrusted`; trailer field ahead of `%s`; newlines split at the producer; hash-guarded action; hash as an argv element, never a shell fragment). No new network endpoint, auth path, file-access pattern or schema change was introduced. `Cargo.toml` is untouched — no new dependency.

## Todo file

`.planning/todos/pending/2026-08-18-git-view-show-co-author-model-and-full-commit-message-on-ent.md` was **left in place**. The plan's `files_deleted` names it, but the batch orchestrator owns todo lifecycle and moves it on completion; `.planning/todos/` was not touched by this executor.

## Self-Check: PASSED

- `src/state_reader/git_ops.rs` — FOUND
- `src/action.rs` — FOUND
- `src/app.rs` — FOUND
- `src/ui/screens/mod.rs` — FOUND
- `src/ui/screens/detail.rs` — FOUND
- `src/ui/screens/render_escape_guard.rs` — FOUND
- Commit `5abbe20` — FOUND
- Commit `4e95090` — FOUND
- Commit `4a8094e` — FOUND
- `git rev-list --count 858a3075..HEAD` = 3, matching the `commits:` recorded above
