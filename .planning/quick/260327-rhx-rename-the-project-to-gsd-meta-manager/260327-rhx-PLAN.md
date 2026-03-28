---
phase: quick
plan: 260327-rhx
type: execute
wave: 1
depends_on: []
files_modified:
  - src/cli.rs
  - src/config.rs
  - src/main.rs
  - src/error.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/add_project.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/create_project.rs
  - CLAUDE.md
  - .planning/PROJECT.md
autonomous: true
requirements: []

must_haves:
  truths:
    - "Binary name is gsd-meta-manager"
    - "Config directory is ~/.config/gsd-meta-manager/"
    - "Log directory uses gsd-meta-manager"
    - "All UI titles say GSD Meta Manager"
    - "CLAUDE.md references gsd-meta-manager throughout"
  artifacts:
    - path: "src/cli.rs"
      contains: "gsd-meta-manager"
    - path: "src/config.rs"
      contains: "gsd-meta-manager"
    - path: "src/main.rs"
      contains: "gsd-meta-manager"
  key_links: []
---

<objective>
Rename the project from "gsd-manager" to "gsd-meta-manager" across all source code and documentation.

Purpose: The name "gsd-manager" conflicts with the `/gsd:manager` command. "gsd-meta-manager" clearly distinguishes this TUI tool.
Output: All references updated, project compiles and runs under new name.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@Cargo.toml
</context>

<tasks>

<task type="auto">
  <name>Task 1: Rename all source code references from gsd-manager to gsd-meta-manager</name>
  <files>src/cli.rs, src/config.rs, src/main.rs, src/error.rs, src/ui/screens/normal.rs, src/ui/screens/add_project.rs, src/ui/screens/delete_confirm.rs, src/ui/screens/create_project.rs</files>
  <action>
    Replace all occurrences of the old name with the new name in source files. Cargo.toml already has `name = "gsd-meta-manager"` so skip it.

    Specific replacements:

    1. src/cli.rs line 5: `#[command(name = "gsd-manager"` -> `#[command(name = "gsd-meta-manager"`
    2. src/config.rs line 40: `.join("gsd-manager")` -> `.join("gsd-meta-manager")`
    3. src/main.rs line 28: `d.join("gsd-manager")` -> `d.join("gsd-meta-manager")`
    4. src/main.rs line 30: `"gsd-manager.log"` -> `"gsd-meta-manager.log"`
    5. src/main.rs line 54: `gsd-manager add` -> `gsd-meta-manager add`
    6. src/error.rs line 1: `// Error types for gsd-manager.` -> `// Error types for gsd-meta-manager.`
    7. src/ui/screens/normal.rs: `.title(" GSD Manager ")` -> `.title(" GSD Meta Manager ")`
    8. src/ui/screens/add_project.rs: `.title(" GSD Manager ")` -> `.title(" GSD Meta Manager ")`
    9. src/ui/screens/delete_confirm.rs: `.title(" GSD Manager ")` -> `.title(" GSD Meta Manager ")`
    10. src/ui/screens/create_project.rs: `.title(" GSD Manager ")` -> `.title(" GSD Meta Manager ")`
  </action>
  <verify>
    <automated>cd /home/blk/projects/rust/gsd-manager && cargo build 2>&1 | tail -5 && ! grep -r "gsd-manager" src/ --include="*.rs" | grep -v "gsd-meta-manager" && echo "ALL CLEAN"</automated>
  </verify>
  <done>No source file contains "gsd-manager" (without "meta-"), project compiles successfully</done>
</task>

<task type="auto">
  <name>Task 2: Update CLAUDE.md and PROJECT.md references</name>
  <files>CLAUDE.md, .planning/PROJECT.md</files>
  <action>
    In CLAUDE.md, replace all occurrences of "gsd-manager" with "gsd-meta-manager" and "GSD Manager" with "GSD Meta Manager". Key locations:
    - Project title at top
    - toml library description: `~/.config/gsd-meta-manager/config.toml`
    - tracing-subscriber description: `~/.local/share/gsd-meta-manager/gsd-meta-manager.log`

    In .planning/PROJECT.md, replace "GSD Manager" with "GSD Meta Manager" in the project name/title. Leave historical planning documents (.planning/milestones/) unchanged since they are archived records.
  </action>
  <verify>
    <automated>cd /home/blk/projects/rust/gsd-manager && ! grep -l "gsd-manager" CLAUDE.md | grep -v "gsd-meta-manager" && grep "GSD Meta Manager" CLAUDE.md && echo "DOCS CLEAN"</automated>
  </verify>
  <done>CLAUDE.md and PROJECT.md consistently use "gsd-meta-manager" / "GSD Meta Manager"</done>
</task>

</tasks>

<verification>
- `cargo build` succeeds
- `grep -r "gsd-manager" src/ --include="*.rs"` returns only lines containing "gsd-meta-manager"
- `grep "GSD Manager" CLAUDE.md` returns nothing (all replaced with "GSD Meta Manager")
</verification>

<success_criteria>
All active source code and documentation references use "gsd-meta-manager" / "GSD Meta Manager". Historical planning artifacts in .planning/milestones/ are left unchanged. Project compiles cleanly.
</success_criteria>

<output>
After completion, create `.planning/quick/260327-rhx-rename-the-project-to-gsd-meta-manager/260327-rhx-SUMMARY.md`
</output>
