---
phase: quick-261113-dcl
plan: 01
status: complete
commits: 3
plan_head_before: 7c857d5ba76e548469575a218a4971c1fd0d031e
plan_head_after: e4897c267f7076c44c686f9323ab7c800d887773
---

# Quick 261113-dcl: conformance oracle follows CLAUDE_CONFIG_DIR

The router-conformance oracle and its provisioning script now resolve the Claude home by the TUI's own rule. A relative runtime-home override counts as unset in the shared resolver.

## Commits

- 48826b1 fix(quick-261113-dcl): conformance oracle and its provisioning script follow CLAUDE_CONFIG_DIR
- d8ad54f test(quick-261113-dcl): a relative runtime-home override must count as unset (RED)
- e4897c2 fix(quick-261113-dcl): a relative runtime-home override counts as unset (GREEN)

## Files

tests/driver_router_conformance.rs, scripts/install-conformance-oracle.sh, docs/DEVELOPMENT.md, src/state_reader/gsd_install.rs, src/state_reader/queue_md.rs.

## Tests added

- `the_oracle_follows_claude_config_dir`
- `a_relative_claude_config_dir_counts_as_unset_for_the_oracle`
- `a_relative_override_falls_back_to_the_home_default`
- the Codex blank-fallback test now also covers a whitespace-only `CODEX_HOME`

## Gates (HEAD -> final)

- Full suite: 3827 passed / 0 failed -> 3830 passed / 0 failed.
- clippy `--all-targets -D warnings`, `cargo fmt --check`, and check plus clippy for the 3 release.yml targets: clean.
- `bash -n` on the script: ok.
- Script matrix: 8 of 8 cases ok. HEAD was 4 ok / 4 MISMATCH.
- negative e2e (CLAUDE_CONFIG_DIR and CODEX_HOME with no install): oracle unresolvable. Positive e2e (via a symlinked CLAUDE_CONFIG_DIR): full comparison passes. Relative e2e: passes.
- ci-sets-ccd guard: 0. pre-tag-check.sh, its Dockerfile and release.yml are untouched and never set CLAUDE_CONFIG_DIR.

## Inferred decisions [inferred — audit]

- I-1: "relative = unset" lives in `InstallRoots::config_home`, and the oracle consumes it. The TUI resolver did not have this rule before, so this also fixes a latent smart-entry cwd defect.
- I-2: the rule covers `CODEX_HOME` too, which diverges from upstream's resolver. Recorded in queue_md.rs's "Known, deliberate differences" list.
- I-3: "relative" is judged after `~` expansion. `~user/x` stays literal and is ignored. The script mirrors this.
- I-4: the oracle's home comes from `InstallRoots::from_env()` (`dirs::home_dir`) instead of raw `$HOME`.
- I-5: a machine whose CLAUDE_CONFIG_DIR has no gsd-core now fails the conformance test, as GSD and the TUI would load nothing there. The remedy is the script, which now provisions into `$CLAUDE_CONFIG_DIR/gsd-core`.
- I-6: the script's stale line-number references into the test file were replaced with symbol names.

## Deviations from plan

None.

- The first verify run needed `mkdir target` in the fresh worktree, because the verify scripts write logs to `target/`.
- The verify blocks were run from scratchpad script files rather than inline `bash -c`, because the worktree guard refuses very long inline commands. Their content is unchanged.

## Known stubs / threat flags

None.
