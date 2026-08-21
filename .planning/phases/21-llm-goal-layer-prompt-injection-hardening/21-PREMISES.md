# Phase 21 — Premise Validation (gap-closure round 4)

**Authored:** 2026-08-21, before any round-4 plan. Every claim below was checked against the
tree at HEAD (`0c4f712`) by direct read or by `rtk proxy` grep — not taken from any SUMMARY.
The plans `21-13-PLAN.md` and `21-14-PLAN.md` are shaped by these verdicts; where a verdict is
BROKEN, the plan does the structural thing rather than the fourth point fix.

---

## Premise 1 — Per-arm blankness validation at the match site

**Verdict: BROKEN.**

**Evidence.** `command_source` (`src/driver/mod.rs:372-409`) carries the blankness rule three
times or not at all: the `Goal` arm has had `!goal.trim().is_empty()` since 21-07 (`:405`),
the `Command` arm gained `!command.trim().is_empty()` in 21-11 (`:392`), and the `Routed` arm
(`:400`) has nothing — `(None, Some(target_phase)) => Ok(CommandSource::Routed(target_phase.to_string()))`.
Three cycles, three arms, one at a time, each fix correct and each scoped to exactly the arm
that just broke. The payload type is `String`, which can hold `"   "`, so every arm — and every
*future* arm — must independently remember the invariant. The compiler enforces nothing.

**What the type-level fix costs.** Almost nothing here: `CommandSource` (`:314`) is
`pub(crate)` and consumed **only inside `src/driver/mod.rs`** (confirmed by
`rtk proxy grep -rn "CommandSource" src/` — the only hits outside `mod.rs` are `error.rs`'s
unrelated error variants and `run.rs`'s unrelated second enum). A payload newtype with a
private field and a single validating constructor (`NonBlank`, in a nested module so even
`driver/mod.rs`'s own code cannot bypass the constructor) makes a blank payload
**unrepresentable**: `CommandSource::Command(NonBlank)`, `Routed(NonBlank)`, `Goal(NonBlank)`.
The three per-arm trims collapse into one validation written once. A fifth argv source that
reuses `NonBlank` inherits the invariant at compile time; one that does not is forced through
`variant_name`'s wildcard-free match and guard eight anyway.

**Does the codebase have an idiom for this?** Partially. The house idiom is
"single production constructor + a guard test that checks single-ness"
(`DrivableProject::from_registry`, the decomposition capability, guard eight). That idiom is
*weaker* — it guards the construction **site**, not the constructed **value**, which is
exactly the distinction that lost three times: `command_source` genuinely is the single
constructor and still built three blank-holding values. `CommandSource`'s own doc already
argues in the type-level register ("resolving once and matching exhaustively is what makes a
fourth source a compile error"); the newtype completes that argument for the payload half.

**Implication for the plan.** 21-13's core is the `NonBlank` newtype, not a fourth trim.

---

## Premise 2 — Two enums named `CommandSource`

**Verdict: QUESTIONABLE as named; the abstraction itself HOLDS, the collision is real debt
that comes due now.**

**Evidence.** `src/driver/run.rs:614` declares `enum CommandSource { Fixed(String), Routed { target_phase } }`
— a genuinely different concept (per-iteration command provenance inside the run loop) from
`driver/mod.rs:314`'s argv-resolution enum. Two concepts, one name. The cost is no longer
hypothetical: guard eight (`tests/spawn_seam_guard.rs:2148-2165`) spends its entire header
justifying parenthesis-suffixed needles that exist **only** to avoid matching the wrong enum,
and carries a dedicated "collision watchdog" assertion (`:2286-2299`). A guard whose needles
are shaped by a name collision is one refactor away from silent blindness. 21-11 recorded the
rename as accepted debt; 21-REVIEW.md round 3 (WR-02) says it "should be done."

**Second finding at the same site, worse than the name.** `run.rs:2602` re-derives the source
from raw `args` — the exact "re-deriving is how CR-01 happened" pattern `mod.rs:537` warns
about — and its spelled-out unreachable arm is
`(None, None) => CommandSource::Fixed(String::new())`: it **manufactures the precise blank
value three cycles have been spent refusing**, as the fallback for a state believed
unreachable. If any future change makes it reachable, a blank command reaches the spawn seam
with no refusal anywhere.

**Implication for the plan.** 21-14 renames `run.rs`'s enum to `IterationSource` (variants
`Fixed`/`Routed` unchanged) and replaces the blank-manufacturing arm with a typed
`Err(DriveError::NoCommandSource)` return. After the rename, guard eight's needles stop being
delicate and its collision watchdog becomes a simple "exactly one `enum CommandSource` is
declared under `src/`" assertion.

---

## Premise 3 — Grep-over-source as the enforcement mechanism

**Verdict: QUESTIONABLE — legitimate for exactly one class of property, currently used
dishonestly in two places, and load-bearing where a type should be.**

**Where a textual guard is genuinely the only option.** "Exactly one function tree-wide calls
`JournalRun::finish`" (guard six) is a cross-module single-call-site property of a `pub`
method. Rust's visibility system cannot express it (no friend visibility; a capability-token
parameter would be a larger redesign of the journal API). A textual scan is the honest tool —
**if** it names every approximation with its failure direction and bounds its silent ones.

**Where the guards are currently dishonest** (all confirmed by direct read):
- Guard six's header (`:1691-1727`) names three limits but omits the live fourth:
  `terminal_write_hits` skips from a file's column-zero `mod tests {` **to EOF**
  (`test_region_start(file).unwrap_or(usize::MAX)`, then `*number >= boundary → continue`).
  `src/state_reader/mod.rs` has the marker at :311 and production `pub fn count_backlog_items`
  at :530 — invisible to the guard, independently corroborated by clippy's
  `items_after_test_module`. 37.5% of `src/` lines sit past a marker.
- Guard eight (`:2136-2299`) names **no** limitation, in the same file and round where guard
  six was rewritten specifically to stop that. Its real gaps: `Self::Command(` and imported
  variants match no needle; `enclosing_fn` (`:1130-1136`) does no brace tracking; it inherits
  the marker-to-EOF skip.
- The matrix's oracle is tautological: `DEGENERATE` (`mod.rs:1674`) and
  `empty_numbered_entry` (`:1706-1715`) both define "blank" with the same `str::trim` the
  production guard uses — the enumeration structurally cannot contain a payload the guard
  mishandles (U+200B demonstrates it; reproduced in 21-REVIEW.md WR-03).

**What moves from text to the type system / exhaustive match in this round:**
- Payload blankness → the `NonBlank` type (Premise 1). Guard eight stops being the
  load-bearing defense for blankness; it remains useful for "resolution happens once."
- Matrix column coverage → tied to `command_source`'s **arity**: the `PositionBuilder` fn
  type returns a 3-tuple matching the resolver's signature, so a fourth argv parameter breaks
  every builder at compile time; column-to-variant coverage is asserted against a single
  `ALL_VARIANT_NAMES` const that `variant_name`'s wildcard-free match anchors (closing IN-02's
  duplicated-literal gap at the same time).
- The degenerate rule itself → **uniform**: every degenerate payload in every position is
  `Err(NoCommandSource)`. No disjunction, no per-column hand-picked sweep, no exemption
  comment. An exemption is no longer a thing the test's shape can express.
- Guard six's silent boundary gap → made **loud**: a new assertion that no column-zero item
  declaration follows any file's `mod tests {` marker (plus moving `count_backlog_items`
  above the marker, which also clears the clippy lint). The remaining approximations get
  named limits with directions, using 21-REVIEW.md's ready wording.

**What makes a surviving textual guard honest:** a limits block enumerating each
approximation with its failure direction (loud/silent), a non-vacuity control per silent
direction, and no blanket claims — the standard guard six's header now half-meets and guard
eight doesn't meet at all. 21-14 brings both to it.

---

## Premise 4 — Is ROADMAP criterion 1 well-formed?

**Verdict: QUESTIONABLE — it bundles several properties, but re-scoping it mid-streak is the
wrong move; the plans name the missing invariant explicitly instead.**

**Evidence.** Criterion 1 bundles: (a) plan structure/machine-checkability, (b) review-before-
run, (c) preview honesty, (d) refusal of degenerate invocations before side effects. All three
failures landed on an *implicit* sub-invariant nobody wrote down: **"no unvalidated argv
payload reaches a render or a persisted record."** Each verification found (a)-(c) fine and
(d) broken in a new spot; because (d) had no name, each fix was scoped to the spot.

**Why not split the ROADMAP criterion now.** Verification keys on ROADMAP text; rewriting the
goalposts during a failing streak makes the history illegible and looks like gaming the gate.
The correct move is downstream: 21-13 states the invariant as its own `must_haves` truth
("every argv payload carrying no visible instruction is refused with `NoCommandSource` before
any file is created, uniformly across every `CommandSource` variant — enforced by type"), so
the verifier checks the class, not the instance. If the phase passes, a follow-up may propose
splitting criterion-1-shaped criteria at the next milestone boundary; that is a roadmap
decision, not a gap-closure one.

---

## Premise 5 — Is 21-CONTEXT.md / 21-RESEARCH.md stale or contradicted?

**Verdict: HOLDS, with one named gap; no surviving decision is contradicted.**

**Checked.** The four Research Corrections (C-1 `CLAUDE_CODE_DISABLE_CLAUDE_MDS`, C-2
strict-mcp regression guard, C-3 arrival-assertion rule, C-4 SHA-256) all still describe the
shipped mechanisms; criteria 3-5 verified against them for four consecutive passes. D-30
("`""` means field-absent on the tolerant read path") is not only intact — it is the
load-bearing reason blankness is corruption rather than cosmetics, and this round's plans
lean on it. The always-park resolution, two-seam count, and enum-refusal decisions are
untouched by anything three cycles revealed.

**The gap.** No decision in CONTEXT.md governs **argv payload validity**. The phase's premise
inventory implicitly assumed argv values are non-degenerate; three cycles proved that premise
false three ways. This is an absence, not a contradiction — and it is why the fix belongs in
the type (a decision the plan records) rather than in another comment.

---

## Premise 6 — Is the phase too big? Should some of this be a follow-up phase?

**Verdict: HOLDS — finish it here. The remaining scope is one coherent closure, not a
sprawl.**

**Evidence.** Criteria 2-5 have been verified and re-verified across four passes; the entire
remaining failure surface is one defect family in one function plus the honesty of the guards
built around it. Splitting that into a new phase would carry the identical scope forward under
a new number — cost without narrowing. What *does* get adjudicated out, each with a recorded
reason (written into `deferred-items.md` by 21-14, so nothing drops silently):

| Carried item (round 2) | Adjudication | Reason |
|---|---|---|
| `current_prompt_inputs` absent from `BLOCKING_HELPERS` (`tests/async_blocking_guard.rs`) | **OUT — defer** | Async-hygiene (sync disk reads under an async fn), not the failing criterion's class; the fix forces production `spawn_blocking` rewiring in `approve_plan` and `execute_run` — real scope, zero bearing on criterion 1. |
| Spawn-gate plan-half tautology comment (`src/driver/run.rs:2300-2328`) | **OUT — defer** | The comment now states plainly that the plan half is a no-op there and why that is sound (decompose-once). Converting the argument to a checked property is hardening, not a gap; the honest doc is in place. |
| Dead `PlanStep::rationale` | **OUT — defer** | No production reader; cosmetic dead field, no security or honesty bearing. |
| `plan_target_phase(plan).unwrap_or_default()` blank route (`src/driver/mod.rs:977`) | **IN — 21-13** | Same defect *class* as the criterion-1 failures (a blank `target_phase` reaching a persisted record), merely via the model seam instead of argv. Excluding it would repeat the exact scoping bet that lost three times. |

**Why the incremental-fix shape is still wrong even though the phase should finish:** the
three-cycle record shows each narrow fix was correct and each narrow *scope* was the defect.
21-13/21-14 are therefore scoped to the **class** (type-level unrepresentability + uniform
enumeration + honest guards), after which criterion 1's failure mode has no fourth arm to
land on: there is no arm-local validation left to forget, no hand-picked column left to
exempt, and no shared predicate left to agree with itself.

---

## Process failures designed against (both plans carry these as prohibitions)

1. **Premature `REQUIREMENTS.md` marking (twice: `828d7cc` revert, `760d71f` re-offense,
   `0c4f712` re-revert).** At HEAD all five phase-21 requirement rows correctly read
   `[ ]`/Gaps Found (confirmed by direct read). Both plans prohibit the executor from
   flipping any requirement status; that edit belongs exclusively to a `passed` verification.
2. **`rtk` output filtering (bit three times).** Every count-bearing or presence-bearing
   check in both plans is written `rtk proxy <cmd>` or plain `cargo`/`grep` explicitly.
