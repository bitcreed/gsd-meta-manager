---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 25
subsystem: security
tags: [unicode, trojan-source, ratatui, newtype, render-escaping, fixture-coverage, multibyte-panic]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    provides: "crate::text::Untrusted and Rendered (21-23's carrier); render_for_terminal; render_escape_guard's Screen census and behavioural probe"
provides:
  - "the eight `.planning/` carriers typed as crate::text::Untrusted — BacklogItem (4 fields), ArchiveFile::name, PhaseArchive::{name, display_name}, BrowserEntry::name, ClaudeSession::session_id, ProjectViewCache::{archive_milestones, archive_file_name, browser_file_name}"
  - "38 compiler-named render/logic sites resolved, against the 5 verification pass 9 found by reading"
  - "detail.rs::shorten_session_id — the sid[..8] byte slice rewritten as a char operation (T-21-25-05)"
  - "probe_ctx populating every cache the eleven DetailScreen tabs read, plus four within-tab states: fifteen probe states, all rendering a populated branch"
  - "DETAIL_TAB_ARRIVAL — per-state arrival measured against a chrome baseline rather than by containment, observed red by planting in both directions"
  - "four live leaks found by the populated fixture: the Defaults tab's entry.value, the Browse tab's breadcrumb, four half-escaped sites on the Driver tab, and the markdown body both file viewers draw"
  - "normal.rs's status footer escaped at the RENDER site — the one deliberate inversion of the escape-at-the-producer rule, argued at the site with its residual"
  - "LIMIT 1 rewritten strictly narrower, with the wording it replaces quoted beside it"
affects: [21-26, verification-pass-10]

actuals:
  tokens: 26350
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Retype the carrier, then resolve ONLY the sites the compiler names — applied to eight carriers at once rather than tab by tab"
    - "Chrome-baseline arrival: measure a tab's own contribution as occurrences ABOVE what the same state renders with every tab-body source emptied, because a screen that draws the key in its block title makes plain containment answer true for every tab"
    - "Escape at the CONSUMER when the trust boundary runs through the middle of a format! — stated at the site as an inversion, with its residual and what would remove it"
    - "Escape a document per LINE, never over the whole document, because the control class replaces \\n"

key-files:
  created: []
  modified:
    - src/state_reader/backlog.rs
    - src/archive.rs
    - src/browser.rs
    - src/session_detector.rs
    - src/app.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/render_escape_guard.rs

key-decisions:
  - "D-21-19 (costly): retype the eight `.planning/` carriers rather than escape the five named sites — the compiler named 38, and 38 against 5 is the argument"
  - "D-21-20: ProjectViewCache::archive_milestones becomes Vec<Untrusted> rather than staying Vec<String> with escaping at the render"
  - "D-21-21: ctx.status_message is escaped at the RENDER site, not at its six producers — the trust boundary is inside a format!"
  - "D-21-22: probe_ctx populates EVERY cache the eleven tabs read, in one task, rather than tab by tab across rounds"
  - "D-21-23: per-tab arrival is RECORDED and measured against a chrome baseline — plain containment was measured WRONG and is reported as such"
  - "D-21-24: the clippy prohibition on src/browser.rs is expressed as a CHECKED property rather than as 'do not touch the file'"

patterns-established:
  - "When a screen draws one identity in its chrome on every state, per-state arrival must be a COUNT above a measured baseline, never a containment"
  - "A fixture that populates a cache must be planted against (empty the cache, see the named red) before its coverage is claimed"
  - "An inversion of a round's own rule is argued at the site, with the residual and the thing that would remove it, never labelled"

requirements-completed: [SAFE-07, DRIVE-03]

coverage:
  - id: D1
    description: "The five raw `.planning/`-derived render sites verification pass 9 named are closed BY THE COMPILER, not by a reader finding them — and the compiler named 38"
    requirement: SAFE-07
    verification:
      - kind: other
        ref: "cargo build --message-format short after the retype: 27 errors in the production build; cargo test --no-run: a further 11. All quoted below."
        status: pass
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every one of the eleven DetailScreen sub-views renders its POPULATED branch under probe, plus four within-tab states, and arrival is recorded per state"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped (assertion 1b, DETAIL_TAB_ARRIVAL set equality against the chrome baseline)"
        status: pass
      - kind: other
        ref: "planted red, both directions: backlog_items cleared -> names Backlog tab and Backlog tab, expanded; a row flipped to false -> names Sessions tab as an unexpected arrival"
        status: pass
    human_judgment: false
  - id: D3
    description: "Four live leaks the populated fixture found and closed — Defaults entry.value, Browse breadcrumb, four half-escaped Driver sites, the markdown body of both file viewers"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped (assertions 3 and 4, per state)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The session-id truncation is a char operation and a multibyte id no longer panics the render (T-21-25-05)"
    requirement: DRIVE-03
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#a_multibyte_session_id_does_not_panic_the_render"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#a_long_multibyte_session_id_is_truncated_by_characters_not_bytes"
        status: pass
    human_judgment: false
  - id: D5
    description: "normal.rs's status footer renders through render_for_terminal, and the fixture state provably reaches the status branch"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_status_footer_state_reaches_the_status_branch"
        status: pass
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped (NormalScreen [dashboard with a status message])"
        status: pass
    human_judgment: false
  - id: D6
    description: "SAFE-07's structural boundary and precision are unmoved by this plan's diff — reconfirmation by re-run, not new work"
    requirement: SAFE-07
    verification:
      - kind: integration
        ref: "tests/spawn_seam_guard.rs (38 passed / 0 failed) and tests/driver_injection_corpus.rs (13 passed / 0 failed / 10 ignored), both unchanged in git diff --stat"
        status: pass
    human_judgment: false
  - id: D7
    description: "The four pre-existing clippy lints are untouched, measured rather than promised, in a plan that edits src/browser.rs"
    requirement: SAFE-07
    verification:
      - kind: other
        ref: "cargo clippy --all-targets -- -D warnings before and after: exactly four, kinds bool_assert_comparison x3 and cmp_owned x1, files src/browser.rs and src/project_creator.rs"
        status: pass
    human_judgment: false
  - id: D8
    description: "The residuals this plan does NOT close are named with their failure directions — five carrier types still bare String, ArchiveDepth::milestone, the file-body fields, the states no fixture constructs"
    verification: []
    human_judgment: true
    rationale: "A claim about what is NOT covered cannot be discharged by a passing test; it is a disclosure a reader must judge against the tree. The measurements behind each are quoted below."

duration: 62 min
completed: 2026-08-27
status: complete
---

# Phase 21 Plan 25: The `.planning/` Layer, Closed by the Same Carrier Summary

**Retyped the eight `.planning/`-derived carriers to `crate::text::Untrusted`, and the compiler named 38 render and logic sites where verification pass 9 had found 5 by reading — then populated every cache the eleven `DetailScreen` tabs read, which immediately produced four more live leaks that nine rounds of reading had never seen.**

## Performance

- **Duration:** 62 min
- **Completed:** 2026-08-27T17:47Z
- **Tasks:** 3 of 3
- **Files modified:** 10

## Task Commits

1. **RED: the session-id byte slice** — `235c3cc` (test) — committed deliberately red
2. **Task 1: the eight carriers take the type** — `f51ca76` (feat)
3. **Task 2: every cache populated, arrival recorded per tab** — `ea93dd4` (test)
4. **Task 3: the status footer, escaped at the consumer** — `ced37a7` (fix)

## 38 against 5 — the whole argument for this plan's shape

Verification pass 9 read `detail.rs` and found **five** raw `.planning/`-derived render sites. Retyping the carriers those sites read made `cargo build` name **twenty-seven**, and `cargo test --no-run` a further **eleven**: **38 in total**. Pass 9's five are a subset.

**The 27 in the production build**, verbatim from `cargo build --message-format short`:

| File:line | Error |
|---|---|
| `src/app.rs:1240` | `E0308` expected `Vec<Untrusted>`, found `Vec<String>` |
| `detail.rs:1531` | `E0308` expected `&str`, found `&Untrusted` |
| `detail.rs:1535` | `E0308` expected `Untrusted`, found `String` |
| `detail.rs:1601` | `E0599` no method named `len` on `&Untrusted` |
| `detail.rs:1601` | `E0608` cannot index into `&Untrusted` |
| `detail.rs:1610` | `E0277` `Untrusted` doesn't implement `Display` |
| `detail.rs:1654` | `E0308` expected `String`, found `Untrusted` |
| `detail.rs:1658` | `E0277` `String: Borrow<Untrusted>` not satisfied |
| `detail.rs:1672` | `E0308` expected `&str`, found `&Untrusted` |
| `detail.rs:1676` | `E0308` expected `String`, found `Untrusted` |
| `detail.rs:2420` | `E0277` `Untrusted` doesn't implement `Display` |
| `detail.rs:2915` | `E0277` `Untrusted` doesn't implement `Display` |
| `detail.rs:2941` | `E0277` `Untrusted` doesn't implement `Display` |
| `detail.rs:2946` | `E0599` `as_deref` bounds not satisfied on `Option<Untrusted>` |
| `detail.rs:3383` | `E0599` no method named `len` on `&Untrusted` |
| `detail.rs:3384` | `E0608` cannot index into `&Untrusted` |
| `detail.rs:3462` | `E0277` `Cow<'_, str>: From<Untrusted>` not satisfied |
| `detail.rs:3484` | `E0277` `Untrusted` doesn't implement `Display` |
| `detail.rs:3491` | `E0277` `Cow<'_, str>: From<Untrusted>` not satisfied |
| `detail.rs:3532` | `E0277` `Cow<'_, str>: From<Untrusted>` not satisfied |
| `detail.rs:3633` | `E0277` `Cow<'_, str>: From<Untrusted>` not satisfied |
| `detail.rs:3672` | `E0277` `Cow<'_, str>: From<Untrusted>` not satisfied |
| `detail.rs:3675` | `E0277` `Cow<'_, str>: From<Untrusted>` not satisfied |
| `detail.rs:3765` | `E0277` `Cow<'_, str>: From<Untrusted>` not satisfied |
| `detail.rs:3783` | `E0277` `Cow<'_, str>: From<Untrusted>` not satisfied |
| `detail.rs:3790` | `E0277` `Cow<'_, str>: From<Untrusted>` not satisfied |
| `detail.rs:4407` | `E0277` `Untrusted: AsRef<Path>` not satisfied |

**The 11 in the test build:** `browser.rs:148,163,175` (`as_str` on a carrier, in the tests' own name oracle) and `detail.rs:5605,5615,5632,5653,5678,5689,5702,6869` (eight fixture constructions).

**Not one site in this diff was found by reading a list.** The three exceptions are named below and each is disclosed as a site the compiler could NOT name.

## What was retyped, and where each is wrapped

| Carrier | Fields | Wrapped at |
|---|---|---|
| `state_reader::backlog::BacklogItem` | `dir_name`, `number`, `description`, `content` | the directory scan in `parse_backlog_items`, plus the content load in `detail.rs` |
| `archive::ArchiveFile` | `name` | both `read_dir` maps in `archive.rs` |
| `archive::PhaseArchive` | `name`, `display_name` | `parse_phase_dir` — `display_name` wrapped at the END of the `format!`, once, not the fragments |
| `browser::BrowserEntry` | `name` | `list_dir` |
| `session_detector::ClaudeSession` | `session_id` | `read_session_id`'s `/proc` scan |
| `ui::screens::ProjectViewCache` | `archive_milestones`, `archive_file_name`, `browser_file_name` | `app.rs:1240` and `detail.rs`'s own assignments |

Every compiler-named site carries a one-line reason saying WHICH question it answers. `as_raw_for_logic_only()` is used at 28 sites across `src/`, each a sort key, a map key, a path segment, a subprocess argument or a test oracle.

## The three sites the compiler could NOT name, said plainly

The lever gates carriers, not sinks, so a `.planning/`-derived value that travels as a bare `String` or a `PathBuf` is invisible to it. Three such sites were found by reading the ones the compiler DID name, and each is escaped at the render with the gap disclosed at the site:

1. **`detail.rs::archive_breadcrumb`, three draws of `ArchiveDepth::*::milestone`.** That field is `archive_milestones`' raw form taken as a navigation key and a `HashMap` key, so the carrier does not travel through the enum. **Residual: `ArchiveDepth::milestone` is not typed. Direction: under-protection, silent; bounded only by the probe.**
2. **`detail.rs`'s Browse breadcrumb `rel_path`** — a `PathBuf` diff, not a carrier. Found by the probe (red 2 below), not by reading.
3. **`archive::render_markdown_lines`** — a `&str` document body. Found by the probe (red 4 below).

## The Defaults, Browse, Driver and file-view leaks — four reds the populated fixture produced

`probe_ctx` reached five of the eleven tabs' caches only through `view_cache.entry(..).or_default()`, so those tabs rendered their EMPTY branch on every probe run and every site below that branch was exercised by nothing. Populating them produced four reds in sequence, each a live leak, each verbatim:

**1. Defaults tab** — `entry.value`, the value of every key of the project's `.planning/config.json`, drawn raw:

```text
thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (699114) panicked at src/ui/screens/render_escape_guard.rs:1632:29:
DetailScreen (src/ui/screens/detail.rs) [Defaults tab] rendered the RAW hostile identity "demo\u{e0041}r\u{ad}un" into the terminal buffer. What the operator reads is therefore a value the terminal may reorder, hide characters in, or render as a different string entirely — this is Trojan Source (CVE-2021-42574) in a cell. Route what a human READS through crate::text::render_for_terminal; the raw value belongs only in lookups, map keys, path segments, subprocess arguments and persistence.
```

**2. Browse tab** — the breadcrumb path relative to `.planning/`, drawn raw:

```text
thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (702868) panicked at src/ui/screens/render_escape_guard.rs:1670:17:
DetailScreen (src/ui/screens/detail.rs) [Browse tab] rendered ['\u{e0041}'] into the terminal buffer. Those characters render as nothing, so what the operator reads is not what the value is.
```

**3. Driver tab** — `run_id_suffix`, `goal`, `gsd_command` and `run_dir` went through `sanitize_render_line` ALONE: the CONTROL class applied, the invisible-formatting class not. WR-01/WR-02's split, live, on the one path whose single already-composed site (`no_runs_lines`) was the only one the empty fixture ever rendered:

```text
thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (719875) panicked at src/ui/screens/render_escape_guard.rs:1670:17:
DetailScreen (src/ui/screens/detail.rs) [Driver tab] rendered ['\u{ad}', '\u{e0041}', '\u{e0041}', '\u{e0041}'] into the terminal buffer. Those characters render as nothing, so what the operator reads is not what the value is.
```

**4. Browse tab, file view** — `archive::render_markdown_lines`, the ONE render for both file viewers, drew the markdown body raw:

```text
thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (719875) panicked at src/ui/screens/render_escape_guard.rs:1670:17:
DetailScreen (src/ui/screens/detail.rs) [Browse tab, file view] rendered ['\u{e0041}'] into the terminal buffer. Those characters render as nothing, so what the operator reads is not what the value is.
```

Escaped **per LINE, never over the document** — `crate::text::strip_terminal_controls` replaces every C0 control with a visible marker and `\n` is `0x0A`, so escaping first and splitting second would collapse a whole file into one row.

## The per-tab arrival table — all fifteen states, with their measurements

**Plain containment was MEASURED WRONG and is reported as such, because it is the exact defect this plan was written to avoid one level up.** `DetailScreen` draws the registry key into its bordered block's title on every tab, so `clean_text.contains(clean)` answers `true` for all fifteen states whatever the tab body renders. With `probe_ctx`'s `backlog_items` cleared and nothing else changed, the first implementation of the arrival record **still reported the Backlog tab as arriving**. A table of fifteen `true`s built on that would have been coverage theatre.

`chrome_ctx` renders the same fifteen states with every tab-body source emptied (the alias stays registered — the chrome is what is being measured) and the assertion requires strictly MORE occurrences. The baseline is measured on the same run, so it is not a number anybody has to maintain.

| # | State | Cache populated | Occurrences | Chrome baseline | Arrived (body) | Cause where absent |
|---|---|---|---|---|---|---|
| 1 | PhaseList tab | `project_states` | 7 | 2 | yes | — |
| 2 | RoadmapViz tab | `project_states` | 6 | 2 | yes | — |
| 3 | Backlog tab | `backlog_items` (new) | 3 | 1 | yes | — |
| 4 | GitHistory tab | `git_entries` (21-23) | 5 | 1 | yes | — |
| 5 | Pipeline tab | `project_states` | 2 | 1 | yes | — |
| 6 | Queue tab | `project_states` | 2 | 1 | yes | — |
| 7 | Sessions tab | `active_sessions` (new) | 2 | 1 | yes | — |
| 8 | Archive tab | `archive_milestones` (new) | 2 | 1 | yes | — |
| 9 | Defaults tab | `defaults_config` (new) | 4 | 1 | yes | — |
| 10 | Browse tab | `browser_entries` + dirs (new) | 4 | 1 | yes | — |
| 11 | Driver tab | `driver_runs` (new) | 6 | 2 | yes | — |
| 12 | Backlog tab, expanded | `backlog_expanded` (new) | 5 | 1 | yes | — |
| 13 | Archive tab, phase list | `archive_cache` (new) | 4 | 2 | yes | — |
| 14 | Archive tab, file list | `archive_cache` (new) | 4 | 2 | yes | — |
| 15 | Browse tab, file view | `browser_file_content` (new) | 4 | 2 | yes | — |

**Every state arrives, and no state is populated-but-unread.** The `Cause where absent` column is empty because there is nothing absent — not because the check was skipped. Had a cache been populated whose render the tab never reads, the assertion would name it and say which of the two causes to check first.

The plan's `defaults_config` clause allowed for `RENDERS_NO_IDENTITY` if the tab drew only authored labels. It does not: `mode`, `granularity`, `project_code`, `phase_naming` and `response_language` are free-form strings from the project's own `config.json`, and the tab drew them raw (red 1). The plan's Driver clause allowed for "if reaching its populated branch is possible without a live run" — it is; the render only asks whether `driver_runs` is empty.

### The arrival record is non-vacuous, observed red by planting in BOTH directions

**Missing direction** — `probe_ctx`'s `backlog_items` cleared, nothing else changed:

```text
DetailScreen (src/ui/screens/detail.rs): DETAIL_TAB_ARRIVAL claims the clean identity reaches ["Backlog tab", "Backlog tab, expanded"], and it did not. TWO CAUSES, in order of likelihood. (1) The fixture does not reach that render — the cache `probe_ctx` populates is not the one the tab reads, or a second field gates the branch (`backlog_expanded`, `archive_depth`, `browser_depth`, `searching`). Tell this apart by rendering the state and looking for the tab's EMPTY-branch string in the buffer; if it is there, the fixture is the problem. (2) The tab genuinely draws no identity — then the row is wrong and must be flipped to `false` with that reason. Do NOT delete the row: a populated cache whose render is never reached is a fixture that proved nothing, and it must be reported rather than counted as coverage. Arrived: {"Archive tab", "Archive tab, file list", "Archive tab, phase list", "Browse tab", "Browse tab, file view", "Defaults tab", "Driver tab", "GitHistory tab", "PhaseList tab", "Pipeline tab", "Queue tab", "RoadmapViz tab", "Sessions tab"}
```

**Unexpected direction** — the `Sessions tab` row flipped to `false`:

```text
DetailScreen (src/ui/screens/detail.rs): the clean identity reached ["Sessions tab"], which DETAIL_TAB_ARRIVAL records as drawing none. The row was written about a render that has since changed; re-read it, say which value the tab now draws and where its bytes come from, and flip the row to `true`.
```

Both plants reverted immediately; `rtk proxy git status --porcelain` confirmed clean before the commit.

## The multibyte session-id panic (T-21-25-05)

**RED against the ORIGINAL `render_sessions_tab` byte slice, committed at `235c3cc` before any of this plan's other edits:**

```text
thread 'ui::screens::detail::tests::a_multibyte_session_id_does_not_panic_the_render' (594330) panicked at src/ui/screens/detail.rs:3384:32:
end byte index 8 is not a char boundary; it is inside 'U+4E2D' (bytes 6..9 of string)
```

**RE-OBSERVED against the FINAL fixture**, with `shorten_session_id`'s byte slice restored and nothing else changed:

```text
thread 'ui::screens::detail::tests::a_multibyte_session_id_does_not_panic_the_render' (658292) panicked at src/ui/screens/detail.rs:117:12:
end byte index 8 is not a char boundary; it is inside 'U+0915' (bytes 6..9 of string)
```

(The panic names the character with its literal glyph; it is written here as its code point, following the house rule that no raw glyph appears in an artifact.)

**The fixture changed mid-task and the reason is a measurement, not a preference.** The first fixture used CJK. CJK glyphs are TWO terminal cells wide, so the probe's cell-by-cell buffer scrape produced `KA<blank>KA<blank>…` and the assertion failed for a reason with nothing to do with the truncation. `U+0915` DEVANAGARI LETTER KA is three bytes and one cell, which is the shape that exercises the byte boundary without confounding the scrape. Recorded in the test's own doc.

Truncation happens **before** escaping, deliberately: escaping expands each invisible-class character into a six-character `U+XXXX` marker, so escaping first and cutting at eight could print half a marker as if it were data.

## The status footer — the one deliberate inversion (WR-03)

**RED, with the fixture state added and `normal.rs`'s status branch still raw:**

```text
thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (781859) panicked at src/ui/screens/render_escape_guard.rs:1987:17:
NormalScreen (src/ui/screens/normal.rs) [dashboard with a status message] rendered ['\u{e0041}', '\u{e0041}'] into the terminal buffer. Those characters render as nothing, so what the operator reads is not what the value is.
```

**Re-observed after green** by reverting that one line and restoring it (`794749`, same message).

`rtk proxy grep -c "status_message = Some" src/app.rs` re-measured: **6**, and **none of the six was edited** — `git diff` for `src/app.rs` touches only the `ArchiveMilestonesDiscovered` arm. Four of the six interpolate a registry key or a run id into a sentence this build wrote; one is a literal; the sixth (`:2054`) forwards whatever ANY screen handed to `ScreenAction::SetStatusMessage`, which is why the producer set **cannot be closed by inspection**.

**The fixture state is proven to reach the branch, in both directions.** `the_status_footer_state_reaches_the_status_branch` asserts `STATUS_BRANCH_TOKEN` (`"Driving "`, drawn by nothing else on this screen — `render_normal_footer` paints counts, a sort indicator and hints) arrives with `searching: false`, and is ABSENT with `searching: true`. If `searching` were left `true`, the probe would report the state as drawing the search footer and the whole assertion would pass by silence — which is the near-miss this module already records at its `dashboard with filter footer` state.

**The `NormalScreen` disposition row** now reads, in part: *"the surface no row named until 21-25 — draws `ctx.status_message` in its STATUS FOOTER … **THE ESCAPE FOR THIS SURFACE LIVES AT THE RENDER SITE, NOT AT THE PRODUCER**, and a reader who assumes round 9's producer rule holds everywhere will look for it in the wrong file: the trust boundary runs through the middle of a `format!`, so there is no field a carrier could type."* Fixture states: the dashboard, the dashboard with a status message, and the dashboard with the filter footer active.

## LIMIT 1, old and new, side by side

**OLD** (the clause following "The probe sees only what a screen renders under the states its fixture constructs"):

> Not bounded at all within a state: the Backlog, Sessions, Archive, Browse and Defaults tabs, and the Driver tab's run list and run detail, draw from `view_cache` / `archive_cache` / `active_sessions` entries that `probe_ctx` leaves at their defaults, so those tabs render their empty branch and their populated branches are **not** exercised by any committed control. That residual is disclosed, not closed.

**NEW:**

> Every one of those caches is now populated by `probe_ctx`, so all eleven tabs render a POPULATED branch on every run, and three of them are additionally probed in the second state their own dispatch field selects … **What REMAINS, with a concrete example, because a residual with no example is a residual nobody can check.** The residual is now *states no fixture constructs* rather than *tabs no fixture populates*. Concretely: the Defaults tab's string-EDIT overlay — `defaults_editing = Some(idx)` on a `ConfigValueKind::String` entry — draws `defaults_text_buffer` and `entry.key` into a `Clear`ed popup through a code path no probe state reaches, and the Driver tab's `driver_dry_run` preview is another. **Under-detection, silent.**

The new wording is strictly narrower: it names **two concrete states** where the old named **six whole tabs**, and it comes with a per-state arrival check the old had no equivalent of.

## Gates — every number re-measured under `rtk proxy` against the final tree

| Gate | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo test --workspace --no-fail-fast -- --test-threads=2` | **1379 passed / 0 failed / 13 ignored** |
| `cargo clippy -- -D warnings` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | **exactly FOUR** pre-existing lints — `bool_assert_comparison` ×3 at `src/browser.rs:155,156,157`, `cmp_owned` ×1 at `src/project_creator.rs:146` |
| `cargo test --test spawn_seam_guard` | 38 passed / 0 failed / 0 ignored |
| `cargo test --test driver_injection_corpus` | 13 passed / 0 failed / 10 ignored |

**The clippy prohibition, expressed as a CHECKED property (D-21-24) because this plan edits `src/browser.rs`:**

- **Before:** four lints — `bool_assert_comparison` at `src/browser.rs:131,132,133`, `cmp_owned` at `src/project_creator.rs:146`.
- **After:** four lints — `bool_assert_comparison` at `src/browser.rs:155,156,157`, `cmp_owned` at `src/project_creator.rs:146`.

Same count, same two kinds, same two files. The `browser.rs` line numbers moved by exactly 24, which is the size of the insertions above them (the `BrowserEntry` doc comment, the `Untrusted` import and the `list_dir` change); **none of the three `assert_eq!` lines was touched** — the only edit inside that test module replaced `e.name.as_str()` with `e.name.as_raw_for_logic_only()` at three sites, which the compiler demanded.

**Test total delta, every unit attributed:** baseline **1376** (21-23's final) → **1379**, +3.

| + | Test | Task |
|---|---|---|
| 1 | `ui::screens::detail::tests::a_multibyte_session_id_does_not_panic_the_render` | 1 |
| 1 | `ui::screens::detail::tests::a_long_multibyte_session_id_is_truncated_by_characters_not_bytes` | 1 |
| 1 | `ui::screens::render_escape_guard::tests::the_status_footer_state_reaches_the_status_branch` | 3 |

## Re-measured reference counts

| Quantity | Plan's figure (at `dfa11c6`) | Measured here (final tree) |
|---|---|---|
| `BacklogItem` / `backlog_items` refs under `src/` | 30 | **31** |
| `BrowserEntry` / `browser_entries` refs | 28 | **38** |
| `archive_milestones` refs | 8 | **15** |
| `ArchiveFile` / `display_name` refs | 13 | **27** |
| `status_message = Some(` in `src/app.rs` | 6 | **6** |
| `DetailScreen` sub-views (`ALL_SUB_VIEWS`) | 11 | **11** (+4 within-tab states = 15 probe states) |
| `Untrusted` refs under `src/` | — | **112** |
| `as_raw_for_logic_only` call sites under `src/` | — | **28** |
| `Span::raw` under `src/` | 129 (21-23: 139) | **140** |
| `Span::styled` under `src/` | 198 (21-23: 201) | **201** |

## Prohibition compliance

| # | Prohibition | Status |
|---|---|---|
| 1 | No edit to `src/error.rs`, `src/registry.rs`, `src/main.rs`, `src/executor/mod.rs`, `src/driver/**`, `src/cli.rs`, `driver_confirm.rs`, `delete_confirm.rs`, `src/text.rs`, `git_ops.rs`, `src/action.rs`, `tests/registry_test.rs`, `tests/driver_dry_run.rs`, `tests/journal_run_paths.rs` | **Met** — `git diff --stat` against the plan base names none of them. `registry.rs:813` sets `session_id: None`, which needed no change; `action.rs`'s `ClaudeSession` and `Vec<String>` milestones are both handled at the `app.rs` receiver instead. |
| 2 | The four pre-existing clippy lints untouched, CHECKED | **Met** — before/after both quoted above; four, same kinds, same files |
| 3 | No re-disclosure of the coverage hole in place of closing it | **Met** — every tab is populated AND its arrival is measured against a chrome baseline; the containment version that would have re-disclosed by accident is reported as a defect above, not hidden |
| 4 | No escape of `ctx.status_message` at its producers | **Met** — all six re-measured and none edited; the escape is at `normal.rs`'s status branch with the inversion argued there |
| 5 | No work against ROADMAP criterion 4 | **Met** — `tests/driver_injection_corpus.rs` and `tests/spawn_seam_guard.rs` RUN and neither EDITED; `git diff --stat` for both is empty |
| 6 | No bound claimed that no committed control goes red for | **Met** — each residual below carries its direction; the two claims that could have been vacuous (per-tab arrival, the status-branch dispatch) each have a both-directions control |
| 7 | No raw invisible, bidi, tag, variation-selector or control character in any file | **Met** — a `Cf`/`Cc`/tag-block/variation-selector scan over the whole 105,400-character plan diff returns **0**, and the scan is non-vacuous: the same function reports **6 of 6** on a control string built from code points rather than pasted |
| 8 | No `.planning/REQUIREMENTS.md` requirement flipped | **Met** — `git diff --stat` against the plan base does not name it |
| 9 | Every number re-measured under `rtk proxy`, none inherited | **Met** — see the two tables above; the plan's own figures are quoted beside the measured ones and four of them did not reproduce |

## Deviations from Plan

### 1. [Rule 2 — missing critical functionality] `src/ui/screens/driver.rs` edited, and it is not in this plan's `files_modified`

- **Found during:** Task 2, red 3.
- **Issue:** Populating `driver_runs` made the Driver tab render its run list for the first time under probe, and it leaked: `run_id_suffix`, `goal`, `gsd_command` and `run_dir` all went through `sanitize_render_line` alone — the control class applied, the invisible-formatting class not. That is WR-01/WR-02's split, live, and this file's OWN `no_runs_lines` already states that neither class subsumes the other and composes both. It was the one composed site, and it was the only one the empty fixture ever rendered.
- **Fix:** a module-level `shown_capped` composing `display_identity(&sanitize_render_line(..))` — the tree's documented capped composition, correct here because this path draws agent prose — routed through at five sites.
- **Wave-conflict check:** `src/ui/screens/driver.rs` is in NEITHER prohibition 1's list nor plan 21-24's declared files. Prohibition 1 names `src/driver/**` (the non-UI module) and `src/ui/screens/driver_confirm.rs`; this is a third file. Recorded here rather than done quietly.
- **Commit:** `ea93dd4`

### 2. [Rule 1 — bug] The first per-tab arrival implementation was measured wrong and would have been coverage theatre

- **Found during:** Task 2, step (c).
- **Issue:** `clean_text.contains(clean)` cannot tell a tab body from a block title. `DetailScreen` draws the registry key in its bordered block's title on every tab, so the check answered `true` for all fifteen states. Verified by planting: with `backlog_items` cleared the check **still passed**.
- **Fix:** `chrome_ctx` plus `detail_chrome_baseline`, comparing occurrence COUNTS against the same fifteen states rendered with every tab-body source emptied. `states_over_sub_views` was parameterised on its context builder rather than duplicated, so the baseline is measured through the identical state walk.
- **Verification:** re-planted in both directions; both reds quoted above.
- **Commit:** `ea93dd4`

### 3. [Rule 2] `archive::render_markdown_lines` escapes, closing a gap the plan left half-open

- **Found during:** Task 2, red 4.
- **Issue:** The plan says `archive_file_content` and `browser_file_content` stay `String` because they are file bodies, "named in the remainder rather than half-done here". Populating `browser_file_content` showed the body reaching a cell with a tag character intact.
- **Fix:** the fields stay `String` as the plan directs, and the ONE function both viewers render through escapes every line. This makes the disclosure narrower rather than leaving it as written: the FIELD is untyped, the RENDER is closed.
- **Commit:** `ea93dd4`

### 4. [Deviation — a fixture the plan specified is confounded by the render harness]

- **Found during:** Task 1, step (d).
- **Issue:** The plan names `"\u{4e2d}\u{6587}\u{4e2d}\u{6587}\u{4e2d}"` as the multibyte session id. CJK is two terminal cells wide, so the probe's cell-by-cell scrape yields the glyph followed by a blank continuation cell and `text.contains(id)` fails for a reason unrelated to truncation.
- **Resolution:** `U+0915` (three bytes, one cell) — the same byte-boundary shape without the width confound. The original CJK red is still committed at `235c3cc` and both reds are quoted. Recorded rather than silently substituted.
- **Commit:** `f51ca76`

### 5. [Deviation — the plan's fixture allowances were not needed]

- **Found during:** Task 2, step (a).
- **Issue:** The plan allowed the Defaults tab to be recorded `RENDERS_NO_IDENTITY` "if it renders only authored labels", and allowed the Driver tab to be excluded if its populated branch needed a live run.
- **Resolution:** neither escape hatch applies. The Defaults tab draws five free-form config strings (and drew them raw), and the Driver run list needs only a non-empty `driver_runs`. Both are fully covered. Recorded so a reader does not assume the allowances were taken.
- **Commit:** `ea93dd4`

---

**Total deviations:** 5 — 3 auto-fixed findings (Rules 1 and 2), 2 plan-text resolutions recorded rather than patched quietly.
**Impact on plan:** scope grew by one file (`src/ui/screens/driver.rs`) and one function (`render_markdown_lines`), both because the fixture this plan built found live leaks in them. No scope was dropped.

## Recurrence check — did this round enumerate one level down?

Phase standing constraint 5 requires this be answered plainly. **No, and the measurement says so rather than the prose.**

- The five sites were closed by changing **eight types**, and the 38 sites were produced by `cargo build`, not by reading. 38 > 5 is the check the plan asked for, and it passed.
- The fixture hole was closed by populating **every** cache in one task, and the honesty of that was closed by a **measured chrome baseline**, not by a table of `true`s somebody wrote.
- The status footer was closed by **stating the inversion** with its residual, not by adding `display_identity` at a sixth producer.

**Where this round DID add to a list, said plainly.** Three places:

1. `DETAIL_TAB_ARRIVAL` is a hand-maintained table of fifteen labels. Its direction is benign — a missing row makes the `unlisted` assertion go red naming the state, so the list cannot silently shrink — but it is a list.
2. `DETAIL_SUB_STATES` is a hand-written set of four within-tab states. **A fifth dispatch field nobody adds a state for is invisible**, and that is precisely LIMIT 1's new residual.
3. `hostile_gsd_config` names three of `GsdConfig`'s string keys. A fourth string key added tomorrow is not in the fixture. Direction: under-detection, silent.

## Residuals, each with its failure direction

| # | Residual | Direction | Bounded by a committed control? |
|---|---|---|---|
| 1 | Five carrier types keep bare `String` fields — `ProjectState` (5), `RoadmapPhase` (4), `QueuedAction` (1), `AppContext::filtered_aliases`/`selected_alias`, `GsdConfig` | under-protection, silent | **No** — census + probe only. **The bound IMPROVED here**: every tab that reads them now renders its populated branch, so the sampling is materially better. That is an improvement in the bound, not a closure of it. |
| 2 | `ArchiveDepth::*::milestone` is a bare `String` carrying `.planning/`-derived bytes through an enum | under-protection, silent | **No** — escaped at three render sites found by reading; nothing stops a fourth |
| 3 | `archive_file_content` / `browser_file_content` are bare `String` | under-protection at the FIELD | **Yes at the render** — `render_markdown_lines` escapes every line and the probe goes red without it |
| 4 | `ctx.status_message` has no carrier; a seventh producer inherits the render-site escape, a producer that puts a key elsewhere does not | under-protection, silent, outside this field | Partially — the field's render is bounded by the probe; the producer set is open by construction (`SetStatusMessage`) |
| 5 | States no fixture constructs — the Defaults string-edit overlay, the Driver dry-run preview | under-detection, silent | **No** — named with examples so they are checkable |
| 6 | `DETAIL_SUB_STATES` / `hostile_gsd_config` are hand-written lists | under-detection, silent | Partially — a missing STATE is invisible; a missing LABEL is not (`unlisted` assertion) |
| 7 | Homoglyphs are outside the judged class | under-detection, by design | **No** — closed at identity seams by the finite alphabet only |
| 8 | ROADMAP criterion 4's behavioural half | permanently agent-unclosable | **No** — deliberately, and this plan did no work against it |

## Known Stubs

None. No hardcoded empty value, placeholder string, TODO or unwired component was introduced.

## Issues Encountered

**1. The naive arrival check passed a plant, and it was caught before it shipped.** Documented above as deviation 2 and in `chrome_ctx`'s doc. It is worth restating as an issue rather than only as a fix: the first version of this plan's headline mechanism was the exact failure shape the plan was written to prevent, and what caught it was planting rather than reading.

**2. Four `.planning/` render leaks existed at HEAD that nine rounds of review had not found**, and all four were invisible for the same reason — the fixture rendered an empty branch. Two of them (Defaults, Driver) had been live since those tabs were built.

**3. No flake fired during this plan's runs.** The three documented flakes (`driver_reattach` ×2, `envelope_tracer`) all passed in the final full-suite run.

## Next Phase Readiness

`21-26` is unblocked. It inherits the populated probe fixture, so the census/alphabet work it does will be exercised against tabs that render real content rather than empty branches.

**Ready for 21-26.**

## Self-Check: PASSED

- All ten modified files present on disk and named in `git diff --stat` against the plan base `d712061`.
- Commits `235c3cc`, `f51ca76`, `ea93dd4`, `ced37a7` all present in `git log --oneline`.
- Every task-level acceptance criterion re-run against the final tree: the retyped-field greps all return 0; the compiler-named site count is quoted and compared against five; the multibyte panic is quoted verbatim before the fix and its tests pass after; the four clippy lints are quoted before and after; the per-tab table covers all fifteen states; LIMIT 1's two wordings are quoted side by side; both SAFE-07 backstops re-run at their counts with `git diff --stat` naming neither.
- Plan-level `<verification>` re-run: `cargo build` exit 0; `cargo test --workspace --no-fail-fast -- --test-threads=2` **1379/0/13** with every delta attributed; `cargo clippy -- -D warnings` exit 0; `--all-targets` exactly four lints, same kinds, same files; **six verbatim reds** recorded (four tab reds, the multibyte panic, the status-footer red) plus two planted arrival reds and one re-observed status-footer red; `.planning/REQUIREMENTS.md`, `tests/spawn_seam_guard.rs` and `tests/driver_injection_corpus.rs` untouched.
