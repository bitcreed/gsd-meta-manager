---
quick_id: 260515-vyt
status: complete
date: 2026-05-15
---

# Add UAT.md sub-phase detection

## What changed

Mirrors the SECURITY.md pattern (260512-ecm). UAT.md is the artifact produced by
`/gsd:verify-work`; meta-manager now surfaces it in the project detail screen.

- `src/state_reader/disk_status.rs`:
  - `DiskInference.has_uat: bool`
  - Detection branch: `name == "UAT.md" || name.ends_with("-UAT.md")`
  - 3 new tests: `test_uat_md_detected`, `test_standalone_uat_md_detected`,
    `test_empty_dir_has_no_uat`
- `src/ui/screens/detail.rs`:
  - "UAT" row added at end of Execute sub-stages section
  - `has_uat` included in `exec_touched` predicate

## Verification

- `cargo build` clean
- `cargo test` — 121 passed (3 new)
- `cargo clippy` warnings present but pre-existing (browser.rs, project_creator.rs);
  no new warnings from this change

## UAT (for the user)

Manual checks recommended:
- [ ] Project with a phase containing `<phase>-UAT.md` shows `✓ UAT  done` under
  Execute sub-stages.
- [ ] Phase without UAT.md but with summaries shows `○ UAT  not run`.
- [ ] Phase with no execute artifacts at all does not render the Execute sub-stages
  header.
