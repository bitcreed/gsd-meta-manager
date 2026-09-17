---
phase: quick-260915-hae
plan: 01
subsystem: infra
tags: [msrv, rust-version, github-actions, ci, cargo, release, documentation, state-hygiene]

# Dependency graph
requires:
  - phase: quick-260915-f4n
    provides: "The MEASURED refutation of the 1.87 floor and the fully specified msrv CI job this task executes"
  - phase: quick-task-19 (Add GitHub Actions release workflow)
    provides: ".github/workflows/release.yml — the file the msrv job was added to"
provides:
  - "A declared MSRV of 1.88 that is MEASURED true against the committed lockfile, not asserted"
  - "An `msrv` CI job that `publish` depends on, so a crates.io publish cannot happen unless the declared floor compiles first"
  - "A self-correcting manifest/toolchain agreement check, so a future floor bump cannot leave CI certifying a stale floor"
  - "Ten corrected MSRV documentation sites, four of which no longer misattribute the floor's provenance"
affects: [release-process, ci, cargo-manifest, documentation-msrv-sites, planning-state]

actuals:
  tokens: 3100
  tasks: 4
  commits: 3
  plan_head_before: 6e98a0db81cb73d52829fb58ddad7c030941eae2

tech-stack:
  added: []
  patterns:
    - "CI reads the floor from Cargo.toml (`cargo metadata ... .rust_version`) and asserts it against `rustc --version` on MAJOR.MINOR, rather than restating the number — drift fails the build instead of silently certifying a stale floor"
    - "The declared floor is derived, not chosen: it is the maximum `rust-version` across the resolved dependency graph, and every prose site says so plus how to re-measure it"

key-files:
  created:
    - .planning/quick/260915-hae-bump-declared-msrv-to-1-88-ship-msrv-ci-/260915-hae-SUMMARY.md
  modified:
    - Cargo.toml
    - .github/workflows/release.yml
    - README.md
    - CONTRIBUTING.md
    - CLAUDE.md
    - docs/DEVELOPMENT.md
    - docs/GETTING-STARTED.md
    - docs/TESTING.md
    - src/journal/redact.rs
    - .planning/STATE.md
    - .planning/quick/260915-f4n-bump-msrv-from-1-85-to-1-87-in-cargo-tom/260915-f4n-SUMMARY.md

key-decisions:
  - "The floor provenance was RELOCATED as well as rewritten: it now lives in the `[package]` block above `rust-version`, the item it is actually about, instead of on a dependency it was never about."
  - "The provenance is stated as a DERIVATION plus its reproduction command, not as a named source dependency — a hand-named source is the thing that went stale silently and would go stale again."
  - "`needs: msrv` on `publish` is the load-bearing edge; the job without it would run alongside publish and gate nothing."
  - "The agreement check compares MAJOR.MINOR (`cut -d. -f1,2`) so a `1.88` manifest value matches a `1.88.0` toolchain, and treats an empty/`null` manifest value as its own failure — a missing `rust-version` must not compare equal to anything."
  - "`process-wrap` was removed from the docs entirely rather than kept with a 'no longer the binding constraint' aside; the fact survives here, in this SUMMARY, which is the one place it belongs."
  - "No `Cargo.lock` edit and no toolchain pin file: `rust-version` declares a floor, a toolchain file is a pin, and pinning back to `ratatui 0.30.0` would not hold a 1.87 consumer floor anyway."

patterns-established:
  - "A version number nothing tests is documentation, not a contract. Gate the declared floor in CI on the same command that proved it locally, byte-identical."
  - "When a provenance sentence names a source, prefer the derivation plus the command that re-derives it — a named source rots without any edit to the file that names it."

requirements-completed: [QUICK-260915-hae]

coverage:
  - id: T1
    description: "The declared floor is a version the crate can actually be compiled on, MEASURED against the committed lockfile."
    verification:
      - kind: other
        ref: "cargo +1.88 check --all-targets --locked (exit 0); cargo 1.88.0 (873a06493 2025-05-10)"
        status: pass
    human_judgment: false
  - id: T2
    description: "A crates.io publish cannot happen unless that same floor compile passes first in CI."
    verification:
      - kind: other
        ref: "release.yml parses; jobs = ['msrv','publish']; publish.needs == 'msrv'; msrv runs `cargo check --all-targets --locked`"
        status: pass
    human_judgment: false
  - id: T3
    description: "The CI gate reads the floor from Cargo.toml rather than restating it."
    verification:
      - kind: other
        ref: "msrv job step 'Check manifest rust-version matches this job's toolchain' — cargo metadata .rust_version vs rustc --version, reduced with cut -d. -f1,2, empty/null failing separately"
        status: pass
    human_judgment: false
  - id: T4
    description: "No surviving sentence in the repo claims the floor comes from `process-wrap`; every provenance statement names the dependency graph that actually sets it and says how to re-measure it."
    verification:
      - kind: other
        ref: "grep -rn 'process-wrap' README.md CONTRIBUTING.md docs/ src/ -> empty; 'resolved dependency graph' present 1x in DEVELOPMENT.md, 2x in GETTING-STARTED.md, 1x in Cargo.toml [package]"
        status: pass
    human_judgment: false
  - id: T5
    description: "No reader of the tree can find a stale 1.87 MSRV claim in any tracked file outside `.planning/`."
    verification:
      - kind: other
        ref: "grep -rn '1\\.87' Cargo.toml CLAUDE.md README.md CONTRIBUTING.md docs/ src/ .github/ -> empty"
        status: pass
    human_judgment: false
  - id: T6
    description: "`.planning/STATE.md` no longer carries the Phase 15 MUST-SPIKE note, and nothing else in that file moved."
    verification:
      - kind: other
        ref: "git diff d11481d --numstat -- .planning/STATE.md -> 0 14; 12 survivor assertions pass"
        status: pass
    human_judgment: false

# Metrics
duration: ~25min
completed: 2026-09-15
status: complete
outcome: shipped
deliverable_shipped: true
---

# Quick Task 260915-hae: MSRV 1.88 — Declared, Enforced, and Correctly Attributed

**The MSRV stopped being a claim and became a gate: `Cargo.toml` now declares `rust-version = "1.88"` — a value MEASURED to compile every target against the committed lockfile — and `release.yml`'s new `msrv` job re-proves it on that exact toolchain before `publish` can run. Ten documentation sites were brought to 1.88, and the four that claimed the floor came from `process-wrap` now state the measured derivation instead. Separately and atomically, one closed Phase 15 question stopped posing as an open one in STATE.md.**

## Performance

- **Duration:** ~25 min
- **Completed:** 2026-09-15
- **Tasks:** 4 of 4 executed
- **Commits:** 3 (measured: `git rev-list --count 6e98a0d..HEAD`)
- **Files modified:** 10 tracked (9 outside `.planning/`, plus `STATE.md`)

## Task Commits

| Task | Name | Commit | Files |
|---|---|---|---|
| 1 (tracer) | Raise the floor at its source and gate publish on it | `2010cd3` | `Cargo.toml`, `.github/workflows/release.yml` |
| 2 | Bring the ten documentation MSRV sites to the enforced floor | `6c93fba` | `README.md`, `CONTRIBUTING.md`, `CLAUDE.md`, `docs/DEVELOPMENT.md`, `docs/GETTING-STARTED.md`, `docs/TESTING.md`, `src/journal/redact.rs` |
| 3 | Prove no regression and that every Part A fence held | — (verification only, no files modified) | — |
| 4 | Remove the one resolved Phase 15 MUST-SPIKE note | `35953e0` | `.planning/STATE.md` |

Part A (tasks 1-2) and Part B (task 4) share no file and are separate commits, per the plan's own commit boundaries.

## Task 1 local measurement, verbatim

```
cargo +1.88 --version                       ->  cargo 1.88.0 (873a06493 2025-05-10)
cargo +1.88 check --all-targets --locked    ->  EXIT=0
```

The only diagnostic is the pre-existing `unused_mut` warning at `tests/envelope_wrapper_class.rs:6127`, deliberately left alone (scope fence 4). `Cargo.lock` is byte-identical to the plan-time blob `6009a7b418c1295f5b3729577477095ae3195a02` — every cargo invocation carried `--locked` precisely so it could not move.

## The corrected provenance fact

The floor is **not** hand-chosen and is **not** owned by any one dependency. It is the highest `rust-version` declared anywhere in the **resolved dependency graph**, currently **1.88.0**, set jointly by:

- the `ratatui` 0.30.x family (`ratatui 0.30.2`, `ratatui-core`, `ratatui-crossterm`, `ratatui-termina`, `ratatui-termwiz`, `ratatui-widgets`),
- `time` / `time-core` / `time-macros`,
- `darling` / `darling_core` / `darling_macro` 0.23.0,
- the `icu_*` 2.3.x crates reached through `icu_properties`.

It moves under an ordinary `cargo update` with **no edit to any manifest in this repo** — which is exactly how it drifted past the previously declared 1.87 unnoticed, and exactly what the `msrv` job now catches. The one-line command that re-measures it, so the next drift is diagnosable in one step (it is now carried in `Cargo.toml` and in `docs/GETTING-STARTED.md`):

```bash
cargo metadata --format-version 1 --locked \
  | jq -r '.packages[].rust_version | select(. != null)' | sort -V | tail -1
```

Measured while executing: `1.88.0`.

### `process-wrap` — the fact that survives only here

`process-wrap` **9.1.0 does declare `rust-version = "1.87.0"`**. That is a true statement about that crate, and it was true before this bump too. It was simply **never the binding constraint** — the graph moved past it, and the four sentences that named it as the source of this project's floor were wrong about causation, not merely about the number. Task 2's gate is a flat absence check for the string across `README.md`, `CONTRIBUTING.md`, `docs/` and `src/`, so no "no longer the binding constraint" aside was preserved in the docs. This paragraph is the one place the fact is meant to survive.

### `CLAUDE.md:99` deliberately left alone

`CLAUDE.md` line 99 reads `notify-rs GitHub — cross-platform filesystem watching, MSRV 1.85 (HIGH confidence)`. That is a true statement about the **notify** crate's own floor in a research-sources bullet, not about this project's floor. It is the one MSRV-shaped string in the tree that is correct as written and was **not** touched (scope fence 1). `CLAUDE.md`'s diff against the `d11481d` baseline is exactly **1 line added / 1 removed** — only the tech-stack table row's version, with the rest of that row byte-for-byte unchanged.

## Task 3 — regression gates, measured

Run under the default stable toolchain, all through `rtk proxy` so cargo's output was raw, and with raw output captured to a file rather than piped through `grep`/`head`/`awk`:

| Gate | Result |
|---|---|
| `cargo build` | **exit 0** |
| `cargo clippy -- -D warnings` | **exit 0** |
| `cargo test --no-fail-fast` | exit 101 — **2002 passed / 4 failed / 15 ignored** across 48 test binaries |

`--no-fail-fast` is mandatory here and is not a style preference: a plain `cargo test` stops at the failing `driver_reattach` binary, which sorts before every `envelope_*` binary, so it never executes any envelope suite.

### The 4 failures, each accounted for — none is a regression of this task

1. **`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`** (lib unit tests, 1236 passed / 1 failed / 1 ignored) — the documented **environmental** git-version constants assertion: locally installed git differs from the version the constants were derived against. Scope fence 4; not fixed.
2. **`driver_reattach`: `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`** and **`a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step`** (1 passed / 2 failed) — the documented pre-existing spawn/liveness race, tracked at `.planning/todos/pending/2026-08-18-driver-reattach-spawn-artifact-race.md`. Scope fence 4; not fixed.
3. **`envelope_tracer::a_relocated_copy_of_the_stub_refuses_instead_of_acting`** (5 passed / 1 failed) — **a third condition, not in the plan's documented list, and INVESTIGATED rather than waved through.** It panics at `tests/envelope_tracer.rs:185` with `Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }` — a write handle on the generated stub is still open when the test execs it, a whole-suite parallelism race. Evidence it is not this task's regression: **re-run in isolation it passes** — `cargo test --no-fail-fast --test envelope_tracer` → **exit 0, 6 passed / 0 failed**. It is also causally impossible for this task to have produced it: the only source-tree change in Task 1-2 is one doc-comment word in `src/journal/redact.rs`. Recorded as a flake follow-up below; not fixed (scope boundary).

`cargo test` was not re-run against the pre-change tree, so "pre-existing" for item 3 rests on the isolation re-run plus the causal argument, not on a baseline comparison. Stated that way deliberately.

## Part A scope fences — all verified by command

```
FENCE-1 ok: CLAUDE.md is a 1/1-line change; the notify MSRV 1.85 bullet intact
FENCE-2 ok: no rust-toolchain.toml and no rust-toolchain file exists
FENCE-3 ok: Cargo.lock byte-identical to 6009a7b418c1295f5b3729577477095ae3195a02
FENCE-4 ok: the three known failures recorded, not fixed; unused_mut warning untouched
FENCE-5 ok: no new workflow file — the msrv job lives in the existing release.yml
FENCE-6 ok: STATE.md untouched before Task 4 (git diff --quiet d11481d)
FENCE-7 ok: exactly the nine expected tracked non-.planning files changed
```

Fence-7's expected set, matched exactly: `.github/workflows/release.yml`, `CLAUDE.md`, `CONTRIBUTING.md`, `Cargo.toml`, `README.md`, `docs/DEVELOPMENT.md`, `docs/GETTING-STARTED.md`, `docs/TESTING.md`, `src/journal/redact.rs`.

The `publish` job's existing steps were not altered other than adding `needs: msrv` — asserted mechanically (`publish.steps[-1].name == 'Publish'`); its tag trigger and `permissions: contents: read` are untouched, and the `msrv` job declares no `env:` so it receives no secret (T-hae-01).

## Part B — the one STATE.md bullet removed

Removed from `## Deferred Verification` → `### Phase 15 planning notes (autonomous run — review these)`: the single bullet opening `- **All three MUST-SPIKE questions resolved empirically**` and closing `... Phase 20's quota floor is still the real cost control.`, together with the blank line separating it from the next bullet. `git diff --numstat` against the `d11481d` baseline reports exactly **`0 14`**.

Removal loses nothing, measured four ways (re-stated from the plan's authority section, which was itself a live measurement):

1. The evidence is filed: `15-SPIKE-OQ1.md` and `15-01-SUMMARY.md` (`status: complete`) both exist on disk.
2. Its "remains plan 15-01 Task 1, the phase gate" clause is discharged by that same `15-01-SUMMARY.md`.
3. Its "Phase 20's quota floor is still the real cost control" clause is discharged: Phase 20 has executed, and `max-budget-usd` is recorded in 20+ planning documents.
4. `--setting-sources` (the OQ1 finding) is likewise recorded in 20+ documents including `15-SPIKE-OQ1.md` itself.

**Nothing was invented for the other topics.** The Phase 17 OQ4 and Phase 22 OQ5 topics have **nothing in STATE.md to remove** — `grep -n -iE 'worktree|OQ4|podman|OQ5' .planning/STATE.md` returns zero lines, and commits `ef7dc9b` and `31954d5` each state STATE.md was deliberately left untouched. No bullet was written for them.

The `260915-f4n` row in `### Quick Tasks Completed` **stays** — it is a historical log entry, not a stale decision. Twelve survivor assertions confirm the section heading, its four remaining bullets, the Phase 17 and Phase 14 sections, Pending Todos, Blockers/Concerns and that f4n row are all still present and unreflowed.

## The f4n SUMMARY no longer reads as blocking

`.planning/quick/260915-f4n-.../260915-f4n-SUMMARY.md` received the minimal honest edit required by this plan's `<output>`: a `superseded_by: quick-260915-hae` frontmatter key, an `outcome` that records the escalated fork as decided and shipped here, a one-line banner at the top of its body pointing at this SUMMARY, and a `DISCHARGED` marker on its "Blocked on a human decision" section so a reader landing mid-document is not misled. **Its measurements were not rewritten** — they are the evidence this task acted on, and they were correct.

## Deviations from Plan

**None in the deviation-rule sense.** No Rule 1/2/3 auto-fix was applied and no Rule 4 architectural change was made. Every scope fence, literal-string-absence check, provenance-phrase-presence check and the STATE.md `0/14` diff pin passed on first run; nothing was papered over.

One item goes beyond the plan's text and is called out rather than buried: the plan's scope fence 4 named **two** known pre-existing failure classes, and the measured run produced a **third** (`envelope_tracer` `Text file busy`). It was investigated by isolated re-run rather than assumed pre-existing, found green in isolation, and left unfixed as out-of-scope. See the Task 3 section above.

## Issues Encountered

- The plan's baseline `d11481d` is still an ancestor of HEAD and the only intervening commit was the PLAN.md itself, so every `d11481d`-relative fence was valid as written. No fence needed re-derivation.
- `rtk proxy` was used for every cargo invocation, and test output was captured to a file and aggregated from the raw text rather than piped through `grep`/`head`/`awk` — rtk's normal filtering strips `warning:` and `test result:` lines, so a grep over the filtered stream would have succeeded vacuously.

## Known Stubs

None. No stub, placeholder, TODO or unwired data path was introduced — the source-tree change is one doc-comment word.

## User Setup Required

None.

## Follow-ups deliberately not taken

- **MSRV on every push/PR.** The right posture, but it needs its own `ci.yml` (the repo has no PR workflow today, only `release.yml`) and is a behaviour and cost change nobody asked for — scope fence 5.
- **SHA-pinning all four GitHub Actions in one pass (T-hae-02).** `release.yml` now uses four mutable refs (`actions/checkout@v4`, `dtolnay/rust-toolchain@stable`, `dtolnay/rust-toolchain@1.88.0`, `Swatinem/rust-cache@v2`). A version-named branch is not a content pin; pinning one of four while three float buys no real reduction. Note the stakes: the `publish` job holds `CARGO_REGISTRY_TOKEN`.
- **The pre-existing `unused_mut` warning at `tests/envelope_wrapper_class.rs:6127`** — outside this task's scope, not fixed.
- **NEW: the `envelope_tracer` `ExecutableFileBusy` flake** under whole-suite parallelism (`tests/envelope_tracer.rs:185`). Passes in isolation; needs a proper fix (drop/flush the file handle before exec, or retry on `ETXTBSY`) in its own task.

## Next Steps

The floor is enforced. When the next milestone tag is cut, `CLAUDE.md`'s release process step 2 (`cargo update`) may raise the graph's maximum `rust-version` again — the `msrv` job will now fail the release rather than let it publish silently, and the reader will be pointed at the reproduction command in `Cargo.toml` to see the new value in one step.

## Self-Check: PASSED

Re-verified against the repo after writing, rather than asserted from the plan:

- `Cargo.toml` contains `rust-version = "1.88"`, no `1.87` literal, and `resolved dependency graph` inside the `[package]` block — **FOUND**.
- `.github/workflows/release.yml` parses as YAML with `jobs = ['msrv','publish']` and `publish.needs == 'msrv'` — **FOUND**.
- `.planning/quick/260915-hae-.../260915-hae-SUMMARY.md` exists on disk — **FOUND**.
- Commits `2010cd3`, `6c93fba`, `35953e0` all exist in `git log` — **FOUND**.
- `git rev-list --count 6e98a0d..HEAD` = **3**, agreeing with the three task commits listed above — the count is measured from the on-disk ledger, not narrated.

---
*Quick task: 260915-hae*
*Completed: 2026-09-15 — declared floor 1.88, enforced in CI before publish, correctly attributed in ten sites*
