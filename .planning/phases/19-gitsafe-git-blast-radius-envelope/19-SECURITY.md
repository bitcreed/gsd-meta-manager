---
phase: 19
slug: gitsafe-git-blast-radius-envelope
status: blocked
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (high)
threats_open: 3
asvs_level: 1
block_on: high
created: 2026-08-29
updated: 2026-08-29
register_authored_at_plan_time: true
audited_against: HEAD (228e4bc) — the third audit judges the tree after plan
  19-13. Audit 2 judged b8605ef (after 19-11/19-12); audit 1 judged 0ec1fcb.
---

# Phase 19 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Phase 19 built a **git blast radius envelope**: layered controls so a driven agent
cannot force-push, bypass git hooks, reach ambient credentials, or exceed a
per-project pull-request cap. The register below was authored at plan time across
all ten plans (`register_authored_at_plan_time: true`); this audit verifies that
each declared mitigation exists in the implementation and that the test named as
its evidence actually asserts the property.

**Verdict (audit 3, 2026-08-29): OPEN_THREATS.** Three high-severity threats are
open and blocking — `T-19-86` (registered by plan 19-13, still permitted),
`T-19-87` (found in 19-13's *execution*, and **wider than it was registered** —
see audit 3 below), and `T-19-88` (found by this audit). Plan 19-13 genuinely
closed `T-19-60`'s wrapper-operand sub-class, `T-19-81`, `T-19-82` and `T-19-83`,
each re-measured here against the built binary rather than accepted from the
summary. One hundred and one threats total, eighty-two closed, nineteen open
(three at or above the `high` block threshold).

> **The pattern this phase keeps producing, stated once at the top.** Three
> rounds, three corpora, three times the gap has been the cell **one slot over**
> from what the corpus was built to vary. Round 1's wrapper alphabet could not
> draw a wrapper whose operand was a governed name (`T-19-60`). Round 2's decoy
> alphabet closed that and left the *governed program's own operand*
> (`T-19-86`). Round 3's alphabets — `ASSIGNMENT_PREFIXES`, `WRAPPERS`,
> `SHELL_LAYERS`, `DECOY_OPERANDS`, `REFUSED_BASES` — contain **no `$` and no
> `{`/`(` in any position**, so they are structurally incapable of failing on an
> expansion or a brace anywhere in the command. That is where audit 3's two
> blocking findings are: `T-19-88` in the governed program's own **verb** slot,
> and `T-19-87` in the splitter that decides what a segment even is.

**Verdict (audit 2, 2026-08-29): OPEN_THREATS.** Two high-severity threats were
open and blocking — `T-19-60`, **narrowed but not closed** by plans 19-11/19-12,
and `T-19-81`, found during that audit. Both are closed as of audit 3.

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
| Found by audit 1 (`T-19-60`, `T-19-61` … `T-19-73`) | 14 | 1 | 13 (0 at `high`) |
| Found by audit 2 (`T-19-81` … `T-19-85`) | 5 | 3 | 2 (0 at `high`) |
| Registered by plan 19-13 (`T-19-86`) | 1 | 0 | 1 (1 at `high`) |
| Found in plan 19-13's execution (`T-19-87`) | 1 | 0 | 1 (1 at `high`) |
| Found by audit 3 (`T-19-88` … `T-19-90`) | 3 | 0 | 3 (1 at `high`) |
| **Total** | **101** | **82** | **19 (3 at `high`)** |

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
| **T-19-60** | **Elevation of Privilege** | **`resolve_program` command-position selection** | **high** | **mitigate (partial)** | **Audit 1:** the guard classified on `words[0]` alone, so the wrapper family (`env`, `timeout`, `nohup`, `command`, …) and a bare `NAME=VALUE` prefix fell through to a permit. **Audit 2: narrowed, NOT closed.** `policy::resolve_program` replaced `words[0]` and every one of the six measured lines is now refused — but step 5 (`policy.rs:1433-1449`) returns `Governed { index }` for the first token whose basename is governed **without establishing that the token is in a command position**. A wrapper's own *option operand* spelled `git`/`gh`/`glab` captures the index; the real program then becomes `argv[0]` of the classified argv, `classify_git` reads it as the **verb**, `git` is not in the denylist, and the answer is `Allow`. Measured: `env -u git git push --force origin main` → **exit 0**. **Audit 3: CLOSED for the mechanism this row describes.** Plan 19-13 gave step 5 a command-position rule — a governed candidate at the head answers immediately, two or more behind a wrapper prefix are `Refuse(envelope_assertion_failed)` rather than mis-indexed (`policy.rs:1563-1659`). All six lines audit 2 measured at exit 0 were re-measured by audit 3 against the built binary at `228e4bc`, one fresh root per row: `env -u git git push --force origin main` → 2, `env -u git git -c core.hooksPath=/dev/null push --force …` → 2, `env -u gh gh pr create --title x` → 2, `sudo -u git git push --force …` → 2, `env -u git git stash` → 2. The discriminator `env -u SOME_VAR git push --force …` stays 2 and the head shortcut does **not** over-refuse: `git commit -m "git push --force is now blocked"` → 0 and `gh pr create --title "stop git push --force"` → 0 **with a ledger line written**. The successor sub-class is **`T-19-86`**, counted separately so this row is not double-counted. | **closed (wrapper-operand sub-class; successor `T-19-86` open)** |
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
| **T-19-81** | **Tampering / Elevation of Privilege** | **`resolve_program` step 1 — literal-token matching of `ENVELOPE_ENV_KEYS`** | **high** | **mitigate (unimplemented)** | **The envelope-key refusal matches only a token that literally *is* the key or assigns to it. The name can be bound to a shell variable in a prior segment of the *same* Bash tool call and then removed indirectly, which the guard does not see. Measured: `K=GIT_SSH_COMMAND; SSH_AUTH_SOCK=/tmp/evil env -u $K git fetch origin` → **exit 0**. That single tool call removes the `GIT_SSH_COMMAND` braces *and* restores the `SSH_AUTH_SOCK` belt, putting the user's own ssh-agent and default identities back inside a driven run — the escape D-16 and `T-19-23` (high) exist to prevent. Nothing checks an assignment's **value** against `ENVELOPE_ENV_KEYS`, although step 7 already performs exactly that check against `GOVERNED_PROGRAMS`. Independent of `T-19-60`: the pure form `env -u $K git push --force origin main` is still refused, so this is not the decoy mechanism.** **Audit 3: CLOSED.** `resolve_program` step 2b (`policy.rs:1504-1536`) checks a leading assignment's VALUE against `envelope_env_key` **before** the `NoProgram` return — the placement the measured line required, and the summary's stated reason for it is correct. Re-measured: `K=GIT_SSH_COMMAND; SSH_AUTH_SOCK=/tmp/evil env -u $K git fetch origin` → **exit 2 `hook_bypass_blocked`**; so are the class-level spellings `env -u $(echo GIT_SSH_COMMAND) …`, `A=GIT_ASKPASS; env -u ${A} …` and `S=SSH_AUTH_SOCK; env ${S}=/tmp/evil git fetch origin`. **The residual is `T-19-87`, not this row:** the same harm survives where the bound name is not itself a key — `C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin` → **exit 0**, which removes `GIT_CONFIG_COUNT`. | **closed (`T-19-87` carries the residual)** |
| T-19-82 | Information Disclosure | `ENVELOPE_ENV_KEYS` vs. the envelope's **removal** entries | medium | mitigate | `SSH_AUTH_SOCK` and `SSH_AGENT_PID` are removed by `cred::build_env_in` (`cred.rs:402-403`) but absent from `ENVELOPE_ENV_KEYS`, and the drift pin's `filter(\|(_, value)\| value.is_some())` guarantees it will never notice — the pin would not have caught `GIT_SSH_COMMAND` either, had that key been a removal. Measured: `SSH_AUTH_SOCK=/tmp/evil git fetch origin` → exit 0. **Inert on its own** while `GIT_SSH_COMMAND` holds (`IdentityAgent=none`, `IdentitiesOnly=yes`, `-F /dev/null`); live in combination with `T-19-81`. The pin's second filter, `starts_with("GIT_") \|\| starts_with("GH_")`, additionally excludes a future `GITHUB_*`, `GLAB_*` (a governed program) or `SSH_*` key. **Audit 3: CLOSED.** `SSH_AUTH_SOCK`, `SSH_AGENT_PID` and `GSD_MM_ENVELOPE_PROJECT_ROOT` are in `ENVELOPE_ENV_KEYS` (`policy.rs:1246-1259`) and **both** drift-pin filters are deleted, replaced by four floors that each name the filter they stand in for — including `removals.len() >= 2` and a floor requiring a COVERED key beginning with neither `GIT_` nor `GH_` (`policy.rs:3126-3210`). Re-measured at `228e4bc`: `SSH_AUTH_SOCK=/tmp/evil git fetch origin` → 2, `SSH_AGENT_PID=1 git fetch origin` → 2, `GSD_MM_ENVELOPE_PROJECT_ROOT=/tmp/x git fetch origin` → 2. **Residual: `T-19-90`** — `GSD_MM_RUN_ID` is appended by `cred::with_run_id` *after* `build_env_in` returns, so it reaches the child but is invisible to a pin whose source is one `build_env_in` call. | closed (`T-19-90` carries the residual) |
| T-19-83 | Spoofing | `tests/envelope_wrapper_class.rs` `WRAPPERS` alphabet | medium | mitigate | The generative class-level corpus contains wrappers with operands (`env -u SOME_VAR`, `runuser -u me --`) but **no operand whose basename is a governed program**, so it is structurally incapable of failing on the surviving `T-19-60` shape. The discriminating pair, measured: `env -u SOME_VAR git push --force origin main` → exit 2, `env -u git git push --force origin main` → exit 0. This is `T-19-76`'s own failure mode one level up — the corpus certifying the fix cannot fail on the class it certifies. **Audit 3: CLOSED as scoped.** `DECOY_OPERANDS` (`tests/envelope_wrapper_class.rs:1134-1162`) carries **ten** governed operands against a floor of eight — including `env -u /usr/bin/git` for basename normalisation on the decoy, `sudo -u gh --` for the non-adjacent case and `made-up-wrapper-9000 --flag git` for the unnameable wrapper — spliced at a drawn index `0..=depth` with a `wrapped_on_both_sides` floor and 720 generated cases; 19-13's RED run shows the property failing before the fix. **The corpus's NEW blind spot is `T-19-89`.** | closed (`T-19-89` carries the successor) |
| T-19-84 | Repudiation | `T-19-74`'s disclosure in `resolve_program`'s doc | low | mitigate | The doc (`policy.rs:1373-1381`) claims the residual requires `$X` "bound outside this command line" and that "each Bash tool call being its own shell process is what keeps it narrow". Both are false: `env $(echo git) push --force origin main` → exit 0 with no prior binding and no outside state, and `K=…; env -u $K …` binds inside the same tool call in a segment the guard can see. `T-19-79`'s doc-disclosure pin enforces that the paragraph *exists*, not that it is *true*. | open — below `high` (non-blocking) |
| T-19-85 | Repudiation | `T-19-75`'s park reason | low | mitigate | A benign `rg "git push --force" src/` is refused **and parked under `force_push_blocked`**, so the run journal records a force-push refusal for a source search. The accept rationale discloses the refusal but not the misdescribing evidence record (D-24/D-25). | open — below `high` (non-blocking) |

*Status: open · closed · open — below `high` threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` count toward `threats_open`*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

### Registered by plan 19-13, NOT an audit finding

**Provenance, stated first because it is the point of keeping this separate.**
The row below was **not** measured by audit 1 or audit 2. It was found while
*planning* the round that closes `T-19-60`'s wrapper-operand sub-class, and it is
recorded here by plan `19-13` so the boundary of what that round closed is
bounded on both sides. The audit tables above are the audit's own provenance and
this subsection is deliberately outside them. `T-19-86` is **registered and
pinned, not fixed**: a plan cannot both discover a threat and be the plan that
measured it fail first.

| Threat ID | Category | Component | Severity | Disposition | Finding | Status |
|---|---|---|---|---|---|---|
| T-19-86 | Elevation of Privilege | `classify_git`'s denylist default arm, reached through a **governed program's own operand** | high | mitigate (unimplemented — registered, pinned, deferred) | A GOVERNED program handed a governed command as data runs it itself. These resolve at the **head** — correctly, because the head *is* the command position, and the head shortcut is what keeps `git commit -m "git push --force is now blocked"` working — and are then permitted by the denylist's default arm, whose verbs here are `submodule`, `rebase`, `bisect` and `p`. Measured against the tree at `a41e431`, one fresh tempdir per row: `git submodule foreach git push --force origin main` → **exit 0**; `git rebase -x "git push --force origin main" HEAD~3` → **exit 0**; `git bisect run sh -c "git push --force origin main"` → **exit 0**; `git -c alias.p='!git push --force origin main' p` → **exit 0**. Named nowhere in the phase before this registration. Re-measured after 19-13's command-position rule: all four still **exit 0**, by design. | OPEN — BLOCKING, deferred to a later round |

**Consequence for the phase gate, stated rather than left to be inferred:**
19-13 closes the **wrapper-operand** sub-class of `T-19-60` — a token that is not
the effective program capturing the resolver's index because it is spelled
`git`/`gh`/`glab` in a **wrapper's** operand slot — and closes `T-19-81`,
`T-19-82` and `T-19-83`. It does **not** close `T-19-86`, so
**`/gsd-secure-phase 19` is not cleared by 19-13 alone.**

**Where the boundary is pinned**, so a future change that moves it must delete
the rows deliberately rather than discover them failing:

- `tests/envelope_command_position.rs` →
  `the_t_19_86_residual_is_permitted_today_and_this_plan_leaves_it_permitted`
  (all four spellings, asserted PERMITTED).
- `src/envelope/policy.rs` → `resolve_program`'s doc, third bullet of "the three
  shapes this does NOT cover", beside `T-19-74` and `T-19-75`.
- `.planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md`.

---

### Threats found by audit 3 (2026-08-29, after plan 19-13)

**Provenance, stated first.** The table above this one was written by *plan*
19-13 and is deliberately outside the audit tables. The table below is **audit
3's own**, measured against the built binary at `228e4bc` with one fresh
`mktemp -d` envelope root per row. One row — `T-19-87` — was **found by the
executor during plan 19-13**, not by any audit; it is carried here at that
provenance, with only its *severity and blast radius* assessed by audit 3, and
that assessment is what moves it from a `deferred-items.md` note to a blocking
row.

| Threat ID | Category | Component | Severity | Disposition | Finding | Status |
|---|---|---|---|---|---|---|
| **T-19-87** | **Tampering / Elevation of Privilege** | **`policy::tokenize` — `{`, `}`, `(`, `)` are `SEPARATORS` (`policy.rs:968`, `:1067`)** | **high** | **mitigate (unimplemented — found in execution, deferred by 19-13)** | **Found by the executor of plan 19-13, whose plan and plan-check had both asserted the shape was closed by the expansion-prefix rule. The tokenizer flushes the current word at `{`/`}`/`(`/`)` and emits an operator, so `${K}_COMMAND` and `$(…)` are not words — they are *segment boundaries*. The guard then judges fragments the shell never runs. Registered in `deferred-items.md` scoped to "the `T-19-81` class", and pinned in `the_brace_expansion_spelling_is_a_residual_this_plan_does_not_close` with the bound *"the fragmentation does not hide a refused git command, because the segment that carries the command still resolves it."* **Audit 3: that bound is false one word to the right.** Measured at `228e4bc`: `git ${X}push --force origin main` → **exit 0** (no binding of any kind required; bash runs `git push --force origin main`, verified against a shim on `PATH`); `git ${X}stash` → **0**; `git ${X}update-ref -d refs/heads/main` → **0**; `git ${X}config core.hooksPath /tmp/x` → **0**; `gh ${X}pr create --title x` → **0 with NO ledger line and no park** — and the PR cap has no second carrier (`hooks.rs:1263-1270`, `T-19-35`). `$( )` is the same mechanism: `git $(true)push --force origin main` → **0**. **The three-layer line:** `C=GIT_CONFIG; env -u ${C}_COUNT git ${X}push --force origin main` → **exit 0**, and in bash it is `env -u GIT_CONFIG_COUNT git push --force origin main` — layer 1 does not match the `Bash(git push:*)` prefix, layer 2 permits, layer 3's `core.hooksPath` carrier is gone, and `GIT_ASKPASS`/`GIT_CONFIG_GLOBAL`/`GIT_SSH_COMMAND` are untouched so the push authenticates. That is the identical three-leg argument audit 2 used to rate `T-19-60` high and blocking, reproduced through the splitter.** | **OPEN — BLOCKING** |
| **T-19-88** | **Elevation of Privilege** | **the VERB slot of a governed program resolved at the head — `hooks.rs:960` / `:1047-1090`** | **high** | **mitigate (unimplemented)** | **`classify_segments` collapses each `Token` to its `text` at `hooks.rs:960` before calling `classify_git(&words[index+1..])` and `pr_command_label(&words[index..])`, so `Token.expansion` is **structurally unavailable** to every classifier. `resolve_program` refuses an expansion at the head (step 3) and an expansion in the wrapper prefix (step 5), but the word **one slot to the right of the head** — which is the verb every classifier decision turns on — is exempt by an explicit code comment ("an expansion in the program's own ARGUMENTS … untouched", `policy.rs:1634-1637`). Measured, no braces and no parentheses needed: `V=push; git $V --force origin main` → **exit 0**; `V=stash; git $V` → **0**; `V=update-ref; git $V -d refs/heads/main` → **0**; `P=pr; gh $P create --title x` → **0 with no ledger line**; `` gh `true`pr create --title x `` → **0 with no ledger line**. `classify_git` reaches its denylist default arm because the verb is `$V`; `pr_command_label` matches no arm, so the SAFE-06 cap is not merely exceeded — the attempt is never counted, and `git stash`, `git update-ref` and the PR cap have **no `pre-push`/`pre-commit` second carrier**. Distinct from `T-19-74`/`AR-19-10`, which is an expansion-assembled **program** behind a **wrapper**; here the program is spelled literally and resolution is correct. **The mitigation named for the neighbouring cells cannot be written where it is needed without changing the classifier boundary**, because the expansion bit is discarded at `hooks.rs:960`. The unit test whose name claims this class, `a_verb_assembled_by_expansion_and_an_eval_are_both_denied` (`hooks.rs:1917`), actually exercises `$TOOL push --force` — an expansion-assembled *program*, i.e. step 3. Its name asserts a class it does not reach.** | **OPEN — BLOCKING** |
| T-19-89 | Spoofing | the 19-12/19-13 generative alphabets | medium | mitigate | `T-19-76`'s failure mode for the third consecutive round, in the cell adjacent to the one 19-13 filled. **No entry of `ASSIGNMENT_PREFIXES`, `WRAPPERS`, `SHELL_LAYERS`, `DECOY_OPERANDS` or `REFUSED_BASES` contains a `$`, a `` ` ``, a `{` or a `(`** — verified by reading all five alphabets. The corpus therefore cannot generate, and so cannot fail on: an expansion in a base's verb or argument (`T-19-88`), an expansion in a wrapper or decoy operand (the step-5 prefix rule), an assignment prefix whose VALUE is an envelope key or a governed program (steps 2b and 7), or any brace/paren grouping (`T-19-87`). Each of those rules is pinned only by enumerated rows in `tests/envelope_command_position.rs`, so the *generative* half of the evidence is silent on every one of them. `SHELL_LAYERS` in particular offers `sh -c '…'` and `bash -lc "…"` but never `{ …; }` or `( … )`, which are the two shapes that break the splitter. | open — below `high` (non-blocking) |
| T-19-90 | Repudiation | `ENVELOPE_ENV_KEYS` vs. `cred::with_run_id` | low | mitigate | The widened drift pin iterates `build_env_in(...).entries()`. `GSD_MM_RUN_ID` is appended afterwards by `EnvelopeEnv::with_run_id` (`cred.rs:175-192`) and is therefore carried to the driven child while being **outside the pin's source**, so no floor can see it. It is also absent from `ENVELOPE_ENV_KEYS`, although its sibling locator `GSD_MM_ENVELOPE_PROJECT_ROOT` was added by 19-13 with the reasoning "this entry protects the EVIDENCE rather than the containment" — reasoning that applies identically here. Measured: `env -u GSD_MM_RUN_ID git fetch origin` → **exit 0**, and `hooks.rs:1161` then attributes the park to `"unattributed-run"`. Same class as `T-19-62`/`T-19-70`: evidence, not containment. | open — below `high` (non-blocking) |

*`T-19-89` and `T-19-90` are open below the `high` threshold and do **not** count
toward `threats_open`. `T-19-87` and `T-19-88` do.*

### Execution record — plan 19-14 (Rule A). NOT an audit finding.

**Provenance, stated first so a reader cannot mistake this for audit 3's own
work.** Everything below was measured by the EXECUTOR of plan 19-14, against the
tree at `c595141` (before) and after the Rule A fix. It is recorded here because
audit 3's suggested closure names the rows and a later reader needs to find their
post-fix state beside the finding — not because these rows have been
re-audited. **Re-measuring and re-classifying them is `/gsd-secure-phase 19`'s
job, not a plan's.** The audit tables above, the Security Audit Trail, the
Accepted Risks Log and the Sign-Off are untouched by plan 19-14.

#### The audit-3 rows Rule A closes

Driven in-process through `hooks::guard_in`, one fresh `TempDir` envelope root
per row, with the envelope directory WALKED afterwards so a missing ledger line
is observed rather than assumed. Pinned in
`tests/envelope_expansion_slots.rs`.

```
before  after                                 command
exit=0  exit=2 [envelope_assertion_failed]    V=push; git $V --force origin main
exit=0  exit=2 [envelope_assertion_failed]    V=stash; git $V
exit=0  exit=2 [envelope_assertion_failed]    V=update-ref; git $V -d refs/heads/main
exit=0  exit=2 [envelope_assertion_failed]    P=pr; gh $P create --title x        (walk: no ledger line)
exit=0  exit=2 [envelope_assertion_failed]    gh `true`pr create --title x        (walk: no ledger line)
exit=0  exit=2 [envelope_assertion_failed]    git ${X}push --force origin main
exit=0  exit=2 [envelope_assertion_failed]    git $(true)push --force origin main
exit=0  exit=2 [envelope_assertion_failed]    git ${X}stash
exit=0  exit=2 [envelope_assertion_failed]    git ${X}update-ref -d refs/heads/main
exit=0  exit=2 [envelope_assertion_failed]    git ${X}config core.hooksPath /tmp/x
exit=0  exit=2 [envelope_assertion_failed]    gh ${X}pr create --title x          (walk: no ledger line)
exit=0  exit=2 [envelope_assertion_failed]    C=GIT_CONFIG; env -u ${C}_COUNT git ${X}push --force origin main
exit=0  exit=2 [hook_bypass_blocked]          env -u GSD_MM_RUN_ID git fetch origin
```

`T-19-88` is closed. `T-19-90` is closed both by the key and by re-sourcing the
drift pin through `cred::EnvelopeEnv::with_run_id`, the seam that appends it.
**`T-19-87` is NOT closed** — this is six of its eight measured rows, and
`SEPARATORS`, `split_segments` and the tokenizer's separator arm are untouched.
**`T-19-89` is NOT closed** — plan 19-14 delivers the Rule A half of the
alphabet widening; `SHELL_LAYERS` still cannot draw `{ …; }` or `( … )`.

The mechanism, for the record: `hooks.rs` collapsed each `Token` to its `text`
before either classifier ran, so `Token.expansion` was structurally unavailable
to the words each matched arm DECIDES ON. Rule A restores the bit at that
boundary — `policy::expansion_in_decision_region`, called once, before both
classifiers and before the ledger write — over a region that is exactly those
words: the git verb, `config`'s key operand, a forge's first two subcommand
words, and the `gh api` endpoint, method and flag-ness words. Operands stay
free, so `git commit -m "$MSG"`, `gh pr create --title "$TITLE"`,
`gh api repos/o/r/pulls -f title="$T"` and `git -c user.name="$NAME" commit -m x`
all still exit 0 and are pinned doing so.

#### Region cells found while CHECKING plan 19-14

Not measured by any audit. Same method, same provenance caveat. Each was at
exit 0 before the fix and is at `exit=2 [envelope_assertion_failed]` after it.

| Cell | Command | Why a region derived from the wrong scan missed it |
|---|---|---|
| the forge's SECOND subcommand word | `P=create; gh pr $P --title x` | `pr_command_label` matches on TWO words, so a one-word region leaves this unmatched, uncounted and unparked — no ledger line, SAFE-06 bypassed rather than exceeded |
| the `-`-initial `api` flag | `F=f; gh api repos/o/r/pulls -$F title=x`, and `-${F}` | begins with `-`, so it is neither one of the first two subcommand words, nor the method value, nor marker-initial — a clause written only for `$F` leaves it in no part of the region |
| the DISPLACED `api` endpoint | `E=pulls; gh api -f title=x repos/o/r/$E`, and the `-H accept:x` spelling | the two forge scans disagree about which words are flags: `subcommand_words` skips only `FORGE_VALUE_OPTS`, so an option value only the `api` scan skips pushes the endpoint to the THIRD word and out of the region entirely, while the arm's own scan reads it and `endpoint_is_pulls` compares `$E` |
| `classify_config`'s key operand | `git config $K /tmp/x`, `git config ${K} /tmp/x`, `git config set $K /tmp/x` | `classify_config` reaches `is_hooks_path_key` on an operand it cannot read and answers `Allow`, so layer 3 is disarmed exactly as by the literal spelling |
| the `git -c` key half | `git -c $K commit -m x`, `git -c ${K}=/tmp/x commit -m x` | `scan_leading`'s only decision is whether `core.hooksPath` is set at command-line precedence, and it makes it by comparing the KEY half |

The fix takes every index from the scan the classifier itself runs — three
extracted index primitives, including one over `gh_api_posts_a_pull_request`'s
own walk — because a region computed by a SECOND scan is the defect this round
is about, and it had by then sat one slot over four times.

#### Carried forward RED to plan 19-15

Two of `T-19-87`'s eight measured rows are written in
`tests/envelope_expansion_slots.rs`, observed RED, and **left RED** at plan
19-14's end:

```
exit=0  C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin
exit=0  K=GIT_SSH;    env -u ${K}_COMMAND git fetch origin
```

The expansion lands in a wrapper operand SEVERED from the governed program —
`env -u $` | `C` | `_COUNT git fetch origin` — so the last fragment resolves
`git` behind a clean one-word prefix whose verb is the literal `fetch`. No
verb-slot rule can reach them; `19-15` closes them positionally, with
`SEPARATORS` still untouched. They are left failing so that each rule is shown
separately load-bearing rather than one being certified by the other's evidence.
Test names:
`the_severed_git_config_count_row_is_carried_forward_to_19_15_and_is_expected_red`
and `the_severed_git_ssh_command_row_is_carried_forward_to_19_15_and_is_expected_red`.

#### `T-19-91` — registered OPEN, not closed

A git classifier's own DECISION OPERAND, assembled by expansion. Measured at
`c595141`, one fresh envelope root per row, and pinned in
`tests/envelope_expansion_slots.rs::the_t_19_91_residual_is_measured_and_pinned_rather_than_closed`:

```
exit=0  git reflog $S            <- PERMITTED, registered OPEN
exit=0  git reflog show $S       <- PERMITTED, registered OPEN
exit=0  git symbolic-ref $S      <- PERMITTED, registered OPEN
exit=2  git symbolic-ref HEAD $R    [force_push_blocked]        <- already fails closed
exit=2  git push origin $REF       [push_outside_namespace]     <- already fails closed
exit=2  git push $REF              [push_outside_namespace]     <- already fails closed; NOT pinned as a
                                                                  live row, because it is the one shape
                                                                  `push_needs_resolved_dests` answers
                                                                  `true` for, which makes the guard shell
                                                                  out to git in the test's own working
                                                                  directory (`T-19-80`)
```

`classify_reflog` matches its first non-flag token against `delete`, `expire`
and `drop`; `classify_symbolic_ref` counts operands and looks for `-d`. An
operand neither can read falls to an `Allow` arm — structurally identical to the
`config` cell.

**Why `config` is closed here and these are not: ROUND DISCIPLINE and
provenance, and NOT a second-carrier argument.** `git ${X}config core.hooksPath
/tmp/x` is a row in audit 3's own measured bypass list, so closing that cell is
part of making the region principle coherent over rows the audit established.
These were found while checking plan 19-14, and a plan cannot both discover a
threat and be the plan that measured it fail first.

**`reflog` and `symbolic-ref` have NO `pre-push` and NO `pre-commit` second
carrier.** Git runs no hook for either, and `classify_reflog`'s own refusal text
records that the reflog is *the recovery path for every other destructive git
operation*. Only `git push` has a hook behind it, and its refspec operand
already fails closed. A reader who took the asymmetry above for a blast-radius
judgement would be reading a narrowed threat as a covered one.

Severity: **high**, disposition **mitigate (partial — `config` closed; the rest
registered open)**. Registered in `deferred-items.md` and disclosed in
`resolve_program`'s own doc as its fourth residual bullet.

#### Consequence for the phase gate

**`/gsd-secure-phase 19` is NOT cleared by plan 19-14, and will not be cleared
by 19-15 either.** `T-19-86` remains OPEN at `high` by explicit user scoping
decision — its four rows were re-measured after Rule A and all four still exit 0
— and `T-19-91` is registered open at `high`. `T-19-87` and `T-19-89` are
partial. `T-19-61` through `T-19-73`, `T-19-84` and `T-19-85` are open and
unaccepted.

---

## Audit 3 — what the round-2 controls can and cannot fail on

The orchestrator's mandate for this audit was not "do the claimed mitigations
exist" but **"what class can the new controls not fail on?"** — because twice
before, a control was certified by a corpus structurally incapable of failing on
its own class. The answer is recorded as a method rather than a list, so the next
round can repeat it.

### The axes the round-3 corpus varies, and the cell beside each

| Axis | Alphabet | Values | The adjacent cell it cannot draw |
|---|---|---|---|
| assignment prefix | `ASSIGNMENT_PREFIXES` | `""`, `FOO=bar `, `LC_ALL=C TZ=UTC `, `EMPTY= ` | a value that is an envelope key (step 2b) or a governed program (step 7); any `$` |
| wrapper chain | `WRAPPERS` (30, depth 0–3) | all literal program words | a wrapper carrying an expansion — the step-5 prefix rule |
| decoy operand | `DECOY_OPERANDS` (10) | all literal governed names | a decoy carrying an expansion; a governed program's **own** operand (`T-19-86`) |
| outer shell | `SHELL_LAYERS` | none, `sh -c '…'`, `bash -lc "…"` | `{ …; }` and `( … )` — the two shapes that fragment the splitter (`T-19-87`) |
| base command | `REFUSED_BASES` (12) | all literal `git <verb> …` | an expansion in the base's **verb** slot (`T-19-88`) |

Two of those five adjacent cells are live bypasses at `228e4bc`. That is
`T-19-89`, and it is the same finding as `T-19-76` and `T-19-83` at the next
radius out.

### What round 3 genuinely delivered — re-measured, not accepted

Every row below was driven as
`printf '{"tool_name":"Bash","tool_input":{"command":"<CMD>"}}' | GSD_MM_ENVELOPE_ROOT=<fresh tempdir> ./target/debug/gsd-meta-manager envelope guard alpha`
at `228e4bc`, one fresh tempdir per row, with the envelope directory walked
afterwards so a missing ledger line is observed rather than assumed.

```
exit=2 [envelope_assertion_failed]  env -u git git push --force origin main
exit=2 [envelope_assertion_failed]  env -u git git -c core.hooksPath=/dev/null push --force origin main
exit=2 [envelope_assertion_failed]  env -u gh gh pr create --title x
exit=2 [envelope_assertion_failed]  sudo -u git git push --force origin main
exit=2 [envelope_assertion_failed]  env -u git git stash
exit=2 [force_push_blocked]         env -u SOME_VAR git push --force origin main   <- discriminator, still refused
exit=2 [hook_bypass_blocked]        K=GIT_SSH_COMMAND; SSH_AUTH_SOCK=/tmp/evil env -u $K git fetch origin
exit=2 [hook_bypass_blocked]        SSH_AUTH_SOCK=/tmp/evil git fetch origin
exit=2 [hook_bypass_blocked]        SSH_AGENT_PID=1 git fetch origin
exit=2 [hook_bypass_blocked]        GSD_MM_ENVELOPE_PROJECT_ROOT=/tmp/x git fetch origin
exit=2 [hook_bypass_blocked]        env -u $(echo GIT_SSH_COMMAND) git fetch origin
exit=2 [hook_bypass_blocked]        S=SSH_AUTH_SOCK; env ${S}=/tmp/evil git fetch origin
exit=0  git commit -m "git push --force is now blocked"      <- head shortcut does not over-refuse
exit=0  gh pr create --title "stop git push --force"         <- and a ledger line IS written
exit=0  env -u git ls                                        <- disclosed, harmless
```

The head shortcut is therefore **discriminating rather than blanket**, the
two-candidate rule covers all six rows audit 2 measured, and the envelope-key
value check fires at the placement 19-13 claims for it. `T-19-86`'s four rows
were re-measured at `228e4bc` and all four are still **exit 0**, exactly as the
plan discloses — the pin is honest, its red set is non-empty (ten refusal
assertions in the same file, plus the anti-vacuity control
`the_unwrapped_force_push_is_refused_which_proves_this_harness_can_see_a_denial`),
and `resolve_program`'s third residual bullet describes the mechanism correctly.

### The bypasses audit 3 measured

```
exit=0  git ${X}push --force origin main                                     <- T-19-87, NO binding required
exit=0  git $(true)push --force origin main                                  <- T-19-87, paren spelling
exit=0  git ${X}stash                                                        <- T-19-87, no second carrier
exit=0  git ${X}update-ref -d refs/heads/main                                <- T-19-87, no second carrier
exit=0  git ${X}config core.hooksPath /tmp/x                                 <- T-19-87
exit=0  gh ${X}pr create --title x                        (no ledger line)   <- T-19-87, SAFE-06 bypassed
exit=0  C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin                     <- T-19-87, layer 3 carrier removed
exit=0  C=GIT_CONFIG; env -u ${C}_COUNT git ${X}push --force origin main     <- T-19-87, ALL THREE LAYERS
exit=0  V=push; git $V --force origin main                                   <- T-19-88
exit=0  V=stash; git $V                                                      <- T-19-88
exit=0  V=update-ref; git $V -d refs/heads/main                              <- T-19-88
exit=0  P=pr; gh $P create --title x                      (no ledger line)   <- T-19-88
exit=0  gh `true`pr create --title x                      (no ledger line)   <- T-19-88, backtick spelling
exit=0  env -u GSD_MM_RUN_ID git fetch origin                                <- T-19-90
```

**Shell semantics confirmed, not assumed.** Each line above was run under `bash`
with a shim named `git`/`gh` first on `PATH` that prints its own argv. `git
${X}push --force origin main` prints `ARGV: push --force origin main`;
`gh ${X}pr create --title x` prints `ARGV: pr create --title x`;
`` gh `true`pr create --title x `` prints the same. Separately,
`K=GIT_SSH; env -u ${K}_COMMAND sh -c 'echo [$GIT_SSH_COMMAND]'` prints `[]`,
so `T-19-87` does remove the key the guard permitted, and
`C=GIT_CONFIG; env -u ${C}_COUNT sh -c 'echo [$GIT_CONFIG_COUNT]'` prints `[]`.

### Suggested closure

1. **`T-19-88`** — carry the expansion bit past `hooks.rs:960` and refuse a
   governed program whose **verb** (`words[index+1]`, and the first non-flag
   subcommand word for a forge) carries an expansion. This is the exact twin of
   step 3 one word to the right, its cost is disclosed and small (`git commit -m
   "$MSG"` keeps working because the rule is the verb slot, not all arguments),
   and it also removes `T-19-87`'s most damaging spellings as a side effect.
2. **`T-19-87`** — the honest options are (a) treat a segment boundary produced
   by `{`/`}`/`(`/`)` as an **unresolvable** command rather than as N independent
   commands when any fragment mentions a governed program, or (b) refuse a
   segment whose adjacency to the previous segment was created by a brace/paren
   flush. Do **not** simply delete the characters from `SEPARATORS`: `{ cmd; }`
   and `( cmd )` are real grouping syntax and losing them re-opens the
   `echo hi && git push --force` class one level in.
3. **`T-19-89`** — before either fix is certified, add `$`-bearing and
   brace-bearing entries to `ASSIGNMENT_PREFIXES`, `WRAPPERS`, `DECOY_OPERANDS`,
   `SHELL_LAYERS` and `REFUSED_BASES`, and assert the corpus generates them
   (the `MIN_*` floor pattern already in the file). A fourth round certified by
   an alphabet with no `$` in it would be the fourth consecutive round to
   certify a claim it could not have failed on.

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

**Not accepted here.** `T-19-86`, `T-19-87` and `T-19-88` are high-severity,
empirically confirmed bypasses — of `classify_git`'s denylist arm through a
governed program's own operand, of the splitter, and of the verb slot
respectively. Two of the three defeat the SAFE-06 PR cap, which has **no second
carrier**; one of them defeats layers 1, 2 and 3 on a single line with the
credential intact. Accepting any of them is a human decision and this audit does
not make it. (`T-19-60` and `T-19-81`, audit 2's two blockers, are **closed** as
of audit 3 — re-measured, not accepted from the summary.)

`T-19-61` through `T-19-73` were **deliberately left unremediated by an explicit
user scoping decision** after audit 1. They remain open below the `high`
threshold and therefore non-blocking, but **none has been accepted** — they are
unremediated findings awaiting disposition, carried forward at their original
severities. Audit 2 confirms none of them is a blocker on its own merits.

`T-19-84` and `T-19-85` remain open below `high` and unaccepted; `T-19-82` and
`T-19-83` are closed by plan 19-13. `T-19-89` and `T-19-90`, new from audit 3,
are open below `high` and unaccepted.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Open at `high` | Run By |
|------------|---------------|--------|------|---|--------|
| 2026-08-29 (audit 1) | 83 | 69 | 14 | 1 (`T-19-60`) | `/gsd-secure-phase 19` — three `gsd-security-auditor` subagents (opus), orchestrator-verified |
| 2026-08-29 (audit 2) | 97 | 78 | 19 | 2 (`T-19-60`, `T-19-81`) | `/gsd-secure-phase 19` re-run after 19-11/19-12 — one `gsd-security-auditor` subagent (opus), every blocking claim reproduced independently by the orchestrator |
| 2026-08-29 (audit 3) | 101 | 82 | 19 | 3 (`T-19-86`, `T-19-87`, `T-19-88`) | `/gsd-secure-phase 19` re-run after 19-13 — one `gsd-security-auditor` subagent (opus), `ISOLATION=none` at `228e4bc`; every closure re-measured against the built binary and every new finding confirmed in `bash` against an argv-printing shim |

### Audit 3 method (re-audit after plan 19-13)

**State A** (prior SECURITY.md, `status: blocked`, `threats_open: 2`), ASVS L1,
`block_on: high`, `ISOLATION=none` on the main checkout at `228e4bc`. The mandate
was explicitly **not** "do the claimed mitigations exist" but "**what class can
the new controls not fail on?**" — see *Audit 3 — what the round-2 controls can
and cannot fail on* above for the axis-by-axis method and its two blocking
results.

**What was re-derived rather than read.** All twelve refusals and three permits
quoted as round 3's delivery, all fourteen bypasses, and all four `T-19-86` rows
were measured against `./target/debug/gsd-meta-manager envelope guard alpha`
with a fresh `mktemp -d` per row and the envelope directory walked afterwards, so
"no ledger line" is an observation rather than an inference. The shell semantics
of every claimed bypass were then confirmed under `bash` with a shim named
`git`/`gh` first on `PATH` printing its own argv — a guard permit is only a
finding if the shell really runs the dangerous command.

**Gates observed.** `rtk proxy cargo test --no-fail-fast` redirected to a file:
**1497 passed, 0 failed, 13 ignored**, `passed + failed = 1497`, matching
`19-13-SUMMARY.md`'s claim exactly. The documented-flaky `driver_reattach` pair
passed on this run — the baseline is bimodal, which is why the gate is read on
`passed + failed`. `envelope_command_position` and `envelope_wrapper_class` both
ran and both are green.

**Provenance discipline.** Plan 19-13's appended `T-19-86` subsection is left
byte-identical; audit 3's rows are in their own table below it. `T-19-87` is
recorded at its true provenance — **found by the executor of plan 19-13, not by
an audit** — with only its severity and blast radius assessed here.

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
- [ ] `threats_open: 0` confirmed — **3 open at `high`: `T-19-86`, `T-19-87`, `T-19-88`**
- [ ] `status: verified` set in frontmatter

**Approval:** blocked 2026-08-29 (audit 3) — close `T-19-86`, `T-19-87` and
`T-19-88`, then re-run `/gsd-secure-phase 19`.

**Progress is real and worth recording.** Audit 2's two blockers are closed and
were re-measured rather than accepted: `T-19-60`'s wrapper-operand sub-class is
genuinely gone on all six measured lines, and the head shortcut does not
over-refuse. `T-19-81`, `T-19-82` and `T-19-83` are closed on the same standard.
The open set has **shrunk in mechanism and grown in precision**: what remains is
one registered residual the plan disclosed honestly (`T-19-86`), one the executor
found and disclosed honestly but scoped too narrowly (`T-19-87`), and one the
audit found (`T-19-88`).

**What closing them requires:**

1. **`T-19-88`** — carry `Token.expansion` past `hooks.rs:960` and refuse a
   governed program whose verb carries an expansion. Smallest change, and it
   collaterally kills `T-19-87`'s worst spellings.
2. **`T-19-87`** — decide what a `{`/`}`/`(`/`)` flush means when a neighbouring
   fragment mentions a governed program. Not by deleting the characters from
   `SEPARATORS`.
3. **`T-19-86`** — `classify_git` refuses a verb whose own operand is a governed
   command line (`submodule foreach`, `rebase -x`, `bisect run`, `-c alias.*=!…`).
   The forge-side and git-side second layers plan 19-13 added are already the
   right shape for this; they simply do not cover the `-x`/`foreach`/`run`
   operand.

**And the process control, which matters as much as the three fixes.** Add
`$`-bearing and brace-bearing entries to every generative alphabet
(`T-19-89`) **before** certifying any of the above. Three rounds in a row, the
gap has been the cell one slot over from what the corpus could draw; a fourth
round certified by an alphabet containing no `$` would be the fourth consecutive
round to certify a claim it could not have failed on.

---

### Execution record — plan 19-15 (Rule B). NOT an audit finding.

**Provenance, stated first so a reader cannot mistake this for audit 3's own
work.** Everything below was measured by the EXECUTOR of plan 19-15, against the
tree at `cae2793` (19-14's post-state, "before") and after the Rule B fix. It
follows the subsection plan 19-14 appended and edits nothing that precedes it.
**Re-measuring and re-classifying these rows is `/gsd-secure-phase 19`'s job, not
a plan's.** The audit tables above, the Security Audit Trail, the Accepted Risks
Log, the Sign-Off and plan 19-14's subsection are untouched by plan 19-15.

#### The carry-forward, confirmed RED before anything was written

Plan 19-15's first action, before a single line of test or production code, was
to re-run the two rows plan 19-14 left RED and confirm they were still red
against its post-state. Both were — measured at exit 0 where exit 2 was
required:

```
exit=0  C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin    <- still RED, as 19-14 left it
exit=0  K=GIT_SSH;    env -u ${K}_COMMAND git fetch origin  <- still RED, as 19-14 left it
```

Had either been green, Rule B would have been a rule nobody had shown was
needed, and the correct action would have been to report it as a finding rather
than proceed. The red-to-green transition below is therefore evidence rather
than assertion: it is the one number a run that changed nothing could not
produce.

| Row | before (at `cae2793`) | after (Rule B) |
|---|---|---|
| `C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin` | exit 0 | **exit 2**, `envelope_assertion_failed` |
| `K=GIT_SSH; env -u ${K}_COMMAND git fetch origin` | exit 0 | **exit 2**, `envelope_assertion_failed` |

#### The remaining severed-prefix spellings

Same method: driven in-process through `hooks::guard_in`, one fresh `TempDir`
envelope root per row. Pinned in `tests/envelope_expansion_slots.rs`.

| Row | before | after |
|---|---|---|
| `env -u ${C} git fetch origin` (no literal fragment at all) | exit 0 | **exit 2**, `envelope_assertion_failed` |
| `C=GIT_CONFIG_COU; env -u ${C}NT git fetch origin` (two-character tail) | exit 0 | **exit 2**, `envelope_assertion_failed` |
| `C=GIT_CONFIG_COUN; env -u ${C}T git fetch origin` (one-character tail) | exit 0 | **exit 2**, `envelope_assertion_failed` |
| `env -u $(printf %s%s GIT_CONFIG _COUNT) git fetch origin` (substitution) | exit 0 | **exit 2**, `envelope_assertion_failed` |
| `K=GIT_SSH; env -u ${K}_COMMAND git push --force origin main` | exit 2, `force_push_blocked` | **exit 2**, `envelope_assertion_failed` |
| `ROOT=$(git rev-parse --show-toplevel) git status` — **the disclosed COST** | exit 0 | **exit 2**, `envelope_assertion_failed` |

One severed spelling is **excluded from Rule B's evidence and disclosed as
such**: `C=GIT_CONFIG_COUNT; env -u ${C} git fetch origin` was already exit 2
under `hook_bypass_blocked` at `cae2793`, because binding a WHOLE envelope key
name in the same command line is refused on its own account by
`resolve_program` step 2b (`T-19-81`). It is pinned at that identifier so nobody
counts it as a Rule B row.

#### The rule, and the formulation that was WITHDRAWN

Rule B is **positional and reads no name, no substring and no length.**
`tokenize` records that a `(`, `)`, `{` or `}` boundary was a word-splitting
flush — a word was in progress when the character arrived — and a segment whose
IMMEDIATELY preceding operator is a severing CLOSER (`}` or `)`) is a fragment
continuing an enclosing word, so its first token is not a command position and a
governed program found there is refused. The OPENER is excluded: after `{` is a
variable name, and after `(` is the substitution's own contents, which IS a
command position and keeps being classified.

An earlier draft keyed the rule on the literal fragment being a substring of an
`ENVELOPE_ENV_KEYS` entry. **It was withdrawn on two measurements, and both are
recorded so the next audit does not propose it:**

1. **Evadable — move the split point.** `${C}NT` leaves a two-character
   fragment, `${C}T` leaves one, and the bare `${C}` leaves none at all. Any
   minimum length is a floor an author ducks under by cutting one character
   further left. Rows 1–4 of the table above are exactly those spellings, and
   the positional rule refuses all four.
2. **Unshippable — it refuses ordinary shell.** `ROOT`, `DIR`, `RUN`, `CONFIG`
   and `COMMAND` all sit inside envelope key names, `GSD_MM_RUN_ID` included.
   `ROOT=$(git rev-parse --show-toplevel)`, `DIR=$(mktemp -d)`,
   `RUN_ID=$(uuidgen)`, `CONFIG=$(cat cfg)` and `COMMAND=$(which git)` would
   each be refused on every Bash tool call. All five are measured exit 0 after
   Rule B and pinned.

`SEPARATORS` is unchanged, `split_segments` keeps its signature and behaviour,
and the flush flag is computed for `( ) { }` only. `{ git status; }`,
`( git status )` and `(git status)&&git fetch origin` reach exactly the verdicts
their ungrouped spellings reach — measured, and pinned as rows.

#### What is now closed, and what is NOT

**Closed across plans 19-14 and 19-15:**

* **`T-19-87`** — six of eight measured rows by Rule A (the decision region at
  the classifiers' boundary, plan 19-14); the remaining two, plus every further
  split-point spelling found since, by Rule B (the command-position rule, plan
  19-15). Neither half changed what a brace does.
* **`T-19-89`** — plan 19-14 widened `ASSIGNMENT_PREFIXES`, `REFUSED_BASES` and
  `DECOY_OPERANDS`, added `EXPANSION_WRAPPERS` and a fresh-root forge-slot
  property. Plan 19-15 adds the last two axes audit 3 named: `SHELL_LAYERS`
  gains a brace-group and a subshell layer, and a `SEVERED_PREFIXES` alphabet
  varies WHERE the split falls with a refusal property of its own. The
  per-alphabet metacharacter floor now covers every alphabet in audit 3's axis
  table, so an alphabet narrowed back turns a test red.

**NOT closed — `/gsd-secure-phase 19` is NOT cleared by plan 19-15:**

* **`T-19-86`** remains **OPEN at `high`** by explicit user scoping decision.
  Untouched and unremediated; its four rows are still measured at exit 0 and
  pinned unmodified.
* **`T-19-91`** remains **registered OPEN at `high`** exactly as plan 19-14
  wrote it, including the recorded fact that `reflog` and `symbolic-ref` have no
  `pre-push` and no `pre-commit` second carrier.

Because both remain open at `high`, this plan does not clear the phase gate, and
re-measuring and re-classifying every row above is `/gsd-secure-phase 19`'s job
rather than this plan's.
