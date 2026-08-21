---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 07
subsystem: driver
tags: [dry-run, preview, command-source, goal, honesty-contract, rust]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    provides: "21-04's widened command-source refusal that made --goal a third legal source (the change CR-01 rode in on); 21-01's untrusted::bounded; 21-03's approval/digest seam the goal preview points a reader at"
provides:
  - "PreviewScope::GoalNotDecomposed { goal } — the fourth preview scope, whose renderer states that no command can be shown, why, and where the plan is obtainable"
  - "dry_run::build_goal_report(&DrivableProject, &str) -> ScopedPreview with an empty commands vector"
  - "ScopedPreview / render_scoped (promoted from RoutedPreview / render_routed; no alias left behind)"
  - "driver::CommandSource { Command, Routed, Goal } and command_source(..) -> Result<CommandSource, DriveError>"
  - "preview_text(&DrivableProject, &CommandSource) — three arms over a resolved type, no fall-through"
  - "corrected pinned SECTION_COMMANDS naming all three command sources"
  - "the enumerated command-source coverage array a fourth source must be added to"
affects: [21-09, 21-verification, driver-preview, tui-preview-pane]

actuals:
  tokens: 47000
  tasks: 2
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Resolve-once command source: a three-way distinction becomes a named enum computed at one seam and matched exhaustively at every consumer, so a fourth variant is a compile error rather than a silent fall-through"
    - "Unreachability is carried by the type or not claimed — a doc comment asserting a match arm is unreachable is deleted rather than reworded"
    - "Pinned user-visible contract text is corrected in the same commit that falsifies it (CONVENTIONS.md:75 precedent, dry_run.rs:78-83)"

key-files:
  created: []
  modified:
    - src/driver/dry_run.rs
    - src/driver/mod.rs
    - tests/driver_dry_run.rs
    - tests/async_blocking_guard.rs

key-decisions:
  - "Promoted the three-way command source into `CommandSource` rather than adding a fourth `preview_text` arm — adding beside is what shipped CR-01, so the fix removes the shape that allowed it"
  - "`build_goal_report` gathers the same diffstat/push/protection its siblings do: blast radius is knowable without knowing the commands, and it is what the preview exists to show"
  - "The goal is bounded through `untrusted::bounded` at report construction rather than at render time, so no path exists on which a raw TUI-supplied value reaches a terminal"
  - "The `spawn_blocking` closure keeps its braced body: a one-line closure changes the async-blocking scanner's handoff brace scope and silently swallows the join-failure fallback's allowlist entry"
  - "The in-source coverage test builds its project through `DrivableProject::from_registry`, never `for_testing_bypassing_opt_in`, which `tests/spawn_seam_guard.rs` fences out of `src/` entirely"

patterns-established:
  - "Enumerated variant coverage array: one constructed value per enum variant plus a `match`-based non-vacuity sweep, so extending the enum without extending the test is a compile error"
  - "Negative output assertions reference the forbidden phrase through a local `const` and carry a message stating what the phrase's presence would mean"

requirements-completed: [DRIVE-01, DRIVE-03]

coverage:
  - id: D1
    description: "A `--goal <text> --dry-run` invocation prints a preview that names the goal, lists no command at all, and says plainly that a plan cannot be previewed because a preview makes no model call"
    requirement: DRIVE-01
    verification:
      - kind: unit
        ref: "src/driver/mod.rs#a_goal_only_preview_never_claims_an_empty_command_is_the_honest_sequence"
        status: pass
      - kind: unit
        ref: "src/driver/dry_run.rs#a_goal_preview_names_the_goal_and_states_why_no_command_can_be_shown"
        status: pass
      - kind: integration
        ref: "tests/driver_dry_run.rs#a_goal_only_preview_never_claims_an_empty_command_is_the_honest_sequence"
        status: pass
      - kind: integration
        ref: "tests/driver_dry_run.rs#a_goal_only_preview_names_the_goal_and_shows_no_command"
        status: pass
    human_judgment: false
  - id: D2
    description: "`preview_text` dispatches on a resolved `CommandSource` with exactly one arm per legal source and no unreachable arm, so a fourth source is a compile error"
    requirement: DRIVE-03
    verification:
      - kind: unit
        ref: "src/driver/mod.rs#every_command_source_renders_a_preview_with_no_empty_numbered_command"
        status: pass
      - kind: unit
        ref: "src/driver/mod.rs#a_run_naming_both_command_sources_is_refused_rather_than_resolved"
        status: pass
      - kind: unit
        ref: "src/driver/mod.rs#a_stated_goal_alone_is_a_command_source_because_the_plan_supplies_the_target"
        status: pass
      - kind: other
        ref: "rtk proxy cargo build --all-targets (a partial rename does not compile; no deprecated alias exists)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The pinned `SECTION_COMMANDS` contract text names all three command sources, and the four-section byte-offset ordering assertion holds under every `PreviewScope` arm including the new one"
    requirement: DRIVE-03
    verification:
      - kind: unit
        ref: "src/driver/dry_run.rs#the_pinned_commands_text_no_longer_claims_one_command_is_always_the_whole_run"
        status: pass
      - kind: unit
        ref: "src/driver/dry_run.rs#every_scope_renders_the_four_pinned_sections_in_the_same_order"
        status: pass
    human_judgment: false
  - id: D4
    description: "A goal-only preview performs zero git writes and spawns no agent, proved by the before/after `.git` fingerprint and by the tripwire program rather than asserted (D-23)"
    requirement: DRIVE-01
    verification:
      - kind: integration
        ref: "tests/driver_dry_run.rs#a_goal_only_dry_run_also_leaves_the_git_directory_byte_identical"
        status: pass
      - kind: integration
        ref: "tests/driver_dry_run.rs#a_goal_only_dry_run_spawns_no_agent"
        status: pass
      - kind: integration
        ref: "tests/async_blocking_guard.rs#every_blocking_call_inside_an_async_fn_is_handed_off_or_allowlisted"
        status: pass
    human_judgment: false
  - id: D5
    description: "The doc comment asserting that a `preview_text` arm is unreachable is deleted rather than reworded"
    requirement: DRIVE-03
    verification:
      - kind: other
        ref: "git show 70d65b6:src/driver/mod.rs | grep -c 'last arm is unreachable' => 0; the arm itself no longer exists"
        status: pass
    human_judgment: false

duration: 57min
completed: 2026-08-21
status: complete
---

# Phase 21 Plan 07: An Honest Goal-Only Dry-Run Preview Summary

**`--goal X --dry-run` stopped printing an empty command as "the complete and honest sequence": a fourth `PreviewScope` arm plus a resolved `CommandSource` enum that makes the fall-through arm unrepresentable.**

## Performance

- **Duration:** 57 min
- **Started:** 2026-08-21T00:36:26Z
- **Completed:** 2026-08-21T01:33:47Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- **CR-01 is closed at the wiring, not just the renderer.** `preview_text` now takes a `&CommandSource` resolved once in `drive` and matches it with three arms and no fall-through. The `(None, None)` arm that routed a stated goal into `build_report(project, "")` no longer exists as a representable state.
- **The goal preview says what it cannot show, why, and where the answer is.** `PreviewScope::GoalNotDecomposed { goal }` renders under the pinned commands header with an empty command vector — no total, no numbered entry — and points the reader at the same invocation without `--dry-run`, which refuses before anything is created and prints the decomposed plan with its `--approved-plan` digest.
- **`SECTION_COMMANDS`, a released user-visible contract, was corrected in the commit that falsified it.** It now names all three command sources and states that a stated goal shows *no* command at all.
- **The false "the last arm is unreachable" doc is deleted, not reworded.** Unreachability is now a property of the resolved type; the exhaustive match is what carries it.
- **The invocation `21-VERIFICATION.md` recorded as untested is now covered six ways** — four integration tests, two in-module tests, plus the extended four-scope ordering contract and an enumerated coverage array a future fourth source has to be added to.

## Task Commits

Each task was committed atomically:

1. **Task 1 (tracer, TDD RED): failing test for the goal-only preview** — `d8e58bd` (test)
2. **Task 1 (tracer, TDD GREEN): the honest goal-only preview, argv → drive → stdout** — `c7c5156` (feat)
3. **Task 2: the regression tests CR-01 existed because nobody wrote** — `70d65b6` (test)

**Plan metadata:** committed with this SUMMARY (docs).

_TDD gate sequence: `test(...)` → `feat(...)` present in that order. No `refactor(...)` commit — the GREEN implementation needed no cleanup pass._

## Files Created/Modified

- `src/driver/dry_run.rs` — fourth `PreviewScope` arm and its renderer; `build_goal_report`; `RoutedPreview` → `ScopedPreview`; `render_routed` → `render_scoped`; corrected `SECTION_COMMANDS`; two new/extended in-module tests.
- `src/driver/mod.rs` — `CommandSource` enum; `command_source` replacing `command_source_refusal`; goal-aware `preview_text`; the deleted "unreachable" doc; resolved source bound in `drive` and threaded into both the `spawn_blocking` closure and the join-failure fallback; six migrated assertions plus two new tests.
- `tests/driver_dry_run.rs` — `goal_args` helper, `GOAL` and `COMMAND_MODE_TOTAL` consts, four new integration tests (12 total, up from 8).
- `tests/async_blocking_guard.rs` — `"build_goal_report("` added to `BLOCKING_HELPERS`.

## The rendered goal-only preview, verbatim

Captured from `dry_run::render_scoped(&dry_run::build_goal_report(&project, GOAL))` against the `repo()` fixture (one modified tracked file, one untracked file, an `origin` remote and an upstream), so a later reader can check the honesty claim against the bytes rather than against a description of them:

```
DRY RUN — nothing below was executed. No agent was spawned, no run was
journaled, and no git write was performed.

== GSD commands this run would issue ==
A run names one of three command sources: one supplied --command, a routed
sequence the decision router chooses per iteration under --target-phase, or
a --goal stated in plain language. For a supplied command, the line below is
the complete and honest sequence. For a routed run only the FIRST command
can be shown: every command after it is chosen from state this preview does
not produce, so the rest are not withheld — they do not exist yet. For a
stated goal NO command is shown at all, because the plan is a model
consultation away and a preview makes none.
  Stated goal: no command can be shown, and none is withheld. A goal
  becomes an ordered plan only through a model consultation, and a
  preview consults nothing and spawns no process — so the sequence does
  not exist yet rather than being hidden. To SEE the plan, run this same
  invocation without --dry-run: it decomposes the goal, then refuses
  before anything is created and prints the plan together with the
  --approved-plan digest that authorises exactly it.
    goal: get phase 22 verified

== Working tree a commit would capture ==
`git diff --stat HEAD` plus untracked files: the state this run would
inherit and could sweep into a `git add -A && git commit`. It is not the
diff of the command — that cannot be known without running it.
   tracked.txt | 1 +
   1 file changed, 1 insertion(+)
  Untracked:
    brand-new.txt

== Push refspecs this state would produce ==
Computed locally from git config (branch.<b>.remote, remote.pushDefault,
remote.<r>.push, push.default). No network was contacted, no credential was
used, and git's push subcommand was never invoked in any form.
  Remote: origin
  URL:    https://example.invalid/demo.git
  refs/heads/master:refs/heads/master
  Note: `push.default` is `simple` and the upstream `refs/heads/master` has the same name as the local branch, which is the case `simple` allows.

== What this envelope guarantees, and what it does not ==
Mechanically guaranteed: this run cannot reach your ambient git credentials
or your SSH agent — the agent socket is removed rather than emptied, and
git's global and system configuration are redirected into a generated file
that names no credential helper. A push that reaches git through the driven
process tree passes the pre-push hook, which judges the refs git itself
hands it rather than the command line it was asked about. The pull-request
cap is enforced from an append-only ledger this repository does not contain,
so the run cannot reset its own limit by deleting a file it can see.

Not guaranteed: client-side hooks, tool denies and env-injected git
configuration are all defeatable by an agent that can spawn an unsupervised
shell and chooses to. Each layer is documented with what it cannot see, an
agent that unsets GIT_CONFIG_COUNT in a subshell is past the last of them,
an agent that runs the askpass responder itself reads the token, and a
settings file the agent's own CLI silently ignores leaves the pull-request
cap unenforced, because no git hook observes a pull request. The only
boundaries that do not depend on the agent's cooperation are the remote's
own ruleset and the scope of the credential this run was given.

Therefore: enable server-side branch protection on this repository. It is
the one control here an agent cannot talk its way past, and it is this
envelope's conclusion rather than its footnote.
  WARNING — Remote protection: unknown — not probed here: a dry run contacts no network, and the read-only protection query runs once at run start under the run's own environment. Not knowing is reported as not knowing; it is not evidence that the remote is protected.
```

The commands section contains no `N command(s) in the sequence:` line and no numbered entry at all. The goal appears on one line, prefixed `goal:`, so it cannot be misread as a command.

## `SECTION_COMMANDS`, pre and post

**Before (Phase 20 text, falsified by `--goal`):**

```
== GSD commands this run would issue ==
A run is either one supplied --command or a routed sequence the decision
router chooses per iteration. For a supplied command, the line below is the
complete and honest sequence. For a routed run only the FIRST command can be
shown: every command after it is chosen from state this preview does not
produce, so the rest are not withheld — they do not exist yet.
```

**After:**

```
== GSD commands this run would issue ==
A run names one of three command sources: one supplied --command, a routed
sequence the decision router chooses per iteration under --target-phase, or
a --goal stated in plain language. For a supplied command, the line below is
the complete and honest sequence. For a routed run only the FIRST command
can be shown: every command after it is chosen from state this preview does
not produce, so the rest are not withheld — they do not exist yet. For a
stated goal NO command is shown at all, because the plan is a model
consultation away and a preview makes none.
```

This is a breaking, user-visible output change under CONVENTIONS.md:75, rated `costly` in the plan's reversibility block. The byte-offset ordering assertion locates the section by `.find(SECTION_COMMANDS)` and follows the constant automatically; the old text is not hardcoded anywhere except in the staleness assertion that exists to detect its return.

## No deprecated alias was left behind

The plan's acceptance criteria require this stated explicitly. **No `pub use RoutedPreview` or `pub use render_routed` re-export was added.** The rename is complete: `rtk proxy cargo build --all-targets` exits 0, which a partial rename could not do, and `grep -n "RoutedPreview\|render_routed" src/ tests/` matches only two lines of prose in `ScopedPreview`'s own doc explaining what it was renamed from and why an alias would reintroduce the drift.

## The guards can actually fail

A guard that cannot fail is the Phase-20 defect this repository has already paid for, so both new negative assertions were verified against a neutralized build rather than reasoned about:

- **RED, recorded:** the in-module `preview_text` test was written and committed *first* (`d8e58bd`), against the unmodified tree. It failed with the tree printing `1 command in the sequence:` and then a bare `1.` — CR-01 reproduced verbatim, not paraphrased.
- **Stash probe:** with the `GoalNotDecomposed` arm temporarily replaced by the `Complete` counter, both `driver::dry_run::tests::a_goal_preview_names_the_goal_and_states_why_no_command_can_be_shown` and `driver::tests::a_goal_only_preview_never_claims_an_empty_command_is_the_honest_sequence` FAILED. The probe was reverted and the suite returned green.

## Decisions Made

- **Promote rather than extend.** The plan's `<assumption_delta_decision>` called for it and execution confirmed the reasoning: a fourth `preview_text` arm would have fixed the symptom and left the shape that produced it. `CommandSource` is `pub(crate)` with owned variants (`'static` for the `spawn_blocking` move) and `Clone` (the join-failure fallback needs a second copy without re-resolving).
- **Precedence unchanged.** Command wins, then phase, then a non-whitespace goal, then `NoCommandSource`; command-plus-phase stays `AmbiguousCommandSource`. The migrated tests now assert on the *variant* rather than merely on legality, so a goal that resolved to the wrong arm is a failure rather than a pass.
- **Bound the goal at construction.** `untrusted::bounded` is applied inside `build_goal_report`, not in the renderer, so the `PreviewScope` value itself can never carry an unbounded string (T-21-07-02).
- **The four-section ordering contract now covers four scopes.** The existing single ordering test was extended rather than duplicated — the file's own doc explains that two ordering tests are two things that can disagree.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The plan's prescribed test fixture would have broken `tests/spawn_seam_guard.rs`**

- **Found during:** Task 2 (planning the enumerated coverage test), caught while reading the guard before writing.
- **Issue:** Task 2's `<action>` directs the enumerated coverage test in `src/driver/mod.rs` to use a `for_testing_bypassing_opt_in` project. `tests/spawn_seam_guard.rs` asserts that identifier appears on **exactly one** executable line under `src/` — its own `pub fn` definition in `src/executor/mod.rs` — and its line filter drops comments but does **not** exclude `#[cfg(test)]` modules. Following the plan literally would have failed `the_escape_hatch_has_no_call_site_in_src`, and the only ways to make the suite green again would have been to widen the audit or to move the test — both of which weaken a gate that exists to keep the opt-in enforced by the compiler.
- **Fix:** Added a `previewable(root)` helper that builds the project through the production constructor `DrivableProject::from_registry` using the existing `opted_in` fixture — the same route `dry_run.rs`'s own in-module test already takes. The helper's doc records why the hatch is not used, so the next person does not rediscover this.
- **Files modified:** `src/driver/mod.rs`
- **Verification:** `rtk proxy cargo test --test spawn_seam_guard` passes (7 tests); the coverage and preview tests pass.
- **Committed in:** `d8e58bd` (helper, with the RED test) and `70d65b6` (the coverage test that uses it)

**2. [Rule 1 - Bug] A one-line `spawn_blocking` closure silently disarmed the async-blocking guard**

- **Found during:** Task 1 (GREEN), caught by `no_allowlist_entry_is_stale` failing.
- **Issue:** Replacing three cloned `Option`s with one cloned `CommandSource` shortened the `spawn_blocking` call enough that rustfmt-shaped code put the closure body inline: `spawn_blocking(move || preview_text(&cloned, &cloned_source)).await {`. The scanner in `tests/async_blocking_guard.rs` suppresses reporting from the line a handoff marker appears on until the brace depth it opened closes — with no closure braces, the suppression extended over the whole `match` block, swallowing the join-failure fallback. The guard reported the deliberate `("src/driver/mod.rs", "preview_text(")` allowlist entry as stale, which was the *symptom*: the blocking call had become invisible, so the exemption stopped being a decision anybody made.
- **Fix:** Restored the braced closure body so the handoff scope closes before the `Err` arm, keeping the fallback reported and suppressed by its explicit allowlist entry. A comment at the call site records that the brace shape is a guard property rather than a style preference, so a future reformat does not silently undo it. This also honours the plan's instruction that the `spawn_blocking` wrapper and fallback "stay exactly as they are".
- **Files modified:** `src/driver/mod.rs`
- **Verification:** `rtk proxy cargo test --test async_blocking_guard` — 9 passed, including `no_allowlist_entry_is_stale`.
- **Committed in:** `c7c5156` (Task 1 GREEN commit)

**3. [Rule 3 - Blocking] One call site in `tests/driver_dry_run.rs` had to move with the rename**

- **Found during:** Task 1 (GREEN), at the first `--all-targets` build.
- **Issue:** `render_routed` → `render_scoped` broke `a_routed_preview_lists_the_routers_own_first_selection_and_nothing_after_it`. That file is nominally Task 2's, but a rename that does not compile cannot be committed atomically at Task 1.
- **Fix:** Updated the single call. Mechanical, no assertion changed.
- **Files modified:** `tests/driver_dry_run.rs`
- **Verification:** `rtk proxy cargo build --all-targets` exits 0; the test still passes.
- **Committed in:** `c7c5156` (Task 1 GREEN commit)

---

**Total deviations:** 3 auto-fixed (1 bug, 2 blocking)
**Impact on plan:** No scope creep. Two were forced by mechanical guards the plan did not account for, and both fixes *preserve* those guards rather than relaxing them. The third was a one-line consequence of a rename the plan mandated.

## Issues Encountered

- **`cargo fmt --check` is not clean on this tree, and was not made clean.** The installed rustfmt disagrees with the committed formatting in files this plan never touches (`src/app.rs`, `src/driver/reconcile.rs`, `src/driver/escalate.rs`, `tests/driver_escalation_cap.rs`, and pre-existing lines in `src/driver/mod.rs` and `tests/driver_dry_run.rs`) — a version drift, not a regression. Running `cargo fmt` would have reformatted unrelated files, which the scope boundary forbids. The two lines *this plan* added that rustfmt wanted rewrapped were rewrapped by hand, so no new formatting debt was introduced. The gate this plan is judged by is `cargo clippy -- -D warnings` on the lib target, which is clean.

## Verification Results

All six items from the plan's `<verification>` block:

| # | Gate | Result |
|---|------|--------|
| 1 | `rtk proxy cargo build --all-targets` | clean, exit 0 |
| 2 | `rtk proxy cargo clippy -- -D warnings` (lib gate) | clean, exit 0 |
| 3 | `rtk proxy cargo test --test driver_dry_run` | **12 passed**, 0 failed (was 8 in `21-VERIFICATION.md`) |
| 4 | `rtk proxy cargo test --lib driver::` | 232 passed, 0 failed |
| 5 | `rtk proxy cargo test --test async_blocking_guard` | 9 passed, 0 failed |
| 6 | Manual read-back of the verbatim preview | recorded above; contains no numbered command entry |

Full suite for regression safety: **1008 lib tests + every integration target pass, 0 failures.**

## Known Stubs

None. No hardcoded empty value, placeholder string, TODO or unwired component was introduced. The one empty value in this change — `commands: Vec::new()` in `build_goal_report` — is the load-bearing point of the plan, not a stub: it is what makes the preview honest, and four assertions exist specifically to keep it empty.

## Threat Flags

None. No new network endpoint, auth path, file access pattern or schema change at a trust boundary was introduced. The three `mitigate` dispositions in the plan's threat register are all implemented and covered:

| Threat ID | Mitigation | Where proved |
|-----------|------------|--------------|
| T-21-07-01 (Repudiation) | `CommandSource` makes the goal arm total; `GoalNotDecomposed` states plainly that no command can be shown; the false-total phrasing is pinned absent | `every_command_source_renders_a_preview_with_no_empty_numbered_command`, both `..._never_claims_an_empty_command_is_the_honest_sequence` tests |
| T-21-07-02 (Tampering, goal echo) | goal bounded through `untrusted::bounded` at report construction | `build_goal_report` construction site; `untrusted`'s own control-character tests |
| T-21-07-03 (Tampering, read-only) | `.git` fingerprint and tripwire extended to the goal invocation; `build_goal_report(` named in `BLOCKING_HELPERS` | `a_goal_only_dry_run_also_leaves_the_git_directory_byte_identical`, `a_goal_only_dry_run_spawns_no_agent`, `async_blocking_guard` |

T-21-07-04 was dispositioned `accept` at plan time and remains accepted: the corrected contract text discloses that a third command source exists, which is the intended disclosure.

## Next Phase Readiness

- **ROADMAP criterion 1's CR-01 half is closed.** The criterion has a second cause recorded in `21-VERIFICATION.md`, addressed by the sibling gap-closure plans (21-08/21-09/21-10); this plan does not claim the criterion as a whole.
- **21-09 depends on these symbol names** and gets them exactly as specified in `<artifacts_this_phase_produces>`: `PreviewScope::GoalNotDecomposed`, `build_goal_report`, `ScopedPreview`, `render_scoped`, `CommandSource`, `command_source`, and the new `preview_text` signature.
- **One thing a future plan should know:** `preview_text` is now `fn(&DrivableProject, &CommandSource)`. Any TUI-side caller added by Phase 18 must resolve a `CommandSource` rather than passing `Option`s, which is the point — it cannot accidentally construct the state CR-01 lived in.

## Self-Check: PASSED

- `src/driver/dry_run.rs` — FOUND, contains `GoalNotDecomposed`, `build_goal_report`, `ScopedPreview`, `render_scoped`
- `src/driver/mod.rs` — FOUND, contains `enum CommandSource`, `fn command_source`
- `tests/driver_dry_run.rs` — FOUND, contains `fn goal_args`, `build_goal_report`
- `tests/async_blocking_guard.rs` — FOUND, contains `build_goal_report(`
- Commit `d8e58bd` — FOUND
- Commit `c7c5156` — FOUND
- Commit `70d65b6` — FOUND

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-21*
