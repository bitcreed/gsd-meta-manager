---
phase: 38-inbox-digest-and-triage-hygiene
verified: 2026-11-17T18:30:00Z
status: human_needed
score: 5/5 must-haves verified
covered_files:
  - ".planning/phases/38-inbox-digest-and-triage-hygiene/38-01-PLAN.md"
  - ".planning/phases/38-inbox-digest-and-triage-hygiene/38-01-SUMMARY.md"
  - ".planning/phases/38-inbox-digest-and-triage-hygiene/38-02-PLAN.md"
  - ".planning/phases/38-inbox-digest-and-triage-hygiene/38-02-SUMMARY.md"
  - ".planning/phases/38-inbox-digest-and-triage-hygiene/38-03-PLAN.md"
  - ".planning/phases/38-inbox-digest-and-triage-hygiene/38-03-SUMMARY.md"
  - ".planning/phases/38-inbox-digest-and-triage-hygiene/38-04-PLAN.md"
  - ".planning/phases/38-inbox-digest-and-triage-hygiene/38-04-SUMMARY.md"
  - "README.md"
  - "src/agent_skill.md"
  - "src/app.rs"
  - "src/cli.rs"
  - "src/config.rs"
  - "src/inbox.rs"
  - "src/inbox_cli.rs"
  - "src/main.rs"
  - "src/state_reader/owner_items/gate.rs"
  - "src/state_reader/owner_items/labels.rs"
  - "src/state_reader/owner_items/mod.rs"
  - "src/state_reader/owner_items/triage.rs"
  - "src/ui/screens/help.rs"
  - "src/ui/screens/inbox.rs"
  - "src/ui/screens/owner_actions.rs"
  - "src/ui/screens/owner_block.rs"
  - "src/ui/screens/render_escape_guard.rs"
  - "tests/inbox_cli.rs"
  - "tests/owner_digest.rs"
covered_digest: "v4:sha256:efe94357e41f3ec02887d5e7f7f5896958a4c8254def09fdf1429d32c8898729"
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Digest and TUI inbox look on the real fleet. Run plain `gsd-meta-manager inbox`, then open the TUI inbox (`i`) at 120 and 100 columns in your real terminal and theme."
    expected: "The digest fits one screen and its NEXT list feels like the right five things to do. In the TUI, same-phase UAT+VERIFICATION pairs are one row, the Stale fold sits after Old debt, the checks column aligns its digits, and the DIM `decision?` / `looks resolved` labels are legible against your theme."
    why_human: "Ranking feel, DIM color in a real theme and glyph width on the owner's font are subjective or environment-bound. Tests pin text and goldens only."
  - test: "Label accuracy on real data. Scan `inbox --all` or `--json` on the real fleet for `decision?` and `looks resolved` labels."
    expected: "Labels are useful enough as hints. IN-04 (false positives for accept, review, resolved) was skipped on purpose as label-only per D-11; confirm that noise level is acceptable."
    why_human: "Heuristic precision on the owner's real wording is a product judgment, not a goal failure."
---

# Phase 38: Inbox digest and triage hygiene Verification Report

**Phase Goal:** Plain `gsd-meta-manager inbox` answers "what should I do next?" in one screen: a short digest with totals, the `closes after` gate and the few best-payoff next actions, full detail one flag away, phases collapsed in every multi-project view, old and duplicated items folded, likely rulings or already-resolved rows labeled (never hidden or moved).
**Verified:** 2026-11-17
**Status:** human_needed (all automated truths verified; two look-and-feel items remain)

## Goal Achievement

### Observable Truths

| # | Truth (requirement) | Status | Evidence |
|---|---------------------|--------|----------|
| 1 | OWN-24: plain `inbox` is a short digest with totals, `closes after` per project, top-5 ranked NEXT | VERIFIED | Built binary over six scratch-registry fixtures printed 19 lines: totals line, six `closes after` lines, 5 NEXT rows ranked closing-phase first then oldest, `Stale:` line, hint. `triage::next_actions`, 25-line budget with worst-case test (20 projects), golden `inbox-digest.txt`. |
| 2 | OWN-25: `inbox <alias>` full checklist, `--lane` one lane across projects, `--all` legacy output, `--json` additive | VERIFIED | `inbox scripted-checklist` printed the full 51 steps; `--lane test` printed one lane; `--all` printed 102 lines (legacy goldens `inbox.txt`, `inbox-include-all.txt` unchanged); `--json` summary gained only `next_actions`, `stale_folded`; schema pins only add paths in `tests/inbox_cli.rs`. |
| 3 | OWN-26: phases collapsed in every multi-project view (CLI digest, lane views, TUI inbox) | VERIFIED | The 51-step `scripted-checklist` is one line (`51 checks`) in the digest and lane view. TUI rows come from `owner_block::inbox_sources` over the same `triage_entries`; unit tests in `src/ui/screens/inbox.rs` and the golden-pinned owner block. Visual check is under human_verification. |
| 4 | OWN-27: >90 d fold into one stale line, same-phase duplicates merge, checks column aligns | VERIFIED | `Stale: 5 checks in 3 phases older than 90 d` in the digest; `same-phase-merge` shows one row `3 checks -> closes phase - 2 files`; `fold_days` preference (default 90) in config.rs; `checks_cell` alignment with `Grid.checks_w` (IN-01 fix), three owner-block goldens updated. |
| 5 | OWN-28: label-only heuristics `decision?` / `looks resolved`, additive `--json` fields, never hide or move | VERIFIED | `labels.rs` matcher; `heuristic-labels` row shows `4 decision?` and `1 looks resolved` while counts and rows stay in place; `items[].labels` in `--json`; test pins near misses ("not resolved yet"). |

**Score:** 5/5 truths verified

### Requirements Coverage

| ID | Plans claiming it | Status | Notes |
|----|-------------------|--------|-------|
| OWN-24 | 38-01, 38-03 | SATISFIED | digest; was marked complete at 38-01 and is genuinely delivered there, extended by 38-03 |
| OWN-25 | 38-01, 38-02, 38-03 | SATISFIED | `--all` (01), `ALIAS` (02), `--lane` and `--json` (03) all present |
| OWN-26 | 38-01, 38-03, 38-04 | SATISFIED | CLI at 01/03, TUI at 04; the early checkbox was true only for the CLI at 38-01, now complete across all three views |
| OWN-27 | 38-01, 38-02, 38-04 | SATISFIED | fold and merge in the model (01), TUI alignment (04) |
| OWN-28 | 38-01, 38-02, 38-04 | SATISFIED | matcher (01), fixture (02), TUI DIM labels (04) |

All five IDs are checked in `.planning/REQUIREMENTS.md` (lines 115-129, traceability 229-233); no checkbox change needed. No orphaned IDs.

### Code Review

38-REVIEW: 1 warning (WR-01, fixed in acce9eb1) and 5 info (4 fixed, IN-04 skipped deliberately under D-11). Disposition open: 0.

### Behavioral Spot-Checks

| Check | Result |
|-------|--------|
| `cargo test --test inbox_cli --test owner_digest` | 36 + 43 passed |
| `cargo test --lib inbox` | 98 passed |
| Full run reported by the caller | 4488 passed, clippy clean |
| Binary against copied fixtures (scratchpad only, nothing in real projects) | digest 19 lines, lane, alias, `--all`, `--json` as above |

### Anti-Patterns

None found. 38-01 noted a mid-history commit (b787296c) that does not build alone; HEAD builds. Not a goal gap.

### Human Verification Required

See frontmatter: real-terminal look of the digest and TUI inbox, and label noise on real data.
