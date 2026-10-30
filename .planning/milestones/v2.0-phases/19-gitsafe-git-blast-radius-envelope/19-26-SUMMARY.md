---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 26
subsystem: envelope
tags: [security, corpus, red-before-fix, control-carrier, T-19-111, T-19-112, T-19-113, T-19-114, C-15]
requires:
  - "19-25 (round 9's rules) at 406c48e, with all fifteen envelope_* binaries green"
provides:
  - "tests/envelope_control_carrier.rs — the NINTH evidence file, 33 tests, 9 RED"
  - "tests/envelope_wrapper_class.rs section 17 — a FIFTH named axis, 9 tests, 1 RED"
  - "the exhaustive fifteen-carrier enumeration on the record"
  - "the T-19-111 attribution correction with all five sites"
  - "an exact handoff number for 19-27 to gate against"
affects:
  - "19-27 (round 10's rules and corrections) — gated on this plan's recorded RED state"
tech-stack:
  added: []
  patterns:
    - "corpus-first: the alphabet is widened and observed RED before any production line moves (seventh time)"
    - "measure-then-assert: every row driven against the built binary before it is written"
    - "record_only for rows whose post-fix verdict cannot be derived"
key-files:
  created:
    - tests/envelope_control_carrier.rs
  modified:
    - tests/envelope_wrapper_class.rs
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
    - .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
decisions:
  - "CONTROL_CARRIER_CLASSES is a genuine FIFTH axis: no existing axis property can be satisfied by a command reaching no governed program"
  - "The disjointness claim asserted is REACHABILITY, not text — a text-level claim would be false and was measured false"
  - "C-15 gets control (e) — no rule — because it is outside the envelope root; pr_cap_* is deliberately NOT clamped"
  - "T-19-111 is registered SEPARATELY from T-19-86 and must be moved OUT of it"
metrics:
  duration: "one session"
  completed: 2026-09-04
  tasks: 3
  commits: 3
status: complete
actuals:
  tokens: 54000
  tasks: 3
  commits: 3
---

# Phase 19 Plan 26: The Control-Carrier Corpus Summary

**Round 10 leaves the argv plane: the corpus can now draw a carrier that is a FILE, and it goes RED against the pre-fix tree.**

## STATED FIRST — what is NOT cleared

**`/gsd-secure-phase 19` is NOT cleared by this plan, by `19-27`, or by the two
together.** This plan **closes nothing**.

- **`T-19-86`** — OPEN at `high` by explicit user scoping decision. **`T-19-111`
  is moved OUT of it, not into it.**
- **`T-19-91`** — OPEN at `high`, arms unweakened.
- **`T-19-111`** — OPEN at `high`. It gets an **attribution correction and NO
  RULE**; its corpus rows are RECORDED, never asserted.
- **`T-19-112`** — OPEN at `high`, corpus RED.
- **`T-19-113`** — OPEN at `medium`, corpus RED. **`T-19-114`** — OPEN at `low`.
- **`T-19-96`**, **`T-19-110`** — open at `medium`. **`T-19-74`** — core rows
  frozen. **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85`** — open and unaccepted by
  explicit user decision.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** No unqualified
"T-19-60 is closed" appears anywhere in this plan's output.

## The three commits, in order — ZERO `src/` hunks in each

| # | SHA | What | `git show --stat` `^ src/` |
|---|---|---|---|
| 1 | `7196de3` | `test(19-26)`: the NINTH evidence file — the carriers are FILES, RED before the rule | **0** |
| 2 | `d92ea5f` | `test(19-26)`: a FIFTH axis — `CONTROL_CARRIER_CLASSES`, off the argv plane | **0** |
| 3 | `df2243b` | `docs(19-26)`: the round-10 record — the enumeration and the `T-19-111` attribution correction | **0** |

`git diff --numstat HEAD~3..HEAD` across all three:

```text
577    0    .planning/.../19-SECURITY.md
283    0    .planning/.../deferred-items.md
2008   0    tests/envelope_control_carrier.rs
1181   0    tests/envelope_wrapper_class.rs
```

**Zero deletions anywhere under `tests/`.** Both `.planning/` edits are pure
appends at EOF (`@@ -8391,0 +8392,577 @@` and `@@ -1753,0 +1754,283 @@`).
Neither `Cargo.toml` nor `Cargo.lock` was touched. **No `src/` file was opened.**

## The gate — arithmetic STATED and CHECKED

Command: `rtk proxy cargo test --no-fail-fast`, counts read with `rtk proxy grep`
over a redirected log.

| | Before | After |
|---|---|---|
| result lines | 44 | **45** |
| passed | 1720 | 1752 |
| failed | 0 | **10** |
| ignored | 13 | 13 |
| **`passed + failed`** | **1720** | **1762** |
| `envelope_*` binaries | 15 | **16** |

**The identity checked against `git show`, not assumed.** A red test RAN, so
red→green leaves the total unchanged and every increase comes ONLY from new
`#[test]` fns:

```text
tests/envelope_control_carrier.rs :  33 new #[test] fns
tests/envelope_wrapper_class.rs   :   9 new #[test] fns
TOTAL                             :  42
1720 + 42 = 1762  ==  observed passed + failed
```

**The identity holds exactly. No disagreement to report.**

`cargo build` exits 0; `cargo clippy -- -D warnings` exits 0. (`cargo clippy
--tests` is NOT the gate — it fails at base on four pre-existing lints in
`src/browser.rs` and `src/project_creator.rs`, untouched.)

**None of the three documented flakes fired** — not the two `driver_reattach`
failures, not the `envelope_tracer` `ExecutableFileBusy` race. **Absence is not
evidence they are fixed.**

### The COMPLETE RED list — `19-27`'s handoff contract

Ten failures, every one this plan's own:

```text
tests/envelope_control_carrier.rs (9)
  after_19_27_a_command_whose_operand_is_the_pr_cap_ledger_is_refused
  after_19_27_a_command_naming_the_whole_alias_directory_is_refused
  after_19_27_a_command_whose_operand_is_a_hook_stub_is_refused
  after_19_27_a_command_whose_operand_is_the_askpass_responder_is_refused
  after_19_27_the_carrier_rule_reads_a_path_and_not_a_program_name
  after_19_27_reads_under_the_envelope_root_are_refused_too_and_that_is_a_decision
  direction_ii_an_expansion_borne_operand_stays_permitted_and_its_literal_twin_is_refused
  direction_iii_a_symlink_stays_permitted_and_its_measured_partial_mitigation_is_refused
  direction_iv_a_relative_operand_stays_permitted_and_its_measured_partial_mitigation_is_refused

tests/envelope_wrapper_class.rs (1)
  a_command_whose_operand_resolves_under_the_envelope_root_is_refused_after_19_27
```

The three `direction_*` tests are red **only on their MEASURED partial
mitigation** (or, for (ii), on the literal-`..` twin); their fail-open rows are
green today and must stay green after.

### Per-binary counts — all SIXTEEN `envelope_*` binaries RAN

| Binary | passed | failed | | Binary | passed | failed |
|---|---|---|---|---|---|---|
| `envelope_advisory` | 10 | 0 | | `envelope_literal_decision` | 43 | 0 |
| `envelope_argv_deletion` | 20 | 0 | | `envelope_pr_cap` | 11 | 0 |
| `envelope_callee_grammar` | 19 | 0 | | `envelope_reparsed_value` | 34 | 0 |
| `envelope_command_position` | 18 | 0 | | `envelope_tracer` | 6 | 0 |
| `envelope_config_resolution` | 30 | 0 | | `envelope_wiring` | 14 | 0 |
| **`envelope_control_carrier`** | **24** | **9** | | `envelope_wrapper_bypass` | 13 | 0 |
| `envelope_credential` | 6 | 0 | | **`envelope_wrapper_class`** | **49** | **1** |
| `envelope_expansion_slots` | 32 | 0 | | `envelope_hook_refusals` | 7 | 0 |

The fifteen audit 9 observed are unchanged except `envelope_wrapper_class`,
41 → 50. **A run reporting fifteen would be a run in which this plan's own
evidence file did not execute; sixteen ran.**

## The EXHAUSTIVE carrier enumeration

Fifteen live carriers, two stated non-carriers, one covered ENV family. Measured
at `406c48e` against the built binary, fresh `GSD_MM_ENVELOPE_ROOT` per row, whole
root walked. **PD** = planner-derived (measured and RECORDED, never asserted).

| # | Carrier | Source | Layer | Verdict (write / read) | Walk | Observed consequence |
|---|---|---|---|---|---|---|
| C-01 | `<env>/<alias>/pr-ledger.ndjson` | `ledger.rs:47`, `:95-97` | 2 SAFE-06 | 0 / 0 | EMPTY | **OBSERVED** — cap resets; four-call drive below |
| C-02 | `<env>/<alias>/hooks/pre-push` | `hooks.rs:55`, `:127-149` | 3 SAFE-05 | 0 | EMPTY | **OBSERVED end to end** — force push MOVED a bare remote's `main` `8ef079d → f2d230f`, `GIT_CONFIG_COUNT` UNTOUCHED |
| C-03 | `<env>/<alias>/hooks/pre-commit` | `hooks.rs:56` | 3 D-22 | 0 | EMPTY | **UNOBSERVED** — same shape; the sweep boundary was not driven |
| C-04 | `<env>/<alias>/hooks/` | `hooks.rs:83-85` | 3 | 0 | EMPTY | **OBSERVED by construction** — both stubs at once |
| C-05 | `<env>/<alias>/gitconfig` (`GIT_CONFIG_GLOBAL` **and** `_SYSTEM`) | `cred.rs:81` | SAFE-05 | **PD** 0 | EMPTY | **OBSERVED vs real git** — control `/ENV_WINS`; a non-`!` `[alias]` body with `-c include.path=<f>` → **`/INCLUDE_WINS`**; `[credential] helper` reads back, **falsifying `SECTION_ENVELOPE`'s FIRST `Guaranteed` clause** |
| C-06 | `<env>/<alias>/askpass` | `cred.rs:49` | SAFE-05 | 0 / 0 | EMPTY | Overwriting was **unmeasured before this round**; permitted here. Substituted-responder harm **UNOBSERVED** |
| C-07 | `<env>/<alias>/gh/` | `cred.rs:83-91` | SAFE-05 | **PD** 0 | EMPTY | **OBSERVED vs `gh` 2.45.0 offline** — empty dir → "not logged into any GitHub hosts"; `hosts.yml` written → known host, "Active account: true". **Valid-token half UNOBSERVED, deliberately** |
| C-08 | `<env>/<alias>/settings.json` | `hooks.rs:1326`, `:1492` | 1+2 D-07 | **PD** 0 | EMPTY | **MECHANICAL half OBSERVED, BEHAVIOURAL half UNMEASURED** — below |
| C-09 | `<env>/<alias>/` | `mod.rs:221-226` | 1,2,3 | 0 | EMPTY | **OBSERVED by construction** — C-01…C-08 in ONE call. *This is the argument for the ROOT as boundary* |
| C-10 | the binary the stubs exec | `hooks.rs:196-204`, `:1454-1460` | 2,3 | 0 | EMPTY | **UNOBSERVED** — not driven. `T-19-01` covers a relocated COPY, not the target's bytes |
| C-11 | `<project>/.git/config` | git's own resolution | the alias | 0 (×8) | EMPTY | **OBSERVED vs real git** — `T-19-111`, table below |
| C-12 | `<project>/.claude/settings.json` | `policy.rs:2063-2064` | 1 | **PD** 0 | EMPTY | Denies are **TOOL patterns**; the Bash route is denied by NEITHER layer. Practical reachability **UNMEASURED** |
| C-13 | `<project>/.git/info/exclude` D-23 block | `hooks.rs:68-69`, `:558-561` | the sweep | 0 | EMPTY | **UNOBSERVED** — removability is a filesystem fact; the sweep consequence was not driven |
| C-14 | `<project>/.planning/meta-manager/runs/<run_id>/` | D-24/D-25 | park **EVIDENCE** | 0 | EMPTY | **REPUDIATION, never ELEVATION** — `ParkOutcome`'s doc: *"describes the evidence, never the decision"* |
| C-15 | `~/.config/gsd-meta-manager/config.json` | `config.rs:330-335`, read per tool call at `hooks.rs:1239-1258` from `:837-840` | 2 SAFE-01+SAFE-06 | **PD** 0 | EMPTY | **OBSERVED end to end** — the PR cap disarmed by raising a number, **no ledger write, no envelope path named** |
| N-01 | `.git/hooks/` | — | — | — | — | **NOT a carrier BY CONSTRUCTION** — D-09 declines it; that refusal is the point of the env-injected delivery |
| N-02 | `--disallowedTools` | `claude.rs:286-289` | 1 | — | — | **NOT reachable** — argv in the PARENT's spawn |
| E-01 | the ENV handles | `policy.rs:3595-3685` | all | — | — | **ALREADY MODELLED** by `ENVELOPE_ENV_KEYS`. **`GSD_MM_ENVELOPE_ROOT` measured INERT** — the ledger line landed under the root the GUARD was given, **and the walk is the proof** |

## The `T-19-112` four-call cap reset — ONE persistent root

| Step | Ledger before | Exit | Ledger after | Reason |
|---|---|---|---|---|
| call 1 `gh pr create --title x` | MISSING | **0** | 1 | (permit) |
| call 2 | 1 | **2** | 2 | `pr_cap_exceeded` |
| call 3 | 2 | **2** | 3 | `pr_cap_exceeded` |
| guard verdict on `rm -f <ledger>` | 3 | **0** | 3 | (permit) |
| deletion PERFORMED | 3 | — | MISSING | — |
| call 4 | MISSING | **0** | 1 | (permit) |

The ledger appends **before** the verdict (D-20), which is why a refused attempt
still adds a line. **The `: >` spelling, same discipline:** 0 (1 line), 2
`pr_cap_exceeded` (2 lines), guard verdict on `: > <ledger>` **0**, truncation
performed, call 3 **0** again.

## The rebuilt bare-remote table — a CONTROL beside every leg

| Leg | Command | Completed? | remote `main` before → after |
|---|---|---|---|
| **A CONTROL** | plain `git push --force origin HEAD:refs/heads/main` | **NO** | `4cc4cdc` → `4cc4cdc` **UNMOVED** |
| **B CARRIER** | `printf`-written `.git/config` alias `fp = -c include.path=<evil> push --force …`, `git fp` | **YES** | `4cc4cdc` → **`ed58fb9`** **MOVED** |
| **C CONTROL** | the SAME alias, SAME way, **NO carrier**, `git np` | **NO** | `ed58fb9` → `ed58fb9` **UNMOVED** |
| **B repeat** | `git fp` on a new commit | **YES** | `ed58fb9` → **`8ef079d`** **MOVED** |
| **D `T-19-113`** | `cp /bin/true <hooks>/pre-push`, plain force push, `GIT_CONFIG_COUNT` UNTOUCHED | **YES** | `8ef079d` → **`f2d230f`** **MOVED** |
| **D CONTROL** | stub RESTORED, plain force push | **NO** | `f2d230f` → `f2d230f` **UNMOVED** |

**Leg C is the sharpest control**: it isolates the CARRIER from both the ALIAS and
the FILE WRITE. **Leg D's control on both sides** makes it a fact about the STUB
rather than about a drifting fixture. **Every leg reproduced. None failed to
reproduce.**

## Every measured row, by section

**Section 1 — ENVELOPE-ROOT OPERAND rows** (measured exit 0, walk EMPTY; asserted
at the DERIVED post-fix verdict exit 2 `envelope_assertion_failed`; **RED**):
`rm -f <ENV>/alpha/pr-ledger.ndjson`, `truncate -s 0 …`, `cp /dev/null …`,
`shred -u …`, `rm -rf <ENV>/alpha`, `cp /bin/true <ENV>/alpha/hooks/pre-push`,
`cp /bin/true …/pre-commit`, `rm -rf <ENV>/alpha/hooks`,
`cp /dev/null <ENV>/alpha/askpass`, and the literal-`..` twin
`rm -f <ENV>/alpha/hooks/../pr-ledger.ndjson`.

**Section 2 — the FOUR fail-open directions** (pinned exit 0 **before and
after**): (i) `: > <ledger>`, `printf 'exit 0' > <hooks>/pre-push`,
`echo evil > <ENV>/alpha/askpass`, `cat /tmp/evil >> <ENV>/alpha/gitconfig`;
(ii) `D=$(git config --get core.hooksPath); rm -f $D/../pr-ledger.ndjson`,
`H=$(…); cp /bin/true $H/pre-push`; (iii) `rm -f /tmp/l`, `: > /tmp/l`;
(iv) `rm -f pr-ledger.ndjson`, `cp /bin/true pre-push`.
**Their two MEASURED partial mitigations, asserted exit 2 (RED now):**
`ln -s <ENV>/alpha/pr-ledger.ndjson /tmp/l` and `cd <ENV>/alpha`; and the
composite `cd <ENV>/alpha && rm -f pr-ledger.ndjson` **refused by SEGMENT 1**,
with segment 2 still unresolvable — recorded so no reader concludes (iv) closed.

**Section 3 — the NEAR-MISS controls, this round's `--signed no`** (exit 0 before
and after): `rm -f /tmp/pr-ledger.ndjson` and
`cat <ENV-one-character-changed>/alpha/pr-ledger.ndjson`.

**Section 4 — the ORDINARY-OPERAND half** (exit 0 before and after): `rm -f
/tmp/x`, `rm -rf /tmp/scratch`, `cp /bin/true /tmp/t`, `truncate -s 0 /tmp/f`,
`cat /tmp/x`, `ls`, `cargo test`, `rg 'pr-ledger.ndjson' src/`, and
**`git config --get core.hooksPath` pinned UNCHANGED**.

**Section 5 — the DISCLOSED OVER-REFUSAL family** (`cat <ledger>`,
`wc -l <ledger>`, `ls <ENV>/alpha/hooks`, all → exit 2 after). Refusing the READS
is a **decision**: the guard cannot tell a read from a write without a
program-name grammar, which
`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
forbids. Asserted mechanically that a carrier row is drawn under **`shred`**,
absent from both production halves.

**Section 6 — the REPO-SIDE rows, RECORDED and NEVER asserted** — all eight
`.git/config` write spellings plus `git fp`, both `.claude/settings.json`
spellings, `.git/info/exclude`, the run journal, the binary, and
`GSD_MM_ENVELOPE_ROOT=/tmp/fresh gh pr create --title x`. **Verified mechanically**
by `no_repo_side_row_is_asserted_and_this_file_says_so_mechanically`, which reads
this file's own source and fails if an `assert!` appears in that block.

**Section 7 — the FIVE PLANNER-DERIVED rows, RECORDED** — C-05, C-07, C-08, C-12,
C-15. **Not one is asserted.**

## `C-15` — a DISTINCT ROUTE to SAFE-06, not a spelling of `T-19-112`

**Mechanical**: `guard` → `Config::default_path()` → `guard_in` (`hooks.rs:837-840`)
→ `resolve_policy` (`:1239-1258`) **on every Bash tool call** →
`EnvelopePolicy::resolve` (`policy.rs:2116-2139`), where `pr_cap_per_24h` and
`pr_cap_per_run` go through `unwrap_or(DEFAULT_…)` with **NO CLAMP**, `namespace`
alone degrades through `validate_namespace` with a warn, and `credential` clones
through.

**Behavioural**, one persistent root with the caps at `9999`:

| Call | Before | Exit | After |
|---|---|---|---|
| 1 | MISSING | **0** | 1 |
| 2 | 1 | **0** | 2 |
| 3 | 2 | **0** | 3 |
| 4 | 3 | **0** | 4 |

**CONTROL at the default 3/1 caps:** call 1 exit 0; calls 2, 3, 4 exit **2**
`pr_cap_exceeded`. **MID-RUN REPLACEMENT:** two calls at the defaults (0, then 2),
the config replaced, calls 3 and 4 **exit 0** — the per-call re-read observes it
**on the very next call**.

**THE FINDING: four permits, four ledger lines, NO DELETION, NO ENVELOPE PATH
NAMED.** Rule (a) cannot reach a file outside the envelope root, so **C-15 gets
control (e): none**. **`pr_cap_*` is deliberately NOT clamped** — that is a
product decision about what a user may configure, and deciding it inside a
security round would decide it silently.

**`Config::default_path()`'s relative `.config` fallback — MEASURED and NOT
REPRODUCED.** With `HOME` and `XDG_CONFIG_HOME` **both unset**, `dirs` resolved the
home directory from the **passwd database** and the guard opened
`/home/<user>/.config/gsd-meta-manager/config.json` (`openat` trace). The relative
branch is **UNREACHED** in this environment. `HOME` alone DOES steer the path and
raised caps supplied there DO take.

## The design answer — option (d), and the direction each choice FAILS

**(c) — move a carrier out of reach — is DEAD for every one of them**, and saying
that once is worth more than fifteen denials: every envelope carrier must be
readable, writable or executable by the same uid the driven process runs as. A
different uid, a container, or a remote-side control are the only real (c)s; the
first two are outside this phase and the third is what `SECTION_ENVELOPE`'s
conclusion already says.

| Carrier group | Control | Direction it FAILS |
|---|---|---|
| C-01 … C-09 | **(a)** — refuse a segment whose absolute literal word operand resolves LEXICALLY under the envelope root, using `root` and `alias` the guard already holds | **FAILS OPEN in FOUR directions; its silence is a permit in every one** |
| C-01, additionally | **(b)** tamper-evidence — **costed and DEFERRED, no schedule** | n/a. It would create the second carrier `T-19-35`/`AR-19-05`/`hooks.rs:1421-1428` all say SAFE-06 lacks, and break `hooks.rs:789`'s single-pass rule |
| C-02 … C-04 | **(a)**; **(b) structurally UNAVAILABLE** — *the round's sharpest asymmetry*: the thing that would detect a replaced stub is the binary the replacement removed from the path | (a)'s four directions. **D-09's narrative corrected either way** |
| C-11 … C-15 | **(e)** — NO rule. Attribution correction, rows RECORDED, registered, **unaccepted** | **FAILS OPEN entirely, STATED rather than mitigated** |
| `SECTION_ENVELOPE`'s cap clause | advisory correction, **mandatory either way** | n/a |

**Rule (a)'s four fail-open directions, handed to NO pin, NO schedule and NO
witness:** (i) a REDIRECTION TARGET is not an operand — real by **mechanism**,
because `tokenize` deletes the operator *and its target* (`policy.rs:2264-2279`),
and reading `>` would re-open a model five rounds have pinned shut (`SEPARATORS`,
one commit ever, `84a9b05`); (ii) an EXPANSION-BORNE operand cannot be resolved —
refusing every non-literal operand would deny `rm $TMPDIR/x`; (iii) a SYMLINK is
not followed — `readlink`/`canonicalize` is the I/O and the TOCTOU both
prohibited; (iv) a RELATIVE path is not resolved — the guard has no cwd.
**(iii) and (iv) are NARROWED by a measured partial mitigation, not open in every
spelling.**

## The `T-19-111` ATTRIBUTION CORRECTION — required REGARDLESS of the fix

`T-19-86`'s four registered rows all require a governed program to be handed a
governed COMMAND as data. **A non-`!` alias body sitting in a config VALUE is not
that.** Five sites:

1. `src/envelope/policy.rs:1177-1183` — discloses the residue, then calls it
   *"`T-19-86`'s shape"*. **`19-27`'s.**
2. `src/envelope/cred.rs:287-289` — narrows to *"predating the run"*, dropping the
   written-during-the-run half. **RESTORE it. `19-27`'s.**
3. `19-SECURITY.md:7430` — **corrected BESIDE, never edited.** Done here.
4. `19-25-SUMMARY.md:265` — **corrected BESIDE, never edited.** Done here.
5. `deferred-items.md:1619` — *"CLOSED AS SCOPED"* names the excluded BODY (`!`)
   and not the excluded CARRIER (a file). **Corrected beside it here.**

**Crediting a live, non-shell, measured destructive bypass to a threat the user
has explicitly scoped OUT is how it stops being counted.**

## `SECTION_ENVELOPE`'s `Guaranteed` cap clause CHANGES EITHER WAY

`"Pull-request cap: an append-only ledger this repository does not contain."`
(`advisory.rs:250`) is falsified by the four-call drive above. Rule (a) refuses
`rm -f <ledger>` and **not** `: > <ledger>` (direction (i)), **and `C-15` resets
the cap through a file outside the envelope root with no ledger write at all** —
so the repaired text must **not** present rule (a)'s four directions as the
complete residue for SAFE-06.

**Shipping the rule and leaving the claim unqualified would be `T-19-107`'s defect
committed by the round that inherited it.** The pinned-phrase ordering constraint
is carried forward (`advisory.rs:206-217`): `cannot reach your ambient git
credentials` < `defeatable by an agent that can spawn an unsupervised` (53 chars,
the longest) < `enable server-side branch protection`; wrap AROUND, never THROUGH.
That ordering is asserted, unchanged, in
`the_guaranteed_cap_clause_is_pinned_at_its_current_text_and_19_27_must_change_it`
— **the ONE named, bounded exception this plan grants to the no-edit rule on its
own file.** `19-27` updates that one test function and nothing else there.

## D-09's narrative defect

`cp /bin/true <hooks>/pre-push` is exit 0 and removes layer 3 — SAFE-05's only
carrier and `AR-19-03`'s backstop — **without touching `GIT_CONFIG_COUNT`**.
D-09's ceiling paragraph (`cred.rs:241-244`, `mod.rs:167`, `advisory.rs:254`) names
the route the guard **REFUSES** (`unset GIT_CONFIG_COUNT` → `hook_bypass_blocked`,
re-measured green) **while the route that works is refused by nothing**, so a
reader concludes the guard stands underneath layer 3 and it does not. **The
correction is required even if the control were deferred. `19-27`'s.**

## `AR-19-04`'s reasoning gap — RECORDED, NOT un-accepted

Its mitigation, verbatim: *"the envelope regenerates it at each run start"* —
**which does not cover a write DURING the run**, and the C-05 measurement is the
write. **The same shape a THIRD time**: `cred.rs:287-289`'s *"predating the run"*
and `resolve_policy`'s *"a guard that cannot read configuration confines the run
more, never less"* (`hooks.rs:1233-1238`, true for an ABSENT or UNREADABLE config,
silent about a PRESENT and WRITABLE one). **A reassurance reasoning about the case
that is not the threat.**

**No `AR-` row added, edited or renumbered. `AR-19-04` and `AR-19-05` are recorded
and NOT un-accepted — that is a human decision.**

## The byte floors, RE-MEASURED with MY numbers

| Measurement | Method | This plan's number |
|---|---|---|
| `policy.rs` production half | above the first `#[cfg(test)]` (`policy.rs:7147-7151`) | **262,229 bytes**, 5,162 lines |
| `hooks.rs` production half | the same | **74,490 bytes**, 1,572 lines |
| `policy.rs` stripped | `envelope_wrapper_class.rs:763`'s `production_code` | **72,840** of 443,076 raw |
| `hooks.rs` stripped | the same | **33,460** of 99,909 raw |

Audit 9 recorded 261,387 / 74,358 at `cc65220`; the mandate cited 261,386.
**Neither is repeated.**

**All three stripper protections confirmed:** the proportional floor
`>= 180_000` at `policy.rs:7192` **PASSES** at 262,229; the deep anchor
`fn forbidden_repo_path` present at `policy.rs:5141`; exactly ONE `#[cfg(test)]`
sentinel per file (`policy.rs:5163`, `hooks.rs:1573`).
`POLICY_MIN_PRODUCTION_BYTES = 40_000` and `HOOKS_MIN_PRODUCTION_BYTES = 20_000`
unchanged, both passing with wide margin.

**THE STALE COMMENT, recorded and deliberately not fixed here.** `policy.rs:7172`,
`:7185` and `:7194` cite *228,785 bytes, 78.7%*; the real ratio is **68.6%**. The
FLOOR is correct and must not move; the arithmetic in the comment has drifted.
**This plan may not edit `policy.rs`; scheduled for `19-27`.**

## `glab --host` — carried forward UNFIXED, `--hostname` KEPT

`glab` is confirmed **NOT installed**, so the callee's grammar is unconfirmed and
the under-count is **NOT claimed as a live bypass**. **`--hostname` STAYS in
`FORGE_VALUE_OPTS`**: audit 9 re-measured and overturned audit 8's own suggestion
— `glab --hostname …` leaves ONE ledger line and `glab --host …` leaves ZERO, so
removal is a **regression** in the under-counting direction (`T-19-35`).

## `T-19-17r` — OUTSTANDING, and this plan did NOT accept it

`19-17-SUMMARY.md` calls it "accepted". Audits 5-9 and plans 19-22 … 19-25 all
declined. `grep -cE '^\| AR-19-13 \|'` over `19-SECURITY.md` is **0**, verified
after writing. **This plan adds no Accepted-Risks-Log row, creates no `AR-19-13`,
and does not apply the word "accepted" to `T-19-17r` anywhere.** Six agents have
left the acceptance unmade because it is a human decision; **this plan is the
seventh.**

## Deviations from Plan

### `[Rule 1 - Correctness]` The degenerate-proofing block's converse assertion was refined, because the stated form is FALSE

- **Found during:** Task 2.
- **Plan text:** *"assert the converse — that no control-carrier representative
  satisfies any class on the four existing axes."*
- **Measured:** it does not hold, and it cannot. `DELETION_CLASSES`' class 1
  predicate `draws_a_separate_word_redirection` is a **pure text function** and it
  fires on `: > <ENV>/alpha/pr-ledger.ndjson`, because that command really does
  carry a separate-word redirection; `draws_a_tilde` fires on any entry naming
  `~`. Asserting the plan's form would have asserted something untrue and landed
  permanently red.
- **What was written instead:** the assertion the plan's own must_have calls the
  mechanical proof — **"no existing axis predicate can be satisfied by a command
  reaching no governed program"** — in the form that IS true: every
  control-carrier entry's **carrier-bearing segment** reaches no governed program
  (`resolve_program_with_head` → `Ungoverned`/`NoProgram`), every existing-axis
  representative reaches one, and no existing-axis representative satisfies any of
  the SIX carrier classes (class 7 is deliberately excluded and the reason stated:
  it is the COMPLEMENT of the six, so a `git` command naming no carrier satisfies
  it trivially, which is correct and is the point). **The incidental textual
  overlaps are RECORDED with their reason, not asserted.**
- **Commit:** `d92ea5f`.

### `[Rule 1 - Correctness]` The reachability proof reads the CARRIER-BEARING segment, not every segment

- **Found during:** Task 2, by measurement.
- **Issue:** `D=$(git config --get core.hooksPath); rm -f $D/../pr-ledger.ndjson`
  has a FIRST segment that resolves `Governed { index: 0 }` — the command
  substitution really does run a governed `git config --get`.
- **Why this is a finding worth keeping rather than a nuisance:** that read is
  **PERMITTED** (it is the same permitted read `git config --get core.hooksPath`
  the disclosed over-refusal family rests on), and the segment that ACTS on the
  carrier reaches nothing the envelope governs. **The carrier's location is
  fetched by a permitted governed READ and then acted on by an ungoverned
  command; that composition IS direction (ii).** Recorded in the record and in
  the assertion's own doc.
- **Commit:** `d92ea5f`.

### `[Scope clarification]` `CONTROL_CARRIER_REPO_SIDE` is in the axis but its verdicts are RECORDED, not asserted

- **Found during:** Task 2.
- **Issue:** Task 2 lists `CONTROL_CARRIER_REPO_SIDE` among the six
  verdict-preserving alphabets *in the invariance arm*, which would assert its
  entries PERMITTED — while the plan's own honesty prohibition states that
  *"asserting only that they are PERMITTED is also forbidden, because that would
  pin a live bypass as correct"*, and success criterion 6 requires every repo-side
  row RECORDED.
- **Resolution:** the prohibition binds. The alphabet exists and is covered by the
  class structure, the disjointness fence and the class-count floors (pure data
  assertions); **its guard verdicts are driven and PRINTED by
  `the_repo_side_control_carrier_alphabet_is_recorded_and_never_asserted`, with no
  verdict assertion.** The invariance arm drives the other five and says so on its
  own constant.
- **Commit:** `d92ea5f`.

### `[Editorial]` Section 1's askpass row uses the OPERAND spelling `cp /dev/null …`

The plan's section-1 list reads `echo evil > <ENV>/alpha/askpass` and then says
*"(the operand spelling, not the redirection one)"* — but `echo … >` **is** the
redirection spelling. The operand spelling `cp /dev/null <ENV>/alpha/askpass` is
asserted in section 1, and `echo evil > <ENV>/alpha/askpass` is pinned PERMITTED in
section 2 as direction (i). Both were measured exit 0. **This keeps the row
derivable in the direction the rule actually produces.** Commit `7196de3`.

No other deviations. No auto-fix attempt limit was reached. **No `src/` line was
changed, and nothing was fixed that this plan was not asked to fix.**

## Anything green that was expected RED

**None.** Every audit-9 row reproduced at its recorded verdict, every
planner-derived row reproduced, every bare-remote leg reproduced, and the
four-call cap reset and the C-15 four-call drive both reproduced. **Nothing
failed to reproduce, and nothing came up green where red was expected.**

## Known Stubs

None. This plan adds no production code and no placeholder.

## What remains uncovered

1. **`T-19-86`** — OPEN at `high` by **explicit user scoping decision**. Its four
   registered rows and its persisted-alias arm are still exit 0, re-asserted here
   over the same public functions. **`T-19-111` is moved OUT of it, not into it.**
2. **`T-19-91`** — OPEN at `high`, arms unweakened, no decision-operand rule and
   no denylist extension.
3. **`T-19-111`** — OPEN at `high`. **Attribution correction only; NO RULE.**
4. **`T-19-112`** — OPEN at `high`, corpus RED, narrowed but not closed by `19-27`.
5. **`T-19-113`** — OPEN at `medium`, corpus RED. D-09's narrative defect is
   `19-27`'s.
6. **`T-19-114`** — OPEN at `low`, corpus repaired here, closed only by `19-27`.
7. **`C-15`** — registered, control (e), **no rule planned.**
8. **`T-19-96`**, **`T-19-110`** (medium, audit 9's re-rating carried forward),
   **`T-19-74`** (core rows frozen).
9. **`T-19-84`, `T-19-85`, `T-19-61` … `T-19-73`** — open and unaccepted by
   explicit user decision.
10. **`T-19-17r`** — OUTSTANDING, no `AR-19-13`, not accepted.
11. **the `glab --host` forge cell** — unfixed, unconfirmed callee, not claimed
    live.
12. **`policy.rs`'s stale proportional-floor comment** — documentation drift
    scheduled for `19-27`.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.**
`/gsd-secure-phase 19` is not cleared by this plan and will not be cleared by
`19-27`.

## Self-Check: PASSED

Files:

```text
FOUND: tests/envelope_control_carrier.rs
FOUND: tests/envelope_wrapper_class.rs
FOUND: .planning/phases/19-gitsafe-git-blast-radius-envelope/19-SECURITY.md
FOUND: .planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md
```

Commits:

```text
FOUND: 7196de3   test(19-26): the NINTH evidence file
FOUND: d92ea5f   test(19-26): a FIFTH axis — CONTROL_CARRIER_CLASSES
FOUND: df2243b   docs(19-26): the round-10 record
```

Gate: `passed + failed = 1762` over 45 result lines, 16 `envelope_*` binaries,
10 failures all named above, `1720 + 42 = 1762` checked against `git show`.
`cargo build` 0, `cargo clippy -- -D warnings` 0. Zero `src/` hunks in all three
commits; zero deletions under `tests/`.
