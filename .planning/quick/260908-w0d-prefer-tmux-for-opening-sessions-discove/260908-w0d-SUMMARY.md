---
phase: 260908-w0d
plan: 01
subsystem: ui-launch
tags: [tmux, terminal-discovery, argv-safety, cwe-88, xdg]
status: complete
requires:
  - src/terminal_switch.rs (switch_to_session, pre-existing tmux driver)
  - src/ui/screens/detail.rs (resume + new-session launch sites)
provides:
  - crate::terminal_switch::in_tmux
  - crate::terminal_switch::TerminalProbes
  - crate::terminal_switch::LaunchPlan
  - crate::terminal_switch::plan_launch
  - crate::terminal_switch::probe_terminals
  - crate::terminal_switch::open_new_window
  - detail::FALLBACK_TERMINAL_CANDIDATES
  - detail::DISCOVERY_LAUNCHERS
  - detail::resolve_launch_plan
  - detail::claude_resume_args / claude_launch_args
  - detail::tmux_resume_argv / tmux_launch_argv
affects:
  - Sessions tab Enter (resume)
  - Sessions tab `n` (new session)
tech-stack:
  added: []
  patterns:
    - "pure decision function over injected probes; the impure edge holds no ordering of its own"
    - "fail-closed sink on the shared boundary, not a rule each builder must remember"
    - "source-derived adjudication table with a positive control and a two-way expectations check"
key-files:
  created: []
  modified:
    - src/terminal_switch.rs
    - src/ui/screens/detail.rs
decisions: [D-01, D-02, D-03, D-04, D-05, D-06]
metrics:
  duration: one session
  tasks: 3
  files: 2
  completed: 2026-09-08
actuals:
  tokens: 28000
  tasks: 3
  commits: 3
plan_head_before: edcbdd4
---

# Quick Task 260908-w0d: Prefer tmux for opening sessions, and discover the real default terminal — Summary

Both Sessions-tab launch sites now open a new window on the **running tmux server** when the
user is inside tmux, and fall back to the terminal the **desktop actually designates** —
resolved through `xdg-terminal-exec` — rather than to whichever emulator happened to sit
earliest in a hardcoded list.

## What was wrong

Two independent defects, both in `find_terminal()`:

1. **No tmux branch existed.** The resume site and the `n` site both called `find_terminal()`
   and never looked at `$TMUX`. The repo already drove the running tmux server in
   `terminal_switch::switch_to_session`, but that was wired only to the *focus-switch* action,
   never to launching.
2. **No system-default discovery.** `find_terminal()` tried `$TERMINAL`, then
   `["kitty","alacritty","gnome-terminal","xterm"]`, returning the first that existed. On the
   reporting machine that landed on **gnome-terminal** purely by list order, on a system whose
   real default terminal is **ptyxis**.

## What was built

**`plan_launch` is a pure function.** `TerminalProbes` carries the five facts;
`probe_terminals` gathers them and holds no ordering of its own; `plan_launch` decides. The
whole order — including the reported bug's exact measured environment — is now asserted by a
table test that spawns nothing, reads no environment variable, and does not depend on which
terminals are installed on the machine running it.

`LaunchPlan` carries **both** `try_tmux` and `gui`, because the launch order is "tmux, and on
failure the GUI": a caller that gets an `Err` from tmux must fall through without re-deriving
the decision, and re-deriving is exactly where a second, differently-ordered copy would grow.

**`open_new_window` is the fail-closed sink for D-03.** It refuses a program vector shorter
than two elements, and one whose head begins with `-`, **before constructing any
`std::process::Command`** — which is also what lets those refusals be tested on a machine with
no tmux at all. The reasoning lives on the sink rather than on each builder, so every future
caller inherits it.

**One construction site for the fused resume element.** `claude_resume_args` feeds both
`resume_terminal_argv` (GUI) and `tmux_resume_argv` (tmux), and
`both_resume_paths_carry_the_same_fused_element` is what makes "one site" checkable rather
than a claim.

## Fail-first observations (pin 3)

The plan required proving the strengthened pin fails on an undecided launcher. Two mutations
were run, because the first one alone would not have shown the *strengthening* to be
load-bearing.

**Observation 1 — a launcher ADDED (`undecided-terminal` appended to
`FALLBACK_TERMINAL_CANDIDATES`, count 7 → 8).** Verbatim:

```
thread 'ui::screens::detail::tests::the_separator_table_covers_every_launcher_the_discovery_consts_name'
(2944500) panicked at src/ui/screens/detail.rs:8846:9:
assertion `left == right` failed: the discovery consts name ["kitty", "alacritty", "ptyxis",
"gnome-terminal", "xterm", "undecided-terminal", "xdg-terminal-exec", "x-terminal-emulator"];
the separator table was written against exactly seven launchers (5 fallback + 2 discovery). A
launcher added there without a separator decision here inherits `-e` BY SILENCE — which is how
`gnome-terminal` would have been wrong, and `ptyxis` after it.
  left: 8
 right: 7
```

This fired on the **arity** assertion, which the previous version of the pin also had. So it
proves the pin still guards, but says nothing about D-05's strengthening.

**Observation 2 — a launcher SUBSTITUTED (`undecided-terminal` replacing `xterm`, count held
at 7), which is the mutation only the new two-way table can catch.** Verbatim:

```
thread 'ui::screens::detail::tests::the_separator_table_covers_every_launcher_the_discovery_consts_name'
(2945696) panicked at src/ui/screens/detail.rs:8870:17:
the launcher "undecided-terminal" is named by a discovery const and has NO row in this test's
expectations table, so no separator decision was ever made for it. It would inherit `-e` from
`terminal_program_separator`'s `_` arm — silently, and wrongly for any emulator that re-parses
a single string.
```

The old assertion, `matches!(separator, "-e" | "--")`, would have been **GREEN** on exactly
this mutation: `terminal_program_separator` has a `_ => "-e"` arm, so the `matches!` holds for
any string whatsoever. The pin's own claim — "a candidate added without a separator decision
fails this" — was therefore **false before this change**, and only the arity was ever enforced.
That is now recorded in a comment on the test.

Both mutations were reverted; the const is back to its five authored names.

## Test counts

| | passed | failed | ignored |
|---|---|---|---|
| Baseline (`edcbdd4`) | 1929 | 1 | 13 |
| After this task | **1936** | **1** | **14** |
| Delta | +7 | 0 | +1 |

The +7 are the seven new passing tests; the +1 ignored is the live-tmux direct-exec probe. No
other movement. The single failure is the **pre-existing and unrelated**
`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
(installed git 2.53.0 vs derived 2.43.0), which was not touched.

`cargo build` clean. `cargo clippy -- -D warnings` **exit 0**. `cargo clippy --all-targets`
lints were checked to confirm none of them is in `terminal_switch.rs` or `detail.rs` — they sit
in `tests/envelope_*.rs`, `src/browser.rs` and `src/project_creator.rs`, all pre-existing and
all outside the specified gate.

### A measurement trap worth recording

The first summation of the suite reported **1438 passed**, which looked like a catastrophic
regression. It was not: the `grep`/`awk` pipeline I used to sum `test result:` lines was
rewritten by the `rtk` hook into `rtk grep`, which **truncates** its output ("+22 more in
..."). The `cargo test` invocation itself was correctly proxied; the *measuring* commands were
not. Re-running the same sum under `rtk proxy awk` gave 1936. The project CLAUDE.md warns that
rtk filters cargo output — the same warning applies to every command in the measurement
pipeline, not just the one producing the output.

## Deviations from Plan

### Auto-fixed issues

**1. [Rule 3 — Blocking] `DISCOVERY_LAUNCHERS` was dead code and failed the clippy gate**

- **Found during:** Task 2
- **Issue:** The const is a registry the source-derived guard parses; the launcher names the
  runtime actually uses live in `plan_launch` and `probe_terminals`. `cargo build` therefore
  reported `constant DISCOVERY_LAUNCHERS is never used`, which fails `-D warnings`.
- **Fix:** Kept the const in its registry role — matching the precedent of
  `session_detector`'s `CLAUDE_ARGV_SITES` and `SiteDisposition::Excluded`, both of which are
  adjudication vocabulary rather than runtime values — with `#[allow(dead_code)]` and a doc
  paragraph naming what does read it. Because a registry the runtime does not read *can* drift
  from the runtime, a new pin
  (`the_discovery_consts_name_every_launcher_the_plan_can_actually_return`) drives the **real**
  `plan_launch` at each discovery rank and fails if it ever answers with a name the registry
  does not carry. Without that pin the const would be exactly the artefact D-05 exists to
  replace: a list somebody remembers.
- **Files modified:** `src/ui/screens/detail.rs`
- **Commit:** `05b3592`

**2. [Rule 1 — Bug] the live-tmux `#[ignore]`d probe asserted on a pane that no longer existed**

- **Found during:** Task 1
- **Issue:** The probe as specified captured the pane after running `printf`. The program exits
  immediately, so tmux reaps the window before `capture-pane` runs; with `remain-on-exit on`
  the capture returned only `Pane is dead (status 0, ...)`. The test was red for a reason that
  had nothing to do with the property it exists to measure.
- **Fix:** The verdict is read off the **filesystem** instead: the window runs
  `env touch -- <dir>/<payload>`, and the test polls for a file whose *name is the payload*.
  Race-free and strictly stronger as a witness — under `sh -c` the `;` would split the command
  and `$HOME` would expand, so the exact filename could not appear.
- **Files modified:** `src/terminal_switch.rs`
- **Commit:** `46318ea`
- **Result:** verified GREEN against live tmux 3.6 on a private socket. D-03's measurement is
  now re-runnable rather than a fact recorded once in a plan.

**3. [Rule 2 — Missing coverage] pin 2's failure message named only `gnome-terminal`**

- **Found during:** Task 3
- **Issue:** The message explained that `gnome-terminal` "is the one that differs". After this
  task `ptyxis` is a second launcher of the same shape, and a reader hitting the assertion
  would have been told the wrong thing about why.
- **Fix:** Message updated to name both, and why ptyxis gets `--` rather than the default.
- **Files modified:** `src/ui/screens/detail.rs`
- **Commit:** `aa457c2`

### Not deviations

No architectural change was needed (no Rule 4 stop), no authentication gate was hit, and **no
Cargo dependency was added or changed** — `Cargo.toml` is untouched, so T-W0D-SC's legitimacy
gate had nothing to audit. The untracked `.gsd/` directory was not staged at any point.

## Security invariants — preserved, and where each is now pinned

| Invariant | Status | Pinned by |
|---|---|---|
| No command interpreter in any argv (CR-01, T-21-27-01/02) | preserved on both paths | `the_tmux_argvs_are_pinned_at_two_or_more_with_no_interpreter` (values, not source text) + `open_new_window`'s pre-`Command` refusal |
| `RESUME_OPTION_FUSED_PREFIX` keeps its load-bearing trailing `=` (CWE-88, T-21-31-01) | unchanged; now built at ONE site feeding both paths | `the_tmux_resume_argv_carries_a_hostile_session_id_as_one_opaque_element`, `both_resume_paths_carry_the_same_fused_element` |
| Working directory never travels as a `cd` in a program string | preserved; `current_dir` on the GUI path, `tmux new-window -c` on the tmux path | argv-shape assertions + `open_new_window`'s construction |
| A launcher without a separator decision must fail | **strengthened** — was vacuous, now a two-way table | `the_separator_table_covers_every_launcher_the_discovery_consts_name` (observed RED twice) |

The tmux-path assertions are on argv **values**, never on source text. A text grep for
interpreter names would be strictly weaker, and would additionally be invalidated by the very
doc comments that explain why `env` is not one.

## Disclosures — residuals, not resolved items

**D-01 residual: a user inside tmux can no longer force a GUI window by setting `$TERMINAL`.**
This is a real capability loss and it is accepted, not fixed. No new environment variable and
no config key was added to recover it. `$TERMINAL` is not lost: the tmux branch falls through
to it whenever `$TMUX` is unset, no tmux server is reachable, or the `tmux new-window` call
fails — and when it does fall through, the tmux error is now **prefixed onto the status
message** (`tmux: {e} — opened in {term}`) so the fall-through is visible rather than silent.
The trade was made because `$TERMINAL` expresses which *GUI emulator* a user prefers, not
*where a session should appear*; it is usually exported once from a shell profile, so treating
it as a per-invocation intent is what produced the reported bug.

**D-04 residual: bare `ptyxis` may open a TAB rather than a new window, and a single-instance
GUI application may not honour the `current_dir` of the client process.** No
`terminal_leading_options` helper was added and `--new-window` was not prepended, so
`launch_terminal_argv` keeps its exact pinned property — no third element, therefore no
untrusted element — rather than being loosened. `ptyxis` is reachable only from the last-resort
fallback list, i.e. only when `xdg-terminal-exec` is absent, which is rare because
`xdg-terminal-exec` is how ptyxis gets designated as the default in the first place.
`gnome-terminal` (backed by `gnome-terminal-server`) is already the same class, so this is a
**pre-existing property of the fallback list** rather than something ptyxis introduces. It is
recorded on the `ptyxis` row of `FALLBACK_TERMINAL_CANDIDATES`.

**T-W0D-06 (accepted): the `xdg-terminal-exec --print-id` capability probe.** A synchronous,
read-only resolution of the desktop's default-terminal association, run with no untrusted
argument. It is what lets the launcher fall through instead of failing asynchronously — a bare
`spawn()` fails after the point at which anything could be done about it.

## Known Stubs

None. No hardcoded empty value, placeholder string, TODO or FIXME was introduced, and no test
was skipped. The one `#[ignore]`d test added is deliberately ignored (CI has no tmux), was run
manually and observed GREEN against live tmux 3.6, and carries its run command in its doc.

## Not verified here

Both end-user behaviours — pressing Enter and pressing `n` inside a real tmux session, and the
outside-tmux discovery landing on ptyxis — are asserted at the decision and at the argv, not by
driving the TUI. The orchestrator's instruction was explicit that no GUI terminal was to be
spawned for testing. The live-tmux probe covers the tmux delivery boundary itself; the key
handler wiring above it is covered only by reading.

## Self-Check: PASSED

- `src/terminal_switch.rs` — FOUND
- `src/ui/screens/detail.rs` — FOUND
- commit `46318ea` — FOUND
- commit `05b3592` — FOUND
- commit `aa457c2` — FOUND
- `git rev-list --count edcbdd4..HEAD` = 3, matching `commits: 3`
- no file deletions in any of the three commits (`git diff --diff-filter=D` empty)
