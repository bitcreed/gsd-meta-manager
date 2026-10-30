---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 06
subsystem: infra
tags: [branch-protection, github-api, dry-run-preview, pinned-constant, transparency, safety-envelope]

# Dependency graph
requires:
  - phase: 19-gitsafe-git-blast-radius-envelope
    provides: "19-01's `envelope_root`/`envelope_dir_in` and the three-layer contract doc whose 'does NOT see' sentences the honesty statement had to agree with; 19-03's hook refusals and named blind spots; 19-04's `EnvelopeEnv`, `build_env_in` and `url_host`, plus the recorded askpass-readable-by-the-agent limit; 19-05's PR ledger and the recorded settings-file degradation"
  - phase: 16-run-journal
    provides: "`journal::redact` — the shipped redaction table every probe reason passes through; `JournalEvent::Diagnostic`'s stable-`code` convention, the style `ProtectionState::as_str` follows"
  - phase: 14-driver-preview
    provides: "`driver::dry_run`'s three pinned section constants and the ordering test this plan extended, and `state_reader::git_ops::git_read_raw`'s `--no-optional-locks` read shape"
provides:
  - "`envelope::advisory` — the read-only remote-protection probe and the pinned honesty statement"
  - "`envelope::advisory::ProtectionState` (`Protected`/`Unprotected`/`Unknown { reason }`), `as_str`, `reason`, `is_warning`"
  - "`envelope::advisory::probe_protection(project_root, env)` — rulesets with a branch-protection fallback, bounded, read-only, failure as data"
  - "`envelope::advisory::not_probed()` — the honest state for a surface that made no network call"
  - "`envelope::advisory::SECTION_ENVELOPE` — the pinned honesty statement, re-exported from `driver::dry_run`"
  - "`envelope::advisory::envelope_notice(state)` — one producer for the preview and the run journal"
  - "`envelope::advisory::protection_line(state)` and `PROTECTION_WARNING`"
  - "`driver::dry_run::DryRunReport.protection` and `DryRunReport::with_protection`"
  - "`envelope::cred::url_host` promoted to `pub(super)` — one parser, two callers"
affects: [19-07 wiring and run-start journal notice, 19-08 gate, 20 router]

# Actuals (#2632)
actuals:
  tokens: 15300
  tasks: 2
  commits: 3

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Failure separated from absence in the type: `ClientAnswer::Absent` (the remote answered, and the thing is not there) is a distinct variant from `ClientAnswer::Failed` (the query never completed). Folding them together is how a proxy error comes to render as `unprotected` — or, inverted somewhere later, as a false assurance."
    - "Redaction at the one constructor: `ProtectionState::unknown` is private and applies `journal::redact`, so every reason — client stderr, remote URLs, home-directory paths — is sanitised at the single site that can build the variant, rather than at each of the nine places one is produced."
    - "The pinned-part test matches one short phrase per required part rather than the whole paragraph, so a re-wrap of the prose passes and a DROPPED clause fails. A test that pinned the whole text would be reverted the first time somebody reflowed it."
    - "The write-path prohibition is enforced over the whole file including its prose: `the_module_has_no_write_path_at_all` scans for the four write verbs and requires none, which is why the module doc says 'no write path' rather than naming the verbs it declines to use."

key-files:
  created:
    - src/envelope/advisory.rs
    - tests/envelope_advisory.rs
  modified:
    - src/envelope/mod.rs
    - src/envelope/cred.rs
    - src/driver/dry_run.rs
    - src/ui/screens/driver.rs
    - tests/spawn_seam_guard.rs

key-decisions:
  - "The dry-run preview does NOT probe, and says so with the reason. `SECTION_REFSPECS` claims 'No network was contacted' in the same output; a preview that quietly made a network call for the protection state would falsify its own neighbouring claim. So `build_report` yields `not_probed()` — an `Unknown` naming the constraint — and `DryRunReport::with_protection` carries the real state for the caller (19-07's run start) that actually probed."
  - "The probe is bounded by a 10-second budget with a hand-rolled wait loop, not `output()`. `output()` waits forever; a run-start network call that never returns is a run that never begins, and this project already shipped a mitigation for a reproduced 180-240 second hang of exactly that shape."
  - "`Absent` is a separate `ClientAnswer` variant from `Failed`. The branch-protection endpoint answers 404 for an unprotected branch, which is evidence; a network or permission failure is not. Two independent misses — no active branch ruleset AND no branch protection — is what `unprotected` means here, and one failed query is `unknown`."
  - "A rulesets query that fails is not a verdict but a reason to ask the other endpoint only when the host lacks the endpoint (`Absent`); a failed query returns `Unknown` immediately rather than falling through, so a transient failure cannot be laundered into an `unprotected` by the fallback."
  - "`is_github_host` matches `github.com` and its subdomains, never `contains(\"github\")` — `github.com.evil.example` contains it. A host this refuses yields `Unknown`, which warns rather than reassures."
  - "`repo_slug` and `default_branch_of` validate their components before interpolation into an API path. A remote URL is repository-controlled data, and a component carrying `..` would aim the read somewhere else entirely."
  - "`PROTECTION_WARNING` is the word `WARNING`, not a glyph. This text goes to a plain stdout preview and into a run journal; a symbol that renders as a box in somebody's terminal is a warning that did not warn."
  - "The existing ordering test keeps its name although it now asserts four sections. Extending the one assertion rather than adding a second beside it is what keeps exactly ONE place pinning the order — and the plan's own acceptance criterion pins the name to enforce that."
  - "`cred::url_host` was promoted to `pub(super)` rather than copied. A second parser is a second place for the look-alike cases — userinfo containing an `@`, an IPv6 literal's colons, a bare local path — to drift, and the drift would show as a probe reasoning about a host the credential responder never scoped."

patterns-established:
  - "Not-probed is a first-class state, not a missing section: a surface that made no query reports `unknown` carrying the constraint that produced it. An omitted section reads as an absent hazard."
  - "Contract constants live with their producer, not with their first consumer: `SECTION_ENVELOPE` is defined in `envelope::advisory` and re-exported from `driver::dry_run`, because the run journal is the second consumer and a constant owned by the preview would imply the preview owns the claim."

requirements-completed: [SAFE-01, SAFE-02]  # Both are shared with sibling plans (SAFE-01 with 19-01, SAFE-02 with 19-02/05/07/08); the orchestrator's shared-ID gate releases them when the last declaring plan lands.

coverage:
  - id: D1
    description: "The remote's protection state is probed read-only and recorded as `protected`, `unprotected` or `unknown` carrying a reason; every failure mode — no remote, a non-GitHub host, a missing client, a network or permission failure, an exceeded budget, an unreadable answer — yields `unknown` with THAT specific reason"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/envelope_advisory.rs#a_file_url_remote_is_unknown_with_that_as_the_reason_rather_than_unprotected"
        status: pass
      - kind: integration
        ref: "tests/envelope_advisory.rs#a_repository_with_no_remote_at_all_is_unknown_and_says_so"
        status: pass
      - kind: integration
        ref: "tests/envelope_advisory.rs#the_two_offline_reasons_are_distinct_so_a_reader_can_tell_them_apart"
        status: pass
      - kind: unit
        ref: "src/envelope/advisory.rs#the_three_states_carry_stable_identifiers, #an_unknown_carries_its_reason_and_the_known_states_carry_none"
        status: pass
      - kind: unit
        ref: "src/envelope/advisory.rs#an_answer_that_says_the_resource_is_absent_is_told_apart_from_a_failure"
        status: pass
    human_judgment: false
  - id: D2
    description: "`unprotected` and `unknown` both emit a warning wherever they are rendered, and `unknown` is never rendered as `protected`"
    requirement: SAFE-01
    verification:
      - kind: integration
        ref: "tests/envelope_advisory.rs#an_unknown_renders_its_reason_and_never_borrows_the_protected_wording"
        status: pass
      - kind: integration
        ref: "tests/envelope_advisory.rs#an_unprotected_state_renders_with_the_warning_marker"
        status: pass
      - kind: integration
        ref: "tests/envelope_advisory.rs#the_preview_warns_for_an_unprotected_remote_and_shows_the_reason_for_an_unknown_one"
        status: pass
      - kind: unit
        ref: "src/envelope/advisory.rs#both_unprotected_and_unknown_warrant_a_warning_and_protected_does_not"
        status: pass
    human_judgment: false
  - id: D3
    description: "The envelope never applies protection: no code path in this module can mutate a ruleset or protection endpoint, because that needs an administration scope the run credential deliberately lacks"
    requirement: SAFE-02
    verification:
      - kind: integration
        ref: "tests/envelope_advisory.rs#the_module_has_no_write_path_at_all (source-scanning, with an extraction-integrity assertion first)"
        status: pass
      - kind: other
        ref: "grep -cE 'POST|PUT|PATCH|DELETE' src/envelope/advisory.rs returns 0 — the module has no write path even in its prose"
        status: pass
      - kind: other
        ref: "awk '/pub fn probe_protection/,/^}/' src/envelope/advisory.rs | grep -cE '\\-X (POST|PUT|PATCH|DELETE)|--method (POST|PUT|PATCH|DELETE)' returns 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "The honesty statement is a pinned constant whose three required parts each fail the build if dropped, in the same contract-constant style as the existing dry-run section constants"
    requirement: SAFE-01
    verification:
      - kind: integration
        ref: "tests/envelope_advisory.rs#the_honesty_statement_carries_each_of_its_three_required_parts (seven distinctive phrases plus an ordering assertion)"
        status: pass
      - kind: other
        ref: "grep -c 'SECTION_ENVELOPE' src/envelope/advisory.rs returns 2 (definition and its doc reference)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The statement says without hedging what is NOT guaranteed — client-side hooks, tool denies and env-injected git configuration are defeatable by an agent that can spawn an unsupervised shell — and concludes with the server-side branch protection recommendation"
    requirement: SAFE-01
    verification:
      - kind: integration
        ref: "tests/envelope_advisory.rs#the_honesty_statement_carries_each_of_its_three_required_parts (asserts the part-2 and part-3 phrases and that part 3 follows part 2)"
        status: pass
    human_judgment: true
    rationale: "A test proves the clauses are present and ordered. Whether the paragraph READS as an honest account of the real ceiling — rather than as a hedge or a pre-excuse for every future gap — is a judgement only a reader can make, and it is the one thing in this phase that most needs a human's eye. SAFE-01's transparency prohibition is verified by `judgment` for exactly this reason."
  - id: D6
    description: "The dry-run preview renders four sections in a fixed order, and the ordering test that previously asserted three now asserts four, so a section cannot silently disappear"
    requirement: SAFE-01
    verification:
      - kind: unit
        ref: "src/driver/dry_run.rs#the_rendered_report_carries_all_three_section_headers_in_order (four byte offsets; name deliberately unchanged so there is one pin, not two)"
        status: pass
      - kind: unit
        ref: "src/driver/dry_run.rs#every_section_prints_something_even_for_a_clean_tree_with_no_remote"
        status: pass
      - kind: other
        ref: "git diff 740e62f -- src/driver/dry_run.rs | grep -cE '^\\+.*fn the_rendered_report_carries' returns 0 — extended, not duplicated"
        status: pass
    human_judgment: false
  - id: D7
    description: "The preview and the run journal share one producer for the claim text, so they cannot disagree about what was claimed"
    requirement: SAFE-01
    verification:
      - kind: integration
        ref: "tests/envelope_advisory.rs#the_journal_notice_and_the_rendered_preview_carry_the_same_claim_text (all three states)"
        status: pass
    human_judgment: false
  - id: D8
    description: "The probe never runs on the guard's per-tool-call path, and cannot hang a run: it is bounded by its own budget and killed at the deadline"
    requirement: SAFE-02
    verification:
      - kind: other
        ref: "src/envelope/advisory.rs `run_client`'s hand-rolled wait loop with PROBE_BUDGET_SECS and the kill-at-deadline path; the constraint is stated in both the module doc and `probe_protection`'s doc"
        status: pass
      - kind: other
        ref: "19-05's `the_guard_makes_no_network_call_on_any_path_it_takes` still passes — the guard reaches no code in this module"
        status: pass
    human_judgment: true
    rationale: "The budget's code path is present and documented, but no test drives a hanging remote — doing so would need a network fixture the phase's own scope fence (D-35) forbids. That the bound is correct under a real stall is a judgement, and it is recorded rather than claimed as proved."

# Metrics
duration: 34 min
completed: 2026-08-18
status: complete
---

# Phase 19 Plan 06: The Protection Probe and the Pinned Honesty Statement Summary

**A remote whose protection cannot be read is reported as `unknown` carrying the reason it is not known — never as `protected`, never as a silence — and the phase's honest conclusion is now a constant whose three required parts (what IS guaranteed, what is NOT, and therefore enable server-side branch protection) each fail the build if a later refactor drops one, rendered as the fourth section of a preview whose one ordering test was extended rather than duplicated.**

## Performance

- **Duration:** ~34 min
- **Started:** 2026-08-18 (base commit `82ab462`)
- **Completed:** 2026-08-18
- **Tasks:** 2
- **Files modified:** 7 (2 created, 5 modified)

## Accomplishments

- **The one artifact in this phase that is legitimately text is now mechanically pinned.** `SECTION_ENVELOPE` gets the same contract-not-decoration doc treatment the three dry-run section constants carry, and `the_honesty_statement_carries_each_of_its_three_required_parts` asserts a distinctive phrase from each of its three parts plus their order. A rewording passes; a dropped clause is a build failure. That asymmetry is the whole design — pinning the entire paragraph would have been reverted the first time somebody reflowed it.
- **The "what is NOT guaranteed" half carries the concrete items the earlier waves recorded, rather than a generic disclaimer.** An agent that unsets `GIT_CONFIG_COUNT` in a subshell is past the last layer (19-01's ceiling); an agent that runs the askpass responder itself reads the token (19-04's stated limit); a settings file the agent's own CLI silently ignores leaves the pull-request cap unenforced, because no git hook observes a pull request (19-05's recorded degradation). The statement agrees with those module docs rather than restating them more comfortably.
- **A probe failure is a statement about the probe, never about the remote.** Nine distinct failure modes each produce `Unknown` carrying their own reason, and `the_two_offline_reasons_are_distinct_so_a_reader_can_tell_them_apart` asserts two of them do not collapse into one string — because the reason is the entire value of an `unknown`. `ClientAnswer::Absent` is a separate variant from `ClientAnswer::Failed` for the same reason at the layer below: the branch-protection endpoint's 404 is evidence, and a proxy error is not.
- **The module has no write path, in code or in prose.** `the_module_has_no_write_path_at_all` scans the whole file for the four write verbs and requires none — which is why the module doc says "no write path" instead of naming the verbs it declines to use. The test asserts it is scanning the right file before asserting what is absent, so it cannot pass vacuously.
- **The preview does not probe, and says so with the reason.** `SECTION_REFSPECS` claims "No network was contacted" in the same output; a preview that quietly made a network call would have falsified its own neighbouring claim. `build_report` yields `not_probed()` and `DryRunReport::with_protection` carries a state somebody actually probed. Not-probed is a first-class state here, not a missing section — an omitted section reads as an absent hazard.
- **The probe cannot hang a run.** A hand-rolled wait loop with a 10-second budget and a kill at the deadline, rather than `output()`, which waits forever. This project already shipped a mitigation for a reproduced 180-240 second hang of exactly that shape, and the constraint that the probe never runs on the guard's per-tool-call path is stated in both the module doc and the function's own doc.
- **No crate was added.** `Cargo.toml` is untouched, so T-19-SC holds phase-wide.

## Task Commits

1. **Task 1: the remote-protection probe — read-only, failure as data** — `ed46c5c` (feat)
2. **Task 2: the pinned honesty statement, and a fourth section the preview cannot lose** — `e3ada60` (feat)

**Plan metadata:** see the `docs(19-06)` commit.

## Files Created/Modified

- `src/envelope/advisory.rs` (new, 749 lines) — `ProtectionState` + `as_str`/`reason`/`is_warning`/`unknown`, `not_probed`, `SECTION_ENVELOPE`, `envelope_notice`, `PROTECTION_WARNING`, `protection_line`, `probe_protection`, `ClientAnswer`, `run_client`, `first_line`, `says_absent`, `remote_url`, `is_github_host`, `repo_slug`, `is_plain_component`, `active_branch_ruleset`, `default_branch_of`; 12 unit tests
- `tests/envelope_advisory.rs` (new, 314 lines) — 9 integration tests, every fixture offline (`file://` remote, a repository with no remote, and pure rendering)
- `src/envelope/mod.rs` — `pub mod advisory;`
- `src/envelope/cred.rs` — `url_host` promoted to `pub(super)`, with the why-not-copied paragraph at the function
- `src/driver/dry_run.rs` — `SECTION_ENVELOPE` re-export, `DryRunReport.protection` + `with_protection`, the fourth section in `render`, the extended ordering test and two new preview tests; the module doc now describes four sections and why the fourth is not probed
- `src/ui/screens/driver.rs` — the preview-pane test's report literal carries the new field
- `tests/spawn_seam_guard.rs` — `src/envelope/advisory.rs` on the allowlist, added in the same commit as the spawn site

## Decisions Made

See `key-decisions` in the frontmatter for the full list. The three a later reader is most likely to want the reasoning for:

- **The dry run makes no network call.** This was the one genuine fork in the plan. D-26 says the protection state goes into the dry-run preview, and the preview has no `EnvelopeEnv` to run a client under — building one has side effects (a generated gitconfig, an askpass stub, a `gh` directory). More decisively, the preview's own third section claims no network was contacted. Probing there would have made the tool's output contradict itself in the space of ten lines, which is the transparency failure this plan exists to prevent, in miniature. The state is therefore `unknown` **with the constraint as its reason**, and 19-07's run-start wiring supplies the probed state through `with_protection`.
- **A failed query is never laundered into a verdict.** Only an `Absent` answer from the rulesets endpoint (an older host that has no such endpoint) falls through to the branch-protection endpoint. A `Failed` rulesets query returns `Unknown` immediately, so a transient network failure cannot combine with a subsequent 404 to produce `unprotected`.
- **The ordering test keeps its old name.** It now asserts four offsets while being called `..._all_three_section_headers_in_order`. The plan's acceptance criterion pins the name precisely to force the extend-don't-duplicate outcome, and the comment above the test says why the name is stale on purpose. Two ordering tests are two things that can disagree, and the one that gets updated is not necessarily the one somebody reads.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] The probe is bounded by its own budget; the plan specified no timeout**

- **Found during:** Task 1
- **Issue:** The plan's probe sequence is three `gh` invocations and nothing bounds them. `std::process::Command::output()` waits forever. The plan's own threat register (T-19-41) bounds the *guard* path and explicitly excludes the probe from it — but "not on the critical path" is not "cannot hang". A run-start query that never returns is a run that never begins, with no signal distinguishing it from a slow start, which is precisely the failure shape `src/executor/mod.rs:225-239` records as reproduced.
- **Fix:** `run_client` spawns with piped stdio, polls `try_wait` every 25 ms against a 10-second deadline, kills the child at the deadline, and returns `Failed` naming the budget — which becomes an `Unknown` carrying that reason, in the same failure-as-data shape as every other path.
- **Files modified:** `src/envelope/advisory.rs`
- **Verification:** the budget path is exercised structurally (the loop compiles and the deadline branch is reachable); no test drives a real stall, which is recorded honestly as `human_judgment: true` on coverage D8 rather than claimed as proved. D-35 forbids the network fixture that would prove it.
- **Committed in:** `ed46c5c`

**2. [Rule 2 - Missing Critical] `Absent` separated from `Failed`, so a query failure cannot become a verdict**

- **Found during:** Task 1
- **Issue:** The plan's step 4 folds "a network failure, a permission failure, an unparseable response" into `Unknown` — correct — but the branch-protection endpoint answers **404 for an unprotected branch**, which is how `unprotected` is detected at all. A single `Result<String, String>` would have forced either treating every failure as `unprotected` (false alarms, and the wrong direction if ever inverted) or treating the 404 as `unknown` (in which case no repository could ever be reported unprotected).
- **Fix:** `enum ClientAnswer { Body, Absent, Failed }`, with `says_absent` matching the client's own complaint. Two independent misses — no active branch ruleset and no branch protection — is what `unprotected` means; one failed query is `unknown`.
- **Files modified:** `src/envelope/advisory.rs`
- **Verification:** `an_answer_that_says_the_resource_is_absent_is_told_apart_from_a_failure` asserts a 401 and a DNS failure are **not** absence
- **Committed in:** `ed46c5c`

**3. [Rule 2 - Missing Critical] Slug and default-branch components are validated before interpolation**

- **Found during:** Task 1
- **Issue:** `owner`, `repo` and `default_branch` are interpolated into an API path, and all three come from repository-controlled data (the remote URL, and the remote's own answer). A component carrying `..` or a slash would aim the read at some other path entirely.
- **Fix:** `is_plain_component` restricts the slug's two components to `[A-Za-z0-9._-]` and rejects `.`/`..`; `default_branch_of` allows `/` (real branches contain it) but rejects any `..`.
- **Files modified:** `src/envelope/advisory.rs`
- **Verification:** `a_slug_component_that_could_aim_the_read_elsewhere_is_refused`, `the_default_branch_is_read_and_a_traversal_shaped_one_is_refused`
- **Committed in:** `ed46c5c`

**4. [Rule 3 - Blocking] `protection_line` exists as a Task 1 symbol, not a Task 2 one**

- **Found during:** Task 1
- **Issue:** Task 1's acceptance criteria require a test asserting the **rendered** `Unknown` state carries its reason and not the protected wording, but the plan places all rendering in Task 2 (`SECTION_ENVELOPE`, `envelope_notice`). Task 1 was therefore unverifiable as written.
- **Fix:** `protection_line(state)` — the one-line rendering of a state — landed in Task 1, and Task 2's `envelope_notice` composes it with the statement. This is the same producer factoring the plan asks for, split one function earlier.
- **Files modified:** `src/envelope/advisory.rs`
- **Verification:** `an_unknown_renders_its_reason_and_never_borrows_the_protected_wording` passes in the Task 1 commit
- **Committed in:** `ed46c5c`

**5. [Rule 3 - Blocking] `cred::url_host` promoted rather than a second parser written**

- **Found during:** Task 1
- **Issue:** The probe has to answer "is this remote on a GitHub host?", which `cred.rs` already answers for the askpass responder's scoping — including the awkward cases (userinfo containing `@`, an IPv6 literal's own colons, a bare local path, `file://` yielding `None`).
- **Fix:** `url_host` promoted to `pub(super)` with a paragraph at the function stating why a copy would be worse, in the same shape as 19-04's `sh_quote` promotion.
- **Files modified:** `src/envelope/cred.rs`, `src/envelope/advisory.rs`
- **Verification:** the 20 pre-existing `envelope::cred` tests pass unchanged; `only_github_com_and_its_subdomains_are_treated_as_a_probeable_host` covers the new caller's question
- **Committed in:** `ed46c5c`

**6. [Rule 2 - Missing Critical] The "no write path" grep is satisfied over the prose too**

- **Found during:** Task 1
- **Issue:** The plan's second criterion — `grep -cE 'POST|PUT|PATCH|DELETE' src/envelope/advisory.rs` returns 0 — is annotated "for any request-issuing line", but the literal grep covers the whole file including doc comments. The looser reading would also have let a source-scanning test live *inside* the module and swallow its own assertion literals, which is the vacuous-extraction shape 19-02 and 19-05 both hit.
- **Fix:** the file avoids the four verbs entirely, including in prose (the doc says "no write path" and "a request which could change anything on the remote"), and the source-scanning test lives in `tests/envelope_advisory.rs` where its own literals cannot contaminate the target it scans.
- **Files modified:** `src/envelope/advisory.rs`, `tests/envelope_advisory.rs`
- **Verification:** `grep -cE 'POST|PUT|PATCH|DELETE' src/envelope/advisory.rs` → **0**; `the_module_has_no_write_path_at_all` asserts extraction integrity first
- **Committed in:** `ed46c5c`

---

**Total deviations:** 6 (4 missing-critical/security, 2 blocking)
**Impact on plan:** No scope creep. Deviations 1, 2 and 3 each close a way this plan could have shipped a probe that passed its own criteria while being wrong — a query that hangs a run, a network failure laundered into a verdict, and a repository-controlled string aiming a read elsewhere. Deviation 6 makes an acceptance criterion mean what it says. Deviations 4 and 5 are mechanical.

## Issues Encountered

None. Every verification passed on first execution.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo test` | exit 0 — **960 passing, 0 failed** (baseline 937 after 19-05; +23) |
| `cargo test --test envelope_advisory` | exit 0 — 9 tests, **none skipped** (verified with `rtk proxy … --nocapture`) |
| `cargo test --lib envelope::advisory` | exit 0 — 12 tests |
| `cargo test --lib driver::dry_run` | exit 0 — 6 tests (was 4) |
| `cargo test --test spawn_seam_guard` | exit 0 — 7 tests, with the new allowlist entry |
| `cargo test --test driver_dry_run` | exit 0 — unchanged, the three-section integration pin still holds |
| `cargo clippy -- -D warnings` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | **exactly 5** pre-existing lints — `browser.rs:131/132/133`, `project_creator.rs:146`, `state_reader/mod.rs:258`. Count and locations unchanged. Measured with `rtk proxy` per D-34. |
| `cargo clippy --test envelope_advisory / driver_dry_run / spawn_seam_guard -- -D warnings` | exit 0 each (the all-targets run aborts at the pre-existing lib-test lints, so the affected targets were linted directly) |
| `grep -cE 'POST\|PUT\|PATCH\|DELETE' src/envelope/advisory.rs` | **0** — no write path, even in prose |
| `awk '/pub fn probe_protection/,/^}/' … \| grep -cE '\-X (POST\|…)\|--method (POST\|…)'` | **0** |
| `grep -c 'no-optional-locks' src/envelope/advisory.rs` | 2 (≥ 1 required) |
| `grep -c 'SECTION_ENVELOPE' src/envelope/advisory.rs` | 2 (≥ 2 required) |
| Distinct `SECTION_` constants in the ordering test | **4** — `SECTION_COMMANDS`, `SECTION_DIFFSTAT`, `SECTION_REFSPECS`, `SECTION_ENVELOPE` |
| `git diff 740e62f -- src/driver/dry_run.rs \| grep -cE '^\+.*fn the_rendered_report_carries'` | **0** — the existing ordering test was extended, not duplicated |
| Every probe fixture offline | a `file://` remote and a no-remote repository; no test performs a request (D-35) |
| `Cargo.toml` unchanged | **no crate added** — T-19-SC holds phase-wide |

## Pre-existing Failure (out of scope)

`tests/driver_reattach.rs`'s two intermittent failures did **not** reproduce in this plan's full runs (960 passing, 0 failed, across three complete runs). They remain tracked at `.planning/todos/pending/2026-08-18-driver-reattach-spawn-artifact-race.md` and were not investigated or touched, per the scope boundary — nothing in 19-06 goes near `src/driver/run.rs` or the reattach path.

## Known Stubs

None. Every symbol this plan introduces is exercised by a passing test.

`probe_protection` has no production caller yet — 19-07 owns the run-start wiring — but it is not a stub: the two offline fixtures drive it end to end through its real early-exit paths, and `DryRunReport::with_protection` is the seam that receives the state. The three network-dependent branches (rulesets, repository metadata, branch protection) are unexercised by tests **by design**: D-35 forbids the real-host fixture that would exercise them, and their component parts (`active_branch_ruleset`, `default_branch_of`, `says_absent`, `first_line`) are each unit-tested against captured response shapes.

## Threat Flags

None new. No network endpoint was added to this application, no schema changed, and **no crate was added to `Cargo.toml`**.

Against this plan's own register:

| Threat | Disposition | Where it is closed |
|---|---|---|
| T-19-38 probe failure read as safety | mitigated | `Unknown` carries its own reason and cannot borrow the protected wording; asserted over the rendered string in both the module and the preview |
| T-19-39 honesty statement softened by a later refactor | mitigated | pinned constant, one distinctive phrase asserted per required part, plus their order |
| T-19-40 protection mutation from the tool | mitigated | no write path exists by construction; asserted by a whole-file source scan and by the withheld administration scope |
| T-19-41 network probe on the tool-call critical path | mitigated | the probe is reachable only from run start, stated in two docs; 19-05's no-network guard assertion still passes; the probe additionally carries its own kill-at-deadline budget |
| T-19-42 preview and journal disagreeing about what was claimed | mitigated | one producer, `envelope_notice`; a test asserts the preview contains it verbatim for all three states |
| T-19-43 remote URL in the probe's reason string | mitigated | `ProtectionState::unknown` is the single private constructor and applies `journal::redact`; asserted by `a_reason_passes_through_the_redaction_table_at_its_one_constructor` |
| T-19-SC package-manager installs | mitigated | `Cargo.toml` untouched |

One residual is recorded because it is not in the register: **under the envelope environment the external client reads an empty `GH_CONFIG_DIR`**, so on a machine whose only GitHub authentication lives in the user's own client configuration the probe will report `unknown` naming the authentication failure rather than a real verdict. That is the correct answer — an unauthenticated client genuinely does not know — and it is stated at `probe_protection` rather than left to be discovered. It does, however, mean the probe's useful path in practice depends on an ambient `GH_TOKEN` or a future explicit read credential.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Ready for 19-07 (wiring and the run-start notice).** Three calls, in this order: `advisory::probe_protection(project_root, &env)` after `cred::build_env` (it needs the env, and `build_env` resolves the remote host anyway), then `advisory::envelope_notice(&state)` written into the run journal at run start, then — if a preview is rendered for the same run — `report.with_protection(state)`. Do **not** call `probe_protection` from the guard: the constraint is in two docs and 19-05's no-network assertion is what would catch a violation.
- **The journal event shape is 19-07's to choose,** but `ProtectionState::as_str` yields the stable identifiers (`protected`/`unprotected`/`unknown`) in `JournalEvent::Diagnostic`'s `code` style, and `reason()` gives the already-redacted detail — the two fields a `Diagnostic` needs.
- **Note for 19-08 (gate):** `SAFE-01` is shared with 19-01 and `SAFE-02` with 19-02, 19-05, 19-07 and 19-08, so the shared-ID gate holds both until the last declaring plan lands. `REQUIREMENTS.md` was **not** touched by this plan — parallel worktree mode, the orchestrator owns those writes.
- **For a future milestone:** applying protection needs a separately consented credential with an administration scope. That is a product decision about what a user agrees to when opting a project in, and it is recorded in the module doc as the deferred alternative rather than as an omission.
- **No blockers.**

## Self-Check: PASSED

- Both created files present on disk (`src/envelope/advisory.rs`, `tests/envelope_advisory.rs`); all five modified files present in the diff against the wave base `82ab462`.
- Both task commits present in `git log`: `ed46c5c`, `e3ada60`.
- Every task `<acceptance_criteria>` re-run and passing (see Verification Results); the plan-level `<verification>` re-run and passing, including the `rtk proxy` clippy-delta measurement.
- `must_haves.artifacts` confirmed: `src/envelope/advisory.rs` contains `pub enum ProtectionState`; `src/driver/dry_run.rs` contains `SECTION_ENVELOPE`.
- `must_haves.key_links` confirmed: `dry_run.rs` reaches `advisory.rs` through the `SECTION_ENVELOPE` re-export and `advisory::envelope_notice`; `advisory.rs` reaches `git_ops.rs` through `git_read_raw`, whose `--no-optional-locks` shape is named at `probe_protection`'s doc.
- `STATE.md` and `ROADMAP.md` deliberately untouched — parallel worktree mode, the orchestrator owns those writes.

---
*Phase: 19-gitsafe-git-blast-radius-envelope*
*Completed: 2026-08-18*
