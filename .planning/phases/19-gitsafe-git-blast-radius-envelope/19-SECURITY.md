---
phase: 19
slug: gitsafe-git-blast-radius-envelope
status: blocked
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (high)
threats_open: 2
asvs_level: 1
block_on: high
created: 2026-08-29
updated: 2026-08-29
register_authored_at_plan_time: true
audited_against: HEAD (b8605ef) — the second audit judges the tree after plans
  19-11 and 19-12. The first audit (2026-08-29) judged 0ec1fcb.
---

# Phase 19 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Phase 19 built a **git blast radius envelope**: layered controls so a driven agent
cannot force-push, bypass git hooks, reach ambient credentials, or exceed a
per-project pull-request cap. The register below was authored at plan time across
all ten plans (`register_authored_at_plan_time: true`); this audit verifies that
each declared mitigation exists in the implementation and that the test named as
its evidence actually asserts the property.

**Verdict (audit 2, 2026-08-29): OPEN_THREATS.** Two high-severity threats are
open and blocking — `T-19-60`, **narrowed but not closed** by plans 19-11/19-12,
and `T-19-81`, found during this audit. Ninety-seven threats total, seventy-eight
closed, nineteen open (two at or above the `high` block threshold).

> **What changed since audit 1.** Plans 19-11 and 19-12 replaced `words[0]`
> classification with `policy::resolve_program` and closed every one of the six
> originally measured bypass lines — verified independently against the built
> binary, not accepted from the summaries. They also fixed a second live
> vulnerability found in passing (`GIT_SSH_COMMAND` missing from
> `ENVELOPE_ENV_KEYS`). But the declared property — *"the verdict is invariant
> under any wrapper chain"* — is false: `resolve_program` selects the first token
> whose basename is governed **without establishing that it is in a command
> position**, so a wrapper's own option operand spelled `git` captures the index
> and the real command is classified one word off. `env -u git git push --force
> origin main` is permitted. See the T-19-60 section below.

---

## The Layer Model

| Layer | Control | Carrier |
|-------|---------|---------|
| 1 | `--disallowedTools` / `permissions.deny` | agent CLI argv **and** the generated settings file |
| 2 | `PreToolUse` guard — classifies the shell command before it runs | the generated settings file (one carrier for the PR cap) |
| 3 | `pre-push` / `pre-commit` git hooks | `core.hooksPath` injected via `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` |
| — | credential isolation | `GIT_ASKPASS` responder, helper-free generated git config, ambient sockets removed |

The declared ceiling (D-09) is that an agent which unsets `GIT_CONFIG_COUNT` is
past layer 3. `T-19-60` below records that layer 2 does not, in fact, stand
underneath it.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| driven agent → git | The agent issues git commands; the destination refspec is attacker-influenced input crossing into a network write | refspecs, remote URLs |
| driven agent → `PreToolUse` guard | The agent supplies an arbitrary shell command string the guard must classify before it runs | untrusted command lines |
| driven agent → filesystem | The agent can write any file the process can, including inside the driven repository | arbitrary file content, credentials |
| git → hook stub → this binary | git execs the stub, which execs this binary with ref lines on stdin | ref lines, stub path |
| registry alias → filesystem path | A user-typed alias is interpolated into an application data path | alias string |
| user's ambient identity → driven child | The parent holds keychain access, an ssh-agent socket and a global git config the child must not reach | credentials |
| configured credential → git | A token crosses from a secret store into a child process an untrusted agent controls | bearer token |
| envelope → settings file → agent CLI | A generated JSON file is the only carrier of the guard registration; the consumer silently ignores it when invalid | policy |
| worktree files → scanner → report | Untrusted file bytes cross into a report that is printed and journalled | secret-shaped bytes |
| remote host → protection probe | An external service's response, or its absence, decides what the tool reports about protection | protection verdict |
| refusal → evidence | What a later reader believes happened depends on what was written to disk | park events |
| envelope → operator | The honesty statement is the only place a human learns what this envelope does **not** enforce | prose claims |

---

## Threat Register

Ninety-seven threats, in four groups. Bookkeeping is stated explicitly because
audit 1's header ("sixty-nine … sixty-eight closed") folded the audit-found rows
into the plan-time count inconsistently:

| Group | Count | Closed | Open |
|---|---|---|---|
| Plan-time register, plans 19-01 … 19-10 + supply chain | 69 | 69 | 0 |
| Plan-time register, plan 19-11 (`T-19-60a/b`, `T-19-74`, `T-19-75`) | 4 | 4 | 0 |
| Plan-time register, plan 19-12 (`T-19-76` … `T-19-80`) | 5 | 5 | 0 |
| Found by audit 1 (`T-19-60`, `T-19-61` … `T-19-73`) | 14 | 0 | 14 (1 at `high`) |
| Found by audit 2 (`T-19-81` … `T-19-85`) | 5 | 0 | 5 (1 at `high`) |
| **Total** | **97** | **78** | **19 (2 at `high`)** |

### Plan 19-01 — hook stubs, provenance, alias path safety

| Threat ID | Category | Component | Severity | Disposition | Mitigation / Evidence | Status |
|---|---|---|---|---|---|---|
| T-19-01 | Elevation of Privilege | generated hook stub | high | mitigate | `hooks.rs:92` absolute `current_exe()`; `assert_provenance_in` `hooks.rs:242-294`; tests `the_generated_stub_is_three_lines_and_execs_the_absolute_binary_path`, `a_copy_of_the_hook_outside_the_envelope_certifies_nothing`, `a_relocated_copy_of_the_stub_refuses_instead_of_acting` | closed |
| T-19-02 | Tampering | `core.hooksPath` delivery | high | mitigate | `cred.rs:197-250`; `neither_push_writes_into_the_driven_repository_config_or_hooks` asserts `.git/config` byte-identical across a refused and an allowed push | closed |
| T-19-03 | Tampering | alias path join | high | mitigate | `mod.rs:221-226` validates **before** the join → `is_plain_path_component`; hostile-alias corpus in `mod.rs:446`, `envelope_tracer.rs:203/221`, `hooks.rs:1638` | closed |
| T-19-04 | Spoofing | `pre-push` stdin ref lines | medium | mitigate | `hooks.rs:679-694` — field count ≠ 4 refuses; `an_unreadable_ref_line_is_refused_rather_than_skipped` | closed |
| T-19-05 | Repudiation | refusal evidence | medium | mitigate | `a_driven_push_to_main_is_refused_and_the_remote_ref_never_appears` asserts exit code **and** stderr **and** the bare remote's refs (via `git for-each-ref`, not `ls-remote` — same class of evidence) | closed |
| T-19-06 | Information Disclosure | envelope artifact location | low | **accept** | `envelope_root` → `dirs::data_local_dir`; `envelope_tracer.rs:163-167` asserts the hooks dir is outside the worktree | closed (accepted) |
| T-19-07 | Denial of Service | provenance canonicalization cost | low | **accept** | Two `canonicalize` calls per hook run, off the per-tool-call path | closed (accepted) |

### Plan 19-02 — the git argv classifier

| Threat ID | Category | Component | Severity | Disposition | Mitigation / Evidence | Status |
|---|---|---|---|---|---|---|
| T-19-08 | Elevation of Privilege | `classify_git` argv matching | high | mitigate | `policy.rs:546`, `:560-585`, `scan_leading` `:307-339`, case-insensitive keys `:419-421`; `every_force_push_spelling_is_refused` (incl. `-fu`, `--force-with-lease=…`), `a_push_option_value_is_never_mistaken_for_a_refspec` | closed |
| T-19-09 | Tampering | `git -c core.hooksPath=…` | high | mitigate | `policy.rs:321-333` + `classify_config`; 6 spellings and 12 scope rows refused, with a paired allow control proving reads are still permitted | closed |
| T-19-10 | Elevation of Privilege | user-configured `branch_namespace` | high | mitigate | `validate_namespace` `policy.rs:83-99`; invalid value degrades to the tighter default and warns — `an_invalid_configured_namespace_falls_back_to_the_default_rather_than_widening` | closed |
| T-19-11 | Information Disclosure | credential in `config.json` | high | mitigate | `config.rs:236-255` — exactly `Env { var }` and `Command { argv }`, `#[serde(tag = "source")]`; by construction. See `T-19-73` (no drift-pin test) | closed |
| T-19-12 | Tampering | implicit push destination | medium | mitigate | `policy.rs:450-459` — empty `resolved_push_dests` refuses; `a_push_whose_destination_cannot_be_resolved_is_refused_never_allowed` | closed |
| T-19-13 | Repudiation | park reason drift | medium | mitigate | `ParkReason::as_str` `policy.rs:178-188`; `every_park_reason_has_a_distinct_stable_snake_case_identifier` pins all seven in order | closed |
| T-19-14 | Denial of Service | over-broad denylist blocking legitimate GSD skills | medium | **accept** | Rationale and revisit condition at `policy.rs:226-241`; control `an_unanticipated_plumbing_verb_is_allowed_because_this_is_a_denylist` | closed (accepted) |

### Plan 19-03 — the worktree credential scan

| Threat ID | Category | Component | Severity | Disposition | Mitigation / Evidence | Status |
|---|---|---|---|---|---|---|
| T-19-15 | Information Disclosure | `ScanReport::render` | high | mitigate | `Finding` has no matched-text field (`scan.rs:73-80`); `the_rendered_report_never_contains_the_planted_secret` asserts on rendered bytes with two positive controls; process-level twin in `envelope_hook_refusals.rs:208` | closed |
| T-19-16 | Information Disclosure | credentials on ignored paths | high | mitigate | `scan_worktree` uses `std::fs::read_dir` only — no ignore machinery; `a_credential_on_a_path_the_ignore_rules_cover_still_blocks_the_push` first asserts `git check-ignore -q` succeeds, so the fixture cannot prove nothing | closed |
| T-19-17 | Tampering | nested repo / runs dir staged | high | mitigate | One predicate `forbidden_repo_path`, two enforcement points; `a_commit_made_with_verification_suppressed_is_refused_by_the_push_hook` commits with `--no-verify` then asserts the push is refused | closed |
| T-19-18 | Spoofing | absent external scanner read as clean | high | mitigate | `scan.rs:392-406` runs built-in rules first and unconditionally; `is_clean()` `:188-190` never reads `external`; `run_gitleaks` returns a value and cannot mutate findings; `Failed`/`Blocked` **add** a finding. Verified empirically on this machine (gitleaks absent) — dirty tree still exits 1. See `T-19-72` (unpinned) | closed |
| T-19-19 | Denial of Service | unbounded worktree walk | medium | mitigate | `ScanLimits` with budget-first ordering; `an_exhausted_total_budget_marks_the_rest_skipped_rather_than_ending_the_scan` names all remaining files | closed |
| T-19-20 | Tampering | `.git/info/exclude` accumulation | medium | mitigate | Marker-delimited, filtered-and-rewritten, atomic persist; `assert_eq!(once, twice)` byte comparison + `git check-ignore` proving the rule is live | closed |
| T-19-21 | Elevation of Privilege | symlink out of the worktree | medium | mitigate | `DirEntry::file_type()` does not follow; `a_symlink_is_recorded_and_not_followed` plants the secret outside the root | closed |
| T-19-22 | Repudiation | a scan that declined files reporting clean | medium | mitigate | One `ScanReport`; skip list printed on both branches, asserted on the allowing path too | closed |

### Plan 19-04 — credential isolation

| Threat ID | Category | Component | Severity | Disposition | Mitigation / Evidence | Status |
|---|---|---|---|---|---|---|
| T-19-23 | Information Disclosure | ambient credential reachability | high | mitigate | `cred.rs:402-403/412-419/443-446`; the helper test **plants** `credential.helper = store` in a fake HOME and proves the control resolves it first — not a vacuous absence check | closed |
| T-19-24 | Information Disclosure | token placement | high | mitigate | `cred.rs:711-732` — stdout is the only writer; `nothing_the_envelope_wrote_holds_the_credential` walks the envelope dir. See `T-19-63` for the `Env` source correction | closed |
| T-19-25 | Spoofing | agent adds a second remote to harvest the token | high | mitigate | `cred.rs:673-678` host check precedes credential resolution; host is **baked** at `:395`, not re-derived | closed |
| T-19-26 | Elevation of Privilege | silent ambient fallback | high | mitigate | `resolve_credential` `cred.rs:617-648` has no ambient branch to remove; fail-closed names `credential_unavailable` | closed |
| T-19-27 | Denial of Service | auth prompt into null stdio | high | mitigate | `GIT_TERMINAL_PROMPT=0` + `BatchMode=yes`; bounded 30 s test asserts termination, ran green rather than skipped | closed |
| T-19-28 | Information Disclosure | credential in log output | medium | mitigate | `cred.rs:728` uses the shipped `journal::redact`, not a fork | closed |
| T-19-29 | Tampering | agent rewrites the generated git config | medium | **accept** | Config regenerated per run start (`driver/run.rs:134`); the honest answer is D-27's server-side recommendation | closed (accepted) |

### Plan 19-05 — the `PreToolUse` guard and the PR-cap ledger

| Threat ID | Category | Component | Severity | Disposition | Mitigation / Evidence | Status |
|---|---|---|---|---|---|---|
| T-19-30 | Elevation of Privilege | settings file silently ignored | high | mitigate | Typed tree `hooks.rs:1200-1245`; persist → read back **from the path** → refuse on mismatch (`:1334-1344`, `:1382-1413`). See `T-19-64` | closed |
| T-19-31 | Elevation of Privilege | shell command the splitter cannot recover | high | mitigate | `split_command`/`split_segments` return `None` and the guard denies (`policy.rs:975-982`, `hooks.rs:873-881`); `a_command_whose_words_cannot_be_recovered_is_denied`, `a_verb_assembled_by_expansion_and_an_eval_are_both_denied`. The literal mitigation is present and non-vacuously tested. **The adjacent uncovered shape is `T-19-60`** | closed |
| T-19-32 | Tampering | agent resets its own cap | high | mitigate | Ledger under `<envelope>/<alias>/`, asserted by a path test; `.claude/**` denied on both carriers | closed |
| T-19-33 | Denial of Service | guard latency on the critical path | high | mitigate | `the_guard_makes_no_network_call_on_any_path_it_takes` (with an extraction-integrity assert); `GUARD_TIMEOUT_SECS = 5` asserted; ~6 ms measured on a debug build | closed |
| T-19-34 | Tampering | driven repo `.claude/settings.json` | high | mitigate | `policy.rs:842-843`; both carriers asserted by `every_pattern_the_settings_file_denies_is_also_carried_on_argv` | closed |
| T-19-35 | Repudiation | a PR opened by a path the guard did not see | medium | **accept** | `ledger.rs:166-181` write-before-permit biases to over-count; `CapVerdict::refusal_detail` says so in the refusal itself. The cap has **no git-hook second carrier** — stated at `hooks.rs:1263-1270`. Confirmed still accurate; see `T-19-64` | closed (accepted) |
| T-19-36 | Tampering | torn ledger write | medium | mitigate | Append-only NDJSON; `ends_mid_line` + `tally` count the unparseable line toward the cap rather than skipping it | closed |
| T-19-37 | Information Disclosure | guard request contents in logs | low | mitigate | `guard_in:913` and `main.rs:448-451` redact; the journal park detail is the constant `"a tool call the envelope refused"` | closed |

### Plan 19-06 — branch-protection probe and honesty statement

| Threat ID | Category | Component | Severity | Disposition | Mitigation / Evidence | Status |
|---|---|---|---|---|---|---|
| T-19-38 | Spoofing | probe failure read as safety | high | mitigate | `protection_line` `advisory.rs:296-313` is a total match; `PROTECTED_CLAIM` appears in exactly one arm; the `Unknown` arm carries `WARNING`, the token `unknown` and an explicit disclaimer. See `T-19-67` | closed |
| T-19-39 | Tampering | honesty statement softened | high | mitigate | Pinned constant; 7 phrases plus first-occurrence ordering asserted | closed |
| T-19-40 | Elevation of Privilege | protection mutation from the tool | high | mitigate | `the_module_has_no_write_path_at_all` with an anti-vacuity check; every `run_client` call is `["api", <path>]`. See `T-19-69` | closed |
| T-19-41 | Denial of Service | network probe on the tool-call critical path | high | mitigate | One production call site inside `establish_envelope`; the guard's own no-network scan still passes; 10 s kill-at-deadline budget | closed |
| T-19-42 | Repudiation | preview and journal disagreeing | medium | mitigate | One producer `envelope_notice`, exactly two consumers; same claim text asserted over all three states | closed |
| T-19-43 | Information Disclosure | remote URL in the probe's reason | low | mitigate | `ProtectionState::unknown` is the single constructor and applies `redact`. See `T-19-68` | closed |

### Plan 19-07 — wiring the envelope into the driver

| Threat ID | Category | Component | Severity | Disposition | Mitigation / Evidence | Status |
|---|---|---|---|---|---|---|
| T-19-44 | Elevation of Privilege | a run starting with a partial envelope | high | mitigate | `EnvelopeAssertionFailed` returned before the lock and before the journal; both tests assert `!runs.exists()` afterwards | closed |
| T-19-45 | Tampering | permission-bypass flag reintroduced | high | mitigate | Pinned argv vector with a positive control (`dontAsk`), wildcard-free `match` on `PermissionMode` (E0004 on a new variant), repo-wide grep clean — the 4 source hits are 3 doc comments and one in-`mod tests` hostile fixture | closed |
| T-19-46 | Elevation of Privilege | child env set outside the one closure | high | mitigate | One closure at `executor/claude.rs:480-569`; `EnvelopeEnv` applied as a pure value. See `T-19-71` | closed |
| T-19-47 | Repudiation | success derived from the agent's prose | high | mitigate | Prose grep confined to the file header; every assertion reads an exit code, a file, a git ref or a journal event | closed |
| T-19-48 | Denial of Service | blocking establishment inside an `async fn` | high | mitigate | `spawn_blocking` with clone-not-move; guarded tree-wide by `tests/async_blocking_guard.rs` | closed |
| T-19-49 | Tampering | a second capability-token construction site | medium | mitigate | `tests/spawn_seam_guard.rs` green with the allowlist unchanged; `from_registry` has one production definition | closed |
| T-19-50 | Information Disclosure | envelope paths in argv | low | **accept** | The settings path and hooks dir are not secrets; the token is never on argv, proved by `nothing_the_envelope_wrote_holds_the_credential` | closed (accepted) |
| T-19-56 | Repudiation | a refusal that leaves no trace | high | mitigate | Single-funnel verified call-site by call-site: 9 `park_refusal` sites → one `park` → one `park_at`, with **no branch on reason, tool, verb or refspec** between production and the append. Four named tests read the event off disk from a different process (`strace`-confirmed real execs). See `T-19-61` and `T-19-62` | closed |
| T-19-57 | Tampering | child unsets the locator to suppress its own park | medium | **accept** | Refusal is decided before `park` is called; `a_re_entry_with_the_locator_absent_still_refuses_and_says_it_could_not_park` asserts exit code and reason unchanged plus a `NOT recorded` line | closed (accepted) |
| T-19-58 | Spoofing | a park appended to the wrong run's journal | medium | mitigate | `read_active_run` is the only resolver; `run_paths`' `Option` handled not unwrapped; four hostile pointer shapes append nothing and follow nothing | closed |

### Plan 19-08 — verification gates

| Threat ID | Category | Component | Severity | Disposition | Mitigation / Evidence | Status |
|---|---|---|---|---|---|---|
| T-19-51 | Spoofing | a vacuous gate over filtered output | high | mitigate | The control **fired on itself**: the criterion's own `grep '^warning:'` under `-D warnings` was found returning 0 vacuously and was reported two anchored ways instead | closed |
| T-19-52 | Spoofing | a guard that cannot fail | high | mitigate | Proved fail-first by a real plant observed red then reverted, plus four standing synthetic control arms and a non-empty `source_files()` assert | closed |
| T-19-53 | Repudiation | completion claimed on a green build | high | mitigate | Each requirement marked against a named test run individually; traceability table records which test proves which criterion | closed |
| T-19-54 | Tampering | marker set weakened to clear a hit | medium | mitigate | Failure message names weakening the marker set as the response that is **not** correct; every allowlist entry carries a prose reason; the stale-entry test has already caught one drift | closed |
| T-19-55 | Denial of Service | guard false positives from substring matching | medium | mitigate | Word-boundary matching plus literal blanking, with both control arms | closed |
| T-19-59 | Repudiation | half a criterion ticked on the other half's test | high | mitigate | Five criteria quoted verbatim and split into 24 clause rows; park clauses each name an on-disk `Parked` event; composed clauses marked `[C]` | closed |

### Plans 19-09 / 19-10 — gap closure

| Threat ID | Category | Component | Severity | Disposition | Mitigation / Evidence | Status |
|---|---|---|---|---|---|---|
| T-19-09-01 | Repudiation | `SECTION_ENVELOPE` | high | mitigate | All nine enumerated residual disclosures verified present after the rewrite. See `T-19-70` (only seven mechanically pinned) | closed |
| T-19-09-02 | Tampering | `tests/envelope_advisory.rs` pin | high | mitigate | `git show --stat 45fe2e4` touches `advisory.rs` only; `git diff 395d0bb~1 HEAD -- tests/envelope_advisory.rs` shows **additions only, zero deletions** | closed |
| T-19-09-03 | Information Disclosure | rendered preview / run journal | low | **accept** | Statement unchanged in substance, already user-facing on both surfaces, carries no secret | closed (accepted) |
| T-19-09-04 | Denial of Service | legibility cap | low | **accept** | `MAX_TOKENS = 215` with an arithmetic derivation; the failure is loud and carries the measured count | closed (accepted) |
| T-19-10-01 | Repudiation | `tests/driver_lock.rs` failure reporting | medium | mitigate | `Child::try_wait` polled alongside the holder record; control test asserts status **and** stderr reach the message | closed |
| T-19-10-02 | Tampering | `write_settings_in` read-back | high | mitigate | Sequence intact (persist → read back from the path → refuse); all three 19-10 commits touch **only** `tests/driver_lock.rs` | closed |
| T-19-10-03 | Denial of Service | captured child output | medium | mitigate | `ChildCapture` redirects to files in a `TempDir`; no pipe to drain, no deadlock | closed |
| T-19-10-04 | Tampering | the alias-uniqueness gate | medium | mitigate | Needles assembled at runtime; `!aliases.is_empty()` anti-vacuity assertion precedes the substantive ones | closed |
| T-19-10-05 | Elevation of Privilege | test aliases as path components | low | **accept** | Validated by `envelope_dir_in`; the gate `expect`s `Some`, so a hostile literal is a red test | closed (accepted) |

### Plan 19-11 — structural program resolution (the T-19-60 fix)

| Threat ID | Category | Component | Severity | Disposition | Mitigation / Evidence | Status |
|---|---|---|---|---|---|---|
| T-19-60a | Tampering | `GIT_CONFIG_COUNT` assignment prefix | high | mitigate | `tampers_with_envelope_env` `policy.rs:1263-1269` refuses any word assigning to or bare-naming an envelope key under `hook_bypass_blocked`, at `resolve_program` step 1 — **before** anything is parsed past. Measured by this audit: `GIT_CONFIG_COUNT=0 git push --force origin main` → exit 2 `hook_bypass_blocked`; `env -u GIT_CONFIG_COUNT git push --force` → exit 2; `GIT_SSH_COMMAND=ssh git push origin main` → exit 2. Drift-pinned against `EnvelopeEnv::entries()` (`policy.rs:2782-2824`), pin proved fail-first. **Literal spellings only — see `T-19-81`** | closed |
| T-19-60b | Elevation of Privilege | a command handed to another program as one quoted string | high | mitigate | `resolve_program` step 5(b) `policy.rs:1440-1448` follows a whitespace-bearing token whose first word names a governed program, covering `script -c`, `ssh host "…"` and `command eval "…"` without naming them. Measured: `bash -lc "git push --force origin main"` and `sh -c` → exit 2. Over-refusal cost declared as `T-19-75` | closed |
| T-19-74 | Elevation of Privilege | expansion-assembled program behind a wrapper | medium | **accept** | `env $X push --force` with `$X` bound outside the command line resolves to `Ungoverned` and is permitted. Disclosed at `policy.rs:1373-1381`, pinned on both sides by `tests/envelope_wrapper_class.rs`. Layer 3 still refuses the push, which is the difference from `T-19-60`. **The disclosure's narrowing argument is factually wrong — see `T-19-84`** | closed (accepted) |
| T-19-75 | Denial of Service | over-refusal from the quoted-payload rule | medium | **accept** | `rg "git push --force" src/` is refused. Verified **discriminating**, not blanket: `rg "git status" src/` → exit 0, `rg "git push --force" src/` → exit 2. Blast radius measured and narrow — commit messages, PR titles, `sed`, `jq` and heredocs are unaffected. Disclosed at `policy.rs:1382-1386`. **Park-reason wrinkle undisclosed — see `T-19-85`** | closed (accepted) |

### Plan 19-12 — the class-level control corpus

Five control-quality threats, all about whether the corpus certifying the fix can
fail. Each is closed on its own scope; the adjacent uncovered shape is `T-19-83`.

| Threat ID | Category | Component | Severity | Disposition | Mitigation / Evidence | Status |
|---|---|---|---|---|---|---|
| T-19-76 | Spoofing | a corpus incapable of failing on its own class | medium | mitigate | 12 wrapper names asserted absent from production logic; fail-first proved by mutating `GOVERNED_PROGRAMS`. Verified: no wrapper-name list exists in `src/` (`NESTED_SHELLS` deleted, only comment references survive) | closed |
| T-19-77 | Spoofing | a property that passes by generating nothing | medium | mitigate | Four floors asserted before the generative loop; RHS measured from the unwrapped base in the same run | closed |
| T-19-78 | Spoofing | a fix certified by a suite that would pass if everything were denied | high | mitigate | 1080 wrapped permits asserted with empty stdout, plus the `rg` discrimination pair. Independently corroborated: `ls`, `echo hi`, `env git status`, `rg "git status" src/` all exit 0 | closed |
| T-19-79 | Repudiation | a disclosed residual deleted without notice | medium | mitigate | Doc-disclosure pin over a 3364-byte extracted region with an anti-vacuity floor asserted first. **Pins that the paragraph exists, not that it is true — see `T-19-84`** | closed |
| T-19-80 | Denial of Service | a property that consults a repository per case | low | **accept** | Refspec-less push excluded, one `TempDir` for the whole property, 0.2 s total | closed (accepted) |

### Supply chain

| Threat ID | Category | Component | Severity | Disposition | Mitigation / Evidence | Status |
|---|---|---|---|---|---|---|
| T-19-SC | Tampering | package-manager installs | high | mitigate | `git log --all -- Cargo.toml Cargo.lock` filtered to phase-19 commits returns **zero**. The only `Cargo.toml` changes in the surrounding range are phase-21 commits `f434d71` and `ce9bc2e`. The argv classifier, the shell splitter and the worktree walk are all hand-rolled against `std` | closed |

---

### Threats found by audit 1 (2026-08-29)

Two auditors surfaced surfaces that the plan-time register does not cover. They
are recorded here so they have a durable home rather than living only in a
transcript.

`T-19-61` through `T-19-73` were **deliberately left unremediated** by an
explicit user scoping decision taken after audit 1. Audit 2 re-checked each for
side effects of the 19-11/19-12 edits and carried them forward at their original
severities. They are **open and not accepted**.

| Threat ID | Category | Component | Severity | Disposition | Finding | Status |
|---|---|---|---|---|---|---|
| **T-19-60** | **Elevation of Privilege** | **`resolve_program` command-position selection** | **high** | **mitigate (partial)** | **Audit 1:** the guard classified on `words[0]` alone, so the wrapper family (`env`, `timeout`, `nohup`, `command`, …) and a bare `NAME=VALUE` prefix fell through to a permit. **Audit 2: narrowed, NOT closed.** `policy::resolve_program` replaced `words[0]` and every one of the six measured lines is now refused — but step 5 (`policy.rs:1433-1449`) returns `Governed { index }` for the first token whose basename is governed **without establishing that the token is in a command position**. A wrapper's own *option operand* spelled `git`/`gh`/`glab` captures the index; the real program then becomes `argv[0]` of the classified argv, `classify_git` reads it as the **verb**, `git` is not in the denylist, and the answer is `Allow`. Measured: `env -u git git push --force origin main` → **exit 0**. | **OPEN — BLOCKING** |
| T-19-61 | Repudiation | `hooks::deny` | medium | mitigate | `hooks.rs:1144-1145` write the decision and the stderr mirror with `?` **before** `park_refusal` at `:1156`, so a failed stdio write skips the park for every reason. A broken **stderr** additionally makes `main.rs`'s `eprintln!` double-panic and abort (wait status 134), which a `PreToolUse` consumer reads as a non-blocking error. Not agent-reachable (see below). A second unparked `?` path exists at `hooks.rs:337` (`read_ref_lines`). | open — below `high` (non-blocking) |
| T-19-62 | Repudiation | park coverage | medium | mitigate | The single-funnel property is true today but held **only by reading**. Four of seven `ParkReason` variants and five of nine `park_refusal` call sites have no on-disk coverage; four named single-line deletions leave the whole suite green. Worst: deleting `park_refusal` at `hooks.rs:444` (`refuse_paths`, the reserved-path refusal for *both* hooks) — `tests/envelope_hook_refusals.rs` contains zero journal assertions. | open — below `high` (non-blocking) |
| T-19-63 | Information Disclosure | `CredentialSource::Env` | medium | mitigate | `resolve_credential` reads the variable from the **parent** environment (`cred.rs:619`) and the spawn closure scrubs only `CLAUDE*` plus the `EnvelopeEnv` entries, so the configured variable is inherited by the driven child and readable in its `/proc/<pid>/environ`. Within D-17's accepted class in principle, but it falsifies `write_askpass_stub_in`'s stated "not at rest / not visible in the process table" property for that source. Fix: emit a removal entry for the configured variable name in `build_env`. | open — below `high` (non-blocking) |
| T-19-64 | Repudiation | second-carrier table row 5 | low | mitigate | `hooks::settings_json` has no production caller (`grep -rn settings_json src/` finds only its doc row and its unit test); the driver passes `--settings <path>`. The table's argv-delivery mitigation for the PR cap therefore describes an unwired capability. D-07's "degrade, never disarm" still holds. | open — below `high` (non-blocking) |
| T-19-65 | Repudiation | `askpass_into` | low | mitigate | `ParkReason::CredentialUnavailable` has **no producer**. `cred.rs:717-731` refuses a SAFE-05 credential failure, prints redacted stderr, exits 1, and never calls `park`. Outside T-19-56's declared scope (pre_push / pre_commit / guard) but it is a fourth re-entry point that refuses without a journal trace. | open — below `high` (non-blocking) |
| T-19-66 | Tampering | `hooks::git_dir` | low | mitigate | `hooks.rs:620-642` reads the `gitdir:` pointer from an agent-writable `.git` file and returns it unvalidated before `write_exclude_block` creates directories under it. Not a traversal past a validated boundary, and an agent that can write `.git` already owns the repository's git dir for real git too — but it is unregistered. | open — below `high` (non-blocking) |
| T-19-67 | Spoofing | `protection_line` `{reason}` | low | mitigate | The interpolated reason can carry an agent-set `remote.origin.url` and remote-controlled `gh` stderr. `journal::redact` is a table over secret shapes and home paths; it does not neutralise English prose, so the protected wording is quotable **into** an `unknown` line. The state token, `WARNING` marker and disclaimer all remain, so a human is not misled; a downstream grep would be. | open — below `high` (non-blocking) |
| T-19-68 | Information Disclosure | `ProtectionState::Unknown` | low | mitigate | The variant's field is publicly constructible, so the redacting single constructor is a convention rather than a compiler-enforced boundary. No bypassing site exists today. | open — below `high` (non-blocking) |
| T-19-69 | Elevation of Privilege | T-19-40's source grep | low | mitigate | The grep forbids `POST`/`PUT`/`PATCH`/`DELETE` literals, but `gh api -f`/`--field` **implies** POST with no explicit method. The second leg (withheld `Administration` scope) is a documented product decision, not a code-enforced control. | open — below `high` (non-blocking) |
| T-19-70 | Repudiation | honesty-statement pins | low | mitigate | Only seven of the ~ten enumerated residual disclosures are mechanically pinned. `removed, not emptied`, `naming no credential helper`, `GIT_CONFIG_COUNT`, `askpass responder itself reads the token` and `silently ignores` could each be dropped with no test failing. | open — below `high` (non-blocking) |
| T-19-71 | Elevation of Privilege | T-19-46's `cmd.env(` grep | low | mitigate | The needle is binding-name-anchored. `advisory.rs:476-479` already applies `EnvelopeEnv` through a binding named `command`, and `project_creator.rs:72-74` sets child env too. The security property (one applier for the *agent* child) holds; the control has a blind spot. | open — below `high` (non-blocking) |
| T-19-72 | Spoofing | T-19-18's declared evidence | low | mitigate | `19-03-SUMMARY.md:176` cites `tests/envelope_hook_refusals.rs` as proving the `Absent`-never-permits property. No such assertion exists anywhere in `tests/`. The mitigation is present and correct (verified structurally and empirically) but **unpinned** — an edit making `is_clean()` consult `external` turns nothing red. | open — below `high` (non-blocking) |
| T-19-73 | Information Disclosure | `CredentialSource` drift | low | mitigate | T-19-11 holds by construction but has no drift-pin test; a third variant carrying a literal token would compile and pass the suite. Cheap fix: assert `serde_json::from_str::<CredentialSource>(r#"{"source":"literal","token":"x"}"#)` is an `Err`. | open — below `high` (non-blocking) |

**Audit 2 re-check of T-19-61 … T-19-73: all thirteen unchanged.** The
19-11/19-12 diff is exactly four files (`git diff --numstat 5e574c4..HEAD`):
`src/envelope/hooks.rs`, `src/envelope/policy.rs` (additions only, 641/0) and the
two new test files. `cred.rs`, `mod.rs`, `advisory.rs`, `scan.rs` and `config.rs`
are byte-identical, so `T-19-63` and `T-19-65` … `T-19-73` cannot have moved.
Two were re-verified directly because 19-11 touched their file:

- **`T-19-61` — unchanged.** `hooks::deny` is the function 19-11 deliberately did
  not touch. The `?`-before-`park_refusal` ordering is intact: `hooks.rs:1212`
  and `:1213` both use `?`, `park_refusal` is at `:1224` (line numbers shifted
  from 1144/1156 by insertions above; the sequence is identical). The second
  unparked `?` at `hooks.rs:337` (`read_ref_lines`) is also unchanged. Still not
  agent-reachable; still not a blocker on its own merits.
- **`T-19-62` — unchanged; the new tests neither close nor weaken it.** All nine
  `park_refusal` call sites intact (`:331, :354, :388, :399, :444, :688, :709,
  :826, :1224`); `super::park` still has exactly one non-test caller. Neither
  `tests/envelope_wrapper_bypass.rs` nor `tests/envelope_wrapper_class.rs`
  contains an on-disk `Parked` assertion, so the four named single-line deletions
  still leave the suite green. Not a blocker on its own merits.

---

### Threats found by audit 2 (2026-08-29, after plans 19-11 / 19-12)

| Threat ID | Category | Component | Severity | Disposition | Finding | Status |
|---|---|---|---|---|---|---|
| **T-19-81** | **Tampering / Elevation of Privilege** | **`resolve_program` step 1 — literal-token matching of `ENVELOPE_ENV_KEYS`** | **high** | **mitigate (unimplemented)** | **The envelope-key refusal matches only a token that literally *is* the key or assigns to it. The name can be bound to a shell variable in a prior segment of the *same* Bash tool call and then removed indirectly, which the guard does not see. Measured: `K=GIT_SSH_COMMAND; SSH_AUTH_SOCK=/tmp/evil env -u $K git fetch origin` → **exit 0**. That single tool call removes the `GIT_SSH_COMMAND` braces *and* restores the `SSH_AUTH_SOCK` belt, putting the user's own ssh-agent and default identities back inside a driven run — the escape D-16 and `T-19-23` (high) exist to prevent. Nothing checks an assignment's **value** against `ENVELOPE_ENV_KEYS`, although step 7 already performs exactly that check against `GOVERNED_PROGRAMS`. Independent of `T-19-60`: the pure form `env -u $K git push --force origin main` is still refused, so this is not the decoy mechanism.** | **OPEN — BLOCKING** |
| T-19-82 | Information Disclosure | `ENVELOPE_ENV_KEYS` vs. the envelope's **removal** entries | medium | mitigate | `SSH_AUTH_SOCK` and `SSH_AGENT_PID` are removed by `cred::build_env_in` (`cred.rs:402-403`) but absent from `ENVELOPE_ENV_KEYS`, and the drift pin's `filter(\|(_, value)\| value.is_some())` guarantees it will never notice — the pin would not have caught `GIT_SSH_COMMAND` either, had that key been a removal. Measured: `SSH_AUTH_SOCK=/tmp/evil git fetch origin` → exit 0. **Inert on its own** while `GIT_SSH_COMMAND` holds (`IdentityAgent=none`, `IdentitiesOnly=yes`, `-F /dev/null`); live in combination with `T-19-81`. The pin's second filter, `starts_with("GIT_") \|\| starts_with("GH_")`, additionally excludes a future `GITHUB_*`, `GLAB_*` (a governed program) or `SSH_*` key. | open — below `high` (non-blocking) |
| T-19-83 | Spoofing | `tests/envelope_wrapper_class.rs` `WRAPPERS` alphabet | medium | mitigate | The generative class-level corpus contains wrappers with operands (`env -u SOME_VAR`, `runuser -u me --`) but **no operand whose basename is a governed program**, so it is structurally incapable of failing on the surviving `T-19-60` shape. The discriminating pair, measured: `env -u SOME_VAR git push --force origin main` → exit 2, `env -u git git push --force origin main` → exit 0. This is `T-19-76`'s own failure mode one level up — the corpus certifying the fix cannot fail on the class it certifies. | open — below `high` (non-blocking) |
| T-19-84 | Repudiation | `T-19-74`'s disclosure in `resolve_program`'s doc | low | mitigate | The doc (`policy.rs:1373-1381`) claims the residual requires `$X` "bound outside this command line" and that "each Bash tool call being its own shell process is what keeps it narrow". Both are false: `env $(echo git) push --force origin main` → exit 0 with no prior binding and no outside state, and `K=…; env -u $K …` binds inside the same tool call in a segment the guard can see. `T-19-79`'s doc-disclosure pin enforces that the paragraph *exists*, not that it is *true*. | open — below `high` (non-blocking) |
| T-19-85 | Repudiation | `T-19-75`'s park reason | low | mitigate | A benign `rg "git push --force" src/` is refused **and parked under `force_push_blocked`**, so the run journal records a force-push refusal for a source search. The accept rationale discloses the refusal but not the misdescribing evidence record (D-24/D-25). | open — below `high` (non-blocking) |

*Status: open · closed · open — below `high` threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` count toward `threats_open`*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## T-19-60 after 19-11/19-12 — narrowed, not closed

### What the fix does deliver

Verified independently against the built binary, not accepted from the
summaries. The guard was driven as
`printf '{"tool_name":"Bash","tool_input":{"command":"<CMD>"}}' | GSD_MM_ENVELOPE_ROOT=<fresh tempdir> ./target/debug/gsd-meta-manager envelope guard alpha`,
**one fresh tempdir per row** (the PR ledger persists and otherwise produces
misleading cap-exhaustion refusals):

```
exit=2  git push --force origin main                          <- control, correct
exit=2  env git push --force origin main                      <- FIXED (force_push_blocked)
exit=2  GIT_CONFIG_COUNT=0 git push --force origin main       <- FIXED (hook_bypass_blocked)
exit=2  timeout 60 git push --force origin main               <- FIXED (force_push_blocked)
exit=2  command git push --force origin main                  <- FIXED (force_push_blocked)
exit=2  env gh pr create --title x  (2nd call)                <- FIXED, ledger line written
exit=2  /usr/bin/env git push --force origin main             <- FIXED
exit=2  nohup nice -n 5 stdbuf -o0 timeout 60 env git push …  <- FIXED (deep chain)
exit=2  made-up-wrapper-9000 git push --force origin main     <- FIXED (unnamed wrapper)
exit=2  GIT_SSH_COMMAND=ssh git push origin main              <- FIXED (hook_bypass_blocked)
exit=2  env -u GIT_CONFIG_COUNT git push --force origin main  <- FIXED (hook_bypass_blocked)
exit=2  X=git; $X push --force                                <- FIXED (envelope_assertion_failed)
exit=0  ls                                                    <- permitted, correct
exit=0  echo hi                                               <- permitted, correct
exit=0  env git status                                        <- permitted, correct
exit=0  rg "git status" src/                                  <- permitted, correct
```

`env gh pr create --title x` in a fresh root exits 0 **and writes a ledger line**
(`{"command":"gh pr create","platform":"github"}`), so the SAFE-06 cap now counts
the wrapped form. `NESTED_SHELLS` is genuinely gone and no wrapper-name list
replaced it. The mechanism is real and the six original lines are genuinely
closed.

### The shape that survives

`resolve_program` step 5 (`policy.rs:1433-1449`) returns `Governed { index }` for
the **first token in the segment whose basename is in `GOVERNED_PROGRAMS`**,
skipping only tokens that start with `-`. It never establishes that the token it
found is in a *command position*. So any earlier token literally spelled
`git`/`gh`/`glab` — typically a wrapper's own **option operand** — captures the
index. `classify_segments` then applies `classify_git` to `words[index+1..]`, the
real `git` becomes `argv[0]` of that slice, `scan_leading` breaks on it (it does
not start with `-`), `classify_git` reads it as the **verb**, the verb `git` is
not in the denylist, and the answer is `Allow`.

Measured by this audit, each in a fresh root:

| Command line | exit | Consequence |
|---|---|---|
| `env -u SOME_VAR git push --force origin main` | 2 | correct — the discriminator |
| **`env -u git git push --force origin main`** | **0** | **force push permitted** |
| **`env -u git git -c core.hooksPath=/dev/null push --force origin main`** | **0** | **all three layers cleared, credential intact** |
| **`env -u gh gh pr create --title x`** | **0** | **PR cap bypassed, no ledger line, no park** |
| `sudo -u git git push --force origin main` | 0 | force push permitted |
| `env -u git git stash` | 0 | permitted; `stash` has no hook carrier |
| `env $(echo git) push --force origin main` | 0 | permitted (layer 3 still fires) |

**Why row 2 is high and blocking.** It is a *single simple command* — no
assignment, no expansion, no semicolon, no shell nesting — and it defeats all
three layers with the credential intact:

1. **Layer 1** — `Bash(git push:*)` is a prefix rule owned by the agent CLI; the
   line starts with `env`.
2. **Layer 2** — measured exit 0 above.
3. **Layer 3** — `git -c core.hooksPath=` is the one form that outranks the
   envelope's env-injected setting, which `policy.rs`'s own `T-19-09` refusal
   says outright (D-09). Verified empirically:
   `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=/ENV_WINS git -c core.hooksPath=/CLI_WINS config --get core.hooksPath`
   prints `/CLI_WINS`. No `pre-push` hook runs.
4. **Credentials survive.** `GIT_ASKPASS`, `GIT_CONFIG_GLOBAL/SYSTEM` and
   `GIT_SSH_COMMAND` are untouched, so the push authenticates. This is **strictly
   worse than the original T-19-60**, whose D-09 note recorded that the escape at
   least cost the agent its token.

Rows 4 and 6 have **no second carrier at all** — `git stash`, `update-ref` and
the PR cap have no `pre-push`/`pre-commit` backstop (`hooks.rs:1263-1270`,
`T-19-35`) — so for those the guard is the only control and it is bypassed
outright.

**Why the 19-12 class-level corpus could not catch it.** Its `WRAPPERS` alphabet
contains wrappers *with operands* (`env -u SOME_VAR`, `runuser -u me --`) but no
operand whose basename is a governed program. Recorded as `T-19-83`.

**Suggested closure.** Return `Governed` only for a token in a command position —
the first non-option word after the head, having accounted for wrapper option
operands — and `Refuse` rather than mis-index when command position cannot be
established structurally. Independently and more cheaply: `classify_git` should
**refuse**, not `Allow`, an argv whose verb is itself `git`/`gh`/`glab`, which
kills the whole decoy family in one line. Then add `env -u git`-shaped operands
to the 19-12 alphabet so the corpus can fail on this class.

## T-19-81 — the second blocking threat, in detail

`resolve_program` step 1 refuses a token that literally *is* an envelope key or
assigns to one. It does not look at what an assignment's **value** is, even
though step 7 already performs exactly that check against `GOVERNED_PROGRAMS`.
So the key name can be bound in a prior segment of the **same** Bash tool call
and removed indirectly:

```
exit=2  env -u $K git push --force origin main                                  <- refused (no decoy)
exit=2  K=GIT_CONFIG_COUNT; env -u $K git push --force origin main              <- refused
exit=0  K=GIT_SSH_COMMAND; SSH_AUTH_SOCK=/tmp/evil env -u $K git fetch origin   <- PERMITTED
exit=0  SSH_AUTH_SOCK=/tmp/evil git fetch origin                                <- PERMITTED
```

The third line is the finding. In one tool call it removes `GIT_SSH_COMMAND` —
the braces carrying `IdentitiesOnly=yes`, `IdentityAgent=none`, `BatchMode=yes`
and `-F /dev/null` — **and** restores `SSH_AUTH_SOCK`, the belt. Neither half is
refused: the removal is indirect so step 1 does not see the key, and
`SSH_AUTH_SOCK` is not in `ENVELOPE_ENV_KEYS` at all (`T-19-82`). The socket is
discoverable from inside the run (`ls -d /tmp/ssh-*/agent.*`). Together this puts
the user's own ssh-agent and default identities back within reach of a driven
run, which is exactly what D-16 and `T-19-23` (high) exist to prevent, and what
`SECTION_ENVELOPE` states outright is not possible.

It is **independent of `T-19-60`**: the pure form without the `git` decoy is
still refused, so this is not the same mechanism re-measured.

**Suggested closure.** Refuse a leading assignment whose **value** is covered by
`envelope_env_key`, symmetric to step 7's `GOVERNED_PROGRAMS` check; and treat an
expansion-bearing operand of a removal-style flag as unresolvable rather than
skippable.

## `ENVELOPE_ENV_KEYS` — completeness and drift-pin strength

The orchestrator asked this specifically, on the grounds that a key list wrong
once may be wrong again. It was.

**Completeness — enumerated from `cred::build_env_in` (`cred.rs:398-475`) rather
than taken from the constant:**

| Entry | Kind | Covered by `ENVELOPE_ENV_KEYS`? |
|---|---|---|
| `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_n` / `GIT_CONFIG_VALUE_n` | set | yes (prefix entries) |
| `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_SYSTEM` | set | yes |
| `GIT_ASKPASS`, `GIT_TERMINAL_PROMPT` | set | yes |
| `GIT_SSH_COMMAND` | set | yes — **added by 19-11, this was the live bug** |
| `GH_CONFIG_DIR` | set | yes |
| `SSH_AUTH_SOCK` | **removed** | **NO** — `T-19-82` |
| `SSH_AGENT_PID` | **removed** | **NO** — `T-19-82` |
| `GSD_MM_ENVELOPE_PROJECT_ROOT` | set | no (evidence locator; within accepted `T-19-57`) |
| `GSD_MM_RUN_ID` | set | no (evidence locator; within accepted `T-19-57`) |

**Verdict: complete for every SET `GIT_`/`GH_` key; incomplete for the two
removals.**

**Drift-pin strength: partial — it will catch the next omission only if that
omission is a SET key whose name begins exactly `GIT_` or `GH_`.** The pin
(`policy.rs:2782-2824`) carries two filters, and both exclude live surface:

- `filter(|(_, value)| value.is_some())` excludes **every removal entry**.
  Removal is not an implementation detail here — `EnvelopeEnv`'s own doc
  (`cred.rs:129-135`) says D-16 "turns on removal rather than overwriting". This
  filter is precisely why `SSH_AUTH_SOCK`/`SSH_AGENT_PID` are invisible to the
  pin, and it means **the pin would not have caught `GIT_SSH_COMMAND` either had
  that key been a removal**.
- `name.starts_with("GIT_") || name.starts_with("GH_")` defensibly excludes
  `GSD_MM_*`, but also silently excludes a future `GITHUB_TOKEN`
  (`"GITHUB_TOKEN".starts_with("GIT_")` is `false`), `GLAB_*` — for a program
  this envelope *governs* — and `SSH_*`. A `GLAB_CONFIG_DIR` added tomorrow, the
  exact analogue of `GH_CONFIG_DIR`, would sail past.

The `set.len() >= 5` anti-vacuity floor is real but loose (satisfied 9-to-5).

**Fail-first proof, performed and reverted.** `"GIT_SSH_COMMAND"` was deleted
from the constant and the single unit test run — **RED**:

```
panicked at src/envelope/policy.rs:2815:13:
`GIT_SSH_COMMAND` is SET in the driven child's environment by `cred::build_env_in`
but is not covered by `ENVELOPE_ENV_KEYS`, so a command that reassigns or removes
it is not refused. Add it to the constant rather than narrowing this test.
test result: FAILED. 0 passed; 1 failed
```

Reverted with `git checkout -- src/envelope/policy.rs`; tree clean afterwards.
So the pin is genuinely non-vacuous for the class it covers — it simply covers
less than its doc comment implies.

**Suggested closure (`T-19-82`).** Add `SSH_AUTH_SOCK` and `SSH_AGENT_PID` to
`ENVELOPE_ENV_KEYS`; drop the `value.is_some()` filter so removals are pinned
too; and widen the name filter beyond `GIT_`/`GH_`.

---

## T-19-60 as recorded by audit 1 — the original mechanism

Retained for the record; the `words[0]` mechanism below is closed.

**What it was.** The `PreToolUse` guard decided what a command was by looking at
`words[0]` and nothing else. `classify_segments` (`src/envelope/hooks.rs:929-1054`)
resolves `policy::program_name(words[0])` and then asks three questions of it: is
it `eval`, is it a nested shell with a `-c` payload, is it `git`; finally
`pr_command_label` asks whether it is `gh`/`glab`. A command whose *first token*
is anything else — a wrapper binary or a shell variable assignment — matches none
of them and falls through to `Ok(None)`, which `guard_in` (`hooks.rs:901`) turns
into a permit.

**Reproduced against the built binary.** Driven as
`gsd-meta-manager envelope guard alpha` with `GSD_MM_ENVELOPE_ROOT` pointed at a
temporary directory, feeding a `PreToolUse` request per line:

```
exit=2  git push --force origin main                      <- denied, correct
exit=0  env git push --force origin main                  <- PERMITTED
exit=0  GIT_CONFIG_COUNT=0 git push --force origin main   <- PERMITTED
exit=0  timeout 60 git push --force origin main           <- PERMITTED
exit=0  command git push --force origin main              <- PERMITTED
exit=0  gh pr create --title x                            <- permitted, ledger line written
exit=0  env gh pr create --title x                        <- PERMITTED, NO ledger line
```

The ledger after the run holds exactly one entry, for the unwrapped
`gh pr create`. The wrapped form was not counted.

**Why it is high and not medium.** Three independent guarantees fail on one line:

1. **The layered-defence claim inverts.** `GIT_CONFIG_COUNT=0 git push --force`
   is a single, unquoted, non-`eval` command. The assignment neutralises layer 3
   (`core.hooksPath` is no longer injected, so no `pre-push` hook runs) *and*
   the same assignment is what makes layer 2 fail to recognise the command. The
   documented ceiling (D-09) says an agent past `GIT_CONFIG_COUNT` is past the
   last layer; in practice one token clears layers 2 and 3 simultaneously.
2. **Layer 1 does not backstop it.** `--disallowedTools Bash(git push:*)` is a
   prefix rule owned by the agent CLI. A command beginning with `env` or with an
   assignment does not match that prefix. This repository asserts only that the
   two deny lists are *identical* (`tests/envelope_pr_cap.rs:404`), never that
   the CLI's matcher catches wrapper forms.
3. **The PR cap (SAFE-06) is bypassed by a four-letter prefix**, with no ledger
   line and therefore no park. This is *not* the accepted `T-19-35` — that risk
   is about a CLI that ignores the settings file entirely. Here the guard runs,
   sees the command, and permits it.

**Why it is not covered by an existing acceptance.** `T-19-14` accepts a denylist
over *git verbs*; this is the *program token*, one level up. `T-19-31`'s stated
mitigation (an unrecoverable split denies) is present and correctly tested — the
words here are perfectly recoverable, they are simply attributed to the wrong
program. The design intent already covers this class: deviation 2 of
`19-05-SUMMARY.md` added `sh -c` following precisely because "its outer program
is `bash` and the classifier would have seen nothing." The wrapper and
assignment-prefix family is the same argument, unimplemented.

**Suggested closure.** Before selecting the program: strip leading `NAME=VALUE`
tokens, and treat `env`/`command`/`nohup`/`timeout`/`nice`/`stdbuf`/`ionice`/`sudo`
as transparent (skipping their own options and `-u`/`-i`/assignment operands),
then re-classify. Any wrapper whose operand shape is not understood should
**deny** rather than fall through. Add a test row per form, and a control
asserting the fall-through default is deny for an unrecognised wrapper.

---

## The two items flagged for this review

### 1. `deny`'s `?` before `park_refusal` — not exploitable

The premise is confirmed: `hooks.rs:1144-1145` use `?`, and `park_refusal` is at
`:1156`, after both. Traced and reproduced:

- **Where the `Err` lands.** Every `guard_in` refusal is `return deny(...)`;
  `deny` → `guard_in` → `guard` → `main.rs:438-454`, whose `Err` arm redacts and
  calls `std::process::exit(2)`. Reproduced with a broken stdout pipe: exit **2**,
  no decision JSON, `park_refusal` never runs.
- **Enforcement holds for a stdout failure.** Exit 2 is the hook protocol's
  blocking code and is `deny`'s stated second carrier. Only the journal park is
  lost — a D-24/D-25 evidence gap, the same class as the already-accepted
  `T-19-57`.
- **A broken *stderr* is worse but still not an escape.** `main.rs`'s `eprintln!`
  panics, the panic hook's own stderr write fails, double-panic aborts — wait
  status 134, which a `PreToolUse` consumer reads as a *non-blocking* error. That
  is the one arm where the deny would degrade to a permit.
- **The agent cannot reach it.** The guard is spawned by the agent CLI as a
  `PreToolUse` hook; the CLI owns and drains both pipes. A Bash tool call cannot
  touch the fds of a sibling process it did not spawn. Exhaustion is not a route
  either — the decision is ~300 bytes against a 64 KiB pipe buffer, so there is
  no partial write and no block. The agent can of course run
  `gsd-meta-manager envelope guard <alias>` itself with closed stdio, but that
  instance's verdict governs nothing.

**Verdict: not exploitable, but a genuine correctness defect.** It contradicts
`park_refusal`'s own stated discipline ("write the refusal to stderr, park, then
exit non-zero — and the exit code does not depend on what this returned"), and
`deny` is the only refusal path in the module that makes the park conditional;
`pre_push` and `pre_commit` get the ordering right. Recorded as `T-19-61`. The
fix is one line of class: `let _ = writeln!(…); let _ = writeln!(…);
park_refusal(…); Ok(2)`.

### 2. The single-funnel property — true today, medium regression risk

**The property is true.** Verified call-site by call-site rather than by
inspection of the happy path. Nine `park_refusal` sites exist, all in
`src/envelope/hooks.rs` (`:331`, `:354`, `:388`, `:399`, `:444`, `:688`, `:709`,
`:810`, `:1156`). `super::park` has exactly one non-test caller; `park_at` has
exactly one. Along `policy.rs:292` → `hooks.rs:1019` → `hooks.rs:900` →
`hooks.rs:1156` → `hooks.rs:50` → `mod.rs:388` → `mod.rs:326` there is **no match
on reason, tool, verb or refspec**; the only `match` is on the locator env var.
`ParkReason::as_str` is a total seven-arm map.

**The coverage is thin.** Only four of the seven reasons have a test asserting an
on-disk `Parked` event: `ForcePushBlocked`, `HookBypassBlocked`, `SecretDetected`,
`PrCapExceeded` (all in `tests/envelope_wiring.rs`, each read from a process other
than the one that refused — confirmed non-vacuous via `strace`, showing real
`git` and stub execs). `PushOutsideNamespace` and `EnvelopeAssertionFailed` are
asserted on **stderr only**. `CredentialUnavailable` has no producer at all
(`T-19-65`).

**Named edits that break the property with a green suite:** an early return in
`deny` for `EnvelopeAssertionFailed`; deleting `park_refusal` at `hooks.rs:709`,
`:331`, `:388`, `:399` or `:810`; and worst, deleting it at `hooks.rs:444`
(`refuse_paths` — the reserved-path/nested-repo refusal for *both* hooks), since
the on-disk `ForcePushBlocked` assertion reaches the journal through the guard's
`deny`, not through `refuse_paths`, and `tests/envelope_hook_refusals.rs` has
zero journal assertions.

**Verdict: medium silent-regression risk, non-blocking under `block_on: high`.**
Recorded as `T-19-62`. The cheapest closure is the parameterized control the
register implies but never asked for: iterate a hand-spelled `[ParkReason; 7]`
(spelled out, not derived — the discipline `ENVELOPE_REASONS` already uses in
`tests/driver_refusal_record.rs:169`), drive each through the funnel against a
live fixture run, and assert the on-disk event.

---

## Known residual exposure — re-verified against the code

These four were disclosed and tracked before this audit. Each was checked for
whether the code still matches the description.

| # | Disclosed residual | Still accurate? |
|---|---|---|
| 1 | **D-09 ceiling** — an agent that unsets `GIT_CONFIG_COUNT` in a subshell is past the last layer | **Accurate for its own layer; the surrounding defence-in-depth belief is not.** `hooks_path_env` injects exactly one key (`cred.rs:249-251`), hooks are installed only under `<envelope>/<alias>/hooks/`, and the other envelope entries (`GIT_CONFIG_GLOBAL`/`SYSTEM`, `GIT_ASKPASS`, `GH_CONFIG_DIR`, `GIT_TERMINAL_PROMPT`, the `SSH_*` removals) are independent and survive. The doc sentence at `cred.rs:243` is exactly right. What is **not** right is the implied backstop: `T-19-60` shows layer 2 permits the very command that performs the unset, so one token clears layers 2 and 3 together. An unconfigured run still cannot authenticate (`GIT_ASKPASS` and the helper-free config survive), which is what keeps this at "escape the envelope's controls", not "escape with the token". |
| 2 | **D-17** — an agent inside the driven run can execute the askpass responder and read the token | **Accurate and documented**, at `cred.rs:488-495`. The stub is written 0700 and its path is handed to the child in `GIT_ASKPASS`, so it need not be guessed; nothing in `disallowed_tools()` or the guard denies executing a program; the baked host is in plain text in the stub, so the agent can supply a matching prompt. The host restriction (`T-19-25`) is indeed the only gate, and it is a gate against *git being pointed at a second remote*, not against the agent. **One correction:** the same paragraph claims the token "is not at rest anywhere, is not visible in the process table". For `CredentialSource::Env` the variable is read from the parent and inherited by the child, so `echo $VAR` in the run suffices — recorded as `T-19-63`. |
| 3 | **T-19-35** — the PR cap has no git-hook second carrier and goes silently unenforced if the CLI ignores the settings file | **Accurate and documented**, at `hooks.rs:1263-1270` ("No git hook observes `gh pr create`"). Confirmed that the cap is the *only* one-carrier control: `permissions.deny` (all ten patterns) is duplicated onto `--disallowedTools` via `driver/run.rs:1663` → `executor/claude.rs:286-289`, the push boundary has `pre-push`, the swept-worktree boundary has `pre-commit`. **One inaccuracy:** the second-carrier table cites `settings_json` rendering the value inline on argv as the cap's remaining mitigation, and `settings_json` has no production caller — recorded as `T-19-64`. Separately, `T-19-60` shows the cap is *also* bypassable while the guard is running, which is a different failure than the one T-19-35 accepts. |
| 4 | **`gitleaks` is optional; only the `Absent` arm runs here, and it must never fail open** | **Accurate — it is structurally incapable of failing open.** `scan_with_external` (`scan.rs:392-406`) runs `scan_worktree` first and unconditionally; the only external-driven mutation is a `push` onto `findings`; `run_gitleaks` returns a value and never receives `&mut report`; and `is_clean()` (`:188-190`) is `findings.is_empty() && !root_unreadable` — it **never reads `self.external`**. The error arms are correctly polarised: only `Err(NotFound)` maps to the inert `Absent`; every other error and any non-zero exit maps to `Failed`/`Blocked`, which *add* a blocking finding. Verified empirically on this machine (gitleaks confirmed absent): a dirty worktree still yields one finding and exit 1. **The gap is evidentiary, not behavioural:** the test cited as proof does not exist — recorded as `T-19-72`. |

---

## The two residuals 19-11 registered — are they correctly classified?

### `T-19-74` — `accept` at medium is correct *as scoped*; the disclosure is wrong

**(a) Is the justification true?** Partly. Refusing every `$VAR` in an ungoverned
segment would indeed refuse ordinary work — `echo $(git rev-parse HEAD)` and
`cd "$HOME"` both measured exit 0 today and would stop. But the stated cost is
inflated: a narrower rule (an expansion token in a *command position*) touches
neither. And the justification **does not transfer** to the envelope-key half,
where a targeted fix — checking assignment *values* against `ENVELOPE_ENV_KEYS`,
symmetric to step 7 — costs nothing. That half is `T-19-81`, and it is **not**
covered by this acceptance.

**(b) Do the two claimed narrowings hold?**

| Claimed narrowing | Measured | Holds? |
|---|---|---|
| step 1 refuses envelope-key assignments | `GIT_CONFIG_COUNT=0 git push --force` → 2; `env -u GIT_CONFIG_COUNT git push --force` → 2 | only for **literal** spellings |
| — same, indirect | `K=GIT_SSH_COMMAND; SSH_AUTH_SOCK=… env -u $K git fetch` → **0** | **NO** (`T-19-81`) |
| step 7 refuses a governed name bound in the same command line | `X=git env $X push --force` → 2 `envelope_assertion_failed` | yes, for the same *simple* command |
| — self-contained substitution | `env $(echo git) push --force origin main` → **0** | **NO — undisclosed** (`T-19-84`) |

**(c) Honestly disclosed?** The mechanism is disclosed at `policy.rs:1373-1381`
and pinned by `resolve_programs_own_doc_still_discloses_the_residual_it_does_not_cover`
plus two-sided bound tests — that machinery works and the residual cannot be
deleted silently. **But the narrowing argument is false as written.** It says
`$X` "was bound outside this command line" and that "each Bash tool call being
its own shell process is what keeps it narrow"; `env $(echo git) push --force`
needs no prior binding and no outside state at all, and `K=…; env -u $K …` binds
inside the same tool call in a segment the guard can see. The doc pin enforces
that the paragraph *exists*, not that it is *true*. Recorded as `T-19-84` (low).

**Classification: `accept` at medium is correct and non-blocking** — layer 3
still refuses the force push for the pure program-name half, which is the whole
difference from `T-19-60`.

### `T-19-75` — `accept` is correct, honestly disclosed, blast radius narrower than feared

Both directions verified: `rg "git push --force" src/` → **exit 2**
`force_push_blocked`; `rg "git status" src/` → **exit 0**. It discriminates as
claimed rather than blanket-denying.

The trigger is narrow: step 5(b) fires only when a token *contains whitespace*
**and** its first word's basename is a governed program, and refuses only if the
payload then classifies as a refusal. Blast radius measured rather than assumed:

| Realistic driven-run command | exit |
|---|---|
| `rg "git push --force" src/` | 2 |
| `grep -rn "git stash" .` | 2 |
| `printf 'git update-ref -d refs/heads/x\n'` | 2 |
| `git commit -m "fix: stop git push --force bypassing the guard"` | 0 |
| `gh pr create --title "block git push --force" --body x` | 0 |
| `echo "run git stash to save work"` | 0 |
| `git log --grep="git stash"` · `sed -i 's/git stash/x/' README.md` · `jq` · heredocs | 0 |

Commit messages, PR titles, `sed`, `jq` and heredocs are unaffected; the cost is
confined to `grep`/`rg`/`echo`/`printf` over a string that *starts* with
`git `/`gh `/`glab `. Disclosed at `policy.rs:1382-1386` and pinned by
`a_search_for_an_allowed_git_command_runs_and_a_search_for_a_refused_one_does_not`.

**Classification: `accept` is correct and non-blocking.** One undisclosed
wrinkle: the false positive parks an on-disk `force_push_blocked` event, so a
benign `rg` writes a journal record that misdescribes what happened (D-24/D-25).
Recorded as `T-19-85` (low).

---

## Accepted Risks Log

Nine `accept` dispositions, carried here so they do not resurface as OPEN on the
next audit. Each is documented in its plan's threat register; those with a test
name it.

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-19-01 | T-19-06 | The envelope directory path is not itself secret; D-02 places it outside the repository so it cannot be swept into a commit, which is the disclosure that matters. Asserted by `envelope_tracer.rs:163-167` | phase-19 plan author | 2026-08-29 |
| AR-19-02 | T-19-07 | Two path canonicalizations per push are not on the agent's per-tool-call critical path; the guard's registered timeout covers the path that is | phase-19 plan author | 2026-08-29 |
| AR-19-03 | T-19-14 | An allowlist over git verbs would be stronger and would break unanticipated GSD plumbing commands. The pre-push and pre-commit hooks make the denylist's gaps non-fatal. Revisit condition recorded at `policy.rs:226-241` | phase-19 plan author | 2026-08-29 |
| AR-19-04 | T-19-29 | The generated git config lives outside the repository but the child can still write it if it locates the path; the envelope regenerates it at each run start and the honest answer is D-27's server-side branch protection | phase-19 plan author | 2026-08-29 |
| AR-19-05 | T-19-35 | The guard is the only observation point for a PR attempt, so the ledger biases to over-count via write-before-permit. The limit is stated in `CapVerdict::refusal_detail` and in the refusal message itself | phase-19 plan author | 2026-08-29 |
| AR-19-06 | T-19-50 | The settings path and hooks directory are not secrets; the token is the secret and it is never on argv (D-17). Proved by `nothing_the_envelope_wrote_holds_the_credential` | phase-19 plan author | 2026-08-29 |
| AR-19-07 | T-19-57 | Refusal is decided before `park` is called, so suppressing the run-journal locator costs the agent its own evidence and buys it nothing; a second stderr line names the unrecorded park. Asserted by `a_re_entry_with_the_locator_absent_still_refuses_and_says_it_could_not_park` | phase-19 plan author | 2026-08-29 |
| AR-19-08 | T-19-09-03, T-19-09-04 | The honesty statement is unchanged in substance and already user-facing on both surfaces; the 190/215-token legibility cap failing loudly on a future addition is the cap doing its job | phase-19 plan author | 2026-08-29 |
| AR-19-09 | T-19-10-05 | Test aliases are compile-time literals in this repository's own test file, validated by `envelope_dir_in`; the gate `expect`s `Some`, so a hostile literal is a red test. No untrusted input reaches the join | phase-19 plan author | 2026-08-29 |
| AR-19-10 | T-19-74 | Closing the expansion-assembled-program residual requires refusing every `$VAR` in an ungoverned command, which also refuses `echo $(git rev-parse HEAD)` and `cd "$HOME"` — a control that fails into unusability gets switched off. Layer 3 still refuses the force push for this half. Disclosed in `resolve_program`'s doc and pinned on both sides | 19-11 plan author | 2026-08-29 |
| AR-19-11 | T-19-75 | The over-refusal is confined to driven runs, legible rather than silent, and **discriminating** — the payload is classified, so `rg "git status" src/` is permitted. Measured blast radius excludes commit messages, PR titles, `sed`, `jq` and heredocs | 19-11 plan author | 2026-08-29 |
| AR-19-12 | T-19-80 | Excluding the refspec-less push case and sharing one `TempDir` keeps the generative property at 0.2 s, so it cannot become the reason the suite is skipped | 19-12 plan author | 2026-08-29 |

**Not accepted here.** `T-19-60` and `T-19-81` are high-severity, empirically
confirmed bypasses — of layers 1, 2 and 3 and the SAFE-06 cap, and of the D-16
ssh isolation respectively. Accepting either is a human decision and this audit
does not make it.

`T-19-61` through `T-19-73` were **deliberately left unremediated by an explicit
user scoping decision** after audit 1. They remain open below the `high`
threshold and therefore non-blocking, but **none has been accepted** — they are
unremediated findings awaiting disposition, carried forward at their original
severities. Audit 2 confirms none of them is a blocker on its own merits.

`T-19-82` through `T-19-85` are new, open below `high`, and likewise unaccepted.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Open at `high` | Run By |
|------------|---------------|--------|------|---|--------|
| 2026-08-29 (audit 1) | 83 | 69 | 14 | 1 (`T-19-60`) | `/gsd-secure-phase 19` — three `gsd-security-auditor` subagents (opus), orchestrator-verified |
| 2026-08-29 (audit 2) | 97 | 78 | 19 | 2 (`T-19-60`, `T-19-81`) | `/gsd-secure-phase 19` re-run after 19-11/19-12 — one `gsd-security-auditor` subagent (opus), every blocking claim reproduced independently by the orchestrator |

### Audit 2 method (re-audit after plans 19-11 and 19-12)

**State A** (prior SECURITY.md, `status: blocked`, `threats_open: 1`),
`register_authored_at_plan_time: true`, ASVS L1, `block_on: high`, `ISOLATION=none`
on the main checkout at `b8605ef`. The mandate was to verify the T-19-60
remediation rather than accept it.

**What was re-derived rather than read.** Every refusal and permit quoted in this
document above was measured by the orchestrator against the built binary
(`./target/debug/gsd-meta-manager envelope guard alpha`, fresh `mktemp -d`
envelope root per row, `PreToolUse` JSON on stdin), not taken from
`19-11-SUMMARY.md` or `19-12-SUMMARY.md`. The three legs of the surviving
`T-19-60` bypass were each confirmed separately: layer-2 exit 0, git's own
`-c` > `GIT_CONFIG_*` precedence measured with a real `git config --get`, and the
absent PR-ledger file after `env -u gh gh pr create`. `T-19-81` was isolated from
`T-19-60`'s decoy mechanism by measuring the pure form (`env -u $K git push
--force`, exit 2) alongside it.

**Gates observed.** `rtk proxy cargo test --no-fail-fast` redirected to a file
(so the RTK filter could not elide the middle of the output): **37 suites, 1472
passed, 0 failed, 13 ignored**, including `envelope_wrapper_bypass` (13/13) and
`envelope_wrapper_class` (11/11). `rtk proxy cargo clippy -- -D warnings` exit 0,
zero warnings. `T-19-SC` still holds: `Cargo.toml` and `Cargo.lock` are untouched
by 19-11 and 19-12.

> **Tooling note for the next auditor.** Plain `cargo test` fail-fasts at
> `tests/driver_reattach.rs`, and `envelope_*` sorts *after* `driver_*`, so an
> unqualified run skips every envelope suite while reporting a count that looks
> like baseline. `--no-fail-fast` is mandatory. Separately, the RTK hook both
> strips `warning:`/`test result:` lines *and* elides the middle of long output —
> redirect to a file and count there, or the suite total is unknowable.

**Regression check.** `src/envelope/policy.rs` is additions-only (641/0), so
`T-19-08` … `T-19-14` are byte-identical; each named evidence test was re-run
individually and passed. `src/envelope/hooks.rs` hunks are confined to
`classify_segments` and two doc regions — `deny`, `write_settings_in`,
`refuse_paths`, the hook stubs and the ledger paths are untouched, so `T-19-30`,
`T-19-32`, `T-19-34`, `T-19-36` and `T-19-56` are unchanged. `T-19-33`'s
`GUARD_TIMEOUT_SECS = 5` and no-network scan are intact. One deliberate
behavioural narrowing, plan-sanctioned and not a registered-threat regression: an
unsplittable `-c` payload is now refused only when
`policy::mentions_governed_program(payload)` holds (`hooks.rs:1030-1039`), rather
than unconditionally; the outer `split_segments` still denies the common
unbalanced-quote case. **Effective-coverage caveat:** `T-19-08` and `T-19-09`
remain closed as unit-level classifiers but are reachable *around* at the guard
entry point (`env -u git git push --force-with-lease origin main` → exit 0) —
that is `T-19-60`'s surviving mechanism, not a weakening of their own code.

**ID allocation.** `19-12-PLAN.md`'s threat model already allocated `T-19-76` …
`T-19-80`, so audit-2 findings are numbered from `T-19-81` to avoid collision.

### Audit 1 method

**State B (no prior SECURITY.md).** The register was parsed from the
`<threat_model>` blocks in all ten `19-NN-PLAN.md` files and cross-checked against
the `## Threat Flags` sections of the summaries. Three auditors verified
disjoint slices (T-19-01..22, T-19-23..37, T-19-38..59 plus 09-* and 10-*) against
the implementation, running test targets with raw output via `rtk proxy` to defeat
the RTK output filter, and using `strace` where test vacuity was plausible. The
blocking finding was reproduced independently by the orchestrator against the
built binary before being recorded.

**Gates observed during the audit:** full suite green (0 failed);
`cargo clippy -- -D warnings` exit 0; `--all-targets` shows 4 pre-existing lints,
no growth over the 5 recorded at 19-08. `T-19-SC` holds phase-wide: no crate was
added to `Cargo.toml` by any phase-19 commit.

**Process gap noted:** `19-09-SUMMARY.md` and `19-10-SUMMARY.md` carry no
`## Threat Flags` section, unlike the other eight. The underlying commits were
reviewed directly — 19-09 touched only `src/envelope/advisory.rs`, 19-10 only
`tests/driver_lock.rs` — so no new attack surface went undeclared, but the
declaration is missing.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [ ] `threats_open: 0` confirmed — **2 open at `high`: `T-19-60`, `T-19-81`**
- [ ] `status: verified` set in frontmatter

**Approval:** blocked 2026-08-29 (audit 2) — close `T-19-60` and `T-19-81`, then
re-run `/gsd-secure-phase 19`.

**What closing them requires** (both are one-function changes; neither expands
phase scope):

1. **`T-19-60`** — `classify_git` refuses rather than `Allow`s an argv whose verb
   is itself `git`/`gh`/`glab`; and/or `resolve_program` establishes command
   position before returning `Governed`. Add `env -u git`-shaped operands to the
   19-12 wrapper alphabet (`T-19-83`) so the corpus can fail on this class.
2. **`T-19-81`** — refuse a leading assignment whose **value** is covered by
   `envelope_env_key`, symmetric to step 7's existing `GOVERNED_PROGRAMS` check.

`T-19-82` (add the two removal keys, drop the drift pin's `value.is_some()` and
`GIT_`/`GH_` filters) is cheap and closes the enabling half of `T-19-81`, but is
non-blocking on its own.
