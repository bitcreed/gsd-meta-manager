---
phase: "38"
slug: "inbox-digest-and-triage-hygiene"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-17"
---

# Phase 38 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| project `.planning/` text → digest, `inbox ALIAS`, lane view → terminal | phase, item, workstream and file names reach new CLI lines | untrusted project-authored text |
| project `.planning/` text → `inbox --json` → agents | project strings reach new JSON keys (`next_actions`, `entry_files`, `labels`) | untrusted project-authored text |
| project `.planning/` text → TUI buffer | merged rows, `file` sub-headings, extras, label suffixes, Stale fold | untrusted project-authored text |
| registry config → `fold_days` | a user-edited preference decides what folds | user config value |
| TUI key press → acknowledge writer / multiplexer launch | `A` and `v` on a row that may stand for several files | validated argv |
| owner's real projects → public fixtures and SUMMARY evidence | live shapes and D-14 numbers enter a public crate | metadata and counts only |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-38-01 | Tampering | digest NEXT/project lines, `entry_checklist` file sub-lines, label suffixes | medium | mitigate | `terminal_capped` on file name, item and gap names in `entry_checklist` and on alias/name/command in `collapsed_of` (`src/inbox.rs`); labels are two constants; `hostile_strings_are_escaped_in_the_digest_and_lane_views` (`tests/inbox_cli.rs`) | closed |
| T-38-02 | Repudiation | fold, merge and project cap hide work | medium | mitigate | `Stale:`, `+N more projects`, `Unreadable:` lines count what is not shown; merged entries keep every part's items (`entry_checklist`); totals and `owner` untouched | closed |
| T-38-03 | Elevation | synthetic command of a merged entry | medium | mitigate | entry reuses a part's own validated `RunCommand`; CLI only prints it; TUI launch re-validates (see T-38-11) | closed |
| T-38-04 | DoS | label regexes on hostile or huge text | low | accept | Rust `regex` is linear time; static patterns (`owner_items/labels.rs`) | closed |
| T-38-05 | Tampering | `preferences.owner.fold_days` hostile value | low | mitigate | `owner_fold_days` clamps to 1..=3650, non-integers read 90; test `owner_fold_days_defaults_to_ninety_and_clamps` (`src/config.rs`) | closed |
| T-38-06 | Tampering | `summary.next_actions[]`, `entry_files`, collapsed lane lines | medium | mitigate | `terminal_capped` / `opt_text` on alias, workstream, phase in `next_actions` JSON; `command` is the typed command's text; `hostile_strings_are_escaped_in_json_and_text` and the lane-view hostile test | closed |
| T-38-07 | Repudiation | agents misread `closes_phase` or `folded_stale` | low | mitigate | README row for `summary.next_actions[].closes_phase`; `status --skill` (`src/agent_skill.md`) says labels and `folded_stale` never change a count | closed |
| T-38-08 | Info disclosure | D-14 evidence in a committed SUMMARY | low | mitigate | 38-03-SUMMARY holds numbers only; `inbox_writes_nothing_anywhere` proves the run read-only | closed |
| T-38-09 | Tampering | `A` on a merged row | medium | mitigate | refused when `row.src.parts.len() > 1` before any group lookup (`src/ui/screens/inbox.rs`); test `a_merged_row_refuses_acknowledge` | closed |
| T-38-10 | Tampering | escape injection through TUI sub-headings, extras, labels, Stale fold | medium | mitigate | project strings through `shown`; `render_escape_guard.rs` draws the merged row and the Stale fold from hostile text | closed |
| T-38-11 | Elevation | `v` on a merged row | medium | mitigate | one part's own `RunCommand`; `run_plan` refuses unreadable sources (`owner_actions.rs`); test `v_on_a_merged_row_runs_the_shared_command` pins the argv | closed |
| T-38-12 | DoS | `triage_entries` on every frame | low | accept | pure, linear, no I/O; `async_blocking_guard` covers the render path | closed |
| T-38-13 | Tampering | `inbox ALIAS` lines | medium | mitigate | lines come from `entry_checklist` / `group_checklist` / `totals_line` / `failed_lines`; `inbox evil` hostile run, written and fresh copy | closed |
| T-38-14 | Repudiation | single-project view hides folded or merged work | medium | mitigate | `STALE (> N d)` section lists folded entries; merged entries list every item under `file NAME`; alias golden | closed |
| T-38-15 | Info disclosure | real project text in public fixtures/goldens | medium | mitigate | fixtures hold invented text; grep of the four new fixtures and three new goldens for private project names finds none | closed |
| T-38-SC | Supply chain | dependencies | low | accept | no dependency added; `Cargo.lock` unchanged in phase 38 (only `[profile.dev.package.regex*]` opt-level entries in `Cargo.toml`) | closed |

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-38-01 | T-38-04 | `regex` guarantees linear-time matching; patterns are static | plan-time disposition (38-01 PLAN) | 2026-11-17 |
| AR-38-02 | T-38-12 | Triage is pure and linear over in-memory groups; no I/O on the render path | plan-time disposition (38-04 PLAN) | 2026-11-17 |
| AR-38-03 | T-38-SC | No package added or installed; lockfile unchanged | plan-time disposition (38-01..04 PLAN) | 2026-11-17 |

*Accepted risks do not resurface in future audit runs.*

---

## Notes

- Register authored at plan time (all four PLANs carry `<threat_model>`); ASVS level 1, block_on high. No threat is high, all 16 closed at grep/code-read depth, so the workflow short-circuits past the auditor (L1 rule). Verified by the orchestrator, unattended.
- SUMMARY Threat Flags: none raised in 38-01..04.
- Code review (38-REVIEW.md): 6 findings, 0 open (38-REVIEW-DISPOSITION.md); IN-04 (label false positives) skipped, a hint-quality issue with no security effect since labels never change counts or actions.
- [inferred] `Cargo.toml` dev-profile opt-level entries for `regex` crates are build settings, not a dependency change; T-38-SC stays accepted.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-17 | 16 | 16 | 0 | gsd-secure-phase (orchestrator, L1 short-circuit) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-17
