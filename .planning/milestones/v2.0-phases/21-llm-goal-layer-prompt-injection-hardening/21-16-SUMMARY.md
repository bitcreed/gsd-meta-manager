---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 16
subsystem: guards
tags: [security, guard-honesty, self-testing, gap-closure]
requires: [21-15]
provides:
  - "guard nine — drive_args_declares_no_raw_argv_string_field, with a shared-code-path planted-defect control"
  - "post_marker_offenders — the boundary self-check's collection loop extracted, offenders + scanned-files count"
  - "guard eight per-needle non-vacuity, replacing the line-count bound"
  - "one_of_each — the second anchor direction for ALL_VARIANT_NAMES"
  - "the two round-4 prohibitions as standing assertions"
  - "an_over_length_phase_token_is_refused_by_the_bound_not_truncated_into_a_phase"
affects:
  - tests/spawn_seam_guard.rs
  - tests/driver_goal_seam.rs
  - src/driver/mod.rs
  - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md
tech-stack:
  added: []
  patterns:
    - "one pure collection fn, two consumers — the live assertion and a planted-defect control that therefore witnesses the live scan"
    - "a guard limit is a measured fact (asserted by a control) rather than a sentence"
    - "non-vacuity over DISTINCT NEEDLES rather than over matched lines"
key-files:
  created: []
  modified:
    - tests/spawn_seam_guard.rs
    - tests/driver_goal_seam.rs
    - src/driver/mod.rs
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md
key-decisions:
  - "D-16-1 through D-16-4 executed as written; no decision reversed."
  - "Three of the plan's mechanical checks were literal greps that my own corrective prose would have broken; the prose was reworded so the checks read exactly as specified rather than the checks being reinterpreted (deviation 1)."
  - "The over-length fixture's third assertion was strengthened beyond the plan: a char-count bound alone is satisfied by rendering the token in full, so it also asserts the truncation marker and non-equality with the raw token (deviation 2)."
requirements-completed: []
duration: "~1h"
completed: 2026-08-22
coverage:
  - deliverable: "A guard that reads the TYPE rather than a list"
    verification:
      - kind: test
        ref: "tests/spawn_seam_guard.rs#drive_args_declares_no_raw_argv_string_field"
        status: pass
      - kind: test
        ref: "tests/spawn_seam_guard.rs#the_raw_argv_field_scanner_reports_a_planted_string_field"
        status: pass
      - kind: command
        ref: "planted-defect run against the LIVE tree: a real `pub goal_file: Option<String>` on DriveArgs produced (1) a compile error from from_argv's destructure and (2) guard nine's named red"
        status: pass
    human_judgment: false
  - deliverable: "The boundary self-check can see the tree it audits"
    verification:
      - kind: test
        ref: "tests/spawn_seam_guard.rs#the_boundary_self_check_sees_restricted_visibility_items"
        status: pass
      - kind: test
        ref: "tests/spawn_seam_guard.rs#no_production_item_follows_a_test_module_marker"
        status: pass
    human_judgment: false
  - deliverable: "Guard eight's non-vacuity counts distinct needles"
    verification:
      - kind: test
        ref: "tests/spawn_seam_guard.rs#every_command_source_variant_is_named_only_where_it_is_built_or_matched"
        status: pass
    human_judgment: false
  - deliverable: "ALL_VARIANT_NAMES anchored in both directions"
    verification:
      - kind: test
        ref: "src/driver/mod.rs#all_variant_names_matches_the_variant_set_in_both_directions"
        status: pass
    human_judgment: false
  - deliverable: "visibly_empty_numbered_entry is falsifiable on its own"
    verification:
      - kind: test
        ref: "src/driver/mod.rs#the_visibly_empty_detector_is_falsifiable_on_its_own"
        status: pass
    human_judgment: false
  - deliverable: "goal.rs's length-bound refusal has live coverage again"
    verification:
      - kind: test
        ref: "tests/driver_goal_seam.rs#an_over_length_phase_token_is_refused_by_the_bound_not_truncated_into_a_phase"
        status: pass
      - kind: command
        ref: "planted-defect run: deleting the bound layer makes it fail with left: goal_phase_absent_from_roadmap"
        status: pass
    human_judgment: false
  - deliverable: "DRIVE-04 boundary and precision (probe-authored)"
    verification:
      - kind: test
        ref: "tests/driver_escalation_cap.rs"
        status: pass
    human_judgment: false
  - deliverable: "SAFE-07 boundary and precision (probe-authored)"
    verification:
      - kind: test
        ref: "tests/driver_injection_corpus.rs"
        status: pass
    human_judgment: false
  - deliverable: "SAFE-08 — the fixed command alphabet is the refuse-not-repair contract"
    human_judgment: true
    rationale: "Flagged assumption, deliberately not auto-resolved: this plan reads SAFE-08's 'fixed enum of GSD commands' as router::SAFE_COMMAND_ALPHABET as consumed by goal::legality, and assumes Task 4 strengthens rather than alters that contract. Surfaced for the verifier."
---

# Phase 21 Plan 16: Guard Honesty and the Type-Reading Guard Summary

The guard machinery made as honest as the domain 21-15 closed, plus the guard
four cycles never had: one that reads the **type** rather than a list. Pass 5's
four guard-accuracy defects are closed with self-testing controls rather than
better sentences; round 4's two violated prohibitions become standing
assertions; and `goal.rs`'s length-bound refusal has live coverage again.

- **Duration:** ~1h · **Tasks:** 4 · **Commits:** 4 · **Files touched:** 4

## Commits

| SHA | Type | What |
|---|---|---|
| `40a137e` | test | **TRACER** — guard nine, with its planted-defect control |
| `2eefa73` | test | Task 2 — the boundary self-check can see the tree it audits |
| `1a31ab7` | test | Task 3 — per-needle non-vacuity, `one_of_each`, detector pins, two prohibitions promoted |
| `05accae` | test | Task 4 — the length-bound layer's one fixture |

## Demonstrated-detection evidence (required, quoted verbatim)

### Guard nine — a planted raw argv field, against the LIVE tree

The synthetic control arms pass, but I also ran the real experiment: I added
`pub goal_file: Option<String>` to the actual `DriveArgs` and watched both
mechanisms fire in sequence.

**Evidence 1 — 21-15's exhaustive destructure refuses to compile:**

```
error[E0063]: missing field `goal_file` in initializer of `DriveArgs`
   --> src/driver/mod.rs:456:12
    |
456 |         Ok(DriveArgs {
    |            ^^^^^^^^^ missing `goal_file`
```

Then, having done what a future editor would do — added it to `RawDriveArgs`,
threaded it through the destructure and the constructor, and fixed every
literal — the second mechanism fires, which is the whole reason guard nine
exists (the destructure forces the field to be **handled**, not to be handled
by giving it a payload type):

**Evidence 2 — guard nine's named red:**

```
thread 'drive_args_declares_no_raw_argv_string_field' (193511) panicked at tests/spawn_seam_guard.rs:2809:5:
a `DriveArgs` field is declared with a bare `String` type. Every argv-derived string field
must be `payload::NonBlank`, whose private field is the only thing that makes a blank payload
unrepresentable — four gap-closure cycles each protected a hand-picked subset and each subset
was exactly one item short. `DriveArgs::from_argv`'s exhaustive destructure forces a new field
to be HANDLED; it does not force it to be handled by giving it a payload type, and this is the
check that does. If the new field genuinely is not an argv payload, it does not belong on this
struct; if it is, type it `payload::NonBlank` and give it a row in the degenerate matrix.
Offending declarations:
  src/driver/mod.rs:141: pub goal_file: Option<String>,

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 28 filtered out
```

The planted defect was then reverted (`git checkout -- src/ tests/…`); the tree
at `40a137e` is clean and the guard green.

The synthetic control arms (`the_raw_argv_field_scanner_reports_a_planted_string_field`)
additionally pin, over **the same `raw_string_argv_fields` function the live
assertion consumes**: both planted raw fields reported and nothing else
(`alias: payload::NonBlank` not flagged, `claude_args: Vec<OsString>` not
flagged on a substring, `dry_run: bool` not flagged, `RawDriveArgs`'s own
`String` out of the extracted region); a declaration **wrapped across two
lines** still reported (pass 5: a name split across lines defeats a full-name
grep, and a guard silenceable by rustfmt is not a guard); a fully protected
declaration reporting nothing.

### The boundary self-check — a planted post-marker `pub(crate)` item

`the_boundary_self_check_sees_restricted_visibility_items` calls
`post_marker_offenders` — the same function the live assertion calls — over a
synthetic file and asserts it reports:

```
(5, "pub(crate) fn smuggled() {}")
(6, "pub(super) const X: u8 = 0;")
```

and does **not** report `pub fn above_the_marker()` (before the marker) or the
indented `    pub(crate) fn indented_and_therefore_invisible()` — limit 1, now a
measured fact rather than a guess. `scanned_files == 1` is asserted in the same
test, so the floor is proved computed rather than defaulted.

Against the pre-fix `ITEM_OPENERS` (no `"pub("`), that first assertion fails:
`starts_with("pub ")` matches neither planted line.

### The length-bound layer — deleting it makes the new test fail

Planted-defect run before committing Task 4: I deleted the
`untrusted::bounded(named_phase) != named_phase` refusal from
`src/driver/goal.rs` and re-ran the new test:

```
thread 'an_over_length_phase_token_is_refused_by_the_bound_not_truncated_into_a_phase' panicked at tests/driver_goal_seam.rs:1533:5:
assertion `left == right` failed: the over-length token REUSES the existing reason …
Against a build that truncates instead of refusing, this reads
`goal_phase_absent_from_roadmap`: the roadmap-membership arm, reached because the token was
silently repaired into a phase the model did not choose
  left: "goal_phase_absent_from_roadmap"
 right: "goal_phase_not_plain_component"
```

Exactly the failure mode the assertion message predicts. The layer was restored
and the test is green.

## Guard eight's per-needle counts

The three type-qualified needles, counted over the production regions of the
two driver files (comment lines excluded):

| Needle | Production matches |
|---|---|
| `CommandSource::Command(` | 9 |
| `CommandSource::Routed(` | 8 |
| `CommandSource::Goal(` | 8 |

Each ≥ 1, so no needle has gone blind; and each of the three allowlisted sites
is matched by all three, which is what the new assertion checks. The `Self::`
needles match zero sites and are excluded by D-16-3, named as limit 6 with its
direction.

## Allowlist diff

`COMMAND_SOURCE_ALLOWLIST` gained no entry in this plan. It carries three, the
third added by 21-15 with its role stated in the const's doc:

```rust
const COMMAND_SOURCE_ALLOWLIST: &[(&str, &str)] = &[
    ("src/driver/mod.rs", "command_source"),   // constructs
    ("src/driver/mod.rs", "preview_text"),     // matches
    ("src/driver/run.rs", "iteration_source"), // matches, never builds (21-15)
];
```

No guard was greened by widening an allowlist or weakening a needle in either
plan of this round. Guard eight's needle set is **unchanged**; what changed is
the non-vacuity taken over it, which is strictly stronger.

## Prohibition audit

| # | Prohibition | Check | Result |
|---|---|---|---|
| 1 | MUST NOT let any guard header claim more than its scan performs | See the limits-block table below | PASS |
| 2 | MUST NOT green a guard by widening an allowlist or weakening a needle | Allowlist diff above: **no entry added in this plan**; `COMMAND_SOURCE_VARIANTS` unchanged; `ITEM_OPENERS` was WIDENED to detect more, not narrowed to detect less. The live boundary scan stayed green over the unchanged tree with the wider opener set — 0 post-marker items, now measured by openers that can actually see them. | PASS |
| 3 | MUST NOT flip any `.planning/REQUIREMENTS.md` requirement | `rtk proxy git log --format=%H -- .planning/REQUIREMENTS.md` over `199d334..HEAD` → **no commits**; `--name-only \| grep -c REQUIREMENTS` → **0** | PASS |

### Prohibition 1 in detail — each rewritten limits block beside the behaviour that bounds it

| Guard | Limit, as rewritten | What bounds it (behaviour, not prose) |
|---|---|---|
| Boundary self-check | "1. An item that is INDENTED … 2. An item whose first token is outside `ITEM_OPENERS`. … The previous version of this paragraph bounded both by asserting where the tree declares its items, which is not a bound on either: gap 2 was live and at column zero when that sentence was written — 51 `pub(crate) ` and `pub(super) ` items that `starts_with("pub ")` could not match — while the header called the skipped region 'provably empty'." | `the_boundary_self_check_sees_restricted_visibility_items` plants both restricted-visibility spellings after a marker and asserts the **live** collection fn reports them, and plants an indented item and asserts it is **not** reported. `scanned_files >= 10` survives the extraction as a returned value. |
| Guard eight, limit 1 | "…Two things bound it: the PER-NEEDLE non-vacuity below — each of the three type-qualified needles must match at least one allowlisted line tree-wide, and each allowlisted site must be matched by all three… **This replaced a LINE COUNT, and the replacement is the point** (pass-5 warning 3). The bound used to be `contributed >= 3` per allowlisted function… That stops biting the moment a site grows a fourth line naming any variant, which 21-15 did." | The two loops in `every_command_source_variant_is_named_only_where_it_is_built_or_matched`: per-site distinct-needle coverage, and per-needle tree-wide ≥ 1. Counts quoted above. |
| Guard eight, limit 6 (new) | "The three `Self::`-qualified needles may legitimately match ZERO sites, and are therefore excluded from the per-needle floor (D-16-3). They exist to CATCH an evasion spelling, not to be used… **Under-detection — silent** for that spelling specifically, bounded by the type-qualified triple, which any real construction site must also name." | The type-qualified triple's own per-needle floor, plus the variant-import assertion that makes the cheapest evasion loud. |
| Guard nine (new), limits 1–3 | "1. A field whose type hides behind a local `type` alias for `String` … **Under-detection — silent.** Bounded by the fact that the tree declares no such alias today, which this guard asserts on its own line rather than assuming. 2. The field-line heuristic requires this tree's `pub <name>:` declaration style … **Under-detection — silent.** Bounded by the non-vacuity floor below. 3. The struct-body extraction is column-zero brace based … **Over- and under-detection**, cross-referenced to `no_production_item_follows_a_test_module_marker`." | Limit 1: `no_type_alias_hides_a_string_from_guard_nine` (a separate test, currently 0 aliases — `rtk proxy grep -c "type .*= String" src/driver/mod.rs src/driver/run.rs` → `0`, `0`). Limit 2: the `>= 10` field-lines and `>= 6` NonBlank floors inside the live assertion. Limit 3: cross-referenced to the boundary self-check, which now has its own control. |
| `visibly_empty_numbered_entry`'s doc | "…the reason is narrower than the old doc claimed (pass-5 WR-02). The old text credited this detector with breaking the trim tautology round-3 WR-03 found. It does not: the tautology is broken by the LITERAL `DEGENERATE` array… No degenerate payload ever reaches this detector at all, because every one of them is refused at the parse boundary before anything renders." | `the_visibly_empty_detector_is_falsifiable_on_its_own` — four direct pins, two must-report and two must-not. |

## Deviations from Plan

**[Rule 1 — plan's mechanical checks would have read the wrong number] Corrective
prose reworded so three literal greps read exactly as specified.**
Found during: Task 2 and Task 3. Writing the corrections *as corrections* — a
sentence that quotes the old, wrong claim in order to refute it — made three of
the plan's acceptance greps read 2, 2 and 2 instead of 1, 0 and 1:
`grep -c '"pub("'`, `grep -c "production style puts items at column zero"`, and
`grep -c "scanned_files >= 10"`; a fourth, `grep -c "would keep calling"` in
`src/driver/mod.rs`, read 1 instead of 0 because the *correct* residual claim
happened to reuse the phrase. In every case the property was right and the
witness was noisy. Fix: the prose was reworded to say the same thing without
the literal token (e.g. "a bound about where the tree declares its items" in
place of the quoted sentence). All four checks now read exactly as the plan
specifies. Commits `2eefa73`, `1a31ab7`.

**[Rule 2 — a plan assertion was too weak as written] The over-length fixture's
third assertion strengthened.**
Found during: Task 4. The plan asks that "the refusal's rendering of the
offending value is bounded, not the full 201 characters". A char-count bound
alone (`<= MAX + marker.len()` = 212) is **satisfied by rendering the token in
full** (201), so as written it would not detect the defect it names. My first
attempt at a length comparison also failed for an unrelated reason — the
truncation marker's multi-byte `…` makes the bounded value longer in *bytes*
than the 201-byte token. Fix: three assertions — the char-count bound, `!=` the
raw token, and `ends_with(TRUNCATION_MARKER)` (a shortened value must be
*marked* shortened, which is `untrusted`'s own stated contract). Commit
`05accae`.

**Total deviations:** 2 (2 auto-fixed, 0 escalated).
**Impact:** neither weakens a plan property; one restores the plan's own
witnesses, the other strengthens an assertion the plan under-specified.

## Residuals and things I am not fully confident about

1. **The `--test-threads=2` gate hit the OTHER documented flake this run.**
   `tests/envelope_tracer.rs::a_relocated_copy_of_the_stub_refuses_instead_of_acting`
   failed once in the final whole-suite run with
   `the generated stub is executable: Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }`
   — verbatim the write-then-exec race `deferred-items.md` records for that
   binary. Re-run alone: `6 passed; 0 failed`. Not a regression, and not caused
   by anything in this round (that file is in no `21-*` plan's `<files>`), but
   it means the honest gate result is "1334 passed, 1 documented flake" rather
   than a clean zero. Reported rather than re-run until green.

2. **Guard nine reads declaration TEXT, and the plan's truth 1 says so.** A
   field hidden behind a `String` type alias, or declared outside the
   `pub <name>:` style, is silent under-detection. Limit 1 is now asserted
   (`no_type_alias_hides_a_string_from_guard_nine`); limit 2 is bounded only by
   the field-count floor, which would catch a heuristic that broke *wholesale*
   but not one field styled differently from its eleven siblings.

3. **`the_degenerate_payload_set_is_spelled_in_exactly_one_place` relies on
   `executable_hits` filtering line comments.** Three files under `src/` contain
   the literal `"\n  \n"` inside doc/line comments that *explain* the historical
   three-of-six hand copy; they are filtered and the code-level count is 1. If
   someone ever writes a blank-shape list inside a block comment (`/* … */`),
   this guard would report it as an offender — over-detection, loud, which is
   the acceptable direction, but it is not documented in a limits block because
   the test is small enough to read whole. Flagging it rather than leaving it
   for the reviewer to notice.

4. **SAFE-08's flagged assumption is not resolved.** This plan reads "fixed enum
   of GSD commands" as `router::SAFE_COMMAND_ALPHABET` via `goal::legality`'s
   refuse-not-repair parse, and assumes Task 4 strengthens rather than alters
   that contract. Surfaced per the edge-probe audit, deliberately not
   auto-resolved.

## Tests added (by name — no count floor, per the plan)

`tests/spawn_seam_guard.rs` (6):
- `the_raw_argv_field_scanner_reports_a_planted_string_field`
- `drive_args_declares_no_raw_argv_string_field`
- `no_type_alias_hides_a_string_from_guard_nine`
- `the_boundary_self_check_sees_restricted_visibility_items`
- `no_match_arm_in_the_driver_manufactures_a_blank_value`
- `the_degenerate_payload_set_is_spelled_in_exactly_one_place`

`src/driver/mod.rs` (2):
- `all_variant_names_matches_the_variant_set_in_both_directions`
- `the_visibly_empty_detector_is_falsifiable_on_its_own`

`tests/driver_goal_seam.rs` (1):
- `an_over_length_phase_token_is_refused_by_the_bound_not_truncated_into_a_phase`

**Deleted:** none. **Net:** +9 (1326 after 21-15 → 1335 total).

## Verification

| Gate | Result |
|---|---|
| `rtk proxy cargo build --all-targets` | clean, 0 warnings |
| `rtk proxy cargo clippy --lib -- -D warnings` | clean |
| `rtk proxy cargo clippy --all-targets` | 4 lints, all pre-existing (`src/browser.rs` ×3, `src/project_creator.rs` ×1) — unchanged from the measured baseline |
| `rtk proxy cargo test --test spawn_seam_guard` | 32 passed, 0 failed (26 before this round) |
| `rtk proxy cargo test --test driver_goal_seam` | 21 passed, 0 failed |
| `rtk proxy cargo test --lib` | 1031 passed, 0 failed |
| `rtk proxy cargo test --workspace --no-fail-fast -- --test-threads=2` | **1334 passed, 1 failed** — the documented `envelope_tracer` write-then-exec race only; `6 passed; 0 failed` when that binary is re-run alone |

Every count-bearing and presence-bearing check in this SUMMARY was run through
`rtk proxy`.

## Issues Encountered

One: the documented `envelope_tracer` flake fired in the final whole-suite run
(residual 1 above). Re-ran the binary alone and it passed. Not chased further,
per `deferred-items.md`'s disposition for that test.

## Next Phase Readiness

Phase 21's round-5 gap closure is complete: both plans executed, both SUMMARYs
committed. `.planning/REQUIREMENTS.md`, `STATE.md` and `ROADMAP.md` were not
touched by any commit of either plan — requirement status is the verification's
to decide.

## Self-Check: PASSED
