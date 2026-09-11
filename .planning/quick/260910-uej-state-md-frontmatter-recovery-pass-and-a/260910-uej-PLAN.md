---
phase: quick-260910-uej
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/state_md.rs
  - src/state_reader/mod.rs
  - src/app.rs
  - src/ui/screens/detail.rs
  - tests/state_reader_test.rs
autonomous: true
requirements: [260910-uej]

estimate:
  tokens: 85000
  raw_tokens: 85000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "A STATE.md whose only fault is a top-level unquoted plain scalar containing a colon-space sequence parses, and the value round-trips intact — colons, em-dashes and all."
    - "Recovery is a THIRD outcome, visible in the UI and distinguishable from both a clean parse and an unreadable file — never a silent success."
    - "A file still unreadable after the repair attempt names its YAML fault POSITION as FILE-relative line/column numbers; no parser message text reaches any rendered string."
    - "Every currently-valid STATE.md produces a byte-identical outcome with the repair path in place."
    - "Multi-byte content in a repaired or reported value causes no byte-index panic."
    - "No file outside this repository is written; the repair is in-memory only."
  artifacts:
    - src/state_reader/state_md.rs
    - src/state_reader/mod.rs
    - src/app.rs
    - src/ui/screens/detail.rs
    - tests/state_reader_test.rs
  key_links:
    - "read_frontmatter's serde_yml error branch -> repair_frontmatter_block -> single reparse -> FrontmatterOutcome::Recovered"
    - "body-relative YAML line -> FILE-relative line via the measured delimiter offset (body line 1 is file line 2 in sentriq's file)"
    - "ProjectState's new fields are NON-String by construction, so tests/spawn_seam_guard.rs's free_string_fields census stays satisfied without widening THIRD_PARTY_STRINGS"
    - "parse_project_state's if-let pair becomes an exhaustive match, so a fourth variant is a compile error rather than a silent skip"
---

<objective>
`/home/blk/projects/flutter/sentriq/.planning/STATE.md` shows `! STATE.md unreadable` because one
frontmatter line is a 1276-char unquoted plain YAML scalar containing colon-space. Give the reader a
single conservative quote-in-place repair pass, surface the repair as a distinct degraded state, and
— when the file is still broken — report the YAML fault position as numbers.

Purpose: a recoverable file should read (and say it was repaired); an unrecoverable one should say
*where* it broke instead of just *that* it broke.
Output: a repair pass + a `Recovered` outcome + a numeric fault position, all covered by tests, with
the real sentriq file confirmed READ-ONLY.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md

@src/state_reader/state_md.rs
@src/state_reader/mod.rs
@src/app.rs
@src/ui/screens/detail.rs
@src/driver/untrusted.rs
</context>

<measured_facts>
Everything below was MEASURED at planning time against the installed crates and the real file. Do
not re-derive it; do verify it still holds if something contradicts you.

1. **`untrusted.rs` is at `src/driver/untrusted.rs`**, not `src/state_reader/untrusted.rs`. The rule
   it enforces is already cited verbatim in `state_md.rs:360-368` and `mod.rs:100-107`: a serde
   error quotes the document that produced it, so the parser message goes to `tracing::warn!` and
   **stops there**; the UI carries only a classification this crate wrote.

2. **`tests/spawn_seam_guard.rs:709-773` is a two-directional census.** `free_string_fields` scans
   `pub struct ProjectState {` in `src/state_reader/mod.rs` and collects every field whose declared
   type is exactly `String`, `Option<String>` or `Vec<String>`, then diffs that set both ways
   against `THIRD_PARTY_STRINGS` in `src/driver/untrusted.rs`. Adding a field of one of those three
   types to `ProjectState` FAILS that guard unless the census is widened. This is why the new
   fields in Task 1 and Task 2 are typed rather than stringly — see D-02.

3. **`serde_yml` 0.0.13 is a deprecated shim re-exporting `noyalib::compat::serde_yaml`.** The
   error type carries `pub fn location(&self) -> Option<Location>`, and `Location` exposes
   `line()`, `column()` (both **1-based**) and `index()` (0-based bytes). `location()` returns
   `None` for several variants, so the position is genuinely optional.

4. **The real failure, measured:**
   `YAML parse error at line 6, column 218: mapping values are not allowed in this context`,
   `location()` = `Some(line=6, column=218, index=333)`.
   **Line 6 is BODY-relative.** The body handed to `serde_yml::from_str` starts after the opening
   `---`, so body line 1 is file line 2 and the fault is at **file line 7** — the number the label
   must show.

5. **The quote-in-place repair was proved to work on the real file:** re-emitting only the
   `stopped_at` line with a double-quoted value makes the whole document parse; `stopped_at` comes
   back at 1270 bytes, `status` is `"planning"`, `current_phase` is `Integer(9)`, and the
   already-quoted 1622-byte `last_activity_desc` is untouched and intact.

6. **sentriq's frontmatter, line by line (measured):** exactly ONE offending line — file line 7
   (`stopped_at`, plain, colon-space). File line 8 (`last_updated`) and file line 10
   (`last_activity_desc`) are ALREADY QUOTED, and line 10 also contains a colon-space inside its
   quotes. File line 12 is `progress:` (a key with an empty value, opening a nested mapping) and
   lines 13-16 are indented nested keys. The "leave already-quoted, block, flow, list, comment,
   empty-value and indented lines alone" rule is load-bearing on this very file.

7. **Pre-existing test failures:** the `driver_reattach` pair fails on `master` and is documented in
   deferred items. `cargo test` fail-fast stops at that binary and never reaches the `envelope_*`
   suites, so the gate command is `--no-fail-fast`. Do not chase those two.

8. `ProjectState` derives `Debug, Clone, Default, PartialEq` (`mod.rs:13`). New field types must
   satisfy those; wrapping in `Option<..>` covers `Default`.
</measured_facts>

<inferred_decisions>
The human operator is unavailable. These were decided from the code's own conventions and are
flagged for later audit.

- **D-01 (inferred): recovery is a FOURTH enum variant on `FrontmatterOutcome`, not a boolean on
  `Parsed`.** The type's own doc says `Absent` and `Unreadable` are separate *on purpose* because
  collapsing outcomes is what let the original bug ship. A flag on `Parsed` would let a caller that
  never reads the flag treat a repaired file as clean; a variant makes every exhaustive match site a
  compile error until it has classified the new state. This matches the repo's standing
  "no wildcard, a new variant is a compile error" convention (STATE.md, 18-10).

- **D-02 (inferred): the fault position and the recovery signal are carried as NON-String types.**
  Two independent reasons converge: the `untrusted.rs` rule forbids third-party text in the UI
  (numbers only), and `spawn_seam_guard.rs`'s census (measured fact 2) turns any new
  `String`/`Option<String>`/`Vec<String>` field on `ProjectState` into a guard failure. A `Copy`
  position struct and a `bool` satisfy both without widening `THIRD_PARTY_STRINGS`.

- **D-03 (inferred): `FrontmatterFault` is left exactly as it is.** Its doc defends being a pure
  classification, `describe()` returns `&'static str`, and `tests/state_reader_test.rs:823` asserts
  on its equality. The position rides beside it on the `Unreadable` variant rather than inside it,
  so none of that changes meaning.

- **D-04 (inferred): out-of-range positions are DROPPED, not clamped to a magic constant.** A line
  number is bounded by the block it came from, so the check is `line <= <lines in the frontmatter
  block>` — a validity test against measured reality rather than an invented ceiling. This also
  bounds the rendered label width by the file's own line count. `noyalib` takes the same posture
  ("out-of-range lines fall back to plain Display").

- **D-05 (inferred): the real sentriq file is exercised by an `#[ignore]`d, env-var-addressed,
  READ-ONLY test — never by a committed absolute path.** The repo's convention
  (`picsync_shaped_planning_dir`, `state_md.rs:669`) is to commit the *shape* and let real-file
  reads be opt-in and self-skipping.
</inferred_decisions>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: End-to-end "a repairable STATE.md reads, and says it was repaired" — one path only</name>
  <files>src/state_reader/state_md.rs, src/state_reader/mod.rs, src/app.rs, src/ui/screens/detail.rs</files>
  <read_first>
    - src/state_reader/state_md.rs:342-474 (the outcome enum, the fault enum and `read_frontmatter`)
    - src/state_reader/mod.rs:90-108 (the `state_md_unreadable` / `state_md_fault` field pair and its doc)
    - src/state_reader/mod.rs:341-365 (`parse_project_state`'s two `if let` reads of the outcome)
    - src/app.rs:368-400 (`UNREADABLE_STATE_LABEL` and `format_phase_display`)
    - src/ui/screens/detail.rs:81-103 (`unreadable_state_line`, the banner that already renders `describe()`)
  </read_first>
  <behavior>
    - `repair_frontmatter_block("stopped_at: a. Earlier: b\n")` returns a changed block whose
      `stopped_at` reparses to exactly `a. Earlier: b`.
    - A line already carrying a quoted value (`k: "x: y"`) is returned unchanged.
    - A block scalar header (`k: |`, `k: >`), a flow collection (`k: [a, b]`, `k: {a: 1}`), a list
      item (`- x`), a comment (`# ...`), an empty value (`k:`) and an indented nested key
      (`  k: v`) are each returned unchanged.
    - A value containing `"` and `\` round-trips through the repair unchanged.
    - `read_frontmatter` on a block with no colon-space fault returns `Parsed`, never `Recovered`.
    - `read_frontmatter` on the sentriq shape returns `Recovered`.
    - A `ProjectState` whose STATE.md was recovered renders a phase cell that is neither the plain
      phase label nor `UNREADABLE_STATE_LABEL`.
  </behavior>
  <action>
Wire ONE path from the parse failure through to the rendered cell. No second repair strategy, no
retry loop, no other call sites.

**1. The repair, in `src/state_reader/state_md.rs`.** Add a private
`repair_frontmatter_block(yaml: &str) -> Option<String>`. It walks the block line by line and
returns `Some(rewritten)` only when it changed at least one line, `None` otherwise. A line is a
repair candidate only when ALL of these hold — anything uncertain is left exactly as-is:
  - the line has no leading whitespace (a top-level mapping entry; an indented key is out of scope);
  - it does not start with `#` or `-`;
  - it splits on the FIRST `:` followed by a space or end-of-line, into a key and a value;
  - the key is a plain unquoted identifier-shaped token (no quote character, no leading `[` or `{`);
  - the value, after trimming, is NON-EMPTY (so `progress:` is never touched);
  - the value's first character is not a quote (`"` or `'`), not a block-scalar indicator (`|`, `>`),
    not a flow opener (`[`, `{`), not an anchor/alias/tag sigil (`&`, `*`, `!`);
  - and the value is not a legal plain scalar: it contains a colon-space sequence, or it ends with a
    bare colon.
A candidate line is re-emitted as the original key, a colon, a space, and the value wrapped in a
YAML double-quoted scalar. Build that scalar with a helper that escapes a backslash as two
backslashes, a double quote as backslash-quote, and every control character as its YAML escape
(`\n`, `\r`, `\t`) or a `\xNN` form. Operate on `char`s, never on byte indices — this value is
third-party text and a multi-byte character is exactly what arrives without warning
(`untrusted.rs::bounded` documents this defect class).

**2. The outcome, same file.** Add a fourth variant to `FrontmatterOutcome`:
`Recovered { frontmatter: Box<StateFrontmatter>, repaired_lines: u32 }`. Document it as the third
state the enum's existing doc argues for: not clean, not unreadable, and never silently either. Do
NOT touch `FrontmatterFault` (per D-03).

**3. The single retry, in `read_frontmatter`.** In the `Err(e)` arm at the `serde_yml::from_str`
call, keep the existing `tracing::warn!` of the full parser message (that is the only place the
message is allowed to go), then call `repair_frontmatter_block`. On `Some(repaired)` reparse ONCE.
If the reparse yields a mapping, build the same `StateFrontmatter` the success path builds and
return `Recovered`; factor the mapping-to-`StateFrontmatter` construction into one private helper so
the two paths cannot drift. If the repair returns `None`, or the reparse fails, or it yields a
non-mapping, fall through to the existing `Unreadable(InvalidYaml)` behaviour unchanged.

**4. `parse_state_md`** already matches exhaustively — give `Recovered` its own arm returning the
frontmatter, so a repaired file is readable through the legacy entry point.

**5. The signal on `ProjectState`, in `src/state_reader/mod.rs`.** Add
`pub state_md_recovered: bool` beside `state_md_unreadable`, documented in the same register: the
frontmatter was read only after an in-memory repair, the file on disk is unchanged, and the value
shown may differ from what a strict reader would accept. It is a `bool`, not a message — per D-02.

**6. Make the read exhaustive.** Replace the two sequential `if let` reads of `outcome` at
`mod.rs:358-364` with ONE `match` over all four variants and no wildcard arm. The `Recovered` arm
sets `state_md_recovered = true` and then populates every field the `Parsed` arm populates; extract
that population into a local closure or a private fn so the two arms cannot diverge. This is the
point of D-01: the `if let` pair would have skipped a fourth variant in silence.

**7. The cell, in `src/app.rs`.** Leave `UNREADABLE_STATE_LABEL` and its value untouched — an
existing test asserts it by equality. Add `pub const RECOVERED_STATE_MARKER: &str = "~ ";` and, in
`format_phase_display`, after the `state_md_unreadable` early return, prefix the marker onto whatever
label the function would otherwise have produced when `state.state_md_recovered` is set. Document
why a marker and not a silent success: the value behind the label was repaired by this tool, so a
reader who cannot tell has been told something slightly false about the repository — the same
argument `TRUNCATION_MARKER` makes in `untrusted.rs`.

**8. The detail pane, in `src/ui/screens/detail.rs`.** Beside `unreadable_state_line`, add a
`recovered_state_line` returning `Some` only when `state.state_md_recovered`, with a fixed
this-crate-authored phrase naming the repair (a yellow/degraded style rather than the banner's red,
because this is a condition rather than a fault). Render it at the same position the unreadable
banner uses. The two are mutually exclusive by construction; do not assert that here, Task 3 does.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib state_reader::state_md</automated>
    <automated>rtk proxy cargo build</automated>
  </verify>
  <done>The sentriq SHAPE (`stopped_at: a. Earlier: b`) reaches `FrontmatterOutcome::Recovered` with the value intact, `ProjectState.state_md_recovered` is set, the phase cell carries the marker, the detail pane carries the line, and every listed no-op case is returned unchanged. `cargo build` is clean.</done>
  <reversibility rating="reversible">An enum variant, a bool field and a render prefix; all removable in one revert with no data migration and nothing written to disk.</reversibility>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Carry the YAML fault position — FILE-relative numbers, nothing else</name>
  <files>src/state_reader/state_md.rs, src/state_reader/mod.rs, src/app.rs, src/ui/screens/detail.rs</files>
  <read_first>
    - src/driver/untrusted.rs:1-64 and 262-296 (the boundary's own statement of what may cross it, and the char-not-byte rule)
    - src/state_reader/state_md.rs:403-429 as it stands after Task 1 (the error arm that now owns both the repair attempt and the fall-through)
  </read_first>
  <behavior>
    - `read_frontmatter("---\nstatus: [unclosed\n---\n")` stays `Unreadable` and reports line 2
      (FILE-relative: the body's line 1 is the file's line 2).
    - The sentriq shape, made unrecoverable, reports file line 7.
    - A fault the parser gives no location for yields `None` and the label falls back to the bare
      `UNREADABLE_STATE_LABEL` unchanged.
    - A reported line beyond the frontmatter block's own line count is dropped, not rendered.
    - `format_phase_display` for an unreadable state with a position renders
      `! STATE.md unreadable (line 7)` and contains no parser vocabulary.
  </behavior>
  <action>
**1. The type, in `src/state_reader/state_md.rs`.** Add
`pub struct FrontmatterFaultPosition { pub line: u32, pub column: u32 }` deriving
`Debug, Clone, Copy, PartialEq, Eq`. Its doc states the rule it exists to honour: these are the only
two things the parser knows that may cross into a rendered string, because a number cannot carry an
instruction and the message it came from quotes the third-party document — the reasoning
`FrontmatterFault`'s own doc already makes, one field over.

**2. The variant.** Change `FrontmatterOutcome::Unreadable(FrontmatterFault)` to the struct form
`Unreadable { fault: FrontmatterFault, position: Option<FrontmatterFaultPosition> }` and update the
existing `Unreadable(_)` patterns (there are several in this file's test module and one in
`parse_state_md`) to `Unreadable { .. }`. Every non-YAML fault — `Unterminated`, `NotAMapping` —
passes `position: None`; only a `serde_yml` error can supply one.

**3. The offset, in `read_frontmatter`.** The location `serde_yml` reports is BODY-relative
(measured fact 4). Compute the file-relative line as follows, and write the derivation down:
  - the body slice handed to `from_str` is already in hand; take `content.len() - body.len()` as the
    byte length of everything before it (the leading whitespace, the opening `---` and its newline);
  - count the `\n` in that prefix and add 1 — that is the FILE line of body line 1;
  - the file line of the fault is `body_line + that - 1`.
Do this arithmetic in `u64`/`usize` and convert down with `u32::try_from(..).ok()`, dropping the
position on overflow rather than wrapping. The column is passed through unchanged: it is an offset
within a line, not a line index, so no translation applies.

**4. The validity gate (D-04).** Drop the position — yield `None` — when the reported line is 0 or
exceeds the number of lines in the frontmatter block plus the offset. A line number that cannot
name a line in the file it came from is not information, and dropping it also bounds the rendered
label's width by the file's own line count. Do not clamp to an invented ceiling.

**5. The carrier on `ProjectState`, in `src/state_reader/mod.rs`.** Add
`pub state_md_fault_position: Option<FrontmatterFaultPosition>` beside `state_md_fault`, set from
the `Unreadable` arm of the match Task 1 made exhaustive. Extend the existing doc-comment argument
at `mod.rs:100-107`: that comment currently says a parser message would have been held "for the sake
of a line number" — the line number is now held WITHOUT the message, which is the distinction the
comment was drawing. Note in the doc that the field is deliberately not a `String`, and why
(`tests/spawn_seam_guard.rs`'s census plus the `untrusted.rs` rule — D-02).

**6. The label, in `src/app.rs`.** Keep `UNREADABLE_STATE_LABEL` as the bare form. Add
`pub fn unreadable_state_label(position: Option<FrontmatterFaultPosition>) -> String` returning the
bare constant when `position` is `None` and `format!("{UNREADABLE_STATE_LABEL} (line {})", line)`
when it is `Some`. Call it from `format_phase_display`'s early return. The column is NOT rendered in
the cell — the cell is width-constrained and a line number is what a user acts on; it is carried on
the state and rendered in the detail pane instead.

**7. The detail pane, in `src/ui/screens/detail.rs`.** In `unreadable_state_line`, append
` (line {line}, column {column})` to the existing `describe()` banner when a position is present.
Interpolate the two `u32`s directly; do NOT route them through `shown`/`render_for_terminal`, and
say why in a comment beside the existing one that makes the same point about `describe()`: these are
integers this crate computed, not text the repository wrote.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib state_reader::state_md</automated>
    <automated>rtk proxy cargo test --lib app::tests</automated>
    <automated>rtk proxy cargo clippy -- -D warnings</automated>
  </verify>
  <done>`read_frontmatter("---\nstatus: [unclosed\n---\n")` is `Unreadable` with `position` naming file line 2; the sentriq shape made unrecoverable names file line 7; a location-less error yields `None`; the cell reads `! STATE.md unreadable (line 7)`; no `serde_yml`/parser message text appears in any value reachable from `ProjectState` or a rendered `Line`. Clippy is clean at `-D warnings`.</done>
  <reversibility rating="reversible">A struct-form variant and one `Option` field; the render change is additive and the bare label constant is unchanged.</reversibility>
</task>

<task type="auto">
  <name>Task 3: The proofs — no-op, UTF-8, the census, and the real file read-only</name>
  <files>tests/state_reader_test.rs, src/state_reader/state_md.rs</files>
  <read_first>
    - src/state_reader/state_md.rs:505-512 (the `REAL_GSD_STATE_MD` fixture and the comment explaining why it exists)
    - src/state_reader/state_md.rs:616-683 (the outcome-distinguishing test and the read-the-repo's-own-STATE.md test whose self-skipping shape Task 3 reuses)
    - tests/state_reader_test.rs:795-845 (the integration-level `state_md_unreadable` / `state_md_fault` assertions)
  </read_first>
  <action>
**1. The sentriq shape, as a committed fixture** in `state_md.rs`'s test module, beside
`REAL_GSD_STATE_MD`. Name it for what it reproduces and carry the four structural features measured
on the real file (measured fact 6), reduced: a plain `stopped_at` whose value contains
`. Earlier: ` and at least two em-dashes; an ALREADY-QUOTED sibling key whose quoted value also
contains a colon-space (the line that must not be touched); a bare `progress:` opening an indented
nested mapping; and a plain numeric `current_phase`. Assert:
  - the outcome is `Recovered` with `repaired_lines == 1`;
  - `stopped_at` round-trips **exactly**, asserted against the fixture's own literal rather than a
    substring test, so a repair that mangled or truncated the value fails here;
  - the em-dashes survive;
  - the already-quoted sibling's value is intact and unchanged;
  - `current_phase` still reads as `"9"` (the repair did not quote a number into a different shape);
  - the nested `progress` counts still arrive.

**2. The no-op proof.** A test that runs every already-valid fixture in this module —
`REAL_GSD_STATE_MD` at minimum, plus the repo's own `.planning/STATE.md` read through the same
self-skipping shape `the_real_planning_state_md_of_this_repository_parses` uses — and asserts the
outcome is `Parsed`, never `Recovered`. Then assert the stronger property directly:
`repair_frontmatter_block` returns `None` for each of those blocks, so the repair is a no-op on them
by the function's own contract and not merely because the error branch was never entered. Make the
`None` assertion the one that carries the argument in the failure message.

**3. The still-unrecoverable case.** `read_frontmatter("---\nstatus: [unclosed\n---\n")` stays
`Unreadable` AND reports file line 2. Add a second arm on the sentriq shape mutated so the repair
cannot help (for example an unclosed flow collection on the same line as the colon-space value), and
assert both that it stays `Unreadable` and that the reported line is the line the fixture put the
fault on — counted in the FIXTURE, so the offset arithmetic is pinned rather than restated.

**4. Multi-byte safety.** A fixture whose repairable value is built from em-dashes and a 4-byte
character positioned so that a naive byte slice at the key/value split or at any escape boundary
would land mid-character. Follow the precedent in
`untrusted.rs::truncation_inside_a_multibyte_character_lands_on_a_character_boundary`: assert the
PRECONDITION that the naive byte index is not a char boundary first, so the test cannot pass
vacuously against a byte-indexing implementation, then assert the repair produces the value intact.

**5. The escape round-trip.** A repairable value containing a literal `"` and a literal `\` beside
the colon-space, asserted to come back byte-identical. Without this, an escape bug shows up as a
reparse failure that silently degrades to `Unreadable` and looks like "not recoverable".

**6. The integration arm**, in `tests/state_reader_test.rs`: a `TempDir` `.planning/` whose STATE.md
is the sentriq shape, parsed through `parse_project_state`, asserting `state_md_recovered` is true,
`state_md_unreadable` is false, `state_md_fault` is `None`, and the real `status` and
`current_phase` arrived. Add the mutually-exclusive assertion Task 1 deferred: for each of the three
fixtures (clean / recovered / unreadable), at most one of `state_md_recovered` and
`state_md_unreadable` is set.

**7. The real-file confirmation (D-05).** An `#[ignore]`d test in `tests/state_reader_test.rs` that
reads a path from an environment variable, returns early when the variable is unset, opens the file
READ-ONLY with `std::fs::read_to_string`, and prints the outcome plus the recovered `stopped_at` (or
the fault and its position) via `println!`. It must contain no write, no create and no absolute
path literal. Run it against
`/home/blk/projects/flutter/sentriq/.planning/STATE.md` with `-- --ignored --nocapture` and report
the raw output in the summary, including whether it recovered and what `stopped_at` came back as.

**8. The census check.** `tests/spawn_seam_guard.rs` diffs `ProjectState`'s
`String`/`Option<String>`/`Vec<String>` fields against `THIRD_PARTY_STRINGS` in BOTH directions
(measured fact 2). The new fields were typed specifically to stay outside that set (D-02). Run that
binary and confirm it is green; if it is not, the fix is to correct the FIELD TYPE back to a
non-string carrier, never to widen the census — widening it would mean third-party text is now on
`ProjectState`, which is the property this plan must not break.

**9. The gate.** Run all three commands below and report their RAW output in the summary — no pipe
into `grep`, `head`, `tail` or `awk` for the pass/fail determination, because `rtk` re-applies
filtering downstream of a pipe and a filtered stream turns a real regression into a vacuous pass.
`--no-fail-fast` is mandatory: a plain `cargo test` stops at the pre-existing `driver_reattach`
failure and never reaches the `envelope_*` suites, so the total is identical no matter what changed.
The `driver_reattach` pair is expected to fail; anything else is this plan's.
  </action>
  <verify>
    <automated>rtk proxy cargo build</automated>
    <automated>rtk proxy cargo test --no-fail-fast</automated>
    <automated>rtk proxy cargo clippy -- -D warnings</automated>
    <automated>rtk proxy cargo test --test spawn_seam_guard</automated>
  </verify>
  <done>All four commands report their raw results; `cargo clippy -- -D warnings` exits 0; `cargo test --no-fail-fast` shows no failure other than the documented `driver_reattach` pair and a test count strictly above the pre-change baseline; `spawn_seam_guard` is green with `THIRD_PARTY_STRINGS` UNCHANGED; and the `#[ignore]`d read-only test has been run against the real sentriq STATE.md with its raw output quoted in the summary.</done>
  <reversibility rating="reversible">Test-only additions plus one fixture constant.</reversibility>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| foreign `.planning/STATE.md` -> `read_frontmatter` | third-party bytes, changeable under a `git pull` the user never read, enter the parser |
| `serde_yml` error -> `ProjectState` -> rendered `Line`/cell | the one place a parser-authored string could cross into the UI |
| repaired block -> reparse -> `StateFrontmatter` | this tool rewrites third-party content in memory and then believes the result |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-UEJ-01 | Information disclosure | `read_frontmatter` error arm -> `app::format_phase_display`, `detail::unreadable_state_line` | high | mitigate | Only `u32` line/column cross the boundary (Task 2 steps 1-7). The full parser message stays in the existing `tracing::warn!` and nowhere else. `FrontmatterFault::describe()` remains the only rendered prose and remains `&'static str`. |
| T-UEJ-02 | Denial of service | `repair_frontmatter_block`, the escape helper, the position gate | medium | mitigate | All string work is over `char`s, never byte indices (Task 1 step 1) — the `untrusted.rs::bounded` defect class, pinned by the Task 3 step 4 precondition assertion. Position arithmetic uses `u32::try_from(..).ok()` and drops on overflow; an out-of-range line is dropped, bounding the label's width by the file's own line count (D-04). |
| T-UEJ-03 | Tampering | the repair pass | medium | mitigate | The repair is IN MEMORY and runs ONLY in the parse-error branch; nothing is written to disk and no file outside this repo is opened for anything but reading. The candidate rule leaves quoted, block, flow, list, comment, empty-value and indented lines untouched; Task 3 step 2 proves `repair_frontmatter_block` returns `None` on every valid fixture, so a valid file's semantics cannot be altered. |
| T-UEJ-04 | Spoofing | the recovered state | medium | mitigate | A repaired file must not present as a clean one. `Recovered` is a distinct variant (D-01), `state_md_recovered` is a distinct field, the cell carries a marker and the detail pane a line — the `TRUNCATION_MARKER` argument applied to a document instead of a string. |
| T-UEJ-05 | Elevation of privilege | `ProjectState`'s string surface vs `THIRD_PARTY_STRINGS` | medium | mitigate | New fields are typed (`bool`, `Option<FrontmatterFaultPosition>`) so the two-directional census in `tests/spawn_seam_guard.rs` stays satisfied with `THIRD_PARTY_STRINGS` unchanged (D-02, Task 3 step 8). Widening the census is explicitly forbidden as the remedy. |
| T-UEJ-SC | Tampering | npm/pip/cargo installs | n/a | accept | This plan adds NO dependency. `Cargo.toml` is not in `files_modified`; `serde_yml`, `noyalib` and every other crate involved are already resolved in `Cargo.lock`. No package-legitimacy gate applies. |
</threat_model>

<verification>
- `rtk proxy cargo build` — clean.
- `rtk proxy cargo test --no-fail-fast` — raw output reported; only the documented `driver_reattach`
  pair fails; the passed count exceeds the pre-change baseline by the number of tests added.
- `rtk proxy cargo clippy -- -D warnings` — exit 0.
- `rtk proxy cargo test --test spawn_seam_guard` — green, with `src/driver/untrusted.rs` unmodified.
- `git diff --stat` shows no change under any path outside this repository, and no change to
  `src/driver/untrusted.rs` or `Cargo.toml`.
- The `#[ignore]`d real-file test, run against the sentriq path with `-- --ignored --nocapture`,
  reports `Recovered` and a `stopped_at` beginning `Completed 260910-p8v (DTC history is
  vehicle-scoped` and ending in the measured 1270-byte value.
</verification>

<success_criteria>
- sentriq's STATE.md recovers rather than reading as unreadable, with `stopped_at` intact.
- The recovery is visible in both the dashboard cell and the detail pane, and is a distinct third
  state — not a silent success and not the unreadable banner.
- A still-broken file's cell names its FILE-relative fault line; no parser message text is reachable
  from any rendered string.
- Every valid STATE.md is byte-identically unaffected, proved by `repair_frontmatter_block`
  returning `None` on each rather than by the error branch merely not firing.
- No byte-index panic path on multi-byte content, proved by a fixture whose naive byte index is
  asserted NOT to be a char boundary.
- `THIRD_PARTY_STRINGS` unchanged; `spawn_seam_guard` green.
</success_criteria>

<output>
Create `.planning/quick/260910-uej-state-md-frontmatter-recovery-pass-and-a/260910-uej-SUMMARY.md` when done.
Record the five inferred decisions D-01..D-05 in the summary under a heading marking them for later
audit — the operator was unavailable and did not approve them.
</output>
