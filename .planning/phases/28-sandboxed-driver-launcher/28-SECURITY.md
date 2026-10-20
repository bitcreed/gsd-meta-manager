---
phase: "28"
slug: "sandboxed-driver-launcher"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-20"
---

# Phase 28 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Register authored at plan time (28-01..28-09 `<threat_model>` blocks). Audited by gsd-security-auditor at HEAD a5b2801, after the code-review fixes (0951aea..b678e78).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| sandboxed driver -> host filesystem | everything outside project + config dir read-only, credentials unreadable | host credential files (secret) |
| sandboxed driver -> host IPC (`$XDG_RUNTIME_DIR`) | agent, bus and user-manager sockets are capability channels | IPC capabilities |
| sandboxed driver -> run dir / lock files | driver must not take the run lock or edit tool-owned files | LOG.md, state.json, locks |
| sandboxed driver -> hook client -> store gate | every hook payload untrusted; counts from payload shape | dispatch/gap counts |
| launcher -> child process (argv, env) | argv world-readable in /proc; env is not | OAuth access token (secret) |
| host settings.json -> per-run settings | user settings can carry auth overrides | auth config |
| CLI args -> run dir path and prompt file | names where the store writes | paths |
| driver-controlled checkout -> host-side git / remote | `.git/config`, hooks, history may be hostile; push uses real credentials | git credentials, repo contents |
| driver-writable VERIFICATION.md / CLAUDE_CONFIG_DIR -> counters | files can be forged | gap / stop-guard counts |
| run-dir names and frontmatter -> terminal | driver-influenced text reaches render path | escape sequences |
| harness -> paid model usage; live artefacts -> repository | real spend; evidence is committed | USD, transcripts |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-28-01 | Spoofing | second launcher as second writer | high | mitigate | run lock before `Session::start` (`run.rs:522`), rc 3; `launcher_e2e.rs:439,480` | closed |
| T-28-02 | Tampering | driver writes LOG/state/locks | high | mitigate | ro-bind run_dir + `/dev/null` over locks (`sandbox.rs:275-280`); live probe | closed |
| T-28-03 | Information Disclosure | OAuth token in argv/files | high | mitigate | env only (`run.rs:1292`); Debug redacts (`auth.rs:62`); `launcher_e2e.rs:182` | closed |
| T-28-04 | Tampering | run-id/run-dir escape | medium | mitigate | `validate_run_dir` (`serve.rs:186-238`) before lock | closed |
| T-28-05 | Spoofing | `driver stop` wrong pid | medium | mitigate | Held probe, pid 0/1/self refused, re-probe (`stop.rs:102-152`) | closed |
| T-28-06 | Elevation of Privilege | silent unsandboxed run | high | mitigate | refusal default (`host.rs:152-186`); per-launch `--unsandboxed`; posture recorded | closed |
| T-28-07 | Information Disclosure | setup-token at rest/output | high | mitigate | 0600 `create_new` + fsync + rename (`auth.rs:297-371`); no echo tests | closed |
| T-28-08 | Denial of Service | degraded descendants | medium | mitigate | TERM -> 12 s -> KILL, final group KILL (`run.rs:421-456`) | closed |
| T-28-09 | Information Disclosure | host creds in sandbox | high | mitigate | tmpfs/null masks (`sandbox.rs:141-171,260-268`), WR-02 pattern env scrub; live probe re-run 2026-10-20 PASS | closed |
| T-28-10 | Elevation of Privilege | agent/D-Bus via runtime dir | high | mitigate | tmpfs + socket rebind (`sandbox.rs:289-295`); live probe | closed |
| T-28-11 | Tampering | driver disables own gate | high | mitigate | `--setting-sources user`; settings.json + bin ro (`sandbox.rs:283-350`) | closed |
| T-28-12 | Spoofing | auth precedence override | medium | mitigate | strip + apiKeyHelper dropped (`config_dir.rs:58-64,185-190`) | closed |
| T-28-13 | Information Disclosure | refresh token into sandbox | high | mitigate | accessToken only (`auth.rs:186-209`) | closed |
| T-28-14 | Tampering | driver self-reports counts | high | mitigate | gate counts from payload shape / on-disk scan (`protocol.rs:155-159`, `store.rs:1009-1014`) | closed |
| T-28-15 | Denial of Service | gate fails open | high | mitigate | `onFailure: block`; trip before write/kill (`run.rs:1026-1034`) | closed |
| T-28-16 | Tampering | forged `stop-guard.count` | low | mitigate | clamp (`stop_guard.rs:108-132`) | closed |
| T-28-17 | Repudiation | kill with no record | medium | mitigate | `cap-tripped` logged before stop (`store.rs:1214-1228`) | closed |
| T-28-18 | Spoofing | stop wrong run | medium | mitigate | same-string confirm + lock-only revalidation (`driver_confirm.rs`, `stop.rs:99-107`) | closed |
| T-28-19 | Tampering | escapes in run ids/frontmatter | medium | mitigate | `shown_capped`; render escape guard probe | closed |
| T-28-20 | Tampering | `.git/config`/hooks hijack push | high | mitigate | pinned `gsdmm.pushUrl`, hooksPath `/dev/null` (`push.rs:316-376,618-627`) | closed |
| T-28-21 | Information Disclosure | secret in tip pushed | high | mitigate | full-tree scan before push, park on hit (`push.rs:562-615`) | closed |
| T-28-22 | Tampering | out-of-namespace/force push | high | mitigate | `{tip}:{dest}` no force; `check_destination` (`push.rs:499,626`) | closed |
| T-28-23 | Denial of Service | PR spam | medium | mitigate | open_pr default false; `record_and_check` before `gh` | closed |
| T-28-24 | Elevation of Privilege | hooks run in mirror | medium | mitigate | hooksPath `/dev/null` re-asserted each launch | closed |
| T-28-25 | Denial of Service | runaway spend | high | mitigate | ceiling_ok + `--max-budget-usd` (`launcher-acceptance.sh:764-846`); WR-06 bounds; ledger $10.04 / $20 | closed |
| T-28-26 | Repudiation | rubric edited post-hoc | medium | mitigate | `prereg_gate` (`launcher-acceptance.sh:149-165`) | closed |
| T-28-27 | Information Disclosure | tokens/transcripts committed | high | mitigate | `evidence_gates` (`launcher-acceptance.sh:932-945`) | closed |
| T-28-SC (x9) | Tampering | package installs (plans 28-01..09) | high | accept | no Cargo.toml/Cargo.lock change in phase 28 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-28-01 | T-28-SC | No new crates or package installs in phase 28; Cargo.lock untouched | plan-time disposition (28-01..09) | 2026-10-20 |
| AR-28-02 | T-28-25 (IN-04) | Campaign ledger read-modify-append without lock; only concurrent `run` invocations could both pass the ceiling check. Single-operator use + per-launch `--max-budget-usd` bound exposure. [inferred — audit] | gsd-secure-phase (unattended) | 2026-10-20 |
| AR-28-03 | T-28-09 (WR-02 residual) | Credential hiding remains a pattern denylist; `GIT_CONFIG_*`, settings keys `awsAuthRefresh`/`awsCredentialExport`/`otelHeadersHelper`, and a blanket mask of `~/.config/gsd-meta-manager` not added. Beyond the declared mitigation. [inferred — audit] | gsd-secure-phase (unattended) | 2026-10-20 |

*Accepted risks do not resurface in future audit runs.*

### Follow-ups (non-blocking, outside declared register scope)

- Sibling run dirs under `.planning/meta-manager/drivers/` are writable from inside the sandbox (only the current run dir is ro-bound, `sandbox.rs:274-280`).
- `auth.rs:159` doc claims the refresh token never enters process memory; `host_access_token` parses the whole credentials file (the token still never reaches the sandbox).

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-20 | 36 | 36 | 0 | gsd-security-auditor (ASVS L1, block_on high) + live isolation probe re-run post-fix |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-20
