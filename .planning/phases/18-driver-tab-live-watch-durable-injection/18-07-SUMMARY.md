---
phase: 18-driver-tab-live-watch-durable-injection
plan: 07
subsystem: ui
tags: [rust, ratatui, text-input, wizard, injection, command-picker, goal, sanitiser]

requires:
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    provides: "`DriverConfirmScreen`, `DEFAULT_DRIVE_COMMAND`, `do_start_run`'s dispatch model, and `App::start_driver_run`'s `goal: Option<&str>` parameter"
  - phase: 18-driver-tab-live-watch-durable-injection
    plan: 01
    provides: "`journal::inbox::new_message_id` — the client-generated correlation id (D-05)"
  - phase: 18-driver-tab-live-watch-durable-injection
    plan: 04
    provides: "`Action::DriverInjectRequested`, the widened `Action::DriverStartRequested { goal }`, and `sanitize_render_line`"
provides:
  - "`DriverInjectScreen` — the injection input (Surface 4), whose key handler contains no filesystem API at all"
  - "`driver_inject::no_live_run_message` — the caller's exact refusal copy, formatted where the screen lives"
  - "`DriverStartScreen` / `StartStep` — the two-step command-then-goal wizard (Surface 5)"
  - "`DriverConfirmScreen::new_start(alias, command, goal)` and the command- and goal-aware Start prompt"
  - "`driver_confirm::dispatch` as `pub(super)`, and `driver_confirm::tests::{ctx_with_project, ALIAS}` as `pub(crate)` — one dispatch mechanism and one full-field `AppContext` fixture for all three driver screens"
affects: [18-09, 18-10]

tech-stack:
  added: []
  patterns:
    - "A multi-field wizard built from the single shared `ctx.input_buffer`: the committed field lives on the screen struct, the buffer serves only the active field, and no `AppContext` field is added"
    - "One value feeds both the rendered confirmation and the dispatched action, so a prompt that names a different command from the one it starts is not expressible"
    - "Footer copy extracted to named `const`s so its *absences* (no `[Tab] suggestions`) are assertable without rendering a frame"
    - "Structural guarantees checked from two directions: a grep asserts the key handler names no filesystem API and no completion key; a test asserts the resulting behaviour"

key-files:
  created:
    - src/ui/screens/driver_inject.rs
    - src/ui/screens/driver_start.rs
  modified:
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/mod.rs

key-decisions:
  - "`DriverAction` stays a `Copy` unit-variant enum; the chosen command and goal live on `DriverConfirmScreen` instead — this keeps `normal.rs` (18-06's file, in a sibling worktree) compiling untouched while giving the same single-source guarantee"
  - "The interpolated **command** is sanitised as well as the goal: picker suggestions can come from a `gsd-tools smart-entry` subprocess, so a command string is not necessarily something the user typed"
  - "Sanitising is strictly a rendering concern — the dispatched `command` and `goal` are verbatim, because one is an argv value and the other is `RunRecord.goal`"
  - "The module doc names no filesystem API even in prose, so the plan's grep criterion is non-vacuous rather than defeated by a comment explaining the rule"
  - "Plan tasks 2 and 3 were executed in the reverse order, so that no intermediate commit fails to build"

metrics:
  duration: 32min
  tasks: 3
  files-modified: 4
  tests-added: 18
  commits: 3

completed: 2026-07-29
status: complete
---

# Phase 18 Plan 07: The Injection Input and the Start Flow Summary

**There are now two places for a human to type — a steering message into a live run and a command plus a goal into a new one — and neither the key handler that queues a message nor the confirmation that starts a run can lie about what it is doing: the first touches no filesystem API at all, and the second renders and dispatches the same `String`.**

## Performance

- **Duration:** 32 min
- **Started:** 2026-07-29T16:50:00Z
- **Completed:** 2026-07-29T17:22:00Z
- **Tasks:** 3
- **Files created:** 2 · **modified:** 2

## Accomplishments

- **OBS-03 is reachable for the first time.** `App::start_driver_run` has
  accepted `goal: Option<&str>` since Phase 17 and every caller passed `None`
  because *"there is no screen to type one into — that is Phase 18's"*.
  `DriverStartScreen` Step B is that screen. Nothing below the UI needed new
  plumbing: the typed string flows verbatim into `Action::DriverStartRequested`,
  into `RunRecord.goal`, into `ObservedRun.goal`.
- **`DEFAULT_DRIVE_COMMAND` is no longer the only value a run can carry.** Its
  own doc said *"Phase 18's command picker is what makes the field carry more
  than one value"*; it is now the default **selection** that an empty command
  field commits, and the picker seeds from
  `queue_md::suggest_next_commands(state)` — the same call that already backs
  `EnqueueScreen`'s Tab completion, so "what should I run next" has exactly one
  answer in the tree (D-23).
- **The injection key handler contains no filesystem API, and the grep that
  proves it is non-vacuous.** `enqueue.rs`, the file this screen is modelled on,
  calls `queue_md::save_queue` inline in its `Enter` arm. Here the append is
  dispatched as an `Action` and performed by 18-05's `spawn_blocking` scheduler
  (D-06, D-22, D-28). The module doc explains the rule **without naming a single
  filesystem API**, deliberately, so
  `rg 'fs::|OpenOptions|write_all|sync_data|save_' src/ui/screens/driver_inject.rs`
  returns nothing rather than matching a comment about the rule.
- **The correlation id is minted in the key handler, at queue time** (D-05), so
  the `queued` state is addressable before any other process has seen the line —
  and because text alone is not a correlation key, a user may legitimately send
  the same sentence twice.
- **The confirmation cannot name a different command from the one it starts.**
  `DriverConfirmScreen.command` is a single `String` read by both `prompt_text`
  and `do_start_run`; T-18-41 is closed structurally, not by a convention, and
  `a_start_from_the_picker_dispatches_the_chosen_command_and_the_verbatim_goal`
  asserts the dispatched pair end to end by driving the screen the wizard pushed.
- **Both interpolated strings are sanitised (T-18-38), and only for rendering.**
  The goal *and* the command pass through `sanitize_render_line` before reaching
  a prompt — the command matters because picker suggestions can originate in a
  `gsd-tools smart-entry` subprocess, not from the user's keyboard. The
  dispatched values stay verbatim, because one is an argv value and the other is
  the record OBS-03 renders.
- **Every pre-existing refusal path is intact.** The concurrency-cap /
  not-opted-in / unknown-alias behaviour is unchanged, `DrivableProject::from_registry`
  remains the real gate, and both halves of the load-bearing refusal test — the
  visible message *and* `rx.try_recv().is_err()` — still pass unmodified (T-18-37).
- **No word in this plan's copy claims a delivery outcome.** "sent", "received",
  "read" and "acknowledged" appear nowhere for an injected message (D-07); the
  success copy is `Queued — waiting for the driver to pick it up.` and it
  promises nothing about timing.
- **No new dependency, no new input model, no clippy regression.**
  `Cargo.toml`/`Cargo.lock` are byte-identical to the base; `tui-textarea` gains
  no call site (D-22); `cargo clippy --all-targets` still reports exactly the 5
  pre-existing lints. Test count **612 → 630**.

## Task Commits

1. **Task 1: The injection input screen (Surface 4)** — `72c14c1` (feat)
2. **Task 3: The confirmation names what it will actually run (Surface 5 Step C)** — `e8d05fc` (feat)
3. **Task 2: The two-step start flow (Surface 5 Steps A/B)** — `6c7e54d` (feat)

Tasks 2 and 3 were committed in the reverse of the plan's order; see deviation 1.

## Files Created/Modified

**Created**
- `src/ui/screens/driver_inject.rs` — `DriverInjectScreen`,
  `no_live_run_message`, the `FOOTER_HINT` constant, and 6 inline tests.
- `src/ui/screens/driver_start.rs` — `StartStep`, `DriverStartScreen`, the four
  footer-copy constants, and 8 inline tests.

**Modified**
- `src/ui/screens/driver_confirm.rs` — the `command` / `goal` fields and
  `new_start`, the command-aware `prompt_text`, `goal_row`, the two-or-three-row
  render, the widened `do_start_run`, a rewritten scope-fence header,
  `pub(super) dispatch`, `pub(crate) mod tests`, and 4 new inline tests.
- `src/ui/screens/mod.rs` — two `pub mod` declarations. **Nothing else** — no
  `ProjectViewCache` field was added or moved (18-05 owns `driver_runs`).

## Decisions Made

- **`DriverAction` stays `Copy`; the payload lives on the screen.** The plan
  specified widening `DriverAction::Start` to carry `command: String` and
  `goal: Option<String>`. Doing that literally would have forced an edit to
  `src/ui/screens/normal.rs:245-248`, which is **18-06's file executing
  concurrently in a sibling worktree**. Carrying the two values on
  `DriverConfirmScreen` instead delivers every guarantee the plan asked for —
  one value feeds both the prompt and the dispatch, so T-18-41 is closed
  structurally — while leaving the dashboard's `r` / `x` / `o` bindings and all
  five pre-existing tests compiling untouched. `new()` keeps its two-argument
  signature and defaults to `DEFAULT_DRIVE_COMMAND` / `None`, which is an honest
  description of what the dashboard's `r` does today; `new_start` is the
  picker's entry point.
- **The command is sanitised too, not just the goal.** The plan's `<action>`
  names only the goal, but T-18-38 says *"escape sequences in a goal **or
  command**"*, and `suggest_next_commands` prefers a `gsd-tools smart-entry`
  subprocess — so a command reaching the prompt is not necessarily something a
  human typed. Applied as Rule 2 (a `mitigate` row in the threat register is a
  correctness requirement).
- **Sanitising never reaches a dispatched value.** `prompt_text` sanitises a
  local copy. The `command` in `Action::DriverStartRequested` is an argv value
  and the `goal` is what `RunRecord.goal` will contain; a sanitiser that
  rewrote either would silently change what runs and what the run is recorded
  as wanting.
- **The module doc names no filesystem API, on purpose.** A doc comment
  explaining "never call `sync_data` here" would make the plan's grep criterion
  match its own explanation, converting a structural check into a vacuous one.
  The rule is stated in full; the API names live in D-06 and in 18-05's code,
  where they are load-bearing.
- **The no-completion guarantee is checked from two directions.**
  `driver_inject.rs` names no completion key at all — so a grep proves there is
  no such arm and the key provably falls through to the same do-nothing arm an
  unbound key does — and `FOOTER_HINT` is a named constant a test asserts offers
  no completion, so the screen cannot advertise something it does not do.
- **Enter-on-empty differs between the two new screens, and the asymmetry is
  documented at the code.** The command field accepts the default; the injection
  field does nothing. Two screens copied from one idiom behaving differently on
  the same key is precisely what a later reader will "fix", so the reason (there
  is a sensible default for which command to run and none for what a human wants
  to say) is in `driver_start.rs`'s module doc.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Plan tasks 2 and 3 executed in reverse order**
- **Found during:** planning task 2
- **Issue:** Task 2 (`driver_start.rs`) pushes a confirmation carrying a command
  and a goal, which is Task 3's `DriverConfirmScreen::new_start`. In the plan's
  order, Task 2's commit would not build.
- **Fix:** Task 3 was executed and committed second, Task 2 third. Both are
  self-contained; no content changed. Every commit builds, tests and passes
  clippy on its own.
- **Files modified:** none beyond the plan's own list.

**2. [Rule 3 - Blocking] `DriverAction::Start` was not widened; the payload went on the screen instead**
- **Found during:** task 3
- **Issue:** `src/ui/screens/normal.rs:247` constructs `DriverAction::Start` as a
  unit variant. Making it a struct variant is a compile error there — and
  `normal.rs` belongs to sibling plan **18-06**, executing concurrently in
  another worktree, where the edit would either be clobbered or produce a merge
  conflict.
- **Fix:** `DriverConfirmScreen` carries `command: String` and
  `goal: Option<String>`; `DriverAction` stays `Copy` and keeps working as a
  discriminator in `prompt_text` / `prompt_color`. `new()` defaults, `new_start()`
  is explicit. Every acceptance criterion and every `must_haves` truth is met.
- **Files modified:** `src/ui/screens/driver_confirm.rs`
- **Verification:** `git diff` touches no file outside this plan's scope; all 9
  `driver_confirm` tests pass, including the 5 pre-existing ones.
- **Committed in:** `e8d05fc`

**3. [Rule 2 - Missing Critical] The interpolated command was not being sanitised**
- **Found during:** task 3
- **Issue:** The plan's `<action>` sanitises the goal row only. T-18-38's
  register row is *"escape sequences in a goal **or command** echoed into the
  confirmation"*, disposition `mitigate`, severity `high` — and the picker's
  suggestion list can come from a subprocess, so a command is a disk/subprocess
  -sourced string on a straight path to a rendered prompt.
- **Fix:** `prompt_text` sanitises the command before interpolating it;
  `an_escape_bearing_goal_and_command_are_sanitised_before_they_reach_the_prompt`
  feeds a clear-screen and an OSC window-title set through both paths and
  asserts no `ESC` survives **while asserting the prose is still shown**, so the
  test cannot pass by deleting everything.
- **Files modified:** `src/ui/screens/driver_confirm.rs`
- **Committed in:** `e8d05fc`

**4. [Rule 3 - Blocking] `driver_confirm`'s test fixture and `dispatch` were private**
- **Found during:** task 1
- **Issue:** The acceptance criteria require the new screens' tests to assert on
  *"the `UnboundedReceiver<Action>` returned by `ctx_with_project`"*, which lived
  in `driver_confirm.rs`'s private `mod tests`. The alternative was a sixth and
  seventh full-field `AppContext` fixture, each of which breaks on any addition
  to that struct — the churn 18-04 already flagged.
- **Fix:** `mod tests` and `ctx_with_project` / `ALIAS` are `pub(crate)`;
  `dispatch` is `pub(super)`. All three driver screens now share one fixture and
  one dispatch mechanism. Both changes are inside this plan's file scope.
- **Files modified:** `src/ui/screens/driver_confirm.rs`
- **Committed in:** `72c14c1`

**5. [Rule 2 - Missing Critical] Two acceptance greps were satisfiable only by weakening the code, so the guarantees were restructured instead**
- **Found during:** task 1
- **Issue:** The plan requires
  `rg 'fs::|OpenOptions|write_all|sync_data|save_' driver_inject.rs` and
  `rg 'KeyCode::Tab' driver_inject.rs` to return **no match**, while its
  `<action>` also requires a module doc explaining the filesystem rule and a
  test proving Tab offers no completion. Written naively, both greps match the
  doc comment and the test — turning two structural checks into noise.
- **Fix:** the doc states the rule in full without naming any filesystem API
  (the names live in D-06, which it cites); the completion guarantee is proved
  by a `FOOTER_HINT` constant a test asserts contains no `Tab`, plus a
  do-nothing assertion on an arbitrary unbound key — which covers the completion
  key precisely *because* the grep proves no arm names it. Both greps now return
  nothing, and the guarantees are stronger than either check alone.
- **Files modified:** `src/ui/screens/driver_inject.rs`
- **Committed in:** `72c14c1`

**6. [Rule 2 - Missing Critical] Eight inline tests in `driver_start.rs` rather than the seven named**
- **Found during:** task 2
- **Issue:** The plan names seven behaviours. An eighth guarantee — that the
  **goal** field offers no Tab completion, and that the goal hint says
  `[Esc] back` because the code goes back — had no test.
- **Fix:** added `the_goal_field_offers_no_completion_and_neither_prompt_mentions_a_cursor`.
  Both named-in-the-criteria tests are present and passing.
- **Files modified:** `src/ui/screens/driver_start.rs`
- **Committed in:** `6c7e54d`

---

**Total deviations:** 6 auto-fixed (3 blocking, 3 missing-critical). No Rule 4
(architectural) decision arose.
**Impact on plan:** Deviation 2 is the only one that changes a named artifact's
shape, and it does so to respect a concurrent sibling's file boundary while
delivering the same guarantee. No scope creep, no dependency, no requirement
left short.

## Issues Encountered

- **A plan-specified enum widening collided with a sibling worktree's file.**
  18-07's artifact list names `DriverAction::Start` widened to carry two values;
  the only construction site of that variant is in 18-06's `normal.rs`. This is
  the second wave-parallelism gap in this phase (18-04 hit the same class of
  problem with `journal::RunSummary`): a plan's file scope should cover every
  file that a change to its named artifacts forces an edit in. Resolved without
  touching the sibling's file — see deviation 2.
- **`cargo fmt` reports drift across ~200 sites tree-wide**, including files
  untouched by this phase, so the installed rustfmt disagrees with the repo
  generally. Formatting is not part of the project gate
  (`build && test && clippy -- -D warnings`) and running it would have produced
  an enormous unrelated diff, so it was left alone; the three drift sites inside
  this plan's new code are line-wrapping only.
- Every count in this summary was taken through `rtk proxy`. Plain `cargo`
  output is filtered by the `rtk` summarising wrapper, and a criterion that
  greps for `warning:` or `test result:` without it passes **vacuously**.

## Verification

| Gate | Result |
|---|---|
| `cargo build` | pass |
| `cargo test` | **630 passed, 0 failed** (baseline 612; +6 `driver_inject`, +4 `driver_confirm`, +8 `driver_start`) |
| `cargo clippy -- -D warnings` | pass |
| `rtk proxy … cargo clippy --all-targets … \| wc -l` | **5** — the pre-existing count, unchanged |
| `git diff --stat aef3143 HEAD -- Cargo.toml Cargo.lock` | empty — no new dependency |
| `rg -nE 'tui_textarea\|TextArea' driver_start.rs driver_inject.rs` | no match — `tui-textarea` gained no call site (D-22) |
| `rg -nE 'fs::\|OpenOptions\|write_all\|sync_data\|save_' driver_inject.rs` | no match — the key handler touches no filesystem API |
| `rg -n 'KeyCode::Tab' driver_inject.rs` | no match — no completion arm exists |
| `rg -n 'suggest_next_commands' driver_start.rs` | 4 matches — the suggestion source is reused, not re-derived |
| `git diff HEAD~1 -- src/ui/screens/mod.rs` (each commit) | module declarations only — no `ProjectViewCache` field touched |
| `rg -n 'sent\|received\|acknowledged' driver_inject.rs driver_start.rs` | no match as an injected-message state (D-07) |

### Threat register

| Threat ID | Disposition | Evidence |
|---|---|---|
| T-18-37 (start against a project that never opted in) | **mitigated** | `do_start_run`'s affordance check and `DrivableProject::from_registry` are both unchanged; `starting_a_run_on_a_project_that_has_not_opted_in_sets_an_error_and_spawns_nothing` passes unmodified, including its `rx.try_recv().is_err()` half |
| T-18-38 (escape sequences in a goal or command) | **mitigated** | `an_escape_bearing_goal_and_command_are_sanitised_before_they_reach_the_prompt` feeds a clear-screen and an OSC title set through both interpolation paths |
| T-18-39 (filesystem work on the render thread) | **mitigated** | `driver_inject.rs` contains no filesystem API at all — asserted by grep — and the append is 18-05's `spawn_blocking` scheduler |
| T-18-40 (a queued message with no addressable identity) | **mitigated** | the id is minted in the key handler; `enter_on_a_non_empty_buffer_…` asserts it is non-empty on the dispatched action |
| T-18-41 (a confirmation naming a different command) | **mitigated** | one `String` feeds both the prompt and the dispatch; `a_start_from_the_picker_dispatches_the_chosen_command_and_the_verbatim_goal` asserts through the pushed screen |
| T-18-42 (package-manager installs) | **accepted** | zero dependencies added; `Cargo.toml` / `Cargo.lock` byte-identical to the base |

## Known Stubs

None introduced by this plan. Two declared seams belong to later plans and are
named at the code:

1. **Neither new screen is reachable by a keypress yet.** `i` and `s` on the
   Driver tab are **18-10**'s key routing. `no_live_run_message` exists in
   `driver_inject.rs` carrying the exact refusal copy the caller must set, so
   the guard and the screen cannot drift when 18-10 wires it.
2. **`Action::DriverInjectRequested` still has a log-only handler in `app.rs`**
   (18-04's Known Stub 3). The durable append is **18-05**'s
   `schedule_inbox_append`. Until it lands, a queued message is dispatched and
   logged but not written — the screen makes no claim about delivery either way,
   so nothing on screen asserts a state the mechanism cannot back.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **18-09** (the Driver tab render) can show the four-state injection display:
  the ids it correlates against are minted here, at queue time.
- **18-10** (key routing) needs `i` → `DriverInjectScreen::new(alias, run_id)`
  guarded by a live run, with `driver_inject::no_live_run_message(alias)` on
  `ctx.status_message` and **no `Action` dispatched** when there is none; and
  `s` → `DriverStartScreen::new(alias)`. The dashboard's existing `r` continues
  to reach `DriverConfirmScreen::new(alias, DriverAction::Start)` with the
  default command, and re-pointing it at the wizard is a one-line change in
  `normal.rs` whenever its owner wants it.
- **18-05** owns the append that turns a dispatched `DriverInjectRequested` into
  a durable line and a `queued` state.

**Carried obligations**

- `requirements-completed` is deliberately empty. TRANS-05, OBS-03 and STEER-01
  are not reachable by a user until 18-10 binds a key to these screens; this
  plan builds the surfaces they close on.

## Self-Check: PASSED

All four files present on disk (`driver_inject.rs`, `driver_start.rs`,
`driver_confirm.rs`, `mod.rs`); all three commits present in `git log`
(`72c14c1`, `e8d05fc`, `6c7e54d`); `git diff --diff-filter=D` empty for each
commit — no file was deleted; `Cargo.toml` / `Cargo.lock` unchanged from the
base; working tree clean apart from this summary.

---
*Phase: 18-driver-tab-live-watch-durable-injection*
*Completed: 2026-07-29*
