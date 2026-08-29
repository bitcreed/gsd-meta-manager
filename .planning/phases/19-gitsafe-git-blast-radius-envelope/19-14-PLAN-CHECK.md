# Plan check — 19-14 / 19-15 split at e1bf0fa (focused re-check, 4 items)

**Verdict: BLOCK on item 1 only.** Items 2, 3 and 4 pass. The blocker is a third `api` spelling,
found by the same probe that found `-${F}`, and it has the same one-edit fix.

## Item 1 — the fix is right, but a THIRD spelling sits in no part of the region

The clause is correctly stated (truth 30, prohibition 47, Task 3 edit at `:543`) and the three
directions hold: `-f` has a readable key half and stays a countable flag; `title="$T"` is neither
`-`-initial nor marker-initial and stays an operand; `-$F` and `-${F}` are both `-`-initial with an
expansion before the first `=` and are caught.

The remaining hole is not in the clause — it is in **which scan reports the endpoint**. The region
takes the endpoint as one of "the first TWO non-flag subcommand words", i.e. from
`subcommand_words`, which skips only `FORGE_VALUE_OPTS` (`-R`, `--repo`, `--hostname`,
`policy.rs:1773`). But `gh_api_posts_a_pull_request` takes its `path` from its own scan, which skips
all of `GH_API_VALUE_OPTS` — `-f`, `-F`, `--field`, `-H`, `--header`, `-q`, `-t`, … (`:1800`). Any
of those option **values** is a non-flag word to `subcommand_words`, which displaces the endpoint
out of the first two:

```
E=pulls; gh api -f title=x repos/o/r/$E
```

`subcommand_words` = `["api", "title=x", "repos/o/r/$E"]` — the endpoint is the **third** word.
The token is not marker-initial and not `-`-initial, so it is in no part of the region. Meanwhile
`gh_api_posts_a_pull_request` skips the `-f` pair, sets `implies_post`, takes
`path = "repos/o/r/$E"`, and `endpoint_is_pulls` (`:1882`) compares the last path segment against
`"pulls"` — it is `$E`, so it returns false. `posts && false` → `pr_command_label` returns `None`
→ the pull request opens with no refusal, no ledger line and no cap charge. The `-H accept:x`
spelling is the same hole with one more displacing option.

This is exactly the drift the plan's own principle forbids — "every index reported by the scan the
classifier itself runs" — applied everywhere except this one slot. **Required:** for an `api` argv,
the endpoint decision word must be reported by `gh_api_posts_a_pull_request`'s own scan, extracted
as an index primitive the way edit 1 extracts the subcommand index primitive, rather than taken from
`subcommand_words`' first two. Add `E=pulls; gh api -f title=x repos/o/r/$E` and the `-H` spelling
to Task 1's `api` group with empty-walk assertions, and add the displaced-endpoint slot to the
forge-slot property.

I probed the other decision inputs of that arm and found no fourth: the inline `-X=$M` /
`--method=$M` value is covered by "the method value in both its spellings", whole-token
`GH_API_IMPLIES_POST` membership is covered by the two flag-ness clauses, and flag-skipping is
covered by the `-`-initial clause.

## Item 2 — PASS

The second-carrier justification is gone from every site. The only surviving "second carrier"
mentions in `19-14` are the correct ones: the audit-3 `${X}stash` / `${X}update-ref` rows, and the
explicit **absence** statements. The true reason — round discipline, plus `config` being a row in
audit 3's own measured set — and the recorded absence for `reflog` and `symbolic-ref` with only
`push $REF` having a hook behind it, appear at all four registration sites plus the prohibition
(`:51`), which now bans the false reason by name. `19-15` carries the same statement forward at
`:37` and `:467` and requires it in its SUMMARY at `:531`.

## Item 3 — PASS, and this form of the gate is sound

Both gates are non-vacuous standing alone, in both flake modes:

- **19-14.** `passed + failed` is mode-independent (the `driver_reattach` pair always *runs*;
  1495+2 = 1497+0 = 1497), so `> 1497` holds across modes and a zero-test run reports exactly 1497
  and fails. The 2-or-4 failure count is the correct span (driver 0 + carry 2, driver 2 + carry 2),
  and the name whitelist stops an unrelated regression being absorbed. The strongest part is the
  per-binary constraint: `--test envelope_expansion_slots` must show **exactly** the two
  carry-forward failures with every other row green — a flake in another binary cannot forge that.
- **19-15.** Double-locked, which is what makes the handoff unforgeable. A run that did nothing
  fails twice over: the total is unchanged so `> recorded total` fails, **and** the two
  carry-forward rows would still be failing, which violates "0 or 2, `driver_reattach` names only".
  The carry-forward rows green is enforced mechanically, not only in prose —
  `--test envelope_expansion_slots >/dev/null || exit 1` is in the verify block.
- **The handoff itself.** Because the total is mode-independent, `19-14` cannot record a
  mode-dependent number that `19-15` then clears by accident. The pre-flight confirmation
  (prohibition `:47`, Task 1 action, `<done>` at `:314`) stops with a finding if either row is
  already green, which is the right response — it is prose-enforced because a red observation
  cannot be a machine gate, and it is stated as a stop condition rather than a note.

## Item 4 — PASS

`19-14`: `gap_ids: [T-19-88, T-19-90]`, `envelope_command_position.rs` correctly absent from
`files_modified`, and prohibition `:49` bans the positional rule, the `Token` flag, `SEPARATORS`,
`split_segments`, `tokenize`'s separator arm and the false-test deletion by name — all
diff-checkable. Both partial threat rows say so explicitly (`T-19-87` "partial — six of eight rows",
`T-19-89` "partial — the Rule A half").

Nothing is stranded either way. Rule A needs only `Token.expansion`, which already exists, so it
needs nothing from `19-15`. Rule B needs `tokenize`, the flush flag and the false-test deletion,
all of which are in `19-15`'s `files_modified`. I checked the one case where the split could have
left a landmine: the false `T-19-87` test survives untouched through `19-14`, and both of its rows
stay green under Rule A — the permit half's segment has the literal verb `fetch`, and the
force-push half's segment has the literal verb `push` and still refuses under
`force_push_blocked` — so `19-14`'s green gate on that binary is honest, and the identifier
correction correctly travels with the rows into `19-15`. Neither plan is independently
unexecutable: `19-14` stands on `19-13`, `19-15` declares `depends_on: [19-14]` and begins by
confirming its inherited red state.
