---
phase: quick-260512-ecm
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/disk_status.rs
  - src/ui/screens/detail.rs
autonomous: true
requirements:
  - QUICK-260512-ecm: Detect SECURITY.md sub-phase artifact and surface in pipeline drill-down
must_haves:
  truths:
    - "When a phase directory contains `<phase>-<plan>-SECURITY.md` (e.g. `01-01-SECURITY.md`) or a bare `SECURITY.md`, the dashboard reports the artifact as present"
    - "On the project detail screen, the Plan sub-stages section shows a `Security` row with a ✓ marker when SECURITY.md exists, and `○` when it does not (mirroring the existing Patterns/UI-Spec/Plan-Check rendering)"
    - "Phases that already use the secure sub-phase (e.g. usbee phase `01-tile-popover-hotplug-daemon-missing-state-v0-1`) light up the Security row when meta-manager points at that project"
    - "Detection does not alter overall pipeline status (D/R/P/E/V) — SECURITY.md is informational, like the other Plan sub-stage artifacts"
  artifacts:
    - path: "src/state_reader/disk_status.rs"
      provides: "DiskInference.has_security field and SECURITY.md detection branch"
      contains: "has_security"
    - path: "src/ui/screens/detail.rs"
      provides: "Security row in Plan sub-stages and inclusion in plan_touched"
      contains: "\"Security\""
  key_links:
    - from: "src/state_reader/disk_status.rs (infer_disk_status loop)"
      to: "DiskInference.has_security"
      via: "filename match: name == \"SECURITY.md\" || name.ends_with(\"-SECURITY.md\")"
      pattern: "has_security"
    - from: "src/ui/screens/detail.rs::build_substage_lines"
      to: "DiskInference.has_security"
      via: "push_substage(&mut lines, \"Security\", inf.has_security)"
      pattern: "push_substage.*Security"
---

<objective>
Add SECURITY.md to the list of GSD sub-phase artifacts that meta-manager detects and surfaces. This mirrors the existing per-phase artifact detection (PATTERNS.md, PLAN-CHECK.md, VALIDATION.md, UI-SPEC.md, UI-CHECK.md, AI-SPEC.md, REVIEW.md, UI-REVIEW.md) — no new abstraction, just one more entry in the same two places.

Purpose: GSD's optional `/gsd:secure-phase` produces `<phase>-<plan>-SECURITY.md` (confirmed via usbee: `~/projects/rust/usbee/.planning/phases/01-tile-popover-hotplug-daemon-missing-state-v0-1/01-01-SECURITY.md`). Currently meta-manager ignores it. Users running the secure sub-phase get no visual feedback in the TUI.

Output:
- `DiskInference.has_security: bool` populated by the existing single-pass directory scan
- "Security" row in the Plan sub-stages section of the project detail screen, with ✓ / ○ marker matching the existing rendering
- All existing tests still pass; one new test asserts `SECURITY.md` is detected
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
</execution_context>

<context>
@.planning/PROJECT.md
@.planning/STATE.md
@CLAUDE.md

# Existing pattern being mirrored (read these before editing):
@src/state_reader/disk_status.rs
@src/ui/screens/detail.rs

<interfaces>
<!-- Key types and existing patterns the executor must mirror. -->

From src/state_reader/disk_status.rs (current state):

```rust
#[derive(Debug, Clone, Default)]
pub struct DiskInference {
    pub status: DiskStatus,
    pub plan_count: u32,
    pub summary_count: u32,
    pub has_plans: bool,
    pub has_summaries: bool,
    pub has_context: bool,
    pub has_research: bool,
    pub has_verification: bool,
    /// Sub-stage artifacts (per /gsd-settings Planning + Execution toggles).
    pub has_patterns: bool,
    pub has_plan_check: bool,
    pub has_validation: bool,
    pub has_ui_spec: bool,
    pub has_ui_check: bool,
    pub has_ai_spec: bool,
    pub has_review: bool,
    pub has_ui_review: bool,
}
```

Detection branches in `infer_disk_status` use this exact shape (lines 91–122):

```rust
if name == "UI-REVIEW.md" || name.ends_with("-UI-REVIEW.md") {
    has_ui_review = true;
    continue;
}
// ... similar for REVIEW, UI-SPEC, UI-CHECK, AI-SPEC, PATTERNS, PLAN-CHECK, VALIDATION
```

Note: each branch uses `continue;` to prevent the artifact from being double-counted by the later generic `PLAN.md` / `SUMMARY.md` / `CONTEXT.md` / `RESEARCH.md` / `VERIFICATION.md` matchers. SECURITY.md does not collide with any generic suffix (no `-PLAN.md` etc. would ever end in `SECURITY`), but the `continue;` is still the idiomatic placement and keeps the branch shape uniform with its peers.

The struct is constructed in the return literal at lines 165–182 — every field is listed explicitly, so `has_security` must be added there too.

From src/ui/screens/detail.rs (current state, lines 3030–3067):

```rust
fn build_substage_lines(inf: &DiskInference) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    let plan_touched = inf.has_plans
        || inf.has_patterns
        || inf.has_plan_check
        || inf.has_validation
        || inf.has_ui_spec
        || inf.has_ui_check
        || inf.has_ai_spec;
    if plan_touched {
        lines.push(Line::from(Span::styled(
            "  Plan sub-stages:",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
        )));
        push_substage(&mut lines, "Patterns", inf.has_patterns);
        push_substage(&mut lines, "UI-Spec", inf.has_ui_spec);
        push_substage(&mut lines, "AI-Spec", inf.has_ai_spec);
        push_substage(&mut lines, "Plan-Check", inf.has_plan_check);
        push_substage(&mut lines, "UI-Check", inf.has_ui_check);
        push_substage(&mut lines, "Nyquist", inf.has_validation);
    }
    // ... exec section
}
```

`push_substage(&mut Vec<Line>, label: &'static str, present: bool)` renders one line with a ✓/○ marker plus the label.
</interfaces>

<reference_artifact>
Confirmed filename pattern from a real GSD project:
- `~/projects/rust/usbee/.planning/phases/01-tile-popover-hotplug-daemon-missing-state-v0-1/01-01-SECURITY.md`
- Format: `<padded-phase>-<padded-plan>-SECURITY.md` — exactly matches the existing pattern used by PLAN/SUMMARY/PATTERNS/etc. (suffix `-SECURITY.md`, or bare `SECURITY.md` for the standalone case).
</reference_artifact>
</context>

<tasks>

<task type="auto" tdd="true">
  <name>Task 1: Detect SECURITY.md in disk_status.rs</name>
  <files>src/state_reader/disk_status.rs</files>
  <behavior>
    - New test `test_security_md_detected`: write `05-01-SECURITY.md` into a tempdir, call `infer_disk_status`, assert `result.has_security == true`.
    - New test `test_standalone_security_md_detected`: write bare `SECURITY.md`, assert `result.has_security == true`.
    - New test `test_empty_dir_has_no_security`: empty dir, assert `result.has_security == false`.
    - Existing test `test_plans_only_returns_planned` and friends must still pass (SECURITY.md detection must NOT affect `plan_count`, `summary_count`, or the derived `DiskStatus`).
  </behavior>
  <action>
    Edit `src/state_reader/disk_status.rs`:

    1. Add `pub has_security: bool,` to the `DiskInference` struct (place it adjacent to `has_verification` on line 24 — it is the closest peer semantically: an optional, top-level-ish per-phase artifact that does NOT currently gate a pipeline stage). Keep `#[derive(... Default)]` — bool defaults to false, so all existing call sites using `..Default::default()` keep compiling.

    2. In `infer_disk_status`, add `let mut has_security = false;` adjacent to the other `let mut has_*` declarations (~line 76).

    3. Inside the `for entry in entries.flatten()` loop, add a detection branch mirroring the existing early-filter branches (the cluster of `if name == "UI-REVIEW.md" || ...`). Place it adjacent to the other sub-stage detectors (e.g. right after the VALIDATION branch, lines 119–122):

        ```rust
        if name == "SECURITY.md" || name.ends_with("-SECURITY.md") {
            has_security = true;
            continue;
        }
        ```

        The `continue;` is required for pattern consistency with peers — even though SECURITY does not collide with the generic PLAN/SUMMARY/CONTEXT/RESEARCH/VERIFICATION matchers, the uniform shape makes the cluster trivially auditable.

    4. Add `has_security,` to the `DiskInference { ... }` return literal at the bottom of the function (line 165 onwards). Place it alphabetically-ish or grouped near `has_verification` — match the visual grouping already used.

    5. Add the three new tests inside `#[cfg(test)] mod tests { ... }` at the bottom of the file, following the exact style of `test_context_only_returns_discussed` and `test_empty_dir_has_no_plans_or_summaries`.

    Do NOT touch `DiskStatus` (the enum), `derive_all_stage_statuses`, or the status-derivation logic. SECURITY is informational, not a pipeline gate.
  </action>
  <verify>
    <automated>cargo test --lib state_reader::disk_status -- --nocapture</automated>
  </verify>
  <done>
    - `DiskInference` has a public `has_security: bool` field
    - `SECURITY.md` and `*-SECURITY.md` files set `has_security = true`
    - Three new tests pass; all existing `disk_status` tests still pass
    - `cargo check` reports no warnings introduced by this task
  </done>
</task>

<task type="auto">
  <name>Task 2: Render Security row in Plan sub-stages</name>
  <files>src/ui/screens/detail.rs</files>
  <action>
    Edit `src/ui/screens/detail.rs::build_substage_lines` (currently ~lines 3030–3067):

    1. Extend the `plan_touched` expression to include `|| inf.has_security` so the Plan sub-stages section renders whenever Security alone is present (matches the existing behavior for every other sub-stage flag):

        ```rust
        let plan_touched = inf.has_plans
            || inf.has_patterns
            || inf.has_plan_check
            || inf.has_validation
            || inf.has_ui_spec
            || inf.has_ui_check
            || inf.has_ai_spec
            || inf.has_security;
        ```

    2. Inside the `if plan_touched { ... }` block, add a new `push_substage` call for "Security". Place it BEFORE `Patterns` so it reads naturally with the conceptual flow (Security is a phase-level concern that frames the rest of planning):

        ```rust
        push_substage(&mut lines, "Security", inf.has_security);
        push_substage(&mut lines, "Patterns", inf.has_patterns);
        // ... rest unchanged
        ```

    Do NOT add `has_security` to the Execute-section trigger (`exec_touched`) — SECURITY is a planning sub-phase. Do NOT add a new dedicated pipeline stage (no [S] in `STAGE_LABELS`); the user's framing is "optional sub-phase", not a sixth pipeline stage.
  </action>
  <verify>
    <automated>cargo check --all-targets 2>&1 | grep -v '^#' | grep -E '^(warning|error):' | wc -l | tr -d ' ' | { read n; test "$n" = "0" && echo OK || { echo FAIL=$n; exit 1; }; }</automated>
  </verify>
  <done>
    - `build_substage_lines` references `inf.has_security` in exactly two places (the `plan_touched` boolean and one `push_substage` call labelled "Security")
    - The "Security" row sits at the top of the Plan sub-stages list
    - `cargo check --all-targets` produces zero warnings and zero errors
  </done>
</task>

<task type="auto">
  <name>Task 3: Full build + test verification against real-world fixture</name>
  <files>(no source edits — verification only)</files>
  <action>
    Run the full test suite and a manual sanity check against the usbee project to confirm end-to-end detection works.

    1. `cargo nextest run` (fall back to `cargo test` if nextest is not installed). All tests must pass.

    2. `cargo clippy --all-targets -- -D warnings`. Zero clippy warnings.

    3. Sanity check the real-world fixture without launching the TUI:

        ```bash
        cargo run --release --quiet -- --help >/dev/null 2>&1 || true
        # Confirm SECURITY.md exists in the reference project
        ls ~/projects/rust/usbee/.planning/phases/01-tile-popover-hotplug-daemon-missing-state-v0-1/01-01-SECURITY.md
        ```

       Then write a one-off integration assertion (inline test in `disk_status.rs` test module is acceptable; or skip this step if the new unit tests from Task 1 already cover the suffix pattern). The unit tests cover the contract, so this real-world check is optional and only worth doing if `cargo nextest run` somehow passes without exercising the new field.

    4. If everything is green, you are done. If clippy complains about the new field being unused in some `..Default::default()` call site, that is expected (it is unused-by-name, not unused-in-scope) — no action required unless clippy actually errors.
  </action>
  <verify>
    <automated>cargo nextest run 2>/dev/null || cargo test</automated>
  </verify>
  <done>
    - Full test suite green (`cargo nextest run` or `cargo test`)
    - `cargo clippy --all-targets -- -D warnings` is clean
    - `cargo check` is clean
    - The reference SECURITY.md file at `~/projects/rust/usbee/.planning/phases/01-tile-popover-hotplug-daemon-missing-state-v0-1/01-01-SECURITY.md` matches the suffix pattern (`*-SECURITY.md`) the detector now recognizes
  </done>
</task>

</tasks>

<verification>
End-to-end check:
1. `cargo nextest run` → all tests pass, including the three new `disk_status` tests.
2. `cargo clippy --all-targets -- -D warnings` → clean.
3. Manual TUI check (optional, not gating): register the usbee project, open the detail view for phase `01-tile-popover-hotplug-daemon-missing-state-v0-1`, confirm the Plan sub-stages section shows `✓ Security` at the top.
</verification>

<success_criteria>
- `DiskInference` exposes `has_security: bool`
- The disk scanner detects both `SECURITY.md` and `*-SECURITY.md`
- The detail screen's Plan sub-stages section displays a "Security" row with ✓ when present, ○ when absent
- The Plan sub-stages section now renders even if SECURITY.md is the only planning artifact on disk
- No new abstractions introduced; the change is an exact mirror of the existing per-artifact pattern
- All existing tests still pass; zero clippy warnings; zero build warnings
- SECURITY.md does NOT alter the top-level D/R/P/E/V pipeline status
</success_criteria>

<output>
After completion, create `.planning/quick/260512-ecm-add-support-for-gsd-s-optional-secure-su/260512-ecm-SUMMARY.md` describing:
- Files touched (with line ranges for the new branches)
- New tests added
- Confirmation that the reference usbee phase now surfaces the Security row
</output>
