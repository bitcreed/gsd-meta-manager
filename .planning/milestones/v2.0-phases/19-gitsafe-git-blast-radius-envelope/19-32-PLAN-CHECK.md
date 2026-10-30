# 19-32 / 19-33 plan-check — round 13

**Verdict: ISSUES FOUND — 1 blocker, 2 warnings.** Every substantive claim in checks 1–6 verified
against source and by arithmetic; the blocker is a two-plan seam, not a design error.

## Recurring pattern

**Pattern 3 — the undischargeable two-plan seam — fires for the NINTH time**, and this round it is
self-inflicted rather than inherited. `19-33` Task 3 is declared SEVERABLE in four places
(frontmatter truth, prohibition, action, done) on the ground that `T-19-124` is `medium` and
non-blocking. **But `19-32` commits the linearity RATIO as a live `#[test]`, and no severance path
retires it.** Patterns 1, 2 and 4 did not fire.

## Blocker

- **B1 — the severance path is unreachable; `19-33` deadlocks four commits in.** `19-32` Task 1
  commits the ratio assertion (`t(8000)/t(2000) < 8`, RED at ≈14) as a `#[test]` in
  `tests/envelope_word_set.rs`. `19-33` Task 3's own verify requires
  `cargo test --test envelope_word_set` to **pass** (`"FAIL: the evidence file is RED after the work
  bound"`, 19-33-PLAN.md:612), and Task 5's gate requires `cargo test --no-fail-fast` to exit 0
  unconditionally (`:836`) with `done` stating **"reports ZERO failures"** (`:838`). Under a
  severance both are unsatisfiable: the ratio test stays red forever. The verification block
  qualifies only the *ceiling* clause with "or Task 3 was SEVERED" (`:918`), never the zero-failure
  clause. **Fix:** `19-32` must commit the ratio row in a form a severance can retire honestly — a
  named `#[ignore]` with the measured number in its message, or `record_only` with `19-33`
  promoting it on landing — and `19-33`'s Task 3 and Task 5 gates must each carry an explicit
  severance branch tolerating exactly that one named failure. Without it the escape hatch is
  decorative and the executor discovers it after four commits.

## Warnings

- **W1 — `19-33`'s entire final gate depends on a commit-message prefix `19-32` never mandates.**
  `BASE=$(git log --format=%H -n 1 --grep='^docs(19-32)')` (`19-33:836`); `19-32`'s three commit
  instructions (`:720`, `:898`, `:1065`) say only "commit as its own commit". Twelve rounds of
  precedent make `docs(19-32)` likely, not contractual. Mandate the literal subject line in
  `19-32` Task 3.
- **W2 — `19-32` Task 1's RED gate is `grep -q 'test result: FAILED'`, which any failing test in
  the file satisfies.** It cannot distinguish "RED on the derived post-fix rows" from "RED because
  a fixture broke". The `done` field demands the failing names verbatim, so the discipline exists
  in prose but not in the automated check. Assert the count or the names.

## What was verified and holds

1. **Axis 1 is complete.** Every word-discarding site in `tokenize` enumerated: the IO_NUMBER
   digits-only `text.clear()` (`policy.rs:3548`, can carry no path), `is_fd_allocation_prefix` →
   `redirection_unresolvable` (fails closed), `skip_redirection_target` returning `None` (fails
   closed), and the target push gated on `pathname_target && target.literal` (`:3581`). With the
   third class added, the only residue is a non-literal target — already the declared axis-2
   silence — and an incomplete production, which fails closed. No probe found a fourth route.
2. **The ancestor clause is right and stops in the right place.** `ledger_path_in` refuses a
   non-plain-component alias, so `{ancestors of <root>/<alias> at or under <root>}` = `{<root>}`
   exactly. `word_is_within`'s `word.len() < dir.len()` early return (`:5706`) is the mechanism, as
   claimed. `GSD_MM_ENVELOPE_ROOT` is user-settable, so the ancestor-vs-widened-prefix distinction
   is a reachable configuration, not a thought experiment. Newly refused ordinary command found:
   `ls <root>` itself (and `ls /tmp` under a user-set `GSD_MM_ENVELOPE_ROOT=/tmp`) — bounded to one
   word, disclosed in the plan, not an outage.
3. **`T-19-124`'s arithmetic checks.** Byte floor at ~18 bytes over `/a/a/…`×8000: candidates with
   `2*(8000-i) < 18` → i > 7991 → 8–9 of 8 000. "Nine of eight thousand" holds; the floor prunes
   the cheap end of a quadratic. The ratio pin is non-vacuous: process-spawn overhead is ~10 ms
   against a 390 ms denominator, so it cannot deflate 15 to under 8. Linearity is the right property.
4. **The fenced enumeration holds, and measurement really did choose the design.** Design (A) turns
   `only_a_literal_pathname_target_reaches_the_segment_and_the_tokens_do_not_move`
   (`policy.rs:10693`, five `targets_of(...).is_empty()` rows incl. `ls <<<here`, `ls >&2`, `ls <&0`)
   AND the twelve-operator grammar pin (`:10586`/`:10603`, `!operator.pathname_target` for the five)
   red. Design (B) touches neither. **Exactly two rows flip for `T-19-123`** — `policy.rs:10023` and
   `tests/envelope_interior_path.rs:2341`; the third `"/tmp/envroot"` hit (`:2283`) is a containment
   input, not a verdict pin, and the binary's-parent row (`:2348`) does not move. Confirmed by grep.
5. **Class 2's extension vacates no cell, and both collisions are correctly diagnosed.**
   `REDIRECTION_OPERATORS` contains `<<<` (`:3795`) and `draws_a_redirection_target_carrier`
   (`:8003`) asks only `is_redirection_target && starts_with(ROOT)` — the here-string collision is
   real. `CONTROL_CARRIER_REDIRECTION_TARGETS`'s four entries (`:8363-8366`) use only `>` and `>>`,
   both pathname operators, so requiring a pathname operator drops nothing.
   `REPRESENTATIVE_ENVELOPE_ROOT = "/tmp/gsd-envelope-root"` is the ROOT, so a word that IS the root
   satisfies class 1 — the class-1 collision is real too. Entry 7 at `:8450` is as described.
6. **The corpus can fail on this round's class.** The RED list is derived and specific; both new
   alphabets are fail-closed; `MIN_CONTROL_CARRIER_CLASSES` moves 12 → 14 with an exact-equality
   assertion at `:8949`. `SEPARATORS` is at `policy.rs:2422` as claimed.
7. **Verify steps can fail.** All five blocks are runnable and discriminating — no print-only block
   this round (last round found seven). See W2 for the one weak assertion.
8. **Gate arithmetic.** 18 `envelope_*` binaries today → 19 with `envelope_word_set.rs`; both gates
   assert `-eq 19`. Baseline 1871 on `passed + failed` under `--no-fail-fast` is carried correctly,
   and the RTK `rtk proxy` discipline is applied to every `grep` over cargo output.
9. **Scope.** `19-32` mandates zero `src/` hunks and zero `tests/` deletions with mechanical gates on
   all three commits; `T-19-86`, `T-19-91`, `T-19-111`, `T-19-112`, `T-19-115`, `T-19-116`,
   `T-19-121`, `T-19-96`, `T-19-110`, `T-19-74`, `T-19-61…73/84/85`, `C-08`, `C-11…C-15`,
   `FORGE_VALUE_OPTS`, the `pr_cap_*` clamp, `T-19-23` and `AR-19-04/05` are each held by an explicit
   prohibition. **`T-19-17r` stays OUTSTANDING with no `AR-19-13`**, gated by `grep -cE` in both
   plans. The alias-body credential route is recorded as `T-19-86`'s and not folded.
10. **The finish-line prohibition exists.** `19-33` frontmatter prohibition (`:83`): *"MUST NOT
    write audit 12's finish-line sentence"*, restated at `:51`, `:296`, `:740-743`, `:838`, `:979`,
    `:987`. `19-32` mirrors it at `:25` and `:402`.

**Scope note (advisory, not a finding):** `19-32` at 3 tasks / 132 k tokens and `19-33` at 5 tasks /
158 k both sit far above the 2–3 task, ~50 % budget target. That is this phase's established shape
across thirteen rounds and the tasks are genuinely sequential; noted, not counted against.
