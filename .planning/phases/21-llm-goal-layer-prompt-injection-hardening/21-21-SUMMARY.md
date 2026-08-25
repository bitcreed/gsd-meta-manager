---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 21
subsystem: ui
tags: [unicode, trojan-source, bidi, ratatui, render-surface, derivation, deny-by-default]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening (round 7, plan 21-19)
    provides: "text::display_identity, text::is_invisible_formatting_char (the derived class), text::is_identity_char (the finite alphabet), test_support::LOOK_ALIKE_PAIRS / DEGENERATE"
provides:
  - "A source-derived, deny-by-default census over the `Screen` trait: a recursive `read_dir` walk of `src/` enumerates every implementor and is asserted equal, in BOTH directions, to a disposition table that adjudicates but never enumerates."
  - "A behavioural render probe that renders every implementor in every state its row names, through the real `Screen::render`, into a ratatui `Buffer` — blind to no sink spelling."
  - "The whole TUI render surface escaped: registry keys plus every `.planning/`-read status, milestone, phase number/name, HANDOFF context, queued command, workstream name, project path and filter text."
  - "`impl Display for Alias` withdrawn, so every future interpolation of a judged identity is a compile error that must be resolved deliberately."
  - "`Display for AliasRefusal` escapes at the ONE producer, closing every present and future refusal echo without naming any of them."
  - "`registry::LegacyRegistryKey` — no `Display`, no coercion, two accessors named for the questions they answer — so `remove` accepting raw and echoing escaped is enforced by the compiler rather than by memory."
  - "`src/ui/project_list.rs` deleted: 327 lines the build never compiled, whose only surviving effect was to make the TUI look escaped to two independent readers."
  - "Dated, append-only corrections to `21-19-PLAN.md` and `21-19-SUMMARY.md`, filed in the commit that falsified them; named-shape row 8 reopened from CLOSED."
affects: [render surface, any future Screen implementor, any future refusal echo, round 9]

actuals:
  tokens: 34623    # chars/4 over the realized diff: 138492 chars / 4
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "Enumeration by filesystem walk; adjudication by table; the two asserted equal in both directions."
    - "Behavioural render probes over a ratatui Buffer instead of syntactic scans over sink call sites."
    - "Escaping at the one producer of a type rather than at each of its consumers."
    - "A newtype with no Display and deliberately unattractive accessor names, used to force a choice the compiler can check."

key-files:
  created:
    - src/ui/screens/render_escape_guard.rs
  modified:
    - src/registry.rs
    - src/main.rs
    - src/text.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/delete_confirm.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/driver_inject.rs
    - src/ui/screens/driver_start.rs
    - src/ui/screens/enqueue.rs
    - src/ui/screens/add_project.rs
    - src/ui/screens/create_project.rs
    - src/ui/screens/queue_delete_confirm.rs
    - src/ui/roadmap_widget.rs
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-19-PLAN.md
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-19-SUMMARY.md

key-decisions:
  - "D-21-1: delete src/ui/project_list.rs rather than wire it up — it was orphaned at c297631 and superseded by screens/normal.rs; dead code a reader mistakes for shipped is itself the hazard."
  - "D-21-2: withdraw impl Display for Alias; the value is the compile error on the FIFTH site, not the four that existed."
  - "D-21-3: escape inside Display for AliasRefusal — one producer beats three consumers — and delete the duplicate outer escape, licensed by a committed idempotence pin."
  - "D-21-4: close the TUI by a source-derived Screen census plus a behavioural probe, NOT by retyping the UI layer (measured at 25 direct compile errors plus an unbounded cascade)."
  - "D-21-5 (WR-04): keep AliasRefusal::NotPlainComponent and pin it; the pin doubles as the certificate that D-19-2's alphabet clause did not subsume the structural clause."
  - "D-21-6: probe fixtures drawn BY IMPORT from test_support::LOOK_ALIKE_PAIRS, never respelled."
  - "New, forced by measurement: the disposition table's reason column states WHICH values a screen draws and WHERE THEY COME FROM — never 'escaped' or 'safe' — because the reason column is the adjudication a future reader inherits."

patterns-established:
  - "A guard's clean result must be distinguishable from a guard that stopped checking: every comparison here is extracted pure and driven by a permanent synthetic control in both directions plus the clean direction."
  - "Arrival before property: assert the benign value REACHED the buffer before asserting anything about the hostile one, so silence cannot pass."
  - "The invisible-class assertion is applied to every render state whatever its disposition — that, not the escaped-form assertion, is what forces breadth."

requirements-completed: [DRIVE-01, DRIVE-03, SAFE-08]
# Copied verbatim from this plan's `requirements` frontmatter, as the template
# requires. **This field is a declaration of the plan's SCOPE, not a completion
# verdict.** Requirement status in .planning/REQUIREMENTS.md is decided
# exclusively by a passed verification (828d7cc, 0c4f712 — twice reverted, six
# rounds held), and no commit of this plan touches that file (proved below).
# This round resolves NONE of the three flagged edge-probe assumptions for these
# IDs; they are inherited by round 9. See "Edge-probe assumptions" below.

coverage:
  - id: D1
    description: "The TUI render surface is enumerated by a recursive read_dir walk over the `Screen` trait, not by a list; an unadjudicated implementor and a stale row are each reported as their own harm."
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_census_matches_the_tree"
        status: pass
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_census_reports_an_unadjudicated_screen_and_a_stale_row"
        status: pass
    human_judgment: false
  - id: D2
    description: "A twelfth Screen implementor, added tomorrow in a file this plan never touches, goes red with nobody editing a list."
    verification:
      - kind: manual_procedural
        ref: "planted `impl` of the trait in src/driver/liveness.rs; ran the census; captured the verbatim red; removed it; `git status --porcelain` clean"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every Screen implementor's disposition is CHECKED by a real render, not claimed: escaped identity present where identity arrives, absent where the row says it draws none, and ZERO invisible-class characters in the buffer of every state of every implementor."
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped"
        status: pass
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#every_adjudicated_screen_has_a_probe_fixture"
        status: pass
    human_judgment: false
  - id: D4
    description: "`src/ui/project_list.rs` deleted with evidence, no dangling doc citation, and no test result moved."
    verification:
      - kind: manual_procedural
        ref: "rtk proxy grep -rn \"mod project_list\" src/ -> exit 1; rtk proxy ls src/ui/project_list.rs -> No such file; suite counts identical before and after"
        status: pass
    human_judgment: false
  - id: D5
    description: "Withdrawing `impl Display for Alias` makes every interpolation of a judged identity a compile error; the four sites the compiler named were each resolved deliberately."
    verification:
      - kind: manual_procedural
        ref: "cargo check --all-targets after the withdrawal; compiler's own site list quoted verbatim in this SUMMARY"
        status: pass
    human_judgment: false
  - id: D6
    description: "Every refusal echo — present and future — emits the escaped form because `Display for AliasRefusal` escapes at the producer, with the two untouched echo sites absent from this plan's diff."
    verification:
      - kind: unit
        ref: "src/registry.rs#a_refusal_never_carries_an_invisible_character_into_its_own_message"
        status: pass
      - kind: e2e
        ref: "built binary, `add` and `envelope scan` arms with a hostile alias: 0 raw U+202E, project's U+ notation present"
        status: pass
      - kind: unit
        ref: "src/registry.rs#escaping_an_already_escaped_identity_changes_nothing"
        status: pass
    human_judgment: false
  - id: D7
    description: "`remove` still accepts a raw legacy alias byte-identically (D-17-3) and no longer echoes it unescaped, enforced structurally by `LegacyRegistryKey`."
    verification:
      - kind: e2e
        ref: "built binary before/after against a hand-built legacy config.json; both runs quoted with code points in this SUMMARY"
        status: pass
      - kind: integration
        ref: "tests/registry_test.rs#a_legacy_alias_the_alphabet_refuses_is_still_removable (unmodified; git diff empty)"
        status: pass
    human_judgment: false
  - id: D8
    description: "`AliasRefusal::NotPlainComponent` is pinned for `..` and `.`, certifying that D-19-2's alphabet clause did not subsume the structural clause."
    verification:
      - kind: unit
        ref: "src/registry.rs#the_two_values_that_still_reach_not_plain_component_still_reach_it"
        status: pass
    human_judgment: false
  - id: D9
    description: "The records CR-01 falsifies are corrected in the commit that falsifies them, append-only and dated, with named-shape row 8 reopened."
    verification:
      - kind: manual_procedural
        ref: "git diff --numstat -> 56/0 and 78/0 (zero deletions); grep REOPENED -> present"
        status: pass
    human_judgment: false
  - id: D10
    description: "Whether the render surface is now genuinely closed to a human's satisfaction — in particular whether the disclosed residual (tabs whose populated cache branches no committed control exercises) is an acceptable place to stop for this round."
    verification: []
    human_judgment: true
    rationale: "This is a scope judgment, not a property. The probe proves what it renders; it cannot prove that what it does not render is unimportant. A human has to decide whether the residual named in the limits block is acceptable or whether round 9 must close it."

duration: 6h 47m
completed: 2026-08-25
status: complete
---

# Phase 21 Plan 21: Deriving the Render Surface Summary

**The TUI's render surface is now closed by a recursive `read_dir` walk over the `Screen` trait plus a behavioural probe over a rendered ratatui `Buffer` — proven by planting a real twelfth implementor in `src/driver/liveness.rs` and watching the census name it — while `impl Display for Alias` is withdrawn, `AliasRefusal` escapes at its one producer, and `LegacyRegistryKey` makes `remove`'s accept/echo split a compile error rather than a memory.**

## Performance

- **Duration:** 6h 47m (first commit `d8e0518` 2026-08-25T16:41:03-06:00 → last commit `2b5ca7a` 2026-08-25T17:24:56-06:00; wall time includes measurement work between commits)
- **Tasks:** 3 (one tracer, two execute)
- **Commits:** 5
- **Files changed:** 20 (1 created, 1 deleted, 18 modified) — 1893 insertions, 437 deletions

## Accomplishments

- **The enumeration is a filesystem walk, and the table only adjudicates.** `screen_implementors_from_source()` walks `src/` recursively with `std::fs::read_dir`, drops comment lines, assembles its needle at runtime so it cannot report itself, and asserts it found production source at all. `the_screen_census_matches_the_tree` asserts the derived set equal to `SCREEN_IDENTITY_DISPOSITIONS` in BOTH directions, reporting an unadjudicated implementor and a stale row as separate harms.
- **The acceptance bar was measured, not argued.** A real `impl` of the trait was planted in `src/driver/liveness.rs` — two directory levels down, nothing to do with the UI, untouched by this plan's fix — and the census named it. Removed; tree confirmed clean.
- **Every disposition is checked by a render.** `the_screen_renders_identity_escaped` builds each implementor in every state its row names (all eleven detail sub-views, all four driver-confirm prompts, both wizard steps), renders through the real `Screen::render`, and asserts arrival → escaped form → zero invisible-class characters.
- **The whole TUI is escaped**, scoped as the plan states it: every string this build renders that it did not itself author. Registry keys, and the status, milestone, phase numbers and names, HANDOFF pause context, queued commands, workstream names, project paths and filter text read from `.planning/`.
- **The compiler names identity interpolations now.** `impl Display for Alias` is withdrawn; the four sites it reached were each resolved deliberately with a one-line statement of which question the site asks.
- **One producer closed every refusal echo**, including two sites that do not appear in this plan's diff — the property verification pass 8 asked to be proven rather than asserted.
- **`remove` accepts raw and echoes escaped**, both halves measured against the built binary with a hand-built legacy `config.json`.
- **327 lines of dead code deleted**, with the evidence, and with the two doc comments citing line numbers inside it corrected in the same commit.
- **Three separate reds, plus two the plan did not ask for**, all recorded verbatim.

## Task Commits

1. **Task 1 (TRACER) RED arm: census + probe, observed failing** — `d8e05180ce4a79ba4fb1a68d6c7af364debe8ba3` (test)
2. **Task 1 fix: the destructive confirm escaped; the orphan deleted** — `f3a2a679cce94ab40ef366d69b570cf915900980` (fix)
3. **Task 2: all eleven implementors adjudicated; twelfth-screen red proven** — `6abd828d1cfd9c7d9d925bf59bfd813675dbe2c6` (fix)
4. **Task 3: trait impl withdrawn, producer escapes, accept/echo split, records corrected** — `b8b9d78aaf62845333f83ba24c21246510653231` (feat)
5. **Follow-up: the census's own doc no longer inflates a cross-check grep** — `2b5ca7a33928b5d2fa24db466f7553095b1c21bf` (docs)

## The reds, verbatim

### RED 1 — the census, against a table holding one of eleven (commit `d8e0518`)

```
thread 'ui::screens::render_escape_guard::tests::the_screen_census_matches_the_tree' (1346160) panicked at src/ui/screens/render_escape_guard.rs:417:9:
the derived render surface and the disposition table disagree:

UNADJUDICATED IMPLEMENTOR: `AddProjectScreen` (in src/ui/screens/add_project.rs) implements the trait but no row adjudicates whether it renders identity. A screen nobody adjudicated is a screen nobody escaped. Add a row to SCREEN_IDENTITY_DISPOSITIONS stating which values it draws and where they come from.
```

…and nine more, one per unadjudicated implementor: `CreateProjectScreen`,
`DetailScreen`, `DriverConfirmScreen`, `DriverInjectScreen`, `DriverStartScreen`,
`EnqueueScreen`, `HelpScreen`, `NormalScreen`, `QueueDeleteConfirmScreen`.

### RED 2 — the probe, against the unescaped destructive confirm at HEAD (commit `d8e0518`)

```
thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (1346161) panicked at src/ui/screens/render_escape_guard.rs:516:21:
DeleteConfirmScreen (src/ui/screens/delete_confirm.rs) did not render "demoU+E0041rU+00ADun". Route what a human READS through crate::text::display_identity; the value used for lookups, map keys, path segments, comparisons and persistence stays RAW.
```

### RED 3 — THE TWELFTH SCREEN, planted for real (commit `6abd828`)

A throwaway implementation of the trait was added to `src/driver/liveness.rs` —
a file two directory levels below `src/`, with nothing to do with the UI, and
one this plan's fix does not otherwise touch.

```
thread 'ui::screens::render_escape_guard::tests::the_screen_census_matches_the_tree' (1531855) panicked at src/ui/screens/render_escape_guard.rs:863:9:
the derived render surface and the disposition table disagree:

UNADJUDICATED IMPLEMENTOR: `TwelfthScreenNobodyAdjudicated` (in src/driver/liveness.rs) implements the trait but no row adjudicates whether it renders identity. A screen nobody adjudicated is a screen nobody escaped. Add a row to SCREEN_IDENTITY_DISPOSITIONS stating which values it draws and where they come from.
```

Nobody edited a list. The implementor was then removed and the tree confirmed
clean — `rtk proxy git status --porcelain` does not name `src/driver/liveness.rs`.

### RED 4 — one escaped site reverted to raw (commit `6abd828`)

```
thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (1534569) panicked at src/ui/screens/render_escape_guard.rs:1018:17:
NormalScreen (src/ui/screens/normal.rs) [dashboard] rendered ['\u{e0041}', '\u{ad}'] into the terminal buffer. Those characters render as nothing, so what the operator reads is not what the value is.
```

Restored. Two different mechanisms (census, probe), two different reds.

### RED 5 — the arrival assertion, proven load-bearing (commit `f3a2a67`)

The plan asks what the probe reports when the fixture supplies no identity.
Measured by emptying the `DeleteConfirmScreen` fixture to
`ctx_with_aliases(&[])` and `DeleteConfirmScreen::new(String::new())`:

```
panicked at src/ui/screens/render_escape_guard.rs:511:21:
DeleteConfirmScreen (src/ui/screens/delete_confirm.rs) is adjudicated as rendering identity, but a clean identity handed to its fixture never reached the buffer. Either the disposition is wrong or the fixture does not put the screen in a state that renders it — and until this passes, every assertion below it would pass by silence.
```

### RED 6 — UNPLANNED, and the most important one: a vacuous fixture state

Not asked for by the plan. While reviewing why `NormalScreen`'s "filter footer"
state passed, it turned out the state set `ctx.filter_text` but left
`NormalScreen.searching` false. `render_footer` dispatches on `searching`, so
the state rendered the NORMAL footer and never looked at the filter at all — the
assertion had been passing by silence. Setting `NormalScreen { searching: true }`
turned it red immediately:

```
NormalScreen (src/ui/screens/normal.rs) [dashboard with filter footer] rendered ['\u{e0041}'] into the terminal buffer. Those characters render as nothing, so what the operator reads is not what the value is.
```

`render_search_footer` had been drawing `ctx.filter_text` raw the whole time.
The fixture now carries a comment saying exactly this, so the next reader
inherits the trap rather than rediscovering it.

## The measurement that corrected an inherited claim

Round 7's review and verification pass 8 both asserted, without measuring, that
a hostile legacy key would be **reordered** in the TUI. Measured here with a
scratch reporter against the unescaped `delete_confirm.rs`:

```
HOSTILE INPUT   : "demo\u{e0041}r\u{ad}un"
ESCAPED FORM    : "demoU+E0041rU+00ADun"
RENDERED ROW    : "Remove \"demo\u{e0041}run\"? This only unregisters it — project files are not deleted. [y/n]"
INVISIBLE CHARS : ['\u{e0041}']
CONTAINS RAW    : false
CONTAINS ESCAPED: false
CONTAINS CLEAN  : false
```

ratatui 0.30's `Buffer` **DROPS zero-width graphemes** (`U+00AD` is simply gone)
and passes the **tag block through INTACT** (`U+E0041` survives). So the TUI does
not reorder — it silently *deletes*, and a legacy key renders as a DIFFERENT
string that can collide with a real project of that name. Both harms close under
the same escaping, but the direction of the assertions had to change:
`CONTAINS RAW: false` means **a probe asserting "the raw form is absent" would
pass vacuously against an unescaped site, forever**. That is why this probe
asserts arrival of the clean stem first and then presence of the escaped form.

## A second inherited claim corrected: the two refusal echoes were not raw

The plan (and T-21-21-04) expected `src/main.rs`'s two other refusal echoes to be
emitting RAW invisible bytes. Measured with a scratch program against this
toolchain, they were not — `{alias:?}` already escapes the whole class:

```
U+202E Cf                              -> "demo\u{202e}"   survivors: []
U+00AD Cf                              -> "demo\u{ad}"     survivors: []
U+E0041 Cf (tag)                       -> "demo\u{e0041}"  survivors: []
U+180E Cf                              -> "demo\u{180e}"   survivors: []
U+FE0F Mn + Default_Ignorable          -> "demo\u{fe0f}"   survivors: []
U+034F Mn + Default_Ignorable          -> "demo\u{34f}"    survivors: []
U+E0100 Mn + Default_Ignorable (VS17)  -> "demo\u{e0100}"  survivors: []
```

So the harm was never "an invisible character reaches the terminal through a
refusal". It was this phase's own shape one level down: **the guarantee rested on
`core::char::is_printable` — a standard-library table that no test in this tree
pins, no doc in this tree names, that moves with a toolchain upgrade, and that is
a SECOND spelling of a class this project already derives for itself.** Two
spellings of one judgment is precisely the defect D-19-2 removed from
`Alias::new`. Escaping at the producer replaces the accident with the project's
own derived class in the project's own notation, certified by
`a_refusal_never_carries_an_invisible_character_into_its_own_message`.

## The compiler's own list, beside the planner's

| Planner (21-21-PLAN.md) | Compiler (this tree, HEAD numbering) | Agrees? |
|---|---|---|
| `src/registry.rs:640` | `src/registry.rs:644` | file yes, line **no — 4 off** |
| `src/main.rs:103` | `src/main.rs:103` | yes |
| `src/main.rs:111` | `src/main.rs:111` | yes |
| `src/main.rs:371` | `src/main.rs:371` | yes |
| **count: 4** | **count: 4** | **yes** |

The count is exactly four, as the planner measured. The registry line number is
not: `640` at HEAD is inside the `Err` arm of the refusal-logging `warn!` above
the real site, where `alias` is still the raw `String` and therefore **not** an
`Alias` interpolation. Recorded rather than absorbed.

**The order-of-compilation trap, reproduced.** The library target compiles before
the binary target, so the FIRST `cargo check --all-targets` after the withdrawal
reported `src/registry.rs` and nothing else — one site when there were four. The
remaining three appeared only after the library site was fixed. A mechanism that
reports one site when there are four is this phase's signature failure shape;
here it is a property of the tool rather than of the code, and anyone repeating
this must re-check after each fix rather than believing the first list.

## D-17-3, both halves, measured against the binary

Hand-built legacy `config.json`; the hostile key constructed from `chr(0x202E)`
in a generator script, never pasted (prohibition 8).

**BEFORE** (arm reverted to its HEAD form, rebuilt):

```
exit code: 0
stdout raw repr: "Removed project 'gsd-\u{202e}nur'\n"   <- respelled; see note below
stdout code points: [... 0x67, 0x73, 0x64, 0x2d, 0x202e, 0x6e, 0x75, 0x72, 0x27]
keys after: [['0x6b','0x65','0x65','0x70','0x6d','0x65']]
legacy key still present: False       unrelated key survived: True
```

The `0x202e` in the code-point list is the Trojan Source spoof, reproduced: a
terminal renders that line as `Removed project 'gsd-run'`.

> **Note on the `raw repr` line above — and it is a finding about this SUMMARY,
> not a formatting nicety.** The Python `repr` genuinely emitted the raw
> `U+202E`, and pasting it verbatim put a raw bidi override into this file. That
> is a **prohibition-8 violation in the artifact written to end overclaiming** —
> this phase's signature shape, one more time, in the document auditing it. It
> was caught by re-running the prohibition-8 scanner against the SUMMARY itself
> after committing it, and the character is respelled `\u{202e}` above. The
> **code-point list** beneath it is the load-bearing evidence and always was: it
> shows `0x202e` present BEFORE and absent AFTER without needing the character
> itself to appear anywhere.

**AFTER** (`LegacyRegistryKey`):

```
exit code: 0
stdout raw repr: "Removed project 'gsd-U+202Enur'\n"
stdout code points: [... 0x67, 0x73, 0x64, 0x2d, 0x55, 0x2b, 0x32, 0x30, 0x32, 0x45, 0x6e, 0x75, 0x72, 0x27]
keys after: [['0x6b','0x65','0x65','0x70','0x6d','0x65']]
legacy key still present: False       unrelated key survived: True
```

**Acceptance is byte-identical** — both runs removed the same key and left the
same one — and the printed line changed from raw to escaped. No `0x202e` in the
AFTER output.

That the raw route is IMPOSSIBLE rather than merely discouraged was measured by
writing the interpolation and compiling:

```
error[E0277]: `LegacyRegistryKey` doesn't implement `std::fmt::Display`
   --> src/main.rs:164:46
    |
164 |             println!("Removed project '{}'", key);
    |                                        --    ^^^ `LegacyRegistryKey` cannot be formatted with the default formatter
    |
    = help: the trait `std::fmt::Display` is not implemented for `LegacyRegistryKey`
```

Then removed. `tests/registry_test.rs` is unmodified — `rtk proxy git diff --
tests/registry_test.rs` is empty — and
`a_legacy_alias_the_alphabet_refuses_is_still_removable` passes.

## The falls-out proof

`src/main.rs`'s two other refusal echoes are **absent from this plan's diff**
(`rtk proxy git diff -- src/main.rs | grep 'Error: {refusal}'` shows exactly one
added line, at `judged_alias_or_exit`, where the duplicate outer escape was
removed). Run against the built binary with a hostile alias:

```
=== main.rs Add arm  (was :91, NOT in this task's diff) ===
  exit: 1
  first line: 'Error: the alias "gsd-U+202Enur" carries a character that renders as nothing, ...'
  raw U+202E occurrences in output: 0
  contains the project's own U+ notation: True

=== main.rs Scan arm (was :359, NOT in this task's diff) ===
  exit: 1
  first line: 'Error: the alias "gsd-U+202Enur" carries a character that renders as nothing, ...'
  raw U+202E occurrences in output: 0
  contains the project's own U+ notation: True
```

Closed by the producer, not patched.

## The derivation, proven by three agreeing numbers

```
$ rtk proxy grep -rn "impl Screen for" src/ --include=*.rs | rtk proxy grep -c .
11

$ rows in SCREEN_IDENTITY_DISPOSITIONS
11

$ the set screen_implementors_from_source() returns
11   (asserted equal to the table, both ways, by the passing
      the_screen_census_matches_the_tree)

$ rtk proxy grep -n "read_dir" src/ui/screens/render_escape_guard.rs | rtk proxy grep -c .
3    (>= 1: the walk reaches the tree by read_dir, not by a path list)

$ rtk proxy grep -n "fn census_offences\|census_offences(" src/ui/screens/render_escape_guard.rs
358:fn census_offences(          <- one definition
884:        let offences = census_offences(&derived, &table);      <- the control
911:            census_offences(&derived, &derived).is_empty(),    <- the clean direction
923:        let offences = census_offences(&derived, &table);      <- the live census
```

**The grep count was 12 until commit `2b5ca7a`**, because this module's own doc
comment spelled the implementation header literally while describing the
twelfth-screen experiment. The walk already drops comment lines — deliberately,
so a doc naming the trait cannot forge a member — but a naive `grep -rn` does
not, and a reader cross-checking would have got 12 against the census's 11. The
doc was reworded and the reason recorded beside it. Reported here rather than
silently fixed, because "the numbers agree" is exactly the kind of claim this
phase has been burned by.

## Deletion evidence: `src/ui/project_list.rs`

```
$ rtk proxy grep -rn "mod project_list" src/          -> exit 1 (no declaration, BEFORE the deletion)
$ rtk proxy ls src/ui/project_list.rs                 -> No such file or directory (AFTER)
$ rtk proxy git log --oneline -S"mod project_list" -- src/ui/mod.rs
c297631 feat(05-02): refactor InputMode to Screen trait + screen stack architecture
1e8dbdc feat(01-03): wire async event loop, terminal lifecycle, and App state machine
```

`c297631` is the commit that removed the declaration: it introduced the `Screen`
trait and `screens/normal.rs`, which supersede this file. `src/ui/mod.rs`
declares only `roadmap_widget` and `screens`. 327 lines the build never
compiled, whose only surviving effect was to hold the single UI call to
`display_identity` and thereby certify the TUI as escaped to two independent
readers.

**The premise held — a file the build never compiled cannot change a test result:**

| Gate | Before deletion | After deletion |
|---|---|---|
| `cargo test --lib` | 1051 passed, 0 failed, 1 ignored | 1051 passed, 0 failed, 1 ignored |
| `cargo test --workspace --no-fail-fast -- --test-threads=2` | 1365 passed, 0 failed, 14 ignored | 1365 passed, 0 failed, 14 ignored |

The two doc comments citing line numbers **inside** the deleted file —
`src/ui/screens/driver_inject.rs` (`project_list.rs:279`) and
`src/ui/screens/add_project.rs` ("the old code rendered project_list") — are
corrected in the same commit, so the deletion leaves no dangling citation.

## Prohibition audit — all ten, against THIS plan's own diff

Every row is a command and its output. No row is a grep for a sentence of prose.

| # | Prohibition | Command | Output | Verdict |
|---|---|---|---|---|
| 1 | No hand-written list of render sites | `grep -n "read_dir" src/ui/screens/render_escape_guard.rs \| grep -c .` | `3` | **HELD.** Enumeration is a `read_dir` walk; the table is asserted equal to it in both directions and was observed catching a planted twelfth implementor (RED 3). |
| 2 | No work against ROADMAP criterion 4 | `git diff f1a9d0d..HEAD --stat -- tests/driver_injection_corpus.rs` | *(empty)* | **HELD.** File untouched; its 10 `#[ignore]`d tests still report `10 ignored` in the workspace run. |
| 3 | No regression of `remove`'s raw acceptance (D-17-3) | `git diff f1a9d0d..HEAD -- tests/registry_test.rs` | *(empty)* | **HELD.** Pin unmodified and passing; binary-level BEFORE/AFTER above shows byte-identical acceptance. |
| 4 | No fixing the four pre-existing `--all-targets` lints | `cargo clippy --all-targets -- -D warnings` | 3× `assert_eq!` with a literal bool in `src/browser.rs:131-133`; 1× owned-instance-for-comparison in `src/project_creator.rs:146` — **4 errors, unchanged** | **HELD.** `git diff --stat` names neither file. |
| 5 | No deleting `WITNESS_ALLOWED_ELSEWHERE` | `git diff f1a9d0d..HEAD --stat -- tests/spawn_seam_guard.rs` → *(empty)*; `grep -c WITNESS_ALLOWED_ELSEWHERE tests/spawn_seam_guard.rs` → `5` | file untouched, table present | **HELD.** This plan touches no file under `tests/` at all (`git diff f1a9d0d..HEAD -- tests/` is empty). |
| 6 | No certifying a structural claim with a prose grep | *(this table; every row is a command, a count, a compiler error, a test name, or a binary run)* | — | **HELD.** Every mechanism claim above names a test observed red without the fix, or a binary-level run. |
| 7 | No claiming a bound no committed control goes red for | *(the limits block in `render_escape_guard.rs`'s module doc)* | four residuals, each with its failure direction and an explicit statement of whether a control bounds it or it is disclosed only | **HELD** — see "Limits and residuals" below, which repeats them and adds the ones found during execution. |
| 8 | No raw invisible/bidi/tag/VS character in any file | scanner over `git diff f1a9d0d..HEAD` (code) → `diff lines scanned: 2869` / `found: 0`; then **re-run against this SUMMARY itself** | code: **0**. This SUMMARY, first attempt: **1** — a raw `U+202E` at line 359, pasted from a Python `repr`. Respelled `\u{202e}`; re-scan: **0**. | **HELD, after a caught violation.** The first scan was scoped to the code diff and was already stale by the time the SUMMARY existed. Recorded rather than quietly fixed: the prohibition was violated in the very artifact auditing it, which is this phase's signature shape. See the note under the D-17-3 measurement. |
| 9 | No rewriting or deleting a line of the 21-19 records | `git diff --numstat f1a9d0d..HEAD -- .../21-19-PLAN.md .../21-19-SUMMARY.md` | `56  0  21-19-PLAN.md` / `78  0  21-19-SUMMARY.md` | **HELD.** Zero deletions in both. |
| 10 | No flipping a REQUIREMENTS.md requirement | `git log --format=%H f1a9d0d..HEAD -- .planning/REQUIREMENTS.md` | *(empty)* | **HELD.** |

## Named-shape audit

| # | Named shape | Owner | Resolved by |
|---|---|---|---|
| 1 | The escaping mechanism shipped into a file the module tree does not contain (CR-01) | 21-21 T1 | Deleted with evidence (above). Cannot recur: `the_screen_renders_identity_escaped` inspects a rendered `Buffer`, which an uncompiled file cannot contribute to. |
| 2 | The live TUI rendering identity raw at an unenumerated number of sites (CR-01) | 21-21 T1/T2 | `render_escape_guard::the_screen_census_matches_the_tree` (both ways) + `::the_screen_renders_identity_escaped` (every implementor, every state). Both observed red. |
| 3 | A destructive confirm rendering a name that is not the key (T-21-21-01) | 21-21 T1 | `src/ui/screens/delete_confirm.rs` — prompt, toast and live-run refusal escaped; red-first (RED 2), then green. |
| 4 | `Removed project '{}'` echoing raw while `remove` must keep accepting raw | 21-21 T3 | `registry::LegacyRegistryKey`; both halves measured against the binary; the no-`Display` compile error quoted. |
| 5 | Two `Add` refusal echoes raw while a third escapes | 21-21 T3 | `Display for AliasRefusal` escapes at the producer; the two sites are absent from the diff and emit escaped output at the binary level. **Premise partly corrected** — see "A second inherited claim corrected" above. |
| 6 | `NotPlainComponent` reachable for two values with zero test consumers (WR-04) | 21-21 T3 | `registry::tests::the_two_values_that_still_reach_not_plain_component_still_reach_it`, which also asserts the other direction. |
| 7 | 21-19 truth 5, `21-19-SUMMARY.md:180`, named-shape row 8 marked CLOSED | 21-21 T3 | Dated append-only corrections in commit `b8b9d78`, the commit that falsified them; row 8 REOPENED. |
| 8 | `WITNESS_ALLOWED_ELSEWHERE`'s doc; `hooks.rs:155`; `advisory.rs:569`; `narrow_visible`'s message; `carries_visible_content`'s free-text residual | **21-22** | **DELEGATED.** Not this plan's work; this plan touches no file under `tests/`. |
| 9 | Criterion 4's behavioural half | **nobody, deliberately** | **UNOWNED BY DESIGN.** Permanently agent-unclosable (needs an authenticated Claude subscription); tracked in `deferred-items.md`; prohibition 2 forbids chasing it and was held. |

## Record corrections

| Record | Location | What was corrected | Shape |
|---|---|---|---|
| `21-19-PLAN.md` | `must_haves.truths`, the render-surface truth (fifth counting from one) | Its THIRD surface, "the TUI project list", did not exist in the built binary. Measured: `display_identity` had three production call sites at HEAD, one of them in `src/ui/project_list.rs`, which `src/ui/mod.rs` has not declared since `c297631`. The first two surfaces it names are correct. | Appended `<correction>` block, dated, quoting the truth verbatim. **56 additions, 0 deletions.** |
| `21-19-SUMMARY.md` | Accomplishments bullet at line 180 | "the three render sites stopped being reorderable" — there were two, not three. | Appended correction section, dated, quoting the bullet verbatim. |
| `21-19-SUMMARY.md` | Named-shape row 8 | Marked **CLOSED** on the strength of the site that did not ship. | **REOPENED**, with a table naming what now closes each of its three halves. **78 additions, 0 deletions.** |

Both corrections are filed in `b8b9d78` — the same commit that did the work
falsifying them — so the record and the correction cannot drift apart.

## Limits and residuals — each with its failure direction

Repeated from the module doc, plus those found during execution. Prohibition 7
applies to this section most of all.

| # | Residual | Direction | Bounded by a committed control? |
|---|---|---|---|
| 1 | The probe sees only what a screen renders under the states its fixture constructs. | **Under-detection, silent.** | **Partially.** Each disposition row NAMES its states, and `DetailScreen` is rendered in all eleven sub-views rather than only its default. **NOT bounded within a state:** the Backlog, Sessions, Archive, Browse and Defaults tabs, and the Driver tab's run list and run detail, draw from `view_cache` / `archive_cache` / `active_sessions` entries that `probe_ctx` leaves at their defaults, so those tabs render their EMPTY branch and their populated branches are exercised by **no committed control**. Disclosed, not closed. |
| 2 | The probe judges the invisible class, not homoglyphs. | **Under-detection, by design.** | **No — disclosed only.** TR39 confusables remain a separate roadmap item. At identity *seams* the harm is closed by the finite alphabet (`text::is_identity_char`), which admits no Cyrillic; in free text it stays open. |
| 3 | A render surface not reached through `Screen::render` — a widget drawn from elsewhere, or a future second entry point beside `ui::render`. | **Under-detection, silent.** | **Partially.** How the residual can GROW is bounded by the census, not the probe: a new entry point that IS a `Screen` is reported by `the_screen_census_matches_the_tree` (the control observed red at RED 3). A new entry point that is NOT a `Screen` is reported by nothing here. |
| 4 | The probe cannot assert the RAW form is absent — ratatui deletes zero-width graphemes before a cell exists. | Limit on the **assertion**, not on the code. | **Yes.** Replaced by arrival-of-the-clean-stem plus presence-of-the-escaped-form, both observed red (RED 5 and RED 2 respectively). |
| 5 | *(found during execution)* A fixture can put a screen in a state that never exercises the path under test, and the assertion then passes by silence. | **Under-detection, silent.** | **No mechanism.** Caught here only by reading (RED 6). The fixture now documents the specific trap it fell into; nothing prevents the next one. This is the sharpest residual in the plan and it is named rather than papered over. |
| 6 | `AliasRefusal`'s Display no longer depends on `core::char::is_printable`, but `{:?}` is still used to QUOTE the already-escaped value. | Cosmetic only. | **Yes.** `a_refusal_never_carries_an_invisible_character_into_its_own_message` asserts against the class directly and additionally asserts the project's own `U+` notation is present, so a std-lib change cannot silently become the thing doing the work again. |

## Deferred

| Item | Why not here | Owner |
|---|---|---|
| **The type-level UI derivation** — propagate a no-`Display` identity newtype into `AppContext::filtered_aliases` and every screen's `alias` field. | The right long-term shape, and explicitly NOT done here (D-21-4). Measured by planting the newtype and reverting: **25 direct compile errors** across `app.rs`, `screens/mod.rs`, `normal.rs` and `add_project.rs`, plus an unbounded cascade through `selected_alias` (19 mentions). A whole-layer mechanical retype in a round that had to converge. It would also still not reach the non-alias `.planning/` identities the census does. | future round |
| The four pre-existing `cargo clippy --all-targets` lints (`src/browser.rs:131-133`, `src/project_creator.rs:146`). | Prohibition 4. Confirmed pre-existing and untouched. | **21-22** to record as a deferred todo |
| Populated-cache branches of six detail tabs (residual 1 above). | Would require constructing archive, git, session and backlog caches in the fixture; a materially larger fixture than this round's scope. | future round |
| TR39 confusables (residual 2). | Out of scope by 21-19's prohibition 5. | separate roadmap item |
| ROADMAP criterion 4's behavioural half. | Permanently agent-unclosable; prohibition 2. | `deferred-items.md` |

## Edge-probe assumptions carried forward

The plan's `<edge_probe_audit>` surfaced 7 applicable rows: 4 auto-resolved into
**21-22**, and 3 flagged as assumptions and **not** resolved — SAFE-08, DRIVE-01
and DRIVE-03, all arriving `unclassified`. This round is a doc-and-hardening
round: it introduces no new boundary and no new arithmetic for any of the three,
so there is no honest boundary or precision predicate to write for them.
Manufacturing one would be the overclaim this plan exists to end. **They remain
open, unresolved, and inherited by round 9.** The assumption behind leaving them
— that their edges are covered by the criteria verification pass 8 marked
VERIFIED (criteria 1, 2, 5) — is an assumption, and is labelled as one here as
it was in the plan.

## Decisions Made

Followed the plan's decision table (D-21-1 … D-21-6) as written; all six are
listed in the frontmatter with their measured rationale. Three decisions were
forced during execution and are recorded here because the plan did not cover them:

1. **`text::is_invisible_formatting_char` widened from module-private to
   `pub(crate)`.** The probe asserts zero characters of the class reach a
   terminal cell and must consult the ONE production spelling; a probe with its
   own copy would be a second spelling that can drift. The doc at the site records
   the single consumer and why. No public API change.
2. **`ctx_with_aliases` in `screens/mod.rs`'s test module widened to
   `pub(super)`.** The plan asks the probe to reuse it rather than grow a sixth
   full-field `AppContext` literal.
3. **`detail.rs` got a file-local `shown()` helper** rather than fifty inline
   `crate::text::display_identity(...)` calls, so the identity-split rule is
   stated once, in one doc, and a reader of the diff sees the rule rather than
   fifty instances of it. Its doc is explicit that what holds those call sites
   honest is the behavioural probe, **not** the discipline of whoever adds the
   next one.

## Deviations from Plan

### Auto-fixed issues

**1. [Rule 1 — Bug] Two reachable panics from byte-slice truncation of disk-read text**

- **Found during:** Task 2, while escaping the sites the probe named.
- **Issue:** `&self.command_text[..47]` in `src/ui/screens/queue_delete_confirm.rs`
  and `&phase.name[..name_max.saturating_sub(3)]` in `src/ui/roadmap_widget.rs`
  panic when the byte index is not a UTF-8 char boundary. Both strings are read
  off disk from `.planning/queue.md` and `.planning/ROADMAP.md` — files the tool
  does not own — so both were a denial of service driven by third-party content,
  at a trust boundary this phase exists to harden.
- **Fix:** truncate by `char` rather than by byte, and escape *before* measuring,
  since the escaped form is what occupies cells.
- **Verification:** both sites are on paths the probe renders; suite green.
- **Committed in:** `6abd828`.

**2. [Rule 3 — Blocking] The census's live assertion had to be `#[ignore]`d for one commit**

- **Found during:** Task 1.
- **Issue:** the plan's Task 1 says the census "is red for [the other ten rows]…
  and the commit message says so", while the same task's acceptance criteria
  require `cargo test --lib` → 0 failures. Those two are not simultaneously
  satisfiable.
- **Fix:** `the_screen_census_matches_the_tree` carries
  `#[ignore = "red until Task 2 adjudicates the remaining ten implementors —
  deny-by-default working as designed"]` for exactly one commit, with its
  verbatim red output in the commit message (RED 1 above). Task 2's commit
  un-ignores it. The red was observed and recorded; only its effect on the suite
  gate was deferred.
- **Verification:** `d8e0518` shows 1050 passed / 0 failed / 2 ignored;
  `6abd828` onward shows 0 ignored in `--lib`.
- **Committed in:** `d8e0518` (ignored) → `6abd828` (un-ignored).

**3. [Rule 2 — Missing critical] A third human-read site in `delete_confirm.rs`**

- **Found during:** Task 1.
- **Issue:** the plan names the confirmation prompt and the removal toast. The
  live-run refusal (`'{alias}' still has a driver run …`) is a third sentence a
  human reads in the same file, composed from the same raw alias.
- **Fix:** escaped, applying the plan's own stated rule at a site in the file the
  task already owns.
- **Committed in:** `f3a2a67`.

### Deviations from the plan's stated facts, reported rather than absorbed

| Plan states | Measured | Consequence |
|---|---|---|
| Withdrawing `Display for Alias` reaches `src/registry.rs:640` | The compiler names `src/registry.rs:644`; `640` is inside the *preceding* `warn!`'s `Err` arm, where `alias` is still a raw `String` | Count (4) and file are right; the line number is off by 4. No change to the work. |
| T-21-21-04: `src/main.rs:91` and `:359` echo `AliasRefusal` **raw** | They echoed `{:?}`-escaped text; the class was already covered — by `core::char::is_printable`, a std-lib table, not by this project's derived class | The harm is real but different: a second, unpinned, undocumented spelling of one judgment. Fixed as planned; premise corrected here and in the code's doc. |
| `rtk proxy grep -rn "NotPlainComponent" src/ tests/` → "three hits, all `src/registry.rs`" | That literal command yields **nine** at HEAD — six are `GoalReason::PhaseNotPlainComponent` in `src/driver/goal.rs`, a different enum whose name contains the substring | The plan's *claim* (three, all registry.rs, zero test consumers) is true of the `AliasRefusal` variant; its stated *command* does not produce that number. Before: 3 occurrences, 0 test consumers. After: 5 occurrences, 1 test consumer. |

---

**Total deviations:** 3 auto-fixed (1 bug, 1 blocking, 1 missing-critical) + 3
corrections to the plan's stated facts.
**Impact on plan:** none to its structure. Every mechanism the plan specifies was
built, observed red, and observed green. The three fact corrections make the
record more accurate rather than changing what was done.

## Issues Encountered

- **`driver_reattach` is a parallelism flake, characterised.** Under
  `--test-threads=2` it fails intermittently and on a *different* test each run
  (`a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` once,
  `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`
  another). Under `--test-threads=1` the binary is **3/3 green**, repeatedly. This
  plan touches no file under `tests/` at all, so it is neither caused nor masked
  by this work. Recorded with its shape so the next round does not re-diagnose it.
- No authentication gates. No architectural decisions requiring escalation.

## Known Stubs

None. Every mechanism this plan introduces is wired to real production render
paths and is exercised by a committed test that was observed failing without the
fix. The unexercised render *branches* named in residual 1 are not stubs — the
code there is real and shipped; what is missing is fixture coverage, and that is
disclosed as such rather than counted as done.

## Verification results

| Gate | Result |
|---|---|
| `rtk proxy cargo build` | clean |
| `rtk proxy cargo test --lib` | **1055 passed, 0 failed, 0 ignored** |
| `rtk proxy cargo test --test registry_test` | **15 passed, 0 failed** |
| `rtk proxy cargo test --workspace --no-fail-fast -- --test-threads=2` | 1368 passed, **1 failed**, 13 ignored — the failure is the documented `driver_reattach` parallelism flake |
| `rtk proxy cargo test --workspace --no-fail-fast -- --test-threads=1` | **1369 passed, 0 failed, 13 ignored** (35 test binaries) |
| `rtk proxy cargo clippy --lib -- -D warnings` | clean |
| `rtk proxy cargo clippy --all-targets -- -D warnings` | 4 errors — the pre-existing lints in `src/browser.rs` and `src/project_creator.rs`, untouched by prohibition 4 |

Every number above was re-measured under `rtk proxy` against this plan's own
final tree. None is inherited from `21-REVIEW.md`, `21-VERIFICATION.md`, or the
plan's text.

## Next Phase Readiness

- The render surface is closed by three mechanisms with measured reach, and a
  twelfth screen added tomorrow in any file under `src/` goes red with nobody
  editing a list.
- `21-22` owns named-shape row 8 and should record the four pre-existing
  `--all-targets` clippy lints as a deferred todo.
- Round 9 inherits: the three flagged edge-probe assumptions (SAFE-08, DRIVE-01,
  DRIVE-03), residual 1's unexercised cache branches, and residual 5 — a fixture
  that puts a screen in the wrong state still passes by silence, and nothing
  here prevents the next one.
- `.planning/REQUIREMENTS.md` is untouched; requirement status remains for a
  passed verification to decide.
- STATE.md and ROADMAP.md are deliberately not modified by this plan — this
  executor ran in a worktree and the orchestrator owns those writes.

## Self-Check: PASSED

Files this SUMMARY claims were created, checked on disk:

```
$ rtk proxy ls -1 src/ui/screens/render_escape_guard.rs .planning/.../21-21-SUMMARY.md
.planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-21-SUMMARY.md
src/ui/screens/render_escape_guard.rs

$ [ -f src/ui/project_list.rs ]
CONFIRMED DELETED: src/ui/project_list.rs
```

Commits this SUMMARY claims, checked in `git log`:

```
$ git log --format='%h %s' f1a9d0d..HEAD
db6194b docs(21-21): complete the deriving-the-render-surface plan
2b5ca7a docs(21-21): stop the census's own doc from inflating a cross-check grep
b8b9d78 feat(21-21): withdraw the trait impl, escape at the producer, split accept from echo
6abd828 fix(21-21): adjudicate all eleven implementors, and prove a twelfth goes red
f3a2a67 fix(21-21): escape the destructive confirm, and delete the orphan that certified it
d8e0518 test(21-21): the render-surface census and probe, observed RED before any fix
```

All five task commits present, in the stated order, with the red arm preceding
its fix. The sixth is this document.

Prohibition-8 scanner re-run over the three `.planning/` artifacts this plan
writes, after the violation above was corrected:

```
21-21-SUMMARY.md: 0 raw invisible/bidi/tag/VS characters
21-19-PLAN.md:    0 raw invisible/bidi/tag/VS characters
21-19-SUMMARY.md: 0 raw invisible/bidi/tag/VS characters
```

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-25*
