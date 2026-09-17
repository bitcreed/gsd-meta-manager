---
phase: quick-260915-f4n
plan: 01
subsystem: infra
tags: [msrv, rust-version, github-actions, ci, cargo, release]

# Dependency graph
requires:
  - phase: quick-task-19 (Add GitHub Actions release workflow)
    provides: .github/workflows/release.yml — the file the msrv job was to be added to
provides:
  - "A MEASURED refutation of the declared MSRV: rustc 1.87.0 cannot compile this crate against the committed Cargo.lock. The true floor is 1.88."
  - "The measured dependency-level cause: two DIRECT deps (ratatui 0.30.2, icu_properties 2.3.0) declare rust-version 1.88, reached by semver-compatible patch drift in Cargo.lock."
  - "A scoped hand-off for the follow-up task that must decide the floor before any CI gate can be honest."
affects: [release-process, ci, cargo-manifest, documentation-msrv-sites]

actuals:
  tokens: 0
  tasks: 1
  commits: 0
  plan_head_before: 31954d5

tech-stack:
  added: []
  patterns: []

key-files:
  created:
    - .planning/quick/260915-f4n-bump-msrv-from-1-85-to-1-87-in-cargo-tom/260915-f4n-SUMMARY.md
  modified: []

key-decisions:
  - "Task 1 decision rule branch (b) applied: BOTH `--all-targets` and lib/bins-only forms fail on 1.87, so the declared floor is FALSE and the plan ends at Task 1. Task 2 (the msrv CI job) was NOT written and Task 3 was reduced to the scope-fence checks."
  - "No `Cargo.toml` edit, no doc edit, no lockfile pin-back. Raising or restoring the floor changes the consumer contract and invalidates six documentation sites; that is a scope change owning its own task, not a silent edit inside a quick task."
  - "The 1.88 measurement was taken in ADDITION to the plan's two required measurements so the follow-up task inherits a definite floor instead of a refutation alone. It modified no repo file."

patterns-established:
  - "A declared `rust-version` is not evidence: measure it with `cargo +<floor> check --all-targets --locked` before writing any CI gate that claims to enforce it."

requirements-completed: []

coverage:
  - id: D1
    description: "MEASURED: rustc 1.87.0 cannot compile this crate against the committed Cargo.lock — the declared floor of 1.87 is false."
    verification:
      - kind: other
        ref: "cargo +1.87 check --all-targets --locked (exit 101); cargo +1.87 check --locked (exit 101)"
        status: pass
    human_judgment: false
  - id: D2
    description: "MEASURED: rustc 1.88.0 compiles every target against the committed Cargo.lock — the true floor is 1.88."
    verification:
      - kind: other
        ref: "cargo +1.88 check --all-targets --locked (exit 0, 20.45s)"
        status: pass
    human_judgment: false
  - id: D3
    description: "NOT DELIVERED — an `msrv` job in .github/workflows/release.yml gating `publish` via `needs: msrv`."
    verification: []
    human_judgment: true
    rationale: "Task 2 was not executed. The plan's Task 1 decision rule branch (b) forbids proceeding when the declared floor is refuted, because a CI gate can only be written once a human decides WHICH floor the project is contracting to."
  - id: D4
    description: "All five scope fences verified by command: CLAUDE.md untouched, no toolchain pin file, no .rs/Cargo.toml/Cargo.lock change, no STATE.md bullet removed, zero tracked non-.planning files changed."
    verification:
      - kind: other
        ref: "git diff --quiet HEAD -- <paths> fence battery (5/5 ok)"
        status: pass
    human_judgment: false

# Metrics
duration: 5min
completed: 2026-09-15
status: complete
outcome: halted-at-task-1-by-decision-rule-b; the escalated fork was decided (raise the floor to 1.88) and shipped by quick-260915-hae
deliverable_shipped: false
superseded_by: quick-260915-hae
---

> **No longer blocking.** The human fork this task escalated was decided — the declared floor was
> raised to 1.88 — and the `msrv` CI gate it specified was shipped by quick task **260915-hae**.
> See `.planning/quick/260915-hae-bump-declared-msrv-to-1-88-ship-msrv-ci-/260915-hae-SUMMARY.md`.
> The measurements below are unchanged; they are the evidence that task acted on, and they were
> correct.

# Quick Task 260915-f4n: MSRV Enforcement Summary

**The declared MSRV of 1.87 was MEASURED and is FALSE — `cargo +1.87 check --locked` exits 101 on this crate; the true floor is 1.88, reached by semver-compatible patch drift in `Cargo.lock` on two DIRECT dependencies. The plan's Task 1 decision rule branch (b) fired, so the `msrv` CI job was deliberately NOT written and no repo file was changed.**

## The titled work was already done — and was never needed

Recorded here because it is the first thing a reader of the task title needs:

- `Cargo.toml:5` already reads `rust-version = "1.87"`. It landed in commit **41a7b3f**
  (`chore(15-01): raise MSRV to 1.87 and add the transport dependencies`).
- `git log -S` confirms the string `rust-version = "1.85"` has **never** existed in
  `Cargo.toml`. There was no 1.85 to bump from.
- There is no `rust-toolchain.toml` and no `rust-toolchain` file. Nothing to bump there either.

So the titled work (`bump MSRV from 1.85 to 1.87 in Cargo.toml`) was a **no-op**, and the real
deliverable was enforcement: an `msrv` CI job proving the floor before any crates.io publish.
Executing Task 1 established that the enforcement cannot be written yet, because the thing it
would enforce is not true.

## Performance

- **Duration:** ~5 min
- **Started:** 2026-09-15T17:00:04Z
- **Completed:** 2026-09-15T17:05Z
- **Tasks:** 1 of 3 executed (Task 2 forbidden by the decision rule; Task 3 reduced to its fence checks)
- **Files modified:** 0 tracked files

## BLOCKER — the declared floor of 1.87 is false

### Task 1 measurement, verbatim

Toolchain installed: `rustup toolchain install 1.87 --profile minimal`

```
cargo +1.87 --version   ->  cargo 1.87.0 (99624be96 2025-05-06)
rustup run 1.87 rustc --version  ->  rustc 1.87.0 (17067e9ac 2025-05-09)
```

Command run, verbatim, and its exit code:

```
cargo +1.87 check --all-targets --locked      EXIT=101
```

Verbatim first error:

```
error: rustc 1.87.0 is not supported by the following packages:
  darling@0.23.0 requires rustc 1.88.0
  darling_core@0.23.0 requires rustc 1.88.0
  darling_macro@0.23.0 requires rustc 1.88.0
  icu_collections@2.3.0 requires rustc 1.88
  icu_locale_core@2.3.0 requires rustc 1.88
  icu_properties@2.3.0 requires rustc 1.88
  icu_properties_data@2.3.0 requires rustc 1.88
  icu_properties_data@2.3.0 requires rustc 1.88
  icu_properties_data@2.3.0 requires rustc 1.88
  icu_provider@2.3.1 requires rustc 1.88
  ignore@0.4.31 requires rustc 1.88
  instability@0.3.12 requires rustc 1.88
  instability@0.3.12 requires rustc 1.88
  instability@0.3.12 requires rustc 1.88
  ratatui@0.30.2 requires rustc 1.88.0
  ratatui-core@0.1.2 requires rustc 1.88.0
  ratatui-crossterm@0.1.2 requires rustc 1.88.0
  ratatui-widgets@0.3.2 requires rustc 1.88.0
  time@0.3.54 requires rustc 1.88.0
  time-core@0.1.9 requires rustc 1.88.0
Either upgrade rustc or select compatible dependency versions with
`cargo update <name>@<current-ver> --precise <compatible-ver>`
where `<compatible-ver>` is the latest version supporting rustc 1.87.0
```

The decision rule's branch (a) escape hatch — "dev/test targets have drifted but the shipped
crate honours the floor" — was tested and does **not** apply:

```
cargo +1.87 check --locked   (lib and bins only)   EXIT=101
```

It emits the same error minus `ignore@0.4.31` (that one is a dev-target-only path). The refusal
is a manifest-level `rust-version` check across the resolved graph, not a source-compatibility
failure, so narrowing the target set cannot help. **Branch (b) applies.**

### The true floor, measured

Taken in addition to the plan's two required measurements so the follow-up inherits a number
rather than only a refutation:

```
cargo +1.88 check --all-targets --locked      EXIT=0   (Finished dev profile in 20.45s)
```

`rustc 1.88.0 (6b00bc388 2025-06-23)`. The only diagnostic is one pre-existing
`unused_mut` warning in `tests/envelope_wrapper_class.rs:6127` — unrelated to the floor.

**The true floor is 1.88.**

### Why it drifted — the cause is `Cargo.lock`, not `Cargo.toml`

Two of the named packages are **direct** dependencies of this crate (measured with
`cargo tree -i`, both report `└── gsd-meta-manager v1.6.0` as their only dependent):

| Dependency | `Cargo.toml` requirement | Locked version | Its `rust-version` |
|---|---|---|---|
| `ratatui` | `"0.30"` | **0.30.2** | **1.88.0** |
| `icu_properties` | `"2.2.0"` | **2.3.0** | **1.88** |

The versions this project's manifest names honour 1.87:

| Dependency | Version | Its `rust-version` |
|---|---|---|
| `ratatui` | 0.30.0 | 1.86.0 |
| `icu_properties` | 2.2.0 | 1.86 |

Everything else in the error list is transitive behind these two plus `tracing-appender`
(`darling`/`instability` behind `ratatui`, `time` behind `ratatui-widgets` and
`tracing-appender`, `ignore` on a dev path).

So the floor was raised by **semver-compatible patch upgrades in the lockfile**, most plausibly
by the `cargo update` that `CLAUDE.md`'s release process mandates at every milestone (step 2).
Nothing in `Cargo.toml` was ever edited to cause it, which is exactly why it went unnoticed: a
patch bump silently raised the effective MSRV while the declared one stood still. **This is the
failure mode the requested CI gate exists to catch, and it had already happened before the gate
could be written.**

### What the follow-up task must decide (it is a real fork, not a typo fix)

Do **not** treat this as "change 1.87 to 1.88 and move on" without reading this. The two options
are not equivalent:

1. **Raise the declared floor to 1.88.** Honest, one-line manifest change — but it changes the
   consumer contract and invalidates every MSRV statement in the tree. MEASURED with grep, not
   copied from the plan:

   | Site | What it says |
   |---|---|
   | `README.md:61` | `Requires **Rust 1.87+**.` |
   | `README.md:186` | `- **Rust 1.87+** (for building from source)` |
   | `CONTRIBUTING.md:23` | `Prerequisites: Rust 1.87+ (stable)` |
   | `docs/DEVELOPMENT.md:10-11` | `Rust 1.87+` **plus the process-wrap provenance claim** |
   | `docs/GETTING-STARTED.md:13` | `>=1.87` **plus the process-wrap provenance claim** |
   | `docs/GETTING-STARTED.md:150,154` | a troubleshooting heading quoting `requires rustc 1.87 or newer` **plus the provenance claim a third time** |
   | `docs/TESTING.md:14` | `1.87+ stable` |
   | `src/journal/redact.rs:218` | `this crate's MSRV is 1.87` |
   | `Cargo.toml:36-38` | comment asserting `process-wrap`'s 1.87.0 is "where the `[package] rust-version` floor above comes from" |

   **The provenance claim is the subtle part and appears FOUR times** (`Cargo.toml`,
   `DEVELOPMENT.md`, and twice in `GETTING-STARTED.md`): all four assert the floor comes from
   `process-wrap` 9.1.0. That is now false in two ways — the floor is 1.88, and it comes from
   `ratatui`/`icu_properties`, not `process-wrap`. A find-and-replace of `1.87` → `1.88` would
   leave all four claims standing and wrong. They must be rewritten, not bumped.
2. **Pin the lockfile back** to `ratatui 0.30.0` / `icu_properties 2.2.0` to restore a real
   1.87 build. **This does not preserve the floor for consumers** and should not be chosen
   believing it does: `Cargo.toml` says `ratatui = "0.30"`, a caret requirement matching
   `>=0.30.0, <0.31.0`, so a downstream resolution that is not bound by this repo's lockfile
   picks 0.30.2 and needs 1.88 regardless. (That is semver reasoning about the requirement, not
   a measurement — it was not measured here, because measuring it would have required
   re-resolving and thus writing `Cargo.lock`, which scope fence 4 forbids.) Genuinely holding a
   1.87 consumer floor would mean narrowing the requirements themselves (`=0.30.0`), freezing
   out patch fixes — and it fights the release process's own `cargo update` step every
   milestone.

Option 1 is the coherent one. It is recorded here as a decision for a human, not taken.

**Once the floor is settled, the original deliverable is still worth shipping and is fully
specified in `260915-f4n-PLAN.md` Task 2** — an `msrv` job on `dtolnay/rust-toolchain@<floor>`,
a `cargo metadata`-vs-`rustc --version` agreement step so a future bump cannot leave CI
certifying a stale floor, and `needs: msrv` on `publish`. Only the version literal changes.

## Accomplishments

- Measured the real floor instead of trusting the manifest, and found the manifest wrong. This
  is the entire value of the task: the crate has been published to crates.io having never been
  compiled on its declared floor, and it would not have compiled.
- Localised the cause to two direct dependencies and to lockfile drift rather than to a
  source-level use of a post-1.87 feature — which tells the follow-up that the fix is a
  contract decision, not a code change.
- Established the true floor (1.88) so the follow-up task starts from a number.
- Left the tree byte-for-byte unchanged, so nothing has to be unwound if the fork is decided
  the other way.

## Task Commits

**None.** Task 1 modifies no repo file by design, and its decision rule forbade Tasks 2 and 3
from modifying anything. `git rev-list --count 31954d5..HEAD` is **0** with **zero** code
changes — the legitimate zero case, not uncommitted work. `git status --short` shows only the
pre-existing untracked `.gsd/`, which predates this task and was not created by it.

## Files Created/Modified

- `.planning/quick/260915-f4n-.../260915-f4n-SUMMARY.md` — this file (docs; committed separately by the orchestrator)

No tracked file outside `.planning/` was changed.

## Decisions Made

- **Branch (b), not branch (a).** Branch (a) was tested rather than assumed: the lib/bins-only
  form was actually run and also exited 101. Shipping a "narrowed but honest" gate was therefore
  not available — there is no form of the command that passes on 1.87.
- **No silent floor edit.** Editing `Cargo.toml` to 1.88 inside this quick task would have been
  a one-character change that alters the published consumer contract and falsifies seven prose
  sites, under a task titled as a no-op bump. Recorded and escalated instead.
- **Measured 1.88 anyway.** Slightly beyond the literal decision rule, which only requires
  recording the failure. Justified because the rule's purpose is to hand off a scoped finding,
  and "1.87 is wrong" without "1.88 is right" would force the next agent to re-derive it. Cost:
  one toolchain download, zero repo writes.
- **Task 3's stable-toolchain gates deliberately NOT run.** `cargo build`, `cargo test
  --no-fail-fast` and `cargo clippy -- -D warnings` were not executed, and no pass/fail/ignored
  counts are reported — stating otherwise would be the vacuous kind of evidence this repo's
  conventions exist to prevent. Task 3 exists to prove Task 2's edit caused no regression; Task 2
  made no edit, and `git diff HEAD` over `src`, `tests`, `Cargo.toml` and `Cargo.lock` is empty,
  so there is no delta any of the three could detect. The fence half of Task 3 WAS run.

## Deviations from Plan

None in the deviation-rule sense — no Rule 1/2/3 auto-fix was applied and no Rule 4 architectural
change was made. The plan terminated through its own documented Task 1 decision rule, branch (b),
which is planned behaviour rather than a deviation.

Tasks not executed, as that rule requires:
- **Task 2 (wire the msrv job into release.yml)** — forbidden by branch (b).
- **Task 3 (regression gates)** — the build/test/clippy half is moot with a zero-byte diff (see
  Decisions). The scope-fence half was executed in full.

## Scope Fences — all five verified by command

```
FENCE-1 ok: CLAUDE.md untouched                        (git diff --quiet HEAD -- CLAUDE.md)
FENCE-2 ok: no STATE.md bullet removed                 (0 lines matching '^-- \[' in git diff -U0)
FENCE-3 ok: no toolchain pin file                      (rust-toolchain.toml / rust-toolchain absent)
FENCE-4 ok: no source or manifest change               (Cargo.toml, Cargo.lock, src, tests clean)
FENCE-5 ok: ZERO tracked non-.planning files changed   (git diff --name-only HEAD -- . ':!.planning' is empty)
```

Fence 5 is satisfied in its stronger form: the plan expected exactly one changed file
(`release.yml`); branch (b) produced none.

Fence 2 note: no STATE.md blocker bullet was added for this finding, deliberately. Fence 2
reserves the `### Blockers/Concerns` section for a separate consolidation pass, so the blocker
lives here and in the return to the orchestrator instead.

## Issues Encountered

- **The task's own premise was false in two layers.** The title's premise (1.85 in `Cargo.toml`)
  was refuted at planning time; the plan's own premise (that 1.87 is a true floor worth gating
  on) was refuted at execution time. Both refutations came from measurement, and the second is
  the one that matters.
- **`rtk proxy` used for every cargo and rustup invocation.** rtk strips lines from cargo output,
  and the `error: rustc 1.87.0 is not supported by...` block is exactly the kind of line a
  filtered stream can hide. No output was piped through `grep`/`head` to produce any count
  reported above.

## User Setup Required

None.

## Next Phase Readiness

**DISCHARGED by quick task 260915-hae (2026-09-15)** — all three steps below were executed there.
Kept verbatim as the handoff record. As written at the time:

**Blocked on a human decision, not on further agent work.** The follow-up task must:

1. Decide the fork above — almost certainly: declare the floor `1.88` in `Cargo.toml`.
2. Update the nine MSRV statement sites in the table above — and REWRITE (not bump) the four
   `process-wrap` provenance claims, which are wrong about the source of the floor as well as
   about its value.
3. Then execute this plan's Task 2 verbatim with `1.88.0` substituted for `1.87.0`, so the floor
   is enforced in CI and this class of silent drift cannot recur.

## Follow-ups deliberately not taken

- **MSRV on every push/PR.** Would need its own `ci.yml`; the repo has no PR workflow at all
  today, only `release.yml`. Running the floor check per-push is the right posture but is a
  behaviour and cost change the requester did not ask for (scope fence 5).
- **SHA-pinning all four GitHub Actions in one pass (threat T-f4n-02).** `release.yml` currently
  uses four mutable refs (`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`,
  `Swatinem/rust-cache@v2`, and the proposed `@1.87.0`/`@1.88.0`). Pinning one while three float
  buys nothing; pinning all four is a coherent change of its own. Note the stakes: the `publish`
  job holds `CARGO_REGISTRY_TOKEN`.
- **The `unused_mut` warning at `tests/envelope_wrapper_class.rs:6127`** — pre-existing, outside
  this task's scope, not fixed.

## Self-Check: PASSED

Every claim above re-verified against the repo after writing, rather than asserted from the plan:

- `.../260915-f4n-SUMMARY.md` exists on disk.
- `41a7b3f` exists and is `chore(15-01): raise MSRV to 1.87 and add the transport dependencies`.
- `git log -S 'rust-version = "1.85"' -- Cargo.toml` returns **no commits** — the string never
  existed, confirming the task title's premise independently of the plan.
- All nine MSRV documentation sites re-measured by grep; the plan's list was CORRECT but
  INCOMPLETE (it missed the provenance claim's second and third occurrences in
  `docs/GETTING-STARTED.md:150,154` and `docs/DEVELOPMENT.md:11`). Corrected above.
- `git rev-list --count 31954d5..HEAD` = **0**, with `git diff --name-only HEAD -- . ':!.planning'`
  empty — the zero-commit claim and the zero-change claim agree, so the 0 is the legitimate
  no-work-to-commit case rather than uncommitted work.

## STATE.md bookkeeping — deliberately NOT written

No `Quick Tasks Completed` row was added for this task. The row's schema carries a commit hash,
and there is no commit; a row reading `—` under a title that describes an MSRV bump would read as
"the bump shipped" to every future scanner of that table, which is the opposite of what happened.
The task's record is this SUMMARY. Whoever resolves the fork above should add the row when the
follow-up actually lands, citing that commit.

---
*Quick task: 260915-f4n*
*Completed: 2026-09-15 — halted at Task 1 by decision rule branch (b); deliverable not shipped*
