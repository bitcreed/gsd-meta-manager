---
phase: quick-260926-gd5
plan: 01
subsystem: docs
tags: [docs, keymap, readme]
status: complete
requires: []
provides:
  - README dashboard key table matching NormalScreen::handle_key
  - README detail-view navigation table plus Tab-specific keys table
  - GETTING-STARTED alias-then-path registration, `c` key, sub-tab arrows
  - CONFIGURATION eight-tab count
affects: [README.md, docs/GETTING-STARTED.md, docs/CONFIGURATION.md]
tech-stack:
  added: []
  patterns: []
key-files:
  created: []
  modified:
    - README.md
    - docs/GETTING-STARTED.md
    - docs/CONFIGURATION.md
decisions:
  - "[inferred] README hosts the per-tab keys as one new Tab-specific table (GETTING-STARTED calls README the full key-binding table)"
  - "[inferred] Phases / Waves-pane / Sessions > Agents rows moved from the navigation table into the Tab-specific table"
  - "[inferred] Experimental-only keys get one gated sentence pointing to CONFIGURATION.md, not table rows"
  - "[inferred] Dashboard `/` row says the help overlay lists the filter syntax, not that `?` inside the filter shows it (`?` is typed into the filter there)"
metrics:
  duration: 3m
  completed: 2026-09-26
actuals:
  tokens: 3200
  tasks: 3
  commits: 3
plan_head_before: 57d431c71639b9bda4bac42cbf177639dd2953f9
---

# Quick 260926-gd5: Fix README / GETTING-STARTED key tables against the keymap, Summary

The README dashboard table now matches `NormalScreen::handle_key`. `c` creates a project and `n` is gone, and rows were added for `b`, `s`, `Tab`, `Ctrl+C` and the arrow aliases. The detail-view navigation table was corrected and a Tab-specific keys table was added for every tab and level. GETTING-STARTED now describes the alias-then-path `a` dialog, and CONFIGURATION gives the right tab count (eight). Only docs changed.

## Commits

| Task | Commit | Description |
|------|--------|-------------|
| 1 | 2a83c50 | README dashboard table (`c` not `n`, b/s/Tab/Ctrl+C rows, `d` only unregisters), Quick start alias-then-path, experimental-keys note |
| 2 | 63b7f70 | README detail-view navigation corrections, new Tab-specific keys table |
| 3 | 0bb3874 | GETTING-STARTED `a` flow / `c` / sub-tab arrows / Enter-or-Space, stale TOML callout replaced; CONFIGURATION eight tabs |

## Verification

- All three task `<verify>` commands printed OK.
- `git diff --quiet ab92327 -- src tests Cargo.toml Cargo.lock` exits 0. The changed-file set since ab92327 outside `.planning` is exactly README.md, docs/GETTING-STARTED.md and docs/CONFIGURATION.md.
- `KeyCode::Char('c')` sits at normal.rs:525 and pushes `CreateProjectScreen::new_name()`. README's `c` row reads "Create a new project…".
- Before writing, I re-read the source lines behind every correction: normal.rs:505-715 and 786-840, add_project.rs, create_project.rs, delete_confirm.rs, help.rs:200-275 and 336, detail.rs:1758-1797, 1885-1895, 2968-3010, 3086-3165, 3340-3351, 3663-3795, 3830-3882, 4344-4750, 4934-4942, 5011 and 5093-5115, plus app.rs:2387.
- `cargo test` was not run on purpose (plan: no test reads these docs).

## Deviations from Plan

**1. [Rule 1 - Bug in plan text] Dashboard `/` row: `?` does not show filter syntax while filtering**
- **Found during:** Task 1
- **Issue:** The plan's row text said "`?` shows the filter syntax". In filter mode, `handle_search_key` (normal.rs:786-799) pushes every `Char(c)`, `?` included, into the filter text. The filter syntax is actually listed in the help overlay (help.rs:336), which `?` opens only outside the filter.
- **Fix:** The row now reads "The help overlay (`?`) lists the filter syntax".
- **Commit:** 2a83c50

**2. [Rule 1 - Precision] Experimental note scopes `Shift+D` to the detail view**
- **Found during:** Task 1
- **Issue:** The planned sentence grouped `Shift+D` with "the dashboard's" keys. `Shift+D` is bound in `DetailScreen` (detail.rs:3835), not on the dashboard.
- **Fix:** The note now says "`Shift+D` in the detail view opens the experimental Driver tab".
- **Commit:** 2a83c50

**3. [Rule 3 - Verify-grep compatibility] Wording kept on one line for the verify greps**
- Quick start step 3 now reads "Register a project in the TUI (press `a`, …)", so the verify's literal `press \`a\`` grep matches. The GETTING-STARTED Waves paragraph was rewrapped so that `` `Enter` or `Space` on a `` sits on one line. Meaning is unchanged.

## Findings (not fixed — code)

- F1: help.rs:248 row `q / Esc` reads "Quit / Back" at the dashboard level. On the dashboard `Esc` is unbound
  outside the `/` filter (normal.rs:715, 811).
- F2: the dashboard block of help.rs (:202-217) omits `Tab` (normal.rs:692), although the dashboard footer shows
  `[Tab]session` (normal.rs:1225). Help lists `Tab` only as a detail-view key (help.rs:268).
- F3: several bound keys appear only in footer hints (detail.rs:9157-9249) and never in the help overlay:
  - Sessions `n` (detail.rs:4581)
  - Queue `a`/`e`/`d`/`x`/`Enter`/`Space`/`J`/`K` (:4675-4770, :3882)
  - Git `Enter`/`p` (:3966, :4638)
  - Config `Enter`/`x`/`d`/`r`/`/` (:4250, :4440, :4549, :4488, :4429)
  - Docs Files `g`/`p`/`e` (:4349, :4364, :4952)
  - Roadmap `PgUp`/`PgDn` (:3647, :3795)

  This contradicts help.rs:1-2 ("the only place keys are documented").
- F4: help.rs:228 documents `e` as "Enqueue next action (detail view)". On Queue, Docs > Files and
  Docs > Milestones, `e` edits or opens instead (detail.rs:4907-4979). Help documents only the Backlog and
  Waves-pane exceptions.
- F5: help omits some key aliases the handlers accept:
  - The tab bar also descends on `j`/`Space` (detail.rs:2992).
  - `k` climbs to the tab bar (:3344).
  - `Space` also focuses the Waves pane (:3877/:3881).
  - `Tab` on the Sessions tab targets the highlighted session (:4516).
- F6: the Waves-pane footer shows `[q]uit` (detail.rs:9272-9273), but `q` there returns to the dashboard
  (detail.rs:1769-1772) rather than quitting the app.
- F7: the Docs > Milestones footer shows `[e]dit` (detail.rs:9207-9208), but `e` refuses every archived file
  (detail.rs:4936-4942, archive.rs:161-190).
- F8: help.rs:216 "Delete project" and the dashboard footer `[d]el` (normal.rs:1233) are misleading. The key only
  unregisters the project (delete_confirm.rs:94).

Also observed during execution: in filter mode `?` is typed into the filter (normal.rs:786-799). The help overlay's Filter Syntax section (help.rs:336) is reachable only from outside the filter. This is consistent behaviour, not a disagreement, and it is recorded here only because the plan assumed otherwise.

## Inferred decisions

- [inferred] README is the home for the per-tab keys, because GETTING-STARTED.md calls README "the full
  key-binding table". The per-tab keys therefore go in one new README table rather than a new doc.
- [inferred] The Phases, Waves-pane and Sessions > Agents rows moved out of the navigation table into the new
  Tab-specific table, with their wording preserved and a Tab column added. The navigation table keeps only
  level and tab movement.
- [inferred] Experimental-only keys (dashboard `r`/`x`/`o`, detail `Shift+D`, the Driver-tab keys) are NOT table
  rows. The help overlay hides them with the flag off, and README gets one sentence pointing to
  docs/CONFIGURATION.md instead.
- [inferred] The stale GETTING-STARTED "Despite what README.md says… TOML" blockquote was replaced with a plain
  JSON pointer, because README already says `config.json`.
- [inferred] The CONFIGURATION.md tab count was fixed (ten → eight, `visible_tab_count(false)` = `TAB_COUNT - 1` = 8).
- [inferred] The dashboard `/` row points to the help overlay for the filter syntax (deviation 1).

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: README.md, docs/GETTING-STARTED.md, docs/CONFIGURATION.md (modified)
- FOUND commits: 2a83c50, 63b7f70, 0bb3874
