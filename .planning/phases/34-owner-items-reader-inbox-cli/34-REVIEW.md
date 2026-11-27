---
phase: 34-owner-items-reader-inbox-cli
reviewed: 2026-11-27T06:25:01Z
depth: standard
files_reviewed: 24
files_reviewed_list:
  - Cargo.toml
  - src/app.rs
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
  - tests/inbox_cli.rs
  - tests/owner_audit_conformance.rs
  - tests/owner_digest.rs
  - tests/status_cli.rs
findings:
  critical: 0
  warning: 2
  info: 2
  total: 4
  necessity: 0
status: issues_found
---

# Phase 34: Code Review Report

**Reviewed:** 2026-11-27T06:25:01Z
**Depth:** standard
**Files Reviewed:** 24
**Status:** issues_found

## Summary

This is a re-review. I re-read the six fix commits from the previous review (CR-01, WR-01 to WR-05, IN-01, IN-03) and traced each fix against the surrounding code. I then reviewed the rest of the phase diff in the listed files: `inbox.rs`, `inbox_cli.rs`, `cli_targets.rs` (the shared resolver), `status_cli.rs`, `config.rs::update_config`, the `app.rs` digest, reparse, registry-adoption and filter changes, the router G0 gate, and the `scan.rs` and `rules.rs` owner reader.

I did not build or run the tests. Findings come from reading the code and tracing inputs by hand.

**The fixes hold up.**
- **CR-01 (fence scan reuse):** `FenceScan::closed_before` is correct at the opener, closer and unterminated boundaries. Every refresh path either reuses a closed scan or spends the shared work budget. An exhausted budget becomes `Unreadable::TooLarge`, never a clean file.
- **WR-01 (containment):** `contained` now bounds the canonical phase directory to the canonical planning tree.
- **WR-03 (read cap):** `read_text_file` checks the length and also bounds the read itself with `take`.
- **WR-04 (`apply_r7`):** I checked the invariant behind the removed `file_unreadable` guard. Every current, unacknowledged file with `unreadable` or `unparsed_blocks` yields a Fix group in `build_digest_with`. That happens in `lane_entries` and the `group_unreadable` handling, so `has_fix` excludes the phase before the guard point. The removal is sound.
- **IN-03 (`fix_phases` key):** the key now uses the canonical phase spelling.

**New findings.** Both warnings are gaps in hardening this phase otherwise makes. WR-01 is an incomplete WR-02 fix. WR-02 is a new unbounded, blocking read next to the code the phase hardened.

**Carried over, not re-reported:** the earlier review's IN-02 (id collision after `terminal_capped`), IN-04 (parse cache bounded by entry count) and IN-05 (todo listing failure reads as zero) are still open in 34-REVIEW-DISPOSITION.md. I confirmed `inbox_cli.rs:112` (`list_pending(&root).unwrap_or_default()`) is unchanged.

No necessity findings. I read the necessity rubric and found no guard against a state this stack rules out.

## Warnings

### WR-01: A VERIFICATION with an unterminated or non-mapping frontmatter block still reads as clean (WR-02 fixed only half the class)

**File:** `src/state_reader/owner_items/frontmatter.rs:112-134` (also `src/state_reader/owner_items/scan.rs:234-246`)

**Issue:** The WR-02 fix made `extract_checked` return `Err(Unparseable)` only for a closed block whose YAML fails to parse, or one that carries an anchor or alias. Two equally easy damage modes still return `Ok(Mapping::new())`:
- An unterminated block (the closing `---` is missing). `locate_fence` returns `closed: false`, so `extract_checked` returns `Ok(empty)`.
- A closed block whose YAML parses to a scalar or sequence (`Some(_) => Ok(Mapping::new())`).

Trace for a VERIFICATION file `---\nstatus: human_needed\nhuman_verification:\n  - check the invoice PDF\n` with no closing `---`:
1. `extract_checked` returns `Ok(empty)`.
2. `report_status` returns `Gate::Omit`.
3. `parse_document` returns `None`.
4. `read_audit` returns early on `parsed.as_ref().as_ref()?`, so the file is absent from the digest.

The pending human check disappears with no Fix-lane entry. This is the failure the WR-02 fix commit describes: "the file may be hiding a pending check, so it is unreadable, never clean". It is also inconsistent with the dashboard. `disk_status::read_verification_status` reports `Missing` for the same file, and the router treats `Missing` as a gate. The dashboard flags the project while the inbox says there is nothing for the owner to do.

Mirroring upstream's `{}` for these shapes may be intended for conformance. If so, it is a declared divergence candidate, as WR-02's anchor case already is. It is not currently declared or tested; the only unterminated-frontmatter test (`scan.rs:858`) covers a UAT.

**Fix:** In `extract_checked`, treat "opened but never closed" and "closed but not a mapping" as `Err(Unparseable)` when the caller is a VERIFICATION. Keep `Ok(empty)` only for "no fence at all".
```rust
let Some(fence) = locate_fence(text) else { return Ok(Mapping::new()); };
if !fence.closed {
    return Err(Unparseable);          // opened, never closed: damaged, not absent
}
...
match parsed {
    Some(Value::Mapping(map)) => Ok(map),
    Some(_) | None => Err(Unparseable),
}
```
`extract` (which UAT uses) keeps its contract through `unwrap_or_default()`. Add a `scan.rs` test next to `a_verification_with_unparseable_frontmatter_is_unreadable_not_clean`, and declare the divergence in the conformance oracle if upstream differs.

### WR-02: The new G0 `HANDOFF.json` read is an unbounded, blocking `read_to_string` in a reader this phase hardens against hostile project trees

**File:** `src/state_reader/mod.rs:1008-1013` (siblings at `:1001`, `:902`, `:1023`)

**Issue:** `parse_project_state` is what `inbox` and `status` call for every registered project, including projects the user does not own. This phase hardened the sibling readers for that threat:
- `disk_status::read_text_file` opens with `O_NONBLOCK`, requires a regular file and (since WR-03) caps the read at 1 MiB.
- `scan::open_checked` does the same for the owner files.

The G0 addition reads `planning_dir.join("HANDOFF.json")` with plain `std::fs::read_to_string`. If `HANDOFF.json` is a FIFO, the open blocks forever. That is exactly what the `read_text_file` doc comment says it prevents ("a FIFO named like an artifact cannot park the reader"). The same call would also buffer a multi-gigabyte file. Because `inbox` is a one-shot CLI that reads every registered project, one project with a planted FIFO hangs the whole command and every other project's report with it.

The line above it (`.continue-here.md`) and the STATE.md and ROADMAP.md reads have the same flaw. They predate this phase. The new G0 read adds one more instance and reads a file that `detect_handoff` (`mod.rs:719`) already reads, so the file is opened twice per parse.

**Fix:** Route the new read through the hardened reader, and ideally reuse the handoff already read:
```rust
if crate::state_reader::disk_status::read_text_file(planning_dir.join("HANDOFF.json"))
    .is_ok_and(|content| !content.trim().is_empty())
{
    state.pause_recorded = true;
}
```
Then migrate `.continue-here.md`, STATE.md and ROADMAP.md in a follow-up, or at least record that those reads are outside the hardening.

## Info

### IN-01: `inbox::phase_key` orders decimal phases lexically, unlike the digest's own sort

**File:** `src/inbox.rs:317-320` (used at `:683`, `:744`, `:1000`, and in the `entry_sections` and `push_groups` sorts)

**Issue:** `phase_key` returns `(leading integer, raw string)`. `2.10` and `2.9` both give `(2, ..)`, then the raw strings compare byte-wise, so `2.10` sorts before `2.9`. The digest sorts groups with `PhaseNum::parse` in `rules::sort_key`, which understands decimals. The text views re-sort with this weaker key, so the `inbox ALIAS`, `--lane` and `--all` section order differs from the D-20 order for decimal phases (inserted phases such as `6.1` to `6.10`). It also shadows the unrelated `phase_num::phase_key` that `rules.rs` imports, which is easy to confuse. The effect is cosmetic ordering only.

**Fix:** Sort with `PhaseNum::parse(phase)` as `rules::sort_key` does, for example `(PhaseNum::parse(phase).is_none(), PhaseNum::parse(phase), phase.to_owned())`. Rename the local helper so it no longer shares a name with `phase_num::phase_key`.

### IN-02: The 1 MiB cap makes an oversized artifact indistinguishable from an absent one in `disk_status`

**File:** `src/state_reader/disk_status.rs:431-470` (callers at `:876`, `:977`, `:1191`, `:1288`, `:1413`)

**Issue:** `read_text_file` now returns an error for a file over 1 MiB. The callers collapse every error to "absent": `read_verification_status` returns `Missing`, `read_uat_status` returns the default, and a PLAN's superseded check is skipped. The owner reader keeps the distinction (`Unreadable::TooLarge`), so the same oversized VERIFICATION reads "unreadable" in the inbox and "no verification written" on the dashboard and in the router journal reason (`gate_verification_missing`). The router still parks, so this is not fail-open. The journal reason and the dashboard are just misleading, and an oversized PLAN skips its `status: superseded` check and counts as a surviving plan.

**Fix:** For the status readers, read a bounded prefix (the frontmatter is at the top) instead of refusing the file. Or map a too-large error to the `Unparseable` verification status and an unknown UAT status, so it is not read as "not written".

---

_Reviewed: 2026-11-27T06:25:01Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
