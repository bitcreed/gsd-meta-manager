---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 27
subsystem: envelope
tags: [security, control-carrier, T-19-111, T-19-112, T-19-113, T-19-114, T-19-107, C-15, honesty]
requires:
  - "19-26 (the control-carrier corpus) at 22bb5f7, with 10 rows RED and 16 envelope_* binaries"
provides:
  - "policy::envelope_carrier_operand — a pure ABSOLUTE+LITERAL+LEXICAL+COMPONENT-WISE path predicate, no I/O"
  - "ONE call site in classify_segments, before the resolution match"
  - "the T-19-111 attribution correction at all five sites"
  - "a repaired SECTION_ENVELOPE Guaranteed cap clause, 213 tokens against an unraised 215 cap"
  - "D-09's corrected ceiling narrative in cred.rs and advisory.rs"
  - "the corrected proportional-floor comment, with the byte-versus-character explanation"
affects:
  - "any later round touching SECTION_ENVELOPE, classify_segments' per-segment loop, or T-19-111"
tech-stack:
  added: []
  patterns:
    - "red-before-fix: the carry-forward RED set confirmed still red against the base commit before any production line moved"
    - "converse-check: the PERMITTED set re-driven after the fix, because a rule that turns a fail-open row red is a rule that quietly widened"
    - "measure-then-assert: every new row driven against the built binary, with a CONTROL beside it, before it was written"
    - "record_only for a row whose post-fix verdict cannot be derived from the placement"
key-files:
  created:
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-27-SUMMARY.md
  modified:
    - src/envelope/policy.rs
    - src/envelope/hooks.rs
    - src/envelope/cred.rs
    - src/envelope/advisory.rs
    - tests/envelope_control_carrier.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "Option (d): each carrier's control chosen by measurement, not symmetry — (a) for the nine envelope carriers, (b) costed-and-deferred for the ledger, (b) structurally unavailable for the stubs, (e) for the repo-side four and C-15"
  - "The boundary is the envelope DIRECTORY, not a filename list, because rm -rf takes nine carriers in one call"
  - "Reads are refused as well as writes — the guard cannot tell them apart without a program grammar it is forbidden to have"
  - "The rule is raised BEFORE the resolution match, not in the Ungoverned arm, so a governed program touching a carrier is reached"
  - "No revisit condition and no version witness for the four fail-open directions: they are reachable today, so a schedule would observe the wrong thing"
  - "T-19-111 is moved OUT of T-19-86; no rule and no acceptance is written for it"
  - "src/envelope/mod.rs:165-168 was NOT opened — outside files_modified; recorded as a documentation gap"
metrics:
  duration: "one session"
  completed: 2026-09-04
  tasks: 4
  commits: 4
status: complete
actuals:
  tokens: 27000
  tasks: 4
  commits: 4
---

# Phase 19 Plan 27: The Carrier-Operand Rule and the Corrections Summary

**The guard's own controls are files, and after this round a command that names one by an absolute literal operand is a command the guard refuses — in one route of several, and the other routes are named rather than implied.**

## STATED FIRST — `/gsd-secure-phase 19` is NOT cleared by this plan

- **`T-19-86`** — OPEN at `high` by explicit user scoping decision. Unchanged and
  unremediated; its four registered rows and its persisted-alias arm still exit 0
  and its pins are green and UNMODIFIED. **`T-19-111` is moved OUT of it, not into
  it.**
- **`T-19-91`** — OPEN at `high`, arms unweakened.
- **`T-19-111`** — OPEN at `high`. **This plan writes NO rule for it**, only the
  attribution correction, and **no acceptance is made** — that is a human decision.
- **`T-19-112`** — **NARROWED, explicitly NOT closed, and in ONE ROUTE OF
  SEVERAL.** `C-15` and the deferred option (b) are the others.
- **`T-19-113`** — **NARROWED, NOT closed.** D-09's narrative corrected regardless.
- **`T-19-114`** — closed for the control-carrier axis; successor named in the record.
- **`T-19-96`**, **`T-19-110`** — open at `medium`. **`T-19-74`** — core rows frozen.
  **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85`** — open and unaccepted.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** No unqualified
"T-19-60 is closed" appears anywhere in this plan's output.

## The four commits, in order

| # | SHA | What | `git show --stat` |
|---|---|---|---|
| 1 | `6944b52` | `feat(19-27)`: the carrier-operand predicate, `T-19-111`'s attribution, the floor comment | `src/envelope/policy.rs` +544 −18 |
| 2 | `7bc86f2` | `feat(19-27)`: the ONE call site, before the resolution match | `hooks.rs` +64 −0, `tests/envelope_control_carrier.rs` +194 −0 |
| 3 | `ad4211b` | `docs(19-27)`: the honesty repairs | `advisory.rs` +4 −2, `cred.rs` +53 −1, `tests/envelope_control_carrier.rs` +40 −15 |
| 4 | `60f11e8` | `docs(19-27)`: the record | `19-SECURITY.md` +524 −0, `deferred-items.md` +182 −0 |

`git diff --stat HEAD~4..HEAD`: **7 files, 1605 insertions, 36 deletions.** Neither
`Cargo.toml` nor `Cargo.lock` was touched (`T-19-SC`). Both `.planning/` edits are
pure EOF appends (`@@ -8968,0 +8969,524 @@`, `@@ -2036,0 +2037,182 @@`).

## `19-26`'s RED set — confirmed STILL RED first, then GREEN

**Re-run against the unmodified tree at `22bb5f7`, before any production line
moved.** Verbatim:

```text
tests/envelope_control_carrier.rs
  test result: FAILED. 24 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out
failures:
    after_19_27_a_command_naming_the_whole_alias_directory_is_refused
    after_19_27_a_command_whose_operand_is_a_hook_stub_is_refused
    after_19_27_a_command_whose_operand_is_the_askpass_responder_is_refused
    after_19_27_a_command_whose_operand_is_the_pr_cap_ledger_is_refused
    after_19_27_reads_under_the_envelope_root_are_refused_too_and_that_is_a_decision
    after_19_27_the_carrier_rule_reads_a_path_and_not_a_program_name
    direction_ii_an_expansion_borne_operand_stays_permitted_and_its_literal_twin_is_refused
    direction_iii_a_symlink_stays_permitted_and_its_measured_partial_mitigation_is_refused
    direction_iv_a_relative_operand_stays_permitted_and_its_measured_partial_mitigation_is_refused

tests/envelope_wrapper_class.rs
  test result: FAILED. 49 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
failures:
    a_command_whose_operand_resolves_under_the_envelope_root_is_refused_after_19_27
```

**All ten are the exact set `19-26-SUMMARY.md` records. Not one was already
green** — which would have been a finding to report rather than a task to
complete.

**After commit 2, the transition:**

```text
tests/envelope_control_carrier.rs
  test result: ok. 33 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
tests/envelope_wrapper_class.rs
  test result: ok. 50 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

Every one green at the identifier the corpus derived — `envelope_assertion_failed`,
the general unresolvable one — and never `hook_bypass_blocked`.

## The CONVERSE check — `19-26`'s PERMITTED rows are still PERMITTED

**A rule that turned any of them red would be a rule that quietly widened, and
that would have been a finding about the RULE rather than an assertion to edit.**
Every one re-driven against the built binary after the fix, still exit 0: the four
redirection-target rows (direction i), both expansion-borne rows (ii), both
symlink rows (iii), both relative rows (iv), both near-miss controls
(`rm -f /tmp/pr-ledger.ndjson` and the one-character-changed root), the whole
ordinary-operand half, and `git config --get core.hooksPath` pinned UNCHANGED.
**Not one config-resolution verdict moved.**

## Final gate — the arithmetic STATED and CHECKED

`rtk proxy cargo test --no-fail-fast`, counts read with `rtk proxy grep` over a
redirected log (D-34), never a plain `grep`.

| | `19-26` | this plan |
|---|---|---|
| result lines | 45 | **45** |
| passed | 1752 | **1771** |
| failed | **10** | **0** |
| ignored | 13 | 13 |
| **`passed + failed`** | **1762** | **1771** |
| `envelope_*` binaries | 16 | **16** |

```text
src/envelope/policy.rs (mod tests)   :  5 new #[test] fns   (6944b52)
tests/envelope_control_carrier.rs    :  4 new #[test] fns   (7bc86f2)
TOTAL                                :  9
1762 + 9 = 1771  ==  observed passed + failed
```

**A red test RAN, so red→green leaves the total unchanged and every increase comes
ONLY from new `#[test]` fns. The identity holds exactly.**

`cargo build` 0. `cargo clippy -- -D warnings` 0. (`cargo clippy --tests` is NOT
the gate — four pre-existing lints in `src/browser.rs` and `src/project_creator.rs`
fail at base, untouched.)

### Per-binary counts — all SIXTEEN `envelope_*` binaries RAN

| Binary | passed | failed | | Binary | passed | failed |
|---|---|---|---|---|---|---|
| `envelope_advisory` | 10 | 0 | | `envelope_literal_decision` | 43 | 0 |
| `envelope_argv_deletion` | 20 | 0 | | `envelope_pr_cap` | 11 | 0 |
| `envelope_callee_grammar` | 19 | 0 | | `envelope_reparsed_value` | 34 | 0 |
| `envelope_command_position` | 18 | 0 | | `envelope_tracer` | 6 | 0 |
| `envelope_config_resolution` | 30 | 0 | | `envelope_wiring` | 14 | 0 |
| **`envelope_control_carrier`** | **37** | 0 | | `envelope_wrapper_bypass` | 13 | 0 |
| `envelope_credential` | 6 | 0 | | **`envelope_wrapper_class`** | **50** | 0 |
| `envelope_expansion_slots` | 32 | 0 | | `envelope_hook_refusals` | 7 | 0 |

**A DOCUMENTED FLAKE FIRED, recorded rather than smoothed away.** The verification
run after commit 2 reported ONE failure —
`driver_reattach::a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`,
panicking at `driver_reattach.rs:542` with *"the run record is on disk: Os { code:
2, kind: NotFound }"*. One of the two documented `driver_reattach` flakes. **Not
fixed, not worked around, out of scope.** It did not fire on the runs after
commits 1 and 3 or on the final gate run; **absence is not evidence it is fixed**,
and `driver_reattach` exercises no envelope guard path.

## The rule — its four conditions, and the boundary each is

**A segment is refused when any of its words is ABSOLUTE, LITERAL, and — after
LEXICAL normalisation — equal to or under `envelope_dir_in(root, alias)`, compared
COMPONENT-WISE.**

| Condition | The boundary it is |
|---|---|
| **ABSOLUTE** | the guard has no cwd — `guard_in`'s `project_root` is the JOURNAL locator, not the shell's working directory. Direction (iv) |
| **LITERAL** (`Token.literal`) | a word the shell may rewrite is a word the guard cannot resolve; refusing every non-literal operand of an ungoverned command would deny `rm $TMPDIR/x` (AR-19-11). Direction (ii). **This round reads that bit for the first time outside a governed program's decision words** |
| **LEXICALLY NORMALISED**, `..` collapsed textually, **no link followed** | `canonicalize`/`read_link` is I/O and TOCTOU on the guard's critical path. Direction (iii) |
| **COMPONENT-WISE** | `rm -f /tmp/pr-ledger.ndjson` and a one-character-changed root are pinned PERMITTED; a raw `starts_with` over `<root>/alpha` would also refuse a sibling `alpha2` |

**The boundary is the DIRECTORY and not a filename list**, because
`rm -rf <root>/<alias>` takes nine carriers in ONE call. **The predicate names no
program and no filename at all**, asserted mechanically over its own doc-stripped
source against `canonicalize`, `read_link`, `symlink_metadata`, `metadata`,
`current_dir`, `exists` and `Command::new`, with three positive controls so the
absence cannot pass because the slice was wrong.

## The placement, and its three stated consequences

**Raised ONCE**, at the top of `classify_segments`' existing per-segment loop, on
the segment the loop already holds, **BEFORE `resolve_program_with_head`**. One
hunk: `@@ -966,0 +967,64 @@ fn classify_segments(`, **64 insertions, zero
deletions.** No new `ParkReason`, no second pass, no second scan, no third reading
site.

| Consequence | Row | Measured |
|---|---|---|
| BEFORE the resolution match, so a GOVERNED carrier is reached | `git config --file <ENV>/alpha/gitconfig --get user.name` | exit **2**; control outside the envelope exit **0** |
| the same, through a leading git option | `git --git-dir <ENV>/alpha/gh status` | exit **2**; control exit **0** |
| BEFORE the ledger write | no refused command consumes PR-cap budget | walk EMPTY on every refused row |
| re-raised at DEPTH through `NestedPayload` re-entry | `bash -lc "rm -f <ENV>/alpha/pr-ledger.ndjson"` | exit **2**; control exit **0** |

### The two ORDERING pins, at deliberately DIFFERENT identifiers

| Command | Identifier |
|---|---|
| `rm -f <ENV>/alpha/pr-ledger.ndjson && git push --force origin main` | `envelope_assertion_failed` |
| `git push --force origin main && rm -f <ENV>/alpha/pr-ledger.ndjson` | **`force_push_blocked`** |
| CONTROL `rm -f /tmp/x && git push --force origin main` | `force_push_blocked` |
| `git -c core.hooksPath=/dev/null config --file <ENV>/alpha/gitconfig --get x` | `envelope_assertion_failed` |
| CONTROL `git -c core.hooksPath=/dev/null status` | **`hook_bypass_blocked`**, unperturbed |

**A second pass would answer the same identifier for both members of each pair.**
That is what the pairs are for.

## The FOUR fail-open directions — in four places, handed to NO control

Written into the predicate's doc, the refusal's doc, `cred.rs`'s WHAT IS NOT
COVERED list, and the record:

1. **A REDIRECTION TARGET is not an operand** — `: > <ledger>`,
   `printf 'exit 0' > <hooks>/pre-push`. **Real by MECHANISM**: `tokenize`
   consumes bash's redirection production and emits no token for the operator OR
   ITS TARGET. Reading `>` would re-open a model five rounds have pinned shut.
2. **An EXPANSION-BORNE operand cannot be resolved** — and the composition is
   worth naming exactly: the carrier's location is fetched by a **PERMITTED
   GOVERNED READ** (that first segment resolves `Governed { index: 0 }`) and then
   acted on by an ungoverned command.
3. **A SYMLINK is not followed** — **NARROWED** by `ln -s <env>/…` naming an
   envelope path as its own operand.
4. **A RELATIVE path is not resolved** — **NARROWED** by `cd <env>/<alias>`, with
   the composite refused **BY SEGMENT ONE** and segment two still unresolvable.

**NO revisit condition and NO version witness was created for any of them.**
`19-25`'s witness schedules a re-audit against a FUTURE GIT; **these four are
reachable by the agent TODAY**, so a schedule would observe the wrong thing
entirely — a version string that never moves while the residue is exercised — and
attaching one would be `T-19-107`'s shape arriving in the round that inherited the
lesson.

## The design answer — option (d)

**Option (c) is DEAD for every envelope carrier, stated ONCE**: each must be
readable, writable or executable by the same uid the driven process runs as. A
different uid, a container or a remote-side control are the only real (c)s; the
first two are outside this phase and the third is what `SECTION_ENVELOPE`'s own
conclusion already says.

| Carrier group | Control | Fails |
|---|---|---|
| the nine under `<envelope>/<alias>/` | **(a)**, written here | OPEN in four directions |
| the ledger, additionally | **(b)** — **COSTED AND DEFERRED, no schedule, no pin, no witness** | n/a |
| the hook stubs | **(a) only**; **(b) STRUCTURALLY UNAVAILABLE** | (a)'s four |
| the repo-side four **and `C-15`** | **(e)** — no rule, registered, **unaccepted** | OPEN entirely |
| `SECTION_ENVELOPE`'s cap clause | **repaired** — honesty, not a control | n/a |

**The two (b) reasons are different, which is why the carriers get different
controls.** The ledger's (b) is *available and expensive*: append-only entries
carry `run_id` and `at`, so a truncation IS detectable at the next append — but it
needs a SECOND CARRIER `hooks.rs:1421-1428`, `T-19-35` and `AR-19-05` all say
SAFE-06 does not have, and it breaks the *"single pass over one append-only file"*
latency rule. The stub's (b) is *structurally unavailable*: the thing that would
detect a replaced `pre-push` is the binary the replacement removed from the path.
`T-19-01`'s relocated-copy mitigation is correct and **not weakened**.

## The `T-19-111` attribution correction — all five sites

`T-19-86`'s four rows all require a GOVERNED program handed a GOVERNED COMMAND AS
DATA on the line the guard reads. **A non-`!` alias body in a config VALUE is not
that** — it is `REPARSED_COMMAND_SECTIONS`' own K1 class through a carrier outside
argv.

| # | Site | Before → After |
|---|---|---|
| 1 | `policy.rs:1177-1183` | *"That is `T-19-86`'s shape and it stays OPEN."* → *"That is `T-19-111`'s shape and it stays OPEN"* + the one-sentence reason + the measurement + "no rule, no acceptance". The *"or one written by a means that is not `git config`"* half KEPT. **EDITED** |
| 2 | `cred.rs:287-289` | *"a repo-local `.git/config` alias predating the run is live"* → the *"or one written by a means that is not `git config`"* half **RESTORED**, re-attributed to `T-19-111`. **EDITED, doc-only** |
| 3 | `19-SECURITY.md:7430` | **CORRECTED BESIDE**, quoted, never edited |
| 4 | `19-25-SUMMARY.md:265` | **CORRECTED BESIDE**, quoted, never edited |
| 5 | `deferred-items.md:1619` | `T-19-108`'s *"CLOSED AS SCOPED"* corrected beside it to name the excluded **CARRIER** (a file) as well as the excluded **BODY** (`!`) |

**`T-19-111` is moved OUT of `T-19-86` and never into it.**

## D-09's corrected ceiling

| Site | Before → After |
|---|---|
| `advisory.rs:254` | `An agent that unsets GIT_CONFIG_COUNT in a subshell is past the last layer.` → `An agent that unsets GIT_CONFIG_COUNT in a subshell, or rewrites the` / `hook stubs and ledger the envelope installed, is past the last layer.` |
| `cred.rs:241-244` | the environment-route sentence, **plus** the measured file route (`cp /bin/true <hooks>/pre-push` at exit 0, `GIT_CONFIG_COUNT` untouched, bare remote's `main` MOVED, restoring the stub restoring the refusal), the note that point 3's *"cannot uninstall it by editing a file in the repository"* is true and is a different claim, the NARROWED-not-closed statement, the four open directions, and `T-19-01` recorded as unweakened |
| `mod.rs:165-168` | **READ, NOT opened. Recorded as a documentation gap for a later round.** |

## The `SECTION_ENVELOPE` repair

**The complete pin hit set was enumerated BEFORE the text moved** — 40 hits from
`grep -rn 'SECTION_ENVELOPE\|append-only ledger\|Pull-request cap\|envelope_notice' src/ tests/`,
across `driver/run.rs` (2), `driver/dry_run.rs` (7), `advisory.rs` (5),
`hooks.rs` (1), `policy.rs` (2), `tests/envelope_wrapper_class.rs` (1),
`tests/envelope_advisory.rs` (10) and `tests/envelope_control_carrier.rs` (10).

| Before | After |
|---|---|
| `Pull-request cap: an append-only ledger this repository does not contain.` | `The pull-request count lives in` / `an append-only ledger this repository does not contain.` |

**What changed is the KIND of statement.** The old clause named a CONTROL and then
described the ledger, so a reader took the cap as guaranteed. The new one states a
LOCATION FACT and stops. **It makes no completeness claim and enumerates nothing**
— `C-15` is a fifth route and the deferred option (b) a sixth, and a clause listing
four directions and stopping would be `T-19-107`'s shape in a shorter sentence.

**Six mechanical constraints, verified against the RENDERED constant:** 213 tokens
against the **unraised** 215 cap; widest line 74 of 80; no pinned phrase wrapped
through; the three pinned phrases in their pinned ORDER; `\x20` indentation
intact; **no residual disclosure deleted**. Nothing was shortened — two clauses
were added, both LIMITATIONS.

**Assertions updated: exactly ONE**, the one `19-26` named in advance —
`the_guaranteed_cap_clause_is_pinned_at_its_current_text_and_19_27_must_change_it`.
**No `tests/envelope_advisory.rs` assertion needed updating**, because the repair
kept the phrase `append-only ledger this repository does not contain` VERBATIM;
that file shows a **ZERO diff** and all ten of its tests pass UNMODIFIED —
including `the_honesty_statement_carries_each_of_its_three_required_parts`, its
phrase-order assertion and
`the_honesty_statement_stays_short_enough_that_a_reader_finishes_it`.
`driver/dry_run.rs`'s three consumers pass UNMODIFIED (`driver_dry_run` 15/0,
`envelope_advisory` 10/0).

## The NAMED FILE EXCEPTIONS, stated AS exceptions

| File | Bound | Held? |
|---|---|---|
| `src/envelope/hooks.rs` | ONLY the call site in `classify_segments`' loop | **YES** — one hunk, 64 insertions, **zero deletions**; every named item shows ZERO diff lines. **The `settings_json` second-carrier row was NOT repaired** |
| `src/envelope/cred.rs` | ONLY the `hooks_path_env` limit paragraph, **zero non-doc lines** | **YES** — `git diff --unified=0` filtered for non-`///`, non-blank lines returns **NOTHING** |
| `src/envelope/advisory.rs` | ONLY `SECTION_ENVELOPE`'s literal | **YES** — 4 insertions, 2 deletions, all inside the constant |
| `src/envelope/mod.rs` | the possible FOURTH | **DECLINED** — not in `files_modified`, and the verification requires `git diff --stat` touch no file outside it. The plan's own fallback |

## The over-refusal cost, from both sides

**Reads are refused as well as writes, and the reason is stated**: the guard
cannot tell one from the other without a program grammar
`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
mechanically forbids. So `cat <ledger>`, `wc -l <ledger>` and
`ls <env>/<alias>/hooks` are refused and **a run cannot inspect its own envelope
directory.** **The permitted twin**: `git config --get core.hooksPath` stays at
exit 0, pinned UNCHANGED, and the refusal names the directory (AR-19-11). Every
ordinary-operand row stays at exit 0. **Not one config-resolution verdict moved.**

The refusal never quotes the command back (SAFE-04), carries
`ParkReason::EnvelopeAssertionFailed` and **not** `HookBypassBlocked` (D-24), and
reuses neither the include nor the hooks-path wording.

## The corrected floor comment — both measured numbers, and why the recorded ones disagreed

| Measurement | Number |
|---|---|
| production half at BASE (`22bb5f7`) | **262,229 bytes**, 5,162 lines |
| production half AFTER this plan's additions | **277,570 bytes**, 5,414 lines |
| ratio written into the comment | 180,000 / 277,570 = **64.8%** |
| audit 9 (`cc65220`) | 261,387 |
| the round-10 mandate | 261,386 |

**The discrepancy is NOT "one byte of newline convention".** `production.len()` is
`String::len()`, which counts **BYTES**; this file's prose carries **842
multi-byte UTF-8 characters**. Counted as CHARACTERS the base half is **261,387**
— audit 9's number exactly. Counted as BYTES, which is what the assertion
compares, it is **262,229**. Both recorded numbers were character counts.

**The FLOOR stays at `180_000` and its strictness is unchanged.** The stale line
references were corrected too (`fn scan_leading` 416 not 374; 40,000 at ~752 not
~778; 180,000 at ~3,613 not ~3,622; `fn forbidden_repo_path` at ~5,157 of 5,414
not ~4,592 of 4,613). All three stripper protections keep their strictness;
`POLICY_MIN_PRODUCTION_BYTES = 40_000` and `HOOKS_MIN_PRODUCTION_BYTES = 20_000`
are unchanged.

## Mechanism pins — rounds 4 through 9 are not dead code

`SEPARATORS` is **byte-identical** (one commit ever, `84a9b05`);
`policy::is_separator(">")` is `false`; round 5's literalness bit, round 6's
deletion model, round 7's fail-closed callee grammar, round 8's confinement clause
and round 9's re-parse clause are all non-dead, with `aliasx.`/`notalias.` and both
`T-19-86` `!`-bodied rows at exit 0. `Token.literal` was neither cleared nor
repurposed. Each of `policy.rs` and `hooks.rs` still has exactly ONE `#[cfg(test)]`
sentinel.

## `AR-19-04`, `AR-19-05`, `T-19-17r` — recorded, not decided

**`AR-19-04`'s reasoning gap** — *"the envelope regenerates it at each run start"*
does not cover a write DURING the run. **The same shape a fourth time**, beside
`cred.rs`'s *"predating the run"* (repaired here) and `resolve_policy`'s *"a guard
that cannot read configuration confines the run more, never less"*. **RECORDED
beside it and nothing more; NOT un-accepted.** `AR-19-05` likewise. **No `AR-` row
was added, edited, renumbered, accepted or un-accepted.**

**`T-19-17r` — OUTSTANDING for the ninth time.** `grep -cE '^\| AR-19-13 \|'` over
`19-SECURITY.md` is **0**, verified after writing. **This plan adds no
Accepted-Risks-Log row, creates no `AR-19-13`, and does not apply the word
"accepted" to `T-19-17r` anywhere.** Eight agents deliberately left the acceptance
unmade; **this plan is the ninth.**

## Carried forward UNFIXED

- **`C-08` — `settings.json`'s claimed-but-unwired second carrier. Deliberately
  NOT repaired**: repairing a delivery is a change to the SPAWN SEAM and is out of
  this round's scope. Its **behavioural half remains UNMEASURED** and is claimed in
  neither direction; the mechanical half is settled
  (`grep -rn 'settings_json' src/` is two hits while production pushes
  `--settings <path>`).
- **`C-05`, `C-07`, `C-12`, `C-15`** — planner-derived, control (e), no rule, no
  acceptance. **`pr_cap_*` is deliberately NOT clamped**: a product decision a
  security round must not make silently.
- **the `glab --host` forge cell** — unfixed, **`--hostname` KEPT** in
  `FORGE_VALUE_OPTS`. `glab` is confirmed NOT installed, so a pin over that cell
  cannot run against its real callee and a pin that skips is fail-open.
- **`T-19-110`** at `medium`.

## Deviations from Plan

### `[Rule 1 - Correctness]` The plan's own governed-program carrier row would not have turned a mis-placed rule red

- **Found during:** Task 2, by measurement, before either row was written.
- **Plan text:** *"the GOVERNED-PROGRAM carrier row … `git config --file
  <envelope>/alpha/gitconfig alias.x '<body>'` refused for its CARRIER OPERAND,
  with a comment stating that this row exists to turn a mis-placed rule red."*
- **Measured:** that command IS refused by the carrier clause — **and its control
  OUTSIDE the envelope is ALSO refused, at the SAME reason identifier**
  (`envelope_assertion_failed`), by round 9's re-parse clause over `alias.x`. So
  the row would stay green under an `Ungoverned`-arm placement and would certify
  nothing, which is the exact defect the plan wrote it to catch.
- **What was written instead:** two governed spellings whose keys earn no other
  refusal, so the carrier clause is the ONLY thing that can produce the verdict,
  each with a control **measured at exit 0**:
  `git config --file <ENV>/alpha/gitconfig --get user.name` and
  `git --git-dir <ENV>/alpha/gh status`. **The plan's own spelling is RECORDED
  beside them** with its overlap stated, asserted in neither direction — an
  assertion over the identifier alone could not tell the two mechanisms apart, and
  one over the message would pin round 9's wording from round 10's file.
- **Commit:** `7bc86f2`.

### `[Scope]` `src/envelope/mod.rs:165-168` was READ and NOT opened

- **Found during:** Task 3, step 5.
- **Issue:** the plan's Task 3 offers a choice — correct `mod.rs`'s ceiling
  paragraph as a fourth bounded exception, or record it as a gap. Its own
  prohibition list says *"do not touch … `mod.rs`"*, and `src/envelope/mod.rs` is
  **not in this plan's `files_modified`**, while the verification requires that
  `git diff --stat` touch no file outside it.
- **Resolution:** the mechanical constraint settles it. **Not opened**, and the
  paragraph is **recorded as a documentation gap for a later round** — the fallback
  the plan itself provides.
- **Commit:** `ad4211b` (the decision), `60f11e8` (the record).

### `[Editorial]` The floor comment's stale line references were corrected alongside its stale arithmetic

The plan names `:7172`, `:7185` and `:7194` and the *228,785 bytes, 78.7%*
arithmetic. The same block also cited `fn scan_leading` at line 374, 40,000 bytes
at ~778, a production half running to 4,613 lines, and `fn forbidden_repo_path` at
~4,592 of 4,613 — every one drifted by the same cause. They were corrected in the
same edit, inside the same authorised block. **The floor's VALUE and strictness did
not move.** Commit `6944b52`.

No other deviations. No auto-fix attempt limit was reached. **Nothing was fixed
that this plan was not asked to fix.**

## Anything that came up green where RED was expected, or red where PERMITTED was expected

**None in either direction.** All ten carry-forward rows were still RED at
`22bb5f7`; all ten went green after commit 2; every one of `19-26`'s PERMITTED
fail-open, near-miss and ordinary rows was still PERMITTED after the fix.

## Known Stubs

None. This plan adds no placeholder, no `TODO`, no `FIXME` and no skipped test.
Every new `#[test]` fn asserts or, where the verdict could not be derived from the
placement, `record_only`s with the reason stated.

## What remains uncovered

1. **`T-19-86`** — OPEN at `high` by **explicit user scoping decision**. Its four
   registered rows and its persisted-alias arm still exit 0. **`T-19-111` is moved
   OUT of it, not into it.**
2. **`T-19-91`** — OPEN at `high`, arms unweakened.
3. **`T-19-111`** — OPEN at `high`. **Attribution correction only; NO RULE, NO
   acceptance.**
4. **`T-19-112`** — **NARROWED, NOT CLOSED**, in one route of several: the four
   fail-open directions, `C-15`, and the deferred option (b).
5. **`T-19-113`** — **NARROWED, NOT CLOSED.** Same four directions; option (b)
   structurally unavailable.
6. **`T-19-96`**, **`T-19-110`** — open at `medium`. **`T-19-74`** — core rows
   frozen.
7. **`T-19-84`, `T-19-85`, `T-19-61` … `T-19-73`** — open and unaccepted by
   explicit user decision.
8. **`C-15`**, **`C-05`**, **`C-07`**, **`C-08`**, **`C-12`** — registered, control
   (e), no rule, no acceptance. `C-08`'s behavioural half **UNMEASURED**.
9. **`T-19-17r`** — OUTSTANDING, no `AR-19-13`, not accepted.
10. **the `glab --host` forge cell** — unfixed, `--hostname` KEPT.
11. **`src/envelope/mod.rs:165-168`** — D-09's ceiling paragraph, still uncorrected.
12. **the three documented flakes** — one fired during this plan's verification and
    was not fixed.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed. `/gsd-secure-phase 19`
is not cleared by this plan.**

## Self-Check: PASSED

Files:

```text
FOUND: src/envelope/policy.rs
FOUND: src/envelope/hooks.rs
FOUND: src/envelope/cred.rs
FOUND: src/envelope/advisory.rs
FOUND: tests/envelope_control_carrier.rs
FOUND: .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
FOUND: .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
```

Commits:

```text
FOUND: 6944b52   feat(19-27): the carrier-operand predicate
FOUND: 7bc86f2   feat(19-27): the ONE call site
FOUND: ad4211b   docs(19-27): the honesty repairs
FOUND: 60f11e8   docs(19-27): the round-10 record
```

Gate: `passed + failed = 1771` over 45 result lines, 16 `envelope_*` binaries,
0 failures, `1762 + 9 = 1771` checked against `git show`. `cargo build` 0,
`cargo clippy -- -D warnings` 0. Zero deletions under `tests/` other than the ONE
pre-authorised assertion `19-26` named in advance. `grep -cE '^\| AR-19-13 \|'`
is 0.
