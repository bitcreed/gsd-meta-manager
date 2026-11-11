---
phase: "33"
slug: "terminal-multiplexer-backend-seam-herdr-support"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-10"
---

# Phase 33 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register source: the `<threat_model>` blocks of 33-01-PLAN.md and 33-02-PLAN.md (authored at plan time).
Both SUMMARY files report `## Threat Flags: None` (no unplanned surface). ASVS level 1, block_on `high`.
Verification depth: L1 (grep plus test-name evidence). Per the workflow short-circuit rule
(threats_open 0, register authored at plan time, ASVS 1), the gsd-security-auditor was not spawned.

Re-audit 2026-11-10 (after the 33-REVIEW-FIX commits 8425be8..bce8ebf touched src/multiplexer/herdr.rs and mod.rs):
every mitigation re-grepped against current code; citations refreshed to current line numbers. No mitigation was
weakened. WR-01 tightens T-33-09 (cwd tier no longer focuses a plain shell or a contradicted pane); WR-02's
canonicalization only affects which panes are probed, not the exact-equality match; WR-03 removed the unused
`is_try_next` (fall-through on `RefusedForSafety` reuses no refused vector); UI-01 keeps raw backend errors out of the
status line (logged via tracing only). No new spawn site or CLI verb; no new threat surface.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| TUI to tmux server | argv vector crosses into `tmux new-window`; tmux execs directly only with 2+ arguments | program argv, cwd |
| TUI to herdr server socket | one NDJSON `layout.apply` request over a same-user Unix socket; herdr execs the command array | program argv, cwd |
| Untrusted session id to argv / join | resume id scraped from another process's argv; travels as one fused element, compared (never passed) in the herdr join | untrusted string |
| Process env and /proc to detection | `TMUX`, `HERDR_*` read once into `EnvSnapshot`; comm/ppid of same-user ancestors order backends | env values, process metadata |
| herdr CLI output to the TUI | pane/tab ids, cwds, titles parsed tolerantly as data; ids become argv of herdr's own parser | ids, paths, titles |
| Test runner to live seam | `cargo test` inherits `HERDR_ENV=1` inside herdr | test isolation |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-33-01 | Tampering / EoP | herdr launch path | high | mitigate | Launch via one `layout.apply` request with `command` = `ExecArgv` elements (src/multiplexer/herdr.rs:94-113, `open` at 289); tests `launch_inside_herdr_opens_a_tab_through_layout_apply` (mod.rs:1707) and `layout_apply_request_carries_a_hostile_element_as_one_array_element` (herdr.rs:782) | closed |
| T-33-02 | Tampering | ExecArgv sink | high | mitigate | Private tuple field, derives only `Debug, Clone` (no Default/From); `ExecArgv::new` refuses <2 elements and `-` head (mod.rs:214-235; refusal text made backend-neutral by IN-02, 80e56ad); test `exec_argv_refuses_a_vector_tmux_would_hand_to_an_interpreter` (mod.rs:880) | closed |
| T-33-03 | Tampering | cwd transport | medium | mitigate | tmux `-c` argv element (tmux.rs:147), herdr JSON `cwd` (herdr.rs:108, asserted herdr.rs:795 / mod.rs:1728), GUI `Command::current_dir` (gui.rs:110-115) | closed |
| T-33-04 | Tampering | herdr JSON encoding | medium | mitigate | Request built with `serde_json::json!` (herdr.rs:100); hostile element `lit a b;touch X $HOME *` round-trips unchanged (herdr.rs:782-798) | closed |
| T-33-05 | Denial of Service | herdr socket on UI thread | medium | mitigate | Connect on helper thread (herdr.rs:357) bounded by `CONNECT_TIMEOUT` 2 s, write timeout, `REPLY_TIMEOUT` 5 s deadline, `MAX_REPLY_BYTES` 1 MiB cap (herdr.rs:333-338; worst case ~9 s documented by IN-04, 401f652); test `a_listener_that_closes_without_a_reply_is_a_failure` (herdr.rs:874) | closed |
| T-33-06 | Tampering | test suite vs real seam | medium | mitigate | `multiplexer::launch(` / `focus_message(` called only from src/ui/screens/detail.rs (4952, 5378, 5415) and src/ui/screens/normal.rs:840; tests inject backends/env/spawner | closed |
| T-33-07 | Tampering | resume of a Codex row | medium | mitigate | `resumable_session_id` runs before `resolve_launch_plan` (detail.rs:4934-4946); test `resumable_session_id_refuses_every_codex_session` (detail.rs:21869) | closed |
| T-33-08 | Repudiation | new spawn sites | low | mitigate | tests/spawn_seam_guard.rs `SPAWN_ALLOWLIST` lists src/multiplexer/gui.rs, herdr.rs, tmux.rs (lines 62-68) | closed |
| T-33-09 | Tampering | herdr focus join | high | mitigate | Exact session-id / integer pid equality, ambiguity refused on every tier (herdr.rs:191-245, 671-771). Post-review: WR-01 (8425be8) narrows the cwd tier to agent panes of the session's kind with unreadable process-info (`cwd_fallback`, herdr.rs:760) and labels it `matched by working directory only`; WR-02 (d947b32) widens candidate selection to canonicalized-equal paths (`same_dir`, herdr.rs:710) but the match itself stays exact pid/session-id. Tests `pid_tier_never_matches_a_pid_that_is_only_a_prefix` (1186), `the_order_of_panes_never_decides_the_match` (1278), `session_id_tier_refuses_two_panes_with_the_same_value` (1160), `an_ambiguous_cwd_is_refused_and_nothing_is_focused` (1544), `a_plain_shell_pane_in_the_same_directory_is_never_focused` (1625), `a_pane_whose_process_info_names_other_processes_is_not_the_session` (1640) | closed |
| T-33-10 | Tampering (CWE-88) | pane/tab ids into herdr argv | medium | mitigate | `validate_id` refuses empty and `-`-leading ids (herdr.rs:628-636), applied before every id reaches argv (herdr.rs:215, 245, 263); tests `validate_id_refuses_empty_and_dash_leading_ids` (1074), `a_dash_leading_pane_id_is_refused_before_it_reaches_argv` (1656) | closed |
| T-33-11 | Elevation of Privilege | herdr CLI verbs | high | mitigate | `CLI_VERBS` allowlist (pane list, pane process-info, agent focus, tab focus) checked before `Command::new` (herdr.rs:434-475); test `herdr_cli_refuses_every_verb_outside_the_allowlist` asserts fixture log empty for pane run/send-text/send-keys, agent start (herdr.rs:1669) | closed |
| T-33-12 | Spoofing | /proc comm ancestry | low | accept | See Accepted Risks Log | closed |
| T-33-13 | Denial of Service | herdr CLI calls on UI thread | low | accept | See Accepted Risks Log | closed |
| T-33-14 | Information Disclosure | herdr JSON titles | low | accept | See Accepted Risks Log; `terminal_title` appears only in test fixtures (herdr.rs:1021-1022), never rendered or logged | closed |
| T-33-SC (01) | Tampering | package installs | low | accept | See Accepted Risks Log | closed |
| T-33-SC (02) | Tampering | package installs | low | accept | See Accepted Risks Log | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-33-01 | T-33-12 | Same-user processes only and the env gate must also hold; worst case is a wrong but real backend, which still execs directly through ExecArgv | plan-time threat model (33-02-PLAN) | 2026-11-10 |
| AR-33-02 | T-33-13 | Each herdr CLI call measured under 10 ms; a wedged server could stall blocking `.output()`, same residual as the existing tmux CLI path (documented at herdr.rs herdr_cli doc comment) | plan-time threat model (33-02-PLAN) | 2026-11-10 |
| AR-33-03 | T-33-14 | Terminal titles are never rendered or logged; status lines carry only pane counts and fixed text | plan-time threat model (33-02-PLAN) | 2026-11-10 |
| AR-33-04 | T-33-SC | No package installed or added; std, serde, serde_json already present | plan-time threat model (33-01/33-02-PLAN) | 2026-11-10 |

*Accepted risks do not resurface in future audit runs.*

[inferred] Acceptance attribution: the user was unavailable; these risks are carried forward as accepted from the plan-time threat models rather than re-accepted interactively. Flagged for audit.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-10 | 16 | 16 | 0 | gsd-secure-phase orchestrator (L1 short-circuit, auditor not spawned) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-10

## Security Audit 2026-11-11

| Metric | Count |
|---|---|
| Threats found | 16 |
| Closed | 16 |
| Open | 0 |
