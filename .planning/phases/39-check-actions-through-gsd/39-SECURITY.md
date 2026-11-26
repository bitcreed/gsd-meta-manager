---
phase: "39"
slug: "check-actions-through-gsd"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-17"
---

# Phase 39 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Register source: the `<threat_model>` blocks of 39-01, 39-02 and 39-03 PLAN.md (authored at plan time).
SUMMARY threat flags: none beyond the plan registers. ASVS L1, block_on `high`; verification depth is
grep-level evidence plus a run of the named tests (all green on 2026-11-17).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| project `.planning/` text → instruction and popup | check names, file names, phase tokens, workstream names, ROADMAP phase names, `999.*` directory names | project-authored text (untrusted) |
| owner keystrokes → reason text / typed phase | free text in the reason field and the phase step | owner input |
| TUI → `claude` argv in the project | the prompt is one argument of a new Claude session that runs gsd-core verify-work (`sed \| xargs` parse) | slash command + instruction |
| TUI → live session | a running Claude session in the same project; focused, never driven | none (focus only) |
| TUI → clipboard / terminal (OSC 52) | prompt handed to the owner for pasting into a live session | cleaned prompt text, base64 |
| item cursor identity → settle target | a stale or vanished id after a watcher refresh | `OwnerItem::id` |
| test code → gsd-core oracle and temp copies | integration tests run `gsd-tools`/`bash` and write files | temp-dir fixtures |
| source tree → guard | the scan is the evidence that the app writes no GSD file | source text |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-39-01 | Elevation of privilege | `settle_mux_argv` / `settle_terminal_argv` (CWE-88) | high | mitigate | `src/ui/screens/detail.rs:3184-3205`: authored `owner_scrubbed_program()` then `plan.prompt()` as one element; prompt = `RunCommand` Display + instruction (`owner_settle.rs:458`); tokens via `is_command_token`/`is_workstream_name`; test `settle_argv_is_the_scrubbed_program_and_one_prompt_element` | closed |
| T-39-02 | Tampering | `clean` / `instruction` (xargs quoting, `--ws` redirect) | high | mitigate | `owner_settle.rs:180` maps `'` `"` `\`, collapses `-` runs; test `hostile_text_cannot_break_the_argument_parse_or_the_popup` | closed |
| T-39-03 | Spoofing | `settle_body` (bidi, zero-width, C0/C1) | medium | mitigate | `clean` drops `is_control` and `is_invisible_formatting_char`; popup values through `shown`; escape-guard fixtures in `settle_popup.rs` tests | closed |
| T-39-04 | Tampering | `SettleConfirmScreen::confirm` TOCTOU | medium | mitigate | `settle_recheck` re-resolves by identity (prompt, cwd, live-session presence); recheck tests in `owner_settle.rs`/`settle_popup.rs` | closed |
| T-39-05 | Tampering | root-scoped settle with `active-workstream` set | medium | mitigate | refused in `settle_target_for`; test `the_pointer_refuses_only_root_groups` | closed |
| T-39-06 | Tampering | native write into the project | high | mitigate | `tests/owner_settle_guard.rs` forbids `fs::write(`, `OpenOptions`, `Command::new(`, `std::process`, `.spawn()` in the settle modules; tracer flow compares fixture tree byte for byte | closed |
| T-39-07 | Tampering | prompt injection via project-authored check names | medium | mitigate | names single-line, capped at 80 chars, quoted; full argv shown in default-No popup; wording left to human UAT | closed |
| T-39-08 | Denial of service | second GSD session racing on STATE.md | medium | mitigate | live session with exact cwd focused, never relaunched; test `a_live_session_is_focused_not_relaunched` | closed |
| T-39-09 | Elevation of privilege | `move_target` / `SettlePhaseScreen` (CWE-88) | high | mitigate | typed entry ASCII digits, then letters/`.` only, `TYPED_PHASE_MAX = 16` (`settle_popup.rs:139,270`); M must pass `is_command_token` and sits inside the instruction after the command; tests `move_target_resolves_typed_phases`, `move_candidates_skip_completed_source_and_bad_tokens` | closed |
| T-39-10 | Spoofing | Move rows of `settle_body`, `SettlePhaseScreen` render | medium | mitigate | names pass `clean`; drawn values through `shown`/`fit_cells`; escape-guard fixtures | closed |
| T-39-11 | Tampering | item sub-cursor after a refresh | medium | mitigate | cursor holds `OwnerItem::id`; vanished id cleared; Yes recheck by id; tests `a_vanished_item_falls_back_to_its_row` (`inbox.rs`), `step_with_items_enters_and_leaves_rows` (`owner_block.rs`) | closed |
| T-39-12 | Tampering | Move instruction files a todo in the wrong scope / promotes 999.x | medium | mitigate | instruction names phase M number and roadmap name and forbids 999.x promotion; owner reads it in default-No popup; todo landing checked in human UAT | closed |
| T-39-13 | Tampering | `settle_recheck` for Move (roadmap renamed) | low | mitigate | target re-resolved from fresh candidates; test `move_recheck_refuses_a_renamed_phase` | closed |
| T-39-14 | Denial of service | `backlog_for` directory read on key press | low | accept | see AR-39-01 | closed |
| T-39-15 | Spoofing | live-session clipboard text differs from popup | medium | mitigate | copies `fresh.prompt()` after `settle_recheck`; test `a_live_session_gets_the_prompt_by_osc52` decodes the OSC 52 payload | closed |
| T-39-16 | Tampering | OSC 52 built from project-derived text | medium | mitigate | prompt is `clean`ed; `crate::clipboard::osc52` base64-encodes it with the existing 64 KiB cap | closed |
| T-39-17 | Elevation of privilege | injection into a live session | high | mitigate | settle path calls only `multiplexer::focus_message` and the clipboard; guard forbids process APIs in settle modules; test `a_live_session_gets_the_prompt_by_osc52` | closed |
| T-39-18 | Repudiation | `owner_settle_guard.rs` passing vacuously | medium | mitigate | `EXECUTABLE_FLOOR = 400`, per-file presence check, planted-line tests per marker (`owner_settle_guard.rs:45,171-177`) | closed |
| T-39-19 | Tampering | `owner_settle_oracle.rs` writing outside temp / running project-shipped `gsd-tools` | medium | mitigate | writes and `current_dir` inside `TempDir`; oracle only from `gsd_install::install_candidates` | closed |
| T-39-20 | Tampering | gsd-core verify-work argument-parse drift | low | mitigate | pinned pipeline lines asserted verbatim against installed `verify-work.md` in `tests/owner_settle_oracle.rs` | closed |
| T-39-SC | Tampering | npm/pip/cargo installs | low | accept | see AR-39-02 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-39-01 | T-39-14 | one `read_dir` of `phases/`, no file opened, only on `m` and on Yes; `async_blocking_guard` stays green | plan 39-02 (recorded unattended) | 2026-11-17 |
| AR-39-02 | T-39-SC | no package installed or added (RESEARCH Package Legitimacy Audit: none) | plans 39-01..03 (recorded unattended) | 2026-11-17 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-17 | 21 | 21 | 0 | gsd-secure-phase (orchestrator, L1 short-circuit; named tests run: owner_settle_guard 3, owner_settle_oracle 4, lib settle filters 50 — all pass) |

Decision (unattended): `threats_open: 0`, register authored at plan time, ASVS 1 → the workflow's
short-circuit applies, so no auditor subagent was spawned. Prompt-injection wording (T-39-07) and todo
landing (T-39-12) are additionally covered by the human UAT carried in 39-03-SUMMARY.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-17
