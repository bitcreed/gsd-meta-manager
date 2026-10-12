---
quick_id: 261012-kee
mode: quick
type: execute
autonomous: true
---

# gsd-core v1.16.0 sync finale — baseline, docs, help text, clippy

Final task of the gsd-core v1.16.0 sync (REF=v1.16.0 = 87e87d34b, PREV=ec81d0d10).
Executed inline by the coordinator-dispatched agent (single task, no worktree).

## Task 1: Re-measure and record the baseline

- files: src/state_reader/config_json.rs, docs/GSD-CORE-SYNC.md, README.md
- action: measure `git describe --tags --always v1.16.0`, `git show v1.16.0:package.json`,
  `npm view @opengsd/gsd-core version`; set the three constants; rewrite the doc
  comments, the sync record header/baseline, collapse "Why VERSION and COMMIT
  disagree", update "Next sync"; update README's compatibility note.
- verify: `cargo test the_sync_record the_readme_compatibility the_gsd_core_sync_baseline`

## Task 2: Upstream-behaviour comments and help text

- files: src/state_reader/queue_md.rs, src/ui/screens/detail.rs
- action: re-verify resolver/smart-entry against v1.16.0 and cite 1.16.0; move
  search-provider key-file paths to `$GSD_HOME/.gsd/` per fd1bfd09e.

## Task 3: clippy

- files: src/ui/screens/detail.rs
- action: fix `field_reassign_with_default` in the 261006 builtin-default test.
- verify: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `cargo test --no-fail-fast`, `cargo test --test driver_router_conformance -- --nocapture`
