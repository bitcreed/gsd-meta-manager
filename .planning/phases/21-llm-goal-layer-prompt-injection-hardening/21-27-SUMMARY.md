---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 27
subsystem: security
tags: [command-injection, argv, execve, prompt-injection, rust, trait-probe, census]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    provides: "`crate::text::Untrusted`, the two-accessor carrier and its runtime trait probe (21-23/21-25); `test_support::LOOK_ALIKE_PAIRS`; the alphabet-spelling census idiom (21-26)"
provides:
  - "The Sessions-tab resume and new-session spawns build an argv vector with NO command interpreter in it; the working directory travels through `Command::current_dir`"
  - "`resume_terminal_argv` / `launch_terminal_argv` — pure functions carrying the whole spawn decision, so the property is provable without spawning a terminal in CI"
  - "`terminal_program_separator` — a per-emulator separator table that stops the security fix from silently deleting the resume capability"
  - "`as_raw_for_logic_only`'s doc naming THREE questions and the `shell_command_fragment` rule: argv, never a program string"
  - "A committed census reporting any executable line under `src/` that hands a command interpreter an interpolated program; returns zero, observed red by planting"
  - "`Untrusted`'s absent-trait claim certified at SIX absences (was three) with six control arms, each new one observed red by planting its impl"
affects: [21-28, 21-29, 21-30, any future site that reaches a subprocess]

actuals:
  tokens: 13809
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "argv-not-program-string: a value reaching a subprocess is its own argv element; the interpreter is deleted from the path rather than quoted for"
    - "Runtime-assembled census needles, split so no line of the censusing module spells its own needle — extended here to a SECOND census (`tests/spawn_seam_guard.rs`) after a measured collision"
    - "Presence-arm-before-absence-arm: every new trait probe is confirmed answering `true` for a type that implements the bound BEFORE its absence assertion is written"

key-files:
  created: []
  modified:
    - src/ui/screens/detail.rs
    - src/text.rs

key-decisions:
  - "D-21-32: CR-01 closed by DELETING the command interpreter from the spawn path, not by quoting for it"
  - "D-21-33: the argv is built by a pure named function so the property is provable at the argv"
  - "D-21-34: the emulator/program separator is chosen by a named function with a table rather than assumed uniform"
  - "D-21-35: the third question is added to the VOCABULARY and to a checked rule, not to the accessor set — a third accessor would mint a supported way to do the unsafe thing"
  - "D-21-36: WR-01 closed by extending the control to six absences rather than by softening the doc's count"

patterns-established:
  - "Sink-kind naming: every comment at an execution sink names whether the value is a lookup, an argv element, or a program fragment"
  - "A census fixture must be inert to EVERY census in the tree, not just its own"

requirements-completed: [SAFE-08, SAFE-07]

coverage:
  - id: D1
    description: "The Sessions-tab resume hands `std::process::Command` an argv vector with no command interpreter in it; a hostile `/proc`-scraped session id is one opaque element reaching `execve` unparsed"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element"
        status: pass
    human_judgment: false
  - id: D2
    description: "The resume still resumes: the id reaching the child is byte-identical to the raw id, and the emulator runs the program rather than consuming its options"
    requirement: "SAFE-07"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#every_terminal_find_terminal_can_return_gets_a_separator_that_keeps_the_program_options"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_separator_table_covers_every_candidate_find_terminal_probes"
        status: pass
    human_judgment: true
    rationale: "The argv and the separator are proven; that a real gnome-terminal/kitty/alacritty/xterm actually resumes the session cannot be asserted in CI — no test can spawn a terminal emulator. A human running the TUI against a live session is the only arm that closes it."
  - id: D3
    description: "`as_raw_for_logic_only`'s doc names three questions and states the `shell_command_fragment` rule, with the round-9 sentence it corrects quoted verbatim"
    requirement: "SAFE-08"
    verification:
      - kind: other
        ref: "rtk proxy grep -c 'shell_command_fragment' src/text.rs -> 4"
        status: pass
    human_judgment: true
    rationale: "A doc's CONTENT is a prose claim; a grep proves the rule name is present, not that the three questions are correctly drawn. A reviewer must read it."
  - id: D4
    description: "A committed census reports any executable line under `src/` handing a command interpreter an interpolated program; returns zero for the fixed tree, cannot report itself, and does not degenerate into a ban on a word"
    requirement: "SAFE-08"
    verification:
      - kind: unit
        ref: "src/text.rs#no_executable_line_under_src_hands_an_interpreter_an_interpolated_program"
        status: pass
      - kind: unit
        ref: "src/text.rs#the_interpreter_census_does_not_report_the_module_that_builds_its_needle"
        status: pass
      - kind: unit
        ref: "src/text.rs#the_interpreter_census_does_not_report_a_fixed_non_interpolated_invocation"
        status: pass
    human_judgment: false
  - id: D5
    description: "`Untrusted`'s absent-trait claim and the control that certifies it name the same set — six absences, six control arms, each new one observed red by planting its impl"
    requirement: "SAFE-08"
    verification:
      - kind: unit
        ref: "src/text.rs#an_untrusted_carrier_implements_none_of_the_string_conversions"
        status: pass
    human_judgment: false

duration: 71 min
completed: 2026-08-27
status: complete
---

# Phase 21 Plan 27: Delete the Interpreter Summary

**CR-01 closed by removing the command interpreter from the Sessions-tab spawn path entirely — argv + `Command::current_dir` instead of a `sh -c` program string built from two `/proc`-scraped values — with the fix proven at the argv by a control observed RED against the construction it replaces.**

## Performance

- **Duration:** 71 min
- **Completed:** 2026-08-27T20:58:56Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- **The interpreter is gone from the spawn path.** `src/ui/screens/detail.rs` built `"cd '{}' && claude --resume '{}'"` and handed it to `sh -c`, interpolating a session id read verbatim from another process's `/proc/<pid>/cmdline` and a `read_link` of `/proc/<pid>/cwd`. Neither passes any alphabet; a single quote in either ran arbitrary code with the operator's privileges. Both values are now argv elements the kernel hands to `execve` unparsed, and the working directory travels through `Command::current_dir`.
- **The vocabulary gained the question it was missing.** `as_raw_for_logic_only`'s doc listed *subprocess arguments* among its accepted uses, collapsing an argv element (inert) with a program fragment (not inert). That collapse is why round 9's compiler-named-sites methodology could not see CR-01 — the site was never retyped away from being, in the doc's own words, a subprocess argument.
- **The rule is checked, not written.** A census over `src/` reports any executable line that both names a command-interpreter binary and interpolates into what it is handed. It returns zero, cannot report itself, does not report a fixed invocation, and was observed red by planting the exact multi-line shape CR-01 had.
- **WR-01: the certificate now matches the claim.** `Untrusted`'s doc said "five", listed six, and the control certified three. All six are now certified with six control arms; each new absence was observed red by planting its impl.

## Task Commits

1. **Task 1: one resume, end to end, with no interpreter left in the path** — `4b9f18b` (fix)
2. **Task 2: name the third question, then make the rule checkable** — `420fb81` (feat)
3. **Task 3: WR-01 — the carrier's certificate stops being three-sixths of its claim** — `5767f2a` (test)

## Re-measured gates — this plan's figures beside the plan's

Every command under `rtk proxy`. The hook-rewritten form strips `warning:` and `test result:` lines and has produced false negatives four times in this phase; nothing below is inherited from the plan text, `21-REVIEW.md` or `21-VERIFICATION.md`.

| Gate | Plan's figure (`b1d0478`) | Re-measured at base `80bc4c1` | After this plan |
|---|---|---|---|
| `cargo build` | exit 0 | exit 0 | exit 0 |
| `cargo test --workspace --no-fail-fast` | 1387 / 0 / 13 | **1386 passed / 1 failed / 13 ignored**; the failure is the documented flake, green on re-run → effectively 1387 run | **1393 passed / 0 failed / 13 ignored, exit 0** |
| `cargo clippy -- -D warnings` | exit 0 | exit 0 | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | 4 lints, `browser.rs:131,132,133` | **4 lints**, `bool_assert_comparison` ×3 at **`src/browser.rs:155,156,157`**, `cmp_owned` ×1 at `src/project_creator.rs:146` | **4 lints, same two kinds, same two files, same lines** |
| `cargo test --test driver_injection_corpus` | 13 / 0 / 10 | 13 / 0 / 10 | **13 / 0 / 10**, file unchanged (absent from `git diff --name-only`) |

**The plan's clippy line numbers were stale** (`browser.rs:131,132,133` at `b1d0478`; they are `155,156,157` at this plan's base). The plan asked for re-measurement and this is the delta.

**Test delta is fully attributed.** 1387 run at base → 1393 run after = **+6**, exactly the six tests this plan adds: `the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element`, `every_terminal_find_terminal_can_return_gets_a_separator_that_keeps_the_program_options`, `the_separator_table_covers_every_candidate_find_terminal_probes`, `no_executable_line_under_src_hands_an_interpreter_an_interpolated_program`, `the_interpreter_census_does_not_report_the_module_that_builds_its_needle`, `the_interpreter_census_does_not_report_a_fixed_non_interpolated_invocation`. Task 3 added assertions to an existing test rather than a new one.

**Acceptance greps, re-measured against the final tree:**

| Check | Required | Measured |
|---|---|---|
| `grep -c resume_terminal_argv src/ui/screens/detail.rs` | ≥ 3 | **8** |
| `grep -c current_dir src/ui/screens/detail.rs` | ≥ 1 | **19** |
| `grep -c shell_command_fragment src/text.rs` | ≥ 2 | **4** |
| `grep -c 'implements_deref_str\|implements_borrow_str\|implements_serialize' src/text.rs` | ≥ 6 | **24** |
| `grep -c 'assert_eq!' src/text.rs` | (re-measure) | **16** |
| `grep -c Under-detection src/text.rs` | present | **3** |
| `git diff --stat` (whole plan) | only the two planned files | **`src/text.rs`, `src/ui/screens/detail.rs`** |

## The five verbatim REDs

Each observed against the unfixed code, each followed by a clean tree.

### 1. The argv control against the pre-fix construction (Task 1)

`resume_terminal_argv` was first written as a pure function reproducing the CURRENT construction, and the control run against it:

```text
thread 'ui::screens::detail::tests::the_resume_argv_carries_a_hostile_session_id_as_one_opaque_element' (1949839) panicked at src/ui/screens/detail.rs:7166:17:
assertion `left == right` failed: the session id "demo\u{200b}" must appear as exactly ONE argv element byte-identical to `as_raw_for_logic_only()`; it appeared 0 times in ["-e", "sh", "-c", "cd '/tmp/stand-in' && claude --resume 'demo\u{200b}'"]. Zero means the id was interpolated into some larger string — which is a program a parser will read, not an argument `execve` hands over unread.
  left: 0
 right: 1
```

**The assertion that fired first** is (1), the one-element byte-identity check, on `demo\u{200b}` — the first member of the imported `LOOK_ALIKE_PAIRS` corpus. The arity assertion did *not* fire, and that is informative: the pre-fix vector also had four elements. Arity alone would have passed. What separates the two shapes is *where the id is*, not how many elements there are.

### 2. The interpreter census against a planted line (Task 2)

The plant was deliberately the **multi-line `.args([..])` shape CR-01 itself had** — interpreter on one physical line, `format!` on another — added to `src/driver/liveness.rs`, a file this plan does not otherwise touch:

```text
thread 'text::tests::no_executable_line_under_src_hands_an_interpreter_an_interpolated_program' (2006122) panicked at src/text.rs:1799:9:
assertion `left == right` failed: 1 executable line(s) under src/ hand a command interpreter a program string with a value interpolated into it. Sites: ["src/driver/liveness.rs:281"]. Every metacharacter of that value is a candidate token there — a quote, a semicolon, a backtick, a dollar-parenthesis or a newline is CODE. The repair is to delete the interpreter: build an argv vector and let `execve` receive the bytes unparsed. Quoting for the interpreter is the weaker answer and `Untrusted::as_raw_for_logic_only`'s doc says why.
  left: 1
 right: 0
```

This red does double duty: it is the census's teeth, **and** it is the evidence that the logical-line join reaches across the interpreter line and the interpolation line — i.e. that the census would have caught CR-01 itself, not merely a single-line caricature of it. After removal, `rtk proxy git status --porcelain` showed ` M src/text.rs` alone.

### 3–5. The three planted trait impls (Task 3)

Planted **one at a time**, each removed with a clean `git status --porcelain` before the next:

```text
thread 'text::tests::an_untrusted_carrier_implements_none_of_the_string_conversions' (2050107) panicked at src/text.rs:2194:9:
`Untrusted` implements `Deref<Target = str>`. This is the one that silently rewrites code that is ALREADY WRITTEN: auto-deref restores the raw path at every existing call site in the tree at once, with no diff at any of them and every test in this repository still green. Remove the impl; the raw path is `as_raw_for_logic_only()` and it is meant to be conspicuous.
```

```text
thread 'text::tests::an_untrusted_carrier_implements_none_of_the_string_conversions' (2053239) panicked at src/text.rs:2202:9:
`Untrusted` implements `Borrow<str>`. A carrier that borrows as `&str` is usable as a `&str` map key and can be handed to any sink that takes one — the same hole as `AsRef<str>`, through a second door.
```

```text
thread 'text::tests::an_untrusted_carrier_implements_none_of_the_string_conversions' (2055074) panicked at src/text.rs:2209:9:
`Untrusted` implements `serde::Serialize`. Persistence must go through `as_raw_for_logic_only()`, which is a choice a reviewer sees in a diff; a derived `Serialize` writes the raw bytes to disk from any struct that happens to contain one, invisibly.
```

**Each presence control arm was confirmed BEFORE its absence arm was written.** The control type is `String`, which genuinely implements all three bounds (`Deref<Target = str>`, `Borrow<str>`, `serde::Serialize`); the three presence assertions were added and run green first, and only then were the absence assertions written. This ordering is the whole discipline — a probe whose bound is subtly wrong answers `false` for everything, at which point the absence passes forever while certifying nothing.

## The fixed spawn site, quoted verbatim

`src/ui/screens/detail.rs`, the Sessions-tab resume:

```rust
                                        let short_id = shorten_session_id(sid);
                                        // ARGV ELEMENTS, every one of them —
                                        // the kernel hands them to `execve`
                                        // unparsed. NOT program fragments,
                                        // which is what round 9's comment here
                                        // wrongly claimed they already were:
                                        // "A SUBPROCESS ARGUMENT: the raw id is
                                        // what `claude --resume` must receive,
                                        // and an escaped one would resume
                                        // nothing." (round 9). The value was a
                                        // fragment of a program handed to an
                                        // interpreter through `-c`, so a quote
                                        // in it ran arbitrary code (CR-01).
                                        // The third sink kind no longer exists
                                        // here: there is no parser left. The
                                        // working directory travels through
                                        // `current_dir`, not as a `cd` written
                                        // into a program. See
                                        // `resume_terminal_argv`'s doc.
                                        match std::process::Command::new(&term)
                                            .args(resume_terminal_argv(&term, sid))
                                            .current_dir(&session.working_dir)
                                            .spawn()
```

**No element of this argv is a command interpreter, and nothing is interpolated into any element.** `resume_terminal_argv` returns exactly four `String`s — the separator, `"claude"`, `"--resume"`, and `sid.as_raw_for_logic_only().to_string()` — none of which is built by concatenation or formatting. `terminal_program_separator` returns a `&'static str` from a match. The working directory is a `&Path` handed to `Command::current_dir`. There is no `format!` and no `+` anywhere in the construction.

## The two corrected comments, side by side

### At the spawn site (round 9 → this plan)

| Round 9 (falsified) | Now |
|---|---|
| *"A SUBPROCESS ARGUMENT: the raw id is what `claude --resume` must receive, and an escaped one would resume nothing."* | *"ARGV ELEMENTS, every one of them — the kernel hands them to `execve` unparsed. NOT program fragments… The value was a fragment of a program handed to an interpreter through `-c`, so a quote in it ran arbitrary code (CR-01). The third sink kind no longer exists here: there is no parser left."* |

Round 9's **second clause was true** — the raw id *is* what `claude --resume` must receive, and that capability requirement is preserved byte-for-byte. Its **first clause was false**, and the false half is the dangerous one: it named the sink kind wrongly, which is what told the next reader the site was answered.

### At `as_raw_for_logic_only` (round 9 → this plan)

| Round 9 (falsified) | Now |
|---|---|
| *"The raw bytes, for lookups, comparisons, map keys, path segments, subprocess arguments and persistence ONLY."* | *"The raw bytes. **Three questions, not two** — and the third one has no site left in this codebase."* followed by the enumeration: **a lookup** (inert), **an argv element** (inert), **a fragment of a program an interpreter will parse** (not inert), and the rule `shell_command_fragment`: *a value reaching a subprocess goes in as its own argv element, and never into a program string an interpreter will parse.* |

There is deliberately **no third accessor**. Minting `as_raw_for_a_shell` would mint a supported way to do the unsafe thing. The rule is checked by the census instead of offered by an API.

## The separator table

| `find_terminal` returns | Separator | Why |
|---|---|---|
| `kitty` | `-e` | takes the program and its arguments after `-e` |
| `alacritty` | `-e` | `-e`/`--command` consumes the remainder |
| `gnome-terminal` | `--` | its `-e` is deprecated and takes a SINGLE string it re-parses; with a real argv it consumes `--resume` as one of its OWN options |
| `xterm` | `-e` | `-e` consumes the remainder |
| anything else (`$TERMINAL`) | `-e` | the xterm-compatible convention |

The match is on the **file name**, so `$TERMINAL=/usr/bin/gnome-terminal` is recognised rather than falling through. Two tests cover it: one over every candidate plus an arbitrary `$TERMINAL` and a path-qualified form, asserting both the separator and that `--resume` lands *after* it; and a second that reads `find_terminal`'s own candidate slice out of the source and fails if it grows a fifth candidate without a separator decision — so the table's coverage claim is checkable rather than a comment.

This is the defect the security fix would otherwise have **introduced**: while the resume was one opaque program string, the emulator never saw the resumed program's options — the interpreter did. Getting `gnome-terminal` wrong fails silently (the emulator swallows `--resume` as an unrecognised option of its own and the operator sees a terminal that resumed nothing), which is a feature deletion wearing a security fix's clothes.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A SECOND site with the identical injection construction, not named in the plan**

- **Found during:** Task 1, while grepping `src/` for interpreter spawn sites to scope Task 2's census.
- **Issue:** `src/ui/screens/detail.rs` (the `'n'` key, "launch new Claude session") carried the same construction — `Command::new(&term).args(["-e", "sh", "-c", &format!("cd '{}' && claude", project.path.display())])`. The interpolant is a registered project path rather than a `/proc` scrape, so it is lower severity, but it is the same defect class. Independently, Task 2's census requires **zero** interpreter-with-interpolation sites under `src/`; leaving this one would have made the census red.
- **Fix:** Same mechanism — a `launch_terminal_argv` sibling function plus `.current_dir(&project.path)`. No interpolation, no interpreter.
- **Files modified:** `src/ui/screens/detail.rs` (in scope for this plan).
- **Verification:** The census returns zero over `src/`; `cargo test --lib -- ui::screens::detail` green.
- **Committed in:** `4b9f18b` (Task 1 commit).

**2. [Rule 1 - Bug] The census's own fixtures were hits for a DIFFERENT census in the tree**

- **Found during:** Task 2, on the first full workspace run — measured, not anticipated.
- **Issue:** `tests/spawn_seam_guard.rs::every_process_spawn_site_in_src_is_on_the_allowlist` scans `src/` for the literal `Command::new(`. The non-ban arm's fixture strings spelled it whole, which put `src/text.rs` on that guard's unexpected-spawn-site list:
  ```text
  thread 'every_process_spawn_site_in_src_is_on_the_allowlist' (2026328) panicked at tests/spawn_seam_guard.rs:451:5:
  a process-spawn site appeared in a file that is not on the allowlist. Confirm it takes a capability type and then add it to `SPAWN_ALLOWLIST` in this file, in the same commit (PITFALLS:521). Unexpected: ["src/text.rs"]
  ```
- **Fix:** The spawn constructor is now assembled at runtime from halves, exactly as the interpreter names already were. **`tests/spawn_seam_guard.rs` was RUN and not edited** — the plan lists it among the files to run and never touch, and adding `src/text.rs` to `SPAWN_ALLOWLIST` would have been the wrong repair (there is no spawn there, only a string that looked like one).
- **Files modified:** `src/text.rs`.
- **Verification:** `cargo test --test spawn_seam_guard` → 38 passed / 0 failed.
- **Committed in:** `420fb81` (Task 2 commit).
- **Generalisation worth carrying forward:** a census fixture must be inert to **every** census in the tree, not just its own. This is now recorded in the fixture's comment.

**3. [Rule 1 - Bug] A `needless_borrow` clippy lint introduced by this plan's own test code**

- **Found during:** Task 1's clippy gate — `cargo clippy --all-targets` reported **five** lints where the invariant is four.
- **Issue:** `resume_terminal_argv(&term, &sid)` where `term` was already `&str` from an array iteration.
- **Fix:** `resume_terminal_argv(term, &sid)`.
- **Verification:** Back to exactly four lints, same two kinds, same two files.
- **Committed in:** `4b9f18b`.

---

**Total deviations:** 3 auto-fixed (3 × Rule 1). **Impact:** deviation 1 closes a real second instance of the Critical this plan exists for and was forced by the plan's own census requirement; deviations 2 and 3 are self-inflicted defects in this plan's new test code, caught by existing gates and fixed at the introducing commit. No scope creep — every change is inside the two files the plan declares.

## Issues Encountered

### The `driver_reattach` flake has a mechanism, and it is cross-worktree

Phase constraint 7 documents `tests/driver_reattach.rs`'s two tests as non-deterministic under parallel execution. **Reporting it explicitly rather than absorbing it**, per that constraint — and with a mechanism this run identified:

- At base, `cargo test --workspace` failed `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` (`assertion left == right failed: exactly one project has a run to observe; left: 0, right: 1`), and passed 3/3 under `--test-threads=1`.
- Later, the same test **failed even under `--test-threads=1`** — which the documented workaround does not predict. `pgrep -af` showed the cause: `/home/blk/projects/rust/gsd-meta-manager/.claude/worktrees/agent-aef024de6e415df0e/target/debug/deps/driver_kill-…` — **a sibling worktree's test binary running concurrently**, i.e. 21-28 or 21-29 executing in this same parallel wave.
- These tests discover runs by scanning `/proc` system-wide (`live_within`, `is_run_alive`). That scan does not stop at the worktree boundary, so a sibling wave agent's driver processes contaminate it. `--test-threads=1` cannot fix machine-wide interference.
- Confirmed by construction: with no sibling `worktrees/agent-*/target/debug/deps` process live, the binary passes **3 passed / 0 failed**, and the final full workspace run is **1393 passed / 0 failed / 13 ignored, exit 0**.

**This is not a regression from this plan** — this plan touches two TUI key handlers and a `#[cfg(test)]` probe, none of which is reachable from the `--drive` path these tests exercise. It is worth recording that the documented "parallel execution" flake is broader than intra-binary parallelism: **it is cross-worktree**, so any wave agent running `--workspace` concurrently with a sibling will see it.

## Residuals, each named with its failure direction

Per this plan's success criteria, every residual is stated with the direction it fails in.

1. **The interpreter census is a source scan over one logical call at a time.** A construction assembled **across statements** — a program string built into a local on one line and handed to an interpreter three lines later — is invisible to it. **Under-detection, silent.** So is a spawn whose interpreter is named by a variable rather than a literal. What bounds these is `as_raw_for_logic_only`'s rule and code review, **not** this census; the doc says so rather than implying the census closes the class.
2. **`project_creator::execute_hook` is deliberately not reported.** It spawns an interpreter on one line and interpolates only into its *error message* on another. That is correct code — the post-create hook **is** a shell command, supplied by the operator's own config — but the census's silence there is a consequence of the per-call scan, not a judgment it made. Named so it is not mistaken for a clean bill of health. (That file is also under prohibition 6 and was not touched: `git diff --name-only` names neither it nor `src/browser.rs`.)
3. **The join rule follows unclosed delimiters, not method chains.** A chain-following rule would join `Command::new("/bin/sh")` in `driver::liveness`'s fixture to its `.args([.., &format!(..)])` three lines below and report correct code. **The chosen rule trades a false-positive risk for under-detection, silently**, and the choice is recorded in the predicate's own doc.
4. **The trait probe certifies six absences, which is not the closed set.** `PartialEq<str>`, `Into<String>`, a future *inherent* method returning `&str` under a different name, and any trait a dependency adds by blanket impl are **not** probed. **Under-detection, silent.** The probe also answers for the type as the test binary sees it, so an impl behind an unenabled Cargo feature is invisible; what bounds *that* is coherence — `Untrusted` is defined in this crate and all six traits are foreign, so the orphan rule leaves nowhere else to put an impl.
5. **`terminal_program_separator`'s unknown-`$TERMINAL` default.** An emulator not in the table that does not follow the `-e` convention gets the wrong separator. **Under-detection of unsupported emulators, but LOUD rather than silent** — the emulator rejects the flag and the operator sees it — which is the direction prohibition 3 asks for: an emulator that cannot be driven by an argv is reported, not silently handled by quoting.
6. **D2 has no automated arrival arm.** No test can spawn a terminal emulator in CI, so "the resume actually resumes in a real gnome-terminal" is human-judgment in the coverage block above. The argv and the separator are proven; the end-to-end is not.

## Prohibition compliance

| # | Prohibition | Status |
|---|---|---|
| 1 | No security rationale that misdescribes the code; every sink comment names the sink KIND | Both new sink comments name the kind explicitly ("ARGV ELEMENTS… NOT program fragments") |
| 2 | The hardening must not delete the capability | Byte-identity of the id is asserted in the argv control; `current_dir` carries the session's directory; the separator table keeps the emulator from eating `--resume` |
| 3 | CR-01 must not be closed by quoting for a shell | No escaping anywhere; the interpreter is removed. Unsupported emulators fail loudly rather than being quoted around |
| 4 | Must not edit 21-28's / 21-29's files or `src/test_support.rs` | `git diff --name-only` (whole plan) = `src/text.rs`, `src/ui/screens/detail.rs`. `LOOK_ALIKE_PAIRS` is drawn **by import**, never respelled |
| 5 | No bare `crate::text::display_identity` call in `detail.rs` | None added; this plan adds no render-path call at all |
| 6 | Must not disturb the four pre-existing clippy lints | Exactly four, same two kinds, same two files, same lines, before and after. Neither file appears in the diff |
| 7 | No work claimed against ROADMAP criterion 4 | `driver_injection_corpus` was RUN (13/0/10, unchanged) and never edited. **No work is claimed against criterion 4; its ten ignored arms need a human with an authenticated Claude subscription and no agent can close it** |
| 8 | No raw invisible/bidi/tag/VS/control character in any file | All such characters appear only as `\u{...}` escapes (`\u{7}` in the metacharacter fixtures); hostile fixtures come from `test_support::LOOK_ALIKE_PAIRS` by import |
| 9 | `.planning/REQUIREMENTS.md` must stay untouched | Untouched — absent from `git diff --name-only` for all three commits |
| 10 | No claim certified by a grep for prose; no count inherited | Every number above was re-measured under `rtk proxy` against the final tree; the plan's stale `browser.rs:131` figure is corrected to `:155` |

Criterion 5's scoping was not re-litigated: `parse_action`'s enum lookup is untouched, and nothing here claims CR-01 was a criterion-5 failure.

## Known Stubs

None. No hardcoded empty value, placeholder string, TODO or FIXME was introduced; every new function has a real implementation and a committed control.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Ready.** Wave-1 siblings 21-28 (CR-02) and 21-29 (WR-05/07/08) are unaffected: this plan's diff names only `src/text.rs` and `src/ui/screens/detail.rs`, and neither is in their file sets. No wave-conflict finding to report.
- **For 21-30 (wave 2, CR-03, also in `detail.rs`):** `detail.rs` now contains `resume_terminal_argv`, `launch_terminal_argv` and `terminal_program_separator` near `find_terminal`, and the tests module gained three tests at the end. The edit-buffer work CR-03 targets is untouched.
- **New constraint for every future plan in this phase:** `src/text.rs` now carries an interpreter census over `src/`. Any new code that hands a command interpreter an interpolated program on one logical call will fail `no_executable_line_under_src_hands_an_interpreter_an_interpolated_program`. The repair is always argv, never quoting.
- **Carry-forward for the wave orchestrator:** the `driver_reattach` flake is **cross-worktree**, not merely intra-binary. Any agent running `cargo test --workspace` concurrently with a sibling will see up to two failures in that binary. Re-run when siblings are idle before treating it as a regression.

## Self-Check: PASSED

- `src/ui/screens/detail.rs` — FOUND (modified, in `git diff --name-only`)
- `src/text.rs` — FOUND (modified, in `git diff --name-only`)
- Commit `4b9f18b` — FOUND in `git log`
- Commit `420fb81` — FOUND in `git log`
- Commit `5767f2a` — FOUND in `git log`
- Task 1 `<verify>` (`cargo test --lib -- ui::screens::detail --nocapture`) — re-run, 56 passed / 0 failed
- Task 2 `<verify>` (`cargo test --lib -- text:: --nocapture`) — re-run, 20 passed / 0 failed
- Task 3 `<verify>` (`cargo test --lib -- text::tests::an_untrusted_carrier_implements_none_of_the_string_conversions --exact`) — re-run, 1 passed / 0 failed
- Plan `<verification>`: `cargo build` exit 0; `cargo test --workspace --no-fail-fast` **1393 / 0 / 13, exit 0**; `cargo clippy -- -D warnings` exit 0; `cargo clippy --all-targets -- -D warnings` exactly four pre-existing lints; `driver_injection_corpus` 13 / 0 / 10 with the file unchanged; `.planning/REQUIREMENTS.md` untouched

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-27*
