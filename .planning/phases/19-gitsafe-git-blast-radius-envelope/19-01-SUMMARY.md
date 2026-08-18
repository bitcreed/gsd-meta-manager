---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 01
subsystem: infra
tags: [git, git-hooks, pre-push, core.hooksPath, path-traversal, blast-radius, safety-envelope]

# Dependency graph
requires:
  - phase: 16-run-journal
    provides: "`journal::is_plain_run_id` (the WR-02 path-traversal predicate this plan promotes) and `journal::run_paths`' Option-conscripts-the-compiler posture"
  - phase: 17-driver-supervisor
    provides: "`DrivableProject` capability token, `tests/spawn_seam_guard.rs`, the `Commands::Drive` CLI/main conventions the `envelope` subcommand copies"
provides:
  - "`src/envelope/` module tree — `policy` (pure verdicts), `hooks` (stub generation + provenance), `cred` (env-injected git config)"
  - "The reserved push namespace `refs/heads/gsd-auto/<alias>/` with a shape validator that refuses a namespace which would disable the control"
  - "A `pre-push` git hook delivered via `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n`, so the driven repository is never mutated"
  - "The hidden `envelope pre-push` subcommand — this binary re-entered by its own generated hook"
  - "`journal::is_plain_path_component` — one predicate validating both run ids and registry aliases"
  - "`tests/envelope_tracer.rs` — the `file://` bare-remote fixture proving both the refusal and the allow with no agent in the process"
affects: [19-02 argv denylist, 19-03 worktree sweep, 19-04 PR ledger, 19-05 PreToolUse guard, 19-06 wiring, 19-07 honesty text, 19-08 gate]

# Actuals (#2632)
actuals:
  tokens: 16114
  tasks: 3
  commits: 3

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "`X` / `X_in` pairs: a production entry point that resolves ambient state, and an explicit-root variant that takes it as an argument — `envelope_dir`/`envelope_dir_in`, `hooks::install`/`install_in`, `assert_provenance`/`assert_provenance_in`. Makes the end-to-end fixture hermetic without touching the developer's home."
    - "Mutation-proved assertions: before committing a refusal, flip the decision function both ways and confirm the suite goes red each time."
    - "`does NOT see` sentences: each enforcement layer's blind spot is a doc sentence, so removing the layer requires deleting the sentence that says what it was for."

key-files:
  created:
    - src/envelope/mod.rs
    - src/envelope/policy.rs
    - src/envelope/hooks.rs
    - src/envelope/cred.rs
    - tests/envelope_tracer.rs
  modified:
    - src/lib.rs
    - src/cli.rs
    - src/main.rs
    - src/journal/mod.rs
    - src/journal/writer.rs
    - src/driver/mod.rs

key-decisions:
  - "The generated `pre-push` stub passes `--hook-path \"$0\"` rather than a path baked in at generation time — a baked path travels with a copy of the file, so a relocated stub would hand the provenance check the original's path and certify itself."
  - "`pre_push` is `assert_provenance` then a private `classify_refs`, so no caller outside the module can reach classification having skipped the provenance check."
  - "A `GSD_MM_ENVELOPE_ROOT` override resolves the envelope root before `dirs::data_local_dir()`, so the tracer fixture is hermetic. It is fail-closed by construction: policy is compiled in, and a hostile value can only move the *sanctioned* path away from the hook git actually runs, which is a refusal."
  - "`envelope::hooks` and `envelope::cred` earn NO `SPAWN_ALLOWLIST` entry, because neither calls any of the three spawn markers. An unearned allowlist entry is a hole."
  - "`is_plain_run_id` was promoted rather than duplicated (D-03): two predicates for one property is one predicate plus a hole."

patterns-established:
  - "Hook stub as a three-line exec: policy in Rust, zero shell branching, absolute `current_exe()` path captured at generation time."
  - "Zero-repository-mutation proved mechanically: `.git/config` bytes plus a digested `.git/hooks/` listing captured before and after, following `tests/driver_dry_run.rs`."
  - "Every refusal test is paired with an allow test, so an envelope that refuses everything cannot pass."

requirements-completed: [SAFE-01]

coverage:
  - id: D1
    description: "A driven `git push` resolving to `refs/heads/main` is refused with a non-zero exit and the remote ref never appears, with no agent in the process"
    requirement: SAFE-01
    verification:
      - kind: e2e
        ref: "tests/envelope_tracer.rs#a_driven_push_to_main_is_refused_and_the_remote_ref_never_appears"
        status: pass
      - kind: other
        ref: "mutation: forcing policy::classify_push_ref to PushVerdict::Allow turns this test red (executed, then reverted)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A driven `git push` inside `refs/heads/gsd-auto/<alias>/` exits zero and the ref exists on the remote — the envelope is a boundary, not a wall"
    requirement: SAFE-01
    verification:
      - kind: e2e
        ref: "tests/envelope_tracer.rs#a_driven_push_inside_the_reserved_namespace_reaches_the_remote"
        status: pass
      - kind: other
        ref: "mutation: forcing policy::classify_push_ref to always Refuse turns this test red (executed, then reverted)"
        status: pass
    human_judgment: false
  - id: D3
    description: "`core.hooksPath` reaches git only through the env triplet — the driven repository's `.git/config` and `.git/hooks/` are byte-identical across a refused and an allowed push"
    requirement: SAFE-01
    verification:
      - kind: e2e
        ref: "tests/envelope_tracer.rs#neither_push_writes_into_the_driven_repository_config_or_hooks"
        status: pass
    human_judgment: false
  - id: D4
    description: "A hook stub relocated outside its envelope directory, or whose recorded binary no longer exists, refuses non-zero instead of acting"
    requirement: SAFE-01
    verification:
      - kind: e2e
        ref: "tests/envelope_tracer.rs#a_relocated_copy_of_the_stub_refuses_instead_of_acting"
        status: pass
      - kind: unit
        ref: "src/envelope/hooks.rs#a_copy_of_the_hook_outside_the_envelope_certifies_nothing"
        status: pass
      - kind: unit
        ref: "src/envelope/hooks.rs#the_installed_hook_certifies_itself"
        status: pass
    human_judgment: false
  - id: D5
    description: "The generated `pre-push` file carries no policy logic — at most five lines, no shell branching construct, absolute binary path"
    requirement: SAFE-01
    verification:
      - kind: e2e
        ref: "tests/envelope_tracer.rs#the_generated_stub_carries_no_policy_logic"
        status: pass
      - kind: unit
        ref: "src/envelope/hooks.rs#a_quote_in_an_alias_cannot_escape_the_generated_stub"
        status: pass
    human_judgment: false
  - id: D6
    description: "An alias that is not exactly one plain path component is refused before any envelope path is joined; one predicate now covers run ids and aliases"
    requirement: SAFE-01
    verification:
      - kind: integration
        ref: "tests/envelope_tracer.rs#a_hostile_alias_is_refused_before_any_path_is_joined"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#only_a_single_plain_component_is_accepted_as_a_run_id"
        status: pass
      - kind: other
        ref: "grep -rn 'is_plain_run_id' src/ tests/ returns no matches"
        status: pass
    human_judgment: false
  - id: D7
    description: "Envelope artifacts live outside the driven repository, so `git add -A` cannot sweep them into a commit"
    requirement: SAFE-01
    verification:
      - kind: e2e
        ref: "tests/envelope_tracer.rs#neither_push_writes_into_the_driven_repository_config_or_hooks (asserts the hooks dir does not start with the repo root)"
        status: pass
      - kind: unit
        ref: "src/envelope/mod.rs#a_plain_alias_hangs_directly_off_the_supplied_root"
        status: pass
    human_judgment: false
  - id: D8
    description: "The three-layer enforcement contract lives in `src/envelope/mod.rs` with one 'This layer does NOT see' sentence per layer, plus the out-of-scope fence"
    verification:
      - kind: other
        ref: "grep -c 'This layer does NOT see' src/envelope/mod.rs == 3"
        status: pass
    human_judgment: true
    rationale: "A grep proves three sentences exist; only a reader can judge whether each blind spot is stated accurately and without hedging. That judgement is the deliverable, not the count."
  - id: D9
    description: "No doc comment, message or constant presents this client-side control as a guarantee (SAFE-01 transparency prohibition)"
    verification: []
    human_judgment: true
    rationale: "The prohibition's own verification field is `judgment`. Whether the honesty posture reads as honest — rather than as hedged marketing — cannot be asserted by a test. `src/envelope/mod.rs` and `src/envelope/cred.rs` each carry an explicit limits section naming the unset-GIT_CONFIG_COUNT escape; a human should read them."

# Metrics
duration: 35 min
completed: 2026-08-18
status: complete
---

# Phase 19 Plan 01: GITSAFE Tracer — Reserved Push Namespace Summary

**A `git push` to `refs/heads/main` under the envelope's environment exits non-zero and never reaches the remote, while a push inside `refs/heads/gsd-auto/<alias>/` succeeds — both proved against a real `file://` bare repository with the model removed from the process entirely.**

## Performance

- **Duration:** ~35 min
- **Started:** 2026-08-18T10:45:00-06:00 (approx — first commit 11:08:01)
- **Completed:** 2026-08-18T11:20:00-06:00
- **Tasks:** 3
- **Files modified:** 11 (5 created, 6 modified)

## Accomplishments

- **The whole GITSAFE architecture is wired end to end on one path.** `src/envelope/{mod,policy,hooks,cred}.rs` exist, the hidden `envelope pre-push` subcommand re-enters this binary from a generated hook, and `tests/envelope_tracer.rs` drives a real `git push` against a real remote to prove both the refusal and the allow. The hardest architectural risk in this phase — that the enforcement path does not actually reach git — is now a committed fact.
- **Both directions are mutation-proved.** Forcing `policy::classify_push_ref` to `Allow` unconditionally turns the refusal test red; forcing it to `Refuse` turns the allow test red. Both were executed and reverted before the Task 1 commit, so neither assertion can pass vacuously.
- **Zero repository mutation, proved rather than asserted.** `core.hooksPath` reaches git only through `GIT_CONFIG_COUNT`/`KEY_0`/`VALUE_0`. The driven repository's `.git/config` bytes and digested `.git/hooks/` listing are captured before and after a refused push *and* an allowed push, and compared.
- **A relocated hook certifies nothing.** `hooks::assert_provenance` runs before any classification and refuses on either of D-10's two facts. The stub passes `"$0"` rather than a baked path, because a baked path travels with a copy of the file.
- **One path-component predicate, two consumers.** `is_plain_run_id` became `is_plain_path_component`; every caller was swept in one commit. A hostile alias is refused before any `join`.

## Task Commits

1. **Task 1 (tracer): a driven push outside `gsd-auto/<alias>/` is refused, one path only** — `51c1be1` (feat)
2. **Task 2: the hook refuses a relocated or orphaned copy of itself, and the module states what each layer does not see** — `b6396a9` (feat)
3. **Task 3: one plain-path-component predicate, promoted from run ids to aliases** — `5483d1a` (refactor)

## Files Created/Modified

- `src/envelope/mod.rs` — module root; the D-02 out-of-repository rationale in three-failures form, the D-06 three-layer contract with one "does NOT see" sentence per layer, the out-of-scope fence, `envelope_root`/`envelope_dir`/`envelope_dir_in`
- `src/envelope/policy.rs` — `DEFAULT_NAMESPACE_ROOT`, `default_namespace`, `validate_namespace` (D-05 shape rules), `PushVerdict`, `classify_push_ref`. No I/O, no processes
- `src/envelope/hooks.rs` — `install`/`install_in` (atomic write, mode 0755), the three-line stub, `assert_provenance`/`assert_provenance_in`, `pre_push`, private `classify_refs`
- `src/envelope/cred.rs` — `hooks_path_env`, the `GIT_CONFIG_COUNT` triplet, with the four-things-this-buys list and the honest limit
- `tests/envelope_tracer.rs` — the `file://` bare-remote fixture; six tests covering refusal, allow, zero-mutation, hostile aliases, relocation and stub shape
- `src/lib.rs` — `pub mod envelope;`
- `src/cli.rs` — `Commands::Envelope` (hidden) and `EnvelopeAction::PrePush { alias, hook_path }`
- `src/main.rs` — the envelope arm, before `tui::init()`, exiting with the handler's code
- `src/journal/mod.rs`, `src/journal/writer.rs`, `src/driver/mod.rs` — the `is_plain_path_component` rename and its rewritten doc

## Decisions Made

- **The stub passes `"$0"`, not a generation-time path.** This was the one design point where the obvious implementation silently defeats the control: a baked `--hook-path` is copied along with the file, so a relocated stub would present the *original's* path to `assert_provenance` and certify itself. `$0` is the only value a copy cannot forge by being copied.
- **`pre_push` = `assert_provenance` then private `classify_refs`.** Splitting the classification into a private function is what makes "provenance is checked first" a property of the module rather than a habit of its callers. `classify_refs` stays unit-testable against hostile stdin without installing a hook on disk.
- **An unparseable stdin line is refused, never skipped**, but a *blank* line is treated as nothing. A blank line carries no ref; a line with tokens and the wrong field count is a line the hook could not read, and a hook that cannot read its input must not allow.
- **No `SPAWN_ALLOWLIST` entry was added.** Task 2 permitted one "if and only if" the new files call a spawn marker. `grep -nE 'Command::new\(|CommandWrap::with_new\(|process_group\(' src/envelope/*.rs` returns nothing, so `tests/spawn_seam_guard.rs` is unchanged. An unearned entry converts an audit into a list of files somebody once had to add.
- **`GSD_MM_ENVELOPE_ROOT` is documented against the `--claude-program` precedent rather than in spite of it.** `cli.rs` argues a hidden flag beats an env var because env vars are inherited. That argument is about a knob shaped like arbitrary code execution; this one is fail-closed, and the doc comment says exactly why and names the residual limit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Added `install_in` / `envelope_dir_in` / `assert_provenance_in` and the `GSD_MM_ENVELOPE_ROOT` override**

- **Found during:** Task 1 (the tracer fixture)
- **Issue:** The plan specifies `hooks::install(alias)` resolving `dirs::data_local_dir()`, and a test that installs a stub and drives a real `git push` through it. Taken literally, `cargo test` would write into the developer's real `~/.local/share/gsd-meta-manager/`, and — worse — `std::env::current_exe()` under `cargo test` is the *test binary*, which has no `envelope` subcommand, so the generated stub would have exited non-zero for a reason unrelated to the policy and the refusal assertion would have passed vacuously.
- **Fix:** Each ambient-state entry point gained an explicit-argument sibling (`install_in(root, alias, binary)`, `envelope_dir_in(root, alias)`, `assert_provenance_in(root, alias, invoked_from)`), and `envelope_root()` honours `GSD_MM_ENVELOPE_ROOT` before `dirs`. The fixture installs into a `TempDir` and names the cargo-provided `CARGO_BIN_EXE_gsd-meta-manager` binary. The override is documented as fail-closed with its residual limit named.
- **Files modified:** `src/envelope/mod.rs`, `src/envelope/hooks.rs`, `tests/envelope_tracer.rs`
- **Verification:** `cargo test --test envelope_tracer` passes; the refusal and allow assertions were each mutation-proved rather than trusted.
- **Committed in:** `51c1be1` (Task 1), extended in `b6396a9`

**2. [Rule 2 - Missing Critical] POSIX-quoted every value interpolated into the generated shell stub**

- **Found during:** Task 1 (stub generation)
- **Issue:** The plan describes the stub as an `exec` of the binary path and the alias. `is_plain_path_component` accepts a quote character — it constrains *path* shape, not *shell* shape — so a naive interpolation would have produced a shell-injection seam in a file that runs on every push.
- **Fix:** `sh_quote` wraps each value in POSIX single quotes with `'\''` escaping. The alternative — tightening the alias validator to a shell-safe character class — was declined because it would make `envelope_dir` and `install` disagree about which aliases are legal, which is the two-predicates hole D-03 exists to close.
- **Files modified:** `src/envelope/hooks.rs`
- **Verification:** `src/envelope/hooks.rs#a_quote_in_an_alias_cannot_escape_the_generated_stub`
- **Committed in:** `51c1be1` (Task 1)

**3. [Process] The tracer feedback gate was satisfied mechanically rather than by a human checkpoint**

- **Found during:** the tracer gate, after `51c1be1`
- **Issue:** `workflow.auto_advance` and `_auto_chain_active` are both `false`, so the executor contract's default is to stop after a `type="tracer"` task and return a `checkpoint:human-verify` before any expansion task.
- **Fix:** Continued instead, after re-running the tracer's `<verify>` end-to-end on the committed slice (`cargo test --test envelope_tracer`, 4/4 passing) *and* proving both assertions load-bearing by mutation. The grounds: `19-CONTEXT.md` records a personal user direction governing this phase — "no questions at any gate; best well-reasoned choice, proceed, record it" — and `19-01-PLAN.md`'s objective states no decision checkpoint is inserted "because gating a decision the developer has already made is re-asking a settled question". The gate's substantive purpose (do not stack expansion on a broken foundation) was met by evidence rather than by prose.
- **Files modified:** none
- **Verification:** the tracer verify was re-run before Task 2 began and again after Task 3.
- **Committed in:** n/a (process)

---

**Total deviations:** 3 (1 blocking, 1 missing-critical/security, 1 process)
**Impact on plan:** No scope creep. Deviations 1 and 2 are both required for the plan's own acceptance criteria to mean anything — without them the tracer would have passed for the wrong reason and the stub would have carried an injection seam. Deviation 3 changed no code.

## Issues Encountered

None. Every verification passed on first execution except where a mutation was deliberately introduced.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo test` | exit 0 — **801 passing** (baseline 773 at `740e62f`; +28, no regression) |
| `cargo clippy -- -D warnings` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | **exactly 5** pre-existing lints — `browser.rs` ×3, `project_creator.rs` ×1, `state_reader/mod.rs` ×1. Count unchanged. Measured with `rtk proxy` per D-34. |
| `cargo test --test envelope_tracer` | exit 0 — 6 tests |
| `cargo test --test spawn_seam_guard` | exit 0 — 7 tests, no unexpected spawn site |
| `grep -c 'This layer does NOT see' src/envelope/mod.rs` | 3 |
| `grep -rn 'is_plain_run_id' src/ tests/` | no matches |
| `grep -n 'pub mod envelope;' src/lib.rs` | exactly 1 |
| `grep -nE '\.git/(config\|hooks)' src/envelope/*.rs` | 1 match, a doc comment in `cred.rs`; no line performs a write |

**Mutation checks (executed, then reverted before commit):**

| Mutation | Expected | Observed |
|---|---|---|
| `classify_push_ref` → always `Allow` | refusal test red | `a_driven_push_to_main_…` failed: "a push to refs/heads/main left the envelope and reached the remote" |
| `classify_push_ref` → always `Refuse` | allow test red | `a_driven_push_inside_the_reserved_namespace_…` failed |

## Known Stubs

None. Every function this plan introduces is wired to a caller and exercised by a passing test. `envelope::install` (the ambient-root variant) has no production caller yet — plan 19-06 owns the driver wiring — but it is not a stub: it is the same code path the fixture exercises through `install_in`, differing only in how the root is resolved.

## Threat Flags

None. No new network endpoint, auth path or schema was introduced. Every trust boundary this plan touches is already in the plan's `<threat_model>`, and T-19-01, T-19-03 and T-19-04 are mitigated by committed tests. `T-19-SC` holds: **no crate was added to `Cargo.toml`.**

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Ready for 19-02** (argv denylist / `classify_git`). `policy.rs` is the module it extends; `PushVerdict` and the reason constants are the shape its `GitVerdict` should sit beside. The `git -c core.hooksPath=…` form is named in `cred.rs`'s doc as 19-02's responsibility to deny.
- **Ready for 19-03/04/05.** `envelope_dir_in` is the seam every later artifact (`pr-ledger.ndjson`, `settings.json`, `askpass`, `gh/`) hangs from, and it already refuses hostile aliases.
- **For 19-06 (wiring):** `cred::hooks_path_env` returns `Vec<(OsString, OsString)>` ready to fold into the spawn closure at `src/executor/claude.rs:414-436`, and `hooks::install(alias)` is the run-start call.
- **For 19-07 (honesty text):** `src/envelope/mod.rs` and `src/envelope/cred.rs` already carry the substance of the limits paragraph; the pinned constant should agree with them rather than restate them differently.
- **Note for 19-08 (gate):** `SAFE-01` is declared by 19-01, 19-06, 19-07 and 19-08. The shared-ID gate correctly reported `0/1 ready`, so `REQUIREMENTS.md` was **not** touched by this plan — the checkbox flips when the last declaring plan produces its SUMMARY.
- **No blockers.**

---
*Phase: 19-gitsafe-git-blast-radius-envelope*
*Completed: 2026-08-18*
