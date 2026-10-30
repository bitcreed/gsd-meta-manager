---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 05
subsystem: infra
tags: [pretooluse-hook, ndjson-ledger, rate-cap, shell-quoting, settings-json, serde, safety-envelope]

# Dependency graph
requires:
  - phase: 19-gitsafe-git-blast-radius-envelope
    provides: "19-01's `envelope_dir_in` / `envelope_root` and the `GSD_MM_ENVELOPE_ROOT` override plus the hidden `envelope` subcommand re-entry; 19-02's `classify_git`, `GitContext`, `ParkReason`, `resolve_push_context`, `disallowed_tools`, `EnvelopePolicy::resolve` and the `pr_cap_*` `DriverOptIn` fields; 19-03's `pre_push`/`pre_commit` hook bodies; 19-04's `cred::PROJECT_ROOT_ENV` and `sh_quote`"
  - phase: 17-driver-supervisor
    provides: "`src/executor/mod.rs:225-239`'s reproduced 180-240 second `PreToolUse` hang and the `--setting-sources project` mitigation — the bug this plan's own hook must not re-create; `tests/spawn_seam_guard.rs`"
  - phase: 16-run-journal
    provides: "`journal::writer`'s `create(true).append(true)` NDJSON discipline and its recorded-asymmetry doc shape; `journal::reader`'s tolerant-parse posture, which this plan deliberately inverts; `journal::redact`"
provides:
  - "`envelope::ledger` — the append-only NDJSON PR ledger, `LedgerEntry`, `ledger_path`/`ledger_path_in`, `CapVerdict` with `refusal_detail`, and `record_and_check`/`record_and_check_in`"
  - "`envelope::policy::split_command` / `split_segments` / `Token` — a hand-rolled POSIX-quoting-aware splitter with no crate added"
  - "`envelope::policy::classify_pr_command` / `pr_command_label` / `program_name` — D-19's three creation shapes, matched on tokens"
  - "`envelope::policy::push_needs_resolved_dests` — the one command shape the guard reads a repository for, sharing `scan_leading` and `push_operands` with the classifier"
  - "`envelope::hooks::guard` / `guard_in` and `GuardRequest` — D-06 layer 2, network-free and deny-by-default"
  - "`envelope::hooks::write_settings` / `write_settings_in` / `settings_value` / `settings_json` / `verify_settings` / `EnvelopeSettings` tree / `GUARD_TIMEOUT_SECS`"
  - "`cli::EnvelopeAction::Guard` and its `main.rs` arm"
  - "`envelope::cred::RUN_ID_ENV` — the one spelling of the guard's run-id variable"
affects: [19-06 wiring, 19-07 honesty text, 19-08 gate, 20 router]

# Actuals (#2632)
actuals:
  tokens: 31900
  tasks: 3
  commits: 5

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Compiling RED, carried forward from 19-02: the `test(...)` commit ships the real API with only the DECISION stubbed to permit, so the history stays buildable and every refusal row is proved load-bearing in committed history rather than by a mutation somebody performed and reverted."
    - "Second-carrier table at the generator: `settings_value`'s doc tabulates every control the file registers against the argv flag or git hook that also carries it — and names outright the ONE control that has no git-hook counterpart, rather than leaving the table to imply full coverage."
    - "Permit-is-silence: a deny-only guard writes nothing on a permit. Answering `allow` would auto-approve commands the user's own permission rules would have prompted for, turning a bound into an authority."
    - "Fixture self-validation: `registry_with_caps` asserts its own file loads and carries the caps, because `resolve_policy` degrades an unreadable registry to the tighter defaults — so a typo would have silently tested the defaults while claiming to test the configured caps."

key-files:
  created:
    - src/envelope/ledger.rs
    - tests/envelope_pr_cap.rs
  modified:
    - src/envelope/policy.rs
    - src/envelope/hooks.rs
    - src/envelope/cred.rs
    - src/envelope/mod.rs
    - src/cli.rs
    - src/main.rs

key-decisions:
  - "A permit writes NOTHING to stdout. The plan says to write the permission decision in the shape the protocol expects; emitting `permissionDecision: allow` would make a deny-only control into an approval authority that auto-approves what the user's own rules would have prompted for. Silence leaves the ordinary permission flow exactly where it was."
  - "A denial travels twice: the protocol JSON on stdout AND exit code 2 with the reason on stderr. One carrier is a carrier a schema drift silently disarms — the same argument D-07 makes about the settings file, applied to the guard's own answer."
  - "`split_segments` exists alongside `split_command`. A single-command splitter is a hole rather than a control: `echo hi && git push --force` has `echo` as its first word, so a guard that classified only the first command would allow the force push behind the `&&`."
  - "`sh -c` payloads are followed to a bounded depth (2). `bash -c \"git push --force\"` is an ordinary thing for an agent to write, not an evasion, and a guard that judged the wrapper would judge nothing."
  - "A program token subject to shell expansion, and any `eval`, are denied outright. The value is unknowable before the shell runs, and the guard refuses what it cannot classify."
  - "`append_entry` terminates a torn tail before appending. Without it the torn record and the new one fuse into a single unparseable line and count as ONE attempt where two happened — the append-only format's own failure mode swallowing the attempt being recorded."
  - "An unparseable ledger line counts toward the WINDOW only, never the per-run bound. A line whose run cannot be read cannot be attributed to one, and attributing it to every run would make a single torn write refuse the first PR of every future run forever — a permanent denial of service wearing the costume of caution."
  - "The per-run bound is deliberately not time-scoped. A run is a bounded thing; letting its slots expire mid-run would silently turn the per-run cap into a second rolling-window cap."
  - "`push_needs_resolved_dests` shares `scan_leading` and `push_operands` with `classify_push` rather than re-deriving git's option grammar. A second copy would be a second thing to keep in step, and the drift would be a guard resolving a context for the wrong argv — or failing to resolve one for the right argv, which reads as an unresolvable destination and refuses a push that was inside the namespace."
  - "`resolve_policy` degrades an unreadable registry to the DEFAULTS rather than erroring, and the direction is what makes it safe: every default is the tighter value, so a guard that cannot read configuration confines the run more, never less. The alternative — refusing every tool call because a config file moved — is a control that fails into unusability and therefore gets switched off."
  - "The pull-request cap has NO git-hook second carrier, and `settings_value`'s doc says so. No git hook observes `gh pr create`, because it is not a git operation. D-07's 'degrade, never disarm' still holds — an ignored settings file leaves every push boundary standing — but this is a degradation and it is recorded rather than papered over."

patterns-established:
  - "Inverted tolerance with the reason at the function: `tally`'s doc states why a malformed line is COUNTED here while `journal::reader` skips one — the consumer differs, not the format. A skipped journal line drops one observation; a skipped ledger line under-counts a cap."
  - "The over-count bias is in the refusal message, not only in the doc: `CapVerdict::refusal_detail` tells the user that a recorded-but-failed attempt costs a slot, because a limitation that is discovered is a bug report while a limitation that is stated is a limitation."
  - "Extraction-integrity assertions in source-scanning tests: the network-free guard test asserts the extracted function body actually contains `classify_segments` before asserting what it does not contain, so a broken extraction cannot pass vacuously."

requirements-completed: []  # SAFE-06 is declared by 19-05 alone but SAFE-02 is shared with 19-02/06/07/08; the orchestrator's shared-ID gate releases both when the last declaring plan lands.

coverage:
  - id: D1
    description: "The third pull-request creation in a rolling 24-hour window is permitted and the fourth is refused with `pr_cap_exceeded`; an entry older than 24 hours releases its slot and one exactly at the boundary does not"
    requirement: SAFE-06
    verification:
      - kind: integration
        ref: "tests/envelope_pr_cap.rs#the_third_pull_request_in_the_window_succeeds_and_the_fourth_is_refused"
        status: pass
      - kind: integration
        ref: "tests/envelope_pr_cap.rs#an_attempt_that_has_fallen_out_of_the_window_releases_its_slot"
        status: pass
      - kind: integration
        ref: "tests/envelope_pr_cap.rs#an_attempt_exactly_at_the_boundary_still_counts_against_the_window"
        status: pass
      - kind: unit
        ref: "src/envelope/ledger.rs#three_distinct_runs_are_permitted_in_the_window_and_the_fourth_is_refused"
        status: pass
      - kind: unit
        ref: "src/envelope/ledger.rs#an_entry_one_second_inside_the_window_counts_toward_it, #an_entry_exactly_at_the_boundary_counts_because_the_window_is_inclusive, #an_entry_one_second_past_the_boundary_does_not_count"
        status: pass
      - kind: other
        ref: "RED commit 3789ada: the same rows fail against an always-permit decision; they pass at 4ed779b"
        status: pass
    human_judgment: false
  - id: D2
    description: "The window is decided by RFC3339 comparison at second precision with no floating-point arithmetic; an unparseable timestamp counts toward the cap and the running count saturates rather than wrapping"
    requirement: SAFE-06
    verification:
      - kind: unit
        ref: "src/envelope/ledger.rs#a_line_whose_timestamp_cannot_be_parsed_is_counted_toward_the_cap"
        status: pass
      - kind: unit
        ref: "src/envelope/ledger.rs#the_running_count_saturates_rather_than_wrapping"
        status: pass
      - kind: other
        ref: "grep -nE '\\bf(32|64)\\b|as f64|as f32' src/envelope/ledger.rs returns no match"
        status: pass
      - kind: other
        ref: "grep -c 'saturating_' src/envelope/ledger.rs returns 10"
        status: pass
    human_judgment: false
  - id: D3
    description: "The second pull-request creation within a single run is refused even when the 24-hour window has capacity, because the per-run cap is a separate bound"
    requirement: SAFE-06
    verification:
      - kind: integration
        ref: "tests/envelope_pr_cap.rs#a_second_attempt_in_one_run_is_refused_while_the_window_still_has_capacity"
        status: pass
      - kind: unit
        ref: "src/envelope/ledger.rs#a_second_attempt_in_the_same_run_is_refused_even_though_the_window_has_capacity"
        status: pass
      - kind: unit
        ref: "src/envelope/hooks.rs#the_pull_request_cap_is_enforced_from_the_guard_and_the_third_still_succeeds"
        status: pass
    human_judgment: false
  - id: D4
    description: "The ledger entry is written before the tool call is permitted, so a recorded-but-failed attempt costs a slot — and the refusal message states that bias rather than leaving it to be discovered"
    requirement: SAFE-06
    verification:
      - kind: integration
        ref: "tests/envelope_pr_cap.rs#the_refused_attempt_is_on_disk_because_the_ledger_records_before_it_permits"
        status: pass
      - kind: unit
        ref: "src/envelope/ledger.rs#the_refused_attempt_is_on_disk_because_the_entry_is_written_before_it_is_permitted"
        status: pass
      - kind: unit
        ref: "src/envelope/ledger.rs#the_refusal_states_the_over_count_bias_rather_than_leaving_it_to_be_discovered, #a_refusal_names_the_per_run_bound_when_that_is_the_one_that_was_exceeded"
        status: pass
    human_judgment: false
  - id: D5
    description: "The ledger is an append-only file outside the driven repository, so an agent cannot reset its own cap by deleting a tracked file, and a torn write costs at most the last line"
    requirement: SAFE-06
    verification:
      - kind: unit
        ref: "src/envelope/ledger.rs#the_ledger_sits_under_the_envelope_and_carries_no_component_of_the_repository"
        status: pass
      - kind: unit
        ref: "src/envelope/ledger.rs#a_torn_final_line_is_counted_toward_the_cap_rather_than_skipped"
        status: pass
      - kind: unit
        ref: "src/envelope/ledger.rs#appending_onto_a_torn_tail_does_not_fuse_the_two_records_into_one"
        status: pass
      - kind: unit
        ref: "src/envelope/ledger.rs#a_hostile_alias_has_no_ledger_at_all"
        status: pass
    human_judgment: false
  - id: D6
    description: "The guard parses a shell command into argv and applies the shared classifier, so `git  push   --force` and `git push -f` reach the same verdict rather than being prefix-matched — and a command it cannot recover is denied, never allowed"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/envelope/hooks.rs#the_two_spellings_of_a_force_push_reach_the_same_verdict_as_each_other"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#a_single_quoted_argument_is_recovered_whole, #a_double_quoted_argument_containing_spaces_is_one_word, #a_backslash_escaped_space_does_not_split_a_word, #a_run_of_whitespace_is_one_boundary_not_several_empty_words"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#an_unterminated_quote_is_unrecoverable_and_yields_none"
        status: pass
      - kind: unit
        ref: "src/envelope/hooks.rs#a_command_whose_words_cannot_be_recovered_is_denied, #a_verb_assembled_by_expansion_and_an_eval_are_both_denied, #a_request_whose_json_cannot_be_parsed_is_denied, #a_bash_request_with_no_readable_command_is_denied"
        status: pass
      - kind: unit
        ref: "src/envelope/hooks.rs#a_force_push_hidden_behind_a_separator_is_still_seen, #a_force_push_inside_a_nested_shell_is_still_seen"
        status: pass
      - kind: unit
        ref: "src/envelope/hooks.rs#a_push_inside_the_reserved_namespace_is_permitted_and_writes_nothing (the paired allow)"
        status: pass
    human_judgment: false
  - id: D7
    description: "D-19's three pull-request creation shapes are recognised on tokens, and read-only forge commands are not"
    requirement: SAFE-06
    verification:
      - kind: unit
        ref: "src/envelope/policy.rs#the_github_pull_request_creation_form_is_recognised, #a_raw_api_post_to_a_pulls_path_is_recognised_in_both_its_spellings, #the_gitlab_merge_request_creation_form_is_recognised"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#a_read_only_listing_form_is_not_a_creation"
        status: pass
      - kind: integration
        ref: "tests/envelope_pr_cap.rs#the_refused_attempt_is_on_disk_... (asserts `gh pr list` adds no ledger line)"
        status: pass
    human_judgment: false
  - id: D8
    description: "The guard makes no network call and carries an explicit registered timeout, so it cannot re-create the reproduced 180-240 second PreToolUse hang"
    requirement: SAFE-02
    verification:
      - kind: unit
        ref: "src/envelope/hooks.rs#the_guard_makes_no_network_call_on_any_path_it_takes (source-scanning, with an extraction-integrity assertion)"
        status: pass
      - kind: integration
        ref: "tests/envelope_pr_cap.rs#the_generated_settings_file_registers_the_guard_with_the_named_timeout_constant"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#only_a_push_with_no_refspec_needs_the_repository_consulted"
        status: pass
      - kind: other
        ref: "awk '/^pub fn guard/,/^}/' src/envelope/hooks.rs | grep -cE 'reqwest|ureq|hyper|curl' returns 0"
        status: pass
    human_judgment: false
  - id: D9
    description: "The settings file is serialised from typed structs, never templated; it is deserialised back and compared after writing, and a mismatch refuses the run rather than warning"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/envelope_pr_cap.rs#a_one_byte_corruption_of_the_settings_file_makes_the_comparison_return_an_error"
        status: pass
      - kind: integration
        ref: "tests/envelope_pr_cap.rs#a_settings_value_that_differs_from_the_file_is_a_mismatch_not_a_pass"
        status: pass
      - kind: other
        ref: "awk '/pub fn write_settings/,/^}/' src/envelope/hooks.rs | grep -cE 'format!|push_str|concat!' returns 0"
        status: pass
    human_judgment: false
  - id: D10
    description: "Every control the settings file carries is also carried by argv or by a git hook, so a silently ignored settings file leaves other layers standing"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/envelope_pr_cap.rs#every_pattern_the_settings_file_denies_is_also_carried_on_argv"
        status: pass
      - kind: integration
        ref: "tests/envelope_pr_cap.rs#the_argv_delivery_renders_the_same_value_the_file_carries"
        status: pass
    human_judgment: true
    rationale: "The tests prove the deny list and the argv list are the same values from the same source, and that `settings_json` renders the same value the file carries. They cannot settle whether the doc's second-carrier table is an HONEST account — specifically its admission that the pull-request cap has no git-hook counterpart and would degrade to unenforced if the file were ignored. Whether that reads as candour or as a hedge is a reader's judgement, and D-07's own transparency posture in this phase uses `judgment` for exactly this."
  - id: D11
    description: "The stated limit of the argv split — a variable-assembled verb, `eval`, base64, a written-then-run script — is recorded at the function rather than softened, together with the layer-2/layer-3 framing"
    verification:
      - kind: other
        ref: "src/envelope/policy.rs `split_command`'s doc: the four unhandled constructs enumerated, then the `layer 2 raises the cost of an accident / layer 3 covers deliberate evasion / past layer 3 only the remote's ruleset remains` framing"
        status: pass
    human_judgment: true
    rationale: "A grep proves the paragraph exists. Whether it reads as an honest account of a real ceiling — rather than as a pre-excuse for every future gap — is a judgement only a reader can make. `<specifics>` asks for exactly this framing and the phase's transparency prohibitions verify by judgment."

# Metrics
duration: 48 min
completed: 2026-08-18
status: complete
---

# Phase 19 Plan 05: The PR Cap, the `PreToolUse` Guard and the Settings File Summary

**The fourth pull request in a rolling 24 hours is refused with `pr_cap_exceeded` while the third still succeeds — decided from an append-only ledger outside the driven repository, in integer seconds with no floating-point arithmetic, by a guard that parses the shell command into argv, follows it behind `&&` and into `bash -c`, denies whatever it cannot recover, touches no network, and is delivered by a settings file that is serialised from typed structs and refuses the run if reading it back does not reproduce the value that was written.**

## Performance

- **Duration:** ~48 min
- **Started:** 2026-08-18 (base commit `476f3b9`)
- **Completed:** 2026-08-18
- **Tasks:** 3 (Task 1 executed as RED → GREEN)
- **Files modified:** 8 (2 created, 6 modified)

## Accomplishments

- **ROADMAP success criterion 5 is met and mechanically proved, in both directions.** `the_third_pull_request_in_the_window_succeeds_and_the_fourth_is_refused` asserts the allow and the refusal in one fixture, because three of this phase's five requirements are refusals and an envelope that refused every pull request would satisfy every refusal criterion and be useless. The refusal carries `pr_cap_exceeded`, D-24's identifier, not a fresh string.
- **The refusals are proved load-bearing in committed history rather than by a reverted mutation.** RED commit `3789ada` ships the real API with only the *decision* stubbed to permit: 7 of 15 rows fail there and pass at `4ed779b`. The 8 that pass at both are the first-permit row, the out-of-window row, the path rows and the message rows — they must pass against the stub or they would be proving nothing.
- **The window is exercised with an injected clock, never a sleep.** Every boundary row — 23h59m59s inside, exactly 24h inside, 24h00m01s outside — stamps a ledger entry in the past and records against it. No fixture waits, and the whole suite runs in milliseconds.
- **The guard sees a force push that a naive splitter would miss, in three shapes.** Behind a separator (`echo hi && git push --force`), inside a nested shell (`bash -c "git push --force"`), and spelled either way (`git  push   --force` and `git push -f` reach the same verdict, because argv is parsed rather than prefix-matched). A command whose words cannot be recovered, a verb assembled by expansion, an `eval`, an unparseable request and a Bash call with no readable command are each **denied**, never permitted.
- **No crate was added.** The splitter is hand-rolled against `std` with an exhaustive test table, which is what keeps T-19-SC (the package-legitimacy threat) holding for the whole phase.
- **The guard's latency budget is enforced, not intended.** No network on any path (asserted by a source scan that first proves it extracted the right function body); the repository is consulted for exactly one command shape — a `push` with no refspec — through a predicate that shares git's option grammar with the classifier rather than re-deriving it; the ledger read is one pass. The registered `timeout` is a named constant asserted by a test, because a `PreToolUse` hook with no timeout is precisely the configuration that produced the reproduced 180-240 second hang at `src/executor/mod.rs:225-239`.
- **The settings file's silent-ignore hazard is closed by construction and by a refusal.** Typed serialisation (`awk`-verified: no `format!`, `push_str` or `concat!` in either `write_settings` function), a deserialise-and-compare immediately after `persist` that returns an **error** rather than a warning, and a second-carrier table at the generator that names the one control — the PR cap — which has no git-hook counterpart, instead of implying full coverage.

## Task Commits

1. **Task 1 (RED): the cap's boundary, torn tail and write-before-permit as failing rows** — `3789ada` (test)
2. **Task 1 (GREEN): the append-only PR ledger and the rolling-window cap** — `4ed779b` (feat)
3. **Task 2: the `PreToolUse` guard — parse argv, share the classifier, answer fast** — `84a9b05` (feat)
4. **Task 3: the settings file is generated, round-tripped and compared** — `551457c` (test)
5. **Deviation follow-up: name the guard's run-id variable once** — `aa6cbf0` (fix)

_No REFACTOR commit on the TDD task: the GREEN implementation needed no cleanup pass, and an empty `refactor` commit is ceremony rather than a change. The behaviour-preserving extraction inside `policy.rs` (Task 2's `scan_leading` / `push_operands`) travelled with the feature commit that required it, verified by the 31 pre-existing `classify_git` tests passing unchanged across it._

## Files Created/Modified

- `src/envelope/ledger.rs` (new, 726 lines) — `LedgerEntry`, `ledger_path`/`ledger_path_in`, `CapVerdict` + `refusal_detail`, `record_and_check`/`record_and_check_in`, `append_entry`, `ends_mid_line`, `tally`, `saturating_bump`, `parse_stamp`; 16 unit tests
- `src/envelope/policy.rs` — `Token`, `split_command`, `split_segments`, `tokenize`, `is_separator`, `program_name`, `pr_command_label`, `classify_pr_command`, `subcommand_words`, `gh_api_posts_a_pull_request`, `endpoint_is_pulls`, `push_needs_resolved_dests`; plus the `scan_leading` / `push_operands` extraction that lets the guard and the classifier share one option grammar. 47 tests (was 31)
- `src/envelope/hooks.rs` — `GuardRequest`, `guard`/`guard_in`, `classify_segments`, `shell_c_payload`, `current_run_id`, `resolve_policy`, `read_guard_request`, `deny`; `GUARD_TIMEOUT_SECS`, `EnvelopeSettings`/`SettingsPermissions`/`SettingsHooks`/`HookMatcher`/`HookCommand`, `settings_value`, `guard_command`, `settings_json`, `write_settings`/`write_settings_in`, `persist_settings`, `verify_settings`. 33 tests (was 19)
- `src/envelope/cred.rs` — `RUN_ID_ENV`, beside `PROJECT_ROOT_ENV`
- `src/envelope/mod.rs` — `pub mod ledger;`
- `src/cli.rs` — `EnvelopeAction::Guard { alias }`, with the "every argument resolved here is latency paid on every tool call" rationale at the variant
- `src/main.rs` — the `Guard` arm, before `tui::init()`, failing closed with exit 2 and a redacted diagnostic
- `tests/envelope_pr_cap.rs` (new, 481 lines) — 11 integration fixtures, all offline, agent-free and clock-free

## Decisions Made

See the `key-decisions` frontmatter for the full list. The four that a later reader is most likely to want the reasoning for:

- **A permit writes nothing at all.** The plan says to write the permission decision in the shape the hook protocol expects. Emitting `"permissionDecision": "allow"` would do that and would also make a deny-only control into an **approval authority**, auto-approving commands that the user's own permission rules would otherwise have prompted for. A layer added to bound a run must not quietly widen it. Silence on a permit leaves the ordinary permission flow exactly where it was; a test asserts stdout is empty.
- **A denial travels twice.** The protocol JSON on stdout is the first carrier; exit code 2 with the reason on stderr is the second. This is D-07's own argument — a control with one carrier is a control a schema change silently disarms — applied to the guard's answer rather than only to the file that registers it.
- **An unparseable ledger line counts toward the window but not the per-run bound.** Counting it toward both looked like the more conservative reading, and is not: the ledger is append-only and never trimmed, so one torn write would refuse the first pull request of **every future run, forever**. Counting it in the window is bounded and self-healing — it expires after 24 hours, exactly like the attempt it stands for.
- **The PR cap's missing second carrier is stated rather than papered over.** D-07 requires that every control the settings file carries is also carried by argv or by a git hook. Four of the five are. The cap is not, because no git hook observes `gh pr create` — it is not a git operation. What remains for it is the argv delivery (`settings_json` for `--settings` inline) and the round-trip check. Writing a table that implied otherwise would have been the overstated safety claim D-27 says is worse than a stated limitation.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] `split_segments`: a single-command splitter is a hole, not a control**

- **Found during:** Task 2
- **Issue:** The plan names `split_command(cmd) -> Option<Vec<String>>` and the guard classifying "the argv". `echo hi && git push --force origin main` splits to an argv whose program is `echo`; a guard that classified that argv would never reach `classify_git` and would **allow** the force push sitting behind the `&&`. Every compound command is a bypass of layer 2 in one keystroke.
- **Fix:** `split_segments` splits on unquoted control operators and the guard classifies every segment. The tokenizer records whether each word was quoted, so `git commit -m 'fix; and push'` stays one segment.
- **Files modified:** `src/envelope/policy.rs`, `src/envelope/hooks.rs`
- **Verification:** `a_force_push_hidden_behind_a_separator_is_still_seen`, `a_separator_inside_quotes_is_a_character_not_a_separator`, `every_command_behind_a_separator_is_its_own_segment`
- **Committed in:** `84a9b05`

**2. [Rule 2 - Missing Critical] Nested `sh -c` payloads are followed; expansion-assembled verbs and `eval` are denied**

- **Found during:** Task 2
- **Issue:** `bash -c "git push --force origin main"` is an ordinary thing for an agent to write and is not an evasion at all, but its outer program is `bash` and the classifier would have seen nothing. Symmetrically, `$TOOL push --force` and `eval "git push --force"` are commands whose effect is not knowable before the shell runs, and the plan's rule is that what cannot be recovered is denied.
- **Fix:** payloads of `sh|bash|zsh|dash|ksh -c` are re-split and re-classified to a bounded depth of 2 (the bound is itself latency protection); a segment whose program token carries the expansion flag, or whose program is `eval`, refuses with `envelope_assertion_failed`.
- **Files modified:** `src/envelope/hooks.rs`, `src/envelope/policy.rs`
- **Verification:** `a_force_push_inside_a_nested_shell_is_still_seen`, `a_verb_assembled_by_expansion_and_an_eval_are_both_denied`, `a_word_subject_to_expansion_is_flagged_because_its_value_is_unknowable`
- **Committed in:** `84a9b05`

**3. [Rule 1 - Bug] Appending onto a torn tail fused two records into one and under-counted the cap**

- **Found during:** Task 1 (the RED run surfaced it: the torn-line row reported `used_24h: 1` where two attempts existed)
- **Issue:** `create(true).append(true)` writes immediately after the last byte. A ledger whose final record was torn has no trailing newline, so the new entry concatenates onto it and the pair parses as a single unparseable line — counted **once**. The very failure the append-only format bounds would have swallowed the attempt being recorded, which is the under-counting direction the whole file exists to prevent.
- **Fix:** `append_entry` probes the last byte (`ends_mid_line`, a one-byte seek rather than a whole-file read, because this runs on the critical path) and prefixes a newline when the file does not end in one.
- **Files modified:** `src/envelope/ledger.rs`
- **Verification:** `a_torn_final_line_is_counted_toward_the_cap_rather_than_skipped` (now `used_24h: 2`) and `appending_onto_a_torn_tail_does_not_fuse_the_two_records_into_one`
- **Committed in:** `4ed779b`

**4. [Rule 3 - Blocking] `scan_leading` and `push_operands` extracted so the guard and the classifier share one option grammar**

- **Found during:** Task 2
- **Issue:** The plan's latency rule is that "a bare `push` with no refspec is the only case that touches the repository". Answering "is this a push with no refspec?" needs git's leading-option scan and its operand parser — both of which lived inside `classify_git`/`classify_push`. A second copy would drift from git's grammar, and the drift is bidirectional harm: a context resolved for the wrong argv is latency spent for nothing, and a context *not* resolved for the right argv reads as an unresolvable destination and refuses a push that was inside the namespace.
- **Fix:** `scan_leading(argv) -> (usize, Option<GitVerdict>)` and `push_operands(rest) -> Result<Vec<&str>, GitVerdict>` extracted; `classify_git`, `classify_push` and the new `push_needs_resolved_dests` all read them.
- **Files modified:** `src/envelope/policy.rs`
- **Verification:** the 31 pre-existing `envelope::policy` tests pass unchanged across the extraction (behaviour-preserving), plus `only_a_push_with_no_refspec_needs_the_repository_consulted`
- **Committed in:** `84a9b05`

**5. [Rule 2 - Missing Critical] `gh` global value-flags and `gh api`'s implies-POST fields**

- **Found during:** Task 2
- **Issue:** Two forms would have gone uncounted, and an uncounted creation is the under-counting failure D-20 names. `gh --repo o/r pr create` yields the non-flag words `o/r pr create`, so a match on `["pr", "create"]` fails. And `gh api repos/o/r/pulls -f title=x` opens a pull request without ever spelling `POST`, because `gh api` defaults to POST once a field is supplied — a method-only check misses the most common raw-API creation form.
- **Fix:** `FORGE_VALUE_OPTS` (`-R`, `--repo`, `--hostname`) consumed in `subcommand_words`; `GH_API_IMPLIES_POST` (`-f`, `--raw-field`, `-F`, `--field`, `--input`) treated as POST when no explicit method is given.
- **Files modified:** `src/envelope/policy.rs`
- **Verification:** `the_github_pull_request_creation_form_is_recognised`, `a_raw_api_post_to_a_pulls_path_is_recognised_in_both_its_spellings`, and the paired `a_read_only_listing_form_is_not_a_creation` — which also asserts a `POST` to `…/pulls/7/reviews` is not a creation, so the widening did not over-count
- **Committed in:** `84a9b05`

**6. [Rule 2 - Missing Critical] `cred::RUN_ID_ENV` names the guard's run-id variable once**

- **Found during:** Task 3's review pass
- **Issue:** `current_run_id` read a bare `"GSD_MM_RUN_ID"` literal, and the writer of that variable is plan 19-06's spawn closure. Two ends agreeing on a string neither can see the other type is a typo away from every tool call reporting a different run — the per-run cap silently unenforced, with nothing indicating it.
- **Fix:** `pub const RUN_ID_ENV` in `cred.rs` beside `PROJECT_ROOT_ENV`, with a doc stating that wiring it is the driver's job and not `build_env`'s (which is given an alias and a project root, not a run).
- **Files modified:** `src/envelope/cred.rs`, `src/envelope/hooks.rs`
- **Verification:** `cargo clippy -- -D warnings` and the full `envelope` suite green after the swap
- **Committed in:** `aa6cbf0`

**7. [Process] The `awk '/pub fn guard/,/^}/'` criterion was anchored so it can mean what it says**

- **Found during:** Task 2, checking acceptance criteria
- **Issue:** The criterion greps the guard's function range for HTTP-client names and requires 0. The unanchored range `/pub fn guard/` also matches inside `mod tests` and therefore swallows **the test that asserts the property**, whose assertion list necessarily contains those literals. As written the criterion could never return 0 while the proof of it existed. This is the same shape as plan 19-02's deviation 4.
- **Fix:** the criterion is reported anchored — `awk '/^pub fn guard/,/^}/'` — which covers `guard` and `guard_in` (both at column 0) and excludes the indented test. It returns **0**. No production code changed.
- **Files modified:** none
- **Verification:** `awk '/^pub fn guard/,/^}/' src/envelope/hooks.rs | grep -cE 'reqwest|ureq|hyper|curl'` → `0`; the in-source test additionally asserts it extracted the right body before asserting what is absent, so it cannot pass vacuously
- **Committed in:** n/a (criterion interpretation)

---

**Total deviations:** 7 (4 missing-critical/security, 1 bug, 1 blocking, 1 process)
**Impact on plan:** No scope creep. Deviations 1, 2, 3 and 5 each close a way this plan could have shipped a control that passed its own acceptance criteria without working — a force push behind an `&&`, a force push inside `bash -c`, a torn write that swallowed the attempt after it, and two pull-request creation forms that would never have been counted. Deviation 4 is the mechanical prerequisite for the plan's own latency rule. Deviations 6 and 7 changed no behaviour.

## Issues Encountered

None outside the deviations above. The torn-tail fusion (deviation 3) was surfaced by the RED run rather than discovered later, which is the argument for the RED step rather than a happy accident.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo test` | exit 0 — **937 passing, 0 failed** (baseline 880 after 19-04; +57) |
| `cargo test --lib envelope::ledger` | exit 0 — 16 tests |
| `cargo test --lib envelope::policy` | exit 0 — 47 tests (was 31; the pre-existing 31 pass unchanged across the extraction) |
| `cargo test --lib envelope::hooks` | exit 0 — 33 tests (was 19) |
| `cargo test --test envelope_pr_cap` | exit 0 — 11 tests (≥ 6 required) |
| `cargo test --test spawn_seam_guard` | exit 0 — 7 tests; **allowlist unchanged**, because neither `guard` nor `write_settings` introduces a spawn site |
| `cargo clippy -- -D warnings` | exit 0 |
| `cargo clippy --test envelope_pr_cap -- -D warnings` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | **exactly 5** pre-existing lints — `browser.rs:131/132/133`, `project_creator.rs:146`, `state_reader/mod.rs:258`. Count and locations unchanged. Measured with `rtk proxy` per D-34. |
| `grep -nE '\bf(32\|64)\b\|as f64\|as f32' src/envelope/ledger.rs` | no match — no floating-point arithmetic in the decision |
| `grep -c 'saturating_' src/envelope/ledger.rs` | 10 (≥ 1 required) |
| `awk '/pub fn write_settings/,/^}/' src/envelope/hooks.rs \| grep -cE 'format!\|push_str\|concat!'` | **0** — the settings JSON is serialised from typed structs, never assembled |
| `awk '/^pub fn guard/,/^}/' src/envelope/hooks.rs \| grep -cE 'reqwest\|ureq\|hyper\|curl'` | **0** (anchored — see deviation 7) |
| `grep -c '^#[test]' tests/envelope_pr_cap.rs` | 11 (≥ 6 required) |
| `wc -l tests/envelope_pr_cap.rs` | 481 (≥ 160 required) |
| `grep -c 'pub fn record_and_check' src/envelope/ledger.rs` | 2 (`record_and_check`, `record_and_check_in`) |
| `grep -c 'pub fn write_settings' src/envelope/hooks.rs` | 2 (`write_settings`, `write_settings_in`) |
| `Cargo.toml` unchanged | **no crate added** — T-19-SC holds phase-wide |

**RED-step evidence (committed, not reverted):** at `3789ada` the cap decision is stubbed to permit and 7 of 15 `envelope::ledger` tests fail — the per-run bound, the fourth-in-window refusal, all three window-boundary rows, the torn tail and write-before-permit. All 15 pass at `4ed779b`. The 8 passing at both are the rows that must pass against the stub or they would be proving nothing.

## Pre-existing Failure (out of scope)

`tests/driver_reattach.rs`'s two intermittent failures did **not** reproduce in this plan's full runs (937 passing, 0 failed, on the final gate). They remain tracked at `.planning/todos/pending/2026-08-18-driver-reattach-spawn-artifact-race.md` and were not investigated or touched here, per the scope boundary. Nothing in 19-05 goes near `src/driver/`.

## Known Stubs

None. Every symbol this plan introduces is exercised by a passing test.

`write_settings`, `settings_json` and `guard` (the ambient-root variants) have no production caller yet — plan 19-06 owns the driver wiring — but none is a stub: each is the same code path the tests exercise through its explicit-argument sibling, differing only in how the root, the registry path or the binary is resolved. `cred::RUN_ID_ENV` likewise has a reader and no writer until 19-06; its absence degrades the per-run cap to a shared `unattributed-run` bucket, which **over**-counts across runs rather than under-counting, and is stated at `current_run_id`.

## Threat Flags

None new. No network endpoint was added (that is the point of D8), no schema changed, and **no crate was added to `Cargo.toml`** (T-19-SC holds).

Against this plan's own register:

| Threat | Disposition | Where it is closed |
|---|---|---|
| T-19-30 settings file ignored on validation failure | mitigated | typed serialisation, the run-refusing round-trip compare, and the second-carrier table — with the cap's missing carrier stated |
| T-19-31 shell command the splitter cannot recover | mitigated | `split_command` returns `None` and the guard denies; the limit is stated at the function rather than softened |
| T-19-32 agent resets its own cap | mitigated | ledger under the envelope directory, asserted by a path test; `Write`/`Edit` on `.claude/**` denied by both carriers |
| T-19-33 guard latency on the critical path | mitigated | no network (source-scanned), one-shape repository read, single-pass ledger read, registered timeout asserted by a test |
| T-19-34 driven repository `.claude/settings.json` | mitigated | `Write(.claude/**)` and `Edit(.claude/**)` on both carriers, asserted in `every_pattern_the_settings_file_denies_is_also_carried_on_argv` |
| T-19-35 a PR opened by a path the guard did not see | **accepted and stated** | write-before-permit biases to over-count; `CapVerdict::refusal_detail` says so in the refusal itself |
| T-19-36 torn ledger write | mitigated | append-only NDJSON, the torn line counted rather than skipped, and the fusion bug that would have under-counted it fixed (deviation 3) |
| T-19-37 guard request contents in logs | mitigated | `main.rs`'s `Guard` arm passes its diagnostic through `journal::redact`, and `guard_in` redacts the error it turns into a denial |
| T-19-SC package-manager installs | mitigated | the splitter is hand-rolled against `std`; `Cargo.toml` is untouched |

One residual is recorded because it is not in the register: **the pull-request cap has no second carrier that is a git hook**, so a settings file the CLI ignores leaves the cap unenforced while every push boundary stays standing. That is a degradation rather than a disarming — D-07's requirement — and it is written into `settings_value`'s doc rather than left for 19-07 to discover.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Ready for 19-06 (wiring), and it owns three specific calls.** At run start: `hooks::install(alias)` (already 19-01's), then `hooks::write_settings(alias)` — which **returns the path or refuses the run**, so its `?` is the D-07 gate and must not be downgraded to a warning — then `cred::build_env(alias, project_root)`. The child's argv takes `--settings <that path>` (or `hooks::settings_json(binary, alias)` inline, the argv-carried second carrier) alongside `--disallowedTools` from `policy::disallowed_tools()`, which is the same list the file carries.
- **19-06 must set `cred::RUN_ID_ENV` in the spawn closure.** Until it does, every guard invocation shares the `unattributed-run` bucket: the per-run cap then bounds *all* runs of that alias together, which over-counts rather than under-counts and is therefore safe to ship in this order — but it is not the intended semantics, and `current_run_id`'s doc says so.
- **19-06 should also pass the project root.** `guard` reads `cred::PROJECT_ROOT_ENV`; absent it, the one command shape that consults a repository (a `push` with no refspec) resolves against `.`, which is the child's cwd and usually correct but is not guaranteed to be.
- **For 19-07 (honesty text).** `split_command`'s doc already carries the layer-2/layer-3/remote-ruleset chain in the form the pinned constant should agree with, and `settings_value`'s second-carrier table carries the one honest gap (the cap). The pinned text should reference that reasoning rather than restate it differently — and its "what is NOT guaranteed" half now has a concrete new item: a settings file the CLI ignores leaves the cap unenforced.
- **Note for 19-08 (gate):** `SAFE-06` is declared by **19-05 alone**, so it is releasable; `SAFE-02` is shared with 19-02, 19-06, 19-07 and 19-08 and the shared-ID gate holds it. `REQUIREMENTS.md` was **not** touched by this plan — parallel worktree mode, the orchestrator owns those writes.
- **For Phase 20 (router).** `ParkReason::PrCapExceeded` is now emitted with the counts attached in `CapVerdict`, so a router can distinguish "wait" (window) from "start a new run" (per-run) without re-deriving anything.
- **No blockers.**

## Self-Check: PASSED

- Both created files present on disk (`src/envelope/ledger.rs`, `tests/envelope_pr_cap.rs`); all six modified files present in the diff against the wave base `476f3b9`.
- All five task commits present in `git log`: `3789ada`, `4ed779b`, `84a9b05`, `551457c`, `aa6cbf0`.
- Every task `<acceptance_criteria>` re-run and passing (see Verification Results); the plan-level `<verification>` re-run and passing, including the `rtk proxy` clippy-delta measurement.
- `must_haves.artifacts` confirmed: `src/envelope/ledger.rs` contains `pub fn record_and_check`; `src/envelope/hooks.rs` contains `pub fn write_settings`; `tests/envelope_pr_cap.rs` is 481 lines (≥ 160).
- `must_haves.key_links` confirmed: `hooks.rs` reaches `ledger.rs` through `record_and_check_in` (the explicit-root sibling, matching the `install_in`/`build_env_in` convention this phase established) and `policy.rs` through `classify_git`.
- `STATE.md` and `ROADMAP.md` deliberately untouched — parallel worktree mode, the orchestrator owns those writes.

---
*Phase: 19-gitsafe-git-blast-radius-envelope*
*Completed: 2026-08-18*
