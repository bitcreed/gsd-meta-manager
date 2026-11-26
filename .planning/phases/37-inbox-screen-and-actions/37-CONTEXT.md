# Phase 37: Inbox screen and actions - Context

**Gathered:** 2026-11-14
**Status:** Ready for planning

Gathered unattended (`--auto`). Every decision follows `.planning/research/OWNER-INBOX-UX.md`
(adopted as written by REQUIREMENTS.md: report §3.3, §3.4, §6, §7) and the Phase 34-36 decisions,
unless marked **[inferred]**: decided by the orchestrator from the report, REQUIREMENTS.md,
ROADMAP.md, the Phase 34-36 artifacts (incl. `36-UI-SPEC.md`), a scout of the launch machinery
(`src/multiplexer/mod.rs`, `src/ui/screens/detail.rs`, `src/launcher/process.rs`), gsd-core 1.21.0
(`bin/lib/audit.cjs`, `bin/lib/audit-command-router.cjs`, `workflows/complete-milestone/`), a live
probe of the acknowledge writer on a throwaway project, and a read-only look at the live fleet's
Decide lane.

<domain>
## Phase Boundary

The TUI gets the cross-project inbox screen behind `i` and the three owner actions, all consuming
the digest Phases 34-36 already compute (`AppContext` owner digest map, 36 D-02):

- **Inbox screen** (OWN-20): report §3.3 layout, lane filters `1-4`/`a`, the ordering rule,
  expand, `g` go to project, `o` open file, `y` copy checklist; legible at 100x30.
- **Run** (OWN-21): `v` launches the group's GSD command in the right project through
  `multiplexer::launch`, or focuses a live session.
- **Acknowledge** (OWN-22): `A` runs gsd-core's own acknowledge writer after a confirm popup; the
  app writes nothing into any project.
- **Docs** (OWN-23, remaining half): help overlay and README "Owner inbox" section gain the screen
  and the actions.
- **One reader refinement** (D-17/D-18 below): CONTEXT open questions stop flooding the Decide lane.

Requirements: OWN-20, OWN-21, OWN-22, OWN-23.

Not in this phase: per-item pass/fail entry (OWN-F06, that is `/gsd-verify-work`'s job), a native
acknowledge writer (OWN-F05), todo triage beyond a count (OWN-F03), notifications/digest
(OWN-F07/F08), any change to `needs_human` / `❢` (D1), and any change to the Phase 36 dashboard
cell, summary line or Owner block layout (36 human UAT is pending on that layout).

</domain>

<decisions>
## Implementation Decisions

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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Scope and contract
- `.planning/research/OWNER-INBOX-UX.md` — §3 principle, §3.3 inbox mockup and ordering, §3.4
  actions, §4 R10 commands, §6 Phase 37 criteria and human UAT, §7 D1, D7, D10
- `.planning/REQUIREMENTS.md` — OWN-20..23, OWN-F05/F06 (out of scope)
- `.planning/ROADMAP.md` §"Phase 37" — goal, success criteria 1-4, human UAT
- `README.md` — "Owner inbox" section and the `inbox` schema

### Prior phases
- `.planning/phases/34-owner-items-reader-inbox-cli/34-CONTEXT.md` — Layer A/B split, oracle,
  D-22 counts
- `.planning/phases/35-decisions-code-needed-and-gates/35-CONTEXT.md` — D-02..D-06 Decide/Fix
  signals, D-04 open-question port (refined by D-17/D-18 here), D-11/D-12 counts and
  `closes_after`
- `.planning/phases/36-dashboard-and-detail/36-CONTEXT.md` — D-02 digest map, D-12 interim `i`,
  D-17..D-20 Owner block rows, expansion, folds, focus
- `.planning/phases/36-dashboard-and-detail/36-UI-SPEC.md` — visual tokens, colors, glyphs,
  narrowing order (the inbox must match)
- `.planning/phases/36-dashboard-and-detail/36-UAT.md` — pending human UAT; its layout is frozen

### gsd-core (acknowledge writer, 1.21.0)
- `~/.claude/gsd-core/bin/lib/audit-command-router.cjs` ~60-100 — `routeAuditOpen`, subcommand
  `acknowledge`
- `~/.claude/gsd-core/bin/lib/audit.cjs` ~1384 `resolvePhaseTargetDir` (exact dir token), ~1447
  `cmdAuditAcknowledge` (flags per category, snapshot keys, no commit)
- `~/.claude/gsd-core/workflows/complete-milestone/detail/elaboration.md` ~30-100 — reference
  invocations per category

### Code
- `src/ui/screens/normal.rs` ~806 (interim `i` arm), ~879 (`Tab` focus flow)
- `src/ui/screens/detail.rs` ~2784-2865 (`claude_launch_args`, `mux_launch_argv`,
  `launch_terminal_argv`), ~5131 and ~5594 (`multiplexer::launch` call sites)
- `src/multiplexer/mod.rs` — `ExecArgv::new`, `launch` / `launch_with`, `focus_message`
- `src/launcher/process.rs` — `run_bounded`, `run_bounded_with_stderr`
- `src/state_reader/gsd_install.rs` — `install_candidates` (gsd-tools resolution)
- `src/state_reader/owner_items/rules.rs` ~100-135 (`is_workstream_name`, `command_for`), open
  question Layer B (D-17/D-18 land here)
- `src/state_reader/disk_status.rs` — `plan_count`, `summary_count`
- `src/ui/screens/owner.rs`, `src/ui/screens/owner_block.rs` — shared owner formatters, Owner
  block (reuse, do not restyle)
- `src/ui/screens/confirm_popup.rs` — confirm popup
- `src/inbox.rs` — plain checklist formatter (shared with `y`)
- `src/ui/screens/help.rs`, `src/ui/screens/render_escape_guard.rs`
- `tests/driver_router_conformance.rs` — oracle gating pattern for the acknowledge round trip

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `multiplexer::launch` / `launch_with` (injectable backends) and the `env`-prefixed argv shape.
- `focus_message` and `ctx.active_sessions` matching by `working_dir` (dashboard `Tab`).
- `run_bounded` for a short host command with a time budget.
- `gsd_install::install_candidates` for locating `gsd-tools.cjs` like gsd-core does.
- Owner formatters (`owner.rs`), Owner block item expansion (`owner_block.rs`), `inbox.rs`
  checklist text, `confirm_popup.rs`.
- No clipboard code exists yet (D-09 adds the first).

### Established Patterns
- Blocking work on `spawn_blocking`, results back as `Action`s; render reads caches only.
- Screen stack with `ScreenAction::Push`/`Pop`; status messages via `SetStatusMessage`.
- Every launch path documents CWE-88 reasoning: untrusted strings only as single argv elements,
  path as cwd.
- Non-default states announce themselves; defaults are silent.

### Integration Points
- `normal.rs` `i` arm → `InboxScreen`; `AppContext` owner digest map → inbox rows; new `Action`
  for the acknowledge result; Owner block keys → shared action functions (D-21).

</code_context>

<specifics>
## Specific Ideas

- Report §3.3 mockup is the target look, including the section titles and the footer hint line
  `j/k move · Enter expand · g go to project · v run command · o open file · A acknowledge · y
  copy checklist`.
- The verified acknowledge invocation (probe, 2026-11-14): `gsd-tools audit-open acknowledge
  --category uat_gaps --milestone v1.0 --phase 03 --file 03-UAT.md` wrote
  `audit_acknowledged: {milestone: v1.0, at: 2026-11-14, gap_snapshot: "testing::scenarios=1"}`
  and left the file uncommitted; with `--ws ws1` it targeted the workstream's phase.

</specifics>

<deferred>
## Deferred Ideas

- Injecting the command into an already-running session (today `v` focuses and shows the
  command); would need the driver's steering path.
- Aggregating several debug sessions of one project into one DECIDE row (mockup's "5 debug
  sessions awaiting your confirmation") with a per-session sub-list; D-04 keeps one row each.
- Upstream gsd-core issue to add: `deriveOpenQuestions` counts lines, not questions, and ignores
  whether the phase was planned (extends report §8 item 8).
- Report §6 deferred list (OWN-F01..F08).

### Reviewed Todos (not folded)
`todo.match-phase 37` returned the same 17 keyword-only matches Phases 34-36 reviewed (score
0.2-0.9: TUI badge for release maps, help popup at 80x24, unregister guard, registry glyph
overflow, ambiguous-wide typography, sibling-run tmpfs, driverctl LOG.md redaction, run-lock hard
link, run-file hardening, multi-line queue input, launcher-marker in-flight flag, driverctl
`fm_status` IN-06, capture-pump redaction, managed-policy shapes, unsupervised child, executor
remnants). None touches the inbox screen, its actions or the Decide lane, so none is folded despite
the ≥ 0.4 auto-fold rule; folding them would be scope creep **[inferred]**.

</deferred>

---

*Phase: 37-inbox-screen-and-actions*
*Context gathered: 2026-11-14*
