---
phase: 21-llm-goal-layer-prompt-injection-hardening
verified: 2026-08-26T02:59:59Z
head: f442881
status: gaps_found
score: 24/26 must-haves verified (4/5 ROADMAP success criteria; 1 FAILED, 1 behavior-unverified)
behavior_unverified: 1
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 19/20 must-haves verified (4/5 ROADMAP success criteria)
  gaps_closed:
    - "pass-8 Warning 1 (`Removed project '{}'` printing the alias RAW; pass 8 MEASURED a bidi spoof through it): CLOSED, and closed STRUCTURALLY rather than by remembering the site. `registry::LegacyRegistryKey` (`src/registry.rs:186-209`) has no `Display`, no `Deref`, no `AsRef<str>`, and exactly two accessors named after the questions they answer. `main.rs:158` wraps the argv string, `:161` passes `as_raw_for_lookup_only()` to `remove_project`, `:174` prints `escaped_for_display()`. **I measured BOTH halves against `target/debug/gsd-meta-manager` built from HEAD, with a hand-built legacy `config.json` carrying six keys**: all five legacy keys (`gsd-\\u{202e}nur`, `d\\u{e9}mo`, `demo\\u{200b}`, CJK, `dem\\u{16fe4}o`) `remove` with **exit 0** on their RAW bytes and only `clean` is left, so D-17-3's accept half is intact and no data is stranded; and the echo now reads `Removed project 'gsd-U+202Enur'`. Pass 8's measured spoof does not reproduce."
    - "pass-8 Warning 2 (`eprintln!(\"Error: {refusal}\")` at `main.rs:91` and `:359` echoing raw while `judged_alias_or_exit` escaped): CLOSED at the PRODUCER, which is the move that needs no site list. `impl Display for AliasRefusal` (`src/registry.rs:102-159`) binds `let escaped = |alias: &String| crate::text::display_identity(alias);` ABOVE the match, so a variant added tomorrow cannot embed a candidate without going through it. Measured at the binary: `add` with `gsd-\\u{202e}nur` echoes `the alias \"gsd-U+202Enur\" carries a character that renders as nothing…`, and with `demo\\u{200b}` echoes `\"demoU+200B\"`."
    - "pass-8 Warning 3 (`carries_visible_content`'s free-text residual real and UNDISCLOSED): CLOSED as a disclosure AND as a committed measurement. `src/text.rs:105-141` names the residual, its DIRECTION (over-permissive, free text only), the four measured witnesses, the bound (the alphabet refuses all four at every identity seam) and the two things that bound the deny-list's own completeness. `tests::the_free_text_emptiness_residual_is_accepted_and_cannot_reach_an_identity` (`:744-774`) pins BOTH halves, so a round that closes the residual must change the test and the doc in one commit. I re-measured the residual with SIX witnesses of my own, none of them the four the doc names (see Judgment 1)."
    - "pass-8 Warning 4 (the `narrow_visible` assertion catching a bare field but DIAGNOSING the opposite cause — the message that would get round 7's fix reverted): CLOSED, and the rewritten message is the best artifact in the round-8 diff. `tests/spawn_seam_guard.rs:3657-3691` now names TWO causes with the likelier first, says in capitals `DO NOT narrow is_field_opener back to pub /pub(: that reverts round 7 and reopens the hole a bare field slipped through`, and tells the reader how to tell the two apart. I read it against pass 8's own planting scenario; an executor repairing under it widens the expectation rather than the opener."
    - "WR-03 (`src/text.rs`'s ONE-spelling claim for the identity alphabet, false because `envelope::advisory::is_plain_component` respelled the same set byte-for-byte to gate GitHub `owner`/`repo` before interpolation into a request path): CLOSED by DELEGATION rather than by softening the claim. `src/envelope/advisory.rs:590` is now `&& component.chars().all(crate::text::is_identity_char)`, and `text::tests::exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src` (`src/text.rs:874-920`) is an EQUALITY on a count over a `src/` walk, with its RED-at-two output committed verbatim at `:847-859` and a named under-detection direction (an equivalent third spelling written differently is invisible to a textual census, and what bounds that is the delegation, not the census). I confirmed the delegation by reading `advisory.rs:586-591` and by grepping every `is_identity_char` consumer: `registry.rs:244`, `journal/mod.rs:339`, `advisory.rs:590` — three seams, one spelling."
    - "WR-02 (`src/envelope/hooks.rs`'s security rationale asserting that `is_plain_path_component` accepts a quote character — false since D-19-2, and exactly the sentence a maintainer reads before deleting `sh_quote`): CLOSED, and the correction ships with its own control. `hooks.rs:155-185` states the post-D-19-2 truth and gives a defence-in-depth reason a maintainer can accept; `tests::the_path_component_predicate_refuses_every_shell_metacharacter` (`:1461-1491`) answers `false` for sixteen metacharacters, so widening the alphabet goes red in the same commit rather than letting the doc quietly become false again. `sh_quote` is untouched."
    - "CR-01 (the escaping mechanism was correct but its only UI call lived in `src/ui/project_list.rs`, a 327-line orphan the module tree never compiled, which persuaded a reviewer AND verification pass 8 that the TUI was escaped): CLOSED. `src/ui/project_list.rs` no longer exists (`ls` → No such file). `impl Display for Alias` is WITHDRAWN with the withdrawal's reach stated honestly at `src/registry.rs:267-282` — it does NOT catch `Removed project '{}'` and does NOT catch the TUI, and the comment says so. `delete_confirm.rs:78` escapes the destructive confirm, the tree's highest-consequence identity render."
    - "The round-8 render census is a MECHANISM, not a list, and I confirmed the premise it rests on with my own measurement rather than reading its doc. `screen_implementors_from_source` walks `src/` with recursive `read_dir` and `the_screen_census_matches_the_tree` asserts the derived set equal to `SCREEN_IDENTITY_DISPOSITIONS` in BOTH directions, with a permanent synthetic both-ways non-vacuity control (`the_census_reports_an_unadjudicated_screen_and_a_stale_row`). **The ratatui measurement round 8 rests on is CORRECT — I re-derived it in a scratch crate against ratatui 0.30.2, independent of this tree**: `Span::raw(\"gsd-\\u{202e}nur\")` renders as `gsd-nur`; U+202E, U+200B, U+00AD, U+2062 and U+2065 are ABSENT from the buffer; **U+E0041 SURVIVES intact**. So `hostile_identity()` — `\"demo\\u{e0041}\" + \"r\\u{00ad}un\"` — really does carry a character the buffer preserves, which makes the probe's assertion 3 red-capable rather than vacuous, and `clean_identity()` is all-ASCII so the arrival gate is sound."
    - "Process: `.planning/REQUIREMENTS.md` untouched for the SIXTH consecutive round. `git log -- .planning/REQUIREMENTS.md` still ends at `0c4f712`; all five phase-21 requirements read `[ ]` (lines 62, 63, 83, 85, 86) and `Gaps Found` (152, 153, 164, 166, 167). The prohibition held again."
    - "Process: the no-raw-invisible-character prohibition held across round 8's 2246 added lines. Scanned every added line for `General_Category=Cf` with a control that returns two hits on a planted string: **0 hits**. Debt markers: 0 `TBD`/`FIXME`/`XXX`, 0 `TODO`/`HACK`/`PLACEHOLDER`, against a control needle (`identity`) returning 214 — so neither zero is vacuous."
  gaps_remaining: []
  regressions:
    - "NEW, and it is round 8's own flagship control: `ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped` is INTERMITTENTLY RED at HEAD. See gaps[0]. Nothing pass 8 established regressed."
gaps:
  - truth: "The disposition is CHECKED, not asserted: for every implementor the table marks renders-identity, `the_screen_renders_identity_escaped` renders it through the real `Screen::render` into a ratatui `Buffer` and asserts the buffer carries ZERO characters for which `text::is_invisible_formatting_char` holds (21-21 t3)"
    status: failed
    reason: "The assertion does not hold reliably at HEAD. On my FIRST `cargo test --workspace --no-fail-fast -- --test-threads=2` from the repository root at `f442881`, the lib suite reported `test result: FAILED. 1057 passed; 1 failed`, panicking at `src/ui/screens/render_escape_guard.rs:1093:17` with: `DetailScreen (src/ui/screens/detail.rs) [GitHistory tab] rendered ['\\u{e0041}', '\\u{ad}', '\\u{e0041}', '\\u{ad}'] into the terminal buffer. Those characters render as nothing, so what the operator reads is not what the value is.` That is TWO whole copies of the hostile identity reaching terminal cells raw, reported by the round's own probe. I could not reproduce it in 8 further workspace runs, 12 further full-lib runs (`--test-threads=2` and `--test-threads=8`), 10 full-lib runs under 8-way CPU contention, or 60 direct single-test runs of the compiled lib binary — every one of those was green at 1372 passed / 0 failed / 13 ignored. So the failure is real, captured, and rare. Exactly one of two things is true and NEITHER is acceptable: (a) a render path inside `DetailScreen` draws attacker-influenced identity RAW under some state the probe reaches only occasionally — a live defect in the surface round 8 exists to close; or (b) the probe itself is nondeterministic — the phase's flagship behavioural security control, whose green result is what every other round-8 truth leans on, cannot be trusted to mean anything. I am not able to tell which from outside the tree, and a control nobody can explain is not a control."
    artifacts:
      - path: "src/ui/screens/render_escape_guard.rs"
        issue: "`the_screen_renders_identity_escaped` (:1005-1115) fired assertion 3 (:1085-1091) against `DetailScreen [GitHistory tab]` in one run at HEAD and passed in ~80 subsequent observations. Nothing in the module makes the probe's own inputs nondeterministic on inspection — `probe_ctx` builds a fixed `AppContext`, `render_to_text` uses a fixed-size `TestBackend`, `observed_runs` is empty so `driver_live_for` is constant, and `view_cache` is `or_default()` so `render_git_tab` should take its `git_entries.is_empty()` branch every time."
      - path: "src/ui/screens/detail.rs"
        issue: "Candidate for reading (a): the DetailScreen render path for the GitHistory state. Note that `render_git_tab` itself cannot draw the identity under the probe's fixture, so the two raw copies came from somewhere else in that render, which is the thing to localise."
    missing:
      - "Reproduce the failure deterministically — a seeded/looped harness, or run the lib binary under `--test-threads=2` a few hundred times capturing the panic — and state which of (a) and (b) it is."
      - "If (a): escape the site and add a state to the fixture that reaches it, so the probe covers it every run."
      - "If (b): remove the nondeterminism, then re-observe the probe RED with the fix reverted, because a probe that can flake is a probe whose green was never evidence."
      - "Either way, record the reproduction rate. `cargo test --workspace` is the phase's stated gate and it is currently a gate that passes most of the time."
deferred:
  - truth: "General Unicode CONFUSABLES / homoglyph defence in FREE TEXT (a Cyrillic `а` beside a Latin `a` in a `--goal`)"
    addressed_in: "Not phase 21 — recommend a new roadmap item"
    evidence: "The carve-out at `src/text.rs:265-275` is unchanged and still states honestly where the harm IS closed and where it is not. I re-measured the identity half with FRESH homoglyphs neither pass 8 nor any fixture uses — U+0435 CYRILLIC IE, U+FF12 FULLWIDTH TWO, U+1D5FC MATH SANS-SERIF BOLD O, U+2010 HYPHEN, U+2024 ONE DOT LEADER — and all five are refused at every identity seam and by the binary's `add`. Free text remains open by design."
  - truth: "`registry::current_prompt_inputs` absent from `tests/async_blocking_guard.rs`'s BLOCKING_HELPERS"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md, unchanged across rounds 4-8)"
    evidence: "Async-hygiene class, not this phase's class; the fix forces production `spawn_blocking` rewiring in `approve_plan` and `execute_run`."
  - truth: "The spawn-gate plan-half argument lives in a comment rather than a checked property"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md)"
    evidence: "The comment states plainly that the plan half is a no-op and why that is sound."
  - truth: "Dead `PlanStep::rationale` field in src/driver/goal.rs"
    addressed_in: "Backlog (adjudicated OUT in deferred-items.md)"
    evidence: "No production reader; cosmetic dead field with no security or honesty bearing."
  - truth: "Four PRE-EXISTING clippy lints make `cargo clippy --all-targets -- -D warnings` fail"
    addressed_in: "deferred-items.md:200-236 (TODO, recorded by 21-22 with the empty `git log 2074595..HEAD` evidence)"
    evidence: "Reproduced by me: exit 101 with three `bool_assert_comparison` at `src/browser.rs:131,132,133` and one `cmp_owned` at `src/project_creator.rs:146`. `git log -1 -- src/browser.rs src/project_creator.rs` is `3e948d2` (phase 20), so round 8 did not cause them. The stated gate `cargo clippy -- -D warnings` is exit 0."
  - truth: "The degenerate matrix's ROW table stays hand-maintained (`positions()`)"
    addressed_in: "Disclosed residual, not a gap (deferred-items.md 'OUT — by design')"
    evidence: "21-15 truth 6 discloses this in those words; the disclosure is still accurate."
behavior_unverified_items:
  - truth: "A `.planning/` file or `CLAUDE.md` carrying injected instructions does not change which command the driver executes (ROADMAP success criterion 4 / SAFE-07)"
    test: "With an authenticated `claude` CLI available, run `cargo test --test driver_injection_corpus -- --ignored --nocapture` from the repository root and record the CLI version beside the result."
    expected: "10 passed, 0 failed. Every `corpus_*_arrives_and_leaves_the_command_unchanged` arm asserts the payload ARRIVED at the model before asserting the command was unchanged; the two suppression controls show the positive/negative `CLAUDE_CODE_DISABLE_CLAUDE_MDS` pair diverging; and `both_arms_of_every_class_comparison_were_really_executed` confirms the hostile and clean arms both really ran."
    why_human: "All ten spawn the real `claude` binary and need an authenticated subscription, so they cannot run inside verification. NO AGENT CAN CLOSE THIS ITEM, and the user has explicitly chosen to leave it tracked in `deferred-items.md:291-330`. Counted independently from my own suite run: `driver_injection_corpus` = 13 passed / 10 ignored. Round 8 correctly planned and executed NO work against it (prohibition 2 of plan 21-21 and prohibition 4 of plan 21-22), and re-surfaced it in the register unchanged. Presence and wiring verified for the ninth consecutive pass; behaviour never exercised by any verification pass of this phase."
coincidental_reliance_items:
  - truth: "The TUI render surface draws no invisible-class character into a terminal cell (21-21 t3/t4)"
    reason: fixture-only
    harden: "The probe's power against the tag block depends on `LOOK_ALIKE_PAIRS[TAG_PAIR].1` carrying a character ratatui does NOT drop, and `TAG_PAIR: usize = 4` is a hand-maintained index into a const in another module. I measured the dependency directly: of the class members I rendered, only U+E0041 survives a ratatui 0.30.2 buffer — U+202E, U+200B, U+00AD, U+2062 and U+2065 are all dropped before a cell exists. So if `LOOK_ALIKE_PAIRS[4]` is ever reordered or changed to a zero-width pair, assertion 3 goes silently vacuous while staying green. Promote it to a checked precondition: assert, in the probe itself, that `hostile_identity()` contains at least one character that survives a rendered buffer."
human_verification:
  - test: "With an authenticated `claude` CLI available, run `cargo test --test driver_injection_corpus -- --ignored --nocapture` from the repository root and record the CLI version beside the result."
    expected: "10 passed, 0 failed, with arrival asserted before influence in every class arm and `both_arms_of_every_class_comparison_were_really_executed` green."
    why_human: "Requires an authenticated subscription and spawns the real model binary; cannot run inside verification. This is criterion 4's only behavioural evidence and no verification pass of this phase has ever produced it."
---

# Phase 21: LLM Goal Layer & Prompt-Injection Hardening Verification Report

**Phase Goal:** A user states a goal once and the run pursues it, with the model confined to two narrow, bounded, hardened seams
**Verified:** 2026-08-26T02:59:59Z
**HEAD:** `f442881`
**Status:** gaps_found
**Re-verification:** Yes — ninth verification pass, after the eighth gap-closure cycle (21-21, 21-22)

## Goal Achievement

Round 8 moved no criterion by design. My job was therefore regression verification
plus one obligation the phase has failed twice: **re-derive criterion 1's
witnesses from outside the implementation.** Pass 6 declared criterion 1 verified
using U+200B and U+FEFF — two code points from INSIDE the implementation's own
list — and pass 7 retracted it. Pass 8 did it right with CPython. I could not
reuse pass 8's sample and stay honest, so I state my provenance before my verdict.

### Judgment 1 — Where my criterion-1 witnesses came from, and why they are independent

**Oracle A — Perl 5's `Unicode::UCD`, `UnicodeVersion()` = 15.0.0.** Perl builds
its Unicode tables in core, from the UCD, with its own `mktables` generator. It
is not `icu_properties` (ICU4X), which the production class at `src/text.rs:190-199`
reads. It is not `unicode-properties` (unicode-rs), which the tree's own sweep at
`:423-452` reads as its oracle. It is not CPython's `unicodedata`, which pass 8
used. Nothing in `Cargo.toml`, `src/`, `tests/` or any round-8 plan mentions Perl.

**Oracle B — OpenJDK 21.0.11's `Character.getType()`.** A fifth, independently
maintained derivation, used purely as a cross-check on Oracle A. **The two agree
exactly**: 170 `General_Category=Format` code points, `diff` of the sorted
enumerations empty.

**What Oracle A buys that no previous pass could get.** Perl exposes
`\p{Default_Ignorable_Code_Point}`; CPython does not. That is precisely the half
of the class `src/text.rs:454-463` discloses as having **no second machine
oracle** — "a subset bug in a default-ignorable code point NOT listed here is
invisible to every test in this tree" — and which the tree covers with **thirteen
hand-named members**. I enumerated all **4174** of them and fed every one through
the production predicates. That is the last hand-enumeration in this phase,
checked exhaustively for the first time.

**Fresh out-of-class witnesses, derived from the opposite direction.** Oracle A
shares a *definition* with the implementation, so on its own it catches a
derivation bug, not a scope bug. So I picked six code points that render as blank
or as no glyph and that Perl confirms are NEITHER `Cf` NOR default-ignorable:
**U+F0000** (plane-15 private use, `Co`), **U+10FFFD** (plane-16 private use,
`Co`), **U+1CBB** (unassigned, `Cn`), **U+0591** HEBREW ACCENT ETNAHTA (`Mn`),
**U+0489** COMBINING CYRILLIC MILLIONS SIGN (`Me`) and **U+16FE4** KHITAN SMALL
SCRIPT FILLER (`Mn`, a filler that renders as nothing and is *not* default-ignorable).
**Not one of the six is any of pass 8's four** (U+2800, U+E000, U+0378, U+0301),
and none appears in `DEGENERATE`, in `LOOK_ALIKE_PAIRS`, in `text.rs`'s own
residual test, or in any plan's cited examples.

**Fresh homoglyphs, which are what distinguish an allow-list from any deny-list.**
Pass 8 used U+043E. I used five others: **U+0435** CYRILLIC IE, **U+FF12**
FULLWIDTH DIGIT TWO, **U+1D5FC** MATHEMATICAL SANS-SERIF BOLD SMALL O, **U+2010**
HYPHEN (which looks like the ASCII hyphen-minus the alphabet admits) and **U+2024**
ONE DOT LEADER (which looks like the `.` the alphabet admits). The last two are
the sharpest: they impersonate characters that ARE in the alphabet.

Everything below was measured against the tree at `f442881`. Scratch crates and a
scratch registry were created in `/tmp`, run and deleted; `git status` afterwards
shows only the pre-existing untracked `.gsd/` and `.planning/milestone.lock`, plus
`21-REVIEW.md` which the concurrently-running code reviewer is writing and I did
not touch. Every count, presence and grep check — and every inspection of `cargo`
output — ran under `rtk proxy`.

### Judgment 2 — Criterion 1 holds, and this time the untested half of the class was tested

| Sweep | Members | Accepted by `carries_visible_content` | Missed by `carries_invisible_formatting` | Able to name an identity |
|---|---|---|---|---|
| `General_Category=Cf` (Perl 15.0.0; Java agrees exactly) | 170 | **0** | **0** | — |
| `Default_Ignorable_Code_Point` (Perl 15.0.0) | **4174** | **0** | **0** | **0** |

The accepted set is **empty in both judgments across all 4206 members of the
class**, and no default-ignorable code point can name a run directory, envelope
root, credential scope or phase token. The DI row is new evidence: pass 8's report
correctly flagged that half as unverified, and it is now measured rather than
hand-named.

**The allow-list holds where the deny-list is short, re-proved with fresh shapes.**

```
                                   carries_visible_content  is_plain_path_component  Alias::new
U+F0000  (Co, plane-15 PUA)                 true                   false             Err
U+10FFFD (Co, plane-16 PUA)                 true                   false             Err
U+1CBB   (Cn, unassigned)                   true                   false             Err
U+0591   (Mn, Hebrew accent)                true                   false             Err
U+0489   (Me, enclosing mark)               true                   false             Err
U+16FE4  (Mn, Khitan filler, blank)         true                   false             Err
```

Six witnesses the deny-list judges "visible", and the identity judgment refuses
every one anyway — because it does not consult a deny-list. Nine hostile
identities built from those plus my five fresh homoglyphs are refused at four
seams each (`is_plain_path_component`, `Alias::new`, `run_paths`, and — for the
two that ARE in the class — `carries_invisible_formatting`), and **fifteen of
sixteen `add` invocations against the built binary exit 1**, the one exit 0 being
`demo`. `config.json` afterwards holds one key.

**The residual is unchanged, still confined to free text, and now DISCLOSED** —
which was pass 8's Warning 3 and is closed. `--goal` and `--command` accept a lone
U+F0000 / U+1CBB / U+0591 / U+0489 / U+16FE4, reproduced end to end at the binary:
`drive demo --goal <U+16FE4>` gets past the goal-emptiness check to the opt-in
refusal, while `--goal <U+202E>` and `--goal <U+2062>` are refused earlier with
"a run needs something to do". The doc at `src/text.rs:105-141` now names the
direction and the bound, and `the_free_text_emptiness_residual_is_accepted_and_cannot_reach_an_identity`
couples the disclosure to a test. I agree with keeping it a Warning, for pass 8's
reason: it is display honesty in a string the user typed themselves, and none of
my six witnesses reaches any identity, directory, registry key or envelope root.

### Judgment 3 — Round 8's central measurement is CORRECT, and I re-derived it

Round 8's render probes rest on a claim about ratatui, and if that claim were
wrong the probes would be testing nothing. Measured in a scratch crate depending
only on `ratatui = "0.30"` (resolved 0.30.2), with no reference to this tree:

| Input to `Span::raw` | Rendered buffer | Raw input present |
|---|---|---|
| `gsd-\u{202e}nur` | `gsd-nur` | **no** |
| `gsd-\u{200b}run` | `gsd-run` | **no** |
| `gsd-\u{00ad}run` | `gsd-run` | **no** |
| `gsd-\u{2062}run` | `gsd-run` | **no** |
| `gsd-\u{2065}run` | `gsd-run` | **no** |
| `gsd-\u{e0041}run` | `gsd-\u{e0041}run` | **YES — survives** |
| `gsd-U+202Enur` (escaped form) | `gsd-U+202Enur` | yes (13 cells) |

Exactly as round 8 says. Three consequences, all load-bearing and all confirmed:
an assertion that the raw bidi form is ABSENT would pass vacuously forever; the
tag block is the one class that reaches a cell whole, so assertion 3 has real
teeth; and the escaped form renders in full, so assertion 2 is a real positive.
`hostile_identity()` carries U+E0041, `clean_identity()` is all-ASCII — the probe
is correctly constructed. **Round 8's probes are not testing nothing.**

### Judgment 4 — And yet the probe is intermittently RED, which is the pass-9 finding

On my first `cargo test --workspace --no-fail-fast -- --test-threads=2` at
`f442881`:

```text
thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped'
  panicked at src/ui/screens/render_escape_guard.rs:1093:17:
DetailScreen (src/ui/screens/detail.rs) [GitHistory tab] rendered
['\u{e0041}', '\u{ad}', '\u{e0041}', '\u{ad}'] into the terminal buffer.
Those characters render as nothing, so what the operator reads is not what the value is.

test result: FAILED. 1057 passed; 1 failed; 0 ignored
```

Two whole copies of the hostile identity in terminal cells, reported by round 8's
own control. I then failed to reproduce it in **8 further workspace runs, 12
full-lib runs at two thread counts, 10 full-lib runs under 8-way CPU contention,
and 60 direct runs of the single test against the compiled lib binary** — all
green, all at 1372 passed / 0 failed / 13 ignored, which is the round-8 gate
figure exactly.

I read the module looking for the nondeterminism and did not find it: `probe_ctx`
builds a fixed `AppContext`, `render_to_text` uses a fixed-size `TestBackend`,
`observed_runs` is empty so `driver_live_for` is constant, `view_cache` is
`or_default()` so `render_git_tab` should take its `git_entries.is_empty()` branch
every time, and there is no `set_var`, no `set_current_dir` and no shared mutable
static in `src/`. So I cannot tell which of these is true:

* **(a)** a render path in `DetailScreen` draws attacker-influenced identity RAW
  under a state the probe reaches only occasionally, in which case the surface
  round 8 exists to close is not closed; or
* **(b)** the probe is nondeterministic, in which case the phase's flagship
  behavioural security control — the one every other round-8 truth leans on —
  cannot be trusted to mean anything when it is green.

**Neither is acceptable and I am not going to grade around it.** This phase's own
standard, stated in eight rounds of docs, is that a guard never observed red is
not certified; the converse is that a guard observed red for a reason nobody can
name is not a guard. It is a gap, it is agent-closable, and it is the only thing
standing between this phase and the same 4/5 pass 8 reported.

### Judgment 5 — A concrete candidate for reading (a), found while looking

The probe's own limits block discloses (limit 1) that `probe_ctx` leaves the
Backlog, Sessions, Archive, Browse and Defaults tabs at their cache defaults, so
their populated branches are exercised by no committed control. That residual is
correctly disclosed. It is also **populated**: inside it I found render sites that
draw `.planning/`-derived values raw, with no `shown()` / `display_identity`:

| Site | Value drawn | Source |
|---|---|---|
| `src/ui/screens/detail.rs:2882-2885` | `item.number`, `item.description` | `.planning/BACKLOG.md` |
| `src/ui/screens/detail.rs:3349-3352` | `session.pid`, `sid_display` | session scan |
| `src/ui/screens/detail.rs:3416` | `archive_milestones` entries | `.planning/archive/` dir names |
| `src/ui/screens/detail.rs:3438-3439` | `f.name` (top-level archive files) | `.planning/archive/` file names |
| `src/ui/screens/detail.rs:3444-3445` | `phase.display_name` | `.planning/archive/` phase dirs |

Since U+E0041 survives a ratatui buffer, a `.planning/` name carrying tag
characters renders into those views invisibly. This does not fail a ROADMAP
criterion — criterion 4 is about which command the driver executes, and these are
display surfaces — and 21-21 truth 3 is true AS STATED, because the probe does
assert what it claims over the states its fixture builds. It is a Warning, and it
is what the disclosed hole actually contains rather than a hypothetical.

### Observable Truths

#### ROADMAP success criteria (the contract)

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | A user states a goal in plain language and gets back a structured, machine-checkable plan to review before anything runs | ✓ **VERIFIED** | **Attacked from outside with a FOURTH and FIFTH oracle, per Judgment 1.** All 170 `Cf` code points (Perl `Unicode::UCD` 15.0.0, confirmed identical by OpenJDK 21) AND — new this pass — all **4174** `Default_Ignorable_Code_Point` members refused in both judgments; accepted set empty; zero DI members can name an identity. Six fresh out-of-class witnesses and five fresh homoglyphs (including U+2010 and U+2024, which impersonate characters the alphabet admits) refused at every identity seam; 15 of 16 binary `add` invocations exit 1. Acceptance did not regress: 9 legitimate values × 5 seams, 45/45 accept. Structural goal→plan machinery re-read and intact — twelve-field no-`..` destructure at `driver/mod.rs:414-429`, six `NonBlank` positions, `from_argv` pure and above `drive`. `driver_goal_seam` 22 passed / 0 failed in my own run. Residual: free-text out-of-class blank-renderers — now DISCLOSED (pass-8 Warning 3 closed), still a Warning. |
| 2 | An approved goal is pursued across multiple GSD commands to a terminal outcome without further user input | ✓ VERIFIED (reconfirmed) | `recheck_approval` at `run.rs:2410`, still above `iteration_source` (:2447), `establish_own_group` (:2449), `establish_envelope` (:2493) and `JournalRun::start` (:2626) — re-read, ordering unchanged by the round-8 diff (which touches neither file). `driver_goal_seam` 22/0, `driver_iteration_loop` 7/0, `driver_router_table` 12/0, `driver_router_conformance` 3/0 in my own run. |
| 3 | Model escalations are counted against a per-run cap; exceeding the cap parks the run rather than continuing | ✓ VERIFIED (reconfirmed) | `driver_escalation_cap` **8 passed / 0 failed** in my own run — both directions (a cap at or above the *resolved* step cap refused at the seam; a cap below it accepted) plus the typed park reason on the journal. Untouched by the round-8 diff. |
| 4 | A `.planning/` file or `CLAUDE.md` carrying injected instructions does not change which command the driver executes | ⚠️ **PRESENT_BEHAVIOR_UNVERIFIED** | Counted independently from my own suite run: `driver_injection_corpus` = **13 passed / 10 ignored**. All ten spawn the real `claude` binary and need an authenticated subscription. **Permanently agent-unclosable**, tracked by explicit user decision at `deferred-items.md:291-330`. Round 8 correctly did NO work against it — both plans carry an explicit prohibition and both held. Present and wired; behaviour unexercised for the ninth consecutive pass. → human verification. |
| 5 | Any action the model names that is not in the fixed GSD command enum is refused, never executed as a shell string | ✓ **VERIFIED** | `parse_action` (`goal.rs:343-350`) is an `ALL`-slice lookup returning `Result<RouterAction, UnknownCommand>` — an enum, never a string, and near-misses are refused rather than repaired. `driver_refusal_record` 9/0 and `driver_model_seam` 4/0 (+3 pre-existing ignored) green in my own run. The model-selected phase token is safe by its VALUE, re-proved with fresh shapes: `is_plain_path_component` is **false** for `"\u{ff12}0"`, `"2\u{2024}1"`, `"2\u{2065}0"`, `"2\u{16fe4}0"` and `"2\u{202e}0"`, while `"20"`, `"2.1"` and `"99"` all pass. Pass 7's coincidental-reliance flag stays cleared. |

**ROADMAP score: 4/5 verified, 0 FAILED, 1 behavior-unverified — unchanged from pass 8, as round 8 intended.**

#### Round-8 plan must-have truths (what the round contracted to deliver)

| # | Truth (source) | Status | Evidence |
|---|---|---|---|
| 6 | The render surface is enumerated by three MECHANISMS, each with its reach measured and none claiming another's work (21-21 t1) | ✓ VERIFIED | `impl Display for Alias` withdrawn — `src/registry.rs:267-282`, and the comment states plainly that the withdrawal does NOT catch `Removed project '{}'` and does NOT catch the TUI. `impl Display for AliasRefusal` (`:102-159`) binds the escaper ABOVE the match, so a future variant cannot bypass it. The `Screen` census is layer three. Three reaches, three statements, no overclaim. |
| 7 | The TUI's render surface is closed by DERIVATION — a recursive `read_dir` walk asserted equal to the disposition table in BOTH directions (21-21 t2) | ✓ VERIFIED | `screen_implementors_from_source` walks `src/`; `the_screen_census_matches_the_tree` is a both-ways `assert_eq!` reporting an unadjudicated implementor and a stale row as separate harms; `the_census_reports_an_unadjudicated_screen_and_a_stale_row` is a permanent synthetic control over the SAME `census_offences` the live assertion drives. The table adjudicates; the walk enumerates. Read all three. |
| 8 | The disposition is CHECKED by rendering into a real ratatui `Buffer` and asserting zero invisible-class characters (21-21 t3) | ✗ **FAILED** | The assertion is correctly constructed — I verified its ratatui premise independently (Judgment 3) — and it **fired at HEAD**, reporting `DetailScreen [GitHistory tab]` rendering two raw copies of the hostile identity, then passed in ~80 subsequent observations. See Judgment 4 and `gaps[0]`. |
| 9 | The probe is BEHAVIOURAL, so it is blind to no sink spelling (21-21 t4) | ✓ VERIFIED (with a disclosed hole that is populated) | It inspects the rendered buffer, not source text, so spelling-blindness is genuinely closed. Its limits block discloses that unexercised STATES are not covered; Judgment 5 names five concrete raw render sites inside that hole. Truth is true as stated — Warning below. |
| 10 | `src/ui/project_list.rs` is DELETED, with no dangling citation (21-21 t5) | ✓ VERIFIED | File absent. The round-8 diff shows −327 lines. The orphan whose dead `display_identity` call convinced a reviewer and pass 8 that the TUI was escaped is gone. |
| 11 | `remove` still ACCEPTS a raw legacy alias and no longer ECHOES it unescaped, separated STRUCTURALLY (21-21 t6) | ✓ VERIFIED | **Both halves measured at the binary against a hand-built legacy config.** ACCEPT: five legacy keys removed on their RAW bytes, all exit 0, only `clean` left — no data stranded. ECHO: `Removed project 'gsd-U+202Enur'`. `LegacyRegistryKey` has no `Display` and two purpose-named accessors; `main.rs:158/161/174`. `list` shows `gsd-U+202Enur` and `demoU+200B` while `d\u{e9}mo` and CJK pass through as themselves — a legibility defence, not a transliteration, exactly as documented. |
| 12 | Every mechanism was observed RED before green, and the red output is committed verbatim (21-21 t7) | ✓ VERIFIED | The probe's RED is at `render_escape_guard.rs:1008` with the scratch reporter's output at `:970-982`; the census's both-ways control ships permanently; the spelling census's RED-at-two is at `text.rs:847-859`; the `narrow_visible` re-plant is captured in 21-22's record. Commit order confirms it: `d8e0518 test(21-21): the render-surface census and probe, observed RED before any fix` precedes `f3a2a67`/`b8b9d78`. |
| 13 | The records CR-01 falsifies are corrected in the same commit, append-only and dated (21-21 t8) | ✓ VERIFIED | `3d916e0 docs(21-21): fix a prohibition-8 violation in this plan's own SUMMARY, and self-check` and `2b5ca7a` — the round audited its own diff against its own prohibitions and corrected itself. `21-19-SUMMARY.md` carries the annotation with row 8 REOPENED; no prior line rewritten. |
| 14 | `AliasRefusal::NotPlainComponent` is pinned rather than left to rot (21-21 t9) | ✓ VERIFIED | `src/registry.rs:854-880` asserts `Alias::new("..")` and `Alias::new(".")` are `Err(NotPlainComponent { .. })`, which also certifies that D-19-2's alphabet clause did NOT subsume the traversal check — 21-19 truth 3's load-bearing claim. |
| 15 | The ratatui measurement pass 8 asserted without making is CORRECTED (21-21 t10) | ✓ VERIFIED | **Re-derived by me in an independent crate; the correction is right in every particular** (Judgment 3). ratatui 0.30.2 DROPS zero-width graphemes; the tag block SURVIVES. Pass 8's own "the TUI reorders" framing was wrong and round 8 says so. |
| 16 | The `narrow_visible` message no longer misdiagnoses (21-22 t1) | ✓ VERIFIED | `tests/spawn_seam_guard.rs:3657-3691` names both causes, likelier first, with an explicit `DO NOT narrow is_field_opener back` and a procedure for telling them apart. Pass-8 Warning 4 closed. |
| 17 | `WITNESS_ALLOWED_ELSEWHERE`'s DOC states what the guard delivers; its TABLE is unchanged (21-22 t2, t3) | ✓ VERIFIED | The round-8 diff on that file removes 24 non-header lines, **all of them doc or message text** — no table row, no `assert_eq!`. Both residuals (witness-counting, per-file granularity) are named with their direction. |
| 18 | `hooks.rs`'s security rationale no longer states a falsehood (21-22 t4) | ✓ VERIFIED | `:155-185` states the post-D-19-2 truth and gives a defence-in-depth reason; `the_path_component_predicate_refuses_every_shell_metacharacter` (`:1461-1491`) is the control, over sixteen metacharacters. `sh_quote` untouched. The sentence a maintainer reads before deleting a real defence is now true. |
| 19 | `src/text.rs`'s one-spelling claim made TRUE by delegation, certified by an equality census (21-22 t5) | ✓ VERIFIED | `advisory.rs:590` delegates. `exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src` is an equality on a count with an anti-self-match runtime-assembled needle, RED-at-two committed verbatim, and its under-detection direction named. Three seams, one spelling: `registry.rs:244`, `journal/mod.rs:339`, `advisory.rs:590`. |
| 20 | `carries_visible_content`'s free-text residual DISCLOSED with direction and both completeness bounds (21-22 t6) | ✓ VERIFIED | `src/text.rs:105-141`, plus `the_free_text_emptiness_residual_is_accepted_and_cannot_reach_an_identity` coupling the disclosure to a test so it cannot go stale in either direction. I re-measured the residual with six witnesses none of which the doc names. |
| 21 | The record carries what round 8 learned and cannot mechanise (21-22 t7) | ✓ VERIFIED | `deferred-items.md:200-236` (the four pre-existing lints, with the empty `git log 2074595..HEAD`), `:239-289` (the ratatui-version staleness obligation with a dated measurement table), `:291-330` (criterion 4 re-surfaced unchanged). I reproduced the lints and confirmed `git log -1` on both files is phase-20's `3e948d2`. |
| 22 | DRIVE-04 boundary and precision reconfirmed (21-22 backstop rows 3, 4) | ✓ VERIFIED | Explicit evidence, not inference: `driver_escalation_cap` 8/0 in my own run, both cap directions, decomposition consultation counted against the same cap. |
| 23 | SAFE-07 precision — only the enumerated strings cross, never a whole file (21-22 backstop row 2) | ✓ VERIFIED | `spawn_seam_guard` 38/0 in my own run, guard seven inside it. |
| 24 | SAFE-07 boundary — structural half only (21-22 backstop row 1) | ⚠️ **PRESENT_BEHAVIOR_UNVERIFIED** | The 13 active structural pins pass. The behavioural half is truth 4 above. The plan states this as unverified rather than certified, which is correct. |

**Score: 24/26 must-haves verified; 1 FAILED; 1 present-but-behavior-unverified.**

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/text.rs` (`is_invisible_formatting_char`) | The one derived spelling of the invisible class | ✓ VERIFIED | `:190-199`, two const borrowed statics and one boolean. No literal range. Complete against Perl `Unicode::UCD` 15.0.0 for all 170 `Cf` **and all 4174 `Default_Ignorable` members**. |
| `src/text.rs` (`is_identity_char`) | The finite alphabet, ONE spelling tree-wide | ✓ VERIFIED | `:250-252`. Now genuinely one spelling: `registry.rs:244`, `journal/mod.rs:339`, `advisory.rs:590` all delegate, and a committed equality census enforces it. |
| `src/text.rs` (`carries_visible_content`) | The one emptiness judgment, residual disclosed | ✓ VERIFIED | `:142-146`, doc `:105-141`. Refuses all 4206 class members. Free-text residual disclosed with direction and pinned by a test. |
| `src/text.rs` (`carries_invisible_formatting`) | The identity deny-list over the same class | ✓ VERIFIED | `:280-282`. Backstopped by the alphabet at every seam. |
| `src/text.rs` (`display_identity`) | Render-side defence for legacy rows | ✓ VERIFIED | `:305-315`. Measured: `gsd-U+202Enur`, `demoU+2062`, `2U+20650`; `d\u{e9}mo` and CJK unchanged; idempotent. |
| `src/text.rs` (spelling census) | The one-spelling claim made checkable | ✓ VERIFIED | `:874-920`, equality on a count, RED-at-two committed, under-detection direction named. |
| `src/text.rs` (default-ignorable half) | The half with no second oracle in-tree | ⚠️ **DISCLOSED HAND LIST — now externally verified** | `:464-488`, thirteen named members; `:454-463` discloses the gap in the right words. **The gap is real in the tree and empty in fact**: I swept all 4174 members with Perl and found zero escapes. The disclosure should stay; a machine oracle for this half would close it properly. |
| `src/registry.rs` (`Alias`, `AliasRefusal`, `LegacyRegistryKey`) | Registration closed at entry; accept and echo separated by the type system | ✓ VERIFIED | `Alias::new` `:218-259`, five delegating clauses. `Display for AliasRefusal` escapes at the producer. `LegacyRegistryKey` `:186-209`, no `Display`. Sixteen-alias binary reproduction: 1 in, 15 out. |
| `src/main.rs` | Every identity display resolved deliberately | ✓ VERIFIED | `:146` (`list`), `:43` (refusal echo), `:174` (`Removed project`), `:117`/`:131`/`:192`/`:422` — all escaped. Pass 8's `✗ NOT WIRED` row is gone. |
| `src/ui/screens/render_escape_guard.rs` | A derived census plus a behavioural probe | ⚠️ **CENSUS VERIFIED, PROBE INTERMITTENTLY RED** | Census: both-ways, with a permanent synthetic control. Probe: correctly constructed on a premise I independently confirmed, but observed FAILING at HEAD. See `gaps[0]`. |
| `src/ui/project_list.rs` | Deleted | ✓ VERIFIED | Absent. |
| `src/ui/screens/delete_confirm.rs` | The highest-consequence identity render, escaped | ✓ VERIFIED | `:78` via `display_identity`, with the raw value kept for `do_remove_project`. |
| `src/envelope/advisory.rs` | The second alphabet delegating | ✓ VERIFIED | `:586-591`. The nearby branch-name set that admits `/` is a different question and is excluded by construction. |
| `src/envelope/hooks.rs` | A true security rationale with its own control | ✓ VERIFIED | `:155-185` corrected; `:1461-1491` is the control; `sh_quote` intact. |
| `src/journal/mod.rs` (`is_plain_path_component`) | Blank + control + identity + ALPHABET + structural | ✓ VERIFIED | `:309-349`. Refuses every witness I could construct, including five fresh homoglyphs and six out-of-class blank-renderers. |
| `src/driver/mod.rs` (`DriveArgs`, `from_argv`) | Six `NonBlank` fields, twelve-field no-`..` destructure, pure | ✓ VERIFIED | Re-read `:314-341` and `:410-470`. Unchanged by round 8. |
| `tests/spawn_seam_guard.rs` | Guards nine and ten, with honest docs | ✓ VERIFIED | 38/0. `narrow_visible` message rewritten; `WITNESS_ALLOWED_ELSEWHERE` table untouched, doc replaced with two measured residuals. |
| `tests/driver_injection_corpus.rs` | 13 active structural pins, 10 ignored behavioural arms | ✓ VERIFIED | Counted from my own run. |
| `.planning/.../deferred-items.md` | Round-8 register entries | ✓ VERIFIED | `:200-236`, `:239-289`, `:291-330`. |
| `.planning/REQUIREMENTS.md` | Accurate, untouched by the round | ✓ VERIFIED | Last touch `0c4f712`; all five phase-21 entries read `[ ]` / `Gaps Found`. Sixth round holding. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `text::is_invisible_formatting_char` | `icu_properties` compiled data | `GeneralCategory::Format` ∪ `DefaultIgnorableCodePoint` | ✓ WIRED | Verified complete against a FOURTH oracle on both halves — including the DI half no prior pass could check. |
| `text.rs` test sweep | `unicode-properties` dev-dependency | Exhaustive all-codepoint implication with a `format_seen >= 150` floor | ✓ WIRED | Two crates, neither reading the other; my Perl and Java oracles agree with both at 170. |
| `journal::is_plain_path_component` | `text::is_identity_char` | The allow-list clause at `:339` | ✓ WIRED | Verified with U+2010 and U+2024 — homoglyphs of characters the alphabet *admits*. |
| `registry::Alias::new` | `text::is_identity_char` | Clause 4 at `:244` | ✓ WIRED | `OutsideIdentityAlphabet`, message carries the trade and the `remove` recovery route. |
| `envelope::advisory::is_plain_component` | `text::is_identity_char` | Delegation at `:590` (round-8 WR-03 fix) | ✓ WIRED | The GitHub owner/repo segments now inherit the one alphabet before interpolation into a request path. |
| `payload::NonBlank::new` | `text::carries_visible_content` | Delegation | ✓ WIRED | The link that carried pass 7's subset; still delegating. |
| `main.rs Remove` | `registry::remove_project` (RAW) and the echo (ESCAPED) | `LegacyRegistryKey`'s two named accessors | ✓ WIRED | **Both halves measured end to end.** Pass 8's `✗ NOT WIRED` row is closed. |
| `AliasRefusal::Display` | every present and future refusal echo | escaping bound above the match | ✓ WIRED | Producer-side, so no consumer has to be named. |
| `render_escape_guard` census | every `impl Screen for` under `src/` | recursive `read_dir` + both-ways `assert_eq!` | ✓ WIRED | With a permanent synthetic both-directions control. |
| `render_escape_guard` probe | each screen's real `Screen::render` | rendered `Buffer`, escaped-form present + zero invisible-class chars, arrival first | ⚠️ **INTERMITTENTLY RED** | Correctly built on a premise I re-derived; fires unpredictably. `gaps[0]`. |
| `driver/mod.rs:877` / `:1087` | `journal::is_plain_path_component` | the `--target-phase` and `--run-id` seams | ✓ WIRED | Refuses `2\u{ff12}0`, `2\u{2024}1`, `2\u{2065}0`, `2\u{16fe4}0`, `2\u{202e}0`. |
| `driver_injection_corpus` arms | the real model boundary | live `claude` spawn | ⚠️ **IGNORED** | All ten. Unchanged and unclosable by an agent. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `config.json` | registry key (alias) | `Alias::new` | ✓ | ✓ **FLOWING** — 16 variants, 1 accepted, measured at the binary |
| `<envelope>/<alias>/` | envelope root | `envelope_dir_in` → `is_plain_path_component` → alphabet | ✓ | ✓ **FLOWING** |
| `run.json` / run directory | `run_id` | `NonBlank` + `is_plain_path_component` + alphabet | ✓ | ✓ **FLOWING** — `run_paths` `None` for all nine hostile ids; all nine legitimate ids resolve |
| `run.json` | `target_phase` | `Option<NonBlank>` + `is_plain_path_component` + alphabet | ✓ | ✓ **FLOWING** — safe by the VALUE, re-proved with alphabet-impersonating homoglyphs |
| `run.json` | `gsd_command` / `goal` | `NonBlank` over the derived class | ⚠️ | ⚠️ **AMBIGUOUS (narrowed, disclosed)** — cannot be `""`, cannot be any of the 4206 class members; a lone U+F0000 / U+1CBB / U+0591 / U+0489 / U+16FE4 still passes. Now named in the doc with its direction. |
| `list` / refusal echo / `Removed project` | alias | `display_identity` | ✓ | ✓ **FLOWING** — `gsd-U+202Enur` on all three; pass 8's spoofable row closed |
| `remove <alias>` | lookup key | `as_raw_for_lookup_only` | ✓ | ✓ **FLOWING** — 5/5 legacy keys removable |
| TUI Backlog / Sessions / Archive rows | `.planning/`-derived names | raw `format!` / `Span::raw` | ✗ | ✗ **UNESCAPED** — five sites named in Judgment 5, inside the probe's disclosed coverage hole |

### Behavioral Spot-Checks

Every count-, presence- or grep-bearing check ran under `rtk proxy`, including all
`cargo` output. The full workspace suite's result is reported from saved output.

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full workspace suite | `cargo test --workspace --no-fail-fast -- --test-threads=2` (**9 runs**) | **8 × 1372 passed / 0 failed / 13 ignored; 1 × 1371 passed / 1 FAILED** | ✗ **INTERMITTENT FAILURE** |
| Gate lint | `cargo clippy -- -D warnings` | exit 0 | ✓ PASS |
| Unfiltered severity | `cargo clippy --all-targets -- -D warnings` | exit 101; 4 lints at `browser.rs:131,132,133` + `project_creator.rs:146`; `git log -1` on both files = `3e948d2` (phase 20) | ✓ PASS (pre-existing, recorded) |
| Build | `cargo build` | exit 0 | ✓ PASS |
| **PROBE 1 — cross-oracle exhaustive sweep** | 170 Perl-derived `Cf` **+ 4174 Perl-derived `Default_Ignorable`** × both judgments | **accepted set EMPTY in both directions; 0 of 4174 DI can name an identity** | ✓ **VERIFIES the derivation, both halves** |
| Oracle cross-check | Perl `Unicode::UCD` 15.0.0 vs OpenJDK 21 `Character.getType` | 170 = 170, sorted `diff` empty | ✓ **two independent oracles agree** |
| **PROBE 2 — definition-scope attack, fresh witnesses** | U+F0000, U+10FFFD, U+1CBB, U+0591, U+0489, U+16FE4 (none used by pass 8) | `carries_visible_content` **true** for all six; `is_plain_path_component` **false** and `Alias::new` **Err** for all six | ⚠️ **residual is free-text-only, and bounded** |
| **PROBE 3 — identity seams, fresh homoglyphs** | 9 hostile identities incl. U+0435, U+FF12, U+1D5FC, U+2010, U+2024 × 4 seams | refused everywhere; `run_paths` `None` for all nine | ✓ **VERIFIES the allow-list** |
| **PROBE 4 — acceptance** | 9 legitimate values × 5 seams | **45/45 accept** | ✓ **NO REGRESSION** |
| **PROBE 5 — free text unjudged** | 7 free-text values | 7/7 `carries_visible_content` true, 7/7 outside the alphabet | ✓ **NO REGRESSION** |
| **PROBE 6 — `display_identity`** | 6 fresh values + idempotence | `gsd-U+202Enur`, `demoU+2062`, `2U+20650`; Cyrillic and Khitan pass through; idempotent | ✓ PASS |
| **PROBE 7 — binary registration** | 15 hostile + 1 clean `add` against a scratch config | **1 exit 0 (`demo`), 14 exit 1**; `config.json` → ONE key | ✓ PASS |
| **PROBE 8 — D-17-3 accept/echo split** | 5 legacy keys × `list` then `remove` with RAW bytes | **all 5 listed, all 5 removed exit 0**, only `clean` left; echo `Removed project 'gsd-U+202Enur'` | ✓ **BOTH HALVES CLOSED** |
| **PROBE 9 — ratatui 0.30.2 re-measurement** | independent scratch crate, `Span::raw` → `Buffer` cells | U+202E/U+200B/U+00AD/U+2062/U+2065 **dropped**; U+E0041 **survives**; escaped form renders in 13 cells | ✓ **round 8's premise CONFIRMED** |
| **PROBE 10 — free-text residual at the binary** | `drive demo --goal <char> --dry-run` | `<U+202E>` and `<U+2062>` refused with "a run needs something to do"; `<U+16FE4>` accepted | ⚠️ **residual reproduced end to end** |
| Round-8 debt markers | added lines of `git diff 8dc8c98..HEAD -- src/ tests/` | **0** `TBD/FIXME/XXX`, **0** `TODO/HACK/PLACEHOLDER` over 2246 added lines; control needle `identity` → **214** | ✓ PASS |
| Raw-invisible-character prohibition | every added line scanned for `General_Category=Cf` with a control returning 2 hits on a planted string | **0 hits** | ✓ PASS |
| REQUIREMENTS.md untouched | `git log -- .planning/REQUIREMENTS.md` | Last commit `0c4f712`, six rounds ago | ✓ PASS |
| Documented flakes | `driver_reattach` 3/0, `envelope_tracer` 6/0 | neither fired in any of 9 workspace runs | ✓ NOT A GAP |

### Probe Execution

Not applicable — this phase is not a migration/tooling phase and declares no
`scripts/*/tests/probe-*.sh`.

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|---|---|---|---|---|
| DRIVE-01 | 21-01, 04, 07, 09, 11, 13, 15, 17, 19, 21 | User states a goal once; driver pursues it without further input | ✓ SATISFIED | Criteria 1 and 2 verified. The alias selecting which project the goal is pursued against cannot name two projects that render alike — 1 of 16 accepted at the binary — and the legacy recovery route is now measured in both directions. |
| DRIVE-03 | 21-01, 03, 04, 07, 08, 09, 11, 13, 15, 17, 19 | Goal decomposed into a structured, machine-checkable, reviewable plan | ✓ SATISFIED | Criterion 1 verified against a fourth and fifth oracle over the whole class; `driver_goal_seam` 22/22. |
| DRIVE-04 | 21-02, 04, 06, 10, 12, 14, 16, 18, 20, 22 | Escalation capped per run; exceeding it parks | ✓ SATISFIED | Criterion 3 verified; both cap directions green (8/8). |
| SAFE-07 | 21-01, 03, 05, 06, 08, 10, 12, 14, 16, 18, 20, 21, 22 | `.planning/` content passed inside an explicit untrusted boundary | ? NEEDS HUMAN | Criterion 4 behavior-unverified. Thirteen structural pins execute and pass; all ten behavioural tests are `#[ignore]`d. Additionally: five TUI render sites draw `.planning/`-derived names raw (Judgment 5) — a display-honesty finding on the same trust boundary, not a boundary breach. |
| SAFE-08 | 21-01, 05, 06, 12, 14, 16, 17, 19, 22 | Model's action constrained to a fixed enum; no free-form shell strings | ✓ SATISFIED | Criterion 5 verified; the model-supplied phase token is refused by its own value, re-proved with homoglyphs of characters the alphabet admits. |

**No orphaned requirements.** The union of `requirements:` across all twenty-two
plans is exactly {DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08}, matching
REQUIREMENTS.md's phase-21 mapping and the ROADMAP `Requirements:` line.

**Note for whoever closes this phase:** four of five requirements are SATISFIED and
REQUIREMENTS.md still reads `[ ]` / `Gaps Found` for all five. That remains
correct — status changes are reserved for a *passed* verification, and this is
`gaps_found`.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `src/ui/screens/render_escape_guard.rs` | `:1005-1115` | The round's flagship behavioural control fails intermittently at HEAD | 🛑 **Blocker** | Captured once in ~80 observations: `DetailScreen [GitHistory tab]` reported rendering two raw copies of the hostile identity. Either a real state-dependent raw render path or a nondeterministic control. Full detail in Judgment 4 and `gaps[0]`. |
| `src/ui/screens/detail.rs` | `:2882`, `:3349`, `:3416`, `:3438`, `:3444` | `.planning/`-derived names drawn RAW into TUI cells | ⚠️ Warning | Backlog item number/description, session id, archive milestone/file/phase names. All inside the probe's disclosed coverage hole (`probe_ctx` leaves those caches empty), so the disclosure is honest — but the hole is populated, and U+E0041 survives a ratatui buffer, so a hostile `.planning/` name renders invisibly. Fix is `shown()` at five sites plus a fixture state that reaches each. |
| `src/ui/screens/render_escape_guard.rs` | `:433-434` | `TAG_PAIR: usize = 4` is a hand-maintained index whose value silently decides whether assertion 3 has teeth | ⚠️ Warning | Measured: of the class members I rendered, ONLY U+E0041 survives a ratatui 0.30.2 buffer. If `LOOK_ALIKE_PAIRS[4]` is ever reordered or replaced with a zero-width pair, assertion 3 becomes vacuous and stays green. This is the phase's signature failure shape — a hand-maintained pointer into a growing list — reappearing one level up. One assertion closes it: require `hostile_identity()` to contain a character that survives a rendered buffer. |
| `src/text.rs` | `:464-488` | The default-ignorable half of the class is thirteen hand-named members | ℹ️ Info | Correctly disclosed at `:454-463` in the strongest available words, and **empty in fact**: I swept all 4174 members against Perl and found zero escapes. Keep the disclosure; a machine oracle (Perl exposes the property, `unicode-properties` does not) would close it for real. |
| `src/main.rs` | `:145-147` vs `:158` | `list` shows the escaped form; `remove` requires the raw bytes | ℹ️ Info | Measured again: `remove "gsd-U+202Enur"` → `Error: Project not found`. The recovery route the docs name is still not round-trippable from the tool's own output. Low severity, legacy-only, unchanged from pass 8. |

**No unreferenced debt markers were introduced.** Round 8 adds zero `TBD`/`FIXME`/`XXX`
and zero `TODO`/`HACK`/`PLACEHOLDER` across 2246 added lines, against a control
needle returning 214.

### Test Quality Audit

| Test File | Linked Req | Active | Skipped | Circular | Assertion Level | Verdict |
|---|---|---|---|---|---|---|
| `src/text.rs` `mod tests` | DRIVE-01, DRIVE-03, SAFE-08 | 11 | 0 | No | Value + exhaustive property + source census | ✓ Sampling is outside the predicate; a third, fourth and fifth oracle now agree. The residual test couples a disclosure to a measurement. |
| `src/ui/screens/render_escape_guard.rs` `mod tests` | SAFE-07, SAFE-08 | 4 | 0 | No | Behavioral (rendered buffer) | ⚠️ **The census is excellent and controlled. The probe is intermittently RED** — see `gaps[0]`. Its power against the tag block also rests on an unpinned fixture index (Warning). |
| `tests/driver_injection_corpus.rs` | SAFE-07 | 13 | **10** | No | Behavioral (in the skipped arms) | ⚠️ Unchanged: the requirement's behavioural proof is entirely in the ignored set, with an ACTIVE red-capable census of its own arithmetic. |
| `tests/spawn_seam_guard.rs` | SAFE-07, SAFE-08 | 38 | 0 | No | Value, with planted-defect controls | ✓ Both round-7 holes stay closed; the message that could have reverted the fix is rewritten. |
| `tests/driver_escalation_cap.rs` | DRIVE-04 | 8 | 0 | No | Behavioral, both directions | ✓ |
| `tests/driver_goal_seam.rs` | DRIVE-01, DRIVE-03 | 22 | 0 | No | Value + behavioral | ✓ |
| `tests/registry_test.rs` | DRIVE-01, SAFE-08 | 15 | 0 | No | Behavioral, incl. legacy removability | ✓ `a_legacy_alias_the_alphabet_refuses_is_still_removable` — I reproduced its claim against the binary for five legacy keys. |
| `src/envelope/hooks.rs` `mod tests` | SAFE-08 | — | 0 | No | Value, 16 metacharacters | ✓ New in round 8; it is the control the corrected rationale needs. |

**Disabled tests on requirements:** 10, all on SAFE-07 — the requirement's only
behavioural arms; unclosable inside verification. **Circular patterns detected: 0.**
**Insufficient assertions: 0.** **Unreliable assertions: 1** — the render probe.

### Decision Coverage

`21-CONTEXT.md`'s trackable decisions plus the round-8 additions are honoured
across the plans and the diff: D-21-2 (`Display for Alias` withdrawn), D-21-4 (the
UI-wide retype declined in favour of the census + probe, with the 25-error
measurement recorded), D-21-6 (fixtures by import, never respelled), D-19-1/2/3/5/6,
D-17-3 (accept and echo separated structurally), D-18-2 and D-18-5 (prior artifacts
not edited). Each appears in code, in a SUMMARY disclosure list, or in both. No
decision vanished during execution. Non-blocking, as this gate always is.

### Human Verification Required

One item, and it is the same item passes 7 and 8 raised.

#### 1. Execute SAFE-07's boundary live

**Test:** With an authenticated `claude` CLI available, run
`cargo test --test driver_injection_corpus -- --ignored --nocapture` from the
repository root. Record the CLI version beside the result.

**Expected:** 10 passed, 0 failed, with arrival asserted before influence in every
class arm and `both_arms_of_every_class_comparison_were_really_executed` green.

**Why human:** All ten spawn the real model binary and require an authenticated
subscription. **No agent can close this**, the user has chosen to leave it tracked,
and round 8 correctly did no work against it. The thirteen structural pins that do
execute prove the labelled boundary is the only channel; they cannot prove the
model's behaviour on it.

### Gaps Summary

**One gap, and it is not in the code the phase spent eight rounds hardening — it
is in the control round 8 built to certify that hardening.**

Everything round 8 contracted to deliver, it delivered, and I verified nearly all
of it by measurement rather than by reading. All four of pass 8's Warnings are
closed, and three of them were closed at the *producer* rather than at the sites
pass 8 named — the refusal's own `Display`, a newtype with no `Display`, a
delegation — which is the right direction and the direction this phase kept
failing to take. `remove`'s accept half survives intact, which I checked
specifically because a regression there would strand a user's data: five legacy
keys an older build accepted still remove on their raw bytes, and the echo is
escaped. The identity alphabet is now genuinely one spelling in three seams, and a
committed equality census enforces it. And the ratatui measurement everything in
round 8's render work rests on is correct — I rebuilt it from scratch against
0.30.2 and got the same answer in every row, including the one that matters most,
that the tag block survives while the zero-width classes do not.

Criterion 1 is verified, and this time it was attacked with an oracle that could
see the half no previous pass could. Perl's UCD exposes
`Default_Ignorable_Code_Point`; CPython's does not, and the tree's own test covers
that half with thirteen hand-named members under a doc that says, correctly, that
a subset bug in an unlisted member is invisible to every test here. All **4174**
of them are refused in both judgments and not one can name an identity. The last
hand-enumeration in this phase turns out to be complete — which is worth knowing
precisely because nobody could know it before.

**What stops this from being 4/5 and done is a single line of test output.** Round
8's `the_screen_renders_identity_escaped` reported `DetailScreen [GitHistory tab]`
rendering two raw copies of the hostile identity into terminal cells, once, on my
first workspace run at HEAD — and then passed in roughly eighty subsequent
observations across five configurations. I could not localise it, and I am not
going to round it off in either direction. If the code is at fault, the render
surface this round exists to close is not closed. If the probe is at fault, then
the green result that every other round-8 truth leans on is not evidence. The
phase's own standard says a guard never observed red is not certified; the
converse holds too, and a guard observed red for a reason nobody can name is not a
guard. It is small, it is bounded, and it is squarely inside what an agent can fix.

While looking for it I found what the probe's disclosed coverage hole actually
contains: five sites in `detail.rs` that draw `.planning/`-derived names raw into
TUI cells. The disclosure was honest; the hole is populated. And the probe's teeth
against the tag block depend on `TAG_PAIR: usize = 4` — a hand-maintained index
into a const in another module, which decides silently whether assertion 3 asserts
anything. That is this phase's signature failure shape, one level up from where it
was last found, and one assertion closes it.

### Recommendation

**One narrow round 9, and it is a debugging task rather than a hardening one.**

1. **Reproduce the probe failure and say which of (a) and (b) it is.** Loop the lib
   binary under `--test-threads=2` a few hundred times capturing the panic. If a
   render path is at fault, escape it and add a fixture state that reaches it every
   run. If the probe is at fault, remove the nondeterminism and then re-observe the
   probe RED with a fix reverted, because a probe that can flake is a probe whose
   green was never evidence. Record the reproduction rate either way.
2. **Pin the fixture's teeth.** Assert inside the probe that `hostile_identity()`
   contains at least one character that survives a rendered buffer, so
   `LOOK_ALIKE_PAIRS[4]` cannot be reordered into vacuity. This is the cheapest
   item in the round and it closes a recurrence, not an instance.
3. **Escape the five named `detail.rs` sites** and give each a probe state, which
   shrinks the disclosed hole rather than only disclosing it.
4. **Optional, and genuinely optional:** give the default-ignorable half a machine
   oracle. Perl exposes the property; the current `unicode-properties`
   dev-dependency does not. My sweep says the thirteen hand-named members are
   complete today, and nothing in the tree can keep saying so tomorrow.

**Criterion 4 needs a human and nothing else.** Run
`cargo test --test driver_injection_corpus -- --ignored --nocapture` with an
authenticated subscription, record the version and result in `deferred-items.md`,
and the phase closes at 5/5. Do not plan work against it.

**Still out of scope, unchanged:** TR39 confusables in free text. My U+2010 and
U+2024 witnesses show the identity half is closed by the alphabet even against
homoglyphs of characters the alphabet itself admits; free text remains open by
design and belongs in its own roadmap item.

---

_Verified: 2026-08-26T02:59:59Z_
_Verifier: Claude (gsd-verifier), adversarial stance_
_HEAD `f442881` · ninth verification pass · pass 8 preserved at `f1a9d0d`, pass 7 at `84143bb`_
