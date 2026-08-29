# Plan check — 19-16 / 19-17 (round 5, the inversion) — RE-CHECK at 6701769

**Verdict: BLOCK, on one surviving blocker.** Blockers 2 and 3 and both warnings from the first pass
are properly closed. Blocker 1 was closed for a word carrying ONE brace expansion; a word carrying
**two adjacent expansions** still produces `git` and is still permitted under the revised rule — and
the six-class floor still cannot draw it.

## Finding 1 — BLOCKER. The product computation is per-`{`, and bash composes across `{`s

Three live cells, all measured at **exit 0** against the built binary today, all of which bash
assembles into a real force push:

```
{g..g}{i..i}t push --force origin main     bash: [git]   guard: exit 0
{g,g}{i,i}{t,t} push --force origin main   bash: [git]…  guard: exit 0
{g..g..1}it push --force origin main       bash: [git]   guard: exit 0
```

Trace the first through the revised spec (19-17 objective `:191-196`, Task 1 edit 2 `:310-322`).
Products are computed per-`{` from "the word's literal prefix (the text in progress when the `{`
arrived)" × alternatives/range members × "the literal suffix it reads on to **the next unquoted
whitespace or operator**". `{` and `}` are in `SEPARATORS` — they *are* operators — so:

- first `{`: prefix empty; suffix stops immediately at the following `{` ⇒ empty; product `g`;
- second `{`: the intervening `}` already flushed the word, so the prefix is empty; suffix `t`;
  product `it`.

Neither product is governed, both sets enumerate cleanly, so 2(b) clears the word and 2(a) never
fires — the head word is still `it`. The escape hatch the plan intends here, "a nested `{` in the
prefix or suffix makes them unenumerable", **can never fire as written**, because the suffix scan
terminates at the operator `{` before it can contain one. The two rules contradict each other and the
default reading is the permissive one.

`{g..g..1}it` is a second, separate gap: a three-part range with an increment is neither "each member
of a `..` range" as stated nor any listed unenumerable trigger.

**Required, both:**
1. Products must be composed over the WHOLE word, not per-`{` — the cartesian product across every
   brace expansion in one whitespace-delimited word. Minimally and equivalently safe: state that a
   prefix or suffix run that is **terminated by `{` or `}` rather than by whitespace or a real command
   operator makes the product set unenumerable ⇒ refuse**, and say so where the suffix scan is
   defined, not only in the unenumerable list.
2. Name the range shapes: a `..` range that is not exactly two parseable endpoints (an increment, a
   malformed range) is unenumerable ⇒ refuse.

## Finding 2 — BLOCKER (same defect, corpus side). No class and no entry draws a MULTI-expansion word

The six classes are whole-word splice, concatenated splice, range, literal pair, glob, tilde
(19-16 `:462-472`). Every new `REFUSED_BASES` entry carries exactly one brace pair
(`{g..g}it …`, `g{i,i}t …`, prefix/suffix-only, the forge twin). Nothing in any alphabet has two
brace pairs in one word, and no predicate requires one — so the corpus is again structurally
incapable of failing on the cell that just walked through. **Required:** a seventh class,
MULTI-EXPANSION (predicate: two or more `{`…`}` expansion pairs within one whitespace-delimited
word), with `{g..g}{i..i}t push --force origin main` and `{g,g}{i,i}{t,t} push --force origin main`
in `REFUSED_BASES`, plus `{g..g..1}it push --force origin main` under the range class.

## Finding 3 — WARNING. `19-17`'s diff gate contradicts its own `files_modified`

`tests/envelope_literal_decision.rs` is now in `files_modified` (`:12`) for the two deferred
additions, but verification `:643-644` still asserts the diff touches "no file under `tests/`". One
of the two must move; as written the executor's own gate is unsatisfiable or ignored.

## What the revision closed, verified

- **Blocker 3 — cost rows.** Fixed generally, not just locally. The derivation prohibition
  (19-16 `:50`) states the right reason — this plan measures PRE-fix so its measure-first discipline
  cannot catch a wrong POST-fix expectation — and requires a named clause (1 / 2a / 2b) beside every
  pinned expectation, with an undevisable row deferred to `19-17` to measure and pin. It is
  enforceable, not prose: the `<done>` at `:433` names both deferred rows, `19-17`'s test-edit
  prohibition (`:61`) narrows the exception to exactly those two as NEW `#[test]` fns, and `19-17`
  Task 2 edit 5 measures and pins them. The new-cost prohibition (`:52`) closes the second half —
  a shape already refused for another reason is pinned as an *interaction* under `19-15`'s pattern
  rather than counted as a cost. The `FOO={a,b}` Rule B trace is the right call and is deferred, not
  guessed.
- **Warnings.** `echo {git,x}` / `echo git` is now a disclosed 2(b) cost. Gate arithmetic is now an
  identity rather than an assertion: red→green leaves `passed + failed` unchanged, the only increase
  is new `#[test]` fns, `19-17` requires the brace table to be at least one new fn (`:348-354`), and
  Task 3 checks `expected = 19-16's total + new fns counted from git show` with disagreement declared
  a finding. Both flake modes: `passed + failed` is mode-independent, so the identity holds in each.
- **`ls {git,svn}-repo` is permitted for the right reason** — products `git-repo`/`svn-repo`, neither
  basename governed — and is pinned as the control that tells a product test from a mention test. I
  probed for an under-refusal the narrowing opened and found none that matters: an alternative that
  merely *names* a governed program without producing one (`{git,x}/foo push`) runs `git/foo`, not
  `git`.
- **Nothing previously verified was disturbed.** The literalness trigger set, `tokenize` as sole
  `Token` producer with `at()` (`policy.rs:2174-2181`) the single read site, `T-19-93`'s
  ledger-line + `pr_cap_exceeded` + `…/pulls/7` bar, the `split_segments_with_heads` mechanism pin,
  the claimed-unaffected list (re-measured, all exit 0), the `T-19-86`/`T-19-91`/`T-19-96` scope
  boundaries, the append-only `19-SECURITY.md` rule and the zero-`src/` seam are all intact. The
  product computation is still inside the ONE lookahead — no second scan was introduced.

Measurement note: all guard verdicts above were driven against `./target/debug/gsd-meta-manager` with
JSON built by `python3 -c json.dumps` (a shell-quoted heredoc mangles the payload and reports false
refusals), one fresh `GSD_MM_ENVELOPE_ROOT` per row.
