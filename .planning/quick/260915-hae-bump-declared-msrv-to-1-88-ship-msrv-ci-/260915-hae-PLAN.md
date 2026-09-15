---
phase: quick-260915-hae
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - .github/workflows/release.yml
  - README.md
  - CONTRIBUTING.md
  - CLAUDE.md
  - docs/DEVELOPMENT.md
  - docs/GETTING-STARTED.md
  - docs/TESTING.md
  - src/journal/redact.rs
  - .planning/STATE.md
autonomous: true
requirements: [QUICK-260915-hae]
user_setup: []

estimate:
  tokens: 55000
  raw_tokens: 55000
  tasks: 4
  confidence: low

must_haves:
  truths:
    - "The declared floor in Cargo.toml is a version the crate can actually be compiled on, MEASURED against the committed lockfile, not asserted."
    - "A crates.io publish cannot happen unless that same floor compile passes first in CI."
    - "The CI gate reads the floor from Cargo.toml rather than restating it, so a future bump cannot leave CI certifying a stale floor."
    - "No surviving sentence in the repo claims the floor comes from `process-wrap`; every provenance statement names the dependency graph that actually sets it and says how to re-measure it."
    - "No reader of the tree can find a stale 1.87 MSRV claim in any tracked file outside `.planning/`."
    - "`.planning/STATE.md` no longer carries a Phase 15 MUST-SPIKE note asking for review of questions that are closed, and nothing else in that file moved."
  artifacts:
    - "Cargo.toml with `rust-version = \"1.88\"` and a corrected floor-provenance comment in the `[package]` block"
    - ".github/workflows/release.yml containing an `msrv` job that `publish` depends on"
    - ".planning/quick/260915-hae-bump-declared-msrv-to-1-88-ship-msrv-ci-/260915-hae-SUMMARY.md"
  key_links:
    - "Cargo.toml `rust-version` <-> the toolchain the `msrv` job installs (asserted equal inside the job on MAJOR.MINOR, so drift fails the build instead of silently testing a stale floor)"
    - "`msrv` job <-> `publish` job via `needs: msrv` (without this the job runs but gates nothing)"
    - "the measured maximum `rust_version` across the resolved graph <-> every prose provenance sentence (the anchor phrase `resolved dependency graph` is what a future reader greps for when the floor moves again)"
---

<objective>
Ship the two pieces the halted quick task `260915-f4n` handed off, and clear one resolved note
out of STATE.md.

**Part A — the floor.** `260915-f4n` MEASURED that `Cargo.toml`'s declared
`rust-version = "1.87"` is FALSE: `cargo +1.87 check --all-targets --locked` exits 101, and the
lib/bins-only escape hatch exits 101 too. `cargo +1.88 check --all-targets --locked` exits 0. The
cause is lockfile drift on two DIRECT dependencies with caret requirements — `ratatui = "0.30"`
resolved to 0.30.2 and `icu_properties = "2.2.0"` resolved to 2.3.0, both now declaring 1.88 —
reached by the ordinary `cargo update` that this project's own release process mandates at every
milestone. That is not a fault to pin away from; it is the steady state. The human decision is
made: **raise the declared floor to 1.88**, correct the four places that claim the floor comes
from `process-wrap` (it does not, and did not even before this bump), and wire the floor into CI
so this class of silent drift cannot recur unnoticed.

**Part B — one stale STATE.md note.** Verified by a fresh read at planning time, not inherited:
`.planning/STATE.md` carries, under `## Deferred Verification` ->
`### Phase 15 planning notes (autonomous run — review these)`, a bullet reporting all three
Phase 15 MUST-SPIKE questions CONFIRMED. Its work is done and its evidence is filed elsewhere.
The other three topics the dispatcher listed (Phase 17 OQ4, Phase 22 OQ5, the MSRV note) were
re-checked by grep against the live file and **do not exist there as removable decision bullets**
— nothing is invented for them. See `<part_b_authority>` for the measurement that authorises the
one removal.

Purpose: a version number nothing tests is documentation, not a contract — and a provenance
sentence that names the wrong dependency is worse than none, because it sends the next reader to
the wrong manifest.
Output: an enforced 1.88 floor, ten corrected MSRV sites, and one fewer closed question posing as
an open one.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@Cargo.toml
@.github/workflows/release.yml
@.planning/quick/260915-f4n-bump-msrv-from-1-85-to-1-87-in-cargo-tom/260915-f4n-SUMMARY.md
</context>

<measured_facts>
Everything below was measured against the working tree while writing this plan. Use it; do not
re-derive it unless a command contradicts it.

**The complete MSRV site inventory** — `grep -rn '1\.87' Cargo.toml CLAUDE.md README.md
CONTRIBUTING.md docs/ src/ .github/` returns exactly these 13 lines across 8 files:

| Site | Current text (abridged) | Treatment |
|---|---|---|
| `Cargo.toml:5` | `rust-version = "1.87"` | bump |
| `Cargo.toml:37-38` | comment: process-wrap's `rust-version = "1.87.0"` "is where the `[package] rust-version` floor above comes from" | **REWRITE + RELOCATE** |
| `CLAUDE.md:25` | `\| Rust \| 1.87+ (stable) \| Language \| ...` | bump the number ONLY |
| `README.md:61` | `Requires **Rust 1.87+**.` | bump |
| `README.md:186` | `- **Rust 1.87+** (for building from source)` | bump |
| `CONTRIBUTING.md:23` | `Prerequisites: Rust 1.87+ (stable) with ...` | bump |
| `docs/TESTING.md:14` | `` `1.87+ stable` `` | bump |
| `docs/GETTING-STARTED.md:13` | table cell: `` `>=1.87` `` + `rust-version = "1.87"` + provenance | bump + **REWRITE** |
| `docs/GETTING-STARTED.md:150` | heading `### "error: package requires rustc 1.87 or newer"` | bump |
| `docs/GETTING-STARTED.md:153-154` | provenance, 3rd occurrence | **REWRITE** |
| `docs/DEVELOPMENT.md:10-11` | `**Rust 1.87+**` + provenance, 2nd occurrence | bump + **REWRITE** |
| `src/journal/redact.rs:218` | doc comment: `this crate's MSRV is 1.87` | bump |

`CLAUDE.md:99` also matches `1.85` — `notify-rs ... MSRV 1.85`. That is a true statement about
the **notify** crate's own floor in a research sources bullet, NOT this project's floor. **Do not
touch it.** It is the one MSRV-shaped string in the tree that is correct as written.

**The true provenance, measured just now:**

```
cargo metadata --format-version 1 --locked \
  | jq -r '.packages[] | select(.rust_version != null) | "\(.rust_version)\t\(.name) \(.version)"' \
  | sort -V | tail -8
```

returns `1.88.0` for `ratatui-core 0.1.2`, `ratatui-crossterm 0.1.2`, `ratatui-termina 0.1.0`,
`ratatui-termwiz 0.1.2`, `ratatui-widgets 0.3.2`, `time 0.3.54`, `time-core 0.1.9`,
`time-macros 0.2.32` — with `darling 0.23.0` and the `icu_* 2.3.x` family at the same value.
`process-wrap 9.1.0` declares `1.87.0`; it is a real fact about that crate and has simply never
been the binding constraint since the graph moved past it.

**Environment, verified:** `rustup toolchain list` already contains
`1.88-x86_64-unknown-linux-gnu`; `jq`, `python3` and PyYAML are all present; `git status --short`
shows only the pre-existing untracked `.gsd/`.

**Baseline for the scope fences.** `HEAD` at planning time is
`d11481dd9f986f29ad16f0fdc864c2632187b52c` (`d11481d`), and `git hash-object Cargo.lock` is
`6009a7b418c1295f5b3729577477095ae3195a02`. The verify blocks below diff against that SHA rather
than against `HEAD~N`, because the per-task commits this plan produces would make any
history-relative offset wrong; and they compare the lockfile by blob hash, which is true
regardless of git history. If a rebase ever makes `d11481d` a non-ancestor, re-derive both from
the commit that PLAN.md was written against — do not weaken the fences to make them pass.

**The anchor phrase `resolved dependency graph` appears nowhere in the tree today.** Task 2 uses
it as the non-vacuous positive gate on the rewritten provenance sentences.
</measured_facts>

<part_b_authority>
This section records the live measurement that authorises Part B's single deletion, so the
executor does not have to re-litigate it — but it must still re-read the file before editing,
because line numbers below are from planning time.

**Present and removable — ONE bullet.** `.planning/STATE.md` lines 117-129 (plus the blank line
130 that separates it from the next bullet), inside
`### Phase 15 planning notes (autonomous run — review these)`, opening with
`- **All three MUST-SPIKE questions resolved empirically**` and closing
`... Phase 20's quota floor is still the real cost control.`

Removal loses nothing, measured four ways:
1. The evidence is filed: `.planning/phases/15-transport-foundation/15-SPIKE-OQ1.md` (17.8K) and
   `15-01-SUMMARY.md` (`status: complete`) both exist on disk.
2. The bullet's only forward-looking clause — "remains plan 15-01 Task 1, the phase gate" — is
   discharged by that same `15-01-SUMMARY.md`.
3. Its other forward-looking clause — "Phase 20's quota floor is still the real cost control" —
   is also discharged: Phase 20 has executed (`20-RESEARCH.md`, `20-CONTEXT.md`,
   `20-05-SUMMARY.md` all exist), and `max-budget-usd` is recorded in 20+ planning documents
   including `20-CONTEXT.md`, `15-CONTEXT.md` and `.planning/REQUIREMENTS.md`.
4. `--setting-sources` (the OQ1 finding) is likewise recorded in 20+ documents including
   `15-SPIKE-OQ1.md` itself.

The section's "review these" framing was a request for human review of an autonomous Phase 15
planning run. That run is four phases behind the current position (Phase 19) and has been
reviewed and closed out.

**Absent — invent nothing.** `grep -n -iE 'worktree|OQ4|podman|OQ5' .planning/STATE.md` returns
**zero** lines. Commits `ef7dc9b` (Phase 17 OQ4) and `31954d5` (Phase 22 OQ5) each state
"STATE.md deliberately untouched". There is no bullet to remove for either. Plan and do nothing
for them.

**Absent as a decision bullet — do not delete.** `grep -n -i 'msrv' .planning/STATE.md` returns
two lines: the `Last activity:` line (line 67) and the `260915-f4n` row in
`### Quick Tasks Completed` (line 400). The table row is a historical log of a completed/halted
task, not a stale decision. **It stays.** The `Last activity:` line is routine end-of-task
bookkeeping owned by the quick workflow's own Step 7, outside this plan.
</part_b_authority>

<scope_fences>
Hard prohibitions. Each has a verify in Task 3 or Task 4.

1. **Do NOT edit `CLAUDE.md:99`** (the `notify ... MSRV 1.85` sources bullet). Only the
   tech-stack table row at `CLAUDE.md:25` changes, and in it only the version number — the rest
   of that row stays byte-for-byte.
2. **Do NOT create a `rust-toolchain.toml` or `rust-toolchain` file.** `rust-version` declares a
   *floor*; a toolchain file is a *pin*. Pinning would force every contributor onto exactly 1.88,
   hide latest-stable breakage, and impose a toolchain download per clone. Floor-in-manifest plus
   floor-tested-in-CI is the correct shape and is what this plan builds.
3. **Do NOT modify `Cargo.lock`.** Every cargo invocation in this plan carries `--locked`
   precisely so it cannot. The lockfile drift is the *finding*, not a thing to undo: pinning back
   to `ratatui 0.30.0` / `icu_properties 2.2.0` would not hold a 1.87 consumer floor anyway (the
   caret requirements still admit the 1.88-requiring patches for anyone resolving outside this
   lockfile) and it would fight the mandated release-process `cargo update` every milestone.
4. **Do NOT "fix" the known pre-existing test failures.** They are not regressions of this task:
   the `driver_reattach` pair (a spawn/liveness race, tracked in the pending todos) and a
   git-version constants assertion in the envelope policy suite that fails on this machine for an
   environmental reason (locally installed git differs from the pinned version). Record them as
   observed and move on. Also leave the pre-existing `unused_mut` warning at
   `tests/envelope_wrapper_class.rs:6127` alone.
5. **Do NOT add a new workflow file.** MSRV on every push/PR is a desirable follow-up needing its
   own `ci.yml`; it is a behaviour and cost change nobody asked for. Record it as a follow-up.
6. **Do NOT touch any part of `.planning/STATE.md` other than the single bullet named in
   `<part_b_authority>`.** Not Pending Todos, not other Decisions bullets, not Blockers/Concerns,
   not Session Continuity, not Current Position, not the Quick Tasks Completed table, not the
   remaining four bullets of the Phase 15 section, not its heading. The routine end-of-task
   bookkeeping row and `Last activity:` line are the orchestrator's Step 7, not this plan's work.
7. **Do NOT alter any existing step of the `publish` job** in `release.yml` other than adding its
   `needs:` key. The tag trigger and `permissions: contents: read` stay untouched.
</scope_fences>

<tasks>

<task type="tracer">
  <name>Task 1: Raise the floor at its source and gate publish on it — manifest to CI, end to end</name>
  <files>Cargo.toml, .github/workflows/release.yml</files>
  <precondition>The `1.88` rustup toolchain is installed (`rustup toolchain list` contains a `1.88-` entry). It was present at planning time; if it is missing, run `rustup toolchain install 1.88 --profile minimal` first. If that download fails (offline/proxy), halt and report — do not substitute a different toolchain and call the floor proven.</precondition>
  <action>
This is the thin end-to-end slice of the whole enforcement path: declared floor -> that exact
toolchain -> a real compile of every target against the committed lockfile. Land it first so the
documentation pass in Task 2 is describing something already true and already enforced, rather
than the other way round.

**1a. `Cargo.toml:5`** — change `rust-version = "1.87"` to `rust-version = "1.88"`.

**1b. `Cargo.toml` floor provenance — rewrite AND relocate.** The comment at lines 36-38 sits on
the `process-wrap` dependency and reads, in part, that that dependency's own
`rust-version = "1.87.0"` "is where the `[package] rust-version` floor above comes from". That
causal claim is false and was false before this bump. Do not bump the number inside it — the
whole sentence is wrong about which manifest sets the floor.
<!-- planner-discipline-allow: process-wrap -->

Split it:
  - **Leave on the dependency** only the sentence that is about the dependency: that `tokio1` is
    not a default feature and the crate is inert without it. Leave the adjacent
    no-line-framing-crate note alone.
  - **Move the floor provenance into the `[package]` block**, directly above `rust-version`,
    which is the item it is actually about. State there, as facts rather than as a claim about
    one dependency: the floor is the highest `rust-version` declared across the resolved
    dependency graph; it is currently 1.88.0, set by the `ratatui` 0.30.x family plus `time`,
    `darling` and the `icu_*` 2.3.x crates reached through `icu_properties`; it moves under an
    ordinary `cargo update` with no manifest edit, which is exactly how it drifted past the
    previously declared value unnoticed; and CI's `msrv` job is what now catches that. Include
    the one-line `cargo metadata ... | jq ... | sort -V | tail -1` reproduction command from
    `<measured_facts>` so the next reader re-measures instead of trusting the comment.
  - The text MUST contain the exact phrase `resolved dependency graph` (Task 2's gate anchors on
    it) and MUST NOT name `process-wrap` as a source of the floor.

**1c. `.github/workflows/release.yml`** — add a second job named `msrv`, and add `needs: msrv` to
the existing `publish` job. **The `needs:` edge is the load-bearing part** — without it the job
runs alongside publish and gates nothing, which is the usual way this exact pattern ships broken.

The `msrv` job, in the file's existing style (two-space indent, job-level `name:` string):
  - `runs-on: ubuntu-latest`
  - `actions/checkout@v4`, matching the ref style already in the file
  - `dtolnay/rust-toolchain@1.88.0` — the fully-qualified form. No `components:`: the job
    compiles, it does not lint.
  - `Swatinem/rust-cache@v2`, matching the existing job.
  - A manifest/toolchain agreement step, which must NOT be skipped — it is the key link that
    keeps a future floor bump from leaving CI certifying a stale one. Read the manifest floor via
    `cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].rust_version'`; read the
    installed compiler via `rustc --version`; reduce both to MAJOR.MINOR with
    `cut -d. -f1,2` so a `1.88` manifest value matches a `1.88.0` toolchain; fail with a
    `::error::` annotation naming both values and telling the reader to update the toolchain ref
    in this job. Treat an empty or `null` manifest value as a failure in its own right, with its
    own message — a missing `rust-version` must not silently compare equal to anything. Quote
    every expansion (threat T-hae-04).
  - The compile step, running `cargo check --all-targets --locked` — byte-identical to the
    command `260915-f4n` proved exits 0 on 1.88. `--all-targets` is deliberate and matches the
    project's published claim that the floor covers development including the test suite;
    `--locked` is deliberate and proves the floor against the committed lockfile, which is what a
    consumer gets.

**1d. Prove it locally before committing.** Run, through `rtk proxy` so the output is raw:

  rtk proxy cargo +1.88 check --all-targets --locked

It must exit 0. Record the exit code and `cargo +1.88 --version` for the SUMMARY. If it does not
exit 0, something changed since this plan was written: halt and report the verbatim first error
rather than editing the lockfile or lowering the claim.
  </action>
  <verify>
    <automated>set -e
grep -qx 'rust-version = "1.88"' Cargo.toml
test -z "$(grep -n '1\.87' Cargo.toml)"
sed -n '1,/^\[dependencies\]/p' Cargo.toml | grep -q 'resolved dependency graph'
test "$(git hash-object Cargo.lock)" = "6009a7b418c1295f5b3729577477095ae3195a02"
rtk proxy cargo +1.88 check --all-targets --locked
python3 -c "
import yaml
w=yaml.safe_load(open('.github/workflows/release.yml'))
j=w['jobs']
assert 'msrv' in j, 'no msrv job'
assert j['publish'].get('needs') in ('msrv',['msrv']), 'publish does not need msrv: %r' % j['publish'].get('needs')
steps=j['msrv']['steps']
uses=[s.get('uses','') for s in steps]
assert any(u.startswith('dtolnay/rust-toolchain@1.88') for u in uses), 'toolchain not pinned to 1.88: %r' % uses
runs=' '.join(s.get('run','') for s in steps)
assert 'cargo check --all-targets --locked' in runs, 'compile step is not the proven command'
assert 'rust_version' in runs and 'rustc --version' in runs, 'no manifest/toolchain agreement check'
assert 'cut -d. -f1,2' in runs, 'no MAJOR.MINOR reduction in the agreement check'
pub=j['publish']
assert pub['steps'][-1]['name']=='Publish', 'publish steps were reordered'
print('OK msrv gates publish; jobs=%r' % list(j))
"</automated>
  </verify>
  <done>`Cargo.toml` declares `rust-version = "1.88"`, contains no `1.87` literal, and carries the corrected floor provenance inside its `[package]` block; `Cargo.lock` is unchanged; `cargo +1.88 check --all-targets --locked` exits 0; `release.yml` parses as YAML, defines an `msrv` job on `dtolnay/rust-toolchain@1.88.0` that asserts manifest/toolchain agreement on MAJOR.MINOR before compiling, and `publish` carries `needs: msrv` with its own steps unreordered.</done>
</task>

<task type="auto">
  <name>Task 2: Bring the ten documentation MSRV sites up to the enforced floor</name>
  <files>README.md, CONTRIBUTING.md, CLAUDE.md, docs/DEVELOPMENT.md, docs/GETTING-STARTED.md, docs/TESTING.md, src/journal/redact.rs</files>
  <action>
Work from the inventory table in `<measured_facts>`, but **re-grep first** — run
`grep -rn '1\.87' README.md CONTRIBUTING.md CLAUDE.md docs/ src/` and treat the live result as
authoritative. The table was measured at planning time and should match; if it does not, the tree
moved and the extra site still needs treating.

**Version bump only** — change the number, touch nothing else on the line:
  - `README.md:61`, `README.md:186`
  - `CONTRIBUTING.md:23`
  - `docs/TESTING.md:14`
  - `docs/GETTING-STARTED.md:150` (the troubleshooting heading quoting the rustc error text)
  - `src/journal/redact.rs:218` (a doc comment; the `1.80` in the same sentence is about
    `LazyLock` stabilisation and is correct — leave it)
  - `CLAUDE.md:25` — the tech-stack table row. Change **only** `1.87+` to `1.88+`. The rest of
    that row's cells, spacing and pipe layout stay byte-for-byte. The prior quick task fenced
    `CLAUDE.md` out entirely; that fence does not apply here — the floor genuinely moved, so the
    number is wanted. `CLAUDE.md:99` remains untouched (scope fence 1).

**Bump AND rewrite the provenance** — three prose sites, which together are the third, and
fourth occurrences of a claim that is wrong twice over (wrong value AND wrong source
dependency), the first two having been handled in `Cargo.toml` by Task 1:
  - `docs/DEVELOPMENT.md:10-11` (Prerequisites bullet)
  - `docs/GETTING-STARTED.md:13` (the Rust toolchain table cell — note this is one long single
    line carrying the version, the `rust-version` restatement and the provenance together)
  - `docs/GETTING-STARTED.md:153-154` (the troubleshooting body under the heading bumped above)

A find-and-replace of the version number would leave all three sentences standing and still
pointing the reader at the wrong manifest. Replace the causal claim with the measured one, in
each site's own register (a prose bullet, a dense table cell, a troubleshooting paragraph — do
not paste one sentence into all three):
  - the floor is the highest `rust-version` declared across the **resolved dependency graph**,
    currently 1.88.0;
  - it is set by the `ratatui` 0.30.x family together with `time`, `darling` and the `icu_*`
    2.3.x crates reached through `icu_properties` — not by any single hand-chosen dependency;
  - it can move under an ordinary `cargo update` with no manifest edit, which is why CI's `msrv`
    job now verifies it before any publish.

Each of the three MUST contain the exact phrase `resolved dependency graph`, and **none of them
may mention `process-wrap` at all** — the gate below is a flat absence check across the docs and
`src/`, so do not preserve a "no longer the binding constraint" aside, however true. The place
that fact belongs is this task's SUMMARY.
<!-- planner-discipline-allow: process-wrap -->
<!-- planner-discipline-allow: 1.87 -->

The troubleshooting heading at `docs/GETTING-STARTED.md:150` quotes a rustc error string; keep it
quoted as an error message, just with the new version, so a user searching for the text cargo
actually prints still lands on it.
  </action>
  <verify>
    <automated>set -e
test -z "$(grep -rn '1\.87' Cargo.toml CLAUDE.md README.md CONTRIBUTING.md docs/ src/ .github/)"
test -z "$(grep -rn 'process-wrap' README.md CONTRIBUTING.md docs/ src/)"
test "$(grep -c 'resolved dependency graph' docs/DEVELOPMENT.md)" = "1"
test "$(grep -c 'resolved dependency graph' docs/GETTING-STARTED.md)" = "2"
grep -qx 'Requires \*\*Rust 1.88+\*\*.' README.md
grep -q '1.88+\*\* (for building from source)' README.md
grep -q 'Rust 1.88+ (stable)' CONTRIBUTING.md
grep -q '1.88+ stable' docs/TESTING.md
grep -q "MSRV is 1.88" src/journal/redact.rs
grep -q 'rustc 1.88 or newer' docs/GETTING-STARTED.md
grep -qF '| Rust | 1.88+ (stable) | Language | Zero-cost abstractions, single-binary distribution, no GC pauses, Cargo ecosystem |' CLAUDE.md
grep -q 'MSRV 1.85 (HIGH confidence)' CLAUDE.md
CLAUDE_NUMSTAT="$(git diff d11481d --numstat -- CLAUDE.md)" || { echo "FAIL: git diff errored"; exit 1; }
test "$(printf '%s' "$CLAUDE_NUMSTAT" | awk '{print $1"/"$2}')" = "1/1"
echo "OK all MSRV sites at 1.88; provenance rewritten in 3 prose sites; CLAUDE.md is a one-line change"</automated>
  </verify>
  <done>No tracked file outside `.planning/` contains the literal `1.87`; `process-wrap` appears in no doc or source file; the three prose provenance sites carry the `resolved dependency graph` anchor; the seven bump-only sites read 1.88; `CLAUDE.md` shows exactly one line added and one removed, with its notify sources bullet intact.</done>
</task>

<task type="auto">
  <name>Task 3: Prove no regression and that every Part A scope fence held</name>
  <files>(no repo files modified — verification only)</files>
  <action>
Run the project's normal gates under the default stable toolchain. Tasks 1 and 2 changed one
manifest line, one manifest comment, one CI file and nine doc/comment lines — a delta in the
build, test or clippy results would mean something unexpected happened and must be investigated,
not accepted.

Run all three through `rtk proxy` so the output is raw:

  rtk proxy cargo build
  rtk proxy cargo test --no-fail-fast
  rtk proxy cargo clippy -- -D warnings

`--no-fail-fast` is MANDATORY on the test run and is not a style preference. A plain `cargo test`
stops at the first failing test BINARY; `driver_reattach` fails (the documented pre-existing
pair) and sorts before every `envelope_*` binary, so a plain run never executes any envelope
suite and reports a stable-looking count regardless of what the tree contains.

Do NOT pipe any of these through `grep`, `head` or `awk` to extract counts. rtk already filters
cargo output, so a grep over a filtered stream succeeds vacuously and manufactures a false
number. Read the raw tail directly and record the pass/fail/ignored counts as printed.

The known pre-existing failures are listed in scope fence 4 — record them as observed, do not
fix them, and do not report them as this task's regressions.

Then confirm each Part A fence by command (the verify block below runs them):
  - no `rust-toolchain.toml` / `rust-toolchain` file was created
  - `Cargo.lock` is unchanged
  - `.planning/STATE.md` is untouched at this point (Task 4 owns it, and it has not run yet)
  - the set of tracked non-`.planning` files changed by Tasks 1-2 is exactly the nine expected
    ones and nothing else
  </action>
  <verify>
    <automated>set -e
rtk proxy cargo build
rtk proxy cargo clippy -- -D warnings
rtk proxy cargo test --no-fail-fast || echo "TEST_EXIT=$? (compare against the known pre-existing failures in scope fence 4)"
test ! -e rust-toolchain.toml
test ! -e rust-toolchain
echo "FENCE-2 ok: no toolchain pin file"
test "$(git hash-object Cargo.lock)" = "6009a7b418c1295f5b3729577477095ae3195a02" || { echo "FENCE-3 FAIL: Cargo.lock changed"; exit 1; }
echo "FENCE-3 ok: Cargo.lock byte-identical to the plan-time blob"
git diff --quiet d11481d -- .planning/STATE.md
echo "FENCE-6 ok: STATE.md untouched before Task 4"
CHANGED_RAW="$(git diff --name-only d11481d -- . ':!.planning')" || { echo "FENCE-7 FAIL: git diff errored"; exit 1; }
CHANGED="$(printf '%s\n' "$CHANGED_RAW" | sort | tr '\n' ' ')"
EXPECT="$(printf '%s\n' .github/workflows/release.yml CLAUDE.md CONTRIBUTING.md Cargo.toml README.md docs/DEVELOPMENT.md docs/GETTING-STARTED.md docs/TESTING.md src/journal/redact.rs | sort | tr '\n' ' ')"
test "$CHANGED" = "$EXPECT" || { echo "FENCE-7 FAIL: changed=[$CHANGED] expected=[$EXPECT]"; exit 1; }
echo "FENCE-7 ok: exactly the nine expected files changed"</automated>
  </verify>
  <done>`cargo build` and `cargo clippy -- -D warnings` exit 0; `cargo test --no-fail-fast` shows no failure beyond the documented pre-existing ones, with counts recorded verbatim for the SUMMARY; no toolchain pin file exists; `Cargo.lock` and `.planning/STATE.md` are unchanged; and Tasks 1-2 together changed exactly the nine expected tracked files.</done>
</task>

<task type="auto">
  <name>Task 4: Remove the one resolved Phase 15 MUST-SPIKE note from STATE.md</name>
  <files>.planning/STATE.md</files>
  <action>
**Re-read `.planning/STATE.md` before editing.** The line numbers in `<part_b_authority>` are
from planning time; anchor the edit on content, not on them.

Delete exactly one bullet from `## Deferred Verification` ->
`### Phase 15 planning notes (autonomous run — review these)`: the one opening
`- **All three MUST-SPIKE questions resolved empirically**` and running through its final
sentence about Phase 20's quota floor, together with the single blank line that separates it from
the following bullet. That is 14 lines removed and 0 added.
<!-- planner-discipline-allow: All three MUST-SPIKE questions resolved empirically -->

Its authority to go is recorded in `<part_b_authority>`: all three questions are CONFIRMED, both
of its forward-looking clauses are discharged, and its content survives in
`15-SPIKE-OQ1.md`, `15-01-SUMMARY.md` and twenty-plus other planning documents. The section's
"review these" framing was a request for human review of a Phase 15 autonomous run that is four
phases behind the current position and has been closed out.

**Nothing else moves.** The `### Phase 15 planning notes` heading stays. The other four bullets in
that section stay (the structural `type:"result"` finding, the decision-coverage gate note, the
ROADMAP UI-hint note, the `--research-phase` misreading note, the raw-transcripts note). The
Phase 17 and Phase 14 sections stay. Decisions, Pending Todos, Blockers/Concerns, Current
Position, Session Continuity, Performance Metrics and the whole Quick Tasks Completed table —
including the `260915-f4n` row, which is a historical log entry and not a stale decision — all
stay. Do not reflow, re-wrap or re-indent any surviving line.

Do **not** add a row for this task or update `Last activity:` here. That bookkeeping is the quick
workflow's own Step 7 and runs outside this plan.

Nothing is planned for the Phase 17 OQ4 / Phase 22 OQ5 topics: grep confirms no bullet for either
exists in this file, and both closing commits state STATE.md was deliberately left untouched.
Inventing one would be worse than the gap.
  </action>
  <verify>
    <automated>set -e
STATE_NUMSTAT="$(git diff d11481d --numstat -- .planning/STATE.md)" || { echo "FAIL: git diff errored"; exit 1; }
test "$(printf '%s' "$STATE_NUMSTAT" | awk '{print $1"/"$2}')" = "0/14"
test "$(grep -c 'All three MUST-SPIKE questions resolved empirically' .planning/STATE.md)" = "0"
test "$(grep -c '^### Phase 15 planning notes' .planning/STATE.md)" = "1"
test "$(grep -c 'Structural finding no research document anticipated' .planning/STATE.md)" = "1"
test "$(grep -c 'Decision-coverage gate is live again' .planning/STATE.md)" = "1"
test "$(grep -c 'ROADMAP gained an authoritative' .planning/STATE.md)" = "1"
test "$(grep -c 'was interpreted as' .planning/STATE.md)" = "1"
test "$(grep -c 'Seven raw spike transcripts staged' .planning/STATE.md)" = "1"
test "$(grep -c '^### Phase 17 gap-closure notes' .planning/STATE.md)" = "1"
test "$(grep -c '^### Phase 14 planning notes' .planning/STATE.md)" = "1"
test "$(grep -c '260915-f4n' .planning/STATE.md)" -ge "1"
test "$(grep -c '^### Pending Todos' .planning/STATE.md)" = "1"
test "$(grep -c '^### Blockers/Concerns' .planning/STATE.md)" = "1"
echo "OK one bullet removed (0 added / 14 removed); section heading, four sibling bullets, other sections and the f4n log row all intact"</automated>
  </verify>
  <done>`git diff --numstat` on `.planning/STATE.md` reports exactly `0 14`; the MUST-SPIKE bullet is gone; the section heading, its four remaining bullets, the Phase 17 and Phase 14 sections, Pending Todos, Blockers/Concerns and the `260915-f4n` log row are all still present and unreflowed.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| GitHub Actions runner -> crates.io | The `publish` job holds `CARGO_REGISTRY_TOKEN`; anything running in that workflow is inside the blast radius of a publish credential. |
| Third-party GitHub Action -> workflow runner | `dtolnay/rust-toolchain` executes code in the same workflow as that token. |
| Cargo.toml contents -> shell in the `msrv` job | A manifest value is read with `cargo metadata` and interpolated into a shell comparison. |
| Declared MSRV -> downstream consumers | Raising the floor changes the published consumer contract; anyone on 1.87 loses the ability to build this crate from crates.io. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-hae-01 | Elevation of Privilege | `msrv` job co-resident with `publish` in release.yml | medium | mitigate | The `msrv` job declares no `env:` and receives no secret; `CARGO_REGISTRY_TOKEN` stays scoped to the `Publish` step that already had it. Adding a job does not widen secret exposure because Actions secrets reach a step only through an explicit `env:`, and the new job sets none. Task 1's verify asserts the publish step list is unreordered. |
| T-hae-02 | Tampering | `dtolnay/rust-toolchain@1.88.0` (mutable branch ref) | medium | accept | A version-named branch is mutable, so this is not a content pin. Accepted because it is the posture the file already has — `@stable`, `actions/checkout@v4` and `Swatinem/rust-cache@v2` are all mutable refs — and pinning one of four to a SHA while three float buys no real reduction. Carried forward as a follow-up: SHA-pin all four in one pass. |
| T-hae-03 | Denial of Service | raising the declared floor to 1.88 | low | accept | Consumers on 1.87 can no longer build from crates.io. Accepted as the deliberate, human-made decision this task exists to execute: the crate already could not be built on 1.87 against its own lockfile, so the floor claim was the fiction and the bump only makes the published contract honest. |
| T-hae-04 | Injection | `cargo metadata` `rust_version` interpolated into the agreement step's shell | low | mitigate | The value is only ever compared and used to fail the job; it never reaches a command position. Every expansion is quoted, and an empty or `null` value fails with its own message rather than comparing equal. |
| T-hae-05 | Repudiation | deleting a STATE.md bullet | low | mitigate | The deletion is one commit with a message naming what was removed and why, and the removed content survives in `15-SPIKE-OQ1.md` and `15-01-SUMMARY.md`. Task 4's verify pins exactly `0 14` added/removed, so a wider deletion cannot pass as this one. |
| T-hae-SC | Tampering | npm/pip/cargo installs | n/a | n/a | This plan adds **no** package to `Cargo.toml` and modifies no dependency requirement; `Cargo.lock` is fenced unchanged. The only download is a rustup toolchain, which is not a package install. The Package Legitimacy Gate does not apply. |
</threat_model>

<verification>
1. `cargo +1.88 check --all-targets --locked` exits 0 against the committed lockfile — measured, not asserted.
2. `release.yml` parses, defines `msrv`, and `publish` carries `needs: msrv`; the job fails loudly if `Cargo.toml`'s `rust_version` ever stops matching the toolchain it installs.
3. The literal `1.87` survives in no tracked file outside `.planning/`, and `process-wrap` is named in no doc or source file.
4. The three prose provenance sites plus the `Cargo.toml` `[package]` block all carry the `resolved dependency graph` anchor.
5. `cargo build` and `cargo clippy -- -D warnings` green under stable; `cargo test --no-fail-fast` shows only the documented pre-existing failures, with counts recorded.
6. `.planning/STATE.md` shows exactly `0 14` added/removed and every named survivor is still present.
</verification>

<success_criteria>
The MSRV stops being a claim and becomes a gate: declared at a value the crate can actually be
compiled on, re-proven in CI before any crates.io publish, self-correcting against future manifest
bumps, and described everywhere by the dependency graph that really sets it rather than by a
dependency that never did. Separately and atomically, one closed question stops posing as an open
one in STATE.md.
</success_criteria>

<output>
Create `.planning/quick/260915-hae-bump-declared-msrv-to-1-88-ship-msrv-ci-/260915-hae-SUMMARY.md` when done.

Commit Part A and Part B separately, per the atomic-commit convention — Tasks 1-2 are the MSRV
change (manifest + CI, then docs), Task 4 is the STATE.md cleanup, and they share no file.

The SUMMARY must record:
- The verbatim Task 1 local measurement: `cargo +1.88 --version` and the exit code of
  `cargo +1.88 check --all-targets --locked`.
- The measured `cargo test --no-fail-fast` pass/fail/ignored counts, read from raw output, with
  the known pre-existing failures named as such.
- The corrected provenance fact and the one-line command that re-measures it, so the next floor
  drift is diagnosable in one step.
- That `process-wrap` 9.1.0 does declare `rust-version = "1.87.0"` — a true fact that was simply
  never the binding constraint — since Task 2 removes that name from the docs entirely and this
  is the one place the fact should survive.
- That `CLAUDE.md:99`'s `notify ... MSRV 1.85` bullet was deliberately left alone.
- For Part B: the single bullet removed, the four-way evidence that removal lost nothing, and
  that the Phase 17 OQ4 and Phase 22 OQ5 topics had **nothing** in STATE.md to remove — so
  nothing was invented for them.
- Follow-ups deliberately not taken: MSRV on every push/PR (needs its own `ci.yml`, scope fence
  5); SHA-pinning all four GitHub Actions in one pass (T-hae-02); the pre-existing `unused_mut`
  warning at `tests/envelope_wrapper_class.rs:6127`.

**Also update `.planning/quick/260915-f4n-bump-msrv-from-1-85-to-1-87-in-cargo-tom/260915-f4n-SUMMARY.md`
so it no longer reads as blocking.** It currently carries `outcome: halted-at-task-1-by-decision-rule-b`
and a "Blocked on a human decision" section, which is now false. Make the minimal honest edit:
add a `superseded_by: quick-260915-hae` frontmatter key, change `outcome` to record that the fork
it escalated was decided and shipped here, and add a one-line banner at the top of its body
pointing at this task's SUMMARY. **Do not rewrite its measurements** — they are the evidence this
task acted on, and they were correct.
</output>
