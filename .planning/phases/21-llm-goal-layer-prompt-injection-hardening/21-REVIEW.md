---
phase: 21-llm-goal-layer-prompt-injection-hardening
reviewed: 2026-08-21T19:05:00Z
round: 3
depth: standard
diff_base: fad82eca2952a6673f277d2d5700193200bc05bd
head: 36269d14a0a9f279ce0c6a9393ad33b7cf6ac3ef
supersedes: "round 2 (this file's previous contents) is preserved in git at commit 92a4b4f"
files_reviewed: 7
files_reviewed_list:
  - src/driver/mod.rs
  - src/driver/goal.rs
  - src/error.rs
  - src/journal/mod.rs
  - tests/spawn_seam_guard.rs
  - tests/driver_goal_seam.rs
  - tests/driver_dry_run.rs
findings:
  critical: 1
  warning: 3
  info: 4
  total: 8
status: issues_found
---

# Phase 21 (gap closure, round 3): Code Review Report

**Reviewed:** 2026-08-21T19:05:00Z
**Depth:** standard
**Diff base:** `fad82ec..36269d1` (`src/`, `tests/` only)
**Files Reviewed:** 7
**Status:** issues_found

> **Note on this file:** round 2's review is preserved in git at commit `92a4b4f`
> (`docs(21): add code review report`). This file replaces it in the working tree.

## Summary

The four commissioned findings are genuinely closed, and I checked each against
the code rather than against the SUMMARYs:

* **review-CR-02 closed for `--command`.** `command_source` (`src/driver/mod.rs:392`)
  now carries the `if !command.trim().is_empty()` guard, the ambiguity arm still
  matches first (`:382`), and a blank command does not fall through to `--goal`
  (`:399`). Reproduced against the built binary at HEAD: `--command '   ' --dry-run`
  exits 1 with `NoCommandSource` and creates nothing.
* **review-CR-01 closed.** `journal::parse_approval_token` appears on exactly one
  executable line in production (`src/driver/mod.rs:644`), inside `drive`, above
  `if args.dry_run` at `:649` and 180 lines above `decompose` at `:828`.
  `approve_plan`'s signature no longer carries `args` (`:921-925`). The parse is
  genuinely pure and genuinely total — `parse_approval_token`
  (`src/journal/mod.rs:720-737`) opens nothing, spawns nothing, cannot panic on
  third-party argv, and never echoes the offending value.
* **The other three invocation-shape refusals are also pure**, so WR-09's
  "answered identically whether or not the run is real" holds for all four:
  `bounds::resolve` (`src/driver/bounds.rs:225-247`) and `escalate::resolve`
  (`src/driver/escalate.rs:371-389`) are functions of their arguments;
  `is_plain_path_component` touches no disk. The deliberate carve-out is intact:
  an **absent** `--approved-plan` stays `None` (`src/driver/mod.rs:646`) and a
  goal-only preview still renders.
* **review-WR-03 closed and accurate.** I checked each greppable claim in the
  rewritten `plan_digest` paragraph: `recheck_approval` has exactly two production
  call sites (`src/driver/mod.rs:965`, `src/driver/run.rs:2322`); every read of
  `RunRecord::approved_plan` outside the write is in a test module; and the test
  the doc now names really does pass a separator-less single half and really does
  get `SeparatorAbsent` (`tests/driver_goal_seam.rs:740-768`).
* **The new integration assertions are not vacuous.** Each names a mutation that
  turns it red: moving `recorded_approval` below `if args.dry_run` fails
  `tests/driver_dry_run.rs:482-495` and both goal-seam arms; deleting the
  `command_source` match guard fails `tests/driver_dry_run.rs:508-546`. The
  seam-spawn control arm (`tests/driver_goal_seam.rs`, arm three, asserting `== 1`)
  is what makes the two zeroes load-bearing, and it is present.
* Guard six's widening is real: the scan iterates `source_files()` tree-wide, it
  matches the UFCS spelling, and it gained a distinct-contributing-files assertion
  and a stale-allowlist assertion. `cargo test --test spawn_seam_guard` — 25 passed.

**But the recurrence happened again, in the same shape, in the one column the new
matrix guard exempts by hand.** The centrepiece of this round is
`every_command_source_refuses_or_previews_cleanly_for_every_degenerate_payload`
(`src/driver/mod.rs:1875`), a degenerate-payload × argv-position matrix that
claims to make a fourth recurrence a test-time certainty. Its `--target-phase`
column is excluded from the by-name refusal sweep by a comment (`:1976-1983`)
whose stated reason — *"`journal::is_plain_path_component` returns false for the
empty string"* — is true for exactly **one** of the four payloads in `DEGENERATE`.
`--target-phase '   '` is accepted end to end, creates a lock file, a run
directory, a `journal.jsonl` and a `run.json` recording `target_phase: "   "`, and
exits 0 — while the byte-identical `--command '   '` is refused for free. I
reproduced all of it against the built binary at HEAD.

Two further guard-accuracy defects: guard six's rewritten header still omits a
*live* silent under-detection that clippy's own `items_after_test_module` lint
already flags in this tree, and the new guard eight — the sibling added in the
same round that was commissioned to stop guards over-claiming — names no
limitation at all while claiming to have turned a grep result into an enforced
property.

**Build state.** `cargo build` clean. `cargo test --test spawn_seam_guard` 25
passed. `cargo clippy --all-targets` reports exactly 5 warnings, all pre-existing
in `src/browser.rs:131-133`, `src/project_creator.rs:146` and
`src/state_reader/mod.rs:311` — I confirmed none of those files is in this
round's diff, so they are **not** reported as findings (one of them is, however,
cited as corroboration in WR-01). All count-bearing and presence-bearing checks
in this review were run under `rtk proxy` or plain `rustc`/`python3`.

## Critical Issues

### CR-01: The degenerate-payload matrix exempts its `--target-phase` column with a reason that is false for 3 of the 4 payloads it enumerates, and a whitespace-only `--target-phase` is accepted end to end where the identical `--command` is refused

**Status: reproduced** — preview, real run and `run.json` all captured against
`target/debug/gsd-meta-manager` built from HEAD (`36269d1`).

**File:** `src/driver/mod.rs:1976-1983` (the false carve-out),
`src/driver/mod.rs:400` (the `Routed` arm with no emptiness rule),
`src/driver/mod.rs:559-565` (the seam claimed to refuse it),
`src/driver/mod.rs:1674` (`DEGENERATE`), `src/journal/mod.rs:242-254`

**Issue.** The matrix test's closing sweep asserts by name that every payload in
`DEGENERATE` is refused for `--command` and for `--goal`, and excludes
`--target-phase` with this comment:

```rust
// The two positions whose degenerate payloads are refusals rather than
// clean previews, asserted by name rather than left to the disjunction
// above. The `--target-phase` column is deliberately absent: a blank
// phase is NOT a second emptiness bug here, because `drive` refuses a
// non-plain path component at its own seam
// (`journal::is_plain_path_component` returns false for the empty
// string), pinned by
// `a_target_phase_that_is_not_a_plain_path_component_is_refused_without_touching_disk`.
```

Both halves of that justification are wrong:

1. **`is_plain_path_component` returns `true` for three of the four payloads.**
   `DEGENERATE` is `["", "   ", "\t", "\n  \n"]` (`:1674`). The predicate
   (`src/journal/mod.rs:242-254`) short-circuits on `is_empty()` and otherwise
   asks whether the value is a single `Component::Normal` equal to itself — which
   every whitespace string is. Measured with the function's exact body compiled
   standalone under `rustc`:

   ```
   ""      -> false
   "   "   -> true
   "\t"    -> true
   "\n  \n"-> true
   " "     -> true
   ```

2. **The test it cites pins only traversal.**
   `a_target_phase_that_is_not_a_plain_path_component_is_refused_without_touching_disk`
   (`src/driver/mod.rs:1392-1416`) sets exactly one value,
   `"../../../../escaped"`. It says nothing about blankness.

The matrix's `--target-phase` cells therefore land in the `Ok` branch, where the
only assertions are "the preview carries `SECTION_COMMANDS`" and "renders no
empty numbered entry". A routed preview of a nonexistent phase satisfies both —
`build_routed_report` gets `Decision::Park`/`NoRule`, pushes no commands, and the
`empty_numbered_entry` detector has nothing to find. **The guard written to make a
fourth recurrence impossible passes for the wrong reason on one of its three
columns**, which is the identical failure the round-2 `--command` enumeration had.

**Reproduction (built binary at HEAD).** Preview — refused on one flag, rendered
on its sibling:

```
$ gsd-meta-manager drive demo --config cfg.json --command '   ' --dry-run
Error: a run needs something to do: pass `--command <c>` ...              (exit 1)

$ gsd-meta-manager drive demo --config cfg.json --target-phase '   ' --dry-run
DRY RUN — nothing below was executed. ...
== GSD commands this run would issue ==
  Routed run: no command would be issued. From the state on disk now,
  the router would park (router_state_unverified:    ) and the run would stop for
  a human rather than choose.                                             (exit 0)
```

Real run — the artefacts the `--command` refusal exists to prevent, all created:

```
$ gsd-meta-manager drive demo --config cfg.json \
    --target-phase '   ' --run-id 2026-08-21T00-00-00Z-blank --max-steps 2
$ find proj/.planning -type f
  proj/.planning/meta-manager/runs/run.lock
  proj/.planning/meta-manager/runs/.gitignore
  proj/.planning/meta-manager/runs/2026-08-21T00-00-00Z-blank/run.json
  proj/.planning/meta-manager/runs/2026-08-21T00-00-00Z-blank/journal.jsonl
$ jq '{target_phase, gsd_command, outcome}' .../run.json
  { "target_phase": "   ",
    "gsd_command": "(routed: see the decided journal records)",
    "outcome": "parked:router_state_unverified" }
```

**Why this is Critical and not a Warning.** The standard this round set for
`--command ''` was preview honesty plus record integrity. Both are violated
identically here: the preview prints a park reason built from a value that is
nothing, and `run.json` records `target_phase: "   "` as evidence of what the run
drove toward. And the consequence is strictly *worse* than the defect that was
called Critical — the blank `--command` was refused before anything was created,
whereas the blank `--target-phase` costs a lock file, a run directory, a journal
and a committed run record for an invocation with nothing to do.

Worth noting alongside it: the same seam also admits an embedded newline, which
reaches the CLI preview unsanitised —

```
$ gsd-meta-manager drive demo --config cfg.json --target-phase "$(printf 'a\nEVIL')" --dry-run
  the router would park (router_state_unverified: a
EVIL) and the run would stop for
```

`src/driver/dry_run.rs` uses no `sanitize_render_line` (only the TUI does), and
`RouterAction::command_for`'s doc (`src/driver/router.rs:362-366`) asserts that
`phase` "was validated at the seam as a single plain path component ... so this
interpolation cannot smuggle a flag, a second command or a path." That is
**pre-existing**, predates this round's diff, and is reported here only as
context for how much the carve-out comment is leaning on `is_plain_path_component`.

**Fix.** Three parts, all small, and the first two must ship together:

```rust
// src/driver/mod.rs, command_source — the same rule the two siblings carry
(None, Some(target_phase)) if !target_phase.trim().is_empty() => {
    Ok(CommandSource::Routed(target_phase.to_string()))
}
(None, Some(_)) => Err(DriveError::NoCommandSource),
```

(Alternatively tighten `journal::is_plain_path_component` to reject a
whitespace-only value — that also closes the `--run-id` sibling, which accepts
`'   '` for the same reason — but then the matrix's `Err(other) => panic!` arm
must be widened, because the refusal would be `TargetPhaseInvalid` rather than
`NoCommandSource`.)

Then delete the carve-out comment and put `--target-phase` into the by-name sweep
alongside its two siblings:

```rust
for payload in DEGENERATE {
    for (position, build) in [("--command", ...), ("--target-phase", ...), ("--goal", ...)] {
        assert!(matches!(command_source(...), Err(DriveError::NoCommandSource)), ...);
    }
}
```

Third: add an arm to
`a_target_phase_that_is_not_a_plain_path_component_is_refused_without_touching_disk`
for `"   "`, so the test the comment cites actually pins what the comment claims
it pins. Verify the fix by re-running the reproduction above — the real run must
leave `.planning/meta-manager` absent.

## Warnings

### WR-01: Guard six's rewritten header still omits a live silent under-detection — production code after a file's `mod tests {` marker is skipped to EOF, and this tree has such code today

**Status: precondition reproduced** (marker and trailing production `fn` located
and corroborated by clippy); the skip itself is a three-line read of the scanner.

**File:** `tests/spawn_seam_guard.rs:1703-1723` (the "what is still approximate"
block), `tests/spawn_seam_guard.rs:1776-1781` (`test_region_start`),
`tests/spawn_seam_guard.rs:1796-1799` (the skip), `src/state_reader/mod.rs:311`
and `:530`

**Issue.** The header now lists three limits and, correctly, refuses to make a
blanket claim about the guard's failure direction. Limit 1 reads:

> The production/test boundary is found by a LINE MARKER (`mod tests {` at column
> zero), not by parsing. A file that spelled its test module differently would be
> scanned in full, and its tests' own journal closes would be reported as
> production offenders. **Over-detection — loud.**

That names only one direction of the marker approximation. The other direction is
the one that is live in this tree: `terminal_write_hits` computes
`let boundary = test_region_start(file).unwrap_or(usize::MAX);` and then
`if *number >= boundary { continue; }` — so **everything after the first
column-zero `mod tests {` is skipped to end of file**, whether it is a test or
not. `src/state_reader/mod.rs` puts `mod tests {` at line 311 of 547 and then
declares a genuine production function afterwards:

```
src/state_reader/mod.rs:311:  mod tests {
src/state_reader/mod.rs:530:  pub fn count_backlog_items(planning_dir: &Path) -> u32 {
```

`cargo clippy --all-targets` independently flags it: `warning: items after a test
module --> src/state_reader/mod.rs:311:1`. Lines 311–547 of that file are
invisible to guard six. Tree-wide the boundary excludes **27,280 of 72,706 lines
under `src/` (37.5%)** across 58 of 74 files — correct for the test regions, but
the header never says the exclusion runs to EOF or that a production item landing
past a marker is silently exempt.

Name the mutation the guard would not catch: add `journal.finish("aborted")` to a
new production helper placed after line 311 of `src/state_reader/mod.rs`. Guard
six stays green, and the run's `escalations_used` goes unstamped — which is
exactly the property the guard exists to hold.

Two lesser approximations are also unnamed, both silent:
`enclosing_fn` (`tests/spawn_seam_guard.rs:1130-1136`) is a nearest-preceding-`fn`
backward scan with **no brace tracking**, so any terminal write that lands between
`fn finish_run(` and the next `fn` declaration in `src/driver/run.rs` is
attributed to `finish_run` and allowlisted; and `collect`
(`tests/spawn_seam_guard.rs:245-247`) drops an unreadable file with `continue`,
so a file the scan cannot read is a silently empty contribution.

**Fix.** Add limit 4 to the header, in the register the other three now use:

```
// 4. The boundary excludes everything from the marker to END OF FILE, not just
//    the test module. A production item declared AFTER a file's `mod tests {`
//    is invisible — `src/state_reader/mod.rs:530` is such an item today, and
//    clippy's `items_after_test_module` already reports it. **Under-detection
//    — silent.** What bounds it: nothing in this guard. Fixing the clippy lint
//    in that file removes the only live instance.
// 5. `enclosing_fn` finds the nearest PRECEDING `fn` and does not track braces,
//    so a write between `fn finish_run(` and the next declaration is
//    attributed to the helper and allowlisted. **Under-detection — silent.**
```

Cheaper still, and it closes limit 4 outright: assert in the guard that no file
under `src/` has a top-level item after its marker (one `regex`-free line-scan),
or simply move `count_backlog_items` above the test module and keep the
invariant true by construction.

### WR-02: Guard eight names no limitation at all while claiming to have turned a grep result into an enforced property — the same over-claiming shape as review-WR-01, in the guard added to close it

**Status: inferred** (by reading the needles, the allowlist and `enclosing_fn`).
No violating call site exists in the tree today — I checked: the only
`CommandSource::` occurrences outside `src/driver/mod.rs` are `run.rs`'s unrelated
enum.

**File:** `tests/spawn_seam_guard.rs:2136-2165` (header),
`tests/spawn_seam_guard.rs:2168-2172` (`COMMAND_SOURCE_VARIANTS`),
`tests/spawn_seam_guard.rs:2208-2226` (the scan)

**Issue.** The header states the property flatly — *"Until now that was a grep
result somebody ran once; here it is a property a test enforces"* — and then
documents only the name-collision reasoning. It never says what the scan cannot
see. Guard six, twenty lines up the same file and rewritten in the same round,
now names each of its limits with a direction, precisely because a generous
self-description was review-WR-01. Guard eight was written to a lower standard in
the same commit.

What it actually enforces is *"no production line under `src/`, before that file's
`mod tests {` marker, contains the literal `CommandSource::Command(`,
`CommandSource::Routed(` or `CommandSource::Goal(` outside two named functions"*
— which is narrower than "`command_source` is the single production constructor"
in at least four ways, all silent:

1. **`Self::Command(`.** An `impl CommandSource { fn from_argv(..) -> Self { Self::Command(..) } }`
   is a second production constructor and matches no needle.
2. **An imported variant.** `use crate::driver::CommandSource::Command;` then
   `Command(x)` — same.
3. **`enclosing_fn` has no brace tracking** (`:1130-1136`). Any variant-naming
   line placed between `fn command_source` (`src/driver/mod.rs:372`) and
   `fn preview_text` (`:446`) — a `const`, a free-standing block, a nested item —
   is attributed to `command_source` and allowlisted.
4. **It inherits WR-01's boundary blind spot**, since it copies the same
   `test_region_start(file).unwrap_or(usize::MAX)` skip.

The `contributed >= COMMAND_SOURCE_VARIANT_COUNT` check is a good non-vacuity
control for a needle that stopped matching *inside the allowlisted functions*, but
it says nothing about a new spelling appearing elsewhere.

**Fix.** Add a limits block to guard eight's header modelled on guard six's, naming
items 1–4 with the direction each fails in. Then close the cheapest one for real
by adding two needles beside the three:

```rust
const COMMAND_SOURCE_VARIANTS: &[&str] = &[
    "CommandSource::Command(", "CommandSource::Routed(", "CommandSource::Goal(",
    // Self-qualified construction from inside an `impl CommandSource`.
    "Self::Command(", "Self::Goal(",
];
```

(`Self::Routed(` would collide with `run.rs`'s enum, which is itself worth
recording as the reason the rename in `21-11-SUMMARY.md`'s "carried forward" note
should be done.) Add a one-line assertion that no `use ...CommandSource::` import
exists under `src/`, which makes item 2 loud instead of silent.

### WR-03: The matrix's payload axis is bounded by the same `str::trim` predicate as the production guard, so it can only confirm the guard and never falsify it — `--command $'\u200b'` renders a visually empty numbered entry today

**Status: reproduced** against the built binary at HEAD.

**File:** `src/driver/mod.rs:1663-1674` (`DEGENERATE` and its doc),
`src/driver/mod.rs:1706-1715` (`empty_numbered_entry`),
`src/driver/mod.rs:392` (the production guard)

**Issue.** `DEGENERATE`'s doc states the coupling as a virtue:

> Whitespace-only is the predicate `str::trim` already answers, and the production
> guard is written against `trim` for exactly that reason: the test and the code
> must agree about what "blank" means or the enumeration is checking a different
> property from the one the seam enforces.

The consequence is the opposite of what is claimed: because the enumeration's
notion of "blank" is *defined* by the same predicate the guard uses, the
enumeration cannot contain a payload the guard mishandles. It is a tautology, not
a check. `empty_numbered_entry` closes the loop — it decides "empty" with
`tail.trim().is_empty()`, the same predicate again.

A payload outside `char::is_whitespace` demonstrates it. U+200B ZERO WIDTH SPACE
survives `trim`, so `command_source` resolves it, and the preview prints
CR-02's exact shape under the exact header the whole cycle exists to make honest
(shown through `cat -A`, `M-bM-^@M-^K` is the UTF-8 of U+200B):

```
$ gsd-meta-manager drive demo --config cfg.json --command $'\u200b' --dry-run
...For a supplied command, the line below is the complete and honest sequence....
  1 command in the sequence:$
    1. M-bM-^@M-^K$
```

`empty_numbered_entry` cannot flag it, and on a real run the value is what gets
spawned and what lands in `run.json`'s `gsd_command`. The `DEGENERATE` doc
anticipates "a fifth blank shape (a vertical tab, a non-breaking space)" — both of
those *are* `char::is_whitespace` and are already covered; the shapes that are
not covered are exactly the ones the doc does not think of, because it is
thinking in `trim`'s vocabulary.

Exploitability is low: the operator supplies `--command` themselves, and the TUI's
own path is guarded separately by `committed.starts_with('/')`
(`src/ui/screens/driver_start.rs:223`). This is a Warning for the *guard-design*
defect it demonstrates, not for the payload.

**Fix.** Break the tautology by making the two predicates different, so the test
can falsify the guard rather than restate it. Concretely, judge the *rendered*
entry on visible width rather than on `trim`:

```rust
fn empty_numbered_entry(rendered: &str) -> Option<&str> {
    rendered.lines().find(|line| {
        let trimmed = line.trim();
        trimmed.split_once('.').is_some_and(|(head, tail)| {
            !head.is_empty()
                && head.chars().all(|c| c.is_ascii_digit())
                // Not `tail.trim()`: a payload the seam's `trim` does not strip
                // still renders as nothing, and a detector that reused the
                // seam's predicate could only ever agree with it.
                && tail.chars().all(|c| c.is_whitespace() || c.is_control()
                    || matches!(c, '\u{200b}'..='\u{200f}' | '\u{2060}' | '\u{feff}'))
        })
    })
}
```

and add at least one non-`is_whitespace` blank to `DEGENERATE`
(`"\u{200b}"`, `"\u{feff}"`). Whether the *production* seam should also reject
them is a separate call — but the guard must be able to ask the question.

## Info

### IN-01: `DriveError::NoCommandSource`'s message does not describe the case it is now most often raised for, and never mentions `--goal`

**File:** `src/error.rs:718-723`, `src/driver/mod.rs:399`

**Issue.** Reusing `NoCommandSource` for a supplied-but-blank command was a
deliberate key decision, but its rendered text is:

> a run needs something to do: pass `--command <c>` to run one GSD command, or
> `--target-phase <N>` to let the decision router choose each command from
> observed project state

A user who typed `--command '   '` is told to pass `--command`. A user who typed
`--goal '   '` — the pre-existing case — is told about two flags, neither of which
is the one they used, and `--goal` is not named at all even though it has been a
first-class command source since 21-07. Reproduced: that is the verbatim stderr
from `--command '   '` against HEAD.

**Fix.** Extend the message to name all three sources and to say that a value made
only of whitespace counts as absent — the refusal is otherwise a bug report rather
than an error message, which is the standard the rest of this file holds.

### IN-02: `variant_name`'s arms are duplicated as two hand-written string arrays, and one of them does not fail when a fourth variant appears

**File:** `src/driver/mod.rs:1691-1697`, `:1810`, `:1945-1946`

**Issue.** The compile-forcing mechanism is real — I accept 21-11's E0004 evidence,
and `variant_name` is a genuine wildcard-free `match`. But the two consumers each
re-spell its arms as a literal: `for expected in ["Command", "Routed", "Goal"]`
(`:1810`) and `assert_eq!(resolved, vec!["Command", "Goal", "Routed"])` (`:1946`).
The second fails loudly if a fourth variant is reachable from an existing argv
position (a new name appears in `resolved`). The first does not: after a fourth
variant is added and `variant_name` extended, the `sources` array can stay at
three entries and the loop over three names still passes — a per-variant check
that has quietly stopped covering a variant, which is the exact criticism levelled
at its index-keyed predecessor.

**Fix.** Derive the expected set from one place, e.g. a
`const ALL_VARIANT_NAMES: [&str; 3]` that `variant_name`'s own module doc points at
and that a `debug_assert` ties to the match's arm count, or build the expectation
from `sources.iter().map(variant_name)` and assert the *set* rather than iterating
a literal.

### IN-03: The approval-token parse now fires on invocations where an approval is meaningless, and the asymmetry with a well-formed-but-irrelevant token is undocumented

**File:** `src/driver/mod.rs:641-647`

**Issue.** `recorded_approval` is computed unconditionally, so
`drive --command /gsd:progress --approved-plan garbage` is now refused with
`PlanApprovalMalformed`, where before the move it was ignored (the value was only
read inside `approve_plan`, which a command run never reaches). Meanwhile
`--command /gsd:progress --approved-plan 'sha256:aa+sha256:bb'` is still accepted
and silently discarded. Refusing malformation everywhere is defensible and is what
WR-09 asks for, but the pairing — malformed is fatal, well-formed is ignored — is
not stated anywhere, and `--approved-plan`'s clap help (`src/cli.rs`) still reads
as though the flag only pertains to a goal run.

**Fix.** One sentence in the `recorded_approval` comment block naming the
widening, and a clause in the clap help saying the flag is validated on every
invocation but only consulted for a `--goal` run.

### IN-04: The new `driver_dry_run` test inherits the silent `repo() -> None` skip

**File:** `tests/driver_dry_run.rs:508-511`

**Issue.** `a_blank_command_is_refused_in_preview_and_in_a_real_run` opens with
`let Some(repo_dir) = repo() else { return; };`. In a sandbox where `git init` or
`git commit` is unavailable, the test passes without executing a single assertion.
This is the file's established pattern rather than something the round invented,
but a review-CR-02 regression test that can pass by doing nothing is worth naming
in a phase whose whole subject is checks that pass for the wrong reason.

**Fix.** Have `repo()` record its unavailability once and have a single
`#[test] fn the_git_fixture_is_available()` assert it — so an environment that
silently skips half the suite fails one named test instead of reporting green.

---

## Carried forward from round 2 — still open, out of this round's commissioned scope

Not re-litigated here, and not counted in this report's totals. Recorded so the
next verification does not read this file as "everything from round 2 is closed."
Each re-checked against HEAD:

* **round-2 WR-02** — `registry::current_prompt_inputs(` is still absent from
  `BLOCKING_HELPERS` (`tests/async_blocking_guard.rs:124-144`) and is still called
  synchronously from `approve_plan` (`src/driver/mod.rs:927`) and from
  `async fn execute_run` (`src/driver/run.rs:2325`).
* **round-2 WR-04** — the spawn gate still passes `&approved.plan_digest` on both
  sides of `recheck_approval` (`src/driver/run.rs:2321-2328`); the argument that
  this is sound still lives only in a comment.
* **round-2 WR-05** — `PlanStep::rationale` still has no production reader.
* **round-2 IN-01…IN-04** — the "digest" wording in the goal preview, the
  representable `GoalNotDecomposed` + non-empty command list, the
  `plan_target_phase(plan).unwrap_or_default()` empty target phase
  (`src/driver/mod.rs:977` — note this is a *second* route to a blank
  `target_phase`, and unlike CR-01's it bypasses the argv seam entirely), and the
  zero-budget "was already spent" wording.

---

_Reviewed: 2026-08-21T19:05:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
_Round 2 preserved at git commit 92a4b4f_
