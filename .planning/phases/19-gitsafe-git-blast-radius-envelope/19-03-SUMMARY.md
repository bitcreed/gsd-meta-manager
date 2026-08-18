---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 03
subsystem: infra
tags: [secret-scanning, pre-push, pre-commit, git-hooks, gitignore-blind-spot, worktree-sweep, info-exclude, safety-envelope]

# Dependency graph
requires:
  - phase: 16-run-journal
    provides: "`journal::redact`'s already-shipped and already-tested `PARTS` credential table, and `journal::RUNS_SUBDIR` — the path the exclude block and the swept-path predicate both derive from"
  - phase: 19-gitsafe-git-blast-radius-envelope
    provides: "19-01's `src/envelope/` module tree, `hooks::install`/`assert_provenance`, the `envelope` re-entry subcommand and the `file://` bare-remote fixture; 19-02's `ParkReason` taxonomy, which this plan reuses rather than extends"
  - phase: 14-driver-preview
    provides: "`state_reader::git_ops::git_read_raw` — the `--no-optional-locks` failure-as-data read shape, promoted to `pub(crate)` here rather than copied"
provides:
  - "`journal::redact::SecretClass` — one pattern table, two consumers, one place to add a rule"
  - "`journal::redact::credential_alternation()` / `credential_rule_names()` — the scanner's narrower view of that table"
  - "`envelope::scan` — the full-worktree credential scan that consults no ignore rules, its reported skip list, and its byte caps"
  - "`envelope::policy::forbidden_repo_path` — one predicate both hooks ask about a swept path"
  - "`envelope::hooks::pre_commit` and the generated `pre-commit` stub"
  - "`envelope::hooks::write_exclude_block` — the envelope's single persistent repository mutation, idempotent and marker-delimited"
  - "`cli::EnvelopeAction::{PreCommit, Scan}` and their dispatch arms"
  - "`tests/envelope_hook_refusals.rs` — the D-33 gitignored-secret fixture, its paired allow, and the sweep refusals"
affects: [19-04 PR ledger, 19-05 PreToolUse guard, 19-06 wiring, 19-07 honesty text, 19-08 gate]

# Actuals (#2632)
actuals:
  tokens: 26400
  tasks: 3
  commits: 4

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Class-tagged shared table: a fourth tuple member selects which consumers read a rule, so narrowing a consumer never forks the table. One builder produces both alternations, so the two cannot drift in how a match is identified."
    - "`is_clean()` is not `findings.is_empty()`: a scan whose root could not be read never ran, and 'found nothing' must not become 'allow'. The distinction is a dedicated field, not an inference from the skip list."
    - "Whole-file matching with a carried newline cursor, rather than a per-line pass — the only shape that sees a multi-line PEM block, which is the single most important rule in the table."
    - "Committed mutation evidence: the RED commit ships the real API stubbed, so the proof that each assertion is load-bearing lives in history rather than in a reverted working-tree edit."

key-files:
  created:
    - src/envelope/scan.rs
    - tests/envelope_hook_refusals.rs
  modified:
    - src/journal/redact.rs
    - src/envelope/policy.rs
    - src/envelope/hooks.rs
    - src/envelope/mod.rs
    - src/cli.rs
    - src/main.rs
    - src/driver/spawn.rs
    - src/state_reader/git_ops.rs
    - tests/spawn_seam_guard.rs
    - tests/envelope_tracer.rs

key-decisions:
  - "The scan matches over the WHOLE file text and derives the line number from the match offset. A line-by-line pass is simpler and silently misses the PEM rule, because `BEGIN` and `END` are never on the same line — a checked-in private key would have scanned clean."
  - "`is_clean()` is false when the scan could not read its own root. A scan that never ran finds nothing for the same reason a clean scan does, and only one of those may permit a push."
  - "An individual unreadable file is disclosed but does NOT block. Blocking every push over one permission-denied file is how a control gets switched off, which is the same argument D-12 makes about home-directory paths."
  - "`git_ops::git_read_raw` was promoted to `pub(crate)` rather than reimplemented in `hooks.rs`. A second copy would be a second place for `--no-optional-locks` to go missing, and it would have cost a `SPAWN_ALLOWLIST` entry that nobody would have questioned."
  - "The swept-path refusal parks under `ForcePushBlocked`, D-24's destructive-git family, rather than inventing an eighth reason at a call site — the mapping 19-02 already recorded for `stash` and `update-ref`."
  - "`pre_push` runs all three checks rather than short-circuiting: a human whose push was blocked should learn everything wrong with it in one round trip, not one problem per attempt."

patterns-established:
  - "One predicate, two enforcement points: `forbidden_repo_path` is asked by the commit gate and by the push backstop, so the two cannot drift and the drift cannot be invisible."
  - "Provenance selects its sanctioned counterpart by the hook's own filename against a closed list, so one check covers every hook the envelope generates and an unrecognised filename is refused before any comparison."
  - "Mutation-proof for a blind-spot test: teach the walk to honour the ignore rules, watch the fixture go red with `findings: none` and the branch on the remote, revert."

requirements-completed: [SAFE-03]

coverage:
  - id: D1
    description: "A push carrying a credential written to a path the repository's ignore rules cover is blocked before it leaves the machine, and the remote ref never appears"
    requirement: SAFE-03
    verification:
      - kind: e2e
        ref: "tests/envelope_hook_refusals.rs#a_credential_on_a_path_the_ignore_rules_cover_still_blocks_the_push"
        status: pass
      - kind: other
        ref: "mutation: teaching scan_file to skip paths the repository's .gitignore covers turns this test red with 'findings: none' and '[new branch] HEAD -> gsd-auto/gitignored/sweep' (executed, then reverted)"
        status: pass
      - kind: unit
        ref: "src/envelope/scan.rs#a_path_the_repositorys_ignore_rules_cover_is_scanned_anyway"
        status: pass
    human_judgment: false
  - id: D2
    description: "The paired allow: a clean worktree pushes inside the reserved namespace and succeeds, so the scanner is a boundary rather than a wall"
    requirement: SAFE-03
    verification:
      - kind: e2e
        ref: "tests/envelope_hook_refusals.rs#a_clean_worktree_pushes_inside_the_namespace_and_succeeds"
        status: pass
    human_judgment: false
  - id: D3
    description: "The scanner reuses `journal::redact`'s Credential-tagged rules rather than forking the table; a new rule is added in one place"
    requirement: SAFE-03
    verification:
      - kind: unit
        ref: "src/journal/redact.rs#the_credential_alternation_matches_every_credential_shape"
        status: pass
      - kind: unit
        ref: "src/journal/redact.rs#both_classes_still_reach_redact_so_the_corpus_output_is_unchanged"
        status: pass
      - kind: other
        ref: "grep: src/envelope/scan.rs holds no pattern string; it reaches the table only through credential_alternation() and credential_rule_names()"
        status: pass
    human_judgment: false
  - id: D4
    description: "Only Credential rules block a push; the path-hygiene rules that redact `/home/<user>` from logs are excluded in both the slash and the dash-encoded spelling"
    requirement: SAFE-03
    verification:
      - kind: unit
        ref: "src/journal/redact.rs#the_credential_alternation_ignores_a_home_directory_path_in_both_spellings"
        status: pass
      - kind: other
        ref: "the RED commit 5cb601e runs these same rows against the full-table alternation and they fail — the exclusion is proved load-bearing in committed history"
        status: pass
    human_judgment: false
  - id: D5
    description: "Redaction behaviour is unchanged by the class split: the corpus test and its idempotence sibling pass with no edit to either body"
    requirement: SAFE-03
    verification:
      - kind: unit
        ref: "src/journal/redact.rs#redaction_is_idempotent_over_the_whole_corpus"
        status: pass
      - kind: unit
        ref: "src/journal/redact.rs#every_corpus_case_redacts_exactly_as_specified"
        status: pass
      - kind: other
        ref: "git diff 740e62f -- src/journal/redact.rs | grep -E '^-' | grep -c 'fn .*idempot' == 0"
        status: pass
    human_judgment: false
  - id: D6
    description: "A file declined for size, binary content, an unreadable permission, a symlink or an exhausted budget is named in the scan result on BOTH outcomes"
    requirement: SAFE-03
    verification:
      - kind: unit
        ref: "src/envelope/scan.rs#the_skip_list_is_printed_on_a_clean_report_as_well_as_a_blocking_one"
        status: pass
      - kind: unit
        ref: "src/envelope/scan.rs#a_file_over_the_per_file_cap_is_reported_as_skipped_rather_than_omitted"
        status: pass
      - kind: unit
        ref: "src/envelope/scan.rs#a_file_with_a_nul_byte_in_its_first_8_kib_is_recorded_binary"
        status: pass
      - kind: unit
        ref: "src/envelope/scan.rs#a_symlink_is_recorded_and_not_followed"
        status: pass
      - kind: unit
        ref: "src/envelope/scan.rs#an_exhausted_total_budget_marks_the_rest_skipped_rather_than_ending_the_scan"
        status: pass
      - kind: e2e
        ref: "tests/envelope_hook_refusals.rs#the_skip_list_reaches_a_human_on_the_allowing_path_too"
        status: pass
    human_judgment: false
  - id: D7
    description: "A detection blocks the push with a non-zero exit and names the file, the line and the rule — and never the matched text"
    requirement: SAFE-03
    verification:
      - kind: unit
        ref: "src/envelope/scan.rs#the_rendered_report_never_contains_the_planted_secret"
        status: pass
      - kind: unit
        ref: "src/envelope/scan.rs#a_planted_private_key_blocks_and_names_its_file_and_line"
        status: pass
      - kind: e2e
        ref: "tests/envelope_hook_refusals.rs#a_credential_on_a_path_the_ignore_rules_cover_still_blocks_the_push (asserts stderr carries the file, rule=pem and secret_detected, and NOT the planted key bytes)"
        status: pass
    human_judgment: false
  - id: D8
    description: "A `gitleaks` binary on PATH runs as an additional block; its absence is recorded and is never a reason to allow"
    requirement: SAFE-03
    verification:
      - kind: e2e
        ref: "tests/envelope_hook_refusals.rs (every push prints 'gitleaks not on PATH — additive only, and its absence is never a reason to allow' and the built-in verdict still decides)"
        status: pass
    human_judgment: true
    rationale: "`gitleaks` is not installed on this machine and D-35 forbids an acceptance criterion that requires installing it, so the Blocked and Failed arms are proved only by construction and by reading. A reviewer with the binary on PATH is the only way to exercise them; the Absent arm — the one that must never fail open — is exercised on every test run."
  - id: D9
    description: "A commit whose staged paths include `.claude/worktrees/**`, the runs directory, or a path inside a nested repository is refused by the pre-commit hook"
    requirement: SAFE-03
    verification:
      - kind: e2e
        ref: "tests/envelope_hook_refusals.rs#staging_a_worktree_path_is_refused_by_the_commit_hook (with its paired allow in the same test)"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#the_swept_worktree_and_runs_paths_are_refused"
        status: pass
      - kind: unit
        ref: "src/envelope/policy.rs#an_ordinary_path_and_a_near_miss_prefix_are_allowed"
        status: pass
      - kind: unit
        ref: "src/envelope/hooks.rs#a_nested_repository_is_found_at_any_depth_but_the_root_is_not_one"
        status: pass
    human_judgment: false
  - id: D10
    description: "A push whose new commits touch those same paths is refused by the pre-push backstop, which is what catches a commit made with the verification step suppressed"
    requirement: SAFE-03
    verification:
      - kind: e2e
        ref: "tests/envelope_hook_refusals.rs#a_commit_made_with_verification_suppressed_is_refused_by_the_push_hook"
        status: pass
      - kind: unit
        ref: "src/envelope/hooks.rs#the_pushed_ranges_are_the_sha_pair_of_every_well_formed_line"
        status: pass
    human_judgment: false
  - id: D11
    description: "The `.git/info/exclude` block is idempotent, marker-delimited, rewritten in place, and live — git honours it"
    requirement: SAFE-03
    verification:
      - kind: e2e
        ref: "tests/envelope_hook_refusals.rs#the_exclude_block_is_idempotent_and_survives_content_already_present (byte comparison across two runs, plus `git check-ignore` proving the rule is live)"
        status: pass
      - kind: unit
        ref: "src/envelope/hooks.rs#the_exclude_block_is_idempotent_and_leaves_unrelated_lines_alone"
        status: pass
      - kind: unit
        ref: "src/envelope/hooks.rs#the_exclude_block_writes_into_a_linked_worktrees_real_git_directory"
        status: pass
    human_judgment: false
  - id: D12
    description: "The exclude write is documented as an explicit exception to the touch-nothing posture, with the reason the tracked ignore file was not used"
    verification: []
    human_judgment: true
    rationale: "Whether the doc reads as an honest exception rather than as a rationalisation is a judgement a test cannot make. `write_exclude_block`'s doc states the mutation, why an ignore rule is inert for the human, and why an untracked per-repository location prevents the failure rather than punishing it — a reader should confirm it is the first of those and not the third."

# Metrics
duration: 30 min
completed: 2026-08-18
status: complete
---

# Phase 19 Plan 03: The Pre-Push Secret Scan and the Worktree-Sweep Refusal Summary

**A credential written to a path the repository's ignore rules cover blocks a driven push before it leaves the machine — proved by a fixture, and proved load-bearing by a mutation that turns it red — while `.claude/worktrees/**`, the runs directory and any nested repository are refused at commit and again at push, with an idempotent `.git/info/exclude` block as the inert third layer.**

## Performance

- **Duration:** ~30 min
- **Started:** 2026-08-18T11:47:00-06:00 (base commit `1f0ae75`; first commit 11:53:15)
- **Completed:** 2026-08-18T12:17:00-06:00
- **Tasks:** 3 (Task 1 executed as RED → GREEN)
- **Files modified:** 12 (2 created, 10 modified)

## Accomplishments

- **The named blind spot is closed, and the proof is a mutation rather than an assertion.** `tests/envelope_hook_refusals.rs` plants a PEM private key at `secrets/prod.pem`, asserts with `git check-ignore` that the fixture really is testing an ignored path, and requires the push to be refused. Teaching the walk to honour `.gitignore` turns that test red with `findings: none` and `[new branch] HEAD -> gsd-auto/gitignored/sweep` on the remote — executed, observed, then reverted. Without the mutation the test would pass against a scanner with the blind spot wide open, because the secret is also findable by a dozen other means.
- **The scanner reuses the pattern table instead of forking it.** `PARTS` gained a fourth member, `SecretClass`, **in its existing order** — the WR-15 note records that a naive reorder once regressed seven of eight fixtures — and there are now two consumers of one table. `redact` reads every rule; the scanner reads only the twelve `Credential` ones. A new rule is still added in exactly one place.
- **The class split changed no redaction behaviour, and the proof is that nothing was edited to accommodate it.** The corpus test and the idempotence test over it both pass with their bodies untouched; `git diff 740e62f -- src/journal/redact.rs | grep '^-' | grep -c 'fn .*idempot'` returns 0.
- **The scan matches over whole file text, not line by line.** The obvious per-line implementation never sees a `BEGIN`/`END` pair, so a checked-in PEM private key — the most important rule in the table — would have scanned clean. The line number is derived from the match offset with a carried cursor.
- **`is_clean()` is deliberately not `findings.is_empty()`.** A scan whose root could not be read finds nothing for exactly the same reason a clean scan does, and a report that could not distinguish them would be the fail-open this control exists to refuse.
- **One predicate, two enforcement points.** `policy::forbidden_repo_path` is asked by `pre-commit` about staged paths and by `pre-push` about the paths the introduced commits touch. The push check is the backstop for a commit made with the verification step suppressed, which the commit hook by construction never sees — and the fixture makes exactly that commit and requires the push to be refused.
- **The exclude block is inert, idempotent and live.** Written twice it is byte-identical; unrelated content in the file survives; there is exactly one marker pair; and `git check-ignore` confirms git actually honours the rule rather than the test merely confirming that bytes were written.
- **The worktree question is answered rather than deferred.** `src/driver/spawn.rs`'s "Phase 19 owns the decision" comment is gone, replaced by D-21's four recorded reasons and a pointer at the mechanism that closed the hazard instead of at a future phase.

## Task Commits

1. **Task 1 (RED): the Credential-only alternation, against the full-table stub** — `5cb601e` (test)
2. **Task 1 (GREEN): tag `PARTS` by `SecretClass` — one table, two consumers** — `4a71c12` (feat)
3. **Task 2: the full-worktree credential scan and its reported skip list** — `9977e5b` (feat)
4. **Task 3: the hooks refuse — a secret anywhere, a swept worktree, an inert exclude block** — `f0e21cf` (feat)

_No REFACTOR commit for Task 1: the GREEN implementation needed no cleanup pass, and an empty `refactor` commit is ceremony rather than a change._

## Files Created/Modified

- `src/envelope/scan.rs` (new, 750 lines) — `Finding`, `SkipReason`, `ExternalScanner`, `ScanReport` (+ `is_clean`, `render`), `ScanLimits`, `scan_worktree`, `scan_with_external`, `run_gitleaks`, `collect_findings`; 11 unit tests
- `tests/envelope_hook_refusals.rs` (new, 408 lines) — the D-33 gitignored-secret fixture, its paired allow, the skip-list disclosure on the allowing path, the commit refusal with its own paired allow, the suppressed-verification backstop, the exclude-block idempotence, and the two-stub shape check
- `src/journal/redact.rs` — `SecretClass`, the class-tagged `PARTS`, `build_alternation`, `credential_alternation`, `credential_rule_names`, module doc point 4; 3 new tests
- `src/envelope/policy.rs` — `WORKTREES_DIR`, `forbidden_repo_prefixes`, `forbidden_repo_path`; 4 new tests
- `src/envelope/hooks.rs` — `PRE_COMMIT_HOOK`, `SANCTIONED_HOOKS`, the exclude markers, `write_stub`, the filename-selected provenance check, `pre_commit`, the extended `pre_push`, `refuse_paths`, `has_nested_git`, `refuse_swept_paths`, `read_ref_lines`, `pushed_ranges`, `write_exclude_block`, `git_dir`; 6 new tests
- `src/envelope/mod.rs` — `pub mod scan;`
- `src/cli.rs` / `src/main.rs` — `EnvelopeAction::{PreCommit, Scan}` and their arms, with the repository root resolved from the hook's working directory
- `src/driver/spawn.rs` — D-21's answer replaces the deferral
- `src/state_reader/git_ops.rs` — `git_read_raw` promoted to `pub(crate)` with the reason recorded at the line
- `tests/spawn_seam_guard.rs` — `src/envelope/scan.rs` allowlisted, in the same commit, for the `gitleaks` shell-out
- `tests/envelope_tracer.rs` — `run_stub` gains `current_dir(work)`, matching how git runs a hook

## Decisions Made

- **The RED commit ships a compiling stub, following 19-02.** `credential_alternation` and `credential_rule_names` exist at `5cb601e` and return the whole table, so the three failing assertions fail on *behaviour*, the history stays bisectable, and the proof that the path-hygiene exclusion is load-bearing is permanent rather than a reverted working-tree edit.
- **Whole-file matching over line-by-line.** Stated above; it is the difference between catching a checked-in private key and not.
- **One unreadable file discloses but does not block; an unreadable root blocks.** The asymmetry is deliberate and is the same argument D-12 makes: a scanner that blocked every push over one permission-denied file gets switched off, whereas a scan that never ran must never read as an allow.
- **`git_read_raw` promoted rather than copied.** A second three-line git read in `hooks.rs` would have been a second place for `--no-optional-locks` to go missing, and it would have needed a `SPAWN_ALLOWLIST` entry that nobody would have questioned. The envelope now adds exactly one spawn site in the whole phase, for `gitleaks`.
- **Provenance selects its counterpart by filename against a closed list.** Adding `pre-commit` could have meant a second provenance function; instead the existing one derives which sanctioned file to compare against from `invoked_from`'s own name, and a name the envelope never generates is refused before any comparison is attempted.
- **`gitleaks` output is discarded, never folded into the report.** A leak scanner's output names the secrets it found. Reproducing it would be the SAFE-04 bug arriving through the one path that looks most like diligence.
- **The tracer gate was satisfied mechanically, as in 19-01.** `19-CONTEXT.md` records the governing user direction for this phase — no questions at any gate, best well-reasoned choice, proceed and record it — and this plan has no `type="tracer"` task in any case; the equivalent obligation was met by running each task's `<verify>` and the plan-level gate before the next task began.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The plan's credential-rule count is thirteen; the table holds twelve**

- **Found during:** Task 1
- **Issue:** The plan's action text says "tag the thirteen credential-shaped rules `Credential` and the four path-hygiene rules `PathHygiene`", which totals seventeen. `PARTS` holds sixteen rules, and the `<behavior>` block's own enumeration lists twelve credential shapes: `pem`, `authz`, `env`, `bearer`, `skant`, `sk`, `ghpat`, `gh`, `aws`, `slack`, `jwt`, `uinfo`. The four path-hygiene rules are `dtmp`, `dhome`, `stmp`, `shome`.
- **Fix:** Tagged twelve `Credential` and four `PathHygiene`, matching the table and the `<behavior>` enumeration rather than the prose count. `the_credential_alternation_matches_every_credential_shape` asserts the shape table and `credential_rule_names()` are the **same length**, so the count is now pinned by a test rather than by prose in either place.
- **Files modified:** `src/journal/redact.rs`
- **Verification:** `credential_rule_names().len() == 12`, asserted; every one of the twelve has its own named shape row.
- **Committed in:** `4a71c12`

**2. [Rule 3 - Blocking] `pre_commit` and `pre_push` take the repository root and the hook path explicitly**

- **Found during:** Task 3
- **Issue:** The plan specifies `pub fn pre_commit(alias: &str) -> anyhow::Result<i32>` and, in the same sentence, that it "calls `assert_provenance` first". Those are incompatible: `assert_provenance` needs the path the hook was invoked from, which is the whole point of D-10's `$0`. The staged-list read and the worktree scan additionally need a repository root.
- **Fix:** `pre_commit(alias, invoked_from, repo_root)` and `pre_push(alias, stdin, invoked_from, repo_root)`. `main.rs` resolves the root from `std::env::current_dir()`, which is the top of the worktree because that is where git runs a hook from — so the scan's root and the path checks' root are the same root with no flag to get wrong.
- **Files modified:** `src/envelope/hooks.rs`, `src/main.rs`, `src/cli.rs`
- **Verification:** `tests/envelope_hook_refusals.rs` runs both hooks the way git runs them; the relocated-copy test in `tests/envelope_tracer.rs` still proves provenance is checked before anything else.
- **Committed in:** `f0e21cf`

**3. [Rule 2 - Missing Critical] `ScanReport` gained `root_unreadable`, and `is_clean()` consults it**

- **Found during:** Task 2
- **Issue:** The plan's `ScanReport` shape plus `is_clean()` as "no findings" fails open on the case the plan's own prohibition names: a scan whose root cannot be read produces zero findings and would have reported clean, permitting the push. The skip list would have disclosed it, but nothing would have *acted* on it.
- **Fix:** An explicit `root_unreadable` field, set when `read_dir` fails on the root itself, consulted by `is_clean()` and stated loudly in `render()`. Deliberately distinct from a skipped file, which discloses without blocking.
- **Files modified:** `src/envelope/scan.rs`
- **Verification:** `a_scan_that_could_not_read_its_root_is_never_reported_as_clean`
- **Committed in:** `9977e5b`

**4. [Rule 2 - Missing Critical] `write_exclude_block` resolves a `.git` pointer file**

- **Found during:** Task 3
- **Issue:** A driven run inside a linked worktree has a `.git` **file**, not a directory. A writer that assumed a directory would have created `.git/info/` beside the pointer file, where git never looks — an ignore rule that is silently inert, which is worse than an absent one because it looks present.
- **Fix:** `git_dir()` reads the `gitdir:` pointer and resolves it, absolute or relative.
- **Files modified:** `src/envelope/hooks.rs`
- **Verification:** `the_exclude_block_writes_into_a_linked_worktrees_real_git_directory`, which also asserts nothing is created beside the pointer file.
- **Committed in:** `f0e21cf`

**5. [Rule 3 - Blocking] `tests/envelope_tracer.rs::run_stub` gained `current_dir(work)`**

- **Found during:** Task 3
- **Issue:** `run_stub` invoked the hook with the *test process's* working directory, which is this project's own checkout. Once `pre_push` scans its working directory, the tracer's control assertion — "the sanctioned hook must ALLOW this ref" — would have been scanning `gsd-meta-manager` itself, including `target/`, and would have gone green or red for reasons having nothing to do with the fixture.
- **Fix:** One line, plus a comment saying which failure it prevents. Not in this plan's `files_modified`, but leaving it would have made an existing test's outcome depend on the contents of the developer's checkout.
- **Files modified:** `tests/envelope_tracer.rs`
- **Verification:** `cargo test --test envelope_tracer` — 6 passing, unchanged.
- **Committed in:** `f0e21cf`

**6. [Rule 3 - Blocking] `credential_alternation` / `credential_rule_names` are `pub`, not `pub(crate)`**

- **Found during:** Task 1 (RED)
- **Issue:** The plan specifies `pub(crate)`. At the RED commit and at the end of Task 1 there is no in-crate consumer — `envelope::scan` arrives in Task 2 — so a `pub(crate)` item would trip `dead_code` and fail `cargo clippy -- -D warnings` at both commits.
- **Fix:** `pub`, consistent with `redact` and `redact_value` in the same already-public module. The visibility carries no security weight here; the constraint that matters is that the scanner reaches the table through these functions rather than holding its own copy of the patterns, and that is unchanged.
- **Files modified:** `src/journal/redact.rs`
- **Verification:** clippy clean at every commit in this plan.
- **Committed in:** `5cb601e`, `4a71c12`

---

**Total deviations:** 6 (2 missing-critical/security, 3 blocking, 1 bug)
**Impact on plan:** No scope creep. Deviations 3 and 4 each close a fail-open the plan's own prohibitions forbid but its written shape permitted. Deviations 2, 5 and 6 are corrections to signatures and visibility that the plan's text made impossible to satisfy as written. Deviation 1 is an arithmetic correction, now pinned by a test so neither count can drift again.

## Issues Encountered

None. Every verification passed on first execution except where a mutation was deliberately introduced.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo test` | exit 0 — **856 passing** (825 at 19-02, 839 after Task 2; +31 this plan, no regression, nothing ignored) |
| `cargo clippy -- -D warnings` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | **exactly 5** pre-existing lints — `browser.rs` ×3, `project_creator.rs` ×1, `state_reader/mod.rs` ×1. Count unchanged. Measured with `rtk proxy` per D-34. |
| `cargo test --lib journal::redact` | exit 0 — 10 tests |
| `cargo test --lib envelope::scan` | exit 0 — 11 tests |
| `cargo test --test envelope_hook_refusals` | exit 0 — 7 tests |
| `cargo test --test envelope_tracer` | exit 0 — 6 tests, unchanged |
| `cargo test --test spawn_seam_guard` | exit 0 — 7 tests |
| `grep -c 'SecretClass' src/journal/redact.rs` | 22 (floor: 3) |
| `git diff 740e62f -- src/journal/redact.rs \| grep '^-' \| grep -c 'fn .*idempot'` | 0 — the idempotence test body was not edited |
| `grep -nE 'check-ignore\|exclude-standard\|ls-files' src/envelope/scan.rs` | no match — the walk decides what to read without git's exclude machinery |
| `grep -nE '^\s*(walkdir\|ignore\|globset)' Cargo.toml` | no match — no crate was added (T-19-SC holds) |
| `grep -c 'gsd-meta-manager envelope' src/envelope/hooks.rs` | 7 (floor: 2; both marker delimiters present) |
| `grep -n 'RUNS_SUBDIR' src/envelope/policy.rs` | 4 matches — the runs path is derived, never re-spelled |
| `git diff 740e62f -- src/driver/spawn.rs \| grep -c 'Phase 19 owns'` | 1, on the removed side; `grep -c 'Phase 19 owns' src/driver/spawn.rs` is 0 |
| `#[test]` count in `tests/envelope_hook_refusals.rs` | 7 (floor: 5) |

**Mutation check (executed, then reverted before commit):**

| Mutation | Expected | Observed |
|---|---|---|
| `scan_file` skips paths the repository's `.gitignore` covers | the D-33 fixture goes red | `a_credential_on_a_path_the_ignore_rules_cover_still_blocks_the_push` failed: report read `findings: none`, and git reported `* [new branch] HEAD -> gsd-auto/gitignored/sweep` — the secret reached the remote |

**Committed RED evidence (not reverted):** at `5cb601e` the credential alternation is the full table; the two home-directory negative rows and the rule-count row fail there and pass at `4a71c12`. The twelve positive shape rows pass at both, which is correct — they must, or they would be proving nothing about the split.

## Known Stubs

None. Every symbol this plan introduces is exercised by a passing test. `write_exclude_block`, `pre_commit` and `EnvelopeAction::Scan` have no *production* caller yet — plan 19-06 owns the driver wiring, and until then the hooks are installed only by the fixtures — but none is a stub: each is fully implemented and each is driven end to end by a committed test against a real repository.

## Threat Flags

None. No new network endpoint, auth path or schema was introduced.

- **T-19-15** (report discloses the secret) — mitigated by construction and asserted behaviourally, twice: `Finding` has no field for the matched text, `the_rendered_report_never_contains_the_planted_secret` checks the rendered bytes, and the fixture checks the real hook's stderr.
- **T-19-16** (credentials on ignored paths) — mitigated and mutation-proved.
- **T-19-17** (nested repository / runs directory staged) — mitigated at commit and again at push.
- **T-19-18** (absent external scanner read as clean) — the built-in rules always run first; `Absent` is recorded and printed on every push.
- **T-19-19** (unbounded walk) — explicit per-file and total caps; exhaustion is a named skip reason per file.
- **T-19-20** (exclude block accumulation) — byte-comparison idempotence test.
- **T-19-21** (symlink escape) — not followed, recorded.
- **T-19-22** (a scan that declined files reporting clean) — one structure, printed on both outcomes.
- **T-19-SC** — **no crate was added to `Cargo.toml`.** The walk is hand-rolled against `std`.

One surface worth naming for the phase's own record rather than as a flag: `src/envelope/scan.rs` is the phase's only new process-spawn site, for `gitleaks`, and it is allowlisted in `tests/spawn_seam_guard.rs` in the same commit that introduced it.

## User Setup Required

None — no external service configuration required. `gitleaks` is optional by design; its absence is reported on every push and never permits one.

## Next Phase Readiness

- **Ready for 19-06 (wiring).** `hooks::install(alias)` now generates **both** stubs, so the run-start call is unchanged. `hooks::write_exclude_block(project_root)` is the second run-start call, and it is idempotent, so calling it on every run is correct rather than merely tolerable.
- **Ready for 19-05 (PreToolUse guard).** `policy::forbidden_repo_path` is available to the guard if a `Write`/`Edit` denial wants the same path rules, and `ParkReason::SecretDetected` already has its emitter here.
- **For 19-07 (honesty text):** `src/envelope/scan.rs`'s module doc carries the substance of SAFE-03's limits — the walk's caps, what a report may contain, and why an absent external scanner is not a gap that permits. The pinned constant should reference that reasoning rather than restate it differently.
- **Note for 19-08 (gate):** `SAFE-03` is declared by this plan alone among the phase's plans, so `REQUIREMENTS.md` is safe to flip; the orchestrator owns that write in worktree mode.
- **No blockers.**

## Self-Check: PASSED

- Both created files present on disk: `src/envelope/scan.rs`, `tests/envelope_hook_refusals.rs` (408 lines, floor 150).
- All four task commits present in `git log`: `5cb601e`, `4a71c12`, `9977e5b`, `f0e21cf`.
- Every task `<acceptance_criteria>` re-run and passing (see Verification Results); the plan-level `<verification>` re-run and passing.
- Every `must_haves.key_links` pattern present: `SecretClass::Credential` in `redact.rs`, `scan_worktree`/`scan_with_external` reached from `hooks.rs`, `forbidden_repo_path` called from `hooks.rs`.
- `STATE.md` and `ROADMAP.md` deliberately untouched — parallel worktree mode, orchestrator owns those writes.

---
*Phase: 19-gitsafe-git-blast-radius-envelope*
*Completed: 2026-08-18*
