---
phase: "36"
slug: "dashboard-and-detail"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-14"
---

# Phase 36 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| project `.planning/` files → owner digest (background task) | untrusted project text and file sizes parsed by the Phase 34-35 reader inside `spawn_blocking` | untrusted project-authored text |
| background task → App state (`Action` channel) | digests arrive asynchronously and may race each other | in-memory digests |
| `AppContext.owner_digests` → dashboard summary row and `You` cell | numbers and fixed words only, sharing a frame with project-derived aliases | counts |
| filter text typed by the owner → `parse_filter` | local input parsed into a closed enum | local input |
| dashboard `i` key → status line | must not trigger any action outside the TUI | none |
| project `.planning/` text → Owner block header, rows, expanded lines and fold lines | untrusted strings drawn into the terminal | untrusted project-authored text |
| Owner block keys → project | the block stays read-only; actions belong to Phase 37 | none |
| refreshed digest → stored cursor and expansion | state from one digest applied to another | view state |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-36-01 | Denial of service | digest computation on render/async path | high | mitigate | `read_owner_digest` called only inside `spawn_blocking` (`src/app.rs` `schedule_reparse`, `src/ui/screens/mod.rs` `schedule_owner_digest`); listed in `BLOCKING_HELPERS` (`tests/async_blocking_guard.rs:136`); absent from normal.rs, main.rs, detail.rs, owner_block.rs | closed |
| T-36-02 | Tampering | first-fill result racing a reparse | medium | mitigate | `OwnerDigestLoaded` inserts only when `!owner_digests.contains_key` (`src/app.rs:3706`); `a_first_fill_never_replaces_a_reparse_digest` (`src/app.rs:5931`) | closed |
| T-36-03 | Spoofing | `You` cell before the digest arrives | medium | mitigate | `None` renders blank; `a_project_without_a_digest_has_a_blank_you_cell` (`src/ui/screens/normal.rs:3195`); WR-01 fix keeps gone/`.planning`-less projects without a digest | closed |
| T-36-04 | Denial of service | startup fan-out of blocking tasks | low | mitigate | tasks spawn via `spawn_blocking` after the first state load; skip non-present projects | closed |
| T-36-05 | Information disclosure | digest kept for an unregistered alias | low | mitigate | `owner_digest_storable` guard; `owner_digests.retain` in prune (`src/app.rs:3781`); removal drops digest (WR-02); exhaustive destructure test (`src/app.rs:6459`) | closed |
| T-36-06 | Tampering | hostile alias/digest text on dashboard | medium | mitigate | cell and summary hold numbers and fixed words; escape-guard state with `hostile_owner_digest` (`src/ui/screens/render_escape_guard.rs:1015`) | closed |
| T-36-07 | Elevation of privilege | dashboard `i` key | low | mitigate | status message only; `the_i_key_points_at_the_inbox_command` (`src/ui/screens/normal.rs:2977`) | closed |
| T-36-08 | Spoofing | summary row totals | low | mitigate | `owner_totals` (`src/ui/screens/owner.rs:216`) over the cells' numbers; `summary_totals_equal_the_sum_of_the_visible_cells` (`normal.rs:2874`); pending shown as `M loading` | closed |
| T-36-09 | Tampering / Spoofing | escape/bidi/zero-width injection in Owner block header, rows, fold line | high | mitigate | every digest string through `super::detail::shown` (17 call sites in `src/ui/screens/owner_block.rs`); Owner-block escape-guard state; `a_long_text_digest_stays_inside_the_block` (`detail.rs:36100`) | closed |
| T-36-10 | Denial of service | huge digest rendered every frame | low | mitigate | height cap, visible window only, in-memory reads; no digest computation in detail.rs/owner_block.rs | closed |
| T-36-11 | Spoofing | row command the owner may copy | low | mitigate | commands from token-checked builder (`is_command_token`, `src/state_reader/owner_items/rules.rs:74`) and drawn through `shown()`; nothing runs them | closed |
| T-36-12 | Elevation of privilege | Owner block keys | medium | mitigate | `Enter`/`Space` toggle view state only; `e o v A y` swallowed; `owner_block_swallows_action_keys` (`detail.rs:36239`); no process, fs or write path in owner_block.rs | closed |
| T-36-13 | Denial of service (panic) | cursor/scroll after watcher refresh | medium | mitigate | identities + clamp; `owner_block_state_survives_a_digest_refresh` (`detail.rs:36628`) | closed |
| T-36-14 | Tampering / Spoofing | injection through expanded item, gap, file and open-fold lines | high | mitigate | all strings through `shown()`; expanded escape-guard state; `an_expanded_long_text_block_stays_inside_the_block` (`detail.rs:36722`) | closed |
| T-36-15 | Denial of service | expanded group with thousands of items/gaps | low | mitigate | `MAX_ITEMS = 5`, `MAX_GAPS = 2` with `… N more` lines (`owner_block.rs:48-50, 912-925`); windowed body | closed |
| T-36-SC | Tampering | npm/pip/cargo installs | low | accept | no package added; `Cargo.toml` untouched in phase 36 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-36-01 | T-36-SC | No dependency was added or installed in this phase; supply-chain surface unchanged | plan-time disposition (36-01..04 PLAN) | 2026-11-14 |

*Accepted risks do not resurface in future audit runs.*

---

## Notes

- Register authored at plan time (all four PLANs carry `<threat_model>`); ASVS level 1, block_on high. Per the workflow short-circuit, L1 grep-depth verification was done by the orchestrator and the auditor agent was not spawned.
- SUMMARY Threat Flags: none raised in 36-01..04.
- Code review (36-REVIEW.md): 12 findings, all fixed (36-REVIEW-DISPOSITION.md, open: 0); WR-01, WR-02 and WR-04 strengthen T-36-03, T-36-05 and T-36-01.
- Evidence run 2026-11-14: `cargo test --locked --lib` filtered to owner/escape-guard tests (63 passed) and `--test async_blocking_guard` (8 passed).

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-14 | 16 | 16 | 0 | gsd-secure-phase (orchestrator, L1 grep) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-14
