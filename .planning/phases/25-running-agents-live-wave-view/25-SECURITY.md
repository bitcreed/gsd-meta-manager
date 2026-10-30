---
phase: "25"
slug: "running-agents-live-wave-view"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-28"
updated: "2026-10-29"
audited_at: "42a8e18"
---

# Phase 25 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register source: the `<threat_model>` blocks of 25-01..25-07-PLAN.md (authored at plan time). The `## Threat Flags` sections of 25-01..25-05-SUMMARY.md report no new surface beyond the register; 25-06 and 25-07 SUMMARY have no Threat Flags section. ASVS level 1 and 0 open threats at plan-register time, so per secure-phase step 3 this was an L1 (grep-depth plus test-run) verification and no auditor was spawned. Audited at HEAD `54a73bf` (2026-10-28), after review fixes WR-03..06 and quick tasks (notably 260926-06g) had touched `src/agents/**` and `tests/agents_*`.

Test evidence (`cargo test --no-fail-fast`, 2026-10-28, HEAD `54a73bf`): agents_claude 13/13, agents_fixers 10/10, agents_process 7/7, agents_scan 13/13, agents_unmerged 10/10, agents_waves 9/9, agents_workstreams 4/4, agents_worktree_plans 9/9 (1 ignored), async_blocking_guard 8/8, plus lib unit tests filtered to the named mitigation tests, `commits_ahead` and `render_escape_guard` 31/31. 0 failures.

Dependency check: `git log <25-01 first commit>^..<25-07 last commit> -- Cargo.toml Cargo.lock` is empty.

Re-checked 2026-10-29 at HEAD `42a8e18` after quick 261029-1dp (`ce86206`, removal of the dead `AgentRuntime` config key and doc-comment fixes). `git diff 54a73bf..HEAD` over every cited file: only `src/ui/screens/detail.rs` changed, and only a test doc comment and an assert message at `:21260-21277`, below every cited anchor in it (highest `:19157`), so no anchor moved; `src/agents/adapters/mod.rs` changed only a module doc comment (`:15-17`) that no row cites. `src/agents/**` no longer references `src/executor/runtime.rs` (deleted). No dependency changed (`Cargo.toml` / `Cargo.lock` untouched). Tests re-run: agents_claude 13, agents_fixers 10, agents_process 7, agents_scan 13, agents_unmerged 10, agents_waves 9, agents_workstreams 4, agents_worktree_plans 9 (1 ignored), async_blocking_guard 8, cited lib tests 37, all pass, 0 failures.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| git porcelain output -> parser | paths, branch names and lock reasons controlled by another process or a hostile clone | untrusted text |
| scan -> live agent worktree | an index refresh or `index.lock` would interfere with a running agent | git index state (availability) |
| adapter code -> core scan | an adapter reading an undocumented format may fail or panic | adapter verdicts |
| Claude config root -> adapter | undocumented JSON rewritten while agents run; transcripts hold the user's conversation | meta JSON, transcript mtimes (privacy-sensitive) |
| agent id / plan id -> filesystem path | ids from worktree dirs, branches or agent text become path components | path components |
| agent description / commit subjects -> attribution and estimate | agent-authored text selects plan ids, phases and finding ids | untrusted text |
| blocking scan task -> event loop | results cross a channel into render-read state | `ProjectAgents` per alias |
| `ctx.agent_views` -> terminal | descriptions, agent types, branches and paths are agent- or clone-authored | untrusted display text |
| keypress -> sub-view | a mis-routed key could launch a session or swallow text | user input |
| runtime-written files -> classifier -> Status cell | mtimes, meta flags and lock state decide what the dashboard claims is running | liveness verdicts |
| REVIEW.md -> frontmatter reader | large file in `.planning/` | findings counts |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-25-01 | Tampering (live agent availability) | `git_read_raw`, `dirty_count` | high | mitigate | `--no-optional-locks` plus `GIT_OPTIONAL_LOCKS=0` in the one wrapper (`src/state_reader/git_ops.rs:171-180`); `scanning_a_live_agent_worktree_never_touches_its_index` (`tests/agents_scan.rs:354`) passing | closed |
| T-25-02 | Tampering (option injection) | `commits_ahead`, `scan_worktrees` | medium | mitigate | base accepted only as 40/64 lowercase hex (`src/state_reader/git_ops.rs:661`); `--output=x` refused (`git_ops.rs:1892`) | closed |
| T-25-03 | Tampering / Info Disclosure (path traversal) | agent id, ledger read | medium | mitigate | `valid_agent_id` `^[A-Za-z0-9_-]{1,64}$` (`src/agents/worktrees.rs:335`); admin dir must be absolute and exist (`worktrees.rs:396`) | closed |
| T-25-04 | Denial of Service | many projects / worktrees | low | mitigate | `.git/worktrees` prefilter skips git when no linked worktree exists (`src/agents/worktrees.rs:430`) | closed |
| T-25-05 | Denial of Service | panicking adapter | medium | mitigate | `catch_unwind` per adapter (`src/agents/mod.rs:343-344`), per project (`mod.rs:647`), adapter construction (`mod.rs:640`) and around the whole scan (`src/app.rs:1858`); residual accepted (AR-25-01) | closed |
| T-25-06 | Tampering (repository integrity) | all of `src/agents/` | high | mitigate | `no_file_under_src_agents_writes_reads_proc_or_spawns` (`tests/agents_scan.rs:632`) walks the directory at runtime, passing at HEAD; process facts arrive as an injected `ProcessProbe` snapshot (`src/agents/processes.rs:4-8`, see notes) | closed |
| T-25-07 | Spoofing (terminal escapes) | branch, lock reason | medium | mitigate | wrapped with `Untrusted::from_untrusted_source` at parse (`src/agents/processes.rs:199`, `unmerged.rs:218`, `worktree_plans.rs:198`, `worktree_phases.rs:180`); rendering covered by T-25-22 | closed |
| T-25-08 | Information Disclosure (privacy) | transcripts (`*.jsonl`) | high | mitigate | stat only; `read_meta_capped` refuses non-`.meta.json` paths (`src/agents/adapters/claude.rs:65`); `the_meta_reader_refuses_a_transcript_path` (`claude.rs:877`) passing | closed |
| T-25-09 | Denial of Service | oversized / hostile meta | medium | mitigate | `META_READ_CAP` = 64 KiB via `Read::take` (`src/agents/adapters/claude.rs:57`); `Value` parse with `.ok()` | closed |
| T-25-10 | Tampering (path traversal) | `agent-<id>.meta.json` lookup | medium | mitigate | `valid_agent_id` re-checked before join; `an_unsafe_agent_id_never_reaches_the_filesystem` (`src/agents/adapters/claude.rs:907`) passing | closed |
| T-25-11 | Spoofing | crafted `.claude/` in a cloned project | low | mitigate | metas read only from the resolved user config root (`src/agents/adapters/claude.rs:84-107`), never the project tree | closed |
| T-25-12 | Spoofing | relative `CLAUDE_CONFIG_DIR` | low | mitigate | ignored unless non-empty and absolute (`src/agents/adapters/claude.rs:107`); `a_relative_or_empty_config_dir_falls_back_to_home` (`claude.rs:777`) passing | closed |
| T-25-13 | Spoofing (display integrity) | lock-reason pid | medium | mitigate | adapter parses nothing of the pid (`src/agents/adapters/claude.rs:22-29`); core reads it only as a death signal, a live owner never upgrades a row (`src/agents/processes.rs:10-12`, see notes); `a_locked_worktree_with_a_stale_transcript_reads_stalled_even_with_a_live_pid` (`tests/agents_claude.rs:244`) passing | closed |
| T-25-14 | Tampering (terminal escapes) | `agentType`, `description` | medium | mitigate | wrapped `Untrusted` at extraction, rendered via `.shown()`; covered by T-25-22 probe | closed |
| T-25-15 | Tampering (path traversal) | worktree SUMMARY check | medium | mitigate | `PlanRef::from_id` validation; dir from `find_phase_dir` over main's listing; `hostile_or_malformed_ids_never_attribute` (`src/agents/waves.rs:966`) passing | closed |
| T-25-16 | Repudiation / display integrity | plan states | low | mitigate | `finished` kept separate from `done`; frontmatter `wave:` the only wave source; `tests/agents_waves.rs` 9/9 | closed |
| T-25-17 | Tampering (option injection) | `log_subjects` | medium | mitigate | range `HEAD` or `<hex>..HEAD`, `-n` bounded (`src/state_reader/git_ops.rs:708`); `log_subjects_refuses_a_range_that_is_not_head_or_a_hex_base` (`git_ops.rs:1932`) passing | closed |
| T-25-18 | Denial of Service | overlapping / stuck scans | medium | mitigate | `agents_scan_in_flight` gate (`src/app.rs:1844`, set `:1854`, cleared first in handler `:2200`); closure in `catch_unwind` always sends; `no_second_agents_scan_spawns_while_one_is_in_flight` (`app.rs:5550`), `the_agents_scan_handler_clears_the_in_flight_flag_for_an_empty_result` (`app.rs:5569`) passing | closed |
| T-25-19 | Denial of Service (frozen UI) | blocking I/O on the async loop | medium | mitigate | scan only inside `spawn_blocking` (`src/app.rs:1855`); `tests/async_blocking_guard.rs` 8/8 | closed |
| T-25-20 | Information Disclosure | tracing output | low | mitigate | `src/agents` tracing calls log counts and alias only (`unmerged.rs:306`, `worktree_plans.rs:214`, `worktree_phases.rs:194`, `mod.rs:347,641,651`); no description logged | closed |
| T-25-21 | Integrity (stale entries) | `agent_views` for removed projects | low | mitigate | wholesale replacement plus registered-alias filter (`src/app.rs:2205-2209`); `an_agents_scan_for_an_unregistered_alias_is_dropped` (`app.rs:5607`) passing | closed |
| T-25-22 | Tampering / Spoofing (escape, bidi) | Agents sub-view text | high | mitigate | Agents probe row in `render_escape_guard` (`src/ui/screens/render_escape_guard.rs:1291`); `the_dashboard_agent_summary_draws_no_identity` (`:3482`) passing | closed |
| T-25-23 | Denial of Service | `m` swallowing typed text | low | mitigate | guarded `m` arms on Sessions/Agents only (`src/ui/screens/detail.rs:5304`, `:5323`) | closed |
| T-25-24 | Elevation (unintended action) | `n` / `Enter` on Agents | low | mitigate | `enter_and_n_do_nothing_on_the_agents_sub_view` (`src/ui/screens/detail.rs:19157`) passing | closed |
| T-25-25 | Tampering (option injection) | git ranges in estimate | medium | mitigate | phase text only in regex comparison and `find_phase_dir`; ranges via `log_subjects` allowlist (T-25-17) | closed |
| T-25-26 | Repudiation / transparency | the estimate | low | mitigate | always rendered with `~`, widest form says `fixed` (`src/agents/waves.rs:794-796`; `the_fixer_estimate_is_a_whole_ladder_form_at_80_120_and_200`, `src/ui/screens/normal.rs:3797`); shown only while a fixer is running (`src/agents/fixers.rs:289`); REVIEW-FIX.md clause superseded by WR-04 (see notes) | closed |
| T-25-27 | Denial of Service | large REVIEW.md | low | mitigate | `REVIEW_READ_CAP` = 256 KiB (`src/agents/fixers.rs:51`, applied `:224`); computed only behind the running-fixer gate (`:289`) | closed |
| T-25-28 | Spoofing (status misrepresentation) | `classify_facts`, `is_active`, `derive` vote, Status cell | medium | mitigate | `MAX_AGENT_AGE_SECS` age bound -> `Ended` (`src/agents/mod.rs:121`); `an_aged_out_orphan_leaves_the_status_cell_byte_identical` (`src/ui/screens/normal.rs:3706`), `a_finished_orphan_within_the_age_bound_leaves_the_status_cell_byte_identical` (`:3768`), `a_released_lock_orphan_days_stale_reads_ended_and_is_not_active` (`tests/agents_claude.rs:305`), `running_agents_outvote_orphans_for_the_active_phase` (`src/agents/waves.rs:1227`) passing | closed |
| T-25-29 | Tampering (future-dated mtime) | `age_secs` | low | accept | clock-skew clamp to age 0 (`src/agents/mod.rs:124`); display-only, same-user only (AR-25-02) | closed |
| T-25-30 | Tampering (destructive repair) | orphaned worktrees | high | mitigate | classify only, never prune/unlock/remove/write; enforced by `no_file_under_src_agents_writes_reads_proc_or_spawns` (`tests/agents_scan.rs:632`) passing | closed |
| T-25-31 | Denial of Service | `fixers::estimate` | low | mitigate | running-fixer gate returns before any git call or file read (`src/agents/fixers.rs:289`); `a_finished_fixer_run_yields_no_estimate` (`tests/agents_fixers.rs:480`) passing | closed |
| T-25-32 | Information Disclosure | logs | low | mitigate | no description/type in any `src/agents` tracing call (see T-25-20) | closed |
| T-25-SC | Tampering | package installs | low | accept | no new crates: Cargo.toml / Cargo.lock untouched across 25-01..25-07 commits (AR-25-03) | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-25-01 | T-25-05 (residual) | A panic inside an adapter is caught, but the global panic hook still runs (restores the terminal) before unwinding is caught; adapters are panic-free by construction. | plan-time disposition (25-01 PLAN) | 2026-10-28 |
| AR-25-02 | T-25-29 | A future-dated transcript mtime reads `Live` via the D-C08 clamp; only a same-user process can set it, effect is display-only and no action follows from liveness. | plan-time disposition (25-07 PLAN) | 2026-10-28 |
| AR-25-03 | T-25-SC | Phase adds no dependencies; verified Cargo.toml / Cargo.lock untouched across phase commits. | plan-time disposition (25-01..25-07 PLAN); confirmed by secure-phase run | 2026-10-28 |

*Accepted risks do not resurface in future audit runs.*

### Notes for later audit (outside the register, not scored)

- `[inferred — cleanup 2026-10-28]` T-25-13's plan text says the lock-reason pid is "never parsed". Quick 260926-06g added `parse_lock_owner` in core `src/agents/processes.rs:40`, used solely as a death signal (gone owner -> `Ended`); a live owner never upgrades a row, and the Claude adapter still parses nothing of it. The threat (a live pid making a stalled agent look live) remains mitigated and the pinning test passes, so it is scored closed.
- `[inferred — cleanup 2026-10-28]` T-25-06: the same quick task gave `src/agents` process evidence, but as an injected `ProcessProbe` snapshot taken by the caller (`src/app.rs:1859`); `src/agents` itself still reads no process table, and the runtime guard test passes. Scored closed.
- `[inferred — cleanup 2026-10-28]` T-25-26: the "hidden once REVIEW-FIX.md exists" clause was removed by review fix WR-04 (`src/agents/fixers.rs:179-181`), because a stale REVIEW-FIX.md hid the count for later iterations. Transparency now rests on the `~` marker and the running-fixer gate, which still hides the estimate once the run is over. Scored closed.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-28 | 33 | 33 | 0 | gsd-secure-phase at HEAD `54a73bf` (orchestrator, L1 short-circuit; no auditor per ASVS 1 rule) |
| 2026-10-29 | 33 | 33 | 0 | gsd-secure-phase (State A re-check at HEAD `42a8e18` after 261029-1dp; ASVS L1 short-circuit, auditor skipped; cited files diffed `54a73bf..HEAD`, only comment-level changes in `detail.rs` / `adapters/mod.rs`, no cited anchor moved; 9 integration suites + 37 cited lib tests pass via `rtk proxy cargo test --no-fail-fast`; no disposition changed `[inferred — cleanup 2026-10-28]`) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-28 at HEAD `54a73bf`, re-verified 2026-10-29 at `42a8e18`
