---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 35
subsystem: testing
tags: [rust, argv, cwe-88, proc, wire-format, round-trip, session-detection]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    provides: "`resume_terminal_argv`'s fused `--resume=<id>` producer (21-31) and the shared 28-fixture `hostile_session_ids()` corpus"
provides:
  - "`session_id_in_cmdline(cmdline: &[u8]) -> Option<Untrusted>` — the pure argv→id parse, split out of the `/proc` I/O, accepting BOTH the fused and the split wire forms"
  - "the producer↔consumer round-trip control that spans the pair, observed RED before the fix"
  - "the parser's negative, boundary and leftmost-wins arms, beside the function they test"
  - "the in-place correction of the falsified `no control is added in this file, deliberately` reasoning"
  - "the standing phase-21 `ui.safety-gate` rationale with its falsification condition, measured"
affects: [session detection, Sessions tab resume loop, any future argv builder feeding /proc scraping]

actuals:
  tokens: 10274
  tasks: 4
  commits: 6

tech-stack:
  added: []
  patterns:
    - "Round-trip control: drive the real producer's output through the real consumer rather than re-spelling either side"
    - "Split pure parse from I/O so the parse half is assertable and callable across a module boundary"
    - "Wrap at the innermost point: the pure half returns `Option<Untrusted>`, never `Option<String>`"

key-files:
  created: []
  modified:
    - src/session_detector.rs
    - src/ui/screens/detail.rs
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md

key-decisions:
  - "D-21-66: the CONSUMER learns the fused form; the producer is not un-fused, and both wire forms are accepted"
  - "D-21-67: the pure parse is split out as `pub(crate) fn session_id_in_cmdline`, with the `Untrusted` wrap moved to that innermost point"
  - "D-21-68: the round-trip control lives in `detail.rs`'s test module — ONE new `pub(crate)` symbol, not two"
  - "D-21-69: extraction proven behaviour-preserving FIRST (green at the unchanged 1110), THEN the control observed RED, THEN the fix"
  - "D-21-70: the falsified paragraph is retained verbatim with the correction placed immediately at it"
  - "D-21-71: the phase-21 `ui.safety-gate` rationale becomes a standing record with a stated falsification condition"

patterns-established:
  - "A control that lives on one side of a two-sided property certifies one side"
  - "Corrections go BESIDE what they correct, dated, quoting it verbatim — never replacing it"

requirements-completed: [DRIVE-01, DRIVE-03, DRIVE-04, SAFE-07, SAFE-08]

coverage:
  - id: D1
    description: "The argv this build emits is an argv this build can read back — every fixture of the shared 28-fixture hostile corpus, across 5 terminals, round-trips byte-identically through the real producer and the real consumer"
    requirement: "DRIVE-01"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_argv_this_build_emits_is_an_argv_this_build_can_read_back"
        status: pass
    human_judgment: false
  - id: D2
    description: "Both wire forms are accepted — the FUSED form driven from the real producer, and the SPLIT form constructed explicitly (the shape a hand-typed `claude --resume <id>` produces)"
    requirement: "DRIVE-03"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#a_session_id_survives_the_round_trip_in_both_wire_forms"
        status: pass
    human_judgment: false
  - id: D3
    description: "The parser's negative shapes yield nothing and the leftmost id-bearing element wins deterministically; the pass-through stays non-validating"
    requirement: "DRIVE-04"
    verification:
      - kind: unit
        ref: "src/session_detector.rs#session_id_in_cmdline_reads_both_wire_forms_and_no_other_shape"
        status: pass
    human_judgment: false
  - id: D4
    description: "The producer is untouched: the fused argv that keeps a hostile id from becoming an option of the resumed program is preserved, not weakened"
    requirement: "SAFE-08"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element"
        status: pass
      - kind: other
        ref: "rtk proxy git diff 69f99e2 HEAD -- src/ui/screens/detail.rs (single hunk at 7719 inside mod tests; 0 deletions)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The `Untrusted` wrap survives at the innermost point — exactly one non-comment `from_untrusted_source` in the module"
    requirement: "SAFE-07"
    verification:
      - kind: other
        ref: "rtk proxy grep -vE '^\\s*//' src/session_detector.rs | rtk proxy grep -c from_untrusted_source == 1"
        status: pass
    human_judgment: false
  - id: D6
    description: "The falsified `no control is added in this file, deliberately` reasoning is corrected in place, quoting it verbatim, with the original paragraph left standing and unreworded"
    verification:
      - kind: other
        ref: "whitespace-normalized diff of HEAD 69f99e2:150-154 against the quoted passage — identical; 0 deletion lines in the file's round diff; heading grep returns exactly 1 hit"
        status: pass
    human_judgment: true
    rationale: "Whether the correction adequately states what the reasoning got RIGHT and what it MISSED is a judgment about prose, not a property a test can assert. The mechanical parts (verbatim fidelity, paragraph retained, zero deletions) are verified; the adequacy of the explanation is not."
  - id: D7
    description: "The phase-21 `ui.safety-gate` standing rationale, the seven flagged edge rows, round 12's closure with its residual, criterion 4 re-surfaced verbatim, and the `driver_reattach` re-affirmation are written into `deferred-items.md`, append-only"
    verification:
      - kind: other
        ref: "rtk proxy git diff --numstat -- deferred-items.md == 261 insertions / 0 deletions; criterion 4 diffs byte-identical against ROADMAP.md:450; gsd-tools check ui-plan-gate 21 == block: false; visual-design grep == 0"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-08-28
status: complete
---

# Phase 21 Plan 35: The argv this build emits is an argv this build can read back — Summary

**Round 11's correct security fix silently deleted resume detection: the producer fused `--resume=<id>` into one argv element while the consumer still scanned `windows(2)` for a standalone `--resume`. The consumer now reads both wire forms, and a producer↔consumer round-trip control — observed RED against the unfixed parser before the fix landed — spans the pair so the two can never drift silently again.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-08-28T19:59Z (approx; first task commit 20:05:56)
- **Completed:** 2026-08-28T20:24Z
- **Tasks:** 4 (Task 1 is a TDD tracer producing 3 commits)
- **Files modified:** 3

## Accomplishments

- **The gap is closed at the consumer, and only there.** `session_id_in_cmdline` accepts the fused single element AND the split two-element window. The producer was not touched — round 11 measured that a `--` end-of-options separator closes the injection and deletes the resume in the same stroke, so fusion is correct and stays.
- **The control was observed RED first**, against a parser proven behaviour-identical to HEAD by an intermediate green at the unchanged 1110. That ordering is what makes the red attributable to the wire-format defect rather than to the extraction.
- **Both wire forms are asserted, not inferred.** The split arm is constructed explicitly because no producer in this build emits it any more — but every session not started by this TUI still arrives in it.
- **The falsified reasoning is corrected at the sentence and left standing**, with what it got right (the security property is a sink property) and what it missed (the functional property is a property of the pair) both named.
- **Two decisions this phase kept re-deriving are now written down once**, dated, with a falsification condition on the one that can expire — and that condition was *measured*, not assumed.

## Task Commits

1. **Task 1(a): extract the parse, behaviour-preserving** — `246c973` (refactor)
2. **Task 1(b): the round-trip control, OBSERVED RED** — `6cca4ca` (test)
3. **Task 1(c/d): teach the consumer the fused form; fence the producer** — `68a9ff2` (fix)
4. **Task 2: both wire forms + the parser's boundaries** — `4a47b0f` (test)
5. **Task 3: correct the falsified reasoning in place** — `fbc0cc7` (docs)
6. **Task 4: the standing records and the round's gate** — `f8ca0bd` (docs)

## The RED-then-GREEN sequence, in order, with the numbers quoted

This is the heart of the round, so it is reported as measurements rather than as claims. Every figure is from `rtk proxy` (the hook-rewritten path strips `test result:` lines and would answer vacuously).

### (1) After the extraction ALONE — the unchanged base

```
test result: ok. 1110 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.08s
```

Exactly the 1110 base. The extraction moved logic without changing it, so the next step's red cannot be blamed on the move (D-21-69).

### (2) After adding the control, BEFORE the fix — RED, verbatim

```
thread 'ui::screens::detail::tests::the_argv_this_build_emits_is_an_argv_this_build_can_read_back' (3342701) panicked at src/ui/screens/detail.rs:7777:17:
assertion `left == right` failed: the session id "demo\u{200b}" was emitted by this build and this build could not read it back. Emitted argv: ["-e", "claude", "--resume=demo\u{200b}"]; wire bytes: [99, 108, 97, 117, 100, 101, 0, 45, 45, 114, 101, 115, 117, 109, 101, 61, 100, 101, 109, 111, 226, 128, 139, 0]. The producer FUSES the id to its option name in one element (`--resume=<id>`) while the consumer scans for a STANDALONE `--resume` element and takes the one after it — the two modules spell the same option name independently, across a module boundary, and they have drifted. [...]
  left: None
 right: Some("demo\u{200b}")

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1110 filtered out; finished in 0.00s
```

The failing fixture is `"demo\u{200b}"`, the first imported `LOOK_ALIKE_PAIRS` hostile value. **`left: None` is the defect itself**, not a symptom: that `None` is what a Sessions-tab row gets for a session this TUI resumed. The control was committed in this red state (`6cca4ca`).

### (3) After the fix — GREEN

```
test ui::screens::detail::tests::the_argv_this_build_emits_is_an_argv_this_build_can_read_back ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1110 filtered out; finished in 0.00s

test result: ok. 1111 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.08s
```

### (4) After Task 2

```
test result: ok. 1113 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 5.07s
```

**1113 = 1110 base + 1 + 2.** Every delta attributed by name:

1. `ui::screens::detail::tests::the_argv_this_build_emits_is_an_argv_this_build_can_read_back` (Task 1)
2. `ui::screens::detail::tests::a_session_id_survives_the_round_trip_in_both_wire_forms` (Task 2)
3. `session_detector::tests::session_id_in_cmdline_reads_both_wire_forms_and_no_other_shape` (Task 2)

## The two signatures

```rust
fn read_session_id(pid: u32) -> Option<Untrusted> {
    let cmdline = std::fs::read(format!("/proc/{}/cmdline", pid)).ok()?;
    session_id_in_cmdline(&cmdline)
}

pub(crate) fn session_id_in_cmdline(cmdline: &[u8]) -> Option<Untrusted> {
```

`read_session_id` keeps its name, signature and private visibility, and is now the `/proc` read plus a call.

## Every `<behavior>` row, and the arm that asserts it

| # | Behaviour | Test | Arm |
|---|---|---|---|
| 1 | Round trip, FUSED, 28 fixtures × 5 terminals, byte-identical | `a_session_id_survives_the_round_trip_in_both_wire_forms` (and `the_argv_this_build_emits_is_an_argv_this_build_can_read_back`) | `proc_cmdline_encoding(&resume_terminal_argv(term, &sid))` |
| 2 | Round trip, SPLIT, 28 fixtures, byte-identical | `a_session_id_survives_the_round_trip_in_both_wire_forms` | `nul_join_cmdline(&["claude", "--resume", raw.as_str()])` |
| 3 | Empty fused suffix yields no id **and scanning continues** | `session_id_in_cmdline_reads_both_wire_forms_and_no_other_shape` | `["claude","--resume="]` → `None`; `["claude","--resume=","--resume=later"]` → `Some("later")` |
| 4 | Whitespace-only fused suffix yields no id | same | `["claude","--resume=   "]` → `None`; and the continue-scanning pair |
| 5 | Bare option name as FINAL element yields no id, no panic | same | `["claude","--resume"]` → `None`, **and** raw `b"claude\0--resume"` (no trailing NUL) → `None` |
| 6 | No resume option at all yields no id | same | `["claude","--print","hello"]` → `None` |
| 7 | Option name that merely BEGINS with the spelling yields no id | same | `["claude","--resumes","abc"]` and `["claude","--resume-session","abc"]` → `None` |
| 8 | Leftmost wins, both orders | same | split-then-fused → `"first"`; fused-then-split → `"first"` |
| 9 | Every fixture NUL-free before joining | `nul_join_cmdline` precondition (shared by both arms) | assertion below |

All nine rows are asserted. None is unasserted.

Row 5 is worth naming: the first arm goes through the kernel's trailing NUL (so the successor is the empty final element), while the second is raw bytes *without* a trailing NUL, which is the only shape that genuinely reaches the `args.get(index + 1) == None` branch. Without the second, that branch would be unexercised.

### The split arm, constructed explicitly (Task 2 acceptance criterion)

```rust
            // --- SPLIT: constructed explicitly, not inferred ------------------
            // Program name, bare option name, then the fixture: the two-element
            // window a hand-typed `claude --resume <id>` puts on the wire.
            let wire = nul_join_cmdline(&["claude", "--resume", raw.as_str()]);
            let read_back = crate::session_detector::session_id_in_cmdline(&wire);
```

### The NUL-free precondition's failure message

```
"the argv element {element:?} carries a NUL byte. NUL is the SEPARATOR of this
 encoding, so the element would be split in two and everything after the NUL
 would be read as a separate argument: the assertion below would then be
 checking a value this test invented rather than one that survived the wire."
```

### The round-trip tests' shared class doc, in full

```
    // ======================================================================
    // THE ROUND-TRIP CONTROLS — and why a one-sided control missed this class
    //
    // The property these two tests assert is a property of the PAIR, not of
    // either function: *what this build emits, this build must be able to read
    // back.* Neither `resume_terminal_argv` nor `session_id_in_cmdline` can be
    // wrong on its own here — each is individually correct — and that is
    // precisely why no control over either one could see the defect.
    //
    // What happened: round 11 FUSED the id to its option name at the producer
    // (`--resume=<id>`), correctly, to close CWE-88. That changed the WIRE
    // FORMAT the consumer parses. Every control in this repository lived on one
    // side or the other — the producer's controls asserted the argv's shape,
    // and the consumer had no parser control at all — so NO CONTROL SPANNED
    // BOTH. A build that had silently lost its resume detection passed the
    // whole gate green, for a full verification pass.
    //
    // The two modules spell the same option name INDEPENDENTLY:
    // `RESUME_OPTION_FUSED_PREFIX` here, and `RESUME_OPTION_NAME` /
    // `RESUME_OPTION_FUSED_PREFIX` in `crate::session_detector`. Nothing in the
    // type system couples them and nothing ever will — they are two byte
    // literals in two modules. These tests are the ONLY thing coupling them
    // (T-21-35-02), which is why they consume the real producer's output rather
    // than re-spelling the option name a third time.
    //
    // A control exercising only one side will miss the next instance of this.
    // ======================================================================
```

## The three call sites (real producer, real consumer, shared corpus)

```rust
                let argv = resume_terminal_argv(term, &sid);
                let read_back = crate::session_detector::session_id_in_cmdline(&cmdline);
            for raw in hostile_session_ids() {
```

No fixture is declared by the tests; the corpus is imported (D-21-6), and its size is asserted at 28 so a shrunken corpus could not pass silently.

## The parser stays NON-VALIDATING — stated in words

**The only NEW condition is on the ARGUMENT'S SHAPE** — which of the two wire forms an element is, if either. **The only condition on its VALUE remains non-emptiness after `trim()`, unchanged from HEAD**, and an empty value still does not stop the scan (row 3 asserts exactly that). Nothing inspects the id's first byte, character set or length.

That restraint is load-bearing, not stylistic: `claude` resumes by session **title** as well as by UUID, so a rule tight enough to refuse `-h` would also refuse legitimate title resumes — and those sessions would vanish from the Sessions tab with no message to the operator. That is under-detection in the SILENT direction, which is the direction a user cannot see.

## The producer fence

```
$ rtk proxy git diff 69f99e2 HEAD -- src/ui/screens/detail.rs | grep -E '^(@@|---|\+\+\+|-)'
--- a/src/ui/screens/detail.rs
+++ b/src/ui/screens/detail.rs
@@ -7719,6 +7719,84 @@ mod tests {
[after Task 2, a second hunk in the same test module]

$ rtk proxy git diff --stat 69f99e2 HEAD -- src/ui/screens/detail.rs
 src/ui/screens/detail.rs | 181 +++++++++++++++++++++++++++++
 1 file changed, 181 insertions(+)
```

**181 insertions, ZERO deletions, every hunk inside `mod tests`.** `resume_terminal_argv` is at `:703` and `RESUME_OPTION_FUSED_PREFIX` at `:585` — both far outside every hunk. The producer gained a test and nothing else.

Both existing sink controls re-run unchanged and green:

```
test ui::screens::detail::tests::the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1110 filtered out

test ui::screens::detail::tests::the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1110 filtered out
```

## The acceptance greps

```
$ rtk proxy grep -n "session_id_in_cmdline" src/session_detector.rs src/ui/screens/detail.rs
src/session_detector.rs:157:    session_id_in_cmdline(&cmdline)
src/session_detector.rs:209:pub(crate) fn session_id_in_cmdline(cmdline: &[u8]) -> Option<Untrusted> {
src/ui/screens/detail.rs:7759:    /// the REAL consumer, `crate::session_detector::session_id_in_cmdline`.
src/ui/screens/detail.rs:7774:                let read_back = crate::session_detector::session_id_in_cmdline(&cmdline);
```

Every hit classified: `:157` the `read_session_id` call · `:209` the declaration · `:7759` a doc mention · `:7774` the test call. (Taken before Task 2/3 added lines; the classification is unchanged, the line numbers shift.)

```
$ rtk proxy grep -vE '^\s*//' src/session_detector.rs | rtk proxy grep -c "from_untrusted_source"
1
```

Exactly **1**. The comment-filtered form is required because the doc block discusses the wrap in prose, and an unfiltered count would be answered by the documentation rather than by the code.

## The correction — verbatim fidelity, proven

```
$ rtk proxy grep -n "No control is added in this file, deliberately" src/session_detector.rs
199:/// # No control is added in this file, deliberately
```

Exactly one hit: the original heading is **retained**, not deleted or renamed.

```
$ rtk proxy git diff 69f99e2 HEAD -- src/session_detector.rs | grep '^-[^-]'
-    for window in args.windows(2) {
-        if window[0] == b"--resume" {
-            let val = String::from_utf8_lossy(window[1]);
-                // The ONE place a session id enters this build, so the ONE
-                // place it is wrapped.
```

Five deletions across the whole round, **all of them the parser replacement's own executable and inline-comment lines**. **Zero `///` doc lines were deleted anywhere** — every line of the falsified passage appears as *context*.

**The quotation, checked mechanically rather than by eye.** HEAD `69f99e2:150-154` and the correction's *The reasoning this corrects, verbatim:* line, both whitespace-normalized:

```
$ diff head.norm quoted.norm
Files are identical
```

The only difference in the raw text is line wrapping, because the quote re-wraps after its bold lead-in. Word for word it is byte-identical.

**The correction states all four things:**

1. *"The security property genuinely IS a property of the sink. […] Adding a second *security* assertion here would still certify a claim this function does not make, and would still let the class be counted as closed twice. That half of the reasoning survives this round intact."*
2. *"The **functional** property is a property of the **pair**, not of either side: *what this build emits, this build must be able to read back.*"*
3. *"Every control lived on one side or the other […] so **no control spanned both**. […] a build that had silently lost its resume detection passed the whole gate green for a full verification pass."*
4. *"What has been added is **wire-format parsing and its functional control**, never value validation. The validator declined above stays declined […] The new tests assert which SHAPES carry an id; they assert nothing whatever about which VALUES are acceptable."*

**The wrap sentence:** *"This function remains the one place a session id **enters this build from another process**; [`session_id_in_cmdline`], the pure half split out of it, is the one place the id is **wrapped** — which is why that half returns `Option<Untrusted>` and never `Option<String>`. That is a restatement of where the boundary sits, not a loosening of it."*

## The round's gate, measured on the delivered tree

| Gate | Base at `98610bf` | Round 12 | Verdict |
|---|---|---|---|
| `cargo build` | exit 0 | **exit 0** | unchanged |
| `cargo test --lib` | 1110 / 0 / 0 | **1113 / 0 / 0** | +3, all attributed by name |
| `cargo test --workspace --no-fail-fast` | — | **0 failed binaries** | see flake note |
| `cargo clippy -- -D warnings` | exit 0 | **exit 0** | unchanged |
| `cargo clippy --all-targets -- -D warnings` | fails, 4 pre-existing | **fails, exactly 4** | unchanged |
| `cargo doc --no-deps` | exit 0 | **exit 0** | unchanged |
| `git diff -- Cargo.toml Cargo.lock` | — | **empty** | no dependency change |

**The four `--all-targets` lints, named as PRE-EXISTING.** Three would be a failure of this round exactly as five would be:

```
      1 error: this creates an owned instance just for comparison
      3 error: used `assert_eq!` with a literal bool
   --> src/browser.rs:155:9
   --> src/browser.rs:156:9
   --> src/browser.rs:157:9
   --> src/project_creator.rs:146:27
```

Exactly four, same two kinds, same two files. Neither file is in this round's `files_modified`; neither was opened.

**`cargo doc --no-deps` exits 0.** The one rustdoc warning naming `session_detector.rs` is the pre-existing *"public documentation for `ClaudeSession` links to private item `read_session_id`"* at line 9, untouched by this round. The correction added no new one, and both directions of the `resume_terminal_argv` ↔ `read_session_id` cross-reference still resolve.

**The round's whole diff:**

```
 .../deferred-items.md    | 261 ++++++++++++++++++++
 src/session_detector.rs  | 274 ++++++++++++++++++++-
 src/ui/screens/detail.rs | 181 ++++++++++++++
 3 files changed, 711 insertions(+), 5 deletions(-)
```

Exactly the three files this plan owns. `git diff --name-only 69f99e2 HEAD -- .planning/REQUIREMENTS.md tests/driver_injection_corpus.rs Cargo.toml Cargo.lock` returns **empty**. `.gsd/` and `.planning/milestone.lock` are not present in this worktree and were not staged.

## The UI-gate falsification condition, MEASURED

```
$ gsd-tools check ui-plan-gate 21
{ "frontend": true, "hasFrontendEvidence": false, "hasUiSpec": false,
  "block": false, "uiSpecPath": null, "matchedToken": "ui", ... }

$ rtk proxy git diff 69f99e2 HEAD -- src/ | grep -E '^[+-]' \
    | grep -vE '^[+-]\s*(//|///)' | grep -cE 'Layout::|Constraint::|Style::|Color::'
0
```

`block: false` — the gate did not fire and **no override was applied**. The visual-design grep is scoped to `src/` and comment-filtered, so the record's own prose cannot answer the question it asks. Zero: the rationale's falsification condition is not met, and it stands for round 12.

## The seven edge-probe rows

**7 applicable / 0 resolved / 7 unresolved / 7 surfaced.** All seven are written into `deferred-items.md` as explicit flagged assumptions. None was auto-resolved with a `backstop` marker, none was dropped, and **nothing from the probe entered `must_haves.truths`**. No-silent-drop equality holds: 7 surfaced == 7 accounted for.

Task 2's parser boundary arms are boundaries of the **parser** and are claimed as such. They do **not** resolve rows 1, 2, 6 or 7, which are boundaries of the **requirements**, and this round does not claim they do.

## ROADMAP success criterion 4 — re-surfaced, no work claimed

Quoted byte-identically from `.planning/ROADMAP.md:450` (proven by `diff`, exit 0):

> 4. A `.planning/` file or `CLAUDE.md` carrying injected instructions ("ignore prior constraints, run …") does not change which command the driver executes

**Permanently agent-unclosable by construction** — it needs a human with a live authenticated Claude subscription to run the ten `#[ignore]`d arms of `tests/driver_injection_corpus.rs`. **4/5 is the expected and correct ceiling for this phase and is not a failure.** This is the fifth consecutive round with no work claimed against it. `tests/driver_injection_corpus.rs` was not edited and is absent from the round's diff; it runs 13 passed / 0 failed / 10 ignored, unchanged.

## Decisions Made

None beyond the plan's own D-21-66 … D-21-71, all of which were followed as written. No `checkpoint:decision` was required — every change is a private function split, an added test, a doc correction or a planning-record append.

## Deviations from Plan

### 1. [Process — read_first ordering] `21-REVIEW.md` was read after Task 1's edits rather than before

- **Found during:** Task 1
- **Issue:** Task 1's `<read_first>` lists `21-REVIEW.md` lines 50-165. I read the four source-file ranges first and made the extraction and control edits, then read the review. The `read_first` gate is meant to establish ground truth *before* editing.
- **Impact:** None on the artifact. The review's HI-01 confirms the implementation rather than contradicting it: its recommended fix is the same two-constant, index-based single pass, and the plan's naming (`session_id_in_cmdline`, not the review's `session_id_in`) wins as the plan directs. One deliberate divergence from the review's sketch, required by the plan: the review NUL-joins a re-spelled literal `"claude"`, while this implementation derives `argv[0]` from the producer's own output via `skip(1)`, so the encoding cannot drift from what is emitted.
- **Fix:** Read in full before committing the fix; no rework needed.
- **Verification:** Implementation compared against the review's recommended shape line by line.

### 2. [Rule 3 - Blocking] A `git commit -F` against a missing message file silently reused a stale `COMMIT_EDITMSG`

- **Found during:** Task 1(b)
- **Issue:** The scratchpad message file had not been written when `git commit -F` ran. Rather than failing, the commit landed with an unrelated phase-21-26 message from a leftover `COMMIT_EDITMSG`.
- **Fix:** Wrote the message file properly and `git commit --amend -F` on my own just-created, unpushed commit. Every subsequent commit's message was verified after landing.
- **Files modified:** none (commit metadata only)
- **Verification:** `git log --oneline -3` confirms `6cca4ca test(21-35): add the producer/consumer round-trip control, OBSERVED RED`.
- **Committed in:** `6cca4ca`

---

**Total deviations:** 2 (1 process/ordering, 1 blocking tooling issue).
**Impact on plan:** No scope creep, no artifact change. Neither deviation altered what shipped.

## Issues Encountered

**None affecting the deliverable.** Two things worth recording:

- **The `driver_reattach` flake did NOT fire this round.** `test result: ok. 3 passed; 0 failed`, and the whole-workspace run reported zero failed binaries. **This green is not evidence the flake is fixed**, and neither this summary nor the record claims it is — a ~3/5 flake passes two runs in five. Round 11's refusal to reinstate the `--test-threads=1` mitigation sentence stands and was not regressed.
- **`rtk` filtering was avoided throughout.** Every count-, presence- and grep-bearing check in this round ran under `rtk proxy`, never the hook-rewritten path, because the hook strips `test result:` and `warning:` lines and a grep for them would succeed vacuously.

## Estimate vs. actuals (calibration note)

The plan estimated **95,000 tokens** at `confidence: low`; the realized diff is **41,096 chars → ~10,274 estimateTokens**, a ~9× overestimate. The gap is explicable rather than random: `src/ui/screens/detail.rs` alone is 393,386 chars ≈ 98k estimateTokens if read in full, which is almost exactly the estimate. **The 95k was a read-surface estimate, not a diff estimate.** Reading that file in targeted ranges rather than whole is what closed the gap. Future estimates for plans touching one very large file should state which of the two quantities they are estimating.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- The one gap verification pass 12 missed is closed, with the control that would have caught it now in place and proven falsifiable.
- **This is the last pending plan of phase 21** (35 of 35). The phase is ready for verification.
- **Expect 4/5, not 5/5.** ROADMAP criterion 4 is permanently agent-unclosable and 4/5 is the correct ceiling; a verifier that re-litigates it is doing work that cannot succeed.
- **Standing residual, direction SILENT:** the round trip couples this build's producer to this build's consumer, but `claude`'s own option parser remains a dependency behaviour of a version. A future CLI that changes `--resume`'s arity invalidates the fused form with every test in this repository green. Tracked under `21-34`'s existing CLI-staleness obligation.
- STATE.md, ROADMAP.md and REQUIREMENTS.md were deliberately **not** touched — the orchestrator owns those writes after the wave completes.

## Self-Check: PASSED

- All three modified files exist on disk and carry the round's changes.
- All 7 commits exist and are reachable: `246c973`, `6cca4ca`, `68a9ff2`, `4a47b0f`, `fbc0cc7`, `f8ca0bd`, `f30318b`.
- `cargo test --lib` re-run on the delivered tree: **1113 passed / 0 failed / 0 ignored**.
- Working tree clean; no untracked or unstaged files left behind.
- `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md` and `tests/driver_injection_corpus.rs` are absent from the round's diff.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-28*
