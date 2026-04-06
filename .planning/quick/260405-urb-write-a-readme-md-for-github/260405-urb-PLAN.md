---
phase: quick
plan: 260405-urb
type: execute
wave: 1
depends_on: []
files_modified:
  - README.md
autonomous: true
must_haves:
  truths:
    - "README.md exists at project root"
    - "GitHub visitors understand what the project does within 30 seconds"
    - "Installation and usage instructions are accurate and actionable"
  artifacts:
    - path: "README.md"
      provides: "GitHub-facing project documentation"
      min_lines: 80
  key_links: []
---

<objective>
Write a professional README.md for the GSD Meta Manager GitHub repository (bitcreed/gsd-meta-manager).

Purpose: Give GitHub visitors a clear understanding of what the project is, how to install it, and how to use it.
Output: README.md at project root.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@./CLAUDE.md
@./Cargo.toml
@./LICENSE
</context>

<tasks>

<task type="auto">
  <name>Task 1: Write README.md</name>
  <files>README.md</files>
  <action>
Create README.md at the project root with these sections:

1. **Title and badges** — "GSD Meta Manager" as h1. No badges yet (no CI configured), but leave a comment placeholder for future badges.

2. **Tagline** — One-liner: a TUI command center for managing multiple GSD-run projects from a single interface.

3. **Screenshot placeholder** — HTML comment noting where to add a screenshot/gif later: `<!-- TODO: Add screenshot or terminal recording here -->`

4. **What it does** — 2-3 sentences expanding on the core value: see the state of every GSD project at a glance and act on any of them without leaving the TUI. Emphasize that it reads `.planning/` state directly from disk — no need to run Claude/GSD to check status. Mention it's a companion tool to the [GSD workflow system](https://github.com/anthropics/claude-code/tree/main/.claude/get-shit-done).

5. **Features** — Bulleted list covering:
   - Unified dashboard with color-coded project status
   - Live filesystem watching (auto-refresh on `.planning/` changes)
   - Vim-style navigation (`j`/`k`, `/` search, `Enter` to drill in)
   - 9-tab detail view: Phases, Roadmap (ASCII DAG visualization), Backlog, Git History, Pipeline, Queue, Sessions, Archive, Config
   - Queue management (create, edit, delete, reorder items)
   - New project creation from within the TUI
   - Claude session detection (shows which projects have active Claude instances)
   - Paused project detection (parses HANDOFF files)
   - Milestone archive browser with inline markdown rendering
   - Search and filter across projects

6. **Installation** — Two methods:
   - From source: `cargo install --path .`
   - Build manually: `cargo build --release`, binary at `target/release/gsd-meta-manager`
   - Note: Requires Rust 1.85+

7. **Usage** — Brief section:
   - Launch: `gsd-meta-manager`
   - Register a project: press `a` and enter the path to a GSD project directory
   - Key bindings table (compact): `j/k` navigate, `Enter` open detail, `Tab`/`Shift+Tab` switch tabs, `/` search, `a` add project, `d` delete project, `n` new project, `q` quit, `?` help
   - Configuration stored at `~/.config/gsd-meta-manager/config.toml`

8. **How it works** — Brief technical paragraph: reads `.planning/` directories to infer phase status, roadmap progress, and workflow state. Uses filesystem watching (notify) for live updates. Single async event loop (tokio) handles input, filesystem events, and rendering without blocking.

9. **Requirements** — Rust 1.85+ (for building). Single binary with no runtime dependencies.

10. **Contributing** — Keep brief: "Contributions welcome. Please open an issue to discuss changes before submitting a PR."

11. **License** — "MIT License — see [LICENSE](LICENSE) for details."

Style guidelines:
- Professional, concise, no fluff
- Use code formatting for commands and paths
- No emojis
- Keep total length between 100-180 lines
- Use ATX-style headers (##)
  </action>
  <verify>
    <automated>test -f README.md && wc -l README.md | awk '{if ($1 >= 80) print "OK: "$1" lines"; else print "FAIL: only "$1" lines"}'</automated>
  </verify>
  <done>README.md exists at project root with all required sections, accurate installation/usage instructions, and professional formatting suitable for GitHub.</done>
</task>

</tasks>

<verification>
- README.md exists and renders correctly as markdown
- All sections present: title, description, features, installation, usage, how it works, contributing, license
- No broken links (LICENSE file exists)
- No inaccurate claims about features or installation
</verification>

<success_criteria>
README.md at project root, 100-180 lines, covers all standard GitHub README sections with accurate project information.
</success_criteria>

<output>
After completion, create `.planning/quick/260405-urb-write-a-readme-md-for-github/260405-urb-SUMMARY.md`
</output>
