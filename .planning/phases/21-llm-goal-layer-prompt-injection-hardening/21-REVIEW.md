---
phase: 21-llm-goal-layer-prompt-injection-hardening
round: 7
reviewed: 2026-08-25T00:00:00Z
depth: deep
diff_base: 2074595
head: 8dc8c98
previous_round: "Round 6's 21-REVIEW.md is preserved in git at f1faa3e (`git show f1faa3e:.planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-REVIEW.md`). This file replaces it."
files_reviewed: 12
files_reviewed_list:
  - Cargo.toml
  - src/text.rs
  - src/test_support.rs
  - src/journal/mod.rs
  - src/journal/writer.rs
  - src/registry.rs
  - src/driver/mod.rs
  - src/driver/goal.rs
  - src/main.rs
  - src/ui/project_list.rs
  - tests/driver_dry_run.rs
  - tests/driver_goal_seam.rs
  - tests/registry_test.rs
  - tests/spawn_seam_guard.rs
  - tests/driver_injection_corpus.rs
files_read_but_unchanged_this_round:
  - src/ui/mod.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/delete_confirm.rs
  - src/envelope/hooks.rs
  - src/envelope/advisory.rs
  - src/envelope/mod.rs
  - src/cli.rs
findings:
  critical: 1
  warning: 4
  info: 3
  total: 8
status: issues_found
---

# Phase 21 Round 7: Code Review Report

**Reviewed:** 2026-08-25
**Depth:** deep (cross-file: module tree, call chains, doc-vs-behaviour reconciliation)
**Range:** `2074595..8dc8c98` (17 commits, plans 21-19 and 21-20)
**Status:** issues_found — 1 Critical, 4 Warning, 3 Info

## Summary

**The round does what it says at the three levels that matter, and it breaks the six-round
pattern for the class and for identities.** `is_invisible_formatting_char` (src/text.rs:144-153)
carries no literal range; `is_identity_char` (src/text.rs:204-206) is a finite alphabet and is a
clause of `is_plain_path_component` (src/journal/mod.rs:339) — so the run-id seam
(src/driver/mod.rs:1087), the model-selected target-phase seam (src/driver/mod.rs:877), the goal
seam (src/driver/goal.rs:667), `run_paths` (src/journal/mod.rs:385), `read_active_run`
(src/journal/writer.rs:491), `envelope_dir_in` (src/envelope/mod.rs:222) and `Alias::new`
(src/registry.rs:145) all inherit it with **no bypass route found**. The `..`/`.` traversal
check and the single-`Component::Normal` check survive and are still load-bearing (`.` is in the
alphabet). Free text is correctly NOT judged by the alphabet — `is_identity_char` has exactly
two production consumers and neither is on a `--goal`/`--command` path. The independent-oracle
sweep is genuinely independent and genuinely non-vacuous (verified below).

**Where it re-enacts the pattern is the render half.** D-19-5 claims three escaped render sites;
one of them is a file that is not in the module tree, and the live TUI renders aliases raw at
~10 sites. That is exactly this phase's signature failure — a control shipped into a place the
mechanism does not reach, certified by a file list rather than by a test.

**Verification I ran myself (all via `rtk proxy`):**
`cargo test --test spawn_seam_guard --test driver_injection_corpus` → 38 passed / 0 failed, and
13 passed / 0 failed / 10 ignored. `grep -c '#\[ignore' tests/driver_injection_corpus.rs` → 18;
`grep -cE '^[[:space:]]*#\[ignore'` → 10; `grep -cE '^[[:space:]]*fn corpus_'` → 8 (7 arms +
the `corpus_run` helper, correctly excluded by the census's runtime-joined suffix).
`grep -rn "mod project_list" src/` → exit 1, no hits.

## Critical Issues

### CR-01: The TUI half of D-19-5 shipped into a file that is not compiled; the live project list still renders aliases raw

**File:** `src/ui/project_list.rs:156` (dead), `src/ui/mod.rs:1-2`, `src/ui/screens/normal.rs:740,742` (live)

**Issue:** 21-19 Task 3(c) added `crate::text::display_identity(alias)` to
`src/ui/project_list.rs`. **`src/ui/project_list.rs` is not declared as a module anywhere.**
`src/ui/mod.rs` declares only `roadmap_widget` and `screens`; `rtk proxy grep -rn "mod
project_list|path = \"project_list" src/` returns nothing, and `git log -S"mod project_list"`
shows the declaration was deleted in `c297631` ("refactor InputMode to Screen trait + screen
stack architecture"). The file has been orphaned dead code since that refactor. The escaping
edit therefore does not exist in the built binary.

The **live** project list is `src/ui/screens/normal.rs:734-743`, which renders
`Span::raw(alias.clone())` / `Line::from(alias.clone())` — raw. So the exact reproduction
D-19-5 exists for (`gsd-\u{202e}nur` rendering as `gsd-run`) is **unmitigated in the TUI**.

Additional live raw alias render sites, all outside the claimed three:
- `src/ui/screens/delete_confirm.rs:65` — `Remove "{alias}"? …[y/n]`. This is the highest-
  consequence one: the operator reads a spoofed name and confirms a destructive action on it.
- `src/ui/screens/delete_confirm.rs:142` — `Removed "{alias}"` toast.
- `src/ui/screens/detail.rs:2430`, `:3737` — `" Project: {alias} "` tab title.
- `src/ui/screens/detail.rs:1891`, `:1894`; `src/ui/screens/normal.rs:530`, `:533`;
  `src/ui/screens/add_project.rs:192`; `src/ui/screens/create_project.rs:218`;
  `src/ui/screens/driver_confirm.rs:585`, `:587`; `src/ui/screens/driver_inject.rs:83`, `:95`;
  `src/ui/screens/driver.rs:876`, `:884`.

`rtk proxy grep -rn "display_identity" src/ tests/` returns three call sites total, one of them
dead — and **no test anywhere exercises a render site**, only the `display_identity` unit test
in src/text.rs. So the claim rests on prose plus a files-touched list, which 21-19 prohibition 3
forbids as a way to certify a structural claim.

Falsified artifacts as shipped: 21-19-PLAN `must_haves.truths[5]` ("the TUI project list render
aliases through a helper that escapes…"); 21-19-SUMMARY:180 ("the three render sites stopped
being reorderable"); 21-19-SUMMARY:300 named-shape row 8 marked **CLOSED**; 21-19-SUMMARY:328
naming `src/ui/project_list.rs` as the TUI cell.

**Fix:**
```rust
// src/ui/screens/normal.rs — the LIVE project list
let escaped = crate::text::display_identity(alias);
let alias_cell: Line = match row_badge(ctx, alias) {   // lookups keep the RAW key
    Some(badge) => Line::from(vec![
        Span::styled(badge.glyph, Style::default().fg(badge.color).add_modifier(badge.modifier)),
        Span::raw(escaped),
    ]),
    None => Line::from(escaped),
};

// src/ui/screens/delete_confirm.rs:63-66
let prompt = format!(
    "Remove \"{}\"? This only unregisters it \u{2014} project files are not deleted. [y/n]",
    crate::text::display_identity(&self.alias)
);
```
Then either delete `src/ui/project_list.rs` or re-declare it — leaving an orphaned near-copy of
a live screen is how this defect happened. Add a committed control that goes red if a render
site regresses, e.g. a `spawn_seam_guard`-style scan asserting that no executable line under
`src/ui/` interpolates a bare alias binding into a rendered `String`/`Span`, with the escaped
sites as the positive control. Correct the three SUMMARY claims and reopen named-shape row 8.

## Warnings

### WR-01: `WITNESS_ALLOWED_ELSEWHERE`'s exact-count claim is an overclaim, in the register 21-20 prohibition 2 exists to end

**File:** `tests/spawn_seam_guard.rs:3828-3830`, restated at `:3933-3938`

**Issue:** The const doc asserts: *"The COUNT is exact on purpose: an allowed site cannot grow a
second member of the set — the first step of becoming the hand copy this guard exists to catch —
without breaking this loudly."* That sentence is false in two directions the doc does not name:

1. The census counts **witnesses**, not **members**. An allowed site can grow `""`, `"   "`,
   `"\t"`, `"\u{feff}"`, `"\u{00ad}"`, `"\u{e0041}"` or `"\u{fe0f}"` — seven of the ten members
   — and no count moves. "A second member of the set" is precisely what the scan cannot see.
2. The census is per-**file**, not per-**line**. An allowed file can delete one legitimate
   witness occurrence and add a hand-copy occurrence in the same file; `actual == expected`
   still holds. The claimed "first step" is silent.

The guard's own limits block (`:3926-3932`) names only the *other* under-detection direction (a
copy carrying none of the three witnesses). 21-20 prohibition 2 requires every unbounded
direction to be named **with its failure direction**, and says "it applies to this plan's own
diff most of all."

**Judgement on the mechanism itself (asked for explicitly): defensible, not a reintroduction.**
Three properties separate it from the enumeration defect this round exists to kill:
(a) it is a `assert_eq!(actual, expected)` on a `BTreeMap` in **both** directions, so a stale
row breaks loudly — the character class could only ever rot silently; (b) it governs a
test-hygiene scan, not a production predicate — a wrong row weakens a duplicate-detector, it
does not admit a hostile value at a seam; (c) I verified all four rows are correct at HEAD by
tracing the witness literals (`src/journal/writer.rs:1091` suffix list ×1 each for witnesses 1
and 2; `src/text.rs:531,663` ×2 for witness 1; `src/text.rs:618` ×1 for witness 2 — the doc
comment at `:308` is comment-filtered by `executable_lines`). The defect is the **doc**, not
the table.

**Fix:** Delete or qualify the "cannot grow a second member" sentence in both places and replace
it with the two directions above:
```rust
/// The count is exact so an allowed site cannot grow a second occurrence of THIS
/// WITNESS unnoticed. It bounds nothing else: the site may grow any of the seven
/// members that are not witnesses, and the census is per-file, so an occurrence
/// swapped for a hand-copy occurrence inside an allowed file is invisible.
/// Under-detection, silent, in both directions.
```

### WR-02: `src/envelope/hooks.rs:155` now states a falsehood about `is_plain_path_component`, and it is a security rationale

**File:** `src/envelope/hooks.rs:151-156`

**Issue:** `stub_body`'s doc justifies POSIX-quoting with: *"An alias is a plain path component,
which is a weaker constraint than 'shell-safe' — `is_plain_path_component` accepts a quote
character."* After D-19-2 the alphabet clause (src/journal/mod.rs:339) refuses `'`, `"`, `` ` ``,
`$`, `;`, space and every other shell metacharacter, so the stated premise is now false. The
generated code is still correct (`sh_quote` is the right defence and should stay), but the
*reason a future maintainer reads before deciding whether to keep it* is now wrong — and a
maintainer who checks the claim, finds it false, and concludes the quoting is redundant would
remove a real defence. 21-19's stale-doc sweep was count-focused
(`grep -rn "7x6\|42-cell\|six payloads\|; 6\]"`) and could not catch a claim about what the
predicate *accepts*.

**Fix:** Rewrite the paragraph to the post-D-19-2 truth and keep the quoting on defence-in-depth
grounds:
```rust
/// Both interpolated values are POSIX-quoted by [`sh_quote`] rather than relying on
/// the alias alphabet. Since D-19-2 an alias is drawn from `[A-Za-z0-9._-]`, which is
/// shell-safe — but the quoting stays: it is the only thing that survives a later
/// widening of the alphabet, and a generated script is not a place to depend on a
/// predicate defined three modules away.
```

### WR-03: A second, independent spelling of the identity alphabet survives at `src/envelope/advisory.rs:569`

**File:** `src/envelope/advisory.rs:562-570`

**Issue:** `is_plain_component` spells `c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')`
— byte-for-byte the same set as `text::is_identity_char` — and uses it to gate GitHub
`owner`/`repo` segments before they are interpolated into a request path. src/text.rs:155 claims
"**The ONE spelling of the identity alphabet**"; that claim is false tree-wide. 21-19-SUMMARY:389
records the site and defers it, which honours the disclosure prohibition, but the doc in
src/text.rs was not narrowed to match, so the artifact and the tree disagree. The second set at
`:598` (adds `/`) is genuinely a different set and is fine.

**Fix:** Either delegate `advisory.rs:569` to `crate::text::is_identity_char` (it is the same
set and the same question), or narrow src/text.rs:155's claim to "the one spelling for
`is_plain_path_component` identities" and name `advisory.rs:569` as the deliberate exception with
its reason.

### WR-04: `AliasRefusal::NotPlainComponent` is now near-unreachable and has no pin

**File:** `src/registry.rs:145-158`

**Issue:** Placing the alphabet clause above the path-component clause makes
`NotPlainComponent` reachable only for `"."` and `".."` — every separator, traversal and control
shape now exits through `OutsideIdentityAlphabet` or an earlier clause. Its message ("is not a
single plain directory name, so it cannot name an envelope root") is therefore emitted for
values where the more useful alphabet message would also have been true, and there is no test
that reaches it at all: `rtk proxy grep -n "NotPlainComponent" src/registry.rs tests/*.rs` →
three hits, all in `src/registry.rs`, zero in tests. A refusal variant with no committed
consumer is the shape that rots into a wrong message.

**Fix:** Add a two-value pin in `src/registry.rs`'s test module asserting
`Alias::new("..")` and `Alias::new(".")` are `Err(AliasRefusal::NotPlainComponent { .. })` —
this also certifies that the alphabet clause did **not** subsume the traversal check, which is
the load-bearing claim in 21-19's must_haves.

## Info

### IN-01: `src/envelope/mod.rs` is declared in 21-19's `files_modified` and was not modified

**File:** `.planning/phases/…/21-19-PLAN.md:18`

**Issue:** `git diff --stat 2074595..HEAD` shows no `src/envelope/mod.rs` change. It inherits
the alphabet transitively via `is_plain_path_component`, so nothing is missing behaviourally —
but a declared-and-untouched file is the mirror image of CR-01's declared-and-dead file, and the
merge guard cannot distinguish "correctly unnecessary" from "forgotten".

**Fix:** Note it in the SUMMARY's deviation list as "declared, transitively covered, no edit
needed" so the next round's guard has the adjudication.

### IN-02: Out-of-declared-scope wave-1 edits check out clean

**Files:** `tests/driver_dry_run.rs:605-611`, `tests/driver_goal_seam.rs:1490-1510`

**Issue (none — recorded as a clean result):** I diffed both files in full. `driver_dry_run.rs`
is a **comment-only** change (deviation 3, stale count prose). `driver_goal_seam.rs` is the
`LOOK_ALIKE_PHASE_TOKENS` addition plus its doc (deviation 1). **Nothing rode along.** The
deviation-1 routing is also correct *and strengthens* the assertion rather than weakening it:
`HOSTILE_PHASE_TOKENS`' consumer asserts `!offending().chars().any(char::is_control)`, which
none of the three witnesses satisfies; `LOOK_ALIKE_PHASE_TOKENS`' consumer
(`tests/driver_goal_seam.rs:1519-1552`) asserts the refusal reason is
`REASON_PHASE_NOT_PLAIN_COMPONENT` and **not** `REASON_PHASE_ABSENT_FROM_ROADMAP`, against a
fixture roadmap that declares `"20"` — a strictly stronger claim than "it was refused".
Deviations 2 and 3 are likewise correct: without the `is_ascii_alphanumeric` filter a folder
named `"..."` sanitizes to `"..."` (non-empty, all-identity-chars) and would register as a
useless alias; the four count corrections are accurate and the historical sentence was
correctly left intact.

### IN-03: Non-vacuity of the new assertions — verified, both hold

**Files:** `src/text.rs:377-406`, `tests/driver_injection_corpus.rs:1374-1446`

**Issue (none — recorded as a clean result):**
- `format_seen >= 150` is **genuinely non-vacuous**. The counter increments beside the
  membership assertion inside the oracle-filtered branch, so a mis-scoped `u32 -> char` filter,
  a missing `general-category` feature (a compile error, by the explicit Cargo.toml pin at
  `Cargo.toml:120`), or an inverted trait import all drive `format_seen` to ~0 and trip the
  floor. The floor is a lower bound only, so it cannot flake on a Unicode refresh. The oracle
  (`unicode-properties`) and the implementation (`icu_properties`) share no code — I confirmed
  `is_invisible_formatting_char` reads only ICU4X (src/text.rs:144-153) and
  `unicode_properties` appears only inside `#[cfg(test)]` in src/text.rs and inside the test-only
  `visibly_empty_numbered_entry` helper in src/driver/mod.rs.
- The census **does** trip on an empty or mis-scoped scan: both counts are `assert_eq!` on exact
  values (10 and 7), so zero fails as loudly as eleven. **The 14-vs-18 gap is not a mis-scope.**
  Measured at HEAD: raw substring `#[ignore` = **18**, line-anchored = **10**,
  `^\s*fn corpus_` = **8**. The plan's 14 was measured *before* the census's own prose landed;
  the census's four new mentions of the attribute moved raw 14→18 and anchored 10→10, which is
  the anchoring being load-bearing rather than a scan looking at the wrong set. The `fn corpus_`
  count of 8 vs the asserted 7 arms is the `corpus_run` helper at `:798`, correctly excluded by
  the runtime-joined suffix. The doc at `:1221-1223` states 18-vs-10 accurately.

---

## Does this round break the six-round enumeration pattern?

**For the character class: yes, structurally.** The predicate is a query against two
independently maintained derivations of the standard, and the falsifying fixture is an
exhaustive sweep with a committed arrival floor. There is no list left to be one item short of.

**For identities: yes, and more strongly.** A finite alphabet has no "next level down." The
`..`/`Component::Normal` structural checks correctly survive underneath it, and every identity
seam in the tree consumes it through one clause with no bypass.

**For the guard machinery: partially.** `WITNESS_ALLOWED_ELSEWHERE` is a hand-maintained table,
but it is bidirectionally pinned and governs a hygiene scan rather than a boundary — that is a
different risk class, not the same defect renamed. Its **doc**, however, claims a bound the scan
does not perform (WR-01), which is the fifth consecutive round in which the overclaim lives in
the artifact written to end overclaiming.

**For the render surface: no — it re-enacts the pattern one level up.** The mechanism is sound
and the sampling of *where to apply it* never moved off the executor's own file list. Three
sites were named, one of the three is not in the module tree, and roughly ten live sites were
never enumerated. Nothing in the tree goes red for any of it. That is CR-01, and it is the same
shape as the six rounds before it: a correct predicate, applied to a set nobody derived.

---

_Reviewed: 2026-08-25_
_Reviewer: Claude (gsd-code-reviewer), round 7 — independent of the executor and of the concurrent verifier_
_Depth: deep_
