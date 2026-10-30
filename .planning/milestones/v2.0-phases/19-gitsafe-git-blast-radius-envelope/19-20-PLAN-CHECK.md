# 19-20 / 19-21 plan check — round 7 (the callee's grammar)

**Verdict: PASS WITH CHANGES.** Five findings, all WARNING, all fixable in plan text.
No blocker. The fail-closed rule covers the class; the seam claim holds; the corpus
is capable of failing on T-19-100 pre-fix; the gate arithmetic is satisfiable in
every flake mode.

## Recurring patterns

- **One slot over — in the prose, not the rule.** The drift pin's stated limit
  ("staleness costs refusals rather than bypasses") is correct for the *silence*
  of the constants and wrong for their *entries*. See finding 3. The rule itself
  is not one slot over.
- **Seam unsatisfiability — not reproduced, but one latent entry.** Three rounds
  of handoff defects are genuinely corrected here (post-fix verdicts asserted,
  three underivable rows recorded not asserted, unknown alphabet split out). One
  alphabet entry could still reintroduce it. See finding 4.
- **Corpus incapable of failing — not reproduced.** Nine rows measured exit 0
  today are asserted exit 2; the property is red pre-fix by construction.

## Findings

1. **WARNING — the ratio headroom is measured in chars, not bytes.** Rust's
   `code.len()`/`raw.len()` are byte lengths. Re-measured at `ec4c700`:
   `policy.rs` raw 263,360 / stripped 65,947 = **25.0406%**, headroom **107
   stripped bytes (~428 comment bytes)**, not 246/984; `hooks.rs` 99,909 /
   33,460 = 33.4905%. Fix the figure in 19-20 must_haves truth 11, Task 2's
   derivation block and the 19-SECURITY record, and derive
   `POLICY_MIN_PRODUCTION_BYTES`/`HOOKS_MIN_PRODUCTION_BYTES` from 65,947 /
   33,460. Conclusion (replace the ratio) is unaffected and strengthened.

2. **WARNING — the first recovery step is not universally available.** Confirmed
   on git 2.43.0: `git --shallow-file=/tmp/s version` → `unknown option:
   --shallow-file=/tmp/s`. The attached spelling is always *self-contained*
   (safety claim sound — no `--x=y` token consumed a following word in any
   probe), but it is not always *accepted*. Both plans state the recovery as
   "spell it attached, which needs no list change at all"; qualify it ("where git
   accepts the attached form") in the refusal message draft, 19-21 truth 8 and
   the 19-SECURITY cost section.

3. **WARNING — "staleness costs refusals rather than bypasses" is false for a
   stale entry.** Silence fails closed; an *entry* does not. A self-contained
   entry that the runtime git treats as value-taking, or a value-taking entry the
   runtime git rejects, shifts the verb — the `--super-prefix` direction, which
   the plans acknowledge for the value list but then generalise away in the pin's
   stated limit. Correct the pin doc and the record: the fail-closed default
   covers absence; both constants' *contents* can still mis-index, which is why
   the pin probes both directions.

4. **WARNING — keep bare `-` out of `GIT_GLOBAL_OPTIONS`.** Class 5 is defined as
   "`--` and a bare `-`", while `git - push --force origin main` is pinned
   PERMITTED in both plans. If an executor adds `-` as a class-5 alphabet entry,
   the generative refused-base arm asserts a refusal that the rule may never
   produce — permanently red in a file 19-21 may not edit. This is the exact
   three-round seam defect one entry over. Require the class-5 alphabet to be
   exactly `--` and assert `-` absent from it.

5. **WARNING — stale binary count in 19-21's gate.** Task 3 requires "all twelve
   `envelope_*` binaries" to have run; there are twelve today and **thirteen**
   after 19-20 adds `envelope_callee_grammar`. Say thirteen, or "every
   `envelope_*` binary".

## Verified (no change needed)

- **Verb-shifting completeness.** git 2.43.0 accepts no short-option bundling
  (`-pc`, `-pP`, `-cuser.name=x`, `-C/tmp` all `unknown option`), so post-fix
  arms (a)–(f) leave no spelling that shifts the verb without reaching *grammar
  not established*. Breaks on non-`-`, `-`, `--` are safe (real git rejects `-`
  and `""`); an option whose value looks like an option is consumed correctly;
  the scan being a loop is covered by the `-c a=b --attr-source` cell.
- **Structural rule (a) probed for a counterexample and found sound.** No
  `--opt=value` token consumed a following word; boolean `--no-pager=1` /
  `--bare=1` are rejected outright.
- **All post-fix verdicts in 19-20 are derivable from the current code**:
  `stash`, `update-ref -d`, `symbolic-ref HEAD <ref>` (2 operands) →
  `ForcePushBlocked`; `reflog delete` → `ForcePushBlocked`; `config
  core.hooksPath /tmp/x` (2 key operands → write) → `HookBypassBlocked`; the
  layer-2 row via the `-c` key check; the composition rows via round 6's
  deletion. The three underivable rows are recorded, never asserted.
- **`--super-prefix` and `--signed no` measurements reproduce exactly**:
  `unknown option: --super-prefix` both forms; `git push --dry-run --signed no
  origin refs/heads/gsd-auto/alpha/w` → `error: src refspec origin does not
  match any` / `failed to push some refs to 'no'`, while
  `--recurse-submodules on-demand` → `Everything up-to-date`. Both fixes are one
  entry each.
- **Gate arithmetic.** Baseline `passed + failed = 1610` confirmed against
  `19-19-SUMMARY.md` (1605 + 5 identity). 19-20's `> 1610` is satisfiable only via
  new `#[test]` fns and it adds many; 19-21's `= 19-20 total + new fns` is forced
  non-zero by the three derived rows plus the drift-pin fns and by the
  no-folding prohibition. Red→green leaves the total unchanged in all three flake
  modes; flakes still run and still count.
- **Scope fences intact**: `T-19-86`, `T-19-91`, `T-19-96`, `T-19-74`,
  `T-19-61…73`/`84`/`85` untouched; `T-19-17r` OUTSTANDING with no `AR-19-13`
  row (grep confirms the ID is the right one and absent); no unqualified
  "T-19-60 is closed"; both plans state they do not clear `/gsd-secure-phase 19`;
  19-20 zero `src/` hunks; audit-table fence explicit; mechanism pins for rounds
  4/5/6 carried and asserted non-vacuous; `SEPARATORS` / `is_separator(">")`
  pinned.
- **Nothing stranded.** 19-20 is executable alone (red is its declared output);
  19-21 declares the dependency, needs no replacement exception, and is
  additions-only under `tests/`.
