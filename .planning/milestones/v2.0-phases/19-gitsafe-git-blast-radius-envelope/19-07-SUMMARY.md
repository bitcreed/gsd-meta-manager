---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 07
subsystem: infra
tags: [spawn-seam, argv, child-environment, journal-park, terminal-record, safety-envelope]

# Dependency graph
requires:
  - phase: 19-gitsafe-git-blast-radius-envelope
    provides: "19-01's `envelope_root`/`envelope_dir`, `hooks::install`, `assert_provenance` and the `GSD_MM_ENVELOPE_ROOT` override; 19-02's `ParkReason`, `classify_git`, `disallowed_tools` and `GitVerdict::Refuse`; 19-03's `pre_push`/`pre_commit` bodies and the full-worktree credential scan; 19-04's `EnvelopeEnv`, `build_env`, `PROJECT_ROOT_ENV` and `RUN_ID_ENV`; 19-05's `write_settings` (whose `?` is the D-07 gate), `guard`/`guard_in` and the PR ledger; 19-06's `probe_protection`, `envelope_notice` and `ProtectionState`"
  - phase: 17-driver-supervisor
    provides: "the single production `ExecutionOptions` construction site and the `spawn_blocking` clone-not-move pattern; `DrivableProject::from_registry`'s single call site and `tests/spawn_seam_guard.rs`; `outcome_label` and the two `journal.finish` call sites"
  - phase: 16-run-journal
    provides: "`JournalEvent::Parked`'s schema, `EMITTED_KINDS`/`RESERVED_KINDS` and the mechanical-complement test; `run_paths`' `Option` (the WR-02 fix); `writer::read_active_run`, `JournalWriter::open`/`append`, `reader::read_all`"
  - phase: 15-agent-executor
    provides: "`build_argv`'s flag-then-value discipline and its exhaustive `match options.target`; the ONE spawn closure that builds the child's environment"
provides:
  - "`executor::ExecutionOptions` fields `envelope_disallowed_tools`, `envelope_settings` and `envelope_env` — the three carriers that cross the spawn"
  - "`executor::claude::build_argv` emitting `--disallowedTools` (comma-joined) and `--settings`"
  - "the spawn closure applying `EnvelopeEnv` entries beside the CLAUDE* scrub — the one place the child's environment is built"
  - "`driver::run::establish_envelope` and `EstablishedEnvelope` — the four-layer establishment under one `spawn_blocking`"
  - "`error::DriveError::EnvelopeAssertionFailed { reason, detail }`"
  - "`driver::envelope_refusal` — the positioned assertion in `drive`'s ordered chain"
  - "`envelope::ParkOutcome`, `envelope::park_at(project_root, reason, needs)` and `envelope::park(reason, needs, detail)` — the single run-journal appender"
  - "`envelope::cred::EnvelopeEnv::with_run_id` — the driver's fold-in of `RUN_ID_ENV`"
  - "`driver::run::terminal_label` and `PARKED_LABEL_PREFIX` — `parked:<reason>` in the terminal `run.json`"
  - "`journal::EMITTED_KINDS` gains `\"parked\"`; `RESERVED_KINDS` loses it"
  - "`tests/envelope_wiring.rs` with the shared `parked_events` and `start_run_with_envelope` helpers"
affects: [19-08 gate, 20 router]

# Actuals (#2632)
actuals:
  tokens: 33400
  tasks: 4
  commits: 6

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Absence separated from failure at the establishment step: a project with no `.git` has no repository for an ignore rule to protect, so the exclude block is *unnecessary* rather than *unwritable* and is skipped; a `.git` that exists and cannot be written still refuses. Refusing every non-git project would be a control failing into unusability, which is the shape of control that gets switched off."
    - "The reason travels WITH the verdict: `classify_segments` returns `(ParkReason, String)` rather than a message the caller re-classifies. Two derivations of the same fact is how a journal comes to disagree with the refusal it records."
    - "The second stderr line is written inside `park`, not by each caller. 'Every caller remembers to report an unrecorded park' is not a property anything can check; a line inside the one appender is."
    - "Test-binary envelope isolation: six driver fixtures redirect `GSD_MM_ENVELOPE_ROOT` to a per-binary `OnceLock<TempDir>`, because driving a run now writes an envelope and a test may not write into the developer's real `~/.local/share`."

key-files:
  created:
    - tests/envelope_wiring.rs
  modified:
    - src/executor/mod.rs
    - src/executor/claude.rs
    - src/driver/run.rs
    - src/driver/mod.rs
    - src/envelope/mod.rs
    - src/envelope/hooks.rs
    - src/envelope/cred.rs
    - src/journal/mod.rs
    - src/error.rs
    - tests/driver_inbox.rs
    - tests/driver_kill.rs
    - tests/driver_kill_startup.rs
    - tests/driver_lock.rs
    - tests/driver_optin.rs
    - tests/driver_reattach.rs
    - tests/driver_tracer.rs

key-decisions:
  - "The `--disallowedTools` patterns are comma-joined into ONE argv value rather than pushed as a space-separated variadic. The installed CLI documents the flag as taking a 'comma or space separated list' (verified against `claude --help` at the pinned 2.1.214-2.1.220 range), and a variadic would keep consuming words — including the next flag — the first time one is appended after it."
  - "`cred::write_gitconfig` is NOT called separately at establishment, although the plan's step list names it. `build_env` already writes the generated config, and calling it twice would resolve the user's identity twice and could write two different files. One writer, one file."
  - "`hooks::write_exclude_block` is skipped when the project root has no `.git`. Absence is not failure: a project with no repository has no history for a swept file to reach and no `git status` to quieten. A `.git` that exists and cannot be written still refuses, so the refusal is not weakened for any caller that actually has a repository. The predicate asks about `.git` — both the directory and the linked-worktree pointer file — which is exactly the pair `hooks::git_dir` resolves, so the two cannot disagree about what a repository is."
  - "`EnvelopeEnv::with_run_id` is a separate call rather than a `build_env` parameter. `build_env` is given an alias and a project root, not a run — `RUN_ID_ENV`'s own doc says so — and a run id invented inside it would be a second place run ids come from. The driver already owns the id, because it is on the driver's argv."
  - "The envelope assertion is split across two positions with ONE refusal variant. `drive`'s ordered chain answers the cheap, pure question (is there a root, may this alias have an envelope) before dispatch; `execute_run` answers the expensive one (can the four layers actually be written) before the lock and before the journal. Both return `DriveError::EnvelopeAssertionFailed` carrying `ParkReason::EnvelopeAssertionFailed`. Doing the whole establishment in `drive` would have meant threading an `EstablishedEnvelope` through `dispatch` and moving the wiring off the single `ExecutionOptions` construction site."
  - "The drive-time refusal writes NO `Parked` event, and the code says so in a comment rather than emitting nothing silently. It sits before dispatch, so there is no run and no journal; the evidence is the typed error, the non-zero exit and the stderr line. The forward constraint is recorded at the position: if it ever becomes reachable after a journal is open, `envelope::park` is the appender — one appender, not a second one written speculatively."
  - "`ParkOutcome::Unresolvable` is unreachable today and is kept anyway. `read_active_run` validates the pointer first, so a traversing `active` file surfaces as `NoActiveRun` before `run_paths` is reached. The variant is the second of two layers: if that validation were ever weakened, this is what still stands between an agent-controlled string and a `join`. D-28 says the `None` is handled rather than unwrapped, and this is what handling it looks like."
  - "`park` writes its own second stderr line for any outcome other than `Appended`, rather than leaving it to `main.rs`. Silence is the one behaviour forbidden here, and a line inside the single appender is a property; 'every caller remembers' is not."
  - "`classify_segments` returns `(ParkReason, String)` instead of a message. The guard's refusal message and its park reason are the same decision, and letting the caller re-classify the message would be deriving it twice."
  - "`terminal_label` returns the LAST park, not the first. A run may be refused more than once, and the terminal record names the state the run ended in. A journal that cannot be read yields the plain outcome label — a park that cannot be read is a park that was not observed, and claiming one would be inventing evidence in the file this phase exists to make trustworthy."
  - "Both `finish` call sites hand `terminal_label` to `spawn_blocking` with an inline re-run on join failure — `driver::drive`'s dry-run arm's own answer to the same question. It keeps the terminal record honest on a path no healthy run reaches, at the cost of a blocking call in a process that is already ending."
  - "`main.rs` gains nothing, and that is a decision rather than an omission. `park` emits its own stderr line, which reaches the same place the `Envelope` arms' `eprintln!` does because it is the same stderr. Adding a second print site would be a second place the same line can be emitted."

patterns-established:
  - "Establishment is all-or-nothing and its failure is a typed refusal positioned before anything is created. A run with three of four layers is a run with one enforcement layer quietly absent and nothing at all saying so — which is the silently-disarmed control D-06's layering exists to prevent."
  - "A doc that describes the previous build is worse than no doc, because it is read as current: `JournalEvent::Parked`'s 'Schema only in this phase' comment was corrected in the SAME commit as the first emission, and `parked` moved between the two kind lists in that commit too."
  - "Cross-module seams spell the other end's full path in a comment, so `grep envelope::park` finds every end of the link and not just the definition."

requirements-completed: [SAFE-01, SAFE-02, SAFE-03, SAFE-05, SAFE-06]  # All five are shared with sibling plans in this phase; the orchestrator's shared-ID gate releases each when the last declaring plan lands. REQUIREMENTS.md was NOT touched — parallel worktree mode.

coverage:
  - id: D1
    description: "The driven child's argv carries `--disallowedTools` with the envelope's tool patterns and `--settings` with the generated settings path, and the pinned baseline vector is byte-identical when no envelope was established"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/executor/claude.rs#the_envelope_deny_list_and_the_settings_path_ride_on_argv (the EXACT vector, not a `contains`)"
        status: pass
      - kind: unit
        ref: "src/executor/claude.rs#the_pinned_vector_is_unchanged_when_no_envelope_was_established"
        status: pass
      - kind: unit
        ref: "src/executor/claude.rs#the_envelope_flags_are_each_absent_on_their_own_when_unset"
        status: pass
      - kind: integration
        ref: "tests/envelope_wiring.rs#the_envelope_deny_list_and_the_settings_file_carry_the_same_controls"
        status: pass
    human_judgment: false
  - id: D2
    description: "No permission bypass is ever emitted, and `PermissionMode` gains no bypass variant — held by a match that would fail to COMPILE, not by a comment"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/executor/claude.rs#the_permission_mode_enum_has_exactly_one_variant_and_it_is_not_a_bypass"
        status: pass
      - kind: unit
        ref: "src/executor/claude.rs#the_envelope_flags_never_smuggle_a_permission_bypass, #the_argv_never_carries_a_permission_bypass"
        status: pass
      - kind: integration
        ref: "tests/envelope_wiring.rs#the_permission_mode_still_has_no_bypass_and_the_target_match_is_still_exhaustive"
        status: pass
      - kind: other
        ref: "grep -rnE 'dangerously-skip-permissions|bypassPermissions' src/ filtered to code lines returns 0 (see deviation 6)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The child's environment is built in exactly one place — the existing spawn closure — and carries the full envelope alongside the agent-variable scrub"
    requirement: SAFE-05
    verification:
      - kind: other
        ref: "grep -rn 'cmd.env(' src/ | grep -v src/executor/claude.rs returns no match"
        status: pass
      - kind: other
        ref: "awk '/let mut wrap = CommandWrap::with_new/,/^\\s*\\}\\);/' src/executor/claude.rs | grep -c 'env_remove' returns 2 — the CLAUDE* scrub plus the envelope removals, both in the one closure"
        status: pass
      - kind: integration
        ref: "tests/envelope_credential.rs (19-04) still green — `EnvelopeEnv`'s removal/assignment distinction is unchanged by the applier"
        status: pass
    human_judgment: true
    rationale: "The greps prove there is exactly one applier and that it removes as well as sets. No test spawns a real agent and reads the child's `/proc/<pid>/environ`, because doing so needs the agent CLI the phase's own scope fence (D-35) keeps out of the suite. That the applied environment is what the child actually sees is therefore reasoned from `Command`'s contract rather than observed, and is recorded as such."
  - id: D4
    description: "Envelope establishment happens once, at the single production `ExecutionOptions` construction site, under `spawn_blocking`, with no second capability-token construction site"
    requirement: SAFE-02
    verification:
      - kind: other
        ref: "one production `ExecutionOptions` construction in src/driver/run.rs (line 1148); the other three are in `mod tests` (see deviation 5)"
        status: pass
      - kind: integration
        ref: "tests/spawn_seam_guard.rs — 7 tests, allowlist unchanged, `DrivableProject::from_registry` still one production call site"
        status: pass
      - kind: other
        ref: "grep -c 'spawn_blocking' src/driver/run.rs returns 9, covering the envelope establishment block and both terminal-label reads"
        status: pass
    human_judgment: false
  - id: D5
    description: "An envelope that cannot be established refuses the run before anything is created — no lock file, no run directory, no run record, no journal — and the refusal names `envelope_assertion_failed`"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/envelope_wiring.rs#an_envelope_that_cannot_be_established_refuses_before_anything_is_created (out of process: exit code, stderr reason, and `runs_root` absent)"
        status: pass
      - kind: integration
        ref: "tests/envelope_wiring.rs#an_alias_that_can_have_no_envelope_is_refused_by_the_ordered_chain (in process: the TYPED error, and nothing on disk)"
        status: pass
    human_judgment: false
  - id: D6
    description: "`JournalEvent::Parked` gains its first emitter, its stale schema-only doc is corrected in the same commit, and `parked` MOVES from `RESERVED_KINDS` to `EMITTED_KINDS` without the complement test being weakened"
    requirement: SAFE-01
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#every_reserved_kind_is_declared_and_none_is_emitted_by_this_phase — assertion bodies unmodified; only the fixture lists moved, exactly as they did for `interjected`"
        status: pass
      - kind: other
        ref: "awk over RESERVED_KINDS returns 0 for \"parked\"; awk over EMITTED_KINDS returns 1 — a move, not a duplication"
        status: pass
      - kind: other
        ref: "git diff 740e62f -- src/journal/mod.rs | grep -c 'Schema only in this phase' returns 2 on the removed side"
        status: pass
    human_judgment: false
  - id: D7
    description: "Every refusal the envelope produces appends a `Parked` event carrying that refusal's own reason into the active run's journal — four reasons, four processes other than the asserting one, four journals read off disk"
    requirement: SAFE-03
    verification:
      - kind: integration
        ref: "tests/envelope_wiring.rs#a_force_push_is_refused_and_lands_a_force_push_blocked_park"
        status: pass
      - kind: integration
        ref: "tests/envelope_wiring.rs#a_hooks_path_rewrite_is_refused_and_lands_a_hook_bypass_blocked_park"
        status: pass
      - kind: integration
        ref: "tests/envelope_wiring.rs#a_worktree_carrying_a_credential_is_refused_and_lands_a_secret_detected_park (a REAL `git push` through the generated pre-push stub)"
        status: pass
      - kind: integration
        ref: "tests/envelope_wiring.rs#a_pull_request_beyond_the_cap_is_refused_and_lands_a_pr_cap_exceeded_park"
        status: pass
      - kind: integration
        ref: "tests/envelope_wiring.rs#a_permitted_command_is_permitted_and_parks_nothing (the paired allow)"
        status: pass
      - kind: unit
        ref: "src/envelope/mod.rs#a_park_against_a_live_run_lands_exactly_one_event_carrying_that_reason, #a_park_never_reports_appended_unless_something_was_appended"
        status: pass
      - kind: other
        ref: "RED commit 47a7015: 9 rows fail against the stubbed decision; all pass at 3e37cb4"
        status: pass
    human_judgment: false
  - id: D8
    description: "Refusal never depends on parking succeeding: the verdict and the non-zero exit are settled before the journal is touched, and a re-entry that cannot resolve a journal still refuses and says on stderr that it could not park"
    requirement: SAFE-06
    verification:
      - kind: integration
        ref: "tests/envelope_wiring.rs#a_re_entry_with_the_locator_absent_still_refuses_and_says_it_could_not_park (exit 2 unchanged, reason unchanged, 'NOT recorded' present, nothing appended)"
        status: pass
      - kind: unit
        ref: "src/envelope/mod.rs#a_project_with_no_active_pointer_yields_no_active_run_and_appends_nothing, #an_active_pointer_that_is_not_a_plain_component_is_refused_and_appends_nothing"
        status: pass
    human_judgment: false
  - id: D9
    description: "The terminal `run.json` for a run whose journal carries a `Parked` event names that park reason in its `outcome`, rather than a generic label"
    requirement: SAFE-01
    verification:
      - kind: unit
        ref: "src/driver/run.rs#the_terminal_record_on_disk_carries_the_park_reason_as_its_outcome"
        status: pass
      - kind: unit
        ref: "src/driver/run.rs#the_terminal_label_names_the_park_reason_from_the_last_park_on_disk, #a_run_whose_journal_carries_no_park_keeps_its_ordinary_terminal_label"
        status: pass
      - kind: integration
        ref: "tests/envelope_wiring.rs — all four end-to-end rows assert the terminal record alongside the exit code and the journal"
        status: pass
    human_judgment: false
  - id: D10
    description: "D-28's five carried-in constraints still hold after a plan that touched four of the five files they name"
    verification:
      - kind: integration
        ref: "tests/envelope_wiring.rs#run_paths_still_returns_an_option_and_still_refuses_a_traversing_id, #liveness_is_still_consumed_as_a_tri_state_rather_than_a_boolean, #the_configuration_path_is_still_first_on_the_drive_argv, #the_permission_mode_still_has_no_bypass_and_the_target_match_is_still_exhaustive"
        status: pass
      - kind: other
        ref: "grep -n 'ExecutionTarget::Host => {}' src/executor/claude.rs still matches — the container tripwire is intact"
        status: pass
      - kind: other
        ref: "git diff 740e62f -- src/driver/spawn.rs | grep -cE '^\\+.*fn drive_argv' returns 0 — no new `drive_argv` argument was needed"
        status: pass
    human_judgment: false
  - id: D11
    description: "No assertion in the new test file derives success from a model's prose; every fact is an exit code, a file on disk, a git ref or a journal event"
    requirement: SAFE-01
    verification:
      - kind: other
        ref: "grep -rniE '(agent|model)[^\\n]{0,40}(summary|said|reported)' tests/envelope_wiring.rs returns 3 matches, all on comment lines (5, 11, 16), none inside an assertion"
        status: pass
    human_judgment: true
    rationale: "The grep proves no assertion literal mentions an agent's report. Whether the SUITE as a whole is genuinely free of prose-derived confidence — rather than merely free of the words the grep looks for — is a reading judgement. It is the judgement D-25 is a `[USER-HARD-REQUIREMENT]` about, and the phase's transparency prohibitions verify by judgment for exactly this reason."

# Metrics
duration: 51 min
completed: 2026-08-18
status: complete
---

# Phase 19 Plan 07: The Envelope Reaches the Spawn Seam, and Every Refusal Parks the Run Summary

**The envelope stops being a module and becomes a boundary: the tool denylist and the settings path ride on argv where a validation failure cannot silently drop them, the full scrub-and-rebuild reaches the child through the one closure that has ever built its environment, a run whose envelope cannot be established is refused before a lock file exists — and a force push, a `core.hooksPath` rewrite, a detected credential and an exceeded pull-request cap each append a `Parked` event carrying their own D-24 reason into the run's journal, read back off disk by a process other than the one that refused, with the reason also on the terminal `run.json`.**

## Performance

- **Duration:** ~51 min
- **Started:** 2026-08-18 (wave base `c107e04`)
- **Completed:** 2026-08-18
- **Tasks:** 4 (Task 4 executed as RED → GREEN)
- **Files modified:** 17 (1 created, 16 modified)

## Accomplishments

- **The three carriers that cross the spawn are all filled, in the one place each belongs.** `--disallowedTools` and `--settings` on argv from `build_argv`; the `EnvelopeEnv` value applied entry-by-entry in the spawn closure that already scrubs `CLAUDE*`; both established at the single production `ExecutionOptions` construction site. The pinned argv vector test was extended **in the same commit** as the flags (D-28), asserting the exact vector with both set and the byte-identical pre-Phase-19 baseline with neither — so the new flags cannot quietly become unconditional.
- **Argv is the carrier for a reason that is now written down where a later reader will find it.** `build_argv`'s doc names the decisive property: a `--settings` file that fails validation is *silently ignored in print mode with no error shown*, and argv cannot be silently dropped because it is argv. That is D-06 layer 1's whole justification, stated at the function rather than in a plan.
- **`JournalEvent::Parked` has its first emitter, and the comment that said otherwise was corrected in the same commit.** `parked` moved out of `RESERVED_KINDS` and into `EMITTED_KINDS` — a move, not a duplication — recorded in the same one-line form `interjected`'s own Phase 18 move uses. `every_reserved_kind_is_declared_and_none_is_emitted_by_this_phase` keeps its assertion bodies untouched: only the fixture lists moved, which is exactly what the complement property means.
- **Four reasons, four processes, four journals read off disk.** `force_push_blocked` and `hook_bypass_blocked` through a spawned `gsd-meta-manager envelope guard`; `secret_detected` through a **real** `git push` that git ran the generated `pre-push` stub for; `pr_cap_exceeded` through the guard against a ledger seeded to the cap. Every one asserts all four of D-25's evidence facts in a single test — non-zero exit, the on-disk `Parked` event, the park reason on the terminal `run.json`, and (for the push rows) the remote ref byte-identical to before — because splitting them would let a refusal that exited non-zero and pushed anyway pass three tests out of four.
- **Silence is structurally impossible, not merely discouraged.** `park` writes its own second stderr line for any outcome other than `Appended`, inside the single appender, so a refusal that could not be recorded says so. A re-entry with the locator unset exits 2 with the same reason and an explicit `NOT recorded` line — the accepted residue T-19-57 names, stated in the code and proved by a test rather than left in a threat table.
- **The RED step is committed history, not a reverted mutation.** At `47a7015` the API is real and only the *decision* is stubbed (`park_at` answers `NoActiveRun`, `terminal_label` never reads the journal, no re-entry calls `park`). **9 rows fail there and pass at `3e37cb4`.** The rows that pass against the stub — no-active-run, the traversing pointer, the permitted-command pairing, and every exit-code assertion — are the ones that *must*, because the refusals themselves already worked and this step was only ever about whether they leave evidence.
- **986 tests passing, from a 960 baseline.** `cargo clippy --all-targets -- -D warnings` still reports exactly the 5 pre-existing lints at unchanged locations, measured with `rtk proxy` per D-34. No crate was added — `Cargo.toml` is untouched, so T-19-SC holds phase-wide.
- **The developer's real `~/.local/share` is no longer written to by the test suite.** Driving a run now establishes an envelope, and six driver fixtures that drive one were silently creating envelope directories under fixture alias names. Each redirects `GSD_MM_ENVELOPE_ROOT` to a per-binary temp root.

## Task Commits

1. **Task 1: the envelope reaches argv — `--disallowedTools` and `--settings`** — `668af7e` (feat)
2. **Task 2: the envelope is established once, and the child's environment built once** — `29ee5e2` (feat)
3. **Task 3: a positioned refusal, and `parked` stops being reserved** — `ae9086f` (feat)
4. **Task 4 (RED): every refusal parks the run, as failing rows** — `47a7015` (test)
5. **Task 4 (GREEN): one appender, three re-entries, four reasons on disk** — `3e37cb4` (feat)
6. **Cross-module link markers, and the ETXTBSY deferral** — `b7acce3` (docs)

_No REFACTOR commit on the TDD task: the GREEN implementation needed no cleanup pass, and an empty `refactor` commit is ceremony rather than a change._

## Files Created/Modified

- `tests/envelope_wiring.rs` (new, 976 lines) — the file-level rule that no assertion may read a model's summary; `parked_events` and `start_run_with_envelope` (the two shared helpers); `RunFixture` with `child_env`, `ask_guard`, `push` and `remote_refs`; 14 tests
- `src/executor/mod.rs` — `ExecutionOptions::envelope_disallowed_tools`, `envelope_settings`, `envelope_env`, and their `Default`
- `src/executor/claude.rs` — `build_argv`'s two new flags and the why-argv paragraph; the `EnvelopeEnv` application in the spawn closure; 5 new tests
- `src/driver/run.rs` — `EstablishedEnvelope`, `establish_envelope`, the establishment block under `spawn_blocking`, the run-start protection notice, `PARKED_LABEL_PREFIX`, `terminal_label`, both `finish` call sites; 3 new tests
- `src/driver/mod.rs` — `envelope_refusal` and its three-clause position comment in `drive`'s ordered chain
- `src/envelope/mod.rs` — `ParkOutcome` + `describe`, `park_at`, `park`; 4 new tests
- `src/envelope/hooks.rs` — `NEEDS_HUMAN`, `park_refusal`, and a park on every refusal branch of `pre_push`, `pre_commit` and `guard`; `classify_segments` and `deny` now carry a `ParkReason`
- `src/envelope/cred.rs` — `EnvelopeEnv::with_run_id`
- `src/journal/mod.rs` — `Parked`'s corrected doc, the `parked` kind move, and the fixture-list move inside the complement test
- `src/error.rs` — `DriveError::EnvelopeAssertionFailed { reason, detail }`, its `Display` and its `source` arm
- `tests/driver_{inbox,kill,kill_startup,lock,optin,reattach,tracer}.rs` — `isolate_envelope_root()`

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The four a later reader is most likely to want the reasoning for:

- **The exclude block is skipped for a project with no `.git`, and that is not a weakened refusal.** This was the one genuine fork. `write_exclude_block` needs a git directory, and seven integration tests drive runs against plain temp directories — so a strict establishment refused every one of them. The distinction that resolves it is *absence versus failure*: a project with no repository has no history a swept file could reach and no `git status` to quieten, so the block is unnecessary rather than unwritable. A `.git` that exists and cannot be written still refuses, so nothing is softened for a caller that actually has a repository. The alternative — refusing every non-git project outright — is a control that fails into unusability, which is the shape of control that gets switched off.
- **Two positions, one refusal variant.** `drive`'s chain answers the pure question before dispatch (is there a root; may this alias have an envelope), and `execute_run` answers the expensive one before the lock and the journal (can the four layers be written). Both return `DriveError::EnvelopeAssertionFailed`. Doing the whole establishment in `drive` would have required threading an `EstablishedEnvelope` through `dispatch` and would have moved the wiring off the single `ExecutionOptions` construction site — trading the plan's own "once, at one site" property for a tidier call graph.
- **The drive-time refusal deliberately emits nothing, and says so.** It sits before dispatch, so there is no run and no journal to park. Emitting nothing *silently* is what the comment prevents: the position states the three clauses, and records the forward constraint that if the assertion ever becomes reachable after a journal is open, `envelope::park` is the appender. A second appender written speculatively now would be a second thing to keep in step for a branch nothing reaches.
- **`ParkOutcome::Unresolvable` is unreachable and stays.** `read_active_run` validates the pointer, so a hostile `active` file becomes `NoActiveRun` before `run_paths` is called. Keeping the branch is what "handles the `None` rather than unwrapping" means in practice (D-28): it is the second of two layers, and an `unwrap` there would be one edit away from reinstating the WR-02 traversal. The test says which layer caught it, so a future weakening of the first layer shows up as a changed outcome rather than as a silent pass.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `write_exclude_block` refused every non-git project, breaking 7 integration tests**

- **Found during:** Task 2
- **Issue:** The plan's establishment list runs `hooks::write_exclude_block(project_root)` unconditionally. It resolves `.git` and errors with `"<root> is not a git repository"` otherwise. Seven `tests/driver_inbox.rs` tests (and the same shape elsewhere) drive runs against plain temp directories, and every one of them failed with `EnvelopeAssertionFailed`. Left as-is, the driver would refuse to run any registered project that is not a git repository.
- **Fix:** the block is written only when `project_root.join(".git").exists()` — the same pair (`.git` directory or linked-worktree pointer file) `hooks::git_dir` resolves, so the predicate and that function cannot disagree. The existence question is asked at the caller rather than inside `write_exclude_block`, because that function's failure *is* the refusal for every caller that has a repository, and weakening it there would weaken it for all of them.
- **Files modified:** `src/driver/run.rs`
- **Verification:** `cargo test --test driver_inbox` back to 7 passing; the reasoning is recorded in `establish_envelope`'s doc as a skip condition with the absence-versus-failure distinction spelled out
- **Committed in:** `29ee5e2`

**2. [Rule 2 - Missing Critical] `cred::RUN_ID_ENV` had no writer, so the per-run PR cap bucketed every run together**

- **Found during:** Task 2 (19-05's explicit handoff note)
- **Issue:** `hooks::current_run_id` reads `RUN_ID_ENV` and falls back to a shared `unattributed-run` placeholder. `build_env` cannot set it — it is given an alias and a project root, not a run — so until the driver folded it in, the per-run cap bounded *all* runs of an alias together. That over-counts rather than under-counting, so it was safe to ship in that order, but it is not the intended semantics.
- **Fix:** `EnvelopeEnv::with_run_id(run_id)`, called by the driver at the one seam that hands the environment to the spawn closure. A separate call rather than a `build_env` parameter, for the reason `RUN_ID_ENV`'s own doc already records.
- **Files modified:** `src/envelope/cred.rs`, `src/driver/run.rs`
- **Verification:** `tests/envelope_wiring.rs`'s `child_env` carries it and the guard fixtures exercise the per-run bucket; `cargo test --lib envelope::cred` green (20 pre-existing tests unchanged)
- **Committed in:** `29ee5e2`

**3. [Rule 1 - Bug] The test suite wrote envelope directories into the developer's real `~/.local/share`**

- **Found during:** Task 2
- **Issue:** Directly caused by this plan: driving a run now establishes an envelope, and `envelope_root()` resolves to `dirs::data_local_dir()` unless overridden. A full `cargo test` created `~/.local/share/gsd-meta-manager/envelope/{a,detached,locked,steered,stoppable,tracer}/` — hook stubs, generated git configs, askpass responders and settings files, under fixture alias names, in the developer's home. This repository's own code calls that out in as many words (`guard_in`: "a test may not write into the developer's real `~/.local/share`").
- **Fix:** `isolate_envelope_root()` in each of the six affected fixtures — a per-test-binary `OnceLock<TempDir>` whose initialiser sets `GSD_MM_ENVELOPE_ROOT` exactly once, held for the process lifetime so spawned child drivers inherit it too.
- **Files modified:** `tests/driver_inbox.rs`, `tests/driver_kill.rs`, `tests/driver_kill_startup.rs`, `tests/driver_lock.rs`, `tests/driver_optin.rs`, `tests/driver_reattach.rs`, `tests/driver_tracer.rs`
- **Verification:** `rm -rf ~/.local/share/gsd-meta-manager/envelope && cargo test` leaves the directory absent; all 986 tests pass
- **Committed in:** `29ee5e2`

**4. [Rule 2 - Missing Critical] Three refusal paths left no trace at all (T-19-56)**

- **Found during:** Task 4
- **Issue:** The plan enumerates the refusal branches in `pre_push`, `pre_commit` and `guard` that classify a *command*, but three refusals fire **before** any classification and would have been the only refusals in the phase with no journal record: `assert_provenance` failing in `pre_push` and in `pre_commit` (the hook stub was relocated — precisely the tampering D-10 closes), and `guard`'s own `envelope_root()` failure. A refusal nobody can audit after the fact is the exact shape T-19-56 names.
- **Fix:** all three park under `ParkReason::EnvelopeAssertionFailed` before returning. The provenance ones use `inspect_err` so the park is attached to the failure path without restructuring the `?`.
- **Files modified:** `src/envelope/hooks.rs`
- **Verification:** `awk '/^pub fn pre_push/,/^}/' | grep -c 'park'` → 3; the same for `pre_commit` → 2 and `guard` → 4; `cargo test --test envelope_hook_refusals` and `--test envelope_tracer` still green (the provenance-refusal fixtures)
- **Committed in:** `3e37cb4`

**5. [Process] `grep -c 'ExecutionOptions::default()' src/driver/run.rs` no longer measures what it was written to measure**

- **Found during:** Task 2, checking acceptance criteria
- **Issue:** The criterion expects `1` — "still a single production construction site". Filling the three envelope fields turns `ExecutionOptions::default()` into `ExecutionOptions { …, ..Default::default() }`, so the literal now appears **0** times in production and 3 times inside `mod tests`. The criterion's *intent* holds exactly; its literal form measures a spelling that the change necessarily altered.
- **Fix:** reported anchored instead. `head -1589 src/driver/run.rs | grep -c 'options = ExecutionOptions'` → **1** (line 1148), and the three `ExecutionOptions::default()` hits are all past `#[cfg(test)]` at line 1590. No production code changed.
- **Files modified:** none
- **Committed in:** n/a (criterion interpretation)

**6. [Process] The permission-bypass negative grep already matched two pre-existing doc lines**

- **Found during:** Task 1
- **Issue:** `grep -rnE 'dangerously-skip-permissions|bypassPermissions' src/` is specified to return no match, but `src/executor/mod.rs:256-257` has carried a doc comment saying *"`bypassPermissions` and `--dangerously-skip-permissions` are never emitted on the host"* since before Phase 19 (confirmed present at `740e62f`). The literal grep therefore matched the sentence that promises the property. This is the same shape as 19-02's deviation 4 and 19-05's deviation 7.
- **Fix:** reported filtered to code lines — `grep -rnE '…' src/ | grep -vE ':[[:space:]]*(///|//!|//)'` → **0**. The property is additionally held by a compile-time proof rather than a grep: `the_permission_mode_enum_has_exactly_one_variant_and_it_is_not_a_bypass` matches on the enum, so a bypass variant fails to compile the test. New assertion literals were assembled from `concat!` fragments, following the discipline the existing bypass test already records, so the tests cannot be what makes a guard report the tokens' presence.
- **Files modified:** none (the fix is the added compile-time test, in `668af7e`)
- **Committed in:** n/a (criterion interpretation)

**7. [Design] The `secret_detected` row is proved at the push boundary, not at commit**

- **Found during:** Task 4
- **Issue:** The plan's behaviour row reads "A commit staging a file matching the secret table exits non-zero and lands one `Parked` event with reason `secret_detected`". As 19-03 shipped it, `pre_commit` has **no** secret scan — it applies `policy::forbidden_repo_path` to the staged list. The full-worktree credential scan is `pre_push`'s, deliberately: it is the backstop that catches a commit made with the verification step suppressed, which `pre_commit` by construction never saw.
- **Fix:** the row is proved where the producer actually is — a real `git push` whose worktree carries a planted PEM, refused by the generated `pre-push` stub with `secret_detected`, parked, and with the remote ref unchanged. Adding a scan to `pre_commit` was declined: it would be scope creep into 19-03's design, it would double the scan cost on every commit, and the boundary that matters for a credential is the one where it leaves the machine.
- **Files modified:** none (test placement only)
- **Verification:** `tests/envelope_wiring.rs#a_worktree_carrying_a_credential_is_refused_and_lands_a_secret_detected_park`, which also asserts the refusal does not reproduce the secret it blocked
- **Committed in:** `3e37cb4`

**8. [Process] `ParkOutcome::Unresolvable` is unreachable, and the row asserting it was rewritten to say so**

- **Found during:** Task 4
- **Issue:** The plan's behaviour row expects a traversing `active` pointer to yield `Unresolvable`. It cannot: `journal::writer::read_active_run` validates the pointer (the WR-02 read-side fix) and answers `None`, so `park_at` returns `NoActiveRun` before `run_paths` is ever called. Asserting `Unresolvable` there would have required either duplicating the validation or weakening `read_active_run`.
- **Fix:** the test asserts the honest outcome — four hostile pointers each yield `NoActiveRun`, append nothing, and follow nothing out of the project — with a message naming which of the two layers caught it, so a future weakening of the first layer surfaces as a changed outcome rather than a silent pass. `Unresolvable` is kept and documented as the second layer, because D-28's requirement is that `run_paths`' `None` is *handled*, and an `unwrap` there would be one edit from reinstating the traversal.
- **Files modified:** `src/envelope/mod.rs`
- **Committed in:** `47a7015` / `3e37cb4`

**9. [Design] `main.rs` gained nothing, deliberately**

- **Found during:** Task 4
- **Issue:** The plan says the envelope dispatch arm should gain "the print-and-exit posture applied to the park outcome, so the second stderr line reaches the same place the first one does".
- **Fix:** it already does, without a change. `park` writes its own second stderr line, and that *is* the same stderr the `Envelope` arms' `eprintln!` writes to — same process, same handle, same destination. Adding a print site in `main.rs` would have meant plumbing the `ParkOutcome` back through three `-> anyhow::Result<i32>` signatures to produce a duplicate of a line that is already emitted, and a second print site is a second place the same line can be emitted. Writing the line inside the one appender is also what makes "silence is forbidden" a property rather than a per-caller discipline.
- **Files modified:** none
- **Committed in:** n/a

**10. [Rule 3 - Blocking] `outcome_label(&outcome)` had to leave the terminal path entirely**

- **Found during:** Task 4, checking acceptance criteria
- **Issue:** The criterion requires `grep -n 'outcome_label(&outcome)' src/driver/run.rs` to have no non-comment hits. The first implementation kept it as the pre-computed fallback for a `spawn_blocking` join failure, which satisfied the intent (neither `finish` call site passes it) while failing the letter.
- **Fix:** both sites now clone the outcome into the task and **re-run `terminal_label` inline** on a join failure — which is `driver::drive`'s dry-run arm's own answer to the same question and this repository's established one. It is also strictly better: the fallback now still reads the journal, so a park is not lost merely because a task failed to join.
- **Files modified:** `src/driver/run.rs`
- **Verification:** `grep -n 'outcome_label(&outcome)' src/driver/run.rs` → no match; `cargo test --lib driver::run` → 19 passing
- **Committed in:** `3e37cb4`

---

**Total deviations:** 10 (3 missing-critical/bug, 2 blocking, 2 design, 3 process)
**Impact on plan:** No scope creep. Deviations 1, 2, 3 and 4 each close a way this plan could have shipped a boundary that passed its own acceptance criteria while being wrong — a driver that refused every non-git project, a per-run cap silently unenforced, a test suite writing into the developer's home, and three refusals leaving no trace. Deviations 7 and 8 correct two behaviour rows against what earlier plans actually shipped rather than implementing what the rows assumed. Deviations 5, 6, 9 and 10 changed no behaviour except deviation 10, which made the fallback strictly more correct.

## Issues Encountered

None outside the deviations above. The RED run surfaced no bugs of its own: the refusals themselves already worked, which is exactly what the RED step was designed to isolate.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo test` | exit 0 — **986 passing, 0 failed** (baseline 960 after 19-06; +26) |
| `cargo test --lib executor::claude` | exit 0 — 19 tests (was 14) |
| `cargo test --lib driver::run` | exit 0 — 19 tests (was 16) |
| `cargo test --lib envelope` | exit 0 — 145 tests (was 141) |
| `cargo test --lib journal` | exit 0 — 86 tests, `every_reserved_kind_is_declared_and_none_is_emitted_by_this_phase` present with assertion bodies unmodified |
| `cargo test --test envelope_wiring` | exit 0 — **14 tests** (≥ 5 required) |
| `cargo test --test envelope_hook_refusals` | exit 0 — 7 tests, fixtures reused not forked |
| `cargo test --test envelope_pr_cap` | exit 0 — 11 tests, fixtures reused not forked |
| `cargo test --test spawn_seam_guard` | exit 0 — 7 tests, allowlist unchanged |
| `cargo clippy -- -D warnings` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | **exactly 5** pre-existing lints — `browser.rs:131/132/133`, `project_creator.rs:146`, `state_reader/mod.rs:258`. Count and locations unchanged. Measured with `rtk proxy` per D-34. |
| `grep -c 'disallowedTools' src/executor/claude.rs` | 4 (≥ 2 required) |
| `grep -rnE 'dangerously-skip-permissions\|bypassPermissions' src/` filtered to code lines | **0** (see deviation 6) |
| `grep -n 'ExecutionTarget::Host => {}' src/executor/claude.rs` | matches — the container tripwire is intact |
| production `ExecutionOptions` construction sites in `src/driver/run.rs` | **1** (line 1148; see deviation 5) |
| `grep -rc 'DrivableProject::from_registry' src/` production call sites | **1** (`src/driver/mod.rs:200`) |
| `awk '/let mut wrap = CommandWrap::with_new/,/^\s*\}\);/' src/executor/claude.rs \| grep -c 'env_remove'` | **2** — the scrub plus the envelope removals, both in the one closure |
| `grep -rn 'cmd.env(' src/ \| grep -v 'src/executor/claude.rs'` | no match — one place builds the child's environment |
| `grep -c 'spawn_blocking' src/driver/run.rs` | 9 (≥ 1 required) |
| `awk` over `RESERVED_KINDS` for `"parked"` / over `EMITTED_KINDS` for `"parked"` | **0 / 1** — a move, not a duplication |
| `git diff 740e62f -- src/journal/mod.rs \| grep -c 'Schema only in this phase'` | **2** on the removed side |
| `awk '/^pub fn pre_push/,/^}/' \| grep -c 'park'` (same for `pre_commit`, `guard`) | **3 / 2 / 4** (≥ 1 each) |
| `grep -c 'GSD_MM_ENVELOPE_PROJECT_ROOT' src/envelope/mod.rs` | 4 (≥ 1 required); the literal-string `env::var` grep finds **0** readers outside it |
| `grep -n 'run_paths' src/envelope/mod.rs` filtered to code lines \| `grep -c 'unwrap\|expect'` | **0** |
| `grep -c 'fn terminal_label' src/driver/run.rs` | 1 |
| `grep -n 'outcome_label(&outcome)' src/driver/run.rs` | no match — both `finish` sites pass `terminal_label` |
| `git diff 740e62f -- src/cli.rs \| grep -cE '^\-.*EnvelopeAction'` | **0** — no action variant's shape changed |
| `git diff 740e62f -- src/driver/spawn.rs \| grep -cE '^\+.*fn drive_argv'` | **0** — no new argument was needed |
| `grep -c '#[test]' tests/envelope_wiring.rs` / `wc -l` | 14 / **976** (≥ 260 required) |
| `grep -rniE '(agent\|model).{0,40}(summary\|said\|reported)' tests/envelope_wiring.rs` | 3 matches, **all on comment lines**, none inside an assertion |
| `Cargo.toml` unchanged | **no crate added** — T-19-SC holds phase-wide |

**RED-step evidence (committed, not reverted):** at `47a7015` the park decision is stubbed and **9 rows fail** — 2 in `envelope::tests`, 2 in `driver::run::tests`, 5 in `envelope_wiring`. All pass at `3e37cb4`. The rows passing at both are the ones that must pass against the stub: the no-active-run row, the traversing-pointer row, the permitted-command pairing, and every exit-code assertion.

## Pre-existing Failures (out of scope)

- **`tests/driver_reattach.rs`** — the two tracked intermittent failures reproduced in 2 of ~6 full-suite runs during this plan and were green in the final 3 consecutive runs. Signature identical to the recorded one (both messages verbatim; the file finishes in ~0.53s instead of ~6.1s). `git diff c107e04 -- tests/driver_reattach.rs` shows only the envelope-root isolation helper. **One honest observation for the tracker:** this plan lengthens the window between "the driver process is live" and "`run.json` exists", because envelope establishment now runs before the lock and the journal. It does not change the root cause — `live_within` synchronises on the process rather than the artifact — but it plausibly widens the race, which strengthens rather than weakens the recorded fix direction.
- **`tests/envelope_tracer.rs#a_relocated_copy_of_the_stub_refuses_instead_of_acting`** — observed **once**, with `ExecutableFileBusy` ("Text file busy") when the fixture execs a stub it has just copied. `git diff c107e04 -- tests/envelope_tracer.rs` is **empty**: this plan touched neither the file, the stub generator, nor `assert_provenance`. Logged to `deferred-items.md` with a fix direction (drop or `sync_all` the copy's handle before exec, or retry on `ExecutableFileBusy`; do not serialise the suite).

## Known Stubs

None. Every symbol this plan introduces is exercised by a passing test.

Two things that look like stubs and are not, stated rather than left to be discovered:

- **`ParkOutcome::Unresolvable` has no reachable producer today.** It is the second of two layers guarding `run_paths`' `Option`, kept because D-28 requires that `None` be handled rather than unwrapped. Its doc says it is unreachable and why. See deviation 8.
- **`ParkOutcome::Failed` is produced only by a journal that will not accept a write**, which no fixture forces. The path is a plain `match` on `JournalWriter::open`/`append`, both of which are exercised by `src/journal/writer.rs`'s own tests.

## Threat Flags

None new. No network endpoint was added, no schema changed (`Parked` was already declared), and **no crate was added to `Cargo.toml`**.

Against this plan's own register:

| Threat | Disposition | Where it is closed |
|---|---|---|
| T-19-44 a run starting with a partial envelope | mitigated | establishment is all-or-nothing; failure is `DriveError::EnvelopeAssertionFailed` returned before the lock and before the journal, proved in and out of process with nothing on disk after it |
| T-19-45 permission-bypass flag reintroduced | mitigated | the pinned argv vector, an **exhaustive match** on `PermissionMode` (a bypass variant fails to compile), and the code-line negative grep |
| T-19-46 child environment set outside the one closure | mitigated | `grep -rn 'cmd.env(' src/` finds one file; `EnvelopeEnv` arrives as a pure value so the closure is the only applier |
| T-19-47 success derived from the agent's prose | mitigated | every assertion reads an exit code, a file, a git ref or a journal event; the prose grep confines matches to the file header |
| T-19-48 blocking envelope establishment inside an `async fn` | mitigated | one `spawn_blocking` with the clone-not-move detail; both terminal-label reads likewise |
| T-19-49 a second capability-token construction site | mitigated | `tests/spawn_seam_guard.rs` green with the allowlist unchanged; the criterion re-asserted after the chain was extended |
| T-19-50 envelope paths in argv visible through the process table | **accepted** | the settings path and the hooks directory are not secrets; the token is never on argv (D-17) |
| T-19-56 a refusal that leaves no trace | mitigated | every refusal branch in `pre_push`, `pre_commit` and `guard` calls the one appender — including the three pre-classification refusals the plan did not enumerate (deviation 4) — and four named tests read the event off disk from a process other than the one that refused |
| T-19-57 child unsets the locator to suppress its own park | **accepted and stated** | refusal is decided before `park` is called; `a_re_entry_with_the_locator_absent_still_refuses_and_says_it_could_not_park` asserts the exit code and reason are unchanged and that a `NOT recorded` line is emitted |
| T-19-58 a park appended to the wrong run's journal | mitigated | the `active` pointer is the only resolver, the per-project advisory lock means at most one live run, a cleared pointer yields `NoActiveRun`, and four hostile pointer shapes are asserted to append nothing and follow nothing |
| T-19-SC package-manager installs | mitigated | `Cargo.toml` untouched |

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Ready for 19-08 (the gate).** Two things it should expect. First, `tests/async_blocking_guard.rs` will see the **inline `terminal_label` re-run** in both `finish` paths and the inline `dry_run::build_report` re-run that already exists in `driver::drive` — both are deliberate join-failure fallbacks in the established house shape, and both belong on that guard's allowlist with the reason recorded, not "fixed". Second, `establish_envelope` is synchronous by design and is called only from inside `spawn_blocking`; its doc says so, but a lexical scanner cannot see through the closure, so it may need naming.
- **The park vocabulary is complete and stable for Phase 20.** `envelope::park`/`park_at` is the single appender, `ParkReason::as_str` is the single taxonomy, and the terminal record carries `parked:<reason>`. A router can read the run's outcome and branch without re-deriving anything and without opening the journal.
- **Two test-suite hygiene items for whoever touches the driver next.** `isolate_envelope_root()` must be called by any new fixture that drives a run, or it will write into the developer's real data directory. And `tests/envelope_tracer.rs`'s ETXTBSY race is logged in `deferred-items.md`.
- **`REQUIREMENTS.md`, `STATE.md` and `ROADMAP.md` were NOT touched** — parallel worktree mode; the orchestrator owns those writes. All five requirements this plan declares are shared with sibling plans, so the shared-ID gate holds them until the last declaring plan lands.
- **No blockers.**

## Self-Check: PASSED

- Created file present on disk: `tests/envelope_wiring.rs` (976 lines). All 16 modified files present in `git diff c107e04 --stat`.
- All six commits present in `git log`: `668af7e`, `29ee5e2`, `ae9086f`, `47a7015`, `3e37cb4`, `b7acce3`.
- Every task `<acceptance_criteria>` re-run and passing, with three reported anchored and one behaviour row reported against what earlier plans actually shipped (deviations 5, 6, 8 and 7 respectively).
- The plan-level `<verification>` re-run and passing, including the `rtk proxy` clippy-delta measurement: exactly 5 pre-existing lints at unchanged locations.
- `must_haves.artifacts` confirmed: `src/executor/claude.rs` contains `--disallowedTools`; `src/driver/mod.rs` contains `envelope_assertion_failed`; `src/envelope/mod.rs` contains `pub fn park_at`; `src/envelope/hooks.rs` contains `park`; `tests/envelope_wiring.rs` is 976 lines (≥ 260).
- `must_haves.key_links` confirmed, each greppable by the pattern the plan names: `envelope::hooks::install` in `src/driver/run.rs`; `EnvelopeEnv` in `src/executor/claude.rs`; `JournalEvent::Parked` in `src/driver/mod.rs`; `envelope::park` in `src/envelope/hooks.rs`; `read_active_run` in `src/envelope/mod.rs`.
- `STATE.md`, `ROADMAP.md` and `REQUIREMENTS.md` deliberately untouched — parallel worktree mode.

---
*Phase: 19-gitsafe-git-blast-radius-envelope*
*Completed: 2026-08-18*
