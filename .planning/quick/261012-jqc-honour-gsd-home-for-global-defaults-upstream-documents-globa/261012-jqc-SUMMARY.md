---
quick_id: 261012-jqc
phase: quick-261012-jqc
plan: 01
subsystem: state_reader / ui labels / docs
tags: [gsd-home, config, defaults, upstream-parity]
status: complete
requires: []
provides:
  - "GSD_HOME-aware user_defaults_path() ($GSD_HOME/.gsd/defaults.json, HOME fallback)"
  - "pub const GSD_HOME_ENV"
affects: [Config tab global scope, preferences screen, help screen, README, docs/CONFIGURATION.md]
key-files:
  created:
    - tests/state_reader_gsd_home.rs
  modified:
    - src/state_reader/config_json.rs
    - src/ui/screens/help.rs
    - src/ui/screens/preferences.rs
    - src/ui/screens/detail.rs
    - README.md
    - docs/CONFIGURATION.md
worktree_branch: worktree-agent-a5ce01b941a8c3f88
completed: 2026-10-12
actuals:
  tasks: 2
  commits: 3
plan_head_before: 7303b41d0a9bcaa12d30370221c806b037ccf7b6
plan_head_after: d042dd67784a492b1091492e4283f403d343d896
---

# Quick 261012-jqc: Honour GSD_HOME for global defaults Summary

`user_defaults_path()` now resolves `$GSD_HOME/.gsd/defaults.json`. It falls back to `$HOME` when GSD_HOME is unset or empty, which matches gsd-core's `GSD_HOME || homedir()` rule (fd1bfd09e, v1.16.0). A pure resolver carries the semantics, and a single env-reading wrapper is the only code that reads the environment. Every user-facing label and doc now names `$GSD_HOME/.gsd/defaults.json`.

## Commits (branch worktree-agent-a5ce01b941a8c3f88, base 7303b41)

| Task | Commit | Subject |
|------|--------|---------|
| 1 RED | 0a7a9a8 | test(261012-jqc): pin GSD_HOME resolution of the global defaults file (red) |
| 1 GREEN | 74b83c5 | feat(261012-jqc): honour GSD_HOME for the global defaults file |
| 2 | d042dd6 | docs(261012-jqc): name the global defaults store $GSD_HOME/.gsd/defaults.json |

## What changed

- `src/state_reader/config_json.rs` now contains:
  - `pub const GSD_HOME_ENV`.
  - A private pure `user_defaults_path_from(gsd_home, home)`, which filters out an empty gsd_home, then uses gsd_home or else home, then pushes `.gsd/defaults.json`.
  - `user_defaults_path()` as the single env wrapper. Its public signature is unchanged, so the detail.rs call sites needed no edit.
  - Three pure unit tests, placed directly after `load_user_defaults_from_reads_a_given_path`. They are not at the module end and not near GatesConfig.
- `tests/state_reader_gsd_home.rs` holds exactly ONE #[test]. It uses a GSD_HOME store (mode "yolo") and a HOME decoy (mode "interactive"), and runs three cases (set, empty, unset) through `user_defaults_path()` -> `load_user_defaults_from()`.
- Labels, all static literals per I-12:
  - help `g` row and its pinned test copy (byte-identical)
  - preferences module doc, the preferences header, and a new assert in `render_labels_both_scopes`
  - in detail.rs, the empty state, the Global block title and its pin, and `config_scope_status` Global and its 3 pins (7 lines total, no other detail.rs hunks)
- README.md: line 275 is the only hunk, so lines 15-20 are byte-identical.
- docs/CONFIGURATION.md: a new `GSD_HOME` env row; `HOME` rewritten as the fallback root; the prose mentions at ~199, ~226 and ~236 updated.

## Verification

- RED: the integration test failed at case 1, resolving the HOME decoy path, before GREEN. It passes after GREEN.
- `cargo test --lib state_reader::config_json`: 25 passed. `cargo test --lib ui::screens`: 600 passed.
- Env-mutation grep on config_json.rs: 0. `#[test]` count in tests/state_reader_gsd_home.rs: 1.
- Label counts: help.rs 2, preferences.rs 3, detail.rs 7, README.md 1, as the plan expected.
- `rtk proxy cargo test --no-fail-fast`: 59 suites, **2952 passed, 1 failed, 15 ignored**. The single failure is the expected local one, `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (local git differs from the pinned 2.55.0).
- `cargo clippy -- -D warnings`: exit 0 (release gate 5).
- `cargo clippy --all-targets`: exit 0. It emits 2 warnings, both at `src/ui/screens/detail.rs:29809-29810` (`field_reassign_with_default` in test code). Both are PRE-EXISTING: this plan's detail.rs hunks are only at 7942, 8080, 12387, 22017, 22367, 22431 and 27374. They were left alone per the scope boundary.
- `cargo fmt --check`: exit 0.

## Deviations from Plan

None in behaviour. Two notes:

- Ordering: `GSD_HOME_ENV` was added in the RED commit (0a7a9a8) so that the red test could compile and fail on an assertion rather than a build error. The plan lists the constant as a step before RED, so this is consistent with it.
- SUMMARY location: the worktree-isolation guard refused writes to the main-checkout path. This SUMMARY was therefore written at the same relative path inside the worktree, uncommitted. The coordinator copies it to the main checkout.

## Inferred decisions — for later audit

- **ID-1 Upstream parity on emptiness.** Mirror `GSD_HOME || homedir()`. Unset and empty-string GSD_HOME both fall back to HOME. Any non-empty value is used verbatim, with no trimming or relative-path normalization. This is recorded in the resolver's doc comment.
- **ID-2 HOME fallback unchanged.** The fallback stays `std::env::var_os("HOME")`. Upstream's `os.homedir()` passwd lookup when HOME is unset is a pre-existing divergence and out of scope, so None stays None.
- **ID-3 Race-free env tests.** The semantics are tested through a private pure function. The env wiring is proven in a dedicated integration binary with exactly ONE #[test]. Lib unit tests never call set_var or remove_var.
- **ID-4 Label wording.** Labels use upstream's static literal `$GSD_HOME/.gsd/defaults.json`, plus "GSD_HOME defaults to ~ / your home directory" where there is room. The resolved path is never drawn (I-12).
- **ID-5 detail.rs labels included.** The Config tab's three user-visible labels and their pins were updated, confined to those exact literals.
- **ID-6 Out of scope (guardrails).** Not touched: the `*_api_key` help strings in detail.rs (~11440-11461), code comments using the tilde shorthand, README.md lines 15-20, and anything near `GatesConfig`.
- **ID-7 CONFIGURATION.md made self-consistent.** The env table and the three other defaults-path mentions were updated.
- **ID-8 Env-name constant.** `pub const GSD_HOME_ENV: &str = "GSD_HOME";`.

## Follow-up for the batch coordinator

- The `*_api_key` help strings in detail.rs (~11440-11461) still name the tilde key-file paths, which upstream fd1bfd09e re-documented under `$GSD_HOME/.gsd/`. That region belongs to 261012-jqb's help refresh or a later task.
- Pre-existing `clippy --all-targets` warnings at detail.rs:29809-29810 (`field_reassign_with_default`, test code) are not from this item.
- Merge note: config_json.rs overlaps with sibling 261012-jqb (GatesConfig/AuditConfig). This item's hunks are confined to the region around `user_defaults_path`, roughly lines 648-720.

## Threat Flags

None. No new surface: GSD_HOME has the same trust as HOME (T-jqc-02 accepted), and labels remain static literals (T-jqc-01).

## Self-Check: PASSED

- FOUND: tests/state_reader_gsd_home.rs
- FOUND: 0a7a9a8, 74b83c5, d042dd6 (all ancestors of HEAD d042dd6)

## Coordinator note (inferred — for later audit)

- quick-batch 261012-jqa assigned jqc to wave 1 because both plans list `src/state_reader/config_json.rs` and `src/ui/screens/detail.rs`. The coordinator ran both executors in parallel anyway (explicit operator instruction), with plans confined to disjoint regions. Merge order was jqb then jqc via `worktree.cleanup-wave`; both merged cleanly. Combined tree: 2958 passed, 1 failed (expected envelope::policy git-version test), 15 ignored; fmt and `clippy -D warnings` clean.
