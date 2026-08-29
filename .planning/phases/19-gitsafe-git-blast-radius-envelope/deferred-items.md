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
