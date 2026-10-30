---
phase: "24"
slug: "roadmap-tab-redesign-and-detail-tab-consolidation"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
block_on: high
register_authored_at_plan_time: true
created: "2026-10-28"
updated: "2026-10-29"
audited_at: "e15f3e6"
---

# Phase 24 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
>
> Written during the 2026-10-28 re-verification of Phase 24 (verify:post hook). The
> code has moved on since execution: later quick tasks (workstream bands, lane
> corners, gsd-core 1.15 parsing parity) touched `src/state_reader/roadmap_md.rs`,
> `src/ui/roadmap_graph.rs`, `src/ui/roadmap_view.rs` and `src/ui/screens/detail.rs`.
> Every mitigation below was re-checked at HEAD `4e424e2`, not taken from the
> SUMMARY files. `PhaseList`, `layout_graph` and `RoadmapGraphWidget` were removed
> on purpose. The `PhaseList` hits that remain are `archive::ArchiveDepth::PhaseList`
> (unrelated), comments and test strings.
>
> Re-checked 2026-10-29 at HEAD `e15f3e6` after quick 261029-1dp (`ce86206`, removal of
> the dead `AgentRuntime` config key, doc-comment fixes in `src/ui/screens/detail.rs` and
> `src/ui/screens/mod.rs`). Of the files cited below only `detail.rs` changed since
> `4e424e2`, and only a test doc comment and an assert message at `:21260-21277`, below
> every cited anchor (highest `:18886`), so no anchor moved. No dependency changed
> (`Cargo.toml` / `Cargo.lock` untouched). See the audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| project `.planning/ROADMAP.md` / `STATE.md` → reader | third-party text that may be hostile (control chars, bidi, pathological size) | phase ids, names, goals, deps, milestone labels |
| private source repositories → public crate | fixtures copied from private roadmaps ship in `tests/` on crates.io | roadmap structure, phase titles |
| `ProjectState` → driver model seam | a new text field could reach a prompt without a label | none added |
| ROADMAP text → list model → terminal cells | the model is the last stop before cells; the widget is the last hop | escaped strings only |
| keypress / `opened_on` → tab index → sub-view | a wrong mapping lands on the wrong tab and runs that tab's arrival work | UI integrity |
| stored cursor → state after ROADMAP reload | an index into a list that has changed | UI integrity / DoS |
| archive directory contents → cells; background archive tasks → cache | milestone versions/names, a cache keyed by alias then milestone | `Untrusted` text, per-project isolation |
| render loop | must not do I/O | — |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation (evidence at HEAD) | Status |
|-----------|----------|-----------|----------|-------------|-------------------------------|--------|
| T-24-01 | Tampering | goal / milestone_name text | high | mitigate | Goals are stored as `Untrusted` (`src/state_reader/roadmap_md.rs:1050,1069`). `milestone_name: Option<Untrusted>` (`src/state_reader/mod.rs:107`). Band label is `Untrusted` (`roadmap_md.rs:1381,1665`). The probe `render_escape_guard::the_screen_renders_identity_escaped` passes | closed |
| T-24-02 | DoS | regexes / range expansion over hostile ROADMAP | medium | mitigate | `regex` crate only, linear time (`roadmap_md.rs:80-308`, `OnceLock`). Range expansion is bounded by known ids: `a_build_dependency_range_expands_only_to_known_ids` (`roadmap_md.rs:2768`, `1-999999999` → known ids only) passes. Since the gsd-core 1.15 parity work, GSD `Depends on` ranges are read as their endpoints with no expansion (`parse_depends_on`, `roadmap_md.rs:163`), which is stricter still | closed |
| T-24-03 | Info Disclosure | vendored private roadmaps in public crate | high | mitigate | `vendored_roadmap_fixtures_are_sanitised` (`roadmap_md.rs:3118`) passes. The residual (kept daily-vow/sentriq titles) is accepted, see AR-24-01 | closed |
| T-24-04 | EoP | goals reaching the driver model seam | medium | mitigate | The new text fields are `Untrusted`, not `String`. `src/driver/untrusted.rs` was last changed in Phase 31 (`9caf162`), not by Phase 24. `tests/spawn_seam_guard.rs` 10/10 pass | closed |
| T-24-05 | Tampering | build phases leaking into GSD-facing logic | high | mitigate | Separate `ProjectState.planned_phases` (`src/state_reader/mod.rs:78,932`). No `planned_phases` reference under `src/driver/`. `ttbook_fixture_gsd_phase_list_is_unchanged_by_build_phase_support` passes. `tests/driver_router_conformance.rs` 6/6 pass (oracle present locally) | closed |
| T-24-06 | Tampering / Spoofing | escape/bidi sequences in model strings | high | mitigate | `list_model_stores_only_escaped_text` (`src/ui/roadmap_graph.rs:2862`) passes | closed |
| T-24-07 | DoS | cyclic / deep / huge dependency graphs | medium | mitigate | Iterative `break_cycles` (`roadmap_graph.rs:106`). `list_cycle_terminates_with_one_note`, `list_many_cycles_collapse_into_one_bounded_note`, `list_large_chain_stays_one_lane` and `list_many_roots_zig_zag_within_two_lanes` pass | closed |
| T-24-08 | DoS | multibyte ids/names | low | mitigate | Widths come from `chars().count()`, never byte slicing (`src/ui/roadmap_view.rs:17,205`; `roadmap_graph.rs:712,723`) | closed |
| T-24-09 | Tampering (integrity) | `tab_index` / `sub_view_from_index` / digit arms | medium | mitigate | `tab_index` and `sub_view_from_index` (`src/ui/screens/detail.rs:2275,2315`). `every_tab_index_round_trips_through_its_sub_view`, `nine_and_zero_are_inert_in_the_detail_view` and `the_driver_sub_view_is_the_last_tab_index_in_both_directions` pass | closed |
| T-24-10 | Repudiation / Info Disclosure | escape-guard coverage after PhaseList removal | medium | mitigate | `ALL_SUB_VIEWS` (`src/ui/screens/render_escape_guard.rs:1519`) and `DETAIL_TAB_ARRIVAL` set equality. `the_screen_census_matches_the_tree` and `every_adjudicated_screen_has_a_probe_fixture` pass | closed |
| T-24-11 | DoS | stored Driver view with flag off | low | mitigate | `effective_sub_view` (`detail.rs:2348`) falls back to `RoadmapViz`, used at the read sites (`detail.rs:3820` and others) | closed |
| T-24-12 | Tampering / Spoofing | escape/bidi reaching cells via name/goal/band label | high | mitigate | `roadmap_view_stores_and_draws_only_escaped_text` (`roadmap_view.rs:2566`) passes. The widget draws only model strings | closed |
| T-24-13 | DoS | panic on multibyte truncation / tiny terminal | medium | mitigate | `roadmap_view_never_panics_at_tiny_sizes` and `a_tiny_area_renders_without_panicking` pass. Truncation counts chars | closed |
| T-24-14 | DoS | lane explosion on wide fan-out | low | mitigate | `LANE_CAP_NARROW = 4` / `LANE_CAP_WIDE = 6` (`roadmap_view.rs:40,42`). `lanes_past_the_cap_collapse_into_one_overflow_column` passes | closed |
| T-24-15 | Tampering (integrity) | Roadmap → Phases / Archive handoff | medium | mitigate | Handoff through `tab_index`. The target is matched by `phase_key` (`detail.rs:1729,1956`) | closed |
| T-24-16 | DoS | cursor / `pipeline_selected` out of range after reload | medium | mitigate | Cursor is stored as a key and re-resolved by `resolve_cursor` (`roadmap_graph.rs:1359`). `pipeline_selected.min(last)` clamps (`detail.rs:1749,3553`) | closed |
| T-24-17 | Tampering | build-phase id in status message | medium | mitigate | `crate::text::render_for_terminal(&planned.number)` before formatting (`detail.rs:1717-1719`) | closed |
| T-24-18 | EoP | Roadmap key arm reaching the driver | low | accept | No Roadmap arm starts, stops or injects. Driver arms stay guarded by `current_view == Driver`. See AR-24-02 | closed |
| T-24-19 | Tampering / Spoofing | goal, planned-phase name, `milestone_name` reaching cells | high | mitigate | `render_escape_guard`: `the_planned_phase_arrives_whole_in_the_roadmap_detail_pane` and `the_screen_renders_identity_escaped` pass | closed |
| T-24-20 | DoS | file I/O on render thread | medium | mitigate | `render_roadmap` (`detail.rs:6897-7013`) has no `std::fs` / `fs::` / `read_to_string`. `roadmap_model_for` (`detail.rs:2850`) reads in-memory state only | closed |
| T-24-21 | Repudiation (silent regression) | reported defects returning | medium | mitigate | Fixture render tests pass: `roadmap_model_for_daily_vow_has_one_row_for_phase_20`, `roadmap_model_for_sentriq_uses_a_synthetic_band`, `roadmap_model_for_ttbook_lists_build_phases_as_phases`, `roadmap_model_for_a_renumbered_roadmap_lists_the_current_phase_once` | closed |
| T-24-22 | Tampering (integrity) | `opened_on` / digits / `m` routing | medium | mitigate | `switch_to_sub_view` (`detail.rs:3580`) is the single arrival rule. `opened_on_archive_lands_on_docs_milestones` (`detail.rs:18886`) passes | closed |
| T-24-23 | Info Disclosure / Integrity | archive cache fix regression (cross-project milestone) | high | mitigate | All five debug-fix tests pass by exact name, `5 passed` (`archive_milestone_view_keeps_its_content_across_the_periodic_prune`, `the_same_milestone_version_in_two_projects_does_not_share_a_cache_entry`, `archive_milestone_view_reloads_in_place_when_planning_files_change`, `the_archive_milestone_list_picks_up_a_newly_archived_milestone_in_place`, `an_in_place_reload_that_shrinks_the_listing_keeps_the_cursor_on_a_row`) | closed |
| T-24-24 | Tampering | milestone names in strip / breadcrumb | low | mitigate | The strip is static text. Names go through the existing `Untrusted::shown()` paths, which the escape probe covers (green) | closed |
| T-24-25 | DoS | `m` swallowing typed text | low | mitigate | The `m` arm is guarded by `matches!(current_view, Browse \| Archive)` (`detail.rs:5304-5305`). Neither sub-view has a text-input mode | closed |
| T-24-SC | Tampering | package installs | low | accept | No dependency was added in any of plans 24-01..07. See AR-24-03 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-24-01 | T-24-03 (residual) | The sanitisation guard proves that Goal bodies are replaced and that no home paths remain. What it leaves is daily-vow/sentriq phase titles and generic milestone names. Some of these are already public in `MOCKUPS.md`. The fixtures have also shipped on crates.io since v1.10.0 (`Cargo.toml` excludes only `/.planning/`, `/CLAUDE.md`, `/assets/`), so the exposure cannot be reversed, and removing them now would not reduce it. 24-01-PLAN marked this "accepted [INFERRED — audit]" and flagged it for the operator. `[inferred — cleanup 2026-10-28]` | plan 24-01 / secure-phase agent (operator sign-off pending) | 2026-10-28 |
| AR-24-02 | T-24-18 | No Roadmap key arm reaches the driver. The disposition comes from the plan. | plan 24-05 | 2026-10-28 |
| AR-24-03 | T-24-SC | No package was installed. RESEARCH § Package Legitimacy Audit found no new packages. | plans 24-01..07 | 2026-10-28 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-28 | 26 | 26 | 0 | gsd-secure-phase (State B, ASVS L1 short-circuit; mitigations re-checked at HEAD `4e424e2`; 49 + 5 + 3 lib tests, `spawn_seam_guard` 10, `driver_router_conformance` 6, all pass) |
| 2026-10-29 | 26 | 26 | 0 | gsd-secure-phase (State A re-check at HEAD `e15f3e6` after 261029-1dp; ASVS L1 short-circuit, auditor skipped; cited files diffed `4e424e2..HEAD`, only `detail.rs` test doc comment changed; anchors spot-checked; 32 cited lib tests, `spawn_seam_guard` 10, `driver_router_conformance` 6 pass via `rtk proxy cargo test --locked --no-fail-fast`; no disposition changed `[inferred — cleanup 2026-10-28]`) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-28, re-verified 2026-10-29 at `e15f3e6` (AR-24-01 operator acknowledgement still advisable `[inferred — cleanup 2026-10-28]`)
