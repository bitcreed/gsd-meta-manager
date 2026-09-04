# Plan check — 19-22 (corpus) + 19-23 (rules)

**Verdict: PASS WITH CHANGES.** One WARNING, no blocker. Checked at `63b82be`, main tree,
read-only, nothing implemented.

## Recurring pattern found: none of the three

- **One slot over** — not found. The section-only test was probed against every git config
  indirection mechanism (below); no adjacent cell escapes it.
- **A corpus incapable of failing on its class** — not found. `19-22` widens first, the
  permitted-base constraint is asserted mechanically rather than left to care, and the RED
  set is measured against the built binary before it is written.
- **An undischargeable two-plan seam** — not found. `19-23` touches no file `19-22` wrote
  except `tests/envelope_config_resolution.rs`, additions only; every `19-22` assertion is
  reachable from `19-23`'s mandated design without a body replacement.

## The one finding

**WARNING — the fail-open admission names a compensating control that cannot compensate.**
`19-23` Task 1 point 4 mandates the sentence *"a future git that adds a THIRD indirection
section is not covered and this rule fails OPEN on it; the only control over that direction
is the pin below."* The pin measures that `include` and `includeIf` outrank the envelope's
injection and that confined carriers do not. It **cannot observe a section it does not name**,
so it holds the *reverse* direction (a git that stops honouring a known include) and offers
nothing over the residual direction the sentence attributes to it. This is `T-19-107`'s own
failure mode — a false reassurance in a control's own doc — reproduced in the round that
registered it, in the very sentence written to prevent it. Fix: say there is **no automated
control** over a third indirection section; the pin holds only the two known sections'
behaviour, and re-audit is the compensating control. Same correction in the
`19-SECURITY.md` record (Task 4 bullet 5) and in `19-22`'s design-question paragraph.

## Check 1 — is the section-only test sufficient? (most effort spent here)

Sufficient for the command-line carrier. Every mechanism enumerated and checked:

- `-c include.path=`, `-c includeIf.<cond>.path=`, `--config-env=` attached and separate-word,
  case-varied — all reduce to the same `config_key_of` → section comparison. Re-measured here
  against `git version 2.43.0` with the envelope's own triplet as control: control `/ENV_WINS`;
  `include.path`, `INCLUDE.PATH`, `INCLUDEIF.gitdir:<p>.PATH` all `/INCLUDE_WINS`;
  `GIT_CONFIG_PARAMETERS` `/PARAM_WINS`. Every precedence claim in both plans reproduces.
- **Reading only the section is a strength, not a hiding place.** `[include]` honours exactly
  one variable and `includeIf`'s subsection is an open condition family; not reading either
  covers both by construction, and the only cost is over-refusal of a variable git ignores —
  confirmed: `git -c include.pathx=… config --get core.hooksPath` exits 1 having read nothing.
  Disclosed and pinned. Section-right-mechanism-different does not exist: no non-`path`
  variable in either section has an effect, so a refusal there is safe-direction only.
- **Dotless keys cannot be indirections** — verified: `git -c a=b config --get a` →
  `error: key does not contain a section`. Confining them is correct, not a concession.
- **Non-section carriers**: `-C`, `--git-dir`, and a *persisted* `git config include.path`
  all splice at repo precedence. Measured here: repo-level include yields `/INCLUDE_WINS`
  alone but `/ENV_WINS` under the injection — **inert**, so the `git config` subcommand path
  is not a second escape and correctly needs no clause.
- The admitted residue (a future third section) is real and correctly the only one.

## Check 2 — fail-open admission scoping

Correctly placed: the rule's own doc, the record, a prohibition (`honesty`), a `must_haves`
truth, and the objective. Fail-closed is genuinely unavailable — the complement over config
sections is unbounded and `git -c user.name=…`/`core.pager`/`a=b` are measured permitted and
pinned. The refusal to call this a fifth inversion is correct and correctly placed. Only the
compensating-control attribution is wrong (finding above).

## Check 3 — is `GIT_CONFIG_PARAMETERS` the only missing carrier?

Yes, for `core.hooksPath`. `ENVELOPE_ENV_KEYS` read at `policy.rs:2816-2874`: it holds
`GIT_CONFIG_COUNT`, `GIT_CONFIG_KEY_`, `GIT_CONFIG_VALUE_`, `GIT_CONFIG_GLOBAL`,
`GIT_CONFIG_SYSTEM` and the SSH/locator entries. `GIT_CONFIG_PARAMETERS` is the only env
carrier that outranks the injection; `GIT_CONFIG_NOSYSTEM` defeats a key the envelope sets.
`GIT_DIR`/`GIT_WORK_TREE`/repo config all rank below the injection. Legacy `GIT_CONFIG` is
uncovered but affects only the `git config` command itself — not worth an entry, not a gap.
The inertness argument is verified in source: `write_gitconfig`'s doc at `cred.rs:253-271`
states the file is *"pointed at by both `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM`"*, and
`19-23` Task 2 pins that the two paths are the same path — so a later divergence turns the
pin red rather than spending the inertness silently. Correctly recorded as a defeat with an
inert harm, never a bypass.

## Check 4 — can the widened corpus fail on this round's class?

Yes. `CONFIG_RESOLUTION_CLASSES` is a fourth axis with its own predicates, three disjoint
alphabets and its own floors; the three existing axes are held byte-identical and the plan is
granted no deletion. Degenerate-proofing includes the negative
(`git commit -m "include.path=…"` satisfies none). The permitted-base fence is asserted
mechanically, and the five `--force` compositions are correctly demoted to controls —
constraint 2 is honoured, not merely stated. Floors carry stated arithmetic (audit 5's
50-against-40 defect addressed). Pre-fix RED is guaranteed by rows measured at exit 0 today
on permitted bases.

## Check 5 — seam dischargeable

Yes. `-c a=b` fenced in both plans and in `19-23`'s prohibitions; `CALLEE_KNOWN_LEADING_PREFIX`
confirmed at `tests/envelope_wrapper_class.rs:5197` with round 7's whole property spliced
behind it, and real git runs `git -c a=b version` at rc 0 — verified. Every derivable row is
asserted at its post-fix verdict; the two underivable rows are recorded, not asserted, and
`19-23` appends their pins as new fns. Both ordering rows are pinned in opposite directions,
which is what forbids a second-pass implementation. No replacement exception needed or granted.

## Check 6 — the `cred.rs` exception

Genuinely narrow. Verified the target block: `hooks_path_env`'s doc does claim *"the one form
that outranks this injection is `git -c core.hooksPath=… push`"* — false by four measured
forms. Confined to that paragraph, `T-19-61`…`T-19-73` explicitly not taken on, exception
named in prohibitions, record and SUMMARY. Minor: the Task 2 verify uses
`git diff --stat -- src/envelope/cred.rs`, which proves one file but not confinement to
`:241-248`; the `done` criterion carries that burden in prose. Acceptable, not a finding.

## Check 7 — spawn control fixes

Every figure re-measured and correct: `SPAWN_MARKERS` in `policy.rs:6171` holds two entries
while `tests/spawn_seam_guard.rs:108` holds three; `CommandWrap::with_new(` is used at
`src/executor/claude.rs:480`; `fn scan_leading` at line 374; the sole `#[cfg(test)]` sentinel
at 4263; `fn forbidden_repo_path` at 4241 (the proposed deep anchor, valid); `policy.rs`
6,541 lines / 307,496 bytes, so a 4,262-line production half is consistent. Exactly one
sentinel line today, so the sentinel-count assertion is green on arrival. Self-invalidation
hazard (the marker literal must stay below the sentinel) is named rather than discovered.

## Check 8 — gate arithmetic

Satisfiable in both plans. Baseline 1639 confirmed in `19-21-SUMMARY.md:224` with the
identity checked at line 233. `19-22` names its expected failures instead of demanding zero
(round 3's defect), gates on `passed + failed > 1639` with the increase attributed only to
new `#[test]` fns, and requires fourteen `envelope_*` binaries — thirteen exist on disk, so
fourteen is reachable and is a real check that the new file ran. `19-23` gates on `19-22`'s
recorded total plus new fns and forbids folding pins into existing fns, which is what keeps
the identity satisfiable. `--no-fail-fast` and `rtk proxy` discipline present in both. No
floor is unreachable.

## Check 9 — scope

Clean. `T-19-86` four rows plus the persisted-alias arm kept at exit 0, closure framed as a
*restoration* of layer 3's catch and never a closure; `T-19-91`, `T-19-96`, `T-19-74`,
`T-19-84`, `T-19-85`, `T-19-61`…`T-19-73` untouched; `glab --host` carried forward unfixed
with the not-installed caveat; `T-19-17r` OUTSTANDING with no `AR-19-13` and the word
"accepted" prohibited; both plans state they do not clear `/gsd-secure-phase 19`; no
unqualified "T-19-60 is closed". `19-22` carries zero `src/` hunks. `SEPARATORS` and both byte
floors (`POLICY_MIN_PRODUCTION_BYTES = 40_000` at line 831, `HOOKS_MIN_PRODUCTION_BYTES =
20_000` at 838) held; the 22.37% note is load-bearing and correctly used to justify keeping
them. `CONFIG_VALUE_OPTS` at `policy.rs:997` does carry `--comment`, and git 2.43.0 answers
``error: unknown option `comment'`` at rc 129 — `T-19-106`'s stale entry is real. `git -v`
runs at rc 0 and `git -v XVALUE version` terminates like `--version` — `T-19-107` is real.
Both `-cuser.name=x` and `-C/tmp` are rejected by this git, as `19-23` claims.

Fix the one WARNING and execute.
