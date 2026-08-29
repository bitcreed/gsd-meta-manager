# Plan check — 19-16 / 19-17 (round 5, the inversion) at f365ff1

**Verdict: BLOCK on findings 1 and 2.** The inversion is real and finding 3 onward pass. But the
recurring defect is present again, one slot over, and — for the fifth consecutive round — the corpus
`19-16` writes cannot fail on it.

## Finding 1 — BLOCKER. Clause 2 is enumerative after all, and the class one slot over is live

Clause 2(b) as written tests whether **a comma-separated alternative names a governed program**. A
brace expansion CONCATENATED with literal characters produces a governed program that no alternative
spells:

```
$ bash -c 'printf "[%s]" {g..g}it push'      ->  [git][push]
$ ... envelope guard alpha  '{g..g}it push --force origin main'   ->  exit 0   (measured today)
```

Trace under the plan's own rule: `{` takes case 3, contents `g..g` is a range so the simple command
is MARKED and segmentation is unchanged; the segments are `[g..g]` and `[it, push, --force, origin,
main]`. **(a)** no segment resolves `Governed` — the head word is `it`. **(b)** there is no
comma-separated alternative at all, and even in the `g{i,i}t` spelling the alternatives are `i`,
which names nothing. So the command is permitted after `19-17` and bash force-pushes. The word-level
bit cannot reach it either: the composed word never exists as a token.

`19-16` states the right principle for the position-0 row — "a rule that also asks what the splice
can PRODUCE" — and then both plans specify the weaker alternative-names test everywhere it is
operative (19-16 truths/objective, 19-17 truth 5, objective, Task 2 edit 2, threat T-19-92).

**Required:** clause 2(b) must ask what the splice can produce — expand each marked expansion's
alternatives AND `..` range members against the literal prefix/suffix of the word they sit in, and
refuse when any produced word's basename is a governed program. State the range case explicitly;
`..` is currently named only in the classifier, never in clause 2.

## Finding 2 — BLOCKER. The corpus cannot fail on finding 1, which is the fourth-round defect again

Every splice entry `19-16` adds is a WHOLE-WORD splice (`git {push,--force} origin main`,
`{git,push,--force,origin,main}`, `git push {--force,origin} main`, plus decoy/wrapper/severed
spellings). There is no concatenated splice and no `..` range anywhere in the widened alphabets, and
the new per-class floor asks only that the corpus can draw "a brace expansion" — satisfied by
`{a,b}` without ever generating the live cell. This is `T-19-76` / `T-19-83` / `T-19-89` / `T-19-95`
for the fifth time, in the same plan that exists to end it.

**Required:** `REFUSED_BASES` (and one severed/decoy spelling) gains `{g..g}it push --force origin
main` and `g{i,i}t push --force origin main`; the per-class floor is split into *whole-word splice*,
*concatenated splice* and *range* rather than one "brace expansion" class; both cells are measured
and pinned RED in `tests/envelope_literal_decision.rs`.

## Finding 3 — BLOCKER. Two cost rows assert a refusal the rule cannot produce

`rg "git status" {src,tests}` is pinned as newly REFUSED (19-16 cost table; 19-17 truth 12 and
T-19-17r). Under clause 2 it is not: segments `[rg, "git status"]` and `[src,tests]`, nothing
resolves `Governed`, no alternative names a governed program. Measured permitted today, and it stays
permitted after. `19-16` measures rows PRE-fix, so its measure-first discipline cannot catch this —
the row lands RED and `19-17` is then forbidden to edit it, forcing a stall.

**Required:** drop the `rg` pair from the cost table in both plans, or replace it with a shape clause
2 actually reaches. (`git commit -m {a,b}`, `git add {src,tests}/x.rs`, `FOO={a,b} git status` all
trace correctly through 2a and are fine.)

## Finding 4 — WARNING. Clause 2(b)'s own over-refusal is undisclosed

2(b) refuses any ungoverned command whose brace alternatives merely mention a governed name:
`echo {git,x}`, `ls {git,svn}-repo` — both permitted today, both refused after, neither in the cost
list nor pinned beside a twin. Add them to the disclosed cost with their twins, or narrow 2(b) to
the head word of the simple command.

## Finding 5 — WARNING. `19-17`'s gate is satisfiable only if the brace table is a NEW `#[test]` fn

`passed + failed` strictly greater than `19-16`'s recorded total holds only because Task 1 adds the
brace-classification table; `19-17` changes no test file. If the executor writes the table as extra
assertions inside the existing tokenizer quoting table, the total is EQUAL and the gate is
arithmetically unsatisfiable — round 3's failure mode. State "at least one new `#[test]` fn" in the
Task 1 `<done>`.

## What passes

- **The bit's trigger set is complete, not enumerative.** Bash's word rewrites are brace, tilde,
  parameter/command/arithmetic, `$IFS` re-splitting of those, pathname, and quote removal. `$`/backtick
  cover the middle three (`$'…'`, `${…}`, `$((…))`, backtick all begin with a marked character);
  `*?[`, `~` and the brace classification cover the rest; quote removal resolves to a known literal.
  Alias and history expansion are off non-interactively; extglob and process substitution both require
  `(`, which stays a separator under Rule B's geometry. Nothing reaches a decision WORD unmarked.
  The gap is at the command level (finding 1), not in the bit.
- **Collecting in `tokenize` is sound.** `tokenize` is the sole producer of `Token`; `split_command`,
  `split_segments` and `split_segments_with_heads` all route through it, and `hooks.rs` calls only
  `split_segments_with_heads` (`:893`, and `:1026` for the `NestedPayload` re-split, which the plan
  threads). `expansion_in_decision_region`'s `at()` (`policy.rs:2174-2181`) is verified as the single
  read site of `Token.expansion`; the one-closure claim holds.
- **`T-19-93` and its bar are real.** Measured: quoted spelling exits 0 with one ledger file, unquoted
  exits 0 with an EMPTY walk — exactly as claimed. Ledger line + second creation under
  `pr_cap_exceeded` + `…/pulls/7` uncounted is unsatisfiable by refusing. Absorbing comma-free pairs
  reopens nothing Rule B closed: Rule B's severing runs off `${`(case 1) and `(`/`)`, untouched, and
  the `git reflog delete HEAD@{0}` pin catches an absorb that loses a refusal.
- **The mechanism pin is sufficient.** Folding case 1 into case 3 makes `${C}_COUNT` one word, which
  collapses the line to a single segment with `head_is_command_position == true` — the pin goes red.
  It is the only `{`-driven path `19-17` touches.
- **Over-refusal cost is defensible and legible.** Re-measured every claimed-unaffected row against
  the built binary: `git commit -m "use ${HOME} here"`, `rg "x" src/*`, `git add src/*.rs`,
  `echo {a,b}`, `mkdir -p {src,tests}`, `cp x{,.bak}`, `git log -1 HEAD@{0}`, `git reflog show
  HEAD@{0}`, `{ git status; }`, `( git status )`, `cd ~/projects` all exit 0 today and are untouched
  by the rule as specified. `git add {src,tests}/x.rs` refused is defensible — the argv read is not
  the argv that runs — and the refusal names the decision it could not establish.
- **Gate non-vacuity.** `19-16`: a run that added nothing reports exactly 1533 and fails `> 1533`,
  mode-independently. `19-17`: double-locked (total unchanged AND the RED names still failing),
  subject to finding 5.
- **Scope and seam.** `T-19-96` registered-and-prohibited is diff-checkable (absent from
  `files_modified`, region-not-widened prohibition, pinned row). `T-19-86` four rows, `T-19-91`
  open, `T-19-74` frozen, `T-19-61…73`/`84`/`85` untouched, `19-SECURITY.md` append-only, no
  unqualified "T-19-60 is closed", neither plan clears `/gsd-secure-phase 19`. `19-16` has zero
  `src/` in `files_modified`; nothing is stranded across the seam.

Method note: my first pass measured several rows through a shell heredoc that mangled the JSON and
reported false refusals; those were re-measured with escaped JSON and are reported above at their
true verdicts. The `{g..g}it` and `{git,push,…}` measurements carry no quotes and were unaffected.
