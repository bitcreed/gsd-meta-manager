---
phase: quick-260916-vr1
verified: 2026-09-17T00:00:00Z
status: passed
score: 7/7 must-haves verified
covered_files:
  - ".planning/quick/260916-vr1-git-view-show-co-author-model-and-full-commit-message-on-ent/260916-vr1-PLAN.md"
  - ".planning/quick/260916-vr1-git-view-show-co-author-model-and-full-commit-message-on-ent/260916-vr1-SUMMARY.md"
  - ".planning/todos/pending/2026-08-18-git-view-show-co-author-model-and-full-commit-message-on-ent.md"
  - "src/action.rs"
  - "src/app.rs"
  - "src/state_reader/git_ops.rs"
  - "src/ui/screens/detail.rs"
  - "src/ui/screens/mod.rs"
  - "src/ui/screens/render_escape_guard.rs"
covered_digest: "v1:sha256:8a85c24f32a026982f365aad5fe401593b623520877853d268fbb441f66183dc"
behavior_unverified: 0
overrides_applied: 0
---

# Quick Task 260916-vr1: Git view — co-author column and full commit message Verification Report

**Item Goal:** Git view — show co-author model and full commit message on Enter (area: ui, severity: minor)
**Verified:** 2026-09-17
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A git log row shows the commit's co-author(s) after the human author, dimmed | ✓ VERIFIED | `src/ui/screens/detail.rs:3835-3852` appends `Span::styled(co_authors, Style::default().fg(Color::DarkGray))` after the author span, gated on `budget.show_co_authors` and `entry.co_authors`. Ran the real format string (`git log --format="...%(trailers:key=Co-authored-by,valueonly,separator=%x1e)..."`) against THIS repo — commits `4a8094e`/`4e95090`/`5abbe20` (this very item's own commits) produce the trailer value `Claude Opus 5 (1M context) <noreply@anthropic.com>` in the 4th field, parsed by `parse_co_authors` (`git_ops.rs:696-724`) to the name only. |
| 2 | A commit with no Co-authored-by trailer renders no extra column and no dangling separator | ✓ VERIFIED | QD-01: `parse_co_authors` returns `None` when no value survives (`git_ops.rs:717-722`); render only emits `"  ("`/`")"` when `budget.show_co_authors && entry.co_authors` both hold (`detail.rs:3843-3852`). Confirmed against real data: merge commits `50e25bd`/`858a307` in this repo have an empty trailer field (no Co-authored-by trailer) and parse to `co_authors: None`. Test `a_git_row_without_co_authors_has_no_trailing_parentheses` passes. |
| 3 | Enter on a commit shows the full commit message ABOVE the touched-file list | ✓ VERIFIED | `detail.rs:3888-3960`: outer split log 40% / detail 60% (was 60/40, now inverted per QD-05 — wider than the previous 40%); inner split message 60% / files 40%, message pane rendered first at `split[0]`. Test `the_commit_message_renders_above_the_file_list` passes. |
| 4 | A multi-line commit body renders as multiple lines, each escaped | ✓ VERIFIED | `load_commit_detail` splits `git show -s --format=%B` output with `lines()` into `Vec<Untrusted>`, trimming trailing blank lines (`git_ops.rs:729-745`). Render maps each line through `.shown()` into its own `Line` (`detail.rs:3939-3949`). Test `a_commit_body_is_carried_one_untrusted_per_line` passes. |
| 5 | PageUp/PageDown scroll the commit pane while it is open, and page the selection when it is closed | ✓ VERIFIED | `detail.rs:1719-1739` (PageDown) and `1892-1910` (PageUp) branch on `cache.git_commit_detail.is_some()`: pane-open scrolls `git_commit_scroll` via `clamp_scroll`; pane-closed pages `git_selected` and calls `close_git_commit_detail()`. Test `page_down_scrolls_the_commit_pane_while_open_and_pages_the_selection_otherwise` passes. |
| 6 | A detail load that arrives for a commit other than the selected one is discarded | ✓ VERIFIED | `ProjectViewCache::apply_loaded_commit_detail` (`mod.rs:1136-1150`) always clears `loading_commit_detail`, opens the pane only when `selected_hash == Some(hash)`. `app.rs:1189-1199` routes `Action::GitCommitDetailLoaded` through it. Test `a_commit_detail_for_an_unselected_hash_is_discarded` passes. |
| 7 | At a narrow width the subject is truncated before the attribution columns, and the co-author column is dropped before the author is | ✓ VERIFIED | `git_row_budget` (`detail.rs:207-280`): subject takes the remainder after fixed costs; co-author column drops when remaining width falls below `GIT_ROW_MIN_SUBJECT_COLS` (12); author is never dropped (no branch removes it). Tests `a_git_row_truncates_the_subject_before_the_attribution_columns` and `a_narrow_git_row_drops_the_co_author_column_and_keeps_the_author` pass. |

**Score:** 7/7 truths verified (0 present, behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/state_reader/git_ops.rs` | `GitLogEntry.co_authors`, `GitCommitDetail`, `load_commit_detail`, `parse_git_log_line` | ✓ VERIFIED | All present (`git_ops.rs:546-599, 640-751`), wired into `load_git_log` and consumed in `detail.rs`. |
| `src/ui/screens/mod.rs` | `ProjectViewCache::git_commit_detail` / `git_commit_scroll` / `loading_commit_detail` + open/close methods | ✓ VERIFIED | Fields at `mod.rs:925-934`; `open_git_commit_detail`/`close_git_commit_detail`/`apply_loaded_commit_detail` at `1103-1150`, called from `app.rs` and all six `detail.rs` close sites. |
| `src/ui/screens/detail.rs` | commit message pane, `git_commit_viewport`, row budget | ✓ VERIFIED | `git_row_budget`/`GIT_ROW_MIN_SUBJECT_COLS` at `207-280`; `git_commit_viewport: Cell<ViewportMetrics>` at `558`, set at `3921`; message pane rendered `3888-3960`. |
| `src/action.rs` | `Action::GitCommitDetailLoaded { alias, hash, detail }` | ✓ VERIFIED | `action.rs:93-97`, replaces the old `GitDiffStatLoaded` (confirmed absent via grep). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `load_commit_detail` | `Action::GitCommitDetailLoaded` | Enter-arm `tokio::spawn` | ✓ WIRED | Confirmed by reading the Enter handler and `action.rs` variant shape; exercised indirectly by `apply_loaded_commit_detail` tests. |
| `Action::GitCommitDetailLoaded` | `ProjectViewCache::open_git_commit_detail` | `app.rs:1189-1199` → `apply_loaded_commit_detail` → `open_git_commit_detail` | ✓ WIRED | `app.rs:1198` calls `cache.apply_loaded_commit_detail(&hash, detail)`. |
| `GitLogEntry.co_authors` (`Untrusted`) | `.shown()` at the `List` row | `detail.rs:3812` `entry.co_authors.as_ref().map(\|c\| c.shown()...)` | ✓ WIRED | Confirmed by code read; also reaches the escape-guard probe via `hostile_git_entry`'s `co_authors: Some(field())` (`render_escape_guard.rs:1334`), and `the_screen_renders_identity_escaped` passes with it populated. |
| `git_commit_viewport` (recorded at render) | `clamp_scroll` → PageUp/PageDown arms | `detail.rs:3921` set, read at `1729`/`1900` | ✓ WIRED | Confirmed by code read; same `Cell<ViewportMetrics>` protocol as the three sibling panes. |

### Data-Flow Trace (Level 4)

Ran the actual `load_git_log` format string against this repository's real `git log` (not a fixture): the 5-field `\x1f`-delimited output contains real commit hashes, real dates, the real author name, and — critically — the trailer field is non-empty for this item's own three feature commits (carrying `Claude Opus 5 (1M context) <noreply@anthropic.com>`) and empty for the two merge commits with no trailer. This confirms the arity guard (QD-11) does not empty the log against a real, unmodified format string, and that `parse_co_authors` receives real trailer data end to end, not just fixture data. The `a_co_authored_by_trailer_is_parsed_case_insensitively_from_a_real_repo` test additionally proves the same path against a throwaway sandbox repo the test creates.

| Artifact | Data variable | Source | Produces real data | Status |
|----------|---------------|--------|---------------------|--------|
| Git log row co-author column | `entry.co_authors` | `git log --format=...%(trailers:key=Co-authored-by,...)...` via `load_git_log` | Yes — confirmed against this repo's real history | ✓ FLOWING |
| Commit message pane | `detail.body` | `git show -s --format=%B <hash>` via `load_commit_detail` | Yes — code reads real subprocess output, no static fallback | ✓ FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Real git log format string produces 5-field lines with real commits (arity-guard risk item) | `git log --max-count=5 --format="%h\x1f%ad\x1f%an\x1f%(trailers:key=Co-authored-by,valueonly,separator=%x1e)\x1f%s" --date=short` | 5 real commits returned, all 5-field, none dropped; 3 carry a real co-author trailer, 2 (merges) carry an empty trailer field | ✓ PASS |
| `parse_git_log_line` + `parse_co_authors` named tests | `cargo test --lib git_ops::` | 5/5 named tests pass, including the real-repo trailer-parsing test | ✓ PASS |
| Commit-pane cache/action wiring named tests | `cargo test --lib commit` | 3/3 named tests pass (unselected-hash discard, open/close reset, PageDown branch) | ✓ PASS |
| Row budget named tests | `cargo test --lib git_row` | 3/3 named tests pass (no-co-author row, subject truncation, co-author-dropped-before-author) | ✓ PASS |
| Message-pane layout named tests | `cargo test --lib commit` (superset) | `the_commit_message_renders_above_the_file_list`, `the_commit_message_takes_the_whole_detail_area_below_eight_rows`, `the_commit_message_pane_renders_from_its_stored_offset` all pass | ✓ PASS |
| Escape-guard probe with co-author field populated | `cargo test --lib the_screen_renders_identity_escaped` | pass | ✓ PASS |
| `Untrusted` has no `Display`/`Into<Cow<str>>` (structural-safety claim for the disclosed gap) | `grep -n "^impl.*for Untrusted" src/text.rs` | only `impl std::fmt::Debug for Untrusted` found | ✓ PASS |
| Full workspace suite, `--no-fail-fast` (run once) | `rtk proxy cargo test --no-fail-fast -- --test-threads=4` | **2068 passed / 1 failed** — the single failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, the documented environmental git-version constant mismatch (installed git 2.53 vs pinned constant) | ✓ PASS (matches known non-regression, matches SUMMARY's claimed AFTER count exactly) |
| `cargo clippy --lib -- -D warnings` | `rtk proxy cargo clippy --lib -- -D warnings` | exit 0 | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| `todo:.../2026-08-18-git-view-show-co-author-model-and-full-commit-message-on-ent.md` | 260916-vr1-PLAN.md | Show co-author trailer and full commit message on Enter | ✓ SATISFIED | All 7 truths above verified against real code and real repo data. |

### Anti-Patterns Found

None. Scanned all six modified source files for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` and stub-shaped patterns (`return null`, hardcoded empty props, console-log-only handlers) — none found in the new code paths. `.trim_end().ends_with(...)` test-instrument bug disclosed in the SUMMARY was fixed, not left as a debt marker.

### Probe Execution

Not applicable — this item is a UI feature quick task, not a migration/tooling phase; no `scripts/*/tests/probe-*.sh` declared or referenced.

### Human Verification Required

None. Every must-have truth was settled with either a real-repo command run in this session or a passing named unit test read against the actual code, per the operator's instruction to settle everything checkable rather than escalate.

## Judgment Call: `probe_ctx` Coverage Gap (Item 5 of the verification brief)

The SUMMARY discloses that `probe_ctx` (in `render_escape_guard.rs`) does not populate `git_commit_detail`, so the escape-guard probe reaches the new co-author span on the log row but not the commit-message pane.

**Verdict: acceptable bounded gap, not a hole.** Evidence:
- Confirmed `probe_ctx` (`render_escape_guard.rs:1022-1063`) has no `git_commit_detail` assignment — the gap is real and exactly as described.
- Confirmed `Untrusted` (`src/text.rs:568`) implements only `Debug` (`text.rs:655`) — no `Display`, no `AsRef<str>`, no `Into<Cow<str>>` was found anywhere in the file for this type. This means `detail.body.iter().map(|line| Line::from(Span::raw(line.shown())))` (`detail.rs:3944-3949`) is the ONLY way a body line can reach the render — there is no code path where an unescaped `Untrusted` value could compile into the pane. The safety property the probe would otherwise exercise dynamically is already enforced statically by the type system, for this specific attack class.
- The SUMMARY's alternative — retyping `GitDiffStat::file_stats` (a pre-existing bare `Vec<String>`, not touched by this item) so the whole pane becomes probe-populatable — would indeed be an unrelated carrier retrofit outside this item's scope, as the executor argued.
- This narrows to a residual gap only in probe *coverage breadth*, not in render *correctness*: a future carrier that does implement `Display`/`Into<Cow<str>>` and gets substituted into the message pane without updating the probe would not be caught by this probe. That is a real but narrow and pre-existing class of risk (the same class the module's own doc already calls "LIMIT 1"), not something this item introduced or worsened.

No override needed — this is not a FAIL or an UNCERTAIN; it is a correctly-disclosed, correctly-bounded scope decision.

## Notes (non-blocking)

- The todo file `.planning/todos/pending/2026-08-18-...md` was **not** moved to `completed/` despite the PLAN's `files_deleted` frontmatter naming it. The SUMMARY explains this is because the batch orchestrator (this is one of several sibling quick-batch items merged together) owns todo lifecycle, not the individual executor. This is a process/bookkeeping detail outside the observable UI goal and does not affect the truths verified above; flagged here for the batch orchestrator's own audit trail, not as a phase gap.

## Gaps Summary

None. All 7 must-have truths verified against real code and real repository data (not fixtures alone); all named tests pass; full suite matches the SUMMARY's claimed count exactly (2068 passed / 1 failed, failure is the known environmental non-regression); `cargo build`/`clippy --lib -D warnings` clean; working tree left clean (no throwaway probes committed).

---

_Verified: 2026-09-17_
_Verifier: Claude (gsd-verifier)_
