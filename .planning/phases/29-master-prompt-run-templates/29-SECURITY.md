---
phase: "29"
slug: "master-prompt-run-templates"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-20"
---

# Phase 29 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

The register was written at plan time (`<threat_model>` blocks in 29-01..29-08 PLAN.md). No SUMMARY raised a new threat flag. At ASVS L1 with zero open threats the workflow short-circuits past the `gsd-security-auditor` dispatch. Every closure below therefore rests on orchestrator evidence: file:line references, plus a full `cargo test --no-fail-fast` run on 2026-10-20 at HEAD `1d395c2`. That run gave 3680 passed, 1 failed, 24 ignored. The one failure is the git-version witness `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, which is expected to fail on this machine and is unrelated to Phase 29. All of the suites cited here passed: driver_prompt 14, driverctl_store 47, driverctl_reconcile 7, next_cli 12, driver_new 13, launcher_e2e 29, launcher_reconcile 7 and prompt_acceptance 18 (+2 ignored paid entries).

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| prompt text -> real model (driver) | the embedded asset becomes the driver's instructions in the sandbox | instructions (integrity) |
| sandboxed driver -> driverctl store (socket) | every record crosses here; the store is the only LOG/state writer | run records (integrity) |
| human DIRECTION.md -> store / gate / launcher | control word and item list are read on every log, hook and launch | control decisions; spend trigger |
| project files -> `next` output -> driver context | `.planning/` state becomes text the model reads | untrusted artifact text |
| sandboxed driver -> host binary via shim | the driver can execute the gsd-mm binary | process execution |
| cloned repo `.planning/` tree -> `driver new` writes | a repo can plant symlinks where the tool writes | filesystem writes |
| CLI args (slug, run id, `--prompt-file`) -> paths / instructions | user input becomes directory names and prompts | paths, prompt bytes |
| run dir -> project git index | run files must never be staged | goal text, LOG |
| harness -> paid model usage; artefacts -> repo; results -> verdict | unattended spend, committed evidence, fixed pass rule | money/quota; tokens/transcripts; verdict integrity |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-29-01 | Tampering | prompt driverctl block drifting from the CLI | medium | mitigate | `tests/driver_prompt.rs:292` `every_flag_alternative_in_the_prompt_is_a_protocol_value`; driverctl lines lifted from the asset (`:331`) | closed |
| T-29-02 | DoS | prompt telling the driver to wait for a human | medium | mitigate | `tests/driver_prompt.rs:346` negation required on every AskUserQuestion/tmux/stdin mention; instruction shapes banned (`:349`) | closed |
| T-29-03 | Repudiation | asset version vs Rust constant disagree | medium | mitigate | `src/launcher/prompt.rs:25` `DRIVER_PROMPT_VERSION`; `tests/driver_prompt.rs:23` header-equals-constant; `custom_version` `prompt.rs:92` | closed |
| T-29-04 | EoP | router hint overriding done-when / stop rules | medium | mitigate | `src/launcher/assets/MASTER-PROMPT.md:159` single citation says "advisory"; `tests/driver_prompt.rs:377` | closed |
| T-29-05 | Tampering | relaunched driver dispatching over crashed work | high | mitigate | `src/driverctl/store.rs:1006-1013` rejects `dispatched` while `awaiting_recovery` blocks; second `recovered` rejected `:1015-1018`; `tests/driverctl_reconcile.rs:241,364` | closed |
| T-29-06 | EoP | driver working after human pause/stop | high | mitigate | gate deny `src/launcher/gate.rs:72-74` (test `:414`); store rejects `dispatched` `src/driverctl/store.rs:1000-1004`; `tests/driverctl_store.rs:1698,1735` | closed |
| T-29-07 | Repudiation | launch inheriting previous `prompt_version` | medium | mitigate | set-or-remove per `begin_launch` `src/driverctl/store.rs:907-908`; `tests/driverctl_store.rs:1855` (three launches) | closed |
| T-29-08 | Tampering | `prompt=` injecting `\|`/newline into LOG header | medium | mitigate | `header_safe` `src/driverctl/store.rs:405,907`; `tests/driverctl_store.rs:1901` hostile value stays one field | closed |
| T-29-09 | DoS | removed DIRECTION item wedging dispatch | medium | mitigate | filter on current items `src/driverctl/store.rs:1006-1008`; `tests/driverctl_reconcile.rs:349` | closed |
| T-29-10 | EoP | prompt injection via `next` hint | medium | mitigate | output built from typed `Decision`/`as_str()` only `src/next_cli.rs:119-139`; `tests/next_cli.rs:341` `artifact_prose_never_reaches_the_output` | closed |
| T-29-11 | Tampering | `next` creating files in project / ro home | medium | mitigate | dispatched before `init_tracing` `src/main.rs:109-116`; `tests/next_cli.rs:300` snapshot of HOME/XDG/`.planning` | closed |
| T-29-12 | EoP | shim used to run other subcommands in sandbox | low | accept | shim refuses non-`next` `src/launcher/config_dir.rs:271-276`; not a boundary (ro `/`) — see AR-01 | closed |
| T-29-13 | Tampering | `--target-phase` path traversal | low | mitigate | `is_plain_path_component` before any read `src/next_cli.rs:149-150`; `tests/next_cli.rs:249` | closed |
| T-29-14 | Tampering | planted symlink chain redirecting `driver new` writes | high | mitigate | `symlink_metadata` per component + canonical containment `src/driver_templates/mod.rs:161-237`; `tests/driver_new.rs:320` | closed |
| T-29-15 | Tampering | slug path tricks | medium | mitigate | slug regex + `valid_run_id` before any create `src/driver_templates/mod.rs:45,141`; `tests/driver_new.rs:294` | closed |
| T-29-16 | Tampering/Repudiation | clobbering an existing run | high | mitigate | `create_dir` EEXIST `src/driver_templates/mod.rs:240-243`, `create_new` `:181`; `tests/driver_new.rs:250` bytes+mtimes unchanged | closed |
| T-29-17 | Info Disclosure | run files committed to project history | medium | mitigate | `*` `.gitignore` written first `src/driver_templates/mod.rs:256`; `tests/driver_new.rs:155-207` `git add -A` + `ls-files` proof | closed |
| T-29-18 | Tampering | `driver check` mutating DIRECTION.md | low | mitigate | `tests/driver_new.rs:452,482` bytes and mtime asserted unchanged | closed |
| T-29-19 | DoS | tests killing foreign processes / leaking fakes | low | mitigate | own pid files re-checked via `/proc/<pid>/environ` `tests/launcher_reconcile.rs:162-182`; fake systemd/bwrap shims | closed |
| T-29-20 | Repudiation | custom prompt recorded as shipped version | medium | mitigate | source decides version `src/launcher/run.rs:258-276`; `tests/launcher_e2e.rs:100,122` (byte-equal still `custom:`) | closed |
| T-29-21 | DoS (financial) | launching on malformed/paused DIRECTION | medium | mitigate | `preflight_direction` `src/launcher/run.rs:229-249` before any write in both paths `:552,1220`; `tests/launcher_e2e.rs:232,299` | closed |
| T-29-22 | Tampering | prompt file swapped between check and daemon read | low | mitigate | parent canonicalizes + is_file `src/launcher/run.rs:1227-1240`; version hashed from the bytes actually read `:265-275` | closed |
| T-29-23 | Repudiation | rubric edited after results | medium | mitigate | rubric commit `acceb35` alone, no later edits; prereg gate `scripts/prompt-acceptance.sh:121-130` (exit 9); `tests/prompt_acceptance.rs:237` | closed |
| T-29-24 | Info Disclosure | tokens/transcripts committed as evidence | high | mitigate | `evidence_gates` + `envelope scan` `scripts/prompt-acceptance.sh:1087-1092`; `tests/prompt_acceptance.rs:383`; 0 tracked `*.jsonl`/`claude-config` under `acceptance/` | closed |
| T-29-25 | DoS (financial) | rehearsal/CI/`--ignored` spending money | high | mitigate | consent 10 / prereg 9 / ceiling 8 gates `scripts/prompt-acceptance.sh:22-26,97-100`; `tests/prompt_acceptance.rs:196,237,276,337`; `#[ignore]` entries call the gated harness only `:830-836` | closed |
| T-29-26 | DoS (financial) | runaway or unconsented spend | high | mitigate | `gate="blocking-human"` 29-08-PLAN.md:94; consent recorded `e7006ca` before evidence commits; per-launch `--max-budget-usd` `scripts/prompt-acceptance.sh:813`; ledger test `tests/prompt_acceptance.rs:410` | closed |
| T-29-27 | Info Disclosure | tokens/claude-config in committed evidence | high | mitigate | same gates as T-29-24; committed-tree scan `tests/prompt_acceptance.rs:383` passes at HEAD | closed |
| T-29-28 | Repudiation | verdict edited after the fact | medium | mitigate | `check` re-derives verdict from evidence `scripts/prompt-acceptance.sh:7,368`; rubric `acceb35` precedes every evidence commit | closed |
| T-29-SC | Tampering | npm/pip/cargo installs | high | accept | `Cargo.toml` unchanged across Phase 29 (empty diff from first 29-0x commit to HEAD) — see AR-02 | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-01 | T-29-12 | The `next`-only shim is a convenience, not a boundary: `/` is ro-bound in the sandbox, so the binary is callable directly. The real boundary is the sandbox itself: a ro run dir, no host secrets, and pushes only from the host (Phase 28 D-24). | plan 29-03 threat model | 2026-10-20 |
| AR-02 | T-29-SC | No package was added in this phase; `sha2`, `chrono` and `regex` were already dependencies. | plan threat models 29-01..08 | 2026-10-20 |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-20 | 29 | 29 | 0 | gsd-secure-phase orchestrator (L1 short-circuit, auditor not spawned) |

Observation (non-blocking) [inferred — audit]: when DIRECTION.md has no items, `src/driverctl/store.rs:1007` keeps every awaiting id blocking (fail-closed). That is safe, and the launch preflight lint already refuses such a DIRECTION.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-20
