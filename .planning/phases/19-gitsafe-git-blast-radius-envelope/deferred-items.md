# Deferred Items — Phase 19

Out-of-scope discoveries logged during execution. Not fixed, by the scope
boundary rule: they are not caused by the current plan's changes.

## `tests/driver_reattach.rs` is intermittently flaky (found during 19-04)

**Symptom.** Two tests fail non-deterministically:

- `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` —
  `assertion left == right failed: exactly one project has a run to observe`,
  `left: 0`.
- `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`
  — `the run record is on disk: Os { code: 2, kind: NotFound }`.

Both look like the same race: the spawned driver's process is live (the liveness
probe finds its cmdline) before it has written `run.json`, so the reconcile scan
that follows finds nothing on disk.

**Proved pre-existing.** The wave base `5e1170b` was extracted with `git archive`
into a clean directory and `cargo test --test driver_reattach` reproduces the
same two failures there, with plan 19-04's changes absent entirely. Observed
passing 3/3 on some runs and failing 2/3 on others in both trees.

**Why it is not fixed here.** Nothing in 19-04 touches `src/driver/`, and the
envelope is not wired into the driver until plan 19-06. Fixing a spawn/write race
in the reattachment path is a change to Phase 17's code with its own verification
needs, not a deviation of this plan.

**Suggested owner.** A follow-up quick task, or plan 19-06 if it finds the same
race while wiring the envelope into the driver's spawn path.

### Ownership settled by the orchestrator (wave 4 post-merge gate)

19-04's executor proved the flake reproduces at the wave base `5e1170b` (waves 1-3 merged).
The orchestrator extended that check to `0a84023` — the commit **immediately before Phase 19
began** — via `git archive` into a clean tree:

```
run1 FAILED. 1 passed; 2 failed  (0.53s)
run2 FAILED. 1 passed; 2 failed  (0.52s)
run3 FAILED. 1 passed; 2 failed  (0.54s)
run4 FAILED. 1 passed; 2 failed  (0.52s)
```

4/4 red with **no Phase 19 code present at all**. At Phase 19 HEAD the same file is
intermittently green (1 of 3 full-suite runs clean, failure set never larger than these two
tests). The defect therefore predates Phase 19 entirely and belongs to the Phase 17/18
reattachment path — no Phase 19 plan introduced or worsened it.

**Signature of a failing run:** the file completes in ~0.5s instead of ~6.1s. The driver
process is reported live by the cmdline probe before it has written `run.json`, so the
reconcile scan that follows finds nothing on disk. The fix is to wait on the artifact, not
the process — `live_within` is the wrong synchronisation primitive for these two assertions.

**Consequence for this phase:** the post-merge gate cannot be relied on for a clean binary
pass/fail while this flake is live. Waves are accepted on the bounded-failure-set rule
instead: a wave passes if the only failures are these two tests and the total passing count
advances as expected.

## `tests/envelope_tracer.rs` — ETXTBSY when a just-copied stub is exec'd (found during 19-07)

**Symptom.** `a_relocated_copy_of_the_stub_refuses_instead_of_acting` fails intermittently
under a full parallel `cargo test` with:

```
the generated stub is executable: Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }
```

Observed once in ~6 full-suite runs during 19-07; green in every run of the file alone.

**Cause (likely).** The fixture copies the generated stub and executes the copy. ETXTBSY is
the kernel refusing to `exec` a file that still has an open writer descriptor somewhere — the
classic write-then-exec race. Nothing about it is about the policy under test.

**Not caused by 19-07.** `git diff c107e04 -- tests/envelope_tracer.rs` is empty: this plan
did not touch the file, the stub generator, or `assert_provenance`. The file belongs to
19-01/19-03.

**Fix direction.** Drop or `sync_all` the copy's handle before exec, or retry a bounded number
of times on `ExecutableFileBusy`. Do **not** serialise the suite — that hides the race.

## `tests/driver_lock.rs` — one-off lock-acquisition timeout (found during 19-VERIFICATION)

**Symptom.** `the_lock_is_released_when_the_holding_process_dies` failed once under full-suite
parallel load with "the child driver never took the lock within 30s". Passed cleanly in
isolation twice.

**Orchestrator follow-up.** Three further full-suite runs under load did NOT reproduce it
(two fully green at 993 passing; the third showed only the two known `driver_reattach`
flakes). It is therefore rarer than the documented `driver_reattach` pair.

**Why it is recorded anyway.** Two independent observers reached the same hypothesis from
different directions. 19-07's executor flagged, unprompted, that moving envelope
establishment ahead of the lock "plausibly widens that race without changing its cause"; the
verifier then independently proposed that `establish_envelope()` — hook stub writes,
settings-file generation plus round-trip readback, `.git/info/exclude` write, cred config
generation, all inside one `spawn_blocking` **before** `lock::acquire` — is new I/O on the
run-startup critical path that did not exist before Phase 19.

**Assessment.** A test-timing observation, not a mechanism defect: no SAFE-0x success
criterion depends on lock-acquisition latency. But it is the one item in this phase where
Phase 19's own changes are the plausible cause, unlike the `driver_reattach` pair which is
proved pre-existing.

**Suggested owner.** Whoever fixes the `driver_reattach` race — the fix direction is the
same (wait on the artifact/state, not a fixed budget), and Phase 20 builds on this envelope
and will add further startup work.

---

## Re-observed during 19-11 (T-19-60 gap closure), 2026-08-29

The `driver_reattach` pair reproduced **deterministically** on this machine during 19-11's
full-suite gate — 3/3 isolated runs failed, not intermittently:

- `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step`
  (`tests/driver_reattach.rs:450` — `exactly one project has a run to observe`, left 0 right 1)
- `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`
  (`tests/driver_reattach.rs:542` — `the run record is on disk: NotFound`)

Both spawn the real driver binary and then read `run.json`; `live_within(pid, …)` succeeds, so
the driver comes up and is visible in `/proc` — the run **record** is what is missing. The
shape is the same startup race already recorded above, now presenting as a consistent loss
rather than a flake on this host.

**Proved not caused by 19-11.** `src/envelope/hooks.rs` was reverted to its pre-Task-3 state
(leaving `policy::resolve_program` present but with no production caller, so the guard behaved
exactly as it did at the plan's base commit `5e574c4`) and the same two tests failed
identically. 19-11 touches only the `PreToolUse` guard's program resolution, which
`driver_reattach` never exercises.

**Not fixed here** — out of scope for a T-19-60 gap-closure plan, and the plan forbids taking
on neighbouring findings. Recorded so the count is not mistaken for a 19-11 regression:
19-11's gate is 1245 passed / 2 failed / 13 ignored, and the 2 are this pair.

---

## `T-19-86` — a governed program's own operand naming a governed command (registered by 19-13)

**Symptom.** A GOVERNED program handed a governed command as data runs it itself, and the
guard permits it. Measured against `a41e431`, one fresh tempdir per row, and re-measured
unchanged after 19-13's command-position rule:

```
exit=0  git submodule foreach git push --force origin main
exit=0  git rebase -x "git push --force origin main" HEAD~3
exit=0  git bisect run sh -c "git push --force origin main"
exit=0  git -c alias.p='!git push --force origin main' p
```

**Mechanism.** These resolve at the **head** — correctly, because the head *is* the command
position — and are then permitted by `classify_git`'s denylist **default arm**, whose verbs
here are `submodule`, `rebase`, `bisect` and (after `scan_leading` consumes `-c alias.p=…`)
`p`. The head shortcut is not the defect and must not be removed: without it, every commit
message and PR title quoting a git command would be refused (`T-19-75` widened from `rg` to
every commit, AR-19-11).

**Why 19-13 did not close it.** Found while *planning* 19-13, which is the round that closed
`T-19-60`'s wrapper-operand sub-class. A plan cannot both discover a threat and be the plan
that measured it fail first — that is the corpus-vacuity failure this phase has now recorded
twice. 19-13 therefore **narrowed its closure claim** and registered this instead. Every
statement 19-13 makes about `T-19-60` carries the qualifier *for the wrapper-operand
sub-class*.

**Pinned at its current PERMITTED verdict** in
`tests/envelope_command_position.rs::the_t_19_86_residual_is_permitted_today_and_this_plan_leaves_it_permitted`,
and disclosed in `resolve_program`'s own doc. A future change that moves the boundary must
delete those rows deliberately rather than discover them failing.

**Consequence.** `/gsd-secure-phase 19` is **not cleared by 19-13 alone.**

**Fix direction (not prescriptive).** Extending `classify_git`'s denylist to `submodule`,
`rebase`, `bisect` and `alias.*` config keys closes the four measured spellings and is a
denylist, so it inherits every gap a denylist has. A structural answer would classify a
governed verb's own command-valued operands the way `resolve_program` already classifies a
`-c` payload — which needs a per-verb notion of which operand is a command line, and that is
a design question rather than a one-function change.

**Suggested owner.** A round-4 gap-closure plan, before `/gsd-secure-phase 19` is re-run.

## `T-19-87` — `${VAR}` fragments a command so the guard never sees its shape (found during 19-13 execution) — **CLOSED by 19-14 + 19-15**

**Symptom.** An envelope key removed through a brace expansion is permitted:

```
exit=0  K=GIT_SSH; env -u ${K}_COMMAND git fetch origin
```

**Mechanism, and why it is not the resolver.** `split_words` treats `{` and `}` as
**separators** (`policy.rs`'s `SEPARATORS`, and the tokenizer arm beside `;` and `|`), so
`${K}_COMMAND` never survives as one expansion-carrying token. The line fragments into three
segments — `env -u $` | `K` | `_COMMAND git fetch origin` — and the guard judges each on its
own. The third resolves `git` behind an ungoverned head with exactly one candidate and no
expansion between them, so it is `git fetch`, which is allowed. 19-13's expansion-prefix rule
cannot see the shape, because by the time resolution runs the shape is gone.

**How it was found.** 19-13's plan and its plan-check both asserted this line was closed by
the expansion-prefix rule, reasoning that `${K}_COMMAND` carries `Token.expansion`. It does
not. The claim was written from reading the tokenizer's `$` handling without running it; the
executor caught it when the row stayed green after the fix. The equivalent brace-free
spelling — `env -u $K git fetch origin` — **is** closed, and is what
`the_indirect_spelling_no_literal_match_can_find_is_refused_too` now pins.

**~~Bounded on one side, and that bound is asserted.~~ THAT BOUND WAS MEASURED FALSE.** The
entry originally claimed the fragmentation does not hide a refused git command, because the
segment carrying the command still resolves it. `19-SECURITY.md`'s third audit measured that
false one word to the right: the fragmentation hides `env -u GIT_SSH_COMMAND`, and that harm
lands whether or not the git command behind it is itself refused. The test asserting the
bound was **deleted** by plan 19-15 rather than re-worded, with a tombstone in
`tests/envelope_command_position.rs` naming where its two rows went.

### STATUS: **CLOSED** across plans 19-14 and 19-15

**Which rule closed each half:**

* **Rule A — plan 19-14, the DECISION REGION** (`policy::expansion_in_decision_region`, called
  once in `hooks::classify_segments`'s `Governed` arm). Six of the eight measured rows: every
  spelling where the flush lands in a word the classifier's own matched arm READS — the git
  verb, `config`'s key operand, the forge's first two subcommand words. `git ${X}push --force
  origin main`, `git $(true)push …`, `git ${X}stash`, `git ${X}update-ref …`,
  `git ${X}config core.hooksPath /tmp/x`, `gh ${X}pr create --title x`.
* **Rule B — plan 19-15, the COMMAND-POSITION rule**
  (`policy::resolve_program_with_head` over `policy::split_segments_with_heads`). The
  remaining two rows, where the flush lands in a wrapper prefix SEVERED from the governed
  program and no verb-slot rule can see the shape: `C=GIT_CONFIG; env -u ${C}_COUNT git fetch
  origin` and `K=GIT_SSH; env -u ${K}_COMMAND git fetch origin`. Plus every further
  split-point spelling found since — no literal fragment, one- and two-character tails, and
  the `$(printf …)` substitution form.

**Neither half changed what a brace does.** `SEPARATORS` is unchanged, `split_segments` keeps
its signature and behaviour, and the flush flag is computed for `( ) { }` only, on a
word-in-progress condition. `{ git status; }` and `( git status )` reach the same verdicts as
`git status`, asserted as rows. Rule B's one disclosed cost —
`ROOT=$(git rev-parse --show-toplevel) git status` refused while both halves alone are
permitted — is pinned as a pair.

Pinned in `tests/envelope_expansion_slots.rs` (the re-homed rows, the severed-prefix
spellings, the cost pair, the grouping rows and the opener-exclusion permits) and in
`tests/envelope_wrapper_class.rs` (the `SEVERED_PREFIXES` generative alphabet).

## `T-19-91` — a git classifier's own decision operand, assembled by expansion (registered by 19-14)

**Symptom.** A git verb's own decision operand, assembled by shell expansion, is handed to a
classifier that cannot read it and falls to an `Allow` arm. Measured against `c595141`, one
fresh tempdir per row, driven in-process through `hooks::guard_in`:

```
exit=0  git reflog $S            <- PERMITTED
exit=0  git reflog show $S       <- PERMITTED
exit=0  git symbolic-ref $S      <- PERMITTED
```

and the two spellings of the same question that already fail CLOSED, measured and pinned
beside them so the residual's width is honest rather than assumed:

```
exit=2  git symbolic-ref HEAD $R    [force_push_blocked]      <- two operands is a write whatever they say
exit=2  git push origin $REF        [push_outside_namespace]  <- an unreadable refspec carries no namespace prefix
exit=2  git push $REF               [push_outside_namespace]  <- refused, but repository-dependent (see below)
```

**Mechanism.** `classify_reflog` looks for the first non-flag token and matches it against
`delete`, `expire` and `drop`; `classify_symbolic_ref` counts operands and looks for
`-d`/`--delete`. Handed `$S`, neither test matches, and both answer `Allow`. So
`S=delete; git reflog $S` destroys the reflog and `S=-d; git symbolic-ref $S` deletes the
ref. This is structurally identical to the `config` cell that 19-14 DID close —
`classify_config` reaching `is_hooks_path_key` on an operand it cannot read.

**Why 19-14 closed `config` and not these: round discipline and provenance.**
`git ${X}config core.hooksPath /tmp/x` is a row in **audit 3's own measured bypass list**,
so closing that cell is part of making 19-14's decision-region principle coherent over rows
the audit had already established. These three were found while *checking* plan 19-14, and a
plan cannot both discover a threat and be the plan that measured it fail first — the
corpus-vacuity failure this phase has now recorded three times.

**This is NOT a second-carrier argument, and the distinction is the whole point of writing
it down.** `reflog` and `symbolic-ref` are LISTED verbs that reach their own classifiers and
fall to an `Allow` arm on an unreadable operand. Neither has a `pre-push` and neither has a
`pre-commit` behind it — git runs no hook for either — and `classify_reflog`'s own refusal
text records that the reflog is *the recovery path for every other destructive git
operation*. **Only `git push` has a hook behind it**, and its refspec operand already fails
closed. A later reader who took the asymmetry for a blast-radius judgement would read a
narrowed threat as a covered one, which is exactly what this paragraph exists to prevent.

**On `git push $REF`.** It is refused today, but by the no-refspec path: with a single
operand `classify_push` falls to `ctx.resolved_push_dests`, which is the ONE shape
`policy::push_needs_resolved_dests` answers `true` for and which makes the guard shell out to
`git` in the caller's working directory. Its verdict therefore depends on the repository the
test happens to run in (`T-19-80`), so it is recorded here rather than pinned as a live row;
the repository-free half of the same question is the `git push origin $REF` row.

**Pinned at its measured verdicts** in
`tests/envelope_expansion_slots.rs::the_t_19_91_residual_is_measured_and_pinned_rather_than_closed`,
and disclosed in `resolve_program`'s own doc as its fourth residual bullet beside `T-19-74`,
`T-19-75` and `T-19-86`. A future change that closes it must delete those rows deliberately
rather than discover them failing.

**Fix direction (not prescriptive).** Extending `expansion_in_decision_region`'s git arm to
report each classifier's own decision-operand index — the way it already reports
`classify_config`'s via `config_key_operand_index` — closes all three with the same
primitive-per-scan discipline and no new denylist entry. It needs one index primitive per
classifier arm, which is mechanical but is a per-verb notion of which operand decides.

**Consequence.** `/gsd-secure-phase 19` is **not cleared by 19-14 or by 19-15.** `T-19-86`
remains open at `high` by user scoping decision, and this is a second `high` alongside it.

**Suggested owner.** The same round-5 gap-closure plan as `T-19-86`, before
`/gsd-secure-phase 19` is re-run.


---

## Audit 4 (2026-08-29, at `b72237e`) — a correction to `T-19-91` above, and four new items

Everything below this line was measured by **audit 4** — `/gsd-secure-phase 19`
re-run after plans 19-14 and 19-15 — against the built binary, one fresh
`GSD_MM_ENVELOPE_ROOT` per row, envelope directory walked afterwards so a missing
ledger line is observed rather than inferred, and every claimed bypass re-run
under `bash` against argv-printing `git`/`gh`/`glab` shims. The full write-up is
in `19-SECURITY.md`; this file carries the actionable residue.

### Correction — `T-19-91`'s `git push $REF` row is wrong as written

The row above records `exit=2  git push $REF  [push_outside_namespace]  <- refused,
but repository-dependent`, and `resolve_program`'s fourth residual bullet argues
from it that "its refspec operand already fails CLOSED".

**Measured, the repository-dependence goes both ways.** Outside any repository
and inside this one, `git push $REF` is exit 2. **Inside a repository whose
current branch is inside the envelope's namespace** — a branch under
`refs/heads/gsd-auto/alpha/`, which is the state a driven run is designed to be
in — it is **exit 0**. Measured in a purpose-built fixture repo on
`gsd-auto/alpha/work` with an upstream configured.

Not pinning it was the RIGHT call (`T-19-80`); recording it as "already fails
closed" was not. `classify_push`'s refspec operand is a **third** arm of
`T-19-91`'s shape that answers `Allow` on an operand it cannot read, alongside
`classify_reflog` and `classify_symbolic_ref`. The threat stays `T-19-91` at
`high` — same component, same mechanism, measured more completely — and the
second-carrier asymmetry the registration draws is still correct, because `push`
does have `pre-push` behind it while `reflog` and `symbolic-ref` have nothing.

`src/envelope/policy.rs:1735-1742` is narrower and correct (it claims only that
`git push origin $REF` is refused, which audit 4 re-measured at exit 2). The
`19-SECURITY.md` row, `19-14-SUMMARY.md` and `19-15-SUMMARY.md` all state the
stronger and false claim. Correct disposition: **cwd-dependent, and permitted in
the in-namespace configuration.**

### `T-19-92` (high, OPEN — BLOCKING) — a `{` that opens a brace EXPANSION is read as a brace GROUP

**Symptom.** Measured at `b72237e`, fresh root per row, walk after:

```
exit=0  git {-c,core.hooksPath=/dev/null,push,--force,origin,main}   <- ALL THREE LAYERS
exit=0  git {push,--force} origin main
exit=0  git {update-ref,-d,refs/heads/main}                          <- no second carrier
exit=0  git {config,core.hooksPath,/tmp/x}
exit=0  gh {pr,create} --title x                    (walk: no ledger line)
exit=0  gh {api,repos/o/r/pulls} -f title=x         (walk: no ledger line)
exit=0  glab {mr,create} --title x
exit=0  bash -lc "git {push,--force} origin main"   <- through the nested-payload path
```

Confirmed under `bash` against the shims — the first prints
`ARGV: -c core.hooksPath=/dev/null push --force origin main`.

**Mechanism.** `tokenize` marks a separator as a word-splitting flush only when a
word was in progress **immediately before** the character, and
`Segment::head_is_command_position` acts on **closers** only. Bash's grammar has a
second condition the flag does not encode: `{` is the reserved word opening a
group only at a **command position**; inside a simple command it opens a brace
**expansion** whose contents bash splices back into that same command. So
`git {push,--force} origin main` is split into `git`, `push,--force` and
`origin main`; the first segment resolves `Governed` with an EMPTY argv,
`classify_git` returns `Allow` for a bare `git`, and the guard permits.

**Why high.** Three legs, each measured rather than argued. Layer 1 — the line
begins `git {-c,…`, no `Bash(git push:*)` prefix match. Layer 2 — exit 0 above.
Layer 3 — re-measured on this machine,
`GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=/ENV_WINS git {-c,core.hooksPath=/CLI_WINS,config,--get,core.hooksPath}`
prints `/CLI_WINS`, so command-line `-c` outranks the envelope's env-injected
`core.hooksPath` and no `pre-push` hook runs. Credentials survive, so the push
authenticates. Two further rows have **no second carrier at all**:
`git update-ref` and the SAFE-06 PR cap, which is bypassed UNCOUNTED rather than
exceeded.

**Same component as `T-19-87`** (`tokenize` treating `{ } ( )` as `SEPARATORS`) —
the direction Rule B did not take. Rule B's doc argues the opener is safe because
"after `{` is the parameter expansion's variable name"; true for `${`, false for
a ` {` opening a brace expansion.

**Fix direction (not prescriptive).** Discriminate group from expansion by
whether the current segment is empty when `{` arrives — bash's own rule — and
treat a splice back into an enclosing simple command as making that whole command
unresolvable rather than as N independent segments. **Do NOT delete the characters
from `SEPARATORS`**: `{ cmd; }` and `( cmd )` are real grouping syntax and losing
them re-opens the `echo hi && git push --force` class one level in. The paired
cost must be pinned from both sides, as Rule B's was: `{ git status; }` and
`( git status )` must keep reaching their ungrouped verdicts, and their
force-push spellings must keep reaching `force_push_blocked` under the base's own
identifier.

### `T-19-93` (high, OPEN — BLOCKING) — `gh`'s own `{owner}`/`{repo}` placeholders are uncounted by the PR cap

**Symptom.**

```
exit=0  gh api repos/{owner}/{repo}/pulls -f title=x       (walk: NO ledger line)
exit=0  gh api "repos/{owner}/{repo}/pulls" -f title=x     (walk: ledger line present)
```

**Mechanism.** `gh api` documents `{owner}`, `{repo}` and `{branch}` as
placeholders substituted from the repository of the current directory — it is the
idiomatic spelling in `gh`'s own manual. Because `{` and `}` are `SEPARATORS`, the
unquoted endpoint is fragmented before either forge scan sees it,
`pr_command_label` matches no arm, and the creation is never counted. Bash
confirms the braces are literal (no comma, so no brace expansion);
`gh` is what substitutes them.

**Why high.** This is not an evasion — it is the documented spelling, so the cap
under-counts by accident for an honest agent and can be under-counted
deliberately by a hostile one. The cap has **no second carrier**
(`hooks.rs:1263-1270`, `T-19-35`). Rated on the standard this phase already used
twice: SAFE-06 bypassed rather than exceeded.

**Fix direction.** Either teach the forge scans the placeholders so
`repos/{owner}/{repo}/pulls` reaches `endpoint_is_pulls` intact, or stop the
tokenizer fragmenting a brace pair containing no comma. **The bar is "count", not
"refuse"** — an uncounted pull request is the failure mode — so the fix must be
verified by WALKING the envelope root and finding a ledger line, not by an exit
code.

### `T-19-94` (medium, open — below `high`) — pathname (glob) expansion in a decision slot

**Symptom**, with a matching file in the working directory:

```
exit=0  git pus? --force origin main
exit=0  git ?ush --force origin main
exit=0  git stas?                                  <- no second carrier
exit=0  gh p? create --title x
exit=0  touch push && git pus? --force origin main <- precondition met in ONE tool call
```

Confirmed under the shims: with `push` present, `git pus? --force origin main`
prints `ARGV: push --force origin main`.

**Mechanism.** Rule A recognises shell assembly by ONE bit, and `tokenize` sets
`Token.expansion` only for an unquoted `$` or backtick. Pathname expansion sets
nothing. `touch push` is `Ungoverned` and the two segments are judged
independently, so the precondition is satisfiable inside a single tool call.

**Medium rather than high on the precondition alone** — it needs a matching
filename where `T-19-92` needs nothing. Same root cause; a fix for either should
be written so it covers both.

**Fix direction.** Treat an unquoted glob metacharacter in a decision word exactly
as `Token.expansion` is treated. The cost is bounded and should be measured the
way Rule A's was: an operand carrying a glob (`git add src/*.rs`,
`rg "x" src/*`) must keep working, because only the decision region is in scope.

### `T-19-95` (medium, open — below `high`) — the alphabets cannot draw a `{a,b}` or a glob

`EXPANSION_METACHARACTERS` (`tests/envelope_wrapper_class.rs:1717`) is exactly
`['$', '`', '{', '(']`. Verified by reading every entry of `ASSIGNMENT_PREFIXES`,
`WRAPPERS`, `REFUSED_BASES`, `DECOY_OPERANDS`, `EXPANSION_WRAPPERS`,
`SHELL_LAYERS` and `SEVERED_PREFIXES`, and by grepping the file: **not one entry
anywhere contains a comma inside braces, and not one contains a `*`, a `?` or a
`[`.** Every entry satisfying the floor does so through an expansion MARKER.

The corpus is therefore structurally incapable of generating, and so of failing
on, `T-19-92` and `T-19-94`. This is `T-19-76`'s failure mode for the **fourth
consecutive round** (after `T-19-83` and `T-19-89`), and it is the reason the
closure order below puts the corpus first.

### Closure order for round 5 — the corpus FIRST

1. **`T-19-95`** — extend `EXPANSION_METACHARACTERS` beyond the four expansion
   markers to the characters that make a word unreadable (at minimum a comma
   inside braces, and `*`, `?`, `[`), add entries carrying them to every alphabet,
   and assert the corpus generates them with the `MIN_*`-floor pattern already in
   the file. **Before certifying anything below.** Three rounds running, the gap
   has been the cell one slot over from what the corpus could draw, and each time
   it was the NEXT audit that found it rather than the round's own evidence.
2. **`T-19-92`** — the brace-expansion / brace-group discrimination.
3. **`T-19-93`** — the `gh` placeholder endpoint, verified by a ledger line.
4. **`T-19-94`** — glob metacharacters in a decision word.

Then, separately: correct the `T-19-91` `git push $REF` row wherever it appears,
and make `resolve_program_with_head`'s post-filter **exhaustive**. That filter is
currently `Governed | NestedPayload => Refuse, other => other` — a wildcard, so a
future `ProgramResolution` variant meaning "reaches a governed program" would
compile, pass a severed head silently, and turn no test red. Rule B is the newest
control in the file and this is the one place it can be defeated by an addition
rather than a deletion. Same discipline `T-19-45` already establishes for
`PermissionMode`: one arm per variant, so a new variant is an E0004 rather than a
quiet permit.

**Consequence.** `/gsd-secure-phase 19` is **not cleared**. Four threats are open
at `high`: `T-19-86`, `T-19-91`, `T-19-92`, `T-19-93`.

**Suggested owner.** The round-5 gap-closure plan, together with `T-19-86` and
`T-19-91`.

---

## `T-19-96` — a glob in a PUSH FLAG (registered by 19-16)

**Symptom.** Measured at `e842fa3`, one fresh envelope root, walk after:

```
exit=0  git push --forc? origin refs/heads/gsd-auto/alpha/w   <- the glob in a push FLAG
exit=2  git push --force origin refs/heads/gsd-auto/alpha/w   [force_push_blocked]  <- the control
```

Confirmed under `bash` against an argv-printing `git` shim: with a file named
`--force` in the working directory the shell assembles the literal command. The
verdict is invariant under the working directory — measured identically with and
without an in-namespace project root — because the explicit refspec means
`push_needs_resolved_dests` answers false and no push context is resolved.

**Mechanism.** `19-14`'s git decision region is the VERB plus `classify_config`'s
key operand. `classify_push`'s FLAGS are a further arm, structurally identical to
`classify_reflog`'s and `classify_symbolic_ref`'s operands (`T-19-91`):
`classify_push` reads `--force`, `-f`, `--force-with-lease`, `--delete` and
`--no-verify` by name, and a flag it cannot read falls to an `Allow` arm. A glob
sets no `Token.expansion` bit at all, which is the same root cause as `T-19-94`
one slot to the right.

**Why it is REGISTERED rather than fixed.** Round discipline, and *not* a
blast-radius judgement. It was found while PLANNING round 5, and a plan cannot
both discover a threat and be the plan that measured it fail first — the
discipline `19-14` established for `T-19-91` and `19-13` for `T-19-86`. Plan
19-16 measures it, pins it at its measured verdict, and adds no rule; plan 19-17
is scoped to the decision region `19-14` defined and does not extend it either.

**Second carrier.** `git push` DOES have `pre-push` behind it, which `git stash`
and `git update-ref` do not. That asymmetry is why this is rated `medium` rather
than `high`, and it is a narrowing rather than a covering: layer 3 sees what git
does regardless of how git was invoked, but only for `push`.

**Pinned at.**
`tests/envelope_literal_decision.rs::the_t_19_96_push_flag_glob_is_measured_and_registered_rather_than_fixed`,
which asserts the measured exit 0 and the literal control's exit 2. If a later
change reaches this cell the pin turns red and the change is disclosed rather
than absorbed.

**Severity.** medium, open — below `high`, so it does not count toward
`threats_open`.

**Suggested owner.** A round-6 gap-closure plan, together with `T-19-86` and
`T-19-91`, or `/gsd-secure-phase 19`'s re-audit.

---

## Round-5 corpus status (recorded by 19-16) — `T-19-92` … `T-19-95` all still OPEN

Plan 19-16 wrote the corpus and the reproducers and stopped. **It closes
nothing**; plan 19-17 writes the rule. Which file carries which class:

* **`tests/envelope_literal_decision.rs`** (new, 41 tests, 21 RED) — audit 4's
  `T-19-92`/`T-19-93`/`T-19-94` reproducers re-measured at `e842fa3`, all
  fourteen reproducing at their recorded verdicts; the position-0 splice; the
  concatenated, multi-expansion, range, increment-range, nested-alternative and
  quoted-run spellings; the flag-slot splice pinned in the IN-NAMESPACE
  configuration; the two glob decision-operand cells; the `T-19-93` counted pair
  with its quoted positive control and its `…/pulls/7` boundary control; the cost
  rows each beside a permitted twin and a written clause derivation; the
  must-not-move controls; the Rule B mechanism pin over
  `policy::split_segments_with_heads`; the `T-19-91` in-namespace measurement;
  and the registered-only `T-19-96` row.
* **`tests/envelope_wrapper_class.rs`** (6 new tests, 5 RED) — the seven
  unreadable classes with degenerate-proof predicates and per-alphabet,
  per-class and counted floors; 22 new `REFUSED_BASES` entries; brace entries in
  `DECOY_OPERANDS`, `EXPANSION_WRAPPERS` and a new `SEVERED_BRACE_PREFIXES`;
  invariant glob and literal-brace entries in `ASSIGNMENT_PREFIXES`, `WRAPPERS`
  and `PERMITTED_BASES`; and two new forge-slot properties, one asserting refusal
  and one asserting a LEDGER LINE.

`T-19-95` is closed only when plan 19-17's rule is certified by this corpus,
because a corpus is evidence about a control and there is no control yet.

**Two rows are NOT what round 5 was written expecting**, and both are recorded in
`19-SECURITY.md`'s plan-19-16 execution record rather than absorbed: the
comma-list spellings of the concatenated class are class COVERAGE rather than
reproduced force pushes (a comma list of N alternatives produces N words, and the
surplus word lands where real git rejects it), and `git push {--force,origin} main`
is cwd-dependent — exit 0 in the namespace a driven run is designed to be in,
exit 2 outside it under an unrelated arm.

---

## Round-5 closure status (recorded by 19-17) — `T-19-92` … `T-19-95` CLOSED

Plan 19-17 wrote the rule the `19-16` corpus was left RED against. All 26 RED
names were confirmed still failing against `09e83bd` before any production line
moved, and all 26 are green after. Full record, with every before/after verdict
and the walked ledger listings, is in `19-SECURITY.md`'s appended plan-19-17
execution record.

* **`T-19-92` — CLOSED, by clause 2.** The tokenizer gained bash's own three-way
  question about `{`: a parameter expansion (case 1, today's behaviour, which is
  what keeps Rule B load-bearing), the reserved word opening a group (case 2,
  today's behaviour, so `{ cmd; }` is unchanged), or a brace pair resolved by
  lookahead. A pair carrying a comma or a two-endpoint range MARKS the enclosing
  simple command, and `resolve_program_with_head` refuses it when a segment
  resolves `Governed`/`NestedPayload` (**2a**) or when a word the whole-word
  product scan can PRODUCE has a governed basename, or the products cannot be
  enumerated (**2b**). Both halves were needed: `{git,push,--force,origin,main}`
  is reached only by 2(b) and `git push {--force,origin} main` only by 2(a).
  Products rather than names, because no alternative of `{g..g}it` spells `git`;
  whole-word rather than per-`{`, because a per-`{` scan answers `g` and `it` for
  `{g..g}{i..i}t`; quote-removed, because otherwise `"g"{i,i}"t"` reads
  `"g"i"t"`. `SEPARATORS` is unchanged and `(`/`)` were not touched.
* **`T-19-93` — CLOSED, by COUNT, in the tokenizer's literal-brace branch and
  nothing else.** `gh api repos/{owner}/{repo}/pulls -f title=x` now writes
  exactly one ledger line and a second creation in the same root is refused under
  `pr_cap_exceeded`; `…/pulls/7` still writes nothing. **Neither forge scan was
  changed.** The rejected alternative — a placeholder tolerance in
  `endpoint_is_pulls` — is recorded in that function's own doc with its three
  reasons.
* **`T-19-94` — CLOSED, by clause 1**, subsumed by the inversion rather than
  added beside it: `*`, `?` and `[` are three of the classes that clear
  `Token.literal`, read in the ONE closure that read `Token.expansion`, over a
  region that did not move. Operands stay free — `git add src/*.rs` and
  `rg "x" src/*` are re-measured permitted.
* **`T-19-95` — CLOSED.** The rule is certified by a corpus that can draw a brace
  expansion, a literal brace pair, a concatenated splice, a multi-expansion word,
  a range, a glob and a tilde, with per-alphabet and per-class floors — and that
  corpus was observed RED first, in commits with zero `src/` hunks. The axis
  earned its keep beyond the enumerated rows: the generative forge-slot property
  caught `gh api repos/o/r/pulls -? title=x`, a cell no enumerated row covered,
  after clauses 1 and 2 had already landed.

### Still OPEN and untouched by 19-17

* **`T-19-86`** (high, OPEN) — unchanged, by explicit user scoping decision. All
  four rows re-measured at exit 0 against the built binary.
* **`T-19-91`** (high, OPEN) — the code-side RECORD is corrected (the bare
  `git push $REF` is cwd-dependent and permitted in the in-namespace
  configuration, making `classify_push`'s refspec operand a third arm of the
  shape). **Not closed, not renumbered, remedy unchanged**, second-carrier
  asymmetry unweakened.
* **`T-19-96`** (medium, open) — registered by `19-16`, re-measured at exit 0 and
  its literal twin at exit 2 under `force_push_blocked`, **not fixed**. Widening
  the decision region to `classify_push`'s flags is the same move as closing
  `T-19-91` and is the next round's to decide.
* **`T-19-74`** — accepted (AR-19-10); core rows re-measured permitted.
* **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85`** — open, unaccepted, untouched.
  `cred.rs`, `advisory.rs`, `scan.rs` and `config.rs` were not opened.

### Newly disclosed cost, accepted (`T-19-17r`)

A brace expansion anywhere in a governed simple command, a splice whose products
name a governed program even in an ungoverned command, and a glob or tilde in a
decision word are now refused. Every one is pinned in
`tests/envelope_literal_decision.rs` beside its PERMITTED twin and the clause
that produces it, and `ls {git,svn}-repo` is pinned permitted as the control that
keeps clause 2(b) a product test rather than a mention test. The widest is
`rg "git status" {src,tests}` (clause 2a), where nothing governed executes at
all.

**`/gsd-secure-phase 19` is NOT cleared by plan 19-17.** `T-19-86` and `T-19-91`
remain OPEN at `high`, and `T-19-96` is registered open. Only the
**wrapper-operand** sub-class of `T-19-60` is closed; `T-19-86` and `T-19-91` are
both sub-classes of it.

---

## Round-6 corpus status (recorded by 19-18) — `T-19-97`, `T-19-98`, `T-19-99` all still OPEN

Plan 19-18 wrote the corpus and the reproducers and STOPPED. **It closes
nothing.** `19-19` writes the rule and is gated on the RED state recorded here.

### `T-19-99` (medium, OPEN) — the corpus could not draw the deletion axis

Audit 5's finding: `UNREADABLE_CLASSES` names seven classes, each with a
degenerate-proof predicate and three kinds of floor, and every one of them is a
word-**ASSEMBLY** class. `grep -rnE '"(git|gh|glab)[^"]*[<>][^"]*"' tests/ src/`
found no guard-driven row carrying a redirection anywhere in the repository, and
neither corpus file held a backslash-newline. The corpus was structurally
incapable of generating, and so of failing on, `T-19-97` and `T-19-98`.

**Corpus written and observed RED at `aa24f9d`.** `tests/envelope_wrapper_class.rs`
gains section 14: `DELETION_CLASSES`, a SECOND named axis standing beside a
byte-identical `UNREADABLE_CLASSES` (1010 insertions, **zero deletions**; no
existing floor, alphabet entry, property or assertion lowered, deleted or
narrowed). Five degenerate-proof quoting-aware predicates, an 11-entry
`DISPLACING_REDIRECTIONS` alphabet spliced BETWEEN the governed program and its
decision words, two `CONTINUATION_SPLICES`, and floors stated as EXACT
equalities derived from the generation arithmetic: 330 cases over 15 slots, per
class 165 / 135 / 240 / 15 / 15.

**STATUS: OPEN.** `T-19-99` is closed only when `19-19`'s rule is certified by
this corpus, because a corpus is evidence about a control and there is no
control yet.

### `T-19-97` (high, OPEN — BLOCKING) — a redirection is a word the shell DELETES

Sixteen audit-5 rows re-measured at `fccb5de` — one fresh `GSD_MM_ENVELOPE_ROOT`
per row, the whole root walked afterwards — **every one reproduced at exit 0**,
the three forge rows with EMPTY walks. All confirmed under `bash` against
argv-printing `git`/`gh`/`glab` shims that write argv to a side file rather than
to stdout, so a `>/dev/null` row cannot swallow its own evidence. Layers 1, 2
and 3 all defeated on a single line, with the D-09 `-c` precedence re-measured
(`/CLI_WINS` against the paired `/ENV_WINS` control). Three rows have **no
second carrier at all**.

**The seven cells found while PLANNING round 6 are folded into this class rather
than registered as new threat IDs**, with the provenance caveat `19-14`
established (found while planning, not by an audit). All seven at exit 0 with
empty walks at `fccb5de`, all confirmed under the shims:

| Cell | Line | Why the audit's rows do not reach it as written |
|---|---|---|
| `&>` | `git &>/tmp/o push --force origin main` | `&` is in `SEPARATORS`, so the GUARD splits one simple command into two; bash's `&>` is ONE redirection operator |
| `{v}>` | `git {v}>/tmp/o push --force origin main` | bash 4.1 fd allocation, carried into the verb slot by round 5's own literal-brace branch. **Post-fix verdict left for `19-19`**; recorded unasserted |
| `>\|` | `git >\|/tmp/o push --force origin main` | two-character operator |
| `<>` | `git <>/tmp/o push --force origin main` | two-character operator, first char `<` and second `>` |
| `<<` | `git <<EOF push --force origin main` | bash runs it even as a single line |
| `<file` | `touch input.txt && git <input.txt push --force origin main` | **PRECONDITION row** — without the file bash runs NOTHING |
| `x2>` | `git x2>/tmp/o push --force origin main` | **the OVER-DELETION control.** `ARGV[git]: [x2] [push] [--force] [origin] [main]` — `x2` IS argv. **Permitted today and must STAY permitted** |

**Corpus RED at `f964926`**, `tests/envelope_argv_deletion.rs` — the fifth
evidence file. The PERMITTED half is pinned beside the refused half, so the
corpus can also fail on a blanket refusal and on over-deletion.

**STATUS: OPEN — BLOCKING.** No `src/` line changed. `19-19` writes the rule.

### `T-19-98` (high, OPEN — BLOCKING) — a line continuation the tokenizer keeps

Eight audit-5 rows re-measured at `fccb5de`, **every one reproduced at exit 0**,
the forge row with an empty walk. Every row driven from a FILE whose bytes
`od -c` verified before bash was driven over it, including the DOUBLE-QUOTED
spelling that reaches the same place through the quote loop's own backslash
branch.

`tokenize`'s backslash arm (`policy.rs:2063-2072`) keeps the escaped character
and deliberately does not clear `literal`, on the reasoning that escaping is
what makes a character literal — **which is true of every character except a
newline**, the one character a backslash DELETES rather than protects. Measured
directly: `split_segments_with_heads("git pu\\`+NL+`sh --force origin main")`
reports the token `("pu\nsh", literal = true)` where the program receives
`push`.

**Audit 5's three discarded rows are NOT re-added** — single-quoted
`\`+newline, `\`+CR, `\`+TAB. The single-quoted spelling was re-run once as a
sanity check of this round's shim harness and confirmed audit 5's discard
(`ARGV[git]: [pu\` / `sh] …` — one mangled word, no force push); it was not
added as a row. Recorded so audit 6 does not spend the measurement again.

**STATUS: OPEN — BLOCKING.**

### The two pre-existing FALSE REFUSALS this round REMOVES

Measured at `fccb5de` in the in-namespace configuration, each beside its
permitted one-line twin:

* `git push origin refs/heads/gsd-auto/alpha/w > log.txt` → exit 2
  `push_outside_namespace`, *the refspec `>` resolves to `refs/heads/>`*. The
  redirection word is read as an EXTRA refspec.
* `git push \`+NL+` origin refs/heads/gsd-auto/alpha/w` → exit 2
  `push_outside_namespace`, *the refspec `origin` resolves to
  `refs/heads/origin`*. **A different mechanism**: the whitespace after the
  continuation flushes it into its own word in the REMOTE slot, so every operand
  shifts one slot right — `T-19-97`'s displacement arriving through `T-19-98`'s
  mechanism.

Both pinned pre-fix in the two `#[test]` fns `19-19` is required to replace:
`the_redirected_in_namespace_push_is_falsely_refused_today_and_19_19_must_move_it`
and `the_continued_in_namespace_push_is_falsely_refused_today_and_19_19_must_move_it`.
**The round removes two measured over-refusals rather than adding any.**

### `T-19-17r` — the bookkeeping gap, OUTSTANDING and the acceptance NOT made

`19-17-SUMMARY.md` calls `T-19-17r` "the new over-refusal cost, **accepted** and
pinned from both sides". Audit 5 confirmed the measurement and both pins —
`rg "git status" {src,tests}` → exit 2 and `rg "git status" src/` → exit 0, at
`tests/envelope_literal_decision.rs:1355-1379` with the producing clause named
— but there is **no `AR-19-13` row in the Accepted Risks Log and no register
row**. It is described as accepted while documented nowhere an audit reads.

**Plan 19-18 records this gap and leaves it OPEN. It adds no `AR-19-13` row and
makes no acceptance.** Accepting a risk is a human decision and audit 5
explicitly declined to make it on the developer's behalf. The next round either
adds the log row or drops the word from the summary.

**STATUS: OUTSTANDING — bookkeeping only, the cost itself is measured and pinned
from both sides.**

### Audit 5's disclosed corpus limit, forwarded to `19-19`

`sh {-c,"git push --force …"}` is refused because a quote inside an alternative
is unenumerable — correct and fail-closed, but the corpus cannot distinguish
that refusal from an enumerated one. Forwarded to `19-19` because the only place
the distinction is observable is a unit assertion over the private whole-word
product scan in `policy.rs`'s own test module, and plan 19-18 may not touch
`src/`.

### `cargo clippy --tests -- -D warnings` fails at the base commit (found during 19-18)

Pre-existing and OUT OF SCOPE for a plan with a zero-`src/`-hunks prohibition.
Verified by removing this plan's new test file and re-running at `fccb5de`:
exit 101, four lint errors, all in `src/` files this plan does not touch —
three `clippy::bool_assert_comparison` at `src/browser.rs:155-157` and one
`clippy::cmp_owned` at `src/project_creator.rs:146`. Plan 19-18's new code adds
**zero** clippy findings. Fixing these requires a `src/` hunk and belongs to
whichever round is allowed to make one.

### Still OPEN and untouched by 19-18

* **`T-19-86`** (high, OPEN) — by explicit user scoping decision. All four rows
  still at exit 0 and
  `the_t_19_86_residual_is_permitted_today_and_this_plan_leaves_it_permitted`
  is green and UNMODIFIED.
* **`T-19-91`** (high, OPEN) — three arms unweakened: `git reflog $S`,
  `git reflog show $S` and `git symbolic-ref $S` at exit 0 with no second
  carrier, and the bare `git push $REF` cwd-dependent and permitted
  in-namespace, which is a THIRD arm and not "already fails closed". No remedy
  added, no denylist extended.
* **`T-19-96`** (medium, open) — unchanged.
* **`T-19-74`** — accepted (AR-19-10); core rows frozen and untouched.
* **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85`** — open, unaccepted, untouched.
  `cred.rs`, `advisory.rs`, `scan.rs` and `config.rs` were not opened.
* **`tests/driver_reattach.rs`** — pre-existing and flaky; both tests passed in
  this plan's gate run, which is not a change 19-18 made and not a fix.

**`/gsd-secure-phase 19` is NOT cleared by plan 19-18, by `19-19`, or by the two
together.** `T-19-86` and `T-19-91` remain OPEN at `high`. Only the
**wrapper-operand** sub-class of `T-19-60` is closed; `T-19-86` and `T-19-91`
are both sub-classes of it and both remain open.

---

## Round-6 closure (recorded by 19-19) — `T-19-97`, `T-19-98`, `T-19-99` CLOSED

Plan `19-19` wrote the rule the `19-18` corpus was red against. All five RED
names `19-18-SUMMARY.md` listed were confirmed **still failing** against
`1d1229e` before any production line moved, and all five are green after.

### `T-19-97` (high) — **CLOSED**

**Closed by:** the redirection production consumed inside `tokenize`'s ONE walk
— an optional **bare** digits-only IO_NUMBER, one of twelve operators
(`< > >> <> >| <& >& &> &>> << <<- <<<`) matched longest-first with `&>`/`&>>`
recognised **before `&` reaches the separator arm**, and the target word —
emitting **no token for either the operator or the target**. A deleted word
never becomes a `Token`, so every decision index is over the surviving argv
automatically; no second reading site, no index primitive changed, and
`first_unreadable_decision_word`'s one closure untouched.

Anything the production does not COMPLETE marks the simple command
UNRESOLVABLE (`Segment::redirection_unresolvable`) and is refused in the arms
`resolve_program_with_head` already had — no second filter, no wildcard.

`SEPARATORS` is byte-identical and `is_separator(">")` is still `false`.

### `T-19-98` (high) — **CLOSED**

**Closed by:** `\`+newline consumed as a LINE CONTINUATION producing **no
character**, in the unquoted backslash arm and in the double-quote loop's
backslash branch, and **never starting a word**. The single-quote loop is
untouched. `Token::literal` is deliberately left TRUE — a deletion is not a
rewrite, and the bit is right about these words.

### `T-19-99` (medium) — **CLOSED**

**Closed by:** `19-18`'s `DELETION_CLASSES` corpus certifying a control that
now exists. The corpus was written FIRST and observed red in commits with zero
`src/` hunks, so the round is not the sixth consecutive one to certify a claim
it could not have failed on.

### The cost, net NEGATIVE

**Removed** — two measured FALSE REFUSALS, both pinned pre-fix by `19-18` in
the two `#[test]` fns it named, both now exit 0 beside their one-line twins:
`git push origin refs/heads/gsd-auto/alpha/w > log.txt` (the redirection word
read as an extra refspec) and its `\`+newline spelling (the continuation
flushed into the remote slot).

**Added** — two shapes, each pinned beside its permitted twin: an unresolvable
redirection in a **governed** simple command (`git >` refused, `ls >` and
`cargo test >` permitted; bash does not run `git >` either), and a `{name}`
fd-allocation prefix, deliberately not modelled (`git {v}>/tmp/o push …` and
its permitted-half twin `git {v}>/tmp/o status` both refused).

Ordinary redirection keeps working and `gh pr create --title x > /tmp/o` stays
COUNTED with one ledger line — the measured cost that rejected the
blanket-refusal design.

### Unchanged and still OPEN

`T-19-86` (high), `T-19-91` (high), `T-19-96` (medium), `T-19-74`'s residual,
`T-19-84`, `T-19-85` and `T-19-61` … `T-19-73` are **untouched and
unaccepted**. **`/gsd-secure-phase 19` is NOT cleared.**

The **`T-19-17r` bookkeeping gap stays OUTSTANDING**: still no `AR-19-13` row
and no register row, and `19-19` did **not** add the acceptance — that is a
human decision audit 5 explicitly declined to make.

Only the **wrapper-operand** sub-class of `T-19-60` is closed; `T-19-86` and
`T-19-91` are both sub-classes of it and both remain open at `high`.

### A threshold worth knowing about

`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
requires production code (comments and the `#[cfg(test)]` module stripped) to
exceed a QUARTER of `policy.rs`. `19-19`'s documentation pushed the ratio to
**24.88%** and turned that control red; the assertion was NOT edited — the
redundant prose was tightened until production code was back above the floor.
**The margin is now thin.** The next round that documents `policy.rs` heavily
should expect to meet this and budget prose accordingly.

---

## Round 7 (plan 19-20) — the corpus for the CALLEE's grammar exists and is RED

Written by plan `19-20` at base `c21c13f`. **This plan closes nothing.** It writes
the corpus and the reproducers and stops; `19-21` writes the rule.

### `T-19-100` — `scan_leading` / `leading_git_option` / `GIT_GLOBAL_VALUE_OPTS` — OPEN, `high`

An incomplete, unpinned enumeration of **git's own global-option grammar** which
**fails OPEN** at `policy.rs:488` (`(None, 1)`). The guard advances one word,
lands on the option's VALUE, and reads that value as the verb.

**Corpus written and RED**: `tests/envelope_callee_grammar.rs`, section 1 — the
nine audit-6 rows asserted at exit 2 with a DERIVED reason identifier each. Every
row measured at exit 0 against the built binary with a fresh envelope root and an
empty walk before it was written as an assertion; every grammar claim confirmed by
a two-sided probe of real git 2.43.0. Four rows have **no second carrier at all**
(`stash`, `update-ref -d`, `reflog delete`, `config core.hooksPath`).

**Twelve cells found while PLANNING are folded into this class as spellings of it,
not registered as new threat IDs** — same provenance caveat `19-14` established
(found while planning, not by an audit). They are the stale over-consuming entry
(`--super-prefix`), the unknown option after a known one (the scan is a loop),
both orders of deletion-and-callee-grammar, a continuation inside the option name,
the nested payload, the sequence, the short bundle, the bare dash, the attached
short `-C`, `--`, and the hooks key ahead of the gap. The full table with exit
codes and walks is in `19-SECURITY.md`'s plan-19-20 record.

**Two of them are LABELLED mis-indexes of commands real git does not run, never
live bypasses**: `git -pc user.name=x status` (`unknown option: -pc`) and
`git - push --force origin main` (`unknown option: -`). `git --super-prefix push …`
is the same — it is not live only because git rejects the option; a stale entry
for an option git ACCEPTS would be one.

**CLOSED by plan `19-21` (`e592f38`).** The clause that closed it:
`leading_git_option` now answers a three-valued grammar question whose default
is **grammar not established**, and `scan_leading` refuses on it at
`ParkReason::EnvelopeAssertionFailed` through its existing refusal channel — the
same fail-closed treatment `resolve_program`'s wrapper axis has. `--attr-source`
and `--shallow-file` were ADDED to `GIT_GLOBAL_VALUE_OPTS`, `--super-prefix`
REMOVED, `GIT_GLOBAL_SELF_CONTAINED_OPTS` added as knowledge the guard never had,
and a real-git drift pin now probes every entry of all three constants
two-sided. Cost measured at zero on git 2.43.0 and one refusal per future global
option, pinned from both sides.

**ONE ROW OF THE CORPUS COULD NOT BE SATISFIED AND WAS LEFT RED, NOT EDITED.**
`git --super-prefix x push --force origin main`, pinned at `force_push_blocked`
in `the_already_correct_planning_cells_keep_their_verdicts_as_controls`, carries
leading tokens IDENTICAL to `git --super-prefix x status`, which
`after_19_21_the_unknown_option_cost_rows_…` pins at `envelope_assertion_failed`.
Since `scan_leading` is a pure argv function and `classify_git` returns its
refusal immediately, no rule obeying this plan's prohibitions produces both;
satisfying the first requires the classifier's verdict to take precedence over
the scan's refusal, which is a second reading site. Both rows are REFUSED at
exit 2 with an empty walk under either rule — only the reason identifier
differs. See `19-SECURITY.md`'s plan-19-21 record for the proof. **Resolving it
is a decision about `19-20`'s assertion and is left open.**

**RESOLVED 2026-09-04 by a scoped follow-up (`989f21a`) — the row was CORRECTED,
not weakened, and no `src/` byte moved.** The `19-21` executor's analysis was
re-verified by MEASUREMENT before anything was edited, against the built binary
with one fresh `GSD_MM_ENVELOPE_ROOT` per row and the envelope root WALKED
afterwards, so the empty walk was observed rather than inferred:

| Command | exit | identifier | walk |
|---|---|---|---|
| `git --super-prefix x status` | 2 | `envelope_assertion_failed` | EMPTY |
| `git --super-prefix x push --force origin main` | 2 | `envelope_assertion_failed` | EMPTY |
| `git -C/tmp push --force origin main` (control) | 2 | `force_push_blocked` | EMPTY |
| `git -- push --force origin main` (control) | 2 | `force_push_blocked` | EMPTY |
| `git -c core.hooksPath=/dev/null --attr-source HEAD push` (control) | 2 | `hook_bypass_blocked` | EMPTY |
| `git - push --force origin main` (control) | 0 | — | EMPTY |

`--super-prefix` is confirmed ABSENT from `GIT_GLOBAL_VALUE_OPTS` and from
`GIT_GLOBAL_SELF_CONTAINED_OPTS` after `19-21` (read from `policy.rs`, and
present in `GIT_GLOBAL_VALUE_OPTS` at the pre-fix base `097dba2`), and git
2.43.0 rejects it in ALL THREE forms — `git --super-prefix version`,
`git --super-prefix x version` and `git --super-prefix=x version` each print
`unknown option:`.

**So `force_push_blocked` was a PRE-FIX OBSERVATION MIS-LABELLED AS POST-FIX.**
It held at `097dba2` only because the stale entry consumed `x` and put `push` in
the verb slot; removing that entry was a deliberate, audit-confirmed part of the
fix. The identifier moved because the constant changed, and
`envelope_assertion_failed` is the accurate one — the guard genuinely cannot
establish the grammar. **The exit code and the empty walk stay asserted; only
the identifier changed.** No filter was added in `classify_git` after
`scan_leading` returns, `SEPARATORS` is untouched, and rounds 4/5/6's mechanism
pins are unmodified and green.

**Sweep for the same defect.** Every identifier-asserting row in
`tests/envelope_callee_grammar.rs` and `tests/envelope_wrapper_class.rs` was
checked against the constants this round changed (`--super-prefix` removed from
`GIT_GLOBAL_VALUE_OPTS`; `-c`/`--config-env`/`--attr-source`/`--shallow-file`
added; `GIT_GLOBAL_SELF_CONTAINED_OPTS` new; `recurse-submodules` added to
`PUSH_VALUE_OPTS`). **This was the ONLY row of the class.** Nothing else needed
changing: `-C/tmp` depends on `-C`, which did not change; the `--` and
`core.hooksPath` cells are structural; `git push --signed no …` depends on
`signed` staying OUT of `PUSH_VALUE_OPTS`, which `19-21` respected; the
`--attr-source=HEAD` attached-form pin is produced by the structural rule and
needs no constant entry; and `envelope_wrapper_class.rs` asserts only exit codes
and taxonomy membership on this axis, never a specific identifier. No other test
file references any of the changed spellings.

**This resolution does NOT clear `/gsd-secure-phase 19`.** `T-19-86` and
`T-19-91` remain OPEN at `high`; only the WRAPPER-OPERAND sub-class of `T-19-60`
is closed; `T-19-17r` stays OUTSTANDING with no `AR-19-13` and is NOT accepted;
`T-19-96`, `T-19-74`, `T-19-84`, `T-19-85` and `T-19-61` … `T-19-73` are
untouched; and the `glab --host` cell keeps its unconfirmed-callee caveat.

### `T-19-101` — every generative alphabet, and the named class axes — OPEN, `medium`

`T-19-76`'s failure mode for the **SIXTH** consecutive round, and this time the
gap moved AXIS rather than one cell over: `UNREADABLE_CLASSES` names seven
word-ASSEMBLY classes and `DELETION_CLASSES` names five word-REMOVAL classes, and
**both are axes of the SHELL's grammar**. Audit 6 verified mechanically that
nothing anywhere in `tests/` modelled the CALLEE's —
`grep -rn "attr-source\|shallow-file\|GIT_GLOBAL_VALUE_OPTS" tests/` returned
nothing at all.

**Corpus written and RED**: `tests/envelope_wrapper_class.rs` section 15 —
`CALLEE_GRAMMAR_CLASSES`, a THIRD named axis standing beside a byte-identical
`UNREADABLE_CLASSES` and a byte-identical `DELETION_CLASSES`, with five
degenerate-proof predicates, a class-tagged `GIT_GLOBAL_OPTIONS` alphabet spliced
where `scan_leading` actually reads, a SEPARATE `GIT_GLOBAL_UNKNOWN_OPTIONS`
alphabet kept out of the invariance arm because its entries are not
verdict-preserving, and floors whose arithmetic is stated (210 cases / 120 refused
/ 90 permitted / 14 slots; per-class 147/70/42/0/14; 56 unknown-alphabet cases).

Of the 120 refused-arm cases, **16 are at exit 0 today** — the 8 `--attr-source
HEAD` and 8 `--shallow-file /tmp/s` cases. That is what makes the property red
pre-fix by construction.

**CLOSED by plan `19-21`.** The clause that closed it: the corpus now certifies
a control that EXISTS. `19-20` wrote it first, observed it RED in commits with
zero `src/` hunks, and `19-21` confirmed the complete seven-name RED set STILL
RED against `097dba2` before a single production line moved. Six of the seven
turned green; the seventh is the irreducible row recorded under `T-19-100`
above. All thirteen `envelope_*` binaries ran.

### `T-19-102` — `push_operands` / `PUSH_VALUE_OPTS` — OPEN, `low`

The same enumeration defect in the **over-refusal** direction.
`git push --recurse-submodules on-demand origin refs/heads/gsd-auto/alpha/w` →
exit 2 `push_outside_namespace`, while real git runs the line to completion
(`Everything up-to-date`). `recurse-submodules` is absent from `PUSH_VALUE_OPTS`,
so `on-demand` is read as the repository and `origin` as a refspec resolving to
`refs/heads/origin`.

**Corpus written and RED**: the false refusal is asserted at its **POST-fix exit
0**, beside its permitted twin — red now, green after, and no body for `19-21` to
replace. `git push --signed no origin refs/heads/gsd-auto/alpha/w` is pinned
**REFUSED** as the discrimination control, because real git reads `no` as the
repository (`error: src refspec origin does not match any`) and a fix copied from
`git push -h` would add `signed` and introduce a real mis-parse.

**CLOSED by plan `19-21` (`fe49142`).** The clause that closed it:
`PUSH_VALUE_OPTS` gained `recurse-submodules` and **nothing else** — in
particular NOT `signed`, whose pin stays REFUSED and green. `push_operands`
deliberately did NOT get `scan_leading`'s fail-closed default, because its
unknown-flag direction is an over-refusal rather than a bypass and a fail-closed
version would refuse `git push --dry-run origin <ref>` (AR-19-11); its
OVER-consuming direction is covered instead by the real-git drift pin added over
`PUSH_VALUE_OPTS`, whose negative controls are `--signed` and `--dry-run`.

### The `glab --host` forge cell — RECORDED, callee UNCONFIRMED, deliberately NOT fixed

`glab --host gitlab.com mr create --title x` → exit 0 with **ZERO** ledger lines;
`glab --hostname …` → exit 0 with **one**. `FORGE_VALUE_OPTS` is
`["-R", "--repo", "--hostname"]` — the same hand-maintained enumeration of a
callee's option grammar, in a third component, failing in the UNDER-COUNTING
direction (SAFE-06).

**`glab` is NOT installed on this machine**, so whether glab accepts `--host` as a
separate-value global flag is not confirmed against the callee. **This is not
claimed as a live bypass.** `gh` was swept and is clean — `gh --repo o/r`,
`gh -R o/r`, `gh --hostname h.example`, `gh api --hostname h.example` and
`gh --version` all leave exactly one ledger line.

**Still OPEN and untouched after plan `19-21`.** That plan fixed the same defect
class in `scan_leading` and in `push_operands` and deliberately did not touch
`FORGE_VALUE_OPTS`, `GH_API_VALUE_OPTS` or `subcommand_word_indices`. The callee
claim remains unconfirmed — `glab` is still not installed — so this is still not
claimed as a live bypass. It needs a machine with `glab` installed.

**Out of round 7's three-item scope. Do not fix `FORGE_VALUE_OPTS`,
`GH_API_VALUE_OPTS` or `subcommand_word_indices` as part of it.** A candidate for
the round after `19-21`, and it needs a machine with `glab` installed to confirm
the callee's grammar first.

### `T-19-17r` — still OUTSTANDING, and plan 19-20 did NOT accept it

Unchanged by this round. `19-17-SUMMARY.md` calls it "accepted"; there is still
**no `AR-19-13` row in the Accepted Risks Log and no register row**. Audits 5 and
6 both confirmed the measurement and both pins and both explicitly declined to
make the acceptance, because accepting a risk is a human decision. **Plan 19-20
recorded the gap and made no acceptance.** The next round either adds the log row
or drops the word.

### The anti-vacuity ratio note above is now SUPERSEDED

The entry warning that the `policy.rs` production-code ratio floor was thin is
resolved rather than merely tightened. Plan `19-20` re-measured it **in BYTES**
(Rust's `len()` is a byte length; a `str`-character reading of `policy.rs` is 612
bytes light): raw 263,360 / stripped 65,947 = **25.0406%**, headroom **107
stripped bytes ≈ 428 comment bytes** — less than `19-21`'s own doc additions. The
ratio assertion was **deleted and replaced** by absolute per-file floors,
`POLICY_MIN_PRODUCTION_BYTES = 40_000` and `HOOKS_MIN_PRODUCTION_BYTES = 20_000`,
which are independent of comment volume. **No prose needs to be budgeted for it
any more.**

### Unchanged and still open

`T-19-86` (OPEN, `high`, four rows still at exit 0, by explicit user scoping
decision), `T-19-91` (OPEN, `high`, arms unweakened — `reflog $S`, `reflog show
$S`, `symbolic-ref $S` at exit 0 with no second carrier, bare `git push $REF` at
exit 0 in-namespace), `T-19-96` (registered, not fixed), `T-19-74` (core rows
frozen and re-measured permitted), and `T-19-61`…`T-19-73`, `T-19-84`, `T-19-85`
(open and unaccepted by explicit user decision). **Because `T-19-86` and `T-19-91`
remain open at `high`, neither `19-20` nor `19-21` clears `/gsd-secure-phase 19`.**

### The `envelope_tracer` ETXTBSY entry above is CONFIRMED, not superseded

`19-19-SUMMARY.md` presents this flake as newly observed. It has been carried here
since **19-07** with the same symptom string, diagnosis and frequency. Recorded in
`19-SECURITY.md`'s plan-19-20 record as a provenance slip rather than a defect;
audit 6 ran the test 8/8 green in isolation, and it fails closed by ERRORING, so
it cannot mask a regression. Out of scope — do not "fix" it.

## Round 8 (plan 19-22) — the corpus for git's CONFIG RESOLUTION exists and is RED

Recorded by plan 19-22. **Nothing here is closed.** `T-19-103`, `T-19-104`,
`T-19-105`, `T-19-106` and `T-19-107` are all OPEN at this plan's end; `19-23`
writes the rules. **`T-19-86` and `T-19-91` remain OPEN at `high`, so
`/gsd-secure-phase 19` is not cleared by this plan.**

Everything below was measured against the BUILT BINARY at `d0eb738` with one fresh
`GSD_MM_ENVELOPE_ROOT` per row and the envelope directory WALKED afterwards (EMPTY
on every row unless stated), and every precedence claim was confirmed against the
REAL `git` binary (`git version 2.43.0`) using the envelope's own
`GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` injection as the control. **Every audit-7 row
reproduced at its recorded verdict; none failed to reproduce.**

### `T-19-103` — git's own CONFIG RESOLUTION reaches `core.hooksPath` — OPEN, `high`

`scan_leading` decides ONE question about a `-c` assignment: is the KEY half
`core.hooksPath` (`is_hooks_path_key`, `policy.rs:750`)? A key in the `include` or
`includeIf` section names a FILE whose contents are spliced in **at the precedence
of the directive that named them** — command-line precedence, outranking the
envelope's injected triplet — **without the string `core.hooksPath` appearing.**

Ten rows measured at exit 0 on LAYER-2-PERMITTED bases and now asserted at exit 2
`envelope_assertion_failed` in `tests/envelope_config_resolution.rs`:
`-c include.path=` on `commit -m x`, on `status` and on the in-namespace push;
`-c includeIf.gitdir:/tmp/.path=`; `--config-env=include.path=EVILVAR` and its
separate-word spelling; `-c INCLUDE.PATH=`; the quoted spelling `-c
"include.path=…"`; `-c include.pathx=` (the disclosed cost); and the
indirection-first ordering row.

**Confirmed end to end against real git with a bare remote, reproduced rather than
cited:** an in-namespace push the `pre-push` hook REFUSES completed and MOVED the
remote's ref under the carrier (`11b417c -> 0e482a9`), and the `pre-commit` point
fell the same way. Four legs, SHAs before and after each.

**Spellings of this class found while PLANNING** (provenance caveat `19-14`
established: found while planning, not by an audit), folded in here rather than
registered as new threat IDs:

- **the quoted carrier** — `git -c "include.path=…" status` -> exit 0. Quoting is
  removed by the tokenizer before the scan reads the key, so it is neither an
  evasion nor a defence.
- **the near miss with no section** — `git -c includepath=… status` -> exit 0, and
  it must STAY exit 0. This round's `--signed no`.
- **the near miss with the wrong section** — `git -c notinclude.path=… status` ->
  exit 0, stays 0. The SECTION decides, not the letters.
- **the disclosed cost** — `git -c include.pathx=… status` -> exit 0 today,
  refused after. Real git IGNORES the key (measured: the injection's `/ENV_WINS`
  still wins), so this is an over-refusal in the SAFE direction, stated in the
  corpus rather than found by audit 8.
- **the two ordering rows** — both exit 2 `hook_bypass_blocked` today; pinned at
  DIFFERENT post-fix identifiers because the scan is left to right, which is what
  forbids a second-pass implementation.
- **the `~` collision** — `git -c includeIf.gitdir:~/p/.path=… status` -> exit 2
  `envelope_assertion_failed` **TODAY**, by the existing rewriting-character
  clause and NOT by this round's rule. A CONTROL, labelled one.
- **the dotless key** — `git -c a=b status` -> exit 0, `git -c a=b push --force
  origin main` -> exit 2 `force_push_blocked`. Real git RUNS `git -c a=b version`
  at rc 0 and errors only when something READS the key. This is
  `CALLEE_KNOWN_LEADING_PREFIX` (`tests/envelope_wrapper_class.rs:5197`) with
  round 7's whole generative property spliced behind it; refusing it would be
  permanently red in a file `19-23` may not edit.

### `T-19-104` — `GIT_CONFIG_PARAMETERS` is absent from `ENVELOPE_ENV_KEYS` — OPEN, `high`

Git's own internal carrier for `-c`. It OUTRANKS the envelope's triplet (measured
`/PARAM_WINS` against the control's `/ENV_WINS`) and git **EXPORTS** it, so one
prefix disarms every git SUBPROCESS of the command — which is how it composes with
`T-19-86`. Six rows measured at exit 0 and asserted at exit 2
`hook_bypass_blocked`, including all three environment spellings and the
`echo GIT_CONFIG_PARAMETERS` bare-word cost twin.

**A gap in the LIST, not in the mechanism**, and the paired discriminator proves
it: all three spellings of `GIT_CONFIG_COUNT=0` are refused TODAY at
`hook_bypass_blocked` on both a refused and a permitted base, and `echo
GIT_CONFIG_COUNT` is refused too. One list entry derives every row.

**Why the existing drift pin structurally cannot see it**: it is sourced from
`cred::EnvelopeEnv::with_run_id(build_env_in(…))` — the keys the envelope SETS or
REMOVES — and this is a key the envelope neither sets nor removes but which
DEFEATS one it sets. The fix is a SECOND SOURCE, not a wider filter.

**`GIT_CONFIG_NOSYSTEM` — folded in here, NOT a new threat ID and NOT a bypass.**
Found while planning (`19-14` provenance caveat). The DEFEAT is measured against
real git: `GIT_CONFIG_SYSTEM=<file with credential.helper=evil> git config --get
credential.helper` prints `evil`, and adding `GIT_CONFIG_NOSYSTEM=1` makes it exit
1 having read nothing. Guard-side `GIT_CONFIG_NOSYSTEM=1 git push origin
refs/heads/gsd-auto/alpha/w` -> exit 0. **The HARM is INERT**: `cred::write_gitconfig`
points BOTH `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` at the same helper-free
file, so suppressing the system read removes a deny the global pointer duplicates.
Whether it earns a list entry is `19-23`'s design decision; this plan asserts no
post-fix verdict for it.

### `T-19-105` — the corpus could not draw the config-resolution axis — OPEN, `medium`

`T-19-76`'s failure mode for the EIGHTH consecutive round, and for the THIRD round
running the gap moved AXIS rather than one cell over. Audit 7 verified it
mechanically: `grep -rn "include\.path\|includeIf" src/ tests/` returned nothing
at all, and `grep -rn "GIT_CONFIG_PARAMETERS" src/ tests/` returned nothing at
all. Both greps still returned nothing while planning.

**Repaired here as a FOURTH named axis** — `CONFIG_RESOLUTION_CLASSES` in
`tests/envelope_wrapper_class.rs`, standing beside a byte-identical
`UNREADABLE_CLASSES`, `DELETION_CLASSES` and `CALLEE_GRAMMAR_CLASSES` — with five
degenerate-proof predicates, three mutually-disjoint alphabets
(`CONFIG_CONFINED_CARRIERS`, the only one the invariance arm draws;
`CONFIG_INDIRECTION_CARRIERS`; `CONFIG_ENV_CARRIERS`, spliced as an assignment
prefix), a mechanical DOTLESS fence protecting `CALLEE_KNOWN_LEADING_PREFIX`, a
mechanical PERMITTED-BASE fence, and floors whose arithmetic is stated: 161 cases
(69 on refused bases, 92 on permitted) over 21 slots, per-class 70 / 105 / 21 / 28
/ 28.

**Closed only by `19-23`**, because a corpus is evidence about a control and there
are no controls yet.

### `T-19-106` — the callee-grammar drift pin's REACH — OPEN, `medium`

`git config --comment core.hooksPath /dev/null` measured at **exit 0**, while real
git answers ``error: unknown option `comment'`` — `CONFIG_VALUE_OPTS`
(`policy.rs:997`) carries a stale entry, the same enumeration defect one component
over. Registered here; the rule is `19-23`'s. Not fixed by this plan.

### `T-19-107` — the "ZERO over-refusal cost on git 2.43.0" claim — OPEN, `low`

`git -v` measured at **exit 2 `envelope_assertion_failed`** while real git runs it
at rc 0 printing `git version 2.43.0`. A false reassurance in a control's own doc.
Registered here; the rule is `19-23`'s. Not fixed by this plan.

### The `T-19-86` PERSISTED-ALIAS arm — RECORDED under `T-19-86`, which is NOT closed

Measured: `git config alias.p "!git push --force origin HEAD:refs/heads/main"` ->
exit 0, and `git p` -> exit 0 — two separately-permitted tool calls the stateless
guard cannot correlate. The register names four `T-19-86` rows and not this one.

**Layer 3 catches the inner push TODAY**, which is `AR-19-03` working, **and that
is precisely what `T-19-103` removes.** Closing `T-19-103` is therefore a
**RESTORATION of layer 3's catch and never a closure of `T-19-86`.** `T-19-86`
stays OPEN at `high` with all five arms at exit 0, unchanged, unnarrowed and
unre-scoped.

### The `glab --host` forge cell — unchanged, callee still UNCONFIRMED, deliberately NOT fixed

`glab` is confirmed **NOT INSTALLED**, so whether it accepts `--host` as a
separate-value global flag is not confirmed against the callee. Audit 7 declined
to upgrade it; plan 19-22 declines too. Not claimed as a live bypass;
`FORGE_VALUE_OPTS`, `GH_API_VALUE_OPTS` and `subcommand_word_indices` untouched.

### `T-19-17r` — still OUTSTANDING, and plan 19-22 did NOT accept it

`19-17-SUMMARY.md` calls it "accepted". There is still **no Accepted-Risks-Log
row, no `AR-19-13` and no register row**. Audits 5, 6 and 7 all confirmed the
measurement and both pins at `tests/envelope_literal_decision.rs:1355-1379` and
all three deliberately declined to make the acceptance, because accepting a risk
is a human decision. **Plan 19-22 makes no acceptance either.** The next round
either adds the log row or drops the word.

### Unchanged and still open

`T-19-86` (OPEN, `high`, now five arms at exit 0, by explicit user scoping
decision), `T-19-91` (OPEN, `high`, arms unweakened — `reflog $S`, `reflog show
$S`, `symbolic-ref $S` at exit 0 with no second carrier, bare `git push $REF` at
exit 0 in-namespace), `T-19-96` (registered, not fixed), `T-19-74` (core rows
frozen), and `T-19-61`…`T-19-73`, `T-19-84`, `T-19-85` (open and unaccepted by
explicit user decision). **Because `T-19-86` and `T-19-91` remain open at `high`,
plan 19-22 does not clear `/gsd-secure-phase 19` and neither will `19-23`.** Only
the WRAPPER-OPERAND sub-class of `T-19-60` is closed.

### The three documented flakes — none fired, and that is not evidence they are fixed

`tests/driver_reattach.rs`'s two
(`a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step`,
`a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`)
and `tests/envelope_tracer.rs`'s ETXTBSY stub-write race
(`a_relocated_copy_of_the_stub_refuses_instead_of_acting`). All three are
pre-existing, environmental and out of scope — do not "fix" them.

### The byte floors, re-measured (they are load-bearing, not cosmetic)

`policy.rs` raw 307,496 / stripped 68,785 against `POLICY_MIN_PRODUCTION_BYTES =
40_000`; `hooks.rs` raw 99,909 / stripped 33,460 against
`HOOKS_MIN_PRODUCTION_BYTES = 20_000`. **`policy.rs`'s ratio is now 22.37%, so the
25% ratio assertion `19-20` deleted would be RED TODAY.** Both floors unchanged by
this plan, which is granted no deletion of any kind.

## Round 8 closure (plan 19-23) — the rules exist and the corpus certifies them

Recorded by plan 19-23. Every row was re-measured against the BUILT BINARY with
one fresh `GSD_MM_ENVELOPE_ROOT` per row and the envelope directory WALKED
afterwards (EMPTY on every row), and every precedence claim was re-confirmed
against the REAL `git` binary (`git version 2.43.0`) with the exact
`GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` triplet `cred::hooks_path_env` emits as the
control — reproduced this round rather than cited from `19-22`.

**`19-22`'s complete RED set was confirmed STILL RED before any production line
moved.** All nine names were failing at `b9d8eca`; none was already green.

### `T-19-103` — git's own CONFIG RESOLUTION reaches `core.hooksPath` — **CLOSED**

**Closed by the CONFINEMENT CLAUSE** in `scan_leading`'s existing
`if let Some(assignment)` block, ordered after the rewriting-character refusal and
before `is_hooks_path_key`: a key whose **SECTION** names an indirection
(`config_key_names_an_indirection_section`, compared `eq_ignore_ascii_case`
against `INDIRECTION_SECTIONS`) makes the command unresolvable at
`ParkReason::EnvelopeAssertionFailed`, raised inside the ONE scan and returned
through the channel that already carried two refusals.

The SUBSECTION and the VARIABLE are **never read**, so `includeIf`'s open
condition family and any future variable in either section are covered by
construction. A DOTLESS key stays CONFINED, which is what keeps
`CALLEE_KNOWN_LEADING_PREFIX` and round 7's whole generative property green. No
new `ParkReason`, no second reading site, `hooks.rs` unopened.

**RESIDUE, disclosed and NOT handed to any control**: a future git that adds a
THIRD indirection section is not covered, the rule **fails OPEN** on it, and there
is **NO automated control over that direction**. The real-git pin holds the
REVERSE direction and cannot observe a section it does not name. Re-audit by a
human is the only compensating control.

**Disclosed over-refusal**: `git -c include.pathx=/tmp/evil.cfg status` — a key
git IGNORES, refused because the rule reads the section and not the variable.
Safe direction.

### `T-19-104` — `GIT_CONFIG_PARAMETERS` absent from `ENVELOPE_ENV_KEYS` — **CLOSED**

**Closed by one `ENVELOPE_ENV_KEYS` entry**, which turns all six rows green
through the mechanism that already worked — the three-spelling
`GIT_CONFIG_COUNT=0` discriminator was already refused; only the LIST was short.
`GIT_CONFIG_NOSYSTEM` was added beside it as a MEASURED DEFEAT with an **INERT**
harm, **never called a bypass**.

**Closed durably by a SECOND PIN SOURCE**, `ENVELOPE_ENV_DEFEATING_KEYS`, because
the existing drift pin is sourced from the keys the envelope SETS or REMOVES and
**structurally cannot see** a key the envelope neither sets nor removes but which
DEFEATS one it does. Each entry carries the defeated key in the DATA and each
defeat is MEASURED against real git. The `write_gitconfig` double pointer that
makes `GIT_CONFIG_NOSYSTEM`'s harm inert is now itself pinned, so a later change
cannot spend the inertness silently.

### `T-19-105` — the corpus could not draw the config-resolution axis — **CLOSED**

Closed in the only sense available to it: `19-22`'s corpus was written first,
observed RED in commits with zero `src/` hunks, and **confirmed still RED before
any production line moved here**. It now certifies controls that exist.

**One defect in that corpus was found by execution and corrected**: the five
`--force` composition rows asserted `force_push_blocked` while the demotion of
those rows to verdict-only controls was stated in the control's own comment, in
`19-22-SUMMARY.md`, in `19-22-PLAN-CHECK.md` Check 4 and 5, and in
`19-23-PLAN.md:668` — and implemented in none of them. It was unsatisfiable
against section 7's ordering pin. The executor halted and reported rather than
working around it; the orchestrator authorized a named, narrow correction scoped
to the identifier constant on those five rows, and decided CARRIER BEFORE VERB
explicitly. See the plan-19-23 execution record in `19-SECURITY.md`.

### `T-19-106` — the callee-grammar drift pin's REACH — **CLOSED**

`CONFIG_VALUE_OPTS` **LOST `--comment`** — `git config --comment core.hooksPath
/dev/null` moved from exit 0 (with `/dev/null` read as the key) to exit 2
`hook_bypass_blocked`, agreeing with its twin — and gained a two-sided real-git
pin that reads git's own classification (``requires a value`` / ``unknown
option``), needing no per-entry variant value. The four STRUCTURAL arms of
`leading_git_option` are pinned; the two whose stated premises this git
contradicts (`-c<rest>`, `-C/tmp`) had their docs corrected **without their
behaviour changing**, and arm 4's one-element restriction is recorded.

**`FORGE_VALUE_OPTS` and `GH_API_VALUE_OPTS` remain UNPINNED** — `glab` is
confirmed not installed and a pin that skips is fail-open. `subcommand_word_indices`
untouched.

### `T-19-107` — the "ZERO over-refusal cost on git 2.43.0" claim — **CLOSED**

False by one measured row. `-v` joined `GIT_GLOBAL_SELF_CONTAINED_OPTS` after the
probe classified it, and `git -v` and `git -v status` moved from exit 2
`envelope_assertion_failed` to exit 0 beside `git --version`. The claim is
corrected in the constants' docs and the pin's doc, and **recorded BESIDE**
`19-21-SUMMARY.md` and the appended plan-19-21 record rather than as an edit to
either.

### Still OPEN and unchanged by this plan

- **`T-19-86`** — OPEN at `high` by explicit user scoping decision. Four
  registered rows still at exit 0, pin green and UNMODIFIED, and the
  **PERSISTED-ALIAS arm still at exit 0 on both calls**, re-measured this round.
  **Closing `T-19-103` RESTORES layer 3's catch of that arm — a RESTORATION, never
  a closure.**
- **`T-19-91`** — OPEN at `high`, arms unweakened, no decision-operand rule added,
  no denylist extended.
- **`T-19-96`** — left exactly as pinned by `19-16`.
- **`T-19-74`** — core rows frozen.
- **`T-19-84`, `T-19-85`, `T-19-61` … `T-19-73`** — open and unaccepted by
  explicit user decision. The `cred.rs:241-248` doc correction is the ONE named,
  narrow exception and `T-19-61` … `T-19-73` were NOT taken on.
- **the `glab --host` forge cell** — carried forward UNFIXED with its
  unconfirmed-callee caveat, deliberately not upgraded without evidence.
- **the `T-19-17r` bookkeeping gap** — still **OUTSTANDING**. No
  Accepted-Risks-Log row, no `AR-19-13`, and the word "accepted" is not applied to
  it. Accepting a risk is a human decision and three audits declined to make it.

**`T-19-86` and `T-19-91` remain OPEN at `high`, so `/gsd-secure-phase 19` is NOT
cleared by this plan.** Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.

---

## Plan 19-24 — round 9's CORPUS (the rules are `19-25`'s)

**This plan closes NOTHING.** It writes the corpus and the reproducers for the
REGION round 8's rule does not reach and stops, RED, with zero `src/` hunks.
`T-19-105`, `T-19-106`, `T-19-108` and `T-19-109` are all OPEN at this plan's end.
**`T-19-86` and `T-19-91` remain OPEN at `high`, so `/gsd-secure-phase 19` is NOT
cleared by this plan, by `19-25`, or by the two together.** Only the
WRAPPER-OPERAND sub-class of `T-19-60` is closed.

### `T-19-108` — a carrier inside a value the guard CONFINED — OPEN, `high`

**Corpus RED, rules are `19-25`'s.** Round 8's confinement clause is the right
SHAPE of rule — it asks whether an assignment can be BOUNDED, not whether it spells
a name — but it reads config in TWO regions (the leading-option region
`scan_leading` walks, and the environment) while git resolves from a THIRD: a value
the guard itself confined and let through. Git re-parses a non-`!` `alias.<name>`
body as a git command line IN-PROCESS **including its leading options**, so
`alias.q` correctly answers `false` to `config_key_names_an_indirection_section`,
is CONFINED, and carries the indirection into a position the scan never reads.

Measured against the built binary, fresh envelope root per row, walk EMPTY on every
one; confirmed against real git 2.43.0 with the envelope's own
`GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` injection as the control (`/ENV_WINS`):

- `-c alias.q="-c include.path=<f> push --force origin HEAD:refs/heads/main" q` →
  exit 0 on a SINGLE line; `-c alias.a='-c include.path=<f> config --get
  core.hooksPath' a` → `/INCLUDE_WINS`.
- The PERSISTED arm is live in every spelling (`--global`, `--worktree`, `--file`,
  `--add`, `--replace-all`) and at both persistence levels.
- The bare-remote fixture was REBUILT with a CONTROL beside every leg: the plain
  in-namespace push refused with the ref unmoved; **the carrier-free alias control
  also refused with the ref unmoved** (which is what isolates the CARRIER from the
  ALIAS); the persisted non-shell alias carrying the include moved the ref
  `0084939` → `4a77396`; the `-c` delivery moved it `4a77396` → `d833ba0`; the
  `pre-commit` point fell the same way.

**`T-19-108` will close only AS SCOPED.** The non-shell alias body closes; audit
7's `!`-bodied destructive pair does NOT — that is `T-19-86`.

### `T-19-109` — the counted-completeness claim — OPEN, `low`

Registered here; the correction is `19-25`'s. `cred.rs:246-260`'s "FIVE forms"
table and the "RESTORATION of layer 3's catch" claim in the plan-19-22 record, the
plan-19-23 record and `19-23-SUMMARY.md` are incomplete: a sixth form exists — a
non-`!` `alias.<name>` body carrying `-c include.path=<f>`, measured
`/INCLUDE_WINS` against the control's `/ENV_WINS` and measured moving a bare
remote's ref. **This plan carries zero `src/` hunks and does not correct
`cred.rs`.**

### `T-19-105` — the corpus could not draw the class — OPEN for the NINTH round

`T-19-76`'s failure mode for the ninth consecutive round, and **for the SECOND
round running the gap was one REGION over on the SAME axis rather than one axis
further out.** Audit 8 verified mechanically that `CONFIG_RESOLUTION_CLASSES`'s
five classes were all argv or environment carriers and that
`grep -rn "alias\.[a-z]*=\"-c\|alias.*include\.path" src/ tests/` returned nothing
at all; re-verified at this plan's base commit, it still returned nothing.

Repaired here in the only sense available to a corpus plan: a **SIXTH class on the
EXISTING axis** — *a carrier delivered inside a config VALUE the guard confines* —
with `MIN_CONFIG_RESOLUTION_CLASSES` raised 5 → 6, a `CONFIG_REPARSED_VALUE_CARRIERS`
alphabet plus a `CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS` arm, and floors
re-derived and stated as exact equalities (264 cases / 111 refused / 148 permitted
/ 5 persisted / 21 slots; per class 70 / 203 / 21 / 28 / 28 / 75, all confirmed by
the counting test on the first run). **Standing it up as a FIFTH AXIS would have
modelled a REGION as if it were a STAGE and lost the distinction audit 8 drew.**

**Two deliveries, open for two DIFFERENT reasons**, asserted mechanically: the `-c`
delivery IS read by round 8's region and IS confined (that overlap *is*
`T-19-108`); the PERSISTED delivery is not read by that region at all, because
`config` is the verb and `scan_leading` stops there. A rule written only inside
`scan_leading` closes only the first.

### `T-19-106` — the drift pins' REACH — **CORRECTED from CLOSED to OPEN, `medium`**

**The entry at "Plan 19-23 … `T-19-106` — **CLOSED**" above is a MIS-CLOSURE and is
corrected here rather than edited there.** Audit 8 re-opened it at `medium`:
`GH_API_VALUE_OPTS` is a pure `gh api` constant, **`gh` 2.45.0 IS installed**, and
the stated reason for leaving it unpinned ("`glab` is confirmed not installed") is
a fact about a DIFFERENT callee.

Re-measured at this plan's base commit with an **ENDPOINT-LESS** probe form
(`gh api <opt>`), so no row touches the network. All seventeen entries answer
`flag needs an argument`; `gh api --bogus-opt` answers `unknown flag:
--bogus-opt`; and **`gh api --paginate` answers `accepts 1 arg(s), received 0`** —
a POSITIONAL error, not a flag-level one, which is exactly what makes it the second
negative control: the flag was ACCEPTED and consumed NO value. The endpoint-bearing
form `gh api repos/o/r --paginate` makes a REAL HTTP REQUEST, so a pin written that
way would be non-hermetic and would fail open on a machine without network or auth.

**And a correction to audit 8's own suggestion, measured.** Removing `--hostname`
from `FORGE_VALUE_OPTS` would be a **REGRESSION in the under-counting direction**:
`glab --hostname gitlab.com mr create --title x` leaves exactly ONE ledger line
today, while `glab --host …` — an option NOT in that constant — leaves ZERO. The
disposition is **"record why", not "remove"**, and the work is `19-25`'s.

### `T-19-86` — RECORDED MEASURABLY WIDER, and NOT closed

**OPEN at `high` by explicit user scoping decision. This plan does not fix, close,
narrow or re-scope it, and adds no remedy.** Its four registered rows plus the
persisted-alias arm were re-measured at exit 0 and their pins are green and
UNMODIFIED.

**Audit 8 found it measurably WIDER than the register credits, and this plan
records that with a measurement.** Three documents attribute a layer-3 catch to the
closure of `T-19-103`: the `pre-push` hook fires when the alias body runs, so the
inner push is caught. **That catch is ABSENT when the alias body carries a carrier
of its own** — a `!` body runs in a child that inherits the injection and then
applies its own command-line carrier on top of it *inside that child*. Re-measured
at this plan's base commit, after `19-23` landed: audit 7's `!`-bodied destructive
pair still moved the bare remote's ref, `d833ba0` → `9687d94`.

**Audit 7's destructive demonstration therefore still works after this round**, and
closing it would mean taking on `T-19-86`, whose two rows are pinned PERMITTED in
`tests/envelope_command_position.rs:550` and
`tests/envelope_config_resolution.rs:1539-1543` — files this round may not edit.
**Git's rule is the FIRST BYTE**, measured in nine spellings, and the corpus fences
it mechanically so `19-25` cannot be handed an undischargeable seam.

### The fail-open residue's missing REVISIT CONDITION — recorded, no acceptance invented

Recorded exactly as audit 8 recorded it: the admission is complete and correctly
unclaimed in five places, the residual is acceptable and properly bounded, and the
single gap is that its only control is a human reading a future git's release notes
and **nothing schedules that**. `19-25` adds the revisit condition and a version
witness. **That witness is a SCHEDULE and not a CONTROL** — it says WHEN to look and
cannot say WHAT changed; it does not observe a new indirection or re-parsed section
appearing, so the residues stay uncovered by any automated control and audit 8's
judgement of them is unchanged. **This plan invents no acceptance and adds no `AR-`
row.**

### `policy.rs:6644`'s stale proportional-floor comment — DOCUMENTATION DRIFT, not fixed

The comment cites "228,785 / 78.7%"; re-measured at this plan's base commit by
splitting the file at its `#[cfg(test)]` sentinel, `policy.rs` is raw 372,920 /
production **228,101**, i.e. 180,000 is **78.9%**. **The floor itself is correct and
load-bearing**; only the arithmetic in its own comment has drifted. **This plan
carries zero `src/` hunks and may not edit `policy.rs` at all**, so it is recorded
here and left for a later round.

### Still OPEN and unchanged by this plan

- **`T-19-86`** — see above: OPEN at `high`, recorded WIDER, NOT closed.
- **`T-19-91`** — OPEN at `high`, arms unweakened (`git reflog $S`,
  `git reflog show $S`, `git symbolic-ref $S` at exit 0 with no second carrier), no
  decision-operand rule added, no denylist extended.
- **`T-19-96`** — left exactly as pinned by `19-16`.
- **`T-19-74`** — core rows frozen.
- **`T-19-84`, `T-19-85`, `T-19-61` … `T-19-73`** — open and unaccepted by explicit
  user decision. `cred.rs`, `advisory.rs`, `scan.rs`, `config.rs`, `mod.rs` and
  `hooks.rs` were not touched.
- **the `glab --host` forge cell** — carried forward UNFIXED with its
  unconfirmed-callee caveat. `command -v glab` was re-run at this plan's base
  commit and found nothing, so the callee's grammar is still unconfirmed and it is
  NOT claimed as a live bypass.
- **the `T-19-17r` bookkeeping gap** — still **OUTSTANDING**. `19-17-SUMMARY.md`
  calls it "accepted"; audits 5, 6, 7 and 8 all confirmed the measurement and both
  pins and **all four deliberately declined to make the acceptance**. There is no
  Accepted-Risks-Log row, no `AR-19-13` and no register row, and the word
  "accepted" is not applied to it anywhere in this plan's artifacts. **This plan
  does not make the acceptance, because accepting a risk is a human decision.**
- **the three documented flakes** — the two `tests/driver_reattach.rs` failures and
  the `tests/envelope_tracer.rs` ETXTBSY stub-write race, all pre-existing,
  environmental and out of scope.

## Plan 19-25 — the rules (round 9's second half)

**`/gsd-secure-phase 19` is NOT cleared by this plan.** `T-19-86` and `T-19-91`
both remain OPEN at `high`. Only the WRAPPER-OPERAND sub-class of `T-19-60` is
closed.

### `T-19-108` — a carrier inside a value the guard CONFINED — **CLOSED AS SCOPED**, `high`

**Closed for the NON-SHELL alias body, in all three deliveries:**

- the `-c` carrier and the `--config-env` carrier, by the RE-PARSE clause in
  `scan_leading`'s existing assignment block — `REPARSED_COMMAND_SECTIONS` +
  `config_key_names_a_reparsed_command_section` +
  `reparsed_command_assignment_is_a_shell_body`, at
  `ParkReason::EnvelopeAssertionFailed`, ordered after the confinement clause and
  before `is_hooks_path_key`;
- the PERSISTED write, by the same question asked at `classify_config`'s key
  operand — the operand `is_hooks_path_key` already reads — with git's one-byte
  rule applied to the VALUE WORD via `ConfigScan::value_word`;
- at both keys the body can carry: `include.path` and `core.hooksPath` by name.

**NOT CLOSED — and this arm stays OPEN under `T-19-86`:** a `!`-bodied alias body
carrying its own carrier. `git config alias.q '!git -c include.path=<evil> push
--force origin HEAD:refs/heads/main'` then `git q` — **audit 7's destructive pair
STILL WORKS after this plan**, and it is asserted still working rather than
described. A `!` body is a whole command line handed to a governed program as
DATA, which is `T-19-86`: open at `high`, out of scope by explicit user decision,
with both its rows pinned PERMITTED in files this plan may not edit.

**Three fail-open directions, none with an automated control**: an `alias.*`
already persisted where the guard never saw the write; the `!`-bodied arm above;
and a future git that re-parses a SECOND config value as a git command line. All
three are stated in the constant's doc, the predicate's doc, the refusal helper's
doc and the plan-19-25 record, and **none is handed to a pin**. A revisit
condition plus a non-skipping version witness was added to BOTH
`INDIRECTION_SECTIONS` and `REPARSED_COMMAND_SECTIONS`; **the witness is a
SCHEDULE, not a CONTROL** — it says WHEN to look and cannot say WHAT changed —
so the residues stay uncovered. **No `AR-` row was added and no risk was
accepted.**

### `T-19-109` — the counted-completeness claim — **CLOSED**, `low`

`cred.rs`'s `hooks_path_env` limit paragraph's counted "FIVE forms" table is
replaced by a statement of the REGIONS each closure covers — the argv
leading-option region, the environment, the `git config` write operand — and what
is NOT covered, **counting nothing**. A named, narrow exception to this phase's
`cred.rs` fence, changing **ZERO non-doc lines** (verified). The "RESTORATION of
layer 3's catch" correction is recorded BESIDE the 19-22 and 19-23 subsections and
`19-23-SUMMARY.md` rather than as an edit to any of them.

### `T-19-106` — the drift pins' REACH — **CLOSED**, `medium`

Both halves done. `GH_API_VALUE_OPTS` gets its two-sided pin over all seventeen
entries against real `gh` 2.45.0, in the **ENDPOINT-LESS** probe form so no row
touches the network, with **both** negative controls and their **measured strings**
pinned rather than paraphrased (`unknown flag: --bogus-opt`; `accepts 1 arg(s),
received 0`, a POSITIONAL error). It does not skip when `gh` is absent.
**`--hostname` is KEPT in `FORGE_VALUE_OPTS`** with its measured reason recorded —
removal is an under-counting regression (`T-19-35`), the entry is INERT for `gh`
and load-bearing for `glab`. The `gh` half is pinned two-sided; the `glab` half
cannot be pinned on this machine and is NOT.

### `T-19-105` — the corpus could not draw the class — **CLOSED**, `medium`

Closed by `19-24`'s sixth class on the existing config-resolution axis, which is
what a corpus is for: it failed on this class before any production line moved,
and this plan's clause is the rule it certified.

### `T-19-110` — region 2's OPERAND GRAMMAR under-reads a dash-leading value — **OPEN**, `high`

**NEW. Found by `19-25` while writing region 2, and NOT fixed by it.**

`scan_config`'s walk treats any word beginning with `-` as an OPTION, so
`is_write`'s classic-form test (`key_operand_count() >= 2`) does not see a VALUE
whose first byte is `-`. Git 2.43.0 does not agree: once the KEY has been seen the
next word is the VALUE whatever its first byte is. Measured against real git with
both config pointers at an empty file:

```
git config alias.x -q         -> exit 0, alias.x=-q
git config alias.y --global   -> exit 0, alias.y=--global
git config core.hooksPath -c  -> exit 0, core.hooksPath=-c
```

Measured against the BUILT BINARY, the consequence is that **plan 19-02's by-name
`core.hooksPath` deny at region 2 is reachable past**:

```
exit 2 hook_bypass_blocked   git config core.hooksPath /dev/null
exit 2 hook_bypass_blocked   git config core.hooksPath -
exit 0                       git config core.hooksPath -c
exit 0                       git config core.hooksPath --
```

**This is NOT `T-19-108`** — it is a gap in region 2's operand grammar, not in the
re-parse question. `19-25`'s re-parse clause is not blind to it (it reads
`ConfigScan::value_word`, the WORD after the key), so the clause's own reach is
complete; **the rest of the gap is not closed.**

**Why it was not fixed here:** correcting it means widening `is_write`, which moves
verdicts for keys outside this round's class with **no corpus able to fail on
them**. The discipline this phase exists to enforce is that a rule is written
against a corpus observed RED first, and that corpus does not exist yet. The rows
are RECORDED (never asserted) in
`tests/envelope_reparsed_value.rs::the_region_2_operand_grammar_gap_is_recorded_by_19_25_and_is_not_closed_by_it`,
beside the two spellings the deny does reach.

**The remedy shape for a future round:** write the RED corpus first — every
`git config <key> <dash-leading-value>` spelling, both the `core.hooksPath` family
and the discrimination controls — then correct the operand walk, not `is_write`'s
threshold alone.

### Unchanged and still OPEN at this plan's end

- **`T-19-86`** — OPEN at `high` by explicit user scoping decision. Its four
  registered rows and its persisted-alias arm still exit 0, its pins are green and
  UNMODIFIED. **Recorded MEASURABLY WIDER than the register credits**, including
  `T-19-108`'s unclosed `!`-bodied arm, and **audit 7's destructive pair still
  works after this plan.** Not fixed, not narrowed, not re-scoped.
- **`T-19-91`** — OPEN at `high`, arms unweakened, no decision-operand rule added
  and no denylist extended.
- **`T-19-96`** — left exactly as pinned by `19-16`.
- **`T-19-74`** — core rows frozen.
- **`T-19-84`, `T-19-85`, `T-19-61` … `T-19-73`** — open and unaccepted by explicit
  user decision. The ONE named exception taken is `cred.rs`'s `hooks_path_env`
  limit paragraph (doc only, zero non-doc lines); `advisory.rs`, `scan.rs`,
  `config.rs`, `mod.rs` and `hooks.rs` were not opened.
- **the `glab --host` forge cell** — carried forward UNFIXED. `command -v glab`
  re-run and found nothing, so the callee's grammar is unconfirmed and it is NOT
  claimed as a live bypass; a pin that skips is fail-open.
- **`policy.rs`'s stale proportional-floor comment** — documentation drift,
  recorded, not fixed; now staler, since the production half grew from 228,101 to
  261,386 bytes.
- **the `T-19-17r` bookkeeping gap** — still **OUTSTANDING**. `19-17-SUMMARY.md`
  calls it "accepted"; audits 5 through 8 and plans 19-22 through 19-24 all
  declined to make the acceptance. **This plan adds no Accepted-Risks-Log row,
  creates no `AR-19-13` (verified: count 0), and does not apply the word
  "accepted" to it**, because accepting a risk is a human decision.
- **the three documented flakes** — the two `tests/driver_reattach.rs` failures and
  the `tests/envelope_tracer.rs` ETXTBSY stub-write race. **None fired in this
  plan's gate run**; absence is not evidence they are fixed.

## Plan 19-26 — the corpus (round 10's first half)

**`/gsd-secure-phase 19` is NOT cleared by this plan, by `19-27`, or by the two
together.** `T-19-86` and `T-19-91` both remain OPEN at `high`; `T-19-111` and
`T-19-112` are open at `high`. Only the WRAPPER-OPERAND sub-class of `T-19-60` is
closed. **This plan closes NOTHING.**

### `T-19-111` — a K1 alias body delivered by a FILE the guard never sees written — OPEN, `high`

**Registered SEPARATELY from `T-19-86` and it must be moved OUT of it, not into
it.** `T-19-86`'s four registered rows all require a governed program to be handed
a governed COMMAND as data; a non-`!` alias body sitting in a config VALUE is not
that. **Filing a live, non-shell, measured destructive bypass under a threat the
user has explicitly scoped OUT is how it stops being counted.**

Measured against real git 2.43.0 with a rebuilt bare remote and a control beside
every leg: the plain force push REFUSED and `main` UNMOVED; the `printf`-written
`.git/config` alias carrying `-c include.path=<evil>` COMPLETING and MOVING `main`
`4cc4cdc → ed58fb9`; **the same alias, written the same way, carrying NO carrier
REFUSED and moving nothing** — the control that isolates the CARRIER from both the
ALIAS and the FILE WRITE. All eight write spellings are exit 0 with an EMPTY walk,
as is `git fp`.

**ITS CORPUS ROWS ARE RECORDED RATHER THAN RED, AND THE REASON IS STATED:**
`19-27` writes **no rule** for the repo-side carriers, so an assertion in either
direction is wrong. A row asserted REFUSED lands permanently red in a file `19-27`
may not edit; a row asserted PERMITTED pins a live bypass as correct. **`19-22`
asserted such a row against its own comment, its own SUMMARY and its own
plan-check, and it halted `19-23` mid-plan.** Verified mechanically by
`no_repo_side_row_is_asserted_and_this_file_says_so_mechanically`, which reads
this file's own source and fails if an `assert!` appears in the repo-side block.

**No acceptance is made — that is a human decision.**

### The `T-19-111` ATTRIBUTION CORRECTION — required REGARDLESS of whether the fix lands

Audit 9: *"the honest first step is the ATTRIBUTION, not the rule"*. Five sites,
by file and line:

1. `src/envelope/policy.rs:1177-1183` — discloses the residue (including *"or one
   written by a means that is not `git config`"*) and then calls it *"`T-19-86`'s
   shape"*. **`19-27`'s.**
2. `src/envelope/cred.rs:287-289` — narrows it to *"predating the run"*, dropping
   the written-during-the-run half. **The half must be RESTORED. `19-27`'s.**
3. `19-SECURITY.md:7430` — inside the plan-19-25 record. **Corrected BESIDE, in
   this plan's own appended subsection, never by editing it.**
4. `19-25-SUMMARY.md:265` — same. **Corrected BESIDE, never by editing it.**
5. `deferred-items.md:1619` — the `T-19-108` closure entry, whose *"CLOSED AS
   SCOPED"* names the excluded BODY (`!`) and not the excluded CARRIER (a file).
   **Corrected here, beside it: the exclusion is the CARRIER as well as the body,
   and `T-19-108`'s closure does not cover a `.git/config` written by a means the
   guard never watched.** The entry at `:1619` is NOT edited.

### `T-19-112` — the SAFE-06 PR-cap ledger as a FILE — OPEN, `high`, corpus RED

`<envelope>/<alias>/pr-ledger.ndjson` (`ledger.rs:47`, `:95-97`). Five write
spellings measured at exit 0 with an EMPTY walk (`rm -f`, `truncate -s 0`,
`cp /dev/null`, `shred -u`, and `rm -rf <ENV>/alpha`, which takes NINE carriers in
one call). **Driven END TO END in ONE PERSISTENT root over four calls**: permitted
(1 line), refused at `pr_cap_exceeded` (2), refused (3), deletion at exit 0, **call
4 permitted again with a fresh one-line ledger.**

**It falsifies an affirmative `Guaranteed` claim.** `SECTION_ENVELOPE` lists the
cap under **Guaranteed** (`advisory.rs:250`), and `hooks.rs:1421-1428`, `T-19-35`
and `AR-19-05` all say SAFE-06 has **no second carrier** — its remaining
mitigations are the argv delivery and `write_settings_in`'s round-trip check,
**neither of which observes the ledger file at all.** The guard is the only
observation point and its record is deletable by the process it observes.

Corpus **RED** at this plan's end: nine assertions in
`tests/envelope_control_carrier.rs` plus the generative fail-closed arm in
`tests/envelope_wrapper_class.rs`.

### `T-19-113` — the hook stubs as FILES — OPEN, `medium`, corpus RED

`<envelope>/<alias>/hooks/{pre-push,pre-commit}` (`hooks.rs:127-149`).
`cp /bin/true <hooks>/pre-push` is exit 0, and **driven end to end** it removed
layer 3 and let a force push MOVE a bare remote's `main` `8ef079d → f2d230f`
**with `GIT_CONFIG_COUNT` UNTOUCHED**; restoring the stub restored the refusal.
Carried at audit 9's `medium` and **not re-rated**: `SECTION_ENVELOPE` discloses
that client-side hooks are defeatable and the trust-boundary table declares the
filesystem write.

**D-09's NARRATIVE DEFECT, registered separately and its correction required even
if the control is deferred.** The ceiling paragraph (`cred.rs:241-244`,
`mod.rs:167`, `advisory.rs:254`) names the route the guard REFUSES — `unset
GIT_CONFIG_COUNT`, re-measured green at exit 2 `hook_bypass_blocked` — **while the
route that works is refused by nothing**, so a reader concludes the guard stands
underneath layer 3 and it does not. **`19-27`'s.**

**Option (b) — tamper-evidence — is structurally UNAVAILABLE here**, and that is
this round's sharpest design asymmetry: the thing that would detect a REPLACED
stub is the binary the replacement removed from the path. Prevention is the only
control that can fire.

### `T-19-114` — every generative alphabet on all four existing axes draws a governed COMMAND LINE — OPEN, `low`, corpus REPAIRED

`T-19-76`'s failure mode for the **TENTH consecutive round**, after `T-19-83`,
`T-19-89`, `T-19-95`, `T-19-99`, `T-19-101` and `T-19-105` — **and the FIRST in
which the gap was a different KIND of carrier rather than a wider alphabet of the
same one.** Re-verified mechanically at `406c48e`: `grep -c 'pr-ledger'` is **0**
in `tests/envelope_wrapper_class.rs` and **0** in
`tests/envelope_command_position.rs`, and the single hit in
`tests/envelope_reparsed_value.rs:580` is a COMMENT about the positive walk
control. `grep -c 'hooks/pre-push\|\.git/config\|askpass\|settings\.json'` over all
three is **0 / 0 / 0**.

Repaired by a **FIFTH named axis** `CONTROL_CARRIER_CLASSES` with
`MIN_CONTROL_CARRIER_CLASSES = 7`, seven degenerate-proof classes, an
`ENVELOPE_ROOT_OPERAND_CARRIERS` alphabet with its OWN fail-closed property, six
verdict-preserving alphabets, a NO-PROGRAM-NAMES fence, a
PATH-PREFIX-NOT-BASENAME fence, extended disjointness and permitted-base fences,
and floors whose arithmetic is derived, stated and checked as exact equalities.
**Closed only by `19-27`.**

### The FIVE PLANNER-DERIVED carriers — registered at the severity their MEASUREMENT supports

- **`C-05`, `<env>/<alias>/gitconfig`** (`cred.rs:81`) — **measured against real
  git**: a non-`!` `[alias]` body carrying `-c include.path=<f>`, written into the
  file, resolves `/INCLUDE_WINS` against the control's `/ENV_WINS` — **`T-19-111`'s
  K1 body at the GLOBAL level, never touching `.git/config`** — and
  `[credential] helper = …` reads back through `git config --get
  credential.helper`, **falsifying `SECTION_ENVELOPE`'s FIRST `Guaranteed`
  clause**. **ALREADY ACCEPTED as `AR-19-04`**; see the reasoning gap below.
- **`C-06`, `<env>/<alias>/askpass`** (`cred.rs:49`) — *reading* it is already
  disclosed; **overwriting it was unmeasured before this round** and is measured
  permitted here. **The behavioural consequence of a substituted responder is
  UNOBSERVED and is not inferred.**
- **`C-07`, `<env>/<alias>/gh/`** (`cred.rs:83-91`) — **measured against `gh`
  2.45.0, offline**: empty directory → *"not logged into any GitHub hosts"*;
  `hosts.yml` written → a KNOWN host with *"Active account: true"*, token read from
  the written file. **UNOBSERVED, deliberately: whether a VALID token would be
  honoured — a test may not supply one.**
- **`C-08`, `<env>/<alias>/settings.json`** — **MECHANICAL half settled, HARM
  UNMEASURED.** `grep -rn 'settings_json' src/` returns exactly TWO hits:
  `hooks.rs:1467` (the definition) and `hooks.rs:1419` (the D-07 doc row CLAIMING
  it). Production pushes `--settings` with a **PATH**. **If the harm holds, the
  `PreToolUse` guard REGISTRATION — layer 2's entire delivery — is single-carried
  by a file inside the directory `T-19-112` proved writable at exit 0.** Whether
  the already-spawned CLI re-reads its settings mid-run is a property of a
  closed-source binary and **cannot be driven without a live session and a network
  call.** **Registered at the severity the mechanical half alone supports, with the
  harm stated as UNCONFIRMED. Claims in this codebase about a second carrier have
  been wrong twice; nothing is asserted in either direction.**
- **`C-12`, `<project>/.claude/settings.json`** — layer 1 denies `Write(.claude/**)`
  and `Edit(.claude/**)` as **TOOL patterns** (`policy.rs:2063-2064`), and its own
  doc says *"A control whose carrier the agent can edit is not a control"*, **but
  the Bash route around them is denied by NEITHER layer**: layer 1's Bash patterns
  are git verbs and layer 2 permits a segment reaching no governed program.
  Measured exit 0 with an EMPTY walk. **Whether the loaded project-tier file makes
  that reachable in practice is UNMEASURED.** RECORDED, never asserted.

### `C-15` — `~/.config/gsd-meta-manager/config.json` — a DISTINCT ROUTE to SAFE-06 — OPEN, control (e), NO RULE PLANNED

Found by a plan-check, **one TIER out from the rest**: the first enumeration walked
the envelope directory exhaustively and stopped at its edge, and **the file that
FEEDS the policy is as much a carrier as the file that RECORDS it.**

`guard` hands `Config::default_path()` to `guard_in` (`hooks.rs:837-840`) →
`resolve_policy` (`hooks.rs:1239-1258`) reads it on **every Bash tool call** →
`EnvelopePolicy::resolve` (`policy.rs:2116-2139`) takes `pr_cap_per_24h` and
`pr_cap_per_run` through `unwrap_or(DEFAULT_…)` with **NO CLAMP**, while
`namespace` alone degrades through `validate_namespace` with a warn and
`credential` copies through.

**Driven end to end in ONE persistent root with the caps raised to 9999: FOUR
`gh pr create` calls, FOUR permits, FOUR ledger lines, NO DELETION ANYWHERE and NO
ENVELOPE PATH NAMED** — against a control at the default 3/1 caps where calls 2, 3
and 4 are refused at `pr_cap_exceeded`. A mid-run replacement is observed **on the
very next call**.

**So it is a DISTINCT ROUTE from `T-19-112`, not a spelling of it**, and **rule (a)
cannot reach it** — the file is not under the envelope root, and a rule that
reached it would have the guard adjudicating its own configuration directory on
every call. **Control (e): none. `19-27` writes no rule for it.**

**DO NOT CLAMP `pr_cap_*`.** That is a product decision about what a user may
configure, not a guard rule, and deciding it inside a security round would decide
it silently.

**`Config::default_path()`'s relative `.config` fallback — MEASURED and NOT
REPRODUCED.** With `HOME` and `XDG_CONFIG_HOME` both unset, `dirs` resolves the
home directory from the PASSWD DATABASE and the guard opens an ABSOLUTE path
(`openat` trace), so the relative branch at `config.rs:330-335` is UNREACHED in
this environment. `HOME` alone DOES steer the path and raised caps supplied there
DO take.

### `AR-19-04`'s reasoning gap — RECORDED, and explicitly NOT un-accepted

Its mitigation, verbatim: *"the envelope regenerates it at each run start"* —
**which does not cover a write DURING the run**, and the `C-05` measurement above
is the write. **The same shape a THIRD time**: `cred.rs:287-289`'s *"predating the
run"* and `resolve_policy`'s *"a guard that cannot read configuration confines the
run more, never less"* (`hooks.rs:1233-1238`, true for an ABSENT or UNREADABLE
config and silent about a PRESENT AND WRITABLE one). **A reassurance reasoning
about the case that is not the threat.**

**`AR-19-04` and `AR-19-05` are RECORDED and NOT un-accepted. No `AR-` row is
added, edited or renumbered. Un-accepting a risk is a human decision.**

### `SECTION_ENVELOPE`'s `Guaranteed` cap clause must CHANGE EITHER WAY

Rule (a) NARROWS `T-19-112` without CLOSING it — `rm -f <ledger>` is refused after
`19-27` and `: > <ledger>` is not (direction (i)) — **and `C-15` resets the cap
through a file outside the envelope root entirely, with no ledger write at all.**
An unqualified *"Pull-request cap: an append-only ledger this repository does not
contain."* stays falsified by measurement either way. **Shipping the rule and
leaving the claim unqualified would be `T-19-107`'s defect committed by the round
that inherited the lesson.** The repaired text must NOT present rule (a)'s four
directions as the complete residue for SAFE-06, and it must be written WITHOUT
moving the three pinned phrases out of order and WITHOUT wrapping through one
(`advisory.rs:206-217`). **`19-27`'s.**

### Rule (a)'s FOUR fail-open directions — disclosed, and handed to NO control

(i) a REDIRECTION TARGET is not an operand — real by MECHANISM (`tokenize` deletes
the operator AND its target, `policy.rs:2264-2279`), and reading `>` would re-open
a model five rounds have pinned shut; (ii) an EXPANSION-BORNE operand cannot be
resolved — refusing every non-literal operand would deny `rm $TMPDIR/x`; (iii) a
SYMLINK is not followed — following one is filesystem I/O and TOCTOU on the guard
path; (iv) a RELATIVE path is not resolved — the guard has no cwd.

**(iii) and (iv) are NARROWED by a MEASURED partial mitigation** — `ln -s <ENV>/…
/tmp/l` and `cd <ENV>/alpha` both name an envelope path as their OWN operand — and
the two-segment composite `cd <ENV>/alpha && rm -f pr-ledger.ndjson` is refused **by
SEGMENT 1**, with segment 2 still unresolvable.

**NOT ONE is scheduled, promised, or handed to a pin or a witness.** All four go
in the rule's own doc, in the record, and here. **Option (b) for the ledger is
costed and DEFERRED with no schedule**; option (b) for the stub is structurally
UNAVAILABLE.

### `T-19-110` — carried forward at `medium`

Audit 9's re-rating recorded as a MEASUREMENT: the reproducers confirmed and four
more found, **every reachable write losing the precedence contest at repo-local
and at global**, and the force push after it still refused. It clears nothing and
is not this round's scope. Untouched.

### `policy.rs`'s stale proportional-floor comment — documentation drift, scheduled for `19-27`

`policy.rs:7172`, `:7185` and `:7194` cite *228,785 bytes, 78.7%* against a
production half **re-measured at this plan's base commit as 262,229 bytes** — the
real ratio is **68.6%**. **The FLOOR ITSELF (`>= 180_000` at `:7192`) is correct
and must not move**; it is the arithmetic in the comment that has drifted. This
plan may not edit `policy.rs` at all; `19-27` carries `policy.rs` hunks anyway.

Also re-measured here: `hooks.rs`'s production half at **74,490 bytes**;
`policy.rs` stripped by `tests/envelope_wrapper_class.rs`'s `production_code` at
**72,840** of 443,076 raw; `hooks.rs` stripped at **33,460** of 99,909.
`POLICY_MIN_PRODUCTION_BYTES = 40_000` and `HOOKS_MIN_PRODUCTION_BYTES = 20_000`
unchanged, both passing with wide margin; the deep anchor `fn forbidden_repo_path`
present at `policy.rs:5141`; exactly ONE `#[cfg(test)]` sentinel per file
(`policy.rs:5163`, `hooks.rs:1573`).

### Unchanged and still OPEN at this plan's end

- **`T-19-86`** — OPEN at `high` by explicit user scoping decision. Its four
  registered rows and its persisted-alias arm still exit 0 and its pins are green
  and UNMODIFIED, re-asserted here over the same public functions. **`T-19-111` is
  moved OUT of it, not into it.** Not fixed, not narrowed, not re-scoped.
- **`T-19-91`** — OPEN at `high`, arms unweakened, no decision-operand rule added
  and no denylist extended.
- **`T-19-96`** — left exactly as pinned by `19-16`.
- **`T-19-74`** — core rows frozen.
- **`T-19-84`, `T-19-85`, `T-19-61` … `T-19-73`** — open and unaccepted by explicit
  user decision. **No `src/` file was opened at all by this plan.**
- **the `glab --host` forge cell** — carried forward UNFIXED. `glab` is confirmed
  NOT installed, so the callee's grammar is unconfirmed and it is NOT claimed as a
  live bypass. **`--hostname` STAYS in `FORGE_VALUE_OPTS`**: audit 9 re-measured
  and overturned audit 8's own suggestion — `glab --hostname …` leaves ONE ledger
  line and `glab --host …` leaves ZERO, so removal is a REGRESSION in the
  under-counting direction (`T-19-35`).
- **the `T-19-17r` bookkeeping gap** — still **OUTSTANDING**. `19-17-SUMMARY.md`
  calls it "accepted"; audits 5 through 9 and plans 19-22 through 19-25 all
  declined to make the acceptance. **This plan adds no Accepted-Risks-Log row,
  creates no `AR-19-13` (verified: count 0), and does not apply the word
  "accepted" to it**, because accepting a risk is a human decision. **Six agents
  have now deliberately left it unmade; this plan is the seventh.**
- **the three documented flakes** — the two `tests/driver_reattach.rs` failures and
  the `tests/envelope_tracer.rs` ETXTBSY stub-write race. **None fired in this
  plan's gate run**; absence is not evidence they are fixed.
