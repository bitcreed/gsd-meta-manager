---
phase: 21-llm-goal-layer-prompt-injection-hardening
reviewed: 2026-08-27T00:00:00Z
depth: deep
files_reviewed: 2
files_reviewed_list:
  - src/session_detector.rs
  - src/ui/screens/detail.rs
findings:
  critical: 1
  warning: 5
  info: 7
  total: 13
status: issues_found
---

# Phase 21 (round 12): Code Review Report

**Reviewed:** 2026-08-27
**Depth:** deep (cross-file: import graph + call chains traced into `src/terminal_switch.rs`, `src/executor/claude.rs`, `src/test_support.rs`, `src/driver/liveness.rs`)
**Files Reviewed:** 2
**Status:** issues_found

## Summary

Round 12's six claims were checked against the code rather than against the summary.
**Five of the six hold.** The sixth — the round trip's central invariant, *"what this
build emits, this build must be able to read back," asserted as byte-identity* — is
**false for a class of ids the corpus does not carry**, which is the same vacuous-control
shape round 11 found in the hostile corpus and round 10 found in the interpreter census.

Verified independently and confirmed:

- **Claim 1 (behaviour-preserving split).** The old `args.windows(2)` scan and the new
  index loop were walked against six adversarial argv shapes (`--resume` as its own value,
  empty interior element, trailing option name, option name as last element with no
  trailing NUL). They agree on every one. The `index += 1` inside the split branch is a
  genuine addition — the old code did not step over the consumed value — but no shape was
  found where it changes the answer.
- **Claim 2 (both wire forms).** Both are read; both are asserted, and the SPLIT arm is
  constructed explicitly rather than inferred from the fused one. See WR-01 for the third
  spelling that is *not* read.
- **Claim 3 (non-validating).** No first-byte rule, no character class, no length bound.
  Keep-scanning-past-an-empty-value survives and is asserted. **But `trim()` is not only a
  condition — it is a transformation** (CR-01).
- **Claim 4 (innermost wrap).** `session_id_in_cmdline` returns `Option<Untrusted>`; no
  `String` path out of the module exists. Holds.
- **Claim 5 (negative shapes, no panic).** `args[index]` is always guarded by
  `index < args.len()`; the successor is read through `args.get(index + 1)`. Every loop
  iteration advances `index` by at least 1. No panic, no unbounded loop. Holds.
- **Claim 6 (producer untouched).** Verified at the diff: `src/ui/screens/detail.rs` has
  exactly **one** hunk, `@@ -7719,6 +7719,187 @@`, **all additions, zero deletions**, and
  `mod tests` begins at line 5877. Nothing outside `mod tests` changed. Holds.

`cargo clippy --all-targets` is clean apart from the four known out-of-scope lints. The
seven session-id tests pass.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: `trim()` silently REWRITES the session id, so the round-trip byte-identity the round asserts is false — and the corpus cannot see it

**File:** `src/session_detector.rs:283-291` (the returned value is built at `289`)

**Issue:**

```rust
let val = String::from_utf8_lossy(candidate);
let val = val.trim();                                    // ← shadows with the TRIMMED slice
if !val.is_empty() {
    return Some(Untrusted::from_untrusted_source(val.to_string()));   // ← returns the TRIMMED value
}
```

The value returned is `val.to_string()` where `val` is the **trimmed** slice, not the wire
bytes. The docs describe this as a *condition* and never as a *transformation*:

- `session_id_in_cmdline` doc, line 248: *"The only condition on the VALUE remains
  non-emptiness after `trim()`"*
- `read_session_id` doc, line 105: *"nothing here constrains the id beyond non-emptiness
  after `trim()`"*
- `session_id_in_cmdline` doc, line 190: *"they assert nothing whatever about which VALUES
  are acceptable"*
- `resume_terminal_argv` doc, `detail.rs:690`: *"The id reaching the child stays
  byte-identical to `as_raw_for_logic_only()`"*

All four are false for any id with leading or trailing whitespace. Two concrete failures,
both silent:

1. **The round trip is not a round trip.** For `raw = " abc "` the producer emits
   `--resume= abc ` and `session_id_in_cmdline` hands back `"abc"`. The Sessions tab then
   displays, and later re-resumes with, an id **no process on the machine actually
   carries**. The chain `real argv → detector → resume argv → child` is byte-identical
   everywhere *except* here, which is precisely where nobody was looking.
2. **The exact defect round 12 exists to fix is still open for one shape.** For a
   whitespace-only id the producer emits `--resume=   `, the consumer's `!val.is_empty()`
   fails, and `session_id_in_cmdline` returns `None` — *the row goes dead in the Sessions
   tab with no error and no log*, which is the failure sentence quoted verbatim in the new
   test's own assertion message at `detail.rs:7819-7830`.

**Why the new control cannot fail on it — this is the finding, not a footnote.** The
round-trip corpus is `hostile_session_ids()` (`detail.rs:7400-7443`): 7 imported
`LOOK_ALIKE_PAIRS` + 11 shell-metacharacter fixtures + 10 option lookalikes. **Not one of
the 28 carries leading or trailing whitespace.** The 7 imported look-alikes are all
`General_Category=Cf` (U+200B, U+FEFF, U+202E, U+E0041, U+00AD) and none has
`White_Space=Yes`, so `str::trim` leaves them alone; `"a b"` has an *interior* space only.
The corpus is therefore structurally incapable of failing this assertion — the same
complicity `the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program`
names at `detail.rs:7585-7591` ("eighteen hostile fixtures, every one of them a shell
metacharacter class, certifying a claim about option-shaped inputs"), arriving one round
later against a different property.

This is **not** the validator that was declined (D-21-48). The fix makes the parser
*strictly less* opinionated about the value: test emptiness on a trimmed copy, return the
bytes verbatim.

**Fix:**

```rust
if let Some(candidate) = candidate {
    let val = String::from_utf8_lossy(candidate);
    // Emptiness is TESTED on a trimmed copy. The value RETURNED is the wire
    // bytes verbatim, so what this build emits it reads back byte-identically.
    // Trimming the returned value would silently rewrite an id whose title
    // legitimately carries padding, and a whitespace-only id would vanish
    // from the Sessions tab with no message — the defect this round closed,
    // one shape over.
    if !val.trim().is_empty() {
        return Some(Untrusted::from_untrusted_source(val.into_owned()));
    }
}
```

Then extend `hostile_session_ids()` so the control can fail:

```rust
" leading",          // padding must survive the round trip verbatim
"trailing ",
"\tboth\t",
```

And assert the one shape that genuinely cannot round-trip, as a *named* limit rather than
an unnoticed one: a whitespace-only id emitted by the producer reads back as `None`, so
either state that in `session_id_in_cmdline`'s doc with the direction (under-detection,
silent) or make the non-empty test operate on the raw bytes.

## Warnings

### WR-01: the short spelling `-r` is a third legitimate wire form, and it is read by nothing — the doc's coverage claim is false

**File:** `src/session_detector.rs:214`, `260-281` (doc claim at `238-241`)

**Issue:** The doc for `session_id_in_cmdline` claims exhaustive coverage of what arrives
from outside the TUI:

> *"**Split** — ... This is what a human typing the command by hand, or any launcher that
> is not this TUI, still produces. Dropping it would re-break detection for every session
> not started here."*

The installed CLI documents the option as `-r, --resume [value]` — quoted in this very
codebase at `detail.rs:654`. A human typing `claude -r <id>`, or `claude -r=<id>`,
produces a shape the parser does not recognise, so that session reads back as
`session_id: None` and its Sessions-tab row answers `No session ID to resume`. Direction:
**under-detection, and silent** — byte-for-byte the class round 12 exists to close.

This is not a hypothetical the build is unaware of: `hostile_session_ids()` carries `"-r"`
as a fixture and annotates it *"the SHORT spelling of the option being injected into"*
(`detail.rs:7432`). One module knows `-r` exists; the other does not. That is the same
independent-spelling-across-a-module-boundary mechanism the round-12 doc names as the root
cause, still live.

Recognising `-r` is **shape**, not value validation, so it does not touch D-21-48.

**Fix:** add the short spellings to the shape check and to the parser test, and drive the
`-r` split form through the round trip alongside `--resume`:

```rust
/// The resume option's SHORT spelling, split form (`claude -r <id>`).
const RESUME_OPTION_SHORT: &[u8] = b"-r";
/// The short spelling fused (`claude -r=<id>`).
const RESUME_OPTION_SHORT_FUSED_PREFIX: &[u8] = b"-r=";
```

then in the loop, `strip_prefix` against both fused prefixes and compare against both bare
names. Add `parsed(&["claude", "-r", "abc"]) == Some("abc")` and
`parsed(&["claude", "-rx", "abc"]) == None` to
`session_id_in_cmdline_reads_both_wire_forms_and_no_other_shape`.

### WR-02: `String::from_utf8_lossy` fabricates the id from non-UTF-8 wire bytes, and the new harness is structurally incapable of covering it

**File:** `src/session_detector.rs:284`; harness gap at `src/ui/screens/detail.rs:7744`

**Issue:** `/proc/<pid>/cmdline` is arbitrary bytes, which this file's own type doc states
(`session_detector.rs:8-12`: *"nothing constrains it to ASCII"*). `from_utf8_lossy`
replaces every invalid sequence with U+FFFD, so:

- an id containing invalid UTF-8 is returned **corrupted**, displayed to the operator as
  if real, and `resume_terminal_argv` then emits the corrupted bytes — resuming nothing,
  with no message;
- an id made **entirely** of invalid bytes lossy-converts to a non-empty run of U+FFFD,
  passes the emptiness test, and produces a Sessions-tab row for an id that never existed
  — over-detection, also silent;
- the `Untrusted`/`shown()` escape layer never sees the real bytes, so the "every render
  goes through `shown()`" argument at `session_detector.rs:136-140` is defending a value
  that was already rewritten upstream of it.

The new round-trip control cannot reach this class by construction: `nul_join_cmdline`
takes `&[&str]` (`detail.rs:7744`) and `hostile_session_ids()` returns `Vec<String>`, so
every byte that can ever enter the harness is valid UTF-8 by the type system. That
limitation is not stated anywhere in the two new tests' docs, which otherwise enumerate
their own gaps carefully.

**Fix:** minimum, name the limit where the control is defined, and add a direct
non-UTF-8 parser arm that does not go through the `&str` harness:

```rust
// In session_detector::tests — bytes the `&str` round-trip harness cannot express.
assert_eq!(
    session_id_in_cmdline(b"claude\0--resume=\xff\xfe\0")
        .map(|id| id.as_raw_for_logic_only().to_string()),
    Some("\u{fffd}\u{fffd}".to_string()),
    "non-UTF-8 wire bytes are LOSSY-converted, so the id this build reports \
     is not the id the process carries. Direction: the resume emits bytes no \
     session has, silently."
);
```

Better: carry the id as `Vec<u8>`/`OsString` inside `Untrusted` so the wire bytes survive
to the argv, and lossy-convert only at render.

### WR-03: the pairing invariant covers ONE of this build's argv producers; `--session-id <uuid>` is emitted by another and read by none

**File:** `src/ui/screens/detail.rs:7799-7846` (the round trip) — cross-file:
`src/executor/claude.rs:256`, `src/executor/claude.rs:270`, `src/executor/claude.rs:370`

**Issue:** The class comment at `detail.rs:7758-7783` states the invariant as a property
of *this build*: *"what this build emits, this build must be able to read back."* The
control asserts it over exactly one producer, `resume_terminal_argv`. Traced across module
boundaries, this build has a **second** `claude` argv producer:

- `src/executor/claude.rs:370` — `program: PathBuf::from("claude")`, so `pgrep -x claude`
  in `get_claude_pids` (`session_detector.rs:44`) **does** return these pids and
  `build_session` **does** construct a `ClaudeSession` for each;
- `src/executor/claude.rs:255-256` — `push("--session-id"); push(options.session_id)`;
- `src/executor/claude.rs:269-271` — `push("--resume"); push(session)`, the split form.

The split `--resume` is now readable (good). `--session-id` is not recognised by
`session_id_in_cmdline` at all, so every executor-launched session this build starts
appears with `session_id: None` — the identical symptom, from the identical cause
(two modules spelling the option independently, nothing coupling them), on the producer the
new round trip does not reach. Round 12 closed the instance and left the class one producer
short.

Note also that the executor's `--resume` is **split, not fused**, so the CWE-88 argument
injection that round 11 closed at `resume_terminal_argv` is still open on that path if
`options.resume_session` is ever attacker-influenced. That file is outside this review's
scope, but the round trip is what would have surfaced it.

**Fix:** either teach the consumer `--session-id` (shape only — same non-validating rule),
and extend the round trip to drive `executor::claude`'s argv builder through
`session_id_in_cmdline` the same way; or state explicitly in the class comment which
producers the invariant is asserted over and which are knowingly excluded, with the
direction of the residual.

### WR-04: `read_tty` strips `/dev/` specifically to enable an UNANCHORED substring match, and `pts/3` matches `/dev/pts/31`

**File:** `src/session_detector.rs:84-94`, design endorsed at `23-26` — sink at
`src/terminal_switch.rs:57`

**Issue:** `read_tty` returns `"pts/3"` and the `tty` field doc (`lines 23-26`) explicitly
justifies the stripping so that *"a `contains()` match against tmux's `#{pane_tty}` (which
prints "/dev/pts/3") still hits."* The sink does exactly that:

```rust
if pane_tty.contains(tty) {          // src/terminal_switch.rs:57
```

`"/dev/pts/31".contains("pts/3")` is `true`. On any host with 10+ pseudo-terminals — the
normal case for a tmux user, which is the only case this code path runs in — the TUI can
`select-window`/`select-pane` onto **an unrelated pane** and steal the operator's focus.
`list-panes -a` output order decides which; nothing in the loop prefers an exact match.

Two further defects in the same producer: `fd/0` is *not* the controlling terminal (the
field is named and documented as if it were), so a session with redirected stdin yields
`"pipe:[12345]"`, which is then stored as a `tty` and substring-matched; and `strip_prefix`
falling through for such a value is silent.

**Fix:** make the comparison anchored at the sink and keep the producer's form honest:

```rust
// src/terminal_switch.rs
let want = format!("/dev/{tty}");
if pane_tty == want || pane_tty == tty {
```

and in `read_tty`, return `None` for a link that does not start with `/dev/` rather than
passing a pipe path through as a TTY.

### WR-05: two named controls assert one property — the fused round trip is certified twice

**File:** `src/ui/screens/detail.rs:7806-7833` and `7849-7897`

**Issue:** `the_argv_this_build_emits_is_an_argv_this_build_can_read_back` is:

```rust
for term in [5 terms] { for raw in hostile_session_ids() {
    argv = resume_terminal_argv(term, &sid);
    read_back = session_id_in_cmdline(&proc_cmdline_encoding(&argv));
    assert_eq!(read_back, Some(raw)); } }
```

The FUSED arm of `a_session_id_survives_the_round_trip_in_both_wire_forms`
(`detail.rs:7864-7882`) is the same five terminals, the same 28 fixtures, the same
encoder, the same consumer and the same assertion — with the loop nesting swapped. The
first test is **entirely subsumed**; deleting it removes no coverage.

This matters beyond duplication in *this* phase specifically. `read_session_id`'s own doc
(`session_detector.rs:154-155`, restated at `161-163`) warns that a second assertion of an
already-asserted property *"would let the class be counted as closed twice"* — and round 12
then added two named controls for one property, which inflates the control count a verifier
reads. The round-11 failure this round is remediating was a verifier scoring 90/91
must-haves against a build that had lost a capability.

**Fix:** delete `the_argv_this_build_emits_is_an_argv_this_build_can_read_back` and keep
`a_session_id_survives_the_round_trip_in_both_wire_forms`, moving the former's long failure
message (which is the better one — it names the drift mechanism and forbids un-fusing) onto
the surviving fused arm.

## Info

### IN-01: `read_start_time` finds the FIRST `)` in `/proc/<pid>/stat`, not the last

**File:** `src/session_detector.rs:303`
**Issue:** The comment on line 301-302 states the reason correctly — comm is
parenthesised and may contain spaces — but `stat.find(')')` breaks for any comm containing
`)`, e.g. a process named `cl)aude`. Field 22 would then be read from the wrong offset or
not at all. Currently unreachable because pids come only from `pgrep -x claude` (exact
name match), but it is one `pgrep` flag change away from being live, and pid recycling
between `pgrep` and the `/proc` read (IN-02) can already deliver a foreign comm.
**Fix:** `let after_comm = stat.rfind(')')?.checked_add(2)?;`

### IN-02: `pgrep` is resolved through `$PATH`, and pids are used after a TOCTOU window

**File:** `src/session_detector.rs:44`, `59-73`
**Issue:** `Command::new("pgrep")` resolves via the inherited `PATH`; a poisoned `PATH`
executes an attacker's `pgrep` with the operator's privileges on a timer. Separately, every
`/proc/<pid>/...` read in `build_session` happens after `pgrep` returned, so a recycled pid
yields a `ClaudeSession` describing a different process (its cwd, its cmdline, its tty)
with no detection.
**Fix:** invoke an absolute path (`/usr/bin/pgrep`) or read `/proc` directly; and
re-validate `/proc/<pid>/comm == "claude"` inside `build_session` before trusting the reads.

### IN-03: `/proc/<pid>/cmdline` is read unbounded

**File:** `src/session_detector.rs:207`
**Issue:** `std::fs::read` on `cmdline` has no size cap. Since Linux 4.2 a process can
present an arbitrarily large cmdline; `detect_sessions` runs over every `claude` pid on a
timer, so a local process can force repeated large allocations in the TUI.
**Fix:** read with a cap (e.g. `Read::take(64 * 1024)`) — the resume option and its value
appear early in any real argv.

### IN-04: the corpus size `28` is a magic number repeated in three tests; the terminal list in four loops

**File:** `src/ui/screens/detail.rs:7604`, `7859`; `7491`, `7625`, `7651`, `7808`, `7866`
**Issue:** Three separate non-vacuity assertions hard-code `28` with a hand-written
breakdown comment ("7 + 11 + 10"); changing the corpus requires editing all three, and a
stale one fails with a message that misdescribes the corpus. The five-terminal array is
re-spelled in five loops.
**Fix:** `const HOSTILE_CORPUS_LEN: usize = 28;` and
`const TERMINALS: [&str; 5] = [...];` beside `hostile_session_ids()`.

### IN-05: `proc_cmdline_encoding`'s `skip(1)` encodes an unasserted positional assumption about the producer

**File:** `src/ui/screens/detail.rs:7732-7734`
**Issue:** The helper assumes element 0 of `resume_terminal_argv`'s output is the
emulator's program separator. If the producer ever gains a leading element, `skip(1)`
silently drops the wrong one and the round trip asserts against an argv the producer never
emitted — the vacuous-pass shape `nul_join_cmdline`'s NUL guard was written to prevent, on
the other axis. Arity is pinned in a *different* test, so nothing local protects this.
**Fix:** assert the shape in the helper:
`assert_eq!(argv[0], terminal_program_separator(term), ...)`, or take the separator as a
parameter and assert against it.

### IN-06: `session_id_in_cmdline` is `pub(crate)` solely so a test in another module can call it

**File:** `src/session_detector.rs:260`
**Issue:** The only non-test caller is `read_session_id`, in the same module. `pub(crate)`
widens the surface for a test, which slightly weakens the "one place a session id enters
this build" framing the doc leans on.
**Fix:** acceptable as-is, but `#[cfg_attr(not(test), allow(dead_code))]` is not the answer;
prefer documenting that the widened visibility exists for the cross-module round trip, or
move the round trip into an integration test that uses the crate's public surface.

### IN-07: the split branch swallows an element that is itself id-bearing, contradicting the stated precedence rule

**File:** `src/session_detector.rs:271-278`
**Issue:** The doc (line 243) states *"The leftmost id-bearing element wins,
deterministically, by argv index."* For `["claude", "--resume", "--resume=abc"]` the split
branch consumes element 2 as a raw value and returns the whole string `"--resume=abc"` as
the id, rather than `"abc"` from the fused element. Both readings are defensible; neither
is asserted, and the doc's rule reads as if the fused element would win.
**Fix:** add an arm to
`session_id_in_cmdline_reads_both_wire_forms_and_no_other_shape` pinning whichever reading
is intended, so the next reader is not left to infer it from the loop.

---

_Reviewed: 2026-08-27_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
