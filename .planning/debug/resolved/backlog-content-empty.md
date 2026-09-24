---
status: resolved
trigger: "In the TUI's 3:Backlog tab, the items in ~/projects/python/picsync (999.1, 999.2) are listed, but selecting/pressing on any item (in every project) shows only: ┌ Content: 999.2-gnome-notification-live-progress-and-actions ─┐ │  Empty — no .md files in this backlog directory. User asked: check how backlog items are really stored and whether they are .md files in a directory."
created: 2026-09-24T00:00:00Z
updated: 2026-09-24T01:30:00Z
---

## Current Focus
<!-- OVERWRITE on each update - always reflects NOW -->

hypothesis: CONFIRMED and FIXED — see Resolution.
test: done — 15 new regression tests green; revert-and-reconfirm and 9/9 manual mutants killed; loader run read-only against every registered project.
expecting: n/a
next_action: none — archived to .planning/debug/resolved/ (human unavailable; INFERRED: the self-verification below stands in for the human-verify checkpoint, as the orchestrator directed — nothing left is unobservable without a human except subjective look-and-feel in a real terminal)
bug_class: Bohrbug (deterministic — every item in every project, every time)
known_pattern_candidate: none
tdd_checkpoint:
  test_file: "src/state_reader/backlog.rs, src/ui/screens/detail.rs"
  test_name: "a_gitkeep_only_backlog_item_loads_its_roadmap_section; enter_on_a_gitkeep_only_backlog_item_draws_its_roadmap_section; a_multi_line_backlog_body_renders_one_row_per_line_not_one_joined_row"
  status: "green"
  failure_output: "pre-fix: load_backlog_content(..) == None; pane 'the pane drew its empty state'; row `│# Context··First decision line·Second decision line·Third decision line·`"
reasoning_checkpoint:
  hypothesis: "The 3:Backlog content pane shows 'Empty — no .md files' for every item because load_backlog_content (backlog.rs:237) only reads *.md files inside the 999.x phase dir, while GSD's add-backlog workflow stores the item's content as a `### Phase 999.N:` section in ROADMAP.md and leaves the dir with only .gitkeep; ROADMAP.md is never consulted on this path."
  confirming_evidence:
    - "ls: all 11 registered 999.* dirs (5 projects) contain only .gitkeep; zero .md files"
    - "add-backlog.md Step 3/4: entry written to ROADMAP.md `## Backlog` as `### Phase {NEXT}: {desc} (BACKLOG)`; dir gets only .gitkeep"
    - "detail.rs:2754 → backlog.rs:237-240: find_first_md_file(dir)? → None; detail.rs:4291-4293 falls back to the Empty literal"
  falsification_test: "If a loader that also reads the ROADMAP section still yields Empty for picsync 999.1 (read-only run against the real file), or if the RED test passes before the fix, the hypothesis is wrong."
  fix_rationale: "Make the loader read the item's source of truth (ROADMAP.md section keyed by the item's 999.N number, same heading grammar as parse_phase_goals) and append any accumulated .md artifacts; render per-line through archive::render_markdown_lines so the escape stays per line instead of collapsing newlines."
  blind_spots: "project_code-prefixed backlog dirs (PREFIX-999.N-slug) are not listed at all by backlog_dirs — separate pre-existing gap, not addressed; no content-pane scroll (long sections clip at pane height, wrap added); content cached per item until the backlog list reloads."
  candidate_causes:
    - "code: loader reads only phase-dir .md files (CONFIRMED)"
    - "data/convention: GSD stores content in ROADMAP.md, dirs are placeholders (CONFIRMED — the condition that makes the code gap bite)"
    - "code: render path collapses newlines via whole-body shown() (CONFIRMED latent co-defect, surfaces once content loads)"
    - "environment: wrong project path / planning_dir join (ELIMINATED — list populates from the same planning_dir)"
  and_gate: "yes — the symptom needs BOTH the dir-only loader AND the GSD convention of .gitkeep-only dirs; the per-line render defect is a second, independent cause that would make the pane unreadable once content loads. root_cause is a set."

## Session Constraints (from orchestrator — human unavailable)

- Mode: find_and_fix. Human is unavailable: decide from evidence and planning artifacts; mark inferred decisions below as `INFERRED:`. Use AskUserQuestion only for genuinely irreversible forks.
- Work on master in place (no worktree). If any GSD executor agent is dispatched, check the isolation-dispatch step and use ISOLATION=none (known worktree base-check false negative).
- Atomic commits, do NOT push.
- Tests: `rtk proxy cargo test --no-fail-fast` (known local failure: git-version constants test in src/envelope/policy.rs — expected).
- Lint: `rtk proxy cargo clippy --all-targets -- -D warnings` — baseline 11 pre-existing errors in browser.rs, project_creator.rs, tests/envelope_*.rs; add none.
- If /tmp is short on space, use TMPDIR under $HOME.
- Regression tests with a fixture mirroring picsync's layout, sanitized text only (no private content).
- Ignore unrelated existing sessions in .planning/debug/.

## Symptoms
<!-- Written during gathering, then immutable -->

expected: Selecting a backlog item in the 3:Backlog tab shows the item's content (its ROADMAP.md section — Goal, Requirements, Plans, free-text body — rendered like other markdown panes via the Untrusted/escaping path, plus any .md files in the backlog dir when present). "Empty" only when neither exists.
actual: Every backlog item in every project shows "Content: <dir-name>" with body "Empty — no .md files in this backlog directory".
errors: none (UI message only)
reproduction: Open TUI, go to 3:Backlog tab, select any item (e.g. picsync 999.2-gnome-notification-live-progress-and-actions).
started: unknown — likely always broken since Backlog tab was introduced

Pre-gathered evidence (from orchestrator; VERIFY, don't trust blindly):
- ~/projects/python/picsync/.planning/phases/999.1-pixel-gvfs-mtp-fallback-transport/ and 999.2-.../ contain ONLY a .gitkeep.
- Backlog content lives in .planning/ROADMAP.md as `### Phase 999.1: Pixel gvfs/MTP fallback transport (BACKLOG)` sections with **Goal:**, **Requirements:**, **Plans:** and a free-text body (picsync ROADMAP.md ~line 54 onward).
- To check: GSD backlog convention in ~/.claude/gsd-core (grep "999." / "BACKLOG" / add-backlog workflow); other registered projects (user config lists registered paths) for backlog dirs that DO contain .md files (e.g. after /gsd-discuss-phase on a backlog item).
- Suggested code locations: state_reader/roadmap_md.rs parser; Backlog tab rendering in src/ui/screens/detail.rs or wherever it lives.

## Eliminated
<!-- APPEND only - prevents re-investigating after /clear -->

- hypothesis: wrong project path / planning_dir join makes the loader look in the wrong place
  evidence: the list (parse_backlog_items) and the loader both join `project.path/.planning`; the list populates correctly for picsync, and the dir the loader reads exists but holds only .gitkeep
  timestamp: 2026-09-24T00:08:00Z

- hypothesis: the 16 extra baseline test failures ("census walked src/ and found no Rust source", ROADMAP.md NotFound) are tree defects touching this area
  evidence: they disappear entirely after forcing a rebuild (`touch src/lib.rs src/main.rs`) — a stale cached test binary whose baked CARGO_MANIFEST_DIR is not this tree; the stale binary also carried `review_probe` modules that do not exist in this source
  timestamp: 2026-09-24T00:25:00Z

## Evidence
<!-- APPEND only - facts discovered during investigation -->

- timestamp: 2026-09-24T00:05:00Z
  checked: Phase 0 knowledge base (.planning/debug/knowledge-base.md); MemPalace not queried (fallback to KB keyword overlap)
  found: only entry is archive-milestone-view-loading (cache prune key mismatch) — no keyword overlap with "backlog content empty / no .md files"
  implication: no known-pattern candidate

- timestamp: 2026-09-24T00:06:00Z
  checked: ls -la ~/projects/python/picsync/.planning/phases/999.*
  found: 999.1-pixel-gvfs-mtp-fallback-transport/ and 999.2-gnome-notification-live-progress-and-actions/ each contain only `.gitkeep` (0 B)
  implication: orchestrator's pre-gathered claim verified; no .md file exists to load

- timestamp: 2026-09-24T00:07:00Z
  checked: picsync .planning/ROADMAP.md (read-only grep)
  found: `### Phase 999.1: Pixel gvfs/MTP fallback transport (BACKLOG)` at line 54 and `### Phase 999.2: GNOME desktop notification — ... (PROMOTED AND DELIVERED)` at line 90; each has **Goal:**/**Requirements:**/**Plans:** plus a long free-text body, a blockquote, and a `Plans:` checklist; the section ends at the next `### Phase`/`##` heading or the trailing `*...*` footer (line 227)
  implication: the item's content lives in ROADMAP.md; the heading suffix is not always `(BACKLOG)` (999.2 says `(PROMOTED AND DELIVERED)`), so a matcher must key on the phase number, not the suffix

- timestamp: 2026-09-24T00:08:00Z
  checked: src/state_reader/backlog.rs (load_backlog_content :237, find_first_md_file :257, parse_backlog_items :103) and src/ui/screens/detail.rs (Enter handler :2735-2774, render_backlog_tab :4206-4312)
  found: Enter calls `backlog::load_backlog_content(planning_dir, dir_name)` which ONLY does `find_first_md_file(phases/<dir>)` → read. With only .gitkeep it returns None, content stays None, and render falls back to the literal "  Empty — no .md files in this backlog directory" (detail.rs:4293). ROADMAP.md is never consulted anywhere on the backlog path. backlog.rs:111-115 even records "12 of 12 real 999.* directories contain only .gitkeep" (260916-vr0) — the list was fixed to show .md-less dirs, but the content loader was not.
  implication: hypothesis mechanism confirmed by code reading; root cause is the content source, not the list/selection logic

- timestamp: 2026-09-24T00:10:00Z
  checked: GSD convention — ~/.claude/gsd-core/workflows/add-backlog.md (invoked by /gsd-capture --backlog) and commands/gsd/review-backlog.md
  found: add-backlog Step 3 writes the entry to ROADMAP.md under `## Backlog` as `### Phase {NEXT}: {description} (BACKLOG)` with **Goal:** / **Requirements:** / **Plans:** / `Plans:` checklist, and "Write the ROADMAP entry BEFORE creating the directory"; Step 4 creates `.planning/phases/${PREFIX}${NEXT}-${SLUG}` containing only `.gitkeep` ("Phase directories are created immediately so /gsd-discuss-phase and /gsd-plan-phase work on them"). review-backlog step 2 reads ROADMAP.md for the entries and shows "any accumulated context (CONTEXT.md, RESEARCH.md)" from the dir. On promotion the `(BACKLOG)` marker is removed.
  implication: by GSD design the ROADMAP.md section IS the item's content; the directory is a placeholder that only gains .md files if the user runs /gsd-discuss-phase / /gsd-plan-phase on the item. `${PREFIX}` (project_code) can prefix the dir name — separate gap, the current `strip_prefix("999.")` parser would not list such dirs at all.

- timestamp: 2026-09-24T00:12:00Z
  checked: every registered project in ~/.config/gsd-meta-manager/config.json (15 projects) — .planning/phases/*999* contents and ROADMAP.md `#{2,4} Phase 999` headings (read-only python survey)
  found: 11 backlog dirs across 5 projects (picsync 2, gsd-meta-manager 4, shopify-orderly-rescue 3, hitchmatch 1, cdr-configurator 1). ALL 11 contain only `.gitkeep`; ZERO .md files anywhere. 10 of 11 have a matching `### Phase 999.N:` ROADMAP section; the exception is gsd-meta-manager 999.1-milestone-archive-browser-tab (ROADMAP line 927: "Backlog 999.1 ... promoted to Phase 12 in v1.2" — section removed, dir left behind). Heading suffixes seen: `(BACKLOG)`, `(PROMOTED AND DELIVERED)`, `(PROMOTED → v2.0)`.
  implication: the bug hits 11/11 items in every project (matches "in every project"); a ROADMAP-backed loader fixes 10/11, the 11th correctly remains "Empty" (with an accurate message)

- timestamp: 2026-09-24T00:14:00Z
  checked: section boundaries — thematic breaks (`---`/`***`/`___`) inside `#{2,4} Phase` entries across all registered ROADMAPs
  found: 6 occurrences (picsync 1, sentriq 3, mailbot 2); every one is immediately followed by the next `### Phase` heading or by the document footer (`*Roadmap created: ...*`, picsync:225-228). None sits mid-body.
  implication: ending an entry at the next heading of ANY level (the rule parse_phase_goals / parse_planned_build_phases already use, roadmap_md.rs:342-347) OR at a thematic break keeps picsync's footer out of 999.2's body without truncating any measured section

- timestamp: 2026-09-24T00:16:00Z
  checked: render path for the content pane — detail.rs:4288-4307 `content.shown()` → `Paragraph::new(..)`; text.rs:374-391 strip_terminal_controls; text.rs:331 CONTROL_REPLACEMENT = U+00B7; archive.rs:292-323 render_markdown_lines doc
  found: `Untrusted::shown()` runs render_for_terminal over the WHOLE body, and strip_terminal_controls replaces every C0 control — including `\n` (0x0A) — with `·`. archive.rs:317-322 documents exactly this: "escaping first and splitting second would collapse the entire file into one row". The pane also has no `.wrap(..)`.
  implication: SECOND, latent defect in the same pane — the moment any content loads (a .md file OR a ROADMAP section), it renders as a single row joined by `·` and clipped at the pane width. Never observed because content never loaded (0 .md files anywhere). The fix must render per-line via `archive::render_markdown_lines` (the Archive/Browse file-viewer path, which escapes per LINE) and wrap.

- timestamp: 2026-09-24T00:25:00Z
  checked: baseline `rtk proxy cargo test --no-fail-fast` (pre-fix)
  found: first run reused a STALE cached lib test binary ("Finished in 0.21s", no compile) and reported 17 failures, 16 of them census/"walked src/ and found no Rust source" + "ROADMAP.md NotFound" — i.e. the binary's baked CARGO_MANIFEST_DIR did not point at this tree. After `touch src/lib.rs src/main.rs` forced a rebuild: lib 1550 passed / 1 failed (envelope::policy git-version witness — expected locally) / 1 ignored; all suites total passed=2342 failed=1 ignored=15.
  implication: TRUE baseline is 2342/1/15. The 16 extra reds were an artifact of a stale build, not a tree defect (INFERRED: likely built from another checkout sharing target/ — not investigated further, out of scope).

- timestamp: 2026-09-24T00:30:00Z
  checked: baseline clippy `rtk proxy cargo clippy --all-targets -- -D warnings`
  found: 11 located errors — browser.rs:156/157/158, project_creator.rs:146, tests/envelope_carrier_reach.rs:1711, envelope_config_resolution.rs:2206, envelope_control_carrier.rs:978, envelope_wrapper_class.rs:6127/6213/10795/11232
  implication: matches the stated baseline of 11; add none

- timestamp: 2026-09-24T00:40:00Z
  checked: RED phase — 6 new tests in src/state_reader/backlog.rs, 4 in src/ui/screens/detail.rs, run pre-fix
  found: 9 FAIL, 1 PASS (the negative control `an_item_with_neither_a_roadmap_entry_nor_md_files_is_empty`, which must pass both before and after). Loader tests fail with `load_backlog_content(..) == None` on the .gitkeep-only fixture; `enter_on_a_gitkeep_only_backlog_item_draws_its_roadmap_section` fails with "the pane drew its empty state". The render test (driven through a .md file the OLD loader does read) fails with the pane row `│# Context··First decision line·Second decision line·Third decision line·` — the newline-collapse co-defect directly observed.
  implication: both root causes reproduced deterministically by tests; proceed to fix

- timestamp: 2026-09-24T01:00:00Z
  checked: fix applied; targeted tests; revert-and-reconfirm; manual mutation (cargo-mutants absent, Stryker is JS-only)
  found: 29/29 targeted tests green. Revert (loader body + render restored to pre-fix) → 8 RED (every loader/render test except the negative controls); reapply → 29/29 green. Nine hand-seeded mutants at the fix sites (sibling-heading `<=`→`<`, drop thematic-break end, `==`→`starts_with` on the phase key, no fence tracking, keep trailing blanks, drop ROADMAP entry from loader, drop .md files from loader, drop `.wrap(..)`, old empty text) — all 9 KILLED. Working files byte-identical to the fixed copies afterwards (cmp).
  implication: the tests assert the root causes, not just the symptom; this change is what fixes it

- timestamp: 2026-09-24T01:10:00Z
  checked: fixed loader run READ-ONLY against every registered project (throwaway example binary, deleted after; nothing written to any project)
  found: 10 of 11 items now load their ROADMAP section — picsync 999.1 (35 lines) and 999.2 (134 lines, ends at `- [ ] TBD …`, footer `*Roadmap created…*` excluded), shopify-orderly-rescue 999.1/999.2/999.3, hitchmatch 999.1, cdr-configurator 999.1, gsd-meta-manager 999.2/999.3/999.4. gsd-meta-manager 999.1-milestone-archive-browser-tab → None (its ROADMAP entry was removed when it was promoted to Phase 12; dir left behind) → pane correctly draws the new empty text.
  implication: fix verified on real data; the one remaining Empty is correct

- timestamp: 2026-09-24T01:25:00Z
  checked: full suite + clippy after fix (and after rustfmt layout of the new hunks)
  found: `cargo test --no-fail-fast`: passed=2357 failed=1 ignored=15 (baseline 2342/1/15; +15 = 6 backlog + 4 roadmap_md + 5 detail tests; the 1 failure is the expected envelope::policy git-version witness). `cargo clippy --keep-going --all-targets -- -D warnings`: 11 errors, exactly the baseline locations, none in changed files. (Without --keep-going cargo aborts scheduling after the first failing target, so the reported count varies 10-11 run to run — measured.) rustfmt: backlog.rs clean (was clean); roadmap_md.rs 25 diffs = baseline 25; detail.rs 255 ≤ baseline 256 — no new fmt drift in changed hunks.
  implication: no regressions; lint baseline unchanged

## Resolution
<!-- OVERWRITE as understanding evolves -->

root_cause: "(1) src/state_reader/backlog.rs:237-241 `load_backlog_content` read ONLY the first *.md inside `.planning/phases/999.N-<slug>/`, but GSD's backlog capture (gsd-core/workflows/add-backlog.md Steps 3-4) stores the item's content solely as a `### Phase 999.N: … (BACKLOG)` section in ROADMAP.md `## Backlog` and gives the directory only `.gitkeep` — so every item in every project (11/11 dirs, 5 projects) resolved to None and src/ui/screens/detail.rs:4293 drew 'Empty — no .md files in this backlog directory'; (2) latent co-defect, same pane: detail.rs:4288-4290 escaped the WHOLE body with `Untrusted::shown()`, whose control pass turns `\\n` into a visible `·` (text.rs:374-391, CONTROL_REPLACEMENT U+00B7), collapsing any multi-line content into one clipped row — unobserved only because (1) meant nothing ever loaded."
fix: "New `roadmap_md::phase_section(content, phase_id)` returns a `## / ### / #### Phase N:` entry (heading grammar shared with parse_phase_goals via new `phase_heading_re()`), matched by phase_key, ending at a same-or-higher heading or a thematic break outside code fences, trailing blanks trimmed. `backlog::load_backlog_content` now returns that ROADMAP entry followed by every .md file in the item dir (name order, each under a `── <file> ──` label); None only when neither exists. The Backlog content pane renders via `archive::render_markdown_lines(content.as_raw_for_logic_only())` (per-line escape, the Archive/Browse path) with `.wrap(Wrap { trim: false })`; empty text now 'Empty — no ROADMAP.md entry and no .md files for this backlog item'. Docs updated (BacklogItem, render_markdown_lines, detail.rs adjudication string)."
verification:
  target_test: { result: pass }
  mutation_check: { result: pass, reason_if_skipped: "cargo-mutants not installed; Stryker is JS-only — replaced by 9 hand-seeded mutants at the fix sites", mutant_killed: "9/9" }
  no_op_deletion: { result: pass, deletion_justified_by_rca: "n/a — additive: new reader + composition; the only removal is the whole-body shown() render, replaced by the per-line escape the RCA (2) calls for" }
  adjacent_tests: { result: pass, suites_run: ["cargo test --no-fail-fast (all 52 suites): 2357 passed / 1 expected fail / 15 ignored", "render_escape_guard (Backlog expanded probe with hostile content) green", "normal.rs b/3-key backlog tests green", "roadmap_md parse_phase_goals tests green after the regex extraction"] }
  revert_and_reconfirm: { result: pass, bug_returned_on_revert: true, fixed_on_reapply: true }
  real_data: "loader run read-only over all 15 registered projects: 10/11 backlog items load their section; the 11th (entry removed on promotion) is correctly empty"
  guardrail_verdict: accepted
oracle_type: "specified (GSD add-backlog.md defines where content lives) + derived (render must preserve line structure, the render_markdown_lines contract) + implicit guard (no raw ESC/bidi in cells)"
files_changed: [src/state_reader/roadmap_md.rs, src/state_reader/backlog.rs, src/ui/screens/detail.rs, src/archive.rs]
inferred_decisions:
  - "INFERRED: show ALL .md files in the backlog dir (name order, labelled), not just the first — review-backlog.md says to show 'any accumulated context (CONTEXT.md, RESEARCH.md)'."
  - "INFERRED: an entry ends at a heading of the same or HIGHER level (deeper sub-headings stay in) or a thematic break; measured: every thematic break inside a Phase entry across all registered ROADMAPs is an entry terminator (6/6)."
  - "INFERRED: match on the phase number only, ignoring the heading suffix — real suffixes seen: (BACKLOG), (PROMOTED AND DELIVERED), (PROMOTED → v2.0)."
  - "INFERRED: kept list descriptions (humanized slug / first .md heading) and the expanded-mode edit key (opens item.path only) unchanged — minimal fix."
open_items:
  - "No content-pane scroll: long sections (picsync 999.2 = 134 lines) are wrapped but clipped at the pane's height (50% split). j/k/PgUp/PgDn currently move the selection and collapse; adding pane scroll is a key-binding (UX) change — left for a follow-up."
  - "Expanded-mode edit key still reports 'No file found for this backlog item' when the dir has no .md; could open ROADMAP.md instead — product decision, not done."
  - "project_code-prefixed backlog dirs (`${PROJECT_CODE}-999.N-slug`, add-backlog.md Step 4) are not listed at all (`backlog_dirs` requires a `999` prefix) — pre-existing, no registered project uses it."
  - "List rows could show the ROADMAP heading title (e.g. its (PROMOTED …) marker) instead of the humanized slug — enhancement."
  - "A stale cached lib test binary (baked CARGO_MANIFEST_DIR not this tree) produced 16 spurious census failures until a forced rebuild — worth knowing when a local run shows 'walked src/ and found no Rust source'."

## Prevention
<!-- Blameless postmortem, written at archive -->

branching_5_whys:
  - branch: "code — loader read only the directory"
    whys:
      - "Why empty? load_backlog_content read only phases/999.N-*/ *.md files."
      - "Why only the dir? The Backlog tab was designed around a `999.N-*.md` inside the dir — a layout GSD's add-backlog workflow never produces."
      - "Why wasn't the design checked against GSD? 260916-vr0 MEASURED 12/12 dirs as .gitkeep-only and fixed the LIST/count rule to match, but treated the markdown body as 'optional enrichment never present' instead of asking where the content DOES live (backlog.rs:111-115)."
      - "Actionable: when a measurement shows a data source is always empty, find the real source in the upstream tool's workflow before shipping a reader of it."
  - branch: "data/convention — GSD stores content in ROADMAP.md"
    whys:
      - "Why did the dir-only rule bite everywhere? add-backlog.md writes content to ROADMAP.md and only .gitkeep to the dir, for every GSD user."
      - "Actionable: fixtures must mirror what the upstream tool actually writes (gsd-core/workflows/add-backlog.md), not a convenient hand-made layout."
  - branch: "code — whole-body shown() on multi-line text"
    whys:
      - "Why one row? shown() escapes C0 controls, \\n included, into U+00B7."
      - "Why not caught? No test ever rendered MULTI-LINE backlog content; the escape-guard probe's content is a single-line identity string."
      - "Actionable: multi-line Untrusted bodies go through archive::render_markdown_lines (per-line escape), never shown() over the whole body."
why_not_caught: "test gate — every backlog test fixture (normal.rs ctx_with_backlog plants `999.1-BACKLOG.md`) shared the loader's wrong assumption that content is a .md in the dir, and no test crossed the Enter -> load -> render seam with a .gitkeep-only dir or with multi-line content."
recurrence_guard: "regression tests src/state_reader/backlog.rs: a_gitkeep_only_backlog_item_loads_its_roadmap_section, the_last_backlog_section_stops_before_the_document_footer, a_backlog_number_matches_its_own_section_not_a_numeric_neighbour; src/state_reader/roadmap_md.rs: a_phase_section_* / a_heading_or_break_inside_a_code_fence_does_not_end_the_section; src/ui/screens/detail.rs: enter_on_a_gitkeep_only_backlog_item_draws_its_roadmap_section, a_multi_line_backlog_body_renders_one_row_per_line_not_one_joined_row, a_long_backlog_line_wraps_instead_of_being_clipped (all green at d44c160) + KB pattern entry."
