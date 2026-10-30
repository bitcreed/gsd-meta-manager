# Plan check — 19-26 (corpus) and 19-27 (rules + corrections), round 10

**Verdict: PASS WITH CHANGES.** One BLOCKER (the carrier enumeration is not exhaustive), two WARNINGs.
Everything else checked held on measurement.

## Recurring pattern — fired, once

**Pattern 1, "one slot over", recurs for the eighth time — and this round it is one *tier* over.** The
enumeration walks the envelope root, the driven repo and the exec'd binary, and stops. It does not walk
the file the guard itself reads on every tool call.

## Findings

**BLOCKER — F1. The enumeration misses `~/.config/gsd-meta-manager/config.json`, a live SAFE-06 carrier
read per guard invocation.** `guard()` (`src/envelope/hooks.rs:837-840`) passes
`crate::config::Config::default_path()` into `guard_in`, which calls `resolve_policy(config_path, alias)`
(`hooks.rs:1239-1258`) on **every** Bash tool call. That function loads the registry config from disk and
takes `driver_opt_in` straight into `EnvelopePolicy::resolve` (`policy.rs:2116-2137`), where
`pr_cap_per_24h` and `pr_cap_per_run` are `unwrap_or(DEFAULT_*)` with **no clamp** — only `namespace` is
validated-and-degraded, and `credential` is copied through unchecked. `Config::default_path()` is
`$XDG_CONFIG_HOME/gsd-meta-manager/config.json` (`src/config.rs:330-335`) — same uid, no layer-1 tool
deny, no layer-2 governed program, not under the envelope root. So `printf '…"pr_cap_per_run":9999…' >
~/.config/gsd-meta-manager/config.json` disarms the pull-request cap **without deleting the ledger**, and
survives rule (a) by construction. `grep -c 'config\.json\|default_path\|driver_opt_in\|resolve_policy'`
over both plans returns **0 / 0**.

Why this is a blocker and not a note: the plan's own truth calls the enumeration "THIS PLAN'S CENTRAL
PRODUCT" and ships it as *exhaustive* (14 live + 2 non-carriers + 1 covered family, stated so "the
enumeration is exhaustive rather than selective"). Shipping an enumeration labelled exhaustive that omits
a cheaper carrier for the very control the round is built around is `T-19-107`'s registered shape —
a text claiming a reach it does not have — committed by the round that inherited the lesson. It also
falsifies the round's framing that carrier control splits cleanly into "under the envelope root" (a) and
"repo-side" (e): this one is neither.

Bounded fix, inside every existing prohibition: add `C-15` (and the `credential`/namespace consequence)
to the enumeration table as an **(e)** carrier — measured, RECORDED, never asserted, exactly as C-11…C-14
are; add its row to `19-26` Task 1 section 6 and Task 3's `deferred-items.md` registration; correct the
"fourteen"/"seventeen" counts in the objective, the must_haves and the artifact descriptions. No rule, no
`src/` change, no acceptance — rule (a) genuinely cannot reach it and saying so is the honest answer.

**WARNING — F2. `19-26`'s class arithmetic is stated at two different values in the same file.** The
objective and Task 2 specify seven `CONTROL_CARRIER_CLASSES`, `MIN_CONTROL_CARRIER_CLASSES = 7` and six
verdict-preserving alphabets; `key_links` (`:119`) and the `T-19-114` STRIDE row (`:867`) still say "five
classes … four verdict-PRESERVING alphabets". That is exactly the drift the plan's own audit-5 citation
warns about (a floor stated against the wrong maximum). Reconcile both to 7/6.

**WARNING — F3. `advisory.rs`'s repaired `Guaranteed` clause should not be judged only against rule (a)'s
four directions.** Task 3's constraints are right, but if F1 lands the cap is also resettable through a
carrier outside every path the repair discusses. The repaired text stays *true* only because it drops the
unqualified affirmative; make sure the SUMMARY does not present rule (a)'s four directions as the
complete residue for SAFE-06.

## What was checked and held

1. **Rule (a)'s boundary is right.** `envelope_dir_in` (`mod.rs:221-226`) is validate-then-`join`, no I/O.
   `classify_segments` (`hooks.rs:939-961`) already carries `root` and `alias`; no new input. Placement
   before the resolution match is required and correctly argued — `git config --file <env>/alpha/gitconfig
   alias.x …` resolves `Governed` and never reaches the `Ungoverned` arm. `Token.literal` exists
   (`policy.rs:2223`). Component-wise, `..`-collapse and no-filesystem pins are all specified. I probed for
   an envelope-root path the rule would miss and for an ordinary command it would newly refuse; the four
   named directions and the disclosed read-refusal family are the complete set on the argv side.
2. **Finding 1 is correct as written, and it is a live disclosure gap.** `grep -rn 'settings_json' src/`
   returns exactly two hits — the definition (`hooks.rs:1467`) and the D-07 doc row (`:1419`) claiming it.
   Production pushes `--settings <path>` (`claude.rs:290-293`); `--disallowedTools` is genuinely argv-carried
   (`:286-289`). **The second carrier for the guard registration does not exist.** The plans handle this
   correctly: measured, RECORDED, explicitly NOT repaired, harm marked unmeasured.
3. **The four fail-open directions are honestly stated and handed to nothing.** Direction (i) is real, not
   an excuse: `tokenize` DELETES a redirection's operator *and its target* (`policy.rs:2264-2279`, T-19-97),
   so the ledger path is not a word at all — the pinned-PERMITTED `printf … > pre-push` row will not flip
   under rule (a). (iii) and (iv) are genuinely *narrowed*, with both mitigations (`ln -s`, `cd`) refused for
   their own operand and both measured, not asserted. No pin, schedule or witness attached anywhere. No
   third repeat of round 7's or round 8's mistake.
4. **The corpus can fail on this class.** `CONTROL_CARRIER_CLASSES` is a genuine fifth axis — no existing
   predicate is satisfiable by a command reaching no governed program, and Task 2 proves it mechanically in
   both directions rather than in prose. Degenerate-proofing, both new fences, the permitted-base fence, the
   disjointness extension and derived-and-stated floors are all present; existing floors and alphabets stay
   byte-identical; `19-26` is RED by construction with zero `src/` hunks.
5. **The seam is dischargeable.** Every row `19-27` cannot satisfy is RECORDED, never asserted, and the one
   thing `19-27` must change inside `19-26`'s file — the `SECTION_ENVELOPE` pin — is pre-named by test
   function in `19-26` Task 1 section 10, with `19-27` restricted to additions elsewhere. The three fenced
   files are narrow and named (one call site, doc-only, one constant), with `mod.rs:165-168` explicitly
   decided-at-execution and recorded either way. This is not `19-22`'s shape.
6. **The ledger deferral is right.** (b) is genuinely available (append-only), was costed against the
   second-carrier statement (`T-19-35`/`AR-19-05`/`hooks.rs:1421-1428`) and the single-pass latency rule, and
   is deferred with no schedule. (b) for the stub is correctly called structurally unavailable.
7. **Gate arithmetic.** 15 `envelope_*` binaries exist today → 16 after; baseline 1720 on `passed + failed`;
   `--no-fail-fast` mandated; `rtk proxy` mandated around every `grep` of cargo output; `cargo clippy --tests`
   correctly excluded; three flakes out of scope, absence not treated as evidence.
8. **Scope.** `T-19-86` untouched with `T-19-111` moved OUT of it (all five sites enumerated, the two record
   sites corrected BESIDE); `T-19-91`, `T-19-96`, `T-19-110` (medium), `T-19-74`, `T-19-61…T-19-73`,
   `T-19-84/85` untouched; `--hostname` kept; `glab --host` unfixed and not claimed live; `T-19-17r`
   OUTSTANDING with no `AR-19-13` and no "accepted" (`grep -cE '^\| AR-19-13 \|'` gated at 0); no unqualified
   "T-19-60 is closed"; `SEPARATORS` byte-identical; byte floors intact; `AR-19-04`/`AR-19-05` recorded, not
   un-accepted.
