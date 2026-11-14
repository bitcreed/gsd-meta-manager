---
phase: 34-owner-items-reader-inbox-cli
reviewed: 2026-11-14T13:40:00Z
depth: standard
files_reviewed: 32
files_reviewed_list:
  - .github/workflows/release.yml
  - Cargo.toml
  - scripts/install-conformance-oracle.sh
  - src/app.rs
  - src/cli.rs
  - src/cli_targets.rs
  - src/config.rs
  - src/driver/router.rs
  - src/inbox.rs
  - src/inbox_cli.rs
  - src/lib.rs
  - src/main.rs
  - src/state_reader/disk_status.rs
  - src/state_reader/mod.rs
  - src/state_reader/owner_items/deferred.rs
  - src/state_reader/owner_items/frontmatter.rs
  - src/state_reader/owner_items/markdown.rs
  - src/state_reader/owner_items/mod.rs
  - src/state_reader/owner_items/rules.rs
  - src/state_reader/owner_items/scan.rs
  - src/state_reader/owner_items/uat.rs
  - src/state_reader/owner_items/verification.rs
  - src/state_reader/state_md.rs
  - src/status_cli.rs
  - src/text.rs
  - tests/async_blocking_guard.rs
  - tests/driver_router_conformance.rs
  - tests/inbox_cli.rs
  - tests/owner_audit_conformance.rs
  - tests/owner_digest.rs
  - tests/state_reader_test.rs
  - tests/status_cli.rs
findings:
  critical: 1
  warning: 5
  info: 5
  total: 11
  necessity: 1
status: issues_found
---

# Phase 34: Code Review Report

**Reviewed:** 2026-11-14T13:40:00Z
**Depth:** standard
**Files Reviewed:** 32
**Status:** issues_found

## Summary

Reviewed the owner-items reader (Layer A port and Layer B rules), the `inbox` CLI and
document, the shared target resolver, the `status` integration, and the `disk_status`
hardening. The port is careful: slicing is on ASCII boundaries, the escaping of
project-authored text is consistent, `--ws`/phase command tokens are validated, and the
FIFO/symlink/oversize handling in `scan.rs` works for the final path component. The
concrete defects are all in how the reader behaves on a hostile or malformed project,
which is the exact threat model D-17 sets for this code. Each finding below marked
"reproduced" was run against `target/debug/gsd-meta-manager inbox` with a crafted tree.

Read `necessity-review.md`; one necessity finding (WR-04). No structural (fallow) block
was supplied, so there is no `## Structural Findings` section.

Key concerns:

- A 64 KiB `deferred-items.md` made of headings already costs 28 s; a 1 MiB one (the
  cap) does not finish in 60 s (CR-01).
- The 1 MiB read cap and the containment check are defeated by paths the reader does
  not guard (WR-01, WR-03).
- A VERIFICATION file with invalid YAML frontmatter reads as clean (WR-02).

## Critical Issues

### CR-01: Quadratic fence rescan per heading in `deferred-items.md` hangs the reader on a hostile file

**File:** `src/state_reader/owner_items/deferred.rs:374-381` (also `:197-202`, `:211-230`, `:400-404`)
**Issue:** `split_heading_entries` calls `scan_fences_from(lines, i + 1)` for every
heading. `scan_fences_from` allocates a de-indented copy of the whole tail
(`deindented(tail)`) and runs `scan_fenced_blocks` over it, so the cost is
O(headings x lines). A file with no `## Deferred Items` section falls back to the whole
file (`parse_deferred_items`, line 25), so an attacker controls the heading count. The
1 MiB cap does not help: it permits about 260,000 headings.

Reproduced with a project whose current phase directory holds one `deferred-items.md`
made of repeated `# a\n` lines (`inbox <dir> --json`, debug build):

| file size | wall time |
|-----------|-----------|
| 16 KiB    | 1.9 s     |
| 32 KiB    | 7.1 s     |
| 64 KiB    | 28.6 s    |
| 1 MiB     | killed at 60 s (extrapolates to hours) |

The time doubles twice per size doubling, so it is quadratic. The deferred lane is
hidden by default, but the digest is computed anyway. `status` runs the same reader, so
a cloned repository can freeze `inbox`, `status` and (once wired) the TUI refresh. This
is a hang on untrusted input in code whose header (D-17, fact 4) promises fail-safe
handling of hostile files. The same re-scan pattern appears at line 403 (`scan = scan_fences_from(lines, i)`
for each top-level item after an unterminated fence) and at line 267 in
`split_list_entries`.

**Fix:** Do the fence scan once per body and slice it, instead of rescanning per heading.
The de-indented fence scan and the heading tokenizer disagree about indentation, so a
plain "reuse the global scan" is not provably identical; the safe version scans each
segment only up to the next heading and carries a work budget:

```rust
// bound the per-heading scan to its own segment
let seg_end = headings.get(index + 1).map_or(lines.len(), |next| next.line);
scan = scan_fences_from(&lines[..seg_end], i + 1);

// and fail safe when the body is pathological
const MAX_SCAN_LINES: usize = 20_000;
if lines.len() > MAX_SCAN_LINES && headings.len() > 1_000 {
    return None; // caller marks the file Unreadable::TooLarge, never clean
}
```

Add a regression test that parses `"# a\n".repeat(60_000)` under a time bound, and re-run
`tests/owner_audit_conformance.rs` against the oracle after changing the scan.

## Warnings

### WR-01: Containment check is vacuous above the final path component, so the reader follows a symlinked `phases/` or `workstreams/` out of the project

**File:** `src/state_reader/owner_items/scan.rs:103-108` (callers `:179`, `:338`, `:384-392`, `:420-465`)
**Issue:** `contained` compares `canonicalize(phase_dir)` with `canonicalize(phase_dir.parent())`.
Both are resolved through the same symlinked ancestor, so the comparison passes whenever
the symlink sits above the phase directory. `current_phase_dirs`, `subdirectories` and
`archive_dirs` only reject a symlink as the last component (`file_type().is_dir()` does
not follow links; only `milestones` gets an explicit symlink check). The module header
promises "a symlink that escapes makes the file absent", which holds only for the file
itself.

Reproduced: `.planning/phases -> /somewhere/outside/phases`, where the target holds
`01-alpha/01-UAT.md` with a row `SECRET-OUTSIDE-ITEM`. `inbox <project>` lists
`- [ ] SECRET-OUTSIDE-ITEM` and a `/gsd-verify-work 1` command. A cloned repository can
therefore make the inbox read and display UAT/VERIFICATION/deferred-shaped files from
anywhere the user can read (frontmatter fields, row names, reasons).

**Fix:** Bound every candidate to the canonical project root, not to its own parent:

```rust
fn contained(root_real: &Path, phase_dir: &Path, file: &Path) -> Option<PathBuf> {
    let real_dir = phase_dir.canonicalize().ok()?;
    let real_file = file.canonicalize().ok()?;
    (real_dir.starts_with(root_real) && real_file.starts_with(&real_dir)).then_some(real_file)
}
```

Compute `root_real = planning_dir.canonicalize()` once in `scan_planning` and pass it to
`read_audit`. Keep a symlink-escape test for `phases/`, `workstreams/` and
`milestones/ws-*`.

### WR-02: A VERIFICATION file whose frontmatter does not parse reads as clean

**File:** `src/state_reader/owner_items/scan.rs:231-234` and `src/state_reader/owner_items/frontmatter.rs:100-121`
**Issue:** `frontmatter::extract` returns an empty mapping when the YAML fails to parse
(after one repair attempt). `verification::report_status` maps an absent `status` to
`Gate::Omit`, and `parse_document` then returns `None`, so the file contributes nothing:
no item, no unreadable marker, no count. The module header (fact 4) says anything the
parser cannot read is "reported as unreadable checks, never as clean".

Reproduced: a `01-VERIFICATION.md` with `status: human_needed`, a Human Verification
section with one open item, and one stray line `\tbad: [` in the frontmatter. `inbox`
reports `test 0` and lists nothing. The same file without the stray line reports
`test 1`. A human check disappears because of a typo in an unrelated key.

The upstream port treats "unparseable status" as omit, but upstream aborts or logs; this
reader reports a clean inbox, which is the wrong failure direction for a tool whose
core value is not missing a pending action.

**Fix:** Distinguish "no `status` key in a parsed block" (Omit, as upstream) from
"frontmatter fence is closed but the block did not parse" and mark the latter
`Unreadable::Io` (or a new `BadFrontmatter`) so it lands in the Fix lane as an
unreadable check:

```rust
// extract() returns Result<Mapping, FrontmatterError>; None for no fence
Err(FrontmatterError::Unparseable) => Some(ParsedFile {
    unreadable: Some(Unreadable::Io),
    ..ParsedFile::empty("unknown")
}),
```

### WR-03: The 1 MiB read cap is bypassed by `parse_project_state`, which reads the same files with an unbounded `read_to_string`

**File:** `src/state_reader/disk_status.rs:431-460` (callers `:854`, `:917`, `:1015`, `:1131`, `:1228`, `:1353`)
**Issue:** `read_text_file` was added to harden the disk-status readers against hostile
projects (non-blocking open, regular files only), but it calls `file.read_to_string`
with no size limit and follows symlinks wherever they point. `inbox` and `status` call
`parse_project_state` before the owner scan, so the 1 MiB cap and the `contained` check
in `scan.rs` never see the file first.

Reproduced: a sparse 1.5 GB `01-UAT.md` in a current phase directory. `inbox` finishes
with `maxrss=1558736 KB` and reports one "unreadable check" afterwards (the owner reader
did cap it). A larger file exhausts memory. A symlinked `01-UAT.md` pointing at any
regular file outside the project is also read by this path (only the `status:` value
leaks, into the `uat` field of `status --json`, and only for files that start with
frontmatter).

**Fix:** Cap inside the helper and share one implementation with `scan.rs::open_checked`:

```rust
const READ_CAP: u64 = 1 << 20;
if file.metadata()?.len() > READ_CAP { return Err(io::Error::new(InvalidInput, "too large")); }
let mut text = String::new();
file.take(READ_CAP + 1).read_to_string(&mut text)?;
if text.len() as u64 > READ_CAP { return Err(io::Error::new(InvalidInput, "too large")); }
```

Do not apply the cap to `*-PLAN.md` before checking what `MAX_*_BYTES` the caller
already enforces (the waves manifest has its own check at line 1128).

### WR-04: Impossible-state guard: `file_unreadable` in `apply_r7` is always false at its use

**File:** `src/state_reader/owner_items/rules.rs:510-511`
**Category:** necessity
**Issue:** The guarded state cannot occur at that line. `file_unreadable` is true only
when an unacknowledged current-scope file of the phase has `unparsed_blocks > 0` or
`unreadable.is_some()`. `build_digest` turns every such file into a Fix-lane group with
`counts.unreadable > 0` (lines 416-420 give `unreadable >= 1`; line 592-598 pushes the
group because `group_unreadable != 0`). Superseded files are skipped earlier, but only
`human_needed` VERIFICATIONs with `unreadable == None` and `unparsed_blocks == 0` can be
superseded (lines 258-261), so they never satisfy the guard. `in_phase` and `phase_files`
use the same scope, workstream, acknowledged and phase predicates, so that Fix group is
in `groups`, `has_fix` is true, and the loop `continue`s at line 507-509 before
`file_unreadable` is evaluated. The `&& !file_unreadable` term on line 517 is therefore
always true.
**Fix:** Remove `file_unreadable` and its use, and record the invariant ("an unreadable
file always yields a Fix group, so `has_fix` already excludes it") next to the
`has_fix || has_blocked` check. If the redundancy is meant as defense in depth, keep it
but add a test that fails when the invariant breaks.

### WR-05: Wall-clock assertions make `a_cold_inbox_over_twenty_projects_is_under_a_second` flaky

**File:** `tests/inbox_cli.rs:919-950`
**Issue:** The test spawns the real binary, builds 20 projects of 15 phases each, and
asserts `cold < 1 s` and `lib < 1 s`. Under `cargo test` the other test binaries run in
parallel, the dev profile is unoptimized (the `Cargo.toml` change adds opt-level 3 for
`regex` and friends to make this pass), and CI runners are slow and noisy. A pass on the
author's machine does not predict a pass on a loaded runner, and a failure there reads
as a product regression. A time bound also does nothing to catch the quadratic input in
CR-01, which is the real performance hazard.
**Fix:** Assert the work done instead of time: e.g. count parsed files via the existing
cache-hit counter (a second scan must read nothing from disk), keep a generous bound
(10 s) only as a hang detector, and add the adversarial-input test from CR-01.

## Info

### IN-01: Stale `allow(dead_code)` attributes hide an unused production function

**File:** `src/state_reader/owner_items/markdown.rs:15` and `src/state_reader/owner_items/frontmatter.rs:281`
**Issue:** Both attributes carry comments saying the callers "land in a later plan".
`fm_list` now has callers in `verification.rs`, so its attribute is stale.
`collect_section_stopping_at` (markdown.rs:273) has no caller outside its own test, and
the module-wide `#![allow(dead_code)]` stops the compiler from saying so.
**Fix:** Delete `fm_list`'s attribute, delete `collect_section_stopping_at` (or gate it
with `#[cfg(test)]`), and replace the module-wide attribute with item-level ones where
still needed.

### IN-02: Item ids can collide after `terminal_capped` truncation

**File:** `src/inbox.rs:85`
**Issue:** `id` is `<basename>#<test>`; the discriminating suffix is at the end, and
`terminal_capped` cuts at 200 characters. A file name longer than about 195 characters
(legal up to 255) gives every item in it the same truncated id, breaking the
"stable id" meaning of the field in the JSON schema.
**Fix:** Truncate the basename part and keep the suffix, or build the id from
`format!("{}#{}", cap(basename, 150), suffix)` before escaping.

### IN-03: `fix_phases` compares raw phase tokens, unlike every other phase comparison

**File:** `src/state_reader/owner_items/rules.rs:604-611`
**Issue:** The set key is `(workstream, group.phase)` as a string, while supersession,
R7 and `has_open_uat_items` compare with `same_phase_token` (`01` equals `1`). Two phase
directories that resolve to one roadmap phase (`01-alpha` and `1-alpha-extra`) would be
counted as two fix phases.
**Fix:** Normalize the key, e.g. use `PhaseNum::parse(&group.phase)` (falling back to the
raw token) as the set element.

### IN-04: The parse cache is bounded by entry count, not bytes

**File:** `src/state_reader/owner_items/mod.rs:183-264`
**Issue:** 8192 entries, each an `Arc<Parsed>` holding every item of a file of up to
1 MiB. A single 1 MiB UAT yields roughly 200,000 items (the JSON for such a file is
about 32 MB), so a hostile tree can pin gigabytes in the long-lived TUI process once the
digest is wired in. The CLI exits, so it is unaffected today.
**Fix:** Bound by an approximate weight (items or bytes) and clear when the total
exceeds a budget, or cap the items kept per file.

### IN-05: `inbox` reports zero pending todos when listing them failed

**File:** `src/inbox_cli.rs:73` (also `:70` for the clock)
**Issue:** `list_pending(&root).unwrap_or_default()` turns an I/O error into
`todos_count: 0`, so the "Triage" fold and the `hidden.todos` count are silently
wrong, the same fail-open direction as WR-02. `Utc::now()` is also taken once per
project instead of once per run, so ages in one document can straddle a day boundary.
**Fix:** Surface the error as a per-project warning or an error entry, and take `now`
once in `run` and pass it to `read_entry`.

---

_Reviewed: 2026-11-14T13:40:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
