---
phase: 21-llm-goal-layer-prompt-injection-hardening
round: 8
reviewed: 2026-08-25T00:00:00Z
depth: deep
diff_base: f1a9d0d
head: 458a24a
previous_round: "Round 7's 21-REVIEW.md is preserved in git at f1a9d0d (`git show f1a9d0d:.planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-REVIEW.md`). Round 6's is at f1faa3e. This file replaces round 7's."
files_reviewed: 20
files_reviewed_list:
  - src/main.rs
  - src/registry.rs
  - src/text.rs
  - src/envelope/advisory.rs
  - src/envelope/hooks.rs
  - src/ui/roadmap_widget.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/render_escape_guard.rs
  - src/ui/screens/add_project.rs
  - src/ui/screens/create_project.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/driver_inject.rs
  - src/ui/screens/driver_start.rs
  - src/ui/screens/enqueue.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/queue_delete_confirm.rs
  - tests/spawn_seam_guard.rs
files_read_but_unchanged_this_round:
  - src/error.rs
  - src/app.rs
  - src/test_support.rs
  - src/state_reader/git_ops.rs
  - src/journal/mod.rs
  - src/ui/screens/help.rs
deleted_this_round:
  - src/ui/project_list.rs
planning_artifacts_reviewed:
  - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-19-PLAN.md (correction block)
  - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-19-SUMMARY.md (correction block)
  - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-21-SUMMARY.md
  - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-22-SUMMARY.md
  - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md
findings:
  critical: 5
  warning: 5
  info: 4
  total: 14
status: issues_found
---

# Phase 21 Round 8: Code Review Report

**Reviewed:** 2026-08-25
**Depth:** deep (cross-file: census reach, render call chains, binary-level CLI runs, independent dependency re-measurement)
**Range:** `f1a9d0d..458a24a` (15 commits, plans 21-21 and 21-22)
**Status:** issues_found — 5 Critical, 5 Warning, 4 Info

## Summary

**The mechanism round 8 built is genuinely new and genuinely works. The sampling that
decided where to point it did not move, and that is the eighth re-enactment.**

What is real, and I verified each of these myself rather than reading the SUMMARY:

* The `Screen` census IS a `read_dir` walk. I planted `TwelfthScreenNobodyAdjudicated`
  in `src/state_reader/backlog.rs` — a file neither plan touches, in a directory the
  module's own doc never names — and `the_screen_census_matches_the_tree` went red with
  the file path in the message, nobody having edited anything. Reverted; tree clean.
* The probe IS behavioural and IS blind to sink spelling. I planted a raw identity into
  `NormalScreen`'s outer `Block::title` via `format!` — a spelling nothing in the tree
  enumerates — and `the_screen_renders_identity_escaped` went red naming the screen and
  the state. Reverted; tree clean.
* D-17-3's accept/echo split holds at the binary. Against a hand-built legacy
  `config.json` carrying `gsd-\u{202e}nur`: `list` prints `gsd-U+202Enur`, and
  `remove 'gsd-<U+202E>nur'` both *accepts* the raw key and prints
  `Removed project 'gsd-U+202Enur'`.
* `src/ui/project_list.rs` is gone, nothing references it, and the only behaviour it
  ever provided was a `display_identity` call in a file the build never compiled.
* The falsified-record corrections (21-19 truth 5, `21-19-SUMMARY.md` line 180,
  named-shape row 8) landed, are append-only, quote what they correct verbatim, and are
  honest. `21-19-SUMMARY.md`'s section 4 ("A grep for a call site proves the call is
  written… not that it is compiled") is the best piece of writing this phase has produced.
* Gates confirmed under `rtk proxy`: `cargo build --all-targets` clean;
  `cargo test --workspace` → **1372 passed / 0 failed / 13 ignored**;
  `cargo clippy -- -D warnings` exit **0**; `--all-targets` still fails with exactly the
  4 pre-existing lints in `src/browser.rs` and `src/project_creator.rs` (out of scope).

Now the part the round claims and does not have.

**The single number round 8 explicitly exempted from re-measurement is the number that
is wrong.** `21-22-SUMMARY.md`'s prohibition-7 audit says: *"the one number quoted from
21-21 (the ratatui buffer measurement) is quoted as 21-21's measurement, explicitly,
because the plan directs that it be quoted rather than re-derived."* That number —
"ratatui 0.30's `Buffer` DROPS zero-width graphemes before a cell exists" — was measured
once, through one widget (`Paragraph`, in `delete_confirm.rs`), and generalized to the
`Buffer`. I re-measured it (CR-03): it is a `Paragraph` property. In `Block::title` and
in `List`/`ListItem`, **U+202E, U+200B and U+00AD all reach terminal cells intact.** So
the phase's headline narrative — "the TUI does not *reorder*, it silently *deletes*" —
is false for two of the three widget families the TUI actually draws identity with, and
LIMIT 4's justification for declining the raw-absence assertion is false with it.

That single mis-generalization is load-bearing for a live vulnerability. `DetailScreen`'s
GitHistory tab renders a third-party repository's commit `message`, `author`, `date` and
`hash` **raw** through `List`/`ListItem` (`src/ui/screens/detail.rs:3009-3016`), and the
disposition row for `DetailScreen` explicitly *names* "git log text" as identity it
draws. I proved the leak by planting one populated `git_entries` row into `probe_ctx`:
the probe went red immediately. It is green at HEAD only because the fixture leaves the
cache empty. Combined with CR-03, a hostile repository's commit subject containing U+202E
reorders the tool's own git-history view — CVE-2021-42574 in the tool that exists to
detect it (CR-04).

**And the render surface is not the only surface with an identity on it.** Round 8
correctly identified "escape at the ONE producer" as the right shape and applied it to
`AliasRefusal`. It did not apply it to `DriverError` (`src/error.rs:328-345`), whose four
alias-carrying variants interpolate raw and are echoed by four `eprintln!` sites in
`main.rs` — measured at the binary: `drive` on a legacy alias prints
`the project \`gsd-<U+202E>nur\` has not opted in…` raw (CR-02). Nor to the failure half
of the very command the `LegacyRegistryKey` newtype was built for:
`remove_project`'s `bail!("Project not found: {}", alias)` (`src/registry.rs:620`)
propagates through `?` at `main.rs:161` and prints the raw argv — measured:
`Error: Project not found: ev^[[31milM-bM-^@M-.daeh` (CR-01). The newtype forced a
decision at the one `println!` in the diff and was never asked about the line above it.

**On the direct question — is the render surface closed BY DERIVATION, or is it a
hand-list wearing a census?** It is a real derivation with a real, undisclosed floor.
The walk is a *line-oriented textual scan*, not a semantic one. I planted two compiling
`Screen` implementors — one generated by a `macro_rules!` expansion, one with the `impl`
header wrapped across two lines — and the census stayed **green at 11 while the tree held
13** (CR-05). The module doc's headline sentence, *"a twelfth screen added tomorrow in a
file this module has never heard of is discovered without anybody editing anything here"*
(`render_escape_guard.rs:17-19`), is false for both spellings, and neither appears in a
LIMITS block whose stated standard is that every residual names its direction. Directory
reach is complete — that half of the doc's claim is true — but the enumeration's
completeness is bounded by source *formatting*, which is one level below where anyone
looked. The census is a genuine step up from a hand-named list; it is not closure.

**Is the eight-round pattern broken?** No. It is thinner, and it moved one level, which
is real progress. But the shape is unchanged and it recurred three separate times inside
round 8 alone: the ratatui class was sampled from the implementation's own one widget
(CR-03); the escape-at-the-producer mechanism was applied to the one error type the round
was looking at and not to its sibling (CR-02); and the census enumerates the one impl
spelling `rustfmt` happens to emit (CR-05). Round 8's own residual discipline is
excellent where it was applied — WR-01's rewritten doc is honest and both residuals were
measured by planting, exactly as required — but a discipline applied only to the surfaces
the round already had in hand is the pattern, not the cure.

---

## Critical Issues

### CR-01: `remove`'s FAILURE path echoes the raw argv key — the half `LegacyRegistryKey` was never asked about

**File:** `src/registry.rs:620` (reached from `src/main.rs:161`)

**Issue.** `LegacyRegistryKey`'s doc (`src/registry.rs:163-185`) claims that
"**ACCEPTING and ECHOING become two different questions the compiler asks separately**".
The compiler asks it at exactly one line — the success `println!` at `main.rs:174`. One
line above, `remove_project(&mut config, key.as_raw_for_lookup_only())?` propagates an
`anyhow` error whose message is built from the raw key with no type in the way:

```rust
pub fn remove_project(config: &mut Config, alias: &str) -> Result<()> {
    if config.projects.remove(alias).is_none() {
        bail!("Project not found: {}", alias);   // RAW
    }
```

Measured at the binary (`rtk proxy`, output through `cat -v`):

```
$ ./target/debug/gsd-meta-manager --config c5.json remove $'ev\e[31mil<U+202E>daeh'
Error: Project not found: ev^[[31milM-bM-^@M-.daeh
```

Both a raw ESC (`^[[31m`, a full ANSI colour/cursor sequence) and a raw U+202E reach the
terminal. No config prerequisite at all — the value is argv. This is precisely the harm
verification pass 8 measured through the success echo, on the same command, one line
apart, still open. The same `bail!` is spelled at `src/registry.rs:377` and `:586`.

**Fix.** Move the escape to the producer, as round 8 did for `AliasRefusal` — do not fix
this one call site. `remove_project`'s error is not a place a raw identity belongs:

```rust
// src/registry.rs
if config.projects.remove(alias).is_none() {
    bail!(
        "Project not found: {}",
        crate::text::display_identity(&crate::ui::screens::sanitize_render_line(alias))
    );
}
```

Apply the same to `:377` and `:586`. Then add the control the newtype currently lacks
(see WR-04): a test that runs the `Remove` arm's whole path — hit and miss — over
`LOOK_ALIKE_PAIRS` and asserts `is_invisible_formatting_char` finds nothing in either
stream.

---

### CR-02: `DriverError`'s four alias-carrying variants interpolate raw — the one-producer fix was applied to one error type and not its sibling

**File:** `src/error.rs:328-345`; echoed at `src/main.rs:256`, `:263`, `:310`, `:334`; and
into the TUI at `src/ui/screens/driver_confirm.rs:452`

**Issue.** Round 8's central insight is stated well at `src/registry.rs:64-70`: *"One
producer beats three consumers, and beats a list of three consumers that will be four
next round."* It was applied to `impl Display for AliasRefusal` and stops there. The
sibling error type carrying the same values on the same commands was not touched:

```rust
Self::UnknownAlias { alias } => write!(f, "no project is registered under the alias `{alias}`"),
Self::NotOptedIn { alias } => write!(f, "the project `{alias}` has not opted in to being driven; …"),
Self::RootUnusable { alias, root } => write!(f, "the registered path for `{alias}` is not a usable directory: {}", …),
Self::PromptInputsDrifted { alias, drift } => write!(f, "the driver opt-in for `{alias}` needs re-confirming: {}. …", …),
```

Measured at the binary against a legacy `config.json` holding `gsd-\u{202e}nur`:

```
$ ./target/debug/gsd-meta-manager --config c3.json drive $'gsd-<U+202E>nur' --command /gsd:progress
Error: the project `gsd-M-bM-^@M-.nur` has not opted in to being driven; …
```

`M-bM-^@M-.` is a raw U+202E. Note the contrast in the same session: `add` on the same
value prints `the alias "gsd-U+202Enur" carries a character…` — correctly escaped,
because `AliasRefusal` got the fix. Two error types, one judgment, one spelling of it.
`driver_confirm.rs:452` puts the same raw alias into `ctx.error_message`.

**Fix.** Bind the escape once above the match, exactly as `AliasRefusal` now does:

```rust
impl std::fmt::Display for DriverError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let escaped = |alias: &String| crate::text::display_identity(alias);
        match self {
            Self::UnknownAlias { alias } => {
                let alias = escaped(alias);
                write!(f, "no project is registered under the alias `{alias}`")
            }
            // … the other three the same way
```

Extend `registry::tests::a_refusal_never_carries_an_invisible_character_into_its_own_message`
to a shared helper driven over BOTH error types, so the next error type that grows an
alias field inherits the assertion rather than the discipline. Fix
`driver_confirm.rs:452` at the same time.

---

### CR-03: the ratatui measurement the whole probe design rests on is a `Paragraph` property generalized to the `Buffer` — and it is false for `Block::title` and `List`

**Files:** `src/ui/screens/render_escape_guard.rs:98-105` and `:978-985`;
`.planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md:261`;
`src/ui/screens/delete_confirm.rs:75-77`

**Issue.** The claim, stated three times as a property of the `Buffer`:

> **ratatui 0.30's `Buffer` DROPS zero-width graphemes before a cell exists.** `U+00AD`
> is simply gone from the rendered row. So the TUI does not *reorder* a hostile key — it
> silently *deletes* bytes.

It was measured once, through one widget, at one call site. I re-measured it
independently (temporary integration test, since deleted; identical harness, three code
points, three widgets):

```
U+202E: Paragraph survives=false | Block::title survives=true | ListItem survives=true
U+200B: Paragraph survives=false | Block::title survives=true | ListItem survives=true
U+00AD: Paragraph survives=false | Block::title survives=true | ListItem survives=true
```

Three consequences, all load-bearing:

1. The doc's blanket statement is false. Dropping is `Paragraph`'s grapheme/wrapping
   behaviour, not the `Buffer`'s.
2. The narrative "the TUI deletes rather than reorders" is false for `Block::title` and
   `List`. A raw U+202E in either **does** reach a cell and the terminal **will** reorder
   it. That upgrades the harm class on those widgets from collision to Trojan Source.
   (Independently corroborated: when I planted a raw identity into `NormalScreen`'s
   `Block::title`, the probe reported `['\u{e0041}', '\u{ad}']` — the soft hyphen
   survived, which the doc says is impossible.)
3. LIMIT 4's reasoning — *"an assertion that the raw form is absent passes vacuously
   against an unescaped site, and would go on passing forever"* — is true only for
   `Paragraph` sites. For `List` and `Block::title` sites a raw-absence assertion would
   have had real teeth, and the probe declined it on a false premise.

`deferred-items.md`'s STANDING entry records this as a version-pinned dependency property
with a re-measurement obligation; the obligation is right and the recorded value is wrong,
which is worse than not recording it, because a future ratatui upgrade will be checked
against a false baseline.

**Fix.** Re-measure per widget family and correct all four sites to say what is true:
zero-width graphemes are dropped by `Paragraph`'s grapheme handling and are **preserved**
by `Block::title` and `List`. Then add the assertion LIMIT 4 declined, scoped to where it
is non-vacuous — the probe already renders each state to text, so:

```rust
// alongside assertion 3, for every state of every implementor
assert!(
    !hostile_text.contains(hostile.as_str()),
    "{where_} rendered the RAW hostile identity into the buffer"
);
```

Record it red before green (it will go red today against CR-04's GitHistory tab). Update
`deferred-items.md`'s version table with the per-widget result and the date.

---

### CR-04: `DetailScreen`'s GitHistory tab renders third-party commit text raw — proven red by populating one fixture field

**File:** `src/ui/screens/detail.rs:3009-3016`

**Issue.**

```rust
ListItem::new(Line::from(vec![
    Span::styled(&entry.hash, …),
    Span::raw(" -- "),
    Span::raw(&entry.date),
    Span::raw(" -- "),
    Span::raw(&entry.message),      // RAW
    Span::raw("  "),
    Span::styled(&entry.author, …), // RAW
]))
```

The `DetailScreen` disposition row (`render_escape_guard.rs:173-179`) explicitly names
"git log text" among the identity this screen draws and adjudicates it `RENDERS_IDENTITY`.
The probe never checks it: `probe_ctx` calls `ctx.view_cache.entry(identity).or_default()`
and leaves `git_entries` empty, so the tab renders its empty branch. I planted one
populated `GitLogEntry` into `probe_ctx` and re-ran the unmodified probe:

```
panicked at src/ui/screens/render_escape_guard.rs:1093:17:
DetailScreen (src/ui/screens/detail.rs) [GitHistory tab] rendered
['\u{e0041}', '\u{ad}', '\u{e0041}', '\u{ad}'] into the terminal buffer.
```

Plant reverted; tree clean. Note the `\u{ad}` in that output: it reached a cell, because
this is a `List` — CR-03's correction, arriving from the opposite direction.

This is disclosed as residual 1 in the LIMITS block, and I credit the disclosure. But
"disclosed" is not "closed", and the reachability is not theoretical: `git log` subjects
come from whatever repository the user registered, a commit subject is the canonical
Trojan Source carrier, and this tab is the tool's own reason for existing. The Backlog,
Sessions, Archive, Browse and Defaults tabs and the Driver run list are in the same
position.

**Fix.** Route the four fields through `detail.rs`'s existing `shown()` (and see WR-02
about composing `sanitize_render_line` with it), then close the fixture hole that hid it
— that is the durable half:

```rust
fn probe_ctx(identity: &str) -> AppContext {
    // …
    let cache = ctx.view_cache.entry(identity.to_string()).or_default();
    cache.git_entries = vec![GitLogEntry { hash: identity.into(), date: identity.into(),
                                           author: identity.into(), message: identity.into() }];
    cache.backlog_items = /* one hostile entry */;
    cache.archive_entries = /* one hostile entry */;
    // … every cache the eleven tabs read
```

Residual 1 then shrinks to "states no fixture constructs", which is a much smaller and
much more honest claim than the current one.

---

### CR-05: the census's enumeration is a line-oriented textual scan — a macro-generated impl and a wrapped `impl` header are both invisible, and neither is disclosed

**File:** `src/ui/screens/render_escape_guard.rs:323-341` (claim at `:17-19`, LIMITS at
`:63-105`)

**Issue.** The module's headline sentence is:

> It is a **filesystem walk, never a path list**: a twelfth screen added tomorrow in a
> file this module has never heard of is discovered without anybody editing anything here.

The directory half is true — I confirmed it by planting in `src/state_reader/backlog.rs`.
The *enumeration* half is not. The extractor requires a single physical line whose
trimmed form starts with `impl` and contains `Screen for `, then takes an
alphanumeric-or-underscore identifier. I planted two implementors that compile and are
real `Screen`s:

```rust
macro_rules! make_screen { ($t:ty) => { impl crate::ui::screens::Screen for $t { … } }; }
make_screen!(HiddenScreen);           // name extracted from `$t` → empty → `continue`

impl
    crate::ui::screens::Screen for WrappedScreen   // line 1 has no needle; line 2 no `impl`
{ … }
```

Result: `the_screen_census_matches_the_tree` **passed**, reporting 11 implementors while
the tree held 13. Plants reverted; tree clean.

Blast radius is bounded in one direction and only one: an *existing* adjudicated screen
reformatted into either shape is caught, because it becomes a loud `STALE ROW`. A *new*
screen in either shape is silent — which is exactly the threat the doc's sentence names.

The LIMITS block enumerates four residuals and states that "every bound claimed below
names the control that certifies it; every residual names the direction it fails in".
This one is neither bounded nor named. `src/text.rs`'s alphabet-spelling census shares
the same extractor shape and the same floor.

**Fix.** Two changes, both small:

1. **Disclose it** as LIMIT 5, with its direction (under-detection, silent), its bound
   (an existing screen reformatted is caught as a stale row; a new one is not), and the
   fact that it was measured by planting.
2. **Raise the floor** so the residual is narrower than "any unusual formatting". Join
   the file's lines before scanning so a wrapped header is one logical unit, and make an
   `impl` line whose extracted name is empty a *reported offence* rather than a
   `continue` — a macro-generated impl then fails loudly and is adjudicated instead of
   vanishing:

```rust
if name.is_empty() {
    unextractable.push(format!("{path}:{number}: an implementation this scan cannot \
        name (macro-generated, or a generic parameter). Adjudicate it by hand or the \
        census is silently short."));
    continue;
}
```

Commit the red for both spellings, as this round did for the twelfth screen.

---

## Warnings

### WR-01: `list` and `remove` escape only half the class — a raw ESC in a legacy key reaches stdout

**File:** `src/main.rs:190-195` and `:174`

**Issue.** `src/ui/screens/driver.rs:874-878` states the rule the tree agreed on:

> Two classes, two predicates, composed: `sanitize_render_line` answers the ESC / C0 /
> DEL *control* question, `display_identity` answers the invisible-formatting one.
> **Neither subsumes the other.**

`main.rs`'s two CLI echo sites apply only `display_identity`. `is_invisible_formatting_char`
is `General_Category=Cf` ∪ `Default_Ignorable_Code_Point`; ESC is `Cc` and is in neither.
Measured at the binary against a config holding `ev[31mil`:

```
$ gsd-meta-manager --config c4.json list
ev^[[31mil            /tmp   2026-01-01
$ gsd-meta-manager --config c4.json remove $'ev\e[31mil'
Removed project 'ev^[[31mil'
```

There is no ratatui between these and the terminal. `list` was escaped in D-19-5 for
exactly this threat ("rows an older build accepted are still here"); ESC is a strictly
stronger spoof than U+202E and the same defence does not cover it.

**Fix.** Compose both, as `driver.rs` does, and hoist the composition into one named
helper in `crate::text` so there is one spelling of "safe to put on a terminal":

```rust
// src/text.rs
pub fn display_for_terminal(value: &str) -> String {
    display_identity(&crate::ui::screens::sanitize_render_line(value))
}
```

Then use it at `main.rs:192`, `LegacyRegistryKey::escaped_for_display`, and CR-01's
`bail!`s.

---

### WR-02: `detail.rs`'s new `shown()` is a one-predicate spelling of the two-predicate rule, and is currently saved only by an undocumented ratatui behaviour

**File:** `src/ui/screens/detail.rs:26-49`

**Issue.** `shown()` was introduced this round as the file's single statement of the
render-side rule, and it is `display_identity` alone. Every `.planning/`-derived value in
`DetailScreen` — phase names, milestones, HANDOFF context, queued commands — can carry a
C0 control read off disk, and `shown()` does not touch that class while the sibling site
at `driver.rs:878` explicitly says neither class subsumes the other.

I measured the live impact and it is currently nil: ratatui strips C0 from cells in both
`Paragraph` and `List` (temp probe, since deleted — `ESC`, `CR`, `BEL` all produced zero
control cells). So this is a defence resting entirely on undocumented dependency
behaviour — the same class of reliance `deferred-items.md` just booked as a STANDING
obligation for the zero-width class, and (per CR-03) the class of reliance that has
already been mis-measured once this round.

**Fix.** `fn shown(value: &str) -> String { crate::text::display_for_terminal(value) }`
using WR-01's helper, and say in the doc that the composition is deliberate and why.

---

### WR-03: `NormalScreen`'s status-message footer renders raw, and neither its disposition row nor any fixture names that surface

**File:** `src/ui/screens/normal.rs:811`; producers at `src/app.rs:943`, `:1864`, `:2001`

**Issue.**

```rust
let line = Line::from(Span::styled(msg.clone(), Style::default().fg(color)));
```

`ctx.status_message` is rendered raw. Producers that put a registry key into it:

* `app.rs:943` — `format!("Auto-registered: {}", alias)`
* `app.rs:1864` — `format!("Driving {alias} — run {run_id}")`
* `app.rs:2001` — `format!("Stopping {alias} — run {run_id}")`

`delete_confirm.rs:161` was correctly fixed this round; its three siblings were not,
because they live outside a `Screen` and nothing pointed at them. The `NormalScreen`
disposition row (`render_escape_guard.rs:233-239`) enumerates the name column, the phase/
status/milestone cells and the filter footer — it does not mention the status footer at
all, so the adjudication a future reader inherits is incomplete, and no fixture sets the
field.

**Fix.** Escape at the render site (`display_for_terminal(msg)`), since the producers are
many and outside the screen — this is the one place the "escape at the producer" rule
inverts, and saying so in the comment is worth more than the fix. Add a
`"dashboard with a status message"` state to the `NormalScreen` fixture that sets
`ctx.status_message` to the identity, and extend the disposition row's reason column to
name the footer.

---

### WR-04: `LegacyRegistryKey` claims "cannot be interpolated at all", derives `Debug`, and has no committed control

**File:** `src/registry.rs:185-186`, doc at `:163-184`

**Issue.** Three separate problems in one type:

1. The doc says *"**no `Display`** — so it cannot be interpolated at all"*. `#[derive(Debug)]`
   at `:185` means `println!("{key:?}")` compiles and prints the raw field. `str`'s
   `Debug` escapes via `core::char::is_printable` — the exact unpinned standard-library
   table this same file condemns 90 lines earlier (`:75-95`) as "a SECOND spelling of a
   class this project already derives for itself". The escape hatch the type was built to
   remove is re-opened by a derive.
2. `Debug` is unused. `rtk proxy grep -rn "LegacyRegistryKey" src/ tests/` shows five
   hits, none of them a `{:?}`.
3. The no-`Display` property has **no committed control** — only a comment at
   `main.rs:166-173` quoting a compile error somebody once saw. By this phase's own
   standard (prohibition 6: "MUST NOT claim a bound no committed control goes red for"),
   that is a claim, not a certificate. A future `impl Display for LegacyRegistryKey`
   compiles green.

**Fix.** Drop `#[derive(Debug)]`, or replace it with a hand-written `Debug` that prints
`escaped_for_display()`. Correct the doc sentence to "no `Display`, and its `Debug` shows
the escaped form". Add the control as a trybuild/compile-fail case, or — cheaper and
sufficient — a behavioural test that drives the real `Remove` arm over `LOOK_ALIKE_PAIRS`
on both the hit and the miss path and asserts no invisible-class character reaches either
stream (which also covers CR-01).

---

### WR-05: the alphabet-spelling census matches one exact byte string; a reordered textual copy is invisible, and the disclosed residual does not say so

**File:** `src/text.rs:783-786` (needle), residual paragraph at `:769-775`

**Issue.** The needle is `'.' | '_' | '-')` — one exact spelling including the closing
paren. The doc's under-detection paragraph names only *non-textual* constructions ("a
`match` with the same arms, a byte-range comparison, an `is_ascii_*` composition") and
says the census "stops the *textual* copy from being re-introduced silently". It does not.
A textual copy with the arms reordered or spaced differently —
`matches!(c, '-' | '_' | '.')`, or the identical clause wrapped so `'.' |` ends a line —
is a textual copy and is invisible, and the WR-03 defect it was written to catch
(`advisory.rs`) would have been invisible had its author typed the arms in any other
order.

**Fix.** Either normalize before matching (strip whitespace, sort the quoted char
literals on the line) so arm order and spacing stop mattering, or correct the residual
paragraph to say plainly that only this exact arm order and spacing is detected. The
second is a one-line honesty fix and is acceptable; the first is better.

---

## Info

### IN-01: `census_offences` keys on the bare type name, so two same-named `Screen`s collapse

**File:** `src/ui/screens/render_escape_guard.rs:339` and `:402-406`

Both the derived map and the disposition table are `BTreeMap<String /* type name */, String /* path */>`.
Two files each defining a `Screen` with the same type name collapse to one entry on both
sides (last wins by sorted path), and if both are adjudicated the pair can match and one
implementor goes silently unchecked. Key on `(name, path)` and compare sets of pairs.

### IN-02: `roadmap_widget.rs` measures width by `chars().count()`

**File:** `src/ui/roadmap_widget.rs:134`

The `len()` → `chars().count()` change is a correct fix for the byte/char panic (and the
`&s[..n]` panic fix at `:138-146` is genuinely valuable — it was reachable from any
`.planning/ROADMAP.md`). `chars().count()` is still not display width: a CJK phase name
occupies two cells per char and will overflow the box. `unicode-width` would be exact.

### IN-03: truncation can exceed its own budget when `name_max < 3`

**File:** `src/ui/roadmap_widget.rs:139-145`

`format!("{}...", name.chars().take(name_max.saturating_sub(3)))` emits 3 characters when
`name_max` is 0, 1 or 2. Pre-existing and faithfully preserved by the round-8 edit; worth
a `if name_max <= 3 { … }` guard while the code is being touched anyway.

### IN-04: the one number exempted from re-measurement is the one that was wrong

**File:** `.planning/phases/21-llm-goal-layer-prompt-injection-hardening/21-22-SUMMARY.md:422`

Prohibition 7's audit reads: *"every number here is a command output… No number is
inherited… the one number quoted from 21-21 (the ratatui buffer measurement) is quoted as
21-21's measurement, explicitly, because the plan directs that it be quoted rather than
re-derived."* That exemption was granted by the plan and honoured exactly, and it is the
number CR-03 falsifies. Recording it here as a process observation for round 9's plan:
a prohibition against inheriting numbers that carries one named exemption will be
falsified at the exemption. If the plan directs that a measurement be quoted rather than
re-derived, the wave that quotes it should still re-derive it.

---

## What I verified and did not find fault with

Stated explicitly so a later reader does not re-derive it:

* The census's non-vacuity controls are real:
  `the_census_reports_an_unadjudicated_screen_and_a_stale_row` drives the same
  `census_offences` the live assertion consumes, in both directions plus the clean
  direction; `screen_implementors_from_source` asserts it found files and found
  implementors; the needle is split at runtime so the module cannot report itself.
* Every one of the eleven `renders-identity` rows is checked by a real `Screen::render`
  into a `TestBackend` `Buffer` — not a syntactic scan — and `HelpScreen`'s
  `RENDERS_NO_IDENTITY` row is checked in the negative direction with a hostile context
  behind it. The `NormalScreen` `searching: true` note at `:631-636` is a genuine, honestly
  recorded near-miss.
* `delete_confirm.rs:78` escapes via `crate::text::display_identity`;
  `impl Display for Alias` is withdrawn at `src/registry.rs:267`; `display_identity`
  idempotence is pinned at `registry.rs:799` over imported fixtures with a non-vacuity
  assertion. All still true.
* WR-01's rewritten `WITNESS_ALLOWED_ELSEWHERE` doc is honest. Both residuals are stated
  with direction, both were measured by planting rather than argued, the arithmetic checks
  out (`DEGENERATE` has 10 members, `DEGENERATE_WITNESS_HEADS` has 3, so 7 non-witness),
  and the "why the table is kept anyway" adjudication is the right call.
* WR-02's `hooks.rs` correction is right and its new control
  (`the_path_component_predicate_refuses_every_shell_metacharacter`, 16 characters) is
  the certificate the corrected sentence needs. `sh_quote` untouched.
* WR-03's `advisory.rs` delegation is behaviour-preserving:
  `c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-')` and
  `matches!(c, 'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '_' | '-')` are the same set, and
  the emptiness and `.`/`..` clauses were correctly left in place.
* `src/ui/project_list.rs` deletion is clean. The only live reference to it was
  `add_project.rs:65`'s stale comment, corrected in the same commit; the two remaining
  mentions in `driver_inject.rs:11` and `render_escape_guard.rs:7` are deliberate
  historical citations.
* The falsified-record corrections are append-only, quote verbatim, name the measurement,
  and reopen named-shape row 8 honestly. Round 8's own SUMMARYs do not repeat the
  overclaim: `21-21-SUMMARY.md:160-163` explicitly flags the render-surface residual as a
  human judgment call, and criterion 4 is re-surfaced as recording with no progress
  claimed.
* Gates, re-run under `rtk proxy`: build clean; 1372/0/13; `clippy -- -D warnings` exit 0;
  `--all-targets` fails with exactly 4 pre-existing lints in files no phase-21 plan owns.

## Plants made during this review, all reverted

Every one was reverted and `git status --porcelain` confirmed clean afterwards (only the
pre-existing untracked `.gsd/` and `.planning/milestone.lock` remain).

| # | Plant | Where | Result |
|---|---|---|---|
| 1 | `TwelfthScreenNobodyAdjudicated` | `src/state_reader/backlog.rs` | census RED, named the file — the round's claim confirmed |
| 2 | `HiddenScreen` via `macro_rules!` + `WrappedScreen` with a wrapped header | `src/state_reader/backlog.rs` | census GREEN at 11 with 13 in the tree → CR-05 |
| 3 | raw identity into `Block::title` | `src/ui/screens/normal.rs:616` | probe RED — breadth claim confirmed; also showed `U+00AD` surviving → CR-03 |
| 4 | one populated `GitLogEntry` in `probe_ctx` | `src/ui/screens/render_escape_guard.rs` | probe RED on the GitHistory tab → CR-04 |
| 5–7 | three temporary integration tests measuring ratatui grapheme and control-char handling per widget | `tests/zz_reviewer_probe*.rs` | → CR-03, WR-02; all three files deleted |

Binary-level runs used hand-built `config.json` files under the session scratchpad only;
no repository file was modified and nothing was committed.

---

_Reviewed: 2026-08-25_
_Reviewer: Claude (gsd-code-reviewer), round 8, independent of the concurrent gsd-verifier_
_Depth: deep_
