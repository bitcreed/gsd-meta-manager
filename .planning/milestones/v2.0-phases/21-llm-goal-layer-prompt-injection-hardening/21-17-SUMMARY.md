---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 17
subsystem: driver/registry/envelope identity
tags: [identity, unicode, newtype, gap-closure, round-6]
requires: []
provides:
  - "text::carries_invisible_formatting + is_invisible_formatting_char (one class spelling, two judgments)"
  - "test_support::LOOK_ALIKE_PAIRS; pub mod test_support unconditional"
  - "is_plain_path_component's embedded-invisible-formatting clause (D-17-1)"
  - "registry::Alias + registry::AliasRefusal"
  - "DriveError::AliasNotVisible"
  - "guard ten: the argv-alias census"
affects:
  - src/text.rs
  - src/test_support.rs
  - src/lib.rs
  - src/journal/mod.rs
  - src/registry.rs
  - src/error.rs
  - src/main.rs
  - src/app.rs
  - src/ui/screens/add_project.rs
  - src/driver/mod.rs
  - src/driver/goal.rs
  - src/envelope/mod.rs
  - src/envelope/hooks.rs
  - src/envelope/cred.rs
  - tests/spawn_seam_guard.rs
  - tests/registry_test.rs
  - tests/driver_optin.rs
  - tests/driver_goal_seam.rs
tech-stack:
  added: []
  patterns:
    - "newtype with private field + single fallible constructor delegating to shared predicates"
    - "fixture-shape const consumed at every seam rather than hand-copied"
    - "planted-defect control consuming the same extracted fn as the live assertion"
key-files:
  created: []
  modified:
    - src/text.rs
    - src/test_support.rs
    - src/lib.rs
    - src/journal/mod.rs
    - src/registry.rs
    - src/error.rs
    - src/main.rs
    - src/app.rs
    - src/ui/screens/add_project.rs
    - src/driver/mod.rs
    - src/driver/goal.rs
    - src/envelope/mod.rs
    - src/envelope/hooks.rs
    - src/envelope/cred.rs
    - tests/spawn_seam_guard.rs
    - tests/registry_test.rs
    - tests/driver_optin.rs
    - tests/driver_goal_seam.rs
key-decisions:
  - "D-17-1 is_plain_path_component refuses embedded invisible formatting"
  - "D-17-2 registration takes registry::Alias"
  - "D-17-3 remove_project stays raw — the recovery path"
  - "D-17-4 DriveError::AliasNotVisible replaces the borrowed UnknownAlias"
  - "D-17-5 pub mod test_support loses its cfg(test) gate"
  - "D-17-6 envelope_dir_in keeps &str + Option"
  - "D-17-7 DriveArgs.alias stays payload::NonBlank"
requirements-completed: []
requirements: [DRIVE-01, DRIVE-03, SAFE-08]
duration: "~1h"
completed: 2026-08-22
coverage:
  - deliverable: "One class spelling, two judgments (emptiness + identity)"
    verification:
      - kind: test
        ref: "src/text.rs#only_a_value_carrying_a_character_that_renders_as_nothing_is_look_alike"
        status: pass
      - kind: test
        ref: "src/text.rs#the_two_judgments_agree_on_visibility_and_disagree_on_identity"
        status: pass
      - kind: command
        ref: "rtk proxy grep -rln \"2060\" src/ --include=*.rs -> src/text.rs, src/driver/mod.rs"
        status: pass
    human_judgment: false
  - deliverable: "A look-alike identity never resolves beside its visible twin"
    verification:
      - kind: test
        ref: "src/journal/mod.rs#a_look_alike_identity_never_resolves_beside_its_visible_twin"
        status: pass
      - kind: test
        ref: "src/envelope/mod.rs#a_hostile_alias_yields_no_envelope_directory_at_all"
        status: pass
    human_judgment: false
  - deliverable: "Registration closed at the entry via registry::Alias"
    verification:
      - kind: test
        ref: "src/registry.rs#registering_a_look_alike_beside_its_visible_twin_is_refused"
        status: pass
      - kind: test
        ref: "src/registry.rs#every_constructible_alias_can_name_its_own_envelope_root"
        status: pass
      - kind: test
        ref: "tests/registry_test.rs#a_look_alike_alias_cannot_join_its_visible_twin_in_the_registry"
        status: pass
      - kind: command
        ref: "target/debug/gsd-meta-manager add <path> <demo+U+200B> -> exit 1, no config written"
        status: pass
    human_judgment: false
  - deliverable: "Every argv alias entry point is classified (guard ten)"
    verification:
      - kind: test
        ref: "tests/spawn_seam_guard.rs#every_argv_alias_field_is_classified"
        status: pass
      - kind: test
        ref: "tests/spawn_seam_guard.rs#the_alias_census_reports_a_planted_ninth_field"
        status: pass
    human_judgment: false
  - deliverable: "WR-06 corrected: AliasNotVisible, not a borrowed UnknownAlias"
    verification:
      - kind: test
        ref: "src/driver/mod.rs#every_argv_position_refuses_every_degenerate_payload_at_the_parse_boundary"
        status: pass
    human_judgment: false
  - deliverable: "SAFE-08: the phase token's look-alike safety is a property, not a precondition"
    verification:
      - kind: test
        ref: "src/driver/goal.rs#a_look_alike_phase_token_is_refused_even_though_its_visible_twin_is_declared"
        status: pass
      - kind: test
        ref: "tests/driver_goal_seam.rs#a_phase_token_that_renders_like_a_declared_phase_is_refused_by_the_predicate"
        status: pass
    human_judgment: false
  - deliverable: "T-21-17-07: legacy entries fail closed with the recovery route named"
    verification:
      - kind: command
        ref: "envelope pre-push/pre-commit/askpass -> exit 1; guard -> exit 2; each message contains `remove`"
        status: pass
    human_judgment: false
  - deliverable: "Disclosed-not-closed: from_argv still accepts look-alikes by design"
    human_judgment: true
    rationale: "A design boundary, disclosed in truth 7 and in the disclosure list below; the identity half is fixture-pinned at each seam, but the acceptance itself is a judgment call the verifier should adjudicate rather than a behaviour a test can certify as correct."
---

# Phase 21 Plan 17: The Look-Alike Class — Identity Judged Where a Value Becomes an Identity

Closed pass-6 gap 1 by giving the tree the two things the verifier proved it lacked: a predicate
clause for "carries something different from what it renders", and a fixture shape that can
express it. Registration became a classified domain the way `DriveArgs` already was, and the
WR-06 falsehood at `from_argv` was corrected.

**Duration:** ~1h. **Tasks:** 4 (T1 tracer, T2a, T2b, T3). **Files:** 18 modified.
**Commits:** 5 (one red arm + four task commits).

| Task | Commit | What |
|---|---|---|
| T1 red arm | `fbdfd6d` | `LOOK_ALIKE_PAIRS` + the tracer, `#[ignore]`d, red output recorded |
| T1 fix | `88ae851` | the shared char class, `carries_invisible_formatting`, D-17-1's clause, cfg drop, SAFE-08 pins |
| T2a | `f46576f` | `registry::Alias`, `AliasRefusal`, the fourth predicate deleted, `AliasNotVisible` |
| T2b | `337f67a` | the four envelope re-entries over `&Alias`, failing closed with the recovery route |
| T3 | `298d921` | guard ten: the census + planted-ninth control |

> A third-party commit `ca1956b` (`docs(todo): …`, one file under `.gsd/`) landed on `master`
> between `88ae851` and `f46576f`. It was not made by this execution, touches no source, and
> overlaps nothing in this plan. Recorded so the log's shape is not read as a lost commit.

## Red-arm evidence (the tracer contract)

The red arm was written unignored, run against the unfixed tree, and the failure recorded
**verbatim** before any fix existed. Command:
`rtk proxy cargo test --lib a_look_alike_identity`

```text
running 1 test
test journal::tests::a_look_alike_identity_never_resolves_beside_its_visible_twin ... FAILED

---- journal::tests::a_look_alike_identity_never_resolves_beside_its_visible_twin stdout ----

thread 'journal::tests::a_look_alike_identity_never_resolves_beside_its_visible_twin' (123442) panicked at src/journal/mod.rs:2801:13:
"demo\u{200b}" renders exactly as "demo" and carries different bytes; accepting both is how two identities that no reader can distinguish both resolve

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1031 filtered out; finished in 0.00s
```

The test was then marked `#[ignore = "red: closes pass-6 gap 1; un-ignored in the fix commit"]`
with that output quoted in its doc comment, and **committed at `fbdfd6d` before `88ae851`
existed** — `git log` shows the ordering. The `#[ignore]` came off in the fix commit; the test
now passes un-ignored.

## Accomplishments

1. **The missing clause, not another consumer.** `src/text.rs` now spells the zero-width/format
   ranges **once**, in a private `is_invisible_formatting_char`, and two public judgments read
   it: `carries_visible_content` (emptiness) and the new `carries_invisible_formatting`
   (identity). The module doc states the split and why the emptiness judgment structurally could
   never close the look-alike harm.
2. **The missing fixture shape.** `test_support::LOOK_ALIKE_PAIRS` — three pairs that differ in
   bytes and agree on screen — consumed (never hand-spelled) at `is_plain_path_component`,
   `run_paths`, `envelope_dir_in`, `Alias::new`, and the registry integration pin.
3. **D-17-1 at the seam.** `is_plain_path_component` refuses embedded invisible formatting after
   its `is_control` refusal. That one clause closes the run-id, alias-at-seam, envelope,
   credential and phase-token halves together. All six existing acceptance pins re-assert
   unchanged — no conflict arose, so no pin was rewritten.
4. **`registry::Alias`.** Private field, one fallible constructor, four ordered clauses each
   delegating to a shared predicate. `add_project`/`add_project_unchecked` take `&Alias`; the
   `is_empty`/`is_whitespace` bails are deleted, not relocated.
5. **WR-06 corrected.** `DriveError::AliasNotVisible` replaces a borrowed `UnknownAlias` whose
   message asserted a registry fact the pure boundary never checked and pass 6 measured false.
6. **Fail-closed with a route out.** The four envelope re-entry arms judge their alias and refuse
   with their own exit code, each message naming `gsd-meta-manager remove <alias>`.
7. **Guard ten.** An 8-row classification census over `src/cli.rs`, exact-count, with a
   planted-ninth control on the same extracted fn — and additionally probed against the live
   region.

## Prohibition audit

Five prohibitions, each check a **mechanism** run against this plan's final tree. No row is a
grep for a sentence of prose.

| # | Prohibition | Mechanical check | Output | Verdict |
|---|---|---|---|---|
| 1 | No second production spelling of the ranges | `rtk proxy sh -c "grep -rn \"'\\u{200b}'\\.\\.=\" src/ --include=*.rs"` | `src/text.rs:76` (production, the one spelling) and `src/driver/mod.rs:2175` (the deliberately independent test-side oracle, inside the `#[cfg(test)]` module opening at :1413). Corroborating file census: `rtk proxy grep -rln "2060" src/ --include=*.rs` → exactly `src/text.rs`, `src/driver/mod.rs`. **Non-vacuity control:** `rtk proxy grep -c "2060" src/text.rs` → `4` (≥1). | PASS |
| 2 | Registration must not accept what the envelope seam refuses | Behavioral pins, not prose: `registry::tests::every_constructible_alias_can_name_its_own_envelope_root` drives each accepted shape through `Alias::new` **and** `envelope_dir_in`; `registering_a_look_alike_beside_its_visible_twin_is_refused` drives each refused shape through `Alias::new` (looping `LOOK_ALIKE_PAIRS` **and** `DEGENERATE`). | `test registry::tests::registering_a_look_alike_beside_its_visible_twin_is_refused ... ok`; `test registry::tests::every_constructible_alias_can_name_its_own_envelope_root ... ok` | PASS |
| 3 | No mechanism claim certified by a prose grep | Structural-claim inventory below — each claim's certifying control test named with its result. Every control plants a defect and consumes the same code path as the live assertion. | see the inventory table | PASS |
| 4 | No `.planning/REQUIREMENTS.md` flip | `rtk proxy git log --format="%h %s" 0352dda..HEAD -- .planning/REQUIREMENTS.md` | *(empty — no commit of this plan touches the file)* | PASS |
| 5 | No adjudicated-out item dropped silently | The disclosure list below carries D-17-3, D-17-6, truth 7, and T-21-17-07; each is also recorded in the code at its site (`main.rs`'s Remove arm, `envelope_dir_in`'s declaration, `from_argv`'s doc, `judged_alias_or_exit`'s doc). | present in code and in this SUMMARY | PASS |

### Structural-claim inventory (prohibition 3's evidence)

| Claim | Certifying control | Result |
|---|---|---|
| The identity clause refuses look-alikes at every value→identity seam | `journal::tests::a_look_alike_identity_never_resolves_beside_its_visible_twin` — **red-armed**, failure quoted above, committed at `fbdfd6d` before the fix | pass (un-ignored) |
| The two judgments differ where it matters | `text::tests::the_two_judgments_agree_on_visibility_and_disagree_on_identity` — asserts the pair AGREES under emptiness and DISAGREES under identity; a re-implementation of the identity judgment in terms of visibility fails it | pass |
| A look-alike alias cannot join its twin in the registry | `registry::tests::registering_a_look_alike_beside_its_visible_twin_is_refused` + `tests/registry_test.rs::a_look_alike_alias_cannot_join_its_visible_twin_in_the_registry` | pass |
| The census sees growth | `tests/spawn_seam_guard.rs::the_alias_census_reports_a_planted_ninth_field` (synthetic ninth → 9 via the same extracted fn) **plus** a live-region scratch probe: a ninth `alias: String,` appended to `src/cli.rs` turned `every_argv_alias_field_is_classified` red with `left: 9 / right: 8` and the classifier instruction; reverted, guard green | pass |
| The phase-token refusal is a property, not a roadmap coincidence | `goal::tests::a_look_alike_phase_token_is_refused_even_though_its_visible_twin_is_declared` — asserts `PHASES.contains("20")` first, so a membership-only build reads `PHASE_ABSENT_FROM_ROADMAP` and fails | pass |

## Named-shape audit

Rows 1-8 and 17-18 are this plan's. Rows 9-16 and 19 are 21-18's and are listed as delegated.
Row 20 is cited with its adjudicated state, not re-verified.

| # | Named shape | Owner | Final state |
|---|---|---|---|
| 1 | Wholly-invisible payloads (DEGENERATE) | held | **Unchanged and still green** — 7×6 matrix and journal pins pass (`every_argv_position_refuses_every_degenerate_payload_at_the_parse_boundary` ok). |
| 2 | Look-alike alias → two envelope/credential roots | 21-17 T1/T2a | **CLOSED.** `a_look_alike_identity_never_resolves_beside_its_visible_twin` (envelope half asserted), `a_hostile_alias_yields_no_envelope_directory_at_all` (LOOK_ALIKE_PAIRS both directions), `registering_a_look_alike_beside_its_visible_twin_is_refused`. |
| 3 | Look-alike `--run-id` → two run directories | 21-17 T1 | **CLOSED.** Same tracer asserts `run_paths(planning, "2026-08-19T12-00-00Z-aaaa\u{200b}")` is `None` and the visible twin still resolves. |
| 4 | Fourth registry predicate on `add` | 21-17 T2a | **DELETED.** `sed -n '/pub fn add_project/,/^}/p' src/registry.rs \| grep -c 'is_empty\|is_whitespace'` → `0`; non-vacuity `grep -c 'Alias'` on the same extraction → `4`. The only surviving `is_whitespace` in the file is `Alias::new`'s delegating clause 3. |
| 5 | Look-alike phase token held only by roadmap membership | 21-17 T1 | **CLOSED.** Two pins, both asserting the visible member IS declared first. |
| 6 | WR-06 `UnknownAlias` falsehood | 21-17 T2a | **CORRECTED.** `DriveError::AliasNotVisible`; the matrix's `--alias` row asserts it; the false sentence in `from_argv`'s doc is replaced with the correction, riding the commit that falsifies it. |
| 7 | Ninth subcommand adding an alias field unclassified | 21-17 T3 | **CLOSED to the census's bound.** Guard ten + planted-ninth control + live probe. **Residual DISCLOSED:** the judge column is hand-chosen prose (limit 2 in the guard header); the census forces classification, it cannot verify the judge. |
| 8 | `from_argv` accepts look-alikes in every position | 21-17 | **DISCLOSED, not closed — by design** (truth 7, D-17-7). `from_argv` judges visibility; free-text fields legitimately carry ZWJ/ZWNJ. Identity is judged where a value becomes an identity, and each of those seams is now fixture-pinned. A look-alike `--run-id` therefore passes `from_argv` and is refused by `drive` at the seam before anything is created. |
| 9 | guard nine silent spellings | 21-18 | **DELEGATED** — 21-18 Task 1. |
| 10 | `one_of_each` false compile claim | 21-18 | **DELEGATED** — 21-18 Task 2(a). |
| 11 | acceptance-matrix hand-exemption | 21-18 | **DELEGATED** — 21-18 Task 2(b). |
| 12 | tests/ hand-copied DEGENERATE subsets | 21-18 | **DELEGATED** — 21-18 Task 2(c). *(21-17 supplied the enabler: D-17-5 dropped the cfg gate, and `tests/registry_test.rs` already consumes `gsd_meta_manager::test_support::LOOK_ALIKE_PAIRS` as proof the const is reachable from an integration crate.)* |
| 13 | CR-01 tracer blind to the envelope half | 21-18 | **DELEGATED** — 21-18 Task 3(a). |
| 14 | 21-16 truths 1/4 false in the record | 21-18 | **DELEGATED** — 21-18 Task 3(c). |
| 15 | Thirteen-fixture-file coincidental compile bound | 21-18 | **DELEGATED** — 21-18 Task 1. |
| 16 | 21-16-SUMMARY's unmeasured-bound certification | 21-18 | **DELEGATED** — 21-18 Task 3(c)(3). |
| 17 | `remove_project` stays raw | 21-17 T2a/T2b | **DISCLOSED DEVIATION D-17-3.** Recorded at the arm (`src/main.rs`, full reason), in guard ten's table (`BY DECISION D-17-3`), and in the disclosure list below. |
| 18 | `envelope_dir_in` keeps `&str` + `Option` | 21-17 T2b | **DISCLOSED DEVIATION D-17-6.** Recorded in a comment at the declaration naming the deviation and both reasons. |
| 19 | IN-02/IN-03 guard-eight numbering and lost line continuations | 21-18 | **DELEGATED** — 21-18 Task 2(d). |
| 20 | IN-04 `load_config` before `from_argv` | none | **ADJUDICATED pre-existing / out of scope** per review G.4 ("both claims verified"): the ordering predates this phase, the consequence is message ordering only, neither path writes, and the pass-6 verifier carried it into neither gaps nor missing. Cited, not re-verified. |

## Disclosure list

Four things this plan deliberately did **not** close, each recorded in the code at its site as
well as here:

1. **D-17-3 — `remove_project` stays raw.** A deviation from the verifier's newtype list.
   Removal creates nothing, is membership-checked, and is the recovery path for the very entries
   this plan's registration refusals orphan. A removal that could not name what an older build
   registered would make a bad entry permanent — itself a WR-06-class falsehood generator.
   Recorded at `src/main.rs`'s Remove arm and in guard ten's table.
2. **D-17-6 — `envelope_dir_in` keeps `&str` + `Option`.** The second deviation from that list.
   The `Option` **is** the WR-02 caller-enumeration mechanism over seven internal callers, and
   D-17-1's clause already closes the harm at this seam. Replacing a deliberately fallible seam
   with an infallible newtype-taking one would delete a defence layer to add a redundant one.
   Recorded in a comment at the declaration.
3. **Truth 7 / D-17-7 — `from_argv` still accepts look-alikes, in every position.** `NonBlank`
   judges visibility; `--goal` and `--command` legitimately carry format characters. Identity is
   judged at the seams. Disclosed rather than silently relied on.
4. **T-21-17-07 — legacy entries and run directories now fail closed.** Pass 6 measured that an
   older build DID register invisible and look-alike aliases. After this plan those rows stay in
   `config.json` and `list` still renders them, but `envelope_dir_in` returns `None`,
   `Alias::new` refuses them everywhere, the hook re-entry arms exit 1 and the guard exits 2, and
   an existing run directory whose name carries a format character becomes unreadable at
   `journal/mod.rs:329` and `writer.rs:491`. **Accepted with a named route out**, and the route
   is in the message rather than only in this document:

   ```
   $ gsd-meta-manager envelope pre-push <demo+U+200B> --hook-path /nonexistent
   Error: the alias "demo\u{200b}" carries a character that renders as nothing, so on screen it
   is indistinguishable from an alias that does not. …
   This alias was accepted by an older build and no longer names a valid identity, so this hook
   refuses rather than guessing which project it means. To recover: `gsd-meta-manager remove
   <alias>` (removal still accepts it), then re-add the project under a visible alias.
   exit=1
   ```

   Measured for all four arms: `pre-push` exit 1, `pre-commit` exit 1, `askpass` exit 1 (no
   credential emitted), `guard` exit 2. Each message contains `remove`.

## Behavioral evidence (plan verification step 3)

Against the built binary and a scratch config:

```
$ gsd-meta-manager --config <scratch> add <fixture-path> <demo+U+200B>
Error: the alias "demo\u{200b}" carries a character that renders as nothing, so on screen it is
indistinguishable from an alias that does not. Two aliases that render identically would name two
different projects, two separate driver opt-ins and two separate envelope roots — and nothing in
the interface would show you which one you were acting on. Choose an alias whose written form is
what you see
Provide an explicit alias: gsd-meta-manager add <fixture-path> <alias>
exit=1
$ cat <scratch>
(no config file written)

$ gsd-meta-manager --config <scratch> add <fixture-path> demo
Added project 'demo' at <fixture-path>
exit=0
$ cat <scratch>
{ "version": 2, "projects": { "demo": { … } }, … }     # exactly one entry
```

(a) `add_project("demo")` + look-alike → **one** project, second refused with the why-message ✓
(b) `is_plain_path_component("demo\u{200b}")` → false, `envelope_dir_in` → None ✓ (tracer)
(c) `run_paths` with a look-alike run id → None ✓ (tracer)
(d) the goal-seam look-alike token refused with the visible member declared ✓ (two pins)

## Criterion-1 shape: unchanged

The one thing this plan was forbidden to regress.

- `from_argv` still pure (no file, no process), still above `drive`, still called from exactly
  one place in `main.rs`.
- The destructure is still `..`-free: `rtk proxy grep -A16 "let RawDriveArgs" src/driver/mod.rs`
  shows all twelve fields and no rest pattern.
- The six `NonBlank` fields are untouched; `DriveArgs.alias` stays `payload::NonBlank` (D-17-7).
- The diff at `from_argv` is **exactly one arm** — the alias refusal's variant. Same site, same
  purity, same ordering.
- `rtk proxy cargo test --lib every_argv_position` → 2 passed, 0 failed.

## Deviations from Plan

**[Rule 1 — plan location wrong against live code] `HOSTILE_PHASE_TOKENS` is not in `src/driver/goal.rs`.**
Found during: Task 1(d). The plan's `read_first` cited `src/driver/goal.rs:1080-1135` for
`HOSTILE_PHASE_TOKENS`; the const actually lives at `tests/driver_goal_seam.rs:1399`. Verified by
`rtk proxy grep -rn "HOSTILE_PHASE_TOKENS" src/ tests/`. No behavioural consequence — the primary
SAFE-08 pin (the plan's acceptance criterion, `cargo test --lib goal`) is in `src/driver/goal.rs`
and was written there as specified. Commit `88ae851`.

**[Rule 4-adjacent — instruction would make an existing test name assert a falsehood; taken as a scoped deviation] Look-alike tokens went into a NEW const, not into `HOSTILE_PHASE_TOKENS`.**
Found during: Task 1(d). The plan said to add `"2\u{200b}0"` and `"20\u{feff}"` to
`HOSTILE_PHASE_TOKENS`. That array's own doc says every entry is "a real terminal capability",
and its only consumer is
`a_phase_token_carrying_a_control_character_is_refused_by_name_rather_than_stored` — but a
look-alike token carries **no control character** (`U+200B` is `Cf`, not `Cc`, so
`char::is_control` is false for it). Folding them in would have made that test's name a false
statement about two of its fixtures, in the exact register this phase penalises; renaming the
test would have broken the coverage `ref` recorded in `21-08-SUMMARY.md`. Instead a sibling const
`LOOK_ALIKE_PHASE_TOKENS` and a sibling test
`a_phase_token_that_renders_like_a_declared_phase_is_refused_by_the_predicate` were added
alongside, with the reason recorded at the const. Every assertion the plan asked for is made;
only the container differs. Commit `88ae851`. **Flagged for the verifier** — this is a judgment
call about a plan instruction, not a mechanical fix.

**[Rule 1 — compiler-forced, beyond the plan's enumeration] `cred::askpass` also took `&Alias`.**
Found during: Task 2b(a). The plan named `askpass_with_config`; `askpass` is its default-path
wrapper and calls it, so the compiler forced the same change. Mechanical, no behaviour change
(the function has no caller in `src/` or `tests/`). Commit `337f67a`.

**[Rule 1 — refusals moved sites, so their tests moved with them] Two `registry_test.rs` tests were rewritten rather than mechanically converted.**
Found during: Task 2a(f). `add_project_with_empty_alias_returns_error` and
`add_project_with_whitespace_alias_returns_error` read `add_project`'s `bail!` messages, which
this plan deletes: those refusals now fire earlier, at `Alias::new`, and a blank string can no
longer reach the registration signature at all. They were rewritten as
`an_empty_alias_cannot_be_constructed_so_it_never_reaches_registration` and
`a_whitespace_carrying_alias_cannot_be_constructed_either`, asserting the same two properties at
the new site with the move recorded in their docs. Coverage is preserved, not dropped.
Commit `f46576f`.

**[Rule 1 — mechanical] `hooks`'s signature path is `&crate::registry::Alias`.**
Found during: Task 2b. `src/envelope/hooks.rs` has no `registry` in scope; the plan's literal
`alias: &registry::Alias` does not compile there. Fully qualified. Commit `337f67a`.

**[process — not a code deviation] A raw `U+200B` reached commit `16b46e5`'s message and was amended out.**
Found during: Task 2a's commit. The behavioural line quoted the look-alike alias with a literal
zero-width character embedded in the shell form. It was amended to the escape form
(`the alias `demo` + U+200B`); the amended commit is `f46576f`. No source file was affected —
`git grep -P '\x{200b}|\x{feff}|\x{2060}'` over `src/*.rs`, `tests/*.rs` and the round-6 plans
returns nothing. Recorded because the hazard is real and the near-miss is worth the next
executor knowing about.

**Total deviations:** 6 (5 auto-fixed under Rules 1/4-adjacent, 1 process). **Impact:** none on
the plan's properties — every acceptance criterion was met at the site the plan named, with the
two container/wrapper differences recorded above.

## Issues Encountered

None blocking. Two notes:

- A `--test-threads=2` whole-suite run mid-Task-2a reported `passed=1342` where the arithmetic
  says 1343; the immediately following runs report 1343 and 1345 consistently with the tests
  added. `failed=0` in every run. Not chased — no failure was ever observed — but recorded so
  the count is not read as a silent loss.
- None of the three documented flakes (`driver_reattach` ×2, `envelope_tracer`'s relocated stub)
  fired in any of this plan's five whole-suite runs.

## Verification results

| Gate | Command | Result |
|---|---|---|
| Build | `rtk proxy cargo build --all-targets` | clean |
| Clippy | `rtk proxy cargo clippy --lib -- -D warnings` | clean |
| Suite | `rtk proxy cargo test --workspace --no-fail-fast -- --test-threads=2` | **1345 passed, 0 failed** (base at `0352dda`: 1335) |
| Guards | `rtk proxy cargo test --test spawn_seam_guard` | 34 passed, 0 failed |
| Registry | `rtk proxy cargo test --test registry_test` | 14 passed, 0 failed |
| Boundary | `rtk proxy cargo test --lib every_argv_position` | 2 passed, 0 failed |

## Self-Check: PASSED

- Every task's acceptance criteria run and passing (evidence quoted above).
- Red arm committed before the fix, ordering visible in `git log`.
- `git log --oneline 0352dda..HEAD` shows one commit per task plus the red arm.
- No commit of this plan touches `.planning/REQUIREMENTS.md`, `STATE.md` or `ROADMAP.md`.

## Next

Ready for `21-18` (wave 2). Its Task 2(c) depends on D-17-5's cfg-gate drop, which landed in
`88ae851` and is already exercised from an integration crate by
`tests/registry_test.rs::a_look_alike_alias_cannot_join_its_visible_twin_in_the_registry`.
