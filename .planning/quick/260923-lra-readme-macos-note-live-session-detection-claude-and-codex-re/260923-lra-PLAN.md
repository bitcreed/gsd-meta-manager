---
phase: quick-260923-lra
plan: 01
type: execute
wave: 1
depends_on: ["260923-lr9"]
files_modified:
  - README.md
autonomous: true
requirements: [QUICK-260923-lra]

must_haves:
  truths:
    - "A reader of README.md learns that live session detection, for both Claude and Codex, reads Linux /proc and therefore does not work on macOS, where no sessions are detected but the rest of the TUI works."
    - "A reader learns that Codex session detection exists, is best-effort, and offers no resume. Resume stays Claude-only."
    - "The headline session claims ('Claude session awareness' under 'Why GSD Meta Manager?' and the session-detection bullet under Features) point the reader at the platform caveat instead of implying detection works everywhere."
    - "Every README claim about Codex detection matches the merged 260923-lr9 code. Nothing the code does not do is documented."
  artifacts:
    - path: "README.md"
      provides: "A '### Platform support' subsection under '## Requirements' covering /proc, Linux-only detection, macOS behaviour, and best-effort Codex detection with no resume"
      contains: "### Platform support"
  key_links:
    - from: "README.md 'Why GSD Meta Manager?' and 'Features' session bullets"
      to: "README.md '### Platform support'"
      via: "in-page markdown link '](#platform-support)'"
      pattern: "\\]\\(#platform-support\\)"
---

<objective>
Document in README.md that live session detection (Claude and Codex) is Linux-only because it reads `/proc`. On macOS no running sessions are detected, but the rest of the TUI works. Also document that Codex session detection exists, is best-effort, and cannot resume a session.

Purpose: README currently advertises "Claude session awareness", auto-registration from running sessions, and tmux `Tab`-to-switch with no platform caveat. `src/session_detector.rs` builds every session from `/proc/<pid>/cwd`, `/proc/<pid>/cmdline`, `/proc/<pid>/stat` and `/proc/<pid>/fd/0`. On macOS `build_session` therefore returns `None` for every pid, so `detect_sessions()` quietly yields an empty list. Sibling item 260923-lr9 adds interactive Codex CLI detection (detection + badge + tmux switch, no resume). The README should describe what users actually get on each platform and per agent.

Output: README.md with a new `### Platform support` subsection, plus two small cross-referencing edits to existing bullets. This is a docs-only change: no `src/`, no `Cargo.toml`, no tests.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@README.md
@src/session_detector.rs

Grounded facts. These were read at planning time (base de81754) and do not depend on lr9:
- `src/session_detector.rs:29-41`: `detect_sessions()` is documented as "inspecting the Linux /proc filesystem". It runs `pgrep -x claude` and then reads `/proc/<pid>/...`. When a read fails it silently skips that pid, so on macOS the result is an empty list, not an error or crash.
- `src/ui/screens/detail.rs:2362-2378`: resume in the Sessions tab operates on `ctx.active_sessions`, which are DETECTED sessions. On macOS there is nothing to resume from that tab.
- `src/terminal_switch.rs:237`: tmux `Tab`-to-switch matches a detected session's `tty`. On macOS it has nothing to switch to.
- Auto-registration from running sessions (README Features bullet and Quick start step 4) consumes the same detection. On macOS it has nothing to register.
- Launching a NEW session (`claude_launch_args`, `src/ui/screens/detail.rs:1087`) does not use detection.
- README style: `--` for dashes, backticked commands and paths, prose wrapped at about 80 columns (existing lines run up to 84), `###` subsections under `##` sections, and `<!-- generated-by: gsd-doc-writer -->` as line 1.

Facts that belong to sibling 260923-lr9. Per the catalog, verify them in the merged tree before writing, because this plan depends on lr9:
- `SessionKind {Claude, Codex}` on `ClaudeSession`, detected via `pgrep -x codex`
- non-interactive argv and non-tty codex processes excluded
- session id from open thread-writer-locks fds
- agent-neutral badge and help text
- resume disabled for Codex
</context>

## Inferred decisions (for audit)

The human is unavailable, so these were decided from the existing artifacts.

1. **`depends_on: ["260923-lr9"]`, chosen although no file is shared.** lr9 is code-only (`src/`) and this item touches only README.md, so the ordering is semantic rather than file-driven. The README describes lr9's feature. Running after lr9 merges lets the executor ground each Codex sentence in the real code, for example whether non-interactive `codex exec` runs are excluded and whether Codex sessions also auto-register. If lr9 fails, this item should not ship a README that advertises Codex detection that does not exist. The cost is one extra wave for a single docs edit, which is negligible.
2. **Placement: a new `### Platform support` subsection under `## Requirements`, before `### Compatibility`.** The caveat is a platform requirement, and `## Requirements` is where a macOS user looks. The existing headline bullets get a short in-page link to it instead of repeating the caveat.
3. **macOS scope wording stays conservative.** Per the coordinator guidance, the note says the rest of the TUI works. It does not list macOS-specific guarantees (such as launch-in-terminal behaviour) that were not verified on macOS.
4. **Single `type="auto"` task, with no tracer and no TDD.** This is a one-file documentation edit. A tracer slice and TDD cycles add no information here, since documentation is a listed TDD exception.

<tasks>

<task type="auto">
  <name>Task 1: Add README "Platform support" note (Linux /proc, macOS, best-effort Codex, no Codex resume) and cross-link the session bullets</name>
  <files>README.md</files>
  <precondition>Sibling 260923-lr9 (Codex session detection) is merged into this tree: `grep -rq 'SessionKind' src/` succeeds. If it fails, halt and report that lr9 has not landed. Do not document Codex detection that is absent from the code.</precondition>
  <read_first>README.md (whole file, 202 lines); src/session_detector.rs, focusing on `detect_sessions`, the pid discovery, and whatever lr9 added for `SessionKind::Codex`, including its exclusion rules; the resume gating lr9 added (grep `SessionKind` under src/ui/ and src/app.rs).</read_first>
  <action>
Step 0, ground the Codex facts. In the merged tree, confirm from code: (a) Codex sessions are found via a `/proc`-based path, as Claude sessions are; (b) resume is refused or disabled for `SessionKind::Codex`; (c) tmux `Tab`-to-switch works for Codex sessions; (d) whether non-interactive codex runs are excluded from detection. Write only claims you confirmed. If one of (a) through (d) is not what the code does, drop that clause rather than document it. Record in the SUMMARY which file:line backs each Codex sentence.

Step 1, add the subsection. Insert a new `### Platform support` subsection inside `## Requirements`, after the existing two-item bullet list and before `### Compatibility`, with a blank line on each side. Write it as two short prose paragraphs in the README's voice. Use `--` for dashes and backticks for commands and paths, and wrap at about 80 columns, never exceeding 84.
- Paragraph 1 (the macOS note): live session detection, for both Claude and Codex, reads the Linux `/proc` filesystem, so it works on Linux only. On macOS no running sessions are detected. Say plainly what goes quiet: the Sessions tab stays empty, and the features built on detection (auto-registration from running sessions, tmux `Tab`-to-switch into a session, and resuming a detected session) have nothing to act on. Then say that the rest of the TUI, meaning everything read from `.planning/`, works normally. Do not promise more than that for macOS (inferred decision 3).
- Paragraph 2 (Codex): Codex session detection is best-effort. Interactive `codex` CLI sessions are detected and shown alongside Claude sessions, and you can switch to them with `Tab` under tmux, if Step 0 confirmed that. Codex sessions cannot be resumed from the TUI; resume is Claude-only. If Step 0 confirmed the exclusion, add one clause saying non-interactive runs such as `codex exec` are not shown. A short reason for "best-effort" is welcome only if grounded, for example that detection relies on Codex CLI process details that may change between releases.

Step 2, cross-link the headline claims. Keep each edit to a phrase, not a rewrite.
- 'Why GSD Meta Manager?' bullet "**Claude session awareness**" (README lines 22-24): add a short parenthetical such as "(Linux only -- see [Platform support](#platform-support))". Keep the bold label and the rest of the sentence.
- '## Features' bullet "Claude session detection (shows which projects have active Claude instances)" (README line 51): reword it to cover both agents. It should say that it shows which projects have a live Claude instance, that interactive Codex sessions are detected best-effort without resume, and that detection is Linux-only, with the same `[Platform support](#platform-support)` link. Keep it to at most three wrapped lines.

Constraints:
- Leave line 1 (`<!-- generated-by: gsd-doc-writer -->`) unchanged.
- Leave the Usage `--help` block, the key-bindings table, the "Not the same as GSD's claude-orchestration backend" section, Quick start, and Compatibility byte-identical.
- Do not touch any file other than README.md (per inferred decision 1 this plan is docs-only).
- Total diff should be about 25 or fewer added lines.
  </action>
  <verify>
    <automated>cd /home/blk/projects/rust/gsd-meta-manager && test "$(grep -c '^### Platform support$' README.md)" = 1 && awk '/^## Requirements$/{r=NR} /^### Platform support$/{p=NR} /^### Compatibility$/{c=NR} END{exit !(r && p>r && c>p)}' README.md && awk '/^### Platform support$/{f=1;next} /^#/{f=0} f' README.md > /tmp/lra-section.txt && grep -q '/proc' /tmp/lra-section.txt && grep -q 'macOS' /tmp/lra-section.txt && grep -q 'Linux' /tmp/lra-section.txt && grep -qi 'codex' /tmp/lra-section.txt && grep -qi 'claude' /tmp/lra-section.txt && grep -qi 'best-effort' /tmp/lra-section.txt && grep -qi 'resume' /tmp/lra-section.txt && ! awk 'length > 84' /tmp/lra-section.txt | grep -q . && test "$(grep -c '](#platform-support)' README.md)" -ge 2 && awk '/^## Features$/{f=1;next} /^## /{f=0} f' README.md | grep -qi 'codex' && head -1 README.md | grep -qx '<!-- generated-by: gsd-doc-writer -->' && test "$(git diff --name-only HEAD -- . ':(exclude).planning')" = "README.md" && echo LRA-OK</automated>
  </verify>
  <done>
- README.md has exactly one `### Platform support` subsection, sitting between the `## Requirements` bullet list and `### Compatibility`.
- That subsection names `/proc`, Linux, macOS, Claude and Codex, says detection does not work on macOS while the rest of the TUI does, and says Codex detection is best-effort with no resume.
- Both the "Claude session awareness" bullet and the Features session bullet link to `#platform-support`, and the Features bullet mentions Codex.
- Every Codex claim is backed by a file:line in the merged lr9 code, and the SUMMARY records those citations.
- Line 1 is unchanged, no file other than README.md is modified, and the automated check prints `LRA-OK` (run it before the task commit, since it diffs against HEAD).
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| repo docs to users | README.md is published on crates.io (`Cargo.toml` `readme = "README.md"`) and GitHub. Users rely on it for platform expectations. No code, input handling, or runtime behaviour changes. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-lra-01 | Repudiation (misleading claim) | README.md Codex sentences | low | mitigate | Step 0 requires a code citation (file:line in merged lr9) for every Codex claim, and the precondition halts if lr9 has not landed, so the README cannot advertise absent detection or a Codex resume. |
| T-lra-02 | Information disclosure | README.md | low | accept | The text is generic platform documentation. It contains no host paths, credentials, or user data, and only names the public `/proc` interface. |
| T-lra-03 | Tampering | build/runtime | low | accept | Docs-only change. README.md is not compiled or included as a doctest (no `include_str!` of README in `src/`), so the build and test surface is unchanged. |
</threat_model>

<verification>
- The Task 1 `<automated>` check prints `LRA-OK`.
- Reading the rendered subsection, a macOS user understands that sessions will not appear and why, and that everything else works. A Codex user understands that detection is best-effort and that resume is unavailable.
- `git diff --stat` shows README.md only, with about 25 or fewer added lines.
</verification>

<success_criteria>
- README.md documents Linux-only `/proc`-based live session detection for Claude and Codex, with macOS called out explicitly as not detecting sessions while the rest of the TUI works.
- README.md documents best-effort Codex session detection with no resume.
- The headline session bullets link to the caveat. No other README section and no other file changed.
</success_criteria>

<output>
Create `.planning/quick/260923-lra-readme-macos-note-live-session-detection-claude-and-codex-re/260923-lra-SUMMARY.md` when done. Include the file:line citations backing each Codex sentence, and restate the "Inferred decisions (for audit)" section from this plan.
</output>
