# Plan check — 19-16 / 19-17 (round 5, the inversion) — RE-CHECK 2 at 2c5afb2

**Verdict: BLOCK, bounded — one finding, two spellings, both one sentence to fix.** The whole-word
composition is correct for words made of unquoted literal runs and top-level expansions, and items
2, 3 and 4 all pass. It is not closed for two shapes bash composes that the scan's decomposition
does not: a nested expansion inside an alternative, and a quoted literal run.

## Finding — BLOCKER. Two producible `git`s the whole-word scan still clears

Both measured at **exit 0** against the built binary today; bash assembles both into a real force push.

```
{g{i,i}t,x} push --force origin main    bash: [git][git][x]   guard: exit 0
"g"{i,i}"t" push --force origin main    bash: [git][git]      guard: exit 0
{g..g}"it"  push --force origin main    bash: [git]           guard: exit 0
```

**(a) Nested expansion inside an alternative.** The top-level alternatives of `{g{i,i}t,x}` are
`g{i,i}t` and `x`; bash then expands the inner one, producing `git`. A cartesian product "across
every expansion in the word" reads naturally as the word's TOP-LEVEL expansions, so the products are
`g{i,i}t` and `x` — neither basename governed. And the unenumerable trigger list is explicitly
enumerative — unmatched brace, non-two-endpoint range, contents carrying `$`/backtick/glob/tilde, a
non-literal literal run, a count over cap — and **names no nested expansion**. Note the classifier
already treats a nested `{` as a marker of the EXPANSION case, so the sentence exists one paragraph
above; it just never reaches enumeration. Cleared, permitted.

**(b) Quoted literal runs.** `"g"{i,i}"t"` and `{g..g}"it"` are one word to bash, whose literal runs
are quoted. They pass the stated trigger — a quoted run IS literal by the Token bit's own test, which
is deliberate — so the set enumerates. But nothing in the scan's definition says the runs are
**quote-removed before being joined**, and if they are joined as raw text the product is `"g"i"t"`,
whose basename is not governed. This is the same shape as the last two rounds: the permissive
reading is the one an implementer reaches by default.

**Exact edits (both at the scan's definition, 19-17 Task 1 case 3, and mirrored in truth 6 and
19-16's reproducer group):**
1. Add to the unenumerable triggers: *"an alternative or range member that itself contains a `{` —
   the products are composed recursively or the set is unenumerable; a top-level-only enumeration
   answers `g{i,i}t` for `{g{i,i}t,x}`, which bash expands to `git`."*
2. Add to the decomposition sentence: *"literal runs are taken with quoting REMOVED, exactly as the
   tokenizer recovers a word — `"g"{i,i}"t"` decomposes to runs `g` and `t`, not `"g"` and
   `"t"`."*
3. Corpus: `{g{i,i}t,x} push --force origin main` and `"g"{i,i}"t" push --force origin main` into
   `REFUSED_BASES`, the enumerated reproducers and the brace-classification table's products column,
   under the MULTI-EXPANSION class (the first satisfies it; give the quoted one the concatenated
   class).

## The other three, verified — no further action

- **2. The sentences now agree.** The per-`{` reading is gone at every operative site: the extent is
  stated at the definition (19-17 `:326-342`), repeated in truth 6, the objective `:198-207`, T-19-92
  and success criterion 5, and the directive forbidding a `{`/`}` to terminate a run is present. I
  grepped for the old "prefix, joined to … suffix up to the next operator" phrasing in both plans and
  found no survivor; every remaining "per-`{`" occurrence is naming the wrong reading in order to
  forbid it. The contradiction defect is closed.
- **3. The MULTI-EXPANSION predicate is degenerate-proof as literally written.** "Two or more
  top-level `{`…`}` EXPANSION pairs within one whitespace-delimited word": `{a,b}` cannot satisfy it,
  `{a,b} {c,d}` cannot (two words), `{a,b}{x}` cannot (the second pair is literal, not an expansion),
  and prohibition `:48` additionally bans satisfying the splice floors with whole-word or
  single-brace entries. The three rows are genuinely in all three claimed places: `REFUSED_BASES`
  (19-16 `:497-499`), the enumerated reproducers (`:279-283`) and the classification table
  (19-17 `:387-397`), the last asserting products **per WORD** so a per-`{` implementation turns it
  red rather than merely failing a verdict.
- **4. The additions-only assertion is checkable and agrees with the arithmetic.** `git diff
  --numstat` zero deletions on `tests/envelope_literal_decision.rs`, added hunks exactly the two
  deferred cost rows as new `#[test]` fns, every other `tests/` file byte-identical — all mechanical.
  It composes with the gate identity: those two fns plus the ≥1 brace-table fn are the only additions,
  so `expected = 19-16's total + new #[test] fns` is countable from `git show` and holds in both
  flake modes, red→green contributing nothing to the total.

Measurement note: guard verdicts driven against `./target/debug/gsd-meta-manager`, JSON built with
`python3 -c json.dumps`, one fresh `GSD_MM_ENVELOPE_ROOT` per row; bash products from
`printf '[%s]'`.
