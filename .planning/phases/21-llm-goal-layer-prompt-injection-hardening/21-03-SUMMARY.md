---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 03
subsystem: config
tags: [sha256, digest, opt-in, disclosure, serde-flatten, spawn-gate, drift, sha2, residual-exposure]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    plan: "01"
    provides: "driver::untrusted's third-party string census (authoritative for which files feed the seams), plan_digest, the SpawnProfile seam"
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    plan: "02"
    provides: "the fifth sibling taxonomy; both waves explicitly deferred the sha256: upgrade here"
  - phase: 17
    provides: "DriverOptIn, record_opt_in as the single construction site, claude_md_digest"
  - phase: 19
    provides: "the PreToolUse envelope hook the gate must not disturb"
provides:
  - "journal::sha256_digest — a sha256:-prefixed sibling of argv_digest, never a replacement"
  - "config::PromptInput and DriverOptIn::prompt_inputs — the disclosed file set and its digests"
  - "config::DriverOptIn::extra and PromptInput::extra — the flatten that makes the record non-lossy across binary versions"
  - "registry::DISCLOSED_PROMPT_INPUTS, current_prompt_inputs, OptInDrift, check_prompt_input_drift"
  - "OptInError::PromptInputsDrifted — the third refusal at the spawn gate"
  - "The three pinned disclosure constants and render_disclosure in driver_confirm.rs"
affects: [21-04, 21-05, 21-06]

actuals:
  tokens: 17594
  tasks: 3
  commits: 3

tech-stack:
  added:
    - "sha2 0.11.0"
  patterns:
    - "Absent files recorded with a None digest rather than omitted, so appearance is drift"
    - "Legacy detection by 'prefix is not sha256:' rather than an exact legacy match"
    - "Profile attribution owned by code, the approved list owned by the record"
    - "Pinned honesty text whose paired test spells the stale claim out verbatim"

key-files:
  created: []
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/journal/mod.rs
    - src/config.rs
    - src/registry.rs
    - src/executor/mod.rs
    - src/error.rs
    - src/ui/screens/driver_confirm.rs
    - src/driver/dry_run.rs
    - src/driver/mod.rs
    - src/envelope/policy.rs

key-decisions:
  - "Option-a, plus #[serde(flatten)] extra on DriverOptIn — the flatten is what makes the plan's own round-trip acceptance criterion passable at all, because DriverOptIn carried no flatten and nested keys were silently deleted."
  - "Absent files are recorded with a None digest rather than omitted from the list. Omitting them would leave a CLAUDE.md that APPEARS after opt-in reaching the executor's prompt with nothing to contradict it."
  - "Legacy detection matches 'is not sha256:', never an exact fnv1a64:, because the tree's own pre-Phase-19 fixture carries fnv1a:."
  - "The disclosed set is five paths across two profiles, including BOTH HANDOFF spellings — state_reader reads either, and naming one would be under-broad by exactly one file."
  - "The drift check went to DrivableProject::from_registry, not registry.rs, which documents itself as not being the gate."
  - "sha2 0.11.0's comment states a MEASURED graph delta rather than reusing rustix's 'zero new compilation units' sentence, which is false here."

requirements-completed: [SAFE-07, DRIVE-03]

coverage:
  - id: F1
    description: "The digest backing the re-confirmation is SHA-256 behind a sha256: prefix; a legacy-prefixed record re-confirms rather than comparing across hash families, with no migration of any user's config.json"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#sha256_digest_is_prefixed_and_is_sixty_four_lowercase_hex_digits"
        status: pass
      - kind: unit
        ref: "src/config.rs#a_pre_phase_21_opt_in_loads_with_no_migration_and_reads_as_re_confirm"
        status: pass
      - kind: unit
        ref: "src/executor/mod.rs#from_registry_refuses_a_legacy_digest_even_when_the_file_is_unchanged"
        status: pass
    human_judgment: false
  - id: F2
    description: "Opting in lists exactly which files' bytes can reach a prompt, distinguishing the seam profile from the executor profile"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#the_disclosure_renders_every_recorded_path_and_digest"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#the_disclosure_names_the_file_set_the_seams_and_what_is_not_closed"
        status: pass
    human_judgment: true
    rationale: "The list's MEMBERSHIP is a correctness property that no test can establish — a test can only check the list matches DISCLOSED_PROMPT_INPUTS, not that DISCLOSED_PROMPT_INPUTS matches reality. Wave 1's census (src/driver/untrusted.rs:125-126) is the evidence for STATE.md/ROADMAP.md/HANDOFF, and it is prose. A human should confirm no sixth file feeds a prompt."
  - id: F3
    description: "A digest that no longer matches re-confirms at the gate where the opt-in is consulted before a spawn, not at render time"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/executor/mod.rs#from_registry_refuses_when_a_disclosed_file_changed_after_the_opt_in"
        status: pass
      - kind: unit
        ref: "src/executor/mod.rs#from_registry_refuses_a_file_that_appeared_after_the_opt_in"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#the_rendered_list_is_the_recorded_one_not_a_recomputed_set"
        status: pass
    human_judgment: false
  - id: F4
    description: "The residual exposure is stated in pinned user-facing text: the executor profile still loads CLAUDE.md, and the seam-side suppression is not a verified control"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#the_residual_block_says_the_executor_still_loads_claude_md_and_is_not_verified"
        status: pass
    human_judgment: false
  - id: F5
    description: "An absent, unparseable or unknown-family prompt-input entry means the same as a mismatched one — re-confirm"
    verification:
      - kind: unit
        ref: "src/config.rs#a_digest_from_an_unknown_hash_family_reads_as_re_confirm"
        status: pass
      - kind: unit
        ref: "src/config.rs#an_unknown_key_nested_inside_the_opt_in_survives_a_load_and_save"
        status: pass
    human_judgment: false

duration: 95min
completed: 2026-08-19
status: complete
---

# Phase 21 Plan 03: The SHA-256 Upgrade and the Disclosure Surface Summary

**The opt-in's re-confirmation promise is now true: a SHA-256 digest an adversary cannot forge, a
disclosure naming all five files whose bytes can reach a prompt across both spawn profiles, a
residual-exposure block that says plainly what this phase does NOT close, and a drift check at the
spawn gate — observed letting the run start when performed at render time instead.**

## Performance

- **Duration:** ~95 min (including the checkpoint halt and its investigation)
- **Tasks:** 3 of 3
- **Files modified:** 11 source + 11 test files
- **Net:** +1,104 / −39 lines
- **Lib tests:** 977 → 991 (**14 new**)

## Task Commits

1. **Task 1: checkpoint decision** — halted, not decided. Findings committed as `f9adb6e`.
2. **Task 2: sha2, the sha256: sibling, and the doc text it falsified** — `ce9bc2e` (feat)
3. **Task 3: the disclosure surface and the drift check at the gate** — `cd87227` (feat)

## The resolved `sha2`, and the graph claim that had to be rewritten

```
$ cargo add sha2 --dry-run
      Adding sha2 v0.11.0 to dependencies
             Features: + alloc  + oid  - zeroize
```

**Resolved version: 0.11.0.** CONTEXT.md's C-4 was right that research's `0.10.9` was stale as
*latest*. Seven packages newly locked: `sha2 0.11.0`, `digest 0.11.3`, `block-buffer 0.12.1`,
`crypto-common 0.2.2`, `hybrid-array 0.4.14`, `const-oid 0.10.2`, `cpufeatures 0.3.0`.

```
$ cargo tree -p sha2@0.11.0 --depth 1
sha2 v0.11.0
├── cfg-if v1.0.4
├── cpufeatures v0.3.0
└── digest v0.11.3

$ cargo tree -i sha2@0.11.0        →  sha2 v0.11.0 └── gsd-meta-manager v1.6.0
$ cargo tree -i sha2@0.10.9        →  warning: nothing to print.
```

The `-p sha2` invocation was itself **ambiguous** (`sha2@0.10.9` / `sha2@0.11.0`), which is the
tidiest possible confirmation of the finding raised at the checkpoint: a `sha2 0.10.9` entry does
sit in `Cargo.lock`, but it is an **orphan** from an unenabled ratatui backend feature, not a
compiled unit. The `rustix` precedent ("the exact version already present … adds zero new
compilation units") therefore does **not** apply, and the manifest comment says so explicitly
rather than borrowing a justification that happens to be false here.

## Guards, and what each did against the UNFIXED behaviour

Every guard was observed red before being trusted. Each defect was injected, observed, and reverted
from an explicit per-file backup copy in the scratchpad — never `git clean`, `git stash` or a
blanket working-tree reset, per the worktree prohibition. Both restorations were verified with
`diff` against the backup and reported *Files are identical*.

| Guard | Injected defect | Observed |
|---|---|---|
| `an_unknown_key_nested_inside_the_opt_in_survives_a_load_and_save` | `#[serde(flatten)]` → `#[serde(skip)]` on `DriverOptIn::extra` | **FAILED** — `driver_attestation_v3` absent from the saved file; the nested key was deleted while the entry-level one survived |
| `a_digest_from_an_unknown_hash_family_reads_as_re_confirm` | exact `starts_with("fnv1a64:")` instead of `!starts_with("sha256:")` | **FAILED** on the real `fnv1a:` value — `Got: Some(Changed { path: "CLAUDE.md" })` instead of `LegacyDigest` |
| `a_pre_phase_21_opt_in_loads_with_no_migration_and_reads_as_re_confirm` | empty `prompt_inputs` returns `None` (no drift) | **FAILED** — `left: None, right: Some(NothingDisclosed)`; a pre-disclosure opt-in silently passed the gate |
| `from_registry_refuses_when_a_disclosed_file_changed_after_the_opt_in` | drift checked only at render time (gate call removed) | **FAILED** — returned `DrivableProject { alias: "demo", … }`: **the run would have started** on rewritten bytes |
| `from_registry_refuses_a_legacy_digest_even_when_the_file_is_unchanged` | same injection | **FAILED** — returned a token; the SHA-256 upgrade was defeatable by leaving an old record in place |
| `from_registry_refuses_a_file_that_appeared_after_the_opt_in` | same injection | **FAILED** — returned a token; a `CLAUDE.md` planted after opt-in reached the executor with nothing approved |
| `the_residual_block_says_the_executor_still_loads_claude_md_and_is_not_verified` | constant replaced with the weaker predecessor wording ("*never reaches the model … the boundary is complete*") | **FAILED** — "the residual block must name the profile that is still exposed" |
| `sha256_digest_is_prefixed_and_is_sixty_four_lowercase_hex_digits` | — (known-vector arm) | pins the published `abc` vector, so a swap to a *different* hash fails even though the function stays stable against itself |
| `the_two_digest_functions_cannot_be_confused_for_the_same_input` | — (control arm) | the same input through both functions yields different values and different prefixes |
| `the_rendered_list_is_the_recorded_one_not_a_recomputed_set` | — (control arm) | carries its own precondition assertion that the on-disk set really changed, so the equality is not vacuous |

**On the honesty of the legacy guard.** The explicit `LegacyDigest` arm is defence in depth and
correct *reason reporting*, not the sole control: a legacy string can never compare equal to a
freshly computed `sha256:` one, so plain byte comparison already re-confirms. The injected defect
therefore degraded the *classification* (`Changed` instead of `LegacyDigest`) rather than opening a
hole. Stated plainly because a guard described as load-bearing when it is not is exactly the thing
this phase's conventions exist to prevent.

## The checkpoint, and what the tree said

Task 1 was a blocking `checkpoint:decision` and the plan's first task, so it was halted rather than
decided. Investigation before presenting it found the recommended option's justification was wrong
about this tree — and the coordinator adopted the correction.

**Option A's stated pro was false as written.** It claimed safety because "`RegisteredProject::extra`'s
flatten preserves unknown fields". That flatten exists, and a second is on `Preferences`, but
**`DriverOptIn` had neither** — `prompt_inputs` sits one level *deeper* than the protection meant to
cover it. Proven with a throwaway probe against the real `load_config`/`save_config`:

```
entry-level unknown survived: true
NESTED unknown survived:      false
```

Two consequences, recorded here at the coordinator's request so a later reader does not re-derive
them:

1. **The plan's own Task 2 acceptance criterion could not pass against the tree.** It requires a
   test that a config carrying an unknown key *inside `DriverOptIn`* round-trips. **T-21-20's stated
   mitigation — "the existing flatten-preservation posture" — was a paper mitigation at this
   nesting level.** Adding the flatten is what makes the plan's existing safety claim true rather
   than merely asserted; it is not a scope addition.
2. **The failure mode was fail-safe, not a vulnerability.** A deleted `prompt_inputs` reads as
   absent, and absent re-confirms — so the cost was a spurious re-confirmation after a version
   downgrade, never a silent approval. That is the difference between an annoyance and a hole, and
   it is why this was fixed on durability grounds rather than treated as a security defect.

Options B and C were each foreclosed by the plan's own `must_haves` (B contradicts truth[0]'s "no
migration"; C contradicts the `artifacts` entry and both `key_links`), so the checkpoint was a
confirm-with-correction rather than a live three-way fork.

## The three confirmed corrections

1. **Five paths, not two.** `src/driver/untrusted.rs:125-126` is authoritative: `ProjectState` comes
   from `STATE.md` **and `HANDOFF`**. `DISCLOSED_PROMPT_INPUTS` names `CLAUDE.md` (executor),
   `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/HANDOFF.json` and `.planning/HANDOFF.md`
   (model seam). **Both HANDOFF spellings** are listed because `state_reader` reads either, and
   naming one would be under-broad by exactly one file.
2. **The gate is `DrivableProject::from_registry`** (`src/executor/mod.rs`), not `src/registry.rs`,
   which says of itself: *"**Not a gate**: the gate is `DrivableProject::from_registry`"*. The check
   is ordered *after* `RootUnusable`, so a project that moved reports having moved rather than
   reporting drift.
3. **Legacy detection is "prefix is not `sha256:`".** The red-check above shows exactly why: the
   tree's own `PRE_PHASE_19_CONFIG` carries `fnv1a:`, which an exact `fnv1a64:` match misclassifies.

## Decisions Made

1. **Absent files are recorded with a `None` digest rather than omitted.** Had absent files simply
   been left out, a `CLAUDE.md` appearing *after* opt-in would reach the executor's prompt with no
   digest to contradict it and no re-confirmation. Recorded as `None`, its appearance is a mismatch
   like any other. `from_registry_refuses_a_file_that_appeared_after_the_opt_in` is the guard.
2. **Profile attribution lives in code; the approved list lives in the record.** Which files a build
   feeds to a model is a fact about the *build*, so storing it in a user-owned file would let a
   stale record disagree with the binary about what it does. The *list* is recorded — so the
   disclosure rendered is the one approved — while the `[executor]` / `[model seam]` label is looked
   up from `DISCLOSED_PROMPT_INPUTS`. A recorded path this build no longer reads renders as "not
   read by this build" rather than being hidden.
3. **`PromptInput` got the flatten too.** The same hazard, one level deeper, in the same user-owned
   version-shared file. Omitting it would have recreated the exact bug the finding is about.
4. **`record_opt_in` writes `claude_md_digest: None`.** Nothing writes the legacy key any more. It
   is *kept on the struct* purely so an older binary's load-and-save still finds the field it
   expects — which is what makes the upgrade migration-free.
5. **`argv_digest` was left as FNV-1a deliberately.** Its "not a security control" bullet is true of
   it and was not edited to cover both functions. Only its *second* bullet — the one asserting no
   hashing crate existed and none was warranted — was falsified, and it was rewritten in the same
   commit per the `dry_run.rs:78-83` precedent. A command-line fingerprint has no adversary, and
   switching it would churn every `fnv1a64:` value already in a `run.json` for no gain.
6. **`record_opt_in` remains the only `DriverOptIn` construction site outside tests** — verified
   mechanically: every other site failed to compile only under `cargo test --no-run`, which is what
   proves they are all `#[cfg(test)]`.

## Deviations from Plan

### 1. [Rule 3 — Blocking] `sha2` 0.11 dropped the `LowerHex` impl 0.10 had

- **Found during:** Task 2, first compile of `sha256_digest`.
- **Issue:** `format!("sha256:{:x}", hasher.finalize())` fails — 0.11 returns a
  `hybrid_array::Array`, which unlike 0.10's `GenericArray` does not implement `LowerHex`.
- **Fix:** one `write!` per byte, written out by hand with the reason recorded next to it. **No
  `hex` crate was added** — an eighth transitive package for sixteen characters of formatting would
  not survive the manifest's own comment convention.
- **Committed in:** `ce9bc2e`.

### 2. [Rule 1 — Bug] An existing test pinned the behaviour this plan changed

- **Found during:** Task 2 verification.
- **Issue:** `registry::tests::record_opt_in_stamps_a_record_with_a_second_precision_timestamp`
  asserted `claude_md_digest` was `Some` with an `fnv1a64:` prefix — true before this plan, false
  after.
- **Fix:** rewritten **in the commit that changed the behaviour it pinned**, per the same
  `dry_run.rs:78-83` precedent that governs the doc text. It now asserts the legacy key is `None`,
  that `prompt_inputs` carries a `sha256:` digest for `CLAUDE.md`, and that the whole disclosed set
  is recorded rather than only the files that exist.
- **Committed in:** `ce9bc2e`.
- **Recorded rather than quietly updated**, because silently rewriting a test that fails after a
  behaviour change is indistinguishable from moving a goalpost.

### 3. [Planned scope] 11 test files threaded the two new fields

`DriverOptIn` gained two required fields, so every construction site broke — **which is the
mechanism**, exactly as `config.rs:41-45` describes. The three root-backed in-crate fixtures and all
11 integration fixtures now record `current_prompt_inputs(root)`, so they represent genuine opt-ins
that the new gate accepts. `envelope/policy.rs::bare_opt_in` deliberately keeps an empty list: it is
the pre-Phase-19 shape and its tests are about envelope field resolution.

---

**Total deviations:** 1 blocking (Rule 3), 1 auto-fixed (Rule 1), 1 in-scope mechanical pass.
**Impact:** no scope creep, no test weakened, no shipped behaviour removed.

## Issues Encountered

- **`rtk` filtering produced two near-misses worth recording.** `cargo test` was reduced to a
  one-line summary that reported `1 passed` for a probe whose entire purpose was its printed
  output — a vacuous-looking pass. Separately, `git diff | wc -c` returned 26,049 through the
  filter versus **70,377** raw, which would have understated the `actuals` figure below by 2.7×.
  Both were caught by re-running under `rtk proxy`.
- **`tests/driver_reattach.rs` — the recorded pre-existing intermittent.** It failed on one full-suite
  run, and because this plan *modified that file*, it was investigated rather than waved through:
  re-run three times it gave **ok / ok / FAILED**, and the failure is a spawn race
  (`observed.len() == 0`, failing in 0.53s against a 30s budget). A drift refusal is deterministic
  given fixed files and so cannot pass 2-of-3; the fixture also creates `.planning/` *before* the
  config, leaving every disclosed file absent-and-recorded-absent. Not caused by this plan, not
  touched, not investigated further.
- **`cargo fmt --check` is not clean at baseline** (pre-existing, noted by 21-02). No global
  `cargo fmt` was run.

## Known Stubs

None. Every symbol this plan created is implemented, reachable and exercised.

Four honest limits, recorded because they are limits rather than stubs:

1. **`CLAUDE.md` suppression still has NO behavioural proof.** No field on this transport reports
   whether it took effect. This plan does not add any — it *discloses the absence* in the pinned
   residual-exposure text ("not a verified control") and pins that wording with a test. The
   behavioural proof remains **21-05's** corpus. Nothing here claims SAFE-07's suppression control
   is proven.
2. **The executor profile does not suppress `CLAUDE.md` at all.** Strictly larger exposure than the
   two seams, deliberately out of scope (research Q2), and now disclosed loudly in text the user
   reads before opting in rather than fixed quietly or omitted.
3. **`goal::plan_digest` is still FNV-1a.** This plan upgraded the *opt-in* digest, which is what
   C-4 and its `must_haves` name. Research Q4 asks that approval bind to a digest of the plan **and**
   the disclosed files, re-checked at spawn; the **files** half is now enforced at the gate, but the
   **plan** half still rests on `plan_digest`, which remains drift detection rather than a security
   control. Binding the two together belongs with the loop wiring that owns an approved plan
   (21-04/21-06). Recorded explicitly so Q4 is not read as fully discharged.
4. **The disclosed set's MEMBERSHIP cannot be proven by a test** — see coverage item F2. A test can
   only check the rendered list matches `DISCLOSED_PROMPT_INPUTS`; that the constant matches reality
   rests on wave 1's prose census. A sixth file feeding a prompt would make the disclosure
   under-broad with every test still green.

## Threat Flags

None. No new network endpoint, auth path or schema change at a trust boundary. The one new file-access
pattern — reading five project files to digest them — is read-only, bounded to a fixed constant list,
and is itself the mitigation for T-21-16/17/18.

## Next Phase Readiness

- **For 21-05:** `render_disclosure`'s residual-exposure block is the text the corpus work will make
  either provable or falsifiable. If 21-05 proves suppression behaviourally, `SECTION_RESIDUAL_EXPOSURE`
  becomes partly false and must be rewritten **in that commit**, and its paired test's verbatim
  stale-wording list updated with it.
- **For 21-04/21-06:** `OptInError::PromptInputsDrifted` is a new refusal at the gate. Any surface
  reporting spawn failures should render `OptInDrift::describe()` rather than inventing wording.
- **If a sixth prompt-input file is ever read:** add it to `DISCLOSED_PROMPT_INPUTS` in the same
  commit as the code that reads it, or the disclosure becomes under-broad while every test stays green.

## Self-Check: PASSED

- Commits claimed, verified in `git log`: `f9adb6e`, `ce9bc2e`, `cd87227`.
- `cargo build`: clean. `cargo test`: 31 suites ok, 0 failures (with the recorded `driver_reattach`
  intermittent green).
- `cargo test --lib`: **991 passed**, 0 failed — 14 new.
- `cargo clippy -- -D warnings`: clean.
- `cargo clippy --all-targets -- -D warnings`: exactly the 5 known pre-existing lints, in their
  recorded locations (`browser.rs:131/132/133`, `project_creator.rs:146`, `state_reader/mod.rs:311`).
- `grep -ci 'no hashing crate is present' src/journal/mod.rs`: **0** — the falsified bullet was
  rewritten, not left.
- `grep -c 'for_testing' src/registry.rs`: 0, unchanged.
- Probe file `tests/zz_scratch_optin_flatten_probe.rs`: created, run, deleted; confirmed absent.
- Both injected-defect restorations verified byte-identical to their backups via `diff`.
- STATE.md, ROADMAP.md and WINDOWS.md: untouched, per the orchestrator's instruction.
- `actuals.tokens` = 17,594 = 70,377 raw diff chars / 4, measured under `rtk proxy`. Well under the
  plan's 64,000 estimate; recorded as measured rather than adjusted toward it.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-19*
