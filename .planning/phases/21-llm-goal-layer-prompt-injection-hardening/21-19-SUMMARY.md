---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 19
subsystem: security
tags: [unicode, icu4x, icu_properties, unicode-properties, trojan-source, bidi, prompt-injection, allow-list, rust]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    provides: "text::carries_visible_content / carries_invisible_formatting (the two judgments), test_support::DEGENERATE and LOOK_ALIKE_PAIRS, journal::is_plain_path_component, registry::Alias newtype, driver::DriveArgs::from_argv purity"
provides:
  - "text::is_invisible_formatting_char DERIVED from icu_properties (General_Category=Cf union Default_Ignorable_Code_Point) — no literal range survives in the function"
  - "text::is_identity_char — the one spelling of the finite identity alphabet [A-Za-z0-9._-]"
  - "text::display_identity — render-side escaping of the invisible class to visible U+XXXX form"
  - "An exhaustive all-codepoints property test whose oracle is an INDEPENDENT derivation (unicode-properties), carrying a committed format_seen >= 150 non-vacuity floor"
  - "The identity-alphabet clause at journal::is_plain_path_component, so every run directory, envelope root, credential scope, phase token and re-read run id accepts only the finite alphabet"
  - "registry::AliasRefusal::OutsideIdentityAlphabet with the recorded product trade in its message; derive_alias sanitization"
  - "Seam pins for --target-phase and journal::writer::read_active_run, which had none of their own"
affects: [phase-21 verification, TR39 confusables roadmap item, any future identity seam, release process (cargo update refreshes the Unicode class)]

actuals:
  tokens: 181641
  tasks: 3
  commits: 4

tech-stack:
  added: [icu_properties 2.3.0 (production), unicode-properties 0.1.4 with features = ["general-category"] (dev)]
  patterns:
    - "Derive a character class from Unicode's reference data; never enumerate it"
    - "Oracle independence: the implementation and its falsifying corpus read DIFFERENT derivations of the same standard"
    - "Every exhaustive/filtered property test carries a non-vacuity floor asserting its corpus arrived"
    - "Identities get an allow-list (finite, cannot be one item short); free text keeps the derived deny-list"
    - "Escape untrusted identity bytes at every render site, not only at the ingest gate"

key-files:
  created: []
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/text.rs
    - src/test_support.rs
    - src/journal/mod.rs
    - src/journal/writer.rs
    - src/registry.rs
    - src/driver/mod.rs
    - src/driver/goal.rs
    - src/main.rs
    - src/ui/project_list.rs
    - tests/registry_test.rs
    - tests/driver_goal_seam.rs
    - tests/driver_dry_run.rs

key-decisions:
  - "D-19-1: the invisible class is derived from icu_properties (Cf union Default_Ignorable), replacing three literal ranges — costly, released-surface widening, staleness residual disclosed"
  - "D-19-2: identity seams move to the allow-list [A-Za-z0-9._-] via one shared text::is_identity_char — costly, user-visible narrowing, recorded product trade, certified NOT one-way by a test"
  - "D-19-3: oracle independence — production derives from ICU4X, the sweep's oracle from unicode-properties, neither reads the other"
  - "D-19-4: DEGENERATE grows to 10 and LOOK_ALIKE_PAIRS to 6, each gaining outside-the-old-ranges members; both docs corrected to stop claiming the anti-tautology property"
  - "D-19-5: alias render sites escape the invisible class via text::display_identity — legacy-surface defence, display only"
  - "D-19-6: free text keeps the deny-list direction; no allow-list there, no embedded-invisible refusal (ZWJ/ZWNJ stay legal)"

patterns-established:
  - "Derived-not-enumerated character classes: a deny-list over a growing standard can always be one item short; derive it or make the accepted set finite"
  - "Non-vacuity floors on filtered sweeps: count what the filter yielded and assert a floor, or an implication over an empty set passes green forever"
  - "Reversibility ratings are certified by a test, never by prose in a decision table"

requirements-completed: [DRIVE-01, DRIVE-03, SAFE-08]

coverage:
  - id: D1
    description: "The invisible class is derived from Unicode's own data — General_Category=Cf union Default_Ignorable_Code_Point via icu_properties — with no literal character range surviving in the function, and BOTH judgments consume it"
    requirement: "DRIVE-03"
    verification:
      - kind: unit
        ref: "src/text.rs#a_character_the_standard_calls_invisible_is_refused_even_outside_the_old_ranges"
        status: pass
      - kind: unit
        ref: "src/text.rs#named_default_ignorable_members_beyond_cf_are_inside_the_class"
        status: pass
      - kind: other
        ref: "rtk proxy grep -n '200b|200f|2060|2064|feff' src/text.rs — zero hits in lines 144-153 (the function body)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The falsifying corpus no longer shares a derivation source with the implementation: an exhaustive all-codepoints sweep takes its oracle from unicode-properties, and it proves its own corpus arrived via a committed format_seen >= 150 floor"
    requirement: "DRIVE-03"
    verification:
      - kind: unit
        ref: "src/text.rs#every_format_character_the_standard_names_is_inside_the_class"
        status: pass
      - kind: other
        ref: "falsifiability demonstration 4(a): narrowing the production class fails the sweep at U+00AD; 4(b): scoping iteration to ASCII fails the floor at format_seen=0"
        status: pass
    human_judgment: false
  - id: D3
    description: "Identity is judged by a finite allow-list at every identity seam — run directories, envelope roots, credential scopes, phase tokens, re-read run ids and registry aliases"
    requirement: "SAFE-08"
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#an_identity_outside_the_alphabet_names_nothing_at_any_seam"
        status: pass
      - kind: unit
        ref: "src/registry.rs#an_alias_outside_the_identity_alphabet_is_refused_with_the_product_trade"
        status: pass
      - kind: unit
        ref: "src/driver/goal.rs#a_look_alike_phase_token_is_refused_even_though_its_visible_twin_is_declared"
        status: pass
      - kind: e2e
        ref: "binary-level nine-add reproduction — ONE registry key where pass 7 measured nine"
        status: pass
    human_judgment: false
  - id: D4
    description: "The two seams pass 7 named as pin-free carry their own look-alike and outside-the-class pins"
    requirement: "SAFE-08"
    verification:
      - kind: unit
        ref: "src/driver/mod.rs#a_target_phase_that_renders_as_another_is_refused_at_the_seam"
        status: pass
      - kind: unit
        ref: "src/journal/writer.rs#an_active_pointer_naming_a_look_alike_run_is_refused_not_followed"
        status: pass
    human_judgment: false
  - id: D5
    description: "The tool's own identity displays can no longer be reordered by what they render — list, the refusal echo and the TUI row escape the invisible class"
    requirement: "SAFE-08"
    verification:
      - kind: unit
        ref: "src/text.rs#an_invisible_character_renders_as_its_visible_code_point"
        status: pass
      - kind: e2e
        ref: "legacy-key list check piped through cat -v — renders gsd-U+202Enur, no raw U+202E byte"
        status: pass
    human_judgment: false
  - id: D6
    description: "Acceptance did not narrow where the tree depends on it: every pinned accepted value still passes, free text keeps its joiners, and over-detection is bounded past ASCII across nine script families"
    requirement: "DRIVE-01"
    verification:
      - kind: unit
        ref: "src/text.rs#free_text_keeps_its_joiners"
        status: pass
      - kind: unit
        ref: "src/text.rs#visible_characters_are_outside_the_class"
        status: pass
      - kind: unit
        ref: "cargo test --workspace --no-fail-fast -- --test-threads=2 → 1358 passed, 0 failed"
        status: pass
    human_judgment: false
  - id: D7
    description: "The user-visible product trade of the identity alphabet (no non-ASCII aliases; legacy entries fail closed with a remove+re-add recovery route) is an acceptable cost for identities that cannot be one code point short"
    requirement: "DRIVE-01"
    verification:
      - kind: unit
        ref: "tests/registry_test.rs#a_legacy_alias_the_alphabet_refuses_is_still_removable"
        status: pass
    human_judgment: true
    rationale: "The REVERSIBILITY of the trade is certified by a test — a refused legacy entry is still removable, so the narrowing is not one-way. Whether the trade itself is one this project WANTS is a product judgment no test can make, and DRIVE-01's edge probe surfaced it unresolved specifically so a human decides it rather than the executor."

duration: 41 min
completed: 2026-08-25
status: complete
---

# Phase 21 Plan 19: The Derived Class and the Finite Alphabet Summary

**The invisible-character class stopped being a hand-written list and became a query against Unicode's own data, its falsifying corpus moved onto an independently maintained second derivation, and identities inverted from a deny-list to a finite `[A-Za-z0-9._-]` alphabet that cannot be one code point short.**

## Performance

- **Duration:** 41 min
- **Started:** 2026-08-25T17:43:52Z
- **Completed:** 2026-08-25T18:25:00Z
- **Tasks:** 3
- **Files modified:** 14 (13 excluding `Cargo.lock`)
- **Commits:** 4 (one red arm + three task commits)

## Accomplishments

- **The class is derived, not enumerated.** `text::is_invisible_formatting_char` answers `General_Category=Cf` OR `Default_Ignorable_Code_Point` from `icu_properties` compiled data. No literal character range survives in the function body (lines 144-153). Both judgments consume it — including `carries_visible_content`, the emptiness half that was criterion 1's own question and the half no round had named.
- **The sampling moved, which is the part that is not another list.** `every_format_character_the_standard_names_is_inside_the_class` sweeps every code point Rust admits and takes its oracle from `unicode-properties` (unicode-rs). Production reads `icu_properties` (ICU4X) and nothing else. For the first time in seven passes, a subset in the implementation goes red against something that did not come from the implementation.
- **The sweep proves its own corpus arrived.** A `format_seen` counter and a committed `>= 150` floor. This is the plan's sharpest vacuity risk — with the dev-dep's feature off or the iteration scope wrong, the loop yields nothing and the implication passes green forever while asserting nothing, which is the exact failure shape six rounds of fixtures had.
- **Identities inverted to a finite alphabet.** `text::is_identity_char` is one spelling consumed by `journal::is_plain_path_component` and `registry::Alias::new`, so every run directory, envelope root, credential scope, phase token, re-read run id and registry alias accepts only `[A-Za-z0-9._-]`. Bidi (CVE-2021-42574), the tag block, variation selectors and homoglyphs all close in one clause.
- **The two pin-free seams got pins**, and the three render sites stopped being reorderable by what they render.

## Task Commits

1. **Task 1 RED arm: the class outside the list** — `8ca0dd1` (test)
2. **Task 1 fix: derive the class, move the sampling** — `f434d71` (feat)
3. **Task 2: the identity alphabet at every seam** — `a3da3d1` (feat)
4. **Task 3: the two pin-free seams and the spoofable displays** — `e7e5572` (feat)

## The tracer's red arm, verbatim

Committed in `8ca0dd1`, BEFORE the fix, marked `#[ignore]` so the tree stayed green while the evidence landed in history. The `#[ignore]` came off in `f434d71`.

```text
running 1 test
test text::tests::a_character_the_standard_calls_invisible_is_refused_even_outside_the_old_ranges ... FAILED

---- text::tests::a_character_the_standard_calls_invisible_is_refused_even_outside_the_old_ranges stdout ----

thread 'text::tests::a_character_the_standard_calls_invisible_is_refused_even_outside_the_old_ranges' (386400) panicked at src/text.rs:227:13:
"\u{202e}" renders as nothing, so it carries no visible instruction — pass 7 measured this value ACCEPTED because the class was three hand-written ranges instead of the standard's own answer

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1037 filtered out; finished in 0.00s
```

It fails on the FIRST assertion because the class was three literal ranges; every later assertion (the identity half, and `is_plain_path_component` at the seam) fails for the same cause.

## Falsifiability demonstrations (verification step 4)

Performed once in this worktree, each reverted, **neither committed**. Both directions of the independent sweep are red-capable.

**4(a) — narrowing the production class fails a MEMBERSHIP assertion.** The demonstration six rounds of fixtures could not produce. Production's derived body was temporarily replaced with the three old literal ranges:

```text
thread 'text::tests::every_format_character_the_standard_names_is_inside_the_class' (468233) panicked at src/text.rs:271:13:
U+00AD is General_Category=Format per `unicode-properties` and must be inside the production class; the production class is a SUBSET of the standard's, which is the defect that survived six rounds
```

**4(b) — scoping the sweep's own iteration to ASCII fails the FLOOR**, rather than passing green. This is the guard that SHIPS:

```text
thread 'text::tests::every_format_character_the_standard_names_is_inside_the_class' (466965) panicked at src/text.rs:279:9:
the oracle produced only 0 Format characters, so this sweep proved nothing — an implication over an empty set is vacuously true. Unicode 15.0 assigns 170. Look at the `unicode-properties` dev-dependency's `general-category` feature and at this loop's iteration scope.
```

The plan predicted `format_seen = 1` here on the assumption U+007F is `Cf`; it is `Cc`, so the measured count is **0**. Recorded as measured, not as predicted.

**Which is committed and which is a one-time demonstration:** 4(b)'s floor SHIPS in the test body, so a later oracle regression is caught by `cargo test`. 4(a) remains a demonstration only, because the tree cannot ship a deliberately broken production class.

## Binary-level reproductions, re-run against the built tree

**(1) Pass 7's nine-add reproduction → ONE key.** Nine `add` invocations against a scratch config (hostile aliases generated from `chr(0x...)` in a pure-ASCII Python driver, never raw in a file):

```text
demo             exit=0  Added project 'demo' at .../proj
demo+U+202E      exit=1  Error: the alias "demo\u{202e}" carries a character that renders as nothing...
demo+U+202D      exit=1  Error: the alias "demo\u{202d}" carries a character that renders as nothing...
demo+U+2066      exit=1  Error: the alias "demo\u{2066}" carries a character that renders as nothing...
demo+U+061C      exit=1  Error: the alias "demo\u{61c}" carries a character that renders as nothing...
demo+U+FE0F      exit=1  Error: the alias "demo\u{fe0f}" carries a character that renders as nothing...
demo+U+E0001     exit=1  Error: the alias "demo\u{e0001}" carries a character that renders as nothing...
demo+U+00AD      exit=1  Error: the alias "demo\u{ad}" carries a character that renders as nothing...
demo+U+13430     exit=1  Error: the alias "demo\u{13430}" carries a character that renders as nothing...
---- registry keys ----
count: 1
  key: 'demo'
```

**Truthful nuance, recorded rather than glossed:** all eight refusals read the `InvisibleFormatting` message, not `OutsideIdentityAlphabet`. Every one of those eight values carries an invisible byte, so the earlier and more specific clause fires first — which is the ordering `Alias::new`'s doc already states ("the most specific true statement is the one the user reads"). The alphabet message is what a value with NO invisible byte reads:

```text
Error: the alias "демо" uses characters outside A-Z a-z 0-9 . _ - An alias is how this tool
names a project to you and to itself, so the set it accepts is deliberately small and finite:
outside it, two aliases can render identically while naming different projects — through a bidi
override, a tag character, a variation selector or a look-alike letter from another script — and
nothing in the interface would show you which one you were acting on. The project folder itself
may be named anything, in any script; only the alias is restricted. Register it under an ASCII
alias of your choosing. If this alias is an existing entry an older build accepted,
`remove "демо"` still accepts it — remove it and re-add under an alias from this set
```

**(2) The legacy Trojan Source `list` check.** A legacy-shaped `config.json` holding the key `"gsd-\u{202e}nur"` (the exact value pass 7 measured `list` printing as `gsd-run`), rendered and piped through `cat -v`:

```text
ALIAS                PATH                                               ADDED
------------------------------------------------------------------------------------------
gsd-U+202Enur        /.../legacylist/proj                               2026-01-01T00:00:00+00:00

raw U+202E bytes present in list stdout: False
contains 'U+202E': True
```

**(3) `is_plain_path_component("2\u{202e}0")` → false**, and `run_paths(planning, "\u{202e}")` → `None`, both pinned by `an_identity_outside_the_alphabet_names_nothing_at_any_seam`. **(4)** `from_argv` refuses all ten DEGENERATE payloads in all seven argv positions via the grown matrix (`every_argv_position_refuses_every_degenerate_payload_at_the_parse_boundary` → 0 failures).

## Prohibition audit

Every row a mechanism check with its command and output. No prose greps.

| # | Prohibition | Command | Output / result |
|---|---|---|---|
| 1 | Corpus and implementation MUST NOT share a derivation source | `rtk proxy grep -n "unicode_properties" src/text.rs` | `378: use unicode_properties::{...}` — the ONLY hit, and `mod tests` starts at line 272, so it is test-only. Production (lines 1-270) reads `icu_properties` (8 hits, all production or its docs). `src/driver/mod.rs:2299` is a doc mention of `icu_properties`, not a dependency; that file's oracle imports `unicode_properties` at 2313. No test imports the production class as its own expected set. |
| 2 | MUST NOT paste a raw invisible/bidi/tag/variation-selector character into any source or test file | Programmatic scan of all 13 changed files, class computed from `unicodedata` (never spelled) | `files scanned: 13` / `raw invisible characters found: 0` |
| 3 | MUST NOT certify a structural claim with a grep for prose | Every claim in this SUMMARY carries a named test or a measured command output | The tracer's committed red arm (`8ca0dd1`), the independent sweep with its floor, and `a_legacy_alias_the_alphabet_refuses_is_still_removable` are the three claims that were prose in the plan and are tests here. |
| 4 | MUST NOT flip any REQUIREMENTS.md requirement | `git log --name-only --format="=== %h" 1be0225..HEAD -- .planning/` | *(empty)* — this plan's four commits touch NO file under `.planning/` at all. |
| 5 | MUST NOT grow into TR39 confusables/homoglyph defence | `rtk proxy grep -n "TR39" src/text.rs` / `grep -rn "confusable" src/` | `224: /// territory (Unicode TR39) and a partial implementation of it under this name` — the carve-out is intact and its doc updated to say where the homoglyph harm IS closed (the alphabet, at identity seams) and where it is not (free text). Only one `confusable` hit, in that same carve-out doc. No confusables data or implementation added. |
| 6 | MUST NOT drop an adjudicated-out or disclosed item silently | See the Disclosure list below | All five disclosed items recorded at the site, in the plan, and here. |

## Named-shape audit

Every shape pass 7 NAMED, with its final state.

| # | Named shape | Owner | Final state |
|---|---|---|---|
| 1 | The emptiness half: solely-invisible free text outside the old ranges accepted by `carries_visible_content` | 21-19 T1 | **CLOSED.** Class derived; `carries_visible_content` consumes it. All eight pass-7 code points refused, pinned by the tracer (committed red first). Grown DEGENERATE runs through the 7-position matrix. |
| 2 | The identity half: look-alikes outside the old ranges at every seam | 21-19 T2 | **CLOSED.** Alphabet clause at `is_plain_path_component` + `Alias::new`; grown LOOK_ALIKE_PAIRS. |
| 3 | Fixtures sampled from inside the implementation; `LOOK_ALIKE_PAIRS` doc claiming a false property | 21-19 T1 | **CLOSED.** Independent-oracle exhaustive sweep is the falsifier; both consts' docs corrected to stop claiming the anti-tautology property and to point at the sweep by test name. The same false claim in `src/driver/mod.rs`'s detector doc was corrected too. |
| 4 | `run_paths` yielding a directory named by one invisible character (third recurrence) | 21-19 T1/T2 | **CLOSED.** Pinned in both directions by `an_identity_outside_the_alphabet_names_nothing_at_any_seam` (includes `run_paths` and `envelope_dir_in`) and by the LOOK_ALIKE sweep. |
| 5 | Phase token safe only by roadmap membership (`"2\u{202e}0"` → true) | 21-19 T2 | **CLOSED.** The refusal is a property of the value, pinned against a fixture roadmap that DOES declare `"20"` (21-17 truth 6 now true). |
| 6 | `--target-phase` seam carries no look-alike pin of its own | 21-19 T3 | **CLOSED.** `a_target_phase_that_renders_as_another_is_refused_at_the_seam`, with the visible half asserted as a predicate claim + a not-`TargetPhaseInvalid` claim rather than the false "drive succeeds". |
| 7 | Re-read run id (`read_active_run`) carries no pin of its own | 21-19 T3 | **CLOSED.** `an_active_pointer_naming_a_look_alike_run_is_refused_not_followed`, with the visible twin's directory present so `None` cannot be the staleness rule. |
| 8 | `list`/TUI/refusal-echo Trojan Source spoofing | 21-19 T3 | **CLOSED.** `display_identity` at all three render sites; binary-level `cat -v` check quoted above. |
| 9 | Nine registry keys rendering as `demo` | 21-19 T2 | **CLOSED.** Binary re-run yields ONE key; output quoted above. |
| 10 | ZWJ/ZWNJ must stay legal in free text; all pinned acceptances must hold | 21-19 T1 | **HELD.** `free_text_keeps_its_joiners`; every pre-existing acceptance fixture passes unchanged; whole suite 0 failures with nothing reconciled. |
| 11 | Unicode-version staleness of any derived table | 21-19 T1 | **MITIGATED + DISCLOSED.** Sweep-vs-crate skew control; residual disclosed below (T-21-19-05). |
| 12 | Non-`Cf` default-ignorable half has no second machine oracle | 21-19 T1 | **NAMED-MEMBER PINS + DISCLOSED.** `named_default_ignorable_members_beyond_cf_are_inside_the_class`; under-detection direction disclosed below. |
| 13 | TR39 confusables | none — OUT | **OUT OF SCOPE**, prohibition 5. Carve-out doc stays and now states where the homoglyph harm is closed by the alphabet. Recommended as a separate roadmap item. |
| 14 | Guard nine bare-field blindness, allowlist contains-pin, DEGENERATE witness, SAFE-07 arithmetic | 21-20 | **DELEGATED** — see 21-20's audit. Not this plan's work. |

## Disclosure list

Five items adjudicated out or accepted-with-disclosure. Each is recorded at the code site, in the plan, and here.

1. **D-19-1 staleness residual (T-21-19-05).** The class is exactly as current as `icu_properties`' pinned Unicode version. A code point *unassigned* at that version, which a later Unicode release makes `Cf`, is accepted **in free text** until the next `cargo update` (CLAUDE.md release step 2). Identities are version-independent — they are judged by the finite alphabet. Recorded on `is_invisible_formatting_char` and in `Cargo.toml`.
2. **D-19-2 product trade.** An alias, run id, or phase token cannot carry a non-ASCII script. A non-Latin alias an older build accepted stops working; legacy entries fail closed at the envelope seams. The project FOLDER may still be named anything in any script. Recovery is `remove <alias>` + re-add. Recorded on `is_identity_char`, in the refusal message, and certified NOT one-way by `a_legacy_alias_the_alphabet_refuses_is_still_removable`.
3. **T-21-19-02 free-text tag residual (the accept half).** Free text carrying tag characters BESIDE visible content still reaches the model, inside SAFE-07's labelled untrusted boundary. Refusing embedded invisibles in free text would break ZWJ/ZWNJ scripts, and the boundary mechanism is this phase's chosen defence for untrusted content. Identities are closed; solely-invisible free text is refused.
4. **T-21-19-04 legacy fail-closed widening.** Entries an older build registered now refuse at `Alias::new` and the envelope re-entries, and legacy run directories outside the alphabet are unreadable at the journal re-reads. The recovery route is shipped, named in the messages, and now pinned by a test so a later cleanup cannot wrap the remove path in `Alias::new` and make legacy entries permanent.
5. **Premise 7a — the non-`Cf` default-ignorable half has no second machine oracle.** `unicode-properties` does not expose `Default_Ignorable_Code_Point`, so that half is named members with a disclosed **under-detection** direction: a subset bug in a default-ignorable code point not on the list is invisible to every test in this tree. The `Cf` sweep is the load-bearing control. Related: a defect in `unicode-properties` ITSELF is invisible to every test here — only a defect in the implementation's derivation is caught.

## Files Created/Modified

- `Cargo.toml` / `Cargo.lock` — `icu_properties` (production), `unicode-properties` with `features = ["general-category"]` (dev), each with the register comment justifying derived-not-enumerated, two-crate independence, the named feature, and the staleness obligation. Exactly two dependency lines added; nothing removed or changed.
- `src/text.rs` — the derived class; `is_identity_char`; `display_identity`; the independent sweep with its floor; the DI-member pins; the past-ASCII over-detection bound; the joiner acceptance pin; the tracer.
- `src/test_support.rs` — `DEGENERATE` 6→10 and `LOOK_ALIKE_PAIRS` 3→6 with outside-the-old-ranges members; both docs corrected.
- `src/journal/mod.rs` — the alphabet clause on `is_plain_path_component`, its fifth-shape doc correction and honest clause-layering note, and the seam pin.
- `src/journal/writer.rs` — the active-pointer re-read pin.
- `src/registry.rs` — `AliasRefusal::OutsideIdentityAlphabet` + its message; the `Alias::new` clause; `derive_alias` sanitization; the alphabet and `derive_alias` pins.
- `src/driver/mod.rs` — the test-side oracle re-derived from `unicode-properties`; its doc corrections; the detector's outside-the-ranges and past-ASCII pins; the `--target-phase` seam pin.
- `src/driver/goal.rs` — the look-alike phase-token pin extended with three outside-the-old-ranges tokens.
- `src/main.rs`, `src/ui/project_list.rs` — `display_identity` at the `list` row, the refusal echo, and the TUI cell.
- `tests/registry_test.rs` — `a_legacy_alias_the_alphabet_refuses_is_still_removable`; the end-to-end pin extended with a refusing direction.
- `tests/driver_goal_seam.rs` — `LOOK_ALIKE_PHASE_TOKENS` grown by three.
- `tests/driver_dry_run.rs` — stale arity comment corrected.

## Decisions Made

All six decisions are the plan's D-19-1 through D-19-6, implemented as specified. Two implementation choices were made inside them:

- **`const` borrowed accessors rather than `OnceLock`.** `icu_properties` 2.x resolves both constructors as `const fn` over compiled data, so the class lookup is two borrowed static references — allocation-free with no lazy-init cell. Clippy clean.
- **`derive_alias` falls back to `"project"` rather than to a separator run.** The plan's contract allowed either branch; the sanitizer's ordering produces the usable-alias branch (i). An all-non-ASCII folder sanitizes to nothing, is caught by the existing emptiness filter plus a new "carries at least one ASCII alphanumeric" filter, and becomes `"project"` — so `unique_alias` grows `project-2` like any other duplicate instead of queueing every such folder under `"-2"`, `"-3"`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug in the plan text] The goal-seam tokens went to `LOOK_ALIKE_PHASE_TOKENS`, not `HOSTILE_PHASE_TOKENS`**

- **Found during:** Task 2(d)
- **Issue:** The plan says to add `"2\u{202e}0"`, `"2\u{e0041}0"` and `"2\u{ad}0"` to `HOSTILE_PHASE_TOKENS` in `tests/driver_goal_seam.rs`. That array's consuming test is named `a_phase_token_carrying_a_control_character_is_refused_by_name_rather_than_stored` and asserts `!refusal.offending().chars().any(char::is_control)`. None of the three tokens carries a control character, so following the plan literally would have made that test's name assert a falsehood about three of its fixtures. The tree already contains a separate const, `LOOK_ALIKE_PHASE_TOKENS`, whose doc records that this exact folding was rejected as a deviation in 21-17-SUMMARY for this exact reason.
- **Fix:** Added the three tokens to `LOOK_ALIKE_PHASE_TOKENS` instead — same intent, correct home — with a doc note recording the divergence and its reason. Also extended the in-module pin in `src/driver/goal.rs` with the same three tokens, as the plan directed.
- **Files modified:** `tests/driver_goal_seam.rs`, `src/driver/goal.rs`
- **Verification:** `rtk proxy cargo test --lib goal` → 41 passed, 0 failures; whole suite 0 failures.
- **Committed in:** `a3da3d1`

**2. [Rule 2 - Missing critical] `derive_alias` needed an "at least one ASCII alphanumeric" filter, not only an emptiness filter**

- **Found during:** Task 2(c)
- **Issue:** The plan specified mapping disallowed characters to `'-'` and collapsing repeats, relying on the existing `.filter(|s| !s.is_empty())`. That filter does not catch a name that sanitizes to `"-"` or `"..."` — both non-empty, both useless as aliases, and `unique_alias` would grow `"-"` into `"-2"`, `"-3"`. The plan named this residual and asked for a pin; the pin would have failed without the extra filter.
- **Fix:** Added `.filter(|s| s.chars().any(|c| c.is_ascii_alphanumeric()))` after the trim, so such names fall back to `"project"`.
- **Files modified:** `src/registry.rs`
- **Verification:** `a_folder_named_in_a_non_latin_script_derives_a_usable_alias_or_refuses_with_the_hint` passes, asserting the derived value is never `""`, `"-"`, or an all-`'-'` run.
- **Committed in:** `a3da3d1`

**3. [Rule 2 - Missing critical] Stale hand-maintained counts in four docs**

- **Found during:** Task 1(e)
- **Issue:** Growing `DEGENERATE` falsified four prose statements that named its length ("six concrete payloads", "a seventh blank shape", "six of six"). One of them (`src/driver/mod.rs`) additionally credited `DEGENERATE` with the anti-tautology property that this plan's own `test_support` correction denies.
- **Fix:** Corrected all four to count-free phrasing that survives future growth, and rewrote the driver's paragraph to name the sweep as what actually breaks the tautology. Left genuinely HISTORICAL statements ("three of the six blank shapes defined in the very commit that defined six") intact — those describe a past commit and remain true.
- **Files modified:** `src/driver/mod.rs`, `src/journal/mod.rs`, `src/test_support.rs`, `tests/driver_dry_run.rs`
- **Verification:** `rtk proxy grep -rn "7x6\|42-cell\|six payloads\|; 6\]" src/ tests/` → only `LOOK_ALIKE_PAIRS`'s own `; 6]` declaration and one historical sentence.
- **Committed in:** `f434d71`

### Task 1(f) — nothing to reconcile

The plan anticipated that the class widening might break in-tree tests asserting an outside-the-old-ranges character is ACCEPTED, and required each such conflict to be judged and recorded rather than silently rewritten. **The full suite was run to enumerate them and there were none** — 0 failures immediately after the widening, before any test was touched. No acceptance pin in the tree depended on the old subset.

---

**Total deviations:** 3 auto-fixed (1 plan-text bug, 2 missing critical).
**Impact on plan:** No scope creep. Deviation 1 preserves an invariant the tree had already recorded a deviation to protect; deviations 2 and 3 were required for the plan's own pins and prohibitions to hold.

## Issues Encountered

- **`rtk` output filtering was avoided throughout**, per the phase's blocking constraint. Every count-bearing, presence-bearing, grep-bearing, and build/test-output check in this SUMMARY was run as `rtk proxy <cmd>`.
- **Two scratch-script authoring hazards, both caught and corrected.** A first draft of the nine-add reproduction script was written as bash with raw invisible characters pasted into `printf` arguments — a direct prohibition-2 violation. It was deleted unrun and rewritten in Python where the hostile values are built from `chr(0x...)`, keeping every script source pure ASCII. The programmatic scan afterwards confirms zero raw invisible characters across all 13 changed files.
- **The sandbox refused several `grep` invocations** containing the word "alias" in a shell string. Worked around by reading files directly rather than by weakening the check.

## Notes for the verifier

- **`general-category` returns 2 grep hits in `Cargo.toml`, not 1.** Line 120 is the dev-dependency entry (the criterion's target); line 110 is the comment explaining why the feature is named rather than defaulted. The criterion's intent — pinned explicitly, not defaulted — is met more strongly, not less.
- **Pre-existing `is_ascii_alphanumeric` hits, inspected and recorded** as the plan required. `src/journal/writer.rs:585` is a run-id *suffix shape* check (4 alphanumerics), not a character-class identity judgment. `src/envelope/advisory.rs:569` and `:598` judge GitHub owner/repo path segments and a default branch name fetched from the API before URL interpolation; `:569` does spell the same `[A-Za-z0-9._-]` set for a different purpose, and `:598` allows `/` as well, so it is a different set. Both are pre-existing, outside this plan's `files_modified`, and not identity seams under `is_plain_path_component`. **Recommended follow-up (not done here, out of scope):** consider delegating `advisory.rs:569` to `text::is_identity_char` so the alphabet has one spelling tree-wide.
- **`actuals.tokens` uses the "chars/4 over files actually changed" reading** (726,564 chars over the 13 non-lockfile files → 181,641), which is the reading comparable to the plan's `estimate.tokens: 110000`. The alternative "chars/4 over the realized diff" reading gives 7,100. Stated explicitly so the calibration is auditable rather than silently on the wrong scale. On the reading used, this plan ran ~65% over its estimate.
- **TR39 confusables remain a separate roadmap item** and are recommended as such. None of pass 7's twenty reproduced values is a homoglyph.

## Verification results

| Gate | Command | Result |
|---|---|---|
| Build | `rtk proxy cargo build --all-targets` | clean |
| Clippy | `rtk proxy cargo clippy --lib -- -D warnings` | clean |
| Tests | `rtk proxy cargo test --workspace --no-fail-fast -- --test-threads=2` | **35 binaries, 1358 passed, 0 failed, 13 ignored** |

The 13 ignored are the documented pre-existing set: 10 `driver_injection_corpus` tests requiring a live Claude subscription, plus 3 others. That count is unchanged from the pre-plan baseline — the tracer's `#[ignore]` was removed in the fix commit and no new ignore was added. No flake was encountered in any run; the three documented flaky tests passed on every execution.

`from_argv` shape unchanged (verification criterion 6): `rtk proxy grep -A16 "let RawDriveArgs" src/driver/mod.rs` shows all twelve fields destructured with no `..`.

## User Setup Required

None — no external service configuration required. One maintenance obligation is now on the release process, and it is one the process already performs: `cargo update` (CLAUDE.md release step 2) is what refreshes the Unicode class.

## Next Phase Readiness

- Pass-7 gap 1 and gap 2 are closed at their shared root, and row 14 of the named-shape audit is delegated to 21-20 as planned.
- **This plan's four task commits touch no file under `.planning/`** — no requirement was flipped. This SUMMARY is the only `.planning/` file the plan writes; STATE.md and ROADMAP.md are the orchestrator's to write.
- **For the verifier:** the two claims worth re-deriving independently are (a) that no test asserting class membership reads the production class as its expected set, and (b) that the `format_seen >= 150` floor is committed rather than a one-time executor ritual. Both are checkable from the tree without re-running the demonstrations.

## Self-Check: PASSED

**Files claimed, verified present on disk:** `src/text.rs`, `src/test_support.rs`, `src/journal/mod.rs`, `src/registry.rs`, `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-19-SUMMARY.md` — all found.

**Commits claimed, verified in `git log`:** `8ca0dd1`, `f434d71`, `a3da3d1`, `e7e5572`, `6544222` — all five present between the worktree base `1be0225` and HEAD, in the stated order, with the red arm preceding its fix.

**Acceptance criteria re-run:** all three tasks' criteria pass, including the plan-level `<verification>` gates recorded in the results table above. `git log --name-only 1be0225..HEAD -- .planning/` over the four task commits returns empty, confirming no REQUIREMENTS.md change.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-25*
