---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 30
subsystem: ui
tags: [ratatui, prompt-injection, unicode, untrusted-carrier, sealed-trait, census, disclosure]

requires:
  - phase: 21 (21-27, wave 1)
    provides: "`crate::text::Untrusted` with its six-absence certificate and the three-question accessor doc; `detail.rs` as 21-27 left it"
  - phase: 21 (21-28, wave 1)
    provides: "`src/ui/screens/mod.rs` and `render_escape_guard.rs` as 21-28 left them — `DETAIL_SUB_STATES`, the branch-reached discipline, LIMIT 1's two named residual examples"
  - phase: 21 (21-29, wave 1)
    provides: "`test_support::LOOK_ALIKE_PAIRS` at index 6 (the two-invisible-character fixture) and the `src/ui/` composition census"
provides:
  - "`ui::screens::EditBuffer` — the Defaults edit buffer typed over `crate::text::Untrusted`, so an escape cannot be laundered by a copy"
  - "A persistence round-trip byte-identity assertion against the PRE-TYPE oracle, so the display escape cannot reach the operator's config.json"
  - "`ui::screens::RenderDisposition` — a two-variant crate-private enum making a third disposition value inexpressible, at zero invocation-site cost"
  - "A hand-written-`RenderAdjudicated`-impl census, giving WR-02's convention half a control instead of a claim"
  - "`adjudication_reason` its first two readers, plus the finding that one screen's reason uses a forbidden verdict word"
  - "The `Defaults tab, string edit` probe state — the last of LIMIT 1's two named unprobed states"
  - "The round-10 record: the under-disclosure that made CR-02 and CR-03 findings, a corrected carrier table, a complete ten-row WR/IN triage, and COVERAGE.md"
affects: [21-verification, phase-21-close, render-escape-guard, defaults-tab]

actuals:
  tokens: 23895
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Type-the-buffer, not the render: where an escape is laundered by a round trip, the fix is a type on the buffer, not a second escape call"
    - "One deliberately-named raw take plus a byte-identity round-trip pin, so a display escape cannot reach persisted bytes"
    - "A crate-private enum declared `pub` inside a `pub(crate)` module — the `sealed::Sealed` mechanism — lets a `pub` trait method return it without tripping `private_interfaces`"
    - "Structural census exemption: `$crate` identifies a macro body and cannot go stale the way a path:line exemption does"
    - "A self-cleaning pinned exemption (screen AND token) for a violation in a file outside the plan's fence"

key-files:
  created:
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/COVERAGE.md
  modified:
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/render_escape_guard.rs
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md

key-decisions:
  - "D-21-47: CR-03 closed by TYPING the edit buffer, not by adding shown() at the popup — the defect is a laundering MECHANISM, not one unescaped site"
  - "D-21-48: EditBuffer is a wrapper with a hand-written Default, so Untrusted's trait surface stays exactly where 21-27 certified it"
  - "D-21-49: exactly one raw take, named for persistence, with a byte-identity round-trip pin against the PRE-TYPE oracle"
  - "D-21-50: WR-02's vocabulary claim made TRUE with a crate-private enum; constants keep their names, so all eleven invocation sites are byte-identical"
  - "D-21-51: WR-02's other half narrowed to convention and given a census, not claimed closed"
  - "D-21-52: WR-03 closed by giving the method readers, not by deleting it"
  - "D-21-53: IN-02 uses chars().count(); unicode-width NOT added; the residual recorded"
  - "New, at execution time: the enum is declared `pub` inside a `pub(crate)` module rather than as a bare `pub(crate) enum`, because a `pub` trait method returning a `pub(crate)` type trips `private_interfaces` under -D warnings"
  - "New, at execution time: NormalScreen's forbidden-word violation is DISCLOSED in a self-cleaning pinned exemption rather than fixed, because normal.rs is 21-29's file and outside this plan's fence"

patterns-established:
  - "When a single-site revert does not COMPILE, that compiler error is the strongest available red and is captured instead of a test failure"
  - "A control's own control: assert both directions of a matching rule before trusting the rule"
  - "Report both numbers when a grep's literal reading disagrees with its intent, rather than deleting the evidence that makes it disagree"

requirements-completed: [SAFE-07, SAFE-08, DRIVE-01, DRIVE-03, DRIVE-04]

coverage:
  - id: D1
    description: "CR-03 — the Defaults edit popup cannot render an unescaped value, and the escape can no longer be laundered by a copy: the buffer has a type"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped (Defaults tab, string edit)"
        status: pass
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_defaults_string_edit_state_reaches_the_popup_branch"
        status: pass
    human_judgment: false
  - id: D2
    description: "What the operator types is what is persisted, byte-identical — the fix cannot rewrite the user's config.json with a rendering of itself"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/ui/screens/mod.rs#what_the_operator_types_is_what_is_persisted"
        status: pass
    human_judgment: false
  - id: D3
    description: "EditBuffer's push and pop are character operations, asserted over 2-, 3- and 4-byte characters and a combining mark"
    verification:
      - kind: unit
        ref: "src/ui/screens/mod.rs#the_edit_buffer_pushes_and_pops_whole_characters"
        status: pass
    human_judgment: false
  - id: D4
    description: "WR-02 — a third disposition value is not expressible; Screen is still object-safe and adjudication still mandatory, both re-proven by planting"
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_stays_object_safe_after_the_return_type_change"
        status: pass
      - kind: other
        ref: "compile-failure plant at src/driver/liveness.rs:503 — E0308, quoted verbatim in commit c08693f"
        status: pass
      - kind: other
        ref: "E0277 re-plant at src/driver/liveness.rs:500, quoted verbatim in commit c08693f"
        status: pass
    human_judgment: false
  - id: D5
    description: "WR-02's convention half — a census reporting any hand-written RenderAdjudicated impl outside the macro's definition site, observed red by planting"
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#no_hand_written_render_adjudicated_impl_skips_the_macro"
        status: pass
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_adjudication_census_cannot_report_itself"
        status: pass
    human_judgment: false
  - id: D6
    description: "WR-03 — adjudication_reason has two readers and a control that goes red for an empty reason"
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#every_adjudication_reason_is_non_empty_and_names_values_not_verdicts"
        status: pass
    human_judgment: false
  - id: D7
    description: "IN-02 — the Defaults edit popup is measured in characters, not bytes; no dependency added"
    verification:
      - kind: unit
        ref: "src/ui/screens/mod.rs#the_edit_popup_is_measured_in_characters_not_bytes"
        status: pass
      - kind: other
        ref: "rtk proxy git diff Cargo.toml Cargo.lock — empty"
        status: pass
    human_judgment: false
  - id: D8
    description: "DRIVE-04 boundary and precision, reconfirmed by re-run against an unedited file"
    requirement: "DRIVE-04"
    verification:
      - kind: integration
        ref: "tests/driver_escalation_cap.rs (8 passed / 0 failed, file unedited since b1d0478)"
        status: pass
    human_judgment: false
  - id: D9
    description: "The record DISCLOSES COMPLETELY — the under-disclosure that made CR-02 and CR-03 findings is named as such, all ten WR/IN items have a written disposition, and the three carriers round 10 did not retype are named with direction and promote trigger"
    verification: []
    human_judgment: true
    rationale: "Whether a disclosure reads as an ADMISSION rather than as a softer restatement, and whether naming the round-9 omission in those terms is candid rather than deflecting, is a reading judgement no test asserts. What IS mechanically checkable is asserted and quoted: the diff is append-only with zero deletions, the first 657 lines are byte-identical to a pre-edit snapshot, and the triage table has exactly ten rows. A reviewer must read the prose."
  - id: D10
    description: "ROADMAP criterion 4 re-surfaced verbatim with no work claimed against it, for the third consecutive round"
    verification:
      - kind: other
        ref: "git diff --stat b1d0478..HEAD -- tests/driver_injection_corpus.rs (empty); cargo test --test driver_injection_corpus (13 passed / 0 failed / 10 ignored)"
        status: pass
    human_judgment: true
    rationale: "The MEASUREMENT that the file is unedited and its active arms pass is automated and green. The criterion's own BEHAVIOURAL half is permanently agent-unclosable: its ten #[ignore]d arms spawn the real claude binary and need an authenticated subscription. No agent can close it and this plan does not claim to."

duration: 71 min
completed: 2026-08-27
status: complete
---

# Phase 21 Plan 30: The Laundered Escape, Closed at the Type Summary

**`ProjectViewCache::defaults_text_buffer` becomes `EditBuffer` over `crate::text::Untrusted`, so the Defaults edit popup's `Span` is a compile error until it goes through `shown()` — and the round-10 record names the under-disclosure that turned two live carriers into findings instead of known limitations.**

## Performance

- **Duration:** 71 min
- **Started:** 2026-08-27T21:33Z
- **Completed:** 2026-08-27T22:44Z
- **Tasks:** 3
- **Files modified:** 4 (+1 created)

## Accomplishments

- **CR-03 closed at the TYPE, and the defect is precisely characterised.** It was never an unescaped render: `entry.value` IS escaped at the list render. Pressing Enter copied the same value raw into a plain `String` and the popup one render away drew it raw. **An escape laundered by a round trip.** Escaping at the popup would have fixed this field and left the mechanism for the next one.
- **The one thing this fix could have made worse, closed in the same commit.** `take_raw_for_persistence` is the only raw take, pinned byte-identical against the PRE-TYPE oracle over every `LOOK_ALIKE_PAIRS` member, with a non-vacuity arm. A `U+XXXX` spelling reaching `.planning/config.json` would rewrite the operator's config with a rendering of itself and would pass every escaping assertion in the tree.
- **WR-02's false claim made TRUE in one half and NARROWED in the other**, at **zero invocation-site cost** — all eleven `adjudicate_screen!` sites are byte-identical and the other ten screen files are absent from this plan's diff.
- **WR-03's measurement fired, twice.** One genuine violation found and disclosed; and this plan's own first control was false of a correct implementation (`contains("safe")` matching `SAFE-07`) — the WR-08 shape, caught and fixed in the control rather than the subject.
- **Six REDs**, four of them compiler errors or planted defects, each followed by a clean tree.
- **The record discloses completely**: the round-9 omission named as an under-disclosure, a ten-row triage with zero silent drops, and both numbers reported wherever a grep's literal reading disagreed with its intent.

## Task Commits

1. **Task 1 (tracer): CR-03 — the edit buffer becomes a type** — `5461d19` (feat)
2. **Task 2: WR-02, WR-03, IN-02** — `c08693f` (fix)
3. **Task 3: the round-10 record and COVERAGE.md** — `cf55236` (docs)

Tracer feedback gate: the tracer's `<verify>` was re-run end-to-end after `5461d19` and passed, with `git status --porcelain` clean, before any expansion task began.

## Files Created/Modified

- `src/ui/screens/mod.rs` — `EditBuffer` and its six operations; `defaults_text_buffer` retyped; the `disposition` module and `RenderDisposition`; the two constants re-expressed enum-valued and narrowed to `pub(crate)`; the macro's return type; the sealed doc corrected with its falsified sentence quoted; three new tests.
- `src/ui/screens/detail.rs` — eight compiler-named sites resolved through `EditBuffer`'s named operations; the popup renders `shown()`; `DEFAULTS_EDIT_BRANCH_TOKEN`; `first_string_entry` for the probe; the popup width in characters.
- `src/ui/screens/render_escape_guard.rs` — the `Defaults tab, string edit` state and its arrival row; the branch-reached control; `walked_source_files` extracted; `adjudication_impl_offences` and its two tests; the object-safety re-proof; the adjudication-reason control and `REASON_VERDICT_EXEMPTIONS`; the wildcard arm removed; `screen_reason` wired into assertions 2, 3 and 4; LIMIT 1 rewritten.
- `.planning/phases/.../deferred-items.md` — **+333 lines, 0 deletions.**
- `.planning/phases/.../COVERAGE.md` — created.

## Re-measured gates (every command under `rtk proxy`)

**No number below is inherited from this plan's text, from `21-REVIEW.md`, from `21-VERIFICATION.md` or from the wave-1 SUMMARYs' prose.**

| Gate | Plan said (at `b1d0478`) | Measured at base `7daaa0e` (wave-1 merge) | Measured at `cf55236` |
|---|---|---|---|
| `cargo build` | exit 0 | exit 0 | exit 0 |
| `cargo test --workspace --no-fail-fast` | 1387 / 0 / 13 | 1406 / **3** / 13 (all 3 = documented flakes) | **1417 / 0 / 13**, fully green, no flake this run |
| `cargo clippy -- -D warnings` | exit 0 | exit 0 | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | 4 lints at `browser.rs:131,132,133` | 4 lints, same two kinds, **`browser.rs:155,156,157`**, `project_creator.rs:146` | 4 lints, same two kinds, same two files, same lines |
| `cargo test --test driver_escalation_cap` | 8 / 0 / 0 (re-measure) | — | **8 / 0 / 0**, file unedited |
| `cargo test --test driver_injection_corpus` | 13 / 0 / 10 | — | **13 / 0 / 10**, file unedited |

**Delta on the test total: +8 over the 1409 green wave-1 baseline, fully attributed** — T1 +3 (`what_the_operator_types_is_what_is_persisted`, `the_edit_buffer_pushes_and_pops_whole_characters`, `the_defaults_string_edit_state_reaches_the_popup_branch`); T2 +5 (`no_hand_written_render_adjudicated_impl_skips_the_macro`, `the_adjudication_census_cannot_report_itself`, `the_screen_stays_object_safe_after_the_return_type_change`, `every_adjudication_reason_is_non_empty_and_names_values_not_verdicts`, `the_edit_popup_is_measured_in_characters_not_bytes`). T3 adds none — it is documentation.

**The baseline is 1406/3/13, not 1409/0/13, and that is reported rather than smoothed.** The three baseline failures — `driver_reattach` ×2 and `envelope_tracer` ×1 — occurred **before any file had been edited**. See Issues.

**The four pre-existing clippy lints are untouched and that is MEASURED.** This plan edits neither `src/browser.rs` nor `src/project_creator.rs`; `git diff --stat` names neither. **The plan's line numbers are stale for the third consecutive plan** — `21-28` and `21-29` both re-measured them at `155,156,157`, and so did this plan.

## The six REDs

Every one is quoted verbatim in its task's commit message.

| # | What | Kind | Tree after |
|---|---|---|---|
| 1 | The Defaults edit state, before `EditBuffer` | test panic | fixture kept and shipped — see the note below |
| 2 | Single-site revert of the popup's `shown()` | **compile error** (E0599 + E0277) | `git status --porcelain` empty |
| 3 | A hand-written in-crate impl returning a third disposition value | **compile error** (E0308) | empty |
| 4 | An unadjudicated `impl Screen` (the E0277 re-plant) | **compile error** (E0277) | empty |
| 5 | A planted hand-written `RenderAdjudicated` impl | test panic naming `src/driver/liveness.rs:501` | empty |
| 6 | An emptied `adjudication_reason` in `help.rs` | test panic | empty |

**RED 2 deserves its own sentence.** The plan asked for a failing test from a single-site revert. **What actually happened is stronger: the pre-fix code no longer COMPILES.** With `EditBuffer` in place there is no `&self` route to the raw bytes at all, so the revert cannot even be written:

```text
error[E0599]: no method named `len` found for reference `&EditBuffer` in the current scope
    --> src/ui/screens/detail.rs:4268:48

error[E0277]: the trait bound `Cow<'_, str>: std::convert::From<&EditBuffer>` is not satisfied
    --> src/ui/screens/detail.rs:4281:42
```

That is the property the type was added for, demonstrated rather than argued. Reported as a deviation from the criterion's literal wording rather than passed off as the failing test it asked for.

**On "clean tree after each red", stated precisely.** REDs 2–6 were plant-and-restore and left `git status --porcelain` empty. **RED 1 was produced by a fixture addition that is KEPT and ships in `5461d19`**, so the tree was not empty at that moment — it carried exactly the fixture under test. Saying otherwise would be false.

### RED 1, verbatim — CR-03 live in this tree

```text
thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (2414226) panicked at src/ui/screens/render_escape_guard.rs:2377:17:
DetailScreen (src/ui/screens/detail.rs) [Defaults tab, string edit] rendered ['\u{e0041}'] into the terminal buffer. Those characters render as nothing, so what the operator reads is not what the value is.
```

## `EditBuffer`'s full public surface, as shipped

```rust
pub struct EditBuffer(crate::text::Untrusted);

impl Default for EditBuffer { /* hand-written — D-21-48 */ }

impl EditBuffer {
    pub fn seed_from_untrusted_source(raw: String) -> Self;
    pub fn push_char(&mut self, c: char);
    pub fn pop_char(&mut self) -> Option<char>;
    pub fn clear(&mut self);
    pub fn shown(&self) -> crate::text::Rendered;
    pub fn take_raw_for_persistence(&mut self) -> String;
}
```

**No `Display`, no `AsRef<str>`, no `Into<Cow<'static, str>>`, no `Clone`.** One raw take, named for the one question it answers.

### The compiler named EIGHT sites — exactly the eight the plan predicted

`src/ui/screens/detail.rs` at `:1996`, `:1998` (the seed's two arms), `:2764` (push), `:2769` (pop), `:2792` (`is_empty`), `:2795` (`mem::take`), `:4247` (`len`), `:4260` (`Span::styled`).

**And a difference worth stating in one sentence, because it is information about the retype:** three FURTHER consumers — the `.clear()` calls at `detail.rs:801`, `:1060` and `:2775` — did **not** error, because `EditBuffer` deliberately has a `clear()` of its own. So eleven consumers exist, eight were named by the compiler, and the three the compiler was silent about are the three whose operation carried across unchanged by design.

### The persistence round trip

- **Hostile fixture:** every member of `test_support::LOOK_ALIKE_PAIRS` (7 pairs, including `21-29`'s index-6 two-invisible-character member), used as the seed.
- **Edit sequence:** `push_char('a')`, `push_char('b')`, `pop_char()`, then `take_raw_for_persistence()`.
- **Oracle:** the same edit applied to a plain `String` — the PRE-TYPE formulation, never `EditBuffer` again and never the result itself.
- **Result:** byte-identical for every pair; `persisted` contains no `U+`; the take leaves the buffer empty.
- **The non-vacuity arm asserts** that the seeded buffer's `shown()` form **differs** from its raw form. Without it, byte-identity between two identical strings would pass forever for every implementation.

### The character-operation property

Fixture: `'\u{00e9}'` (2 bytes), `'\u{4e16}'` (3), `'\u{1f600}'` (4) and the combining mark `'\u{0301}'`, each asserted `len_utf8() > 1` first. Seed `"\u{4e16}\u{754c}cfg"`. Push-then-pop returns the character and leaves the buffer byte-identical to its pre-push state; popping a buffer whose LAST character is multibyte returns that whole character; an empty buffer pops to `None`.

### The branch-reached token

`DEFAULTS_EDIT_BRANCH_TOKEN = "(Enter to save, Esc to cancel)"` — the popup's own title suffix, a named constant in `detail.rs` so the probe compares against what the render draws rather than respelling it.

**What the probe reports if `defaults_editing` is left `None`:** `the_defaults_string_edit_state_reaches_the_popup_branch`'s second half renders the same Defaults tab with `defaults_editing = None` and asserts the token is ABSENT. If it were present there, the token would say nothing about which branch ran and the first assertion would prove nothing. Both directions are green.

### The new `DETAIL_TAB_ARRIVAL` row, quoted

> `"Draws `defaults_text_buffer` and `entry.key` into a `Clear`ed `Paragraph` popup that overlays the list. The buffer's bytes are a copy of `entry.value` for a `ConfigValueKind::String` row of the project's `.planning/config.json` — free-form text supplied by whoever wrote that file, taken at the moment the operator pressed Enter on the row. The list underneath keeps drawing `entry.value` for every row, so this state draws the same bytes from two different sources through two different widget families. The index and the seed are derived from `defaults_config` by `detail::first_string_entry`, never spelled."`

It names values and sources and contains neither of the two words its own doc forbids.

### LIMIT 1, old and new

**Replaced (quoted verbatim in the file):**

> *"So the Defaults string-EDIT overlay remains the residual STATE, and the sentence below replaces the rest of it."*

**New, strictly narrower:** it records that the 21-25 wording framed the overlay purely as a COVERAGE gap and that **it was also a CORRECTNESS gap** — naming it only as coverage is what let it stand for two more rounds. **Both of that wording's two concrete examples are now closed and by whom:** `driver_dry_run` by **21-28**, the Defaults string-edit overlay by **21-30**.

**The NEW concrete unexercised state it supplies:** the Defaults tab's **DROPDOWN** overlay, taken when `defaults_editing` is `Some(idx)` at a row whose `dropdown_options` are non-empty. `first_string_entry` deliberately skips those rows. **Under-detection, silent — but narrower than what it replaces, and that is MEASURED, not assumed:** `dropdown_options` returns `ConfigValueKind::Enum(&'static [&'static str])` and `Bool`'s two literals, so its options are authored, and what is unprobed there is the KEY in the title, which the tab already draws escaped one render below.

## WR-02, as shipped

```rust
pub(crate) mod disposition {
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub enum RenderDisposition {
        RendersAttackerInfluencedIdentity,
        RendersNoAttackerInfluencedIdentity,
    }
    impl RenderDisposition {
        pub const fn as_str(self) -> &'static str {
            match self {
                Self::RendersAttackerInfluencedIdentity => "renders_attacker_influenced_identity",
                Self::RendersNoAttackerInfluencedIdentity => "renders_no_attacker_influenced_identity",
            }
        }
    }
}
pub(crate) use disposition::RenderDisposition;

pub(crate) const RENDERS_ATTACKER_INFLUENCED_IDENTITY: RenderDisposition =
    RenderDisposition::RendersAttackerInfluencedIdentity;
pub(crate) const RENDERS_NO_ATTACKER_INFLUENCED_IDENTITY: RenderDisposition =
    RenderDisposition::RendersNoAttackerInfluencedIdentity;
```

**The two snake_case strings are byte-identical to the values the constants held before.** They were `"renders_attacker_influenced_identity"` and `"renders_no_attacker_influenced_identity"`; `as_str()` returns exactly those, so no reason, message or serialized disposition changed.

**Zero invocation sites changed, and the eleven were checked BY NAME:** `normal.rs`, `queue_delete_confirm.rs`, `enqueue.rs`, `driver_inject.rs`, `driver_confirm.rs`, `create_project.rs`, `delete_confirm.rs`, `add_project.rs`, `driver_start.rs`, `help.rs`, `detail.rs`. `git diff HEAD` over the first ten is **empty**. The eleventh, `detail.rs`, IS in this plan's diff (for CR-03 and IN-02) and is in the plan's own `files_modified` — so the criterion's literal wording cannot hold and its INTENT is checked instead: `git diff HEAD -- src/ui/screens/detail.rs | grep -i "adjudicate_screen\|RENDERS_"` is empty, i.e. its invocation site is byte-identical too.

`grep -rc "adjudicate_screen!(" src/ui/screens/` returns **1 per file across 11 files** — equal to its pre-task value.

### The wildcard arm's removal is a STRENGTHENING

Round 9's requirement, quoted from the trait's own doc as it stood: *"One of `RENDERS_ATTACKER_INFLUENCED_IDENTITY` or `RENDERS_NO_ATTACKER_INFLUENCED_IDENTITY`, and nothing else. **The probe `panic!`s on any third value.**"*

A third value is no longer detectable at runtime **because it is no longer writable**. The match is now compiler-checked for exhaustiveness, so adding a third variant would fail to COMPILE — earlier and louder than a panic in one test run. A reader who finds the arm gone must not read it as a weakening; the comment at the removal site says so.

### The sealed doc's falsified sentence

> *"Inside the crate the only route is [`adjudicate_screen`], which is what keeps the disposition vocabulary to the two constants below."*

**Which half is now a type property:** the vocabulary. Two variants, a third inexpressible.
**Which half remains convention:** "the only route". `mod sealed` is `pub(crate)` **by design** — that is the property the same doc argues for one paragraph down — so an in-crate hand-write is possible. Narrowed in the doc, dated, and given a census.

### An execution-time design change, reported

The plan specified a `pub(crate)` enum. A **`pub` trait method returning a `pub(crate)` type trips `private_interfaces`**, which `-D warnings` turns into a failure. The enum is therefore declared `pub` **inside a `pub(crate)` module** — precisely the mechanism `sealed::Sealed` already uses in this same file. The type is genuinely crate-private (nothing outside can name the module path) and the lint does not fire. Same property, existing in-tree idiom, no new prohibition engaged.

### The census's structural exemption

`adjudicate_screen!`'s own body is the one legitimate `impl .. RenderAdjudicated for`. It is recognised by **`$crate`**, valid only inside a macro body — not by a `path:line` that goes stale the first time a line moves, and not by exempting the whole file, which would blind the census to a hand-written impl added there. A **non-vacuity arm asserts exactly one macro-body site exists**: zero would mean the needle matches nothing and the equality is vacuous; more than one would mean a second macro emits adjudications.

## WR-03: the measurement, reported BEFORE the control

All eleven adjudicated screens, read off the constructed instance through `&dyn Screen`:

| Screen | Reason length | Forbidden tokens |
|---|---|---|
| AddProjectScreen | 359 | none |
| CreateProjectScreen | 271 | none |
| DeleteConfirmScreen | 436 | none |
| DetailScreen | 2066 | none |
| DriverConfirmScreen | 297 | none |
| DriverInjectScreen | 200 | none |
| DriverStartScreen | 249 | none |
| EnqueueScreen | 311 | none |
| HelpScreen | 442 | none |
| **NormalScreen** | 1346 | **`escaped`** |
| QueueDeleteConfirmScreen | 197 | none |

**Ten clean, one violation.** `NormalScreen`'s reason:

> *"`row_badge`'s lookup keys off the RAW alias while the cell beside it is **escaped** — the worked example of the split."*

**Not rewritten by this plan, and the reason is a fence rather than a judgement.** `src/ui/screens/normal.rs` is `21-29`'s file, merged in wave 1, and this plan's prohibitions fence the screen files. It is DISCLOSED instead — one dated entry in `REASON_VERDICT_EXEMPTIONS` pinned to that screen **and** that token, which **reports itself STALE on every run once the reason is rewritten**. Direction: under-detection, one screen and one token wide; loud in every other direction. Its repair — name the split by provenance (`render_for_terminal`) rather than by verdict — is recorded in `deferred-items.md` with its owner.

### A second finding: this plan's own first control was wrong

`reason.to_ascii_lowercase().contains("safe")` went RED against `DetailScreen` — for **`SAFE-07`**, a requirement ID. **A control false of a CORRECT implementation**: WR-08's shape, and `21-29` deviation #3's shape, a third time in this phase. The CONTROL was fixed to whole-token matching, and **both directions of the token rule are themselves asserted** inside the test (a bare `escaped` must be reported; `SAFE-07` must not).

### The RED, and the new failure-message shape

```text
HelpScreen (src/ui/screens/help.rs) adjudicated itself with an EMPTY reason. The reason is what a future reader inherits about which values this screen draws and where their bytes come from; empty, the adjudication is a disposition with no argument behind it.
```

Assertions 2, 3 and 4 of `the_screen_renders_identity_escaped` now append:

```text
What this screen CLAIMS to draw (adjudication_reason): {screen_reason}
```

so a red names what the screen claims beside what it actually drew.

## IN-02, measured before and after

Title for the `project_code` key is 45 characters (ASCII, so bytes == chars).

| CJK value | Bytes | `inner_w` BEFORE (`len()`) | `inner_w` AFTER (`chars().count()`) | Cells the text really occupies |
|---|---|---|---|---|
| 12 chars | 36 | 45 | 45 | 24 |
| 30 chars | 90 | **90** (popup_w 94) | **45** (popup_w 49) | 60 |

At 12 characters the title dominates and nothing changes — stated because a table showing only the flattering row would be selective. At 30 characters the popup shrinks from 94 cells to 49.

**The residual, stated with its direction:** 49 is now **under** the 60 cells the text occupies. A character count is not display width — a CJK character takes two cells, a combining mark none. Before, the error was over-sizing for wide scripts; now it is **under-sizing, silent**. Both are cosmetic. `unicode-width` was NOT added, and the check is the strong one: **`rtk proxy git diff Cargo.toml Cargo.lock` is EMPTY** — no dependency of any kind. Recorded in `deferred-items.md` beside the existing IN-02/IN-03 entry, which it extends rather than contradicts.

## The record (Task 3)

**Append-only, PROVEN not asserted:** `git diff` on `deferred-items.md` is **333 insertions, 0 deletions**, and `head -657` of the new file is **byte-identical** to a snapshot taken before the edit.

- **The missing disclosure**, quoting round 9's own table intro verbatim and naming both omitted carriers, in the terms that the selective omission is what made CR-02 and CR-03 findings rather than known limitations.
- **The corrected carrier table** — two carriers leave the set; **three** are named as deliberately not retyped, each with its measurement, its SUMMARY by name, direction and promote trigger. The third, `PromptInput::path`/`digest`, was in neither round 9's table nor `21-28`'s plan; `21-28` surfaced it and asked it be recorded here.
- **The ten-row WR/IN triage.** `grep -cE "^\| (WR-0[1-8]|IN-0[12]) \|"` returns exactly **10**.
- **WR-04 recorded as ALREADY CLOSED with evidence.** `src/main.rs:228-256` re-read at this HEAD: no `.to_string()` workaround, comment corrected. Both commits verified present — `7bf8f6b` fixed the source, **`c9345a1`** corrected the comment. **A correction to the plan's own text:** the plan credits `c9345a1` with fixing both; measured, `c9345a1` corrected only the comment left by `7bf8f6b`. `git diff --stat` does not name `src/main.rs`.
- **WR-07's refutation**, quoted verbatim from `21-VERIFICATION.md` with its toolchain (**rustc 1.97.1**), and what `21-29` fixed instead.
- **Criterion 4 re-surfaced VERBATIM** — truth, test, expected, why_human — with round-10 status MEASURED and no work claimed. Third consecutive round.
- **Both DRIVE-04 backstops re-run**: 8 passed / 0 failed, both cap directions and the precision arm named by test, file unedited.
- **The round-10 self-audit** — see below.
- **`COVERAGE.md`** — quoted in full below.

### The round-10 self-audit, over 5380 added lines (19 commits, 25 files)

| Check | Result | Control |
|---|---|---|
| raw `General_Category=Cf` characters | **0** | the same scanner over a planted `U+200B`/`U+00AD` string returns **2** |
| `TBD`/`FIXME`/`XXX` | **5 substring matches, 0 genuine** | control needle matched **3935** lines |
| `TODO`/`HACK`/`PLACEHOLDER` | **2 substring matches, 0 genuine** | same |

**Both numbers are reported rather than the flattering one.** All seven were inspected: **3 × `XXX`** are `U+XXXX`, this project's own display notation, in doc comments this plan added; **2 × `FIXME` + 2 × `TODO`** are prose in `21-28-SUMMARY.md` and `21-29-SUMMARY.md` stating that no marker was introduced.

`.planning/REQUIREMENTS.md` last touched at **`0c4f712`** — untouched by this plan, eighth consecutive round.

### `COVERAGE.md`, in full

It declares **NO EXTERNAL API INTEGRATION** with the reason stated as required, enumerates what round 10's diff actually changes (subprocess argument construction, render-time escape composition, an edit buffer's type, a trait's return type, a sort comparator, source censuses), records that `git diff Cargo.toml Cargo.lock` is empty, identifies **both** detector signals as false positives (one matching a prior plan's own no-integration declaration, one matching "wraps the real API" about an internal Rust API), and **declines the `claude` CLI subprocess seam as a matrix row** — it is a spawned binary this phase hardens, not an API this round integrates: no client, no base URL, no auth exchanged by this code, no response schema, no vendor SDK. Its behavioural half is tracked as ROADMAP criterion 4 and not claimed here. Full text at `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/COVERAGE.md`.

## Decisions Made

D-21-47 … D-21-53 taken as planned. Two execution-time decisions, both reported above: the enum's `pub`-in-`pub(crate)`-module declaration (forced by `private_interfaces`), and disclosing rather than fixing `NormalScreen`'s reason (forced by the wave fence).

**Reversibility:** D-21-47 and D-21-50 are the two `costly` ratings. Both are semver-breaking on a published crate (`ProjectViewCache` field retype; the two constants narrowing to `pub(crate)`) and both land in the next minor. No `one-way` decision; no on-disk format change; **no change to what is persisted**, which is pinned by test.

## Deviations from Plan

### 1. [Rule 1 — Bug in this plan's own new control] The first WR-03 control was FALSE of a correct implementation

- **Found during:** Task 2, step (f)
- **Issue:** `reason.to_ascii_lowercase().contains("safe")` reported `DetailScreen` for `SAFE-07`, a requirement ID. A control that goes red for a right implementation is worse than no control — the exact defect WR-08 reports and `21-29` deviation #3 hit.
- **Fix:** whole-token matching, with both directions of the token rule asserted inside the test.
- **Verification:** the token check fires for a bare `escaped` and does not for `SAFE-07 and DRIVE-01`; both are committed assertions.
- **Committed in:** `c08693f`

### 2. [Rule 3 — Blocking] The enum could not be a bare `pub(crate) enum`

- **Found during:** Task 2, step (a)
- **Issue:** a `pub` trait method returning a `pub(crate)` type trips `private_interfaces`, which `-D warnings` turns into a failure.
- **Fix:** declared `pub` inside a `pub(crate)` module — `sealed::Sealed`'s existing in-tree mechanism. Same crate-private property, no lint.
- **Verification:** `cargo clippy -- -D warnings` exit 0; `RenderDisposition` is unnameable outside the crate.
- **Committed in:** `c08693f`

### 3. [Rule 2 — Transparency] `NormalScreen`'s forbidden word DISCLOSED rather than rewritten

- **Found during:** Task 2, step (f)
- **Issue:** the plan's step (f) says to rewrite a reason containing a forbidden word. The one found is in `src/ui/screens/normal.rs` — `21-29`'s file, outside this plan's `files_modified` and inside its screen-file fence.
- **Fix:** neither rewritten nor hidden. Pinned to screen AND token in a self-cleaning exemption that reports itself stale, recorded in `deferred-items.md` with its direction and its owner.
- **Verification:** the control still goes red for an emptied reason and for any unexempted screen; the exemption asserts the token has not widened.
- **Committed in:** `c08693f`

### 4. [Rule 2 — Transparency] RED 2 is a compile error, not a failing test

- **Found during:** Task 1, step (h)
- **Issue:** the criterion asks for a verbatim test red from a single-site revert. With `EditBuffer` there is no `&self` route to the raw bytes, so the pre-fix code cannot be written at all.
- **Fix:** the two compiler errors are captured verbatim instead, and the difference is stated rather than glossed.
- **Committed in:** `5461d19`

### 5. [Rule 2 — Transparency] `grep -c "impl Default for Untrusted" src/` returns 1, not 0

- **Found during:** Task 1
- **Issue:** the criterion's literal reading conflicts with this phase's rule that a design argument quote its alternative. The one match is a `///` doc line in `EditBuffer`'s own doc explaining why the wrapper exists.
- **Fix:** neither. **Both numbers reported** rather than deleting the evidence: raw grep = **1** (`mod.rs:673`, a `///` line), executable = **0**. `src/text.rs` is absent from `git diff --stat`, which is the load-bearing check. Same shape as `21-28` deviation #4.

### 6. [Rule 2 — Transparency] `git diff --stat` DOES name one of the eleven screen files

- **Found during:** Task 2, step (a)
- **Issue:** the criterion says the diff names none of the eleven screen files, but `detail.rs` is both one of the eleven and in the plan's own `files_modified`. The criterion cannot hold literally.
- **Fix:** the INTENT is checked instead — `detail.rs`'s `adjudicate_screen!` invocation site is byte-identical, and the other ten files are absent from the diff entirely.

---

**Total deviations:** 6 (1 bug in this plan's own control, 1 blocking, 4 transparency).
**Impact on plan:** No scope creep. Deviation 1 is this plan's own discipline catching a control that would have shipped wrong. Deviations 4, 5 and 6 are criteria whose literal wording cannot hold; each reports both readings rather than picking the flattering one.

## Issues Encountered

**The `driver_reattach` flake, REPORTED and not absorbed, and NOT a regression.** The wave-1 baseline run — **before any file had been edited** — reported 1406 passed / **3** failed / 13 ignored: `driver_reattach` ×2 and `envelope_tracer` ×1. Task 1's and Task 2's workspace runs each failed the same two `driver_reattach` tests and each passed **3/3 on an isolated re-run** (Task 2 needed two isolated re-runs — the first was 2/1). **The final full-suite run at `cf55236` was fully green: 1417 / 0 / 13.** Per the orchestrator's own measurement against the untouched base, the variable is other driver-spawning test binaries running concurrently, because these tests discover runs via a system-wide `/proc` scan that does not stop at the process-group or worktree boundary. Recorded in `deferred-items.md` with its promote condition.

**No auth gates. No architectural decisions required.**

## Known Stubs

None. No hardcoded empty value, placeholder string, TODO or FIXME was introduced; every new type, function and control has a real implementation and was observed red before green.

## Limits carried forward (each with its direction)

1. **The Defaults DROPDOWN overlay is unprobed** — LIMIT 1's new concrete example. **Under-detection, silent**, but narrower than what it replaces: its options are authored `&'static str`s (measured), so what is unprobed is the key in the title, escaped one render below.
2. **`NormalScreen`'s adjudication reason uses a verdict word**, pinned and disclosed. **Under-detection, one screen and one token wide, reported-not-red.**
3. **The Defaults popup width is a character count, not display width.** **Under-sizing for wide scripts, silent.** Cosmetic.
4. **Three carriers on the driver path remain call-held**, not type-held: `InboxMessage::text`, `DryRunPreview::report`, `PromptInput::path`/`digest`. **Under-protection, silent.**
5. **The hand-written-impl census is a source scan.** An impl assembled across more than four physical lines, or reached through a re-export, is invisible to it. **Under-detection, silent** — bounded by the E0277 supertrait bound, which is a type property and catches the case that matters (an unadjudicated screen).

## Broken-windows ledger

`gsd-tools windows append` was not invoked: the items above are disclosed residuals with named follow-ups and stated directions rather than stubs, skipped tests or unrun verifies. This plan adds no `#[ignore]`d test, introduces no debt marker (measured: 0 genuine of 7 substring matches, all identified), and every `<verify>` command in the plan was run and is quoted.

## User Setup Required

None.

## Next Phase Readiness

- **`.planning/REQUIREMENTS.md` is untouched by every commit of this plan.** Requirement status is for the phase-close step to decide from a passed verification.
- **`.planning/STATE.md` and `.planning/ROADMAP.md` are untouched** — this plan ran as a worktree executor; the orchestrator owns those writes.
- **ROADMAP is expected to remain at 4/5** and `COVERAGE.md` and `deferred-items.md` both say so. Criterion 4 is permanently agent-unclosable.
- **One follow-up this plan could NOT discharge, named rather than left implicit:** `21-29`'s F3 asks that `ui::screens::tests::ctx_with_aliases` be widened so an input-echo screen can get a buffer-level two-direction spot-check (its S1). `src/ui/screens/mod.rs` is this plan's file, but the widening alone does **not** close S1 — `mod tests` is itself private, so `pub(crate)` on the function is not enough, and the spot-check would have to live in `src/ui/mod.rs`, which this plan does not own. **Not attempted, and S1 stays open**, rather than making a one-line change that would look like progress and close nothing.
- No file owned by 21-27, 21-28 or 21-29 was edited; the two plant-and-restores in `src/driver/liveness.rs` and `src/ui/screens/help.rs` left no trace (`git status --porcelain` empty after each).

## Self-Check: PASSED

- All five key files exist on disk: `src/ui/screens/mod.rs`, `src/ui/screens/detail.rs`, `src/ui/screens/render_escape_guard.rs`, `deferred-items.md`, `COVERAGE.md`.
- `git log --oneline 7daaa0e..HEAD` returns exactly 3 commits: `5461d19`, `c08693f`, `cf55236`.
- `git status --porcelain` empty before this SUMMARY — no uncommitted work, no plant left behind.
- Every task's `<acceptance_criteria>` re-run. The four that cannot pass on their literal wording (RED 2 as a test; the `impl Default for Untrusted` grep; the eleven-screen-files diff check; the plan's stale clippy line numbers) are each reported with BOTH readings in Deviations rather than silently skipped.
- Plan-level `<verification>` re-run at `cf55236`: build exit 0; workspace **1417 / 0 / 13**; clippy exit 0; `--all-targets` exactly 4 pre-existing lints, same two kinds, same two files; `git diff --stat` names neither `src/text.rs`, `src/main.rs`, `src/ui/roadmap_widget.rs`, `Cargo.toml`, `src/browser.rs` nor `src/project_creator.rs`; `tests/driver_escalation_cap.rs` and `tests/driver_injection_corpus.rs` RUN and unchanged; `deferred-items.md` append-only with 0 deletions.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-27*
