---
phase: 21-llm-goal-layer-prompt-injection-hardening
reviewed: 2026-08-27T00:00:00Z
depth: standard
range: 80bc4c1..HEAD
files_reviewed: 20
files_reviewed_list:
  - src/app.rs
  - src/error.rs
  - src/state_reader/backlog.rs
  - src/test_support.rs
  - src/text.rs
  - src/ui/mod.rs
  - src/ui/roadmap_widget.rs
  - src/ui/screens/add_project.rs
  - src/ui/screens/create_project.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/driver_inject.rs
  - src/ui/screens/driver_start.rs
  - src/ui/screens/enqueue.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/queue_delete_confirm.rs
  - src/ui/screens/render_escape_guard.rs
findings:
  critical: 1
  warning: 6
  info: 6
  total: 13
status: issues_found
---

# Phase 21 (round 10): Code Review Report

**Reviewed:** 2026-08-27
**Depth:** standard (per-file, with targeted simulation of the three new censuses)
**Range:** `80bc4c1..HEAD`
**Files Reviewed:** 20
**Status:** issues_found

## Summary

The four round-10 claims were checked against the code rather than against their prose.

**What is true.**

- **21-27 (CR-01) — the interpreter is genuinely gone.** `sh -c` no longer appears in
  either Sessions-tab spawn path. `src/ui/screens/detail.rs:1817-1818` and `:2223-2224`
  both build an argv vector and pass the working directory through
  `Command::current_dir`. A tree-wide grep for `Command::new` / `"-c"` finds no other
  shell-interpolation site introduced by this round; the only remaining `sh -c` is
  `project_creator::execute_hook` (`src/project_creator.rs:69-71`), which runs an
  operator-authored `hooks.post_create` from the user's own `config.json` and is
  correctly named as an out-of-scope case in the census doc.
- I independently ran the 21-27 census's algorithm against the pre-fix
  `src/ui/screens/detail.rs` (extracted from `80bc4c1`). It reports both original sites
  (`:1693` and `:2097`). The "it would have caught CR-01" claim is real, not asserted.
- **21-28 (CR-02)** is type-held as advertised. `DriverOutputLine::text` is
  `crate::text::Untrusted` (`src/ui/screens/mod.rs:475`), `push_record` is the single
  construction site (`src/ui/screens/mod.rs:566-583`), and `driver.rs`'s pane renders it through `.shown()`.
- **21-29's backlog fix** is a correct total order: `f64::total_cmp` over a key that
  filters non-finite values, with antisymmetry and transitivity swept in tests.
- **`WAVE_PENDING` is `[(&str, &str); 0]`** (`src/ui/mod.rs:121`) — empty, as required.
- `cargo test --lib` is 1103/0 green; `cargo clippy --all-targets -- -D warnings`
  reports only the four known pre-existing lints (`browser.rs:155-157`,
  `project_creator.rs:146`). Both run through `rtk proxy`.

**What is not.** One live injection residual survives the CR-01 fix in the exact
function that claims to close it, and three of this round's four new guard mechanisms
have measured blind spots that are wider than the residuals their docs disclose. Each
blind spot below was reproduced by running the guard's own algorithm against a fixture,
not inferred from reading.

---

## Critical Issues

### CR-01: `resume_terminal_argv` turns shell injection into argument injection — a hyphen-leading session id is an unintended option, and the control's fixture set contains none

**File:** `src/ui/screens/detail.rs:619-632` (the argv), `:1817` (the spawn),
`src/session_detector.rs:96-113` (the source), `:7270-7293` (the control's fixture set)

**Issue:**

`resume_terminal_argv` builds:

```rust
vec![
    terminal_program_separator(term).to_string(),  // "-e" or "--"
    "claude".to_string(),
    "--resume".to_string(),
    sid.as_raw_for_logic_only().to_string(),       // untrusted, unvalidated
]
```

`sid` is scraped verbatim from another process's `/proc/<pid>/cmdline`
(`session_detector::read_session_id`, `src/session_detector.rs:96-113`). Nothing
validates its shape: the only filter is `!val.is_empty()` after a `trim()`. It may
therefore begin with `-`.

Deleting the interpreter removed the *shell* metacharacter class. It did not remove the
*option* metacharacter class, and there are still two option parsers in the path:

1. the terminal emulator, and
2. `claude` itself, whose `--resume [sessionId]` takes an **optional** value — so a
   following `-`-leading token is parsed as a new option of `claude` rather than as
   `--resume`'s argument.

A planted session id of `--dangerously-skip-permissions` (or any other `claude` flag)
therefore becomes an option the operator did not type, in a working directory
(`current_dir(&session.working_dir)`) the operator also did not choose. This is CWE-88
(argument injection), the standard successor defect to CWE-78 when a fix converts a
shell string into an argv without adding an end-of-options marker.

The function's own doc asserts the opposite, in terms that are now too strong:

> *"**The third kind no longer exists here** because there is no interpreter left in
> the path to parse anything — that is why a quote, a semicolon, a backtick, a
> dollar-parenthesis or a newline in the session id is now data."*

Every character in that list is now data. `-` is not, and it is the one that still
matters. The list is the same list as the fixture set in
`hostile_session_ids()` (`src/ui/screens/detail.rs:7270-7293`): `a'b`, `a"b`, `a;b`,
`a&&b`, `a|b`, backticks, `$( )`, newline, `\u{7}`, a bare space, and
`'; rm -rf / #`. **Not one fixture begins with a hyphen**, so
`the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element` passes today and
would pass unchanged against a build that shipped this defect. Per this phase's own
standard, a control whose fixture set omits the live case is not a certificate for it.

Exploitability caveat, stated so the rating is not read as wider than it is: the
attacker must be able to run a process named `claude` *as the same user* — a
different user's `/proc/<pid>/cwd` readlink fails and `build_session` drops the row
(`src/session_detector.rs:63`). No privilege boundary is crossed. It is rated Critical
because it is a live injection path inside the function this plan created to close
injection, because the plan's doc makes a completeness claim that is false of it, and
because the fix is one array element.

**Fix:**

```rust
fn resume_terminal_argv(term: &str, sid: &Untrusted) -> Vec<String> {
    vec![
        terminal_program_separator(term).to_string(),
        "claude".to_string(),
        "--resume".to_string(),
        // END-OF-OPTIONS. Deleting the interpreter removed the shell's
        // metacharacters; `-` is the option parser's, and `claude --resume`
        // takes an OPTIONAL value, so a hyphen-leading id is read as a new
        // flag rather than as this option's argument.
        "--".to_string(),
        sid.as_raw_for_logic_only().to_string(),
    ]
}
```

(Confirm `claude` honours `--` before the positional; if it does not, the alternative is
to refuse at the source — `read_session_id` returns `None` unless the value matches the
UUID shape `claude --resume` actually accepts — and to report the refusal rather than
silently dropping it.)

Then extend the control in both directions:

```rust
// in hostile_session_ids()
"-h",
"--dangerously-skip-permissions",
"--print",
"-",
```

plus an assertion that the untrusted element is preceded by `"--"` in the argv, so the
absence of the separator goes red rather than the presence of a quote.

---

## Warnings

### WR-01: `every_render_site_under_ui_composes_both_classes` is exercised by 6 lines in 2 of the 16 files it walks; its name claims a property it does not check

**File:** `src/ui/mod.rs:196-231` (the census), `:357-408` (the assertion)

**Issue:**

The test is named for render sites and its failure message says *"executable call sites
under src/ui/ apply the invisible-formatting half alone"*. Its implementation only ever
inspects lines that contain the literal `display_identity(`. A render site that escapes
**nothing at all** — `Span::raw(untrusted_string)` — carries no needle and is invisible
to it.

Measured at HEAD, `display_identity(` occurrences per file under `src/ui/`:

```
0  roadmap_widget.rs      0  normal.rs             0  add_project.rs
0  driver_inject.rs       0  enqueue.rs            0  driver_start.rs
0  delete_confirm.rs      0  queue_delete_confirm  0  help.rs
0  create_project.rs      0  ui/mod.rs             1  detail.rs   (doc comment only)
3  screens/mod.rs (all doc comments)               1  render_escape_guard.rs (exempt)
3  driver.rs (2 executable + 1 doc)                4  driver_confirm.rs (4 executable)
```

So the assertion is driven by **six executable lines in two files**. Fourteen files —
including the two largest render surfaces, `detail.rs` (7 462 lines) and `normal.rs` —
contribute nothing, and a new un-escaped `Span::raw` in any of them is not a census hit,
not a compile error, and caught only if a `render_escape_guard` probe state happens to
reach it.

This is a control whose coverage **shrinks as the conversion succeeds**: every site
converted from `display_identity` to `render_for_terminal` removes a line the census can
see. The `!files.is_empty()` guard at `:360` only proves the walk found files; there is
no per-file or global non-vacuity guard on the needle (contrast `driver.rs:2252-2268`,
which at least attempts one).

The residuals block at `:346-356` lists two gaps (alias/re-export; join > 4 lines). It
does not list this one, which is the largest.

**Fix:** either (a) rename and re-scope the claim to what is checked —
*`no_display_identity_call_under_ui_stands_outside_a_composition`* — and state in the
residuals that the census says nothing about a site that escapes nothing; or (b) add the
missing half: a non-vacuity guard asserting the needle appears in at least *N* files, and
route the "escapes nothing" question to a second control (widening the
`render_escape_guard` probe states is the only mechanism in the tree that can answer it).
Option (a) is honest and cheap; option (b) is the one that makes the test's current name
true.

### WR-02: the interpreter census's line-join budget is consumed by comment lines — 12 comments between the interpreter and the interpolation defeat it, and the disclosed residual points at a paragraph that was never written

**File:** `src/text.rs:1724-1760` (`interpreter_program_sites`),
`:1713-1722` (`has_an_unclosed_delimiter`), `:1785-1815` (the residual block)

**Issue (a) — comment lines burn the join budget.** In the join loop
(`src/text.rs:1739-1755`), `taken` is incremented *before* the comment check:

```rust
ahead += 1;
taken += 1;
let next = lines[ahead].1.trim();
if next.starts_with("//") {
    continue;                       // line skipped, budget still spent
}
```

I ran the census's algorithm against synthetic fixtures of exactly the CR-01 shape
(`Command::new(&term).args([ "-e", "sh", "-c", <N comment lines>, &format!(...) ])`):

| comment lines between `"-c"` and `&format!` | reported |
|---|---|
| 4  | yes |
| 10 | yes |
| 11 | yes |
| **12** | **no** |
| 13 | no |

The real CR-01 site had five. This codebase's comment blocks routinely run ten to twenty
lines — the round-10 diff itself contains dozens of comment blocks longer than twelve
lines inside expression bodies. The budget is not comfortable; it is one ordinary
comment block away from silent.

**Issue (b) — a residual is cross-referenced but not present.**
`has_an_unclosed_delimiter`'s doc (`src/text.rs:1706-1712`) says: *"Deliberately NOT a
method-chain follower … **The price is stated as a residual on the census itself.**"*
The census's residual block (`src/text.rs:1785-1815`) states three residuals — a
construction assembled across *statements*, an interpreter named by a variable, and
`project_creator::execute_hook`. The method-chain price is not among them.

That price is real and it is the most idiomatic spelling of the defect. Measured against
the census's own algorithm, all of these are missed:

```rust
// missed — method chain, interpreter and interpolation on different lines
Command::new("bash")
    .arg("-c")
    .arg(format!("echo {}", x));

// missed — assembled into a local first
let prog = format!("echo {}", x);
Command::new("sh").args(["-c", &prog]);

// missed — .concat() is not in the marker list
Command::new("sh").args(["-c", &["echo ", x].concat()]);

// missed — `+` without `&`
Command::new("sh").args(["-c", &("echo ".to_string() + x)]);
```

Only the single-physical-line form is reported. This phase's own standing rule is that a
claim certified only by prose is a finding; a residual that a doc *points at* and that
does not exist is the same failure one level down.

**Fix:** move the `taken += 1` after the comment check so skipped lines do not consume
the budget:

```rust
ahead += 1;
let next = lines[ahead].1.trim();
if next.starts_with("//") {
    continue;                       // a comment costs no budget
}
taken += 1;
logical.push(' ');
logical.push_str(next);
```

and write the residual the other doc promises, naming the method-chain shape explicitly
with the fixture above as its worked example. Widening
`interpolates_into_a_string` to include `concat(`, `join(`, `format_args!` and a
`+`-with-any-rhs marker costs nothing and closes two more of the four shapes.

### WR-03: the driver-file composition census over-joins sibling match arms, its non-vacuity guard counts a different set than the census, and one `#[cfg(test)]` attribute silently blinds it

**File:** `src/ui/screens/driver.rs:2091-2137` (`logical_lines`), `:2159-2185`
(`composition_census`), `:2252-2268` (the non-vacuity guard)

**Issue (a) — over-joining excuses a violation.** `logical_lines` merges physical lines
until paren/bracket depth returns to zero AND the line ends in `;`, `{` or `}`. In
`driver.rs` this produces logical units of up to 882 characters spanning tens of physical
lines (measured: the unit starting at `driver.rs:1603`). `composition_census` then
excuses the whole unit if *anywhere* in it a composer name appears. Reproduced with the
census's own algorithm:

```rust
let (a, b) = match kind {
    Kind::One => (
        display_identity(&sanitize_render_line(v)),   // composed
        String::new(),
    ),
    Kind::Two => (
        sanitize_render_line(v),                      // NOT composed
        String::new(),
    ),
};
```

Result: **census hits: []**. Both arms land in one logical unit; the composed arm
launders the un-composed one. Given how much of `driver.rs` is exactly this shape
(`match` over `DriverLineKind` / `TerminalState` / `RunOutcome`, all one logical unit
each), this is the likeliest way the next violation arrives. The census's residual block
(`driver.rs:2200-2205`) discloses only the opposite direction (a composition assembled
across statements reading as un-composed).

**Issue (b) — the non-vacuity guard measures a different set.** `composition_census`
truncates each file at the first `#[cfg(test)]` line (`driver.rs:2168-2172`), but
NON-VACUITY 2 (`:2252-2268`) counts needle lines over the **whole file**. Measured:

```
src/ui/screens/driver.rs          whole file: 3   production: 2
src/ui/screens/driver_confirm.rs  whole file: 4   production: 4
```

If every production call were converted away, `total > 0` would still pass on the
test-section occurrence while the census scanned nothing — which is precisely the
"nothing to find vs all composed" ambiguity the guard exists to remove.

**Issue (c) — a `#[cfg(test)]` attribute is a silent kill switch.** The truncation is at
the *first* `#[cfg(test)]` line, not at the test module. `detail.rs` already carries a
mid-file `#[cfg(test)] pub(super) fn first_string_entry` (added this round,
`src/ui/screens/detail.rs:5342-5343`), so the pattern is live in this codebase. Adding such a
helper near the top of `driver.rs` would silently exclude every render site below it,
with nothing going red.

**Fix:**
- (a) Reject on the *innermost* call rather than on the enclosing unit: extract the
  argument expression of each `sanitize_render_line(` occurrence and require a composer
  in the same parenthesised expression, not anywhere in the statement. Failing that, add
  a fixture control that asserts the two-arm shape above IS reported.
- (b) Compute `total` over `lines[..end]` — the same slice the census scans.
- (c) Truncate at the test **module** (`mod tests` after a `#[cfg(test)]`), or assert
  that exactly one `#[cfg(test)]` exists per census file so a second one goes red.

### WR-04: `EditBuffer`'s trait-absence claim is prose only — no analogue of the control the same round demanded for `Untrusted`

**File:** `src/ui/screens/mod.rs:750-757, 787-846`

**Issue:** the type's doc asserts:

> *"`Span::styled(buffer.clone(), ..)` does not compile, because this type has no
> `Display`, no `AsRef<str>` and no `Into<Cow<'static, str>>`, and the only route to a
> cell is `shown`."*

That is the same sentence `Untrusted` carried before 21-27, and 21-27 (WR-01) rated the
prose-only version a finding, built
`an_untrusted_carrier_implements_none_of_the_string_conversions`, extended it from three
absences to six, and observed each red by planting. **No equivalent control exists for
`EditBuffer`.** The committed tests for it (`what_the_operator_types_is_what_is_persisted`,
`the_edit_buffer_pushes_and_pops_whole_characters`,
`the_edit_popup_is_measured_in_characters_not_bytes`) all assert *behaviour*; none
asserts the absent trait surface. Adding `impl Display for EditBuffer` tomorrow restores
the exact laundering path CR-03 closed, with every test in the repository still green and
the doc still claiming the absence — which is verbatim the argument 21-27 used to justify
building the `Untrusted` control.

The claim is also already slightly false as written: `EditBuffer` does not derive
`Clone`, so `buffer.clone()` fails for a reason unrelated to the three named absences.

**Fix:** reuse the existing probe machinery. `text::tests::trait_probe` is already
`pub(crate)`-shaped; drive the same six absences over `EditBuffer` with the same
`String` presence arms:

```rust
#[test]
fn the_edit_buffer_implements_none_of_the_string_conversions() {
    let b = EditBuffer::seed_from_untrusted_source("demo".into());
    assert!(!implements_display!(b), "…");
    assert!(!implements_as_ref_str!(b), "…");
    assert!(!implements_into_cow_str!(b), "…");
    // + the three String presence arms, so a broken probe fails loudly
}
```

### WR-05: the backlog order is still permutation-dependent for tied entries, and the test's name says otherwise

**File:** `src/state_reader/backlog.rs:49-96` (`parse_backlog_items`), `:112-157`
(the key and comparator), `:283-330` (the control)

**Issue:** the comparator is now a genuine total order over *keys*, but
`backlog_sort_key` maps every unusable suffix to the same `0.0`
(`999.NaN`, `999.nan`, `999.inf`, `999.x`, `999.` and a literal `999.0` all tie).
`slice::sort_by` is stable, so tied elements keep the order `std::fs::read_dir`
(`backlog.rs:51`) returned them in — which is unspecified and varies by filesystem and
by directory-entry churn.

So the Backlog tab's displayed order for tied items is *still* "a function of whatever
`read_dir` happened to return first" — the exact sentence in the failure message at
`:294-297` describing the defect being fixed. The test named
`a_non_numeric_suffix_cannot_make_the_order_depend_on_the_input_permutation`
sidesteps this by asserting only the sequence of *keys* for the tied fixture
(`:312-330`), with a comment explaining why the element sequence is not asserted. The
explanation is correct about `sort_by`'s stability; it is not correct that the
user-visible property in the test's own name is achieved.

**Fix:** make the comparator total on *elements*, which costs one line and removes the
tie class entirely:

```rust
fn backlog_number_ordering(a: &str, b: &str) -> std::cmp::Ordering {
    backlog_sort_key(a)
        .total_cmp(&backlog_sort_key(b))
        // Tie-break on the raw name, so items whose suffix names no position on
        // the number line still have ONE order rather than `read_dir`'s.
        .then_with(|| a.cmp(b))
}
```

then assert the element sequence for the tied fixture too, which is what the test's name
promises.

### WR-06: the `cd '<dir>' &&` → `Command::current_dir` half of the CR-01 fix has no control and no emulator table, while the separator half of the same change got both

**File:** `src/ui/screens/detail.rs:611-616` (the claim), `:1818`, `:2224`

**Issue:** the plan recognised — correctly and at length — that handing an emulator a
real argv exposes the resumed program's options to the emulator's own parser for the
first time, wrote the `terminal_program_separator` table for it
(`src/ui/screens/detail.rs:536-568`), and pinned it with two controls
(`every_terminal_find_terminal_can_return_gets_a_separator_that_keeps_the_program_options`,
`the_separator_table_covers_every_candidate_find_terminal_probes`).

The same change also moved the working directory from *inside* the shell program
(`cd '<dir>' && …`, which executed in the final child) to `Command::current_dir` on the
**emulator client process**. That is a different level, and the doc elides it:

> *"It is set through `std::process::Command::current_dir` at the call site, which the
> process API passes to the child directly rather than as a `cd` written into a program."*

The "child" there is the emulator, not `claude`. Whether the terminal's shell inherits
that cwd is a per-emulator property: `kitty`/`alacritty`/`xterm` fork-and-exec the
program and it is inherited; `gnome-terminal` is a D-Bus-activated client that must
forward the cwd to a pre-existing server process for it to take effect. `gnome-terminal`
is one of exactly four candidates `find_terminal` probes, and it is the same candidate
the separator table had to special-case. There is no test, table or note covering the
cwd half, and the failure mode is silent: the terminal opens, the resume works, and the
session lands in `$HOME` instead of the project.

**Fix:** at minimum, extend `terminal_program_separator`'s table with a cwd column and
say per emulator how the working directory reaches the child, with the same "under-
detection, LOUD/silent" annotation the separator residual carries. If gnome-terminal is
found not to forward it, pass `--working-directory=<path>` for that stem — which is a
table entry beside the separator, not a new mechanism.

---

## Info

### IN-01: `EXEMPTIONS` are whole-file while `WAVE_PENDING` argues for exact `path:line` pinning

**File:** `src/ui/mod.rs:74-94`, `:386-391`

`WAVE_PENDING`'s doc (`:106-113`) argues that an entry pinned to one exact `path:line`
"hides at most that one line and cannot absorb a new violation elsewhere in the file".
The `EXEMPTIONS` filter one screen down uses
`site.starts_with(&format!("{}:", exemption.path))` — a whole-file match over
`render_escape_guard.rs` (3 128 lines). The two mechanisms sit in the same module with
opposite conventions and the inconsistency is unexplained. The exemption is defensible
today (the file is `#[cfg(test)]`-gated, `src/ui/screens/mod.rs:20-21`), but the pinning
argument applies to it verbatim.

**Fix:** pin the exemption to `src/ui/screens/render_escape_guard.rs:2615`, or state in
`EXEMPTIONS`' doc why whole-file is right here and pinned is right there.

### IN-02: dead assertion in `the_separator_table_covers_every_candidate_find_terminal_probes`

**File:** `src/ui/screens/detail.rs:7457`

```rust
assert!(matches!(separator, "-e" | "--"), "{candidate:?} resolved to the unknown separator {separator:?}");
```

`terminal_program_separator` is a two-arm `match` returning `&'static str` literals
`"--"` and `"-e"`; the assertion cannot fail. The load-bearing part of the test is the
`candidates.len() == 4` equality above it.

**Fix:** delete the arm, or replace it with the assertion that has teeth — that each
candidate resolves to the separator the table's markdown row records for it.

### IN-03: "Resumed session {}" is reported on `spawn()` returning, not on the resume working

**File:** `src/ui/screens/detail.rs:1820-1826`, `:2226-2232`

`Command::spawn()` succeeds as soon as the emulator binary is exec'd. An emulator that
rejects the separator or the flags exits immediately (and, having no terminal yet, shows
the operator nothing), while the TUI reports success. `terminal_program_separator`'s doc
claims the unsupported-emulator residual is *"LOUD rather than silent — the emulator
rejects the flag and the operator sees the failure"*; with the status line reporting
success and the emulator's stderr going nowhere, the operator sees a flash and a green
message. Pre-existing shape, but the round-10 doc now rests a residual's direction on it.

### IN-04: `ArchiveDepth::milestone` is the same untyped round trip 21-30 closed for `defaults_text_buffer`

**File:** `src/ui/screens/detail.rs:1866` (raw take), `:4024-4079` (re-escape at render)

`archive_milestones` is `Vec<Untrusted>`; navigating takes
`.as_raw_for_logic_only().to_string()` into `ArchiveDepth::PhaseList { milestone: String }`,
and `archive_breadcrumb` escapes it again with the local `shown()` helper
(`detail.rs:77-79`). Structurally identical to the CR-03 laundering, correctly escaped at
each of the three render sites, honestly disclosed at `:4024-4032` with its direction
("under-protection, silent"), and covered by the archive probe states. Named here so the
one remaining instance of the pattern the round made a defect class is on the record; it
is a candidate for the same `EditBuffer`-style retype next time `crate::archive` is open.

### IN-05: `interpolates_into_a_string`'s marker set is narrower than its own doc

**File:** `src/text.rs:1698-1704`

The doc says *"The formatting macros this codebase builds strings with, plus `&`-string
concatenation."* The list is `["format!", "write!", "writeln!", "+ &", "push_str(&"]`.
`+ &` matches only that exact spacing (`a+&b` and a `+` at a line break both miss), and
`String + &str` where the right-hand side is already a `&str` variable is written `s + x`
with no `&` at all. `concat!`, `.concat()`, `.join()`, `format_args!` and `.replace()` are
absent. See WR-02 for measured misses.

### IN-06: `EditBuffer::push_char` / `pop_char` reallocate the whole buffer per keystroke

**File:** `src/ui/screens/mod.rs:805-818`

Each edit does `as_raw_for_logic_only().to_string()` → mutate → `from_untrusted_source`,
copying the buffer twice per keypress. Harmless at config-value lengths and out of the
declared v1 performance scope, but it is a shape forced by `Untrusted` exposing no
mutator; a `Untrusted::into_raw(self) -> String` (a by-value take, no more permissive
than the existing accessor) would let the wrapper mutate in place without adding a
borrow-shaped escape hatch.

---

## Verification notes

- Every count- or presence-bearing check in this review was run through `rtk proxy`.
- The three censuses were exercised by re-implementing their published algorithms and
  running them against (a) the pre-fix `detail.rs` from `80bc4c1` and (b) synthetic
  fixtures; the pass/fail tables in WR-02 and WR-03 are measurements, not readings.
- `tests/driver_reattach.rs`'s two nondeterministic tests were not run and are not
  reported, per scope.
- The four known clippy lints (`browser.rs:155-157`, `project_creator.rs:146`) are
  out of scope and were confirmed to be the only ones.

---

_Reviewed: 2026-08-27_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
