---
phase: quick-260909-s0n
plan: 01
subsystem: ui/defaults-tab
status: complete
tags: [tui, config, help, ratatui, defaults]
requires: []
provides:
  - "ConfigHelp — definition-site help for every Defaults-tab option"
  - "build_config_help_pane — the always-on help pane under the Defaults list"
  - "dropdown_popup_width — pure, character-measured, area-clamped popup width"
affects:
  - src/ui/screens/detail.rs
tech-stack:
  added: []
  patterns:
    - "Required struct field as a coverage mechanism: a new option with no help is a compile error, not a blank runtime row"
    - "Static-only render path as a security property by construction, rather than by remembering to call shown()"
    - "Populated render fixtures with an anti-empty-state precondition assertion"
key-files:
  created: []
  modified:
    - src/ui/screens/detail.rs
decisions:
  - "ID-01 — help lives on ConfigEntry, supplied through push(), so the compiler enforces coverage"
  - "ID-02 — the pane is always visible; `?` is already the global help key"
  - "ID-03 — String-kinded keys with conventional values stay String; their values are named in prose, choices stays empty"
  - "ID-04 — below HELP_PANE_FLOOR (12 rows) no pane is drawn and the list keeps the whole area"
  - "The test fixture is a POPULATED config parsed through parse_gsd_config, not GsdConfig::default() — measured necessity, not caution"
metrics:
  duration: one session
  completed: 2026-09-09
actuals:
  tokens: 16101
  tasks: 3
  commits: 3
plan_head_before: 7063b258b0f1c8f51cdd410f4c61d68d0f137f85
---

# Quick Task 260909-s0n: Help Text for Every Config Option — Summary

Every one of the 73 options the Defaults tab lists now carries a one-sentence
summary authored at its own `push(...)` call site, rendered in an always-on pane
under the list, with per-value explanations for the six enum options shown both
in the pane and inside the open dropdown.

## What Was Built

**`ConfigHelp` — a required field, not a lookup table (ID-01).** A
`#[derive(Debug, Clone, Copy)]` record of `summary: &'static str` plus
`choices: &'static [(&'static str, &'static str)]`, carried on `ConfigEntry` and
supplied as the seventh parameter of `build_defaults_entries`'s `push` closure.
Adding a 74th option without writing its help does not compile. A
`HashMap<&str, &str>` keyed by config key would have compiled fine and rendered a
blank line — that difference is the whole reason for the field.

**73 summaries, written from GSD's own reference.** Every summary was authored
against `gsd-core/references/planning-config.md`, `bin/shared/config-defaults.manifest.json`,
`bin/lib/capability-registry.cjs` and the relevant workflow docs, not from the key
name. Units are named where a bare number would be the defect
(`subagent_timeout` milliseconds, `workflow.test_gate_timeout` and
`graphify_build_timeout` seconds, `external_job.*_timeout_ms` milliseconds).

**The help pane.** `render_defaults_tab` splits its area with `Layout::vertical`,
reserving `HELP_PANE_HEIGHT` (4) rows, and draws a `Wrap { trim: true }`
`Paragraph` with a `Borders::TOP` block. Below `HELP_PANE_FLOOR` (12) the split is
skipped entirely and the list keeps the full area (ID-04). Both popups still
centre over the **original** `area`, so an open dropdown does not drift upward as
the pane appears.

**The dropdown.** Each option now draws its explanation dim beside its value,
value first. `dropdown_popup_width` was extracted as a pure function measuring
`chars().count()` rather than `len()`, keeping the existing
`.min(area.width - 2)` clamp — so when the clamp bites it is the explanation that
is cut, never the selectable value (T-S0N-02).

## Tasks and Commits

| Task | Name | Commit |
|------|------|--------|
| 1 (tracer) | One option, definition to terminal cell — then all 73 | `f0f7e00` |
| 2 | Explain each choice, in the pane and in the open dropdown | `2e6ed52` |
| 3 | Pin the pane against the terminal buffer, then run the full gate | `ae76ce9` |

## Verification — Measured

| Gate | Baseline (planner, 2026-09-09) | After |
|------|--------------------------------|-------|
| `cargo build` | succeeds | **exit 0** |
| `cargo clippy -- -D warnings` | exit 0 | **exit 0** |
| `cargo test --lib` | 1187 passed / 1 failed / 1 ignored | **1193 passed / 1 failed / 1 ignored** |
| `cargo test --no-fail-fast` | (not in the plan's baseline) | **1942 passed / 1 failed / 14 ignored** |

Six tests added, and the pass count rose by exactly six. The single failure is
unchanged and pre-existing:
`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
— a SCHEDULE assertion firing because the installed git is 2.53.0 while the
constants record 2.43.0. It was red before this task and is red after it, and was
deliberately not fixed here.

All commands were run through `rtk proxy` with output captured to files, per
project memory: `rtk` strips `warning:` and `test result:` lines, so a count read
through the filter under-reports, and a grep piped downstream of `rtk proxy` is
filtered again.

### Tests added

| Test | What it pins |
|------|--------------|
| `every_config_entry_carries_a_non_empty_summary` | 73 options, each with a summary of 20+ characters |
| `every_choice_bearing_entry_documents_exactly_its_dropdown_options` | `help.choices` equals `dropdown_options` for enums, membership otherwise |
| `the_dropdown_popup_never_exceeds_the_area_width` | the clamp at widths 40/80/200, plus a lower bound so a `return 0` cannot satisfy it |
| `the_help_pane_renders_the_selected_entrys_summary` | the summary reaches real cells, AND the previous one leaves when the cursor moves |
| `a_short_area_keeps_the_option_list` | ID-04 at 120x10, and no panic at any height from 3 upward |
| `every_summary_is_within_the_pane_budget_and_is_not_a_restatement_of_the_key` | 160-character ceiling, and four words the key does not already spell |

### Fail-first evidence

Three assertions were proved capable of failing rather than assumed to be:

1. **The choice-coverage test.** Renaming `model_profile`'s `"budget"` choice to
   `"cheap"` produced
   `` `model_profile` documents ["quality", "balanced", "cheap", "adaptive", "inherit"] but its dropdown offers ["quality", "balanced", "budget", "adaptive", "inherit"] ``.
2. **The width test.** RED on a missing `dropdown_popup_width` before the
   function existed (`cannot find function dropdown_popup_width in this scope`).
3. **The render test's empty-state precondition.** Setting `defaults_config` to
   `None` produced
   `the fixture is not populated — the tab painted its empty state, so nothing below this line is about the help pane`.

## Deviations from Plan

**1. [Rule 1 — Wrong fact in the plan] `subagent_timeout` is milliseconds, not seconds.**

- **Found during:** Task 1, while sourcing summaries from GSD's own docs.
- **Issue:** The plan's writing rules named `subagent_timeout` alongside
  `workflow.test_gate_timeout` and `graphify_build_timeout` as "seconds".
  `gsd-core/references/planning-config.md` and
  `workflows/settings-advanced.md` both record it as **milliseconds**
  (`CONFIG_DEFAULTS` default `300000`, documented as five minutes).
- **Fix:** The summary says milliseconds and names the 300000 default. The other
  two remain seconds, which the reference confirms.
- **Why it matters:** a wrong unit in help text is worse than no help — this task
  exists to remove exactly that defect.
- **Commit:** `f0f7e00`

**2. [Rule 3 — Blocking] The test fixture had to be a populated config, not `GsdConfig::default()`.**

- **Found during:** Task 2, as a real RED.
- **Issue:** The plan specified building entries from `GsdConfig::default()` with
  `None` defaults. Under an all-`None` config every layered option resolves to
  `ConfigValueKind::Null`, so `dropdown_options` returns `[]` for all but the
  three plain-`String` enums. The choice-coverage assertion failed with
  `` `workflow.specless_probe_fallback` documents the choice "true", which its dropdown cannot select (it offers []) ``,
  and the `enum_entries == 6` assertion would have failed for the same reason.
- **Fix:** Added `populated_gsd_config()`, a JSON blob covering every key the tab
  reads, parsed through the production `parse_gsd_config`. The entry count is 73
  either way, so `every_config_entry_carries_a_non_empty_summary` still asserts
  what the plan asked; the coverage tests now assert it of real kinds. Going
  through `parse_gsd_config` also makes the fixture a check that this build's
  serde shape still reads GSD's own spelling (`_auto_chain_active`, the
  shape-varying `security_asvs_level`).
- **Commit:** `2e6ed52`

**3. [Scope] Committed on `master`.**

- The orchestrator directed a sequential run on the main working tree with
  worktree isolation off (`origin/HEAD` is unresolved in this repo, and
  `git.branching_strategy` is `none`). Every prior quick task in this repo is
  likewise on `master`. Recorded here because the executor's default protocol
  refuses a default-branch commit.

**4. [Deferred — out of scope] `dynamic_routing.provider_escalation` is modelled as a bool here, but GSD's is a list.**

- `src/state_reader/config_json.rs` types it `Option<bool>`. GSD's
  `model-resolver.cjs` reads it as an **ordered list of alternative model ids**
  (`references/execute-phase-quota-recovery.md`, `model-profiles.md`). The
  summary describes the behaviour honestly ("retries the step on an alternative
  provider's model") without claiming a shape. Fixing the model would change
  parsing and the edit affordance — a separate task, not this one.

## Options Whose Meaning Could Not Be Fully Established

Flagged for the ID-01..ID-04 audit. Every other summary is sourced from a named
GSD document.

| Key | What is uncertain |
|-----|-------------------|
| `capabilities.auto_update` | Present in `config-defaults.manifest.json` (default `false`) but carries no description anywhere in `gsd-core/`. The summary — refreshing installed capability packs to their latest published version — is **inferred** from the key's namespace and the surrounding capability-trust code. |
| `capabilities.strict_known_registries` | GSD's real value is `null` (permissive) / `[]` (lockdown) / `[hosts]` (allowlist), per `capability-trust.cjs`; this build renders it as a bool. The summary describes the policy rather than the boolean, and leaves `choices` empty. |
| `claude_orchestration.*` | Sourced from `bin/lib/claude-orchestration.cjs` (the `BACKEND_VALUES` enum and the `WORKFLOW_TOOL_FLOOR_VERSION` floor). There is no prose reference for this namespace in `gsd-core/references/`. |
| `workflow.windows_enforce` | Sourced from `bin/lib/broken-windows.cjs` and `workflows/ship.md`; no entry in the field reference table. |

## Known Stubs

None. No placeholder or key-echoing summary was written; the 20-character floor,
the 160-character ceiling and the four-novel-words rule are all asserted over all
73 entries.

## Threat Flags

None. No new network endpoint, auth path, file access pattern or schema change.
T-S0N-01 is mitigated by construction: `build_config_help_pane` and the dropdown's
explanation span both draw `&'static str` from `ConfigHelp` only, so no value out
of a project's `.planning/config.json` reaches a cell without `shown()`.
T-S0N-02 is mitigated by `dropdown_popup_width`'s `chars().count()` measure and
retained clamp, driven at three widths by a test.

## Self-Check: PASSED

- `src/ui/screens/detail.rs` — FOUND
- `f0f7e00` — FOUND
- `2e6ed52` — FOUND
- `ae76ce9` — FOUND
- `git rev-list --count 7063b25..HEAD` — 3, matching the `commits:` frontmatter
- `git diff --diff-filter=D` over the range — no deletions
