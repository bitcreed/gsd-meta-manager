---
phase: 21-llm-goal-layer-prompt-injection-hardening
reviewed: 2026-08-21T00:00:00Z
round: 4
depth: deep
diff_base: 6eb1d496fbed77b381c141ea131d70e4014f0998
head: 900c257
supersedes: "round 3 (this file's previous contents) is preserved in git at commit d4874ec"
files_reviewed: 10
files_reviewed_list:
  - src/driver/mod.rs
  - src/driver/run.rs
  - src/driver/goal.rs
  - src/journal/mod.rs
  - src/state_reader/mod.rs
  - src/error.rs
  - src/cli.rs
  - tests/spawn_seam_guard.rs
  - tests/driver_dry_run.rs
  - tests/driver_goal_seam.rs
findings:
  critical: 2
  warning: 5
  info: 3
  total: 10
status: issues_found
---

# Phase 21 (gap closure, round 4): Code Review Report

**Reviewed:** 2026-08-21
**Depth:** deep (cross-file: argv → resolver → record → guard)
**Diff base:** `6eb1d49..900c257` (`src/`, `tests/` only)
**Files Reviewed:** 10
**Status:** issues_found

> **Note on this file:** round 3's review is preserved in git at commit `d4874ec`.
> This file replaces it in the working tree.

## Summary

The `NonBlank` newtype is real. I tried hard to break it and could not: `mod payload`
(`src/driver/mod.rs:324`) is nested, `NonBlank`'s field is private to that module
(`:331`), the only `impl NonBlank` in the tree is at `:333` and carries exactly two
methods, there is no `From`/`Deref`/`AsRef`/`Into`/`serde` on it, no `into_inner`, and
`as_str` hands back a `&str` from which no `NonBlank` can be rebuilt
(`rtk proxy grep -rn 'NonBlank' src/ tests/` returns nine hits, seven of them prose).
All three `CommandSource` variants carry it (`:395`, `:398`, `:402`), and the matrix at
`:2178-2196` is genuinely uniform: one nested loop, no `Ok` branch, no exemption comment,
column axis pinned to `command_source`'s arity via `PositionBuilder`'s 3-tuple. **Claim A
(i)–(iii) and claim B's "exemption-free" half hold.**

**Claim A(iv) does not hold, and it is where this round dies — the same place the last
three died.** The type guarantees the *preview* path. The *run* path never sees a
`NonBlank`: `execute_run` re-derives everything from raw `args`, and two functions in the
same file the plan edited still spell `(None, None) => String::new()` — one of which
writes that empty string straight into `run.json`'s `gsd_command` field, the exact D-30
corruption this phase exists to refuse (**CR-01**). The plan removed the third such arm,
75 lines below them, and left these two untouched with the identical now-refuted comment.
The `(None, None)` refusal it added at `:2643` fires *after* `run.json` has already been
committed at `:2514`.

**The second surviving hole is the blankness predicate split (CR-02).** This round
created two definitions of "blank" in the same commit and they disagree.
`payload::NonBlank::new` refuses zero-width/format characters; `is_plain_path_component`
uses `value.trim().is_empty()`, which U+200B and U+FEFF survive. I reproduced this against
the real library: `is_plain_path_component("\u{200b}") == true` and
`run_paths(root, "\u{200b}") == Some(..)`. `--run-id` is governed by that predicate and by
nothing else — the test at `src/driver/mod.rs:1576` says so in its own doc — and its
degenerate array is `["   ", "\t", "\n  \n"]`: the same commit that added `"\u{200b}"`
and `"\u{feff}"` to `DEGENERATE` (`:1888`) omitted them from the sibling seam's list. A
hand-narrowed enumeration, one level up, in the round that exists to kill hand-narrowed
enumerations.

**On the guards.** The rename is complete and clean (claim G: `enum CommandSource` appears
exactly once under `src/`; every remaining `CommandSource` mention in `run.rs` is prose).
`count_backlog_items`'s move is byte-for-byte behaviour-preserving — I diffed the removed
and added blocks character by character (claim F, first half). Guard eight's `Self::`
needles are indeed type-unqualified over-detection, exactly as the executor characterised
them (claim F, second half — confirmed). Claim E's reasoning is right and I would not
change it: `finish_run` before `return Err` is the same shape the spawn-failure arm at
`:2921` already establishes, and both TUI matches (`src/ui/screens/driver.rs:370`, `:466`)
have `_` fallbacks, so `"no_command_source"` renders as `?`/`Unrecorded` rather than
panicking — I read both arms.

**But three guard headers still overclaim their reach — review-WR-01's species, shipped a
third time** (WR-01, WR-02, WR-03 below). The worst is the new boundary self-check: its
`ITEM_OPENERS` list cannot match `pub(crate) `/`pub(super) `, of which this tree has 51
column-zero instances, so the region it declares "provably empty" is checked with a token
list blind to the tree's dominant item spelling. `count_backlog_items` was caught only
because it happened to be spelled bare `pub `.

**What I tried that failed to break it.** Constructing a `NonBlank` outside `mod payload`
(no route — no public field, no `From`, no serde, no descendant module); finding a fourth
`CommandSource` construction site (guard eight is honest here — 6 hits, all in
`driver/mod.rs`, 3 in `command_source`, 3 in `preview_text`); finding a legitimate value
newly refused by the tightened `is_plain_path_component` (`"20"`, `"2.1"`,
`"2026-08-19T12-00-00Z-aaaa"`, `"demo"`, `"RID"`, `"99"` all still pass — I ran them
against the real library, claim D's second half holds); finding a false refusal in the
matrix (none — the positions table at `:2166-2174` is correct and every legitimate
combination still resolves); finding that `goal::legality`'s second refusal became dead
code after the pin inversion (it did not — `untrusted::bounded` still truncates >200 chars,
so the branch remains reachable, though now untested — WR-05).

**Measured state at HEAD, verified independently by me:** `cargo clippy --all-targets` →
4 warnings, all in `src/browser.rs:131-133` and `src/project_creator.rs:146`, neither file
opened this round (confirms the 5→4 claim). `cargo test --workspace --no-fail-fast --
--test-threads=2` → **all green this run**, including both `driver_reattach` tests, which
corroborates the parent's flake finding. `cargo test --test spawn_seam_guard` → 26 passed.
A green suite is exactly what all three failed cycles also had.

---

## Critical Issues

### CR-01: `execute_run` still manufactures a blank command — twice — and one copy lands in a committed `run.json`

**Files:**
- `src/driver/run.rs:848` — `recorded_command`'s `(None, None) => String::new()`
- `src/driver/run.rs:871` — `digested_command_fragment`'s `(None, None) => String::new()`
- Call sites: `src/driver/run.rs:779` (`gsd_command: recorded_command(args)`) and
  `src/driver/run.rs:2445` (argv digest)
- Ordering: `make_run_record` at `:2448` → `JournalRun::start` writes `run.json` at `:2514`
  → the new refusal fires at `:2643`

**Status: inferred** (by direct reading of a reachable public call path and the call
ordering, plus `rtk proxy grep -n 'String::new()' src/driver/run.rs` → hits at 848 and 871).
Not reproduced end to end: driving it requires calling `execute_run` directly with an
opted-in `DrivableProject` and a real lock, which I did not do inside the repo.

**Issue.** 21-14's `must_haves` prohibition reads: *"MUST NOT manufacture a sentinel or
blank value as the 'safe' arm of an unreachable match; an unreachable state is refused with
a typed error, never represented by the exact value the system elsewhere refuses."* The
plan applied that to one arm — `src/driver/run.rs:2629`'s source match — and left two
siblings in the same file, both with the *identical* justification comment the plan
declared refuted:

```rust
// src/driver/run.rs:843-848
// Unreachable: `driver::drive` refuses a run with neither before
// anything is created. An empty string rather than a panic, because a
// detached driver that panicked here would leave a run directory with no
// terminal record, which is the crash signal D-12 reserves for a genuine
// crash.
(None, None) => String::new(),
```

`recorded_command`'s return value is `RunRecord.gsd_command` (`:779`). The empty string
**already means "field absent" on the tolerant read path (D-30)** — that is the phase's
own load-bearing premise (`21-PREMISES.md`, Premise 5). So on the very path 21-14 now
refuses, the sequence is:

1. `:2448` `make_run_record` → `gsd_command: ""`, `argv_digest` over a blank fragment;
2. `:2514` `JournalRun::start` **writes `run.json` to disk** — run directory, lock,
   `journal.jsonl`, all created;
3. `:2643` the new arm stamps `"no_command_source"` and returns `Err`.

The refusal the round added is downstream of the corruption it was commissioned to prevent.
`must_haves` truth *"no blank command can reach the spawn seam even if the arm ever becomes
reachable"* is satisfied literally (no spawn happens) while the truth it was standing in
for — no blank in a persisted record — is not.

The arm is reachable from outside the crate: `pub mod run` (`src/driver/mod.rs:109`),
`pub async fn execute_run` (`src/driver/run.rs:2261`), `pub struct DrivableProject` /
`pub fn from_registry` (`src/executor/mod.rs:137`, `:155`). That is precisely "a future
direct caller of `execute_run`" — the worry the doc at `:2612-2615` names in its own words,
one screen above two arms that still fabricate.

**Fix.** Make the state unrepresentable at the boundary rather than patching a third arm.
Either (preferred) resolve the source **once** in `drive` and thread the `CommandSource`
into `execute_run`, so `recorded_command`/`digested_command_fragment` match on a
three-variant type with no `(None, None)` to spell — which is what `CommandSource`'s own
doc already argues for and what removes the re-derivation `src/driver/mod.rs:629` warns
about — or, as the minimal change, hoist the refusal above `make_run_record`:

```rust
// src/driver/run.rs, immediately before `let record = make_run_record(...)` at :2448
// Refused BEFORE anything is created, so no run.json carrying a blank
// `gsd_command` is ever written. The stamp-then-return arm below cannot do
// this job: by the time it runs, `JournalRun::start` has committed the record.
if args.command.is_none() && args.target_phase.is_none() {
    return Err(DriveError::NoCommandSource);
}
```

and then make both `(None, None)` arms `unreachable`-by-type or, if they must stay,
return the marker constant rather than `String::new()`. Add a guard-suite assertion in
the same commit: `String::new()` may not appear as a match arm value in
`src/driver/run.rs`'s argv-derived record builders (the plan already asserted
`grep -c 'Fixed(String::new())'` → 0; the needle was too narrow by exactly two lines).

---

### CR-02: two definitions of "blank" ship in the same commit and disagree — `--run-id <U+200B>` is accepted

**Files:**
- `src/journal/mod.rs:263` — `if value.trim().is_empty()` (the tightened predicate)
- `src/driver/mod.rs:348-359` — `NonBlank::new`'s predicate (whitespace ∪ control ∪
  `U+200B..U+200F` ∪ `U+2060..U+2064` ∪ `U+FEFF`)
- Seam: `src/driver/mod.rs:861` — `if !journal::is_plain_path_component(run_id)`
- Narrowed pin: `src/driver/mod.rs:1576` — `for blank in ["   ", "\t", "\n  \n"]`
- Narrowed pin: `src/journal/mod.rs:2715-2734` — hostile list gains `"   "`, `"\t"`,
  `"\n  \n"`, `"a\nEVIL"` and **not** `"\u{200b}"` / `"\u{feff}"`

**Status: REPRODUCED** against the real library (a throwaway crate with a path dependency
on `gsd-meta-manager`, run outside the repo):

```
is_plain_path_component("\u{200b}")             = true
is_plain_path_component("\u{feff}")             = true
is_plain_path_component("\u{2060}")             = true
is_plain_path_component("\u{200b}\u{200c}\u{200d}") = true
is_plain_path_component("   ")                  = false
is_plain_path_component("a\nb")                 = false
run_paths(root, "\u{200b}").is_some()           = true
```

`char::is_whitespace()` is the Unicode `White_Space` property, which excludes U+200B,
U+FEFF and U+2060; `char::is_control()` is category `Cc`, which excludes all of them
(verified directly: `U+200B ws=false ctrl=false`, `U+FEFF ws=false ctrl=false`).

**Issue.** D-13-2 commits the phase to *"blank means NO VISIBLE INSTRUCTION, deliberately
wider than `str::trim`"* — and the doc at `src/driver/mod.rs:336-342` says so in bold,
naming U+200B as the payload that survives `trim`. The very next seam in the same commit
implements blankness as `trim`. The consequence:

- `--run-id <U+200B>` passes `src/driver/mod.rs:861`, `run_paths` builds the layout, and
  the run gets a run directory, a `run.lock`, a `journal.jsonl` and a `run.json` whose
  `run_id` field is a zero-width character. Nothing on disk or on screen names that run.
- `"abc"` and `"abc\u{200b}"` are two distinct, visually identical run ids. The record
  whose whole purpose is to be evidence of what ran cannot be read back by a human.
- The same predicate governs registry aliases (`src/main.rs:244`,
  `src/envelope/mod.rs:204`, `src/envelope/cred.rs:758`), so two visually identical
  aliases resolve to two different envelope/credential paths.

This is the fourth instance of the family, arriving through the mechanism the round was
built to close: an anti-recurrence array extended in one place and hand-copied, narrower,
into its sibling. `src/driver/mod.rs:1576`'s own doc says *"Here, the run id, the predicate
IS the only control"* — and then enumerates three of the six `DEGENERATE` literals.

**Fix.** One predicate, one place. Promote the visibility judgment out of `mod payload`
into a `pub` helper both seams call, so the two cannot drift:

```rust
// src/journal/mod.rs (or a shared `blankness` module both consume)
/// No visible instruction: every char is whitespace, a control character, or a
/// zero-width/format character. The SAME judgment `driver::payload::NonBlank`
/// applies — one predicate, so the argv seam and the path-component seam cannot
/// come to disagree about what "blank" means (round-4 CR-02).
pub fn carries_no_visible_content(value: &str) -> bool {
    !value.chars().any(|c| {
        !(c.is_whitespace()
            || c.is_control()
            || matches!(c, '\u{200b}'..='\u{200f}' | '\u{2060}'..='\u{2064}' | '\u{feff}'))
    })
}

pub fn is_plain_path_component(value: &str) -> bool {
    if carries_no_visible_content(value) {
        return false;
    }
    // ... control-char check and component check unchanged
}
```

and have `NonBlank::new` call it too. Then extend **both** enumerations from one source:
make `src/driver/mod.rs:1576` and `src/journal/mod.rs:2715` consume the same six literals
`DEGENERATE` holds (lift it to a shared test const), so a seventh blank shape lands in
every seam's pin at once. Note the deliberate asymmetry to preserve: `visibly_empty_
numbered_entry` must stay a separate expression (that one is the oracle).

---

## Warnings

### WR-01: the new boundary self-check cannot see `pub(crate)`/`pub(super)` items, while its header claims the skipped region is "provably empty"

**File:** `tests/spawn_seam_guard.rs:1991-2005` (`ITEM_OPENERS`), assertion at `:2036`,
header claim at `:2074-2078`

**Status: REPRODUCED.** Replaying the exact token list against real lines from this tree:

```
'pub(crate) fn count_backlog_items(planning_dir: &Path) -> u32 {' -> False
'pub(super) fn empty_plan() -> Self {'                            -> False
'pub(crate) struct NonBlank(String);'                             -> False
'pub(crate) mod payload {'                                        -> False
```

`rtk proxy grep -rh '^pub(crate) \|^pub(super) ' src/ --include=*.rs | wc -l` → **51**
column-zero items tree-wide that this assertion is structurally blind to.

**Issue.** The test's doc says it *"converts a SILENT UNDER-DETECTION into a LOUD
OVER-DETECTION"* and that *"the only way a skipped region can be trusted is if it is
empty, so this asserts exactly that"*. It asserts a weaker thing: that the region contains
no item whose first token is one of fourteen literals, none of which is `pub(`. `pub ` does
not prefix-match `pub(crate) `. The one violation it did catch (`count_backlog_items`) was
caught only because it happened to be spelled bare `pub fn`.

This matters more than a normal token-list gap because the `--lib -D warnings` clippy gate
does **not** catch post-marker items — `items_after_test_module` requires `cfg(test)`, so
it only fires under `--all-targets`, which the plan explicitly designates as *not* the gate.
This assertion is the only enforcement, and it has a hole the size of the tree's dominant
visibility spelling.

The header's honesty block does name "an item whose first token is outside `ITEM_OPENERS`"
as a limit — but then calls it *"bounded by the fact that the tree's production style puts
items at column zero"*, which is exactly the wrong bound: the offenders **are** at column
zero.

**Fix.**

```rust
const ITEM_OPENERS: &[&str] = &[
    "pub ", "pub(", // `pub(crate)`/`pub(super)`/`pub(in ...)` — 51 such items exist
    "fn ", "async ", "const ", "static ", "struct ", "enum ", "trait ", "impl ",
    "mod ", "type ", "use ", "unsafe ", "extern ", "union ", "macro_rules!", "#[",
];
```

and correct limit 4's bound sentence: the residual gap is an *indented* item inside a
post-marker `mod`, not "a first token outside the list".

### WR-02: `visibly_empty_numbered_entry` is never fed a degenerate payload, so the independence its doc calls "the whole value" is unexercised

**File:** `src/driver/mod.rs:1936-1943` (the doc claim), `:1944` (the fn), call sites
`:2086` and `:2220`

**Status: REPRODUCED** by call-site enumeration:
`rtk proxy grep -n 'visibly_empty_numbered_entry' src/driver/mod.rs` → exactly two calls,
both on `preview_text(&project, &source)` where `source` was built from a **realistic**
payload (`"/gsd:progress"`, `"20"`, `"get phase 22 verified"`). Since the matrix rewrite
(`:2183-2196`) refuses every `DEGENERATE` payload before `preview_text` is reached, no
blank value can ever reach this detector.

**Issue.** The doc says: *"if `payload::NonBlank` were ever loosened — dropping the
zero-width range, say — this detector would keep calling a `U+200B` entry visibly empty and
the matrix would go red. That disagreement is the whole value."* The conclusion is right
and the stated mechanism is wrong. If `NonBlank` were loosened, the matrix would go red at
`:2188`'s literal `matches!(outcome, Err(DriveError::NoCommandSource))` assertion — the
detector would never be invoked, because the loosened value would resolve `Ok` and the
degenerate loop asserts before rendering anything.

Round-3's WR-03 (the `trim` tautology) is genuinely closed — but by the **literal
`DEGENERATE` array**, not by the detector. The detector's widened character classes are
dead weight wearing the credit. Claim C is therefore half-true: the oracle is independent;
the detector is not the thing that makes it so.

**Fix.** Either correct the doc to name the mechanism that actually falsifies (the literal
array), or give the detector a case that exercises it — e.g. assert that a
hand-constructed render containing `"1. \u{200b}"` is classified visibly-empty and one
containing `"1. x"` is not, as a unit test of the detector itself. The second is cheap and
makes the claim true:

```rust
#[test]
fn the_detector_judges_visibility_rather_than_whitespace() {
    assert!(visibly_empty_numbered_entry("1. \u{200b}").is_some(),
        "a zero-width tail is visibly empty — `tail.trim()` would disagree, which is why \
         this predicate is written separately from production's");
    assert!(visibly_empty_numbered_entry("1. x").is_none());
}
```

### WR-03: guard eight's non-vacuity counts lines, not distinct needles, so limit 1's stated bound is weaker than claimed

**File:** `tests/spawn_seam_guard.rs:2367` (`COMMAND_SOURCE_VARIANT_COUNT: usize = 3`),
assertion at `:2439-2448`, claim at `:2286-2291`

**Status: inferred** (code reading).

**Issue.** Limit 1 says an unanticipated spelling is *"bounded by the per-function
`contributed >= COMMAND_SOURCE_VARIANT_COUNT` non-vacuity below, which fails if a needle
stops matching a site that still names all three."* `contributed` counts *lines attributed
to the function*, not *distinct needles matched*. A function contributing four lines via
two needles satisfies `>= 3` while a third needle has gone blind. Today the arithmetic is
3-and-3 so the bound happens to bite; it stops biting the moment `command_source` grows a
fourth construction line.

**Fix.** Assert per-needle coverage rather than a line count:

```rust
for (file, function) in COMMAND_SOURCE_ALLOWLIST {
    for needle in COMMAND_SOURCE_VARIANTS.iter().take(3) { // the type-qualified forms
        let matched = hits.iter().any(|(path, _, enclosing, line)| {
            path == file && enclosing.as_deref() == Some(*function) && line.contains(needle)
        });
        assert!(matched, "{file}::{function} no longer names {needle} — the needle went blind");
    }
}
```

### WR-04: `ALL_VARIANT_NAMES` is a hand-maintained const; a fourth `CommandSource` variant no column produces is not caught

**File:** `src/driver/mod.rs:1901` (`const ALL_VARIANT_NAMES: [&str; 3]`), coverage
assertion at `:2225-2243`

**Status: inferred** (code reading).

**Issue.** The `must_haves` truth claims *"column-to-variant coverage is asserted against
the single `ALL_VARIANT_NAMES` const that `variant_name`'s wildcard-free match anchors"*.
The anchoring is one-directional: adding a fourth `CommandSource` variant is a compile
error in `variant_name` (`:1918`), which forces an author to *classify* it — but nothing
forces them to extend `ALL_VARIANT_NAMES`. If the new variant is produced by no argv
position, `resolved` still holds three names, `expected` still holds three names, and the
matrix stays green while a whole variant goes unexercised. That is the "list a human must
remember to extend" shape one level up from the exemption this round removed.

The **column** axis genuinely is structural (a fourth argv parameter breaks every
`PositionBuilder` at compile time — I confirmed the `for<'a> fn(&'a str) -> (Option<&'a
str>, Option<&'a str>, Option<&'a str>)` signature at `:2163`). The **row** axis is not.

**Fix.** Tie the const's length to the variant set through the same wildcard-free match:

```rust
/// One representative of every variant, built through the exhaustive match below
/// so a fourth variant is a compile error HERE too — not merely in `variant_name`.
fn one_of_each() -> [CommandSource; ALL_VARIANT_NAMES.len()] {
    let sample = payload::NonBlank::new("x").expect("one visible character");
    let all = [
        CommandSource::Command(sample.clone()),
        CommandSource::Routed(sample.clone()),
        CommandSource::Goal(sample),
    ];
    // Wildcard-free: a fourth variant fails to compile here.
    for source in &all {
        match source {
            CommandSource::Command(_) | CommandSource::Routed(_) | CommandSource::Goal(_) => {}
        }
    }
    all
}
```

then assert `one_of_each().map(|s| variant_name(&s))` sorted equals `ALL_VARIANT_NAMES`
sorted, and assert every name in `ALL_VARIANT_NAMES` appears in `resolved`.

### WR-05: the inverted pin left `goal::legality`'s second refusal reachable but untested

**Files:** `src/driver/goal.rs:697-702` (the `untrusted::bounded` refusal),
`tests/driver_goal_seam.rs:1368-1395` (the inverted premise),
`tests/driver_goal_seam.rs:1358-1363` (`HOSTILE_PHASE_TOKENS`)

**Status: inferred** (code reading + `rtk proxy grep -rn 'PhaseNotPlainComponent' tests/
src/driver/goal.rs`, which finds no test reaching the second site; and
`rtk proxy grep -rn 'repeat(' tests/driver_goal_seam.rs src/driver/goal.rs`, whose single
hit at `goal.rs:1386` is a *command* fixture carrying `\n`).

**Verdict on claim H: the resolution is right, and the executor undersold the cost.**
Inverting the assertion rather than deleting it was correct, and recording it as a
deviation with the falsified premise named in place is the right house behaviour. But the
stated cost — *"the test can no longer distinguish which layer refused"* — understates it.
All four `HOSTILE_PHASE_TOKENS` carry C0/C1 control characters (`\u{1b}`, `\n`, `\r`,
`\u{9b}`; `char::is_control()` is true for all four), so every one of them is now refused
at `goal.rs:667` by the predicate. Nothing in the suite reaches `:697` any more.

The branch is not dead — `untrusted::bounded` also truncates at
`MAX_UNTRUSTED_FIELD_CHARS = 200` (`src/driver/untrusted.rs:275-286`), so a >200-character
control-free phase token still reaches it — but that path has no fixture. A defence-in-depth
layer with zero coverage is a layer nobody will notice breaking.

**Fix.** Add one fixture that reaches the second refusal, in the same test:

```rust
// The one shape that still reaches the SECOND refusal after 21-13: no control
// characters (so the predicate passes it) and longer than
// `untrusted::MAX_UNTRUSTED_FIELD_CHARS` (so `bounded` shortens it). Without this
// the goal layer's own bound is untested rather than merely redundant.
let overlong = format!("21{}", "x".repeat(400));
assert!(gsd_meta_manager::journal::is_plain_path_component(&overlong));
let refusal = goal::legality(/* ... a step naming `overlong` ... */)
    .expect_err("a phase token `bounded` would shorten is refused, never truncated");
assert_eq!(refusal.reason(), goal::GoalReason::PhaseNotPlainComponent);
```

---

## Info

### IN-01: a blank `--goal` supplied beside `--command`/`--target-phase` is never validated and is written to `run.json`

**Files:** `src/driver/mod.rs:479-492` (the two arms that ignore `goal`),
`src/driver/run.rs:763` (`goal: args.goal.clone().unwrap_or_default()`)

**Status: inferred** (code reading). `command_source(Some("x"), None, Some("   "))` takes
the `(Some, None)` arm and never touches `goal`; `make_run_record` then records `"   "`
verbatim.

21-13's `must_haves` truth says a payload carrying no visible instruction *"is refused …
in EVERY argv position (`--command`, `--target-phase`, `--goal`)"*. That is true for each
flag **alone** — the matrix's positions table only builds one-flag tuples — and false for
`--goal` beside another source, where the doc's "recorded prose and nothing more" means
the prose is recorded unvalidated. Impact is contained (`from_argv_goal` refuses to
decompose it, `src/ui/screens/driver.rs:755` renders a `trim`-blank goal as absent), so
this is Info rather than a Warning; but it is a cell the matrix's shape *cannot* express,
which is the property the round claimed to establish.

Fix: run `args.goal` through `NonBlank::new` for the record even when it is not the source
— `None` → record no goal at all rather than a blank one — or add a two-flag row to the
matrix that pins the current behaviour deliberately.

### IN-02: a fourth blankness predicate survives at `from_argv_goal`

**File:** `src/driver/run.rs:1945-1948` — `let goal = args.goal.as_deref()?.trim(); if
goal.is_empty() { return None; }`

**Status: inferred.** Harmless today (`command_source` refuses a solitary blank goal first,
including the zero-width forms), and it is defence in depth rather than a bug. Worth naming
because the tree now carries four expressions of "blank" — `NonBlank::new`,
`is_plain_path_component`'s `trim`, `visibly_empty_numbered_entry::visible`, and this one —
and only the third is *deliberately* separate. CR-02's fix should fold this one in too.

### IN-03: guard eight's import assertion misses a type alias and a wrapped `use`

**File:** `tests/spawn_seam_guard.rs:2506-2524`

**Status: inferred.** The assertion requires `trimmed.starts_with("use ")` **and**
`line.contains("CommandSource::")`. Two cheap evasions slip past: a multi-line `use` whose
variant path sits on a continuation line, and `use crate::driver::CommandSource as CS;`
followed by `CS::Command(x)` — an alias, which contains no `CommandSource::` and matches no
needle. Both are covered generically by limit 1 ("an unanticipated spelling"), so the
header is not dishonest; but the alias route is cheaper than the variant import the
assertion was written to close, and naming it costs one sentence. A `use ... as ` line
whose left side names `CommandSource` would be the natural second needle.

---

## Verified and clean (recorded so the verifier need not re-derive)

- **A(i)–(iii):** `mod payload` nested at `src/driver/mod.rs:324`; private field at `:331`;
  exactly one `impl NonBlank` at `:333` with exactly `new` and `as_str`; derives are
  `Debug, Clone, PartialEq, Eq` only — no serde, no `From`, no `Deref`, no `AsRef`, no
  `into_inner`. All three variants carry it (`:395`, `:398`, `:402`).
- **B (exemption-free half):** the matrix at `:2178-2196` has one uniform loop, no `Ok`
  branch for a degenerate cell, no by-name per-column sweep, and no exemption comment
  (`rtk proxy grep -c "deliberately absent" src/driver/mod.rs` → 0). No legitimate
  combination is falsely refused: the realistic half at `:2199-2224` and the
  one-visible-character boundary loop at `:2245-2256` (`"x"`, `" x "`, `"\u{200b}x"`,
  `"x\u{feff}"`) all resolve.
- **D (second half):** every previously accepted shape still passes the tightened
  predicate — reproduced against the real library for `"20"`, `"2.1"`,
  `"2026-08-19T12-00-00Z-aaaa"`, `"demo"`, `"99"`, `"RID"`.
- **E:** adjudicated **correct**. `finish_run(..., "no_command_source", ...)` then
  `return Err` mirrors the spawn-failure arm at `src/driver/run.rs:2921-2929` exactly; the
  arm is unreachable through `drive` (goal runs get `args.target_phase` written back at
  `src/driver/mod.rs:975`); the label is safe — `src/ui/screens/driver.rs:370` and `:466`
  both carry `_` fallbacks, which I read. Stamping beats a bare return for the reason
  given. **This does not rescue the path** — see CR-01 for what the stamp arrives too late
  to prevent.
- **F (first half):** `count_backlog_items`'s move is byte-for-byte identical (removed and
  added blocks compared character by character in the diff). `clippy --all-targets` 5 → 4,
  remaining four in `src/browser.rs:131-133` and `src/project_creator.rs:146` — both files
  untouched this round, confirmed by the diff stat.
- **F (second half):** the executor's characterisation of the `Self::` needles is
  **accurate** — `"Self::Command("`, `"Self::Routed("`, `"Self::Goal("` name no type, the
  direction is over-detection (loud), limit 5 names it, and no such enum exists today.
- **G:** rename complete. `rtk proxy grep -rn 'enum CommandSource' src/` → 1 hit
  (`driver/mod.rs`); every remaining `CommandSource` string in `run.rs` is a doc comment.
  The single-declaration assertion plus the run.rs-specific pin at
  `tests/spawn_seam_guard.rs:2482-2495` is sound: the pair closes the one way a bare count
  of 1 could be satisfied while reintroducing the collision, and `source_files()` walks
  `src/` only (`tests/spawn_seam_guard.rs:219-222`), so the guard's own literal
  `"enum CommandSource"` at `:2462` does not self-trip.
- **Suite state:** `cargo test --workspace --no-fail-fast -- --test-threads=2` fully green
  in my run (both `driver_reattach` tests passed), `cargo test --test spawn_seam_guard`
  26/26, `cargo clippy --all-targets` 4 pre-existing warnings. Per the review context, the
  `driver_reattach` flake is pre-existing and is **not** reported as a finding.

---

_Reviewed: 2026-08-21_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
_Round: 4 — supersedes round 3, preserved at `d4874ec`_
