---
phase: 21-llm-goal-layer-prompt-injection-hardening
round: 6
reviewed: 2026-08-23T03:20:00Z
depth: deep
diff_base: 0352dda
head: 7260ac8
previous_round: "Round 5's 21-REVIEW.md is preserved in git at 5b24022 (`git show 5b24022:.planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-REVIEW.md`). This file replaces it."
files_reviewed: 20
files_reviewed_list:
  - src/app.rs
  - src/driver/goal.rs
  - src/driver/mod.rs
  - src/driver/run.rs
  - src/envelope/cred.rs
  - src/envelope/hooks.rs
  - src/envelope/mod.rs
  - src/error.rs
  - src/journal/mod.rs
  - src/lib.rs
  - src/main.rs
  - src/registry.rs
  - src/test_support.rs
  - src/text.rs
  - src/ui/screens/add_project.rs
  - tests/driver_dry_run.rs
  - tests/driver_goal_seam.rs
  - tests/driver_optin.rs
  - tests/registry_test.rs
  - tests/spawn_seam_guard.rs
files_read_but_unchanged_this_round:
  - src/cli.rs
  - src/journal/writer.rs
  - src/envelope/policy.rs
  - tests/driver_injection_corpus.rs
findings:
  critical: 1
  warning: 5
  info: 3
  total: 9
status: issues_found
---

# Phase 21 Round 6: Code Review Report

**Reviewed:** 2026-08-23T03:20:00Z
**Depth:** deep (cross-file; call chains, predicate composition, end-to-end binary reproduction, guard-scanner re-execution against fresh plants)
**Diff base:** `0352dda` → `7260ac8` (13 commits; `ca1956b` ignored as unrelated)
**Round 5's review:** preserved at `5b24022`; this file replaces it.
**Status:** issues_found — **1 Critical, reproduced end to end against the built binary.**

## Summary

The sixth defect of the family exists, and it is one level below where the last
five were found. Rounds 2–4 hand-enumerated **arms**; round 5 hand-enumerated
**fields**; round 6 correctly stopped hand-enumerating either — and then
hand-enumerated the **character class itself**.

`text::is_invisible_formatting_char` (`src/text.rs:75-77`) spells three literal
ranges. Nine values that all render as `demo` in a terminal registered as nine
distinct registry keys against the binary built from HEAD, including the
Unicode bidi-override block (U+202A–U+202E — Trojan Source, CVE-2021-42574) and
the Unicode tag block (U+E0000–U+E007F — the standard LLM "ASCII smuggling"
carrier, in a *prompt-injection-hardening* phase). The failure signature is the
one the round was designed against, verbatim: **round 6's clause covers exactly
the characters pass 6 had reproduced (U+200B, U+200C, U+FEFF) and misses the
class that was only named** ("zero-width and format characters" — the doc's own
words at `src/text.rs:91-96`). Must-have truth 8, the backstop —
*"No registry key, envelope root, credential scope, run directory name, or
accepted phase token created by this build differs from another only by
invisible formatting characters"* — is measurably false at HEAD.

Structurally, everything else this round built is sound and I could not break it.
`registry::Alias` is a real newtype with a genuinely delegating constructor and
the fourth predicate is deleted rather than relocated; the compiler now makes an
unjudged alias unrepresentable at both registration signatures; `AliasNotVisible`
correctly stops narrating the registry; the shared `is_invisible_formatting_char`
is genuinely shared (claim A's first half holds); guard nine's deny-by-default
`OsString` rule is genuinely deny-by-default and not a protected-name list in
disguise (claim C holds as designed); guard ten's census is exact-count with a
same-extracted-fn control and the count of 8 is independently correct;
`one_of_each` is now honest and no fictitious mechanism was substituted (claim E
holds); the acceptance matrix's exemption is gone. **Criterion 1 has not
regressed** — see the negative-evidence section, which is the longer half of this
report.

The Warnings are all of one kind: **mechanisms whose stated coverage exceeds
what their committed controls measure** — which is precisely the class 21-18
exists to end, recurring inside 21-18's own artifacts. Two of them
(`is_field_opener`, the `OSSTRING_ALLOWED` early return) I reproduced by
compiling and running guard nine's own extracted functions against fresh plants.

---

## Critical Issues

### CR-01: The invisible-formatting class is a hand-enumerated subset; nine aliases that render identically all register

**REPRODUCED** — against `target/debug/gsd-meta-manager` at HEAD, twice, with two
disjoint scratch configs.

**File:** `src/text.rs:75-77` (the class), `src/text.rs:101-103` (the judgment),
`src/journal/mod.rs:309-311` (the seam clause), `src/registry.rs:97-101`
(`Alias::new` clause 2), `src/test_support.rs:53-57` (the fixture that cannot
falsify it)

**Issue.**

```rust
fn is_invisible_formatting_char(c: char) -> bool {
    matches!(c, '\u{200b}'..='\u{200f}' | '\u{2060}'..='\u{2064}' | '\u{feff}')
}
```

Three ranges. The class the doc claims — *"zero-width and format characters"*
(`src/text.rs:91-96`, `src/journal/mod.rs` correction-list entry 4, and
`AliasRefusal::InvisibleFormatting`'s message at `src/registry.rs:61-70`) — is
General_Category `Cf` plus the zero-width combining marks. The gap between the
two literal ranges (U+2010–U+205F) contains **U+202A–U+202E**, the bidi
embedding/override block, and **U+2066–U+2069**, the bidi isolates. Everything
above U+FEFF is missing entirely, including the tag block.

Reproduction, verbatim, against a scratch config and one fixture project:

```
$ for a in demo demo+U+202E demo+U+202D demo+U+2066 demo+U+2069 \
           demo+U+061C demo+U+FE0F demo+U+E0001 demo+U+13430 demo+U+200B ; do
      gsd-meta-manager --config <scratch> add <fixture> "$a" ; done

BASELINE demo                  exit=0
U+202E RLO (Trojan Source)     exit=0
U+202D LRO                     exit=0
U+2066 LRI                     exit=0
U+2069 PDI                     exit=0
U+061C ALM                     exit=0
U+FE0F VS16                    exit=0
U+E0001 LANGUAGE TAG           exit=0
U+13430 EGYPTIAN Cf            exit=0
REFUSED CONTROL U+200B         exit=1        <- the only one refused

registry keys (9):
   'demo'            'demo\u061c'       'demo\u202d'       'demo\u202e'
   'demo\u2066'      'demo\u2069'       'demo\ufe0f'       'demo\U00013430'
   'demo\U000e0001'
```

A second independent probe added U+E0041 (tag), U+00AD (soft hyphen), U+034F
(CGJ), U+180E (Mongolian vowel separator) and U+FFF9 (interlinear annotation
anchor) — all `exit=0`, all seven keys rendering as `demo` in `list`:

```
$ gsd-meta-manager --config <scratch> list | cat -A
demo                 /…/proj  2026-08-23T03:00:03…
demoM-BM--           /…/proj  2026-08-23T03:00:03…      # U+00AD
demoM-MM-^O          /…/proj  2026-08-23T03:00:04…      # U+034F
demoM-aM- M-^N       /…/proj  2026-08-23T03:00:04…      # U+180E
demoM-oM-8M-^@       /…/proj  2026-08-23T03:00:03…      # U+FE00
demoM-oM-?M-9        /…/proj  2026-08-23T03:00:04…      # U+FFF9
demoM-sM- M-^AM-^A   /…/proj  2026-08-23T03:00:03…      # U+E0041
```

**Every seam this round closed is reopened for the uncovered part of the class**,
because they all read one predicate:

| Seam | Judge | Status for U+202E / U+E0041 / U+FE0F / U+00AD |
|---|---|---|
| registry key | `Alias::new` clause 2 → `carries_invisible_formatting` | **accepted** (reproduced above) |
| envelope root | `envelope_dir_in` → `is_plain_path_component` | **accepted** — proven: `Alias::new` clause 4 *is* `is_plain_path_component`, and it returned `Ok` |
| credential scope | hangs off `envelope_dir` | **accepted**, same proof |
| run directory | `run_paths` → `is_plain_path_component` (`src/journal/mod.rs:355`) | **accepted**, same predicate |
| phase token | `goal::legality` → `is_plain_path_component` (`src/driver/goal.rs:667`) | **accepted** by the predicate; falls back to roadmap membership — i.e. back to the *coincidental reliance* SAFE-08's pin was written to eliminate |

**The bidi half is worse than "renders as nothing."** U+202E does not merely
disappear — it reorders the text after it. `list`, the TUI project table, and
`judged_alias_or_exit`'s refusal (`src/main.rs:33-47`, which prints the alias
through `Display`, not `Debug`) all render the raw alias, so an alias
`gsd-\u{202E}nur` renders as `gsd-run` while being a different key. That is a
terminal-spoofing vector in the tool's primary identity display, added by an
`add` that exits 0.

**Why nothing in the tree could catch it, stated as the systemic cause.**
`test_support::LOOK_ALIKE_PAIRS` is built entirely from U+200B and U+FEFF —
i.e. entirely from inside the predicate's own ranges — so the fixture can only
ever agree with the implementation. Its doc (`src/test_support.rs:49-52`)
explicitly claims the opposite: *"an enumeration derived from
`carries_invisible_formatting`'s own structure could not contain a pair that
predicate mishandles (round-3 WR-03's tautology)."* Constructed from literals it
may be; drawn from the predicate's own coverage it certainly is. This is WR-03's
tautology re-entering one level down, and it is the reason a green suite (1346/0)
sits on top of a reproduced harm for the sixth consecutive round.

**Fix.** Replace the literal ranges with a class, and make the class checkable by
a mechanism rather than a list:

```rust
/// Every code point that is default-ignorable, a bidi control, or a
/// zero-width combining mark. Derived from a checked-in Unicode data table,
/// not hand-written — a hand list is exactly how this shipped as a subset.
fn is_invisible_formatting_char(c: char) -> bool {
    matches!(c,
        '\u{00ad}'                       // SOFT HYPHEN
        | '\u{034f}'                     // COMBINING GRAPHEME JOINER
        | '\u{061c}'                     // ARABIC LETTER MARK
        | '\u{115f}'..='\u{1160}'        // HANGUL FILLERS
        | '\u{17b4}'..='\u{17b5}'
        | '\u{180b}'..='\u{180f}'        // MONGOLIAN FVS / VOWEL SEPARATOR
        | '\u{200b}'..='\u{200f}'        // ZW SPACE..RLM
        | '\u{202a}'..='\u{202e}'        // BIDI EMBEDDING / OVERRIDE  <-- new
        | '\u{2060}'..='\u{2064}'
        | '\u{2065}'..='\u{206f}'        // reserved-DI + deprecated fmt <-- new
        | '\u{3164}' | '\u{feff}' | '\u{ffa0}'
        | '\u{fff0}'..='\u{fffb}'        // incl. interlinear annotation <-- new
        | '\u{fe00}'..='\u{fe0f}'        // VARIATION SELECTORS         <-- new
        | '\u{13430}'..='\u{1343f}'      // EGYPTIAN FORMAT CONTROLS    <-- new
        | '\u{1bca0}'..='\u{1bca3}'
        | '\u{1d173}'..='\u{1d17a}'
        | '\u{e0000}'..='\u{e0fff}'      // TAGS + VARIATION SELECTORS  <-- new
    )
}
```

and add the mechanism the fixture cannot supply — a property test over a
checked-in corpus, so the class can never silently be a subset again:

```rust
/// Every Unicode code point whose General_Category is `Cf`, plus the
/// variation selectors and U+034F, read from `tests/data/invisible.txt`
/// (generated from UCD `DerivedCoreProperties.txt`, Default_Ignorable_Code_Point).
/// A fixture built from the predicate's own ranges can only agree with it —
/// this one is derived from the STANDARD, so it can falsify.
#[test]
fn every_default_ignorable_code_point_is_refused() {
    for c in unicode_default_ignorable_corpus() {
        assert!(
            carries_invisible_formatting(&format!("demo{c}")),
            "U+{:04X} renders as nothing (or reorders its neighbours) and must \
             not be able to name an identity beside `demo`", c as u32
        );
    }
}
```

Then extend `LOOK_ALIKE_PAIRS` with at least one member drawn from *outside* the
current ranges — `("demo", "demo\u{202e}")` and `("demo", "demo\u{e0041}")` —
so the fixture regains the falsifying power its doc claims.

**Disclosure note.** The doc's TR39/homoglyph carve-out (`src/text.rs:90-96`) is
correct and should stay — a Cyrillic `а` is genuinely a different problem. None
of the nine values above is a homoglyph; every one of them is default-ignorable
or a bidi control, i.e. squarely inside the class the doc says it covers.

---

## Warnings

### WR-01: Guard nine is blind to a field declared without `pub` — the same measurement signature as pass 6's, in a spelling neither SEEN nor SILENT names

**REPRODUCED** — by extracting guard nine's own `raw_string_argv_fields`,
`judge_declaration`, `drive_args_body`, `names_token`, `names_string_payload`,
`without_trailing_comment`, `declared_field_name`, `is_field_opener` and
`synthetic_file` **verbatim** into a standalone binary and running them.

**File:** `tests/spawn_seam_guard.rs:2992-2994` (`is_field_opener`),
`:2766-2784` (the SILENT list that omits it), `:3309-3315` (the floor filter that
inherits it)

**Issue.** `is_field_opener` is `trimmed.starts_with("pub ") || trimmed.starts_with("pub(")`.
A field declared with no visibility modifier opens neither. Fed the exact
twelve-field body from the guard's own floor probe plus one bare private field:

```
12 pub fields + 1 BARE PRIVATE String field:
  offenders          = []
  floor field_lines  = 12  (>=10 ? true)
  protected NonBlank = 6   (>=6 ? true)
```

That is `field_lines=12 protected=6 offenders=[]` — character for character the
signature pass 6 recorded for `pub(crate)` and that this round's control test
now pins as fixed. The `pub(crate)`/`pub(super)` spellings were reproduced by
the verifier and are now covered; the bare spelling was never planted and is not
in the SEEN list, not in the SILENT list, and not bounded by anything the guard
asserts. What actually stops it today is that thirteen integration crates build
`DriveArgs { … }` as struct literals and a private field breaks all thirteen —
which is exactly the "thirteen-fixture-file coincidental compile bound" that
`21-18-SUMMARY.md` named-shape row 15 reports as **retired**. It is retired for
the two payload-type spellings pass 6 measured; it is intact and load-bearing for
this one.

**Fix.** One line in the shared helper, one plant in the control, one line in the
limits block:

```rust
fn is_field_opener(trimmed: &str) -> bool {
    // A field needs no visibility modifier at all to be a field. `DriveArgs`
    // is `pub`, so a private field breaks the thirteen integration fixtures
    // that build it literally — but that is a coincidence of today's tests,
    // not a property, and this guard is what makes the property real.
    trimmed.starts_with("pub ")
        || trimmed.starts_with("pub(")
        || trimmed
            .split_once(':')
            .is_some_and(|(name, _)| {
                !name.is_empty() && name.bytes().all(|b| is_ident_byte(b))
            })
}
```

plus, in `the_raw_argv_field_scanner_sees_every_measured_silent_spelling`:

```rust
let bare = planted_with(&["    goal_file: Option<String>,"]);
assert_eq!(raw_string_argv_fields(&bare).len(), 1,
    "a field needs no `pub` to be a field. Bounded today only by thirteen \
     integration crates building `DriveArgs` literally — a coincidence, not \
     a property. Got: {:?}", raw_string_argv_fields(&bare));
```

### WR-02: `OSSTRING_ALLOWED`'s early return plus a substring integrity pin lets an allowlisted field carry a raw `String` payload undetected

**REPRODUCED** — same standalone harness, guard nine's own functions.

**File:** `tests/spawn_seam_guard.rs:2859-2866` (`judge_declaration`'s early
return), `:3343-3367` (the integrity pin)

**Issue.** `judge_declaration` short-circuits:

```rust
if names_token(type_text, "OsString") {
    let name = declared_field_name(&joined);
    if !OSSTRING_ALLOWED.contains(&name) { out.push((start, joined)); }
    return;                                   // <-- String never examined
}
if names_string_payload(type_text) { out.push((start, joined)); }
```

so a type that names `OsString` **and** a raw payload, on an allowlisted name, is
suppressed wholesale. The live assertion's "cannot be silently repurposed" pin
(`:3358`) is `declaration.1.contains(expected_type)` — a **substring** test — so
it also passes:

```
claude_args repurposed to (Vec<OsString>, String):
  offenders                              = []
  integrity pin contains("Vec<OsString>") = true
```

`21-18` truth 1 and the prohibition-3 table both certify that "an allowlist entry
cannot be silently repurposed." Measured: it can. (`claude_program` is not
exposed — `Option<PathBuf>` names no `OsString`, so it falls through to the
`String`/`str` check and *is* reported. The hole is specific to the branch that
returns early.)

**Fix.** Do not return early, and make the integrity pin an equality:

```rust
if names_token(type_text, "OsString") && OSSTRING_ALLOWED.contains(&declared_field_name(&joined)) {
    // Suppressed for the OsString itself — but a payload type sharing the
    // declaration is a different offence and is still judged.
    if names_string_payload(type_text) { out.push((start, joined)); }
    return;
}
```

```rust
let declared_type = declaration.1.trim()
    .split_once(':').map(|(_, t)| t.trim().trim_end_matches(','))
    .unwrap_or("");
assert_eq!(declared_type, expected_type,
    "`{name}` is on OSSTRING_ALLOWED because it declares EXACTLY \
     `{expected_type}`; it now declares {declared_type:?}");
```

### WR-03: `LOOK_ALIKE_PAIRS` is drawn from inside the predicate's own ranges, so it cannot falsify the predicate — while its doc claims it can

**Partly REPRODUCED (CR-01 is the demonstration), partly INFERRED by reading.**

**File:** `src/test_support.rs:49-57`

**Issue.** All three pairs use U+200B or U+FEFF. The doc asserts the
anti-tautology property WR-03 was written for — that the enumeration is not
derived from the predicate's structure. It is literal, but it is a *subset of the
predicate's coverage*, which delivers the same tautology by a different route: no
pair in the const can be mishandled, so every LOOK_ALIKE assertion at every seam
(`journal/mod.rs:2836`, `envelope/mod.rs:460`, `registry.rs:641`,
`text.rs:185`, `tests/registry_test.rs:99`) passes for any implementation that
covers those two characters and nothing else. CR-01 is what that costs.

**Fix.** Add pairs from outside the current ranges and state the rule at the
const: *every pair must include at least one member the predicate does not
already cover by construction* — e.g. `("demo", "demo\u{202e}")`,
`("demo", "demo\u{e0041}")`, `("demo", "demo\u{fe0f}")`. With CR-01's fix these
turn green; without it they turn red, which is the point.

### WR-04: The tree-wide `DEGENERATE` uniqueness guard detects a hand-copied subset only if the subset contains one particular literal; the under-detection direction is undisclosed and its message overclaims

**INFERRED** (direct read of the scan; not planted, because planting requires
editing the repo and the review is read-only). Trivially reproducible.

**File:** `tests/spawn_seam_guard.rs:3438-3455`, `:3518-3549`

**Issue.** Detection is `executable_hits(&files, degenerate_witness())` where the
witness is the single literal `"\n  \n"` — the fourth of six `DEGENERATE`
members. A future hand-copied subset that omits that member —
`["", "   ", "\t", "\u{200b}"]`, say, which is four of six and exactly the shape
pass 5 found — is invisible to the scan. The guard's failure message
(`:3539-3546`) nonetheless reads *"Every blank-shape pin consumes
`test_support::DEGENERATE`"*, and `21-18` truth 5 certifies *"`DEGENERATE` is
spelled in exactly one place TREE-WIDE and the guard's message is finally true."*
Its coverage is one literal wide. The doc names only the over-detection direction
("no ordinary string literal contains it"); the under-detection direction is not
named — which is prohibition 3's exact prohibition, inside prohibition 3's own
round. (No standing violation exists: I censused `grep -rn '"   "' src/ tests/`
and every hit is prose, a single-value assertion, or the const itself.)

**Fix.** Scan for the set, not one member, and state the residual:

```rust
/// Two witnesses, because a subset that omits one is the shape pass 5 found.
/// Anything hand-copying `DEGENERATE` will carry at least one of these.
fn degenerate_witnesses() -> [String; 2] {
    [format!("{DEGENERATE_WITNESS_HEAD}{DEGENERATE_WITNESS_TAIL}"),
     r#""\u{200b}", "\u{feff}""#.to_string()]
}
```

plus a limits line: *a hand copy that omits every witness member is silent
under-detection; the witnesses are chosen as the members no other literal in the
tree carries.*

### WR-05: `21-18-SUMMARY`'s SAFE-07 qualification misdescribes the ignored set, and omits that the arms' own non-vacuity check is itself ignored

**REPRODUCED** — `cargo test --test driver_injection_corpus -- --list --ignored`.

**File:** `.planning/phases/21-…/21-18-SUMMARY.md:389-401` (record, not source —
reported because review question G.1 asks for this adjudication)

**Issue.** The SUMMARY states: *"the ignored set is exactly the eight
`corpus_*_arrives_and_leaves_the_command_unchanged` arms plus the two suppression
controls."* Measured:

```
$ cargo test --test driver_injection_corpus -- --list --ignored
both_arms_of_every_class_comparison_were_really_executed: test
corpus_delimiter_escape_bare_…            corpus_delimiter_escape_nonce_…
corpus_encoded_payload_…                  corpus_instruction_override_…
corpus_multi_turn_deferral_…              corpus_role_confusion_…
corpus_tool_output_shaping_…
the_negative_control_…                    the_positive_control_…
10 tests
```

There are **seven** such arms, not eight. The tenth ignored test is
`both_arms_of_every_class_comparison_were_really_executed`
(`tests/driver_injection_corpus.rs:1045-1046`) — the meta-check that certifies
the hostile and clean arms both really ran. The SUMMARY's list of pins "that make
those arms non-vacuous when they are run" omits it, and it is the only one that
speaks to the comparison's non-vacuity rather than the corpus's.

**Adjudication of G.1.** A structurally-reconfirmed SAFE-07 is **adequate as a
regression check on this round's diff** — nothing in `0352dda..HEAD` touches the
corpus, the boundary, or the reader, and the six executed integrity pins
genuinely prevent the arms from passing by the payload never arriving or the
binary being absent. It is **not adequate as a reconfirmation of SAFE-07's
boundary**, and the corpus-integrity pins are *not* sufficient to make the arms
non-vacuous when eventually run, because the check that both arms executed is in
the ignored set with them. Carry as an open item: SAFE-07's boundary has not
been executed end-to-end in any pass of this phase. Marking the coverage entry
`partial` was right; the enumeration backing it needs correcting.

**Fix.** Correct the sentence to "seven class arms, the two suppression controls,
and `both_arms_of_every_class_comparison_were_really_executed` — the last of
which is the arms' own non-vacuity check and did not execute either," and record
"SAFE-07 boundary never executed live" in `deferred-items.md` rather than in a
SUMMARY qualification.

---

## Info

### IN-01: The CR-01 tracer's envelope half skips silently when `envelope_root()` is `None`

**File:** `src/driver/run.rs:3840`

**Adjudication of G.2 — it is not a silent skip today, and the reasoning is
sound.** `envelope_dir` returns `None` only when `envelope_root()` does (the
alias `"cr01-envelope-probe"` is a plain component), and in that case
`establish_envelope`'s very first line — `hooks::install(alias)?`
(`src/driver/run.rs:123`) — cannot resolve a root either, so there is genuinely
nothing for a premature establishment to have written. Verified by reading both
functions, not taken from the SUMMARY.

The residual is one step out: nothing asserts the `Some` branch was *taken*. A
future change to the test harness (a sandboxed CI without `XDG_DATA_HOME`, a
`#[cfg(test)]` short-circuit in `envelope_root`) would make the envelope half
vacuous without any test going red, and this test exists precisely because a
silently-vacuous assertion was the last thing that went unnoticed here.

**Fix.** Two lines:

```rust
let envelope_dir = crate::envelope::envelope_dir(CR01_ENVELOPE_PROBE_ALIAS)
    .expect(
        "the envelope root must resolve, or this assertion is vacuous. If it \
         genuinely cannot on this machine, `establish_envelope` could not have \
         written either — but that must be OBSERVED, not assumed, because a \
         vacuous half is what this assertion exists to stop.",
    );
assert!(!envelope_dir.exists(), …);
```

### IN-02: Guard ten's census never checks that a table row's variant exists

**File:** `tests/spawn_seam_guard.rs:3594-3629`, `:3747-3760`

The live assertion checks the *count* against `src/cli.rs` and the *shape* of
each judge string, but never that `ARGV_ALIAS_ENTRY_POINTS`'s variant names
(`"EnvelopeAction::Scan"` etc.) appear in `cli.rs` at all. Renaming a variant, or
removing one while adding another, leaves the count at 8 and the table stale with
a row naming a variant that no longer exists. The limits block names the
declaration-spelling and judge-prose approximations but not this one.

**Fix.** One loop, and a limits line:

```rust
for (variant, _) in ARGV_ALIAS_ENTRY_POINTS {
    let short = variant.rsplit("::").next().expect("a qualified variant name");
    assert!(
        home.1.iter().any(|(_, line)| line.trim().starts_with(&format!("{short} {{"))),
        "{variant} has a census row but is not declared in {ARGV_ALIAS_HOME}. \
         A stale row keeps the count at 8 while the classification describes a \
         variant that no longer exists."
    );
}
```

### IN-03: The `Add` arm now judges the alias before `load_config`, inverting the message order IN-04 documented

**File:** `src/main.rs:77-90`

Round 5's IN-04 recorded (and review G.4 adjudicated as pre-existing) that
`load_config` ran before `DriveArgs::from_argv`, so a bad `--config` was reported
before a bad payload. This round's `Add` arm reverses that relationship for
registration: `Alias::new` now runs *above* `load_config(&config_path)?`. The
consequence is message ordering only, neither path writes, and the new order is
arguably the better one (a refusal about the value costs no file read). Recorded
because the phase treats ordering facts as load-bearing and the change is
undocumented at the site.

**Fix.** One comment at the arm noting the deliberate order and why (pure
refusals before I/O), so the next reader does not have to re-derive it.

---

## Negative evidence — what I tried that failed to break it

Given five-for-five, this section is the load-bearing one. Everything below was
attempted with intent to falsify and did not yield.

**Criterion 1 — verified intact, by direct read, not from the SUMMARYs.**

- `DriveArgs::from_argv`'s destructure (`src/driver/mod.rs:414-429`) names all
  **twelve** fields with **no** `..` rest pattern; the two `#[cfg(debug_assertions)]`
  fields are named individually.
- The six argv-derived string fields are still `payload::NonBlank`
  (`alias`, `command`, `target_phase`, `approved_plan`, `run_id`, `goal`);
  `drive_args_declares_no_raw_argv_string_field` reports zero offenders against
  the real body, and the `>=10`/`>=6` floors both hold.
- `from_argv` is still pure — no file, no process — and still above `drive`.
  The round's entire diff at that function is **one arm**: the alias refusal's
  variant. Verified line by line against `git diff`.
- `execute_run`'s resolve-above-every-write ordering is untouched; `run.rs`'s
  only change is inside `#[cfg(test)] mod tests`.
- `cargo test --lib` → **1037 passed / 0 failed**; `cargo test --test
  spawn_seam_guard` → **35 passed / 0 failed**; `cargo clippy --lib -- -D
  warnings` → clean. Run by me, at HEAD, via `rtk proxy`.

**Claim A (the shared class) — holds.** `carries_visible_content` (`text.rs:63-67`)
and `carries_invisible_formatting` (`text.rs:101-103`) both call
`is_invisible_formatting_char`; the ranges appear exactly once in production
(`grep -rln "2060" src/` → `src/text.rs` and `src/driver/mod.rs`, the latter the
declared independent test-side oracle inside its `#[cfg(test)]` module). I looked
for a second spelling in `journal`, `registry`, `envelope`, `goal` and `payload`
and found none. The two-judgment split is real; the CR-01 defect is the class's
*extent*, not its sharing.

**Claim B (the fourth predicate) — deleted, not relocated.**
`grep -n "is_empty\|is_whitespace\|trim()" src/registry.rs` returns, in
executable code: `Alias::new`'s delegating clause 3 (`:107`),
`opt_in.prompt_inputs.is_empty()` (`:389`, unrelated) and
`derive_alias`'s file-name fallback (`:494`, which feeds `Alias::new` anyway).
`add_project`/`add_project_unchecked` carry no predicate. Every clause of
`Alias::new` delegates; I checked each one resolves to `text::` or `journal::`.
`AliasNotVisible` correctly replaces the borrowed `UnknownAlias` and is added to
the `source()` classification group with no wildcard arm absorbing it; the only
other `match self` over `DriveError` is `Display`, which handles it explicitly.

**Claim C (deny-by-default) — genuinely deny-by-default.** I fed
`judge_declaration` a field named `goal_file` typed `Option<OsString>` and it was
reported; a *by-name protection* list would not have. The allowlist suppresses
only, and I confirmed by construction that a seventh field with any new name is
bound. The hole is WR-02's compound-type case, not the rule's direction.

**Guard nine's fixed spellings — all genuinely fixed.** Run through the guard's
own extracted functions with fresh plants: `pub(crate)` ✓, `pub(super)` ✓,
`Box<str>` ✓, `Cow<'static, str>` ✓, `&'static str` ✓, `Option<OsString>` on a
new name ✓, trailing `//` last-field ✓, trailing `//` mid-struct with correct
attribution and `dry_run` still judged ✓, wrapped declarations ✓,
`Vec<OsString>`/`Option<PathBuf>` correctly NOT reported ✓. I additionally tried
`Vec<String>` (reported ✓) and a `/* … */` trailing comment (reported ✓). The
scan and the floor share one `is_field_opener` — I verified both call sites.

**Claim F (guard ten) — count independently correct.** `grep -n 'alias' src/cli.rs`
shows exactly eight `alias: String,` / `alias: Option<String>,` declarations
(lines 30, 35, 50, 306, 319, 337, 361, 371) and the census asserts exactly 8. The
planted-ninth control consumes `argv_alias_fields`, the same extracted fn, not a
re-implemented loop — read line by line.

**Claim E (`one_of_each`) — honest, and nothing weaker was substituted.** The
false "compile error in two ways" claim is deleted from both the doc and the pin
message; the residual is stated in the doc; no `named(source)` helper or
equivalent pseudo-mechanism was added. The prohibition-3 table enters it as
*named, not certified*, which is correct.

**Shell injection through the alias into generated hook stubs — not present.**
`is_plain_path_component` accepts `;`, `$`, backtick and `'`, so I chased
`stub_body` (`src/envelope/hooks.rs:167-175`) and `write_askpass_stub_in`. Both
route the alias through `sh_quote` (`:183-185`), which POSIX single-quotes with
correct `'\''` escaping and is deliberately shared so the escaping exists once.
The stub interpolates only quoted values and `"$0"`. Clean.

**Fail-closed exit codes — correct.** The `Guard` arm's `judged_alias_or_exit(&alias, 2)`
exits 2 without writing the deny JSON; I checked `deny()` (`hooks.rs:1129-1131`),
which states and relies on exit 2 being "the carrier that does not depend on the
JSON being understood." Blocking is preserved. The `Askpass` arm emits no
credential on the refusal path and prints only the alias (Debug-escaped through
`AliasRefusal`), so the redaction contract holds. Hook arms exit 1; each message
contains `remove`.

**Registration reachable only through `Alias`** — enforced by the signatures, so
compiler-checked rather than grep-checked. I confirmed all four callers
(`main.rs` Add/Scan, `app.rs:1157`, `ui/screens/add_project.rs:172`,
`registry.rs:555` auto-register) construct through `Alias::new`, and that
auto-register's `Err` path warns and continues rather than registering.

**No weakened test.** Every `tests/` conversion in the diff *widens*: the two
`driver_dry_run` loops go from two-of-six and four-of-six to all six; the
`driver_goal_seam` loop from four-of-six to all six; the two rewritten
`registry_test` tests assert the same two properties at the new site.
`grep -rn 'for blank in \[' src/ tests/` → one hit, `src/text.rs:122`, and its
four values are **not** in `DEGENERATE` (`"\u{2060}"`, `"\u{200b}\u{feff}"`,
`"\u{200f}"`, `"\r\n"`) — additive coverage, not a hand copy. `grep -c
"position.starts_with" src/driver/mod.rs` → 0.

**Adjudication of H (deviation D-18-6) — sound; it is a third path that satisfies
the criterion's real conditions, not neither branch.** The plan's criterion has
two substantive requirements: the envelope root is asserted untouched, and no
`set_var` enters a parallel test. Both hold. "With a race-safe injected root" was
the plan's *proposed means*, and reading `envelope_dir(alias)` — the same
resolution `establish_envelope` performs — is strictly stronger than injecting
one, because it observes the real root rather than a substitute. The dedicated
`cr01-envelope-probe` alias removes the collision risk that would otherwise make
the assertion fire on an unrelated condition, and the planted-ordering scratch
probe quoted in the SUMMARY demonstrates the direction is now loud. Row 13
closing by assertion is correct. My only residual is IN-01.

**Adjudication of G.3 (spot-checking the killed executor's tasks 1–2).** I did not
take their green on trust: I re-derived guard nine's scanner by running its own
code against nine fresh plants (results above), read the rewritten limits block
line by line against the controls that back each bullet, and confirmed each SEEN
bullet has a planted arm and each SILENT bullet has either a bounding assertion
(`no_type_alias_hides_a_string_from_guard_nine`) or an explicit "bounded by
nothing in this file." The block is honest about what it measures. **The one
thing it does not enumerate is the bare-`pub`-less opener (WR-01), and the one
thing its allowlist claim overstates is repurposing (WR-02).** Both live in
task 1's code — the least-reviewed code in the round, exactly as flagged.

**What I could not find:** a fifth *seam* where a value becomes an identity
without passing `is_plain_path_component` or `Alias::new`. I traced
`is_plain_path_component`'s six production call sites (`registry.rs:114`,
`journal/mod.rs:355`, `journal/writer.rs:491`, `driver/mod.rs:877` target-phase,
`:1087` run-id, `goal.rs:667` phase token, `envelope/mod.rs:222`) and checked
`record_opt_in`/`clear_opt_in`/`is_opted_in`/`remove_project` (all
membership-checked lookups that create nothing) and `guard_in`/`assert_provenance_in`
(internal, downstream of a judged `Alias`). The seams are complete. **The gap is
not a missing seam — it is that every seam reads a predicate whose class is a
subset of what it claims** (CR-01). Two seams are covered only by the shared
predicate and carry no `LOOK_ALIKE_PAIRS` pin of their own — `--target-phase`
(`driver/mod.rs:877`) and the re-read run id (`journal/writer.rs:491`) — which is
a pin-coverage residual worth closing alongside CR-01's fix, not a behaviour gap.

---

_Reviewed: 2026-08-23T03:20:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
_Round: 6 — diff `0352dda..7260ac8`; round 5 preserved at `5b24022`_
