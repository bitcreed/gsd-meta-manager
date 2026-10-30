---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 11
subsystem: infra
tags: [security, envelope, pretooluse-guard, shell-parsing, git, threat-t-19-60]

# Dependency graph
requires:
  - phase: 19-gitsafe-git-blast-radius-envelope
    provides: "the `PreToolUse` guard (19-05), the git argv classifier and shell splitter (19-02), the PR-cap ledger (19-05), and the injected child environment (19-04) this plan resolves against"
provides:
  - "`policy::resolve_program` — the effective program of a simple command, resolved structurally rather than read off `words[0]`, so the verdict is invariant under any wrapper chain and any path spelling"
  - "`policy::GOVERNED_PROGRAMS`, `ENVELOPE_ENV_KEYS`, `is_assignment_word`, `tampers_with_envelope_env`, `mentions_governed_program`, `ProgramResolution`"
  - "`classify_segments` acting on the resolved program index, with a fail-closed arm for a governed program it has no classifier for"
  - "the deletion of `NESTED_SHELLS` / `shell_c_payload` in favour of a structural `-c` rule that also covers `bash -lc` and `script -c`"
  - "`tests/envelope_wrapper_bypass.rs` — the six measured bypass lines, an anti-vacuity control, and a paired allow corpus"
  - "the `ENVELOPE_ENV_KEYS` drift pin against `cred::build_env_in`"
affects: [19-12, gsd-secure-phase-19, phase-20-router]

# Actuals (#2632)
actuals:
  # chars/4 over the realized `src/` + `tests/` diff (64 751 chars). For
  # comparison on the other common reading, chars/4 over the WHOLE of the three
  # changed files is 55 022 — the plan's 95 000 estimate was evidently on a
  # whole-file/context scale rather than a diff scale, so both are recorded
  # rather than picking the flattering one.
  tokens: 16188
  tasks: 3
  commits: 3

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Resolve the effective program by scanning for the first token whose BASENAME is in a CLOSED set of governed programs — never by enumerating the OPEN set of things that can precede one (D-08)"
    - "A refusal that is also a permit boundary states which direction fails closed: `Ungoverned` is the ANSWER for an ordinary command, while a governed program with no classifier refuses"
    - "Drift-pin a re-spelled constant against the code that owns the fact (`ENVELOPE_ENV_KEYS` vs `cred::build_env_in`), the discipline `forbidden_repo_prefixes` established"
    - "RED corpus committed as its own artifact, with verbatim failure output, strictly before the fix"

key-files:
  created:
    - tests/envelope_wrapper_bypass.rs
  modified:
    - src/envelope/policy.rs
    - src/envelope/hooks.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md

key-decisions:
  - "`ProgramResolution::Refuse` carries `detail: String` rather than `&'static str`, so the envelope-key refusal can name the key it refused — the convention `GitVerdict::Refuse` in the same module already documents"
  - "`GIT_SSH_COMMAND` belongs in `ENVELOPE_ENV_KEYS`; the plan's key list omitted it and the plan's own drift pin found it"
  - "`GIT_CONFIG_COUNT=0 git push --force` parks under `hook_bypass_blocked`, not `force_push_blocked`, because step 1 refuses the assignment on its own account before resolution reaches the push"
  - "T-19-74 (expansion-assembled program behind a wrapper) and T-19-75 (over-refusal from the quoted-payload rule) are ACCEPTED and disclosed in `resolve_program`'s own doc"

requirements-completed: [SAFE-02, SAFE-06]

coverage:
  - id: D1
    description: "All six measured T-19-60 bypass lines are refused, each carrying a specific D-24 reason rather than passing by being refused for an unrelated cause"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/envelope_wrapper_bypass.rs#a_force_push_behind_env_is_refused"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_bypass.rs#an_envelope_config_assignment_prefix_is_refused_on_its_own_account"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_bypass.rs#a_force_push_behind_timeout_is_refused"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_bypass.rs#a_force_push_behind_command_is_refused"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_bypass.rs#a_wrapper_spelled_as_an_absolute_path_is_refused_because_basenames_are_compared"
        status: pass
      - kind: e2e
        ref: "printf '<PreToolUse JSON>' | GSD_MM_ENVELOPE_ROOT=$TMP ./target/debug/gsd-meta-manager envelope guard alpha — all five force-push lines exit 2 (was 0 for four of them)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The fix is over the CLASS, not a list: a wrapper name that appears nowhere in the production logic resolves identically to `env`, and a chain of five wrapper names is refused"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/envelope/policy.rs#a_wrapper_name_that_appears_nowhere_in_this_crate_resolves_exactly_like_env"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_bypass.rs#a_chain_of_five_wrappers_none_of_which_the_fix_may_know_is_refused"
        status: pass
      - kind: e2e
        ref: "guard binary: `made-up-wrapper-9000 git push --force origin main` exits 2, identically to `env git push --force origin main`"
        status: pass
      - kind: other
        ref: "grep audit — no `WRAPPERS`/`TRANSPARENT_PROGRAMS` constant and no wrapper-name literal in non-comment `src/` or `tests/` code"
        status: pass
    human_judgment: false
  - id: D3
    description: "`env gh pr create` is counted against the SAFE-06 cap: it writes a ledger line and the second attempt in the run is parked with `pr_cap_exceeded`"
    requirement: SAFE-06
    verification:
      - kind: integration
        ref: "tests/envelope_wrapper_bypass.rs#a_wrapped_pull_request_is_counted_written_to_the_ledger_and_parked_on_the_second_attempt"
        status: pass
      - kind: e2e
        ref: "guard binary: two `env gh pr create` calls → exit 0 then exit 2 (`pr_cap_exceeded`); `pr-ledger.ndjson` holds 2 entries (write-before-permit, D-20)"
        status: pass
    human_judgment: false
  - id: D4
    description: "The paired allow corpus still passes, so the fix cannot be a blanket denial: `ls -la`, `echo hi`, `cargo test --lib`, `env git status`, `timeout 5 ls`, `gh pr list`, `tar -czf`, `rg \"git status\" src/` and an in-namespace push are permitted and write nothing to stdout"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/envelope_wrapper_bypass.rs#ordinary_commands_are_still_permitted_and_write_nothing"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_bypass.rs#a_wrapped_read_only_git_command_is_permitted"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_bypass.rs#a_forge_read_is_permitted_and_writes_no_ledger_line"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_bypass.rs#an_in_namespace_push_is_permitted_and_writes_nothing"
        status: pass
      - kind: integration
        ref: "tests/envelope_wrapper_bypass.rs#a_quoted_payload_that_classifies_as_allowed_is_permitted"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#an_ordinary_command_is_ungoverned_which_is_a_permit_and_not_a_gap"
        status: pass
    human_judgment: false
  - id: D5
    description: "`ENVELOPE_ENV_KEYS` cannot drift from the environment `cred::build_env_in` actually builds"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/envelope/policy.rs#every_envelope_key_the_child_environment_actually_sets_is_covered_by_the_constant"
        status: pass
      - kind: other
        ref: "fail-first proof — deleting the `GIT_SSH_COMMAND` entry turns the pin red naming that exact key; observed and reverted"
        status: pass
    human_judgment: false
  - id: D6
    description: "`NESTED_SHELLS` is DELETED and replaced by a structural `-c` rule that additionally covers `bash -lc` and `script -c`, which the five-name list did not"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/envelope/policy.rs#a_dash_c_payload_is_followed_whatever_program_consumes_it"
        status: pass
      - kind: unit
        ref: "src/envelope/hooks.rs#a_force_push_inside_a_nested_shell_is_still_seen (pre-existing, unmodified)"
        status: pass
      - kind: other
        ref: "grep -v '^\\s*//' src/envelope/hooks.rs | grep -c 'NESTED_SHELLS' → 0"
        status: pass
    human_judgment: false
  - id: D7
    description: "The fail-closed edge: a member of `GOVERNED_PROGRAMS` with no classifier arm refuses under `envelope_assertion_failed` instead of falling through to `Ok(None)`"
    requirement: SAFE-02
    verification:
      - kind: other
        ref: "src/envelope/hooks.rs `classify_segments` — the `governed =>` arm binds the program name and returns a refusal; no wildcard, no `Ok(None)` path from a resolved governed program"
        status: pass
    human_judgment: true
    rationale: "Unreachable today by construction — `resolve_program` only returns `Governed` for a member of `GOVERNED_PROGRAMS`, and all three members have classifier arms. It is a standing guard against a future fourth entry, so it cannot be exercised without adding one. Structurally verified by reading the arm; a reviewer should confirm the arm is a binding and not a wildcard."

duration: 24 min
completed: 2026-08-29
status: complete
---

# Phase 19 Plan 11: T-19-60 Wrapper/Assignment-Prefix Bypass Summary

**The `PreToolUse` guard now resolves which token is the effective program structurally — consuming leading `NAME=VALUE` assignment words by the shell's own grammar and then scanning for the first token whose BASENAME names a program the envelope governs — so `env`, `timeout`, `command`, `/usr/bin/env`, a five-name wrapper chain and a `GIT_CONFIG_COUNT=0` prefix all reach exactly the verdict their unwrapped spelling reaches, and no list of wrapper names exists anywhere for the seventh wrapper to be missing from.**

## Performance

- **Duration:** 24 min
- **Started:** 2026-08-29T04:29:00Z
- **Completed:** 2026-08-29T04:53:00Z
- **Tasks:** 3 of 3
- **Files modified:** 4 (1 created, 3 modified)

## Accomplishments

- **Closed `T-19-60`, the one high-severity threat blocking `/gsd-secure-phase 19`.** All five force-push spellings from `19-SECURITY.md`'s reproduction now exit 2 against the built binary; four of them exited 0 before.
- **Closed the SAFE-06 half.** `env gh pr create` now writes a ledger line and the second attempt in the run is parked with `pr_cap_exceeded`. The cap was previously bypassable with a four-letter prefix, with no ledger line and therefore no park and no evidence.
- **The fix is over the class, not a list.** `made-up-wrapper-9000 git push --force` produces the same verdict as `env git push --force`, and `nohup nice -n 10 stdbuf -oL setsid git push --force` is refused, with none of those seven names appearing anywhere in the production logic.
- **Deleted an enumeration rather than adding one.** `NESTED_SHELLS` (a five-name shell list) is gone, replaced by the structural rule that a bundled short option containing `c` hands its following word over as a nested command line — which also covers `bash -lc "…"` and `script -c "…"`, neither of which the old list matched.
- **`GIT_CONFIG_COUNT=0 git push --force` is now refused twice over.** The assignment is refused on its own account under `hook_bypass_blocked` (it is what disarms layer 3), *and* the push behind it would be refused by program resolution.

## Task Commits

Each task was committed atomically, and **the RED corpus is its own commit, strictly before the fix**:

1. **Task 1: RED — the six measured bypass lines, observed failing and committed** — `69c766e` (test)
2. **Task 2: the structural resolver, as a pure function in policy.rs** — `81eb044` (feat)
3. **Task 3: rewire classify_segments onto the resolver and delete the shell list** — `03c5abb` (fix)

**The two commit SHAs the plan asks for, in order:** RED corpus `69c766e`, then the fix `03c5abb` (with the pure resolver `81eb044` between them, at which point the corpus was still red because nothing consumed the resolver yet — verified, exit 101).

## The verbatim RED output (Task 1, against the unfixed tree)

Captured with `rtk proxy cargo test --test envelope_wrapper_bypass` — unproxied, the global RTK hook strips `test result:` lines and the run reports nothing usable (D-34, and 19-08's `T-19-51`).

```
running 13 tests
test a_chain_of_five_wrappers_none_of_which_the_fix_may_know_is_refused ... FAILED
test a_force_push_behind_env_is_refused ... FAILED
test a_force_push_behind_timeout_is_refused ... FAILED
test a_force_push_behind_command_is_refused ... FAILED
test a_forge_read_is_permitted_and_writes_no_ledger_line ... ok
test a_quoted_payload_that_classifies_as_allowed_is_permitted ... ok
test a_wrapped_pull_request_is_counted_written_to_the_ledger_and_parked_on_the_second_attempt ... FAILED
test a_wrapper_spelled_as_an_absolute_path_is_refused_because_basenames_are_compared ... FAILED
test a_wrapped_read_only_git_command_is_permitted ... ok
test an_envelope_config_assignment_prefix_is_refused_on_its_own_account ... FAILED
test the_unwrapped_force_push_is_refused_which_proves_this_harness_can_see_a_denial ... ok
test an_in_namespace_push_is_permitted_and_writes_nothing ... ok
test ordinary_commands_are_still_permitted_and_write_nothing ... ok

failures:

---- a_chain_of_five_wrappers_none_of_which_the_fix_may_know_is_refused stdout ----

thread 'a_chain_of_five_wrappers_none_of_which_the_fix_may_know_is_refused' (1233841) panicked at tests/envelope_wrapper_bypass.rs:131:5:
assertion `left == right` failed: `nohup nice -n 10 stdbuf -oL setsid git push --force origin main` must be REFUSED. `19-SECURITY.md` measured this line at exit 0 against the built binary: the guard reads `words[0]`, sees a token it has no classifier for, and falls through to a permit. stdout:  stderr: 
  left: 0
 right: 2
note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace

---- a_force_push_behind_env_is_refused stdout ----

thread 'a_force_push_behind_env_is_refused' (1233843) panicked at tests/envelope_wrapper_bypass.rs:131:5:
assertion `left == right` failed: `env git push --force origin main` must be REFUSED. `19-SECURITY.md` measured this line at exit 0 against the built binary: the guard reads `words[0]`, sees a token it has no classifier for, and falls through to a permit. stdout:  stderr: 
  left: 0
 right: 2

---- a_force_push_behind_timeout_is_refused stdout ----

thread 'a_force_push_behind_timeout_is_refused' (1233844) panicked at tests/envelope_wrapper_bypass.rs:131:5:
assertion `left == right` failed: `timeout 60 git push --force origin main` must be REFUSED. `19-SECURITY.md` measured this line at exit 0 against the built binary: the guard reads `words[0]`, sees a token it has no classifier for, and falls through to a permit. stdout:  stderr: 
  left: 0
 right: 2

---- a_force_push_behind_command_is_refused stdout ----

thread 'a_force_push_behind_command_is_refused' (1233842) panicked at tests/envelope_wrapper_bypass.rs:131:5:
assertion `left == right` failed: `command git push --force origin main` must be REFUSED. `19-SECURITY.md` measured this line at exit 0 against the built binary: the guard reads `words[0]`, sees a token it has no classifier for, and falls through to a permit. stdout:  stderr: 
  left: 0
 right: 2

---- a_wrapped_pull_request_is_counted_written_to_the_ledger_and_parked_on_the_second_attempt stdout ----

thread 'a_wrapped_pull_request_is_counted_written_to_the_ledger_and_parked_on_the_second_attempt' (1233847) panicked at tests/envelope_wrapper_bypass.rs:298:5:
assertion `left == right` failed: the wrapped form must reach the SAME ledger the unwrapped form reaches. Today it writes nothing, which is what makes the cap bypassable by a prefix.
  left: 0
 right: 1

---- a_wrapper_spelled_as_an_absolute_path_is_refused_because_basenames_are_compared stdout ----

thread 'a_wrapper_spelled_as_an_absolute_path_is_refused_because_basenames_are_compared' (1233849) panicked at tests/envelope_wrapper_bypass.rs:131:5:
assertion `left == right` failed: `/usr/bin/env git push --force origin main` must be REFUSED. `19-SECURITY.md` measured this line at exit 0 against the built binary: the guard reads `words[0]`, sees a token it has no classifier for, and falls through to a permit. stdout:  stderr: 
  left: 0
 right: 2

---- an_envelope_config_assignment_prefix_is_refused_on_its_own_account stdout ----

thread 'an_envelope_config_assignment_prefix_is_refused_on_its_own_account' (1233850) panicked at tests/envelope_wrapper_bypass.rs:131:5:
assertion `left == right` failed: `GIT_CONFIG_COUNT=0 git push --force origin main` must be REFUSED. `19-SECURITY.md` measured this line at exit 0 against the built binary: the guard reads `words[0]`, sees a token it has no classifier for, and falls through to a permit. stdout:  stderr: 
  left: 0
 right: 2


failures:
    a_chain_of_five_wrappers_none_of_which_the_fix_may_know_is_refused
    a_force_push_behind_command_is_refused
    a_force_push_behind_env_is_refused
    a_force_push_behind_timeout_is_refused
    a_wrapped_pull_request_is_counted_written_to_the_ledger_and_parked_on_the_second_attempt
    a_wrapper_spelled_as_an_absolute_path_is_refused_because_basenames_are_compared
    an_envelope_config_assignment_prefix_is_refused_on_its_own_account

test result: FAILED. 6 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

error: test failed, to rerun pass `--test envelope_wrapper_bypass`
```

**Exit code of the RED run: 101.** Seven bypass rows failed; six rows passed — the anti-vacuity control (`the_unwrapped_force_push_is_refused_which_proves_this_harness_can_see_a_denial`, which is what shows the harness could observe a denial at all) plus the five paired allow tests.

After Task 3: **13 passed / 0 failed**, with no pre-existing test modified.

## The T-19-60 reproduction, re-run against the built binary

Driven the same way `19-SECURITY.md` recorded it — `gsd-meta-manager envelope guard alpha` with `GSD_MM_ENVELOPE_ROOT` pointed at a temporary directory, one `PreToolUse` request per line:

```
--- the six measured lines ---
exit=2  git push --force origin main                      (was exit=2 — unchanged control)
exit=2  env git push --force origin main                  (was exit=0)
exit=2  GIT_CONFIG_COUNT=0 git push --force origin main   (was exit=0)
exit=2  timeout 60 git push --force origin main           (was exit=0)
exit=2  command git push --force origin main              (was exit=0)
exit=0  env gh pr create --title x                        (was exit=0 with NO ledger line;
                                                           now writes one — see below)
--- the class, not the list ---
exit=2  made-up-wrapper-9000 git push --force origin main
exit=2  nohup nice -n 10 stdbuf -oL setsid git push --force origin main
--- the paired allow corpus ---
exit=0  ls -la
exit=0  echo hi
exit=0  env git status
exit=0  timeout 5 ls
exit=0  gh pr list --limit 5
exit=0  git push origin refs/heads/gsd-auto/alpha/w:refs/heads/gsd-auto/alpha/w
```

`env gh pr create --title x` correctly exits 0 — it is the first pull request of the run and inside the cap. What changed is that it is now **counted**:

```
exit=0  env gh pr create --title x
exit=2  env gh pr create --title y
  reason: gsd-meta-manager envelope: REFUSED (reason: pr_cap_exceeded) — pull-request
  creation is refused: this attempt is beyond the per-run cap (2 of 1 used by this run) …

<envelope>/alpha/pr-ledger.ndjson:
{"at":"2026-08-29T04:50:24Z","run_id":"unattributed-run","command":"gh pr create","platform":"github"}
{"at":"2026-08-29T04:50:24Z","run_id":"unattributed-run","command":"gh pr create","platform":"github"}
```

Before the fix this file did not exist for the wrapped form.

## Files Created/Modified

- `tests/envelope_wrapper_bypass.rs` (created, 396 lines) — the RED corpus: seven bypass rows each asserting exit 2 **and** a specific D-24 reason, the anti-vacuity control, and the paired allow corpus (D-32).
- `src/envelope/policy.rs` (+641) — `GOVERNED_PROGRAMS`, `ENVELOPE_ENV_KEYS`, `envelope_env_key`, `is_assignment_word`, `tampers_with_envelope_env`, `mentions_governed_program`, `ProgramResolution`, `resolve_program`, plus the unit table and the drift pin. All pure, all total functions of their arguments, in the module whose doc already promises exactly that.
- `src/envelope/hooks.rs` (282 changed) — `classify_segments` rewritten onto the resolver; `NESTED_SHELLS` and `shell_c_payload` deleted; the doc comments that described classification as happening on the first word corrected.
- `.planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md` (+27) — the re-observation of the pre-existing `driver_reattach` pair, with the proof it is not a 19-11 regression.

## Decisions Made

1. **`Ungoverned` is documented as the answer, not a gap.** The guard is registered against every Bash tool call. `ProgramResolution::Ungoverned`'s own doc records that a resolution which denied what it did not recognise would deny `ls`, `cargo test` and `rg` — and that a control which fails into unusability gets switched off. What fails closed is the other direction: once resolution reaches a governed program, `classify_segments` classifies it or refuses it and never returns `Ok(None)` for it.
2. **The one enumeration added is `GOVERNED_PROGRAMS`, and its doc says why it is not the same kind of list as a wrapper list** — it is CLOSED and already defined by `classify_git` and `pr_command_label`, while the set of things that can precede a program is open and unlistable (D-08).
3. **`GIT_CONFIG_COUNT=0 …` parks under `hook_bypass_blocked`.** See deviation 3 below.
4. **T-19-74 and T-19-75 are accepted and disclosed in `resolve_program`'s own doc**, not left for a reader to discover. See "Shapes this fix does NOT cover".

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] `GIT_SSH_COMMAND` added to `ENVELOPE_ENV_KEYS`**

- **Found during:** Task 2, **by the plan's own drift pin** rather than by inspection.
- **Issue:** The plan enumerated `ENVELOPE_ENV_KEYS` as `GIT_CONFIG_COUNT`, `GIT_CONFIG_KEY_*`, `GIT_CONFIG_VALUE_*`, `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_SYSTEM`, `GIT_ASKPASS`, `GIT_TERMINAL_PROMPT`, `GH_CONFIG_DIR`. `cred::build_env_in` also **sets** `GIT_SSH_COMMAND`, which begins with `GIT_`, so the drift pin the same task specifies fails against the plan's own list. Leaving it out would mean `GIT_SSH_COMMAND=ssh git push …` is not refused — and that variable is what carries `IdentitiesOnly=yes`, `IdentityAgent=none`, `BatchMode=yes` and `-F /dev/null`, so reassigning it puts the user's own ssh agent and default identity back within the driven run's reach (D-16). That is a credential-isolation bypass of exactly the kind the refusal exists for.
- **Fix:** Added `"GIT_SSH_COMMAND"` to `ENVELOPE_ENV_KEYS` with a comment recording that the drift pin found it.
- **Verification:** Proved fail-first — deleted the entry, observed the pin red naming that exact key (`` `GIT_SSH_COMMAND` is SET in the driven child's environment by `cred::build_env_in` but is not covered by `ENVELOPE_ENV_KEYS` ``, with the plan's exact eight-key list printed as the covered set), then restored. This is the drift pin doing the job it was specified to do on its first run.
- **Committed in:** `81eb044`

**2. [Rule 2 - Missing critical] `ProgramResolution::Refuse` carries `detail: String`, not `&'static str`**

- **Found during:** Task 2.
- **Issue:** The plan declared `Refuse { reason: ParkReason, detail: &'static str }`. A `&'static str` detail cannot name *which* envelope key a word was reaching for, so the `hook_bypass_blocked` refusal would be a fixed sentence — and the plan's own T-19-60a mitigation is stated as "refusing any word that assigns to or bare-naming a key", which a reader cannot act on without knowing the key. The RED corpus (already committed, so not editable to suit the fix) asserts the reason text contains `GIT_CONFIG_COUNT`.
- **Fix:** `detail: String`. This matches `GitVerdict::Refuse` in the same module, whose doc already records the rationale: *"One line naming the exact token that caused the refusal. Never the whole argv: a detail that quotes the command back is a detail that can carry a secret into the journal (SAFE-04)."* The same SAFE-04 constraint is documented on the new field.
- **Verification:** `Debug, Clone, PartialEq, Eq` still derive; the unit table asserts on `detail.contains("GIT_CONFIG_COUNT")`; `cargo clippy -- -D warnings` exit 0.
- **Committed in:** `81eb044`

**3. [Rule 1 - Internal inconsistency in the plan text] the reason asserted for the `GIT_CONFIG_COUNT=0` row**

- **Found during:** Task 1, before writing the RED corpus.
- **Issue:** The plan's Task 1 bullet list says all six refusal rows assert `policy::REASON_FORCE_PUSH_BLOCKED`, and lists `GIT_CONFIG_COUNT=0 git push --force origin main` among them. But the plan's **normative** design — `resolve_program` step 1, and the must-have truth "*`GIT_CONFIG_COUNT=0 git push --force` is refused TWICE OVER … `policy::tampers_with_envelope_env` closes the first … under `hook_bypass_blocked`*" — refuses that line at step 1, **before** resolution ever reaches the `push --force` behind it. The two readings cannot both hold.
- **Fix:** Followed the normative design. The row asserts exit 2 and `REASON_HOOK_BYPASS_BLOCKED`, **plus** that the reason names `GIT_CONFIG_COUNT` — which is a *stronger* anti-vacuity assertion than the generic one, not a weaker one. `hook_bypass_blocked` is a specific member of D-24's taxonomy (the same one `--no-verify` and `core.hooksPath` park under), so the row still cannot pass by being refused for an unrelated cause such as an unrecoverable split. The reasoning is recorded in a comment on the test itself.
- **Verification:** The row was red before the fix and green after; the guard's decision JSON carries `(reason: hook_bypass_blocked)` and names the key.
- **Committed in:** `69c766e` (assertion), `81eb044` (`resolve_program` step 1)

**4. [Rule 2 - Missing critical] `tar -czf` asserted as `NestedPayload`, not `Ungoverned`**

- **Found during:** Task 2.
- **Issue:** The plan's unit-test list asks for `tar -czf a.tgz dir` "as `Ungoverned` via a `-c` payload that resolves to nothing rather than to a refusal", but its own ordered contract (step 6) mechanically yields `NestedPayload { index: 2 }` for that line — a `-c`-bearing short option whose following word exists.
- **Fix:** Asserted what the contract produces (`NestedPayload { index: 2 }`) **and** asserted the plan's stated intent separately and explicitly: `resolve_program` of the payload `a.tgz` is `Ungoverned`, so the end-to-end verdict is a permit. `tar -czf a.tgz dir` is also in the integration allow corpus and exits 0.
- **Verification:** `src/envelope/policy.rs#a_dash_c_payload_is_followed_whatever_program_consumes_it`; `tests/envelope_wrapper_bypass.rs#ordinary_commands_are_still_permitted_and_write_nothing`.
- **Committed in:** `81eb044`

---

**Total deviations:** 4 auto-fixed (3× Rule 2 missing-critical, 1× Rule 1 correcting an internal inconsistency in the plan text).
**Impact on plan:** No scope creep. Two of the four (1 and 4) were surfaced by controls the plan itself specified, which is the controls working. None weakens a refusal; deviations 1 and 2 each *strengthen* one. Nothing outside `T-19-60` was taken on.

## Scope discipline — what was deliberately NOT touched

Per the plan's prohibitions, and verified by `git diff --stat 5e574c4..HEAD`:

- **`deny` is untouched.** Its `?`-before-`park_refusal` ordering (`T-19-61`) is out of scope and was left exactly as it is, including where Task 3 worked in the same file.
- **`T-19-62` … `T-19-73` untouched.** No parameterized park-coverage control, no change to `cred.rs`'s env scrub, `advisory.rs`, the honesty statement or the second-carrier table.
- **No crate added.** `Cargo.toml` and `Cargo.lock` are byte-identical to the base commit (`git diff 5e574c4..HEAD -- Cargo.toml Cargo.lock` is empty), so `T-19-SC` still holds phase-wide. Every parser here is hand-rolled against `std`.
- **No wrapper-name list anywhere.** Non-comment grep over `src/` and `tests/` finds no `WRAPPERS`/`TRANSPARENT_PROGRAMS` constant and no `env`/`timeout`/`nohup`/`command`/`nice`/`sudo`/`sh`/`bash` literal used as a program-name list. The only constants matching a `WRAPPER|TRANSPARENT|SHELL` name pattern are `MAX_SHELL_RECURSION` (a depth bound, pre-existing) and `tests/driver_refusal_record.rs`'s `SHELL_SMUGGLING_*` fixtures (pre-existing).
- **No pre-existing test modified.** `a_force_push_inside_a_nested_shell_is_still_seen`, `a_verb_assembled_by_expansion_and_an_eval_are_both_denied`, `a_command_whose_words_cannot_be_recovered_is_denied`, `a_push_inside_the_reserved_namespace_is_permitted_and_writes_nothing` and the whole of `tests/envelope_pr_cap.rs` all pass unchanged.

## Shapes this fix does NOT cover

Both are recorded in `resolve_program`'s own doc comment, not only here, so a reader of the code meets them where they apply.

- **`T-19-74` — an expansion-assembled program behind a wrapper. ACCEPTED.** `env $X push --force`, where `$X` was bound *outside* this command line, resolves to `Ungoverned` and is permitted. Closing it would require refusing every `$VAR` in an ungoverned command, which also refuses `echo $(git rev-parse HEAD)` and `cd "$HOME"` — a control that fails into unusability gets switched off. It is narrowed on two sides: step 1 refuses the envelope-key assignments that would pair with it, and step 7 refuses binding a governed program name to a variable **in the same command line** (`X=git made-up-wrapper-9000 run` is refused). Each Bash tool call being its own shell process is what keeps the residual narrow. Pinned by plan 19-12.
- **`T-19-75` — over-refusal from the quoted-payload rule. ACCEPTED.** An argument that literally spells a *refused* git command is refused: `rg "git push --force" src/` does not run. The rule **discriminates** rather than blanket-denying, because the payload is classified — `rg "git status" src/` is permitted, and that is asserted as a paired allow row rather than left as a claim. The cost is confined to driven runs and is legible when it fires.

Also unchanged and still true: the **D-09 ceiling** itself. An agent that unsets `GIT_CONFIG_COUNT` in a place the guard cannot see is past layer 3. What this plan changes is that the *command line* which performs the unset is no longer invisible to layer 2 — the token that disarmed the hooks now refuses on its own account.

## Verification / Final gate numbers

All measured with `rtk proxy`, because the global RTK hook strips `warning:` and `test result:` lines and an unproxied grep for them succeeds having read nothing (D-34).

| Gate | Result |
|---|---|
| `rtk proxy cargo build` | Finished, exit 0 |
| `rtk proxy cargo test` | **1245 passed / 2 failed / 13 ignored**, suite exit 101 |
| `rtk proxy cargo test --test envelope_wrapper_bypass` | **13 passed / 0 failed / 0 ignored**, exit 0 |
| `rtk proxy cargo test --lib envelope::policy` | 59 passed / 0 failed |
| `rtk proxy cargo clippy -- -D warnings` | **exit 0** |
| `rtk proxy cargo clippy --all-targets` | **exactly 5 lints — the recorded pre-existing count, no growth** |
| `grep -v '^\s*//' src/envelope/hooks.rs \| grep -c 'NESTED_SHELLS'` | **0** |
| `git diff 5e574c4..HEAD -- Cargo.toml Cargo.lock` | empty |

## Issues Encountered

**The 2 suite failures are pre-existing and were proved so, not assumed.**

`tests/driver_reattach.rs::a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` and `::a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired` failed 3/3 in isolation on this host. Both spawn the real driver binary and then read `run.json`; `live_within(pid, …)` succeeds, so the driver comes up and is visible in `/proc` — the run **record** is what is missing.

**Proof they are not a 19-11 regression:** `src/envelope/hooks.rs` was reverted to its pre-Task-3 state — leaving `policy::resolve_program` present but with **no production caller**, so the guard behaved exactly as it did at the base commit `5e574c4` — and the same two tests failed identically. 19-11 touches only the `PreToolUse` guard's program resolution, which `driver_reattach` never exercises.

This is the same startup race already recorded in this phase's `deferred-items.md` as proved pre-existing; the re-observation (now deterministic on this host rather than flaky) is appended there. **Not fixed here** — out of scope for a T-19-60 gap-closure plan, and the plan forbids taking on neighbouring findings.

## Known Stubs

None. No hardcoded empty value, placeholder string, `TODO` or `FIXME` was introduced. `resolve_program` was `pub` and unused at the end of Task 2 by design — Task 3 wired it, and it now has a production caller.

## Threat Flags

No new attack surface. This plan **removes** surface rather than adding it: it closes `T-19-60`, `T-19-60a` and `T-19-60b`, adds no network path, no filesystem path, no new endpoint and no crate.

Two new *accepted* residuals are declared above and in code: `T-19-74` (medium, accept) and `T-19-75` (medium, accept). Both were declared in the plan's own threat register at plan time; neither is a discovery of this execution.

One threat-register correction for the next `/gsd-secure-phase 19` run: `T-19-60`'s mitigation is now implemented and evidenced by `tests/envelope_wrapper_bypass.rs`, observed RED first. `19-SECURITY.md`'s `threats_open: 1` / `status: blocked` frontmatter and its Sign-Off block still say otherwise — **this plan deliberately did not edit `19-SECURITY.md`**, because re-running the audit is what should flip it, not the plan that closed the finding writing its own verdict into the audit (D-25).

## Self-Check: PASSED

- `tests/envelope_wrapper_bypass.rs` — FOUND on disk.
- `69c766e` (RED corpus) — FOUND in `git log`.
- `81eb044` (resolver) — FOUND in `git log`.
- `03c5abb` (fix) — FOUND in `git log`.
- RED strictly precedes the fix: `git log --oneline` orders `69c766e` → `81eb044` → `03c5abb`, and the corpus was still red at `81eb044` (exit 101, verified before committing Task 3).
- All four plan `<success_criteria>` re-run and confirmed after the fix, including end-to-end against the built binary.

## Next Phase Readiness

`T-19-60` is closed with committed RED-first evidence, so the one high-severity threat blocking `/gsd-secure-phase 19` is remediated. Ready for **19-12**, which the threat register names as the pin for the accepted `T-19-74` residual.

**Blockers/concerns for whoever goes next:**

1. **Re-run `/gsd-secure-phase 19`** to move `T-19-60` to `closed` and flip `19-SECURITY.md`'s `threats_open` / `status` / Sign-Off. This plan intentionally left that file untouched.
2. **`T-19-61` through `T-19-73` remain open below the `high` threshold and remain *unaccepted*** — unremediated findings awaiting disposition, exactly as `19-SECURITY.md` records. `T-19-61` in particular (`deny`'s `?` before `park_refusal`) was in the file Task 3 edited and was deliberately left alone.
3. **The `driver_reattach` pair now fails deterministically on this host**, where it was previously intermittent. Pre-existing and out of scope here, but the suite is no longer 0-failed, so the next executor should expect 1245/2/13 rather than a clean green.

---
*Phase: 19-gitsafe-git-blast-radius-envelope*
*Completed: 2026-08-29*
