---
phase: quick-260509-zh2
plan: 01
status: complete
date: 2026-05-09
commits:
  - 55baa9d
  - 75e80c2
files_modified:
  - src/state_reader/config_json.rs
  - src/state_reader/disk_status.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/detail.rs
---

# Quick Task: Defaults Layering, /gsd-settings Sections, Pipeline Drill-Down

## What Changed

### 1. `~/.gsd/defaults.json` is layered under project config

`load_user_defaults()` reads the file (if it exists) and feeds it
into `build_defaults_entries` as a fallback. For each row whose
project value is `None`, the value from defaults is shown instead
with a `*` magenta marker after it. This mirrors GSD's own
`config-mutation.js` "hardcoded ← globalDefaults ← userChoices"
merge, but only for display — the project's `.planning/config.json`
is still the source of truth at runtime.

### 2. Six-section layout

The flat General / Search / Git / Workflow / Hooks / Intel / Graphify
split was replaced with the six categories used by GSD's
`/gsd-settings`:

  - **Planning** — research, plan_check, pattern_mapper,
    nyquist_validation, ui_phase, ui_safety_gate, ai_integration_phase,
    subagent_timeout
  - **Execution** — verifier, tdd_mode, code_review, code_review_depth,
    ui_review, node_repair, node_repair_budget
  - **Docs & Output** — commit_docs, skip_discuss, use_worktrees,
    text_mode, response_language
  - **Features** — intel.enabled, graphify.enabled,
    graphify.build_timeout, brave_search, firecrawl, exa_search
  - **Model & Pipeline** — mode, granularity, model_profile,
    parallelization, auto_advance, auto_chain_active,
    branching_strategy, base_branch, *_branch_template
  - **Misc** — context_warnings, research_before_questions,
    discuss_mode, search_gitignored, project_code, phase_naming

Six new `WorkflowConfig` fields were added to support this:
`pattern_mapper`, `ai_integration_phase`, `tdd_mode`, `code_review`,
`code_review_depth`, `ui_review`. They flow through the
get/set/clear/toggle path the same as the existing toggles.

### 3. `[d]` toggle to edit `~/.gsd/defaults.json`

A new `DefaultsEditTarget` enum on `ProjectViewCache` tracks whether
the Defaults tab is editing the project config or the global file.
`d` (when no popup is open) flips between them. The tab title
updates to show which file is active. When toggling into Global mode
on a system that has never created the defaults file, an empty
`GsdConfig` is bootstrapped so the user can populate it; on save we
`mkdir -p ~/.gsd` and write atomically.

`set_config_value`, `clear_config_value`, `set_string_value`, and
`mutate_config_entry` all run against the active target — no logic
duplication.

### 4. Pipeline sub-stage drill-down

`DiskInference` gained eight `has_*` flags for artifacts produced by
optional GSD steps:

  - has_patterns      → PATTERNS.md
  - has_plan_check    → PLAN-CHECK.md
  - has_validation    → VALIDATION.md (Nyquist)
  - has_ui_spec       → UI-SPEC.md
  - has_ui_check      → UI-CHECK.md
  - has_ai_spec       → AI-SPEC.md
  - has_review        → REVIEW.md
  - has_ui_review     → UI-REVIEW.md

`infer_disk_status` matches these before the existing PLAN/SUMMARY
patterns to avoid double-counting (PLAN-CHECK.md and UI-REVIEW.md
both end in -REVIEW.md / -PLAN.md naïvely; the order matters).

The Pipeline tab now appends a sub-stage block under the top-level
D/R/P/E/V line whenever any sub-artifact is present:

```
  Plan sub-stages:
    ✓ Patterns       done
    ○ UI-Spec        not run
    ○ AI-Spec        not run
    ✓ Plan-Check     done
    ○ UI-Check       not run
    ○ Nyquist        not run

  Execute sub-stages:
    ✓ Code Review    done
    ○ UI Review      not run
```

Sections stay collapsed (not rendered) until at least one of their
artifacts shows up, so untouched phases don't carry visual noise.

## Verification

- `cargo build`: clean
- `cargo clippy`: no issues
- `cargo test`: 102 passed (5 suites)

## Notes

- We deliberately don't try to infer "skipped" vs "not started" for
  sub-stages without reading the project's workflow toggles. The
  honest signal is *artifact present or not*; the user can compare
  against their config to interpret the gap.
- The `*` defaults marker only fires for `Option<T>` fields. The three
  required strings (`mode`, `granularity`, `model_profile`) always
  display the project's value as-is — switching to `[d]` mode is the
  way to edit them globally.
