---
phase: quick-260915-hsh
plan: 01
subsystem: build/dependencies
status: complete
tags: [dependencies, cargo-update, msrv, supply-chain, process-wrap, dirs]
requires: []
provides:
  - "Cargo.lock at latest semver-compatible across the whole graph"
  - "dirs 7.0.0"
  - "process-wrap 10.0.0"
  - "MSRV floor re-measured at 1.88.0 and verified on both sides"
affects:
  - Cargo.toml
  - Cargo.lock
tech_stack:
  added: []
  removed: []
  patterns:
    - "Every version claim is read out of Cargo.lock, never out of Cargo.toml (I-03)"
    - "A major bump's breaking changes are checked by diffing the two vendored crate sources, not inferred from a passing compile"
key_files:
  created: []
  modified:
    - Cargo.toml
    - Cargo.lock
decisions:
  - "dirs 7.0.0 accepted after crates.io publisher continuity was proved: 5.0.0 through 7.0.0 all published by `soc`, still sole owner, despite the repository moving to codeberg"
  - "process-wrap 10.0.0 accepted after diffing 9.1.0 vs 10.0.0 sources: the removed panicking accessors (inner_child/inner_child_mut/into_inner_child) are called nowhere in this tree, and ProcessGroupChild's signal/start_kill/kill/wait are unchanged"
  - "MSRV floor held at 1.88.0 — no file moved, CLAUDE.md never opened (I-02)"
  - "generic-array and unicode-width left behind latest; both are held by requirements outside this manifest, disclosed rather than forced"
metrics:
  duration: 17min
  completed: 2026-09-15
actuals:
  tokens: 6000
  tasks: 3
  commits: 3
plan_head_before: d51642b530cc0a29907f188708d99ab8e9b04622
---

# Quick Task 260915-hsh: Update All Cargo Dependencies to Latest — Summary

Advanced the entire Cargo dependency graph to latest in three independently revertable
commits: 64 semver-compatible lockfile moves, then `dirs` 6.0.0 → 7.0.0, then
`process-wrap` 9.1.0 → 10.0.0. Neither escape hatch fired, no source file changed, and the
MSRV floor was re-measured after the updates and held at 1.88.0.

## Commits

| Commit | Task | What |
|--------|------|------|
| `6d1fdcd` | 1 (tracer) | `build(quick-260915-hsh): advance the lockfile to latest semver-compatible versions` |
| `8f8b36a` | 2 | `build(quick-260915-hsh): bump dirs 6 -> 7` |
| `bd4d1d8` | 3 (Part A) | `build(quick-260915-hsh): bump process-wrap 9.1 -> 10.0` |

Part B produced no commit — the measured floor did not move. Each commit touches only
`Cargo.lock` (task 1) or `Cargo.toml` + `Cargo.lock` (tasks 2 and 3), so each reverts alone.

## Gate Numbers — MEASURED, not asserted

All runs are `rtk proxy cargo ...` with output redirected to
`/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/99121bc3-2602-491c-9020-a4abc0ec3173/scratchpad/`,
never piped. Every suite number comes from `cargo test --no-fail-fast`.

| Stage | build | clippy -D warnings | passed | failed | ignored | binaries | log |
|-------|-------|--------------------|--------|--------|---------|----------|-----|
| **Baseline** (unmodified HEAD `d51642b`, this session) | 0 | 0 | **2002** | **4** | 15 | 48 | `base-*.log` |
| After task 1 (lockfile) | 0 | 0 | 2003 | 3 | 15 | 48 | `t1-*.log` |
| After task 2 (dirs 7) | 0 | 0 | 2005 | 1 | 15 | 48 | `t2-*.log` |
| After task 3 (process-wrap 10) | 0 | 0 | 2004 | 2 | 15 | 48 | `t3a-*.log` |
| **Final** | 0 | 0 | **2005** | **1** | 15 | 48 | `final-*.log` |

### Failing test NAMES

Baseline set (4 names):

1. `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
2. `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` (`driver_reattach`)
3. `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired` (`driver_reattach`)
4. `the_t_19_116_replacement_takes_layer_2_as_well_and_that_is_measured_separately`

Final set (1 name): #1 only.

**The final set is a strict SUBSET of the baseline set. No new failing name appeared at any
stage.** Names 2 and 3 are the tracked `driver_reattach` spawn-artifact race and flipped
between runs (both failed at baseline, both passed under task 2, one failed under task 3,
neither failed in the final run) — flakiness, unchanged by this task. Name 4 failed at
baseline and passed at every later stage; it was not fixed by anything here, it is simply
not stable. Name 1 is environmental and reproduces on unmodified HEAD: `src/envelope/policy.rs:10520`
asserts the installed git matches `CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION`, and
this machine runs **git 2.53.0**. The assertion's own panic text calls itself "a SCHEDULE,
not a CONTROL" and says the correct response is to re-derive both constants against the new
git — out of scope for a dependency update, and untouched here.

All **17** `driver_*` and all **19** `envelope_*` binaries executed at every stage, including
`driver_kill`, `driver_kill_startup`, `executor_lifecycle` and `executor_transport`. That is
what `--no-fail-fast` buys: a plain `cargo test` stops at `driver_reattach` and never reaches
any `envelope_*` binary.

## Task 1 — semver-compatible advance (`6d1fdcd`)

`cargo update`, no manifest edit. **64 packages relocked.** Notables:
`clap`/`clap_builder`/`clap_derive` 4.6.4 → 4.6.7, `darling` family 0.23.0 → 0.24.1,
`futures` family 0.3.33 → 0.3.34, `uuid` 1.24.0 → 1.26.1, `thiserror` 2.0.19 → 2.0.20,
`time` 0.3.54 → 0.3.55, `pest` family 2.8.8 → 2.9.1, `cc` 1.3.0 → 1.4.6,
`wasm-bindgen` 0.2.126 → 0.2.128, `smallvec` 1.15.2 → 1.16.1, `either` 1.16.0 → 1.18.0,
`ignore` 0.4.31 → 0.4.33, `palette` 0.7.6 → 0.7.7, `log` 0.4.33 → 0.4.34,
`regex-automata` 0.4.16 → 0.4.18, `mio` 1.2.2 → 1.2.3, `libredox` 0.1.18 → 0.1.24.
Full list: `t1-update.log`.

Several crates the plan expected to move (`tokio`, `serde`, `serde_json`, `anyhow`, `chrono`,
`tempfile`, `icu_properties`, `notify`, `regex`, `tracing-subscriber`) do **not** appear in
the update log because the lockfile already carried them at those versions. The plan said to
treat its list as expected shape rather than a checklist; the resolver decided.

**No crate was pinned back under I-04.** Nothing broke.

### T-hsh-SC — one package NAME appeared for the first time

`palette_math` v0.7.7 was **added** and `fast-srgb8` v1.0.0 **removed**: `palette` 0.7.7
replaced the third-party sRGB math crate with a first-party sub-crate. The threat model
calls a newly-appearing name a blocking finding, so the legitimacy check was run before the
commit:

| Crate | crates.io owner | Repository |
|-------|-----------------|------------|
| `palette` | `Ogeon` (Erik Hedvall) | github.com/Ogeon/palette |
| `palette_math` | `Ogeon` (Erik Hedvall) | github.com/Ogeon/palette |
| `fast-srgb8` (removed) | `thomcc` (Thom Chiovoloni) | github.com/thomcc/fast-srgb8 |

Same sole owner, same repository, first published 2026-08-02. Publisher continuity holds;
finding cleared. It is reached transitively through `ratatui` → `palette`. No other package
name appeared across all three tasks (tasks 2 and 3 each relocked exactly 1 package).

### T-hsh-04 — release-candidate guard

Measured on the final lockfile: **0** occurrences of `rc.`, and **0** lines matching
`^version = "N.N.N-"` (any pre-release, not just rc). `notify` resolves **8.2.0**,
`notify-debouncer-full` resolves **0.7.0**. Both requirements in `Cargo.toml` are still
`"8"` and `"0.7"` — untouched.

## Task 2 — `dirs` 6.0.0 → 7.0.0 (`8f8b36a`)

**Escape hatch: did NOT fire.** **Version proved from `Cargo.lock`:** `name = "dirs"` /
`version = "7.0.0"`; `dirs-sys` unchanged at 0.5.0.

### T-hsh-01 — publisher continuity, checked BEFORE the requirement was edited

The `dirs` repository field now reads `codeberg.org/dirs/dirs-rs` because
`github.com/dirs-dev/dirs-rs` was archived, and a repository move is externally
indistinguishable from a hijack. crates.io `published_by` per version:

| Version | Published by | Date | Yanked |
|---------|--------------|------|--------|
| 7.0.0 | `soc` (id 2022) | 2026-09-05 | no |
| 6.0.0 | `soc` (id 2022) | 2025-01-12 | no |
| 5.0.1 | `soc` (id 2022) | 2023-04-30 | no |
| 5.0.0 | `soc` (id 2022) | 2023-03-19 | no |

`soc` is still the crate's sole owner, and `dirs-sys` has the same sole owner. **Continuity
holds — proceeded.**

### Call sites

**Zero source files changed.** 7.0.0 still exports `home_dir`, `data_local_dir` and
`config_dir` with the same `Option<PathBuf>` signatures, so all seven production call sites
(`state_reader/queue_md.rs`, `project_creator.rs`, `journal/redact.rs`, `main.rs`,
`envelope/mod.rs:215`, `config.rs:331`) compile unchanged and every deliberate None-branch
choice is preserved by construction — nothing was "improved". The security-relevant pair is
covered by measurement rather than by argument: all 19 `envelope_*` binaries executed green,
including `envelope_control_carrier`, which pins the guard registry's behaviour when
`config_dir()` is None.

## Task 3 Part A — `process-wrap` 9.1.0 → 10.0.0 (`bd4d1d8`)

**Escape hatch: did NOT fire.** `Box<dyn ChildWrapper>` still exists and is still storable in
a struct field, so the executor's ownership model is intact. **Version proved from
`Cargo.lock`:** `name = "process-wrap"` / `version = "10.0.0"`.

**Zero source files changed** — and because a passing compile is not evidence that a kill
semantic held, the three breaking changes were read by diffing the vendored 9.1.0 and 10.0.0
sources in `~/.cargo/registry`:

| Breaking change | What it actually removed | This tree |
|---|---|---|
| "Remove panicking child accessors" / "Replace panicking accessors with fallible ones" | `dyn ChildWrapper::inner_child`, `inner_child_mut`, `into_inner_child` → `try_*` | **Calls none of them.** The pipe accessors at `claude.rs:602-613` — `stdin()`, `stdout()`, `stderr()` — still return `&mut Option<_>`, byte-identical to 9.1.0, so `.take().ok_or(SpawnError::PipeUnavailable { pipe })` stands unchanged and no new error variant was invented. |
| "Make stored wrapper operations type-safe" | internal wrapper registry typing | `CommandWrap::with_new` / `wrap()` / `spawn()` keep their shapes; builder at `claude.rs:480-574` untouched. |
| "Make wrapper extension typed" | `generic_wrap.rs` public surface only **gains** `spawn_with_child` | no removal reached us. |

**Teardown surface — the part that matters.** `src/tokio/process_group.rs` differs between
the two versions in exactly three lines, all inside `inner()` / `inner_mut()` /
`into_inner()` (the "expose direct process-group children" bugfix). `ProcessGroupChild`'s
`signal`, `start_kill`, `kill` and `wait` are **unchanged**, and `src/tokio/kill_on_drop.rs`
does not differ at all. So the SIGTERM → grace → SIGKILL ordering and the "an observed exit
is not a reaped group" discipline (CR-01..CR-06) are preserved by the dependency itself, not
merely by our call sites. `child.id()` remains the pgid route, and `SIGTERM` stays a local
`i32` — `signal(&self, sig: i32)` is unchanged, so no typed-signal API forced a `nix`/`libc`
dependency.

`driver_kill`, `driver_kill_startup`, `executor_lifecycle` and `executor_transport` all
green; no `driver_*` name failed that was passing at baseline.

## Task 3 Part B — MSRV floor re-measured

```
cargo metadata --format-version 1 --locked \
  | jq -r '.packages[].rust_version | select(. != null)' | sort -V | tail -1
```

**Measured: `1.88.0`** (`msrv-measured.txt`). `Cargo.toml`'s declared `rust-version = "1.88"`
already equals it. **The floor did NOT move, so nothing was edited** — no MSRV commit, and
per I-02 `CLAUDE.md` was never opened at all. `git diff --name-only d51642b..HEAD` returns
exactly `Cargo.lock` and `Cargo.toml`; the `notify` 8.x-not-9.0-rc guidance is untouched
(9.0 is still a release candidate), the crate's own `version` is still `1.6.0`, and
`.planning/STATE.md` was not modified.

This is a real result, not a non-event: the FLOOR PROVENANCE comment exists because the value
drifted silently once before, and task 1 performed exactly the event that drifts it. Verified
by measurement on **both** sides rather than asserted:

- `cargo +1.88 check --all-targets --locked` → **exit 0**
- `cargo +1.87 check --all-targets --locked` → **exit 101**, `rustc 1.87.0 is not supported
  by the following packages: darling@0.24.1, darling_core@0.24.1, darling_macro@0.24.1
  requires rustc 1.88.0`

Both run with a scratchpad `CARGO_TARGET_DIR` so the working build cache was not disturbed.

The set of crates setting the floor **did** shift under the update even though the value did
not: `darling` 0.24.1 (new to 1.88 — it was 0.23.0 before), `ignore` 0.4.33 and
`instability` 0.3.13 join the `ratatui` 0.30.x family, the `time` 0.3.55 family and the
`icu_*` 2.3.x crates. The provenance comment still names a correct, if no longer exhaustive,
sample, and was left as written because the plan scopes a comment refresh to a floor that
rose. `process-wrap` 10.0.0 declares `rust-version = "1.87.0"`, below the floor, so it did
not raise anything.

## Task 3 Part C — transitive holds, disclosed

After all three tasks, `cargo update --dry-run --verbose` reports exactly two crates still
behind latest — `dirs` and `process-wrap` are gone from that list:

**`generic-array` 0.14.7 (available 0.14.9).** `cargo tree -i generic-array` reports
*"nothing to print"* — it is **not a compiled unit**. It is an orphan lockfile entry in the
same class as the `sha2 0.10.9` orphan already documented in `Cargo.toml`: reachable only
through the equally-orphaned `crypto-common 0.1.7` (which requires `=0.14.7`, an exact pin,
and itself prints nothing) and `block-buffer 0.10.4` (`^0.14`) — the sha2 0.10.x family
entries left by an unenabled `ratatui` backend feature. So the exact pin in `crypto-common
0.1.7` is what holds it, and nothing in the compiled graph depends on it either way.

**`unicode-width` 0.2.0 (available 0.2.2).** Held by **`ratatui` 0.29.0, which requires
`=0.2.0`** — an exact pin. ratatui 0.29.0 is in the graph because our direct dependency
`tui-textarea` 0.7.0 still depends on it, *alongside* our own `ratatui` 0.30.2. The other
dependents impose no upper bound: `ratatui-core` 0.1.2 and `ratatui-widgets` 0.3.2 require
`>=0.2.0`, `unicode-truncate` 2.0.1 requires `^0.2`, `tui-textarea` 0.7.0 requires `^0.2.0`.
(`clap_builder` 4.6.7 declares `^0.2.2` but its `unicode` feature is off, so it is not in the
resolved tree.) The same ratatui-0.29 tail also keeps a second entry, `unicode-width`
0.1.14, alive via `unicode-truncate` 1.1.0. **This clears when `tui-textarea` ships a
ratatui-0.30 release** — not before, and not by anything editable in this manifest.

No action taken on either. This section exists so the next person running `cargo update`
does not re-investigate them.

## Deviations from Plan

None. All three tasks executed as written; both escape hatches were evaluated and neither
fired; no crate was pinned back under I-04.

## Decisions Inferred for Audit

The human was unavailable. The plan's four pre-registered inferences (I-01 process-wrap last,
I-02 CLAUDE.md scoped to the MSRV cell, I-03 bumps proved from the lockfile, I-04 build
breakage in scope for task 1) were all honoured; I-02 and I-04 turned out to be no-ops
because the floor held and nothing broke. One additional call was made at execution time:

- **The T-hsh-SC "blocking finding" on `palette_math` was cleared by evidence rather than
  escalated.** The threat model says a newly-appearing package name is blocking, and the
  deviation rules say a *failed or unverifiable* package is a human checkpoint. Here the name
  was verifiable from crates.io in one query — same sole owner and same repository as the
  `palette` crate it splits out of — so it was settled with evidence and recorded, not handed
  up. Escalating a check the agent can perform would have been the wrong call.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path, file access pattern or schema change; the only
security-relevant surface touched is `process-wrap`'s kill path, which was checked by source
diff (see Task 3 Part A) and found unchanged.

## Self-Check: PASSED

- `.planning/quick/260915-hsh-update-all-cargo-dependencies-to-latest-/260915-hsh-SUMMARY.md` — FOUND
- Commit `6d1fdcd` — FOUND
- Commit `8f8b36a` — FOUND
- Commit `bd4d1d8` — FOUND
- `git status --short` shows no unstaged source change and no half-migrated file; the only
  dirt is the three pre-existing items that were present before this task started.
