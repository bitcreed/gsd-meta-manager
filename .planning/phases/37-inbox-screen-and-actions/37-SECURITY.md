---
phase: "37"
slug: "inbox-screen-and-actions"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-15"
---

# Phase 37 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| project `.planning/` CONTEXT/PLAN/SUMMARY files → owner reader | untrusted project text parsed into Decide-lane entries and the planned-phase gate | untrusted project-authored text |
| owner digest → typed `RunCommand` → `v` launch argv | project-derived phase tokens and workstream names become argv for a new session | validated tokens |
| inbox/Owner block rows → terminal | untrusted strings drawn into the terminal | untrusted project-authored text |
| `o` viewer → project file | read-only file read bounded to `.planning/` | file bytes |
| `A` popup → `gsd-tools audit-open acknowledge` child | one bounded child process; gsd-core is the only writer | argv values, scrubbed env |
| `y` → clipboard tool / OSC 52 | checklist text leaves the process | project-derived text |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-37-01 | Tampering / DoS | CONTEXT entry parser | medium | mitigate | `sanitize_for_display` on every entry (`owner_items/context.rs`), scan size cap, linear 10,000-entry test | closed |
| T-37-02 | Spoofing | planned-question hiding | medium | mitigate | hidden questions counted in `planned_questions`; unreadable files stay listed (`rules.rs`) | closed |
| T-37-03 | Spoofing | planned-phase gate | low | mitigate | gate requires a regular file (`context.rs`), directory-named-like-plan test | closed |
| T-37-30 | Tampering | typed `RunCommand` builders | high | mitigate | `is_command_token` / `is_workstream_name` reject non-ASCII digits and leading `-`; command is only `RunCommand`'s `Display` | closed |
| T-37-31 | Tampering | Owner block goldens | medium | mitigate | goldens written once (38e66683), read at test time | closed |
| T-37-04 | Elevation | `v` launch argv | high | mitigate | launch-time re-validation (`owner_actions.rs`); command is one argv element; cwd is the project path | closed |
| T-37-05 | Tampering | row rendering | high | mitigate | `shown` / `fit_cells` on every cell; `InboxScreen` escape-census fixture | closed |
| T-37-06 | Spoofing | live-session focus match | medium | mitigate | exact `working_dir == project.path` | closed |
| T-37-07 | Elevation | focus branch | medium | mitigate | only `multiplexer::focus_message`; no send-keys/pane-run | closed |
| T-37-08 | Tampering | cursor identity | medium | mitigate | identity-resolved, clamped cursor every frame | closed |
| T-37-09 | DoS | layout degrade | medium | mitigate | `degrade_never_panics` down to 0x0; cursor-vanish tests | closed |
| T-37-10 | Spoofing | loading vs unreadable | medium | mitigate | explicit `Unreadable` state; `OwnerDigestFailed` handler tests | closed |
| T-37-11 | Tampering | header/fold lines | high | mitigate | numbers and fixed words only; census fixture | closed |
| T-37-12 | Info disclosure | failed-digest set | low | mitigate | storable-alias guard, prune, cleared on digest | closed |
| T-37-13 | Info disclosure | `o` path containment | high | mitigate | canonicalized `.planning/` prefix check, `is_file` on handle; `..`, directory, symlink tests | closed |
| T-37-14 | DoS | `o` file size | medium | mitigate | 1 MiB cap read on one handle (`read_capped`, hardened by WR-06) | closed |
| T-37-15 | Tampering | viewer text | high | mitigate | `render_for_terminal` per line; census row | closed |
| T-37-16 | Elevation | no `$EDITOR` / spawn from viewer | medium | mitigate | no `Command::new`/write in phase-37 UI non-test code; `o` pushes the viewer | closed |
| T-37-17 | Spoofing | `g` target | low | mitigate | `goto_key` uses the row alias | closed |
| T-37-18 | Tampering | acknowledge env | high | mitigate | `ACK_ENV_REMOVE` applied to the child; `--ws` always passed for workstream groups; root groups refused while a pointer is set | closed |
| T-37-19 | Tampering | acknowledge argv | high | mitigate | `is_flag_value` on every value (WR-04 extends it to `--ws`); one argv element per value; refusal tests | closed |
| T-37-20 | Tampering | confirm/Yes race | medium | mitigate | `ack_recheck` re-derives the target on Yes; busy set; argv shown is argv run | closed |
| T-37-21 | DoS | acknowledge child | medium | mitigate | 10 s budget, kill and reap, stderr to tempfile, `spawn_blocking`; spawn failure reported distinctly (WR-02) | closed |
| T-37-22 | Tampering | popup and outcome text | high | mitigate | every dynamic value through `shown`; outcome through `render_for_terminal`; stderr capped | closed |
| T-37-23 | Tampering | no native write | medium | mitigate | no fs write/commit in phase-37 non-test code; copy says "uncommitted" | closed |
| T-37-24 | Elevation | launcher resolution | low | accept | plan-time accept; WR-05 now resolves the launcher from the user's install only | closed |
| T-37-25 | Tampering | OSC 52 | high | mitigate | base64-only payload, written after the frame from `pending_osc52` | closed |
| T-37-26 | DoS | clipboard tool | medium | mitigate | null stdio, stdin closed, 2 s kill budget, `spawn_blocking` | closed |
| T-37-27 | Tampering | copied checklist | low | mitigate | `group_checklist` uses `terminal_capped`, same source as the CLI | closed |
| T-37-28 | Elevation | clipboard argv | medium | mitigate | five constant argv vectors; text on stdin; spawn allowlist entry | closed |
| T-37-29 | Info disclosure | clipboard path disclosure | low | accept | plan-time accept; status names the tool or OSC 52 | closed |
| T-37-SC | Supply chain | dependencies | low | accept | `Cargo.lock` unchanged since v2.4.0; no dependency added | closed |

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-37-01 | T-37-24 | Launcher is the user's own gsd-core install; the popup shows the exact argv before Yes | plan-time disposition (37-06 PLAN) | 2026-11-15 |
| AR-37-02 | T-37-29 | Naming the clipboard tool in the status line discloses nothing beyond the local environment | plan-time disposition (37-07 PLAN) | 2026-11-15 |
| AR-37-03 | T-37-SC | No dependency was added or installed in this phase | plan-time disposition (37-01..07 PLAN) | 2026-11-15 |

*Accepted risks do not resurface in future audit runs.*

---

## Notes

- Register authored at plan time (all seven PLANs carry `<threat_model>`); ASVS level 1, block_on high. Verified by the gsd-security-auditor agent (code read and grep).
- SUMMARY Threat Flags: none raised in 37-01..07.
- Code review (37-REVIEW.md): 12 findings, 0 open after fixes (37-REVIEW-DISPOSITION.md). Unregistered residuals from 37-REVIEW-FIX.md, non-blocking: IN-02 (`v` launch environment), WR-01 residual (bracketed paste, cooldown), WR-05 residual (no `env_clear` for the acknowledge child).

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-15 | 32 | 32 | 0 | gsd-secure-phase (gsd-security-auditor, L1) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-15
