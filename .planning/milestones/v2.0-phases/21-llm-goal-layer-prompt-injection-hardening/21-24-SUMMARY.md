---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 24
subsystem: security
tags: [unicode, trojan-source, ansi-injection, newtype, one-producer, error-display]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    provides: "crate::text::Untrusted (the carrier withholding Display/AsRef/Deref/Borrow/Into<Cow>/serde), crate::text::Rendered, crate::text::render_for_terminal, crate::text::strip_terminal_controls — all from 21-23 (wave 1)"
provides:
  - "OptInError's four alias fields and DriveError's AliasNotVisible/RunIdInvalid/TargetPhaseInvalid fields typed as text::Untrusted, so neither Display compiles unescaped and src/main.rs's four echo sites are correct WITHOUT appearing in the diff"
  - "registry::project_not_found — the ONE producer of the not-found sentence, taking the carrier and escaping at the producer"
  - "registry::remove_project(config, &LegacyRegistryKey) — the signature that makes the raw bail! unwritable (CR-01)"
  - "registry::LegacyRegistryKey demoted to a thin wrapper over text::Untrusted, #[derive(Debug)] removed (WR-04), as_untrusted added"
  - "every CLI echo in src/main.rs routed through render_for_terminal, so a raw ESC no longer reaches stdout (WR-01)"
  - "a_refusal_never_carries_a_raw_control_or_invisible_character — ONE control over both error types, both halves of remove, and the two opt-in functions, with a compile-forced subject list"
  - "argv_visible's refuse closure takes Untrusted, so every argv position on the drive boundary inherits the carrier"
affects: [21-25, 21-26, verification-pass-10]

actuals:
  tokens: 15504
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Retype the error FIELD, not the Display body: it protects the value, not just the message, so a caller that formats err.alias itself is caught too"
    - "Bind the escape ONCE above the match, so a variant added tomorrow cannot embed its value without going through it"
    - "Derive a test's class predicate from the production function rather than re-listing the class (is_terminal_control_char asks strip_terminal_controls)"
    - "Gate a control's subject list with a wildcard-free match over each enum, so a new variant is a compile error rather than an untested one"
    - "One helper wraps at the seam (argv_visible), so five closures inherit the carrier instead of five sites being a list that is six next round"

key-files:
  created: []
  modified:
    - src/error.rs
    - src/registry.rs
    - src/main.rs
    - src/driver/mod.rs
    - src/executor/mod.rs
    - src/ui/screens/delete_confirm.rs
    - src/ui/screens/driver_confirm.rs
    - tests/registry_test.rs
    - tests/journal_run_paths.rs

key-decisions:
  - "D-21-13 (costly): retype the error FIELDS rather than escape inside the two Display impls — escaping in Display protects the message, not the value, and driver_confirm.rs was formatting the raw field itself"
  - "D-21-14 (costly/reversible): remove_project takes &LegacyRegistryKey; record_opt_in and clear_opt_in do NOT change signature (18 call sites across four plans' files) and reach the producer at a smaller, disclosed lever"
  - "D-21-15: ONE producer for the not-found sentence, taking the carrier rather than a string, so no call site can reach it with a value it escaped itself"
  - "D-21-16: LegacyRegistryKey wraps Untrusted rather than being deleted — as_raw_for_lookup_only says something as_raw_for_logic_only does not"
  - "D-21-17: #[derive(Debug)] removed rather than hand-written, after measuring that no {:?} consumer exists in src/ or tests/"
  - "D-21-18: do_start_run delegates to OptInError::NotOptedIn's Display; the o-key affordance moved INTO the variant so delegation costs the user nothing"
  - "DEVIATION: impl Display for Rendered ignores the formatter width — reported as a wave conflict, worked around at one call site, NOT fixed in src/text.rs (prohibition 1)"

requirements-completed: [DRIVE-01, DRIVE-03, SAFE-08]

coverage:
  - id: D1
    description: "CR-02 closed at the producer: all seven alias-carrying error fields are text::Untrusted, both Display impls bind the escape above their match, and src/main.rs's four eprintln! sites are correct without appearing in Task 1's diff"
    requirement: SAFE-08
    verification:
      - kind: unit
        ref: "src/error.rs#every_alias_carrying_variant_escapes_the_value_it_names"
        status: pass
      - kind: unit
        ref: "src/registry.rs#a_refusal_never_carries_a_raw_control_or_invisible_character"
        status: pass
      - kind: other
        ref: "git diff --stat for commit 017d82d names src/driver/mod.rs, src/error.rs, src/executor/mod.rs, tests/journal_run_paths.rs — and NOT src/main.rs"
        status: pass
    human_judgment: false
  - id: D2
    description: "CR-01 closed by the parameter type: remove_project takes &LegacyRegistryKey, the raw bail! does not compile, and exactly one executable spelling of the not-found sentence exists"
    requirement: SAFE-08
    verification:
      - kind: other
        ref: "verbatim error[E0277] captured against the new signature and quoted in remove_project's doc"
        status: pass
      - kind: other
        ref: "executable-line count of \"Project not found\" in src/registry.rs == 1 (4 further hits are doc lines quoting the sentence removed)"
        status: pass
      - kind: integration
        ref: "target/debug/gsd-meta-manager remove <hostile absent alias> against a hand-built legacy config.json, piped through cat -v: raw ESC + raw U+202E before, escaped after"
        status: pass
    human_judgment: false
  - id: D3
    description: "WR-01's ESC half closed at the CLI: every echo in src/main.rs goes through render_for_terminal, and no raw ESC reaches stdout from list or from Removed project"
    requirement: SAFE-08
    verification:
      - kind: integration
        ref: "target/debug/gsd-meta-manager list on a legacy key ev\\u{1b}[31mil through cat -v: ^[[31m present before, absent after"
        status: pass
      - kind: other
        ref: "grep -c \"display_identity(\" src/main.rs == 0"
        status: pass
    human_judgment: false
  - id: D4
    description: "D-17-3's accept half is intact: remove still accepts byte-for-byte what an older build registered, with every assertion of the pin untouched"
    requirement: DRIVE-01
    verification:
      - kind: integration
        ref: "tests/registry_test.rs#a_legacy_alias_the_alphabet_refuses_is_still_removable — diff shows ONLY a wrapped constructor call and one import"
        status: pass
      - kind: integration
        ref: "target/debug/gsd-meta-manager remove on both hostile legacy keys of a hand-built config.json: exit 0 each, only the clean key left"
        status: pass
    human_judgment: false
  - id: D5
    description: "One control covers both error types, both halves of remove and the two opt-in functions; its subject list is compile-forced and it was observed red in three directions"
    requirement: DRIVE-03
    verification:
      - kind: unit
        ref: "src/registry.rs#a_refusal_never_carries_a_raw_control_or_invisible_character (88 subject/value pairs)"
        status: pass
      - kind: other
        ref: "pre-fix red against a throwaway copy of d712061: 58 of 88 pairs leaked (42 invisible-class, 16 control-class)"
        status: pass
      - kind: other
        ref: "0 executable `_ =>` arms in src/registry.rs; both classifiers are wildcard-free matches over their whole enum"
        status: pass
    human_judgment: false
  - id: D6
    description: "The o-key affordance survives the delegation of do_start_run's refusal to OptInError::NotOptedIn"
    requirement: DRIVE-01
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#the_delegated_opt_in_refusal_still_names_the_key_to_press"
        status: pass
    human_judgment: false
  - id: D7
    description: "The residuals this plan leaves — record_opt_in/clear_opt_in protected at the message rather than the parameter, and the control not seeing a caller that formats as_raw_for_logic_only itself"
    verification: []
    human_judgment: true
    rationale: "A claim about what is NOT covered cannot be discharged by a passing test; it is a disclosure whose accuracy a reader must judge against the tree. Both are stated below with their failure direction and what bounds them, and both are written at the call sites in src/registry.rs rather than only here."

duration: 50 min
completed: 2026-08-27
status: complete
---

# Phase 21 Plan 24: The Error and Lookup Layers, Closed by the Same Carrier Wave 1 Built

**Retyped seven alias-carrying error fields to `crate::text::Untrusted` and changed one function signature, so CR-01 and CR-02 arrived as consequences of the type system rather than as two more patches — and `src/main.rs`'s four `eprintln!` echo sites became correct without appearing in Task 1's diff at all.**

## Performance

- **Duration:** ~50 min
- **Tasks:** 3 of 3
- **Commits:** 4
- **Files modified:** 9 (7 source, 2 test)

## Task Commits

| # | Commit | Task |
|---|---|---|
| 1 | `017d82d` | Task 1 — both error types carry the carrier |
| 2 | `1c9b1c5` | Task 2 — the lookup key's type makes CR-01's bail unwritable; the promote lands |
| 3 | `bece6df` | Task 3 — WR-01's ESC half at the CLI; the one shared control |
| 4 | `65156fa` | style follow-up — keep the opt-in affordance ASCII (see Deviations) |

## The criterion is what is ABSENT

Task 1's whole claim is that the fix is at the producer. The evidence is `rtk proxy git diff --stat` for commit `017d82d`:

```
 src/driver/mod.rs          |  43 ++++-
 src/error.rs               | 417 ++++++++++++++++++++++++++++++++++-----
 src/executor/mod.rs        |  21 ++-
 tests/journal_run_paths.rs |   8 +-
 4 files changed, 427 insertions(+), 62 deletions(-)
```

**`src/main.rs` is not there.** Its four `eprintln!("Error: {}", err)` sites — the CR-02 finding's own consumers — are correct after that commit and were never touched. A list of consumers would have been four this round and five the next; the field type is what makes the count irrelevant.

The compiler, not a reader, named the nine construction sites that did change:

| File | Sites |
|---|---|
| `src/driver/mod.rs` | 6 |
| `src/executor/mod.rs` | 3 |

## CR-02 at the binary, before and after

A hand-built legacy `config.json` (every hostile character produced by `chr()`, never pasted), through `cat -v`:

```text
BEFORE  Error: no project is registered under the alias `no^[[31msuchM-bM-^@M-.x`
AFTER   Error: no project is registered under the alias `no[31msuchU+202Ex`

BEFORE  Error: the project `gsd-M-bM-^@M-.nur` has not opted in to being driven; ...
AFTER   Error: the project `gsd-U+202Enur` has not opted in to being driven; ... Press `o` on the dashboard to opt it in
```

`^[` is a live ANSI introducer; `M-bM-^@M-.` is the UTF-8 for `U+202E` RIGHT-TO-LEFT OVERRIDE. Both reached a terminal from a value the caller typed, on a command where `add` on the same value printed it escaped one arm away. That asymmetry is T-21-24-02 and it is closed.

## CR-01 at the binary, and the compile error that is the mechanism

```text
BEFORE  Error: Project not found: ab^[[31msentM-bM-^@M-.x
AFTER   Error: Project not found: ab[31msentU+202Ex
```

`remove`'s SUCCESS echo had been made to ask whether to escape in round 8. Its FAILURE echo, one line above, was never asked about — and the failure path has **no config prerequisite at all**: any operator typing any argv reaches it. That is T-21-24-01, rated critical, and it is the one this plan's parameter change closes.

The raw `bail!` was written deliberately against the new signature and compiled. This is the error verbatim, an error **this plan saw**, and it now lives in `remove_project`'s own doc replacing the one `src/main.rs:162-173` quoted from memory:

```text
error[E0277]: `LegacyRegistryKey` doesn't implement `std::fmt::Display`
   --> src/registry.rs:650:40
    |
650 |         bail!("Project not found: {}", key);
    |                                   --   ^^^ `LegacyRegistryKey` cannot be formatted with the default formatter
    |                                   |
    |                                   required by this formatting parameter
    |
help: the trait `std::fmt::Display` is not implemented for `LegacyRegistryKey`
   --> src/registry.rs:216:1
    |
216 | pub struct LegacyRegistryKey(String);
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

## WR-01 at the binary, both halves of the class

```text
BEFORE  ev^[[31mil            /tmp   <- `list`
AFTER   ev[31mil             /tmp

BEFORE  Removed project 'ev^[[31mil'
AFTER   Removed project 'ev[31mil'
```

Note what the BEFORE output already got right: `gsd-U+202Enur` was escaped in both runs. `display_identity` answers `General_Category=Cf` union `Default_Ignorable_Code_Point` and answers it correctly. **`ESC` is `Cc` and in neither of those**, so the other half of the class walked straight through — a legacy row on disk emitting a colour sequence the terminal honours. The tree already said the two classes are independent, at `src/ui/screens/driver.rs:874-878`; what was missing was one place that COMPOSED them, and that place is `text::render_for_terminal`.

`rtk proxy grep -c "display_identity(" src/main.rs` is now **0**. Four echoes route through the composition; three of them (`add` x2, `envelope scan`) carry values that already passed `Alias::new`, so the finite alphabet makes both classes impossible and the call is a no-op today. They are routed anyway, and each says so at the site, so display honesty does not depend on the alphabet staying as narrow as it is.

## D-17-3's accept half, measured rather than assumed

`rtk proxy git diff -- tests/registry_test.rs` for the pin:

```diff
         // The claim D-19-2's reversibility rating rests on.
-        remove_project(&mut config, raw).expect(
+        // **Only the CONSTRUCTOR CALL is wrapped for `21-24`; not one assertion
+        // below or above changed.** ...
+        remove_project(&mut config, &LegacyRegistryKey::from_argv(raw.to_string())).expect(
             "an entry this build refuses must still be removable — otherwise \
              the alphabet narrowing is one-way and a user is stuck with a \
              project they can neither use nor delete",
```

**No assertion line changed** in that test, nor in the other two `remove_project` tests — only the call and one import. At the binary, both hostile legacy keys still removed on their RAW bytes with exit 0, leaving only `['clean']` in the config.

## The one control, and its three reds

`registry::tests::a_refusal_never_carries_a_raw_control_or_invisible_character`. **11 subjects x 8 corpus members = 88 subject/value pairs**, asserted in this order: non-vacuity (per class), then arrival, then the property (both classes).

Subjects: the four alias-carrying `OptInError` variants, the three `DriveError` ones, `remove`'s MISS path (CR-01's own), `remove`'s HIT path plus its success echo, and `record_opt_in` / `clear_opt_in`.

**The subject list is compile-forced.** Two classifiers gate it, each a `match` over its whole enum with no wildcard arm — `rtk proxy` executable-line count of `_ =>` in `src/registry.rs` is **0** (the 2 hits are doc lines saying no such arm may be added):

```rust
fn opt_in_untrusted_fields(err: &crate::error::OptInError) -> usize {
    use crate::error::OptInError as E;
    match err {
        E::UnknownAlias { .. } => 1,
        E::NotOptedIn { .. } => 1,
        E::RootUnusable { .. } => 1,
        E::PromptInputsDrifted { .. } => 1,
    }
}
```

`drive_untrusted_fields` is the same shape over all **19** `DriveError` variants: three return 1 and sixteen return 0. Adding a variant to either enum does not compile until an arm is written, which is the moment the author is asked whether it carries an untrusted value.

**The control class is DERIVED, not re-listed.** `is_terminal_control_char` asks `crate::text::strip_terminal_controls` whether stripping changes the character. A test that re-spelled `ESC`/C0/`DEL`/C1 would have been a second spelling of a class this project derives for itself — the exact defect `registry.rs:88-101` recorded about `core::char::is_printable`, committed in a test instead of a library.

### Red 1 — the pre-fix corpus red

The same control, with **only its construction** adapted to the pre-retype field types (every assertion, message and corpus member identical), run against a throwaway copy of this tree at `d712061` created outside the repository, run, and deleted:

```text
58 of 88 subject/value pairs leaked a raw character (42 invisible-class, 16 control-class):
  [invisible] OptInError::UnknownAlias carried ['\u{202e}'] from "demo\u{202e}"
  [invisible] OptInError::NotOptedIn carried ['\u{202e}'] from "demo\u{202e}"
  [invisible] OptInError::RootUnusable carried ['\u{202e}'] from "demo\u{202e}"
  [invisible] OptInError::PromptInputsDrifted carried ['\u{202e}'] from "demo\u{202e}"
  [invisible] remove (miss) carried ['\u{202e}'] from "demo\u{202e}"
  [invisible] record_opt_in (miss) carried ['\u{202e}'] from "demo\u{202e}"
  [invisible] clear_opt_in (miss) carried ['\u{202e}'] from "demo\u{202e}"
  [control]   OptInError::UnknownAlias carried ['\u{1b}'] from "ev\u{1b}[31mil"
  [control]   remove (hit) echo carried ['\u{1b}'] from "ev\u{1b}[31mil"
  [control]   remove (miss) carried ['\u{9b}'] from "a\u{9b}31mb"
```

Both classes, both error types, both halves of `remove`. `git status --porcelain` clean afterwards; the throwaway tree lived only in the session scratchpad.

### Red 2 — one `shown()` reverted, the other direction

Against the FIXED tree, replacing exactly one `escaped(alias)` in `impl Display for OptInError` with `alias.as_raw_for_logic_only()`:

```text
thread 'registry::tests::a_refusal_never_carries_a_raw_control_or_invisible_character' panicked at src/registry.rs:1428:17:
OptInError::UnknownAlias carried ['\u{200b}'] from "demo\u{200b}" into its own message. Those characters render as nothing, so what the operator reads is not what the value is — Trojan Source (CVE-2021-42574) in a refusal. Message: "no project is registered under the alias `demo\u{200b}`"
```

Reverted immediately; `git diff --stat src/error.rs` empty.

### Red 3 — what the non-vacuity guard reports on an all-ASCII corpus

The plan asked the SUMMARY to STATE this. It is measured instead: the corpus was temporarily swapped for `[("demo","demo"), ("run","run")]` with both witnesses de-clawed.

```text
thread 'registry::tests::a_refusal_never_carries_a_raw_control_or_invisible_character' panicked at src/registry.rs:1385:9:
no corpus member carries an invisible-formatting character, so assertion 3's first half is about a corpus that could not fail
```

Restored immediately. That is why the non-vacuity assertion is per-class rather than global: forgetting the `ESC` witness alone would otherwise leave the control-class half proving nothing while the invisible-class half kept passing.

### Red 4 — Task 1's two pins, before the retype

```text
thread 'error::tests::every_alias_carrying_variant_escapes_the_value_it_names' panicked at src/error.rs:1083:17:
OptInError::UnknownAlias's message carries ['\u{200b}'] — characters that render as nothing — from the alias "demo\u{200b}".

thread 'error::tests::the_debug_route_of_every_alias_carrying_variant_uses_this_projects_own_notation' panicked at src/error.rs:1144:17:
OptInError::UnknownAlias's `{:?}` does not carry "U+200B" — this project's own notation for "demo\u{200b}". It reads "UnknownAlias { alias: \"demo\\u{200b}\" }" instead, which means the escape came from `core::char::is_printable`.
```

## The `{:?}` route: what actually changed, said plainly

**The first half of the `{:?}` pin does NOT go red before the retype, and this SUMMARY does not pretend otherwise.** `str`'s own `Debug` already escaped every member of the invisible class it was handed — `src/registry.rs:65-101` recorded that measurement in round 8. What changed is the **provenance**: the guarantee rested on `core::char::is_printable`, a standard-library table no test in this tree pins, no doc in this tree names, that moves with a toolchain upgrade, and that is a second spelling of a class this project derives for itself.

The **discriminating** assertion is the second one, and it is red before the retype (quoted above): the escape must be in this project's own notation, `U+202E` rather than `\u{202e}`. That is what proves the route now goes through `Untrusted`'s hand-written `Debug` rather than through the standard library's table.

`WR-04` is what the difference costs when the two tables are not the same one: `LegacyRegistryKey`'s **derived** `Debug` printed raw bytes, because a derive prints the field rather than `str::Debug` of an escaped copy.

## Re-measured counts (every number re-measured under `rtk proxy` against the FINAL tree; none inherited from the plan)

| Quantity | Plan's figure (at `dfa11c6`) | Measured here (final tree) | Note |
|---|---|---|---|
| `OptInError` references | 22 in 4 files | **55 in 6 files** | `error.rs` 21, `registry.rs` 15 (new: the control), `executor/mod.rs` 10, `driver/mod.rs` 4, `driver_confirm.rs` 4, `detail.rs` 1. The plan's per-file split for `executor/mod.rs` (10) and `driver/mod.rs` (4) reproduces exactly; the growth is this plan's own control and the delegation. |
| The three `DriveError` variants | 33 in 5 files | **48 in 6 files** | `driver/mod.rs` **19** (reproduces exactly), `error.rs` 18, `registry.rs` 7 (new: the control), `cli.rs` 1, `tests/driver_dry_run.rs` 2, `tests/journal_run_paths.rs` 1 — the last four reproduce exactly. |
| `remove_project` mentions | 6 call sites | **28 mentions in 5 files**; call sites: `main.rs` 1, `delete_confirm.rs` 1, `tests/registry_test.rs` 3, `registry.rs` 2 (the control) = **7** | The plan's "6" reproduces as the pre-plan count; this plan's control adds a seventh and eighth (hit and miss). |
| `record_opt_in` mentions | 13 | **32 in 7 files** | The plan's figure is call sites, not mentions; the signature is unchanged either way (prohibition 2). |
| `clear_opt_in` mentions | 5 | **15 in 3 files** | Same. |
| `bail!("Project not found: …")` spellings | 3 | **0** — replaced by 1 producer | Executable-line count of the sentence in `registry.rs` is **1**; the other 4 hits are doc lines quoting what was removed. |
| `String` fields left on `OptInError` | — | **0** | |
| `String` fields left on `DriveError` | — | **5** | `UnsupportedPlatform.detail` (this build's own platform string), `GoalSeamUnusable.detail` (already bounded and control-stripped at its producer), `PlanApprovalRequired.token` (bounded at construction by `driver::goal::legality`), `Journal.detail` and `EnvelopeAssertionFailed.detail` (this build's own I/O diagnostics). None is an argv or registry value. |
| `LegacyRegistryKey` `{:?}` consumers in `src/` + `tests/` | — | **0** | The measurement that justifies REMOVING the derive rather than hand-writing one. |
| executable `_ =>` arms in `src/registry.rs` | — | **0** | |
| `display_identity(` in `src/main.rs` | — | **0** | |

## Gates

All through `rtk proxy` (a bare `cargo` or `grep` was never used for a count- or presence-bearing check — phase standing constraint 1).

| Gate | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo test --workspace --no-fail-fast -- --test-threads=2` | **1380 passed / 0 failed / 13 ignored** |
| `cargo clippy -- -D warnings` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | exit 101 with **exactly** the 4 pre-existing lints, at `src/browser.rs:131,132,133` and `src/project_creator.rs:146` — untouched per prohibition 5 |
| `git diff --stat` vs base | names none of prohibition 1's files, none of 21-25's files, neither `src/browser.rs` nor `src/project_creator.rs` |
| `.planning/REQUIREMENTS.md` | untouched; its `git log --oneline` still ends at `0c4f712` |
| `tests/driver_injection_corpus.rs` | untouched; `git diff --stat d712061..HEAD` for it is empty |

**Test total delta, every unit attributed:** wave-1 baseline **1376** -> **1380**, +4.

| + | Test | Task |
|---|---|---|
| 1 | `error::tests::every_alias_carrying_variant_escapes_the_value_it_names` | 1 |
| 1 | `error::tests::the_debug_route_of_every_alias_carrying_variant_uses_this_projects_own_notation` | 1 |
| 1 | `ui::screens::driver_confirm::tests::the_delegated_opt_in_refusal_still_names_the_key_to_press` | 2 |
| 1 | `registry::tests::a_refusal_never_carries_a_raw_control_or_invisible_character` | 3 |

**Files changed vs `d712061`:**

```
 src/driver/mod.rs                |  43 +++-
 src/error.rs                     | 431 +++++++++++++++++++++++++++++++++------
 src/executor/mod.rs              |  21 +-
 src/main.rs                      |  95 +++++++--
 src/registry.rs                  | 543 ++++++++++++++++++++++++++++++++++++++++++++++++++--
 src/ui/screens/delete_confirm.rs |  11 +-
 src/ui/screens/driver_confirm.rs |  87 +++++++-
 tests/journal_run_paths.rs       |   8 +-
 tests/registry_test.rs           |  18 +-
```

## Prohibition compliance

| # | Prohibition | Status |
|---|---|---|
| 1 | No edits to 21-23's / 21-25's files | **Met** — see the file list above. `src/text.rs` was NOT edited even though a real bug was found in it; it is reported below as a wave conflict instead. |
| 2 | `record_opt_in` / `clear_opt_in` signatures unchanged | **Met** — both still take `&str`; the carrier is constructed at the `bail!` site with the cost disclosed at each |
| 3 | No regression of `remove`'s raw acceptance | **Met** — the pin's diff shows only a wrapped constructor; re-measured at the binary |
| 4 | No work against ROADMAP criterion 4 | **Met** — `tests/driver_injection_corpus.rs` untouched |
| 5 | The 4 pre-existing clippy lints not fixed | **Met** — `--all-targets` still reports exactly those 4, at the same lines |
| 6 | No second escaping composition | **Met** — every escape in this diff is `render_for_terminal` or `Untrusted::shown()`. No bare `display_identity` was added; the four that existed in `main.rs` were replaced. No `sanitize_render_line` was added (it would have capped a CLI echo at 512 chars). |
| 7 | No unbacked bound claim | **Met** — every residual below carries its direction; the `{:?}` pin's non-red half is disclosed as a provenance change rather than claimed as a fix |
| 8 | No raw invisible/bidi/tag/VS/control character in any file | **Met** — a `Cf`/tag/VS/`Cc` scan of the whole 108,498-char plan diff reports **0**, and the scan is non-vacuous: the same function reports **5** on a planted string built from `chr()` code points. Hostile fixtures are drawn BY IMPORT from `test_support::LOOK_ALIKE_PAIRS`; the two added witnesses are spelled `\u{1b}` and `\u{9b}`. |
| 9 | No prior plan / SUMMARY / `deferred-items.md` line rewritten | **Met** — the `LegacyRegistryKey` doc correction is APPENDED, dated, and quotes verbatim the sentence it corrects. No planning artifact was edited at all. |
| 10 | No `REQUIREMENTS.md` requirement flipped | **Met** — the file's `git log` still ends at `0c4f712` |
| 11 | Every number re-measured, none inherited | **Met** — see the re-measured counts table, including the four figures that did not reproduce and why |

## Recurrence check — did this round enumerate one level down?

Phase standing constraint 5 requires this be answered plainly. **No.** The test:

- CR-02 was closed by changing a **field type**. The nine consumers were produced by `cargo build`, and the four echo sites the finding named are absent from the diff — which is the criterion, not a nicety.
- CR-01 was closed by changing a **parameter type**, so the defective line became a compile error. The verbatim error is quoted; it was not asserted.
- Three spellings of one sentence became **one producer** that takes the carrier, so a fourth site added tomorrow cannot spell it wrong.
- WR-01 was closed by **one composition** consumed by every CLI echo, not by adding `display_identity(...)` at more sites.
- The control's subject list is gated by a **wildcard-free match**, so a new variant is a compile error rather than a name someone forgot to add to a list.
- The control's control-class predicate is **derived from the production function**, not re-listed.
- `argv_visible`'s `refuse` closure was retyped rather than each of its five call sites being wrapped.

**Where this round DID add to a list, said plainly:** the control's corpus adds two hand-built witnesses (`\u{1b}`, `\u{9b}`) beside the imported `LOOK_ALIKE_PAIRS`. That is a list, and it is disclosed as one. Its direction is bounded: the per-class non-vacuity assertion goes RED if either witness is removed (measured — see Red 3), so a shrinking corpus makes the control fail rather than making it vacuous. The `subjects_seen == corpus.len() * 11` equality does the same for the subject list.

## Deviations from Plan

### 1. [Rule 1 - Bug, and a WAVE CONFLICT] `impl Display for Rendered` ignores the formatter width

- **Found during:** Task 3 (step a), by measuring `list` at the binary after the change
- **Issue:** `impl std::fmt::Display for Rendered` at `src/text.rs:430-434` is `f.write_str(&self.0)`, which **ignores the formatter's width and fill**. So `{:<20}` applied to a `Rendered` silently emits no padding at all. Measured: `list` printed `clean /tmp` where it had printed `clean                /tmp` — the project table lost its columns.
- **Why this is not merely cosmetic:** any consumer that formats a `Rendered` with a width gets silently wrong output, and there is nothing at the call site to suggest it. `Rendered` is 21-23's deliberately-ergonomic escaped type, so it is exactly the type call sites are being steered toward.
- **The correct fix is `f.pad(&self.0)`** in `src/text.rs`. **This plan did NOT make it.** `src/text.rs` belongs to plan 21-23 (wave 1) and prohibition 1 forbids editing it; the wave structure exists because two plans editing one file in one wave is how a merge silently drops a fix.
- **What was done instead:** `.to_string()` at the one affected call site, with the reason, the measurement and the correct fix written in a comment there.
- **Reported as a wave-conflict finding for the orchestrator.** It is a one-line change in a file this plan may not touch.
- **Committed in:** `bece6df`

### 2. [Rule 1 - Bug] The pre-existing `o`-affordance assertion was vacuous

- **Found during:** Task 2 (step e)
- **Issue:** `starting_a_run_on_a_project_that_has_not_opted_in_sets_an_error_and_spawns_nothing` asserted `refusal.contains('o')`. The fixture's `ALIAS` is `"proj"`, which contains an `o` — so the assertion was satisfied by the **alias echo alone** and would have kept passing if the key affordance had been deleted outright.
- **Fix:** strengthened (never weakened) to a phrase. After deviation 3 it reads `contains("`o` on the dashboard")`.
- **Committed in:** `1c9b1c5`, refined in `65156fa`

### 3. [Rule 1 - Bug] The affordance's em dash rendered as hostile-looking bytes under `cat -v`

- **Found during:** final binary re-measurement
- **Issue:** the `o`-key affordance added to `OptInError::NotOptedIn` was joined with an em dash (`U+2014`). It is a **visible** character in neither judged class, so it is not a security defect — but under `cat -v` it renders as `M-bM-^@M-^T`, which is the exact notation this plan's own evidence uses for a raw hostile byte. A reader auditing that output should not have to tell the two apart.
- **Fix:** replaced with a full stop. Both `o`-affordance pins went RED on the change (they were asserting `press \`o\`` and the sentence position changed the case), which is the pins working; they now assert the case-independent phrase.
- **Committed in:** `65156fa`

### 4. [Deviation — an acceptance criterion is self-defeating as literally spelled]

- **Found during:** Task 2 verification
- **Issue:** the criterion `rtk proxy grep -c "Project not found" src/registry.rs` **returns exactly 1** was spelled as a raw text grep. It returns **5**. The four extra hits are all doc-comment lines: the producer's own doc quoting the `bail!` it replaces, and `remove_project`'s doc quoting the two verbatim reds that other criteria explicitly require be recorded. A naive text grep cannot tell a comment from an implementation, so the criterion as spelled forbids the evidence its siblings demand. **This is the same shape as 21-23's deviation 5, and it recurred because the criterion was written the same way.**
- **Resolution:** measured the way this tree's own census idiom measures — executable lines only, skipping lines whose trimmed form opens a line comment. **1 executable-line match, 4 comment-line matches.** Both numbers are reported above rather than the flattering one alone.
- **Committed in:** `1c9b1c5`

### 5. [Deviation — the plan's line numbers were stale]

- **Found during:** Task 2 (step a)
- **Issue:** the plan cites the three `bail!` sites at `src/registry.rs:377`, `:586` and `:620`, and `LegacyRegistryKey` at `:161-209` / `:185`. Wave 1's merge moved all of them; the actual sites were `:407`, `:616`, `:650` and `:161-239` / `:215`.
- **Resolution:** located by content rather than by line, and this SUMMARY quotes the found lines. Recorded because a future reader comparing the plan to the diff would otherwise think a site was missed. No scope change.

### 6. [Deviation — a smaller lever than the plan implies, taken deliberately] `argv_visible`

- **Found during:** Task 1 (step c)
- **Issue:** the plan directs the executor to wrap "at construction" at each compiler-named site. Two of `driver/mod.rs`'s six sites are closures passed to one helper, `argv_visible`, which is the single place a refused argv value on that boundary reaches a refusal.
- **Resolution:** `argv_visible`'s `refuse` parameter was retyped to `FnOnce(crate::text::Untrusted)` and the wrap happens inside the helper. Five closures inherit the carrier instead of five sites being a list that is six next round — which is the plan's own argument applied one level up. Recorded because it is a change the plan did not name.
- **Committed in:** `017d82d`

---

**Total deviations:** 6 — 3 auto-fixed bugs (Rule 1), 1 self-defeating acceptance criterion, 1 stale-line-number correction, 1 deliberate lever change. **One of the three bugs is a WAVE CONFLICT that this plan could not fix and the orchestrator must resolve.**

**Impact on scope:** none. Deviation 1 is a real defect left open by prohibition, disclosed with its exact fix.

## Residuals, each with its failure direction

| # | Residual | Direction | Bounded by a committed control? |
|---|---|---|---|
| 1 | `record_opt_in` / `clear_opt_in` are protected at the MESSAGE, not the PARAMETER — a future caller that formats the alias itself is not caught | under-protection, silent | **Partially** — the control drives both functions' messages, so the sentence is bounded; the parameter is not. What bounds the rest: every production path except the legacy rows has already passed `Alias::new`, and the legacy rows are what the escape is for. Written at both call sites in `src/registry.rs`, not only here. |
| 2 | The control does not see a caller that reads `err.alias.as_raw_for_logic_only()` and formats the bytes itself | under-detection, silent | **No** — bounded by the type instead: `as_raw_for_logic_only` is the only route to the bytes and it appears in a diff, which is the property the carrier was built for |
| 3 | `DriveError::PlanApprovalRequired.steps` reaches stdout through `sanitize_render_line`, which applies the CONTROL class only — **not** the invisible-formatting class | under-protection, silent | **No** — and this plan deliberately did not touch it (`src/error.rs:827` is explicitly out of scope per the plan's Task 1 step d). What bounds it: each step is built from a `target_phase` that `driver::goal::legality` already refused unless it is a plain path component, so the invisible class cannot reach it today. **That is a fact about today, not a property of the type** — the same distinction `--run-id '../../../../escaped'` cost the last time it was left to callers. Flagged for 21-26 or a later round. |
| 4 | Five `String` fields remain on `DriveError` | under-protection, silent | **No** — each is named above with the reason it is this build's own value rather than a third-party one; none is an argv or registry value |
| 5 | The control covers messages these two enums and `remove` PRODUCE, not every surface that renders one | under-detection, silent | **Partially** — `driver_confirm`'s delegation pin asserts the screen renders the producer's message byte-for-byte, so at least that surface cannot drift |
| 6 | Homoglyphs remain outside the judged class | under-detection, by design | **No** — closed at identity seams by the finite alphabet only; unchanged by this plan |

## Threat Flags

None. No new network endpoint, auth path, file access pattern or schema change was introduced. `Cargo.toml` is unchanged, no dependency was added, and the `## Package Legitimacy Audit` gate did not fire.

## Known Stubs

None. No hardcoded empty value, placeholder string, TODO or unwired component was introduced.

## Issues Encountered

**1. Two documented flakes did not fire.** `driver_reattach` and `envelope_tracer` passed on every full-suite run in this session. Not re-diagnosed either way.

**2. The `list` column regression was found by MEASURING, not by testing.** No test in this tree asserts `list`'s column alignment, so deviation 1 would have shipped silently had the plan not required a binary-level `cat -v` measurement of that exact command. The `Rendered` padding bug is therefore evidence for the plan's own insistence on binary-level before/after: a unit test on `render_for_terminal` would have been green throughout.

## Next Phase Readiness

`21-25` and `21-26` inherit `registry::project_not_found` (the one-producer pattern applied to a `bail!` sentence), the wildcard-free-classifier idiom for gating a control's subject list, and the derived-from-production control-class predicate.

**One item requires orchestrator action before the wave merges:** deviation 1's `impl Display for Rendered` padding bug in `src/text.rs`. One line, `f.write_str(&self.0)` -> `f.pad(&self.0)`, plus removal of the `.to_string()` workaround at `src/main.rs`'s `list` loop.

**Ready for the wave merge.**

## Self-Check: PASSED

- All nine modified files present on disk and modified; no file created, none deleted (`git diff --diff-filter=D` empty for every commit).
- Commits `017d82d`, `1c9b1c5`, `bece6df`, `65156fa` all present in `git log --oneline`.
- Every task-level acceptance criterion re-run against the final tree; results recorded above, including the one criterion (deviation 4) that could not be satisfied as literally spelled and the executable-line measurement substituted for it.
- Plan-level `<verification>` re-run: `cargo build` exit 0; `cargo test --workspace --no-fail-fast -- --test-threads=2` 1380/0/13 with every delta attributed; `cargo clippy -- -D warnings` exit 0; four verbatim reds recorded (the compile error for the raw `bail!`, the pre-fix corpus red, the `shown()`-revert red, the pre-fix binary `^[[31m`), each followed by a clean tree; binary-level before/after for CR-01, CR-02 and WR-01 through `cat -v`; `REQUIREMENTS.md` and `tests/driver_injection_corpus.rs` untouched.
- `STATE.md` and `ROADMAP.md` NOT modified — the orchestrator owns those writes after the wave.
