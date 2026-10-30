---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 02
subsystem: infra
tags: [git, argv, denylist, force-push, core.hooksPath, config-migration, serde, safety-envelope]

# Dependency graph
requires:
  - phase: 19-gitsafe-git-blast-radius-envelope
    provides: "plan 19-01's `src/envelope/policy.rs` — `PushVerdict`, `classify_push_ref`, `validate_namespace`, `default_namespace` — which this plan extends rather than parallels"
  - phase: 17-driver-supervisor
    provides: "`DriverOptIn`, the `#[serde(default)]` field-with-migration posture, and `config.rs`'s byte-literal migration-fixture discipline"
  - phase: 14-driver-preview
    provides: "`state_reader::git_ops::push_refspecs`, which already resolves the implicit push destination with no network and no credential"
provides:
  - "`envelope::policy::classify_git(argv, ctx) -> GitVerdict` — D-08's whole denied set as one pure function over argv, with no path, no file and no process"
  - "`envelope::policy::ParkReason` and `as_str` — D-24's seven-member taxonomy, the single source of the identifiers Phase 20 reads"
  - "`envelope::policy::resolve_push_context` — the implicit push destination, resolved by reusing `git_ops::push_refspecs`"
  - "`envelope::policy::disallowed_tools()` — layer 1's `--disallowedTools` patterns, including `Write`/`Edit` on `.claude/**`"
  - "`envelope::policy::EnvelopePolicy` and `resolve(alias, opt_in)` — the one place every envelope default is decided"
  - "`config::CredentialSource` — an env var or an argv command, and deliberately never a literal token"
  - "Four `#[serde(default)]` `DriverOptIn` fields, with a pre-Phase-19 config literal proving they load absent"
affects: [19-05 PreToolUse guard, 19-06 wiring, 19-07 honesty text, 19-08 gate, 20 router]

# Actuals (#2632)
actuals:
  tokens: 15800
  tasks: 2
  commits: 3

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Compiling RED: the failing-test commit ships the real API with the decision function stubbed to `Allow`, so the RED step both compiles and proves every refusal load-bearing — a stronger and permanently-committed form of the mutation check 19-01 ran by hand and reverted."
    - "Reason-constant-behind-enum: `ParkReason::as_str` returns the `REASON_*` constants rather than fresh literals, so `grep REASON_FORCE_PUSH_BLOCKED` finds every producer and the enum and the constant cannot disagree."
    - "The closed taxonomy states its own overflow: refusals with no dedicated reason (`stash`, `update-ref`) park under `ForcePushBlocked` as the destructive-git family, and the doc says so rather than leaving a reader to infer it from a surprising string."

key-files:
  created: []
  modified:
    - src/envelope/policy.rs
    - src/config.rs
    - src/registry.rs
    - src/executor/mod.rs
    - src/driver/mod.rs
    - src/driver/dry_run.rs
    - tests/driver_tracer.rs
    - tests/driver_reattach.rs
    - tests/driver_lock.rs
    - tests/driver_kill_startup.rs
    - tests/driver_dry_run.rs
    - tests/driver_kill.rs
    - tests/driver_inbox.rs
    - tests/journal_run_paths.rs

key-decisions:
  - "The RED commit ships a compiling always-`Allow` stub rather than non-compiling tests, so the failure is real, the history stays buildable, and the proof that each refusal is load-bearing is committed instead of performed and reverted."
  - "An unqualified push destination is qualified as `refs/heads/<name>` before classification, because leaving `git push origin main` unrecognised would have made it silently allowed — the exact shape of failure a denylist dies from."
  - "`git config --edit` is refused even though it names no key: it opens the whole file for writing and can therefore set `core.hooksPath` without ever spelling it."
  - "`resolve_push_context`'s doc deliberately does not repeat the four config keys `git_ops::push_refspecs` already enumerates; a second copy of the list is a second thing to keep in step, and the acceptance grep for re-derivation then means what it says."
  - "`CredentialSource::Command` carries `argv: Vec<String>`, never a shell string — with no shell in the path, a credential lookup cannot become an arbitrary-command seam."

patterns-established:
  - "Argv-shape rules as named test rows: each denied form and its near-miss spelling (`--force` vs `--force-with-lease=<v>`, `-f` bundled vs standalone, `config --get` vs `config --global`, `symbolic-ref` read vs write) is its own assertion, so a classifier that passes cannot be passing by accident of shape."
  - "Every refusal test is paired with an allow test from the same verb family, so a classifier that refuses everything fails."
  - "Value-taking flags are consumed explicitly (`-o`, `--push-option`, `--repo`, `--receive-pack`, `--file`, `--type`), because a value read as an operand is how a denylist both false-refuses and false-allows."

requirements-completed: []  # SAFE-02 is declared by 19-02, 19-05, 19-06, 19-07 and 19-08; the shared-ID gate holds it until the last declaring plan produces a SUMMARY.

coverage:
  - id: D1
    description: "`git push --force`, `-f`, bundled `-fu`, `--force-with-lease` with and without `=value`, `--force-if-includes`, `--mirror`, `--delete` and `-d` are each classified as refused with reason `force_push_blocked`"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/envelope/policy.rs#every_force_push_spelling_is_refused"
        status: pass
      - kind: other
        ref: "the RED commit 1df3db7 runs these same rows against an always-Allow classifier and they fail — the refusal is proved load-bearing in committed history, not by a reverted mutation"
        status: pass
    human_judgment: false
  - id: D2
    description: "A `+`-prefixed refspec is refused — a force push spelled as a refspec rather than as a flag"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/envelope/policy.rs#a_plus_prefixed_refspec_is_refused_because_it_is_a_force_push_by_another_spelling"
        status: pass
    human_judgment: false
  - id: D3
    description: "`git push --no-verify` is refused with reason `hook_bypass_blocked`, because it is the flag that makes the pre-push hook unreachable"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/envelope/policy.rs#push_no_verify_is_refused_as_a_hook_bypass"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every writing form of `git config` touching `core.hooksPath` is refused at every scope (`--global`, `--system`, `--local`, `--worktree`, `--file`), in git 2.46's `set`/`unset` subcommand spellings, and at any casing git accepts"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/envelope/policy.rs#every_writing_form_of_config_touching_core_hookspath_is_refused_at_every_scope"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#reading_core_hookspath_is_allowed_because_a_read_is_not_a_write"
        status: pass
    human_judgment: false
  - id: D5
    description: "The `git -c core.hooksPath=… <verb>` and `--config-env` command-line forms — the ones that outrank the envelope's env-injected setting — are refused before the verb is even dispatched"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/envelope/policy.rs#the_command_line_config_form_that_outranks_the_envelope_is_refused"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#a_leading_dash_c_that_sets_an_unrelated_key_does_not_refuse_by_itself"
        status: pass
    human_judgment: false
  - id: D6
    description: "`git stash` is refused in every subcommand form, and `update-ref`, `reflog delete`, `reflog expire`, `filter-branch`, `filter-repo` and `symbolic-ref` writes are refused as the history-rewriting family"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/envelope/policy.rs#every_stash_form_is_refused_because_recovery_runs_through_git_fsck"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#the_history_rewriting_family_is_refused"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#a_read_from_the_same_verb_family_is_allowed"
        status: pass
    human_judgment: false
  - id: D7
    description: "A `git push` with no refspec is judged against the destination `git_ops::push_refspecs` resolves; a destination that cannot be resolved is refused, never allowed"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/envelope/policy.rs#a_push_with_no_refspec_is_judged_against_the_resolved_destination"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#a_push_whose_destination_cannot_be_resolved_is_refused_never_allowed"
        status: pass
      - kind: integration
        ref: "src/envelope/policy.rs#an_unresolvable_repository_yields_no_destination_rather_than_a_guess (drives the impure resolver against a real non-repository directory)"
        status: pass
    human_judgment: false
  - id: D8
    description: "A git verb not on the denylist is allowed, so an unanticipated plumbing command cannot break a GSD skill"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/envelope/policy.rs#an_unanticipated_plumbing_verb_is_allowed_because_this_is_a_denylist"
        status: pass
      - kind: other
        ref: "grep: `classify_git`'s doc contains both `denylist` and `allowlist`, naming the declined stronger alternative, its cost, and the Phase 20 revisit condition"
        status: pass
    human_judgment: true
    rationale: "A grep proves the words are present. Whether the doc reads as an honest record of a declined alternative — rather than as a hedge that pre-excuses every future gap — is a judgement only a reader can make, and the transparency prohibition's own verification field is `judgment`."
  - id: D9
    description: "A pre-Phase-19 v2 config whose `driver_opt_in` carries only `opted_in_at` and `claude_md_digest` loads unchanged, with all four envelope fields absent and `CONFIG_SCHEMA_VERSION` unbumped"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/config.rs#a_pre_phase_19_config_loads_with_every_envelope_field_absent"
        status: pass
      - kind: other
        ref: "grep -n 'pub const CONFIG_SCHEMA_VERSION: u32 = 2;' src/config.rs still matches"
        status: pass
    human_judgment: false
  - id: D10
    description: "Every envelope default is resolved in exactly one function, `EnvelopePolicy::resolve`, and an invalid configured namespace degrades to the safe default rather than widening the control"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/envelope/policy.rs#an_invalid_configured_namespace_falls_back_to_the_default_rather_than_widening"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#pr_caps_default_to_three_and_one_from_an_absent_key_and_from_an_explicit_none"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#an_unconfigured_project_resolves_to_the_default_namespace_and_the_default_caps"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#a_valid_configured_namespace_is_honoured_verbatim"
        status: pass
    human_judgment: false
  - id: D11
    description: "`CredentialSource` names an environment variable or an argv command and has no literal-token variant by construction; the default is no credential"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/config.rs#a_credential_source_round_trips_through_self_describing_json"
        status: pass
    human_judgment: true
    rationale: "The test proves the two variants parse and that the JSON is self-describing. It cannot prove the absent third variant stays absent — that is a design commitment recorded in the type's doc comment, and only a reviewer can confirm the doc states the reason (the registry file has no protection posture) rather than merely stating the rule."

# Metrics
duration: 20 min
completed: 2026-08-18
status: complete
---

# Phase 19 Plan 02: `classify_git` and the Envelope Config Surface Summary

**D-08's entire denied set — every force-push spelling, `--no-verify`, `+`-refspecs, `core.hooksPath` writes at every scope including the `git -c` form that outranks the envelope, `stash`, and the history-rewriting family — is now one pure function over argv, proved by 27 unit tests that need no repository, no process and no network; and the envelope's four `#[serde(default)]` config fields load absent from a pre-Phase-19 config literal.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-08-18T11:26:00-06:00 (base commit `f1089ac` at 11:25:24)
- **Completed:** 2026-08-18T11:45:00-06:00
- **Tasks:** 2 (Task 1 executed as RED → GREEN)
- **Files modified:** 14

## Accomplishments

- **The whole SAFE-02 decision surface is one function, and every row of it is a named test.** `classify_git(argv, ctx)` takes no path, opens no file and spawns no process — `grep -n 'Command::new' src/envelope/policy.rs` returns nothing — which is exactly what let the denied set be proved exhaustively instead of by spawning git thirty times.
- **The refusals are proved load-bearing in committed history, not by a reverted mutation.** The RED commit `1df3db7` ships the real API with the classifier stubbed to `Allow`; thirteen tests fail there and pass at `05f2696`. That is a permanent, re-runnable version of the hand-executed mutation check 19-01 performed and reverted — `git stash`-free, and available to anyone auditing the phase later.
- **The near-miss spellings are individually asserted**, which is the flagged-assumption edge this plan was built on: `-f` standalone and bundled inside `-fu`; `--force-with-lease` bare and `=value`; `config --get` allowed against `config --global` refused; `symbolic-ref --short HEAD` allowed against `symbolic-ref HEAD refs/heads/x` refused; `reflog show` allowed against `reflog expire` refused.
- **The implicit push destination is resolved by reuse, never re-derivation.** `resolve_push_context` calls `git_ops::push_refspecs` and reads the `<dst>` half. An empty result is a refusal: `grep -nE 'branch\.\{|remote\.pushDefault|push\.default' src/envelope/policy.rs` returns no match, so there is provably no second implementation to drift from the preview the user was shown.
- **Every envelope default is decided in one function**, and an invalid configured namespace degrades to the safe default with a warning rather than widening. A user who types `refs/heads/` has disabled the boundary by typo; `EnvelopePolicy::resolve` refuses to honour that.
- **The migration is proved by a byte literal, not a round-trip.** `PRE_PHASE_19_CONFIG` is written the way the pre-Phase-19 binary wrote it; all four new fields come back `None` and `CONFIG_SCHEMA_VERSION` stays 2.

## Task Commits

1. **Task 1 (RED): the full denied set as failing tests against an always-`Allow` classifier** — `1df3db7` (test)
2. **Task 1 (GREEN): `classify_git` — D-08's denied set as one pure function over argv** — `05f2696` (feat)
3. **Task 2: four defaulted `DriverOptIn` fields, `CredentialSource`, `EnvelopePolicy::resolve`, the twelve-site literal sweep and the migration literal** — `c78117e` (feat)

_No REFACTOR commit: the GREEN implementation needed no cleanup pass, and an empty `refactor` commit would be a ceremony rather than a change._

## Files Created/Modified

- `src/envelope/policy.rs` — `ParkReason` + five new `REASON_*` constants + `as_str`; `GitVerdict`; `GitContext`; `classify_git` and its per-verb helpers (`classify_push`, `classify_config`, `classify_reflog`, `classify_symbolic_ref`, `leading_git_option`, `qualify_destination`); `resolve_push_context`; `disallowed_tools`; `EnvelopePolicy` + `resolve` + `DEFAULT_PR_CAP_PER_24H`/`DEFAULT_PR_CAP_PER_RUN`; 27 tests
- `src/config.rs` — four `#[serde(default)]` `DriverOptIn` fields, `CredentialSource`, `PRE_PHASE_19_CONFIG` and its two tests
- `src/registry.rs` — the production opt-in write path names all four fields as `None`, with the "breakage is the feature" reason recorded at the line
- `src/executor/mod.rs`, `src/driver/mod.rs`, `src/driver/dry_run.rs` — the three in-`src` test literals
- `tests/driver_tracer.rs`, `tests/driver_reattach.rs`, `tests/driver_lock.rs`, `tests/driver_kill_startup.rs`, `tests/driver_dry_run.rs`, `tests/driver_kill.rs`, `tests/driver_inbox.rs`, `tests/journal_run_paths.rs` — the eight test literals

## Decisions Made

- **A compiling RED.** The executor contract calls for a `test(...)` commit that fails before the `feat(...)` commit that passes. Committing non-compiling tests would have left a build-broken commit in history and made `git bisect` lie. Shipping the real signatures with the classifier stubbed to `Allow` gives a genuine RED (13 failures, for the right reason), a buildable history, and — the reason it is better than the alternative — a permanent artifact proving each refusal is load-bearing, which 19-01 could only demonstrate by mutating and reverting.
- **An unqualified destination is qualified before classification.** `git push origin main` names a destination git resolves to `refs/heads/main`. A classifier that only understood fully-qualified refs would have found nothing to judge and allowed it. Qualifying as `refs/heads/<name>` is what closes that, and it is asserted directly.
- **`config --edit` is refused.** It writes the whole file without naming a key, so a key-based check would let it through. The alternative — allowing it because it does not name `core.hooksPath` — is a check that inspects the argument instead of the effect.
- **The four config keys `push_refspecs` resolves are named in *its* doc, not repeated in mine.** The plan's acceptance criterion greps `policy.rs` for those key names to prove no second implementation exists. Repeating them in a doc comment would have made that grep fire on prose and mean nothing. Pointing at the single source keeps the grep honest and the list singular — which is the same argument the reuse itself rests on.
- **`ForcePushBlocked` carries the destructive-git family.** D-24 fixes the taxonomy at seven members and `stash`/`update-ref`/`filter-branch` have no dedicated one. Rather than invent an eighth at a call site — which is how a taxonomy stops being one — they park under `ForcePushBlocked` with the specific command in `detail`, and the enum's doc states that mapping outright so a reader meeting `force_push_blocked` on a stash is not misled.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Refused `git config --edit`/`-e` and the `edit` subcommand**

- **Found during:** Task 1 (`classify_config`)
- **Issue:** D-08 enumerates the writing forms by scope (`--global`, `--system`, `--local`, `--worktree`, `--file`) and the plan's behavior table follows it. All of those name the key. `git config --global --edit` names no key and writes the same file, so a key-matching classifier allows it and the agent can set `core.hooksPath` inside the editor session.
- **Fix:** `--edit`, `-e` and the `edit` subcommand refuse with `hook_bypass_blocked` and a detail saying why a keyless write is still a `core.hooksPath` write.
- **Files modified:** `src/envelope/policy.rs`
- **Verification:** covered by `every_writing_form_of_config_touching_core_hookspath_is_refused_at_every_scope`'s sibling path; refusing more than the table requires cannot break an allow assertion, and the allow test (`reading_core_hookspath_is_allowed_because_a_read_is_not_a_write`) still passes, which is what proves it did not over-refuse reads.
- **Committed in:** `05f2696`

**2. [Rule 2 - Missing Critical] Refused an empty-half refspec (`:refs/heads/x`, `refs/heads/x:`)**

- **Found during:** Task 1 (`classify_push`)
- **Issue:** `git push origin :refs/heads/x` deletes a remote ref without ever spelling `--delete` or `-d`. The behavior table covers the flag spellings only, so a classifier built strictly to it would refuse the flag and allow the refspec that does the same thing.
- **Fix:** a refspec with an empty source or an empty destination refuses under `force_push_blocked`, with the detail naming the deletion.
- **Files modified:** `src/envelope/policy.rs`
- **Verification:** the qualification and namespace assertions in `a_push_inside_the_namespace_is_allowed_and_one_outside_it_is_refused` still pass, so the extra rule does not shadow the namespace path.
- **Committed in:** `05f2696`

**3. [Rule 3 - Blocking] Consumed value-taking flags explicitly instead of treating every non-flag token as an operand**

- **Found during:** Task 1 (`classify_push`, `classify_config`)
- **Issue:** `git push -o ci.skip origin` would have read `ci.skip` as the repository and `origin` as a refspec, refusing a legitimate push; `git config --file other.cfg core.hooksPath /tmp/x` would have read `other.cfg` as the key and allowed the write. Both directions of error are reachable from the same omission.
- **Fix:** `PUSH_VALUE_OPTS` and `CONFIG_VALUE_OPTS`, plus inline-value (`--opt=value`) detection and short-bundle handling for `-o`.
- **Files modified:** `src/envelope/policy.rs`
- **Verification:** `a_push_option_value_is_never_mistaken_for_a_refspec`, plus the `--file` row in the config test.
- **Committed in:** `05f2696`

**4. [Process] The doc for `resolve_push_context` names its source function instead of repeating the four config keys**

- **Found during:** Task 1, checking the acceptance criteria
- **Issue:** The plan's action text says to name `branch.<b>.remote`, `remote.pushDefault`, `remote.<r>.push` and `push.default` as the things not to re-derive; its acceptance criterion greps for exactly those strings and requires no match. Written literally, the doc would have failed the criterion that was meant to prove the doc's own claim.
- **Fix:** the doc points at `git_ops::push_refspecs`, whose own doc enumerates the keys in order, and says explicitly that the names are not repeated because a second copy of the list is a second thing to keep in step. The criterion now means what it says: a match is a re-derivation.
- **Files modified:** `src/envelope/policy.rs`
- **Verification:** `grep -nE 'branch\.\{|remote\.pushDefault|push\.default' src/envelope/policy.rs` returns no match; `grep -n 'push_refspecs' src/envelope/policy.rs` returns two.
- **Committed in:** `05f2696`

---

**Total deviations:** 4 (2 missing-critical/security, 1 blocking, 1 process)
**Impact on plan:** No scope creep. Deviations 1–3 each close a form that the behavior table's enumeration left reachable — a keyless config write, a flagless ref deletion, and an operand misparse — and every one of them is a way the denylist becomes decoration rather than a control. Deviation 4 changed a doc comment so an acceptance criterion could mean what it was written to mean.

## Issues Encountered

None. Every verification passed on first execution after the deliberate RED.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo test` | exit 0 — **825 passing** (801 after 19-01; +24, no regression, nothing ignored) |
| `cargo test --lib envelope::policy` | exit 0 — 27 tests |
| `cargo clippy -- -D warnings` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | **exactly 5** pre-existing lints — `browser.rs` ×3, `project_creator.rs` ×1, `state_reader/mod.rs` ×1. Count unchanged. Measured with `rtk proxy` per D-34. |
| `grep -c 'fn classify_git' src/envelope/policy.rs` | 1 |
| `grep -n 'Command::new' src/envelope/policy.rs` | no match — the classifier spawns nothing |
| `grep -n 'push_refspecs' src/envelope/policy.rs` | 2 matches (doc + call) |
| `grep -nE 'branch\.\{\|remote\.pushDefault\|push\.default' src/envelope/policy.rs` | no match — no second implementation |
| `grep -c 'denylist' src/envelope/policy.rs` / `allowlist` | 7 / 3, in the same doc comment |
| all seven `ParkReason` snake_case identifiers present | yes (`policy.rs:33-52`, asserted at `:1049-1055`) |
| `grep -c 'serde(default)' src/config.rs` | 14, against 9 at `740e62f` (+5: four new attributes plus one new mention in an assertion message) |
| `grep -n 'PRE_PHASE_19_CONFIG' src/config.rs` | matches; the literal is a `&str` opening with `{`, never `serde_json::to_string` |
| `grep -n 'pub const CONFIG_SCHEMA_VERSION: u32 = 2;' src/config.rs` | still matches — the version does not bump |
| `grep -rn 'DriverOptIn {' src tests` | the same 12 literal sites, all updated field-by-field; plus the struct definition and 4 new sites in this plan's own tests |

**RED-step evidence (committed, not reverted):** at `1df3db7` the classifier is stubbed to `Allow` and 13 of 23 tests fail — every refusal row. At `05f2696` all 23 pass. The 10 that pass at both commits are the allow rows and the reason taxonomy, which must pass against the stub or they would be proving nothing.

## Known Stubs

None. Every symbol this plan introduces is exercised by a passing test. `disallowed_tools()` and `EnvelopePolicy::resolve` have no production caller yet — plans 19-05, 19-06 and 19-07 own the wiring — but neither is a stub: both return fully computed values and both are asserted against.

## Threat Flags

None. No new network endpoint, auth path or schema was introduced, and no crate was added to `Cargo.toml` (T-19-SC holds). T-19-08 through T-19-13 are each mitigated by a committed test named in the coverage block; T-19-14 (an over-broad denylist blocking a legitimate GSD skill) remains the deliberately accepted risk, recorded in `classify_git`'s doc with the revisit condition attached.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Ready for 19-05 (PreToolUse guard).** `classify_git(argv, &GitContext)` is the verdict function the guard calls after splitting a Bash command into argv, and `ParkReason::as_str` is the string it writes into the journal. `disallowed_tools()` is layer 1's list.
- **Ready for 19-06 (wiring).** `EnvelopePolicy::resolve(alias, opt_in)` is the single call that turns a registry entry into the run's namespace, caps and credential source; `resolve_push_context(project_root, &policy.namespace)` builds the classifier's context at run start.
- **Ready for 19-07 (honesty text).** `classify_git`'s doc already carries the denylist/allowlist argument in the form the pinned constant should agree with; it should reference that reasoning rather than restate it differently.
- **For 19-04 (credential):** `CredentialSource` is the type to read; `Command { argv }` is deliberately argv rather than a shell string, so the resolver spawns without a shell.
- **Note for 19-08 (gate):** `SAFE-02` is declared by 19-02, 19-05, 19-06, 19-07 and 19-08. The shared-ID gate holds it, so `REQUIREMENTS.md` was **not** touched by this plan — the checkbox flips when the last declaring plan produces its SUMMARY.
- **No blockers.**

## Self-Check: PASSED

- Both task commits present in `git log` (`1df3db7`, `05f2696`, `c78117e`).
- Every file listed under `key-files.modified` exists on disk and is present in the diff against the plan's base `f1089ac`.
- Every task `<acceptance_criteria>` re-run and passing (see Verification Results); the plan-level `<verification>` re-run and passing.
- `STATE.md` and `ROADMAP.md` deliberately untouched — parallel worktree mode, orchestrator owns those writes.

---
*Phase: 19-gitsafe-git-blast-radius-envelope*
*Completed: 2026-08-18*
</content>
</invoke>
