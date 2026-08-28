---
phase: 21-llm-goal-layer-prompt-injection-hardening
reviewed: 2026-08-27T00:00:00Z
depth: standard
files_reviewed: 2
files_reviewed_list:
  - src/session_detector.rs
  - src/ui/screens/detail.rs
findings:
  critical: 2
  warning: 6
  info: 4
  total: 12
status: issues_found
---

# Phase 21 (round 13): Code Review Report

**Reviewed:** 2026-08-27
**Depth:** standard
**Files Reviewed:** 2 (`src/session_detector.rs`, `src/ui/screens/detail.rs`)
**Diff base:** `9eeb3601c087ae5109a0b4953563f6236826c5c3..HEAD`
**Status:** issues_found

## Identifier scheme

Round 13 numbers its own findings with a round-scoped `R13-` prefix — `R13-CR-01`,
`R13-CR-02`, `R13-WR-01`..`R13-WR-06`, `R13-IN-01`..`R13-IN-04`. Each review round of
this phase restarts its numbering at 01, so every bare id this round would otherwise
have used already names a different, pre-existing phase-21 finding. The phase has
evidence for at least two such collisions. Historical `WR-04` vs `R13-WR-04`: the
historical one was `LegacyRegistryKey`'s derived `Debug`, closed by c9345a1, while this
round's is the unbounded per-NUL `Vec` allocation. Historical
`CR-01` vs `R13-CR-01`: the historical one was the shell command injection at
`detail.rs:1692-1707`, while this round's is the `--resume`/`-r` successor-arity defect.
Renumbering this round into a higher range (`WR-07`+) rather than prefixing it `R13-`
was rejected: a higher range only defers the collision to the next round that restarts
its numbering at 01, whereas a round prefix is stable and self-describing.

| Was | Now |
|-----|-----|
| `CR-01` | `R13-CR-01` |
| `CR-02` | `R13-CR-02` |
| `WR-01` | `R13-WR-01` |
| `WR-02` | `R13-WR-02` |
| `WR-03` | `R13-WR-03` |
| `WR-04` | `R13-WR-04` |
| `WR-05` | `R13-WR-05` |
| `WR-06` | `R13-WR-06` |
| `IN-01` | `R13-IN-01` |
| `IN-02` | `R13-IN-02` |
| `IN-03` | `R13-IN-03` |
| `IN-04` | `R13-IN-04` |

`21-VERIFICATION.md` (pass 14) cites this review's findings under their OLD bare ids,
including the ad-hoc `WR-05-new` it improvised at its line 109 for what is now `R13-WR-05`.
It is deliberately NOT edited, being a committed record; apply the table above when
reading it.

Corollary, and the rule for reading this document: a BARE id names the phase's
historical finding of that name; an `R13-` id names one of this round's. The bare
`IN-07` in `R13-CR-01`'s fix list is exactly that case — a different round's id, left
unchanged.

## Summary

Round 13 does three things well, and I verified each rather than accepting it: the
generative round-trip property's oracle is genuinely independent (`std::str::from_utf8`
+ `str::trim` computed in the test, no shared refusal predicate exported by the parser);
the non-vacuity floors have real margin, not accidental margin (`whitespace_padded`
observes ~410 against a floor of 20, `ill_formed` ~650 against 100, `hyphen_leading`
~680 against 50 — all structurally guaranteed by the mixture arms rather than luck); and
the rank resolution (`first_resume.or(first_assigned)`, two slots, exhaustive scan,
`slot.is_none()` guard) is correct — `--resume` outranks `--session-id` regardless of
argv index, leftmost-wins holds within a rank, and there is no off-by-one or
first-match-wins bug in it. The index arithmetic in `session_id_in_cmdline` is safe:
`args.get(index + 1)` cannot go out of range, the step-over cannot skip past the end,
and `--resume` as the final byte of a cmdline with no trailing NUL yields `None` rather
than panicking. `from_utf8` is strict, nothing is trimmed on the return path, and no
path in this file substitutes U+FFFD into an id. The full suite is green (1118 passed).

That is where the good news ends. Two defects are load-bearing.

**The parser's model of the `claude` option grammar is wrong for the exact option this
phase is about, and a test asserts the wrong behaviour as correct with a citation I
falsified by re-running the probe** (R13-CR-01). `-r, --resume [value]` is an
**optional-value** option — the project's own record says so at
`deferred-items.md:1404` and `21-31-SUMMARY.md:141` — so `claude` does **not** bind an
option-shaped successor to it. The parser does. Round 13's new rank rule makes this
strictly worse: a bogus Resume-rank value now outranks a genuine `--session-id` value.

**The "total" contract is stated over six spellings and certified over four** (R13-CR-02).
`wire_forms()` in `detail.rs` registers `fused`, `split`, `short-split`,
`short-attached` — and neither `--session-id` spelling, both of which round 13 added to
the parser and one of which is what **this build's own executor emits**. The doc at
`session_detector.rs:411-447` states a total contract with "no third outcome" and cites
the generated property as its certificate. The certificate does not cover two of the six
spellings, and nothing in the tree turns that into a RED. This is the same
"closed claim over an OPEN set" failure the round's own correction block says it fixed,
committed in the same commit as the correction.

No structural findings block was supplied.

## Narrative Findings (AI reviewer)

## Critical Issues

### R13-CR-01: The split-form "value by position" rule is false for `--resume`/`-r`, and the test that pins it cites a measurement that says the opposite

**File:** `src/session_detector.rs:530-537` (the rule), `src/session_detector.rs:1052-1079`
(the test), `src/session_detector.rs:468-482` (the doc), `src/ui/screens/detail.rs:8103-8134`
(the encoders that carry the same false model)

**Issue:**

The parser treats the element after a bare `--resume` or `-r` as that option's value
unconditionally:

```rust
} else if element == RESUME_OPTION_NAME || element == RESUME_OPTION_SHORT_NAME {
    let next = args.get(index + 1).copied();
    index += 1;
    next.map(|value| (value, SessionIdRank::Resume))
```

and `the_split_forms_successor_is_a_value_by_position_even_when_it_is_option_shaped`
asserts this is right, justifying it as *"measured at `claude` 2.1.250"*:

```rust
assert_eq!(parsed(&["claude", "--resume", "-h"]).as_deref(), Some("-h"), ...);
```

I re-ran the probe against the installed binary at the cited version. It says the
opposite:

```
$ claude --version
2.1.250 (Claude Code)

$ claude --resume --version < /dev/null   →  2.1.250 (Claude Code)   exit 0
$ claude -r --version      < /dev/null   →  2.1.250 (Claude Code)   exit 0
$ claude --resume -h       < /dev/null   →  Usage: claude [options] [command] [prompt]
```

`--version` and `-h` were parsed as **new options of `claude`**, not as `--resume`'s
value — precisely because `-r, --resume [value]` is an OPTIONAL-value option. This is
not a new discovery: it is `21-31-PLAN.md:96` probe A, `21-31-SUMMARY.md:141`, and
`deferred-items.md:1404`, all in this phase's own record. The CWE-88 finding that
justified the fusion fix *is* this fact. The test's citation is falsified by the
project's own measurement and by re-measurement today.

The contrast confirms the rule is option-specific rather than universal — `--session-id`
takes a **required** value and genuinely does bind an option-shaped successor:

```
$ claude --session-id --version  →  Error: Invalid session ID. Must be a valid UUID.
```

So the parser is right for `--session-id` and wrong for `--resume`/`-r`, and it applies
one rule to both.

**Consequences, in the direction this phase's vocabulary uses:**

1. **MIS-detection, silent.** A same-user process running `claude --resume
   --dangerously-skip-permissions` — verbatim the attacker cmdline named at
   `21-VERIFICATION.md:495` — makes this parser report
   `--dangerously-skip-permissions` as that session's id. A Sessions-tab row appears
   offering to resume a conversation that does not exist. That is the identical harm
   the R1 refusal class was introduced to remove (`session_detector.rs:418-429`),
   reached by a different route that R1 does not cover, because the bytes *did* appear
   verbatim on the wire.

2. **Round 13's rank rule turns it into shadowing of a real id.** `claude -r -p
   --session-id <real-uuid>` fills the Resume slot with `-p`, steps over it, fills the
   Assigned slot with the real uuid, and then `first_resume.or(first_assigned)` reports
   `-p` — discarding the id the process is genuinely running under. Before the rank
   rule the leftmost-wins scan had the same first half but no mechanism to outrank a
   later genuine value. G2's whole stated purpose is to make driver-launched sessions
   (`--session-id <uuid>`) visible; this rule can silently hide them again.

3. **The generated property cannot falsify it.** `encode_split` and `encode_short_split`
   (`detail.rs:8103-8134`) encode `["claude", "--resume", s]` and assert that **every**
   `s` — including the generated `prefixed` arm's `-`, `--`, `-r`, `--resume`,
   `--session-id` heads — must round-trip. The producer-side encoder and the parser
   share the same wrong model of the CLI's grammar, so a 4096-case property in four
   wire forms agrees with the parser by construction on exactly this point. The oracle
   is independent about *encoding* (R1/R2 from `std`) and not independent about *option
   binding*. This is the phase's recurring defect, one abstraction level up from where
   it was being watched for.

**Fix:** make the successor rule follow the option's arity, which is the fact that was
already measured. For the Resume rank only, refuse an option-shaped successor:

```rust
} else if element == RESUME_OPTION_NAME || element == RESUME_OPTION_SHORT_NAME {
    // `-r, --resume [value]` is an OPTIONAL-value option (measured, claude 2.1.250:
    // `claude --resume --version` prints the version and exits 0 — the successor was
    // read as a NEW OPTION, not as this option's value). So a successor that begins
    // with `-` and is not the single byte `-` is NOT this option's value; `claude`
    // did not bind it, and reporting it is mis-detection. `--session-id <uuid>` takes
    // a REQUIRED value and is deliberately NOT given this rule (measured:
    // `claude --session-id --version` reports `Invalid session ID`, i.e. it bound it).
    let next = args
        .get(index + 1)
        .copied()
        .filter(|value| !(value.starts_with(b"-") && *value != b"-"));
    index += 1;                       // still step over the consumed slot
    next.map(|value| (value, SessionIdRank::Resume))
```

Costs no capability: an id or title beginning with `-` cannot be passed in the split
form to the real CLI either, and the fused form (`--resume=<id>`, what this build emits)
is unaffected. Then:

- rewrite `the_split_forms_successor_is_a_value_by_position_even_when_it_is_option_shaped`
  to assert `None` for the `-h` / `--resume` / `--version` successors under
  `--resume`/`-r`, and keep a positive arm for `--session-id` where the behaviour is
  real; record the three probe transcripts above beside it rather than the falsified
  citation;
- exclude option-shaped `s` from the `split` and `short-split` arms of the generated
  property (or give those forms their own oracle branch), so the property stops
  certifying the wrong model;
- correct the "IN-07" paragraph at `session_detector.rs:468-482`, which currently states
  the false rule as a measured fact.

---

### R13-CR-02: The parser recognises six spellings; the "total" certificate covers four — and the two uncovered ones are `--session-id`, which this build's own executor emits

**File:** `src/ui/screens/detail.rs:8150-8169` (`wire_forms`), `src/session_detector.rs:411-447`
(the totality claim), `src/session_detector.rs:1353-1456` (the only control that touches
`--session-id`)

**Issue:**

`session_id_in_cmdline`'s doc states a contract with no escape hatch
(`session_detector.rs:411-416`):

> For every byte string `s` on the wire, in either form above, this function either
> returns `Some(t)` whose bytes are byte-identical to `s` … **or** returns `None` and
> `s` falls in exactly one of [R1, R2]

and names its certificate (`session_detector.rs:441-447`):

> certified by `…the_round_trip_property_holds_for_every_generated_byte_string` over
> 4096 deterministically generated byte strings **in every registered wire form**

`wire_forms()` registers four rows: `fused`, `split`, `short-split`, `short-attached`.
The parser recognises six spellings — the correction block two screens up
(`session_detector.rs:366-374`) enumerates them itself: `--resume=<id>`, `--resume <id>`,
`-r<id>`, `-r <id>`, `--session-id=<id>`, `--session-id <id>`. The two `--session-id`
forms are in the parser and not in the table. "Every registered wire form" is true only
in the sense that makes the sentence vacuous: the register is what was omitted from.

So for `--session-id` the byte-identity claim, both refusal classes, and the whole
non-vacuity floor (`the_generator_reaches_every_named_class_and_both_branches_of_the_property`
also iterates `wire_forms()`, `detail.rs:8464`) assert nothing. What actually exercises
those two spellings in the whole tree:

- `parsed(&["claude", "--session-id", "u"])` and `parsed(&["claude", "--session-id=u"])`
  — two one-character fixtures (`session_detector.rs:916-930`); and
- `the_executors_own_argv_is_an_argv_this_build_can_read_back`, one UUID plus the string
  `"a-conversation-that-already-exists"`.

No whitespace-padded value, no ill-formed-UTF-8 value, no option-shaped value, no empty
value has ever been driven through `--session-id` in either spelling. G1 (byte identity
under padding) and G3 (R1 refusal) — the two classes rounds 12 and 13 exist for — are
uncertified for the spelling this build itself emits on **every driver-launched run**,
which is the population `DRIVE-01` is about.

Nothing makes this a RED. The census
(`every_claude_argv_option_site_under_src_is_adjudicated`) couples *producers* to
round-trip tests; it does not couple the parser's recognised **spelling set** to
`wire_forms()`, so a spelling added to the parser without a row silently narrows the
certificate while every gate stays green. That is the same mechanism the census was
built to defeat, on the axis the census does not watch.

**Fix:**

1. Add the two missing rows:

```rust
fn encode_assigned_split(s: &[u8]) -> Vec<u8> {
    nul_join_cmdline(&[b"claude".as_slice(), b"--session-id".as_slice(), s])
}
fn encode_assigned_fused(s: &[u8]) -> Vec<u8> {
    let mut element = b"--session-id=".to_vec();
    element.extend_from_slice(s);
    nul_join_cmdline(&[b"claude".as_slice(), element.as_slice()])
}
// … plus WireForm { name: "assigned-split", … } and { name: "assigned-fused", … }
```

2. Make the omission impossible to repeat: assert the table's arity against the parser's
   spelling count in the same way `assembled_option_needles()` is pinned at ten
   (`session_detector.rs:1488-1494`) — e.g. a `const RECOGNISED_SPELLINGS: usize = 6;`
   in `session_detector`, re-exported to the test module, with
   `assert_eq!(wire_forms().len(), RECOGNISED_SPELLINGS)` and a failure message saying a
   spelling taught to the parser without a wire-form row narrows the certificate
   silently.
3. Correct `session_detector.rs:411-447` so the contract's scope matches the register at
   the moment it is written.

## Warnings

### R13-WR-01: The newly adjudicated producer is driven by exactly one benign fixture — the round-10/round-12 corpus defect, reintroduced on the new producer

**File:** `src/session_detector.rs:1405-1455`

**Issue:** `the_executors_own_argv_is_an_argv_this_build_can_read_back` is the control
that discharges `src/executor/claude.rs`'s brand-new `Producer` row. Its resuming arm
uses one hand-picked value:

```rust
let resumed = "a-conversation-that-already-exists".to_string();
```

No leading hyphen, no whitespace padding, no non-ASCII, no `=`. That is verbatim the
corpus shape round 10 was certified with and round 12 was certified with — an
enumeration of one, on the producer the census just declared covered. The sibling
producer gets 28 hostile fixtures × 5 terminals; this one gets one string, and nothing
in the tree discloses the asymmetry.

It matters concretely because `build_argv` emits the resume id **unfused**
(`src/executor/claude.rs:270-271` pushes `--resume` and `session` as two elements) —
the exact standalone-untrusted-element shape `resume_terminal_argv` was fused to remove,
and the shape probe A proves the CLI mis-parses. It is latent today only because
`resume_session` is `None` in every production path (`executor/mod.rs:465`;
`driver/run.rs:1635` states it is never set). The moment a detected `Untrusted` session
id is wired to it — the obvious next feature, and what the Sessions tab is for — CWE-88
returns at the second producer, and this control will still be green.

**Fix:** iterate `hostile_session_ids()` (or the generated corpus) through
`build_argv` in the resuming arm, not one string. When that turns red for
option-shaped ids, fuse `build_argv`'s resume element the way `resume_terminal_argv` is
fused, and note in the row's reason that the producer is fused. Both changes belong in
the same commit as the assertion that forces them.

---

### R13-WR-02: The "a named round trip must actually exist" guard is a raw substring search a comment can satisfy, and `#[cfg(unix)]` lets it pass with no test at all

**File:** `src/session_detector.rs:1609-1623`, `src/session_detector.rs:1353-1355`

**Issue:**

```rust
let definition = format!("fn {test_name}(");
assert!(files.iter().any(|(_, lines)|
    lines.iter().any(|(_, line)| line.contains(&definition))), …);
```

Three gaps in the control that the surrounding doc calls "caught by a walk rather than
by a reader":

1. **Comments are not filtered.** Both other walks in this module filter
   `line.trim_start().starts_with("//")` (`is_option_literal_site`,
   `no_source_line_under_src_requests_a_forked_session`). This one does not, so a doc
   line or a prose paragraph containing `fn the_executors_own_argv_is_an_argv_this_build_can_read_back(`
   satisfies the guard with no test in the tree. That the earlier, subsumed-duplicate-control `WR-05` (not `R13-WR-05`) comment block at
   `detail.rs:7809-7811` deliberately truncates a deleted test's name "so that no whole
   spelling of a test that no longer exists survives in the tree" shows the hazard is
   understood — the guard is what fails to enforce it.
2. **`#[test]` is not required.** A plain helper function of that name passes.
3. **`#[cfg(unix)]`** on `the_executors_own_argv_is_an_argv_this_build_can_read_back`
   means that on a non-unix target the named round trip is not compiled and not run,
   while the guard — a source-text search — still passes. The census then reports the
   executor producer as covered on a platform where nothing drives it.

**Fix:** filter comment lines as the sibling walks do; require the preceding non-blank
non-attribute line region to contain `#[test]`; and either drop the `#[cfg(unix)]` (this
module is `/proc`-based anyway — gate the *body*, not the test) or record the platform
restriction in `CLAUDE_ARGV_ROUND_TRIPS` and assert it.

---

### R13-WR-03: The property test's exemplar map is keyed by class only; its doc claims class × wire form, and its stated bound is wrong

**File:** `src/ui/screens/detail.rs:8199-8226`

**Issue:** The doc says:

> So one exemplar per (class × wire form) is collected, with the rest counted … Bounded
> by construction: at most six exemplars.

The code keys by class alone:

```rust
let mut violations: std::collections::BTreeMap<&'static str, (String, usize)> = …;
```

Four classes exist, so the bound is four, not six, and — the part that costs
information — when the parser violates a class in one wire form only, the report shows
the class and the *first* form that hit it, with no way to tell whether the other three
forms are affected. With R13-CR-02 fixed there will be six wire forms and four classes; the
doc's "six" is wrong under both the old and the new register. In a file where every
other doc paragraph is a load-bearing correction, a doc that describes a diagnostic the
code does not produce is that same earlier `WR-05` (not `R13-WR-05`) hazard in miniature.

**Fix:** key the map by `(class, form.name)` as the doc says, and state the bound as
`classes × wire_forms().len()` rather than a literal:

```rust
let mut violations: BTreeMap<(&'static str, &'static str), (String, usize)> = …;
```

---

### R13-WR-04: `session_id_in_cmdline` allocates 16 bytes of `Vec` per NUL byte of attacker-controlled `/proc` input, unbounded

**File:** `src/session_detector.rs:506`

**Issue:**

```rust
let args: Vec<&[u8]> = cmdline.split(|&b| b == 0).collect();
```

`cmdline` is read whole from another process's `/proc/<pid>/cmdline`
(`session_detector.rs:220`) — untrusted by this module's own framing, and sized by that
process, not by this one. A same-user process named `claude` can present a cmdline of
~2 MB of NUL bytes (the arg area is bounded by `RLIMIT_STACK/4`, commonly 2 MB); the
collect then materialises ~2 M `&[u8]` fat pointers — about **32 MB of transient
allocation for 2 MB of input**, a 16× amplification, repeated for every such PID in a
single `detect_sessions()` pass, with no cap anywhere on the path. Nothing here panics
and no slicing is unsafe, so this is robustness rather than memory corruption, but it is
attacker-sized allocation in a function whose entire documented premise is that its
input is hostile.

**Fix:** stream instead of collecting, or cap the input. Either

```rust
let cmdline = &cmdline[..cmdline.len().min(MAX_CMDLINE_BYTES)];   // e.g. 64 KiB
```

before splitting (documented as a measured bound with its direction: under-detection for
absurdly long argvs), or restructure the scan over `split(..)` with a one-element
lookahead so no `Vec` is built at all. The latter also removes the index arithmetic.

---

### R13-WR-05: The `--` end-of-options terminator is not honoured, so a post-`--` operand is reported as a session id

**File:** `src/session_detector.rs:505-599`

**Issue:** The scan examines every argv element as a potential option. `claude` honours
`--`: I confirmed at 2.1.250 that `claude -- --version` does **not** print the version
(it starts an interactive session, i.e. `--version` was taken as an operand, not an
option). So for `claude -- --resume=abc`, the string `--resume=abc` is a *prompt* and
the process is running no session named `abc` — but `session_id_in_cmdline` reports
`Some("abc")`. Mis-detection: a Sessions-tab row offering to resume a conversation that
does not exist, from a cmdline any local process can plant. Same class as R13-CR-01, narrower
shape. The six-spelling census enumerates spellings but says nothing about the grammar
position they are valid in.

**Fix:** stop the option scan at the first element equal to `b"--"`:

```rust
if element == b"--" { break; }   // end-of-options; everything after is an operand
```

with a test arm asserting `parsed(&["claude", "--", "--resume=abc"]) == None`, and one
asserting a `--resume` **before** the `--` is still read.

---

### R13-WR-06: `read_start_time` splits on the first `)`, not the last — a process whose `comm` contains `)` yields a wrong start time

**File:** `src/session_detector.rs:614`

**Issue:**

```rust
let after_comm = stat.find(')')?.checked_add(2)?;
```

`/proc/<pid>/stat` field 2 is the executable name in parentheses and the kernel does
**not** escape `)` inside it. The comment directly above states the reason the closing
paren is used as the anchor, then uses `find` (first) where the invariant requires
`rfind` (last). A process named e.g. `claude)x` produces
`123 (claude)x) S …`; the scan then starts at `x) S …` and `nth(19)` lands on the wrong
field — a wrong `start_time`, or `None`. Since `comm` is settable by the process itself
(`prctl(PR_SET_NAME)`), this is reachable inside the same hostile-same-user-process
threat model the rest of this file is written against, and `start_time` is what the
Sessions tab sorts and ages rows by.

Disclosure: this line predates the round-13 diff. It is reported because it is in a
reviewed file, it is a one-line correctness bug, and its threat model is this phase's.

**Fix:**

```rust
let after_comm = stat.rfind(')')?.checked_add(2)?;
```

plus a test with a `)`-bearing comm fixture. Extracting the field-22 parse into a pure
`fn start_time_in_stat(stat: &str) -> Option<u64>` — the same producer/consumer split
`session_id_in_cmdline` already got — makes that testable without `/proc`.

## Info

### R13-IN-01: The generator indexes the seed corpus with `% seeds.len()` and no empty guard

**File:** `src/ui/screens/detail.rs:8007-8015`, `src/ui/screens/detail.rs:8049-8051`

**Issue:** `seeds[(next_random(state) % seeds.len() as u64) as usize]` panics with a
divide-by-zero if `hostile_session_ids()` ever returns empty. Every other reach
assumption in this file is a committed floor with a message explaining what a breach
means; this one is an unguarded index that would surface as an opaque arithmetic panic
inside a generator, far from the corpus that emptied.

**Fix:** `assert!(!seeds.is_empty(), "…")` at the top of `generated_session_id_bytes`,
with the same explanatory style the floors use.

### R13-IN-02: The both-branches tallies use floors of 1 while the class floors carry 6×–20× margin

**File:** `src/ui/screens/detail.rs:8482-8502`

**Issue:** `round_tripped >= 1`, `refused_r1 >= 1`, `refused_r2 >= 1`. The class floors
above them are calibrated (20, 100, 50, 200) and I verified their margins are real. These
three are not: a generator or parser collapse that leaves a single case in each branch
passes them. They are defence-in-depth behind the property test, but the doc presents
them as the guard that stops the disjunction passing vacuously, and a floor of 1 does
not do that job to the standard the rest of the file sets.

**Fix:** calibrate against measured observations the way the class floors were, e.g.
`round_tripped >= 1000`, `refused_r1 >= 400`, `refused_r2 >= 20`, each with the observed
value in the message.

### R13-IN-03: The fork-option guard filters only `//` lines, so a block comment or string literal is a false positive and any assembled spelling evades it

**File:** `src/session_detector.rs:1004-1014`

**Issue:** `no_source_line_under_src_requests_a_forked_session` skips lines whose trimmed
start is `//`, then substring-matches. A `/* … */` block comment, or a doc-test/string
literal mentioning the option, becomes a false RED; conversely the option assembled from
fragments (exactly the idiom this very test uses on itself) evades it entirely. The
evasion residual is disclosed for the *census* needle (`session_detector.rs:1100-1106`)
but not for this guard, whose whole job is to enforce the rank rule's premise.

**Fix:** carry the same "what this does NOT see, with its direction" paragraph here, and
skip `/*`-prefixed lines alongside `//`.

### R13-IN-04: The `corpus.len() == 28` shape assertion and its rationale are duplicated across two tests

**File:** `src/ui/screens/detail.rs:7602-7608`, `src/ui/screens/detail.rs:7854-7861`

**Issue:** The same magic 28, the same `7 + 11 + 10` decomposition and near-identical
failure prose appear in two tests. When the corpus changes, one will be updated and the
other will name a decomposition that no longer holds — the drift class this phase keeps
finding.

**Fix:** assert the shape once in a `fn assert_corpus_shape(corpus: &[String])` helper
(or a `const HOSTILE_CORPUS_LEN: usize` beside `hostile_session_ids`) and call it from
both.

---

_Reviewed: 2026-08-27_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
_CLI probes in R13-CR-01/R13-WR-05 were run against `claude 2.1.250` on this machine; transcripts are quoted verbatim._
