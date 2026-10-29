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
> Register authored at plan time (28-01..28-09 `<threat_model>` blocks). Audited by gsd-security-auditor at HEAD a5b2801, after the code-review fixes (0951aea..b678e78). Evidence refreshed at HEAD a69de7a (2026-10-28) after Phase 28.1, Phase 31 deletions and quick batch 261028-n0u (macOS TUI launch/steering, WR-05); all mitigations still present, citations updated. [inferred — cleanup 2026-10-28] Re-verified at HEAD 66e3d94 (2026-10-29) after quick batch 261028-t3c and quick task 261028-v2z (config_dir.rs WR-08/WR-13 fail-closed host merge; lock/state open hardening in run.rs, stop.rs, serve.rs): cited code moved only by line shifts (content-hash checked), mitigations intact, T-28-15 strengthened; citations refreshed. Auditor not re-spawned (ASVS L1 short-circuit). [inferred — cleanup 2026-10-28]

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
| T-28-01 | Spoofing | second launcher as second writer | high | mitigate | run lock (`run.rs:675-688`) before `Session::start` (`run.rs:710`), rc 3 (`mod.rs:173`); `launcher_e2e.rs:690,816,1134` | closed |
| T-28-02 | Tampering | driver writes LOG/state/locks | high | mitigate | ro-bind run_dir + `/dev/null` over locks (`sandbox.rs:587-592`), siblings hidden (`:582-586`); `launcher_e2e.rs:446-457`, `launcher_sandbox.rs:55` | closed |
| T-28-03 | Information Disclosure | OAuth token in argv/files | high | mitigate | degraded: env only (`run.rs:1618-1626`); bwrap: dummy token in sandbox (`sandbox.rs:672`), real token held by launcher egress (`run.rs:1713-1742`); Debug redacts (`auth.rs:74-81`); `launcher_e2e.rs:401` | closed |
| T-28-04 | Tampering | run-id/run-dir escape | medium | mitigate | `validate_run_dir` (`serve.rs:212-264`) before lock (`run.rs:631`, `serve.rs:915`); tests `serve.rs:1071-1126` | closed |
| T-28-05 | Spoofing | `driver stop` wrong pid | medium | mitigate | Held probe, pid 0/1/self refused, re-probe (`stop.rs:103-162`); `launcher_stop.rs:27` | closed |
| T-28-06 | Elevation of Privilege | silent unsandboxed run | high | mitigate | single `decide` (`host.rs:286-320`), off-Linux probes report missing (`host.rs:72,234-239`); TUI forces sandboxed (`driver_launch.rs:571,754`), opt-in only via Red default-No `UnsandboxedConfirmScreen` (`:1180`); posture recorded (`run.rs:922,929`) | closed |
| T-28-07 | Information Disclosure | setup-token at rest/output | high | mitigate | 0600 `create_new` + fsync + rename (`auth.rs:311-390`); `launcher_auth.rs:114` no echo | closed |
| T-28-08 | Denial of Service | degraded descendants | medium | mitigate | TERM -> 12 s -> KILL group (`run.rs:587-597`, `driver/kill.rs:37`); `launcher_degraded.rs:330,374` | closed |
| T-28-09 | Information Disclosure | host creds in sandbox | high | mitigate | `$HOME` tmpfs + allowlist (`sandbox.rs:549-552`), null masks (`:242-254,493-497`), env scrub (`:164-185,765-777`); `launcher_sandbox.rs:199,276,552`; live probe last run 2026-10-20 (not re-run) | closed |
| T-28-10 | Elevation of Privilege | agent/D-Bus via runtime dir | high | mitigate | `/run` + runtime-dir tmpfs, only driverctl/egress sockets rebound (`sandbox.rs:544-545,605-614`), unshare (`:617-624`); `launcher_sandbox.rs:409,428,703` | closed |
| T-28-11 | Tampering | driver disables own gate | high | mitigate | `--setting-sources user` (`sandbox.rs:720`); settings.json/bin/managed ro (`:594-604`); `launcher_sandbox.rs:670` | closed |
| T-28-12 | Spoofing | auth precedence override | medium | mitigate | `apiKeyHelper` dropped (`config_dir.rs:128-134,600-605`); `ANTHROPIC_AUTH_TOKEN` stripped (`sandbox.rs:105`) | closed |
| T-28-13 | Information Disclosure | refresh token into sandbox | high | mitigate | accessToken + expiry only (`auth.rs:179-225`); test `auth.rs:571` | closed |
| T-28-14 | Tampering | driver self-reports counts | high | mitigate | gate counts from payload shape (`protocol.rs:155-159`, `store.rs:1231-1263`) / on-disk scan (`store.rs:1286-1301`) | closed |
| T-28-15 | Denial of Service | gate fails open | high | mitigate | `onFailure: block` (`config_dir.rs:201-204`); host `hooks`/`permissions` merge fail-closed, wrong-shape values dropped so they cannot displace the gate hooks or deny list (`config_dir.rs:359-424,680-713`, WR-08/WR-13); trip marker before write/kill (`run.rs:1321-1346,1796-1800`, `gate.rs:132-139`); `launcher_caps.rs:385` | closed |
| T-28-16 | Tampering | forged `stop-guard.count` | low | mitigate | strict read + clamp (`stop_guard.rs:108-132,236`) | closed |
| T-28-17 | Repudiation | kill with no record | medium | mitigate | `cap-tripped` committed before stop (`store.rs:1459-1474`); test `store.rs:1979` | closed |
| T-28-18 | Spoofing | stop wrong run | medium | mitigate | same-string confirm (`driver_confirm.rs:70-75,103-135,170`), revalidation (`stop.rs:94-108`); test `driver_confirm.rs:810` | closed |
| T-28-19 | Tampering | escapes in run ids/frontmatter | medium | mitigate | `shown_capped` (`driver.rs:196`, 40 sites in `driver_run.rs`); escape guard probes launch/confirm screens (`render_escape_guard.rs:364-389`) | closed |
| T-28-20 | Tampering | `.git/config`/hooks hijack push | high | mitigate | pinned `gsdmm.pushUrl` (`push.rs:76,382-440,561-568`), `GIT_HARDENING` hooksPath `/dev/null` (`:281-293`) | closed |
| T-28-21 | Information Disclosure | secret in tip pushed | high | mitigate | full-tree scan before push, park on hit (`push.rs:715-740`); `launcher_push.rs:373` | closed |
| T-28-22 | Tampering | out-of-namespace/force push | high | mitigate | `check_destination` (`push.rs:203-212,583`); `{tip}:{dest}` no force (`:741-753`) | closed |
| T-28-23 | Denial of Service | PR spam | medium | mitigate | open_pr default false (`config.rs:573-579`); `record_and_check` before `gh` (`push.rs:839`) | closed |
| T-28-24 | Elevation of Privilege | hooks run in mirror | medium | mitigate | `prepare_mirror` each launch (`run.rs:762,853`) re-asserts hooksPath (`push.rs:428-440`) | closed |
| T-28-25 | Denial of Service | runaway spend | high | mitigate | `ceiling_ok` + `--max-budget-usd` (`launcher-acceptance.sh:118-130,766-779,849-853`); ledger $10.04 / $20; selftest PASS | closed |
| T-28-26 | Repudiation | rubric edited post-hoc | medium | mitigate | `prereg_gate` (`launcher-acceptance.sh:155-172,845`) | closed |
| T-28-27 | Information Disclosure | tokens/transcripts committed | high | mitigate | `evidence_gates` (`launcher-acceptance.sh:924,938-949`) | closed |
| T-28-SC (x9) | Tampering | package installs (plans 28-01..09) | high | accept | no Cargo.toml/Cargo.lock change in phase 28 window (`git diff bda1ad9^ a5b2801` empty); later changes belong to 28.1/31/WR-05; none since a69de7a | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-28-01 | T-28-SC | No new crates or package installs in phase 28; Cargo.lock untouched | plan-time disposition (28-01..09) | 2026-10-20 |
| AR-28-02 | T-28-25 (IN-04) | Campaign ledger read-modify-append without lock; only concurrent `run` invocations could both pass the ceiling check. Single-operator use + per-launch `--max-budget-usd` bound exposure. [inferred — audit] | gsd-secure-phase (unattended); **operator accepted 2026-10-20** | 2026-10-20 |
| AR-28-03 | T-28-09 (WR-02 residual) | Credential hiding remains a pattern denylist; `GIT_CONFIG_*`, settings keys `awsAuthRefresh`/`awsCredentialExport`/`otelHeadersHelper`, and a blanket mask of `~/.config/gsd-meta-manager` not added. Beyond the declared mitigation. [inferred — audit] | gsd-secure-phase (unattended); **operator accepted 2026-10-20** | 2026-10-20 |

*Accepted risks do not resurface in future audit runs.*

**Operator acceptance (2026-10-20):** the operator reviewed and accepted AR-28-02 (campaign ledger has no lock) and AR-28-03 (remaining credential locations still visible in the sandbox). The `[inferred — audit]` dispositions above are now operator-confirmed. Phase 28's two hands-on UAT checks (`driver auth setup` browser login; macOS `driver launch` refuse then `--unsandboxed`) remain pending with the operator.

### Follow-ups (non-blocking, outside declared register scope)

- Sibling run dirs under `.planning/meta-manager/drivers/` are writable from inside the sandbox (only the current run dir is ro-bound, `sandbox.rs:274-280`). **Resolved by quick task 261020-f80 (018cc17):** drivers dir hidden by an empty tmpfs; own run dir and lock masks re-bound on top; live-probed.
- `auth.rs:159` doc claims the refresh token never enters process memory; `host_access_token` parses the whole credentials file (the token still never reaches the sandbox). **Resolved by quick task 261020-f80 (this commit):** doc comments in `auth.rs` corrected.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-20 | 36 | 36 | 0 | gsd-security-auditor (ASVS L1, block_on high) + live isolation probe re-run post-fix |
| 2026-10-28 | 36 | 36 | 0 | gsd-security-auditor re-verification at HEAD a69de7a (ASVS L1, block_on high); 8 launcher test binaries + 192 lib tests + acceptance selftest PASS; live isolation probe not re-run (opt-in, touches real $HOME) [inferred — cleanup 2026-10-28] |
| 2026-10-29 | 36 | 36 | 0 | gsd-secure-phase L1 re-verification at HEAD 66e3d94 (ASVS L1 short-circuit, block_on high); citations refreshed after 261028-t3c / 261028-v2z; lib (2573) + launcher_config_dir, launcher_stop, launcher_sandbox, launcher_e2e, driverctl_serve PASS [inferred — cleanup 2026-10-28] |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-20

## Security Audit 2026-10-28

| Metric | Count |
|---|---|
| Threats found | 36 |
| Closed | 36 |
| Open | 0 |
