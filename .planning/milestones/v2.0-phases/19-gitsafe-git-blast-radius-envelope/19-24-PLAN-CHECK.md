# Plan check — 19-24 (corpus) + 19-25 (rules), round 9

**Verdict: PASS WITH CHANGES.** Two WARNINGs, no blocker. Checked at `27c7a27`, main tree,
read-only, nothing implemented. Every load-bearing claim below was re-measured independently
against `git version 2.43.0`, `gh 2.45.0` and the built `./target/debug/gsd-meta-manager` —
not read off the plans.

## Recurring pattern: none of the three

- **One slot over** — not found. K1's single-membership was probed, not accepted (below).
- **A corpus incapable of failing on its class** — not found. All eight bypass/cost rows
  measure exit 0 today on layer-2-**permitted** bases; the discriminators measure exit 2.
- **An undischargeable two-plan seam** — not found. Every row `19-24` asserts is derivable
  from `19-25`'s mandated design, and the two that are not are RECORDED, not asserted.

## 1. Is option (b) one slot over? No — K1 completeness holds under probe

Swept all 29 K2 keys the plans enumerate, plus `include.path`, `includeIf.*.path` and
`core.hooksPath`, each carrying `-c include.path=<evil>`, against the envelope's own
`GIT_CONFIG_COUNT` triplet as control. **Only two came back non-inert, and both are already
closed**: `include.path` (K3, `19-23`) and `core.hooksPath` (by-name deny, `19-02`). Every
other key returned `/ENV_WINS`. K1's "only member" claim survives the probe.

K2 inertness measured, not read: `!`-alias → `/ENV_WINS`, `diff.external` → `/ENV_WINS`,
`filter.<n>.clean` → `/ENV_WINS`. Direct evidence of the mechanism: a `!` alias child's own
environment shows `GIT_CONFIG_COUNT=1 / KEY_0=core.hooksPath / VALUE_0=/ENV_WINS` intact.
No K2 member was found running where the injection does not reach.

## 2. The one-byte shell rule is correct and correctly fenced

All four spellings the plans cite reproduce exactly. Two the plans did **not** test also fall
the right way: a **quoted** body `'"!git …"'` is re-parsed IN-PROCESS (K1) and fails as
`'!git …' is not a git command` — first byte is `"`, so the rule refuses it, which is correct
and conservative; a **tab** before `!` behaves as the leading space does (git refuses to
expand). A body that is only `!` runs a shell. No spelling was found where the first byte is
`!` yet git re-parses in-process. The fence is right.

## 3. The depth argument is stronger than claimed — option (a) is genuinely dead

Depth 2 reproduces (`/INCLUDE_WINS`) and so does **depth 3**. `git -c alias.m='config --get
"core.hooksPath"' m` resolves, confirming git's own `split_cmdline` dequoting. `--config-env`
delivery of an alias body also resolves `/INCLUDE_WINS`. All three grounds for rejecting (a)
are measured facts.

## 4. The corpus can fail on this round's cell

Measured against the built binary, fresh root per row, walk empty in all:

- exit 0: `alias.q` include carrier, `alias.z` hooksPath carrier, persisted `git config
  alias.p '<non-! body>'`, `ALIAS.q`, `--config-env=alias.q=BODYVAR`, and all three disclosed
  over-refusal rows plus `git config alias.co checkout`.
- exit 2 controls: `-c include.path=… push --force` and `-c core.hooksPath=… status`
  (`envelope_assertion_failed` / `hook_bypass_blocked`), `git config core.hooksPath /dev/null`,
  `-c a=b push --force`.
- permitted bases confirmed permitted: `git -c a=b status` = 0, `git p` = 0, and
  `-c alias.p='!git push --force origin main' status` = 0 (the invariance-arm splice).
- both ordering rows are `hook_bypass_blocked` today; row 1 moves to
  `envelope_assertion_failed` only if the clause is raised in the one left-to-right walk.

Sixth class on the existing axis, not a fifth axis — correct reading of audit 8.
`MIN_CONFIG_RESOLUTION_CLASSES` is 5 at `tests/envelope_wrapper_class.rs:6399`, rising to 6.

## 5. The seam is dischargeable

`tests/envelope_command_position.rs:550` and `tests/envelope_config_resolution.rs:1539-1543`
exist exactly as cited and pin `!` bodies permitted. The fence is asserted **mechanically**
(no `!` first byte in `CONFIG_REPARSED_VALUE_CARRIERS`; a `!` entry BY NAME in
`CONFIG_CONFINED_CARRIERS`), not left to prose.

Region 2 reach verified rather than assumed: `git config core.hooksPath /dev/null` refuses at
`hook_bypass_blocked` in **all six** write spellings `19-24` asserts (bare, `--global`,
`--worktree`, `--file`, `--add`, `--replace-all`, plus `--local`), so `classify_config`'s
`key_operand()` resolves the key in every one of them and `19-25`'s clause reaches every row
`19-24` writes. `git config --get alias.p` and `git config include.path /tmp/evil.cfg` are
permitted today and stay so.

`19-25` writes nothing into `tests/envelope_wrapper_class.rs` and only ADDS to
`tests/envelope_reparsed_value.rs`. The two undeliverable rows — the separate-word
`--config-env` spelling and `-c alias.q` with no `=` — are RECORDED, which is precisely the
discipline `19-22` broke. The attached `--config-env=alias.q=BODYVAR` spelling **is** derivable
(`leading_git_option` arm (b) → `config_key_of` → `alias.q` → value `BODYVAR`) and is
correctly asserted rather than recorded.

## 6. The cost is honest and proportionate

Permitted twins do the same work; invocation is genuinely untouched (`git p` = 0 measured, and
the guard is argv-only so it cannot see a definition it did not watch). The refused family is
one legible shape — defining a non-shell alias mid-run — traded against a demonstrated
remote-ref-moving bypass. Accept.

## 7. The three fail-open directions

Stated in the constant's doc, the predicate's doc, the refusal's doc and the record; the
prohibition explicitly forbids handing any of them to a pin; the two-sided pin's own doc must
open by stating the direction it holds and say it is not a control over the residue. Round 7's
failure mode is not repeated. **One wording hazard — see WARNING 2.**

## 8. `FORGE_VALUE_OPTS` — the planner is right, audit 8 is wrong

Measured against the built binary: `glab --hostname gitlab.com mr create --title x` leaves
**exactly one** `alpha/pr-ledger.ndjson` line; `glab --host gitlab.com mr create --title x`
(an option not in the constant) leaves **zero**. Removing `--hostname` moves a counted creation
form to uncounted — the under-counting direction. `gh pr create --hostname` → `unknown flag`,
so it is inert for `gh` and load-bearing for `glab`. Keep-and-record-why is correct.

All seventeen `GH_API_VALUE_OPTS` entries re-measured: every one answers `flag needs an
argument`. `--bogus-opt` → `unknown flag`. Count is exactly 17.

## 9. Gate arithmetic

Baseline **1680** confirmed at `19-23-SUMMARY.md:220`. **14** `envelope_*` binaries exist today,
so fifteen after `19-24` — the "fourteen means this plan's file did not execute" check is sound.
The gate is flake-invariant by construction: a fired flake is a test that RAN, so it lands in
`failed` and `passed + failed` is unchanged. Correct in all three flake modes.

## 10. Scope anchors — all verified

`CALLEE_KNOWN_LEADING_PREFIX` at `:5197`; `CONFIG_RESOLUTION_CASES = 161`, `SLOTS = 21`,
`MIN_CONFIG_RESOLUTION_CLASSES = 5`; `POLICY_MIN_PRODUCTION_BYTES = 40_000` at `:831`,
`HOOKS_MIN_PRODUCTION_BYTES = 20_000` at `:838`; `SEPARATORS` at `policy.rs:1786` with no `>`;
exactly ONE `#[cfg(test)]` at `policy.rs:4614`; `fn forbidden_repo_path` at line 4,592 of a
4,613-line production half. Independently measured production bytes: policy raw 372,920 /
production 228,100 (180,000 = **78.9%**), hooks raw 99,753 / production 74,357 — the plans'
figures are right to within one byte of newline convention.

`grep -cE '^\| AR-19-13 \|'` returns **0**; all fifteen textual occurrences of `AR-19-13` are
prose asserting its absence. Both plans' verify commands use the anchored table-row form, so
they measure the right thing. `deferred-items.md:1399` does mark `T-19-106` **CLOSED** — the
correction `19-24` schedules is needed.

---

## WARNING 1 — the `--paginate` negative control's measured answer is not what the plans describe

Both plans require `--paginate` to "come back as an existing flag that needs no value".
Measured, `gh api --paginate` answers **`accepts 1 arg(s), received 0`** — a *positional*
arity error from the command, not a flag-level answer. The control still works, because that
string is neither `flag needs an argument` nor `unknown flag`, so the classifier's third bucket
is reachable. But an executor who writes the probe expecting a flag-level message will get a
red pin, and one who adds an endpoint to silence it will make the pin issue a network call.

**Fix:** record the measured string in `19-25` Task 3 and state that the control asserts
*absence of both other answers*, with no endpoint operand supplied.

## WARNING 2 — the version witness and "no automated control" can be read as contradicting

`19-25` requires the docs to say direction (iii) — a future git re-parsing a second config
value — has **no automated control**, while the same task adds a version-witness assertion that
fires on a git upgrade. Both are defensible: the witness detects a version *change*, not a new
re-parse section, so it is a schedule and not a detector. But the plan never requires the
witness's own doc to draw that line, and a future auditor reading a residue paragraph beside an
assertion that fires on exactly that trigger has grounds to call it the `T-19-107` shape —
committed, again, by the round that inherits it.

**Fix:** mandate one sentence in the version-witness doc: *this detects that the installed git
changed, not that a new section became re-parsed; it schedules the re-derivation and is not a
control over the residue.*

---

## Not findings, recorded

- The proportional floor's own comment at `policy.rs:6644` says "228,785 bytes, 78.7%"; measured
  production is 228,100 (78.9%). The **code comment** is stale, the plans are current, and
  neither plan may edit it. No action.
- `19-25` declares `SAFE-04`, which the roadmap binds to Phase 16. `19-23` did the same and
  passed review; the refusal-message redaction rule is substantively in scope here. No action.

**Both plans are cleared to execute in order: `19-24` (RED, zero `src/` hunks) then `19-25`.**
