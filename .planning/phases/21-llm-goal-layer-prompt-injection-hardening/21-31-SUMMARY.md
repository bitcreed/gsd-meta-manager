---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 31
subsystem: security
tags: [cwe-88, argument-injection, argv, claude-cli, proc, tui, sessions, rust]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening (21-27, round 10)
    provides: "the interpreter-free resume argv this plan corrects — round 10 deleted the shell (CWE-78) and left claude's own option parser in the path (CWE-88)"
  - phase: 21-llm-goal-layer-prompt-injection-hardening (21-25)
    provides: "ClaudeSession::session_id as Option<Untrusted>, the carrier this plan's residual analysis relies on"
provides:
  - "resume_terminal_argv fusing the untrusted session id into ONE `--resume=<id>` argv element, closing CWE-88 at the Sessions-tab resume"
  - "hostile_session_ids() widened from 18 to 28 fixtures with an option-lookalike block"
  - "the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program — a content-independence shape control, not a forbidden-character list"
  - "launch_terminal_argv_carries_no_untrusted_element_and_is_pinned_at_two — the examined-and-unchanged decision made loud"
  - "the CWE-88 doc correction on resume_terminal_argv, quoting the falsified clause"
  - "read_session_id's declined-validator decision with its measured reason and residual direction"
affects: [21-34, secure-phase-21, verification-pass-12]

actuals:
  tokens: 8320
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "argv fusion (`--option=<value>`) as the structural control against argument injection, chosen over an end-of-options separator BY MEASUREMENT of the receiving binary"
    - "content-independence properties over vector shape, replacing enumerations of forbidden input bytes"
    - "three-step corpus proof (complicity-green / widened-red / old-corpus-green) as the evidence that a fixture widening is load-bearing"

key-files:
  created: []
  modified:
    - src/ui/screens/detail.rs
    - src/session_detector.rs

key-decisions:
  - "D-21-45: gaps[0] closed by FUSING the id into `--resume=<id>`, not by a `--` separator — the separator was measured to delete the resume capability (probe D)"
  - "D-21-46: the corpus was widened FIRST and the pre-existing control captured against the unfixed code before anything was fixed"
  - "D-21-47: the load-bearing property is content-independence of the `-`-leading element count, not a list of forbidden first bytes"
  - "D-21-48: read_session_id keeps its pass-through and gains NO rejecting validator — the CLI resumes by session TITLE, so a rule tight enough to refuse `-h` deletes legitimate sessions silently"
  - "D-21-49: launch_terminal_argv left unchanged and pinned by a two-element equality"
  - "D-21-50: src/executor/claude.rs's three `--option value` pairs left unchanged, measured inert, deferred to 21-34"

patterns-established:
  - "A security fix at an argv sink must assert a CAPABILITY direction in the same control as the security direction — probe D is why."
  - "A dependency-behaviour claim (what the receiving CLI's parser does) is MEASURED against the installed binary and the version recorded beside the measurement."

requirements-completed: [SAFE-08, SAFE-07]

coverage:
  - id: D1
    description: "A session id beginning with a hyphen can no longer become an option of the resumed program (CWE-88 closed at the Sessions-tab resume)."
    requirement: "SAFE-08"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element"
        status: pass
    human_judgment: false
  - id: D2
    description: "The resume still resumes: the id arrives at `claude` byte-identical to as_raw_for_logic_only() as the option's value."
    requirement: "SAFE-08"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program (assertion 2, suffix byte-identity)"
        status: pass
      - kind: manual_procedural
        ref: "probe E: claude --resume=550e8400-e29b-41d4-a716-446655440000 -> 'No conversation found with session ID: 550e8400-e29b-41d4-a716-446655440000'"
        status: pass
    human_judgment: false
  - id: D3
    description: "The committed corpus was complicit and is now load-bearing, proven by the three-step green/red/green measurement."
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "three captured runs quoted verbatim in this SUMMARY and in commit 4ca622c"
        status: pass
    human_judgment: false
  - id: D4
    description: "launch_terminal_argv examined, left unchanged, and pinned at two elements."
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#launch_terminal_argv_carries_no_untrusted_element_and_is_pinned_at_two"
        status: pass
    human_judgment: false
  - id: D5
    description: "read_session_id's pass-through recorded as a decision with its measured reason and residual direction."
    verification: []
    human_judgment: true
    rationale: "Doc-only. Whether the written rationale actually stops the next reader adding a validator (or deleting the fusion) is a judgment about prose that no test can assert."
  - id: D6
    description: "An end-to-end resume of a real session through the TUI Sessions tab against an authenticated Claude subscription."
    verification: []
    human_judgment: true
    rationale: "Requires a terminal emulator and an authenticated subscription; a test cannot spawn an emulator in CI. This is the same human-only surface as ROADMAP criterion 4 and is NOT claimed as closed."

duration: 41 min
completed: 2026-08-27
status: complete
---

# Phase 21 Plan 31: Fuse the Resume Session Id to Its Option Name Summary

**The Sessions-tab resume now hands `claude` a single fused `--resume=<id>` argv element, closing CWE-88 (argument injection) at the only live leak in the phase — and the `--` separator the verification pass recommended was refuted by measurement, because it deletes the resume feature.**

## Performance

- **Duration:** ~41 min
- **Tasks:** 3
- **Files modified:** 2

## The vulnerability, reproduced before it was fixed

Round 10 deleted the shell interpreter from the resume path and, in the same
change, traded CWE-78 for **CWE-88**. `resume_terminal_argv` emitted
`[separator, "claude", "--resume", <untrusted sid>]` with the id as its own
trailing element, and `claude --help` documents `-r, --resume [value]` — an
**optional**-value option. So a session id beginning with `-` was read by
`claude` as a new option. The id is scraped verbatim from another local
process's `/proc/<pid>/cmdline`, so planting one needs no privilege.

## The six binary probes, verbatim

All run against **`claude` 2.1.248 (Claude Code)** — re-measured here, matching
the planner's figure — with stdin at `/dev/null`, a timeout, in a scratch
directory outside this repository, every one under `rtk proxy`.

**spec** — `claude --help | grep -A2 -- '-r, --resume'`

```
  -r, --resume [value]                  Resume a conversation by session ID, or
                                        open interactive picker with optional
                                        search term
```

**A** — `claude --resume --version` — **the injection FIRING**

```
2.1.248 (Claude Code)
EXIT=0
```

The token after `--resume` was parsed as a new option of `claude`. `--version`
stands in here for `--dangerously-skip-permissions`.

**B** — `claude --resume=--version` — **hostile value bound as DATA**

```
Error: --resume requires a valid session ID or session title when used with --print. Usage: claude -p --resume <session-id|title>. Provided value "--version" is not a UUID and does not match any session title.
EXIT=1
```

**C** — `claude --resume -- --version` — injection closed

```
Error: --resume requires a valid session ID or session title when used with --print. Usage: claude -p --resume <session-id|title>
EXIT=1
```

**D** — `claude --resume -- 550e8400-e29b-41d4-a716-446655440000` — **the separator DELETES the capability**

```
Error: --resume requires a valid session ID or session title when used with --print. Usage: claude -p --resume <session-id|title>
EXIT=1
```

**E** — `claude --resume=550e8400-e29b-41d4-a716-446655440000` — **the id BOUND**

```
No conversation found with session ID: 550e8400-e29b-41d4-a716-446655440000
EXIT=1
```

### D refutes the verification pass's recommended fix

**In one sentence:** probe D returns an error byte-identical to probe C's, which
means a `--` separator makes `--resume` receive *nothing at all* — so inserting
one would have closed the injection and silently deleted the resume for every
legitimate session, shipping as a security fix and being discovered as a feature
deletion.

Fusion has neither cost: B shows a hostile value bound as data, E shows a real
id bound and looked up. Note also that B's error reads *"does not match any
session title"* — the value went through the **title** lookup path, which is the
measurement that decided against a validator in Task 3.

## The three-step corpus proof

All three captured under `rtk proxy`, all against **UNFIXED** production code
(`resume_terminal_argv` still emitting four elements).

### Step 1 — COMPLICITY GREEN: the committed control cannot fail for the defect it names

`hostile_session_ids()` carried eighteen fixtures and **not one began with a
hyphen**. With nine option-lookalike fixtures added — including `--version`, the
exact payload probe A measured as firing — and **no production line changed**:

```
running 1 test
test ui::screens::detail::tests::the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1102 filtered out; finished in 0.00s
```

That green is against the unfixed production code. It is the second half of
gaps[0]: **the committed control passes on ids that demonstrably own the
machine**, so it would have passed unchanged against a build shipping the
defect. A fix certified by a corpus that cannot fail is not fixed.

`git diff -U0 -- src/ui/screens/detail.rs` at that moment showed exactly two
hunks, both inside `hostile_session_ids()` (`@@ -7268,2 +7268,20 @@` and
`@@ -7291,0 +7310,20 @@`).

### Step 2 — RED: the widened corpus against the unchanged argv

```
thread 'ui::screens::detail::tests::the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program' (2958753) panicked at src/ui/screens/detail.rs:7498:9:
assertion `left == right` failed: the number of argv elements beginning with `-` DEPENDS ON THE SESSION ID. Observed counts {2, 3}; the ids that produced each: {2: {"'; rm -rf / #", "a\u{7}b", "a\nb", "a b", "a\"b", "a$(b)c", "a&&b", "a'b", "a;b", "a=b", "a`b`c", "a|b", "a\u{200b}bc", "demo\u{200b}", "demo\u{202e}", "demo\u{e0041}", "d\u{200b}emo\u{ad}", "r\u{ad}un", "x\u{feff}"}, 3: {"-", "--add-dir", "--dangerously-skip-permissions", "--print", "--resume", "--settings=/tmp/x.json", "--version", "-h", "-r"}}. A value able to add an option-shaped element to a vector is a value the receiving parser will read as an option — measured at `claude` 2.1.248, `claude --resume --version` prints `2.1.248 (Claude Code)` and exits 0, which is CWE-88 firing. Fuse the id to its option name in ONE element (`--resume=<id>`); do NOT insert a `--` separator, which was measured to delete the resume capability entirely.
  left: 2
 right: 1
test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1103 filtered out; finished in 0.00s
```

The panic names `-h`, `--version`, `--print` and
`--dangerously-skip-permissions` among the offending ids.

### Step 3 — GREEN with the OLD corpus: the red is caused by the fixtures

The **same assertion**, the **same unfixed production code**, driven by the
pre-widening 18-fixture set through a temporary test since deleted:

```
running 1 test
test ui::screens::detail::tests::temporary_old_corpus_drives_the_same_assertion_green ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1104 filtered out; finished in 0.00s
```

**The widening is load-bearing, not decoration:** identical assertion, identical
production code, green with the old corpus and red with the new one. After the
temporary test was deleted, `git grep -n TEMPORARY src/ui/screens/detail.rs`
returned nothing and `git status --porcelain` showed only ` M src/ui/screens/detail.rs`.

### Step 4 — GREEN after the fix

`cargo test --lib -- ui::screens::detail::tests` → **58 passed; 0 failed**
(baseline 56; the +2 are the two controls this plan added).

## The fix

```rust
fn resume_terminal_argv(term: &str, sid: &Untrusted) -> Vec<String> {
    vec![
        terminal_program_separator(term).to_string(),
        "claude".to_string(),
        // ONE element, with the untrusted id FUSED to the option name it
        // belongs to. `execve` hands this over unparsed and `claude`'s own
        // option parser then reads everything after the `=` as this option's
        // VALUE — so an id beginning with `-` is data rather than a new
        // option of its own (CWE-88). The raw id is what must arrive, and it
        // arrives whole: an escaped or truncated one would resume nothing.
        format!("{RESUME_OPTION_FUSED_PREFIX}{}", sid.as_raw_for_logic_only()),
    ]
}
```

- **The vector has three elements**, not four — separator, program name, fused option.
- **The id is the suffix of the third element**, after its first `=` (splitting on the first `=` and not the last is why the fixture `a=b` is in the corpus).
- **No element is the bare id**, so there is nowhere an option parser can reach it as syntax.

`RESUME_OPTION_FUSED_PREFIX = "--resume="` is a named private const so the
option name and the binding `=` have one spelling shared by the builder and
every control that checks it.

## The doc correction, quoted beside what it corrects

**The falsified clause, verbatim:**

> **The third kind no longer exists here** because there is no interpreter left
> in the path to parse anything.

**The correction (abridged from `resume_terminal_argv`'s doc):**

> Its second clause is what made the first one false. Round 10 deleted the
> command interpreter, and that was real: no `-c` program string survives here,
> so CR-01 is genuinely closed. But "no interpreter" is not "no parser".
> **`claude`'s own option parser was in the path the whole time.**
>
> The distinction the original sentence collapsed: `execve` hands argv elements
> to the child **unparsed** — but the PROGRAM `execve` starts parses them
> itself, and that is the entire purpose of an argv. Deleting the shell removed
> one parser from the path. It did not remove the last one, because the last one
> is the program being run.
>
> So what a hostile session id could reach changed KIND rather than
> disappearing. It stopped being a shell metacharacter problem (CWE-78) and
> became an **argument injection** problem (**CWE-88**).

The correction names **CWE-88**, names **`claude`'s option parser** as the parser
that remained, quotes the `spec` probe's `-r, --resume [value]`, and cites
probes **C/D/E** by their measured outputs in a table. The in-line comment at
the resume spawn site was rewritten from *"there is no parser left"* to name the
receiving parser and the fusion.

## Both halves of the outcome

- **(i) The injection is closed.** The id can never be an argv element of its own; assertions (1) and (3) of the new control and the content-independence property all hold across 28 hostile fixtures × 5 terminals. The `-`-leading element count is constant at **2** for every id.
- **(ii) The resume still works.** The id reaching `claude` is byte-identical to `as_raw_for_logic_only()` (assertion 2), the working directory still travels through `Command::current_dir`, the separator is still first so the emulator cannot consume the program's options, and probe E confirms at the real binary that a fused id is bound and looked up. Probe B further confirms the **session-title** resume path still runs, which a validator would have broken.

## Task Commits

1. **Task 1: prove the corpus complicit, then make the widening load-bearing** — `4ca622c` (test) — the deliberate RED, with all three measurements quoted in the commit message
2. **Task 2: fuse the id, pin the sibling builder, correct the doc** — `3638fd9` (fix)
3. **Task 3: decide the input-validation half explicitly** — `8964b19` (docs)

## Files Created/Modified

- `src/ui/screens/detail.rs` — `RESUME_OPTION_FUSED_PREFIX`; `resume_terminal_argv` fused to three elements with the CWE-88 doc correction; `launch_terminal_argv`'s examined-and-unchanged decision doc; `hostile_session_ids()` widened 18 → 28; the new content-independence control; the `launch_terminal_argv` two-element pin; assertion (1) of the pre-existing control rewritten to the fused suffix; the separator control's option lookup repaired; the spawn-site comment renamed to the receiving parser.
- `src/session_detector.rs` — **doc only**, 59 added lines all `///`, zero removals, no executable line changed: `read_session_id`'s declined-validator decision.

## Gate measurements — planner's figure vs. re-measured

Every gate re-measured under `rtk proxy` against the tree actually present, never
inherited from the plan text.

| Gate | Planner at `343c408` | Re-measured here | Delta |
|---|---|---|---|
| `claude --version` | 2.1.248 | **2.1.248 (Claude Code)** | none |
| `resume_terminal_argv` span | :619-630 | **:619-630** (pre-edit) | none |
| `launch_terminal_argv` span | :636-641 | **:636-641** (pre-edit) | none |
| `terminal_program_separator` span | :569-575 | **:569-575** (pre-edit) | none |
| `hostile_session_ids` span | :7270-7293 | **:7270-7293** (pre-edit) | none |
| `LOOK_ALIKE_PAIRS` | 7 entries | **7** | none |
| `cargo clippy -- -D warnings` | exit 0 | **exit 0** before and after | none |
| `cargo clippy --all-targets -- -D warnings` | exactly 4 lints | **exactly 4**, identical | none |
| `cargo test --workspace --no-fail-fast` | 35 bins, 1416/1/13 (flake fired) | **35 bins, 1417/0/13 — fully green** | **flake did NOT fire in my baseline** |
| `cargo test --test driver_injection_corpus` | 13/0/10 | **13/0/10**, file untouched | none |

**The four pre-existing clippy lints, before and after, unchanged:**
`bool_assert_comparison` ×3 at `src/browser.rs:155,156,157` and `cmp_owned` ×1
at `src/project_creator.rs:146`. `git diff --stat` names neither file. A count of
three would have been a failure exactly as five would.

**Final workspace run:** 35 binaries, **1419 passed / 0 failed / 13 ignored**
(Task 2 run). Delta from the 1417 baseline is **+2**, both attributable to tests
this plan added:
`the_resume_argv_never_lets_a_session_id_become_an_option_of_the_resumed_program`
and `launch_terminal_argv_carries_no_untrusted_element_and_is_pinned_at_two`.

`cargo build` exit 0. `git diff --stat` for the whole plan names **only**
`src/ui/screens/detail.rs` and `src/session_detector.rs`.
`.planning/REQUIREMENTS.md` untouched.

## Decisions Made

Followed the plan's D-21-45 … D-21-50 as written. In particular the two the
orchestrator asked about explicitly:

- **`launch_terminal_argv` (D-21-49)** — examined and left unchanged. Every element is authored in this file; the project path travels through `current_dir` outside the argv, so no untrusted value reaches it and there is nothing for a fusion or separator to bind. Pinned by an equality to a two-element authored vector, so a third element appearing tomorrow fails loudly.
- **`read_session_id` (D-21-48)** — pass-through kept, **no validating regex added**. The CLI resumes by session *title* as well as by id (probe B's own error text), so a rule tight enough to refuse `-h` would refuse legitimate title resumes and silently delete those sessions from the Sessions tab — under-detection, SILENT. The argv fusion is the stated load-bearing control.

## Deviations from Plan

### 1. [Rule 1 — Unpredicted measurement] The pre-existing control did NOT stay green across the WHOLE widened corpus

- **Found during:** Task 1, step (d).
- **Issue:** The plan predicted `the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element` would stay GREEN against the full 28-fixture corpus. It did not. Exactly one fixture — `"--resume"`, the option's own name — made it red:

```
thread '...the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element' (2951419) panicked at src/ui/screens/detail.rs:7368:17:
assertion `left == right` failed: the session id "--resume" must appear as exactly ONE argv element byte-identical to `as_raw_for_logic_only()`; it appeared 2 times in ["-e", "claude", "--resume", "--resume"].
  left: 2
 right: 1
```

- **Mechanism, and why it does not change the finding:** this is a **string-equality collision with the builder's own literal**, not detection of CWE-88. The old assertion (1) counted elements *equal to the raw id*, and the unfixed builder emitted a literal `--resume` element of its own, so the count read 2. For the other **nine** hyphen-leading fixtures — including `--version`, the payload measured to own the machine — the control still passed. The complicity finding therefore stands, and is if anything sharper: the control's only red came from an accident of spelling rather than from the defect.
- **Fix:** the COMPLICITY GREEN was captured with that one colliding fixture temporarily withheld (nine option lookalikes still present), and the collision was reported rather than smoothed. Task 2(c)'s rewrite of assertion (1) to match the fused suffix removes the collision permanently — after the fix, all 28 fixtures pass.
- **Files modified:** `src/ui/screens/detail.rs`
- **Committed in:** `4ca622c` (reported in full in the commit message), resolved in `3638fd9`

### 2. [Rule 1 — Arithmetic corrected, not smoothed] Nine hyphen-leading fixtures, not ten

- **Found during:** Task 1, step (e).
- **Issue:** Task 1's acceptance criterion asks that "at least **ten** fixtures returned by `hostile_session_ids()` have `'-'` as their first character". The plan's own enumerated list of ten option lookalikes yields **nine**: the tenth, `a=b`, deliberately does not begin with a hyphen — it is the fusion-character fixture that proves an id containing `=` still arrives whole (the capability direction).
- **Resolution:** the plan's `exactly these ten fixtures` and its 28-fixture total (7 + 11 + 10) were honoured; the non-vacuity arm asserts the **measured** figure (`hyphen_leading >= 9`) with a comment recording why the tenth is exempt. No eleventh fixture was invented to make the criterion's number come out, which would have been smoothing.
- **Files modified:** `src/ui/screens/detail.rs`
- **Committed in:** `4ca622c`

### 3. [Reported, not fixed] `driver_reattach` flake fired once, in Task 3's workspace run

- **Found during:** Task 3 verification.
- **Observed:** `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired` failed with `the run record is on disk: Os { code: 2, kind: NotFound }` — a **different arm** of that binary from the one the planner named, same known class.
- **Isolated re-run:** `cargo test --test driver_reattach` → **3 passed; 0 failed** — green.
- **Attribution:** Task 3 changed only `///` lines; it cannot have caused this. Consistent with the documented mechanism (a system-wide `/proc` scan that does not stop at the process-group or worktree boundary) and with sibling worktree agents running concurrently in this wave. **Reported, not absorbed, and not fixed** — the flake record belongs to `21-34`.

---

**Total deviations:** 2 auto-fixed (both measurement corrections), 1 reported-only.
**Impact on plan:** None on scope. All three sharpen rather than weaken the evidence: the unpredicted red was diagnosed as a literal collision rather than mistaken for a finding, the fixture arithmetic was corrected against the plan's own text rather than padded, and the flake was isolated rather than absorbed.

## Issues Encountered

None beyond the deviations above. The `--` separator trap the plan warned about
was independently re-measured (probes C and D) before the fix was written, and
the measurement held: **the separator is not the fix, and would have deleted the
feature.**

## Known Stubs

None.

## Threat Flags

None — no new network endpoint, auth path, file-access pattern, or schema change
at a trust boundary. The one surface touched (the resume argv) is the threat this
plan closes.

## Not Claimed

- **ROADMAP success criterion 4 is NOT closed and no work is claimed against it.** It needs a human with an authenticated Claude subscription to run the ten `#[ignore]`d arms of `tests/driver_injection_corpus.rs`. That file was **run at its unchanged counts (13/0/10) and edited under no circumstance**. 4/5 remains the correct ceiling for this phase.
- **T-21-31-05 (`src/executor/claude.rs`'s three `--option value` pairs)** — measured inert and left unchanged, per D-21-50. Standing item owned by `21-34`; direction is under-detection, SILENT until a caller appears.
- **The end-to-end resume through a real terminal emulator** — a test cannot spawn one in CI. The property is proven at the argv and at the binary, which is where it can be proven.

## User Setup Required

None — no external service configuration, no new dependency, no `Cargo.toml` or
`Cargo.lock` change. The Package Legitimacy Audit gate does not fire.

## Next Phase Readiness

- gaps[0], the phase's only live vulnerability, is closed with the fix, the capability, and the corpus complicity all measured rather than argued.
- Sibling wave-1 plans `21-32` and `21-33` own disjoint files; this plan contributes **zero** executable occurrences of the `display_identity(` needle to `src/ui/` (the one occurrence in `detail.rs` is on a `///` line at :65 and pre-dates this plan), so `21-33`'s per-file distribution pin is unaffected — prohibition 6 holds.
- Open for verification pass 12: whether the corrected doc actually prevents the next reader re-applying the `--` separator is the one claim here no test can assert.

## Self-Check: PASSED

- `src/ui/screens/detail.rs` — present, modified, contains `RESUME_OPTION_FUSED_PREFIX`, both new controls (9 combined symbol occurrences).
- `src/session_detector.rs` — present, doc-only change, names `resume_terminal_argv` (grep count 1).
- Commits `4ca622c`, `3638fd9`, `8964b19` all present in `git log`, on branch `worktree-agent-a587e1ee26bf504f0`, based on the expected base `2c13fcf`.
- `git status --porcelain` clean after the task commits; `cargo build` exit 0.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-27*
