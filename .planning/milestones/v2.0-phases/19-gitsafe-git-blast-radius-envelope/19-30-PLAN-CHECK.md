# Plan check — 19-30 / 19-31 (round 12)

**Verdict: PASS WITH CHANGES.** No blockers. Two warnings, one informational. Both plans, checked
goal-backward against `policy.rs`, `ledger.rs`, `cred.rs`, `tests/` and the roadmap at `6d75734`.

**No recurring pattern fired on the substance.** The four hunted patterns were checked and none
reproduced in the design: the boundary is not one slot over, the corpus can fail on its own class, the
two-plan seam is dischargeable, and the certifying rows carry discriminating controls. The one pattern
that *does* recur is in the verify plumbing, not the design — see WARNING 1.

## Verified (checked in order, most effort on 1)

1. **Completeness and containment — TRUE, including an edge neither plan states.** `lexical_absolute_components`
   (`policy.rs:5457`) opens with `let word = word.trim_start_matches("./"); if !word.starts_with('/')`.
   So "`i == 0` is today's rule" is imprecise: today also accepts `.//abs/p` via the trim. Containment
   still holds — for such a word the candidate at the first `/` yields the same component list (empty
   parts are dropped), so **no refusal that exists today can be lost**. The `./`-trim is a no-op on every
   candidate (all start with `/`). Recommend the executor record this in the helper's doc.
   The restated residue is a complete trichotomy over what the predicate reads: non-literal (shell may
   rewrite), no absolute path in the text, reachable only through a link. Probed for a carrier reached by
   a word with **no `/` at all** — `cd <ENV>/alpha && rm -f pr-ledger.ndjson` — it falls in clause 2. Tilde,
   glob, brace, expansion → clause 1. Escaped/quoted-space spellings → clause 1. Nothing found outside it.
2. **Cost claim — TRUE.** `word_is_within` returns early on `dir.is_empty() || word.len() < dir.len()`,
   over *component vectors* (verified), and `word_is_exactly` is `!file.is_empty() && word == file`. Over-refusal
   surface is exactly "a literal word whose text contains this run's own envelope directory or binary as a
   `/`-anchored substring". Probed beyond the plans' list: a URL or ref whose path segment happens to contain
   the run's root would also refuse — same family, and the plans state the general surface correctly even
   though they narrate the relative-word case as "the ONE". No newly-refused ordinary command found outside it.
3. **Fenced-file enumeration — INDEPENDENTLY CONFIRMED at zero.** Swept all `=`-attached literal paths under
   `tests/` (`include.path=/tmp/evil.cfg` ×91, `core.hooksPath=/dev/null` ×33, `--git-dir=/tmp/g` ×5,
   `SSH_AUTH_SOCK=/tmp/evil`, `GSD_MM_ENVELOPE_ROOT=/tmp/fresh`, `/ENV_WINS`, `/CLI_WINS`, …): none names an
   envelope root or a binary. Swept interpolations too — no `={root}`/`={}` site binds an envelope dir; the
   only non-zero-index `{}`/`{var}` path sites are `includeIf.gitdir:{repo}/.path={evil}` (a temp repo, and a
   real-git probe, not a guard row) and `/{owner}/{repo}/pulls` API paths. Both named unit pins re-derived by
   hand and stay `false`: `"/tmp/envroot/alpha/../other/x"` (`..` collapses per candidate) and
   `"./alpha/pr-ledger.ndjson"` (2 components < 3). The single moving artefact is the comment at `:9717-9721`,
   correctly named in advance as `19-31`'s WR-02 correction.
4. **Severance — real on the rule, but see WARNING 2.** Task 3 is its own commit, its own files, with an
   explicit STOP-and-sever path; Tasks 1, 2, 4 stand alone.
5. **Both structural hazards correctly located.** The slice anchor at `policy.rs:9828` runs to the first
   `#[cfg(test)]`, so "at or after the anchor" is right. The ledger arm is
   `if let Ok(size) = std::fs::metadata(&path).map(|m| m.len())` exactly as described; a fresh root has no
   ledger, takes the `Err` fall-through, and **`19-30`'s fresh-root row asserted exit 0 before and after
   does catch a check placed outside the `Ok` arm.**
6. **The widened corpus can fail on this class.** Class 12's predicate is specified on the carrier path at a
   non-zero index and explicitly *not* on an attachment character, so the new `=`-bearing cost entries
   (`--git-dir=/tmp/g`, `GSD_MM_ENVELOPE_ROOT=/tmp/fresh`) do not collide into it. The NO-INTERIOR-PATH fence
   requires a non-`=` attachment by count.
7. **Complement collision correctly caught.** `draws_an_ordinary_operand` (`:8074-8085`) is verbatim the
   negation of the other ten — an interior-path entry would land in the invariance arm. Extending it does not
   vacate class 7 (13 entries, growing). `MIN_CONFIG_RESOLUTION_CLASSES` stays 6; `MIN_CONTROL_CARRIER_CLASSES`
   11 → 12 is the only floor that moves.
8. **Gate arithmetic — correct.** `19-29-SUMMARY.md:497` records 1820 `passed + failed`, 46 result lines,
   13 ignored. 17 `envelope_*` binaries exist today; the new file makes 18. Both plans' per-binary loops
   name 16 and 13 respectively plus the explicit ones — exactly the 17 non-flake binaries, correctly omitting
   `envelope_tracer`.
9. **Scope — clean.** Every fenced item verified as fenced. `19-31` opens exactly three `src/` files.
   `advisory.rs` opened by neither. `cred.rs:416-425`'s "already governed" bullet and `policy.rs:5743`'s
   `dd if=X of=Y` sentence both read verbatim and both are false as claimed. No `AR-19-13`, no `T-19-23`
   closure, no `pr_cap_*` clamp, `--hostname` kept, `T-19-116` stated open.

## WARNING 1 — the zero-`src/`-hunks invariant is verified by a command that cannot fail

`19-30` Task 1/2/3 end in `git show --numstat HEAD -- src/ | wc -l` and `19-31` Task 1 in
`git diff --numstat HEAD~1..HEAD -- src/envelope/hooks.rs … | wc -l`. These **print** a count; they never
compare it, so a plan that wrote `src/` beside the corpus passes its own automated gate. The same shape
applies to the RED requirement — `cargo test --test envelope_interior_path … | tail -45` cannot fail on a
green file. This is the phase's own "a plan mandating a test that certifies nothing" pattern in the verify
plumbing rather than the corpus, three rounds after an executor last caught it. The `<done>` blocks do state
both invariants, so the executor still checks them; that is why this is a warning.

**Fix:** replace each with a failing form, e.g.
`test -z "$(git show --numstat HEAD -- src/)" || { echo 'SRC HUNKS PRESENT'; exit 1; }` and, for the RED
requirement, `rtk proxy cargo test --test envelope_interior_path --no-fail-fast >/tmp/red.log 2>&1; grep -q 'FAILED' /tmp/red.log || { echo 'CORPUS NOT RED'; exit 1; }`.

## WARNING 2 — the `cred.rs` repair is ordered before the rule it must describe, and cannot be amended later

`19-31` Task 2 repairs `cred.rs:420-425` and Task 3 (severable) writes the `credential.helper` rule. Task 2's
action makes the repaired text conditional on Task 3's outcome ("If Task 3 lands, name the clause… If Task 3
is severed, say that no rule refuses it") — but Task 3 runs **after**, and `cred.rs` is not in Task 3's
`<files>`, so the repair cannot be corrected once the outcome is known. The executor must therefore either
predict Task 3 or write a claim that a later task falsifies — which is precisely the "claim that must be
maintained" failure this task exists to end. The severance itself is sound; only the ordering is.

**Fix (either):** add `src/envelope/cred.rs` to Task 3's `<files>` and require Task 3, if it lands, to update
the bullet to name the clause (a severed Task 3 then leaves Task 2's "no rule refuses it" true); **or**
reorder Task 3 before Task 2. `cred.rs` is already in `files_modified`, so neither widens scope.

## INFO — `HEAD~4..HEAD` is off by one when Task 3 severs

`19-31` Task 4's verify uses `HEAD~4..HEAD` for the `src/`/`tests/` diffs. Under severance there are three
commits and the range reaches back into `19-30`'s axis commit. The checks it feeds are supersets, so it can
only produce a false *failure*, never a false pass — but the executor should use `HEAD~3..HEAD` if Task 3 was
severed, or anchor on `19-30`'s last SHA.

## Note on the containment wording

Neither plan mentions `lexical_absolute_components`' `trim_start_matches("./")` preamble, which is why
"`i == 0` is today's rule" is one step short of literally true. The conclusion it supports (no existing
refusal can be lost) is correct either way, verified above. Recording the trim in the helper's doc costs
nothing and stops the next reader re-deriving it.
