---
phase: "26"
slug: "driver-validation-spike-003"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
threats_open_nonblocking: 0
asvs_level: 1
block_on: high
created: "2026-10-16"
---

# Phase 26 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.
> Register authored at plan time (26-01/02/03-PLAN.md `<threat_model>`); SUMMARY Threat Flags: none in all three plans.
> Paths below are relative to `.planning/spikes/003-divergent-gap-loop-validation/` unless stated.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| host → sandboxed driver and workers | untrusted Claude processes under the unmodified spike-002 bwrap/systemd profile; only the scratch project is writable | prompts, DIRECTION, caps (low) |
| sandbox outputs → repo | LOG.md, state.json, DIRECTION.md, verification snapshots, cost files copied into committed `experiment/results/` | derived run evidence; risk of transcript/secret leakage (high) |
| host environment → launcher | the executor's Claude session exports CLAUDE_* variables the driven run must not inherit | session ids, messaging tokens (high) |
| host OAuth token → sandbox | access token in the environment only; host refresh revokes it mid-run | OAuth access token (high) |
| transcripts → tool output | dup_check / stage_cost / acceptance read transcripts; their output is committed | transcript text (medium) |
| results files → verdict | verdict computed from run-produced files; hand edits would forge evidence | evidence integrity (medium) |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-26-01 | Information disclosure | run.sh results copy | high | mitigate | explicit derived-file copy list `experiment/run.sh:174-184`; secret gate exits 6 `run.sh:186-192`; no `*.jsonl`/`claude-config` in results (find + git ls-files) | closed |
| T-26-02 | Tampering / EoP | scratch location | high | mitigate | git-work-tree guard exits 2 `run.sh:68-73`; no-remote assert `setup-scratch.sh:183`; all 5 recorded SCRATCH_ROOTs under `~/.local/state/...` (not a work tree) | closed |
| T-26-03 | Spoofing / EoP | env inheritance | medium | mitigate | CLAUDE*/ANTHROPIC* scrub list `run.sh:88-92`, applied `run.sh:154`; names only in `ENV_SCRUBBED`, no values in results | closed |
| T-26-04 | Denial of service | orphaned watcher / scope | medium | mitigate | EXIT trap `run.sh:95-101` kills watcher + stand-in, stops `gsdmm-$RUN_ID-*.scope`; spike-002 `launcher.py:288` keeps `--die-with-parent` | closed |
| T-26-05 | DoS (quota) | Haiku launches | low | mitigate | `run.sh:33-40` haiku, budget 1, 5 min, smokes cap 0; confirmed in l7/smoke1/smoke2 `run-env.txt` | closed |
| T-26-06 | Tampering | drift.py writes | medium | mitigate | temp + `os.replace` `tools/drift.py:184-188`; refuses without `.planning` `:194` and symlinked `feed/` `:197`; `GIT_OPTIONAL_LOCKS=0` `:216`; selftest exercises refusals | closed |
| T-26-07 | Tampering | spike-002 tools | high | mitigate | `git diff --exit-code a16e338 -- .planning/spikes/002-launcher-caps-and-tool-frontmatter` clean; recorded launcher/driverctl sha256 in all 5 `run-env.txt` equal current files | closed |
| T-26-08 | Information disclosure | dup_check, stage_cost, acceptance | high | mitigate | metadata/usage/tool_use-name reads only (`dup_check.py:65-121`, `stage_cost.py:114-134`); planted-marker assertions `dup_check.py:380`, `stage_cost.py:496`, `acceptance.py:1102`; all selftests pass | closed |
| T-26-09 | Repudiation | acceptance A6 | medium | mitigate | A6 grammar/`base=`/`dispatch=k/n`/`generated_by: driverctl`/dispatch audit `acceptance.py:214-245`; read-only | closed |
| T-26-10 | Repudiation (HARKing) | README rubric | medium | mitigate | rubric committed in 4ba6195 before prereg and runs; `rubric-check` prints `RUBRIC-UNCHANGED` vs `PREREG_SHA=4ba6195` | closed |
| T-26-11 | Tampering (vacuous green) | acceptance.py aggregate | medium | mitigate | dup_check/stage_cost exit 2 on empty/missing input (holds). Fixed in Phase 26.1 by f909f93 (26.1-02): `acceptance.py` RESULT is three-way (PASS needs zero FAIL, zero UNDETERMINED and at least one PASS; FAIL exit 1; INCONCLUSIVE exit 2), so an empty or nonexistent results dir prints `RESULT INCONCLUSIVE` and exits 2; the selftest asserts the RESULT line and rc for l7, l8, run1 and run2 over both inputs and pins the pure aggregate | closed |
| T-26-12 | Information disclosure | results, README, MANIFEST commits | high | mitigate | as T-26-01; wider secret-shape scan (sk-, oat01, ghp_, AKIA, Bearer, private-key headers) over spike dir and MANIFEST: no hits | closed |
| T-26-13 | DoS (quota) | Opus runs | medium | mitigate | budget 15 / 90 min `run.sh:43,46`; `RuntimeMaxSec` `launcher.py:281`; 2 Run-1 attempts, Run 2 skipped, `RELAUNCHES=0` everywhere; measured $7.81 of $60 ceiling | closed |
| T-26-14 | Repudiation (forged evidence) | results dirs, verdict | high | mitigate | `cmp` LOG.md/state.json/DIRECTION.md vs scratch RUN_DIR: identical for l7, smoke1, smoke2, run1, run1b; README cites committed files; rubric unchanged | closed |
| T-26-15 | Spoofing / DoS (auth) | host token | medium | mitigate | token-life check exit 5 `run.sh:76-86`; relaunch bounded `RELAUNCHES -lt 2` `:160-167`; credentials read for `expiresAt` only, never copied/printed | closed |
| T-26-16 | Tampering (host repo) | driven agents | high | mitigate | unmodified bwrap `--ro-bind / /` + project bind `launcher.py:279-288`; scratch outside work tree; repo `git status` shows no spike changes | closed |
| T-26-SC | Tampering | package installs | low | accept | stdlib + in-tree spike-002 only; no install commands in scripts | closed (accepted at plan time) |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

### Informational (auditor notes, not counted)

- T-26-06 residual: `drift.py:_atomic_write` (`:186`) opens `.{name}.tmp` inside agent-writable `feed/` without `O_NOFOLLOW`/`O_EXCL`; a pre-planted symlink at the temp name would be followed by the host watcher. Declared mitigation present, so closed at L1.
- T-26-13: the $60 campaign ceiling was a README procedure (`README.md:150-152`), not enforced in `run.sh`; per-launch caps are enforced. Since Phase 26.1 the ceiling is enforced mechanically by `tools/campaign_budget.py` in `run.sh` (exit 8; 26.1-03 commit 2cc903c).
- T-26-01/12: "no `*.jsonl` in results" is guaranteed by the explicit copy list, not a runtime find check in `run.sh`.

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-26-01 | T-26-SC | No packages installed; stdlib and system binaries only (26-RESEARCH § Package Legitimacy Audit) | plan-time disposition (26-01/02/03-PLAN.md) | 2026-10-16 |

T-26-11 was fixed in Phase 26.1 (f909f93, 26.1-02), not accepted.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-16 | 17 | 16 | 1 (non-blocking, T-26-11) | /gsd-secure-phase orchestrator (L1 grep) + gsd-security-auditor |
| 2026-10-18 | 17 | 17 | 0 | 26.1-04 (T-26-11 closed by f909f93) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-16 (automated; threats_open: 0). Pending operator decision on non-blocking T-26-11.
