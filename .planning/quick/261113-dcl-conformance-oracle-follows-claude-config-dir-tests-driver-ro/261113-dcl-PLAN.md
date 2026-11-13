---
phase: quick-261113-dcl
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - tests/driver_router_conformance.rs
  - scripts/install-conformance-oracle.sh
  - docs/DEVELOPMENT.md
  - src/state_reader/gsd_install.rs
  - src/state_reader/queue_md.rs
autonomous: true
requirements: [QUICK-261113-dcl]

must_haves:
  truths:
    - "Default behaviour is unchanged. With CLAUDE_CONFIG_DIR unset, empty, whitespace-only or relative, the provisioning script verifies or installs `$HOME/.claude/gsd-core`, and the conformance oracle probes `<home>/.claude/gsd-core/bin/gsd-tools.cjs` first. The publish job (release.yml) and `./scripts/pre-tag-check.sh --container` never set CLAUDE_CONFIG_DIR, so both resolve exactly as before."
    - "An absolute CLAUDE_CONFIG_DIR, or a `~`/`~/…` one (expanded against home), replaces the home default. The script verifies or installs `$CLAUDE_CONFIG_DIR/gsd-core`, with the don't-clobber rule intact, and the oracle resolves there. `~/.claude` is not consulted as an extra candidate."
    - "The oracle reads the TUI's own candidate list. `oracle_candidates` delegates to `install_candidates(None, roots)`, and `Oracle::resolve` passes `InstallRoots::from_env()`. The test file no longer reads HOME or CODEX_HOME from the process itself."
    - "A relative override counts as unset in the shared resolver (`InstallRoots::config_home`), judged after `~` expansion. This holds for CLAUDE_CONFIG_DIR and CODEX_HOME alike. The TUI's smart-entry launcher, the installed-version detector, the conformance oracle and the provisioning script therefore agree on one rule (I-1, I-2, I-3)."
    - "Gates: the full suite passes at least 3830 tests with 0 failed (HEAD: 3827/0). `cargo clippy --all-targets -- -D warnings` and `cargo fmt --all -- --check` are clean, as are check and clippy for every release.yml matrix target. `bash -n` passes on the script, and `git status` is clean apart from coordinator-owned artifacts."
  artifacts:
    - path: tests/driver_router_conformance.rs
      provides: "oracle_candidates(&InstallRoots) delegating to install_candidates; Oracle::resolve via InstallRoots::from_env; tests the_oracle_follows_claude_config_dir and a_relative_claude_config_dir_counts_as_unset_for_the_oracle"
    - path: scripts/install-conformance-oracle.sh
      provides: "CLAUDE_DIR resolved from CLAUDE_CONFIG_DIR (non-blank, ~ expanded, absolute) else $HOME/.claude; banner line naming the source"
    - path: src/state_reader/gsd_install.rs
      provides: "InstallRoots::config_home treats a relative (after ~ expansion) override as unset; unit test a_relative_override_falls_back_to_the_home_default"
    - path: docs/DEVELOPMENT.md
      provides: "script description names ${CLAUDE_CONFIG_DIR:-~/.claude}/gsd-core"
  key_links:
    - from: tests/driver_router_conformance.rs
      to: src/state_reader/gsd_install.rs
      via: "oracle_candidates -> install_candidates(None, roots); Oracle::resolve -> InstallRoots::from_env()"
    - from: src/state_reader/queue_md.rs
      to: src/state_reader/gsd_install.rs
      via: "gsd_tools_candidates projects install_candidates, so the TUI and the oracle share one list"
    - from: scripts/install-conformance-oracle.sh
      to: tests/driver_router_conformance.rs
      via: "CLAUDE_DIR rule mirrors InstallRoots::claude_home, so the provisioned ORACLE_DIR is the first global path oracle_candidates() probes"
    - from: .github/workflows/release.yml
      to: scripts/install-conformance-oracle.sh
      via: "Provision step runs the script with CLAUDE_CONFIG_DIR unset, so the home default applies (unchanged)"
---

<objective>
Make the router-conformance oracle and its provisioning script follow `CLAUDE_CONFIG_DIR`, as the TUI's gsd-tools resolver already does. `tests/driver_router_conformance.rs` `Oracle::resolve`/`oracle_candidates` reads only HOME and CODEX_HOME today. Its doc comment claims to mirror the TUI's order, but the TUI (`src/state_reader/queue_md.rs` via `InstallRoots::from_env`) puts `${CLAUDE_CONFIG_DIR:-~/.claude}` first. `scripts/install-conformance-oracle.sh` hard-codes the home default for where it verifies or installs the oracle.

Purpose: on a machine where Claude runs with `CLAUDE_CONFIG_DIR` (claude-profile setups), the test and the script must agree with GSD and with the TUI about which gsd-tools is "the" install. They must also stay identical to today where the variable is unset: the CI publish job and `pre-tag-check.sh --container`.

Output: the oracle delegates to the shared resolver. The script resolves CLAUDE_DIR by the same rule. The shared resolver treats a relative override as unset. Docs and comments are updated, and regression tests are added.

**Inferred decisions (user unavailable). Each is marked `[inferred — audit]`; record them in the SUMMARY:**
- **I-1 [inferred — audit]: "blank/relative = unset" lives in the shared resolver, and the oracle consumes that resolver.** The item text says the TUI resolver already treats a relative override as unset. It does not. `InstallRoots::config_home` (`src/state_reader/gsd_install.rs` ~L96-101) treats only a blank (trimmed) value as unset and expands `~`. Any other value is used literally, so a relative one resolves against the cwd. The only way to satisfy both "align with the TUI resolver" and "blank/relative = unset" is to add the relative rule to `config_home` and let the oracle call it. This also fixes a latent TUI defect. `resolve_gsd_tools_from` checks a relative candidate with `is_file()` in the TUI's own cwd, then hands the relative path to `node` with `current_dir(project_root)`, so smart-entry fails and silently drops to the keyword heuristic. The oracle would hit the same defect: `is_file` runs in the crate root while `node` runs in the fixture. Precedent: `agents::adapters::claude::resolve_config_root` already ignores an empty or relative value (T-25-12), and Claude Code refuses a relative CLAUDE_CONFIG_DIR. Reversible (one filter).
- **I-2 [inferred — audit]: the rule covers CODEX_HOME too.** `config_home` is the single helper for both overrides, and the cwd defect is identical for a relative CODEX_HOME. This deliberately diverges from upstream `gsd-run-resolver.md`, which uses `${CODEX_HOME:-$HOME/.codex}` verbatim. It is recorded in queue_md.rs's "Known, deliberate differences" list.
- **I-3 [inferred — audit]: "relative" is judged after `~` expansion.** `~` and `~/x` still resolve against home, as the TUI does today. `~user/x` stays literal, is therefore relative, and is ignored. The script mirrors this exactly.
- **I-4 [inferred — audit]: the oracle's home comes from `InstallRoots::from_env()` (`dirs::home_dir()`)** instead of raw `$HOME`. This is the TUI's own source, and it gives the same answer in CI and in the container, where HOME is set.
- **I-5 [inferred — audit], intended consequence:** a developer whose CLAUDE_CONFIG_DIR has no `gsd-core` (and who has no `~/.codex` install and no `gsd-tools` on PATH) now gets the conformance failure, just as GSD and the TUI would load nothing from `~/.claude`. The remedy is the script, which now provisions into `$CLAUDE_CONFIG_DIR/gsd-core`. On this machine the profile dir's `gsd-core` is a symlink to the home-default install, so local runs stay green (measured at planning time).
- **I-6 [inferred — audit]: fix along the way.** The script's three line-number references into the test file (`…rs:54`, `232-246`, `91-115`) are stale. They become symbol names.

**Cross-item note for the coordinator:** sibling 261113-dck's text assumes `InstallRoots::config_home` already treats relative as unset. This plan makes that true. There is no file overlap with 261113-dck (`src/launcher/*`, `src/oneoff/*`) or 261113-dcj (`src/agents/adapters/claude.rs`, read-only here), so `depends_on: []`.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@tests/driver_router_conformance.rs
@scripts/install-conformance-oracle.sh
@src/state_reader/gsd_install.rs
@src/state_reader/queue_md.rs

Planning-time facts (measured 2026-11-13 at HEAD 7c857d5):
- `install_candidates` and `InstallRoots` (pub fields, `from_env`, `claude_home`, `codex_home_dir`) are public: `gsd_meta_manager::state_reader::gsd_install::{install_candidates, InstallRoots}`. `gsd_tools_candidates` in queue_md.rs is private but is a pure projection of `install_candidates`. With `project_root = None`, `install_candidates` returns exactly the two global candidates, Claude then Codex.
- The CI publish job (release.yml "Provision the GSD conformance oracle") sets no CLAUDE_CONFIG_DIR. `pre-tag-check.sh --container` forwards only `-e HOME=/home/runner`, `CARGO_INCREMENTAL`, `CARGO_TARGET_DIR`, `CARGO_HOME` and `PRE_TAG_CHECK_IN_CONTAINER`, and the Dockerfile sets no such ENV. Default resolution there is therefore unchanged, and none of those three files needs editing.
- This machine: CLAUDE_CONFIG_DIR=/home/blk/.local/share/claude-profiles/player-health, and its `gsd-core` is a symlink to the home-default install. CODEX_HOME is unset, `~/.codex/gsd-core` exists, no `gsd-tools` is on PATH, and node 24 comes via nvm. shellcheck is NOT installed; `bash -n` is the syntax gate.
- Baselines: full suite 3827 passed / 0 failed. The conformance suite has 6 tests, all green. clippy, fmt and the three release.yml targets (aarch64-apple-darwin, x86_64-apple-darwin, x86_64-pc-windows-gnu, all `--all-targets`) are clean. Tests run only on ubuntu-24.04, and other targets are only checked and linted, so Unix-path unit tests (`/h`, `/cc`) are fine.
- Executors must not commit PLAN.md, SUMMARY.md or STATE.md; the quick-batch coordinator commits those. The untracked `.planning/quick-batches/` dir is the coordinator's; leave it alone.
</context>

<!-- planner-discipline-allow: CLAUDE_CONFIG_DIR -->
<!-- planner-discipline-allow: InstallRoots::from_env() -->

<tasks>

<task type="tracer">
  <name>Task 1: End-to-end path: CLAUDE_CONFIG_DIR picks where the oracle is provisioned and where the conformance test finds it</name>
  <files>tests/driver_router_conformance.rs, scripts/install-conformance-oracle.sh, docs/DEVELOPMENT.md</files>
  <read_first>tests/driver_router_conformance.rs (lines 40-160: oracle_candidates, Oracle::resolve; lines 636-665: the missing-oracle assert; lines 820-860: the two oracle-candidate tests), src/state_reader/gsd_install.rs (lines 60-185: InstallRoots, config_home, expand_tilde, install_candidates), src/state_reader/queue_md.rs (lines 80-160), scripts/install-conformance-oracle.sh (whole file), docs/DEVELOPMENT.md (lines 90-100), .github/workflows/release.yml (lines 110-160), scripts/pre-tag-check.sh (the container `run --rm` block near lines 735-745)</read_first>
  <precondition>The home-default GSD install exists at $HOME/.claude/gsd-core/bin/gsd-tools.cjs, `node` runs, and no `gsd-tools` executable is on PATH (all true on this machine at planning time). The verify gates symlink to that install.</precondition>
  <action>
Wire the one path end to end: shared resolver list, then conformance oracle, then provisioning script. Tests never mutate the process environment; they inject `InstallRoots`.

A. tests/driver_router_conformance.rs: the oracle reads the TUI's own candidate list (I-4).
- Import `install_candidates` and `InstallRoots` from `gsd_meta_manager::state_reader::gsd_install`.
- `oracle_candidates` takes `roots: &InstallRoots` and returns `install_candidates(None, roots)`, mapping each `gsd_core_dir` onto `bin/gsd-tools.cjs`. Delete the hand-rolled CODEX_HOME match and anything only it used.
- Rewrite the `oracle_candidates` doc comment. It is the global arm of the production list: `install_candidates` with no project root, the same list `gsd_tools_candidates` in src/state_reader/queue_md.rs projects onto the same shim. The order is therefore `${CLAUDE_CONFIG_DIR:-~/.claude}` then `${CODEX_HOME:-~/.codex}`. An override counts only when non-blank (whitespace-only is unset), and a leading `~` expands against home. Keep the gsd-core 1.15.0 (#4667) Codex-only note and the reason the project-local candidates are omitted. Drop the "cannot call that private fn / mirrors" framing, because it now calls the shared function.
- `Oracle::resolve` builds its candidates from `InstallRoots::from_env()`. That is the TUI's single env read (home via `dirs::home_dir`, CLAUDE_CONFIG_DIR, CODEX_HOME); the two direct process-env reads go away. Reword its doc from "mirrors" to "reads the same list as" the TUI resolver.
- The SKIP `eprintln!` in `resolve` must keep starting with `SKIP: neither`. It and the missing-oracle assert message in `the_rust_rule_table_agrees_with_gsd_s_own_router_over_a_fixture_per_state` name `${CLAUDE_CONFIG_DIR:-~/.claude}/gsd-core/bin/gsd-tools.cjs` in place of the home-only path. Both are format strings, so braces are doubled as `${{…}}`, following the existing CODEX_HOME text.
- [Claude's discretion] The assert message also names `./scripts/install-conformance-oracle.sh` as the way to provision the oracle.
- Tests are pure over injected `InstallRoots` with home `/h`. Port `the_oracle_looks_for_a_claude_install_then_a_codex_install` and `the_oracle_honours_a_non_empty_codex_home` to the new signature, keep every assertion, and add a whitespace-only CODEX_HOME fallback.
- Add `the_oracle_follows_claude_config_dir`:
  - `/cc` gives first candidate `/cc/gsd-core/bin/gsd-tools.cjs` and exactly two candidates, with no `/h/.claude` entry (the override replaces the default, it does not add one).
  - `""` and `"   "` put `/h/.claude/gsd-core/bin/gsd-tools.cjs` first.
  - `~/cc` gives `/h/cc/gsd-core/bin/gsd-tools.cjs`.
  - Relative values belong to Task 2.

B. scripts/install-conformance-oracle.sh. Leave these untouched: `set -euo pipefail`, the npm version pin, the don't-clobber branch logic, and the existing order of the npm install before `mkdir -p "$CLAUDE_DIR"`.
- Replace the fixed CLAUDE_DIR assignment near line 60 with a resolution that mirrors `InstallRoots::claude_home` plus the Task 2 relative rule:
  - The default is `$HOME/.claude`.
  - Read the variable as `${CLAUDE_CONFIG_DIR:-}`, which is safe under `set -u`.
  - If the value has at least one non-whitespace character (test it with every `[[:space:]]` removed), expand a leading tilde: exactly `~` becomes `$HOME`, and a `~/` prefix becomes `$HOME/`.
  - Use the result only if it starts with `/`. Otherwise keep the default and remember the ignored value.
- PITFALL: bash tilde-expands unquoted case patterns and `${v#~/}`-style patterns, so the tilde must be quoted in every pattern (`"~"`, `"~/"*`, `${v#"~/"}`). A `~user/...` value then stays literal, is relative, and is ignored, matching Rust's `expand_tilde`.
- ORACLE_DIR and ORACLE_SHIM keep deriving from CLAUDE_DIR.
- Banner: add `  claude dir       : <dir> (<source>)` above the existing oracle-path line. The source says CLAUDE_CONFIG_DIR, default, or default plus the ignored value and why. The existing labels `  oracle path      :` and `  oracle     :` must stay byte-identical, because the verify gate parses them.
- Comments:
  - The don't-clobber header (near lines 26-33) now names `${CLAUDE_CONFIG_DIR:-$HOME/.claude}/gsd-core`.
  - Add a short WHERE THE ORACLE GOES paragraph covering three things. First, the rule above. Second, why: it is the first global path `oracle_candidates()` probes, which is the TUI's `install_candidates` list, and a relative value is ignored because Claude Code itself refuses one and the probe `cd`s into a fixture. Third, the publish job and `pre-tag-check.sh --container` never set CLAUDE_CONFIG_DIR (release.yml sets none, and the container run forwards only its explicit `-e` variables), so there the path is the home default, exactly as before.
  - The install-branch comment (near lines 297-302) says the symlink lands at `$CLAUDE_DIR/gsd-core`, the first global path `oracle_candidates()` probes.
  - Replace the three stale line-number references into the test file (near script lines 64, 162 and 300) with symbol names: `ORACLE_VERB`, `skeleton()`, and `oracle_candidates()` / `Oracle::resolve()` (I-6).

C. docs/DEVELOPMENT.md (near line 96): the script never touches an existing `${CLAUDE_CONFIG_DIR:-~/.claude}/gsd-core`, where a blank or relative CLAUDE_CONFIG_DIR counts as unset. That is also where the conformance test looks first.

Commit path-scoped as `fix(quick-261113-dcl): conformance oracle and its provisioning script follow CLAUDE_CONFIG_DIR` with `git commit -- tests/driver_router_conformance.rs scripts/install-conformance-oracle.sh docs/DEVELOPMENT.md`, then check `git log -1 --stat`.
  </action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail() { echo "FAIL-$1"; exit 1; }; L=target/dcl-t1.log; cargo test --test driver_router_conformance > "$L" 2>&1 || fail conformance-suite; grep -q "^test the_oracle_follows_claude_config_dir ... ok" "$L" || fail new-oracle-test; T=$(mktemp -d); mkdir -p "$T/cfg" "$T/codex" "$T/pos"; n=$(env GSD_META_MANAGER_ALLOW_MISSING_ORACLE=1 CLAUDE_CONFIG_DIR="$T/cfg" CODEX_HOME="$T/codex" cargo test --test driver_router_conformance the_rust_rule_table_agrees -- --nocapture 2>&1 | grep -c "GSD_META_MANAGER_ALLOW_MISSING_ORACLE is set"); [ "$n" = 1 ] || fail neg-e2e; ln -s "$HOME/.claude/gsd-core" "$T/pos/gsd-core"; env -u GSD_META_MANAGER_ALLOW_MISSING_ORACLE CLAUDE_CONFIG_DIR="$T/pos" CODEX_HOME="$T/codex" cargo test --test driver_router_conformance the_rust_rule_table_agrees > "$L" 2>&1 || fail pos-e2e; rm -rf "$T"; bash -n scripts/install-conformance-oracle.sh || fail script-syntax; S="$PWD/scripts/install-conformance-oracle.sh"; T=$(mktemp -d); mkdir -p "$T/bin" "$T/home/.claude" "$T/cfg" "$T/home/tcfg"; printf "#!/bin/sh\nexit 97\n" > "$T/bin/npm"; chmod +x "$T/bin/npm"; for d in "$T/home/.claude" "$T/cfg" "$T/home/tcfg" "$T/home"; do ln -s "$HOME/.claude/gsd-core" "$d/gsd-core"; done; m=0; r() { k="$1"; w="$2"; shift 2; g=$(cd "$T" && env "$@" HOME="$T/home" PATH="$T/bin:$PATH" "$S" 2>&1 | sed -n "s/^  $k *: //p"); if [ "$g" = "$w" ]; then echo "ok [$*]"; else echo "MISMATCH [$*] want $w got ${g:-none}"; m=1; fi; }; c() { w="$1"; shift; r "oracle    " "$w/bin/gsd-tools.cjs" "$@"; }; c "$T/home/.claude/gsd-core" -u CLAUDE_CONFIG_DIR; c "$T/home/.claude/gsd-core" CLAUDE_CONFIG_DIR=; c "$T/home/.claude/gsd-core" "CLAUDE_CONFIG_DIR=   "; c "$T/home/.claude/gsd-core" CLAUDE_CONFIG_DIR=rel/x; c "$T/cfg/gsd-core" "CLAUDE_CONFIG_DIR=$T/cfg"; c "$T/home/tcfg/gsd-core" "CLAUDE_CONFIG_DIR=~/tcfg"; c "$T/home/gsd-core" "CLAUDE_CONFIG_DIR=~"; r "oracle path     " "$T/nothere/gsd-core" "CLAUDE_CONFIG_DIR=$T/nothere"; [ ! -e "$T/nothere" ] || m=1; rm -rf "$T"; [ "$m" = 0 ] || fail script-matrix; [ "$(grep -c "driver_router_conformance\.rs:[0-9]" scripts/install-conformance-oracle.sh)" = 0 ] || fail stale-linerefs; [ "$(grep -cF "\$HOME/.claude/gsd-core" scripts/install-conformance-oracle.sh)" = 0 ] || fail script-home-default-literal; [ "$(grep -cF "~/.claude/gsd-core" docs/DEVELOPMENT.md)" = 0 ] || fail development-md; [ "$(grep -v "^\s*//" tests/driver_router_conformance.rs | grep -cF "~/.claude/gsd-core")" = 0 ] || fail test-message-literal; [ "$(grep -cF "var_os(\"HOME\")" tests/driver_router_conformance.rs)" = 0 ] || fail raw-home-read; [ "$(grep -v "^\s*//" tests/driver_router_conformance.rs | grep -c "InstallRoots::from_env()")" -ge 1 ] || fail from-env; [ "$(grep -v "^\s*//" tests/driver_router_conformance.rs | grep -c CLAUDE_CONFIG_DIR)" -ge 3 ] || fail test-names-ccd; [ "$(cat scripts/pre-tag-check.sh scripts/pre-tag-check.Dockerfile .github/workflows/release.yml | grep -v "^\s*#" | grep -c CLAUDE_CONFIG_DIR)" = 0 ] || fail ci-sets-ccd; echo verify-ok'</automated>
    Gate baselines, each dry-run at HEAD during planning. The block printed FAIL-new-oracle-test at HEAD, and each stage fails at HEAD as listed:
    - new-oracle-test: the test is absent at HEAD.
    - neg-e2e [HEAD=0 EXPECT==1 @7c857d5]: with CLAUDE_CONFIG_DIR and CODEX_HOME pointing at empty dirs, the oracle must be unresolvable. That proves the home default is replaced, not added.
    - pos-e2e (a pass/fail gate, not a count): it already passes at HEAD, which reaches the home default, so alone it proves nothing. Paired with neg-e2e it proves resolution via CLAUDE_CONFIG_DIR.
    - script-matrix [HEAD: 4 ok / 4 MISMATCH; EXPECT 8 ok]. Cases: unset, empty, whitespace and relative fall back to the home default. Absolute, `~/x` and `~` are followed. An absent absolute dir is targeted by the install branch: the banner shows it, the fake npm refuses, and the dir is not created. Every case runs with a sandboxed HOME and a refusing fake npm, so no case can install or overwrite anything real.
    - Grep gates: stale-linerefs [HEAD=3 EXPECT==0]; script-home-default-literal [HEAD=2 EXPECT==0]; development-md [HEAD=1 EXPECT==0]; test-message-literal (non-comment lines) [HEAD=2 EXPECT==0]; raw-home-read [HEAD=1 EXPECT==0]; from-env [HEAD=0 EXPECT>=1]; test-names-ccd (non-comment lines) [HEAD=0 EXPECT>=3].
    - ci-sets-ccd [HEAD=0 EXPECT==0 @7c857d5 GUARD MUTATION: export CLAUDE_CONFIG_DIR in the release.yml provision step or forward it with -e in the pre-tag-check.sh container run] is a deliberate REGRESSION GUARD. It already passes at HEAD and pins the fact that CI and the container leave the variable unset. This plan must not edit those files.
  </verify>
  <done>Task 1 is done when all of the following hold:
- The verify block prints verify-ok.
- The conformance suite is green, including `the_oracle_follows_claude_config_dir`.
- The oracle is unresolvable when CLAUDE_CONFIG_DIR and CODEX_HOME hold no install, and it passes the full comparison when CLAUDE_CONFIG_DIR holds the only install.
- The script resolves all 8 matrix cases as specified. Default behaviour is byte-identical where the variable is unset.
- The comments near lines 27 and 299 and docs/DEVELOPMENT.md name `${CLAUDE_CONFIG_DIR:-…/.claude}/gsd-core`, and no stale line-number reference remains.
- One path-scoped `fix(quick-261113-dcl): …` commit touches exactly the three files.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Shared resolver: a relative runtime-home override counts as unset (TUI, version detector and oracle)</name>
  <files>src/state_reader/gsd_install.rs, src/state_reader/queue_md.rs, tests/driver_router_conformance.rs</files>
  <read_first>src/state_reader/gsd_install.rs (lines 1-30: module doc; lines 60-185; the tests near lines 595-735), src/state_reader/queue_md.rs (lines 80-160; the tests near lines 620-700), src/agents/adapters/claude.rs (lines 95-113, resolve_config_root, the T-25-12 precedent; read only, because sibling 261113-dcj owns that file), tests/driver_router_conformance.rs (oracle_candidates as Task 1 left it)</read_first>
  <precondition>Task 1 is committed (oracle_candidates delegates to install_candidates), and the home-default GSD install exists at $HOME/.claude/gsd-core/bin/gsd-tools.cjs.</precondition>
  <behavior>
    - Unit test `a_relative_override_falls_back_to_the_home_default` in gsd_install.rs:
      - Home `/h` with CLAUDE_CONFIG_DIR `rel/cc` and CODEX_HOME `rel/cx`: `install_candidates(None, ..)` is `[/h/.claude/gsd-core, /h/.codex/gsd-core]`.
      - `~user/cc` also falls back.
      - With no home, CLAUDE_CONFIG_DIR `~/x` cannot expand, stays relative, and yields no global candidate.
    - The existing absolute, blank and tilde tests in gsd_install.rs and queue_md.rs stay green without edits.
    - Unit test `a_relative_claude_config_dir_counts_as_unset_for_the_oracle` in the conformance file: with home `/h` and CLAUDE_CONFIG_DIR `rel/cfg`, the first oracle candidate is `/h/.claude/gsd-core/bin/gsd-tools.cjs` and no relative candidate is present.
    - Both new tests FAIL before the resolver change, because the relative value is used literally, and pass after it.
  </behavior>
  <action>
RED: add the two tests from the behavior block, run them, and confirm both fail. Commit `test(quick-261113-dcl): a relative runtime-home override must count as unset` with `git commit -- src/state_reader/gsd_install.rs tests/driver_router_conformance.rs`.

GREEN (I-1, I-2, I-3): in `InstallRoots::config_home`, keep an override only when it is non-blank (as today) AND its tilde-expanded path is absolute. Use `Path::is_absolute`, the same test `resolve_config_root` uses. Otherwise fall through to `<home>/<name>`. One helper serves both overrides, so CODEX_HOME gets the same rule. Nothing else changes: candidate order, replacement semantics, blank handling and tilde expansion stay as they are.

Docs, all in the GREEN commit:
- `config_home` doc: state the rule and why.
  - The TUI checks a candidate's existence in its own cwd but runs node with `current_dir(project_root)` (`smart_entry_command` in queue_md.rs), and the oracle runs it with `current_dir(fixture)`. A relative candidate therefore can never be the one that runs.
  - Claude Code itself refuses a relative CLAUDE_CONFIG_DIR.
  - It is the same rule as `agents::adapters::claude::resolve_config_root` (T-25-12).
  - Tag it `[inferred 261113-dcl I-1/I-2]`, matching the existing inferred tags.
- `expand_tilde` doc: without a home, a `~` value stays literal, which makes it relative, so it counts as unset.
- Module doc, after the five-item list near lines 14-19: one sentence saying an override counts only when it is non-blank and absolute after `~` expansion.
- queue_md.rs: the `gsd_tools_candidates` paragraph saying an override counts only when non-blank gains "and absolute after `~` expansion". The "Known, deliberate differences from upstream" list on `resolve_gsd_tools_from` gains an entry: a relative override counts as unset, whereas upstream's resolver uses it as-is, relative to its cwd.
- tests/driver_router_conformance.rs, `oracle_candidates` doc: a relative override (judged after `~` expansion) counts as unset, inherited from the shared resolver.

Commit `fix(quick-261113-dcl): a relative runtime-home override counts as unset` with `git commit -- src/state_reader/gsd_install.rs src/state_reader/queue_md.rs tests/driver_router_conformance.rs`, then check `git log -1 --stat`.

Then run the verify block. Its full-suite, clippy, fmt and cross-target steps are slow on a cold worktree target dir; give them a generous timeout (run in the background if needed) rather than skipping any of them. Fix anything they surface in this task.
  </action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail() { echo "FAIL-$1"; exit 1; }; L=target/dcl-t2.log; cargo test --lib state_reader::gsd_install > "$L" 2>&1 || fail gsd-install-unit; grep -q "a_relative_override_falls_back_to_the_home_default ... ok" "$L" || fail relative-unit; cargo test --lib state_reader::queue_md > "$L" 2>&1 || fail queue-md-unit; cargo test --test driver_router_conformance > "$L" 2>&1 || fail conformance-suite; grep -q "^test a_relative_claude_config_dir_counts_as_unset_for_the_oracle ... ok" "$L" || fail relative-oracle-unit; T=$(mktemp -d); mkdir -p "$T/codex"; env -u GSD_META_MANAGER_ALLOW_MISSING_ORACLE CLAUDE_CONFIG_DIR=rel/cfg CODEX_HOME="$T/codex" cargo test --test driver_router_conformance the_rust_rule_table_agrees > "$L" 2>&1 || fail relative-e2e; rm -rf "$T"; [ "$(grep -v "^\s*//" src/state_reader/gsd_install.rs | grep -c is_absolute)" -ge 1 ] || fail is-absolute; cargo test --no-fail-fast > "$L" 2>&1 || fail full-suite; p=$(awk "/^test result:/{s+=\$4} END{print s+0}" "$L"); f=$(awk "/^test result:/{s+=\$6} END{print s+0}" "$L"); { [ "$p" -ge 3830 ] && [ "$f" = 0 ]; } || fail "suite-count-$p-passed-$f-failed"; cargo clippy --all-targets -- -D warnings > "$L" 2>&1 || fail clippy; cargo fmt --all -- --check > "$L" 2>&1 || fail fmt; for t in aarch64-apple-darwin x86_64-apple-darwin x86_64-pc-windows-gnu; do cargo check --locked --target "$t" --all-targets > "$L" 2>&1 || fail "check-$t"; cargo clippy --locked --target "$t" --all-targets -- -D warnings > "$L" 2>&1 || fail "clippy-$t"; done; echo verify-ok'</automated>
    Gate baselines, each dry-run at HEAD during planning. The block printed FAIL-relative-unit at HEAD; with the new-test checks stripped it stopped at FAIL-is-absolute, then at FAIL-suite-count-3827-passed-0-failed; the clippy/fmt/cross-target tail printed verify-ok.
    - relative-unit and relative-oracle-unit: the tests are absent at HEAD.
    - relative-e2e (a pass/fail gate, not a count): a relative CLAUDE_CONFIG_DIR falls back to the home default and the full comparison runs. It passes at HEAD, which ignores the variable. It fails after Task 1 alone, because the relative path is then used literally. It must pass after Task 2.
    - is-absolute (non-comment lines) [HEAD=0 EXPECT>=1 @7c857d5].
    - Suite passed count [HEAD=3827 EXPECT>=3830 @7c857d5], from three new tests: the_oracle_follows_claude_config_dir, a_relative_override_falls_back_to_the_home_default and a_relative_claude_config_dir_counts_as_unset_for_the_oracle. The failed count must stay 0, as at HEAD. Both counts are read from a log file inside `rtk bypass`.
  </verify>
  <done>Task 2 is done when all of the following hold:
- The verify block prints verify-ok.
- The RED commit (both new tests failing) precedes the GREEN commit.
- `InstallRoots::config_home` treats a blank or relative (after `~` expansion) override as unset, for both CLAUDE_CONFIG_DIR and CODEX_HOME.
- The TUI resolver, the version detector and the oracle share that one rule, and the script applies the same rule.
- The config_home, expand_tilde and module docs, queue_md.rs's two doc spots and the oracle_candidates doc all state it.
- `git status --porcelain` shows nothing from this task. Coordinator-owned PLAN, SUMMARY and STATE files and the untracked `.planning/quick-batches/` dir are excluded.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| process environment → executed script path | CLAUDE_CONFIG_DIR, CODEX_HOME and HOME choose which `gsd-tools.cjs` the conformance test runs through `node`, and which gsd-core the TUI runs for smart-entry |
| process environment → filesystem writes in the publish job | The provisioning script runs in the job that holds the crates.io token. CLAUDE_CONFIG_DIR now chooses where it verifies, or creates a directory and symlink |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-261113-dcl-01 | Elevation of Privilege | `Oracle::resolve`; script CLAUDE_DIR | low | accept | The variable is set by the same principal who runs `cargo test` or the script. It carries the same trust HOME already had, and GSD and the TUI honour it identically. The publish job and the container never set it; the `ci-sets-ccd` regression guard pins that, and the matrix's unset case proves the default path is unchanged. |
| T-261113-dcl-02 | Tampering | relative override in the script, the TUI resolver and the oracle | medium | mitigate | A relative value (judged after `~` expansion) counts as unset in `InstallRoots::config_home` and in the script (I-1, I-2, I-3). The script therefore can never create `<cwd>/…/gsd-core` or npm-install relative to wherever it is invoked. The TUI and oracle never resolve a candidate in one cwd and run it in another. Pinned by `a_relative_override_falls_back_to_the_home_default`, `a_relative_claude_config_dir_counts_as_unset_for_the_oracle`, relative-e2e and the matrix's `rel/x` case. |
| T-261113-dcl-03 | Tampering (destructive) | the don't-clobber rule, now keyed on a CLAUDE_CONFIG_DIR-derived ORACLE_DIR | medium | mitigate | The branch logic is untouched; only ORACLE_DIR's source changes. The verify matrix runs every case against pre-populated dirs and asserts the "already present" result, under a sandboxed HOME and a fake npm that refuses. The absent-dir case asserts the install branch targets the CLAUDE_CONFIG_DIR path and creates nothing when npm fails, so planning-time and execution-time checks can neither install nor overwrite a real GSD. |
| T-261113-dcl-SC | Tampering | npm/pip/cargo installs | low | accept | No dependency is added. The pinned `@opengsd/gsd-core@$GSD_VERSION` install line is untouched, and every cross-target gate runs `--locked`. The verification never runs a real npm. |
</threat_model>

<verification>
- Task 1's block prints verify-ok. It runs the oracle end to end against CLAUDE_CONFIG_DIR (negative and positive), the 8-case script matrix, `bash -n`, and the comment/doc grep gates. It also runs the CI/container regression guard.
- Task 2's block prints verify-ok. It runs the relative-override unit tests in both crates' test surfaces and relative-e2e. It then runs the full suite (at least 3830 passed, 0 failed, read from a log), clippy `-D warnings`, fmt check, and check plus clippy for every release.yml matrix target.
- The CI publish job and `./scripts/pre-tag-check.sh --container` are unaffected by construction: neither sets CLAUDE_CONFIG_DIR, so the script resolves the home default and the oracle probes the home default first. No file of theirs is edited.
- `git log` shows three path-scoped commits: the Task 1 fix, the Task 2 RED test and the Task 2 GREEN fix. `git status` is clean apart from coordinator-owned artifacts.
</verification>

<success_criteria>
- With claude-profile setups, where CLAUDE_CONFIG_DIR points at a profile dir, `cargo test` checks the same gsd-tools that GSD and the TUI load. `./scripts/install-conformance-oracle.sh` verifies or installs it there, never clobbering an existing install.
- With CLAUDE_CONFIG_DIR unset, blank, whitespace-only or relative, everything resolves the home default exactly as before, so the publish job and the container rehearsal are unchanged.
- One rule (non-blank, `~` expanded, absolute, else default) governs the TUI resolver, the version detector, the conformance oracle and the provisioning script. Each copy that cannot share code is pinned by a test or a gate.
- Inferred decisions I-1 to I-6 are listed in the SUMMARY for audit.
</success_criteria>

<output>
Create `.planning/quick/261113-dcl-conformance-oracle-follows-claude-config-dir-tests-driver-ro/261113-dcl-SUMMARY.md` when done. The coordinator commits it, so do not commit it yourself. Record:
- the inferred decisions I-1 to I-6, marked `[inferred — audit]`;
- each gate's HEAD and final values;
- the three commit hashes.
</output>
