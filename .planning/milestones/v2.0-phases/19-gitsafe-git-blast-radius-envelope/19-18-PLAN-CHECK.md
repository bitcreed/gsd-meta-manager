# 19-18 / 19-19 PLAN CHECK

**Verdict: BLOCK** — 3 blockers, 2 warnings. All are seam defects between the two plans, not defects
in the deletion model itself. The model, the `&>` mechanism, the `x2>` control, the negative-cost
claim and the splice position all verify.

## Verified (measured at `af6205c`, fresh envelope root per row)

```
exit=2  git push origin refs/heads/gsd-auto/alpha/w > log.txt      exit=0  twin
exit=2  git push \<NL> origin refs/heads/gsd-auto/alpha/w          exit=0  twin   (push_outside_namespace)
exit=0  git >/dev/null push --force origin main      exit=2  git push --force origin main
exit=0  git &>/tmp/o push --force origin main        exit=0  git x2>/tmp/o push --force origin main
exit=0  git <<EOF push --force origin main           exit=0  git log --grep=">"
exit=2  >/dev/null git push …   exit=2  git push … >/dev/null   exit=2  git >$F push …
exit=0  git >    exit=0  ls >
```
Both pre-existing false refusals are real and their removal cannot mask a genuine refusal (the twin is
permitted, so the post-fix verdict is the twin's, already asserted). The splice position is correct:
prefix and trailing are already refused, so only the between-program-and-decision-words position can go
red. `&` is standalone in `SEPARATORS` and the `&&` consumption sits in the same arm, so recognising
`&>`/`&>>` there is reachable without touching `SEPARATORS`; `!is_separator(">")` is intact. Deletion
coverage is complete over bash: the other argv-deleting constructs are the `#` comment arm (already
matches bash — `git #x push --force` yields `[git]` on both sides) and null unquoted expansion
(`git $EMPTY push …`, fail-closed on round 4's literalness bit). No relocating construct displaces a
decision word. `x2>` is permitted for the right reason (digits-only IO_NUMBER, non-digit prefix
FLUSHED), and no over-deletion path was found.

## BLOCKERS

1. **`{v}>/tmp/o` is in `DISPLACING_REDIRECTIONS` but is NOT verdict-preserving, and the property
   asserts preservation over permitted bases.** 19-18 Task 2 lists it as a minimum alphabet entry and
   asserts "verdict UNCHANGED" for permitted bases; 19-19 marks a `{name}` fd prefix UNRESOLVABLE, so
   `git {v}>/tmp/o status` refuses. `tests/envelope_wrapper_class.rs` is not in 19-19's
   `files_modified` and 19-19 forbids editing it — the property is permanently red. This is the
   round's own "one slot over" pattern inside the corpus. **Fix:** move `{v}>` out of the
   verdict-preserving alphabet into a fail-closed alphabet of its own (or exclude it from the
   permitted-base arm) and say so in the doc.

2. **The two pre-existing FALSE REFUSALS are pinned as assertions in 19-18 and 19-19 is forbidden to
   move them.** 19-18 asserts exit 2 `push_outside_namespace`; 19-19 Task 2 requires both to move to
   exit 0, while its prohibition allows only ADDITIONS to `tests/envelope_argv_deletion.rs`, its DONE
   requires `git diff --numstat` over `tests/` to show ZERO deletions, and its gate requires
   `--test envelope_argv_deletion` fully green. Unsatisfiable as written. **Fix:** name those two
   `#[test]` fns explicitly in both plans as PRE-FIX pins that 19-19 is required to replace, and narrow
   19-19's zero-deletion check to "no deletions outside the two named pre-fix fns".

3. **Refused rows whose post-fix REASON IDENTIFIER 19-18 cannot derive are not exempted the way the
   `>$F` pair is.** 19-18 requires every refused row to assert "a named reason identifier". Under
   19-19, `&>`, `>|`, `<>`, `<<EOF`, `<input.txt`, `<<<x` land on `force_push_blocked` but `{v}>` lands
   on `envelope_assertion_failed` (unresolvable) — a 19-19 design outcome 19-18 cannot know. If 19-18
   guesses wrong the row is red and uneditable. **Fix:** add `{v}>` to 19-18's explicit
   "post-fix verdict left for 19-19" list, and state in 19-18 that the two `>$F`/`>*.log` rows are
   RECORDED IN COMMENTS, not asserted (currently ambiguous — the DONE clause lists them as present
   in the file).

## WARNINGS

4. `&>>` and `<<-` are in 19-19's twelve-operator production but appear in neither `DELETION_CLASSES`
   class 3's list nor `DISPLACING_REDIRECTIONS`. The corpus cannot fail on the two spellings the rule
   uniquely adds. Add both as alphabet entries.

5. 19-18's stated mechanism for the `\`+newline false refusal ("the refspec reads `\norigin`") does not
   match the measurement: the guard reports refspec `origin` resolving to `refs/heads/origin`. Verdict
   and identifier are right; only the written derivation is wrong. 19-18's measure-first prohibition
   will surface it, but correct the text so the comment is not written from the plan.

## Checked and clean
Gate arithmetic satisfiable in both plans (19-18: a whole new file of `#[test]` fns; 19-19: ≥2 mandated
new fns in `policy.rs` plus appended rows — `passed + failed > ` prior total is reachable, unlike
round 3). Baseline 1584 / 0 / 13 with the `driver_reattach` pair excluded is used consistently and
`--no-fail-fast` + `rtk proxy` are mandated. `T-19-17r` recorded OUTSTANDING with `AR-19-13` and the
word "accepted" prohibited in both plans. `T-19-86`, `T-19-91`, `T-19-96`, `T-19-74`,
`T-19-61…T-19-73`, `T-19-84`, `T-19-85` untouched; both plans state `/gsd-secure-phase 19` is not
cleared and forbid an unqualified "T-19-60 is closed". 19-18 has zero `src/` in `files_modified` and a
`git show --stat` check per commit; audit-table fence intact in both. Audit 5's `sh {-c,"…"}` corpus
limit is genuinely addressed by 19-19's unit assertion, correctly deferred out of 19-18. No plan is
independently unexecutable apart from blockers 2 and 3.
