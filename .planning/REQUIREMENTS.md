# Requirements: v2.5.0 Owner Inbox

**Milestone:** v2.5.0
**Defined:** 2026-11-14
**Core Value:** See the state of every GSD project at a glance and act on any of them without leaving the TUI.
**Source:** `.planning/research/OWNER-INBOX-UX.md` (owner-inbox UX report; evidence: `gsd-tools query
audit-uat --raw` over 20 registered projects, snapshot 2026-11-13, gsd-core 1.21.0, plus a file survey
of the same projects). The report is the source of truth for the data model (section 4), the screen
mockups (section 3) and the `inbox` JSON schema (section 3.5); requirements below reference it by
section rather than restating it.

> **Scoping note.** Started unattended. The report's requirements OWN-01..23, its four-phase split
> (34-37) and its defaults for every open decision (D1-D12, section 7) are adopted as written. The
> project-level 4-researcher fan-out was skipped because the report already covers features,
> architecture and pitfalls; the phase researcher does the per-phase deep research at
> `/gsd-plan-phase` **[inferred]**. Calls the orchestrator made are marked **[inferred]**.

## Problem

The owner runs ~20 GSD projects and keeps asking Claude to "find the missing UATs". The app reads only
the `status:` line of `NN-UAT.md` / `NN-VERIFICATION.md`: `status: testing` is not outstanding,
`needs_human` covers only paused / external-job / parked, and archived milestones are never scanned.
gsd-core's own `audit-uat` returns 568 raw items for 17 readable projects, which reduce to about 43
checks the owner can run today (report section 1).

## v2.5.0 Requirements

### Reader and rules (OWN-01..12)

- [x] **OWN-01**: The state reader produces per-item UAT/VERIFICATION data natively (Layer A, report
  section 4) for active and archived phases, root and workstreams. *Test: for every fixture in
  `tests/fixtures/owner/`, the set of (file, test, result, category) equals
  `gsd-tools query audit-uat --raw`.*
- [x] **OWN-02**: The conformance test uses the oracle pattern of `tests/driver_router_conformance.rs`:
  resolved through `gsd_install::install_candidates`, fails when the oracle is missing unless
  `ALLOW_MISSING_ORACLE` is set, fails if any fixture produced no comparison; a source scan asserts no
  non-comment line under `src/` names `audit-uat` as a spawned verb. *Test: itself, and it passes in
  `pre-tag-check.sh --container`.*
- [x] **OWN-03**: R2: a VERIFICATION named in a same-phase UAT `source:` contributes no items; a
  `gaps_found` VERIFICATION is never superseded; no `source:` match shows both (D3 strict). *Test:
  fixture modeled on wordoclock ph 20-25 yields 18 items, not 39.*
- [x] **OWN-04**: R3: `## Gaps` entries with `test: N` merge into issue row N; Fix groups report failed
  checks and gaps separately. *Test: mailbot-ph09 fixture yields 10 failed checks and 14 gaps in one
  Fix group.*
- [x] **OWN-05**: R4: acknowledged files (`audit_acknowledged`, matched as gsd-core's
  `isAuditItemAcknowledged`) are suppressed while the snapshot matches and resurface when it does not
  or the marker is malformed; they are counted, never listed by default. *Test: four fixtures (match,
  changed count, changed status, marker missing `at`), each also compared with the oracle's
  `acknowledged_files`.*
- [x] **OWN-06**: R5/R6: archived and deferred items never enter current counts; a `complete` archived
  UAT with skipped rows yields an Archived group; `deferred-items.md` entries are a count only. *Test:
  daily-vow-v1.5 fixture; deferred-items fixture with 200 entries yields `hidden.deferred == 200` and
  zero groups.*
- [x] **OWN-07**: R7: `closes_phase` and the bookkeeping-only case are computed as specified. *Test:
  table-driven unit test over the 8 combinations (fix present, blocked present, gaps_found, stale,
  missing summary, parse gap, zero open, clean).*
- [ ] **OWN-08**: R8: `closes_after` lists lane totals and open phases for the current milestone only;
  phases with no plans are named "not built", not counted as owner work.
- [x] **OWN-09**: `status: testing` (and any non-terminal UAT status with open items) counts as
  outstanding; `status_cli`'s `uat_outstanding` attention item fires for it. *Test: regression
  fixture; existing `tests/status_cli.rs` key paths unchanged.*
- [ ] **OWN-10**: The Decide/Fix signals of report section 4 ("Other signals") are read with the stated
  fields: paused handoff, human actions pending, phase checkpoint, debug sessions awaiting confirmation
  (Decide, D2) and open investigations (Fix), `gaps_found` verifications, CONTEXT open questions minus
  RESOLVED/ANSWERED/CLOSED bullets (D11), pending-todo count and oldest age. Debug status is normalized
  (`-`/`_`, case) and `knowledge-base.md` and `resolved/` are ignored. *Test: one fixture per row of
  the signal table.*
- [x] **OWN-11**: Safety: 1 MiB read cap, symlink containment, no panic on invalid UTF-8 or
  unterminated frontmatter, every rendered string through `text::render_for_terminal`. *Test: hostile
  fixtures plus the existing `render_escape_guard` coverage extended to the new screens.*
- [x] **OWN-12**: Cost: parsed files are cached by (path, mtime, len) and invalidated by the watcher; a
  cold `inbox --json` over 20 projects finishes in < 1 s; no parse runs on the render path. *Test:
  `async_blocking_guard` extended; a timing assertion on a 20-project synthetic tree.*

### CLI (OWN-13..15)

- [x] **OWN-13**: `gsd-meta-manager inbox --json` emits schema `gsd-meta-manager/inbox` v1 (report
  section 3.5), deterministic (no timestamps, sorted), exit codes as `status`; writes nothing and takes
  no lock. *Test: `tests/inbox_cli.rs` pins every key path against the README contract.*
- [x] **OWN-14**: `inbox` without `--json` prints a plain checklist grouped by lane; `--lane
  test|decide|fix|blocked` and `--include archived,deferred,acknowledged,todos` filter as documented.
  *Test: golden output.*
- [x] **OWN-15**: `status --json` carries the additive per-project `owner: {test, decide, fix_phases,
  blocked, closeable_phases}` (no schema bump); `status --skill` documents `inbox` as the way to find
  missing UATs and pending owner work.

### TUI (OWN-16..23)

- [ ] **OWN-16**: Dashboard `You` column per report section 3.1: at most two lanes in fixed order
  `decide`, `test`, `fix`; never `blocked`; current milestone only, archived debt as dim `(+N old)`;
  hidden below 100 columns; dim `·` when clean. *Test: buffer snapshots at 80/100/140 columns.*
- [ ] **OWN-17**: The summary line shows totals and `[i] inbox`; totals equal the sum of the visible
  cells.
- [ ] **OWN-18**: AttentionFirst gains a `has owner work` rank between "needs a human" and "driven";
  `/y [lane]` filters. `needs_human` and the `❢` badge are unchanged (D1). *Test: rank table unit
  test; existing needs_human tests untouched.*
- [ ] **OWN-19**: The detail Phases tab shows the Owner block of report section 3.2 (no new tab, D5)
  with expandable groups, the `closes after` header, and collapsed Old debt / Acknowledged / Todos
  folds.
- [ ] **OWN-20**: Inbox screen (`i` on the dashboard) per report section 3.3: lane filters `1-4`/`a`,
  the ordering rule (closes-a-phase, then milestone progress, then age), expand, `g` go to project,
  `o` open file, `y` copy checklist. Legible at 100x30.
- [ ] **OWN-21**: `v` runs the group's command through `multiplexer::launch` (focusing an existing
  live session instead when one exists); the argv is built only from the validated phase token and
  workstream name via `ExecArgv::new`.
- [ ] **OWN-22**: `A` launches gsd-core's `gsd-tools audit acknowledge …` after a confirm popup (D7);
  the app itself writes nothing into the project. The row disappears on the next watcher refresh.
- [ ] **OWN-23**: Help overlay and README ("Owner inbox" section plus the `inbox` schema) updated.

## Decisions adopted from the report (section 7)

All defaults taken as written **[inferred — milestone start 2026-11-14]**:

| # | Decision | Adopted default |
|---|---|---|
| D1 | Widen `needs_human` / `❢` to owner work? | No; `You` text cell and a separate sort rank |
| D2 | Debug sessions awaiting confirmation | Decide lane, command `/gsd-debug` |
| D3 | Dedup when UAT `source:` does not name the VERIFICATION | Strict: no supersede, show both |
| D4 | Archived debt on the dashboard | Dim `(+N old)` only; full list in inbox fold and `--include archived` |
| D5 | New detail tab vs block | Block on the Phases tab |
| D6 | `inbox` subcommand vs growing `status` | Separate `inbox`; `status` gets counts only |
| D7 | Acknowledge writer | Launch gsd-core's `audit acknowledge`; no native write |
| D8 | `unknown` rows without a result in terminal files | Hidden and counted; `failed`/`partial` go to Fix |
| D9 | Stale threshold | 30 days, `owner.stale_days` in the app config |
| D10 | Keys | `i` inbox, `/y` filter, `v` run, `A` acknowledge, `y` copy |
| D11 | Open-question bullets marked RESOLVED/ANSWERED | Filtered out by our layer |
| D12 | Milestone and version | v2.5.0 "Owner Inbox", phases 34-37, coarse granularity |

Additional orchestrator calls:

- OWN-13 and OWN-15 are delivered in Phase 34 and extended (new lanes and `closes_after`) in Phase 35;
  OWN-23 is delivered across Phases 36 and 37. Traceability maps each to the phase that completes it
  **[inferred]**.
- The exact `gsd-tools audit acknowledge` CLI spelling and per-category flags are confirmed during
  Phase 37 planning, as the report asks **[inferred]**.
- Upstream gsd-core issues (report section 8) are not milestone requirements; they are reported
  separately **[inferred]**.

## Future Requirements

Deferred (report section 6):

- **OWN-F01**: STATE.md `### Blockers/Concerns` parsing
- **OWN-F02**: Heuristic "this UAT row is really a ruling" detection
- **OWN-F03**: Todo triage UI beyond a count
- **OWN-F04**: Seeds and threads as owner work
- **OWN-F05**: Native acknowledge writer
- **OWN-F06**: Per-item pass/fail entry from the TUI
- **OWN-F07**: Desktop notifications
- **OWN-F08**: A daily digest file

## Out of Scope

| Feature | Reason |
|---|---|
| Widening `needs_human` / the `❢` badge | 11 of 20 projects would light up; the badge stays reserved for hard stops (D1) |
| Per-item badges on the dashboard | Rows are phases, not items; one quiet text cell per project |
| `deferred-items.md` entries as inbox rows | Mostly over-parsed prose; shown as a count only (R6) |
| Escalation/checkpoint files in phase dirs, `async-jobs/`, threads | Not present in any of the 20 surveyed projects |
| Spawning `audit-uat` (or any gsd-core process) from the reader | State must be read from files without running GSD; enforced by the OWN-02 source scan |
| Any native write into a project | Non-intrusive constraint; acknowledge goes through gsd-core (D7) |
| Fixing gsd-core's `audit-uat` behavior | Upstream concern (report section 8); our Layer B compensates |

## Traceability

| Requirement | Phase | Status |
|---|---|---|
| OWN-01 | Phase 34 | Complete |
| OWN-02 | Phase 34 | Complete |
| OWN-03 | Phase 34 | Complete |
| OWN-04 | Phase 34 | Complete |
| OWN-05 | Phase 34 | Complete |
| OWN-06 | Phase 34 | Complete |
| OWN-07 | Phase 34 | Complete |
| OWN-08 | Phase 35 | Pending |
| OWN-09 | Phase 34 | Complete |
| OWN-10 | Phase 35 | Pending |
| OWN-11 | Phase 34 | Complete |
| OWN-12 | Phase 34 | Complete |
| OWN-13 | Phase 34 | Complete |
| OWN-14 | Phase 34 | Complete |
| OWN-15 | Phase 34 | Complete |
| OWN-16 | Phase 36 | Pending |
| OWN-17 | Phase 36 | Pending |
| OWN-18 | Phase 36 | Pending |
| OWN-19 | Phase 36 | Pending |
| OWN-20 | Phase 37 | Pending |
| OWN-21 | Phase 37 | Pending |
| OWN-22 | Phase 37 | Pending |
| OWN-23 | Phase 37 | Pending |

**Coverage:**
- v2.5.0 requirements: 23 total
- Mapped to phases: 23
- Unmapped: 0 ✓

---
*Requirements defined: 2026-11-14*
*Last updated: 2026-11-14 after milestone v2.5.0 start*
