---
status: fixing
trigger: "Every registered project displays the wrong phase. gsd-meta-manager reads .planning/STATE.md but the frontmatter deserialization fails and the whole struct is silently discarded."
created: 2026-09-10
updated: 2026-09-10
---

# Debug: STATE.md frontmatter silently discarded

## Symptoms

- **Expected:** picsync shows `P4: Pixel over ADB` (its `STATE.md` says `current_phase: 4`).
- **Actual:** picsync shows `P3: Vertical Slice & PhotoPrism Import`. This repo shows `P6: Unknown`.
- **Errors:** none surfaced to the UI. A `tracing::warn!` goes to the log file nobody reads.
- **Timeline:** since GSD began writing `gsd_state_version` quoted; affects every project.
- **Reproduction:** register any project with a real GSD `.planning/STATE.md` and read its phase cell.

## Root cause

`StateFrontmatter` (`src/state_reader/state_md.rs`) was a plain `#[derive(Deserialize)]`
struct. `gsd_state_version` was typed `f64`; GSD writes `gsd_state_version: "1.0"` (quoted).
serde cannot coerce the quoted scalar, so the *entire* struct fails and `parse_state_md`
returns `None` — dropping `status`, `current_phase`, `current_phase_name`, `milestone` and
the progress counts together, indistinguishably from an absent STATE.md.

The quoted version field is the *symptom*. The defect is that frontmatter parsing is
all-or-nothing: **one unexpected value anywhere discards every field**, and `#[serde(default)]`
does not help — it covers an *absent* field, never a *present* field of an unexpected shape.
Every other field carried the same landmine (`current_phase_name: 4` or `status: 1` would
have been equally fatal), and every field GSD adds in future inherits it.

Downstream, `format_phase_display` fell through to `completed_phases + 1` — counts taken
from ROADMAP's `## Progress` table, which unconditionally overrides STATE.md — giving
2+1 = "P3" for picsync while the disk scanner independently inferred phase 4 correctly.

## Fix

1. **Structural field-level tolerance.** `StateFrontmatter` now has a hand-written
   `parse` that reads the frontmatter into a `serde_yml::Mapping` and extracts each field
   through lenient scalar accessors. A bad value degrades that one field to its default;
   the rest of the struct survives. Tolerance is structural — the only way to read a new
   field is through the same accessors, so a newly added field is safe by default.
2. **`gsd_state_version` is an opaque, comparable `StateVersion`.** Quoted, unquoted,
   missing, newer-than-supported and garbage all read; none is refused.
3. **Absent vs unreadable are distinct.** `FrontmatterOutcome` separates them;
   `ProjectState::state_md_unreadable` carries it to the render path, which shows
   `! STATE.md unreadable` instead of a plausible-looking wrong number.
4. **Disk frontier as the fallback.** `ProjectState::active_phase_number()` prefers the
   disk-inferred frontier over `completed_phases + 1`, used consistently at every call site.

## Verification

- `cargo build`, `cargo test --no-fail-fast`, `cargo clippy -- -D warnings`.
- picsync parses as phase 4 / "Pixel over ADB"; this repo as phase 19 / "GITSAFE".
- Fixtures now use the real GSD shape (quoted version) plus coverage for unquoted,
  unknown/newer, missing, and a garbage value in one field with the others surviving.
