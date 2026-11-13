---
phase: quick-261113-epo
plan: 01
subsystem: launcher
tags: [sandbox, bwrap, gsd-home, host-dirs]
status: complete
requirements: [QUICK-261113-epo]
key-files:
  modified:
    - src/state_reader/config_json.rs
    - src/launcher/host_dirs.rs
    - src/launcher/home.rs
    - tests/launcher_sandbox.rs
    - docs/DRIVER-SANDBOX.md
    - docs/CONFIGURATION.md
commits: 3
plan_head_before: 55610e9e43ed4ffe7676a37e90c3bafc4a492194
plan_head_after: edd3cbc
actuals:
  tokens: 30000
  tasks: 3
  commits: 3
---

# Quick 261113-epo: Sandbox binds of .gsd and .gsd-capabilities.json honour GSD_HOME

The driver sandbox now binds `.gsd` and `.gsd-capabilities.json` of an absolute `$GSD_HOME` read-only, next to the `~/` defaults. The GSD home is decided by one shared resolver, `resolve_gsd_home_from`.

## Commits

- a22898d feat: resolver, `HostDirs.gsd_home` / `gsd_homes()`, `GSD_RO_REL` under every GSD home, end-to-end test for a GSD home under `$HOME`
- 82f26da feat: outside-`$HOME` placement (read-only re-bind, never a tmpfs), install-root refusal, credential-dir guard, tests
- edd3cbc docs: DRIVER-SANDBOX.md and CONFIGURATION.md

## Tests added

- config_json: `resolve_gsd_home_from_is_gsd_cores_rule`
- host_dirs: `gsd_home_follows_the_shared_resolver_and_keeps_the_default` (and `no_env_yields_exactly_the_defaults` extended)
- home.rs: `a_gsd_home_under_home_binds_the_store_of_both_homes`, `a_gsd_home_outside_home_is_rebound_without_a_tmpfs`, `a_relative_or_degenerate_gsd_home_never_gets_a_tmpfs`, `a_relocated_gsd_home_is_never_an_install_root` (and `no_allowlist_entry_overlaps_a_mask_dir` extended with two GSD_HOME variants)
- tests/launcher_sandbox.rs: `a_gsd_home_under_home_rebinds_its_store_and_keeps_the_default`, `a_gsd_home_outside_home_and_tmp_is_rebound_read_only_without_a_tmpfs`

No test writes the process environment; the launcher suites also pass with `GSD_HOME` set in the process environment.

## TDD red evidence

Task 1: `a_gsd_home_under_home_binds_the_store_of_both_homes` failed (`ro /home/u/gsd/.gsd` missing from the allowlist) and `a_gsd_home_under_home_rebinds_its_store_and_keeps_the_default` failed (no `--ro-bind` triple for the store) before the implementation.
Task 2: `a_gsd_home_outside_home_is_rebound_without_a_tmpfs`, `a_relocated_gsd_home_is_never_an_install_root` and the outside-`$HOME` end-to-end test failed before the implementation.

## Gates

- `cargo fmt --all -- --check`: clean
- `cargo clippy --all-targets --locked -- -D warnings`: exit 0
- `cargo test --no-fail-fast`: exit 0
- cross-target clippy (aarch64-apple-darwin, x86_64-apple-darwin, x86_64-pc-windows-gnu): all three installed, all exit 0
- Task verify commands 1, 2, 3: `verify-ok`

## Deviations from Plan

None - plan executed as written. The second end-to-end test was written during Task 2 rather than up front, so Task 1's commit does not contain a red test.

## Inferred decisions [inferred — audit]

All plan decisions I-1 to I-9 were followed as written. New ones:

- For an absolute GSD-home entry that lies outside `$HOME`, `candidates()` only guards with `touches_mask_dir`; existence is checked by the unchanged `home_allowlist` filter. A degenerate `GSD_HOME=/` therefore names `/.gsd` only if it exists, as a read-only bind, which is harmless.
- The `HostDirs` module doc and `home.rs` module doc each gained a short GSD_HOME paragraph (plan asked for it); wording is mine.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

Commits a22898d, 82f26da and edd3cbc are ancestors of HEAD; all listed files exist; `git status` is clean.
