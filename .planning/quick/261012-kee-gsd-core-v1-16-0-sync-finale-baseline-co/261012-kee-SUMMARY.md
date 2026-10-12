---
quick_id: 261012-kee
phase: quick-261012-kee
plan: 01
subsystem: state_reader / docs / ui help
tags: [gsd-core-sync, v1.16.0, baseline, gsd-home]
status: complete
key-files:
  modified:
    - src/state_reader/config_json.rs
    - src/state_reader/queue_md.rs
    - src/ui/screens/detail.rs
    - README.md
    - docs/GSD-CORE-SYNC.md
completed: 2026-10-12
actuals:
  tasks: 3
  commits: 2
---

# Quick 261012-kee: gsd-core v1.16.0 sync finale

## Measured baseline (REF=v1.16.0)

| Command | Result | Constant |
|---|---|---|
| `git -C ~/projects/node/gsd-core describe --tags --always v1.16.0` | `v1.16.0` (87e87d34b2d8519e30d1af7d910fc98f2af4bc8d) | `GSD_CORE_SYNCED_COMMIT` |
| `git show v1.16.0:package.json` | `1.16.0` | `GSD_CORE_SYNCED_TREE_VERSION` |
| `npm view @opengsd/gsd-core version` | `1.16.0` | `GSD_CORE_SYNCED_VERSION` |

## Done

- Constants 1.14.0 / v1.14.0-111-gec81d0d10 / 1.15.0 -> 1.16.0 / v1.16.0 / 1.16.0;
  doc comments rewritten (the VERSION/COMMIT disagreement no longer applies).
- docs/GSD-CORE-SYNC.md: header + baseline table, "Why VERSION and COMMIT
  disagree" collapsed into "When the three disagree", recipe REF/PREV and
  "Next sync" item 1 rewritten (old item 1 "bump once 1.15.0 ships" dropped,
  list renumbered). Key-inventory tables untouched (jqb/jqc).
- README compatibility note: "synced to gsd-core 1.16.0 (`v1.16.0`), with oracle pin `1.16.0`".
- queue_md.rs: resolver order + smart-entry comments cite 1.16.0.
- detail.rs: six search-provider help strings `~/.gsd/*_api_key` -> `$GSD_HOME/.gsd/*_api_key`;
  `jina` help also names `$GSD_HOME/.gsd/jina_api_key`.
- detail.rs test `project_view_shows_builtin_default_not_global_value`: struct-literal init
  (clippy `field_reassign_with_default`).

## Verification

- `cargo fmt --check`: clean. `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --no-fail-fast`: 2958 passed, 1 failed, 15 ignored. The one failure is the
  expected local-only `envelope::policy` git-version constants test.
- `driver_router_conformance`: 6/6 passed, no SKIP, "5 command comparisons, 4 declared
  divergences, 2 upstream-silent states, over 11 fixtures" against installed
  `~/.claude/gsd-core` VERSION 1.16.0. Installed 1.16.0 == ceiling 1.16.0, so the
  "newer than synced" warning no longer fires.

## Inferred decisions (for audit)

- I-1: Upstream `gsd-run-resolver.md` changed v1.15.0..v1.16.0 only in the non-Claude home
  list (extended/reordered; Claude still first, ahead of Codex) and `smart-entry.cts` still
  hardcodes `/gsd:`; so comments were re-cited to 1.16.0 with no code change. The existing
  "other runtime homes not probed" divergence note still covers the larger home list.
- I-2: `jina` help string also gets the `$GSD_HOME/.gsd/jina_api_key` path — upstream
  CONFIGURATION.md v1.16.0 documents it and fd1bfd09e covers all seven probes.
- I-3: `ec81d0d10` (previous COMMIT) is an ancestor of neither v1.15.0 nor v1.16.0
  (release branch re-cut upstream); recorded in the sync doc so the next delta diffs from a tag.
- I-4: Next-sync recipe placeholders set to REF=v1.17.0 / PREV=v1.16.0.
- I-5: Historical `release-1.15.0` references in test/doc comments describing what the
  260926-gtk sync did were left as history, not rewritten.
