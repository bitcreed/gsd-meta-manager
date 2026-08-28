---
status: issues_found
phase: 21
round: 11
reviewed: 2026-08-27
depth: standard (per-file, plus targeted simulation of the four censuses and a base-vs-HEAD argv trace)
range: 2c13fcf..HEAD
files_reviewed: 9
files_reviewed_list:
  - src/session_detector.rs
  - src/state_reader/backlog.rs
  - src/text.rs
  - src/ui/mod.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/render_escape_guard.rs
  - tests/driver_reattach.rs
findings:
  critical: 0
  high: 1
  medium: 2
  low: 4
  total: 7
---

# Phase 21 (round 11): Independent Code Review

**Range:** `2c13fcf..HEAD` (plans 21-31, 21-32, 21-33, 21-34)
**Verdict:** `issues_found` — 0 CRITICAL, 1 HIGH, 2 MEDIUM, 4 LOW.

## What was checked and how

| Check | Method | Result |
|---|---|---|
| Every construction site of the resume argv | `grep -rn -- "--resume"` over `src/` + `tests/`, then read each hit | Two builders exist; one is the live path and is fused, the other is unreachable (LOW-1) |
| Can any path still emit `--resume` and the id as two argv elements | Read `resume_terminal_argv`, `launch_terminal_argv`, `executor::claude::build_argv`; traced `resume_session` producers | Live path: no. Executor path: yes, but no `Some` producer exists (LOW-1) |
| Are the round-11 controls falsifiable | Hand-traced each new assertion against the pre-fix body | All four planted-defect controls are genuinely red pre-fix |
| 21-33's census pin | Re-measured independently: `find src/ui -name '*.rs' \| wc -l` = 16; per-file non-comment `display_identity(` counts | Pin is CORRECT: driver.rs 2, driver_confirm.rs 4, render_escape_guard.rs 2; six non-exempt in two of sixteen |
| 21-32's non-vacuity pin | Re-measured `sanitize_render_line(` across both census files | `(5, 6, 7)` is CORRECT; the cited line number is not (MEDIUM-1) |
| 21-32's `taken += 1` relocation | Hand-traced the loop; verified the boundary arithmetic `16 - 5 = 11` / `16 - 4 = 12` | Correct, terminating, and the executable window is genuinely unchanged |
| Build health | `rtk proxy cargo test --workspace --no-fail-fast`, `rtk proxy cargo clippy -- -D warnings` | 1110 lib + all integration tests green; clippy clean. The `driver_reattach` flake did not fire this run |

All count-bearing checks ran through `rtk proxy` per the tooling constraint.

---

## HIGH

### HI-01 — 21-31's fusion silently deleted the resume capability's own feedback loop: `read_session_id` cannot parse the argv shape the app now emits

**Files:** `src/session_detector.rs:155-172` ↔ `src/ui/screens/detail.rs:703-715`
**Severity:** HIGH (silent capability regression in the exact feature being hardened; not a security hole)

`resume_terminal_argv` now emits **one** argv element:

```rust
format!("{RESUME_OPTION_FUSED_PREFIX}{}", sid.as_raw_for_logic_only())   // detail.rs:713
```

`read_session_id` — the *only* place a session id enters this build — still matches the **separated** form only:

```rust
for window in args.windows(2) {
    if window[0] == b"--resume" {          // session_detector.rs:160
```

For a child whose `/proc/<pid>/cmdline` is `claude\0--resume=<id>\0`, `args` is
`["claude", "--resume=<id>", ""]`; no `windows(2)` element equals `--resume`, so the
function returns `None`.

**Base-vs-HEAD trace, confirmed against `git show 2c13fcf:src/ui/screens/detail.rs`:**

| | argv the TUI spawns | child cmdline | `read_session_id` |
|---|---|---|---|
| base `2c13fcf` | `[sep, "claude", "--resume", "<id>"]` | `claude --resume <id>` | recovers `<id>` |
| HEAD | `[sep, "claude", "--resume=<id>"]` | `claude --resume=<id>` | **`None`** |

**Consequence.** A session the TUI itself resumed now appears in the Sessions tab with
`session_id: None`. Pressing Enter on that row returns
`"No session ID to resume"` (`detail.rs:1954-1957`). The resume loop that worked at
`2c13fcf` — resume a session, see it, resume it again — is broken, and it fails
*silently*: no error, no log, just a row that can no longer be acted on. This is
precisely the "feature deletion wearing a security fix's clothes" shape that
`terminal_program_separator`'s and `resume_terminal_argv`'s own docs are written to
prevent, arriving one file over from where those docs are looking.

**Why nothing caught it.**

1. The option spelling has **two independent sources of truth**:
   `RESUME_OPTION_FUSED_PREFIX` (`detail.rs:585`, private to that module) and the byte
   literal `b"--resume"` (`session_detector.rs:160`). Nothing couples them, and 21-31's
   T-21-31-01 rationale ("the option name and the `=` that binds its value cannot drift
   apart between the builder and the controls that check it") stops at the module
   boundary.
2. `src/session_detector.rs`'s only relevant test is
   `test_read_session_id_nonexistent_pid` (`:197-201`), which passes because the
   `/proc` read fails. It certifies nothing about the parser. See LOW-4.
3. 21-31's summary explicitly declares "No control is added in this file, deliberately"
   (`session_detector.rs:148-154`), on the argument that the property that matters is a
   property of the sink. That argument is right about *security* and wrong about
   *capability*: the round-trip property (what this build emits, this build can read
   back) is a property of the pair, and no plan in the round owned it.
4. Neither `21-31-PLAN.md` nor `21-31-SUMMARY.md` mentions the round trip at all —
   `grep -n "cmdline"` over the four summaries returns only the threat-model direction
   (attacker → us), never the emit direction (us → us).

**Recommended fix** (in `src/session_detector.rs`, accept both shapes — the separated
form must stay, because a human running `claude --resume <id>` by hand still produces
it):

```rust
const RESUME_OPT: &[u8] = b"--resume";
const RESUME_OPT_FUSED: &[u8] = b"--resume=";

fn read_session_id(pid: u32) -> Option<Untrusted> {
    let cmdline = std::fs::read(format!("/proc/{}/cmdline", pid)).ok()?;
    let args: Vec<&[u8]> = cmdline.split(|&b| b == 0).collect();

    for (index, arg) in args.iter().enumerate() {
        // FUSED: `--resume=<id>`, the shape THIS build emits since 21-31.
        let candidate: &[u8] = if let Some(rest) = arg.strip_prefix(RESUME_OPT_FUSED) {
            rest
        // SEPARATED: `--resume <id>`, what a human typing the command produces.
        } else if *arg == RESUME_OPT {
            match args.get(index + 1) { Some(next) => next, None => continue }
        } else {
            continue;
        };
        let val = String::from_utf8_lossy(candidate);
        let val = val.trim();
        if !val.is_empty() {
            return Some(Untrusted::from_untrusted_source(val.to_string()));
        }
    }
    None
}
```

**And add the control that would have caught it** — a round-trip assertion driving both
committed functions, not a re-spelling of either. It needs the parse split out of the
`/proc` read (e.g. `fn session_id_in(cmdline: &[u8]) -> Option<Untrusted>`), which is
the same "the live assertion and the control must consume the SAME function" split
21-32 already made twice this round:

```rust
// The shape this build EMITS must be a shape this build can READ.
// Red against HEAD: resume_terminal_argv fuses, read_session_id parses only the
// separated form, so the id the TUI hands `claude` is invisible to the detector
// that finds it again — the Sessions tab cannot re-resume its own session.
for raw in hostile_session_ids() {                      // the same corpus, imported
    let argv = resume_terminal_argv("kitty", &Untrusted::from_untrusted_source(raw.clone()));
    let cmdline = std::iter::once("claude")
        .chain(argv.iter().skip(2).map(String::as_str))  // program + its own options
        .flat_map(|a| a.as_bytes().iter().copied().chain(std::iter::once(0u8)))
        .collect::<Vec<u8>>();
    assert_eq!(
        session_id_in(&cmdline).map(|u| u.as_raw_for_logic_only().to_string()),
        Some(raw.clone()),
        "read_session_id cannot recover the id from the argv resume_terminal_argv \
         emits. The two spell `--resume` independently and have drifted."
    );
}
```

Whatever the exact form, the acceptance bar is: it must go **red on HEAD as committed**.

---

## MEDIUM

### ME-01 — 21-32 shipped three present-tense line citations that are stale by exactly the number of lines its own wave inserted

**Files:** `src/ui/screens/driver.rs:2189-2192`, `src/ui/screens/driver.rs:2513-2518`
**Severity:** MEDIUM (measured-false claims in the load-bearing rationale of a control, in a phase whose stated subject is that prose without a measurement is untrustworthy)

Three claims about the committed tree, all wrong, all off by exactly the size of a
change that landed in the same wave:

| Claim (verbatim) | Actual | Off by |
|---|---|---|
| `driver.rs:2189` — "`src/ui/screens/detail.rs` carries two column-zero test attributes, at `:5342` (a `pub(super) fn` helper)" | `detail.rs:5454` | 112 = the lines 21-31 added above it |
| `driver.rs:2191` — "and `:5764` (`mod tests`)" | `detail.rs:5876` | 112 |
| `driver.rs:2516` — "the difference being `driver.rs`'s own test-module call at `:2349`, which the census never scans" | `driver.rs:2651` | 302 = the lines 21-32 itself inserted at `:2157` |

Verified with `grep -n "^#\[cfg(test)\]" src/ui/screens/detail.rs` → `5454`, `5876`,
and `grep -n 'sanitize_render_line(' src/ui/screens/driver.rs` → the only test-module
hit is `:2651`. The *substance* of all three is correct (detail.rs really does carry a
non-module test attribute above its `mod tests`; the whole-file/production-slice
difference really is that one call). Only the coordinates are wrong.

The third one is the one that matters: it is the stated justification for the pinned
constant `WHOLE_FILE_NEEDLE_LINES = 7`, and the assertion's own failure message
instructs the next reader to "re-measure them, say in this comment why they changed".
A reader who follows the citation lands on `driver.rs:2349`, which is inside
`strip_trailing_path_qualifiers`' doc block and carries no needle at all — so the
evidence for the pin appears, to anyone who checks, to not exist. That is the
failure mode this phase has spent eleven rounds naming.

These are distinguishable from the *verbatim compiler/panic captures* elsewhere in the
round (e.g. `screens/mod.rs:794` quoting `787 | pub struct EditBuffer`, which is a
faithful capture from a scratch tree and is declared as such). These three are
present-tense assertions about the committed tree.

**Fix:** re-measure and correct the three numbers. Better, remove the class: cite by
identifier rather than by line (`detail.rs`'s `first_string_entry` helper; `driver.rs`'s
`test_module_marker_lines`-era test call), or — for the third — replace the sentence
with the difference the code already computes (`whole_file - occurrences == 1`), which
cannot go stale.

### ME-02 — `needle_distribution` counts LINES; its name, its doc, and the pinned table all say OCCURRENCES

**File:** `src/ui/mod.rs:279-302` (`needle_distribution`), pinned at `:341-345`
(`MEASURED_REACH`), disclosed at `:378-388`
**Severity:** MEDIUM (the disclosed property is strictly wider than the mechanism, in the
one artifact 21-33 added specifically to stop a disclosed property from being prose)

```rust
let count = lines
    .iter()
    .filter(|(_, line)| !line.trim_start().starts_with("//"))
    .filter(|(_, line)| line.contains(&call))     // <-- one per LINE, not per call
    .count();
```

The function's doc says "Executable **occurrences** of the call needle per file"; the
test's disclosed table is headed "Executable needle **occurrences**"; the totals row
reads "**8** — six non-exempt". Today lines and occurrences coincide (I verified: no
line under `src/ui/` carries two `display_identity(` calls), so the pin is *correct as
committed* — but it is not measuring what it claims to measure. Writing
`display_identity(a) + display_identity(b)` on one line adds an occurrence the reach
pin cannot see, and the pin's entire purpose is to make a disclosed number go red when
the tree moves under it.

The contrast is internal to this same round: `driver.rs`'s census pins
`PRODUCTION_NEEDLE_UNITS` **and** `PRODUCTION_NEEDLE_OCCURRENCES` separately, precisely
because it found a unit carrying two (`driver_confirm.rs:178`/`:180`). 21-33 pinned the
weaker of the two quantities under the stronger one's name.

**Fix:**

```rust
let count = lines
    .iter()
    .filter(|(_, line)| !line.trim_start().starts_with("//"))
    .map(|(_, line)| line.matches(&call).count())
    .sum();
```

`MEASURED_REACH` is unchanged by this today (`2 / 4 / 2`), so the fix is a one-line
change with no pin churn — which is exactly why it should be made now rather than
after a two-call line appears.

---

## LOW

### LO-01 — `executor::claude::build_argv` still emits `--resume` and its value as two argv elements, with no pin holding it unreachable

**File:** `src/executor/claude.rs:267-272`

```rust
if let Some(session) = &options.resume_session {
    push(&mut argv, "--resume");
    push(&mut argv, session);
}
```

This is the CWE-88 shape 21-31 removed from `resume_terminal_argv`. It is **not** live:
`resume_session` has exactly one initializer in the whole tree
(`src/executor/mod.rs:465`, `resume_session: None`) and no `Some` producer anywhere —
verified by `grep -rn "resume_session" --include=*.rs`. 21-31 examined it and declared
it inert (D-21-50, T-21-31-05), and `claude.rs:2044` asserts `--resume` is absent from
the built argv. So the disclosure is honest and I am not calling it a vulnerability.

What is missing is the pin. 21-31 did exactly the right thing for the *other* sibling
builder — `launch_terminal_argv_carries_no_untrusted_element_and_is_pinned_at_two`
(`detail.rs:7737`) turns "examined and found safe" into something that fails when it
stops being true. `resume_session` got the reasoning without the pin, so the first
caller that sets it from a scraped id reopens CWE-88 with every test green.

**Fix (cheapest, no behaviour change):** fuse it — `push(&mut argv, format!("--resume={session}"))` —
and note in the doc that this is the same control `resume_terminal_argv` carries. Or,
if the two-element form is wanted for the executor, add a source-scan pin asserting
`resume_session` has no `Some` producer under `src/`, in the same house style as the
existing `spawn_seam_guard` scans.

### LO-02 — `ui::mod::census`'s join loop has the exact `taken += 1` placement 21-32 repaired in `text.rs`, undisclosed

**File:** `src/ui/mod.rs:212-224`

```rust
while taken < CALL_JOIN_LINES && ahead + 1 < lines.len() && continues_onto_the_next_line(&logical) {
    ahead += 1;
    taken += 1;                     // <-- BEFORE the comment `continue`
    let next = lines[ahead].1.trim();
    if next.starts_with("//") { continue; }
```

This is the statement placement 21-32 moved in `text::tests::interpreter_sites_in` (T-21-32-01).
**The direction here is the safe one** — in this census finding the composer causes a
*skip*, so a budget shortened by comments produces *more* reports (loud
over-detection), not fewer. So it is not a defect and must **not** be "fixed for
consistency" without re-reasoning the direction.

The finding is the disclosure gap: the test's residual 3 (`ui/mod.rs:518-523`) says only
"a composition wrapped across more than `CALL_JOIN_LINES` physical lines reads as
un-composed". With `CALL_JOIN_LINES = 4`, three interleaved comment lines are enough to
trigger the same false positive at three *physical* lines. The disclosed reach is wider
than the mechanism's, which is the same class as ME-02.

**Fix:** one clause in the residual — "…or wrapped across fewer lines with comment lines
interleaved, which spend the budget here (safe direction: this census reports on
*failing* to find the composer)".

### LO-03 — `an_input_echo_screen_...`'s "the escape ACTED" arm is a self-oracle

**File:** `src/ui/screens/render_escape_guard.rs:3272-3741`

```rust
let expected = display_identity(hostile);
assert!(hostile_text.contains(&expected), ...);
```

The oracle is `display_identity`, which is the second half of `render_for_terminal` —
the function under test. The assertion therefore certifies "the render's output contains
`display_identity`'s output", which is true by construction for any correct-shaped
escape and cannot detect a *wrong* escape, only a *dropped* value. The test's own
comment says the self-oracle is deliberate ("rather than by a second spelling that could
disagree with it") and the committed RED fires on the preceding `survivors.is_empty()`
arm, not on this one — so this arm has no observed red.

Not a defect: the arm's stated job is to distinguish "escaped" from "dropped", and it
does that. Flagged so it is not later counted as escaping-correctness coverage. The
correctness of `display_identity` itself is carried by `text.rs`'s alphabet census, and
that is where it should stay attributed.

### LO-04 — `test_read_session_id_nonexistent_pid` is near-vacuous

**File:** `src/session_detector.rs:197-201`

```rust
assert!(read_session_id(999_999_999).is_none());
```

Returns `None` at `std::fs::read(...).ok()?` — the first line — so the assertion holds
for *any* body of the parsing loop below it, including an empty one. It is the only test
touching the function, and it is what let HI-01 ship. Pre-existing, not introduced this
round, but round 11 added 59 lines of doc to this function asserting a decision about its
behaviour without adding anything that can go red about that behaviour.

**Fix:** subsumed by HI-01's recommended round-trip control, which drives the parse over
in-memory NUL-separated fixtures.

---

## Things checked that came back CLEAN

Recorded so a later reader knows these were adversarially examined and not merely
skipped.

- **Fusion completeness (Priority 1).** `resume_terminal_argv` (`detail.rs:703-715`) is
  the only builder reachable from the Sessions-tab resume, and it is called from exactly
  one site (`detail.rs:1929`). Every terminal in `terminal_program_separator`'s table
  produces a 3-element vector with the id fused; no path emits `--resume` and the id
  separately (LO-01 excepted, and it is unreachable). The `--` separator was correctly
  *not* used. `read_session_id`'s non-validating pass-through is, as instructed, not
  flagged — HI-01 is about the parser's *shape coverage*, not about validation.
- **`the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program`**
  is genuinely falsifying. Traced against the base builder
  `[sep, "claude", "--resume", raw]`: the hyphen-leading count is 3 for the nine
  option-lookalike fixtures and 2 for the rest, so `observed.len() == 2` and the content
  independence arm goes red. The `corpus.len() == 28` pin arithmetic checks out
  (`LOOK_ALIKE_PAIRS` is `[(&str, &str); 7]` + 11 + 10).
- **The rewritten assertion (1)** in `the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element`
  handles the two adversarial fixtures correctly: `a=b` (splits on the FIRST `=`, so the
  id arrives whole) and `--settings=/tmp/x.json` (a value that already carries a fused
  `=`). It no longer checks *which* option the id is fused to, but the sibling test's
  `RESUME_OPTION_FUSED_PREFIX` arm covers that.
- **21-32's `taken += 1` relocation (Priority 3).** Hand-traced: `taken` starts at 1 for
  the seed line, the guard `taken >= 16` is evaluated pre-append, and the fixture spends
  4 before the filler — so `LAST_REPORTED_FILLER = 11` / `FIRST_MISSED_FILLER = 12` is
  arithmetically exact, and the executable window is provably unmoved. The comment
  `continue` cannot loop forever (`ahead` strictly increases and is bounded).
- **21-32's relocated verdict (Priority 3).** `occurrence_is_composed` is strictly
  stronger than the whole-unit rule it replaces — it can only report more, never fewer,
  so nothing the old site covered is lost. `strip_trailing_path_qualifiers` terminates
  (each iteration strictly shrinks the slice), never panics (`boundary` comes from
  `char_indices`), and handles both live spellings (`crate::text::display_identity(&` and
  `...(&super::`). The `laundering_fixture` really does join both arms into one logical
  unit at line 3 (traced through `logical_lines`' `closes` rule: `],` does not close, `}`
  does), so the control's red is the verdict's and not the join's.
- **21-33's census pin (Priority 4).** Independently re-measured. 16 `.rs` files under
  `src/ui/`; needle lines `driver_confirm.rs` 4 (`:178`, `:180`, `:304`, `:344`),
  `driver.rs` 2 (`:645`, `:944`), `render_escape_guard.rs` 2 (`:2615`, `:3272`).
  `MEASURED_REACH` matches exactly, `UI_SOURCE_FLOOR = 16` matches, and "six non-exempt
  in two of sixteen" is right. The `render_escape_guard.rs` exemption is **justified, not
  convenient**: both its occurrences are `let expected = display_identity(...)` /
  `let escaped = display_identity(&hostile)` oracle lines, and `stale_exemptions` keeps
  the exemption honest by reporting it if the file stops containing an un-composed call.
  The two doc line numbers cited for those occurrences (`:2615`, `:3272`) are correct —
  unlike ME-01's.
- **21-32's non-vacuity pin `(units, occurrences, whole_file) == (5, 6, 7)`.**
  Re-measured: correct. Only the prose citation of the 7th line is wrong (ME-01).
- **21-33's `EditBuffer` trait probe.** The three `String` presence arms are real
  non-vacuity guards (a broken autoref probe answers `false` to everything and the
  presence arms catch it). The orphan-rule argument bounding the "feature-gated impl"
  residual is sound. "Derives nothing at all, `Clone` included" is literally true —
  `Default` is hand-written at `screens/mod.rs:849`.
- **21-33's backlog tiebreak.** `key(a).total_cmp(&key(b)).then_with(|| a.cmp(b))` is a
  genuine total order over elements (lexicographic composition of two total orders), the
  fixture `["999.zebra", "999.alpha"]` really does tie on the key, and its alphabetical
  order really is the reverse of its input order — so the control is red pre-fix, as its
  quoted panic shows. Scope claim ("display ordering only") checked: no persisted
  artifact reads this order.
- **21-33's rename.** `grep -rn "every_render_site_under_ui_composes_both_classes" src/ tests/`
  returns zero. The rename fully landed.
- **21-34.** `tests/driver_reattach.rs` is comment-only, as claimed — the diff touches no
  executable line, so the binary behaves identically. The known `driver_reattach` flake
  is excluded per instruction; it did not fire in this review's workspace run
  (all suites green).
- **Build health.** `cargo test --workspace --no-fail-fast`: 1110 lib tests + every
  integration suite green. `cargo clippy -- -D warnings` (the project-standard
  invocation per CLAUDE.md): clean. `cargo clippy --all-targets -- -D warnings` reports 4
  errors, all in `src/browser.rs:155-157` and `src/project_creator.rs:146` — files
  untouched by round 11, pre-existing, out of scope.

---

_Reviewed: 2026-08-27_
_Reviewer: Claude (gsd-code-reviewer), adversarial stance_
_Depth: standard + targeted simulation_
_Prior round's REVIEW.md is preserved in git history at `dfa11c6`._
