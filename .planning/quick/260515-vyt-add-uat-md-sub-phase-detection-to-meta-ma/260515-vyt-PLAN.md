---
phase: quick-260515-vyt
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/disk_status.rs
  - src/ui/screens/detail.rs
autonomous: true
requirements:
  - QUICK-260515-vyt: Detect UAT.md sub-phase artifact and surface in pipeline drill-down
---

<objective>
Add UAT.md (User Acceptance Test, produced by `/gsd:verify-work`) to the list of GSD
sub-phase artifacts that meta-manager detects and surfaces, mirroring the existing
SECURITY.md pattern (260512-ecm). UAT files live at
`.planning/phases/<phase>/<phase>-UAT.md` per the verify-work workflow.

Output:
- `DiskInference.has_uat: bool`, populated by the single-pass scan in `infer_disk_status`
- A "UAT" row in the project detail screen's sub-stages, placed at the end as the final
  human-validation step (after Code Review / UI Review)
- New tests asserting bare `UAT.md` and `*-UAT.md` are detected; `UAT.md` does not
  affect plan_count / summary_count / status
</objective>

## Tasks

1. **src/state_reader/disk_status.rs**
   - Add `pub has_uat: bool` to `DiskInference` (next to `has_security`)
   - Add `let mut has_uat = false;` alongside the other locals
   - Add detection branch before the generic PLAN/SUMMARY matchers (mirror SECURITY branch):
     ```rust
     if name == "UAT.md" || name.ends_with("-UAT.md") {
         has_uat = true;
         continue;
     }
     ```
   - Include `has_uat` in the returned `DiskInference` literal
   - Add three tests mirroring the security tests:
     - `test_uat_md_detected` — `<phase>-UAT.md`
     - `test_standalone_uat_md_detected` — bare `UAT.md`
     - `test_empty_dir_has_no_uat`

2. **src/ui/screens/detail.rs**
   - In `build_substage_lines`, treat UAT as the final verification step.
   - Render it in the Execute sub-stages section as the last row, after `UI Review`:
     ```rust
     push_substage(&mut lines, "UAT", inf.has_uat);
     ```
   - Include `inf.has_uat` in the `exec_touched` predicate so the section appears
     when only UAT exists.

3. **Verify**: `cargo build && cargo test && cargo clippy -- -D warnings`

## UAT for this quick task

After execution, manually validate:
- [ ] Point meta-manager at a project that has a `*-UAT.md` file in a phase dir; the
  detail screen Execute sub-stages section shows a green `✓ UAT  done` row.
- [ ] A phase with no UAT.md shows `○ UAT  not run` once any execute artifact exists.
- [ ] A phase with neither summaries, reviews, nor UAT does NOT render the Execute
  sub-stages section (no spurious header).
- [ ] `cargo test` passes including the three new tests.
