# Phase 39: Check actions through GSD - Research

**Researched:** 2026-11-17
**Domain:** Rust TUI (ratatui/crossterm) key flow + typed argv launch of a GSD slash command with a plain-language instruction; gsd-core 1.21.0 verify-work / todo semantics
**Confidence:** HIGH for the codebase and gsd-core mechanics (source read + probes on a throwaway project); MEDIUM for how the model running `/gsd-verify-work` acts on the instruction (human UAT)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Carried forward (not re-decided)
- PROJECT.md "GSD files are read-only" (2026-11-16) and report D7: the app never writes
  `.planning/**` or `$GSD_HOME`; every change goes through a launched GSD command or `gsd-tools`.
- 37 D-11/D-12: commands are built from typed values (verb, validated phase token, validated
  workstream name), never from reader strings; launch through `multiplexer::launch` with
  `ExecArgv::new(["env", "claude", "<command>"])`, the command as ONE argv element, project path as
  cwd; a live session for the project is focused instead of starting a second one.
- 37 D-14..D-16: `A` (gsd-tools `audit-open acknowledge`, bounded child, its popup) stays exactly
  as is **[user]**.
- 38 D-08: a merged same-phase entry (UAT + VERIFICATION) is one row; phase-level actions act on
  all its files. 38 D-11: heuristic labels (`decision?`, `looks resolved`) never move, hide or
  re-rank anything **[user]**.
- 37 D-10: rows refresh from the watcher; nothing is hidden optimistically.
- WR-01: side-effecting letters act only unchorded (`owner_actions::is_plain_action_key`).

#### The three outcomes (OWN-29)
- **D-01 [user]:** Outcomes are exactly: **Won't do** (reason required), **Deferred permanently**
  (don't ask again), **Move to phase M** (re-asked in M; the source phase can close without it).
  The roadmap's "defer/move" is split into these two named outcomes; this is a clarification
  inside OWN-29, not a scope change.
- **D-02 [inferred]:** Reason text: required for Won't do (SC1 "with a reason"), optional for
  Deferred permanently and Move. Single line, control characters stripped, capped at 200 chars.
- **D-03 [inferred]:** Targets. A **check** = one item under an expanded group; a **phase** = a
  group row (merged entry included), meaning all its open checks. Outcomes apply to TEST and
  BLOCKED lane rows (things the owner performs or waits on). FIX, DECIDE (context questions, debug
  sessions, handoff, checkpoint), Old debt (archived) and Acknowledged fold rows are refused with
  a status message, like `A` refusals today. Reason: FIX is code work (`/gsd-plan-phase --gaps`),
  DECIDE rows are not checks, and an archived phase is not addressable by `/gsd-verify-work N`.
- **D-04 [inferred]:** Item-level cursor. The cursor stops today are group rows only. When a group
  is expanded, `j/k` also stop on its item lines (in both hosts: inbox and Owner block), so a
  single check can be selected. On an item line `v`, `o`, `y` and `A` keep acting on the parent
  group (unchanged behavior); only the new outcome key distinguishes item vs group. Layout does
  not change (highlight only), so the frozen 36 Owner-block goldens move only where a test
  selects an item.
- **D-05 [inferred]:** Key. One key, `x` ("settle"), in both hosts (free in the inbox; the Owner
  block consumes its own keys before the phase list's `x`). It opens a small chooser popup:
  `w` Won't do, `p` Deferred permanently, `m` Move to phase, `Esc` cancel. Then the reason field
  (and the phase picker for Move), then the confirm popup. Typical flow: `x w <reason> Enter y` or
  `x m <pick> Enter y`. Footer hint and the help overlay rows gain `x`; README "Owner inbox"
  section documents it.
- **D-06 [inferred]:** Move target picker: the project's phases from `ProjectState` (roadmap,
  current milestone and backlog `999.x`) that are not complete and not the source phase, default
  = the next phase after the source. Free entry is allowed when it passes `is_command_token`
  (same predicate as 37 D-11); an unknown phase is accepted with a warning in the confirm popup
  (GSD may create or reject it), never silently.

#### Command and instruction (OWN-30)
- **D-07 [user]:** Every outcome launches `/gsd-verify-work <phase token>` (`--ws <name>` for
  workstream groups) with the instruction appended as plain text in the same argv element:
  `claude "/gsd-verify-work 24 --ws foo <instruction>"`. Verify-work already handles "skip" and
  "deferred follow-up" answers; we state intent and let GSD record it. No design around missing
  gsd-core features, no upstream requests.
- **D-08 [inferred]:** Instruction templates (one pure, unit-tested builder; exact wording is a
  human UAT item):
  - Won't do (check): `The user will not perform test K ("<name>") in <file>: <reason>. Record
    this decision so it is no longer open and is not asked again.`
  - Deferred permanently (check): `The user defers test K ("<name>") in <file> permanently[: <reason>].
    Record it as deferred so it is not asked again.`
  - Move (check): `Defer test K ("<name>") in <file> as a follow-up for phase M[: <reason>]. Record
    the deferred follow-up here so phase N can close without it, and add a todo/backlog item for
    phase M so the check comes up again when phase M runs.`
  - Phase-level variants name all open checks of the entry (`tests 1, 3, 4 in 24-UAT.md and item 2
    in 24-VERIFICATION.md`) instead of one.
  Check identity = file basename + test number + name (name quoted, single line, capped ~80 chars,
  control characters stripped). VERIFICATION items without a number use their list position.
- **D-09 [user]:** For Move, the single launched command does both: records the deferral and adds
  the todo/backlog item for phase M. The app does not run `gsd-tools todo` or `/gsd-capture`
  itself. This narrows OWN-31's "`gsd-tools todo …` or `/gsd-capture`": filing still goes through
  gsd-core, by instruction.
- **D-10 [user]:** Confirm popup (reuse `confirm_popup.rs`/`ack_confirm.rs` patterns, default No)
  shows: project, target (check or phase with counts), outcome, the exact argv command line and
  the full instruction, wrapped. Cancel launches nothing. Instruction text renders through the
  existing `shown`/`fit_cells` helpers (`render_escape_guard` coverage, OWN-11).
- **D-11 [inferred]:** Live session present (37 D-12 path): focus it and copy the full command
  line to the clipboard through the existing `y` copy path (native tool or OSC 52); the status
  line says to paste it there. Nothing is injected into the session, no second session is
  started (two GSD sessions in one project could race on STATE.md).
- **D-12 [inferred]:** After launch the row stays until GSD's writes reach the watcher (37 D-10);
  status line: `Launched /gsd-verify-work N in <alias>: <outcome> for <target>`.

#### Read-only proof (OWN-31, SC5)
- **D-13 [inferred]:** Automated: (a) a source scan over the new modules and the chooser/confirm
  screens proving no filesystem write API is reachable for project paths (pattern of the OWN-02
  scan); (b) a flow test with a fake launcher that snapshots a fixture project tree (paths +
  hashes) before and after the full `x … y` flow and asserts it is byte-identical, plus argv and
  instruction table tests (each outcome × check/phase × workstream, refusals for FIX/DECIDE/
  archived/acknowledged, bad tokens, bad workstream names, over-long and control-character
  reasons). The "oracle round trip on a throwaway project" from the roadmap's Verification line
  needs a real Claude session (the instruction is interpreted by the model), so it is **human
  UAT**: won't-do, deferred-permanently and move round trips in the real multiplexer on a
  throwaway project, checking that GSD recorded each, that the moved check appears as a todo for
  phase M, and that the instruction wording reads right.

### Claude's Discretion
- Exact popup layout, widths and colors (UI-SPEC territory, reuse 36/37 tokens).
- Module split (e.g. a new `owner_settle.rs` next to `owner_actions.rs`) and how the reason text
  field is implemented (reuse an existing single-line input such as `SlugInputScreen`).
- Whether the chooser and reason field are one popup or two.

### Deferred Ideas (OUT OF SCOPE)
- Outcome actions on FIX rows (accept a gap as won't-fix) and on DECIDE rows (answer a context
  question) — new capabilities, not checks; possible future phase.
- Outcome actions from the `inbox` CLI (non-interactive) — only the TUI is in scope.
- Batch settling of several groups at once.

Reviewed todos: none folded (17 keyword matches, all generic). Musk pass: no cuts.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| OWN-29 | TUI actions on a check or a phase in the inbox and the detail Owner block: "won't do" (with a reason) and "defer/move to phase M"; `A` stays as is | Key arms in `inbox.rs::handle_key` and `detail.rs::handle_owner_block_key` (both read this session); item sub-cursor pattern (Pattern 2); refusal table mirroring `ack_target_for` (Pattern 1); `x` is free in both hosts |
| OWN-30 | Each action launches a GSD command with a prepared plain-language instruction; confirm popup shows exact command and instruction first | `owner_mux_argv` / `owner_terminal_argv` shape (one element after `env -u … claude`); verify-work argument parsing probed (Pitfalls 2-4); popup geometry (inner width max 70, Pitfall 7); `ack_confirm.rs` recheck pattern |
| OWN-31 | Defer/move files a todo for phase M through gsd-core; the app never edits a GSD file | gsd-tools 1.21.0 has no `todo add` verb, only `todo complete` / `todo match-phase` (Pitfall 5): filing is the model running gsd-core's `add-todo` workflow; match-phase keyword scoring tells the instruction what to name; source scan precedent `tests/async_blocking_guard.rs`; fake-launcher flow test with tree snapshot |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **GSD files are read-only** (PROJECT.md 2026-11-16): never write `.planning/**` or `$GSD_HOME`; every change through `gsd-tools` or a launched GSD command with the user's intent in plain language.
- **Non-intrusive**: must not interfere with running GSD instances (D-11: never a second session in a project with a live one; nothing injected).
- **Portability**: any GSD user; respect `CLAUDE_CONFIG_DIR`, `PATH`, `XDG_*` (no hardcoded `~/.claude`).
- **Stack**: Rust, ratatui 0.30, crossterm 0.29, tokio; MSRV `rust-version = "1.89"`, `edition = "2021"` [VERIFIED: Cargo.toml grep]. No new crates are needed for this phase.
- **Logs go to the tracing file**, never stdout.
- **Do not use `std::sync::Mutex` across `.await`**.
- Test fixtures contain no private project text (the crate is public).
- Global user rules: US spelling; conventional commits; path-scoped commits (the orchestrator commits here).
- Release process items (Cargo version bump, container pre-tag check) apply at milestone tag time, not to this phase.

## Summary

Everything the phase needs already exists as a pattern in Phase 37: a typed target with a refusal table (`ack_target_for`), a default-No confirm popup that re-checks on Yes (`AckConfirmScreen`), a launch seam that takes a closure so tests never touch a multiplexer (`run_group(ctx, alias, group, &mut launch)`), the one-element argv builders (`owner_mux_argv`, `owner_terminal_argv`), the live-session focus rule, and the clipboard path (`owner_actions::copy`). Phase 39 adds one pure module (target resolution, instruction builder, argv builders, refusals), a small chain of popups (chooser → reason / phase picker → confirm), an item sub-cursor in both hosts, and guards. No new dependency.

Two gsd-core facts change what the instruction must say, and both were verified by running the installed gsd-core 1.21.0 against a throwaway copy of `tests/fixtures/owner/uat-pending`:
1. **Only one kind of skip is non-blocking in gsd-core: `result: skipped` with a reason that starts `Deferred follow-up`.** A skip whose reason reads "Won't do: ..." is `skipped_unresolved`, blocks `phase uat-passed`, and the app's own reader keeps it in the TEST lane. So a won't-do recorded the "obvious" way is still asked again. The instruction has to name gsd-core's own deferral encoding for all three outcomes.
2. **verify-work's argument parse runs the whole argument string through `sed | xargs` and greps every `--ws NAME` in it.** Trailing text after the phase token is harmless to `init.verify-work` (phase still found), but an odd number of ASCII `'` or `"` makes `xargs` fail, `\` is eaten, and any `--ws x` inside a test name or reason **redirects the workstream** (`${GSD_WS##* }` takes the last match). The instruction builder must emit no ASCII `'` `"` `\` and no `--`.

A third gsd-core fact bounds OWN-31: gsd-tools 1.21.0 has no `todo add` verb, so "files a todo through gsd-core" can only mean the model running `/gsd-verify-work` follows gsd-core's add-todo procedure (as D-09 already decided). Phase M finds that todo only through `todo match-phase` keyword scoring, so the Move instruction must name phase M's number **and roadmap name**.

**Primary recommendation:** Build `src/ui/screens/owner_settle.rs` as the single pure owner of target resolution, refusals, sanitization, instruction text and argv (mirroring `ack_target_for`/`ack_args`), drive it through `run_group`-style closure seams, and phrase every outcome so GSD records `result: skipped` with a reason starting `Deferred follow-up:` and keeps it out of the 999.x promotion unless the outcome is Move.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| `x` key, item sub-cursor, highlight | TUI screens (`inbox.rs`, `detail.rs` Owner block, `owner_block.rs`) | — | View state only; identities, never indices (D-10/D-18 pattern) |
| Target resolution + refusals (lane/kind/scope/ack, token, workstream, active-workstream pointer) | Pure module `owner_settle.rs` | `state_reader::owner_items` predicates | Same split as `ack_target_for`: one function, unit-tested as a table |
| Instruction text + sanitization | Pure module `owner_settle.rs` | `crate::text` helpers | Model-facing text built from typed values; must be xargs-safe and `--ws`-safe |
| Argv (mux and GUI terminal) | `detail.rs` builders (new siblings of `owner_mux_argv`) | `multiplexer::ExecArgv` | Keeps the CWE-88 reasoning and pinned-arity tests in one file |
| Launch / focus / clipboard | `owner_actions.rs` (`launch_real`-style), `multiplexer::launch`, `owner_actions::copy` | App main loop (OSC 52) | Existing seams; tests inject a closure |
| Recording the decision, filing the todo | **External: gsd-core via the launched Claude session** | — | PROJECT.md read-only constraint; D-07, D-09 |
| Proof of no writes | `tests/` source scan + in-crate flow test | gsd-core oracle (optional read-only probes) | D-13 |

## Standard Stack

No new libraries. Everything is in-tree.

### Core (already in Cargo.toml)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30 | Popups, lines, spans | Project stack (CLAUDE.md) |
| crossterm | 0.29 | `KeyCode`, `KeyModifiers` | Project stack |
| tokio | 1.x | `spawn_blocking` for the native clipboard tool (existing `ctx.start_copy`) | Project stack |
| tempfile | (dev, existing) | Throwaway fixture copies in tests | Already used by `tests/owner_acknowledge_roundtrip.rs` |

### In-tree assets to reuse (do not re-implement)
| Asset | Location | Use |
|-------|----------|-----|
| `is_command_token`, `is_workstream_name` | `src/state_reader/owner_items/rules.rs:74-107` | Re-validate phase token and workstream at plan time and on Yes |
| `RunCommand` / `RunVerb::VerifyWork` / `Display` | `src/state_reader/owner_items/mod.rs:497-529` | The slash command spelling; never format `/gsd-verify-work` by hand |
| `owner_scrubbed_program()` | `src/ui/screens/detail.rs:3098-3106` | `env -u <ACK_ENV_REMOVE...> claude` prefix |
| `terminal_program_separator` | `src/ui/screens/detail.rs` | GUI terminal fallback separator |
| `multiplexer::launch`, `focus_message`, `FOCUS_OK_PREFIX` | `src/multiplexer/mod.rs:967-1060` | Launch or focus |
| `owner_actions::copy` + `force_clipboard`/`seam_ran` test seam | `src/ui/screens/owner_actions.rs:585-661` | D-11 clipboard |
| `active_workstream_is_set` | `src/ui/screens/owner_actions.rs:477-482` | Root-group refusal while a pointer is set (T-37-18 analog) |
| `confirm_popup::{ConfirmPopup, render_confirm_popup, body_room, dim_background, click_as_key}` | `src/ui/screens/confirm_popup.rs` | Default-No popup with clip guard |
| `ack_confirm::wrap_at_separators` (currently private `fn`) | `src/ui/screens/ack_confirm.rs:106-144` | Wrap the instruction and command; make it `pub(crate)` |
| `shown`, `fit_cells`, `owner_block::cells` | `src/ui/screens/detail.rs` | Every dynamic string in a cell |
| `crate::text::is_invisible_formatting_char`, `strip_terminal_controls` | `src/text.rs:190, 374` | Building blocks for the instruction sanitizer |
| `owner_block::groups_for_copy`, `entry_for`, `group_for` | `src/ui/screens/owner_block.rs:102-131, 658-667` | Resolve a row identity to its groups at key press and on Yes |

**Installation:** none.

## Package Legitimacy Audit

No external packages are installed by this phase. **Packages removed:** none. **Packages flagged:** none.

## Architecture Patterns

### System Architecture Diagram

```
 key x (plain, WR-01)
   │  inbox.rs handle_key / detail.rs handle_owner_block_key
   ▼
 cursor → (row identity, optional item id)          [Pattern 2 sub-cursor]
   │
   ▼
 owner_settle::settle_target(ctx, alias, row key, item?) ──Err(refusal)──► status line, nothing pushed
   │ Ok(SettleTarget)   (lane TEST/BLOCKED, current scope, not acknowledged,
   │                     token + workstream validated, root group refused if
   │                     active-workstream pointer set)
   ▼
 Push ChooserScreen  ── Esc ──► Pop, nothing
   │ w / p / m
   ▼
 Replace ReasonScreen (+ PhasePicker for m) ── Esc ──► Pop, nothing
   │ Enter (reason valid: required for Won't do)
   ▼
 owner_settle::settle_plan(target, outcome, reason, target_phase?) -> SettlePlan
   │   command = RunCommand{VerifyWork, phase, ws}  +  instruction (sanitized)
   ▼
 Replace SettleConfirmScreen (default No, shows project/target/outcome/command/instruction)
   │ n / Esc ──► Pop, nothing launched
   │ y (not clipped)
   ▼
 recheck: re-resolve target by identity; plan must be identical ──differs──► status, nothing
   │ same
   ▼
 launcher(&SettlePlan)   (real: launch_settle_real; tests: fake closure / fn pointer)
   ├─ live session in project → multiplexer::focus_message + owner_actions::copy(slash cmd + instruction)
   └─ else multiplexer::launch(plan, cwd, settle_mux_argv, settle_terminal_argv)
   ▼
 Pop + status "Launched /gsd-verify-work N in <alias>: <outcome> for <target>"
   ▼
 (external) claude session runs verify-work → writes UAT result / Deferred Follow-Ups,
            add-todo for Move → watcher → digest refresh → row leaves the lane (37 D-10)
```

### Recommended Project Structure
```
src/ui/screens/
├── owner_settle.rs     # NEW, pure: Outcome, SettleTarget, SettlePlan, settle_target(),
│                       #   settle_plan(), instruction(), sanitize_instruction_text(),
│                       #   move_candidates(), launch_settle_real(), settle_group()-style seam
├── settle_popup.rs     # NEW: SettleChooserScreen, SettleReasonScreen (with phase picker),
│                       #   SettleConfirmScreen (+ adjudicate_screen! for each)
├── owner_actions.rs    # unchanged API; maybe a shared host enum (reuse AckHost)
├── detail.rs           # + settle_mux_argv / settle_terminal_argv beside owner_mux_argv; x arm
├── inbox.rs            # + x arm, item sub-cursor, footer [x]
├── owner_block.rs      # + item sub-cursor helpers, item line indices
├── help.rs             # + x rows (Owner block row "v o A y x", Owner Inbox row "x")
└── render_escape_guard.rs  # + fixture rows for the three new screens
src/state_reader/owner_items/rules.rs  # + pub(crate) command phase helper (see Pitfall 6)
tests/owner_settle_guard.rs            # NEW: source scan (no write API, no Command::new)
tests/owner_settle_oracle.rs           # NEW (optional, oracle-gated): encoding + argument-parse probes
README.md                              # Owner inbox section: x
```

### Pattern 1: Typed target with a refusal table (mirror `ack_target_for`)
**What:** One function resolves (alias, row identity, optional item id) into a fully validated `SettleTarget` or a status string. Every value that reaches the instruction or argv is a field here.
**When:** On `x` (before any popup) and again on Yes.
**Shape** (field names are proposals [ASSUMED]; the predicates and types they wrap are real):
```rust
pub(crate) enum Outcome { WontDo, DeferPermanently, Move }

pub(crate) struct SettleTarget {
    pub alias: String,
    pub cwd: PathBuf,                 // project path, never in the argv
    pub command: RunCommand,          // RunVerb::VerifyWork, phase token re-validated, ws re-validated
    pub phase_label: String,          // display only (roadmap number)
    pub checks: Vec<CheckRef>,        // one for an item target, all open checks for a phase target
    pub scope: TargetScope,           // Check(item id) | Phase
    pub key: InboxKey,                // identity for the Yes recheck (T-37-20 analog)
    pub live_session: Option<ClaudeSession>,
}
pub(crate) struct CheckRef { pub file_basename: String, pub number: String, pub name: String }
```
Refusals (status text, nothing pushed): lane FIX / DECIDE / Deferred; `Scope::Archived`; `acknowledged`; `counts.unreadable > 0`; phase token fails `is_command_token`; workstream fails `is_workstream_name`; root-scoped group while `active_workstream_is_set(project)`; unknown alias; fold rows other than Stale (Old debt, Acknowledged); the fold line itself.

### Pattern 2: Item sub-cursor (do not change the `Stop` / `InboxStop` enums)
**What:** Add `item_cursor: Option<String>` (an `OwnerItem::id`) to both `InboxViewState` and `OwnerBlockState`, exactly like the existing `fold_cursor` sub-cursor. `cursor` keeps naming the row, so `v`, `o`, `A`, `y`, `g` keep acting on the parent group without touching their ~25 `Stop::Row` / `InboxStop::Row` match sites (D-04).
**Movement:** `j`/`Down` on an expanded row with shown items enters the first item; on an item, moves to the next shown item, then past the last item to the next row (clearing `item_cursor`). `k` mirrors it (moving up into the previous row lands on its last shown item if that row is expanded). PgUp/PgDn/Home/End/`g`/`G` move rows and clear `item_cursor`. Enter on an item collapses the parent and clears `item_cursor`. A vanished item id (refresh) clears `item_cursor` (stay on the row).
**Shown items only:** `owner_block::item_lines` draws at most `MAX_ITEMS = 5` items per list (`owner_block.rs:49`, `:1182`) [VERIFIED: owner_block.rs:48-49 `const MAX_ITEMS: usize = 5;`], and in the inbox a merged row draws each part's items separately (`inbox_expansion_lines`, per part `take(MAX_ITEMS)`). The sub-cursor walks exactly the item ids the renderer drew. Have the line builders return `(Vec<Line>, Vec<(usize, String)>)` (line index → item id) instead of re-deriving the list, so the highlight and the walk cannot disagree.
**Highlight only:** the item's first line gets the cursor style; layout unchanged (D-04, frozen 36 goldens).

### Pattern 3: Closure launch seam (mirror `run_group`)
```rust
// Source: src/ui/screens/owner_actions.rs:133-144 (run_group), adapted
pub(crate) fn settle_launch(
    plan: &SettlePlan,
    launch: &mut dyn FnMut(&SettlePlan) -> String,
) -> String { launch(plan) }
```
`SettleConfirmScreen` holds `launcher: Option<fn(&SettlePlan) -> SettleOutcome>` like `AckConfirmScreen::runner` (`ack_confirm.rs:47`); `None` = real. Every test injects one, so no test reaches tmux/herdr/a GUI terminal. The real function is called only from the confirm screen's Yes.

### Pattern 4: Popup chain with `Replace`
Chooser `Push`es; chooser → reason uses `ScreenAction::Replace`; reason → confirm uses `Replace`; confirm Yes/No/Esc `Pop`s back to the host. WR-03 (`src/ui/screens/mod.rs:333-353`): a confirm that pops onto the step before it can be re-submitted; `Replace` returns the owner straight to the host.

### Pattern 5: Argv builders beside the existing ones
```rust
// Source shape: src/ui/screens/detail.rs:3085-3122 (read this session)
pub(crate) fn settle_mux_argv(plan: &SettlePlan)
    -> Result<crate::multiplexer::ExecArgv, crate::multiplexer::MuxError> {
    let mut argv = owner_scrubbed_program();      // env -u ... claude (authored)
    argv.push(plan.prompt());                      // ONE element: "{command} {instruction}"
    crate::multiplexer::ExecArgv::new(argv)
}
pub(crate) fn settle_terminal_argv(term: &str, plan: &SettlePlan) -> Vec<String> {
    let mut argv = vec![terminal_program_separator(term).to_string()];
    argv.extend(owner_scrubbed_program());
    argv.push(plan.prompt());
    argv
}
```
`plan.prompt()` = `format!("{} {}", plan.command, plan.instruction)` where `plan.command` is `RunCommand`'s `Display` (so `--ws NAME` precedes the instruction, the D-07 shape). The element starts with `/`, so `claude` cannot read it as an option. herdr carries argv as a JSON array (`herdr.rs:114-133`) and tmux execs multi-element argv without `sh -c`, so spaces and punctuation are safe in transit [VERIFIED: src/multiplexer/herdr.rs:120-131 `"command": argv.as_slice()`].
Pin it with a test like `owner_terminal_argv_carries_no_untrusted_element_and_is_pinned_at_three` (`detail.rs:24646`).

### Anti-Patterns to Avoid
- **Reusing `run_plan` for settle:** it refuses BLOCKED (`"No command for BLOCKED work: unblock it first"`, `owner_actions.rs:65-67`), and BLOCKED groups have `run: None` (`rules.rs:138`). Settle must build its own `RunCommand { verb: RunVerb::VerifyWork, .. }`.
- **Formatting `/gsd-verify-work` by hand:** only `RunCommand::fmt` spells slash commands (`mod.rs:504-529`).
- **Injecting into a live session** (`send-keys` and the like): D-11, T-37-07.
- **Building text from display strings** (`group.command`, row cells): build from typed fields only.
- **Rendering raw instruction text in the popup and assuming it equals the argv:** sanitize first so `shown(instruction) == instruction` (Pitfall 9).
- **Running `gsd-tools` from the product for this phase:** D-09; also keep `ACK_VERB` the only gsd-tools verb in `src/` (existing guard `tests/owner_open_conformance.rs`).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Opening a session where the owner is | Own tmux/herdr calls | `multiplexer::launch` / `launch_with` | Innermost-backend rule, GUI fallback, status phrasing, logged reasons |
| Focusing a live session | Pane search | `multiplexer::focus_message` + `FOCUS_OK_PREFIX` | 37 D-12 path, exact-cwd match (T-37-06) |
| Clipboard | `wl-copy` spawn | `owner_actions::copy` | Native tool on `spawn_blocking`, OSC 52 fallback, busy rule, 64 KiB cap |
| Confirm popup | New modal | `confirm_popup` | Default No, clip guard (`CLIPPED_HINT`), mouse-click mapping |
| Wrapping long commands | Char-based wrap | `wrap_at_separators` | Breaks at `/` and spaces, hanging indent |
| Escaping | Ad hoc replace | `shown` / `fit_cells` / `render_for_terminal` | OWN-11, census probe |
| Slash-command spelling | String literal | `RunCommand` `Display` | D-11 single spelling |
| Recording outcomes / filing todos | Writing UAT.md or `.planning/todos/` | The launched `/gsd-verify-work` | PROJECT.md constraint, D-09 |

**Key insight:** the risky parts here are not rendering; they are the two parsers on the far side of the argv (gsd-core's bash argument parse and the model). The builder must produce text that both read correctly, and the tests must pin that text.

## Common Pitfalls

### Pitfall 1: A "won't do" recorded the obvious way is still open (and blocks the phase)
**What goes wrong:** verify-work's skip branch writes `result: skipped` / `reason: <user's reason>`. gsd-core categorizes any skip whose reason does not start with `Deferred follow-up` as `skipped_unresolved` (or `device_needed`/`server_blocked`/`build_needed` by keyword), which blocks `phase uat-passed`, and the app's reader puts it back in the TEST (or BLOCKED) lane. D-08's "so it is no longer open and is not asked again" fails.
**Evidence:** gsd-core `categorizeItem` [VERIFIED: ~/.claude/gsd-core/bin/lib/uat.cjs:3996-4013 `if (reason && /^["']?deferred follow-up\b/i.test(reason)) return 'deferred';` … `return 'skipped_unresolved';`]; the app's port [VERIFIED: src/state_reader/owner_items/uat.rs:75-136 `rest.strip_prefix("deferred follow-up")` … `return Category::SkippedUnresolved;`]; lane rule `Category::Pending | Category::HumanUat | Category::SkippedUnresolved => Placement::Lane(Lane::Test)` (`rules.rs:51-53`). Probe on a throwaway copy of `uat-pending` with gsd-core 1.21.0:
```
2 skipped skipped_unresolved     # reason: "Won't do: the owner will not perform this check"
3 skipped deferred               # reason: "Deferred follow-up: won't do (owner decision)"
4 skipped device_needed          # reason: needs a physical handset ...
phase uat-passed 1 --uat-only → test 2 "passing": false; test 3 "passing": true, "deferred": true
```
**How to avoid:** every outcome's instruction tells GSD to record `result: skipped` with a reason that **begins with `Deferred follow-up:`**, followed by the outcome and reason, for example `Deferred follow-up: will not do. <reason>` / `Deferred follow-up: deferred permanently. <reason>` / `Deferred follow-up: moved to phase M. <reason>`. This uses gsd-core's own existing encoding (no upstream change, D-07 holds). Write the prefix without surrounding quotes in the instruction: the reader and gsd-core only tolerate an ASCII `"`/`'` before it, not a typographic one.
**Warning signs:** after a round trip the row is still in TEST with `skipped`, or moved to BLOCKED.

### Pitfall 2: `xargs` in verify-work's argument parse breaks on ASCII quotes and backslashes
**What goes wrong:** verify-work runs `PHASE_ARG=$(echo "$GSD_ARGS" | sed -E 's/--ws[[:space:]]+[A-Za-z0-9._-]+//g' | xargs)` [VERIFIED: ~/.claude/gsd-core/workflows/verify-work.md:47-51]. An odd number of `'` or `"` makes xargs print `xargs: unmatched single quote; by default quotes are special to xargs unless you use the -0 option` and stop; `\` is dropped.
**Probe output (this session):**
```
IN : 24 The user won't perform test 2
OUT: [xargs: unmatched single quote; by default quotes are special to xargs unless you use the -0 option
24 The user]
IN : 24 --ws foo Defer test 2 ("a \"b\"") as a follow-up for phase 25
OUT: [xargs: unmatched double quote; ... 24 Defer test 2]
```
Typographic quotes pass through untouched: `(“Saving a note”)` survived the same pipeline.
**How to avoid:** the sanitizer maps `'` → `’`, `"` → `”`/`“` (or drops them), `\` → `/` or drops it, in the reason, the item name, the phase name and the file name; the templates use `“…”` around names and `will not` instead of `won't`. Unit-test: no output contains `'`, `"`, `\`.
**Warning signs:** an xargs warning at the top of the launched session.

### Pitfall 3: `--ws` inside free text hijacks the workstream
**What goes wrong:** `GSD_WS=$(echo "$GSD_ARGS" | grep -oE -- '--ws[[:space:]]+[A-Za-z0-9._-]+')` returns every match, and `--ws=${GSD_WS##* }` uses the last word, so a test name or reason containing `--ws evil` sends the session to workstream `evil`.
**Probe output:** `24 --ws alpha Test 2 (“x --ws evil”) …` → matches `--ws alpha`, `--ws evil` → `resolved ws: evil`.
**How to avoid:** the sanitizer collapses any run of two or more `-` to one `-` (or maps `--` to `–`) in every free-text part. Also keep the real `--ws NAME` before the instruction (`RunCommand::Display` order). Unit-test with a hostile name `x --ws evil` and assert `prompt().matches("--ws").count() <= 1`.

### Pitfall 4: Trailing text after the phase token is fine for init, but keep the token first
**Verified:** `gsd-tools query init.verify-work "1 The user will not perform test 2 (\"Login\") in 01-UAT.md: no device. Record this decision."` on the throwaway project returned `"phase_found": true`, `"phase_number": "01"` (same as with plain `1`). So D-07's "instruction in the same argv element" works for init. Keep the phase token as the first word after the command and never start the instruction with a digit-dot token (a number like `3.` right after the phase could be read as part of the phase argument by the model).

### Pitfall 5: There is no `gsd-tools todo add`; phase M finds the todo only by keyword
**Facts:** gsd-tools 1.21.0 command aliases expose `todo.complete` and `todo.match-phase` only [VERIFIED: ~/.claude/gsd-core/bin/lib/command-aliases.cjs:882-891]. A todo is created by the `add-todo` workflow (reached via `/gsd-capture`), which writes `.planning/todos/pending/${date}-${slug}.md` with `title:`/`area:`/`files:` frontmatter and commits it [CITED: ~/.claude/gsd-core/workflows/add-todo.md:128-162]. Phase M's `/gsd-discuss-phase` calls `todo.match-phase "${PHASE_NUMBER}"` [CITED: workflows/discuss-phase.md:275], which scores todos by keywords from phase M's roadmap name + goal + section (0.2 per matched word, max 0.6), `area` (+0.3) and plan `files` (+0.4); words of 2 characters or fewer and stop words are ignored [VERIFIED: bin/lib/commands.cjs:3271-3365]. A todo that says only "phase 25" scores 0.2 at most ("25" is too short).
**How to avoid:** the Move instruction names phase M by number **and** roadmap name (sanitized), and asks for the todo title to name phase M's name and the check, for example `add a todo (as /gsd-capture does) titled for phase 25 “Inbox digest and triage hygiene”: rerun check 3 “Saving a note” from phase 24`. Optionally assert in an oracle test that a todo with that title scores ≥ 0.4 for phase M (the auto-fold threshold the 39 discuss step used).

### Pitfall 6: BLOCKED rows carry no command and the phase token is the directory token
**What goes wrong:** `run_for` returns `None` for `Lane::Blocked` [VERIFIED: rules.rs:135-139 `Lane::Decide | Lane::Blocked | Lane::Deferred => return None,`], and `OwnerGroup::phase` is the file's directory token while TEST commands use the roadmap number (`phase_labels(...).1`, `rules.rs:369-377`).
**How to avoid:** for TEST rows use the group's own `run` (re-validated); for BLOCKED rows build `RunCommand { verb: RunVerb::VerifyWork, phase: Some(<roadmap number or dir token>), workstream }`. Expose one `pub(crate)` helper in `rules.rs` (the private `scope_state` + `roadmap_phase` lookup that `phase_labels` already does) so both paths agree. Either spelling works for gsd-core (pad-insensitive lookup), but the status line and the popup should match what `v` shows.

### Pitfall 7: The confirm popup can clip at 80x24, and a clipped popup cannot be confirmed
**Facts:** popup width = `area.width.saturating_sub(4).clamp(1, 72)`; `body_room` = `(width - 2, area.height - 5)` [VERIFIED: confirm_popup.rs:169-171, 303-307]. At 80x24: 70 columns x 19 body rows. A clipped popup refuses `y` (`ack_confirm` test `the_popup_clips_at_60x20_and_cannot_confirm`).
**Budget:** Project, Target, Outcome (3-4 rows) + blank + command (1-2) + blank + instruction (≈ chars / 70) + blank + one-line consequence. The scrubbed argv (`env -u` x15 + `claude`) alone is about 300 characters, about 5 rows.
**How to avoid:** cap the instruction (recommend ≤ 700 chars; phase-level lists numbers only, not names, and after ~20 checks says `and N more in <file>`), show the command line as `claude “<prompt>”` plus one authored line `(started with env, GSD session variables removed)` [ASSUMED: acceptable reading of D-10 "exact argv command line"; flag in UI-SPEC], and test that the popup is unclipped at 80x24 for the longest phase-level case and at 100x30.

### Pitfall 8: One `ScreenAction` per key, but the live-session path needs Pop **and** OSC 52
**What goes wrong:** `owner_actions::copy` returns `ScreenAction::WriteOsc52(..)` when no native tool exists (`owner_actions.rs:591-598`), while the confirm screen must also `Pop`. `ScreenAction` has no combined variant (`mod.rs:330-376`).
**How to avoid (pick one in the plan):** add a narrow variant (`PopThen(Box<ScreenAction>)` handled by `App::process_screen_action`), or an `AppContext` mailbox for a pending OSC 52 drained after the key. Test both clipboard branches with `force_clipboard`.
**Also:** the native copy's completion overwrites the status line with `Copied {n} via {tool}` (`mod.rs:2008-2012`), so the "focused, paste it there" hint must survive (compose the hint into the copy outcome, or set it after). And copy **the slash command plus instruction**, not `claude …`: the owner pastes it into an already running Claude session.

### Pitfall 9: Invisible and control characters make the popup lie
**What goes wrong:** `shown()` renders bidi overrides and C0/C1 visibly escaped, but the argv would carry them raw, so what the owner consents to differs from what runs (Trojan-source shape). A newline would also make a multi-line `$ARGUMENTS`.
**How to avoid:** the sanitizer drops C0/C1/DEL and every `is_invisible_formatting_char` character, collapses whitespace to single spaces, trims, then applies Pitfalls 2-3 mappings and the caps (reason 200 chars D-02, name ~80 D-08). Assert in a property-style test that `shown(&instruction) == instruction` for hostile inputs (the 37 hostile string `x\u{1b}[2J\u{7}\u{202e}y\u{200b}z\u{85}` is the house fixture).

### Pitfall 10: Root-scoped groups and the active-workstream pointer
gsd-core resolves the workstream as `--ws`, then `GSD_WORKSTREAM`, then a session or shared pointer (`ACK_ENV_REMOVE` doc, `owner_actions.rs:239-249`). The launch scrubs the env, but `.planning/active-workstream` still redirects a root-scoped `/gsd-verify-work N` into a workstream. `A` refuses this case (`ack_target_for`, T-37-18); `v` does not. Because settle records a decision, apply `A`'s refusal: root group + `active_workstream_is_set(project)` → `Cannot settle <name>: <alias> has an active workstream set; run it in the project` [ASSUMED wording].

### Pitfall 11: VERIFICATION human checks are not cleared by a deferral alone
verify-work "never rewrites VERIFICATION.md itself (its only write is the canonicalization)", and canonicalization flips `human_needed` → `passed` only when `phase uat-passed --uat-only` passes, which needs at least one passing UAT check and no blocking rows [CITED: workflows/verify-work.md:642-677]. A phase-level outcome on an entry whose only open checks are VERIFICATION human items may leave the VERIFICATION rows listed. Not designed around (D-07); make it an explicit human-UAT check and see Open Question 1.

### Pitfall 12: verify-work's own prompts
- With an existing UAT session for the phase, verify-work "offer[s] to resume or restart" (`verify-work.md:150-153`); the instruction should say "resume the existing UAT session for this phase" so the owner is not asked.
- At session end, a non-empty `## Deferred Follow-Ups` triggers `[P] Promote to ROADMAP.md 999.x backlog / [K] Keep` (`verify-work.md:543-577`). Won't do and Deferred permanently should pre-answer **K** (don't create backlog phases for things nobody will do); Move should say "file a todo for phase M; do not promote to the 999.x backlog" unless phase M is itself a 999.x backlog phase.
- The verify-work keyword branches (blocked on "physical device", "server", …; deferred on "later", "defer", …; `verify-work.md:411-465`) are heuristics for conversational answers. The instruction must name the result to write explicitly so a reason like "no physical device" does not route to `result: blocked`.

### Pitfall 13: Owner block rows are per file; inbox rows are merged
The Owner block draws `current_sources` (one row per group, UAT and VERIFICATION separate), the inbox draws `inbox_sources` (same-phase UAT+VERIFICATION merged, D-08). A "phase" target therefore means "all open checks behind this row": the merged entry's parts in the inbox, the one group in the Owner block, every BLOCKED group of `(workstream, phase)` for an aggregate row (`groups_for_copy`). Say so in the popup's Target line (`phase 24: 5 checks in 24-UAT.md, 2 in 24-VERIFICATION.md`).

### Pitfall 14: Hidden items beyond `… N more`
Only the first five items of each list are drawn and can be selected individually. A sixth check can only be settled through the phase-level action (or after another check leaves the list). Acceptable; mention it in help text only if UI-SPEC wants it.

## Code Examples

### Instruction builder and sanitizer (sketch)
```rust
// Proposal [ASSUMED names]; predicates/types referenced are real (read this session).
fn clean(text: &str, cap: usize) -> String {
    let mut out = String::new();
    let mut last_space = true;
    let mut last_dash = false;
    for ch in text.chars() {
        if (ch as u32) < 0x20 || ('\u{7f}'..='\u{9f}').contains(&ch)
            || crate::text::is_invisible_formatting_char(ch) { continue; }
        let ch = match ch { '\'' => '\u{2019}', '"' => '\u{201d}', '\\' => '/', c => c };
        if ch.is_whitespace() {
            if !last_space { out.push(' '); }
            last_space = true; last_dash = false; continue;
        }
        if ch == '-' && last_dash { continue; }      // "--ws" can never form (Pitfall 3)
        last_dash = ch == '-'; last_space = false;
        out.push(ch);
    }
    let out = out.trim().to_owned();
    if out.chars().count() > cap { out.chars().take(cap).collect::<String>() + "\u{2026}" } else { out }
}

// Won't do, one check (wording is human UAT; prefix is gsd-core's contract, Pitfall 1)
format!(
    "The user will not perform test {k} (\u{201c}{name}\u{201d}) in {file}: {reason}. \
     Resume this phase\u{2019}s UAT session and record test {k} as result skipped with a reason \
     that begins Deferred follow-up: will not do. {reason} \u{2014} so it no longer blocks the \
     phase and is not asked again. Keep it in the UAT file; do not promote it to the backlog."
)
```

### Fake-launcher flow test with a tree snapshot (D-13b)
```rust
// In-crate test (needs pub(crate) items and ctx helpers):
// 1. copy tests/fixtures/owner/<fixture>/.planning into a TempDir
// 2. ctx = tests::ctx_with_aliases(&["alder-api"]); point projects["alder-api"].path at the TempDir;
//    ctx.project_states / owner_digests from parse_project_state + read_owner_digest on that copy
// 3. snapshot: walk the TempDir, collect (relative path, sha256 or bytes) into a BTreeMap
// 4. drive keys on InboxScreen: j.. Enter j x w <reason chars> Enter y, with the confirm screen's
//    launcher = fn pointer that records the SettlePlan (static AtomicUsize / thread_local)
// 5. assert recorded plan.prompt() == expected; snapshot after == snapshot before
// 6. repeat with Esc / n at each step: launcher count stays 0
```

### Source scan (D-13a)
Model on `tests/async_blocking_guard.rs` (walks `src/`, skips in-source `#[cfg(test)]` modules, lexical markers). Scan `src/ui/screens/owner_settle.rs` and `src/ui/screens/settle_popup.rs` (and the new arms if split out) for `fs::write(`, `File::create(`, `OpenOptions`, `create_dir`, `fs::rename(`, `remove_file(`, `remove_dir`, `fs::copy(`, `set_permissions(`, `.write_all(`, `Command::new(`, `.spawn()`, `.output()`, `.status()`. Include a planted-line test proving each marker is caught (the `the_narrowed_verb_guard_still_catches_the_read_verb` pattern) and an anti-vacuity count of scanned executable lines.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Any `skipped` with a reason counted as handled | Only `Deferred follow-up` skips are non-blocking (`DEFERRED_REASON_RE`, gsd-core #4546) | gsd-core ≤ 1.21.0 | Instruction must request that exact prefix |
| Config-tab native writes | `gsd-tools config-set` (quick 261116-kft) | 2026-11-16 | Same read-only stance this phase extends |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Claude Code passes all text after `/gsd-verify-work` as `$ARGUMENTS` when the slash command is the initial prompt argument of `claude` | Pattern 5 | Instruction ignored; caught in human UAT (the `v` path already relies on the command part) |
| A2 | The model running verify-work follows an explicit "record as skipped with reason beginning Deferred follow-up:" instruction rather than its keyword heuristics | Pitfall 1, 12 | Row stays open; human UAT detects; reword |
| A3 | Showing `claude “<prompt>”` plus an authored "env, session variables removed" line satisfies D-10 "exact argv" | Pitfall 7 | If the owner wants the full `env -u …` list, the popup needs ~5 more rows (fits 100x30, tight at 80x24) |
| A4 | Proposed type/field/function names (`SettleTarget`, `settle_mux_argv`, `item_cursor`, …) | Patterns | None functional; planner may rename |
| A5 | Refusing root groups while `active-workstream` is set is wanted for settle (it is for `A`, not for `v`) | Pitfall 10 | Slight friction; safer than a misdirected record |
| A6 | Instruction cap ≈ 700 chars and ~20 listed checks keeps the popup unclipped at 80x24 | Pitfall 7 | Measure in a render test; adjust caps |

## Open Questions

1. **VERIFICATION-only phase targets (Pitfall 11).**
   - Known: verify-work does not edit VERIFICATION.md except canonicalization, which needs a passing UAT check.
   - Unclear: whether a won't-do/defer of every human check of a VERIFICATION-only entry clears those rows.
   - Recommendation: keep the outcome available (D-03), add a human-UAT step on a fixture with a VERIFICATION `human_needed` file, and if rows persist, the popup should say so (`A` remains the file-level answer). Do not design around it in code.
2. **Exact instruction wording** is human UAT (D-08). The prefix `Deferred follow-up:` and the absence of ASCII quotes/backslashes/`--` are not wording choices; pin them in tests.
3. **Clipboard + Pop mechanism (Pitfall 8)** is an App-level change either way; the planner picks the narrower one.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain | build/test | ✓ | rustc 1.99.1 (MSRV 1.89) | — |
| node | gsd-core oracle tests | ✓ | v26.13.1 | `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` opt-out exists |
| gsd-core | oracle tests, human UAT | ✓ | 1.21.0 (`GSD_CORE_SYNCED_VERSION = "1.21.0"`, `src/state_reader/config_json.rs:32`) | `scripts/install-conformance-oracle.sh` |
| tmux | human UAT | ✓ | /usr/bin/tmux | herdr |
| herdr | human UAT (owner's multiplexer) | ✓ | ~/.local/bin/herdr | tmux |
| docker / podman | container pre-tag check | ✓ | 29.1.3 / 5.7.0 | — |
| cargo-nextest | optional | ✗ | — | `cargo test` |

**Missing dependencies with no fallback:** none.

## Test Plan Inputs

(`workflow.nyquist_validation` is `false`, so no Validation Architecture section; these are inputs for the planner's verification steps.)

- Framework: `cargo test` (unit tests in-module, integration tests in `tests/`); quick: `cargo test owner_settle`; full: `cargo test --no-fail-fast`; plus `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`.
- Table tests in `owner_settle.rs`: 3 outcomes × {check, phase} × {root, workstream} → exact `prompt()`; refusals (FIX, DECIDE, Old debt, Acknowledged, unreadable, bad token `1;rm`, fullwidth digits, bad workstream `a b`, `../x`, `-h`, `.`, root + active pointer); reason required for Won't do; reason over 200 chars capped; hostile reason/name (ESC, bidi, `'`, `"`, `\`, `--ws evil`, newline) → sanitized; `shown(instr) == instr`; no `'` `"` `\` and at most one `--ws`.
- Argv pin tests: `settle_mux_argv` = `owner_scrubbed_program()` + exactly one element; terminal argv for each separator row.
- Screen tests: Esc/n at chooser, reason, picker, confirm → launcher count 0; Enter defaults to No; clipped popup at 60x20 refuses `y`; unclipped at 80x24 with the longest case; Yes after a refresh that removed the item → `nothing launched`; live session → focus + copy (both clipboard branches) and no launch.
- Cursor tests: item stops only inside expanded rows; `v o A y` on an item act on the parent (compare with the row result); vanished item clears the sub-cursor; Owner block goldens unchanged when no item is selected.
- `render_escape_guard.rs`: fixture rows for the three new screens (hostile alias, file, name, phase name, reason).
- `tests/owner_settle_guard.rs`: source scan (D-13a).
- Flow test with tree snapshot (D-13b).
- Optional oracle test (`tests/owner_settle_oracle.rs`, same fail-loud policy and opt-out as `owner_acknowledge_roundtrip.rs`): on a temp fixture copy, (1) `query init.verify-work "<phase> <instruction>"` built by the product finds the phase; (2) a UAT row rewritten in the encoding the instruction requests is `deferred` for `audit-uat` and passing for `phase uat-passed --uat-only`, and the app's `read_owner_digest` no longer lists it. The test may write into the temp copy (it is test code standing in for GSD), never into the repo.
- Human UAT (D-13): won't-do, deferred-permanently and move round trips in herdr/tmux on a throwaway project; row leaves the inbox after the watcher refresh; Move's todo appears in `.planning/todos/pending/` and `gsd-tools query todo.match-phase M` lists it; no xargs warning at session start; wording reads right.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | local tool |
| V5 Input Validation | yes | `is_command_token`, `is_workstream_name`, instruction sanitizer (controls, invisible formatting, `'` `"` `\`, `--`, caps) |
| V6 Cryptography | no | — |
| V12 Files and Resources | yes | no write API in new code (source scan); cwd is the registered project path |
| V14 Configuration | yes | env scrub (`ACK_ENV_REMOVE`) on the launched session |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Argument injection into `claude` (CWE-88) | Elevation | One argv element starting with `/`; `ExecArgv::new` head/arity checks; pinned argv tests |
| Workstream redirection via `--ws` in free text | Tampering | Sanitizer collapses `--`; real `--ws` first; test with `--ws evil` |
| Active-workstream pointer redirecting a root record | Tampering | Refuse root groups while the pointer is set (T-37-18 analog) |
| Prompt injection through project-authored test names | Tampering | Name capped ~80 chars, single line, quoted; owner sees the full text in the default-No popup before consenting |
| Terminal escape / bidi in popup or status | Tampering / Spoofing | `shown`/`fit_cells`; sanitize so popup text == argv text; census fixtures |
| Popup shows one thing, Yes runs another (TOCTOU) | Tampering | Recheck on Yes by identity; plan must be identical (T-37-20 pattern) |
| Second GSD session racing on STATE.md | DoS / Tampering | Live session → focus + clipboard only (D-11, T-37-07) |
| Native write into the project | Tampering | Source scan + tree-snapshot flow test (D-13) |

## Sources

### Primary (HIGH confidence, read or run this session)
- Codebase: `src/ui/screens/owner_actions.rs`, `ack_confirm.rs`, `inbox.rs`, `owner_block.rs`, `detail.rs` (owner keys, argv builders), `confirm_popup.rs`, `driver_launch.rs` (`SlugInputScreen`), `help.rs`, `render_escape_guard.rs`, `src/state_reader/owner_items/{mod,rules,uat,triage}.rs`, `src/multiplexer/{mod,herdr}.rs`, `src/text.rs`, `tests/owner_acknowledge_roundtrip.rs`, `tests/owner_open_conformance.rs`, `tests/async_blocking_guard.rs`
- gsd-core 1.21.0 install (`~/.claude/gsd-core`): `workflows/verify-work.md` (arg parse 39-52, skip/blocked/deferred 411-465, completion and promotion 520-577, canonicalization 642-677), `bin/lib/uat.cjs` `categorizeItem`, `bin/lib/uat-predicate.cjs` `DEFERRED_REASON_RE`, `bin/lib/commands.cjs` `cmdTodoMatchPhase`, `bin/lib/command-aliases.cjs`, `bin/lib/init.cjs` `cmdInitVerifyWork`, `commands/gsd/capture.md`, `workflows/add-todo.md`, `skills/gsd-verify-work/SKILL.md`
- Probes on a throwaway copy of `tests/fixtures/owner/uat-pending` in the session scratchpad: `query init.verify-work` with appended text, `query audit-uat --raw`, `phase uat-passed 1 --uat-only`, the verify-work `sed | xargs` and `grep -oE --ws` pipelines

### Secondary
- `.planning/phases/37-inbox-screen-and-actions/37-SECURITY.md` threat register (T-37-04, -06, -07, -18, -20, -23)
- `.planning/PROJECT.md` "GSD files are read-only"

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, nothing new, all reused assets read.
- Architecture: HIGH, every pattern has a Phase 37 precedent in the tree.
- gsd-core behavior (encoding, argument parse, todo verbs): HIGH, source read and probed.
- Model behavior on the instruction: MEDIUM, human UAT.

**Research date:** 2026-11-17
**Valid until:** the next gsd-core sync (re-check `verify-work.md` argument parse, `DEFERRED_REASON_RE`, and todo verbs on any `GSD_CORE_SYNCED_VERSION` bump), else 30 days.
