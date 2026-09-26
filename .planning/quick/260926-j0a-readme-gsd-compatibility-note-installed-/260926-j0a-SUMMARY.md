---
phase: quick-260926-j0a
plan: 01
status: complete
subsystem: state_reader / ui
tags: [gsd-core, version-detection, readme, resolver, compatibility]
requires: [260926-gtk, 260926-gtm]
provides:
  - src/state_reader/gsd_install.rs (InstallRoots, install_candidates, GsdVersion, SyncRelation, detect_gsd_install, install_label, startup_summary, newest_newer_than_synced)
  - GSD_CORE_SYNCED_TREE_VERSION = "1.15.0"
  - ProjectState.gsd_install + parse_project_state_with(planning_dir, roots)
  - App::announce_gsd_install / announce_gsd_install_with
  - README `**GSD compatibility:**` note + drift test
affects: [queue_md gsd-tools resolver (now honours CLAUDE_CONFIG_DIR), Config tab border, dashboard border, startup status line]
tech-stack:
  added: []
  patterns: [single shared candidate list projected for two consumers, injected env roots, hand-rolled semver precedence, bounded file read]
key-files:
  created: [src/state_reader/gsd_install.rs]
  modified: [src/state_reader/mod.rs, src/state_reader/config_json.rs, src/state_reader/queue_md.rs, src/ui/screens/detail.rs, src/ui/screens/normal.rs, src/app.rs, src/main.rs, README.md, docs/GSD-CORE-SYNC.md, docs/GETTING-STARTED.md]
decisions:
  - "In-sync range is [GSD_CORE_SYNCED_VERSION 1.14.0, GSD_CORE_SYNCED_TREE_VERSION 1.15.0] inclusive; above is Warning, below is Info (I-1)"
  - "Effective install = first candidate whose gsd-core/VERSION is a regular file; garbage VERSION wins as Unrecognised (I-3)"
  - "gsd-tools resolver is a projection of install_candidates, so it now honours CLAUDE_CONFIG_DIR (I-4)"
  - "Startup project-local count is pluralised properly ('1 project uses' / 'N projects use') instead of the literal 'project(s)' (I-15)"
metrics:
  duration: ~75m
  completed: 2026-09-26
  tasks: 3
  files: 11
plan_head_before: 015978c2c04183d9c7ec3c4f5fd9c907720cad79
actuals:
  tokens: 23195
  tasks: 3
  commits: 6
---

# Phase quick-260926-j0a Plan 01: README GSD compatibility note + installed-GSD version detection Summary

README now carries a test-enforced `**GSD compatibility:**` paragraph naming the synced gsd-core baseline. Each project's effective gsd-core install is found by reading `VERSION` files only, using the same candidate order as the gsd-tools resolver, which is now one shared list. The result is compared with the in-sync range [1.14.0, 1.15.0] and shown in three places: the Config tab's top border, the dashboard border (only when an install is newer), and one startup status line.

## Commits

| Task | Type | Commit | Subject |
|------|------|--------|---------|
| 1 | RED | 0c83f26 | test: add failing installed-GSD detection tests |
| 1 | GREEN | e682663 | feat: detect the effective gsd-core install and show it on the Config tab |
| 2 | RED | f883b18 | test: add failing resolver-unification, startup-line and dashboard tests |
| 2 | GREEN | 2c68395 | feat: one candidate list for resolver and detector; startup line; dashboard newer-only warning |
| 3 | RED | 8ad3b01 | test: add failing README compatibility-note drift test |
| 3 | GREEN | ac7a64a | docs: README GSD compatibility note, detection docs, sync-record README step |

## What was built

- **Detection (`src/state_reader/gsd_install.rs`).**
  - Candidates, in resolver order: `<root>/gsd-core` → `<root>/.claude/gsd-core` → `<root>/.codex/gsd-core` → `${CLAUDE_CONFIG_DIR:-~/.claude}/gsd-core` → `${CODEX_HOME:-~/.codex}/gsd-core`.
  - An override counts only when it is non-blank, and a leading `~` is expanded against home.
  - The first candidate whose `VERSION` is a regular file wins. The `is_file` check runs before the file is opened, and the read is capped at 257 bytes via `take`.
  - A `VERSION` that is over 256 bytes, not UTF-8, or not parseable is reported as `Unrecognised`, and its content is never displayed.
- **Semver.** Hand-rolled: `MAJOR.MINOR.PATCH[-pre][+build]`, with an optional leading `v` and trimming. Prerelease and build identifiers are limited to `[0-9A-Za-z-]`. Precedence follows the semver rules (numeric identifiers compare by value without overflow; numeric sorts below alphanumeric; a shorter prefix sorts lower; a release sorts above its prereleases).
- **Wiring.**
  - `parse_project_state` delegates to `parse_project_state_with(planning_dir, &InstallRoots::from_env())`, which sets `state.gsd_install` right after `project_root` is resolved.
  - `InstallRoots::from_env()` is the only environment read in the feature.
- **Resolver unification.** `queue_md::gsd_tools_candidates(root, &roots)` is now `install_candidates(Some(root), &roots)` mapped to `<dir>/bin/gsd-tools.cjs`.
  - Every existing resolver test was migrated to `InstallRoots` with its name and intent kept.
  - New tests: `gsd_tools_candidates_honour_claude_config_dir` and a test that the resolver list equals the projected candidate list.
  - The doc line "CLAUDE_CONFIG_DIR is not honoured" was removed. The other two listed differences from upstream stay.
- **Surfaces.**
  - **Config tab:** the label is right-aligned on the list block's top border, using the `tab_bar_block` room/`fit_cells` pattern with `shown()`. In the empty branch it appears as a second line under the "No config loaded" message.
  - **Colours:** Ok is DarkGray, Info is Cyan, Warning is Yellow.
  - **Dashboard:** when any project's install is Newer, the outer border shows a right-aligned yellow ` GSD <v> newer than synced 1.15.0 `, naming the highest Newer version. It goes through `render_for_terminal` and `fit_cells` (made `pub(super)`). With no Newer install the render is byte-identical, and a test proves it.
  - **Startup:** `app.announce_gsd_install()` is called in `main.rs` right after `init_change_tracker()`. All file reads happen inside the App method. The winning path is written to the tracing log only.

## Exact README wording

Inserted after the `## What is it?` prose paragraph, before the screenshots:

```
**GSD compatibility:** synced against gsd-core 1.15.0 (the untagged
`release-1.15.0` branch, `v1.14.0-111-gec81d0d10`); the conformance oracle is
pinned to the published 1.14.0. At startup the app reads which gsd-core you have
installed and warns when it is newer than that -- see
[docs/GSD-CORE-SYNC.md](docs/GSD-CORE-SYNC.md) and [Compatibility](#compatibility).
```

Other README changes:
- The first sentence of `### Compatibility`, which named the stale 1.8.0 state formats, now points to that note. It spells no version literals.
- A new "Which GSD you have installed" block explains detection: the override order, the comparison rule, where the result appears, and that it uses file reads only.
- A new "GSD version awareness" Features bullet.
- The Codex bullet now notes that `$CLAUDE_CONFIG_DIR/gsd-core` is found too.

## Label and summary strings

- **Config/label:**
  - `GSD <v> · <source>` (Ok)
  - `GSD <v> · <source> · newer than synced 1.15.0` (Warning)
  - `GSD <v> · <source> · older than 1.14.0` (Info)
  - `GSD (unrecognised VERSION) · <source>` (Info)
  - `GSD not found · app synced to 1.15.0` (Info)
  - `<source>` is one of `project-local Claude`, `project-local Codex`, `project-local (root)`, `global Claude` or `global Codex`.
- **Startup:**
  - `GSD <v> (global Claude) · app synced to gsd-core 1.15.0`
  - `… is newer than synced gsd-core 1.15.0 — some formats may be unrecognised`
  - `… is older than this app's 1.14.0 baseline`
  - `GSD (global Claude) has an unrecognised VERSION · app synced to gsd-core 1.15.0`
  - `No global GSD install found · app synced to gsd-core 1.15.0`
  - Any of these may be followed by ` · N project(s) use a project-local GSD[ (M newer)]`, pluralised (see I-15).

All version values come from the constants. Tests build their fixtures from the constants (ceiling major+1 for Newer, `0.1.0` for Older).

## Gates

- `rtk proxy cargo test --no-fail-fast`: **2757 passed, 1 failed, 15 ignored**, across 56 test targets.
  - The only failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, the known local git-version witness.
  - The ETXTBSY flake did not appear.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: **clean** (exit 0).
- Oracle declaration grep on `config_json.rs`: prints `1`, and the literal is still `"1.14.0"`.
- `cargo test --test spawn_seam_guard`: 41/41 pass, census unchanged. `--test async_blocking_guard`: 9/9 pass.
- Task-level filters all pass: `--lib gsd_install`, `queue_md`, `announce_gsd_install`, `the_gsd_core_sync_baseline_is_recorded`, `the_readme_compatibility_note_names_the_synced_baseline`, `the_sync_record_names_every_modelled_key_and_the_measured_baseline`.

## Inferred decisions (for audit — human unavailable)

- INFERRED I-1 through I-14, as written in the plan, were applied unchanged.
  - The comparison range is [oracle pin 1.14.0, tree 1.15.0].
  - `GSD_CORE_SYNCED_TREE_VERSION` is the new constant.
  - The first `VERSION` regular file wins, and garbage wins as Unrecognised.
  - One shared candidate list, with CLAUDE_CONFIG_DIR now honoured by the resolver.
  - Override semantics follow the installer (non-blank, `~` expansion).
  - The `<root>/gsd-core` candidate is labelled `project-local (root)`.
  - PATH-only installs are not version-detected.
  - Surfaces: Config top border, dashboard only on Newer, one startup line.
  - Auto-register may supersede the startup line.
  - Garbage is never echoed.
  - README is the single place that spells the three values.
  - Detection runs inside `parse_project_state`.
  - The help and key tables are unchanged.
  - Semver is hand-rolled.
- INFERRED I-15: the startup project-local count is pluralised properly (`1 project uses …` / `N projects use …`) instead of the literal `project(s)` in the plan's behaviour text. This is cosmetic and pinned by `startup_summary_counts_project_local_gsd_installs`.
- INFERRED I-16: build metadata (`+…`) is validated with the same charset as prerelease identifiers before it is dropped, so `1.2.3+` is rejected. The plan only said "drop +build"; this is stricter, and the stricter version is not displayed anyway.
- INFERRED I-17: leading zeros in numeric parts (e.g. `01.2.3`) are accepted, where strict semver would reject them. Precedence is unaffected, because numeric identifiers are compared by value.
- INFERRED I-18: `GsdInstallStatus` derives `Eq` in addition to the required `Debug, Clone, PartialEq`. ProjectState's derives are unaffected.
- INFERRED I-19: in the resolver, whitespace-only `CLAUDE_CONFIG_DIR`/`CODEX_HOME` values now fall back to the home default. That follows the installer's `hasNonBlankOverride`, whereas the shell resolver's `${VAR:-…}` would keep them. This follows from I-5 and is recorded because it is a small behaviour change in the smart-entry resolver.
- INFERRED I-20: the help overlay does not describe the Config tab border, so it was not changed (the I-13 condition was checked: `src/ui/screens/help.rs` has no border or version text).
- `tests/driver_router_conformance.rs` `oracle_candidates` was left untouched. It is the conformance harness and out of scope, per the plan.

## Deviations from Plan

None. The plan was executed as written; the small interpretation choices are listed as I-15 to I-20 above.

## Known Stubs

None. The RED-commit stub bodies were all replaced in the matching GREEN commits.

## Threat Flags

None beyond the plan's threat model. T-j0a-01 (bounded read, strict charset, never display garbage, ESC test) and T-j0a-02 (`is_file` check before opening, 257-byte cap) are implemented and tested.

## Self-Check: PASSED

- FOUND: src/state_reader/gsd_install.rs
- FOUND commits: 0c83f26, e682663, f883b18, 2c68395, 8ad3b01, ac7a64a (`git rev-list --count 015978c..HEAD` = 6)
