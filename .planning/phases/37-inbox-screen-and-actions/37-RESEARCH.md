# Phase 37: Inbox screen and actions - Research

**Researched:** 2026-11-14
**Domain:** Rust/ratatui TUI screen + owner-items reader (Layer B) refinement + guarded subprocess actions (multiplexer launch, gsd-core acknowledge writer, clipboard)
**Confidence:** HIGH for the in-repo facts (every claim below was read from source this session), MEDIUM for the clipboard tool behavior, LOW only where tagged `[ASSUMED]`

## Summary

Phase 37 is mostly assembly: the digest map, the Owner block row formatters, the confirm popup, `multiplexer::launch`, `ExecArgv`, `run_bounded_with_stderr` and the `gsd-tools` resolver all exist and only need visibility bumps and one or two small extension points. Five findings change how the work should be planned:

1. **D-17's gate cannot use `DiskInference`.** `summary_count` counts **plans matched by a summary**, not summary files, so a phase with only SUMMARY files (D-19 fixture (b)) reads `plan_count + summary_count == 0` and would stay in Decide; archived and non-roadmap phase directories have no `DiskInference` at all. The planned-phase predicate has to be computed in Layer A's CONTEXT scan from the directory listing it already does (`context.rs` `scan_with`) and carried on `ContextFile` as a Layer B input. This is the wave-1 reader plan.
2. **Three source guards block the phase as specified unless they are edited on purpose:** `tests/owner_open_conformance.rs::no_source_file_names_the_open_audit_verb` fails on ANY non-comment `audit-open` under `src/` (the acknowledge argv needs exactly that string); `tests/focus_entry_guard.rs` pins `focus_message` callers to `detail.rs` and `normal.rs`; `tests/spawn_seam_guard.rs` pins every `Command::new(` to an allowlist (the clipboard child is a new spawn site). Each edit is a deliberate, same-commit allowlist change.
3. **A root-scoped acknowledge can land in a workstream.** gsd-tools resolves the workstream as `--ws` > `GSD_WORKSTREAM` > session pointer > shared `.planning/active-workstream` marker, and there is no CLI spelling for "root". The child must run with `GSD_WORKSTREAM` and the session-key variables removed, and `A` on a root-scoped group must be refused while the project has a non-empty `.planning/active-workstream`.
4. **The "loading forever" gap is one missing channel.** A digest never arrives only when the reader panics (both delivery paths already log and drop it) or before the first fill; the folder-gone / no-`.planning/` case is already derivable. A per-alias failure set on `AppContext`, fed by the existing `ProjectStateLoaded { digest: None }` and a new `OwnerDigestFailed` action, bounds the state with no timer.
5. **`v` needs the reader's typed command, not the string.** The reader builds `/gsd-verify-work {roadmap number}` (e.g. `ph 01` runs `/gsd-verify-work 1`), which the TUI cannot rebuild without re-deriving `phase_labels`. Add a typed `run: Option<RunCommand>` on `OwnerGroup` from which `command` is derived, so the two can never disagree and `v` re-validates the typed tokens.

**Primary recommendation:** Plan four coarse plans: (W1) reader refinement D-17/D-18 + `hidden.planned_questions` + typed `RunCommand`; (W1) shared plumbing (Owner block visibility bumps with byte-identical output, `inbox::group_checklist` extraction, digest-failure channel, bounded runner env-removal, clipboard module + base64); (W2) `InboxScreen` with navigation, filters, ordering, folds, `g`/`o`/`y`; (W3) `v`/`A` actions, `AckConfirmScreen`, Owner block D-21 keys, guard edits, oracle round trip, help and README.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

(Copied verbatim from `37-CONTEXT.md` `<decisions>`.)

### Carried forward (not re-decided)
- D1/D4/D7/D10 of the report: `❢` not widened; archived debt only in folds; acknowledge goes
  through gsd-core, never a native write; keys `i`, `v`, `A`, `y`.
- 36 D-02: the digest lives on `AppContext` keyed by alias; the inbox screen reads that map only.
  No parse on the render path (OWN-12, `async_blocking_guard`).
- 36 D-17 vocabulary: lane tags `TEST`/`DECIDE`/`FIX`/`BLOCKED`, `ph N`, `ws <name>` prefix in
  Magenta, `N checks`, status phrases (`closes phase`, `+ fix N`, `awaiting your confirmation`,
  bookkeeping-only), age `2 d` and `stale`. 36 D-19 folds (Old debt, Acknowledged, Todos count
  only). Counts mean what `inbox --json` / `status --json` `owner` mean (34 D-22, 35 D-11).
- 35 D-05/D-06: handoff and checkpoint groups use `/gsd-resume-work`; 35 D-03: debug groups use
  `/gsd-debug`.
- Every project-authored string renders through the existing `shown`/`fit_cells` helpers and is
  covered by `render_escape_guard` (OWN-11).

### Inbox screen (OWN-20)
- **D-01:** `i` on the dashboard pushes a new `InboxScreen` (replaces the interim 36 D-12 arm in
  `normal.rs` ~806); `Esc`/`q` pops back to the dashboard with the dashboard selection unchanged.
  Bound on the dashboard only **[inferred]**. The selection starts on the first group row, so
  `i`, `j`, `v` (or `i`, `v` for the top row) launches in ≤ 3 keys (SC1).
- **D-02:** Layout per report §3.3: header `Owner inbox ── N projects · test T · decide D · fix F
  phases · blocked B` with the right-aligned lane keys `[1]Test [2]Decide [3]Fix [4]Blocked
  [a]ll`; sections `TEST — closes a phase`, `TEST — phase stays open (also needs fix or unblock)`,
  `DECIDE`, `FIX — code needed`, then `BLOCKED` when shown; a `──` rule and the fold line
  (`▸ Old debt: N checks in P projects (archived milestones)`, `▸ Acknowledged (N files)`,
  `▸ Triage: N pending todos in P projects`); a key-hint footer. Header numbers are fleet totals
  over all registered projects (not the dashboard filter) and equal `inbox --json` `summary`
  **[inferred]**.
- **D-03:** Lane filter state: default = the three actionable lanes (Test, Decide, Fix), matching
  the mockup, which lists Blocked only in the header. `1`-`4` show one lane, `a` shows all four
  including Blocked; pressing the active key again returns to the default. The active filter is
  highlighted in the header (non-default states announce themselves, defaults are silent)
  **[inferred]**.
- **D-04:** Row = one group (rows are phases, not items). Columns: `▸`/`▾`, alias, `ws <name>`
  prefix + `ph N` (or `debug`/`handoff`/`checkpoint`), group name, checks count, age, `stale`
  marker; FIX rows also show the command (as in the mockup); DECIDE rows show their phrase
  (`handoff paused · 1 human action pending`, `N debug sessions awaiting your confirmation` is
  NOT aggregated: one row per debug session, as the digest carries them) **[inferred: no
  aggregation of debug rows, keeps `v`/`A` one-target]**. At 100 columns the command column is
  dropped before the name is truncated (36 D-17 narrowing order). Exact column widths and
  breakpoints come from the 37 UI-SPEC, reusing 36-UI-SPEC tokens and colors.
- **D-05:** Ordering (report §3.3, owner UAT item "ordering in the inbox"): sections in the D-02
  order; inside TEST, closes-a-phase first, then project milestone progress (the dashboard
  Progress ratio, highest first; ties: fewer `closes_after.phases_open` first), then age (oldest
  first), then alias and phase key for a deterministic order **[inferred: progress measure and age
  direction]**. DECIDE and FIX use age (oldest first), then alias/phase. One pure, unit-tested
  sort function.
- **D-06:** `Enter` toggles a group's items (test number, name, expected, why_human,
  reason/blocked_by, merged gaps, source file), same content as the Owner block expansion
  (36 D-18), reusing its item formatter; expansion state lives in memory only. `j/k`, `↑/↓`,
  `PgUp/PgDn`, `g`-free `Home/End` move; folds expand with `Enter` like in the Owner block.
- **D-07:** `g` opens the project's detail view on the Phases tab with the Owner block focused on
  the same group when it is visible there (else the tab top) **[inferred]**, through the existing
  `open_project_view` path; `Esc` from detail returns to the inbox (normal stack pop).
- **D-08:** `o` opens the group's source file **read-only inside the app**: the detail view's
  Docs > Files viewer at that file (deep link), or, if deep-linking costs more than it is worth, a
  pushed read-only viewer reusing `browser::read_md_file` and the Docs renderer **[inferred:
  planner picks]**. Never `$EDITOR` (non-intrusive; the user can still press the existing edit
  key from the Docs view). Root signals open their file (`debug/<slug>.md`, `HANDOFF.json`,
  `.continue-here.md`).
- **D-09:** `y` copies the selected group as the plain checklist text `inbox` already prints for
  that group (one shared formatter with `src/inbox.rs`, so the two can never disagree). On a fold
  row `y` copies the fold's list (Old debt) or is refused with a status message (Acknowledged,
  Todos) **[inferred]**. Clipboard: a native tool when one is present (`wl-copy` under
  `WAYLAND_DISPLAY`, `xclip`/`xsel` under `DISPLAY`, `pbcopy` on macOS, `clip.exe` on Windows),
  fed through stdin by a bounded child off the render thread; otherwise an OSC 52 escape written
  through the terminal backend **[inferred]**. The status line says which path was used and how
  many lines were copied. No new crate unless base64 for OSC 52 cannot be a few lines.
- **D-10:** Live refresh: the screen re-reads the digest map on every render, so a
  `ProjectStateLoaded` (watcher-driven reparse, 36 D-01/D-04) makes a passed or acknowledged row
  disappear without a restart (SC3). Selection is pinned to the group identity (alias, scope,
  workstream, phase/slug, kind) and falls to the next row when its group vanishes
  **[inferred]**.

### Run command (OWN-21)
- **D-11:** `v` builds the command from a typed value, never from the reader's `command` string:
  `{verb, phase_token, workstream}` with verbs per R10 (Test → `/gsd-verify-work N`, Fix from UAT
  or `gaps_found` → `/gsd-plan-phase N --gaps`, debug → `/gsd-debug`, handoff and checkpoint →
  `/gsd-resume-work`), `--ws <name>` appended for workstream groups. `phase_token` and the
  workstream name are re-validated with the reader's predicates (`is_command_token`,
  `is_workstream_name`, `owner_items/rules.rs` ~100-135) at launch; a refusal is a status
  message, nothing starts. BLOCKED groups and groups with unreadable files have no command and
  `v` says so. The phase token is passed as the reader carries it (`/gsd-verify-work 01`, as
  `rules.rs` already emits) **[inferred]**.
- **D-12:** Launch: when `ctx.active_sessions` has a live session whose `working_dir` is the
  project path, `v` focuses it (`multiplexer::focus_message`, the dashboard `Tab` path) and the
  status line carries the command to type there (report §3.4); nothing is injected into the
  session. Otherwise `multiplexer::launch(&resolve_launch_plan(), &project.path, argv, gui_argv,
  success)` with `argv = ExecArgv::new(["env", "claude", "<command>"])` (the `mux_launch_argv`
  shape: leading `env`, command as ONE argv element, project path as cwd, never a `cd` string)
  and a matching GUI-terminal argv builder next to `launch_terminal_argv` (pinned like it by a
  test that no untrusted element enters an interpreter string) **[inferred: argv shape]**.
- **D-13:** Tests: argv builder table test (each verb, with and without `--ws`, refusals for a bad
  token, a bad workstream name, blocked and unreadable groups); a `launch_with` test with fake
  backends proving the cwd and argv; the focus branch with a fake live session. Real
  tmux/herdr launch is human UAT.

### Acknowledge (OWN-22) — resolves the milestone's open question
- **D-14:** **CLI spelling, verified against gsd-core 1.21.0 and on a throwaway project:** the
  verb is `audit-open acknowledge`, not `audit acknowledge` (routed by `routeAuditOpen`,
  `audit-command-router.cjs` ~60-100; handler `cmdAuditAcknowledge`, `audit.cjs` ~1447). argv:
  `node <gsd-tools.cjs> audit-open acknowledge --category <cat> --milestone <version> <target>
  [--ws <name>]`, where `<target>` is
  - `uat_gaps` (a `*-UAT.md` / `*-HUMAN-UAT.md` group, any lane) and `verification_gaps` (a
    `human_needed` or `gaps_found` `*-VERIFICATION.md` group) and `context_questions` (a
    `*-CONTEXT.md` group): `--phase <dir token> --file <basename>`, plus
    `--archived-milestone <v>` for an Old-debt group;
  - `debug_sessions`: `--slug <file stem>`.
  `--phase` must be the phase **directory token exactly** (`03`, `18.1`): `3` fails with "no phase
  directory found" (probed). `--ws <name>` and `GSD_WORKSTREAM` both resolve workstream phases
  (probed). The `query audit-open acknowledge …` spelling the complete-milestone workflow uses
  routes to the same handler; the app uses the shorter form **[inferred]**. Handoff, checkpoint,
  todos and BLOCKED-only aggregates have no category: `A` is refused with a status message.
  ROADMAP SC3 and OWN-22 are corrected to this spelling.
- **D-15:** How it runs: **a bounded direct child** (`launcher::process::run_bounded` shape, cwd =
  project path, ~10 s budget) on `spawn_blocking`, reporting through an `Action`, **not** a
  multiplexer window **[inferred: narrows report §3.4 "same launch path"]**. Reason: the writer
  is a sub-second non-interactive command whose refusals (`no phase directory`, `file not found`,
  ambiguous) must reach the owner; a multiplexer window would swallow the exit status. Still
  gsd-core's own writer (D7 holds: the app writes nothing). `node` from `PATH`; `gsd-tools.cjs`
  from `gsd_install::install_candidates` (project-local first, the same order gsd-core uses); none
  found → refusal message naming the install command. stdout/stderr are capped and rendered
  through `render_for_terminal` in the status line. `--milestone` = the project's (or the
  workstream's) current milestone version from `ProjectState`; when it is unknown (mailbot today:
  `version: null`) `A` is refused rather than guessed **[inferred]**.
- **D-16:** Confirm popup (reuse `confirm_popup.rs`, default No): names the project, the file, the
  category, that the marker is **file-level** (hides all N items of that file in every lane, e.g.
  its blocked rows too), that it lapses by itself when the file changes, that gsd-core edits the
  file's frontmatter and **does not commit** (the project's working tree gets one modified file),
  and the exact argv. After success the row disappears on the next watcher refresh (D-10); the
  status line says "Acknowledged <file> via gsd-core (uncommitted)". Tests: argv builder table
  (each category, archived, workstream, refusals), and an oracle-backed round trip that runs the
  real `gsd-tools audit-open acknowledge` on a temp fixture and asserts the reader then hides the
  group, gated like the OWN-02 oracle (fails when the oracle is missing unless
  `ALLOW_MISSING_ORACLE`), so it runs in `pre-tag-check.sh --container`.

### Decide-lane noise (live-fleet finding, read-only) — reader refinement
Finding on the live fleet (worktree build, `inbox --json --lane decide`, 2026-11-14): fleet
`decide` = 46. **mailbot 19** = all `context_questions`: 11 from `03-CONTEXT.md`, 8 from
`04-CONTEXT.md`. Phase 03 has 25+ executed plans; phase 04 has 12 plans and is executing; the
04 questions are self-labeled "`[OPEN]`, HUMAN, non-blocking" with defaults. **sentriq 17** = 11
from `16-CONTEXT.md` (a phase with plans and 5 summaries, questions addressed to "the planner
subagent", e.g. "Planner picks and justifies") plus 5 debug sessions awaiting confirmation plus 1
paused workstream handoff. The question counts are also inflated: Layer A counts **lines** (bullet
lines and any line containing `?`), as gsd-core's `deriveOpenQuestions` does, so wrapped
continuation lines, an intro sentence and a `---` rule each count (mailbot 03: 11 lines for 6
real questions, of which one "MOVED TO PHASE 4" and three "PARTIALLY RESOLVED"; sentriq 16: 11
lines for 6 numbered questions). Verdict: **noise**. A question in the CONTEXT of a phase that
has already been planned was either answered, decided `[inferred]` by the planner, or deferred;
the owner cannot act on it from the inbox. The debug sessions and the handoff are real signal.
- **D-17:** **Rule (Layer B, `owner_items/rules.rs`):** a `context_questions` group is owner work
  only while its phase has **no plan artifacts yet**: `DiskStatus.plan_count + summary_count ==
  0` for that phase directory (sentriq 16 has only one PLAN file next to five SUMMARY files, so
  the gate counts both) **[inferred]**. Once a phase is planned, its CONTEXT questions are
  dropped from the lanes, from `decide`, from `closes_after` and from Old debt, and counted in a
  new additive `hidden.planned_questions` (per project and in `summary`; schema stays v1:
  additive keys are allowed) **[inferred: key name]**. An acknowledged file stays counted under
  `acknowledged_files` as today. Layer A is untouched, so the `audit-uat`/`audit-open` oracle
  comparison keeps passing. Expected effect on today's fleet: mailbot decide 19 → 0, sentriq 17
  → 6, fleet 46 → 16.
- **D-18:** **Unit (Layer B):** a remaining (not-yet-planned) CONTEXT question group counts one
  item per **top-level question**: a top-level bullet or numbered entry at the section's base
  indent, with its wrapped continuation lines folded into it; when the section has no list
  entries at all, the `?` lines are the items (the oracle's fallback); horizontal rules and empty
  lines never count. The D11 RESOLVED/ANSWERED/CLOSED filter applies to the folded entry, and is
  extended to match the marker after a leading ID token (`**OQ-02 — RESOLVED by H-02`), which the
  35 D-04 matcher misses **[inferred]**. Item text is the entry's first line, capped as today.
- **D-19:** Tests: fixtures under `tests/fixtures/owner/` modeled on the three live shapes with
  synthetic text: (a) a planned phase (PLAN + SUMMARY files) whose CONTEXT has open questions →
  zero Decide groups, `hidden.planned_questions == N`; (b) a phase with only SUMMARY files → same;
  (c) a discussed-but-unplanned phase whose `<open_questions>` section has 4 entries with wrapped
  lines, an intro sentence, a `---` rule and one `**OQ-n — RESOLVED by …**` entry → exactly 3
  items; (d) the oracle conformance test unchanged and green (Layer A untouched); (e) `inbox
  --json` key-path test pins `hidden.planned_questions`; the CLI golden files and README schema
  are updated.

### Help, README (OWN-23)
- **D-20:** Help overlay: a dashboard entry for `i`, and an Inbox section listing every key of the
  screen (`1-4`/`a`, `j/k`, `Enter`, `g`, `o`, `v`, `A`, `y`, `Esc`) and what `A` writes (via
  gsd-core, file-level, uncommitted). README "Owner inbox" section (added in 36) gains the screen,
  the ordering rule, the three actions, the exact acknowledge argv, the clipboard paths, and the
  `hidden.planned_questions` key in the `inbox` schema.
- **D-21:** The Owner block on the detail Phases tab also binds `v`, `o`, `A`, `y` on its group
  rows through the same action functions (36 deferred "launching the row command from the detail
  view" to this phase), **without any change to its rendered layout or footer**, so the pending
  36 UAT layout stays as approved; the help overlay documents the keys **[inferred]**. If a key
  collides with an existing Phases-tab binding the planner drops that key from the block rather
  than moving the existing one.

### Tests and UAT
- **D-22:** Buffer snapshots at 100x30 (SC2 legibility), 120x40 and 80x24 (degrades without
  panic, name truncation last) over a synthetic fleet modeled on the §3.3 mockup; filter states
  (default, each lane, `a`); expanded group; folds; empty inbox (`No owner work` line, dim)
  **[inferred]**. `render_escape_guard` extended to the inbox screen and the confirm popup.
  Ordering unit test (D-05). SC1 key-sequence test: dashboard `i`, `j`, `v` yields a
  `launch_with` call with the expected cwd and argv. SC3: `FileChanged` + reparse with a passed
  UAT and with an acknowledged file → row gone, selection moves (D-10).
- **D-23:** Human UAT kept to the three ROADMAP checks: ordering in the inbox; launch into the real
  multiplexer (tmux and herdr); acknowledge round trip on a throwaway project.

### Musk pass
- Milestone pass already done (`pass_due: false`). Phase pass recorded, no cuts: OWN-20..23 trace
  to the report adopted at the owner's request, and D-17/D-18 are the owner-requested
  investigation's outcome **[inferred]**. Lean lane not used: several subsystems (new screen,
  launch, subprocess, clipboard, reader rule, help, README).

### Claude's Discretion
- Column widths and breakpoints (via the 37 UI-SPEC, consistent with 36-UI-SPEC).
- `o` deep link into Docs vs a pushed read-only viewer (D-08).
- Module split (e.g. `src/ui/screens/inbox.rs`, a shared `owner_actions.rs` used by the inbox and
  the Owner block, the clipboard helper in its own module).
- Number and grouping of plans (coarse granularity).

### Deferred Ideas (OUT OF SCOPE)
- Injecting the command into an already-running session (today `v` focuses and shows the
  command); would need the driver's steering path.
- Aggregating several debug sessions of one project into one DECIDE row (mockup's "5 debug
  sessions awaiting your confirmation") with a per-session sub-list; D-04 keeps one row each.
- Upstream gsd-core issue to add: `deriveOpenQuestions` counts lines, not questions, and ignores
  whether the phase was planned (extends report §8 item 8).
- Report §6 deferred list (OWN-F01..F08).

(Reviewed Todos: the 17 keyword-only matches are not folded; see CONTEXT `<deferred>`.)
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| OWN-20 | Inbox screen (`i` on the dashboard) per report §3.3: lane filters `1-4`/`a`, ordering rule, expand, `g` go to project, `o` open file, `y` copy checklist. Legible at 100x30. | Reuse map (Owner block formatters, `inbox::group_checklist`), screen state design, `g` selection pitfall, `o` viewer recommendation, clipboard module, digest-failure header clause, snapshot conventions |
| OWN-21 | `v` runs the group's command through `multiplexer::launch` (focusing an existing live session instead); argv built only from validated phase token and workstream name via `ExecArgv::new`. | Typed `RunCommand` on `OwnerGroup`, argv builders beside `mux_launch_argv`, `focus_entry_guard` edit, testable split (pure plan + `launch_with`) |
| OWN-22 | `A` launches `gsd-tools audit-open acknowledge …` after a confirm popup; app writes nothing; row disappears on next watcher refresh. | Verified handler flags, `resolve_gsd_tools_from` reuse, `run_bounded_with_stderr`, workstream-pointer pitfall and env removal, `no_source_file_names_the_open_audit_verb` amendment, `AckConfirmScreen`, oracle round trip |
| OWN-23 | Help overlay and README ("Owner inbox" section plus the `inbox` schema) updated. | Help row helpers and uniqueness test, README tests (`the_readme_inbox_fence_has_exactly_the_schema_v1_key_paths`, `the_readme_inbox_table_documents_every_path`), `hidden.planned_questions` rows, `agent_skill.md` sentence |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **State reading:** must not require running Claude/GSD to check status; read files or cached state. The inbox renders from `ctx.owner_digests` only.
- **Non-intrusive:** must not interfere with running GSD instances. `v` never injects into a live session; `A` runs gsd-core's own writer and the app writes nothing.
- **Portability:** any GSD user, not one setup. Clipboard tool choice by environment, `CLAUDE_CONFIG_DIR`/`CODEX_HOME` respected by the existing resolver.
- **GSD workflow enforcement:** edits only through `/gsd:execute-phase`.
- **Release process:** cross-target clippy (`x86_64-pc-windows-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`) is a tag gate; a Windows-only dead-code lint killed v2.2.0 and v2.3.0 publishes. Any `#[cfg(target_os)]` code in the clipboard module must be lint-clean on all three.
- **User global (`~/.claude/CLAUDE.md`):** US spelling; respect env vars over hardcoded paths (`PATH`, `CLAUDE_CONFIG_DIR`, `XDG_*`); path-scoped commits; no singular "they" in copy.
- **No `AGENTS.md`** exists in this worktree (checked); no `.claude/skills` or `.agents/skills`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Planned-phase gate, top-level question units (D-17/D-18) | Reader Layer A scan (`owner_items/context.rs`: listing + parse, cached) | Reader Layer B (`rules.rs` `context_groups`: drop and count) | The directory listing and the section text are read once in Layer A; the owner's rule stays in Layer B, oracle untouched |
| `hidden.planned_questions` JSON | CLI serializer (`src/inbox.rs`) | README + `tests/inbox_cli.rs` schema pins | Additive schema v1 key |
| Typed run command | Reader Layer B (`rules.rs`) | TUI action builder (re-validates) | Single source for `command` string and `v` argv |
| Digest delivery and failure state | App event loop (`app.rs` `update`, `AppContext`) | Blocking task (`schedule_reparse`, `schedule_owner_digest`) | Results arrive as `Action`s; render reads only memory |
| Inbox rows, ordering, filters, folds | UI screen (pure functions over `AppContext`) | Owner block formatters (shared) | No I/O on the render path (OWN-12) |
| `v` launch / focus | UI action fn -> multiplexer seam (`multiplexer::launch`, `focus_message`) | GUI terminal fallback | Existing CWE-88-reviewed seam |
| `A` acknowledge | Blocking task (`spawn_blocking`) running gsd-core child via `launcher::process` | `Action::OwnerAcknowledged` -> status line; watcher reparse hides the row | gsd-core is the only writer (D7) |
| `y` clipboard | Blocking task (native tool child) | Main loop writes OSC 52 after draw | Terminal is owned by ratatui; writes happen outside `render` |
| `o` file view | UI screen (read on key press, bounded) | Docs markdown renderer (`archive::render_markdown_lines`) | Read-only, escaped |

## Standard Stack

No new dependencies. Everything is in-tree.

### Core (existing, verified in `Cargo.toml` this session)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30 (`features = ["crossterm", "unstable-rendered-line-info"]`) | Screen rendering, `TestBackend` snapshots | Project stack [VERIFIED: Cargo.toml] |
| crossterm | 0.29 | Key events; `execute!` for the OSC 52 write in the main loop | Project stack [VERIFIED: Cargo.toml] |
| tokio | 1 (`full`) | `spawn_blocking` for the acknowledge and clipboard children | Project stack [VERIFIED: Cargo.toml] |
| regex | 1 | Existing `QUESTION_LABEL` matcher (D-18 extension) | Already used in `rules.rs` [VERIFIED: rules.rs:748-751] |

### In-tree modules to reuse (not hand-roll)
| Module / fn | Location | Use |
|-------------|----------|-----|
| `multiplexer::launch`, `launch_with`, `ExecArgv::new`, `focus_message` | `src/multiplexer/mod.rs:1017, 967, 254, 1221` | `v` |
| `resolve_launch_plan`, `terminal_program_separator`, `mux_launch_argv` | `src/ui/screens/detail.rs:2567, 2611, 2836` (private) | `v` argv builders (bump to `pub(crate)`) |
| `run_bounded_with_stderr` | `src/launcher/process.rs:444` | `A` child (stdout + last stderr line, temp-file stderr) |
| `resolve_gsd_tools_from`, `GsdToolsCmd` | `src/state_reader/queue_md.rs:153, 18` (private) | Locate `gsd-tools` exactly like the queue's smart-entry does (projection of `install_candidates`, plus PATH fallback) |
| `render_confirm_popup`, `ConfirmPopup`, `dim_background`, `click_as_key` | `src/ui/screens/confirm_popup.rs:233,70,163,148` | `A` popup |
| `archive::render_markdown_lines`, `archive::line_number_lines` | `src/archive.rs:525, 587` | `o` viewer (escapes via `render_for_terminal`, line 530) |
| `owner::owner_totals`, `owner_numbers`, `OWNER_SEP` | `src/ui/screens/owner.rs:216, 54, 27` | Header totals |
| `inbox::detail`, `plural`, `blocked_tally`, `group_line` (private) | `src/inbox.rs:290, 255, 260, 383` | Row phrases, `y` text |

**Installation:** none.

## Package Legitimacy Audit

No external package is installed by this phase. OSC 52 base64 is a ~20-line hand-rolled RFC 4648 encoder (D-09 allows it; `base64 0.22.1` is only a transitive entry in `Cargo.lock:163-164`, not a direct dependency, and must not be added).

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| (none) | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```
 watcher FileChanged ──► App::schedule_reparse ──spawn_blocking──► parse_project_state
                                                          └──────► read_owner_digest (catch_unwind)
                                                                       │ Some / None(panic)
 startup / register ──► AppContext::schedule_owner_digest ─spawn_blocking─┤
                                                                       ▼
                     Action::ProjectStateLoaded{digest} / OwnerDigestLoaded / OwnerDigestFailed (new)
                                                                       ▼
                         AppContext.owner_digests (map)  +  owner_digest_failed (new set)
                                                                       ▼
 key `i` on dashboard ──► InboxScreen.render(ctx)  (pure: rows, sort, filter, folds, header ladder)
                                │ keys
        ┌───────────┬───────────┼─────────────┬───────────────┬──────────────┐
        ▼           ▼           ▼             ▼               ▼              ▼
     Enter/j/k     `g`         `o`           `v`             `A`            `y`
     view state   push Detail  push viewer   run_plan()      ack_request()  checklist text
     (identity)   (Phases,     (bounded      ├─live session─► focus_message  ├─validate─► push AckConfirmScreen
                   Owner focus) read)        └─else─► multiplexer::launch     │   Yes ─► spawn_blocking: node gsd-tools
                                                      (ExecArgv env claude    │          audit-open acknowledge … (cwd=project,
                                                       "/gsd-…", cwd=project) │          env scrubbed, 10 s)
                                                                              │   ─► Action::OwnerAcknowledged ─► status line
                                                                              │   gsd-core writes frontmatter ─► watcher ─► reparse ─► row gone
                                                                              └─ native tool: spawn_blocking child (stdin) ─► Action ─► status
                                                                                 else ScreenAction ─► App.pending_osc52 ─► main loop writes ESC]52
```

### Recommended module split

```
src/state_reader/owner_items/
├── context.rs          # + phase_planned (listing), + top-level entries parse (cached with ParsedContext)
├── rules.rs            # context_groups: planned gate + entry units; RunCommand; pub(crate) validators
└── mod.rs              # ContextFile.{phase_planned, entries}; Hidden.planned_questions; OwnerGroup.run; RunCommand type
src/inbox.rs            # + hidden/summary planned_questions; + pub(crate) group_checklist (push_section calls it)
src/clipboard.rs        # NEW: tool choice (pure), bounded stdin child, base64 + OSC 52 sequence (spawn allowlisted)
src/ui/screens/
├── inbox.rs            # NEW InboxScreen + pure model (rows, sort_groups, grid_for_inbox, header/fold/footer ladders)
├── owner_actions.rs    # NEW shared by inbox + Owner block: run_plan/launch, ack_request/ack argv, copy text, open target
├── ack_confirm.rs      # NEW AckConfirmScreen (popup over Inbox or Detail background)
├── owner_file_view.rs  # NEW read-only viewer for `o` (recommended over the Docs deep link, see below)
├── owner_block.rs      # visibility bumps only; expansion_lines gains an indent shift (0 = identical)
└── detail.rs           # D-21 keys in handle_owner_block_key; owner argv builders beside mux_launch_argv
```

### Pattern 1: Planned-phase gate in Layer A's listing (D-17, corrected)

**Why not `DiskInference`:** verbatim from `src/state_reader/disk_status.rs:1339-1342` and `:1374`:

```
    // The count is of PLANS matched, not of summary files, so two summaries
    // resolving to the same plan cannot push `summary_count` past `plan_count`
    // and fake a completion.
    let plan_count: u32 = plan_ids.len() as u32;
...
    let summary_count: u32 = matched_plans.len() as u32;
```
[VERIFIED: src/state_reader/disk_status.rs:1339-1342,1374]. A SUMMARY-only phase therefore has `plan_count 0, summary_count 0`; D-19 fixture (b) would fail. `phase_disk_statuses` is keyed by roadmap phase (`disk_status_for`, `src/state_reader/mod.rs:281-286`), so archived dirs and dirs absent from the roadmap (the `09-gone` case in `context.rs` tests) have no inference. Live check (read-only): sentriq `16-raw-can-button-replay-poc/` holds `PLAN.md` plus `16-01..05-SUMMARY.md`; it gates either way, but only by luck of the standalone `PLAN.md`.

**What to do:** in `context.rs` `scan_with`, the loop already calls `std::fs::read_dir(&target.path)` and filters names (`context.rs:264-271`). Collect all file names once, compute

```rust
// Layer B input, never compared with the oracle (like DebugSession.fm_unparseable).
fn is_plan_artifact(name: &str) -> bool {
    name == "PLAN.md" || name.ends_with("-PLAN.md")          // -PLAN-CHECK.md / -PLAN-REVIEW.md do not end in -PLAN.md
        || name == "SUMMARY.md" || name.ends_with("-SUMMARY.md")
}
```
and set `ContextFile.phase_planned` (new `pub phase_planned: bool`). Only regular files count (skip directories via `file_type`). Presence, not pairing: any plan or summary means the phase was planned.

In `rules.rs` `context_groups` (`rules.rs:793-859`), after the existing `in_current_milestone` skip and after computing `items`:

```rust
// D-17: questions of a planned phase are not owner work. An acknowledged file keeps today's
// path (flagged group, acknowledged_files += 1); an unreadable one stays a Decide group (fail-safe).
if file.phase_planned && !file.acknowledged && file.unreadable.is_none() {
    digest.hidden.planned_questions += items.len();
    continue;
}
```
Because the group is never pushed, `decide` (counting at `rules.rs:1088-1098`), `closes_after` (`gate::closes_after` over `digest.groups`, `rules.rs:1081`) and `archived.decide` (Old debt) drop automatically. [inferred: count both current and archived dropped questions in `planned_questions`; acknowledged wins over planned; unreadable stays listed.]

### Pattern 2: Top-level question units (D-18)

Parse in Layer A next to `derive_open_questions` (so it is cached in `CONTEXT_CACHE` with `ParsedContext`), store as `ParsedContext.entries: Vec<String>` / `ContextFile.entries: Vec<String>`; Layer A's `questions` (digest input, oracle count) stays byte-identical.

Algorithm (frontmatter `open_questions` list wins exactly as in Layer A: each entry is one question):
1. Same section as Layer A: first level-2 heading starting `open questions` (`context.rs:156-169`); `Section.body` keeps indentation (`markdown.rs:284-291`, only trailing whitespace trimmed).
2. A **list line** is `^\s*([-*+]\s|\d+[.)]\s)`. Base indent = the smallest indent of any list line.
3. A list line at base indent opens a new entry. A non-blank line that is a list line deeper than base, or an indented continuation, or a lazy continuation directly after an entry line (no blank in between) is folded into the open entry.
4. Blank lines never count. A horizontal rule (`---`, `***`, `___`, optional spaces), an ATX heading, or a non-list line at indent <= base that follows a blank line **closes** the open entry and is dropped (this drops intro paragraphs, `</open_questions>` tags and trailing prose).
5. No list line in the section: each line containing `?` (excluding rules) is one entry (the oracle fallback).
6. `is_answered(&folded_entry)` filters; item `name` = the entry's first line, sanitized with `sanitize_for_display`, capped as today; item id = `{basename}#q{entry position}` (1-based over all entries, answered included, so ids stay stable when one is marked answered).

Live shapes this must handle (read-only look this session):
- mailbot `03-CONTEXT.md`: `<open_questions>` wrapper around `## Open Questions`, a bold intro paragraph, nine `- **OQ-0n — …` entries with 2-space continuations, a nested `  - **GD-07**` sub-list inside OQ-09, then `</open_questions>`. Expected: 9 entries; OQ-02/03/04 `RESOLVED` drop; OQ-07/09 `PARTIALLY RESOLVED` and OQ-08 `MOVED TO PHASE 4` stay open -> 6.
- sentriq `16-CONTEXT.md`: heading `## Open questions (do not resolve here — carry into /gsd-discuss-phase)`, six `1.`..`6.` entries with 3-space continuations -> 6.

**D-18 matcher extension, refined:** the CONTEXT example `- **OQ-02 — RESOLVED by H-02 (2026-08-27).** text` is **already answered** today; it is the first positive case of `is_answered_reads_explicit_markers_only` [VERIFIED: rules.rs:2043-2046]. What the matcher misses is an emphasis closer between the ID and the separator, e.g. `- **OQ-02** — RESOLVED by H-02`: `QUESTION_LABEL` is `^[A-Za-z]{1,6}[-.]?[0-9][A-Za-z0-9.-]*(?:[ \t]*[—–:][—–:\s-]*|[ \t]+-+\s+)` [VERIFIED: rules.rs:748-751], so after `**` is stripped, `OQ-02** — …` does not match (the `*` sits before the dash). Extend the label with an optional closer `(?:\*\*|__|\*|_|`)?` before the separator, and add `- **OQ-02** — RESOLVED by H-02` and `1. **Q3** – CLOSED` to the positive table and `- **OQ-04** — [OPEN] keep?` to the negative one.

**Existing test that changes:** `tests/owner_digest.rs::context_questions_fill_decide` asserts 3 items with ids `["02-CONTEXT.md#q4", "02-CONTEXT.md#q5", "02-CONTEXT.md#q6"]` and `decide == 6` [VERIFIED: tests/owner_digest.rs:524-598]. Under D-18 the fixture `02-alpha/02-CONTEXT.md` has 5 base-level entries; the bold line `**Two of the four questions were answered in review.**` directly follows `- Is the archive view in scope?` with no blank and folds into it as a lazy continuation (Layer A counted it as its own line). Open entries become #4 and #5 -> 2 items, `decide` 6 -> 5, and the text line becomes `2 open questions in 02-CONTEXT.md`. The planner updates these three assertions in the same plan. No owner golden file (`tests/fixtures/owner_golden/*.txt`) uses a fixture with CONTEXT questions (`golden_entries`, `tests/owner_digest.rs:344-356`), so the goldens do not change.

### Pattern 3: `hidden.planned_questions` (schema v1, additive)

Touch list (all verified this session):
- `src/state_reader/owner_items/mod.rs:491-501` `Hidden` (derives `Copy, Default`): add `pub planned_questions: usize`. Full struct literals that must add the field: `src/ui/screens/owner_block.rs:1536` (test sample; other `Hidden {` sites use `..Default::default()`).
- `src/inbox.rs:159-166` (error entry) and `:182-189` (read entry) `hidden` objects; `:236-239` `summary` (add `"planned_questions": sum(|d| d.hidden.planned_questions)`). Unit test `an_error_entry_has_the_full_key_set_with_empty_data` (`inbox.rs:929`).
- `tests/inbox_cli.rs`: the `summary` key set (`:217-232`), `hidden_keys` (`:245-252`), the error-entry `hidden` literal (`:315-324`), and `INBOX_PATHS` (`:633-712`): add `"summary.planned_questions"` and `"projects[].hidden.planned_questions"`.
- `README.md`: the schema fence (`:606-668`, summary line `:612` and both `hidden` lines `:656`, `:666`) and the table rows (`:691` neighborhood for `summary.*`, `:748-752` for `hidden.*`); `the_readme_inbox_fence_has_exactly_the_schema_v1_key_paths` and `the_readme_inbox_table_documents_every_path` (`tests/inbox_cli.rs:831-845`) enforce both.
- `src/agent_skill.md:71`: "open CONTEXT questions" -> "open CONTEXT questions of phases not yet planned".
- Keep `render_text` unchanged (no fold for planned questions; UI-SPEC adds none), so the text goldens stay byte-identical.

### Pattern 4: Typed run command on `OwnerGroup` (D-11)

Today the reader builds strings (`rules.rs:113-135`):

```
    match lane {
        Lane::Test => Some(format!("/gsd-verify-work {phase_number}{ws}")),
        Lane::Fix => Some(format!("/gsd-plan-phase {phase_number} --gaps{ws}")),
        Lane::Decide | Lane::Blocked | Lane::Deferred => None,
    }
```
[VERIFIED: src/state_reader/owner_items/rules.rs:130-134], where `phase_number` is the roadmap number, not the directory token: the golden shows `ws  ph 01  Root  1 check  307 d stale  /gsd-verify-work 1` [VERIFIED: tests/fixtures/owner_golden/inbox.txt]. Plus `DEBUG_COMMAND = "/gsd-debug"` (`rules.rs:609`) and `resume_command` (`rules.rs:871-877`).

Recommended shape [inferred]:

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunVerb { VerifyWork, PlanPhaseGaps, Debug, ResumeWork }
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunCommand { pub verb: RunVerb, pub phase: Option<String>, pub workstream: Option<String> }
impl std::fmt::Display for RunCommand { /* "/gsd-verify-work {phase}[ --ws {ws}]" etc. */ }
// OwnerGroup gains `pub run: Option<RunCommand>`; every builder sets
// `command: run.as_ref().map(ToString::to_string)` so the JSON `command` string is unchanged.
```
`OwnerGroup` has no `Default`; 15 struct literals set fields explicitly (`src/inbox.rs` 2, `gate.rs` 2, `mod.rs` 1, `rules.rs` 6, `owner_block.rs` 1, `owner.rs` 2, `render_escape_guard.rs` 1). Add a reader test asserting `group.command == group.run.map(to_string)` over every fixture under `tests/fixtures/owner/`. Make `is_command_token` (`rules.rs:74`, today `pub(super)`) and `is_workstream_name` (`rules.rs:100`, private) reachable as `pub(crate)` through `owner_items` so `owner_actions` re-validates the typed tokens at launch (D-11).

### Pattern 5: Bounded digest state (coordinator ask 2)

Where a digest never arrives today:

| Cause | Today | Evidence |
|-------|-------|----------|
| Reader panic in the reparse task | `digest: None` -> old digest removed, nothing recorded | `app.rs:2993-3000`: `_ => self.ctx.owner_digests.remove(&alias).is_some(),` |
| Reader panic in the first fill | logs, sends nothing | `ui/screens/mod.rs:1913-1924`: "A reader panic logs and sends nothing" |
| Folder or `.planning/` gone | no digest by design; `owner_state_unavailable(alias)` already true | `ui/screens/mod.rs:1985-2001` |
| No stored `ProjectState` / no event channel | silent return (startup race, tests) | `ui/screens/mod.rs:1897-1907` |
| `.planning/` present but unreadable (permissions) | reader returns an empty digest -> reads clean | `context.rs:257-259` returns `Vec::new()` on canonicalize failure; scans skip on `read_dir` error (pre-existing gap, out of scope) |

Recommended types and copy (reuse the existing `None` channel; no timer):
- `AppContext.owner_digest_failed: HashSet<String>` (`ui/screens/mod.rs` next to `owner_digests`, `:1371`).
- New `Action::OwnerDigestFailed { alias: String }` (plain data, `Action` stays `Clone`), sent by `schedule_owner_digest`'s panic branch instead of `return` (`mod.rs:1915-1924`).
- `ProjectStateLoaded` handler (`app.rs:2993-3000`): `Some(digest)` stored -> `owner_digest_failed.remove(&alias)`; `None` while `owner_digest_storable(&alias)` -> `owner_digest_failed.insert(alias)` and set `needs_redraw`.
- `OwnerDigestLoaded` handler (`app.rs:3706-3714`): remove from the set when it stores.
- `prune_driver_maps` (`app.rs:3780-3782`): `owner_digest_failed.retain(|a| registered.contains_key(a))`.
- Do **not** feed the dashboard `You:` summary (36 layout frozen; `summary_owner_rows` keeps counting a failed project as loading there; record as a follow-up).
- Inbox classification per registered alias: `loaded` (`ctx.owner_digest(alias)` is `Some`), `unreadable` (`owner_state_unavailable(alias)` or in `owner_digest_failed`), else `loading`.
- Header copy: DIM ` · {M} loading` (existing) then DIM ` · {U} unreadable` [inferred, matches the UI-SPEC "unreadable" vocabulary and the `inbox --json` `error` entries `folder_missing`/`no_planning`]; both clauses are the first thing every header rung drops, unreadable before loading. Body when no digest at all and `M == 0`, `U > 0`: DIM `No project could be read ({U} unreadable)`. This replaces the UI-SPEC's "accepted gap" row; tests: `OwnerDigestFailed` -> header shows `1 unreadable` not `1 loading`; a later `ProjectStateLoaded` with a digest clears it.

### Pattern 6: Inbox view state and screens

- Keep inbox view state on `AppContext` (e.g. `ctx.inbox_view: InboxViewState { filter, cursor: Option<InboxStop>, cursor_index, expanded: HashSet<InboxKey>, open_folds, fold_cursor, offset }`), reset when `i` pushes the screen. Reason: `AckConfirmScreen` paints its background by re-rendering the host screen fresh, the `queue_delete_confirm.rs` pattern (`detail.render_main_only(frame, area, ctx); confirm_popup::dim_background(frame, area);`), which only works when the view state is not inside the screen value.
- `InboxKey = (alias: String, GroupKey)`; `GroupKey` already carries lane, kind, scope label, workstream, phase, file (`owner_block.rs:55-62`), and the BLOCKED aggregate key (`GroupKey::aggregate`, `:77-84`) is the same identity the Owner block uses, so `g` can focus it there.
- Selection falls to the next row, else previous, else fold line, else none: reuse the Owner block's `resolve_cursor`/`clamp` approach (`owner_block.rs:1302-1409`) over inbox stops.

### Pattern 7: `v` testable split (SC1, D-13)

Pure `owner_actions::run_plan(ctx, alias, &OwnerGroup) -> Result<RunPlan, Refusal>` with `RunPlan { cwd, command, argv: Vec<String> /* ["env","claude",cmd] */, live_session: Option<ClaudeSession> }`; the key handler executes it (`focus_message` or `multiplexer::launch(&resolve_launch_plan(), &cwd, ExecArgv::new(argv), &|t| owner_terminal_argv(t, &cmd), &format!("Launched {cmd} in {alias}"))`). SC1 test: dashboard `i`, `j`, `v` through `NormalScreen`/`InboxScreen::handle_key` with a test seam that captures the `RunPlan` (or a `#[cfg(test)]` launcher hook on `AppContext`), asserting cwd = project path and argv `["env","claude","/gsd-verify-work 24"]`. A second test feeds the plan to `launch_with` with a fake backend and a fake GUI spawner. `FakeBackend` is private to `multiplexer::tests` (`multiplexer/mod.rs:1291`); move it to `multiplexer/fixtures.rs` as `#[cfg(test)] pub(crate)` or test the GUI branch only (plan with no backends, `gui: Some("kitty")`).

GUI argv: `owner_terminal_argv(term, cmd) = [terminal_program_separator(term), "claude", cmd]`, placed beside `launch_terminal_argv` in `detail.rs` (`:2865`) with a pin test: elements 0 and 1 authored, element 2 equals the validated `RunCommand` string, length exactly 3.

### Pattern 8: `A` acknowledge child

Verified gsd-core 1.21.0 facts:
- Router: `subcommands: ['run', 'acknowledge']`, `acknowledge: () => a.cmdAuditAcknowledge(cwd, hubArgs.slice(2), raw)` [VERIFIED: ~/.claude/gsd-core/bin/lib/audit-command-router.cjs:72-95].
- Phase-scoped set: `const PHASE_SCOPED = new Set(['uat_gaps', 'verification_gaps', 'context_questions', 'deferred_items']);` with `--phase`/`--file` required and `ioError(\`no phase directory found for phase "${phase}"…\`)` [VERIFIED: audit.cjs:1487-1496]; `debug_sessions` requires `--slug` [VERIFIED: audit.cjs:1577-1579]. Value flags: `'category', 'milestone', 'at', 'phase', 'file', 'archived-milestone', 'slug', 'seed-id', 'dir', 'filename', 'text'`, `positionals: 0` [VERIFIED: audit.cjs:1451-1458]. `--ws` is stripped globally before dispatch (`parseCliWorkstream`, `active-workstream-store.cjs:369-395`).
- `resolvePhaseTargetDir` compares the directory's leading token exactly (`if (phaseNum !== phase) continue;`) [VERIFIED: audit.cjs:1384-1401], so `--phase` = `OwnerGroup.phase` (the dir token from `dir_token`), never the roadmap number.

argv (program and prefix from `resolve_gsd_tools_from`, `queue_md.rs:153-169`, bumped to `pub(crate)`; it yields `node <shim>` or a `gsd-tools` on PATH):

```
<program> [<shim>] audit-open acknowledge --category <cat> --milestone <v> --phase <dir token> --file <basename> [--archived-milestone <label>] [--ws <name>]
<program> [<shim>] audit-open acknowledge --category debug_sessions --milestone <v> --slug <stem>
```
Category by `Kind`: `Uat` -> `uat_gaps`, `Verification` -> `verification_gaps`, `ContextQuestions` -> `context_questions`, `Debug` -> `debug_sessions`; `Deferred`, `Handoff`, `Checkpoint`, BLOCKED aggregates -> refusal. Tokens re-validated: phase with `is_command_token`, workstream with `is_workstream_name`, file basename and slug with `crate::text::is_identity_char` (all argv elements, never an interpreter string). `--milestone` = `ProjectState.milestone` (`src/state_reader/mod.rs:61` `pub milestone: String`) or the workstream's `ws.state.milestone`; empty -> refusal.

Run with `run_bounded_with_stderr` (`process.rs:444-476`): cwd = project root, 10 s budget, stderr to a temp file, last non-empty stderr line returned. It returns `None` for both "could not start" and "timed out" (`bounded_output.rs:49-52`), so check `node` on `PATH` first (pure `split_paths` scan, as `gsd_tools_on_path` does at `queue_md.rs:172-180`) and treat `None` as the timeout copy.

### Anti-Patterns to Avoid
- **Re-spelling the Owner block's row text** in the inbox: bump visibility and reuse `row_cells`, `phase_cell`, `item_text`, `detail_text`, `gap_text`, `expansion_lines`; the inbox only adds its alias column, drops the lane tag, clears the `closes phase` phrase (section title says it) and shows Command on FIX rows only.
- **Calling `open_project_view` from the inbox:** it refuses unless the root screen is alone (`if self.screen_stack.len() != 1 { return false; }`, `app.rs:3831-3834`). `g` must push `DetailScreen::opened_on(alias, DetailSubView::Pipeline, ctx)` (`detail.rs:1569`) itself.
- **Building the `v` command from `group.command`** (D-11) or from `group.phase` (directory token, wrong for roadmap-number commands).
- **Writing OSC 52 from `render`** or from a key handler: ratatui owns stdout. Route it like `SuspendAndEdit`/`ToggleMouseCapture`: a `ScreenAction` -> `App` field -> main loop writes after the frame (`main.rs:649-651` is the mouse precedent).
- **Waiting on a forking clipboard tool's pipes** (see Pitfall 4).
- **`#[cfg(target_os = …)]`-gated helpers that are dead on one target** (cross-target clippy gate).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Locating gsd-tools | a new `install_candidates` projection | `queue_md::resolve_gsd_tools_from` (bump to `pub(crate)`) | Already mirrors gsd-core's resolver order and the PATH fallback; one list, cannot drift |
| Running a short child with a deadline | own `Command` + thread | `launcher::process::run_bounded_with_stderr` | Temp-file stderr (cannot block), last-line extraction, kill on budget, already on the spawn allowlist |
| Confirm dialog | a custom modal | `confirm_popup::{ConfirmPopup, render_confirm_popup}` | Default No, clipped-is-not-confirmable guarantee, mouse mapping |
| Terminal-safe text | own escaping | `shown`, `fit_cells`, `fit_spans`, `render_for_terminal`, `terminal_capped` | OWN-11 census relies on these |
| Session launch / focus | own tmux/herdr calls | `multiplexer::launch`, `focus_message` | CWE-88 seam, per-session routing guard |
| Markdown file view | own renderer | `archive::render_markdown_lines` + `line_number_lines` | Escapes every line (`archive.rs:530`) |
| Checklist text for `y` | a second formatter | extract `pub(crate) fn group_checklist(alias, group) -> Vec<String>` from `inbox::push_section` (`inbox.rs:455-470`), called by `push_section` | D-09 "the two can never disagree"; goldens prove byte identity |

**Key insight:** every hard part (argv safety, terminal escaping, subprocess deadlines, popup safety) already has a reviewed seam with a guard test; new code that bypasses one will fail a guard or reopen a closed CVE-class bug.

## Runtime State Inventory

Not a rename/refactor phase. One runtime-state note that matters for `A`: gsd-core's **active-workstream pointer** (`.planning/active-workstream`, or a session-scoped file keyed by `GSD_SESSION_KEY`/`CLAUDE_SESSION_ID`/`TMUX_PANE`/…) is runtime state outside git that redirects a root-scoped acknowledge (Pitfall 2). Nothing else: stored data none, live service config none, OS-registered state none, secrets none, build artifacts none — verified by reading the touched modules.

## Common Pitfalls

### Pitfall 1: The `audit-open` source guard
**What goes wrong:** `cargo test --test owner_open_conformance` fails as soon as `src/` contains the acknowledge argv.
**Why:** `const ORACLE_VERB: &str = "audit-open";` and the test flags every non-comment line under `src/` containing it ("the audit oracle is a TEST-time tool and must never be reachable from the product") [VERIFIED: tests/owner_open_conformance.rs:44, 493-510].
**How to avoid:** spell the verb once in one file (e.g. `const ACK_VERB: [&str; 2] = ["audit-open", "acknowledge"];` in `owner_actions.rs`) and narrow the guard in the same commit: allow exactly that file, and assert that every allowed hit line also contains `"acknowledge"` so the read verb (`audit-open` / `audit-open run`) stays forbidden. Keep the non-vacuity floor.
**Warning signs:** a plan that adds the argv without touching `tests/owner_open_conformance.rs`.

### Pitfall 2: Root acknowledge redirected into a workstream
**What goes wrong:** `--phase 03 --file 03-UAT.md` acknowledges `workstreams/<ws>/phases/03-*/03-UAT.md`, or fails with "no phase directory".
**Why:** "Priority: --ws flag > GSD_WORKSTREAM env var > session/shared pointer > null." [VERIFIED: ~/.claude/gsd-core/bin/gsd-tools.cjs:5266] and the resolved name is written back with `env['GSD_WORKSTREAM'] = resolution.ws;` so `planningDir` follows it [VERIFIED: active-workstream-store.cjs:405-443]. Reads fall back from the session pointer to the shared `.planning/active-workstream` marker [VERIFIED: active-workstream-store.cjs:203-245]. The session key comes from inherited env (`GSD_SESSION_KEY`, `CODEX_THREAD_ID`, `CLAUDE_SESSION_ID`, `CLAUDE_CODE_SESSION_ID`, `CLAUDE_CODE_SSE_PORT`, `OPENCODE_SESSION_ID`, `GEMINI_SESSION_ID`, `CURSOR_SESSION_ID`, `WINDSURF_SESSION_ID`, `TERM_SESSION_ID`, `WT_SESSION`, `TMUX_PANE`, `ZELLIJ_SESSION_NAME`) [VERIFIED: active-workstream-store.cjs:21-43]. There is no CLI spelling for "root" (`none` is reserved, `:427-432`).
**How to avoid:** (1) run the child with `GSD_WORKSTREAM` and all those session-key variables removed (`run_bounded_with_stderr` has no `env_remove`; add a variant with a `remove_env: &[&str]` parameter in `process.rs`, same file, still allowlisted); (2) workstream groups always pass `--ws <name>`; (3) refuse `A` on a root-scoped group when `<project>/.planning/active-workstream` exists and is non-empty: `Cannot acknowledge {file}: {alias} has an active workstream set; run it in the project` [inferred copy]; (4) add a fixture with a pointer to the oracle round trip to prove it.
**Warning signs:** an oracle round trip that only covers a project without workstreams.

### Pitfall 3: `focus_message` and spawn-site guards
**What goes wrong:** `focus_entry_guard` and `spawn_seam_guard` go red.
**Why:** `const FOCUS_CALLERS: &[&str] = &["src/ui/screens/detail.rs", "src/ui/screens/normal.rs"];` [VERIFIED: tests/focus_entry_guard.rs:31]; `SPAWN_ALLOWLIST` lists every file allowed to contain `Command::new(` / `process_group(` [VERIFIED: tests/spawn_seam_guard.rs:34-79, 81].
**How to avoid:** add `src/ui/screens/owner_actions.rs` (or whichever file calls `crate::multiplexer::focus_message(`) to `FOCUS_CALLERS`, and `src/clipboard.rs` to `SPAWN_ALLOWLIST` with a one-line reason ("`wl-copy`/`xclip`/`xsel`/`pbcopy`/`clip.exe` with the checklist on stdin. No agent."), each in the commit that adds the call. The acknowledge child reuses `process.rs`, so it needs no allowlist change. Also update the `multiplexer::launch` doc ("called only from `detail.rs`", `multiplexer/mod.rs:1014-1016`).

### Pitfall 4: Clipboard tools that fork
**What goes wrong:** the bounded child never "finishes" and `y` reports a timeout although the copy worked.
**Why:** `wl-copy --help`: `-f, --foreground	Stay in the foreground instead of forking.`; `xclip -help`: `-silent      errors only, run in background (default)` [VERIFIED: local `wl-copy --help`, `xclip -help` this session]. The forked server inherits any piped stdout/stderr, so reading them to EOF blocks until the selection is replaced.
**How to avoid:** spawn with `stdin(piped)`, `stdout(null)`, `stderr(null)`; write the text, drop stdin, then `try_wait` until the parent exits or the budget (2 s [inferred]) passes. Argv: `wl-copy` (text via stdin), `xclip -selection clipboard`, `xsel --clipboard --input`, `pbcopy`, `clip.exe`. Tool choice is a pure function of `(target os, WAYLAND_DISPLAY, DISPLAY, which(tool))` so every branch compiles and is used on every target (no dead-code lint on Windows/macOS).
**Warning signs:** a clipboard helper built on `run_bounded` (it captures stdout).

### Pitfall 5: Owner block output must stay byte-identical
**What goes wrong:** pending 36 human UAT is invalidated.
**How to avoid:** only `pub(crate)` bumps plus one new parameter `expansion_lines(src, width, shift)` where the Owner block passes 0. The 36 snapshot tests in `owner_block.rs` and `detail.rs` must pass unedited; the only 36 test that changes is `owner_block_swallows_action_keys` (`detail.rs:36277-36299`, iterates `['e', 'o', 'v', 'A', 'y', 'x']`), narrowed to `e` and `x`. `owner_block_footer_spans` (`detail.rs:11736`) is not touched.

### Pitfall 6: `g` opens a project that is not the dashboard selection
**What goes wrong:** with the detail view of X on top and the dashboard selection on Y, App code keyed by `ctx.selected_alias()` (Driver-tab tick rescan `app.rs:1172-1188`, launcher run refresh `app.rs:2900`) works on Y.
**How to avoid:** on `g`, select X in the dashboard table when it is in `ctx.filtered_aliases` (the same reasoning as `open_project_view`'s doc, `app.rs:3826-3830`); D-01's "selection unchanged" then applies to `i`/`Esc` without `g`. If X is hidden by the dashboard filter, open anyway [inferred]; only the experimental Driver tab's tick rescan is affected.

### Pitfall 7: gsd-tools warnings on stderr
**What goes wrong:** `Acknowledge failed: gsd-tools: warning: …/.gsd/defaults.json sets …` instead of the real refusal.
**Why:** observed this session: every gsd-tools call on this machine prints `gsd-tools: warning: /home/blk/.gsd/defaults.json sets model_profile, … (#3532)` on stderr before its output.
**How to avoid:** use the **last** non-empty stderr line (what `run_bounded_with_stderr` already returns), not the first as the UI-SPEC copy says; full capped output to the tracing log. Escape with `render_for_terminal` before it reaches the status line.

### Pitfall 8: Status messages expire in 3 s
**What goes wrong:** the owner misses a refusal from `A` or `v`.
**Why:** `Action::Tick` clears `status_message` after 3 s (`app.rs:2711-2718`).
**How to avoid:** keep each message one line with the decision words first (UI-SPEC rule); log the full text. Accepted limitation, do not add a second channel.

### Pitfall 9: Ordering differs from the CLI text
`inbox::render_text` sorts TEST by `closes_phase`, then **remaining phases** (`phases_total - phases_done`, fewer first), then age newest-first (`b.group.age_days.cmp(&a.group.age_days)`) [VERIFIED: src/inbox.rs:638-646]; D-05 wants the progress **ratio** highest first and age oldest first. Write `sort_groups` for the screen only; do not change `render_text` (goldens). Compare ratios by cross-multiplication (`a.done * b.total` vs `b.done * a.total`), `total == 0` as ratio 0. Record the divergence in the README ordering sentence or raise it as a follow-up.

## Code Examples

### Inbox stop identity and fall-through (pattern from `owner_block.rs:1302-1316`)
```rust
// Source: src/ui/screens/owner_block.rs:1302-1311 (resolve_cursor), adapted
pub(crate) fn resolve(view: &InboxViewState, stops: &[InboxStop]) -> Option<usize> {
    if stops.is_empty() { return None; }
    let found = view.cursor.as_ref().and_then(|s| stops.iter().position(|t| t == s));
    Some(found.unwrap_or_else(|| view.cursor_index.min(stops.len() - 1)))
}
```

### Owner argv beside the existing builders
```rust
// Source shape: src/ui/screens/detail.rs:2836-2840 (mux_launch_argv)
pub(crate) fn owner_mux_argv(cmd: &RunCommand) -> Result<ExecArgv, MuxError> {
    // `env` head: len >= 2 by construction; the command is ONE element; cwd travels outside.
    crate::multiplexer::ExecArgv::new(vec!["env".into(), "claude".into(), cmd.to_string()])
}
```

### OSC 52 sequence (no crate)
```rust
// RFC 4648 base64; ESC ] 52 ; c ; <b64> BEL. Test vectors: "" -> "", "f" -> "Zg==", "foobar" -> "Zm9vYmFy".
pub fn osc52(text: &str) -> String { format!("\x1b]52;c;{}\x07", base64(text.as_bytes())) }
```

### Help rows (existing helpers)
```rust
// Source: src/ui/screens/help.rs:192-205 (heading, row); 36 rows at :348-359
heading("Owner Inbox"),
row("1 2 3 4", "Show one lane: Test, Decide, Fix, Blocked (press the key again for the default view)"),
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `gsd-tools audit acknowledge` (report §3.4) | `gsd-tools audit-open acknowledge` | gsd-core 1.21.0 (router `subcommands: ['run', 'acknowledge']`) | argv spelling, D-14 |
| Interim `i` status message (36 D-12) | `InboxScreen` push | this phase | replace `normal.rs:806-814` and its test (`normal.rs:2980`) and help row (`help.rs:233-234`, test `:891-892`) |
| Layer B counts Layer A lines | Layer B counts top-level entries, planned phases hidden | this phase | `decide` drops (fleet 46 -> 16 expected) |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `claude "/gsd-verify-work 24"` starts an interactive session that runs the slash command as its first prompt | Pattern 7 | `v` opens Claude without running the command; human UAT item 2 catches it |
| A2 | `xsel --clipboard --input`, `pbcopy`, `clip.exe` accept the text on stdin and exit (pbcopy/clip.exe synchronously) | Pitfall 4 | One platform path reports failure; covered by fake-tool tests, real check is manual |
| A3 | tmux with `set -g set-clipboard on` forwards OSC 52 to the outer terminal | README copy | Copy silently lost inside tmux; the status copy never claims success for OSC 52 |
| A4 | gsd-core's `ioError` prints the refusal as the last stderr line and exits non-zero | Pitfall 7 | Failure copy shows a less useful line; full output is in the log |
| A5 | Counting archived dropped questions in `planned_questions`, acknowledged-wins-over-planned, unreadable-stays-listed | Pattern 1 | Different numbers in `hidden`; cosmetic, adjustable in one plan |
| A6 | `· {U} unreadable` header clause wording and `No project could be read` body | Pattern 5 | Copy only |
| A7 | 2 s budget for clipboard tools | Pitfall 4 | Slow X servers time out; raise the constant |

## Open Questions (RESOLVED)

1. **D-17 gate source.** CONTEXT names `DiskStatus.plan_count + summary_count`; evidence above shows it misses SUMMARY-only, archived and non-roadmap phases. Recommendation: use the listing-based `phase_planned` (Pattern 1) and note it as a correction of an `[inferred]` decision; the expected fleet effect is unchanged for mailbot and sentriq.
   RESOLVED: adopted in 37-01 (Task 1: `ContextFile.phase_planned` from the CONTEXT scan's directory listing via `is_plan_artifact`; recorded under "Divergences from CONTEXT" in 37-01's and 37-07's SUMMARY).
2. **D-19 (c) "`<open_questions>` section".** Layer A (and therefore Layer B) only reads a `## Open Questions` heading; the live files wrap that heading in `<open_questions>` tags. Recommendation: fixture (c) uses the live shape (tags around the `## Open Questions` heading); a tag-only section without the heading has zero questions in both layers.
   RESOLVED: adopted in 37-01 (Task 2: fixture `context-entries` with `<open_questions>` tags around the `## Open Questions` heading; a tag-only section stays at zero questions in both layers).
3. **`o`: deep link vs viewer (discretion).** The Docs deep link mutates the project's persistent Docs browse state and needs two `Esc` presses (View -> List -> tab bar, `detail.rs:4199-4224`), which breaks the UI-SPEC "`Esc`/`q` return to the inbox". Recommendation: a small pushed `OwnerFileViewerScreen` that reads on key press with the reader's guards (regular file, <= 1 MiB, canonical path inside the project's `.planning/`), renders with `render_markdown_lines` computed once, scrolls, and pops on `Esc`/`q`. Needs a `SCREEN_IDENTITY_DISPOSITIONS` row and fixture (`render_escape_guard.rs:363-391`).
   RESOLVED: adopted in 37-05 (Task 1: the in-app read-only viewer `OwnerFileViewScreen` in src/ui/screens/owner_file_view.rs, with its census row and hostile fixture; D-21 `o` on the Owner block uses the same path).
4. **Old debt fold unit.** UI-SPEC says `{N} checks`; the CLI says "items" once archived questions are in the total (`inbox.rs:704-710`). Recommendation: reuse the CLI rule so the TUI and CLI agree.
   RESOLVED: adopted in 37-04 (Task 2, fold-line ladder: the Old debt fold says `items` once archived questions are in its total, exactly as the CLI does).

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build/tests | ✓ | cargo 1.99.1 | — |
| rustup targets aarch64-apple-darwin, x86_64-apple-darwin, x86_64-pc-windows-gnu | cross-target clippy gate | ✓ | installed | — |
| node | `A`, oracle tests | ✓ | v24.21.0 | refusal message |
| gsd-core | `A`, oracle round trip | ✓ | 1.21.0 (`~/.claude/gsd-core/VERSION`; the profile's `CLAUDE_CONFIG_DIR` install at `~/.local/share/claude-profiles/player-health/gsd-core` also resolves) | `ALLOW_MISSING_ORACLE` opt-out in tests |
| tmux / herdr | human UAT 2 | ✓ / ✓ | `/usr/bin/tmux`, `~/.local/bin/herdr` | GUI terminal fallback |
| docker / podman | `pre-tag-check.sh --container` | ✓ / ✓ | — | — |
| wl-copy / xclip | `y` native path | ✓ / ✓ | — | OSC 52 |
| xsel | `y` alt path | ✗ | — | xclip, OSC 52 |
| `WAYLAND_DISPLAY` / `DISPLAY` in the agent shell | live clipboard check | ✗ (unset here) | — | fake-tool tests; manual check |

**Missing dependencies with no fallback:** none.

## Validation Architecture

(`workflow.nyquist_validation` is `false` in `.planning/config.json`; included because the coordinator asked for it.)

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in `#[test]` / `#[tokio::test]`; ratatui `TestBackend` buffer assertions (no `insta`) |
| Config file | none (Cargo); release gate `scripts/pre-tag-check.sh --container vX.Y.Z` |
| Quick run command | `rtk bypass cargo test --lib -- owner inbox` (filter by module substring; narrow further per task, e.g. `ui::screens::inbox`, `ui::screens::owner_actions`, `state_reader::owner_items`) |
| Full suite command | `rtk bypass cargo test --locked --no-fail-fast --all-targets` (prior phases: 4251 passed, 19 ignored) then `rtk bypass cargo clippy --locked --all-targets -- -D warnings` |

Commands prior phases actually used (from 35/36 SUMMARY and VERIFICATION files): `cargo test --lib -- owner summary attention you_cell digest`, `cargo test --test async_blocking_guard`, `cargo test --locked --no-fail-fast --all-targets`, `cargo clippy --locked --all-targets -- -D warnings` (host plus `x86_64-pc-windows-gnu` and `aarch64-apple-darwin`). Always through `rtk bypass` so result lines are not filtered. `cargo fmt` reformats the unrelated `src/state_reader/disk_status.rs`; revert that file, never commit it (noted in four 35/36 summaries).

### Phase Requirements -> Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| OWN-20 (D-17/18) | planned phase hides questions; SUMMARY-only too; 4 entries -> 3 items | integration | `rtk bypass cargo test --test owner_digest` | ✅ file; ❌ new fixtures `tests/fixtures/owner/context-planned/`, `context-summary-only/`, `context-entries/` |
| OWN-20 (D-18) | entry parser, matcher extension | unit | `rtk bypass cargo test --lib -- owner_items::context owner_items::rules` | ✅ |
| OWN-20 (schema) | `planned_questions` key paths, README fence/table | integration | `rtk bypass cargo test --test inbox_cli` | ✅ (update sets) |
| OWN-20 (Layer A) | oracle unchanged | integration (oracle) | `rtk bypass cargo test --test owner_open_conformance --test owner_audit_conformance` | ✅ |
| OWN-20 (screen) | 140x40/120x40/100x30/80x24 tiers, SC2 legibility, degrade 79..0x0, filters, quiet color, expansion, folds, scroll, empty/loading/unreadable, live refresh SC3 | unit (buffer) | `rtk bypass cargo test --lib -- ui::screens::inbox` | ❌ Wave 0: `src/ui/screens/inbox.rs` tests + `render_inbox_cells(ctx, w, h)` helper |
| OWN-20 (ordering) | `sort_groups` table | unit | same | ❌ |
| OWN-20 (`y`) | text == `inbox::group_checklist`; tool choice by env; fork-safe child; OSC 52 base64 vectors; 64 KiB cap | unit | `rtk bypass cargo test --lib -- clipboard ui::screens::owner_actions` | ❌ |
| OWN-20 (goldens) | `group_checklist` extraction byte-identical | integration | `rtk bypass cargo test --test owner_digest -- golden` | ✅ |
| OWN-21 | argv table, refusals, GUI argv pin, focus branch, SC1 `i`,`j`,`v` | unit | `rtk bypass cargo test --lib -- ui::screens::owner_actions ui::screens::normal` | ❌ |
| OWN-21 | `command == run.to_string()` over all fixtures | integration | `rtk bypass cargo test --test owner_digest -- run_command` | ❌ |
| OWN-22 | ack argv table, refusals (category, milestone, pointer, tokens), popup default No, 60x20 clipped inert, busy file, vanished group | unit | `rtk bypass cargo test --lib -- ui::screens::owner_actions ui::screens::ack_confirm` | ❌ |
| OWN-22 | real `gsd-tools audit-open acknowledge` on a temp copy, reader then hides the group; root group with a pointer fixture | integration (oracle-gated) | `rtk bypass cargo test --test owner_acknowledge_roundtrip` | ❌ Wave 0 (copy the `Oracle::resolve` shape from `tests/owner_open_conformance.rs:72-115`, `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` opt-out) |
| OWN-22 | guard edits | integration | `rtk bypass cargo test --test owner_open_conformance --test focus_entry_guard --test spawn_seam_guard --test async_blocking_guard` | ✅ (edit lists) |
| OWN-20..22 | escape census for InboxScreen, AckConfirmScreen, viewer, footer status | unit | `rtk bypass cargo test --lib -- render_escape_guard` | ✅ (add rows + fixtures) |
| OWN-23 | help rows whole-row, unique descriptions | unit | `rtk bypass cargo test --lib -- ui::screens::help` | ✅ (update `help_lines_documents_every_key_this_phase_binds`) |
| D-21 | Owner block keys act; layout byte-identical | unit | `rtk bypass cargo test --lib -- ui::screens::detail::tests::owner ui::screens::owner_block` | ✅ (narrow swallow test) |
| bounded digest state | `OwnerDigestFailed` -> unreadable; recovery clears; prune | unit (tokio) | `rtk bypass cargo test --lib -- app::tests::owner` | ❌ |

### Sampling Rate
- **Per task commit:** the quick command for the touched module plus `rtk bypass cargo clippy --all-targets -- -D warnings`.
- **Per wave merge:** `rtk bypass cargo test --locked --no-fail-fast --all-targets` and the cross-target clippy for `x86_64-pc-windows-gnu` and `aarch64-apple-darwin` (clipboard module).
- **Phase gate:** full suite green, then `./scripts/pre-tag-check.sh --container` before the milestone tag (it runs the oracle round trip with gsd-core installed).

### Wave 0 Gaps
- [ ] `tests/fixtures/owner/context-planned/`, `context-summary-only/`, `context-entries/` (synthetic text, live shapes; carry `audit-open.expect: same` so the oracle also pins Layer A on them)
- [ ] `src/ui/screens/inbox.rs` test module with `render_inbox_cells` and the UI-SPEC 9-project fixture fleet
- [ ] `tests/owner_acknowledge_roundtrip.rs` (oracle-gated, temp copy, never the repo tree)
- [ ] `FakeBackend` reachable from UI tests (`multiplexer/fixtures.rs`, `#[cfg(test)] pub(crate)`) or GUI-branch-only launch test

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | local single-user tool |
| V5 Input Validation | yes | `is_command_token`, `is_workstream_name`, `is_identity_char` on every argv element derived from project files; `ExecArgv::new`; file viewer containment (canonical path under `.planning/`, regular file, 1 MiB) |
| V6 Cryptography | no | (base64 is encoding, not crypto) |
| V12 Files and Resources | yes | `o` reads only contained regular files; app never writes into a project (gsd-core writes for `A`) |
| V14 Configuration | yes | child env scrubbed of workstream/session variables for `A` |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Argument injection via a hostile phase dir / workstream / file name (CWE-88) | Tampering / Elevation | typed tokens re-validated at launch; each value one argv element; no interpreter string; cwd outside argv |
| Terminal escape injection via project strings (CWE-150) | Tampering | `shown`/`render_for_terminal`; `render_escape_guard` fixtures for every new screen and the status line |
| Acknowledge of the wrong file (workstream pointer, TOCTOU between popup and Yes) | Tampering | env scrub + pointer refusal; re-resolve the group by identity on Yes; popup shows exact argv |
| Path traversal via `o` | Information disclosure | canonicalize and require the prefix `<project>/.planning/`; reader's containment already applied to digest paths |
| Clipboard payload carrying control bytes | Tampering | copied text is the `terminal_capped` checklist; OSC 52 payload is base64 |
| Unbounded child | DoS | 10 s (`A`) and 2 s (`y`) budgets, kill on expiry, off the render thread |

## Recommended plan split (coarse)

(Research-time recommendation with its own numbering; the plans actually written are 37-01..37-07 as listed in ROADMAP.md, and the RESOLVED markers above use those numbers.)

| Plan | Wave | Scope | Depends on |
|------|------|-------|------------|
| 37-01 Reader: planned gate, entries, `planned_questions`, typed `RunCommand` | 1 | `owner_items/{context,rules,mod}.rs`, `inbox.rs` JSON, `tests/inbox_cli.rs`, `tests/owner_digest.rs`, new fixtures, README schema rows, `agent_skill.md` | — |
| 37-02 Plumbing | 1 | Owner block visibility (byte-identical), `inbox::group_checklist`, digest-failure channel (`action.rs`, `app.rs`, `ui/screens/mod.rs`), `process.rs` env-removal variant, `src/clipboard.rs` + spawn allowlist, `queue_md` resolver bump | — (touches `inbox.rs` in a different function than 37-01; serialize the merge or let 37-02 rebase) |
| 37-03 Inbox screen | 2 | `ui/screens/inbox.rs`, `normal.rs` `i`, `g` (selection pin), `o` viewer, `y`, folds, header unreadable clause, OSC 52 main-loop write, escape census rows | 37-01 (`planned_questions` not shown, but `RunCommand` for the `run` line), 37-02 |
| 37-04 Actions and docs | 3 | `owner_actions.rs` (`v`, `A`), `ack_confirm.rs`, `detail.rs` argv builders + D-21 keys, guard edits, oracle round trip, help overlay, README "Owner inbox" text | 37-03 |

## Sources

### Primary (HIGH confidence, read this session)
- `src/state_reader/owner_items/{mod,context,rules,markdown}.rs`, `src/state_reader/disk_status.rs`, `src/state_reader/queue_md.rs`, `src/state_reader/gsd_install.rs`
- `src/inbox.rs`, `src/inbox_cli.rs`, `src/app.rs`, `src/action.rs`, `src/ui/screens/{mod,normal,detail,owner,owner_block,confirm_popup,queue_delete_confirm,render_escape_guard,help}.rs`, `src/multiplexer/mod.rs`, `src/launcher/process.rs`, `src/browser.rs`, `src/archive.rs`, `src/main.rs`
- `tests/{owner_open_conformance,focus_entry_guard,spawn_seam_guard,async_blocking_guard,inbox_cli,owner_digest}.rs`, `tests/fixtures/owner/context-questions/`, `tests/fixtures/owner_golden/inbox.txt`
- gsd-core 1.21.0: `bin/lib/audit.cjs` (1384-1401, 1447-1500, 1577-1579), `bin/lib/audit-command-router.cjs` (59-100), `bin/lib/active-workstream-store.cjs` (21-43, 186-245, 369-443), `bin/gsd-tools.cjs` (5260-5290)
- `.planning/phases/37-inbox-screen-and-actions/{37-CONTEXT,37-UI-SPEC}.md`, `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.planning/research/OWNER-INBOX-UX.md` §3.3/3.4, `.planning/STATE.md`
- Local probes: `wl-copy --help`, `xclip -help`, `node --version`, `rustup target list --installed`, a gsd-tools invocation (stderr warning)
- Live fleet, read-only: mailbot `03-CONTEXT.md` open-questions section, sentriq `16-raw-can-button-replay-poc/` listing and section

### Secondary / Tertiary
- None (no web sources needed; no new packages).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - no new dependencies; all reused functions read at their definitions.
- Architecture: HIGH - delivery paths, guards and seams read in full; module split is a recommendation within CONTEXT discretion.
- Pitfalls: HIGH for guards, workstream resolution, DiskInference counting (source read); MEDIUM for clipboard tool behavior (help text verified, runtime not exercised without a display).

**Research date:** 2026-11-14
**Valid until:** 2026-12-14 (gsd-core pinned at 1.21.0; re-check `cmdAuditAcknowledge` flags on a gsd-core bump)
</content>
</invoke>
