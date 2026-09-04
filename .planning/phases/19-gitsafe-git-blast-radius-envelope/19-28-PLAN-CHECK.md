# Plan check — 19-28 (corpus) and 19-29 (rules + honesty repairs), round 11

**Verdict: PASS WITH CHANGES.** No blockers. Four WARNINGs, one of which is the recurring
pattern. The round's central mechanical claim is correct as written and was re-derived from the
source rather than accepted from the plan.

## Recurring pattern — fired, once

**Pattern 1, "one slot over", recurs for the ninth time — and this time it is one PREDICATE over.**
`19-28` Task 2 step 1 refines exactly two class predicates (1 and 5). It needs three.

## Findings

**WARNING — F1. Class 2 must be refined as well, or the disjointness fence fires on the new
glob/brace REDIRECTION-TARGET entries.** `draws_a_redirection_target_carrier`
(`tests/envelope_wrapper_class.rs:7926-7930`) is `is_redirection_target && text.starts_with(ROOT)`
with no literalness guard. The plan mandates a glob target and a brace target in the new
verdict-preserving redirection alphabet, and the three spelling classes drawing "in BOTH word
positions"; `: > <ENV>/alpha/pr-ledger.ndjso?` and `: > <ENV>/alpha/{pr-ledger.ndjson,x}` (both in
`19-28`'s own permitted list) satisfy class 2 AND the new glob/brace classes. Class 2 is also the
class whose alphabet is moving to the fail-closed arm, so the collision puts verdict-PRESERVING
entries under a fail-closed class's count. Fix inside the plan's existing reach: refine class 2 the
same way class 1 is refined (exclude a target carrying a pathname-expansion or brace metacharacter),
state it as a refinement in its doc, and re-derive `CONTROL_CARRIER_CLASS_COUNTS` from three
refinements rather than two. The tilde target does not collide (it does not start with the root).

**WARNING — F2. The empty-`credential.helper` candidate control REPRODUCES, but the plan's stated
pass/fail metric would report it as failing.** Measured here (git 2.43-era behaviour, `GIT_CONFIG_GLOBAL`
pointed at a `gitconfig` carrying `helper = store`):

- `git config --get-all credential.helper` with the injected empty pair returns **`store` and then an
  empty line, exit 0** — it does NOT "report nothing". `19-28` Task 1 section 2 makes exactly that the
  first question, and an executor answering it literally records the control as NOT reproducing.
- `git credential fill` for `https://github.com` returns the ambient `username`/`password` without the
  pair, and with the pair fails closed (`could not read Username`, prompts disabled). **That is the
  discriminating measurement** — the empty value resets the helper LIST at resolution time even though
  the earlier entry still appears in `--get-all`.
- Cost, measured: `GIT_ASKPASS` is untouched (fill falls through to the prompt path); a later
  `-c credential.helper=store` on the same command line **overrides the reset and the secret comes
  back** — but that spelling is argv-carried and therefore visible to layer 2, unlike every write
  spelling.

Change: make `git credential fill` the branch criterion in `19-28` and in `19-29`'s conditional, and
record the `--get-all` residue as the false-negative it is. Given it reproduces at a cost limited to an
argv-visible override, **`19-29` should be REQUIRED to take the branch, not left the option** — it is the
only control that defends `C-05`'s credential half against all seven spellings, and the redirection rule
alone leaves six open.

**WARNING — F3. The segment-count pin's stated defect is narrower than the defect space it must
catch.** Both plans describe the failure as a SPLIT (`[git]` + `[push, --force, origin, main]`). That is
correct only if the fix marks the emitted target `operator: true` or emits the operator token the
unresolvable arm already emits (`policy.rs:3204-3214`). A target pushed as an ORDINARY word produces
ONE segment with `[git, /dev/null, push, --force, origin, main]` — argv displaced one slot right, which
is `T-19-98`'s shape. The pin as specified catches both (it asserts the exact token list, not only the
count); the FAILURE MESSAGE the plan dictates names only the split. Have the message name both
variants, since the message is what the next executor reads.

**WARNING — F4 (advisory, scope). `19-29` is over the smart zone at 152k / 4 tasks / six `src/`
exceptions with `ledger.rs` and `mod.rs` both first-time opens.** The plan names `T-19-117`'s bound as
the severable item; keep that severance live rather than nominal. `19-28` at 132k / 3 tasks is within
tolerance.

## What was checked and held

1. **The segment-count finding is RIGHT and the `Segment`-borne fix is sound.**
   `split_segments_with_heads` (`policy.rs:2444-2521`) opens its loop with
   `if token.operator { flush current into a Segment … continue; }`, so an operator-marked token does
   split a redirected simple command, and the leading `git` would carry an empty argv. Today neither the
   operator nor the target emits a token in the resolvable case (`consume_redirection!`, `:3193-3214`),
   which is why the row is one segment and exit 2 now. `Segment::redirection_unresolvable` (`:2402-2418`,
   accumulated `:2461-2505`) is exactly the precedent claimed: accumulated in the operator arm of the one
   walk, applied retroactively over `segments[command_start..]`, reset at `; && || | & \n`. Carrying the
   target there leaves `segment.tokens` untouched, so no argv consumer sees anything new. **This is the
   correct shape and the pins make the claim falsifiable.**
2. **`skip_redirection_target` and `redirection_operator_len` are as described.** The former
   (`:3007-3057`) already walks the full extent with single-quote, double-quote and backslash handling —
   the text and the literalness are by-products of a walk that already happens. Its doc's *"Its text is
   never needed, only its extent"* is present verbatim and is the sentence that goes false. The latter
   (`:2968-2992`) is a closed twelve-operator match; `<<`/`<<-` (delimiter), `<<<` (here-string) and
   `>&`/`<&` (fd) genuinely take no pathname, and the split is derivable in the arm that computes the
   length. `SEPARATORS` (`:2297`) is one line, `is_separator` is a bare `contains`, so
   `is_separator(">") == false` by construction and nothing in either plan moves it.
3. **`C-10` as an EXACT PATH is right and the path set is complete modulo the disclosed residue.** A
   prefix over the binary's parent would refuse `ls ~/.cargo/bin`; the near-miss controls pin that from
   both sides. I probed for a missed spelling: `<parent>/../bin/<binary>` is caught by the existing
   lexical `..` collapse; `mv`/`ln -f`/`install` name the same absolute operand; a write via `>` into the
   binary is caught because `19-29`'s predicate applies the two-path set over BOTH word classes. What
   remains — `$(command -v …)`, tilde, relative, and a `PATH` symlink whose target `current_exe()` reports
   instead — is named at full weight, and the last is put to measurement rather than assumed.
4. **Every asserted row has a discriminating control.** Binary rows against the sibling-file and
   `ls <parent>` rows (opposite verdict); the `T-19-118` redirection append against the same append
   outside the root; each tilde/glob/brace row against its absolute-literal twin at exit 2; the ledger
   bound against a just-under-the-bound row that still permits AND still counts. The ordering pins are
   measured in both orders before either is written. No row was found whose control fails at the same
   identifier — `19-27`'s failure mode is fenced by an explicit prohibition.
5. **Scope item 2's seam is dischargeable.** All four `CONTROL_CARRIER_REDIRECTION_TARGETS` entries
   (`:8081-8087`) are absolute literals whose verdict the fix changes, so the move out of the invariance
   arm is forced and mechanical, with four prior precedents. The refinements do not vacate a cell: class
   1's eleven entries (`:8057-8069`) carry no `~ * ? {`, and class 5's two entries
   (`rm -f pr-ledger.ndjson`, `cp /bin/true pre-push`) carry no tilde. See F1 for the third refinement.
6. **The corpus can fail on this round's class.** `T-19-115` is real by reading: `REWRITING_CHARACTERS`
   (`:2535`) is `$ ` ` * ? [ ~`, `Token::literal`'s own table names expansion/pathname/tilde/brace, and
   `envelope_carrier_operand` (`:5401-5406`) requires `token.literal`. The docs at `:5296-5300` do say
   *"fails OPEN in FOUR named directions"* and every cited spelling is `$`-shaped — the arithmetic
   correction is warranted. The new corpus draws `~`, `*`/`?` and `{a,b}` inside carrier paths with a
   fence counting entries rather than prose, and the asserted rows are exit 0 today against exit-2 twins,
   so `19-28` is RED by construction with zero `src/` hunks.
7. **`T-19-117`'s bound is fail-closed and deadline-derived.** Both plans forbid a tail read, a line cap
   and sampling, quote `tally`'s own no-under-count invariant as the reason, derive the value from the
   measured curve against `GUARD_TIMEOUT_SECS` with the margin stated, and explicitly forbid a
   cap-derived value (the caps are unclamped, `C-15`). `ParkReason::EnvelopeAssertionFailed` rather than
   `PrCapExceeded` is right per D-24. The behavioural half is stated UNMEASURED and claimed in neither
   direction, on the same footing as `C-08`'s.
8. **The seven directions are honestly stated and handed to nothing.** No pin, schedule or version
   witness is attached to any of them, and the no-revisit-condition paragraph is kept with its reason and
   extended to all seven. Direction (i) is called NARROWED, never closed. `SECTION_ENVELOPE`'s first
   `Guaranteed` clause is falsified as claimed (`advisory.rs:245-247` asserts both a removed socket and
   *"a generated file naming no credential helper"*), and the repair constraint is real and measured:
   the rendered constant is **213 whitespace tokens against `MAX_TOKENS = 215`**
   (`tests/envelope_advisory.rs:257`) — two tokens of headroom, cap not to be raised, drop a CLAIM never a
   LIMITATION. Tight but feasible, since the repair deletes a false affirmative.
9. **Gate arithmetic.** Baseline **1771** on `passed + failed` confirmed against `19-27-SUMMARY.md:154`
   (`1762 + 9 = 1771`, 45 result lines). Sixteen `envelope_*` binaries exist today → seventeen after.
   `--no-fail-fast` mandated in both plans; `rtk proxy` mandated around every grep of cargo output (D-34);
   `cargo clippy --tests` correctly excluded and the four pre-existing lints left alone; the three
   documented flakes out of scope with absence not treated as evidence and the ETXTBSY race correctly
   identified as `C-10`'s own seam. `19-29`'s "new `#[test]` fns are the only thing that moves the total"
   arithmetic is correct.
10. **Scope.** `grep -cE '^\| AR-19-13 \|'` over `19-SECURITY.md` is **0** and both plans decline
    explicitly; `AR-19-04`/`AR-19-05` recorded and not un-accepted; `T-19-23` not silently re-closed;
    `T-19-86` untouched with `T-19-111` kept OUT of it; `T-19-91`, `T-19-96`, `T-19-110`, `T-19-74`,
    `T-19-61`…`T-19-73`/`T-19-84`/`T-19-85` untouched; `--hostname` kept in `FORGE_VALUE_OPTS`; `glab`
    recorded unmeasurable rather than skipped; `C-08`'s behavioural half unclaimed; no `pr_cap_*` clamp;
    no unqualified "T-19-60 is closed"; `SEPARATORS` byte-identical; byte floors and the stripper's three
    protections preserved with only the editorial `842`→`421` correction (`policy.rs:7450`), and
    `deferred-items.md:2172` corrected BESIDE rather than in place. `19-28` writes zero `src/` hunks;
    `19-29`'s six `src/` files are each named, bounded and stated as an exception.
