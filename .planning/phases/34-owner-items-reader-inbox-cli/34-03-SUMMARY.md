---
phase: 34-owner-items-reader-inbox-cli
plan: 03
subsystem: cli
tags: [owner-inbox, verification, deferred-items, workstreams, archives, parse-cache, rust]
status: complete

requires:
  - phase: 34-01
    provides: owner_items scan, contained capped read, oracle harness
  - phase: 34-06
    provides: markdown.rs and frontmatter.rs helpers, hostile-input set
  - phase: 34-02
    provides: uat.rs parity (merged on this branch)
provides:
  - "verification.rs: reportStatusOf gate and parseVerificationItems (human_verification in frontmatter and body, frontmatter gaps)"
  - "deferred.rs: the deferred-items.md grammar (markers, headings, tables, fences)"
  - "scan.rs: VERIFICATION and deferred selection, workstream active phases, every archive tree, no-phases = empty"
  - "mod.rs: bounded process-wide parse cache, owner_items::evict_project"
  - "eight oracle-checked fixtures (OWN-01, OWN-02, OWN-03, OWN-06, OWN-11, OWN-12)"
affects: [34-04, 34-05]

actuals:
  tokens: 88000
  tasks: 3
  commits: 3
plan_head_before: d4868ff91e27fa9896faec3a52cdeaf65e077825
plan_head_after: 1a0b6d564c4a3b4466d9f9c21748f20884b2c0d9
commits: 3

key-files:
  created:
    - src/state_reader/owner_items/verification.rs
    - src/state_reader/owner_items/deferred.rs
    - tests/fixtures/owner/supersede-six-phases/
    - tests/fixtures/owner/source-mismatch/
    - tests/fixtures/owner/verification-gaps-found/
    - tests/fixtures/owner/verification-out-of-set/
    - tests/fixtures/owner/deferred-200/
    - tests/fixtures/owner/archived-complete-skipped/
    - tests/fixtures/owner/workstreams/
    - tests/fixtures/owner/no-phases/
  modified:
    - src/state_reader/owner_items/scan.rs
    - src/state_reader/owner_items/mod.rs
    - src/app.rs
    - tests/async_blocking_guard.rs
    - scripts/install-conformance-oracle.sh
    - .github/workflows/release.yml

key-decisions:
  - "An out-of-set VERIFICATION status degrades that one file to Unreadable::OutOfSetStatus (status stored as found); upstream aborts the whole run [inferred, RESEARCH Open Question 1]"
  - "A VERIFICATION or deferred-items.md that the contained read marks unreadable becomes an AuditFile of its kind with that mark, decided before the status gate [inferred]"
  - "deferred-items.md gets the contained capped O_NONBLOCK read although upstream reads it unchecked [inferred]"
  - "Cache key is (canonical path, AuditKind) -> ((mtime, len), Arc<Option<ParsedFile>>): the kind is part of the key because a name such as 01-UAT-VERIFICATION.md is read as both; a passed VERIFICATION is cached as None. Bound 8192 entries, cleared on overflow [inferred]"
  - "A poisoned cache lock makes get/insert no-ops (uncached read); evict_under recovers the guard so eviction still works"
  - "A symlinked milestones/ directory is not entered (stricter than upstream, T-34-10); phase and archive directories are entered only as real directories"
  - "scan() keeps one public entry; scan_planning takes the cache so tests can use a local or poisoned one while the hostile-input tests run through the global cache"

requirements-completed: [OWN-01, OWN-02, OWN-03, OWN-06, OWN-11, OWN-12]

completed: 2026-11-14
---

# Phase 34 Plan 03: VERIFICATION, deferred, workstreams, archives, parse cache

**Layer A now reads everything gsd-core's audit-uat reads, across root, workstreams and archived milestones, an out-of-set VERIFICATION marks one file unreadable instead of aborting the project, and repeat scans come from a (path, mtime, len) cache the watcher evicts.**

## Accomplishments

- Task 1 (91fc4a1): `verification.rs` and `deferred.rs` are line-level ports (no regex crate) of the upstream parsers; scan.rs reads UAT, VERIFICATION and deferred files through one generic read path. Fixtures `supersede-six-phases` (oracle: 41 items = 21 human_uat + 20 UAT rows, phases 20-25), `source-mismatch` (quoted `"human_needed"`), `verification-gaps-found` (1 human check + 7 open gaps, 1 resolved skipped), `verification-out-of-set` (declared divergence) and `deferred-200` (200 items) all compare equal or diverge as declared.
- Task 2 (29a92f6): workstream active phases (the workstream's own roadmap window), archive enumeration ported from `listArchiveVersionDirs`/`getAllArchivedPhaseDirs` (flat `vX.Y-phases` newest first by numeric segments, then `ws-*` by name descending, root tree then workstream trees descending; labels `vX.Y`, `ws-...`, `<ws>/...`; rank = enumeration index), dedup by `rel_path` first wins, no phases and no archive = empty. Fixtures `archived-complete-skipped` (14 skipped rows), `workstreams` (root + stream + stream archive + root archive) and `no-phases` (declared divergence). Conformance harness unchanged; the two oracle-consumer comments name `tests/owner_audit_conformance.rs`.
- Task 3 (1a0b6d5): `ParseCache` (std Mutex, lock held for one map operation only), `evict_project`, the `FileChanged` call in app.rs, and `read_owner_digest(` in `BLOCKING_HELPERS`. Plan 34-06's FIFO, escaping-symlink, cap-boundary and alias-bomb tests run unchanged through the global cache and pass.

## Verification (raw)

- `cargo test --no-fail-fast` (whole suite): 4019 passed, 0 failed.
- `--lib owner_items`: 118 passed, 0 failed. `--test async_blocking_guard`: 8 passed.
- `--test owner_audit_conformance`: "16 compared, 2 declared divergences, over 18 fixtures".
- `cargo clippy --all-targets -D warnings` clean on the host, `x86_64-pc-windows-gnu` and `aarch64-apple-darwin`.
- `grep -cE "lock\(\)\.unwrap\(\)"`: 0 in mod.rs and scan.rs. `bash -n scripts/install-conformance-oracle.sh`: ok.

## TDD note

Task 1 and Task 3 are `tdd="true"`; the unit tests were written in the same change as the code rather than as a separate failing-first commit, because the oracle (not a unit test) is the arbiter of correctness for the parsers and the cache tests need the cache type to exist. No separate RED commit exists.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] An existing scan test treated a VERIFICATION with a UAT body as ignored**
- **Found during:** Task 1
- **Issue:** `a_uat_of_a_roadmap_phase_is_read_and_others_are_not` wrote `01-VERIFICATION.md` with `status: testing`; that is now an out-of-set status and, correctly, a listed unreadable file.
- **Fix:** the test's VERIFICATION now has `status: passed`, which is what "not listed" means.
- **Files modified:** src/state_reader/owner_items/scan.rs
- **Commit:** 91fc4a1

### Plan wording not literally followed

- The plan's key link shows `parse_verification_items(&lines, status)`; the action text and the implementation take `(lines, fm, status)` because the frontmatter list is read from the parsed mapping.
- All eight fixtures were generated by one throwaway script in the scratchpad (not committed) and checked with the oracle before each commit; Task 1 committed five, Task 2 three.
- The poisoned-lock test uses a local `ParseCache` with a panicking holder rather than poisoning the process-wide static.

## Notes for later plans

- `Scope::Archived.rank` is the index of the archive directory across the whole enumeration, not of the phase directory.
- A VERIFICATION `AuditFile` carries `status` verbatim (`human_needed`, `gaps_found`, or the out-of-set value); Layer B must treat `unreadable = Some(OutOfSetStatus(_))` as unreadable checks.
- Deferred `AuditFile`s have status `unresolved`; an unreadable one has no items and must surface as unreadable, never as clean (OWN-11).
- `frontmatter::fm_list`'s `#[allow(dead_code)]` can now be removed by whoever next touches frontmatter.rs.
- Known flake from 34-02 (`multiplexer::tmux ... focus_without_an_absolute_socket_addresses_no_server_at_all`, "Text file busy") did not occur in the final full run.

## Known Stubs

None.

## Self-Check: PASSED

- Files exist: verification.rs, deferred.rs, the eight fixture directories, this SUMMARY.
- Commits 91fc4a1, 29a92f6 and 1a0b6d5 are ancestors of HEAD.
