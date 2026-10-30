# Plan check — 19-13 (round 2 gap closure)

**Verdict: BLOCK** (3 blockers, 4 warnings). The structural fix is sound and covers every
measured row; the blockers are in the *closure claim* and in the *gate*, not in the mechanism.

## Lead finding — the corpus-vacuity pattern repeats in a new shape

`T-19-83` diagnosed round 1's alphabet as having no operand whose basename is a governed
program. This plan fixes exactly that axis — `DECOY_OPERANDS` puts `git`/`gh`/`glab` in a
**wrapper's** operand slot. It leaves untested the adjacent slot: a **governed program's own
operand naming a governed command**.

Traced against the code at `0dcc6da`, all of these are PERMITTED today and remain permitted
after every edit in this plan, because the head shortcut answers at index 0 and
`classify_git`'s denylist default arm (`policy.rs:286`) allows the verb:

```
git submodule foreach git push --force origin main
git rebase -x "git push --force origin main" HEAD~3
git bisect run sh -c "git push --force origin main"
git -c alias.p='!git push --force origin main' p
```

`grep -n "submodule\|foreach\|bisect\|rebase\|alias\." src/envelope/policy.rs 19-SECURITY.md
deferred-items.md` returns **nothing** — this shape is named nowhere in the phase.

Neither `REFUSED_BASES` nor the proposed `DECOY_OPERANDS` (wrapper prefixes only) can generate
it, and `tests/envelope_command_position.rs` has no row for it. So the plan would certify
"`T-19-60` is closed by establishing COMMAND POSITION structurally" (must_have truth 2) with a
corpus structurally incapable of failing on this member of the class — the third consecutive
round of the same failure mode.

The head shortcut is *right* and must stay (without it every commit message quoting a git
command is refused). The defect is the **claim**, not the code.

**Required:** (a) demote the closure language — `T-19-60` is closed for the *wrapper-operand*
sub-class; (b) add the four spellings above as pinned rows recording their current PERMITTED
verdict with a comment naming them as an uncovered residual; (c) name the shape in
`resolve_program`'s doc beside `T-19-74`/`T-19-75` and in the SUMMARY's "what remains
uncovered" list; (d) register it as a new threat ID in `19-SECURITY.md` for a later round.
No new production code, no scope creep into T-19-61…T-19-73.

## Blocker 2 — the anti-vacuity gate is defeated by a flake the plan calls deterministic

Measured on this host just now, `rtk proxy cargo test --no-fail-fast`:

- run A: `error: 1 target failed: --test driver_reattach`
- run B: **1472 passed / 0 failed / 13 ignored**, `driver_reattach` 3/3 ok

`deferred-items.md:23` says so outright: "Observed passing 3/3 on some runs and failing 2/3 on
others". The pair is **flaky**, not a fixed pre-existing failure. Consequences:

- `1470 passed / 2 failed` is one sample of a bimodal baseline; the constant total is **1472**.
- The plan's own anti-vacuity rule ("a post-change count equal to 1470 is the signature of
  suites that never ran") is void: a run that adds **zero** tests reports 1472 and passes the
  `> 1470` gate.
- Task 3 `<done>` requires "exactly 2 failures, both the documented `driver_reattach` pair".
  On a green-driver run there are 0 failures and the done criterion is **unsatisfiable**.

**Required:** gate on `passed + failed` strictly greater than **1472**, allow 0 *or* 2 failures
provided every failure name is in the `driver_reattach` set, and additionally require the
explicit per-binary counts for `envelope_command_position` and `envelope_wrapper_class`
(`--test <name>`), which is the only count a flake cannot forge.

## Blocker 3 — Task 3's automated verify cannot fail

`rtk proxy cargo test --no-fail-fast 2>&1 | tail -40 && rtk proxy cargo clippy …` — the
pipeline's exit status is `tail`'s, so the verify block exits 0 on a fully red suite. The
plan's whole gate lives in `<done>` prose. Add `set -o pipefail` or drop the pipe (Tasks 1 and
2 already get this right via the second `$?`-tested invocation).

## Verified sound — what I traced and confirmed

**All six measured bypass rows are covered by the fix as written:**

| measured row | closed by |
|---|---|
| `env -u git git push --force origin main` | edit 1, two candidates (idx 2, 3) behind prefix |
| `env -u git git -c core.hooksPath=/dev/null push --force …` | edit 1, same |
| `env -u gh gh pr create --title x` | edit 1, two `gh` candidates — refused before the ledger write at `hooks.rs:1080`, so no cap budget consumed |
| `env -u git timeout 5 git push --force …` | edit 1 (candidates idx 2, 5). Confirmed the `classify_git` verb arm alone sees `timeout` and would permit — the plan's second-layer framing is correct, not padding |
| `sudo -u git git push --force …` | edit 1 |
| `K=GIT_SSH_COMMAND; SSH_AUTH_SOCK=/tmp/evil env -u $K git fetch origin` | seg 1 by edit 3, seg 2 by edit 2 **and** independently by edit 5 |

**T-19-81 placement is right.** Traced: `K=GIT_SSH_COMMAND` reaches `tampers_with_envelope_env`
with key `K` (`policy.rs:1264-1268`) → `None`, then is consumed as a leading assignment and
returns `NoProgram` at `policy.rs:1408`. Step 7 is unreachable. The value check must go before
that return, exactly as planned.

**The class-level half genuinely defeats the indirect spelling.** `${K}_COMMAND` sets
`Token.expansion` — the tokenizer flags `$` anywhere in a word, inside double quotes too
(`policy.rs:1084`, `:1110`), not only a leading `$`. It sits strictly between head and the
resolved `git`, so edit 2 refuses it. Not merely apparent.

**T-19-74 stays frozen.** `env $X push --force origin main` yields **zero** candidates (`$X`
skipped as expansion, `push` ungoverned), so edit 2's precondition ("a governed candidate
resolved behind a wrapper prefix") never holds and it stays `Ungoverned`/permitted. `X=git`
alone stays `NoProgram` because edit 3 checks values against `envelope_env_key` only, never
`GOVERNED_PROGRAMS`. The asymmetry is correct and correctly documented.

**T-19-82's floors are non-vacuous and correctly sized.** `cred.rs:398-405` produces exactly two
removal entries (`SSH_AUTH_SOCK`, `SSH_AGENT_PID`), so the ">= 2 removals" floor is tight, and
`PROJECT_ROOT_ENV` is `GSD_MM_ENVELOPE_PROJECT_ROOT` (`cred.rs:60`), satisfying the
non-`GIT_`/`GH_` floor. Both deleted filters are correctly identified: `value.is_some()` hides
both removals and the name filter hides the locator.

**RED is real for every item.** `env -u git git push --force` resolves to `Governed{index:2}`
today → argv `git git push …` → verb `git` → denylist default `Allow` → exit 0. The T-19-81 row,
the decoy property and the widened drift pin are all genuinely red against the unfixed tree.
Every symbol the RED files reference exists (`REASON_HOOK_BYPASS_BLOCKED`, `ledger_path_in`,
`guard_in`), so Task 1 compiles pre-fix as required.

**The head shortcut is pinned non-vacuously.** `git commit -m "git push --force is now blocked"`
resolves at index 0 and stays permitted; the rows are green before and after, which is the
correct shape for a cost-avoidance pin — deleting the shortcut turns them red.

## Warnings

1. **The `DECOY_OPERANDS` split is the right call, but the interaction cell is
   under-specified.** Giving decoys their own refusal property rather than folding them into
   `WRAPPERS` is correct — a decoy wrapping is legitimately stricter and would break the
   invariance property's reason comparison, exactly as the file already records for the
   `GIT_CONFIG_COUNT=0` prefix. But "composed into the chain at a drawn position" does not say
   how. If implemented as `wrap(rng, &format!("{decoy} {base}"))` the decoy is always innermost
   and is never exercised *outside* an ordinary wrapper. Specify: the decoy is spliced into the
   `WRAPPERS` chain at a drawn index in `0..=depth`, and add a floor that at least one generated
   case has an ordinary wrapper on each side of the decoy.
2. **A single-candidate decoy still mis-indexes, and the residual is unpinned.**
   `env -u git $X push --force origin main` has exactly one candidate (the decoy at idx 2), so
   it returns `Governed{index:2}` and `classify_git` judges verb `$X` → `Allow`. This is inside
   the accepted `T-19-74` class, but `the_residual_begins_exactly_at_the_command_line_boundary`
   only pins the decoy-free spelling. Add a row asserting this stays permitted, so the accepted
   boundary is visible in its decoy form rather than rediscovered in round 3.
3. **Edit 3 buys an over-refusal the plan does not pin.** Any leading assignment whose value
   names an envelope key (`FOO=GIT_ASKPASS echo hi`) is now refused. By the plan's own standard
   — "a cost that is not pinned is a cost nobody can tell has grown" — it needs a row beside the
   spelling that still works.
4. **Scope.** 3 tasks / 4 files is within budget, but the 120k-token estimate at `confidence:
   low` with a mandatory three-commit RED-then-fix sequence and a full-suite gate is at the top
   of the smart zone. Advisory only; do not re-slice on this alone.

## Clean

- No scope creep into `T-19-61`…`T-19-73`, `T-19-84`, `T-19-85`; the prohibition naming `deny`'s
  `?`-before-`park_refusal` ordering is explicit and correct.
- No wrapper-name or wrapper-flag list anywhere in the five edits; the
  `wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic` control stays
  load-bearing.
- No `Cargo.toml`/`Cargo.lock` in `files_modified` (`T-19-SC` holds).
- `rtk proxy` correctly required wherever a count is read (D-34).
- Dependency frontmatter (`depends_on: [19-11, 19-12]`, wave 3) is acyclic and consistent.
