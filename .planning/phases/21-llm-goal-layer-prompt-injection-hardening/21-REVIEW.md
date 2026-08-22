---
phase: 21-llm-goal-layer-prompt-injection-hardening
round: 5
reviewed: 2026-08-22T18:20:00Z
depth: deep
diff_base: 199d334
head: 1f07923
previous_round: "Round 4's 21-REVIEW.md is preserved in git at c7bcafb (`git show c7bcafb:.planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-REVIEW.md`). This file replaces it."
files_reviewed: 23
files_reviewed_list:
  - src/app.rs
  - src/cli.rs
  - src/driver/mod.rs
  - src/driver/run.rs
  - src/journal/mod.rs
  - src/lib.rs
  - src/main.rs
  - src/test_support.rs
  - src/text.rs
  - src/ui/screens/driver.rs
  - tests/driver_dry_run.rs
  - tests/driver_escalation_cap.rs
  - tests/driver_goal_seam.rs
  - tests/driver_inbox.rs
  - tests/driver_iteration_loop.rs
  - tests/driver_lock.rs
  - tests/driver_optin.rs
  - tests/driver_rate_limit.rs
  - tests/driver_refusal_record.rs
  - tests/driver_tracer.rs
  - tests/envelope_wiring.rs
  - tests/journal_run_paths.rs
  - tests/spawn_seam_guard.rs
files_read_but_unchanged_this_round:
  - src/registry.rs
  - src/driver/spawn.rs
  - src/driver/goal.rs
  - src/envelope/mod.rs
findings:
  critical: 1
  warning: 6
  info: 5
  total: 12
status: issues_found
---

# Phase 21 Round 5: Code Review Report

**Reviewed:** 2026-08-22T18:20:00Z
**Depth:** deep (cross-file; import graph, call chains, end-to-end binary reproduction)
**Diff base:** `199d334` → `1f07923` (10 commits, 2 plans)
**Round 4's review:** preserved at `c7bcafb`; this file replaces it.
**Status:** issues_found — **1 Critical, reproduced end to end against the built binary.**

## Summary

Round 5's three structural anchors are real and I could not break them.
`DriveArgs`'s six argv string fields are `payload::NonBlank` (verified: 0 raw,
9 `payload::NonBlank` mentions in the struct body); `NonBlank`'s privacy was
not widened (exactly `new` + `as_str`, private tuple field, no `From`, no
`into_inner`, no serde, no `Deref`/`AsRef`); `from_argv`'s destructure has no
`..`; `execute_run` resolves its source at `run.rs:2442-2447`, above
`establish_own_group`, the envelope, the lock and `JournalRun::start`; both
`(None, None) => String::new()` arms are gone and `iteration_source` is total
over `IterationSource` without fabricating a case; the `(None, None)` goal arm
is spelled as the explicit `match` guard eight's needle requires; and
`is_plain_path_component` kept both the `is_control()` refusal and the
`Component::Normal` traversal check while replacing only the `trim` short-circuit.
`post_marker_offenders` carries the `scanned_files` floor out of the extraction
and the synthetic control calls that same function. All four literal-grep
acceptance criteria read the numbers the plans specify.

**And the fifth defect of the family is there.** It is not on `DriveArgs`. It is
on the *other* argv path that ends in a filesystem path: `gsd-meta-manager add
<path> <alias>`. `registry::add_project` judges alias blankness with
`alias.is_empty() || alias.contains(char::is_whitespace)` — a **fourth**
production spelling of "blank", untouched by the unification, and precisely the
`trim`-class predicate CR-02 was about. I registered an alias of one `U+200B`
and two visually identical aliases pointing at two different projects against
the binary built from `1f07923`. That falsifies 21-15's must-have truth 4
("exactly ONE production spelling of the invisibility judgment exists") and
truth 5 ("the same predicate governs registry aliases … two visually identical
aliases can no longer resolve to two different envelope paths"), and leaves
threat T-21-15-02 (severity **high**, disposition **mitigate**) only half
mitigated.

Two of the round's own anti-recurrence mechanisms also do less than their docs
say: `one_of_each` produces **no compile error at all** for a fourth
`CommandSource` variant (WR-01, reproduced), and guard nine's field scanner is
blind to `pub(crate)`, to `Box<str>`/`Cow<str>`/`&str`/`OsString`, and to a
trailing `//` comment (WR-02, reproduced) — including the exact `pub(`
spelling the sibling guard in the same file was widened for four commits later.

Gates at HEAD, measured independently: `cargo build --all-targets` clean;
`cargo clippy --lib -- -D warnings` clean; `cargo test --workspace
--no-fail-fast -- --test-threads=2` exit 0, no failures, no flake this run.
A green suite is exactly what all four previous cycles had.

---

## Structural anchors — what I tried that failed to break them

Negative evidence, since four-for-four makes it the most useful thing here.
Everything below was measured at `1f07923` through `rtk proxy`.

| Attempt | Result |
|---|---|
| `sed '/pub struct DriveArgs {/,/^}/p' \| grep -c ': String\|Option<String>'` | **0**, control `grep -c 'payload::NonBlank'` → **9**. Extraction non-vacuous. |
| Escape hatch on `NonBlank` — `pub` field, `From<String>`, `into_inner`, serde, `Deref`, `AsRef` | **None.** `src/driver/mod.rs:546-592` declares `pub struct NonBlank(String)` with a private field and exactly `new`/`as_str`; derives are `Debug, Clone, PartialEq, Eq`. `grep -rn "into_inner\|impl Deref\|impl AsRef"` over `src/` finds nothing on this type. |
| A blank reaching `run.json` through `make_run_record` | Cannot: `goal`/`target_phase` map from `Option<NonBlank>`; `gsd_command` is `recorded_command(&IterationSource)`, total over two variants (`run.rs:910-915`, `938-943`). |
| A write before `execute_run`'s refusal | `command_source`+`iteration_source` at `run.rs:2442-2447`; `establish_own_group` at `:2449`, envelope at `:2492`, lock `:2609`, `JournalRun::start` `:2626`. Nothing above `:2442` writes. Corroborated: `~/.local/share/gsd-meta-manager/envelope/demo/` mtime is unchanged (2026-08-21) across a full `--test-threads=2` suite run today. |
| A `Goal` source fabricating a case in `iteration_source` | `run.rs:734-742` — three arms, `Goal(_) => Err(NoCommandSource)`. No `_` arm, no manufactured value. |
| The point-free `CommandSource::Goal)` spelling defeating guard eight | Not present: `mod.rs:713-716` is the explicit `match goal { Some(goal) => Ok(CommandSource::Goal(goal.clone())), None => Err(...) }`. Guard eight's per-needle floor passes with all three type-qualified needles matching. |
| `is_plain_path_component` loosened by the unification | `journal/mod.rs:268-294` keeps `is_control()` (`:283`) and `Component::Normal` + single-component + `name == value` (`:286-293`). Only the `trim` short-circuit moved. |
| `post_marker_offenders` dropping the non-vacuity floor | It returns `(offenders, scanned_files)`; live assertion asserts `scanned_files >= 10` (`spawn_seam_guard.rs:2166`); the control calls the **same fn** and asserts `scanned_files == 1`. `grep -c "post_marker_offenders("` → 3. |
| `from_argv` admitting a rest-pattern / builder / `Default` | `mod.rs:409-424` destructures all twelve fields by name; `mod.rs:454-469` constructs all twelve. Confirmed by 21-16's live planted-field experiment (`E0063`). |
| The four pre-existing `=> String::new()` hits being on a record path | They are not. `executor/outcome.rs:485` is inside a `#[cfg(test)]` envelope-builder helper; `archive.rs:154` is kebab→Title conversion for a display name; `ui/screens/normal.rs:680` is a workstream cue suffix; `ui/screens/detail.rs:3524` is a docs-browser header. The 21-15 SUMMARY's disclosure is **correct**. |
| The three literal-grep acceptance criteria | `"pub("` → 1; `production style puts items at column zero` → 0; `scanned_files >= 10` → 1; `would keep calling` in `src/driver/mod.rs` → 0. All as specified. |

---

## Critical

### CR-01: `registry::add_project` is a fourth, weaker production spelling of blankness — argv aliases carrying no visible content register, and look-alike aliases resolve to different projects and different envelope roots

**Status: REPRODUCED** end to end against `target/debug/gsd-meta-manager` built from `1f07923`.

**Files:**
- `src/registry.rs:14-20` (`add_project`) and `src/registry.rs:65-71` (`add_project_unchecked`) — the predicate
- `src/main.rs:35-57` — the `Add` arm and its exact-match duplicate check at `:45`
- Falsified claims: `21-15-PLAN.md` `must_haves.truths[4]` and `[5]`; `21-15-SUMMARY.md` coverage row "One production spelling of the invisibility judgment"; threat `T-21-15-02` (high / mitigate)

**Issue.** 21-15's truth 4 states that exactly one production spelling of the
invisibility judgment exists and enumerates its four consumers
(`NonBlank::new`, `is_plain_path_component`, `goal_or_none`, `goal_lines`).
Truth 5 states that "the same predicate governs registry aliases … so two
visually identical aliases can no longer resolve to two different envelope
paths." There is a fifth production judge of alias blankness that the round did
not enumerate, and it is the one that decides what goes *into* the registry:

```rust
// src/registry.rs:14-20 — and again, verbatim, at :65-71
if alias.is_empty() { bail!("Alias cannot be empty"); }
if alias.contains(char::is_whitespace) { bail!("Alias cannot contain whitespace"); }
```

`U+200B` is neither empty nor `char::is_whitespace`. This is the exact
`trim`-class weakness CR-02 named, spelled differently, on the argv path
`gsd-meta-manager add <path> <alias>` — a path the round's own threat table
lists under "registry aliases → `is_plain_path_component`" and treats as
governed.

**Reproduction (`1f07923`, `--config` a scratch file).** Every literal
zero-width character in the transcripts below is rendered as `<U+200B>` so this
review file itself carries none — a document about invisible characters must not
smuggle any into the next reader's context. To re-run, substitute
`$(printf '\u200b')` (bash/zsh) or `(printf '\u200b')` (fish) wherever
`<U+200B>` appears.

```
$ gsd-meta-manager --config $CFG add .../projA demo
Added project 'demo' at .../projA
$ gsd-meta-manager --config $CFG add .../projB "demo$(printf '<U+200B>')"
Added project 'demo<U+200B>' at .../projB
$ gsd-meta-manager --config $CFG add .../projA "$(printf '<U+200B>')"
Added project '<U+200B>' at .../projA

$ gsd-meta-manager --config $CFG list
ALIAS                PATH                                               ADDED
------------------------------------------------------------------------------
demo                 .../projA                                          2026-08-22...
demo<U+200B>                .../projB                                          2026-08-22...
<U+200B>                    .../projA                                          2026-08-22...

$ python3 -c "import json; ..."   # the same file, escaped
'demo<U+200B>' -> .../projB
'<U+200B>'     -> .../projA
'demo'       -> .../projA
```

Two rows of `list` are byte-different and pixel-identical; a third has an empty
`ALIAS` column. The duplicate guard at `main.rs:45`
(`config.projects.contains_key(&alias)`) compares exact bytes, so the look-alike
is not reported as a duplicate.

Both look-alikes are then **fully drivable and indistinguishable downstream**:

```
$ ... drive demo --command '/gsd:progress' --dry-run
Error: the project `demo` has not opted in to being driven; ...
$ ... drive "demo$(printf '<U+200B>')" --command '/gsd:progress' --dry-run
Error: the project `demo<U+200B>` has not opted in to being driven; ...
```

`NonBlank::new("demo\u{200b}")` → `Some` (a `d` is visible), the registry lookup
hits, and `envelope::envelope_dir_in` → `is_plain_path_component("demo\u{200b}")`
→ `true` (independently reproduced: see the table under WR-05). So the two
aliases carry **separate opt-in records and separate envelope roots**
(`<envelope>/demo/` and `<envelope>/demo\u{200b}/`) while every render an
operator sees — `list`, the dashboard, the refusal text, `run.json`'s `alias`
— shows the same four characters. That is exactly the harm T-21-15-02 describes
("two visually identical aliases resolve to different envelope paths") and the
harm the round claims to have removed. The driver being authorised here is "an
autonomous agent with git and push rights"; approving an opt-in for the alias
you *think* you are looking at is the whole trust act.

The wholly-invisible alias adds a second, round-attributable symptom (see WR-06).

**Fix.** Route both registration predicates through the shared judgment, and
close the look-alike case at registration rather than downstream:

```rust
// src/registry.rs, add_project AND add_project_unchecked
if !crate::text::carries_visible_content(alias) {
    bail!("Alias must carry at least one visible character");
}
// The look-alike half: an alias is a directory name and an identity a human
// reads back. An embedded zero-width/format character makes two aliases render
// identically while naming two projects, two opt-ins and two envelope roots.
if alias.chars().any(|c| {
    matches!(c, '\u{200b}'..='\u{200f}' | '\u{2060}'..='\u{2064}' | '\u{feff}')
}) {
    bail!("Alias must not contain zero-width or format characters: two aliases \
           that render identically would name two different projects");
}
if alias.contains(char::is_whitespace) { bail!("Alias cannot contain whitespace"); }
```

The second clause belongs in one place — put it beside `carries_visible_content`
in `src/text.rs` (e.g. `fn carries_invisible_formatting`) so it does not become
the *fifth* spelling. `is_plain_path_component` should then consume it too
(see WR-05), which closes the run-id and phase-token halves in the same change.
Add a matrix-style pin over `test_support::DEGENERATE` plus the look-alike pair.

---

## Warnings

### WR-01: `one_of_each` is not a second anchor — a fourth `CommandSource` variant is a compile error in exactly ONE place, and the set-equality pin stays green with the variant unswept

**Status: REPRODUCED** (standalone `rustc` program mirroring the shipped shape).

**File:** `src/driver/mod.rs:2058-2084` (the doc and the fn), `:2086-2127` (the pin).
**Falsified:** `21-16-PLAN.md` `must_haves.truths[3]` — "a fourth variant is a
compile error in TWO places (`variant_name`, `one_of_each`)" — and the fn's own
doc at `:2067-2072`: "a fourth variant is a compile error HERE too, in two ways:
the array's declared length and the missing construction."

**Issue.** `one_of_each` constructs three variants into a `[…; 3]`. Neither
mechanism it claims exists. An array of declared length 3 holding 3 entries is
satisfied whatever the enum's arity; there is no exhaustiveness constraint of
any kind on a function that merely *constructs* values. Only `variant_name`'s
wildcard-free `match` errors.

So the sequence a fourth variant actually produces is: `variant_name` fails to
compile → the editor adds one arm → `one_of_each` still builds 3 →
`ALL_VARIANT_NAMES.len() == 3 == built.len()` → the sorted name sets are equal
→ `all_variant_names_matches_the_variant_set_in_both_directions` **passes**, and
the matrix's coverage assertion at `:2317` (`for expected in ALL_VARIANT_NAMES`)
sweeps three of four. That is pass-5 warning 4, verbatim, in the test written to
close it.

Reproduction output (four-variant enum, `one_of_each` unchanged):

```
all_variant_names_matches_the_variant_set_in_both_directions: PASSES
  ...with a FOURTH variant ("Resumed") constructed nowhere and swept nowhere.
```

**Fix.** Make the pairing exhaustive by *destructuring*, so the compiler is the
enumerator rather than a length literal:

```rust
fn one_of_each() -> Vec<(&'static str, CommandSource)> {
    // Exhaustive by construction: adding a variant makes this `match` a compile
    // error, and the arm it forces is the construction the sweep needs.
    fn named(source: CommandSource) -> (&'static str, CommandSource) {
        let name = match &source {
            CommandSource::Command(_) => "Command",
            CommandSource::Routed(_) => "Routed",
            CommandSource::Goal(_) => "Goal",
        };
        (name, source)
    }
    vec![
        named(CommandSource::Command(visible("x"))),
        named(CommandSource::Routed(visible("x"))),
        named(CommandSource::Goal(visible("x"))),
    ]
}
```

That still does not force the *vec* to grow. The honest anchor is a
`#[non_exhaustive]`-style trick or a `const ARITY` derived from a
wildcard-free `match` returning an index — but at minimum the doc and the plan
truth must stop claiming a compile error that does not exist, per this plan's
own prohibition 1.

### WR-02: guard nine's field scanner is blind to `pub(crate)`, to every blank-capable string type that is not spelled `String`, and to a trailing `//` comment — and its non-vacuity floor does not bind

**Status: REPRODUCED** (`raw_string_argv_fields` + `drive_args_body` +
`names_bare_string` extracted verbatim from `tests/spawn_seam_guard.rs:2727-2808`
into a scratch binary and fed planted declarations).

**Files:** `tests/spawn_seam_guard.rs:2743` (`!trimmed.starts_with("pub ")`),
`:2749` (`!joined.ends_with(',')`), `:2790-2804` (`names_bare_string`),
`:2918-2924` (the floor's own `starts_with("pub ")` filter),
`:2694-2712` (the limits block).

**Issue.** Guard nine is the round's textual witness for "a seventh argv field
cannot be added raw." Measured against planted declarations, the shapes it
misses are:

| Planted field on `DriveArgs` | Guard nine |
|---|---|
| `pub goal_file: String,` | reported |
| `pub goal_file: Option<String>,` | reported |
| `pub(crate) goal_file: Option<String>,` | **SILENT** |
| `pub(super) goal_file: Option<String>,` | **SILENT** |
| `pub goal_file: Option<Box<str>>,` | **SILENT** |
| `pub goal_file: Option<Cow<'static, str>>,` | **SILENT** |
| `pub goal_file: Option<&'static str>,` | **SILENT** |
| `pub goal_file: Option<OsString>,` | **SILENT** (by design) |
| `pub goal_file: Option<String>, // seventh` (last field) | **SILENT** |

And the floor does not save it. Fed a full twelve-field `DriveArgs` body with
one extra `pub(crate) goal_file: Option<String>`:

```
D offenders=[] field_lines=12 protected=6 -> guard verdict: PASSES (silent)
```

Three separate problems:

1. **`pub(`.** This is the *exact* spelling `ITEM_OPENERS` was widened for in
   commit `2eefa73`, four commits after guard nine landed in `40a137e`, because
   "the tree carries 51 column-zero items in exactly that spelling — the
   dominant restricted-visibility style" (`spawn_seam_guard.rs:1993-1997`). One
   guard in this file learned the lesson this round; the guard added in the same
   round did not. What currently prevents a `pub(crate)` field on `DriveArgs` is
   an *accident*: twelve integration-test files still build `DriveArgs { … }`
   literally (`grep -rc "DriveArgs {" tests/*.rs`), so the field would break
   them. That is coincidental reliance — the same thing pass 5 flagged as
   T-21-15-04 for `alias` and `approved_plan`.
2. **`names_bare_string` only knows `String`.** `Box<str>`, `Cow<str>`,
   `&'static str` and `OsString` all hold `""` and all pass. `OsString` is not
   hypothetical: `DriveArgs` **already carries** `claude_args: Vec<OsString>`,
   and `OsString` is the type argv actually arrives in. None of these is named
   in the limits block, which lists only the `type`-alias gap (limit 1) and the
   declaration-style gap (limit 2) — so this is a silent under-detection the
   guard's header does not disclose, which is 21-16's own prohibition 1.
3. **The trailing-comment / last-field interaction.** `joined.ends_with(',')`
   is false for `pub goal_file: Option<String>, // seventh`; the accumulator
   holds it pending, `drive_args_body` breaks at the closing `}`, and the
   pending declaration is dropped. Mid-struct it is reported but *swallows the
   next field line* into the joined text, so attribution is wrong there too.

**Fix (all three, small):**

```rust
// 1. tests/spawn_seam_guard.rs:2743 and :2921
let is_field = (trimmed.starts_with("pub ") || trimmed.starts_with("pub("))
    && trimmed.contains(':');

// 2. widen the needle set, and say so in the limits block
fn names_a_string_type(text: &str) -> bool {
    ["String", "str"].iter().any(|needle| names_word(text, needle))
        && !text.contains("OsStr")   // …or drop the exemption and allowlist
}                                     //    claude_args by field NAME instead

// 3. strip a trailing line comment before the ends_with(',') test, and flush
//    `pending` when the body ends
let trimmed = trimmed.split("//").next().unwrap_or(trimmed).trim_end();
```

Then add control arms for `pub(crate)`, `Option<Box<str>>` and the
trailing-comment shape beside the existing planted-field control at `:2817`, so
each new claim is a measured fact.

### WR-03: the CR-01 tracer's "nothing created" assertion cannot see the envelope half of the property it exists to hold

**Status: INFERRED** (mechanism verified by reading; not reproduced, because
reproducing it requires reordering production code).

**Files:** `src/driver/run.rs:3788-3797` (the assertion), `:2418-2447` (the
property it guards), `src/envelope/mod.rs:175-181` (`envelope_root`).

**Issue.** The fix's own doc says a run with nothing executable "now leaves no
group moved, **no envelope**, no lock file, no run directory, no record and no
journal" (`run.rs:2431-2434`). The test asserts one thing:

```rust
let runs_root = root.path().join(".planning/meta-manager");
assert!(!runs_root.exists(), …);
```

`establish_envelope` writes to `envelope_root()` —
`GSD_MM_ENVELOPE_ROOT` or `dirs::data_local_dir()/gsd-meta-manager/envelope/`
— which is **outside the project root by construction** (`envelope/mod.rs:13`
says so in as many words). `hooks::write_exclude_block` would write to
`<project>/.git/info/exclude`, but the fixture's temp project has no `.git`
(`opted_in_entry` creates only `.planning`), so that branch is skipped too.

Consequence: move `let argv_source = command_source(…)` from `run.rs:2442` back
below `establish_envelope` at `:2492` — a plausible refactor, and the exact
regression the fix exists to prevent — and
`a_run_with_no_command_source_writes_nothing_before_refusing` **stays green**
while every `cargo test --lib` writes hook stubs, `settings.json`, `gitconfig`
and `askpass` into the developer's real `~/.local/share/gsd-meta-manager/envelope/demo/`.
The 21-15 SUMMARY's residual 4 records that the red-arm commit did exactly that
("writes an envelope into the real user data directory unless
`GSD_MM_ENVELOPE_ROOT` is set") — the executor saw the interaction and did not
turn it into an assertion.

**Fix.** Point the test at a temp envelope root and assert its absence in the
same breath:

```rust
let envelope_root = root.path().join("envelope-root");
std::env::set_var(crate::envelope::ENVELOPE_ROOT_ENV, &envelope_root);
// … call execute_run …
assert!(
    !envelope_root.exists(),
    "a refused run must establish no envelope either — the source is resolved \
     ABOVE `establish_envelope`, and an assertion scoped to .planning/ cannot \
     see that half of the property. Found:\n{}",
    surviving_artifacts(&envelope_root)
);
```

(Use a serialised env guard, or thread the root through the fixture, so
parallel tests do not race on the process-wide variable.)

### WR-04: the standing guard for prohibition 2 scans `src/` only while its name and its message claim tree-wide uniqueness — and three different hand-copied blank-shape subsets live in `tests/`

**Status: REPRODUCED** (grep).

**Files:** `tests/spawn_seam_guard.rs:3067-3097`
(`the_degenerate_payload_set_is_spelled_in_exactly_one_place`), `:219-230`
(`source_files`, rooted at `SRC_ROOT`); offenders at
`tests/driver_dry_run.rs:611`, `tests/driver_dry_run.rs:704`,
`tests/driver_goal_seam.rs:335`.

**Issue.** This is the adjudication the executor asked for (21-15-SUMMARY
residual 1), and the answer is: **yes, the check is narrower than its words, and
it matters.**

```
tests/driver_dry_run.rs:611:    for blank in ["", "   "] {                       # 2 of 6
tests/driver_dry_run.rs:704:    for blank in ["   ", "\t", "\n  \n", ""] {       # 4 of 6
tests/driver_goal_seam.rs:335:  for blank in ["", "   \t ", "\u{200b}", "\u{feff}"] {  # a DIFFERENT 4 of 6
```

Three pins, three different hand-picked subsets, no two the same — which is the
signature the whole round exists to remove. Two of the three are pins this round
**retargeted** (`a_blank_command_is_refused_in_preview_and_in_a_real_run`,
`a_blank_target_phase_is_refused_in_preview_and_in_a_real_run`), so they are
new assertions about the new boundary that omit the two zero-width shapes the
round is about. The guard written to make prohibition 2 enforceable is scoped to
`SRC_ROOT` and reports green; its own failure message says "a blank-shape
payload list is spelled outside `src/test_support.rs`" while three are.

This is *coverage*, not a hole — the in-crate 7×6 matrix pins every position
against every shape through `from_argv`, so nothing is unprotected. But a guard
whose message overclaims its scan is precisely the mechanism 21-16 exists to
close (its prohibition 1), and it is the third consecutive round in which a
guard has been green about a region it never read.

**Fix.** The executor identified the one-word fix and declined it on plan
literalism. Take it:

```rust
// src/lib.rs — a single `const` of six `&'static str`; no runtime cost, and
// `#[cfg(test)]` is what scoped the prohibition's check narrower than its words.
pub mod test_support;
```

Then have the three pins consume `gsd_meta_manager::test_support::DEGENERATE`,
and widen the guard's scan to `src/` **and** `tests/` (with `tests/spawn_seam_guard.rs`
allowlisted for `DEGENERATE_WITNESS`, which it must spell to search for it), or
narrow its name and message to `src/` and add a limits block saying so.

### WR-05: `is_plain_path_component` still accepts embedded zero-width characters, so two run directories, two phase tokens and two aliases can render identically — the half of CR-02 pass 5 named and this round claims closed

**Status: REPRODUCED** (`carries_visible_content` + `is_plain_path_component`
extracted verbatim from `src/text.rs:41-47` and `src/journal/mod.rs:268-294`
into a scratch binary).

**File:** `src/journal/mod.rs:268-294`; claim falsified at
`21-15-PLAN.md` truth 5 and `src/journal/mod.rs:256-263`.

```
"demo"          is_plain_path_component=true   renders_as="demo"
"demo\u{200b}"  is_plain_path_component=true   renders_as="demo"
"de\u{200b}mo"  is_plain_path_component=true   renders_as="demo"
"demo\u{feff}"  is_plain_path_component=true   renders_as="demo"
"\u{200b}"      is_plain_path_component=false
"   "           is_plain_path_component=false
```

The unification closed the **wholly**-invisible case (which is what pass 5
reproduced: a run directory whose entire name was one `U+200B`). It did not
close the case pass 5 named in the same sentence — *"a successful, persisted,
terminal run record that no human can … distinguish from a sibling run called
`abc` vs `abc\u{200b}`"*. Truth 5's wording ("two visually identical aliases can
**no longer** resolve to two different envelope paths") is therefore an
overclaim, and it is the wording a sixth reviewer will read as closed.

The blast radius is the same seven call sites CR-02 listed: `--run-id`
(`driver/mod.rs:1079`, `journal/mod.rs:329`, `journal/writer.rs:491`), registry
aliases (`main.rs:263`, `envelope/mod.rs:204`, `envelope/cred.rs:758`) and the
model-supplied phase token (`goal.rs:667`).

**Fix.** One added clause in the shared module, consumed by
`is_plain_path_component` and by CR-01's registration check, so there is still
one spelling:

```rust
// src/text.rs
/// Whether `value` contains a character that renders as nothing while changing
/// the value's identity. A NAME made of visible characters plus an invisible
/// one is a second name for the first — two run directories, two envelope
/// roots, one thing on screen.
pub fn carries_invisible_formatting(value: &str) -> bool {
    value.chars().any(|c| {
        matches!(c, '\u{200b}'..='\u{200f}' | '\u{2060}'..='\u{2064}' | '\u{feff}')
    })
}

// src/journal/mod.rs, after the is_control() refusal
if crate::text::carries_invisible_formatting(value) { return false; }
```

Pin `("demo", "demo\u{200b}")` as a look-alike pair in the journal's
both-directions test, and re-run the acceptance list (`"20"`, `"2.1"`,
`"2026-08-19T12-00-00Z-aaaa"`, `"demo"`, `"99"`) — none contains a format
character, so nothing legitimate regresses.

### WR-06: `list` shows a project that `drive` says is not registered — a round-attributable inconsistency from D-15-4

**Status: REPRODUCED** (same run as CR-01).

**Files:** `src/driver/mod.rs:426-432` (the alias refusal, hoisted above the
lookup by D-15-4), `src/registry.rs:14-20`, `src/main.rs:64-81`.

```
$ ... list
<U+200B>                    .../projA          2026-08-22...
$ ... drive "$(printf '<U+200B>')" --command '/gsd:progress' --dry-run
Error: no project is registered under the alias `<U+200B>`
```

The project **is** registered — `config.json` holds the key — but D-15-4 moved
the supplied-but-blank refusal above the registry lookup and reuses
`OptInError::UnknownAlias`, so the message states a falsehood about the
registry's contents. Before this round the alias reached the lookup, matched,
and produced the truthful opt-in refusal. The flagged DRIVE-01 assumption in
21-15's edge-probe audit ("moving supplied-but-blank refusals ahead of the
registry lookup … does not change any behavior a consumer depends on") is
narrowly true — nothing is created either way — but the *message* regressed, and
a refusal that misdescribes durable state is the class this phase is about.

**Fix.** Closing CR-01 removes the ability to create such an entry going
forward. For entries already in a user's config, give the boundary its own
refusal rather than borrowing `UnknownAlias`:

```rust
let alias = payload::NonBlank::new(&alias)
    .ok_or(DriveError::AliasNotVisible { alias })?;
// Display: "the alias {alias:?} carries no visible character, so it names no
// project a human can read back — `list` may show it; it cannot be driven."
```

---

## Info

### IN-01: the padded-form matrix hand-exempts `--alias` and `--run-id` on a rationale that does not apply to the function under test

**File:** `src/driver/mod.rs:2560-2580`, specifically `:2563-2566`.

`one_visible_character_is_accepted_in_every_argv_position` narrows two of seven
columns to `&["x"]`, on the reasoning that "both are composed into path
components downstream, where `is_plain_path_component` answers a stricter
structural question that is not this test's subject." But the function under
test is `DriveArgs::from_argv`, which never calls `is_plain_path_component` for
either field — `NonBlank::new` is the only judge (`mod.rs:426`, `:437`). All
four padded payloads (`"x"`, `" x "`, `"\u{200b}x"`, `"x\u{feff}"`) would be
accepted. The exemption costs two columns of coverage for a reason that does not
bear on the assertion, which is structurally the cycle-3 defect ("its matrix
hand-exempted a column on a comment true for 1 of 4 payloads"). Delete the
branch and sweep all seven columns with all four payloads.

### IN-02: guard eight's limits block is numbered 1, 6, 2, 3, 4, 5

**File:** `tests/spawn_seam_guard.rs:2380-2423`. The new limit 6 (`Self::`
needles may match zero sites) was inserted after limit 1's continuation rather
than after limit 5. Cosmetic, but this block is the artifact prohibition 1 asks
a reader to audit.

### IN-03: several assertion messages lost their `\` line continuations and now embed 8–10 space runs in operator-facing text

**Files:** `tests/driver_goal_seam.rs:1514`, `:1521`, `:1530`, `:1536`, `:1549`;
`tests/spawn_seam_guard.rs:2132`, `:2136`. Example: `"the fixture must PASS the
first layer, or this test is re-testing the          predicate instead of the
bound it exists for"`. These are the messages a future reviewer reads when the
guard fires.

### IN-04: `main.rs` calls `load_config` before `DriveArgs::from_argv`

**File:** `src/main.rs:107` vs `:133`. Adjudicating 21-15-SUMMARY residual 5:
**both claims check out.** The ordering is pre-existing (`load_config` has
always preceded `drive`, which needed the config), and it is out of scope. The
consequence is message ordering only — an unparseable `--config` is refused
before a blank payload is — and neither path creates anything. Not a finding;
recorded because the residual asked.

### IN-05: deviation 5 (prose reworded so three literal greps read the specified numbers) lost no assertion — but the criteria, not the response, were the defect

I checked each rewritten paragraph against the property it is supposed to state.
`spawn_seam_guard.rs:2043-2053` still names both remaining silent gaps, still
records that gap 2 was live at column zero with 51 instances, and still points
at the control that bounds it — only the literal sentence
`"production style puts items at column zero"` is paraphrased. Same for
`:1993-1997` and `:2159-2171`. **No assertion was lost.** The finding here is
against the *plan*: an acceptance criterion of the form
`grep -c "<a sentence of prose>" == 0` makes source text the artifact under
test, which invites exactly the "edit the prose to satisfy the grep" move the
executor was forced into and correctly disclosed. Future plans should assert
behaviour (a control that plants the defect) and not prose counts.

---

## Adjudications the executors requested

| # | Question | Adjudication |
|---|---|---|
| G.1 | Is prohibition 2's check narrower than its words, and does it matter? | **Yes and yes.** See WR-04 — three different hand-copied subsets in `tests/`, two of them in pins this round retargeted, and the guard's own message overclaims. Take the one-word fix (`pub mod test_support`). |
| G.2 | Is hoisting ambiguity above the per-position checks preferable? | **No — keep the current order.** "Refused with the flag you mistyped named" is the better message, and the safety property (a blank must not DEMOTE an ambiguous invocation into a legal one) is held by construction: the boundary refuses each position before ambiguity is reachable at all. `a_blank_payload_beside_a_visible_one_is_still_refused_at_the_boundary` pins all three pairings across all six shapes. But see WR-06 — the same hoist did regress one message. |
| G.3 | Is `92ad11f`'s envelope write harmless from `015aea5` on? | **Confirmed at HEAD**, and independently: `~/.local/share/gsd-meta-manager/envelope/demo/` mtime is unchanged (2026-08-21) across a full `--test-threads=2` suite run today. **But it is not asserted** — see WR-03. |
| G.4 | Is `main.rs`'s `load_config`-before-`from_argv` pre-existing and out of scope? | **Both claims verified.** IN-04. |
| G.5 | The matrix row table stays hand-maintained; guard nine is textual, so an aliased field is silent under-detection bounded by its own asserted test. | **The row-table residual is correctly disclosed** at `positions()`'s doc and in truth 6 — accept it. **The guard-nine bound is weaker than stated**: limit 1's `type`-alias bound is asserted, but limit 2's bound (the `>=10`/`>=6` floor) provably does not bind for a single field, and three *unnamed* silent shapes exist besides. WR-02. |
| H.1 | Tracer fixture uses `from_registry` not `for_testing_bypassing_opt_in`. | Correct call; the plan's instruction would have broken the `ESCAPE_HATCH` assertion. No assertion lost. |
| H.2 | `positions()` needed type aliases. | Correct; `clippy --all-targets` is back to the 4 pre-existing lints, verified. No assertion lost. |
| H.3 | Fixtures use `Some(nonblank(x))` with `.expect` rather than the plan's bare `Option`. | **Strictly better than the plan** and the deviation's reasoning is right — the plan's form would silently convert "a blank is refused" into "nothing was supplied". |
| H.4 | `driver_goal_seam`'s blank-goal rows retargeted to the boundary. | Sound retarget; `goal_args` now goes *through* `from_argv`, which is the right direction. The retarget's own payload list is one of WR-04's three hand copies. |
| H.5 | Corrective prose reworded so three literal greps read the specified numbers. | **No assertion lost** — verified paragraph by paragraph. IN-05 records the meta-finding against the criteria. |
| H.6 | The over-length fixture's third assertion strengthened. | **Correct and necessary.** The plan's char bound (`<= MAX + marker.len()` = 212) is satisfied by rendering 201 characters in full; the added `!=` raw-token and `ends_with(TRUNCATION_MARKER)` assertions are what make it detect the defect it names (`tests/driver_goal_seam.rs:1543-1567`). |
| I | Are the 4 pre-existing `=> String::new()` hits genuinely off any argv or record path? | **Yes, all four** — read individually; see the negative-evidence table. The disclosure is accurate. The `"\n  \n"` disclosure is accurate **for `src/`** and incomplete tree-wide (WR-04). |

---

## Gates measured independently at `1f07923`

| Gate | Result |
|---|---|
| `rtk proxy cargo build --all-targets` | clean |
| `rtk proxy cargo clippy --lib -- -D warnings` | clean |
| `rtk proxy cargo test --workspace --no-fail-fast -- --test-threads=2` | **exit 0**, 0 failures; neither documented flake fired |
| `sed '/pub struct DriveArgs {/,/^}/p' \| grep -c ': String\|Option<String>'` | 0 (control: 9 `payload::NonBlank`) |
| `grep -rn "=> String::new()" src/` | 4, all pre-existing, all verified off argv/record paths |
| `grep -c "post_marker_offenders(" tests/spawn_seam_guard.rs` | 3 |
| `grep -c '"pub("' / "production style…" / "scanned_files >= 10"` | 1 / 0 / 1 — as specified |

---

_Reviewed: 2026-08-22T18:20:00Z_
_Reviewer: Claude (gsd-code-reviewer), adversarial stance_
_Depth: deep · Round 5 · diff_base 199d334 → HEAD 1f07923_
_Round 4's review preserved at `c7bcafb`_
