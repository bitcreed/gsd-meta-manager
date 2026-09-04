---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 28
subsystem: envelope
tags: [security, corpus, red, gap-closure, control-carrier, T-19-116, T-19-118, T-19-115, T-19-117]
requires:
  - "19-27 (round 10's rules — the carrier-operand clause at hooks.rs:1018-1029)"
  - "audit 10 (19-SECURITY.md) — the modelled surface is complete; finish the tenth plane"
provides:
  - "tests/envelope_carrier_reach.rs — the TENTH evidence file, RED on exactly five derived-post-fix rows"
  - "the CONTROL-CARRIER axis widened from seven classes to ELEVEN, with three refinements and one alphabet moved arm"
  - "C-10 registered as T-19-116 at high, with a deferred-items row and a control letter for the first time"
  - "the exact handoff numbers 19-29 gates against: 1808 passed+failed, 17 envelope_* binaries, 6 named reds"
affects:
  - "19-29 — which writes the rules and the honesty repairs, and is gated on this plan's recorded RED state"
tech-stack:
  added: []
  patterns:
    - "corpus-before-rule: the evidence file is written and observed RED before any production line moves"
    - "every asserted row names its CONTROL and states what makes the pair discriminating"
    - "record_only for every row no rule will exist for, with a mechanical self-assertion proving it"
key-files:
  created:
    - tests/envelope_carrier_reach.rs
  modified:
    - tests/envelope_wrapper_class.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "The redirection-target channel is a SEGMENT-borne fact, never a token in the stream — pinned falsifiable by the segment-count assertions"
  - "The binary boundary is an EXACT PATH, not a directory prefix, because its directory is shared"
  - "The ledger size bound is DEADLINE-derived and must not be cap-derived, because the caps are unclamped (C-15)"
  - "T-19-115's three spellings get NO rule; the residue's arithmetic is corrected from four to seven instead"
metrics:
  duration: "one session"
  completed: 2026-09-04
actuals:
  tokens: 63000
  tasks: 3
  commits: 3
status: complete
---

# Phase 19 Plan 28: The carriers rule (a) cannot see — Summary

**Read this first.** This plan **closes nothing**. `T-19-86`, `T-19-91`,
`T-19-111` and `T-19-112` all remain **OPEN at `high`**; `T-19-115`, `T-19-116`,
`T-19-117` and `T-19-118` are **all open at this plan's end**; and
**`/gsd-secure-phase 19` is NOT cleared by this plan, by `19-29`, or by the two
together.** Whether `19-29`'s rules close any of them is audit 11's judgement
rather than either plan's claim. **Only the WRAPPER-OPERAND sub-class of
`T-19-60` is closed** — no unqualified "T-19-60 is closed" appears anywhere in
this plan's output.

One-liner: the corpus and the reproducers for `C-10` (the binary outside the
envelope root), `C-05`'s credential half (the target after a `>`), the three
spellings that clear `Token.literal` without a `$`, and the ledger's unbounded
size on the guard's critical path — measured against the built binary and
against real git, committed **RED**, with **zero `src/` hunks**.

## The base, verified

`git diff --numstat 0092009..ca30c90 -- src/ tests/` returns **nothing**, and so
does `git diff --numstat ca30c90..c82f7d8 -- src/ tests/`. This plan's base
`c82f7d8` is byte-identical to the tree audit 10 measured for every file that
decides a verdict.

## The three commits, in order

| # | SHA | What | `src/` hunks |
|---|---|---|---|
| 1 | `6eeec6e` | `tests/envelope_carrier_reach.rs` — the TENTH evidence file (+2706) | **0** |
| 2 | `5c8d67d` | `tests/envelope_wrapper_class.rs` — the axis, seven classes to eleven (+766 −75) | **0** |
| 3 | `e4efd05` | `19-SECURITY.md` + `deferred-items.md` — the records (+1062, −0) | **0** |

`git diff --numstat HEAD~3..HEAD -- src/` is **EMPTY**. `Cargo.toml` and
`Cargo.lock` are untouched (`T-19-SC` holds).

The 75 deletions in commit 2 are doc-comment rewrites and predicate refinements
inside section 17. **Nothing was removed or narrowed, and that is proved
mechanically rather than asserted**: all **44** pre-existing section-17 alphabet
literals are still present, and every `MIN_*` either kept its value or **rose**.
Two test functions were RENAMED (`..._seven_...` → `..._eleven_...`,
`the_five_verdict_preserving_...` → `the_verdict_preserving_...`) because both
widened; no test function was deleted.

## THE COMPLETE RED NAME LIST — `19-29`'s handoff contract

```text
tests/envelope_carrier_reach.rs   (30 passed, 5 failed)
  after_19_29_a_command_whose_operand_is_the_guards_own_binary_is_refused
  after_19_29_a_redirection_that_writes_the_generated_gitconfig_is_refused
  after_19_29_ordering_pin_a_holds_for_a_redirection_target_too
  after_19_29_ordering_pin_b_holds_for_the_binary_carrier_too
  after_19_29_a_ledger_past_the_size_bound_refuses_while_one_just_under_it_still_counts

tests/envelope_wrapper_class.rs   (51 passed, 1 failed)
  a_command_that_names_a_carrier_rule_a_cannot_see_is_refused_after_19_29
```

**No other test may be red.** Every red name in the evidence file is prefixed
`after_19_29_` by construction, so the contract is readable from the name alone.

**THE COMPLETE PERMITTED LIST** — everything else. Specifically, and named
because these are the rows a widened rule would break: the two
EXACT-PATH-not-PREFIX binary controls (`cp /bin/true <BINARY-PARENT>/some-other-file`,
`ls <BINARY-PARENT>`); the binary's discovery rows (`command -v gsd-meta-manager`,
`cat /proc/self/cmdline`); the binary's verdict-preserving spellings
(`$(command -v …)`, tilde, relative-after-`cd`); all tilde, glob and brace rows in
BOTH word positions; the four operand spellings of the gitconfig write and their
outside-the-envelope controls; `git x2>/tmp/o push --force origin main`; `git - push
--force origin main`; `git -c includepath=…`, `notinclude.path=…`, `a=b`,
`aliasx.q=…`, `notalias.q=…`; `git -c alias.p='!git push --force origin main' p`
(`T-19-86`, may not move); `git config --get core.hooksPath`; `rm -f
/tmp/pr-ledger.ndjson`; and `env -u GSD_MM_ENVELOPE_ROOT ls`.

## The gate, with its arithmetic STATED and CHECKED

```text
command   rtk proxy cargo test --no-fail-fast   (counts read with `rtk proxy grep`, D-34)
baseline  1771  passed + failed   (19-27-SUMMARY.md, 45 result lines)
after     1808  passed + failed   (1802 passed, 6 failed, 13 ignored, 46 result lines)
delta     +37
check     35 new #[test] fns in tests/envelope_carrier_reach.rs   (git show HEAD~2)
        +  2 new #[test] fns in tests/envelope_wrapper_class.rs   (git show HEAD~1)
        = 37.   1771 + 37 = 1808.   **The identity holds exactly.**
```

A red test RAN, so red→green leaves the total unchanged and every increase comes
ONLY from new `#[test]` fns.

**SEVENTEEN `envelope_*` binaries RAN** — sixteen would have meant this plan's
own evidence file did not execute. Per-binary counts:

| binary | passed | failed |
|---|---|---|
| `envelope_advisory` | 10 | 0 |
| `envelope_argv_deletion` | 20 | 0 |
| `envelope_callee_grammar` | 19 | 0 |
| **`envelope_carrier_reach`** | **30** | **5** |
| `envelope_command_position` | 18 | 0 |
| `envelope_config_resolution` | 30 | 0 |
| `envelope_control_carrier` | 37 | 0 |
| `envelope_credential` | 6 | 0 |
| `envelope_expansion_slots` | 32 | 0 |
| `envelope_hook_refusals` | 7 | 0 |
| `envelope_literal_decision` | 43 | 0 |
| `envelope_pr_cap` | 11 | 0 |
| `envelope_reparsed_value` | 34 | 0 |
| `envelope_tracer` | 6 | 0 |
| `envelope_wiring` | 14 | 0 |
| `envelope_wrapper_bypass` | 13 | 0 |
| **`envelope_wrapper_class`** | **51** | **1** |

`cargo build` and `cargo clippy -- -D warnings` both exit 0. **`cargo clippy
--tests` is NOT the gate** — it already fails at base on four pre-existing lints
in `src/browser.rs` and `src/project_creator.rs`, which were not touched.

**Documented flakes.** None of the three fired in the gate run — both
`tests/driver_reattach.rs` failures and the `tests/envelope_tracer.rs` ETXTBSY
race were green. **Absence is not evidence they are fixed.** But the **ETXTBSY
race DID fire during measurement**: the direct `cp /bin/true <binary>` in the
end-to-end drive answered `cp: cannot create regular file '…': Text file busy`.
**That is `C-10`'s own seam and this plan exercises it.** It is RECORDED and was
not fixed — the fixture retries with a backoff and then falls back to the atomic
write-temp-then-rename idiom `hooks::write_stub` itself uses, printing which
spelling succeeded.

## The fifteen-carrier disposition table, with a letter for every one

| Carrier | Threat | Severity | Control |
|---|---|---|---|
| `C-01` … `C-04`, `C-06`, `C-07`, `C-09` | T-19-112 / T-19-113 | high / medium | **(a)** rule (a), operand position |
| `C-05` | **T-19-118** | **high** | **(a) PARTIAL** — four OPERAND spellings only; two REDIRECTION spellings open |
| `C-08` | — | planner-derived | **(a)** for the file; BEHAVIOURAL half **UNMEASURED** |
| **`C-10`** | **T-19-116** | **high** | **(a), EXACT PATH, PARTIAL — NEW** |
| `C-11` … `C-15` | T-19-111 (`C-11`) | high / planner-derived | **(e)** no rule, unaccepted |

**Fifteen carriers, fifteen letters.** `C-10` gets a threat id, a severity, a
`deferred-items.md` row and a control letter **for the first time** — `19-27`'s
disposition table had **no row for it at all**, and that absence is how the
widest of the fifteen survived a round that enumerated it.

## `T-19-116` — `C-10`, driven END TO END with a control beside every leg

The bare-remote fixture was **REBUILT**, not cited. Both remote SHAs on every leg:

```text
CONTROL  real binary      git push --force  -> REFUSED, remote main 38b1ae0 UNMOVED
LEG B    binary replaced by /bin/true       -> COMPLETED, main 38b1ae0 -> 153e6c1 MOVED
CONTROL  binary restored, remote rewound    -> REFUSED, remote main 38b1ae0 UNMOVED
```

**LAYER 2, measured SEPARATELY:** the replaced binary answers **exit 0 (PERMIT)**
to `envelope guard alpha` on `git push --force origin main`, where the real
binary answers **exit 2**. **Layer 2 AND layer 3 in one permitted call** —
strictly wider than `C-02` (`T-19-113`, `medium`, layer 3 only) and wider than
`env -u GIT_CONFIG_COUNT` (layer 3 only, **and REFUSED** at
`hook_bypass_blocked`).

Guard-level rows, all measured exit 0 with an empty walk: `cp`, `mv`, `install -m
0755`, `ln -f` and `printf 'x' >` over the binary. `mv` carries the
NO-PROGRAM-NAMES fence — it appears nowhere in either production half.

**`T-19-01` recorded correct-for-what-it-covers and UNWEAKENED**:
`assert_provenance_in` certifies a RELOCATED COPY, and a replaced exec target
never re-enters the binary that would assert provenance.

**`current_exe()` measured, not assumed.** `/proc/self/exe` is fully resolved
(driving a symlink to `python3` reported `/usr/bin/python3.12`), and
`hooks::install` bakes the same call's answer — **so the baked path and the
guard's path agree by construction.** The residue is different and is disclosed:
a write naming a SYMLINK to the binary is a third string neither covers.

## `T-19-118` — `C-05`'s credential half, with the control driven FIRST

```text
CONTROL  the generated file as write_gitconfig_in leaves it
         git config --get credential.helper   -> exit 1, no helper
         git credential fill https://github.com -> exit 128, secret ABSENT
HARM     printf '[credential]\n\thelper = store\n' >> <ENV>/alpha/gitconfig
         (exit 0 through the guard, EMPTY walk — fail-open direction (i))
         git config --get credential.helper   -> `store`, exit 0
         git credential fill https://github.com -> exit 0, secret PRESENT
```

The secret is recorded **PRESENT/ABSENT and never transcribed**. **Both halves of
`SECTION_ENVELOPE`'s FIRST `Guaranteed` clause are false.**

The six write spellings split exactly four-to-two: `cp`, `sed -i`, `tee -a` and
`shred -u` naming the absolute path are **exit 2**; `printf … >>` and `cat … >>`
are **exit 0**. (A `python3 -c` payload naming the absolute path is also exit 2 —
the path is an operand regardless of what the payload does with it.) The
discriminating control is the same append outside the root, exit 0 before and
after.

**`AR-19-04` is RECORDED and NOT un-accepted.** Its rationale — *"the envelope
regenerates it at each run start"* — does not cover a write DURING the run; the
acceptance is about TAMPERING at `medium` and says nothing about credential
REACHABILITY at `high`. `T-19-118` is registered as its OWN row rather than moved
into `T-19-23` or into the acceptance. **No `AR-` row was added, edited,
renumbered or un-accepted.**

## THE EMPTY-`credential.helper` CANDIDATE CONTROL — IT REPRODUCES

**Measured with `git credential fill` as the CRITERION, with
`git config --get-all credential.helper` recorded BESIDE it:**

```text
                            --get-all credential.helper      credential fill
no injected pair            `store`, `store`      exit 0      exit 0, secret PRESENT
EMPTY-helper pair injected  `store`, `store`, then an        exit 128, secret ABSENT
                            EMPTY line            exit 0
```

**The config query still lists the helper while the fill fails closed** — git's
empty value resets the helper list that RUNS, not the list the config query
ENUMERATES. **A `--get-all` metric would have reported a working control as
broken**, which is exactly the false negative this plan's check caught. The
criterion is the one that names the harm.

**The cost, measured:** `GIT_ASKPASS` is **UNTOUCHED** (a responder still
answers, so the envelope's only token channel is intact); `gh` still operates
(the pair is a git key, and `GH_CONFIG_DIR` closes gh's helper from the other
side); and **a later `-c credential.helper=store` on the same command line
OVERRIDES the reset and the secret comes back** — recorded as a **BOUNDED
RESIDUE, bounded because it is ARGV-VISIBLE and already governed** by round 8's
confinement clause and layer 2's whole grammar, unlike EVERY write spelling.

**All of it is RECORDED and asserted NOWHERE.** An assertion would pin a control
that does not exist yet. **`19-29` REQUIRES this control**, because it reads no
command line at all and therefore defends `C-05`'s credential half against all
seven spellings rather than one — and that reason does not depend on this round's
rule landing.

## `T-19-115` — the seven spellings, in both word positions

The docs say rule (a) *"fails OPEN in FOUR named directions"* and every cited
spelling is `$`-shaped. **There are SEVEN**, and the three unnamed ones need no
prior read, no symlink and no cwd: **(v) TILDE, (vi) GLOB, (vii) BRACE LIST** —
round 5's own `Token.literal` classes, which rule (a) requires and round 10's
corpus could not draw.

Each measured exit 0 with its absolute-literal twin at exit 2:
`rm -rf ~/.local/share/gsd-meta-manager/envelope/alpha`; `cp /bin/true ~/…/hooks/pre-push`;
`shred -u ~/…/pr-ledger.ndjson`; `rm -rf <ENV>/alph?` (nine carriers in one call);
`rm -f <ENV>/alpha/*`; `unlink <ENV>/alpha/pr-ledger.ndjso?`;
`rm -f <ENV>/alpha/{pr-ledger.ndjson,x}`; `rm -rf <ENV>/{alpha,beta}`;
`shred -u <ENV>/alpha/{askpass,x}`. **All three were also drawn in
REDIRECTION-TARGET position — the cell nobody had drawn.** The tilde rows were
driven against the **REAL** envelope root as well as a temporary one.

**The `bash` confirmations, and the refinement they forced.** Every OPERAND
spelling REACHES the file. In redirection-target position the tilde reaches
(11 bytes → 0) and the glob reaches **only because it matches exactly one path**
— with two matches bash answers `ambiguous redirect` — while
**`: > <ENV>/alpha/{pr-ledger.ndjson,x}` does NOT reach**, because brace expansion
produces two words and a redirection target must be one. The row stays in the
corpus as verdict-preserving and the record says it is a permit that costs
nothing rather than a bypass. Also recorded: **`sh` (dash) does not glob a
redirection target at all**; the guard is registered against the agent's `Bash`
tool, so bash's semantics are the relevant ones.

`19-29` writes **NO rule** for any of the three — a tilde needs the environment
and a glob needs the filesystem, both forbidden the guard. **The fix is the
ARITHMETIC.**

## `T-19-117` — the ledger as WORK

```text
level        bytes         ls   git status  git push --force  gh pr create  gh api
empty        0             47   48          44                49            48
10,000       1,010,000     45   46          47                96            96
200,000      20,200,000    46   45          46                1008          943
2,000,000    202,000,000   43   43          46                **7900**      7803
```

**7.90 s against `GUARD_TIMEOUT_SECS = 5`.** The **forge/non-forge split** is the
shape of the finding: the cost falls only on commands that reach
`record_and_check_in`. The inflation is delivered by direction (i), and all three
spellings are exit 0 through the guard. `ends_mid_line` refuses a whole-file read
for a one-byte question **three functions above the unbounded read**.

**The derived bound is 8 MiB, DEADLINE-derived and NOT cap-derived** — the caps
are unclamped (`C-15`), so a cap-derived bound is one the agent steers. It must
fail **CLOSED**: a tail read, a line cap or sampling all UNDER-COUNT, violating
`tally`'s own no-under-count invariant in the direction that permits. The
identifier is `envelope_assertion_failed`, not `PrCapExceeded` (D-24). **The row
carries its discriminating control**: 24 MiB refuses while 1 MiB **still permits
AND still counts** — without the second half the row proves only that a large
number refuses.

**The BEHAVIOURAL half — what the agent CLI does past its registered timeout — is
UNMEASURED and is claimed in NEITHER direction.** `glab` is confirmed **not
installed**, so its cell is recorded as unmeasurable against its callee; **no pin
that would skip is written.** `FORGE_VALUE_OPTS` keeps `--hostname`.

## The direction-(i) answer, and the SEGMENT-COUNT pins

**Closing direction (i) does NOT require the deletion model to read redirection
targets as argv.** Round 6's model answers *which words ARRIVE*; rule (a) asks
*does this line NAME a control's path*. Three source-read facts:
`skip_redirection_target` already walks the target's full extent with its own
quoting rules (its *"Its text is never needed, only its extent"* is the sentence
that goes false — a WR-02 correction); `redirection_operator_len` is a closed
twelve-operator grammar of which only seven take a PATHNAME; and
`split_segments_with_heads` excludes operator tokens and flushes at them, **so a
token in the stream would split a redirected simple command and the leading `git`
would carry an empty argv, which `classify_git` answers `Allow` for.**

**The cost:** two additive fields, a return-type change on a private function
with four call sites, a corrected doc sentence, and a pathname/non-pathname split
of a grammar already enumerated. `SEPARATORS` does not move,
`is_separator(">")` stays `false`.

**The measurements that make it falsifiable — GREEN today:**

```text
git >/dev/null push --force origin main   -> 1 segment, tokens EXACTLY
                                             [git, push, --force, origin, main]
git x2>/tmp/o push --force origin main    -> 1 segment, tokens EXACTLY
                                             [git, x2, push, --force, origin, main]
git 2>/dev/null push --force origin main  -> 1 segment, [git, push, --force, origin, main]
git <<EOF push --force origin main        -> 1 segment, [git, push, --force, origin, main]
git >                                     -> 1 segment, [git], unresolvable = true
is_separator(">") == false   is_separator("<") == false   is_separator("&&") == true
```

**The failure message names BOTH variants**: (a) the SPLIT variant (operator
token, two segments, bare `git` answering `Allow`) and (b) the DISPLACED variant
(ordinary word, one segment, caught by the exact-token clause — `T-19-98`'s
shape). The heredoc row is pinned with the comment that `<<`'s target is a
DELIMITER and must never be recorded as a carrier candidate.

## The two ordering pins, at deliberately different identifiers

**PIN A — within one segment, position does NOT move the identifier** (GREEN, in
operand position): `git --git-dir <ENV>/alpha push --force origin main` and
`git push --force origin main <ENV>/alpha` both answer
`envelope_assertion_failed`, against controls at `force_push_blocked`.
**Measured and recorded rather than smoothed: `git --git-dir=<ENV>/alpha push …`
— the `=`-joined spelling — answers `force_push_blocked`**, because the token
does not begin with `/` and `lexical_absolute_components` rejects it.

**PIN B — across segments, order DOES decide it** (GREEN):
`cp <ENV>/… /tmp/x && git push --force …` → `envelope_assertion_failed`; the
reverse → `force_push_blocked`; the outside control → `force_push_blocked`.

The redirection-target twin of A and the binary twin of B are the RED halves,
asserted at their derived post-fix identifiers.

## The axis — seven classes to eleven

**THREE predicates refined, not two.** Class 1 and class 2 gained a
metacharacter/brace-list guard; class 5 gained a tilde exclusion. **Class 2 had
NO literalness guard at all**, and its alphabet is simultaneously the one moving
to the fail-closed arm — so an unrefined class 2 would have placed PERMITTED glob
and brace targets inside a property asserting REFUSAL, which is worse than a
fired fence.

**`CONTROL_CARRIER_REDIRECTION_TARGETS` MOVED out of the invariance arm**, with
the `19-18`/`19-20`/`19-22`/`19-24`/`19-26` precedent named — **the sixth time
that split has been forced by measurement.**
`CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING` carries the four that stay
permitted.

**Four new classes**, each drawing in BOTH word positions: tilde (8), glob (9),
brace (10) — verdict-preserving, no rule — and **the guard's own binary (11)**,
fail-closed and compared as an **EXACT PATH**.

**Arithmetic re-derived and stated as exact equalities: 61 cases, 12 slots, 11
classes.** Per class: 11, 4, 3, 3, 2, 6, 13, 5, 5, 5, 5 — every deliberate
overlap RECORDED (class 3 and class 4 each gain one; classes 8/9/10 each gain the
preserving alphabet's target). The two binary near-miss controls count in class
**7**, not class 11, which is the exact-path boundary asserted at the predicate
level.

**Three fences.** NO-PROGRAM-NAMES extended so the binary class and each new
spelling class draws a row under a program absent from both production halves
(`mv`, `shred`, `unlink`). PATH-PREFIX-NOT-BASENAME gains the binary's two
controls by name **and asserts they do NOT draw class 11**. **NO-EXPANSION-SPELLING
is new** and counts ENTRIES, not prose.

**Nothing else moved.** The four earlier axes, their predicates, their
degenerate-proofing and their floors are byte-identical;
`MIN_CONFIG_RESOLUTION_CLASSES` is still 6; `POLICY_MIN_PRODUCTION_BYTES` and
`HOOKS_MIN_PRODUCTION_BYTES` are unchanged; the stripper's three protections keep
their present strictness.

## Rows RECORDED and never asserted, with the proof

`C-11` … `C-15`, `C-08`'s behavioural half, `C-05`'s alias half and the
empty-helper candidate control are all driven through `record_only` and printed.
**`no_repo_side_or_candidate_row_is_asserted_and_this_file_says_so_mechanically`
reads this file's own text** — with a positive control first — and fails if any
repo-side path fragment or the injected pair appears inside `refuses(`,
`refuses_carrier(`, `permits(`, `permits_carrier(` or an `assert`.

## Mechanism pins and byte floors, re-measured

All rounds 4 through 10 re-asserted in the new file over the same public
functions and all GREEN: round 5's literalness bit; round 6's deletion model and
its over-deletion control; `SEPARATORS` byte-identical with `is_separator(">")`
false and a positive control; round 7's four callee-grammar directions; round 8's
confinement clause with both `--signed no` controls; round 9's re-parse clause
with `aliasx.`/`notalias.` and `T-19-86`'s `!`-bodied row unmoved; round 10's
clause over the envelope-root operand rows, the near-miss, the permitted read,
`env -u GIT_CONFIG_COUNT` at `hook_bypass_blocked` and
`env -u GSD_MM_ENVELOPE_ROOT ls` at exit 0 (`E-01`, measured INERT).

**`Token.literal` is read in a SECOND place by round 10's rule and in a THIRD by
round 11's**, so clearing or repurposing it would silently widen both rules as
well as turning rounds 5 and 6's pins vacuous.

## The `421` correction, and `mod.rs:165-168`

`src/envelope/policy.rs:7450` says *"842 multi-byte characters"*. **842 is the
byte-minus-character DIFFERENCE; the count is 421.** `policy.rs:7450` is
**`19-29`'s to edit** and this plan did not touch it. The same error at
**`deferred-items.md:2172`** sits inside the plan-19-27 section and is **corrected
BESIDE — quoted in this plan's own section and never edited in place.** Every
other number in that comment was re-derived by audit 10 and is right; **the
floor's value and strictness do not move.**

**`mod.rs:165-168` is put explicitly IN SCOPE for `19-29`.** Its residual-limit
paragraph argues the override *"does not lower a boundary that was standing"*
because the same party could unset `GIT_CONFIG_COUNT`. **`T-19-116` contradicts
it**: replacing the binary lowers layers 2 AND 3 without controlling the
environment the TUI starts in, through a command layer 2 permits — while
`env -u GIT_CONFIG_COUNT` is REFUSED. `19-27` read it and correctly declined
because it was outside its `files_modified`; `19-29` lists `src/envelope/mod.rs`.

## `T-19-17r` is OUTSTANDING for the ELEVENTH time

`grep -cE '^\| AR-19-13 \|'` over `19-SECURITY.md` is **0**. **This plan did NOT
accept `T-19-17r`, added no Accepted-Risks-Log row, created no `AR-19-13`, and
did not apply the word "accepted" to it anywhere** — not in a test name, not in a
comment, not in either record. `AR-19-04` and `AR-19-05` are carried forward
**NOT un-accepted, not re-rated and not renumbered.**

## Deviations from plan

### 1. [Rule 2 — missing critical honesty] The relative REDIRECTION target was dropped from the new preserving alphabet, with its reason recorded

**Found during:** Task 2, while deriving the per-class counts.
**Issue:** The plan's step 2 mandated five entries for the new verdict-preserving
redirection alphabet, including a RELATIVE target. Class 5 is an OPERAND class
and round 11's mandated refinement (exclude a tilde) does not widen it to
redirection targets, so `: > pr-ledger.ndjson` would draw **no carrier class at
all** and be counted by class 7 — *"an ordinary path operand"* — **which is
untrue about a relative carrier reach.**
**Fix:** The alphabet carries FOUR entries (expansion-borne, tilde, glob, brace
targets) and its doc states in full why the relative target is absent and where
the relative direction is drawn instead (`CONTROL_CARRIER_RELATIVE`, operand
position). Widening class 5 to cover redirection targets was the alternative and
was declined: it is a widening the plan did not mandate, and the honest smaller
change is to state the gap.
**Files:** `tests/envelope_wrapper_class.rs`. **Commit:** `5c8d67d`.

### 2. [Rule 2] Two tests split green/red so the RED name list is a clean contract

**Found during:** Task 1, on the first full run.
**Issue:** `after_19_29_a_redirection_that_writes_the_generated_gitconfig_is_refused`
and both ordering pins each carried GREEN-today assertions alongside
derived-post-fix RED ones, so a red hid the green half and the handoff contract
would have named tests that are partly already-passing.
**Fix:** Split into
`the_four_operand_spellings_of_the_same_write_are_already_reached_and_the_control_is_permitted`,
`after_19_29_ordering_pin_a_holds_for_a_redirection_target_too` and
`after_19_29_ordering_pin_b_holds_for_the_binary_carrier_too`. Same total
assertion count; every red is now named `after_19_29_*`. Likewise round 10's
fail-closed arm was kept a SEPARATE property from round 11's
(`a_command_that_names_a_carrier_rule_a_cannot_see_is_refused_after_19_29`), so a
later reader can tell a LANDED rule's red from an UNLANDED one's.
**Files:** `tests/envelope_carrier_reach.rs`, `tests/envelope_wrapper_class.rs`.
**Commits:** `6eeec6e`, `5c8d67d`.

### 3. [Rule 1 — a corpus bug in this plan's own first draft] The tilde loop asserted a redirection twin at a verdict it does not have

**Found during:** Task 1's first run.
**Issue:** The tilde rows were written with an unconditional "absolute-literal
twin is exit 2" assertion, but the fourth pair's twin is a REDIRECTION target —
direction (i), permitted today and refused only after `19-29`.
**Fix:** The twin's verdict is now flagged per row; the redirection twin is
RECORDED with the note that it is asserted in section 3 at the verdict it will
have, rather than here at one it does not. **This is the exact failure the
"assert only what you derived" discipline exists to catch, and it was caught by
running the file rather than by reading it.**
**Files:** `tests/envelope_carrier_reach.rs`. **Commit:** `6eeec6e`.

## Findings recorded and NOT fixed

### A BLOCKING SCOPE FINDING FOR `19-29`

**`19-29`'s redirection-target rule turns
`direction_i_a_redirection_target_is_not_an_operand_and_stays_permitted`
(`tests/envelope_control_carrier.rs:633-663`) RED, and that file is NOT in
`19-29`'s `files_modified`.**

The test pins exactly three rows PERMITTED — `: > <ENV>/alpha/pr-ledger.ndjson`,
`printf 'exit 0' > <ENV>/alpha/hooks/pre-push` and `echo evil >
<ENV>/alpha/askpass` — which are three of the four entries of
`CONTROL_CARRIER_REDIRECTION_TARGETS`, the alphabet this plan moved into the
fail-closed arm **precisely because their verdict the fix CHANGES**.

**This is `19-18`'s `{v}>` blocker in a file neither plan may currently edit,
which is the shape that halted `19-23` mid-plan.** `19-28` did not fix it —
editing `tests/envelope_control_carrier.rs` is prohibited by this plan, and round
10's evidence stays attributable to the round that produced it. **`19-29`'s first
action should be to add that file to its `files_modified` and move those three
rows into an `after_19_29_…` property with their reason stated, or to report the
scope conflict.**

Checked and clear: the other directions pinned in the same file — (ii)
expansion, (iii) symlink to `/tmp/l`, (iv) relative — are verdict-preserving and
unaffected, as is `: > /tmp/l` whose target is outside the root. No other test
file names an envelope-root redirection target as a pinned-permitted row, and no
test outside this round's two names the guard's own binary as an operand.

### The `=`-joined global-option spelling

`git --git-dir=<ENV>/alpha push --force origin main` answers
`force_push_blocked`, not `envelope_assertion_failed`, because the token does not
begin with `/`. **Not a defect and not fixed** — it is rule (a)'s literal-ABSOLUTE
condition applied to a governed program's operands exactly as to an ungoverned
one's — but it is recorded because the space-separated twin answers differently
and a reader could mistake the pair for an inconsistency.

## Known Stubs

None. No stub, placeholder or hardcoded empty value was introduced. Every row in
the new evidence file is either driven and asserted, or driven and printed under
a heading that says RECORDED — and the mechanical self-assertion proves the
second set is not asserted.

The **five `after_19_29_*` failures and the one
`a_command_that_names_a_carrier_rule_a_cannot_see_is_refused_after_19_29` failure
are RED BY DESIGN**, not stubs: they are the evidence that the corpus was capable
of failing before the rule existed, and they are `19-29`'s gate.

## What remains uncovered

1. **`T-19-86`** — **OPEN at `high` by explicit user scoping decision.** Four
   registered rows at exit 0, pins green and UNMODIFIED, `T-19-111` kept OUT of
   it.
2. **`T-19-91`** — OPEN at `high`, three arms unweakened.
3. **`T-19-111`** — OPEN at `high`, **no rule**, corpus rows RECORDED and asserted
   in neither direction; `19-27`'s five-site attribution correction stays as
   performed.
4. **`T-19-112`** — OPEN at `high`, NARROWED not closed.
5. **`T-19-113`** — OPEN at `medium`, NARROWED not closed.
6. **`T-19-115`, `T-19-116`, `T-19-117`, `T-19-118`** — **all OPEN at this plan's
   end.** This plan measured them and went RED; it closed none.
7. **`T-19-96`, `T-19-110`, `T-19-74`** — registered open; `T-19-74`'s core rows
   frozen.
8. **`T-19-84`, `T-19-85`, `T-19-61` … `T-19-73`** — open and unaccepted by
   explicit user decision; `scan.rs` and `config.rs` not opened.
9. **`C-08`'s behavioural half** — UNMEASURED, needs a live agent run, claimed in
   neither direction.
10. **`T-19-117`'s behavioural half** — UNMEASURED, a property of a closed-source
    binary, claimed in neither direction.
11. **`C-15`'s `pr_cap_*`** — unclamped, and deliberately so: clamping a configured
    cap is a PRODUCT decision, not a guard rule.
12. **`T-19-17r`** — OUTSTANDING, deliberately unaccepted for the eleventh time.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.**

## Self-Check: PASSED

- `tests/envelope_carrier_reach.rs` — FOUND
- `tests/envelope_wrapper_class.rs` — FOUND
- `.planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md` — FOUND
- `.planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md` — FOUND
- commit `6eeec6e` — FOUND
- commit `5c8d67d` — FOUND
- commit `e4efd05` — FOUND
- `git diff --numstat HEAD~3..HEAD -- src/` — EMPTY (zero `src/` hunks)
- `grep -cE '^\| AR-19-13 \|' 19-SECURITY.md` — 0
