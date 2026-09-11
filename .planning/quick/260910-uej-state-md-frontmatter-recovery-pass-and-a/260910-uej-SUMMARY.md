---
phase: quick-260910-uej
plan: 01
subsystem: state_reader
status: complete
tags: [yaml, frontmatter, recovery, untrusted-input, utf8-safety]
requires:
  - src/state_reader/state_md.rs::read_frontmatter
  - src/driver/untrusted.rs::THIRD_PARTY_STRINGS (unchanged)
provides:
  - src/state_reader/state_md.rs::repair_frontmatter_block
  - src/state_reader/state_md.rs::FrontmatterOutcome::Recovered
  - src/state_reader/state_md.rs::FrontmatterFaultPosition
  - src/state_reader/mod.rs::ProjectState::state_md_recovered
  - src/state_reader/mod.rs::ProjectState::state_md_fault_position
  - src/app.rs::RECOVERED_STATE_MARKER
  - src/app.rs::unreadable_state_label
affects:
  - src/ui/screens/detail.rs (both STATE.md banner sites)
  - src/app.rs::format_phase_display
tech-stack:
  added: []
  patterns:
    - "in-memory conservative repair, never a disk write"
    - "typed (non-String) carriers across the untrusted boundary"
    - "char-wise string work on third-party text"
key-files:
  created: []
  modified:
    - src/state_reader/state_md.rs
    - src/state_reader/mod.rs
    - src/app.rs
    - src/ui/screens/detail.rs
    - tests/state_reader_test.rs
decisions:
  - "Recovery is a fourth FrontmatterOutcome variant, not a bool on Parsed (D-01)"
  - "The fault position is a Copy u32 pair, never a String (D-02)"
  - "FrontmatterFault is unchanged; the position rides beside it (D-03)"
  - "An out-of-range position is dropped, not clamped (D-04)"
  - "The real foreign file is exercised by an #[ignore]d env-var-addressed read-only test (D-05)"
metrics:
  duration: ~45m
  completed: 2026-09-10
commits: 3
plan_head_before: 81cae46bd25e8caa1b96acb87e5297fe47c9a964
actuals:
  tokens: 34000
  tasks: 3
  commits: 3
---

# Quick Task 260910-uej: STATE.md Frontmatter Recovery Pass Summary

A STATE.md whose only fault is a top-level unquoted plain scalar carrying a colon-space now
parses through one in-memory quote-in-place repair, reports itself as `Recovered` rather than
clean, and — when still broken — names its YAML fault as FILE-relative line/column numbers with
no parser text anywhere near a rendered string.

## What Shipped

| Task | Commit | What |
|------|--------|------|
| 1 | `109192f` | `repair_frontmatter_block` + `FrontmatterOutcome::Recovered` + `state_md_recovered` + the `~ ` cell marker + the detail-pane line |
| 2 | `4930e8a` | `FrontmatterFaultPosition`, `Unreadable { fault, position }`, body→file line translation, the validity gate, `! STATE.md unreadable (line N)` |
| 3 | `6e1911e` | The no-op proof, the multi-byte proof, the escape round-trip, the integration + exclusivity arms, the `#[ignore]`d read-only real-file test |

### The new outcome/state type, and how recovery appears in the UI

- **Type:** `FrontmatterOutcome::Recovered { frontmatter: Box<StateFrontmatter>, repaired_lines: u32 }`
  in `src/state_reader/state_md.rs`, carried onto `ProjectState` as `pub state_md_recovered: bool`.
- **Dashboard cell:** `format_phase_display` prefixes `RECOVERED_STATE_MARKER` (`"~ "`) onto the
  label it would otherwise produce — e.g. `~ Routine Event Logging` where a clean read gives
  `Routine Event Logging` and an unreadable one gives `! STATE.md unreadable`. Three visibly
  distinct states.
- **Detail pane:** `recovered_state_line` renders, at the same position as the unreadable banner
  and in **yellow** rather than red (a condition, not a fault):
  `  STATE.md repaired in memory to be read: the file on disk is unchanged`
- **Still-unreadable file:** the cell becomes `! STATE.md unreadable (line 7)`; the detail-pane
  banner appends ` (line 7, column 218)`. `UNREADABLE_STATE_LABEL` itself is byte-unchanged and is
  still exactly what a location-less fault renders.

## Verification — raw pass/fail

All three run **unpiped**, redirected to a file and read back (no `grep`/`head` in the pass/fail path):

| Command | Result |
|---------|--------|
| `rtk proxy cargo build` | **PASS** — exit 0, `Finished dev profile` |
| `rtk proxy cargo test --no-fail-fast` | **PASS for this plan** — exit 101, 2004 passed / 2 failed / 15 ignored; both failures are non-plan (below) |
| `rtk proxy cargo clippy -- -D warnings` | **PASS** — exit 0 |
| `rtk proxy cargo test --test spawn_seam_guard` | **PASS** — 38 passed / 0 failed, with `THIRD_PARTY_STRINGS` and `src/driver/untrusted.rs` UNCHANGED (`git diff --stat HEAD -- src/driver/untrusted.rs Cargo.toml` is empty) |

### Test count: before vs after

| | passed | failed | ignored | run (passed+failed) |
|---|---|---|---|---|
| **Before** (baseline on `81cae46`) | 1986 | 2 | 14 | 1988 |
| **After** | 2004 | 2 | 15 | 2006 |

**+18 passing tests** (12 in `state_reader::state_md`, 3 in `app::tests`, 3 integration) and **+1
ignored** (the opt-in real-file test). No test was removed or weakened.

### The two failures are not this plan's

1. `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
   — **pre-existing and environmental**: constants derived against `git version 2.43.0`, installed
   git is `2.53.0`. Fails identically on the baseline. The test's own message says the repair is to
   re-derive the constants; out of scope here.
2. `a_relocated_copy_of_the_stub_refuses_instead_of_acting` (`tests/envelope_tracer.rs:185`) —
   **a parallel-execution flake**: `Text file busy` while chmod-ing a stub. It PASSED in the
   baseline run, PASSED in the first post-change run, and passes on re-run in isolation
   (`cargo test --test envelope_tracer` → 6 passed / 0 failed).

The baseline itself also carried a third, different flake
(`the_t_19_116_replacement_takes_layer_2_as_well_and_that_is_measured_separately`, `BrokenPipe`)
which passes after. See the deviation on measured fact 7 below.

## Does the real sentriq STATE.md recover?

**Yes.** Run READ-ONLY through the `#[ignore]`d test, raw output:

```
GSD_META_MANAGER_REAL_STATE_MD=/home/blk/projects/flutter/sentriq/.planning/STATE.md \
  cargo test --test state_reader_test a_real_foreign_state_md_reads_read_only -- --ignored --nocapture

running 1 test
Recovered (1 line(s) repaired in memory).
  status        = "planning"
  current_phase = Some("9")
  stopped_at    = 1270 bytes
  stopped_at    = "Completed 260910-p8v (DTC history is vehicle-scoped, the trace recorder covers ..."
test a_real_foreign_state_md_reads_read_only ... ok
```

- Recovered: **yes**, exactly **1** line repaired, `stopped_at` back at the measured **1270 bytes**.
- `stopped_at` truncated to ~80 chars:
  `Completed 260910-p8v (DTC history is vehicle-scoped, the trace recorde`
- `git -C /home/blk/projects/flutter/sentriq status --short .planning/STATE.md` is **empty** — the
  file was opened with `std::fs::read_to_string` and nothing else. Nothing outside this repository
  was written.

## Commits

| SHA | Message |
|-----|---------|
| `109192f` | `feat(quick-260910-uej): a repairable STATE.md reads, and says it was repaired` |
| `4930e8a` | `feat(quick-260910-uej): carry the YAML fault position as FILE-relative numbers` |
| `6e1911e` | `test(quick-260910-uej): the proofs — no-op, UTF-8, exclusivity, and the real file read-only` |

Measured, not narrated: `git rev-list --count 81cae46..HEAD` = **3**.

## Deviations from Plan

### [Rule 1 — Bug] The planned prefix formula would have mislocated every fault

- **Found during:** Task 2.
- **Issue:** the plan's step 3 says to take `content.len() - body.len()` as the byte length of
  everything before the frontmatter body. `body` is a slice out of the MIDDLE of `content`, so that
  difference also counts the entire markdown document after the closing `---` — for sentriq's file
  that is ~1136 lines, and the reported fault line would have been absurd (and then silently
  dropped by the validity gate, so the position would simply never have appeared).
- **Fix:** the prefix is measured as the byte distance between the two slices' start pointers
  (`body.as_ptr() - content.as_ptr()`), which is exactly the leading whitespace plus `---\n`. The
  derivation, and why the subtraction form is wrong, is written into `yaml_fault_position`'s doc.
- **Proof:** `a_reported_line_is_translated_from_the_body_to_the_file` (file line 2) and
  `the_sentriq_shape_made_unrecoverable_names_file_line_seven` (file line 7, counted in the fixture
  rather than restated).
- **Commit:** `4930e8a`.

### [Rule 1 — Guard violation] A second spelling of the identity alphabet

- **Found during:** Task 3's full-suite gate.
- **Issue:** the key-shape check was written as
  `c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.')`, which
  `text::exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src` correctly
  failed: the repo holds the claim that this alphabet is spelled in exactly ONE executable place.
- **Fix:** delegate to `crate::text::is_identity_char`, as that test's failure message instructs.
  Not a softening of the claim.
- **Commit:** `6e1911e`.

### [Rule 3 — Signature] `repair_frontmatter_block` returns `Option<(String, u32)>`

The plan specifies `-> Option<String>`, but `Recovered` carries `repaired_lines` and the fixture
asserts `repaired_lines == 1`. Returning the count alongside the block keeps one source for it. The
`None` contract the no-op proof asserts is identical either way.

### Measured fact 7 was stale

The plan says the pre-existing failing pair is `driver_reattach`. It is not: on `81cae46` the
baseline failures are `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
(environmental, git 2.53.0 vs 2.43.0) and a `BrokenPipe` flake in `tests/envelope_carrier_reach.rs`.
`--no-fail-fast` is still mandatory for the same reason. No `driver_reattach` failure exists in
this tree.

### Fixture geometry (in-plan, worth recording)

`SENTRIQ_SHAPED_STATE_MD` was given the real file's **key order**, so its one faulty line sits on
file line 7 exactly as the real file's does. `sentriq_fault_file_line()` counts that in the fixture
and asserts it is 7 — the offset arithmetic is pinned by measured geometry rather than by a
restated constant.

## Inferred Decisions — FLAGGED FOR LATER AUDIT

The human operator was unavailable throughout. These five came from the planner and were carried
through implementation unchanged; **none was approved by a human.**

- **D-01 — recovery is a FOURTH enum variant, not a boolean on `Parsed`.** Implemented as
  `FrontmatterOutcome::Recovered { .. }`. `parse_project_state`'s two sequential `if let`s became
  ONE exhaustive match with no wildcard arm, so a fifth outcome is now a compile error.
- **D-02 — the position and the recovery signal are NON-String.** `state_md_recovered: bool` and
  `state_md_fault_position: Option<FrontmatterFaultPosition>` (a `Copy` `u32` pair).
  `THIRD_PARTY_STRINGS` is unchanged and `spawn_seam_guard` is green.
- **D-03 — `FrontmatterFault` is untouched.** `describe()` is still `&'static str`; the position
  rides beside the fault on the variant.
- **D-04 — an out-of-range position is DROPPED, not clamped.** The gate is
  `file_line <= block_lines + offset - 1`, plus `line != 0` and `u32::try_from(..).ok()` on
  overflow. Pinned by `a_position_beyond_the_blocks_own_line_count_is_dropped_not_clamped`.
- **D-05 — the real foreign file is read by an `#[ignore]`d, env-var-addressed, READ-ONLY test.**
  `GSD_META_MANAGER_REAL_STATE_MD`; no write, no create, no committed absolute path; returns early
  when unset.

An additional implementation-level inference, also unapproved: a repair candidate's trailing
` # comment`, if any, is folded INTO the quoted value rather than preserved as a comment. The line
was unparseable before the repair so no prior semantics are lost, but a reader could see a value
one comment longer than they wrote. Left out of scope deliberately — see below.

## Deliberately Out of Scope

- **No second repair strategy and no retry loop.** One candidate rule, one reparse. Anything the
  rule does not understand falls through to `Unreadable` unchanged.
- **Indented keys are never repaired.** A leading-whitespace line may be the continuation of a
  multi-line quoted scalar, and rewriting one would corrupt a valid value.
- **Comment stripping inside a repaired value** (above) is not implemented.
- **`FrontmatterFault` gained no variants** — a repaired-then-still-broken file is still
  `InvalidYaml`, now with a position.
- **The two pre-existing/flaky test failures were not chased** (git-version schedule test;
  `Text file busy` parallel flake).
- **`Cargo.toml` / `Cargo.lock` untouched** — this plan added no dependency (T-UEJ-SC accepted).

## Threat Model Outcome

| Threat | Disposition | Where it landed |
|--------|-------------|-----------------|
| T-UEJ-01 information disclosure | mitigated | Only `u32` line/column cross the boundary; the parser message stays in the single `tracing::warn!`. `an_unreadable_state_md_with_a_located_fault_names_its_file_line` asserts the cell carries no parser vocabulary. |
| T-UEJ-02 denial of service | mitigated | All repair/escape work is over `char`s; `a_multibyte_value_survives_the_repair_without_a_byte_index_panic` asserts the naive byte index is NOT a char boundary before asserting the value survives. Position arithmetic is checked and drops on overflow. |
| T-UEJ-03 tampering | mitigated | Repair is in-memory only, runs only in the parse-error branch, and `the_repair_is_a_no_op_on_every_valid_frontmatter_this_module_knows` proves `None` (byte-identical) on every valid fixture including this repo's own STATE.md. |
| T-UEJ-04 spoofing | mitigated | Distinct variant + distinct field + cell marker + detail line; `recovered_and_unreadable_are_never_both_set` pins exclusivity across all three outcomes. |
| T-UEJ-05 elevation of privilege | mitigated | New fields are `bool` and `Option<FrontmatterFaultPosition>`; `THIRD_PARTY_STRINGS` unchanged, `spawn_seam_guard` green. |
| T-UEJ-SC supply chain | accepted | No dependency added. |

## Known Stubs

None. No stub, placeholder, TODO/FIXME, or skipped test was introduced. The one `#[ignore]`d test
is an intentional opt-in real-file read (D-05) and was RUN, with its raw output quoted above.

## Self-Check: PASSED

- `src/state_reader/state_md.rs` — FOUND
- `src/state_reader/mod.rs` — FOUND
- `src/app.rs` — FOUND
- `src/ui/screens/detail.rs` — FOUND
- `tests/state_reader_test.rs` — FOUND
- commit `109192f` — FOUND
- commit `4930e8a` — FOUND
- commit `6e1911e` — FOUND
