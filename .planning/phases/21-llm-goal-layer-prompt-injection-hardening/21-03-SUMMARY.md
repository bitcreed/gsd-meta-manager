---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 03
subsystem: config
tags: [checkpoint, sha256, digest, opt-in, disclosure, serde-flatten, config-migration]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    plan: "01"
    provides: "plan_digest (FNV-1a, awaiting the sha256: upgrade), driver::untrusted's third-party string census, the SpawnProfile seam"
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    plan: "02"
    provides: "the fifth sibling taxonomy; the escalation cap. Neither touched plan_digest, both explicitly left it here."
  - phase: 17
    provides: "DriverOptIn, record_opt_in as the single construction site, claude_md_digest"
provides: []
affects: [21-04, 21-05, 21-06]

actuals:
  tokens: 21000
  tasks: 0
  commits: 1

tech-stack:
  added: []
  patterns: []

key-files:
  created:
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-03-SUMMARY.md
  modified: []

key-decisions: []

requirements-completed: []

duration: 25min
completed: 2026-08-19
status: checkpoint
---

# Phase 21 Plan 03: The SHA-256 Upgrade and the Disclosure Surface — CHECKPOINT

**Halted at Task 1, the plan's `checkpoint:decision` gate, with zero source changes. Pre-checkpoint
investigation found that the recommended option's load-bearing justification is factually wrong
about this tree, and proved it empirically.**

## Status

Task 1 is a `gate="blocking"` `checkpoint:decision` and it is the **first** task in the plan. No
implementation work was in scope before it, so no source file was modified and no task commit
exists. This SUMMARY is the only artifact.

Tasks 2 and 3 remain entirely unexecuted behind the gate.

## Pre-checkpoint investigation

The orchestrator's standing instruction is to check the tree before presenting a checkpoint,
because twice this phase an option turned out to be stale once someone read the code. That check
found four things, one of which changes the recommended option.

### 1. Option A's stated justification is wrong — proven, not reasoned

Option A's pros claim the addition is safe because *"`RegisteredProject::extra`'s flatten preserves
unknown fields"*. That flatten exists (`src/config.rs:62`), and a second one exists on
`Preferences` (`src/config.rs:217`) — but **`DriverOptIn` has neither.** Its six fields
(`src/config.rs:78-124`) carry no `#[serde(flatten)] extra`. `prompt_inputs` would be added one
level *deeper* than the protection that is supposed to cover it.

Proven with a throwaway integration test against the real `load_config` / `save_config`, rather
than argued from serde semantics. A config carrying an unknown key at the entry level and another
nested inside `driver_opt_in`, loaded and re-saved by this build:

```
entry-level unknown survived: true
NESTED unknown survived:      false
```

The nested `prompt_inputs` key was **silently deleted**. The probe file was removed; the tree is
clean and no probe artifact is committed.

Two consequences:

- **Option A is not wrong, it is incomplete.** It additionally requires an `#[serde(flatten)]
  extra: Map<String, Value>` on `DriverOptIn`, matching the technique the two enclosing structs
  already use. Without it, a user who opts in on the new binary and then runs v1.6.0 loses
  `prompt_inputs` on the next save.
- **The plan's own acceptance criterion cannot pass as written.** Task 2 requires *"a test asserts
  a `config.json` written by a newer binary carrying an unknown key inside `DriverOptIn` still
  round-trips that key"*. Against the current tree that test fails. T-21-20's mitigation — "the
  existing flatten-preservation posture" — is a paper mitigation at this level.

The security impact is **fail-safe, not a hole**: a deleted `prompt_inputs` reads as absent, and
absent means re-confirm. The cost is a spurious re-confirmation after a downgrade round-trip, not a
silent approval. That is worth stating precisely so the fix is scoped as durability, not as a
vulnerability.

### 2. Options B and C each violate the plan's own `must_haves`

- **Option B** (replace and migrate on first load) contradicts `must_haves.truths[0]`, which
  requires *"no user's `config.json` needs a migration (C-4)"*, and contradicts `src/config.rs:73-76`,
  the comment that exists specifically to prevent a second migration of a user-owned file.
- **Option C** (no new key; derive the list at render time) contradicts the plan's frontmatter
  `artifacts` entry requiring `src/config.rs` to provide `DriverOptIn::prompt_inputs`, both
  `key_links` entries, and research Q4's requirement that approval bind to the disclosed files.

Selecting B or C is therefore not a free choice between three live options — it requires amending
`must_haves` first. **A is the only option consistent with the plan as written**, which makes this
checkpoint substantially a confirm-with-correction rather than a three-way fork. Recorded plainly
because a checkpoint whose options are stale wastes the gate.

### 3. The disclosed file set is larger than the plan's prose implies

The plan's Task 3 text describes the seam as carrying *"the enumerated third-party strings from
`ROADMAP.md` and `STATE.md`"*. `src/driver/untrusted.rs:125-126` says `ProjectState` is parsed from
the project's `STATE.md` **and its `HANDOFF` file**. A disclosure naming two files when three
contribute bytes is under-broad — which the plan's own prohibition rates as exactly as dishonest as
an over-broad one. The disclosure must name `STATE.md`, `ROADMAP.md` and `HANDOFF` for the seam
profile, plus `CLAUDE.md` for the executor profile.

### 4. Task 3 points at the wrong file for the gate

Task 3's `<files>` and `<read_first>` name `src/registry.rs` as *"where the opt-in is consulted
before a spawn (the gate)"*. `src/registry.rs:166-171` explicitly says the opposite about itself:
`is_opted_in` is *"**Not a gate**: the gate is `DrivableProject::from_registry`"*. The real gate is
`src/executor/mod.rs:159`, which today checks only `driver_opt_in.is_none()`. Task 3 will need
`src/executor/mod.rs` in its file set. Flagged rather than silently corrected, since it changes
which file the drift check lands in.

### 5. `sha2` resolved mechanically — no checkpoint spent

Per CONTEXT.md's explicit instruction, the version was resolved mechanically rather than gated:

```
$ cargo add sha2 --dry-run
      Adding sha2 v0.11.0 to dependencies
             Features:
             + alloc
             + oid
             - zeroize
```

**Resolved version: 0.11.0.** CONTEXT.md's C-4 note is correct that research's `0.10.9` was stale
as *latest*.

One nuance the plan did not anticipate, which bears on the manifest comment's required graph-cost
claim: **`sha2` 0.10.9 is already in `Cargo.lock`** (line 2056), reached by `termwiz` and
`wezterm-blob-leases`. But `cargo tree -i sha2` reports *"nothing to print"* on the host target and
under `--target all -e all` — it is an orphan lockfile entry from an optional ratatui backend
feature that is not enabled, **not a compiled unit**. So the `rustix` precedent ("the exact version
already present … adds zero new compilation units") does **not** apply here, and a comment claiming
it would be false. Adding 0.11.0 adds real compilation units and leaves two `sha2` majors in the
lockfile. The comment block must say that rather than borrowing `rustix`'s sentence. The
`cargo tree -p sha2 --depth 1` output the plan requires will be captured after the add.

### 6. A detail for the legacy-prefix reading

`PRE_PHASE_19_CONFIG` (`src/config.rs:396`) carries `"claude_md_digest": "fnv1a:0123456789abcdef"` —
prefix `fnv1a:`, **not** the `fnv1a64:` that `journal::argv_digest` actually emits. Any legacy
detection must therefore be "anything whose prefix is not `sha256:` re-confirms" rather than an
exact match on `fnv1a64:`. The plan's unknown-hash-family-means-re-confirm design already covers
this; noting it so the implementation does not narrow to an exact legacy match.

## Task Commits

| Task | Name | Commit | Files |
|---|---|---|---|
| 1 | Checkpoint decision — halted, not decided | — | none |

Only this SUMMARY is committed.

## Guards, and what each did against the UNFIXED behaviour

None yet — no guard was written, because no implementation task ran. The empirical probe in
finding 1 is not a guard; it was a throwaway diagnostic and was deleted.

## Deviations from Plan

None. The plan was followed exactly: its first task is a blocking checkpoint, and it was not
decided autonomously.

## Known Stubs

None — nothing was implemented.

Inherited limits this plan has **not** yet addressed, restated so they are not read as closed:

1. **`plan_digest` and `claude_md_digest` are still FNV-1a.** The `sha256:` upgrade is Task 2,
   unexecuted. Waves 1 and 2 both explicitly deferred this here.
2. **`CLAUDE.md` suppression still has no behavioural proof.** No field on this transport reports
   whether it took effect. The behavioural proof is 21-05's corpus, not this plan's. The
   residual-exposure text Task 3 will pin must say so plainly rather than implying the control is
   verified.
3. **The executor profile does not suppress `CLAUDE.md` at all** — strictly larger exposure than
   the two seams, deliberately out of scope per research Q2, and to be disclosed loudly by Task 3
   rather than fixed quietly or omitted.
4. **`DriverOptIn` drops unknown nested keys** (finding 1). Pre-existing, discovered here, and
   currently unmitigated in the tree.

## Issues Encountered

- `rtk` strips `cargo test` output to a one-line summary, which hid the probe's `--nocapture`
  diagnostics entirely on the first run. Re-ran under `rtk proxy` per the phase context's warning.
  Worth noting that the filtered form reported `1 passed` for a test whose entire purpose was the
  printed output — a vacuous-looking pass.
- `tests/driver_reattach.rs` — the recorded pre-existing intermittent. Not touched, not
  investigated, per the plan.

## Next Phase Readiness

Blocked on the Task 1 decision. Once answered, Tasks 2 and 3 proceed with the four corrections
above folded in as Rule 2/Rule 3 adjustments.

## Self-Check: PASSED

- No source file modified: `git status --short` clean apart from this SUMMARY.
- Probe file `tests/zz_scratch_optin_flatten_probe.rs` created, run, and deleted; confirmed absent.
- `cargo add sha2 --dry-run` output recorded verbatim above.
- Empirical result in finding 1 recorded verbatim from the test's own stderr.
- STATE.md, ROADMAP.md and WINDOWS.md: untouched, per the orchestrator's instruction.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Halted at checkpoint: 2026-08-19*
