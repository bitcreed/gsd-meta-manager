---
phase: 19
slug: gitsafe-git-blast-radius-envelope
status: blocked
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (high)
threats_open: 8
asvs_level: 1
block_on: high
created: 2026-08-29
updated: 2026-09-04
register_authored_at_plan_time: true
audited_against: HEAD (f06d153) — the TWELFTH audit judges the tree after plans
  19-30 (the round-12 corpus, RED) and 19-31 (the interior-path rule, the
  credential.helper clause, the ledger KIND check and the two honesty repairs).
  Audit 11 judged 1e56389 (after 19-28/19-29); audit 10 judged 0092009 (after
  19-26/19-27); audit 9 judged cc65220 (after 19-24/19-25); audit 8 judged
  fb43577 (after 19-22/19-23); audit 7 judged 3110d4a (after 19-20/19-21); audit
  6 judged 9534198 (after 19-18/19-19); audit 5 judged ad847b4 (after
  19-16/19-17); audit 4 judged b72237e (after 19-14/19-15); audit 3 judged
  228e4bc (after 19-13); audit 2 judged b8605ef (after 19-11/19-12); audit 1
  judged 0ec1fcb.
register_totals: 138 total / 107 closed / 31 open / 8 at or above `high`
# the eight blocking: T-19-86, T-19-91, T-19-111, T-19-112, T-19-116, T-19-121,
# T-19-122, T-19-123
---

# Phase 19 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

Phase 19 built a **git blast radius envelope**: layered controls so a driven agent
cannot force-push, bypass git hooks, reach ambient credentials, or exceed a
per-project pull-request cap. The register below was authored at plan time across
all ten plans (`register_authored_at_plan_time: true`); this audit verifies that
each declared mitigation exists in the implementation and that the test named as
its evidence actually asserts the property.

**Verdict (audit 4, 2026-08-29): OPEN_THREATS.** Four high-severity threats are
open and blocking — `T-19-86` (registered by plan 19-13, still permitted, still
open by explicit user scoping decision), `T-19-91` (registered by plan 19-14,
still permitted, and **wider than it was registered** — see the audit-4
correction below), and two found by this audit: `T-19-92` and `T-19-93`. Plans
19-14 and 19-15 genuinely closed `T-19-87`, `T-19-88`, `T-19-89` and `T-19-90`,
each re-measured here against the built binary rather than accepted from the
summaries. One hundred and seven threats total, eighty-six closed, twenty-one
open (four at or above the `high` block threshold).

> **The pattern this phase keeps producing, restated for round 4 because it
> moved one level down.** Rounds 1–3 each had the gap in the cell *one slot
> over* from what the corpus was built to vary — the wrapper operand, the
> governed program's own operand, the verb slot, the forge decision words.
> Round 3's principle answered that structurally: a decision region must come
> from the same scan the classifier runs. **That principle holds. It is not
> where round 4's findings are.** Round 4's gap is one *character class* over:
> Rule A decides on the `Token.expansion` bit, which only an unquoted `$` or
> backtick sets, and Rule B decides on one geometry, a closer that severed a
> word. Neither can see the other ways bash makes a word — brace expansion,
> pathname expansion, `$IFS` re-splitting. `T-19-92` and `T-19-94` are two live
> instances, `T-19-93` is the same tokenizer seam reached by `gh`'s own
> documented syntax with no evasion at all, and `T-19-95` records that the
> round-4 alphabets are, for the fourth consecutive round, structurally
> incapable of failing on the class the round certified.

**Arithmetic correction to audit 3's header, recorded rather than silently
fixed.** Audit 3's header reads "One hundred and one threats total, eighty-two
closed, nineteen open". Its own group table sums to **102 total, 82 closed, 20
open**; the header is off by one on the total and by one on the open count. The
audit-4 bookkeeping table below starts from the group rows, not from that
header. Audit 3's text is left as written — this is a correction beside it, not
a rewrite of it.

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

**Audit 4's own bookkeeping, re-derived from the group rows rather than from the
line above.** Audit 3's total row is off by one in both the total and the open
count; its group rows sum to 102/82/20. Starting from those and applying round
4:

| Group | Count | Closed | Open |
|---|---|---|---|
| Everything above, per audit 3's GROUP rows (not its total row) | 102 | 82 | 20 |
| Registered by plan 19-14 (`T-19-91`) | 1 | 0 | 1 (1 at `high`) |
| Closed by plans 19-14 / 19-15, re-measured by audit 4 (`T-19-87` … `T-19-90`) | — | +4 | −4 |
| Found by audit 4 (`T-19-92` … `T-19-95`) | 4 | 0 | 4 (2 at `high`) |
| **Total after audit 4** | **107** | **86** | **21 (4 at `high`)** |

The four that count toward `threats_open`: `T-19-86`, `T-19-91`, `T-19-92`,
`T-19-93`. The seventeen that do not: `T-19-61` … `T-19-73` (13), `T-19-84`,
`T-19-85`, `T-19-94`, `T-19-95`.

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
| **T-19-87** | **Tampering / Elevation of Privilege** | **`policy::tokenize` — `{`, `}`, `(`, `)` are `SEPARATORS` (`policy.rs:968`, `:1067`)** | **high** | **mitigate (unimplemented — found in execution, deferred by 19-13)** | **Found by the executor of plan 19-13, whose plan and plan-check had both asserted the shape was closed by the expansion-prefix rule. The tokenizer flushes the current word at `{`/`}`/`(`/`)` and emits an operator, so `${K}_COMMAND` and `$(…)` are not words — they are *segment boundaries*. The guard then judges fragments the shell never runs. Registered in `deferred-items.md` scoped to "the `T-19-81` class", and pinned in `the_brace_expansion_spelling_is_a_residual_this_plan_does_not_close` with the bound *"the fragmentation does not hide a refused git command, because the segment that carries the command still resolves it."* **Audit 3: that bound is false one word to the right.** Measured at `228e4bc`: `git ${X}push --force origin main` → **exit 0** (no binding of any kind required; bash runs `git push --force origin main`, verified against a shim on `PATH`); `git ${X}stash` → **0**; `git ${X}update-ref -d refs/heads/main` → **0**; `git ${X}config core.hooksPath /tmp/x` → **0**; `gh ${X}pr create --title x` → **0 with NO ledger line and no park** — and the PR cap has no second carrier (`hooks.rs:1263-1270`, `T-19-35`). `$( )` is the same mechanism: `git $(true)push --force origin main` → **0**. **The three-layer line:** `C=GIT_CONFIG; env -u ${C}_COUNT git ${X}push --force origin main` → **exit 0**, and in bash it is `env -u GIT_CONFIG_COUNT git push --force origin main` — layer 1 does not match the `Bash(git push:*)` prefix, layer 2 permits, layer 3's `core.hooksPath` carrier is gone, and `GIT_ASKPASS`/`GIT_CONFIG_GLOBAL`/`GIT_SSH_COMMAND` are untouched so the push authenticates. That is the identical three-leg argument audit 2 used to rate `T-19-60` high and blocking, reproduced through the splitter.** **Audit 4: CLOSED for every measured row.** All EIGHT rows above, plus the two carry-forwards 19-14 left RED (`C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin`, `K=GIT_SSH; env -u ${K}_COMMAND git fetch origin`), plus the six further split-point spellings 19-15 found (`env -u ${C}`, `${C}NT`, `${C}T`, `env -u $(printf %s%s GIT_CONFIG _COUNT)`, the force-push variant, and the disclosed cost row) were re-driven at `b72237e` against the built binary, one fresh `GSD_MM_ENVELOPE_ROOT` per row, envelope directory WALKED afterwards. All sixteen at `exit 2`; every forge row's walk empty. The grouping controls hold in both directions: `{ git status; }`, `( git status )` and `(git status)&&git fetch origin` at `exit 0`, `{ git push --force origin main; }` and `( git push --force origin main )` at `exit 2` under the base's own `force_push_blocked` — so no character changed meaning. **The COMPONENT is not closed, and audit 4 says so rather than letting the row imply it:** `tokenize`'s treatment of `{`/`}`/`(`/`)` as `SEPARATORS` is unchanged, and the direction Rule B did not take is live — see `T-19-92` and `T-19-93`. | **closed (all measured rows; component residual carried by `T-19-92`/`T-19-93`)** |
| **T-19-88** | **Elevation of Privilege** | **the VERB slot of a governed program resolved at the head — `hooks.rs:960` / `:1047-1090`** | **high** | **mitigate (unimplemented)** | **`classify_segments` collapses each `Token` to its `text` at `hooks.rs:960` before calling `classify_git(&words[index+1..])` and `pr_command_label(&words[index..])`, so `Token.expansion` is **structurally unavailable** to every classifier. `resolve_program` refuses an expansion at the head (step 3) and an expansion in the wrapper prefix (step 5), but the word **one slot to the right of the head** — which is the verb every classifier decision turns on — is exempt by an explicit code comment ("an expansion in the program's own ARGUMENTS … untouched", `policy.rs:1634-1637`). Measured, no braces and no parentheses needed: `V=push; git $V --force origin main` → **exit 0**; `V=stash; git $V` → **0**; `V=update-ref; git $V -d refs/heads/main` → **0**; `P=pr; gh $P create --title x` → **0 with no ledger line**; `` gh `true`pr create --title x `` → **0 with no ledger line**. `classify_git` reaches its denylist default arm because the verb is `$V`; `pr_command_label` matches no arm, so the SAFE-06 cap is not merely exceeded — the attempt is never counted, and `git stash`, `git update-ref` and the PR cap have **no `pre-push`/`pre-commit` second carrier**. Distinct from `T-19-74`/`AR-19-10`, which is an expansion-assembled **program** behind a **wrapper**; here the program is spelled literally and resolution is correct. **The mitigation named for the neighbouring cells cannot be written where it is needed without changing the classifier boundary**, because the expansion bit is discarded at `hooks.rs:960`. The unit test whose name claims this class, `a_verb_assembled_by_expansion_and_an_eval_are_both_denied` (`hooks.rs:1917`), actually exercises `$TOOL push --force` — an expansion-assembled *program*, i.e. step 3. Its name asserts a class it does not reach.** **Audit 4: CLOSED.** `policy::expansion_in_decision_region` (`policy.rs:2168`) restores the bit at the decision boundary, called once from `classify_segments`'s `Governed` arm (`hooks.rs:1083`) BEFORE both classifiers and BEFORE the ledger write. Re-measured at `b72237e`, fresh root per row, directory walked: `V=push; git $V --force origin main` → 2, `V=stash; git $V` → 2, `V=update-ref; git $V -d refs/heads/main` → 2, `P=pr; gh $P create --title x` → 2 with an empty walk, `` gh `true`pr create --title x `` → 2 with an empty walk. The region is slot-exact in every adjacent cell audit 4 probed — the forge's SECOND subcommand word, the `-`-initial `api` flag in both spellings, the displaced `api` endpoint including the `-H` spelling, `-X`/`--method` in all four spellings, a marker-initial `api` decision word, `config`'s key operand in three spellings and the `git -c` key half in two — all `exit 2`. The rename of `a_verb_assembled_by_expansion_and_an_eval_are_both_denied` to `a_program_...` is correct: that test exercises step 3, not this class. | **closed** |
| T-19-89 | Spoofing | the 19-12/19-13 generative alphabets | medium | mitigate | `T-19-76`'s failure mode for the third consecutive round, in the cell adjacent to the one 19-13 filled. **No entry of `ASSIGNMENT_PREFIXES`, `WRAPPERS`, `SHELL_LAYERS`, `DECOY_OPERANDS` or `REFUSED_BASES` contains a `$`, a `` ` ``, a `{` or a `(`** — verified by reading all five alphabets. The corpus therefore cannot generate, and so cannot fail on: an expansion in a base's verb or argument (`T-19-88`), an expansion in a wrapper or decoy operand (the step-5 prefix rule), an assignment prefix whose VALUE is an envelope key or a governed program (steps 2b and 7), or any brace/paren grouping (`T-19-87`). Each of those rules is pinned only by enumerated rows in `tests/envelope_command_position.rs`, so the *generative* half of the evidence is silent on every one of them. `SHELL_LAYERS` in particular offers `sh -c '…'` and `bash -lc "…"` but never `{ …; }` or `( … )`, which are the two shapes that break the splitter. **Audit 4: CLOSED as scoped.** The floor `every_alphabet_this_plan_widens_can_draw_an_expansion_metacharacter` (`tests/envelope_wrapper_class.rs:1742`) now covers all six alphabets this table's axes name — `ASSIGNMENT_PREFIXES`, `REFUSED_BASES`, `DECOY_OPERANDS`, `EXPANSION_WRAPPERS`, `SHELL_LAYERS` (through the spelling each layer emits) and `SEVERED_PREFIXES` — and asserts each can draw a `$`, a backtick, a `{` or a `(`. Verified by reading every entry. **The corpus's NEW blind spot is `T-19-95`**: `EXPANSION_METACHARACTERS` is exactly those four characters, and no entry of any alphabet contains a `{a,b}` or a glob character. | closed (`T-19-95` carries the successor) |
| T-19-90 | Repudiation | `ENVELOPE_ENV_KEYS` vs. `cred::with_run_id` | low | mitigate | The widened drift pin iterates `build_env_in(...).entries()`. `GSD_MM_RUN_ID` is appended afterwards by `EnvelopeEnv::with_run_id` (`cred.rs:175-192`) and is therefore carried to the driven child while being **outside the pin's source**, so no floor can see it. It is also absent from `ENVELOPE_ENV_KEYS`, although its sibling locator `GSD_MM_ENVELOPE_PROJECT_ROOT` was added by 19-13 with the reasoning "this entry protects the EVIDENCE rather than the containment" — reasoning that applies identically here. Measured: `env -u GSD_MM_RUN_ID git fetch origin` → **exit 0**, and `hooks.rs:1161` then attributes the park to `"unattributed-run"`. Same class as `T-19-62`/`T-19-70`: evidence, not containment. **Audit 4: CLOSED.** `GSD_MM_RUN_ID` is in `ENVELOPE_ENV_KEYS` and the drift pin is re-sourced through `cred::EnvelopeEnv::with_run_id`, the seam that appends it, with a floor that turns red if the source is narrowed back to a bare `build_env_in`. Re-measured at `b72237e`: `env -u GSD_MM_RUN_ID git fetch origin` → **exit 2 `hook_bypass_blocked`**. | closed |

*`T-19-89` and `T-19-90` are open below the `high` threshold and do **not** count
toward `threats_open`. `T-19-87` and `T-19-88` do.*

> **Audit 4 correction, beside audit 3's line rather than over it.** All four of
> those rows are now **closed** — re-measured, not accepted from the summaries.
> The current `threats_open` is **4**: `T-19-86`, `T-19-91`, `T-19-92`,
> `T-19-93`. See *Threats found by audit 4* at the end of this file.

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
| 2026-08-29 (audit 4) | 107 | 86 | 21 | 4 (`T-19-86`, `T-19-91`, `T-19-92`, `T-19-93`) | `/gsd-secure-phase 19` re-run after 19-14 and 19-15 — one `gsd-security-auditor` subagent (opus), `ISOLATION=none` at `b72237e`; every closure re-measured against the built binary with a fresh envelope root per row and the directory walked afterwards, and every new finding confirmed in `bash` against argv-printing `git`/`gh`/`glab` shims |
| 2026-08-29 (audit 5) | 111 | 90 | 21 | 4 (`T-19-86`, `T-19-91`, `T-19-97`, `T-19-98`) | `/gsd-secure-phase 19` re-run after 19-16 and 19-17 — one `gsd-security-auditor` subagent (opus), `ISOLATION=none` at `ad847b4`; every closure re-measured against the built binary with a fresh envelope root per row and the directory walked afterwards, `T-19-93` verified on the COUNT bar by a walked ledger listing, and every new finding confirmed under `bash` against argv-printing shims with five candidates discarded |
| 2026-08-29 (audit 6) | 114 | 93 | 21 | 3 (`T-19-86`, `T-19-91`, `T-19-100`) | `/gsd-secure-phase 19` re-run after 19-18 and 19-19 — one `gsd-security-auditor` subagent (opus), `ISOLATION=none` at `9534198`; every closure re-measured against the built binary with a fresh envelope root per row and the directory walked afterwards, the four forge rows verified on the COUNT bar by a walked ledger listing with the cap firing on call 2, over-deletion probed as the silent direction, and the one new blocking finding confirmed against the REAL `git` binary by performing a force push that rewrote a bare remote's `main` |
| 2026-09-03 (audit 7) | 119 | 96 | 23 | 4 (`T-19-86`, `T-19-91`, `T-19-103`, `T-19-104`) | `/gsd-secure-phase 19` re-run after 19-20 and 19-21 — one `gsd-security-auditor` subagent (opus), `ISOLATION=none` at `3110d4a`; every closure re-measured against the built binary with a fresh envelope root per row and the root walked afterwards, the two-sided real-git probe re-run independently over every entry of both callee-grammar constants, and the two new blocking findings confirmed against the REAL `git` binary by performing an in-namespace push carrying a planted credential that the `pre-push` hook refuses and a destructive force push that rewrote a bare remote's `main` |
| 2026-09-03 (audit 8) | 121 | 99 | 22 | 3 (`T-19-86`, `T-19-91`, `T-19-108`) | `/gsd-secure-phase 19` re-run after 19-22 and 19-23 — one `gsd-security-auditor` subagent (opus), `ISOLATION=none` at `fb43577`; every closure re-measured against the built binary with a fresh envelope root per row and the root walked afterwards, the config-resolution precedence table re-measured against the REAL `git` binary with the envelope's own injection as the control including the repo-local, WORKTREE and GLOBAL persisted paths audit 7 had not reached, and the one new blocking finding confirmed by re-running audit 7's OWN destructive command pair at `fb43577` — it still rewrote a bare remote's `main` |
| 2026-09-04 (audit 9) | 126 | 102 | 24 | 4 (`T-19-86`, `T-19-91`, `T-19-111`, `T-19-112`) | `/gsd-secure-phase 19` re-run after 19-24 and 19-25 — one `gsd-security-auditor` subagent (opus), `ISOLATION=none` at `cc65220`; every closure re-measured against the built binary with a fresh envelope root per row and the root walked afterwards, `alias` re-confirmed as the ONLY K1 section by sweeping twenty-one config keys against the REAL `git` binary with the envelope's own injection as the control, and the two new blocking findings confirmed end to end — a `.git/config` alias written by `printf >>` that rewrote a bare remote's `main`, with the same alias carrying no carrier REFUSED beside it, and a PR-cap reset driven through the guard until the cap fired and again after the deletion the guard permitted |
| 2026-09-04 (audit 10) | 130 | 103 | 27 | 6 (`T-19-86`, `T-19-91`, `T-19-111`, `T-19-112`, `T-19-116`, `T-19-118`) | `/gsd-secure-phase 19` re-run after 19-26 and 19-27 — one `gsd-security-auditor` subagent (opus), `ISOLATION=none` at `0092009`; every closure re-measured against the built binary with a fresh envelope root per row and the root walked afterwards, rule (a)'s narrowing verified real over nineteen refused rows with both ordering-pin pairs at their different identifiers and all ten fail-open rows re-measured permitted, `C-15` re-driven end to end against a default-caps control, and the two new blocking findings confirmed against the REAL `git` binary — a replaced exec target that let a force push the hook had refused MOVE a bare remote's `main` with `GIT_CONFIG_COUNT` untouched and restored to a refusal by putting the binary back, and a `credential.helper = store` appended to the generated gitconfig that made `git credential fill` return the ambient secret against a control in which the same file named no helper |
| 2026-09-04 (audit 11) | 133 | 105 | 28 | 7 (`T-19-86`, `T-19-91`, `T-19-111`, `T-19-112`, `T-19-116`, `T-19-119`, `T-19-121`) | `/gsd-secure-phase 19` re-run after 19-28 and 19-29 — one `gsd-security-auditor` subagent (opus), `ISOLATION=none` at `1e56389`; every closure re-measured against the built binary with a fresh envelope root per row and the root walked afterwards, round 5's literalness bit and round 6's deletion model re-verified INTACT three ways with the SEGMENT-COUNT pins read for falsifiability in both the SPLIT and the DISPLACED variant, the redirection channel swept over all seven pathname operators and the five non-pathname ones, the empty-`credential.helper` control re-driven against real git with the URL-SCOPED cell probed and eight env strip-spellings measured, the ledger bound driven to its exact discrimination byte, and the two new blocking findings confirmed against the REAL `git` binary — an option-attached write that replaced the guard's own binary and let a force push the hook had refused MOVE a bare remote's `main` (`835b6be` -> `40ad7f7`), restored to a refusal by putting the binary back and reproduced on the ledger as a fired PR cap reset against its space-separated twin at exit 2; and a `-c credential.helper=store` that returned the ambient `~/.git-credentials` secret on a permitted line at round 10's posture as well as this one |
| 2026-09-04 (audit 12) | 138 | 107 | 31 | 8 (`T-19-86`, `T-19-91`, `T-19-111`, `T-19-112`, `T-19-116`, `T-19-121`, `T-19-122`, `T-19-123`) | `/gsd-secure-phase 19` re-run after 19-30 and 19-31 — one `gsd-security-auditor` subagent (opus), `ISOLATION=none` at `f06d153`; every closure re-measured against the built binary with a fresh envelope root per row and the root walked afterwards, `T-19-119`'s whole CLASS swept at fourteen attachment characters over both protected paths, the ledger KIND check driven at FIFO, directory, character-device-symlink and symlink-to-regular, the credential clause driven at five refused spellings and five near-miss controls with the reach re-confirmed against real git in a fake HOME, and the widened rule's CPU cost measured against a REBUILT PRE-ROUND-12 CONTROL BINARY; and three new findings confirmed against the REAL `git` binary with a control beside every leg — a here-string word naming the envelope directory that `xargs rm -rf` deleted, moving a bare remote's `main` and resetting a fired PR cap while its space-separated twin stayed at exit 2; the ANCESTOR of the protected directory (`rm -rf <root>`), which did the same; and a quadratic candidate scan in which a 16 KB command crosses `GUARD_TIMEOUT_SECS = 5` where the pre-round-12 binary answers the same input in 39 ms |

### Audit 12 method (re-audit after plans 19-30 and 19-31)

**State A** (prior SECURITY.md, `status: blocked`, `threats_open: 7`), ASVS L1,
`block_on: high`, `ISOLATION=none` on the main checkout at `f06d153`. The mandate
was to test a NARROWING round rather than a discovery round: **is the residue
condition finally complete, do the round's three new mechanisms create new
reachable state, and is the widened rule's cost what it claims?**

Method, in order:

1. Read `19-SECURITY.md`, `19-30-SUMMARY.md` and `19-31-SUMMARY.md`, plus the two
   appended execution records.
2. Built the tree at `f06d153` and drove the **binary** — `printf '<PreToolUse
   JSON>' | GSD_MM_ENVELOPE_ROOT=$(mktemp -d) ./target/debug/gsd-meta-manager
   envelope guard alpha` — with a fresh envelope root per row and the whole
   envelope root walked afterwards.
3. **Attacked the residue trichotomy directly, which is what the mandate asked
   for.** For each of the three declared silences, constructed the complement: a
   word that is LITERAL, carries an ABSOLUTE path, and needs no link — and asked
   where such a word can still be permitted. Two answers were found and both were
   driven to harm.
4. **Rebuilt a PRE-ROUND-12 CONTROL BINARY** — `git worktree add` at `dd17bfb`
   with a separate `CARGO_TARGET_DIR` — so every "this round changed X" claim
   could be measured on both trees against the same input rather than inferred
   from a diff.
5. Treated each of the round's THREE new mechanisms as new attack surface: the
   `/`-anchored candidate scan (what does it now refuse, and what does it now
   COST?), the `credential.helper` by-name clause (does it over-refuse? does it
   reach what it says?) and the ledger KIND check (does it refuse a fresh root?
   does it still count a symlinked-to-regular ledger?).
6. Rebuilt a bare-remote fixture with `pre-push`/`pre-commit` in `stub_body`'s
   exact three-line shape delivered through the envelope's own
   `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_0`/`GIT_CONFIG_VALUE_0` triplet, and
   measured every leg **with a control beside it**, recording the bare remote's
   SHA before and after and restoring the refusal afterwards.
7. Drove the PR-cap ledger from a FIRED cap in one persistent root for each new
   finding, with the space-separated twin measured beside it.
8. Measured the credential legs against real git in a fake HOME holding a
   `~/.git-credentials`, at this round's posture and without the injected pair.
9. Confirmed every claimed bypass reaches the file under a real `bash`, and
   confirmed every corpus re-spelling the round made reaches it too.
10. Read the gate with `rtk proxy grep` over a redirected log (D-34).

### Audit 11 method (re-audit after plans 19-28 and 19-29)

**State A** (prior SECURITY.md, `status: blocked`, `threats_open: 6`), ASVS L1,
`block_on: high`, `ISOLATION=none` on the main checkout at `1e56389`. The mandate
was to TEST audit 10's claim now that the disposition is done: **is the plane
finished, are the residues true as stated, and does anything this round installed
create a new reachable state?**

Method, in order:

1. Read `19-SECURITY.md`, `19-28-SUMMARY.md` and `19-29-SUMMARY.md`, plus the two
   appended execution records.
2. Built the tree at `1e56389` and drove the **binary** — `printf '<PreToolUse
   JSON>' | GSD_MM_ENVELOPE_ROOT=$(mktemp -d) ./target/debug/gsd-meta-manager
   envelope guard alpha` — with a fresh envelope root per row and `os.walk` over
   the whole root afterwards, the walk proved non-blind on every pass by
   `gh pr create --title x` leaving exactly one `alpha/pr-ledger.ndjson` line.
3. Treated each of the round's THREE new mechanisms as new attack surface as well
   as new defence: the `Segment`-borne redirection channel (does it undo round 5
   or round 6?), the exact-path binary clause (can it be turned into a denial?),
   the empty-`credential.helper` injection (does it add env-reachable state?) and
   the ledger bound (can it be turned into a denial or a bypass?).
4. Re-derived round 5 and round 6's integrity three ways — `SEPARATORS` by
   `git log -L`, `is_separator(">")` from the source, and `segment.tokens` from
   the SEGMENT-COUNT pins, which were READ for falsifiability in both the SPLIT
   and the DISPLACED variant rather than accepted green.
5. Swept the cells adjacent to all six named axes: the redirection channel over
   all twelve operators in both quote forms and both segment positions and
   through the `NestedPayload` recursion; the binary clause at every plausible
   write spelling; the credential control at the URL-scoped key and at eight
   environment spellings that would strip the injected pair; the ledger bound at
   its exact discrimination byte and at a file whose length is not its size.
6. Rebuilt a bare-remote fixture with `pre-push`/`pre-commit` in `stub_body`'s
   exact three-line shape delivered through the envelope's own
   `GIT_CONFIG_COUNT`/`GIT_CONFIG_KEY_0`/`GIT_CONFIG_VALUE_0` triplet, and
   measured every leg **with a control beside it**, recording the bare remote's
   SHA before and after each.
7. Measured the credential legs against real git in a fake HOME holding a
   `~/.git-credentials`, at THIS round's posture and again at round 10's, to tell
   a regression from a pre-existing gap.
8. Confirmed every claimed bypass reaches the file under a real `bash`.
9. Read the gate with `rtk proxy grep` over a redirected log (D-34).

### Audit 10 method (re-audit after plans 19-26 and 19-27)

**State A** (prior SECURITY.md, `status: blocked`, `threats_open: 4`), ASVS L1,
`block_on: high`, `ISOLATION=none` on the main checkout at `0092009`. The mandate
was audit 9's carried one layer further: **with argv fully modelled and the
envelope's own file carriers now partly defended, is there a further layer, or is
the modelled surface complete and the remaining risk exactly the disclosed
residual set?**

Method, in order:

1. Read `19-SECURITY.md`, `19-26-SUMMARY.md` and `19-27-SUMMARY.md`, plus the two
   appended execution records and the carrier enumeration.
2. Built the tree at `0092009` and drove the **binary** — `printf '<PreToolUse
   JSON>' | GSD_MM_ENVELOPE_ROOT=$(mktemp -d) ./target/debug/gsd-meta-manager
   envelope guard alpha` — with a fresh envelope root per row and `os.walk` over
   the whole root afterwards, the walk proved non-blind on every pass by
   `gh pr create --title x` leaving exactly one `alpha/pr-ledger.ndjson` line.
3. Re-measured every `19-26`/`19-27` row: nineteen envelope-root operand rows,
   all ten fail-open rows, both near-miss controls, the whole ordinary-operand
   half, both ordering-pin pairs and both governed-carrier substitutes with their
   outside controls.
4. Swept the cells adjacent to all **six** named axes and off the plane — every
   carrier outside the envelope root, the guard's own binary and the path that
   execs it, the guard's registration and its registered deadline, the state it
   carries between calls, the environment, the clock, the network and its own CLI
   surface.
5. Probed rule (a)'s own condition at the boundary in sixteen path spellings,
   with the absolute-literal control beside each and `bash` confirming which
   permitted spellings really reach the file.
6. Rebuilt a bare-remote fixture with `pre-push` and `pre-commit` delivered
   exactly as the envelope delivers them (`GIT_CONFIG_COUNT=1
   GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=<hooks dir>`) and stubs in
   `stub_body`'s exact three-line shape, and measured every leg **with a control
   beside it**, recording the bare remote's SHA before and after each.
7. Measured the credential leg against real git in a fake HOME holding a
   `~/.git-credentials`, with the generated `gitconfig` exactly as
   `write_gitconfig_in` leaves it as the control.
8. Read the gate with `rtk proxy grep` over a redirected log (D-34).

### Audit 9 method (re-audit after plans 19-24 and 19-25)

**State A** (prior SECURITY.md, `status: blocked`, `threats_open: 3`), ASVS L1,
`block_on: high`, `ISOLATION=none` on the main checkout at `cc65220`. The mandate
was audit 8's brief carried one layer further: **with word assembly, word
deletion, leading-option grammar, config resolution, the config-bearing
environment and now re-parsed config values all modelled, is there a further
layer, or is the modelled surface complete and the remaining risk exactly the
disclosed residual set?**

Method, in order:

1. Read `19-SECURITY.md`, `19-24-SUMMARY.md`, `19-25-SUMMARY.md` and the two
   appended execution records.
2. Built the tree at `cc65220` and drove the **binary** — `printf '<PreToolUse
   JSON>' | GSD_MM_ENVELOPE_ROOT=$(mktemp -d) ./target/debug/gsd-meta-manager
   envelope guard alpha` — with a fresh envelope root per row and `os.walk` over
   the whole root afterwards. The walk is proved non-blind on every pass by a
   positive control: `gh pr create --title x` leaves exactly one
   `alpha/pr-ledger.ndjson` line.
3. Re-measured every `19-24`/`19-25` row, every discrimination control, every
   over-refusal twin and every disclosed-cost pair, then probed the
   carrier-reading boundary in seventeen further spellings neither round pinned.
4. Swept the cells adjacent to all six named axes **and off the axis** — the
   operand grammar of every verb `classify_git` dispatches on, the complete
   `git` global-option grammar in both attached and separate-word spellings,
   twenty-one config keys written by mechanisms other than `-c`/`--config-env`,
   the `PATH`/`GIT_EXEC_PATH`/`--exec-path` binary-selection family, and the
   integrity of the guard's own two file carriers.
5. Rebuilt a bare-remote fixture with `pre-push` and `pre-commit` delivered
   exactly as the envelope delivers them (`GIT_CONFIG_COUNT=1
   GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=<hooks dir>`), with the
   generated helper-free file on both config pointers, and measured every leg
   **with a control beside it**, recording the bare remote's SHA before and
   after each.
6. Read the gate with `rtk proxy grep` over a redirected log (D-34).

### Audit 8 method (re-audit after plans 19-22 and 19-23)

**State A** (prior SECURITY.md, `status: blocked`, `threats_open: 4`), ASVS L1,
`block_on: high`, `ISOLATION=none` on the main checkout at `fb43577`. The mandate
was audit 7's brief carried one layer further: not "do the claimed mitigations
exist" but **"with word assembly, word deletion, leading-option grammar, config
resolution and the config-bearing environment all now modelled, is there a
further layer between the guard's decision and what git actually does?"**

Method, in order:

1. Read `19-SECURITY.md`, `19-22-SUMMARY.md`, `19-23-SUMMARY.md`, the two
   appended execution records and `19-23-PLAN.md`'s blocker section.
2. Built the tree at `fb43577` and drove the **binary** — `printf '<PreToolUse
   JSON>' | GSD_MM_ENVELOPE_ROOT=$(mktemp -d) ./target/debug/gsd-meta-manager
   envelope guard alpha` — with a fresh envelope root per row and `os.walk` over
   the whole root afterwards. The walk is proved non-blind on every pass by a
   positive control: `gh pr create --title x` leaves exactly one
   `alpha/pr-ledger.ndjson` line.
3. Re-measured every `19-22`/`19-23` row, every discrimination control and every
   over-refusal twin, plus case-varied spellings neither round pinned.
4. Re-measured git's config precedence **against real git 2.43.0** with the exact
   `GIT_CONFIG_COUNT`/`KEY_0`/`VALUE_0` triplet `cred::hooks_path_env` emits as
   the control, extending it to the persisted paths the section test cannot
   reach: repo-local, `--worktree`, and global.
5. Swept the cells adjacent to all four named axes, then rebuilt a bare-remote
   fixture with `pre-push` and `pre-commit` delivered exactly as the envelope
   delivers them and re-ran **audit 7's own destructive command pair**.
6. Re-ran the two-sided callee probes independently against the installed `git`
   **and** the installed `gh`.
7. Read the gate with `rtk proxy grep` over a redirected log (D-34).

### Audit 7 method (re-audit after plans 19-20 and 19-21)

**State A** (prior SECURITY.md, `status: blocked`, `threats_open: 3`), ASVS L1,
`block_on: high`, `ISOLATION=none` on the main checkout at `3110d4a`. The mandate
was audit 6's brief carried one layer further: not "do the claimed mitigations
exist" but **"with the shell boundary modelled and the callee's leading-option
grammar now fail-closed, is there a further layer between the guard's decision and
what git actually does?"** — see *Audit 7 — what the round-7 control can and
cannot fail on* below for the axis-by-axis method and its two blocking results.

**The measurement harness.** Every row — closure and finding alike — was driven as

```
printf '{"tool_name":"Bash","tool_input":{"command":"<CMD>"}}' \
  | GSD_MM_ENVELOPE_ROOT=$(mktemp -d) ./target/debug/gsd-meta-manager envelope guard alpha
```

against the **built binary**, with a **fresh `GSD_MM_ENVELOPE_ROOT` per row** and
the whole envelope root **walked with `os.walk` afterwards**, so "no ledger line"
is an OBSERVATION rather than an inference. The walk is proved non-blind on every
pass by a positive control: `gh pr create --title x` leaves
`alpha/pr-ledger.ndjson` with one line on call 1 and the cap fires on call 2 with
two.

**The callee, not the shell — so every claim was settled against the real `git`
binary.** `git version 2.43.0`, in a purpose-built fixture on
`refs/heads/gsd-auto/alpha/w` with a local bare upstream, with `core.hooksPath`
delivered by the **envelope's own mechanism** (`GIT_CONFIG_COUNT` /
`GIT_CONFIG_KEY_n` / `GIT_CONFIG_VALUE_n`, the triplet `cred::hooks_path_env`
emits) rather than a stand-in. Precedence was measured with paired controls in
every case, the `pre-push` and `pre-commit` enforcement points were exercised
separately, and the two blocking findings were each demonstrated by a real push
that moved a bare remote's ref — one carrying a planted credential the hook
refuses, one a destructive force push that rewrote `main`.

**Gates observed.** `rtk proxy cargo test --no-fail-fast` redirected to a file and
counted with `rtk proxy grep`: **1639 passed, 0 failed, 13 ignored** over 42
result lines, `passed + failed = 1639`, matching `19-21-SUMMARY.md` exactly. All
**thirteen** `envelope_*` binaries ran, so `19-20`'s own evidence file executed.
`rtk proxy cargo clippy -- -D warnings` exits 0; `cargo clippy --tests` fails at
the base on four pre-existing lints in `src/browser.rs` and
`src/project_creator.rs` and is out of scope.

**Provenance discipline.** Plans 19-13 … 19-21's appended subsections, the
plan-19-21 blocker-row resolution record and every earlier audit's own tables are
left **byte-identical**; audit 7 checksummed the whole body below the frontmatter
before writing. Audit 7's own findings are in their own section at the end of this
file, and its corrections to statements made in those subsections are recorded as
audit-7 findings BESIDE them rather than as edits to them.

### Audit 4 method (re-audit after plans 19-14 and 19-15)

**State A** (prior SECURITY.md, `status: blocked`, `threats_open: 3`), ASVS L1,
`block_on: high`, `ISOLATION=none` on the main checkout at `b72237e`. The
mandate was audit 3's, sharpened: not "do the claimed mitigations exist" but
**"does the same-scan principle now hold everywhere it must, and what class can
the new controls still not fail on?"**

**The measurement harness, stated because it is why audit 3's findings held and
audit 4's should be reproducible.** Every row — closure and finding alike — was
driven as

```
printf '{"tool_name":"Bash","tool_input":{"command":"<CMD>"}}' \
  | GSD_MM_ENVELOPE_ROOT=$(mktemp -d) ./target/debug/gsd-meta-manager envelope guard alpha
```

against the **built binary**, with a **fresh `GSD_MM_ENVELOPE_ROOT` per row**
(the PR ledger persists and otherwise produces misleading cap-exhaustion
refusals), and with the envelope directory **walked with `find` afterwards**, so
"**no ledger line**" is an OBSERVATION rather than an inference from one
expected path. The walk is proved non-blind on every pass by a positive control:
a permitted `gh pr create --title x` leaves `alpha/pr-ledger.ndjson` in the walk,
and `gh api repos/o/r/pulls -f title="$T"` leaves one too — so an empty walk
beside a refusal means something.

**Shell semantics confirmed, never assumed.** Every claimed bypass was then run
under `bash` with shims named `git`, `gh` and `glab` first on `PATH` printing
their own argv. A guard permit is only a finding if the shell really runs the
dangerous command; three candidate rows were **discarded** on this step —
`{ git,push } --force origin main` is a bash syntax error,
`sh -c 'gh {pr,create} --title x'` does not brace-expand under `dash`, and
`gh -R o/r api repos/o/r/pulls -f title=x` (which the guard permits UNCOUNTED,
through a real disagreement between `subcommand_word_indices` and `scan_gh_api`
about `FORGE_VALUE_OPTS`) is rejected by the real `gh` binary with `unknown
shorthand flag: 'R'` and is therefore **not** registered as a threat. It is
recorded here only so audit 5 does not spend the measurement again.

**Gates observed.** `rtk proxy cargo test --no-fail-fast` redirected to a file:
**1533 passed, 0 failed, 13 ignored** over 39 suites, `passed + failed = 1533`,
matching `19-15-SUMMARY.md` exactly. All eleven `envelope_*` binaries ran.
`tests/envelope_expansion_slots.rs` reports 32 tests and both carry-forward
names — `the_severed_git_config_count_row_is_carried_forward_to_19_15_and_is_expected_red`
and `the_severed_git_ssh_command_row_is_carried_forward_to_19_15_and_is_expected_red`
— are **green, neither renamed, weakened nor `#[ignore]`d**. Plain `cargo test`
remains unusable for this phase: it fail-fasts at `driver_reattach` and
`envelope_*` sorts after `driver_*`.

**Provenance discipline.** Plan 19-13's `T-19-86` subsection and plans 19-14's
and 19-15's execution-record subsections are left **byte-identical**; audit 4
verified this with a checksum before and after writing. Audit 4's own findings
are in their own section at the end of this file, and its corrections to
statements made in those subsections are recorded as audit-4 findings BESIDE
them rather than as edits to them.

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
- [ ] `threats_open: 0` confirmed — **audit 3 recorded 3 open at `high`**
- [ ] `status: verified` set in frontmatter

**Approval:** blocked 2026-08-29 (audit 3) — close `T-19-86`, `T-19-87` and
`T-19-88`, then re-run `/gsd-secure-phase 19`.

> **Superseded by audit 4's sign-off at the end of this file.** `T-19-87` and
> `T-19-88` are closed; `T-19-91`, `T-19-92` and `T-19-93` have joined
> `T-19-86` at `high`. The current gate figure is **`threats_open: 4`**, and the
> block list is `T-19-86`, `T-19-91`, `T-19-92`, `T-19-93`. This block is left
> as audit 3 wrote it so the two rounds stay comparable.

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


---

## Threats found by audit 4 (2026-08-29, after plans 19-14 and 19-15)

**Provenance, stated first.** The two subsections above this one were written by
the EXECUTORS of plans 19-14 and 19-15 and are deliberately outside the audit
tables; audit 4 left them byte-identical. Everything from here to the end of the
file is **audit 4's own**, measured against the built binary at `b72237e` with a
fresh `GSD_MM_ENVELOPE_ROOT` per row and the envelope directory walked
afterwards, and with every claimed bypass re-run under `bash` against
argv-printing `git`/`gh`/`glab` shims. See *Audit 4 method* above for the
harness and for the three candidate rows that were discarded because the shell
did not actually run what the guard had permitted.

**What round 4's findings are NOT.** They are not the same-scan defect again.
Rule A's decision region is slot-exact in every adjacent cell probed, Rule B is
positional and reads no name, and `split_segments` is genuinely defined over
`split_segments_with_heads` so there is one walk. **That principle holds.** The
findings below are one level down: the controls read one bit and one geometry,
and bash makes words in ways that set neither.

| Threat ID | Category | Component | Severity | Disposition | Finding | Status |
|---|---|---|---|---|---|---|
| **T-19-92** | **Elevation of Privilege** | **`policy::tokenize` — a `{` that opens a brace EXPANSION is read as a brace GROUP (`policy.rs:1105`, `:1278`, `:1224`)** | **high** | **mitigate (unimplemented)** | **`tokenize` marks a separator as a word-splitting flush only when a word was in progress IMMEDIATELY BEFORE the character, and `Segment::head_is_command_position` then acts on CLOSERS only. Bash's grammar has a second condition the flag does not encode: `{` is the reserved word opening a group only when it is at a COMMAND POSITION. Inside a simple command it opens a brace EXPANSION whose contents bash splices back into that same command. So `git {push,--force} origin main` is split into the segments `git`, `push,--force` and `origin main`; the first resolves `Governed` with an EMPTY argv, `classify_git` returns `Allow` for a bare `git`, and the guard permits a command the shell will run as `git push --force origin main`. Measured at `b72237e`, fresh root per row, walk after: `git {-c,core.hooksPath=/dev/null,push,--force,origin,main}` → **exit 0**; `git {push,--force} origin main` → **0**; `git {update-ref,-d,refs/heads/main}` → **0** (no second carrier); `git {config,core.hooksPath,/tmp/x}` → **0**; `gh {pr,create} --title x` → **0 with an EMPTY WALK**; `gh {api,repos/o/r/pulls} -f title=x` → **0 with an EMPTY WALK**; `glab {mr,create} --title x` → **0**; and through the nested-payload path `bash -lc "git {push,--force} origin main"` → **0**. Every one confirmed under `bash` against the shims: the first prints `ARGV: -c core.hooksPath=/dev/null push --force origin main`. **The three-leg argument, each leg measured rather than argued.** Layer 1 — the line begins `git {-c,…`, which does not match the `Bash(git push:*)` prefix rule the agent CLI owns. Layer 2 — exit 0 above, nothing written. Layer 3 — re-measured on this machine: `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=/ENV_WINS git {-c,core.hooksPath=/CLI_WINS,config,--get,core.hooksPath}` prints **`/CLI_WINS`**, so the command-line `-c` outranks the envelope's env-injected `core.hooksPath` and no `pre-push` hook runs. Credentials — `GIT_ASKPASS`, `GIT_CONFIG_GLOBAL/SYSTEM` and `GIT_SSH_COMMAND` are untouched, so the push authenticates. That is the identical three-leg argument audit 2 used to rate `T-19-60` high and audit 3 used for `T-19-87`, reproduced through the opener the positional rule excludes. Two of the rows additionally have **no second carrier at all** — `git update-ref` and the SAFE-06 PR cap (`hooks.rs:1263-1270`, `T-19-35`) — so for those the guard is the only control and it is bypassed outright, uncounted rather than exceeded. **Same COMPONENT as `T-19-87`, the direction Rule B did not take.** Rule B's doc argues the opener is safe because "after `{` is the parameter expansion's variable name, never a governed program"; that is true for `${`, and false for a ` {` that opens a brace expansion. Do NOT close this by deleting the characters from `SEPARATORS` — `{ cmd; }` and `( cmd )` are real grouping syntax and losing them re-opens the `echo hi && git push --force` class one level in, which is what audit 3 already recorded. | **OPEN — BLOCKING** |
| **T-19-93** | **Repudiation / Elevation of Privilege** | **the SAFE-06 PR cap vs. `gh`'s own `{owner}`/`{repo}` endpoint placeholders — same tokenizer seam** | **high** | **mitigate (unimplemented)** | **`gh api` documents `{owner}`, `{repo}` and `{branch}` as placeholders substituted from the repository of the current directory, and that is the idiomatic spelling in `gh`'s own manual. Because `{` and `}` are `SEPARATORS`, `gh api repos/{owner}/{repo}/pulls -f title=x` is fragmented before either forge scan sees it, `pr_command_label` matches no arm, and the creation is **never counted**. Measured at `b72237e`, walk after: the unquoted form → **exit 0 with an EMPTY WALK**; the quoted form `gh api "repos/{owner}/{repo}/pulls" -f title=x` → exit 0 **with a ledger line**. Bash confirms the unquoted braces are literal (no comma, so no brace expansion): `printf "[%s]" repos/{owner}/{repo}/pulls` prints them unchanged, and `gh` is what substitutes them. **This is not an evasion.** It is the documented spelling, so the cap under-counts by accident for an honest agent and can be under-counted deliberately by a hostile one; and the cap has **no second carrier**. Rated `high` on the standard this phase has already used twice — SAFE-06 bypassed rather than exceeded, with no `pre-push`/`pre-commit` behind it. Registered separately from `T-19-92` because the mechanism differs: `T-19-92` is bash splicing words back in, this is a literal token the tokenizer fragments and neither forge scan can reassemble. | **OPEN — BLOCKING** |
| T-19-94 | Elevation of Privilege | pathname (glob) expansion in a decision slot — `Token.expansion` is set only by `$` and `` ` `` (`policy.rs:1325`, `:1351`) | medium | mitigate | Rule A refuses a decision word the shell assembles, but it recognises assembly by ONE bit, and `tokenize` sets that bit only for an unquoted `$` or backtick. Pathname expansion sets nothing. Measured at `b72237e` with a matching file in the working directory: `git pus? --force origin main` → **exit 0**, `git ?ush --force origin main` → **0**, `git stas?` → **0** (no second carrier), `gh p? create --title x` → **0**. Confirmed under `bash` against the shims: with a file named `push` present, `git pus? --force origin main` prints `ARGV: push --force origin main`; with `stash` present, `git stas?` prints `ARGV: stash`; with `pr` present, `gh p? create --title x` prints `ARGV: pr create --title x`. The precondition is satisfiable inside a single tool call — `touch push && git pus? --force origin main` → **exit 0** — because `touch push` is `Ungoverned` and the two segments are judged independently. Rated **medium** rather than high on the precondition alone: it needs a matching filename in the working directory, where `T-19-92` needs nothing. Same root cause, and any fix for the one should be written so it covers the other. | open — below `high` (non-blocking) |
| T-19-95 | Spoofing | the 19-14/19-15 generative alphabets and `EXPANSION_METACHARACTERS` | medium | mitigate | **`T-19-76`'s failure mode for the FOURTH consecutive round, in the cell adjacent to the one round 4 filled.** `EXPANSION_METACHARACTERS` (`tests/envelope_wrapper_class.rs:1717`) is exactly `['$', '`', '{', '(']`, and the floor `every_alphabet_this_plan_widens_can_draw_an_expansion_metacharacter` asserts each of the six alphabets can draw one of them. Verified by reading every entry of `ASSIGNMENT_PREFIXES`, `WRAPPERS`, `REFUSED_BASES`, `DECOY_OPERANDS`, `EXPANSION_WRAPPERS`, `SHELL_LAYERS` and `SEVERED_PREFIXES`, and by grepping the file: **not one entry anywhere contains a comma inside braces (`{a,b}`), and not one contains a `*`, a `?` or a `[`.** Every entry satisfying the floor does so through an expansion MARKER — `$`, `` ` ``, `${`, `$(`. The corpus is therefore structurally incapable of generating, and so of failing on, `T-19-92` and `T-19-94`. The floor itself is real and non-vacuous for the class it names; it simply names a smaller class than "a word the shell assembles". This is the same finding as `T-19-76`, `T-19-83` and `T-19-89`, one radius further out, and the third time the alphabet has certified a claim it could not have failed on. | open — below `high` (non-blocking) |

*Status: open · closed · open — below `high` threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` count toward `threats_open`*

`T-19-94` and `T-19-95` are open below the `high` threshold and do **not** count
toward `threats_open`. `T-19-92` and `T-19-93` do.

### The `T-19-91` correction — audit 4's own finding, beside plan 19-14's subsection rather than an edit to it

Plan 19-14's appended subsection and plan 19-15's both record this row:

> `exit=2  git push $REF   [push_outside_namespace]  <- already fails closed`

and `resolve_program`'s fourth residual bullet argues from it that "only `git
push` has a hook behind it, **and its refspec operand already fails CLOSED**".

**Measured at `b72237e`, that is wrong for the configuration the envelope exists
for.** `push_needs_resolved_dests` answers `true` for this shape, so the guard
shells out to `git` in the caller's working directory and the verdict depends on
that repository — which is exactly why plan 19-14 declined to pin it (`T-19-80`),
and that decision was correct. But the recorded VERDICT is only one of the two
answers the cwd can give:

- outside any repository, and inside this repository: `git push $REF` → **exit 2**
  `push_outside_namespace`, as recorded;
- inside a repository whose current branch is **inside the envelope's namespace**
  — a branch under `refs/heads/gsd-auto/alpha/`, which is the state a driven run
  is designed to be in — `git push $REF` → **exit 0**. Measured in a purpose-built
  fixture repo on `gsd-auto/alpha/work` with an upstream configured. The bare
  `git push` is exit 0 there too, correctly; the difference is that with `$REF`
  present the guard has resolved a context for an argv whose refspec it cannot
  read.

So `classify_push`'s refspec operand joins `classify_reflog`'s and
`classify_symbolic_ref`'s as a THIRD arm of `T-19-91`'s shape that answers
`Allow` on an operand it cannot read. The threat is **wider than registered**.

**Two things this does NOT change.** The severity stays `high` and the row stays
`T-19-91` rather than becoming a new id — it is the same component and the same
mechanism, measured more completely. And the second-carrier asymmetry the
registration draws is still right: `push` does have `pre-push` behind it, while
`reflog` and `symbolic-ref` have nothing, so this widening does not make the
whole row worse, it makes its boundary honest.

**Where the record is right and where it is wrong**, so a future reader knows
which text to trust: `src/envelope/policy.rs:1735-1742` is **narrower and
correct** — it claims only that `git push origin $REF` is refused, which audit 4
re-measured at exit 2. The `19-SECURITY.md` row, `19-14-SUMMARY.md` and
`19-15-SUMMARY.md` all state the stronger and false "already fails closed" for
the bare `git push $REF`. Correct disposition: **cwd-dependent, and permitted in
the in-namespace configuration.** This is the `T-19-84` class — a disclosure that
exists and is pinned, but is not true as written — and it is why audit 4 records
it as a finding rather than a footnote.

### The known-open set, as audit 4 verified it

- **`T-19-86` (high, OPEN, unaccepted, deferred by explicit user scoping
  decision).** Correctly bounded and honestly disclosed. All four rows
  re-measured at `b72237e`, fresh root each: `git submodule foreach git push
  --force origin main` → 0, `git rebase -x "git push --force origin main" HEAD~3`
  → 0, `git bisect run sh -c "git push --force origin main"` → 0,
  `git -c alias.p='!git push --force origin main' p` → 0 — exactly as the
  registration says, with
  `the_t_19_86_residual_is_permitted_today_and_this_plan_leaves_it_permitted`
  unmodified and the third residual bullet of `resolve_program`'s doc describing
  the mechanism correctly. Counts toward `threats_open`.
- **`T-19-91` (high, OPEN).** `git reflog $S`, `git reflog show $S` and
  `git symbolic-ref $S` reproduce at exit 0; `git symbolic-ref HEAD $R` → 2
  `force_push_blocked` and `git push origin $REF` → 2 `push_outside_namespace`
  still fail closed. Audit 4 independently enumerated `classify_git`'s arms
  (`policy.rs:298-317`) and confirms the residual covers every arm that decides
  on an operand: `push`, `config` (closed), `reflog`, `symbolic-ref` — `stash`,
  `update-ref`, `filter-branch` and `filter-repo` refuse unconditionally and read
  no operand. The no-second-carrier claim for `reflog`/`symbolic-ref` is correct.
  Widened by the correction above. Counts toward `threats_open`.
- **`T-19-74` (medium, closed/accepted — AR-19-10).** Narrowed at exactly the one
  disclosed spelling and no further. `env -u git $X push --force origin main`,
  which 19-14 converted, → **exit 2** as documented; the accepted core is frozen
  and re-measured: `env $X push --force origin main` → **0** and
  `X=git; env $X push --force origin main` → **0**. The narrowing is disclosed in
  `19-14-SUMMARY.md` as a side effect of closing `T-19-88` rather than as a
  decision to move the acceptance, which is the honest framing. `T-19-84` — that
  the doc's narrowing argument is false as written — remains open and unaccepted,
  and the doc-anchor pin still REQUIRES the false phrase "bound outside this
  command line", so it cannot be corrected without touching that test.
- **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85` — carried forward untouched**,
  open and unaccepted at their original severities, all below `high`. Plans 19-14
  and 19-15 touched `policy.rs`, `hooks.rs` and four test files only;
  `cred.rs`, `mod.rs`, `advisory.rs`, `scan.rs` and `config.rs` are untouched, so
  `T-19-63` and `T-19-65` … `T-19-73` cannot have moved.

### A fail-open seam in Rule B's post-filter — flagged, not counted

`resolve_program_with_head` (`src/envelope/policy.rs:2043`) is implemented as a
post-filter over `resolve_program`, and its match is:

```text
Governed { .. } | NestedPayload { .. }  =>  Refuse(EnvelopeAssertionFailed)
other                                   =>  other
```

The `other` arm is a **wildcard**. Today the remaining variants are `NoProgram`,
`Ungoverned` and `Refuse`, and passing those through is correct and documented.
But a future `ProgramResolution` variant meaning "this segment reaches a program
the envelope governs" — the kind of variant this resolver has already grown twice
— would compile, would pass a **severed head** silently, and no test would go
red. Rule B is the newest control in the file and this is the one place it can be
defeated by an addition rather than by a deletion. **Suggested fix: make the
match exhaustive**, one arm per variant with a comment on each, so adding a
variant is a compile error (E0004) rather than a quiet permit — the discipline
`T-19-45` already establishes elsewhere in this phase for `PermissionMode`.

Audit 4 does **not** register this as a threat: it is not a live bypass, it is a
regression surface. It is recorded here because the wrapping direction was an
execution-time judgement call (`19-15-SUMMARY.md` deviation 3) and this is its
one cost.

**On that judgement call itself: endorsed.** There is exactly one implementation
of the step machinery (`policy.rs:1771-1986`) and one scan;
`resolve_program_with_head` calls it once. The stated reason is real and
checkable — `resolve_programs_own_doc_still_discloses_the_residual_it_does_not_cover`
(`tests/envelope_wrapper_class.rs:1644`) extracts the disclosure with
`doc_comment_above(POLICY_SOURCE, "pub fn resolve_program(")`, so inverting the
definition would have moved the disclosure off the anchor that control depends
on.

---

## Audit 4 — what the round-3 controls can and cannot fail on

The mandate, as in audit 3, was not "do the claimed mitigations exist" but
**"what class can the new controls not fail on?"** The answer is recorded as a
method and a boundary, so audit 5 can repeat it rather than rediscover it.

### The principle round 3 established DOES hold

A decision region must be derived from the same scan the classifier runs for that
decision, never a second scan. Audit 4 probed this specifically and found no cell
one slot over: `expansion_in_decision_region` takes the git verb from
`scan_leading`, `config`'s key from `config_key_operand_index`, the forge's first
two words from `subcommand_word_indices`, and the `gh api` endpoint from
`scan_gh_api` — the arm's own walk, which is the cell that had already drifted.
`subcommand_words` is defined over `subcommand_word_indices`,
`gh_api_posts_a_pull_request` over `scan_gh_api`, `classify_config` over
`scan_config`, and `split_segments` over `split_segments_with_heads`. Every
adjacent slot audit 4 probed is refused, including the forge's SECOND subcommand
word, the `-`-initial `api` flag in both spellings, the displaced endpoint in the
`-f` and `-H` spellings, `-X`/`--method` in all four spellings and a
marker-initial `api` decision word. **This is the first round of the four whose
declared mechanism survived the audit intact.**

### Where the boundary now is, in one paragraph

**Rule A decides on one bit. Rule B decides on one geometry.** Rule A refuses a
decision word carrying `Token.expansion`, which `tokenize` sets only for an
unquoted `$` or a backtick. Rule B refuses a segment whose immediately preceding
operator is a `}` or `)` that severed a word already in progress — closers only,
by design, because the opener exclusion is what keeps `echo $(git rev-parse
HEAD)` and `ROOT=$(git rev-parse --show-toplevel)` working. Between them they
cover the case where the shell assembles a word from a **parameter expansion or a
command substitution**, and they cover it well.

**They cannot fail on the other ways bash makes a word**, none of which sets
either signal:

- **brace expansion** — `{a,b}`, `{1..9}`: sets no expansion bit, and its opener
  is excluded by Rule B (`T-19-92`);
- **pathname expansion** — `*`, `?`, `[…]`: sets no expansion bit (`T-19-94`);
- **tilde expansion** — `~`, `~user`: sets no expansion bit;
- **`$IFS`-driven re-splitting** of a word already expanded, which changes the
  argv the shell finally builds after the guard has answered;
- **anything bash constructs after the guard answers at all** — a script written
  to a file and then run, a `-c` payload assembled at run time, `eval`. The last
  two are refused by name; the first is disclosed in `split_command`'s doc and is
  not new.

And separately from word assembly: neither rule touches a **whole command line
handed to a governed program as data**, which is `T-19-86`, unchanged.

**The one-sentence version for the next round.** The corpus has now varied *where*
the gap is three times and *what makes a word unreadable* once, and both times
the answer was a cell it could not draw: `T-19-89` was "no alphabet contains a
`$`", `T-19-95` is "no alphabet contains a `{a,b}` or a `*`". The alphabets model
`$`-shaped assembly and nothing else, so any control certified by them is
certified against `$`-shaped assembly and nothing else.

### Suggested closure, in order — (d) FIRST

1. **(d) — widen the corpus BEFORE certifying anything.** Extend
   `EXPANSION_METACHARACTERS` beyond `['$', '`', '{', '(']` to the characters that
   make a word unreadable rather than only the ones that mark an expansion — at
   minimum a comma inside braces and `*`, `?`, `[` — and add entries carrying them
   to `ASSIGNMENT_PREFIXES`, `WRAPPERS`, `REFUSED_BASES`, `DECOY_OPERANDS`,
   `EXPANSION_WRAPPERS`, `SHELL_LAYERS` and `SEVERED_PREFIXES`, with the
   `MIN_*`-floor pattern already in the file asserting the corpus generates them.
   **This is listed first on purpose.** Three rounds running, the gap has been the
   cell one slot over from what the corpus could draw; a fifth round certified by
   an alphabet that cannot draw a comma-in-braces would be the fifth consecutive
   round to certify a claim it could not have failed on, and the first three of
   those were each found by the NEXT audit rather than by the round's own
   evidence.
2. **(a) — `T-19-92`.** Discriminate a brace GROUP from a brace EXPANSION by
   whether the current segment is empty when `{` arrives — bash's own rule is that
   `{` is the reserved word only at a command position — and treat a splice back
   into an enclosing simple command as making that whole command unresolvable
   rather than as N independent segments. Do **not** delete the characters from
   `SEPARATORS`. The paired cost must be pinned from both sides exactly as Rule
   B's was: `{ git status; }` and `( git status )` must keep reaching their
   ungrouped verdicts.
3. **(b) — `T-19-93`.** Either teach the forge scans `gh`'s `{owner}`/`{repo}`/
   `{branch}` placeholders so `repos/{owner}/{repo}/pulls` reaches
   `endpoint_is_pulls` intact, or stop the tokenizer fragmenting a brace pair that
   contains no comma. The correctness bar here is not "refuse" but **"count"** —
   an uncounted pull request is the failure mode, so the fix must be verified by
   WALKING the envelope root and finding a ledger line, not by an exit code.
4. **(c) — `T-19-94`.** Treat an unquoted glob metacharacter in a decision word
   exactly as `Token.expansion` is treated. The cost is bounded and should be
   measured the same way Rule A's was: an operand carrying a glob
   (`git add src/*.rs`, `rg "x" src/*`) must keep working, because only the
   decision region is in scope.

Then, and separately from the four above: correct the `T-19-91` `git push $REF`
row wherever it appears, and make `resolve_program_with_head`'s post-filter
exhaustive.

---

## Audit 4 Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log — twelve, unchanged by
      round 4; audit 4 accepts nothing new
- [x] Every closure re-measured against the built binary with a fresh envelope
      root per row and the directory walked afterwards
- [x] Every new finding confirmed under `bash` against argv-printing shims;
      three candidates discarded because the shell did not run what the guard
      permitted
- [x] Plan 19-13's, 19-14's and 19-15's appended subsections left byte-identical
- [ ] `threats_open: 0` confirmed — **4 open at `high`: `T-19-86`, `T-19-91`,
      `T-19-92`, `T-19-93`**
- [ ] `status: verified` set in frontmatter

**Approval: blocked, 2026-08-29 (audit 4).**

**Not accepted here.** `T-19-92` and `T-19-93` are high-severity, empirically
confirmed bypasses — one defeating layers 1, 2 and 3 on a single line with the
credential intact and the `-c` precedence re-measured on this machine, the other
defeating the SAFE-06 cap through `gh`'s own documented syntax. Both have rows
with **no second carrier**. `T-19-86` and `T-19-91` remain open at `high` by
scoping decision and by round discipline respectively. Accepting any of the four
is a human decision and this audit does not make it.

**Progress is real and it is the right kind.** Both of audit 3's own blocking
findings are closed, and closed *structurally* rather than row by row: `T-19-88`
by restoring the expansion bit at the decision boundary with every index taken
from the scan that guards it, and `T-19-87` by a positional rule that reads no
name, no substring and no length. Neither was certified by the other's evidence —
19-14 left two rows RED on purpose and 19-15 confirmed them still RED before
writing a line. Two execution-time judgement calls caught real defects that three
review rounds had not: the carry-forward split, without which both plans' gates
were arithmetically unsatisfiable, and the substitution of the bare-brace row,
which was about to be a control that could not fail on its own class. **The open
set has shrunk in mechanism and grown in precision for the second round running.**
What remains is one residual deferred by explicit decision (`T-19-86`), one
registered honestly and now measured wider than it was written (`T-19-91`), and
two the audit found in the one place four rounds of alphabets have never looked.

---

## Execution record — plan 19-16 (the corpus, RED). NOT an audit finding.

**Provenance, stated first.** This subsection was written by the EXECUTOR of
plan 19-16. It is not audit 4's, it re-measures nothing on audit 4's behalf, and
it edits nothing above it — no audit table, no Security Audit Trail entry, no
Accepted Risks Log row, no sign-off, and none of the subsections plans 19-13,
19-14 and 19-15 appended. Re-measuring and re-classifying those rows is
`/gsd-secure-phase 19`'s job; an audit's own tables are its provenance.

**What plan 19-16 is.** The corpus and the reproducers for round 5, written and
committed RED **before** a single line of the rule exists. Plan 19-17 writes the
rule. The split is deliberate: four consecutive rounds certified a control with a
corpus structurally incapable of failing on the class that got through it, and
the first three were each found by the NEXT audit rather than by the round's own
evidence. **This plan closes nothing.**

### What was measured, and how

Every row was driven at `e842fa3` against the built binary as

```
printf '<PreToolUse JSON>' | GSD_MM_ENVELOPE_ROOT=$(mktemp -d) \
  ./target/debug/gsd-meta-manager envelope guard alpha
```

one fresh envelope root per row, with the whole envelope directory walked
afterwards so a missing ledger line is OBSERVED rather than inferred. Every
claimed permit was then re-run under `bash` against shims named `git`, `gh` and
`glab`, first on `PATH`, printing their own argv. The JSON was built with
`json.dumps` rather than by hand.

**All fourteen audit-4 rows reproduced at their recorded verdicts.** No audit-4
row failed to reproduce, so no finding of that kind is reported.

```
exit=0  git {-c,core.hooksPath=/dev/null,push,--force,origin,main}   empty walk
exit=0  git {push,--force} origin main
exit=0  git {update-ref,-d,refs/heads/main}
exit=0  git {config,core.hooksPath,/tmp/x}
exit=0  gh {pr,create} --title x                                     empty walk
exit=0  gh {api,repos/o/r/pulls} -f title=x                          empty walk
exit=0  glab {mr,create} --title x                                   empty walk
exit=0  bash -lc "git {push,--force} origin main"
exit=0  gh api repos/{owner}/{repo}/pulls -f title=x                 empty walk
exit=0  git pus? --force origin main
exit=0  git ?ush --force origin main
exit=0  git stas?
exit=0  gh p? create --title x                                       empty walk
exit=0  touch push && git pus? --force origin main
```

Walked listings, measured rather than described. After a REFUSED forge row the
root is empty; after a PERMITTED `gh pr create` it holds one file:

```
refused  gh {pr,create} --title x        ->  (the envelope root is empty)
counted  gh pr create --title x          ->  alpha/pr-ledger.ndjson (112 bytes), 1 ledger line
counted  gh api "repos/{owner}/{repo}/pulls" -f title=x
                                         ->  alpha/pr-ledger.ndjson (112 bytes), 1 ledger line
         and the SECOND call in that same root -> exit 2, pr_cap_exceeded
```

### The cells found while PLANNING round 5 — same provenance caveat 19-14 set

Found while planning, not by an audit. Measured at `e842fa3`, one fresh root
each, argv from the `bash` shims beside each:

```
exit=0  {git,push,--force,origin,main}              ARGV[git]: [push][--force][origin][main]
exit=0  {env,git} push --force origin main          ARGV[git]: [push][--force][origin][main]
exit=0  {g..g}it push --force origin main           ARGV[git]: [push][--force][origin][main]
exit=0  {g..g}{i..i}t push --force origin main      ARGV[git]: [push][--force][origin][main]
exit=0  {g..g..1}it push --force origin main        ARGV[git]: [push][--force][origin][main]
exit=0  {g..g}"it" push --force origin main         ARGV[git]: [push][--force][origin][main]
exit=0  {g..g}h pr create --title x                 ARGV[gh]:  [pr][create][--title][x]
exit=0  git {--no-pager,push} --force origin main   ARGV[git]: [--no-pager][push][--force][origin][main]
exit=0  git {stash,-q}                              ARGV[git]: [stash][-q]
exit=0  git push origin refs/heads/gsd-auto/alpha/w --forc{e,e}
                                                    ARGV[git]: [push][origin][refs/…/w][--force][--force]
exit=0  git commit -m {a,b}                         ARGV[git]: [commit][-m][a][b]
exit=0  git config core.hooksPat? /tmp/x
exit=0  gh api repos/o/r/pul?s -f title=x                                    empty walk
exit=0  git ~push --force origin main               ARGV[git]: [~push][--force][origin][main]
exit=2  X=push,--force; IFS=,; git $X origin main   [envelope_assertion_failed]
```

### Two findings of this plan, recorded rather than smoothed over

**1. A comma list produces N WORDS, so most of the "concatenated splice"
spellings are class COVERAGE and not reproduced harms.** The plan and audit 4's
successor material describe eight concatenated/multi-expansion/nested/quoted
spellings as "assembled by bash into a real force push". Measured under the
shims, that is true of FOUR of them and false of the rest:

```
GENUINE   {g..g}it …          ARGV[git]: [push][--force][origin][main]
GENUINE   {g..g}{i..i}t …     ARGV[git]: [push][--force][origin][main]
GENUINE   {g..g..1}it …       ARGV[git]: [push][--force][origin][main]
GENUINE   {g..g}"it" …        ARGV[git]: [push][--force][origin][main]

COVERAGE  g{i,i}t …           ARGV[git]: [git][push]…    real git: 'git' is not a git command
COVERAGE  {g,g}{i,i}{t,t} …   ARGV[git]: [git]x7 [push]…
COVERAGE  {g{i,i}t,x} …       ARGV[git]: [git][x][push]… real git: 'x' is not a git command
COVERAGE  "g"{i,i}"t" …       ARGV[git]: [git][push]…
COVERAGE  g{it,x} …           ARGV[git]: [gx][push]…
COVERAGE  {g,x}it …           ARGV[git]: [xit][push]…
COVERAGE  {g,x}{h,h} pr …     ARGV[gh]:  [gh][xh][xh][pr]… real gh: unknown command "gh" for "gh"
```

A brace expansion carrying a comma list of N alternatives produces N words, so a
concatenated comma splice in a PROGRAM slot always leaves a surplus word in the
subcommand slot that the real tool rejects. Only a RANGE whose endpoints are
equal produces exactly one word. The comma spellings are kept in the corpus and
are refused by the same rule — the guard is blind to them identically and one
character separates them from the range spellings — but they are **labelled
COVERAGE in `tests/envelope_literal_decision.rs`, row by row, with the printed
argv**, so a later reader cannot mistake one for a reproduced force push. This is
audit 4's own discard discipline applied to rows audit 4 did not measure.

**The one exception is the FLAG slot**, where a surplus word is harmless because
git accepts a repeated `--force`:
`git push origin refs/heads/gsd-auto/alpha/w --forc{e,e}` is a GENUINE
concatenated COMMA splice, measured exit 0 with its literal control at exit 2
under `force_push_blocked`, and it consults no repository.

**2. `git push {--force,origin} main` is CWD-DEPENDENT, and out of namespace it
is masked by an unrelated arm.**

```
project root outside the namespace (this checkout, on master)
  -> exit 2, push_outside_namespace   — the braces fragment the line into a
     `git push` with no refspec, and the no-refspec arm refuses it for a reason
     that has nothing to do with the splice
project root INSIDE the namespace (refs/heads/gsd-auto/alpha/work, upstream
configured — the state a driven run is DESIGNED to be in)
  -> exit 0
```

Both spellings assemble `ARGV[git]: [push][--force][origin][main]`. So it is a
live bypass in the configuration the envelope exists for. It is pinned in the
IN-NAMESPACE configuration with the repository passed to `guard_in` as an
explicit parameter, so the pin does not depend on the test process's own working
directory (`T-19-80`), and it is pinned from its other side out of namespace with
an assertion that accepts either refusal identifier.

### Rows DISCARDED, recorded so audit 5 does not spend the measurement again

None this round. Every candidate the plan named was assembled by `bash` into a
command that really reaches the governed program; the eleven rows above that are
labelled COVERAGE reach it and are refused by the same rule, but the argv the
shell finally builds is not itself destructive, which is a weaker claim than
audit 4's three discards (`{ git,push }` a syntax error, `sh -c '…'` not
brace-expanding under `dash`, and real `gh` rejecting `-R` as a shorthand).

### `T-19-93` — the bar is COUNT, and it is asserted as a ledger line

`{owner}`/`{repo}` are `gh`'s own documented placeholders and bash passes them
through byte-identically: `printf "[%s]" repos/{owner}/{repo}/pulls` prints
`[repos/{owner}/{repo}/pulls]`, measured. Refusing the line is not the fix — an
agent following `gh`'s manual would be denied, which is how a safety control gets
switched off (AR-19-11). So the assertions written here are:

* the unquoted placeholder form is PERMITTED **and** the walked root holds
  exactly one ledger line — RED today, measured exit 0 with an empty walk;
* a SECOND placeholder-form creation in the SAME root is refused under
  `pr_cap_exceeded` — the one assertion an implementation cannot satisfy by
  refusing the first;
* the quoted spelling is the POSITIVE control and passes today: exit 0 with one
  ledger line, second call exit 2 `pr_cap_exceeded`;
* `…/pulls/7` is the boundary control and passes today: exit 0 with an EMPTY
  walk in both the quoted and unquoted spellings, because a single pull request
  is not the collection. The placeholder tolerance therefore cannot be
  implemented as "any endpoint containing braces counts".

### `T-19-91` — the in-namespace measurement, recorded; the threat stays OPEN

Audit 4's correction reproduced in a purpose-built fixture on
`gsd-auto/alpha/work` with a local bare upstream configured, the repository
passed to the guard explicitly:

```
in namespace      git push $REF          -> exit 0                        (audit 4 confirmed)
in namespace      git push origin $REF   -> exit 2 push_outside_namespace (still fails closed)
out of namespace  git push $REF          -> exit 2 push_outside_namespace
                  git reflog $S          -> exit 0
                  git reflog show $S     -> exit 0
                  git symbolic-ref $S    -> exit 0
                  git symbolic-ref HEAD $R -> exit 2 force_push_blocked
```

**`T-19-91` remains OPEN at `high`.** No decision-operand rule was added for
`reflog`, `symbolic-ref` or `push`; the denylist was not extended; the code-side
record correction is plan 19-17's work. The second-carrier asymmetry the
registration draws still holds and is restated here so nobody reads a narrowed
threat as a covered one: `git push` has `pre-push` behind it, while `git reflog`
and `git symbolic-ref` have no `pre-push` and no `pre-commit` — git runs no hook
for either.

### `T-19-96` — REGISTERED, measured, and deliberately NOT fixed

```
exit=0  git push --forc? origin refs/heads/gsd-auto/alpha/w   <- a glob in a PUSH FLAG
exit=2  git push --force origin refs/heads/gsd-auto/alpha/w   [force_push_blocked]
```

Invariant under the working directory: measured identically with and without an
in-namespace project root, because the explicit refspec means no push context is
resolved. `19-14`'s git decision region is the VERB plus `config`'s key operand;
`classify_push`'s FLAGS are a further arm, exactly as `classify_reflog`'s and
`classify_symbolic_ref`'s operands are.

**Registered rather than fixed on ROUND DISCIPLINE, not on blast radius.** It was
found while planning round 5, and a plan cannot both discover a threat and be the
plan that measured it fail first — the discipline `19-14` established for
`T-19-91` and `19-13` for `T-19-86`. Blast radius is explicitly not the argument:
`git push` does have `pre-push` behind it, which `git stash` and
`git update-ref` do not, but that is a narrowing and not a covering. Pinned at
its measured verdict in
`tests/envelope_literal_decision.rs::the_t_19_96_push_flag_glob_is_measured_and_registered_rather_than_fixed`,
so if plan 19-17's rule happens to reach it the pin turns red and the change is
disclosed rather than absorbed. Also registered in `deferred-items.md`.

### What plan 19-16 leaves RED, by design

`passed + failed` over `cargo test --no-fail-fast` moves from **1533** to
**1580**; the increase is exactly the 47 new `#[test]` functions this plan adds
(41 in `tests/envelope_literal_decision.rs`, 6 in
`tests/envelope_wrapper_class.rs`). 26 of them are RED and every name is listed
in `19-16-SUMMARY.md`. The only other failures in the run are the two
pre-existing flaky `tests/driver_reattach.rs` names, which are documented in
`deferred-items.md` and out of scope. `--test envelope_expansion_slots` (32),
`--test envelope_command_position` (18) and `--lib envelope` (184) are fully
green — the widening did not disturb round-4's evidence.

### What this plan CLOSES: nothing

`T-19-92`, `T-19-93`, `T-19-94` and `T-19-95` are all **open** at this plan's
end. `T-19-95` in particular is closed only when plan 19-17's rule is certified
by the corpus written here, because a corpus is evidence about a control and
there is no control yet.

**`/gsd-secure-phase 19` is NOT cleared by this plan, by plan 19-17, or by the
two together.** `T-19-86` remains OPEN at `high` by explicit user scoping
decision and its four rows still exit 0; `T-19-91` remains OPEN at `high` and was
widened, not remedied. `T-19-61` … `T-19-73`, `T-19-84` and `T-19-85` are open,
unaccepted and untouched — `cred.rs`, `advisory.rs`, `scan.rs` and `config.rs`
were not opened. Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed;
`T-19-86` and `T-19-91` are both sub-classes of `T-19-60` and both remain open at
`high`.

**No `src/` file was modified by any of this plan's three commits**, and neither
`Cargo.toml` nor `Cargo.lock` was touched (`T-19-SC`).

---

## Execution record — plan 19-17 (the inverted rule)

**This subsection is an EXECUTION RECORD made by plan 19-17, not an audit
finding.** It appends to this document and edits nothing above it: no audit
table, no Security Audit Trail entry, no Accepted Risks Log row, no sign-off,
and none of the appended subsections of plans 19-13, 19-14, 19-15 or 19-16.
Re-measuring and re-classifying these rows is `/gsd-secure-phase 19`'s job.

### Read this first: the gate is NOT cleared

**`/gsd-secure-phase 19` is NOT cleared by this plan.**

* **`T-19-86`** — OPEN at `high`, by explicit user scoping decision. Untouched.
  All four rows re-measured at exit 0 against the built binary.
* **`T-19-91`** — OPEN at `high`. Its code-side RECORD is corrected here; the
  remedy is unchanged and no decision-operand rule was added.
* **`T-19-96`** — registered open, pinned at its measured verdict, not fixed.
* **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85`** — open, unaccepted, untouched.
* **`T-19-74`** — accepted (AR-19-10); core rows re-measured permitted.

Only the **wrapper-operand** sub-class of `T-19-60` is closed. `T-19-86` and
`T-19-91` are both sub-classes of it and both remain OPEN at `high`.

### The definition of LITERAL, as implemented

A decision word must be **LITERAL — the shell hands it to the program
byte-identically to how it is written.** The evidence is POSITIVE and is
collected by `policy::tokenize` while the word is consumed, as `Token.literal`.
It is never a test on the recovered text, because that text has had its quoting
removed and could not tell `gh api "repos/{owner}/{repo}/pulls"` — literal, and
counted before this plan — from a word the shell rewrites.

The bit is cleared when the tokenizer consumes, **outside quotes**:

| class | characters | why it is not literal |
|---|---|---|
| expansion | `$`, `` ` `` | parameter, command and arithmetic expansion, and every `$IFS` re-split of the result |
| pathname | `*`, `?`, `[` | the result depends on the working directory, so it is unknowable **whether or not a file matches today** — a precondition an agent satisfies with `touch push` in the same tool call |
| tilde | `~` | the result depends on the passwd database of the machine the command will run on |
| brace | a `{`…`}` pair classified as an EXPANSION, or a `{` with no match | bash splices its alternatives back into the enclosing command, so the word never exists as written |

Inside DOUBLE quotes only `$` and `` ` `` clear it. It is **deliberately not
cleared** for anything inside single quotes, for a backslash-escaped character
(escaping is exactly what makes a character literal), or for a `{`…`}` pair with
no comma and no range, which bash passes through unchanged.

It is **not a second `expansion`**: `Token.expansion` is unchanged, because
`resolve_program` steps 3 and 5 decide on it and this plan did not move them.

### The two clauses

**Clause 1 — the word.** `first_unreadable_decision_word` (renamed from
`expansion_in_decision_region`) reads the bit in the **one closure** that read
`Token.expansion`. Every clause above it is unchanged and every index is still
reported by the scan the classifier itself runs. **The decision region did not
move.**

**Clause 2 — the simple command**, folded into `resolve_program_with_head`. A
command a brace expansion splices into is refused when **(a)** a segment
resolves `Governed`/`NestedPayload`, **or (b)** a word the splice can PRODUCE has
a governed basename, or the products cannot be enumerated. The product scan is a
**whole-word** scan reaching to the next unquoted whitespace or real command
operator, **never stopping at a `{` or `}`**, joining literal runs with their
**quoting removed**.

### Every `19-16` RED row, before and after

Measured against `./target/debug/gsd-meta-manager envelope guard alpha`, one
fresh `GSD_MM_ENVELOPE_ROOT` per row, the envelope directory walked afterwards.
"before" is `19-16`'s recorded measurement at `09e83bd`, re-confirmed by this
plan's RED run before any production line moved.

#### `T-19-92` — clause 2(a): a segment resolves `Governed`

| command | before | after | reason id | walk |
|---|---|---|---|---|
| `git {-c,core.hooksPath=/dev/null,push,--force,origin,main}` | 0 | **2** | `envelope_assertion_failed` | empty |
| `git {push,--force} origin main` | 0 | **2** | `envelope_assertion_failed` | empty |
| `git {update-ref,-d,refs/heads/main}` | 0 | **2** | `envelope_assertion_failed` | empty |
| `git {config,core.hooksPath,/tmp/x}` | 0 | **2** | `envelope_assertion_failed` | empty |
| `git {stash,-q}` | 0 | **2** | `envelope_assertion_failed` | empty |
| `git {--no-pager,push} --force origin main` | 0 | **2** | `envelope_assertion_failed` | empty |
| `git push origin refs/heads/gsd-auto/alpha/w --forc{e,e}` | 0 | **2** | `envelope_assertion_failed` | empty |
| `gh {pr,create} --title x` | 0 | **2** | `envelope_assertion_failed` | empty |
| `gh {api,repos/o/r/pulls} -f title=x` | 0 | **2** | `envelope_assertion_failed` | empty |
| `glab {mr,create} --title x` | 0 | **2** | `envelope_assertion_failed` | empty |
| `bash -lc "git {push,--force} origin main"` | 0 | **2** | `envelope_assertion_failed` | empty |
| `git push {--force,origin} main` (in namespace) | 0 | **2** | `envelope_assertion_failed` | empty |

The last row was cwd-dependent before (exit 0 in namespace, exit 2 out of it
under an unrelated arm). It is now refused before any push context is resolved,
so the verdict no longer depends on the repository.

#### `T-19-92` — clause 2(b): NOTHING in the line resolves `Governed`

The head word is `it`, `g`, `t`, or a comma list naming nothing. No
alternative-NAME test reaches any of these, and the composed word never exists
as a token.

| command | before | after | how it is reached |
|---|---|---|---|
| `{git,push,--force,origin,main}` | 0 | **2** | products contain `git` |
| `{env,git} push --force origin main` | 0 | **2** | products contain `git` |
| `{g..g}it push --force origin main` | 0 | **2** | concatenated: product `git` |
| `g{i,i}t push --force origin main` | 0 | **2** | concatenated: product `git` |
| `g{it,x} push --force origin main` | 0 | **2** | concatenated: product `git` |
| `{g,x}it push --force origin main` | 0 | **2** | concatenated: product `git` |
| `{g..g}{i..i}t push --force origin main` | 0 | **2** | **multi-expansion**: composed ACROSS both `{`s |
| `{g,g}{i,i}{t,t} push --force origin main` | 0 | **2** | **multi-expansion**: 8 products, all `git` |
| `{g..g..1}it push --force origin main` | 0 | **2** | **UNENUMERABLE**: increment range |
| `{g{i,i}t,x} push --force origin main` | 0 | **2** | **UNENUMERABLE**: nested `{` in an alternative |
| `"g"{i,i}"t" push --force origin main` | 0 | **2** | quoted runs, joined **quote-removed** |
| `{g..g}"it" push --force origin main` | 0 | **2** | quoted run, joined **quote-removed** |
| `{g..g}h pr create --title x` | 0 | **2** | product `gh`; walk empty |
| `{g,x}{h,h} pr create --title x` | 0 | **2** | product `gh`; walk empty |

Rows 7–8 are why the scan is whole-word rather than per-`{`: a per-`{`
computation answers `g` and `it` for `{g..g}{i..i}t`, neither governed and both
enumerating cleanly. Row 10 is why a nested `{` is a fail-closed trigger rather
than a recursion. Rows 11–12 are why the runs are quote-removed before they are
joined.

#### `T-19-94` and the glob decision-operand cells — clause 1

| command | before | after | decision word |
|---|---|---|---|
| `git pus? --force origin main` | 0 | **2** | the git verb |
| `git ?ush --force origin main` | 0 | **2** | the git verb |
| `git stas?` | 0 | **2** | the git verb (no second carrier) |
| `touch push && git pus? --force origin main` | 0 | **2** | the git verb of the second segment |
| `gh p? create --title x` | 0 | **2** | a forge subcommand word; walk empty |
| `git config core.hooksPat? /tmp/x` | 0 | **2** | the `git config` key operand |
| `gh api repos/o/r/pul?s -f title=x` | 0 | **2** | the `gh api` endpoint; walk empty |
| `gh api repos/o/r/pulls -? title=x` | 0 | **2** | a `gh api` word whose flag-ness decides |
| `git ~push --force origin main` | 0 | **2** | the git verb (CLASS COVERAGE, not a measured bypass) |

The `-?` row is a **finding of this execution**, not of `19-16`'s enumerated
corpus: it was caught by the generative property
`a_forge_decision_slot_carrying_a_splice_or_a_glob_is_refused_in_every_generated_slot`
after clauses 1 and 2 had landed. `api_flag_ness_is_unreadable` is a textual
INDEX-selection predicate and still knew only `$` and a backtick, so a `?` or a
`{` in a flag marker put the word in no part of the region at all. It is widened
to the same class the bit covers, for the same reason `scan_leading`'s `-c` key
half is. That is the axis-vs-enumeration value `19-16`'s widening was for.

### `T-19-93` — closed by COUNT, and the evidence is a walked LEDGER LISTING

Closed by the tokenizer's literal-brace branch and by nothing else. **Neither
forge scan was changed.**

```
gh api repos/{owner}/{repo}/pulls -f title=x   (one fresh envelope root, walked)
  call 1 -> exit 0
    alpha/pr-ledger.ndjson (112 bytes)      ledger lines found: 1
  call 2 -> exit 2  reason pr_cap_exceeded
    alpha/pr-ledger.ndjson (224 bytes)      ledger lines found: 2

gh api "repos/{owner}/{repo}/pulls" -f title=x   (the positive control)
  call 1 -> exit 0    1 ledger line
  call 2 -> exit 2    pr_cap_exceeded

gh api repos/{owner}/{repo}/pulls/7 -f body=x    (the BOUNDARY)
  call 1 -> exit 0    ledger lines found: 0
  call 2 -> exit 0    ledger lines found: 0
```

Before this plan the unquoted form was exit 0 with an **empty walk** on both
calls — the SAFE-06 cap BYPASSED rather than exceeded, with no second carrier
(`T-19-35`). It now behaves byte-for-byte like the quoted spelling, and the
`…/pulls/7` boundary still writes nothing, so the tolerance did not degrade into
"any endpoint with braces counts".

**Why the tokenizer rather than the forge scans**, recorded so a later reader can
check the reasoning rather than the outcome. It is where the defect is: the
braces are literal in bash and the tokenizer was wrong about them, so a tolerance
in `endpoint_is_pulls` would be a second consumer compensating for a splitter
that mangles the word — the shape this phase produced four times. It generalises:
the same spelling can appear in a git verb, a `config` key operand and a `-c`
assignment, and a scan tolerance fixes one cell. And it is free: the tokenizer
had to learn to tell a brace expansion from a brace pair anyway for `T-19-92`.

### What the inversion NEWLY REFUSES, each with its permitted twin and clause

| refused after 19-17 | clause | permitted twin (measured, before AND after) |
|---|---|---|
| `git commit -m {a,b}` | 2a | `git commit -m ab` — exit 0 |
| `git add {src,tests}/x.rs` | 2a | `git add src/x.rs tests/x.rs` — exit 0 |
| `rg "git status" {src,tests}` | 2a | `rg "git status" src/` — exit 0 |
| `echo {git,x}` | **2b's own over-refusal** | `echo git` — exit 0 |
| `git config --get-regexp branch.*` | 1 | `git config --get-regexp 'branch.*'` — exit 0 |
| `git -c 'user.na*e=x' commit -m y` | 1 (textual) | `git -c user.name="$NAME" commit -m x` — exit 0 |
| `gh api repos/o/r/pulls -? title=x` | 1 | `gh api repos/o/r/pulls -f title="$T"` — exit 0 |

**`ls {git,svn}-repo` is PERMITTED — measured exit 0 — and it is the control that
makes clause 2(b) a PRODUCT test rather than a MENTION test.** Its products are
`git-repo` and `svn-repo`, neither of whose basename is governed. A rule that
refused it would be testing for a mention, and a mention test is one slot away
from the class.

The whole allow corpus is re-measured at exit 0: `echo {a,b}`,
`mkdir -p {src,tests}`, `cp x{,.bak}`, `ls *.rs`, `rg "x" src/*`,
`git add src/*.rs`, `cd ~/projects`, `git commit -m "use ${HOME} here"`,
`gh pr create --title 'fix $PATH handling'`, `git config user.email "$EMAIL"`,
`echo $(git rev-parse HEAD)`, `ROOT=$(git rev-parse --show-toplevel)`,
`{ git status; }`, `( git status )`, `(git status)&&git fetch origin`,
`git log -1 HEAD@{0}` and `git reflog show HEAD@{0}`. The grouping refusals still
fire: `{ git push --force origin main; }`, `( git push --force origin main )` and
`git reflog delete HEAD@{0}` are all exit 2 under `force_push_blocked`.

### The two rows `19-16` measured but could not derive

| row | pre-fix | post-fix | classification |
|---|---|---|---|
| `FOO={a,b} git status` | 2 `envelope_assertion_failed` | 2 `envelope_assertion_failed` | **INTERACTION, not a new cost.** Rule B's geometry still holds — `}` severs the word and `head_is_command_position` is `false` — and clause 2(a) reaches the same segment. Asserted over the MECHANISM as well as the verdict, because a verdict alone cannot tell "still an interaction" from "newly a cost" |
| `rg "git status" {src,tests}` | 0 | 2 `envelope_assertion_failed` | **A NEW COST, by clause 2(a).** The segment `rg "git status"` resolves `NestedPayload` and the simple command is brace-spliced. It is the widest cost this round adds: bash runs `rg "git status" src tests`, in which nothing governed executes |

### The exhaustive post-filter

`resolve_program_with_head` closes the fail-open seam audit 4 flagged but did not
register. It was `Governed | NestedPayload => Refuse, other => other`; a future
variant meaning "this segment reaches a program the envelope governs" would have
compiled, passed a severed head silently and turned no test red. Its five arms:

1. `Governed` — refuse on a severed head (Rule B) or a brace-spliced command (2a);
2. `NestedPayload` — the same, and this is what reaches `rg "git status" {src,tests}`;
3. `NoProgram` — pass, unless clause 2(b) fires;
4. `Ungoverned` — pass, unless clause 2(b) fires; this is the arm the whole
   PRODUCE class arrives on, since nothing in it resolves `Governed`;
5. `Refuse` — passes through WHOLE, so step 1's `HookBypassBlocked` is not
   overwritten by this one.

**No wildcard arm.** A sixth variant is now an E0004.

### The `T-19-91` correction — record only, threat stays OPEN at `high`

`resolve_program`'s fourth residual bullet argued that `push`'s "refspec operand
already fails CLOSED". That is corrected. The narrow claim — `git push origin
$REF` is refused — is right and was re-measured at exit 2. The **bare**
`git push $REF` is a different shape: `push_needs_resolved_dests` answers true,
so the verdict is resolved from a repository and is **cwd-dependent, measured at
exit 0 in a repository whose current branch is inside the envelope's namespace**,
which is the state a driven run is designed to be in.

So `classify_push`'s refspec operand is a **third arm of `T-19-91`'s shape**
beside `classify_reflog`'s and `classify_symbolic_ref`'s. `T-19-91` is **not
closed, not renumbered, not re-scoped**, and the second-carrier statement is
unweakened: `push` has `pre-push` behind it while `reflog` and `symbolic-ref`
have no `pre-push` and no `pre-commit` — git runs no hook for either. The operand
rows are re-measured unchanged: `git reflog $S`, `git reflog show $S` and
`git symbolic-ref $S` all exit 0; `git symbolic-ref HEAD $R` exits 2.

### Rule B is still load-bearing, asserted mechanically

The parameter-expansion case (a `{` immediately preceded in-word by an unquoted
`$`) keeps today's behaviour byte-for-byte, and it exists for exactly this
reason. `rule_b_still_reports_a_severed_head_as_not_a_command_position` asserts
over `policy::split_segments_with_heads` directly rather than over an exit code,
and it is green and unmodified: `C=GIT_CONFIG; env -u ${C}_COUNT git fetch
origin` still reports `head_is_command_position == false` on its last segment.
`SEPARATORS` is byte-for-byte unchanged, and `(` and `)` were not touched.

### Two findings about `19-16`'s test files

Both are recorded in the files themselves as well as here. Neither is fixable by
any production change, and neither is a defect in the rule.

1. **`MIN_UNREADABLE_FORGE_SLOT_CASES` was unreachable by construction.** It was
   50 against a stated arithmetic of "7 slots x 2 spellings x (1 or 4
   displacers) x 2 depths = 68", while the loop gives one displacer to six slots
   and four to `ApiDisplacedEndpoint` — `(6 + 4) x 2 x 2 = 40`. It was invisible
   until the rule landed, because the property failed earlier at its per-case
   refusal assertion. Corrected to the recounted 40. **No refusal assertion was
   touched and no alphabet was narrowed**; all 40 cases are refused with an empty
   walk.
2. **`the_marked_payload_splice_is_measured_pre_fix_and_deliberately_not_pinned_post_fix`
   pinned `exit 0` despite its name, its own comment and `19-16-SUMMARY.md` all
   saying it asserts nothing post-fix.** Its failure message scopes itself to
   "if this changes BEFORE `19-17` runs", so it was a handoff guard, and its
   contract was discharged by this plan's confirmed RED run at `09e83bd`. **The
   rule was not weakened to keep it green**: dropping `NestedPayload` from clause
   2(a) would have passed it and traded a disclosed false positive for a possible
   false negative in a clause the plan states twice as load-bearing.

### The gate

`rtk proxy cargo test --no-fail-fast`: **1584 passed, 0 failed, 13 ignored**.
`passed + failed` = **1584** = `19-16`'s recorded 1580 **plus the 4 new `#[test]`
functions** this plan adds — two in `policy.rs`'s own test module (the
brace-classification table with its per-WORD products column, and the
segment-level splice-fact pin) and two in `tests/envelope_literal_decision.rs`
(the two deferred cost rows). A red test RAN, so red→green leaves the total
unchanged and the increase is the new functions and nothing else. All 26 of
`19-16`'s RED names are green. `cargo build` and `cargo clippy -- -D warnings`
exit 0; `cargo clippy --all-targets` reports the same **four** pre-existing
warnings and no fifth. No crate was added and neither `Cargo.toml` nor
`Cargo.lock` was touched (`T-19-SC`).

---

## Threats found by audit 5 (2026-08-29, after plans 19-16 and 19-17)

**Provenance, stated first.** The two subsections above this one were written by
the EXECUTORS of plans 19-16 and 19-17 and are deliberately outside the audit
tables; audit 5 left them, and every earlier appended subsection, byte-identical.
Everything from here to the end of the file is **audit 5's own**, measured
against the built binary at `ad847b4` with a fresh `GSD_MM_ENVELOPE_ROOT` per
row and the envelope directory walked afterwards, and with every claimed bypass
re-run under `bash` against argv-printing `git`/`gh`/`glab` shims.

### The question this audit was set, answered plainly

**Did the inversion break the pattern, or is it enumerative after all?**

**The inversion is genuine, and it is the first control in this phase that is
not an enumeration of shell syntax.** `Token.literal` is positive evidence
collected while the word is consumed; clause 2 reads what a splice can PRODUCE
rather than what its alternatives are called. Audit 5 enumerated bash's word
expansions one at a time against the bit's trigger set and clause 2 and found
**no gap in word ASSEMBLY**: brace expansion, tilde, parameter, command and
arithmetic expansion, pathname expansion, `$IFS` re-splitting, ANSI-C `$'…'`
quoting and every concatenated, nested, ranged and quoted splice spelling are
covered, several of them by mechanisms the corpus never had to name. `T-19-92`,
`T-19-93`, `T-19-94` and `T-19-95` are closed on re-measurement, and `T-19-93`
is closed on the harder bar — COUNT, verified by a walked ledger listing.

**And the class is still open, because word assembly was never the whole
class.** The inversion answers *"is this word handed over as written"*. It does
not answer *"is this word handed over at all"*, and it does not answer *"does
the tokenizer's own quote removal agree with bash's"*. Two mechanisms sit
outside it, both measured live at `ad847b4`:

* a **shell redirection** is a word the guard reads and the shell **deletes**
  from argv, which displaces every decision word one slot right (`T-19-97`);
* a **backslash-newline line continuation** is two characters bash **deletes**
  and the tokenizer **keeps**, so the word the guard reads is not the word the
  program receives (`T-19-98`).

Neither is an expansion. Both words are perfectly literal by the bit's own test,
and correctly so — the bit is right about them and the model is what is
incomplete. **So the pattern held for a fifth round, but it moved axis rather
than moving one slot over:** rounds 1–4 were all "the cell beside the one the
corpus varies"; round 5's gap is a different question about the same boundary.
The honest summary is that the inversion closed the **word-assembly** class
completely and revealed that the boundary needs a second, symmetric invariant —
*the words the guard classifies must be exactly the words the program receives,
no more and no fewer* — of which literalness is one half.

### The new rows

| Threat ID | Category | Component | Severity | Disposition | Finding | Status |
|---|---|---|---|---|---|---|
| **T-19-97** | **Elevation of Privilege** | **`policy::tokenize` / `SEPARATORS` — a REDIRECTION is an ordinary word to the guard and is REMOVED from argv by the shell (`policy.rs:1175-1180`)** | **high** | **mitigate (unimplemented)** | **`SEPARATORS`' own doc records the decision: "`>` and `<` are deliberately absent: a redirection does not start a new command, so treating it as a separator would hide the command it redirects. It stays an ordinary word and travels into the classifier with the rest." It travels into the classifier and **displaces the decision word by one slot**. `scan_leading` skips only tokens beginning with `-`, so `>/dev/null` becomes the git VERB; `classify_git` finds it in no denylist arm and answers `Allow`. Measured at `ad847b4`, one fresh root per row, walk after: `git >/dev/null push --force origin main` → **exit 0**; `git 2>/dev/null push --force origin main` → **0**; `git 1>/dev/null …` → **0**; `git >>/tmp/x …` → **0**; `git <input.txt …` → **0**; `git <<<x …` → **0**; `git push>/dev/null --force origin main` → **0** (the operator need not even be a separate word); `git >/dev/null stash` → **0**; `git >/dev/null update-ref -d refs/heads/main` → **0**; `git >/dev/null reflog delete HEAD@{0}` → **0** (literal spelling refuses under `force_push_blocked`); `git >/dev/null config core.hooksPath /tmp/x` → **0**; `gh >/dev/null pr create --title x` → **0 with an EMPTY WALK on BOTH calls in one root**; `gh >/dev/null api repos/o/r/pulls -f title=x` → **0 with an EMPTY WALK**; `glab >/dev/null mr create --title x` → **0**; and through the nested payload `bash -lc "git >/dev/null push --force origin main"` → **0**, through a group `{ git >/dev/null push --force origin main; }` → **0**, and behind a sequence `echo hi && git >/dev/null push --force origin main` → **0**. Every row confirmed under `bash` against the shims: `git >/tmp/o 2>&1 push --force origin main` prints `ARGV[git]: [push] [--force] [origin] [main]`; `gh >/tmp/o api repos/o/r/pulls -f title=x` prints `ARGV[gh]: [api] [repos/o/r/pulls] [-f] [title=x]`. **The three-leg argument, each leg measured rather than argued.** Layer 1 — the line begins `git >`, which does not match the `Bash(git push:*)` prefix rule the agent CLI owns. Layer 2 — `git >/dev/null -c core.hooksPath=/dev/null push --force origin main` → **exit 0**, nothing written. Layer 3 — re-measured on this machine: `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=/ENV_WINS git >/tmp/l3out -c core.hooksPath=/CLI_WINS config --get core.hooksPath` prints **`/CLI_WINS`**, with the paired control (same line without `-c`) printing `/ENV_WINS`, so the command-line `-c` still outranks the envelope's env-injected `core.hooksPath` and no `pre-push` hook runs. Credentials — `GIT_ASKPASS`, `GIT_CONFIG_GLOBAL/SYSTEM` and `GIT_SSH_COMMAND` are untouched, so the push authenticates. That is the identical three-leg argument audit 2 used for `T-19-60`, audit 3 for `T-19-87` and audit 4 for `T-19-92`. Four of the rows additionally have **no second carrier at all** — `git stash`, `git update-ref`, `git reflog delete` and the SAFE-06 PR cap (`hooks.rs:1263-1270`, `T-19-35`) — so for those the guard is the only control and it is bypassed outright, the cap **uncounted rather than exceeded**. **Not a word-assembly defect**: `>/dev/null` is a fully LITERAL token and `Token.literal` is correctly `true` for it. The inversion cannot see this, because it asks whether a word is rewritten and never whether a word survives into argv. `git push --force origin main >/dev/null`, with the redirection AFTER the decision words, is correctly refused at exit 2 — the hole is a redirection BEFORE a decision word. **Do NOT close this by adding `>` and `<` to `SEPARATORS`**: the existing doc's reason is correct, and splitting there would hide the command being redirected. The shape of the fix is to parse redirections the way bash does and remove them from the argv the classifiers read, or — fail-closed and cheaper — to treat a redirection word standing before a governed program's decision word as making the command unresolvable. | **OPEN — BLOCKING** |
| **T-19-98** | **Elevation of Privilege** | **`policy::tokenize`'s backslash arm — a line continuation is KEPT in the word instead of deleted (`policy.rs:2063-2072`)** | **high** | **mitigate (unimplemented)** | **The arm pushes the escaped character verbatim and deliberately does not clear `literal`, on the stated reasoning that "escaping is exactly what makes a character literal". That is true of every character except a NEWLINE: `\`+newline is not an escape, it is a LINE CONTINUATION, and bash deletes **both** characters before the word is assembled. The tokenizer keeps the newline, so the recovered word is `pu\nsh` where the program receives `push` — and the word is marked LITERAL, correctly by the bit's own test and wrongly about the world. Measured at `ad847b4`, one fresh root per row, walk after (each `\`+newline written as a real two-character sequence in the tool-call command): `git pu\`+NL+`sh --force origin main` → **exit 0**; `git \`+NL+`push --force origin main` → **0**; `git sta\`+NL+`sh` → **0** (no second carrier); `gh p\`+NL+`r create --title x` → **0 with an EMPTY WALK on BOTH calls in one root** — SAFE-06 bypassed rather than exceeded; `git config core.hooks\`+NL+`Path /tmp/x` → **0**, disarming layer 3 exactly as the literal spelling would; the DOUBLE-QUOTED spelling `git "pu\`+NL+`sh" --force origin main` → **0**, through the quote arm's own backslash branch; and through the nested payload `bash -lc "git pu\`+NL+`sh --force origin main"` → **0**. Every row confirmed under `bash` from a script file whose bytes were verified with `od -c`: `git pu\`+NL+`sh --force origin main` prints `ARGV[git]: [push] [--force] [origin] [main]`. **The three-leg argument.** Layer 1 — the command string does not begin `git push`, so the prefix rule does not match. Layer 2 — `git -c core.hooks\`+NL+`Path=/dev/null pu\`+NL+`sh --force origin main` → **exit 0**, confirmed under the shims to assemble `ARGV[git]: [-c] [core.hooksPath=/dev/null] [push] [--force] [origin] [main]`. Layer 3 — disarmed by that same `-c`, on the precedence re-measured for `T-19-97` above. Credentials intact. **Rows DISCARDED on the shell-semantics step, recorded so audit 6 does not spend the measurement again:** `git 'pu\`+NL+`sh' …` prints `[pu\ ` / ` sh]` — inside SINGLE quotes bash performs no continuation, so it is two words and the guard's permit is harmless; `git pu\`+CR+`sh …` and `git pu\`+TAB+`sh …` are genuine escapes of those characters and produce `pu\rsh` / `pu<TAB>sh`, not `push`. Only the NEWLINE case is a finding, which is exactly why the arm's reasoning is right for every character it names and wrong for the one it does not. **Same root class as `T-19-97`, opposite direction**: there the shell removes a whole word the guard reads, here the shell removes two characters the guard keeps. Neither is an expansion and neither clears the literalness bit. The fix is one arm: consume `\`+newline as a line continuation producing NO character, as bash does — or, fail-closed, clear `literal` for it. | **OPEN — BLOCKING** |
| T-19-99 | Spoofing | the 19-16/19-17 generative alphabets and `UNREADABLE_CLASSES` | medium | mitigate | **`T-19-76`'s failure mode for the FIFTH consecutive round.** The round-5 corpus is a real and large improvement — `UNREADABLE_CLASSES` (`tests/envelope_wrapper_class.rs:3011`) names seven classes, each with a degenerate-proof predicate and three kinds of floor, and it genuinely fails on the class round 5 closed. **But all seven classes are word-ASSEMBLY classes** — whole-word splice, concatenated splice, range, multi-expansion word, literal brace pair, glob, tilde — and there is no class for a word the shell DELETES from argv or for a character pair the shell removes. Verified mechanically rather than by reading: `grep -nE '"[^"]*[<>][^"]*"' tests/envelope_wrapper_class.rs tests/envelope_literal_decision.rs` returns **nothing**, and neither file contains a backslash-newline; widened to the whole tree, `grep -rnE '"(git\|gh\|glab)[^"]*[<>][^"]*"' tests/ src/` finds **no guard-driven row carrying a redirection anywhere in the repository**. The corpus is therefore structurally incapable of generating, and so of failing on, `T-19-97` and `T-19-98`. This is the same finding as `T-19-76`, `T-19-83`, `T-19-89` and `T-19-95`, one axis further out — and the axis is the point: round 5 widened the alphabets along "what makes a word unreadable", which is the axis the round's own control is about, so the corpus still models exactly the control it certifies and nothing beside it. | open — below `high` (non-blocking) |

*Status: open · closed · open — below `high` threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` count toward `threats_open`*

`T-19-99` is open below the `high` threshold and does **not** count toward
`threats_open`. `T-19-97` and `T-19-98` do.

### Audit 5's bookkeeping, re-derived from audit 4's group rows

| Group | Count | Closed | Open |
|---|---|---|---|
| Everything through audit 4 | 107 | 86 | 21 (4 at `high`) |
| Closed by plans 19-16 / 19-17, re-measured by audit 5 (`T-19-92` … `T-19-95`) | — | +4 | −4 |
| Registered by plan 19-16 (`T-19-96`, medium) | 1 | 0 | 1 (0 at `high`) |
| Found by audit 5 (`T-19-97` … `T-19-99`) | 3 | 0 | 3 (2 at `high`) |
| **Total after audit 5** | **111** | **90** | **21 (4 at `high`)** |

The four that count toward `threats_open`: `T-19-86`, `T-19-91`, `T-19-97`,
`T-19-98`. The seventeen that do not: `T-19-61` … `T-19-73` (13), `T-19-84`,
`T-19-85`, `T-19-96`, `T-19-99`.

### The closures, re-measured rather than accepted from the summaries

Driven as
`printf '<PreToolUse JSON>' | GSD_MM_ENVELOPE_ROOT=$(mktemp -d) ./target/debug/gsd-meta-manager envelope guard alpha`,
one fresh root per row, the envelope directory walked with `os.walk` afterwards.

* **`T-19-92` — CLOSED.** All 25 rows the phase records for this class are at
  `exit=2 [envelope_assertion_failed]` with an **empty walk**: the eleven
  clause-2(a) rows including `bash -lc "git {push,--force} origin main"` and the
  flag-slot `--forc{e,e}`, and all fourteen clause-2(b) rows including the four
  range spellings, the two unenumerable spellings (`{g..g..1}it`, `{g{i,i}t,x}`),
  the two quote-removed-join spellings (`"g"{i,i}"t"`, `{g..g}"it"`), the two
  multi-expansion spellings and the forge twins. The cwd-dependence of
  `git push {--force,origin} main` is gone: refused in the in-namespace fixture
  before any push context is resolved.
* **`T-19-93` — CLOSED, on the COUNT bar, evidenced by a walked ledger.**
  `gh api repos/{owner}/{repo}/pulls -f title=x` → call 1 exit 0 with
  `alpha/pr-ledger.ndjson` (112 b, **1 ledger line**), call 2 exit 2
  `pr_cap_exceeded` (224 b, 2 lines) — byte-for-byte the quoted control's
  behaviour. The `…/pulls/7` boundary is unchanged in both spellings: exit 0,
  **0 ledger lines**, so the tolerance did not degrade into "any endpoint with
  braces counts". `gh api repos/{owner}/{repo}/pulls -X POST -f title=x` is also
  counted. **A refusal here would have been the regression**, and there is none.
* **`T-19-94` — CLOSED.** `git pus?`, `git ?ush`, `git stas?`, `gh p? create`,
  `touch push && git pus? --force origin main`, `git config core.hooksPat?`,
  `gh api …/pul?s`, `gh api …/pulls -? title=x` and `git ~push` all at exit 2
  with empty walks. Audit 5 swept the adjacent glob cells the corpus does not
  enumerate and every one is also refused: `git pus[h]`, `git pus*`, `git p*h`,
  `gh a?i`, `-X P?ST`, `--method P?ST` and `git -c core.hooksPat?=/tmp/x`.
  One row is a permit and it is **correct**: `gh api repos/o/r/pulls -{f}
  title=x` → exit 0, because `{f}` has no comma and no range, so bash passes it
  through byte-identically and `gh` really does receive `-{f}`. The word IS
  literal; `api_flag_ness_is_unreadable` names its index and the bit then clears
  it, which is the documented design and the same reason a quoted `-'*'` passes.
* **`T-19-95` — CLOSED as scoped.** `UNREADABLE_CLASSES` names seven classes
  with degenerate-proof predicates, the per-alphabet, per-class and counted
  floors all count the same thing, and the corpus demonstrably failed on its own
  class: 19-16 committed 26 named RED tests with zero `src/` hunks and 19-17
  turned all 26 green. **The corpus's NEW blind spot is `T-19-99`.**

### The known-open set, as audit 5 verified it

- **`T-19-86` (high, OPEN, unaccepted, deferred by explicit user scoping
  decision).** All four rows re-measured at `ad847b4`, fresh root each, all at
  **exit 0**: `git submodule foreach git push --force origin main`,
  `git rebase -x "git push --force origin main" HEAD~3`,
  `git bisect run sh -c "git push --force origin main"`,
  `git -c alias.p='!git push --force origin main' p`. Exactly as registered.
  Counts toward `threats_open`.
- **`T-19-91` (high, OPEN).** `git reflog $S`, `git reflog show $S` and
  `git symbolic-ref $S` reproduce at exit 0; `git symbolic-ref HEAD $R` → 2
  `force_push_blocked` and `git push origin $REF` → 2 `push_outside_namespace`
  still fail closed. **The third arm audit 4 found is now recorded correctly**,
  and audit 5 re-measured it in a purpose-built fixture on
  `refs/heads/gsd-auto/alpha/work` with a local bare upstream: `git push $REF`
  → **exit 0 in namespace**, exit 2 `push_outside_namespace` outside it.
  `resolve_program`'s fourth residual bullet, `19-17-SUMMARY.md` and the plan
  19-17 execution record all now say **cwd-dependent and permitted in the
  in-namespace configuration** rather than "already fails closed", and the
  second-carrier asymmetry is restated unweakened. **The record correction is
  discharged; the threat is not.** Counts toward `threats_open`.
- **`T-19-96` (medium, OPEN, registered by plan 19-16, not fixed).**
  Re-measured: `git push --forc? origin refs/heads/gsd-auto/alpha/w` → **exit
  0**, its literal twin `--force` → exit 2 `force_push_blocked`. Correctly
  registered, correctly pinned, and correctly rated medium on the same
  precondition standard audit 4 applied to `T-19-94` — it needs a matching
  filename in the working directory. Audit 5 notes it is one arm of the same
  shape as `T-19-91`: a `classify_push` slot outside the decision region.
  Below `high`; does not count toward `threats_open`.
- **`T-19-74` (medium, closed/accepted — AR-19-10).** Core rows frozen and
  re-measured permitted: `env $X push --force origin main` → **0** and
  `X=git; env $X push --force origin main` → **0**. `T-19-84` — that the doc's
  narrowing argument is false as written — remains open and unaccepted.
- **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85` — carried forward untouched**,
  open and unaccepted at their original severities, all below `high`. Plans
  19-16 and 19-17 touched `policy.rs`, `hooks.rs` and two test files only;
  `cred.rs`, `mod.rs`, `advisory.rs`, `scan.rs` and `config.rs` were not opened,
  so `T-19-63` and `T-19-65` … `T-19-73` cannot have moved.
- **`T-19-SC` — still holds.** Neither `Cargo.toml` nor `Cargo.lock` was touched
  by any plan-19-16 or plan-19-17 commit.

### The claimed-unaffected commands, re-measured

Every row of the allow corpus is at **exit 0**: `git commit -m "use ${HOME}
here"`, `gh pr create --title 'fix $PATH handling'` (with a ledger line),
`git -c user.name="$NAME" commit`, `ROOT=$(git rev-parse --show-toplevel)`,
`git add src/*.rs`, `echo {a,b}`, `{ git status; }`, `( git status )`,
`(git status)&&git fetch origin`, `mkdir -p {src,tests}`, `cp x{,.bak}`,
`ls *.rs`, `rg "x" src/*`, `cd ~/projects`, `git config user.email "$EMAIL"`,
`echo $(git rev-parse HEAD)`, `git log -1 HEAD@{0}`, `git reflog show HEAD@{0}`,
`gh api repos/o/r/pulls -f title="$T"` and `gh pr create --title "$TITLE"` (both
still COUNTED), `git config --get-regexp 'branch.*'`, `FOO=ab git status`,
`git commit -m ab`, `git add src/x.rs tests/x.rs` and `rg "git status" src/`.
The grouping refusals still fire — `{ git push --force origin main; }`,
`( git push --force origin main )` and `git reflog delete HEAD@{0}` are all exit
2 under `force_push_blocked` — so `{ cmd; }` and `( cmd )` were discriminated
rather than deleted. **`ls {git,svn}-repo` is permitted**, which is the control
that makes clause 2(b) a PRODUCT test rather than a MENTION test.

### The mechanism pin, verified non-vacuous

`rule_b_still_reports_a_severed_head_as_not_a_command_position`
(`tests/envelope_literal_decision.rs:1506`) asserts over
`policy::split_segments_with_heads` **directly rather than over an exit code**,
over three severed spellings (`${C}_COUNT`, `${K}_COMMAND` and the bare
`${C}`), and it is green and unmodified. It cannot be satisfied by marking every
segment severed, because
`a_brace_group_opener_is_still_a_command_position_which_is_why_the_opener_is_excluded`
asserts the opposite fact for `{ git status; }`. In the source, case 1 of the
three-way `{` classification (`policy.rs:1886`,
`let parameter_expansion = started && previous_was_unquoted_dollar;`) is
genuinely distinct from the literal-brace branch. **Rule B did not become dead
code when `{` handling changed**, and `SEPARATORS` (`policy.rs:1180`) is
byte-for-byte unchanged.

### The four execution-time judgements, assessed independently

1. **19-16's COVERAGE relabelling — HONEST, and the rows earn their place.**
   The arithmetic is right and audit 5 re-derived it: a comma list of N
   alternatives produces N words, so a concatenated comma splice in a PROGRAM
   slot always leaves a surplus word the real tool rejects; only a range with
   equal endpoints yields exactly one word. Confirmed under the shims —
   `g{i,i}t push …` gives `ARGV[git]: [git] [push] …`. The relabelling is not a
   quiet downgrade: `tests/envelope_literal_decision.rs:677-704` carries the
   **measured argv and the real-tool error for every COVERAGE row in the test
   file itself**, and the classes are split into
   `…_comma_spellings` (COVERAGE) and `…_range_spellings` (GENUINE). The rows
   earn their place on a checkable argument — the guard is blind to them
   identically and one character separates them from the range spellings — and
   two of them carry mechanisms **no other row exercises**: `{g{i,i}t,x}` is the
   only row that forces a nested `{` to be an unenumerable TRIGGER rather than a
   top-level decomposition, and `"g"{i,i}"t"` is the only row that forces
   literal runs to be joined QUOTE-REMOVED. Deleting either would leave a
   clause of `brace_word_products` uncovered. Two genuine comma reproducers
   (`{env,git} push …`, `git {--no-pager,push} …`) were added in compensation
   and both re-measured refused. **This is audit 4's own discard discipline
   applied to rows audit 4 did not measure, and it is the right call.**
2. **The `api_flag_ness_is_unreadable` widening — CORRECT, and the region did
   not move.** The pre-19-17 predicate (`09e83bd:policy.rs:2103`) was
   `starts_with('$') || starts_with('`')` with a `-`-initial key half testing
   the same two characters. It is now `REWRITING_CHARACTERS` plus `{`
   (`policy.rs:2977-2990`) — a **strict superset**, so it names more indices and
   never fewer, and the region cannot have narrowed. It still only SELECTS an
   index; the decision is still `Token.literal` read in the one closure of
   `first_unreadable_decision_word`, which audit 5 confirmed by measurement:
   `gh api repos/o/r/pulls -{f} title=x` is NAMED by the predicate and then
   PASSES, correctly, because bash hands `gh` the bytes `-{f}` unchanged. The
   same widening was applied symmetrically to `scan_leading`'s `-c` key half, so
   the two key-half readability tests agree rather than drift — which is the
   defect this phase produced four times. The finding itself is the strongest
   evidence in the round for the corpus's value: it was caught by the
   **generative** forge-slot property after every enumerated row was green.
   Reporting it as a deviation rather than absorbing it is correct.
3. **`MIN_UNREADABLE_FORGE_SLOT_CASES` — the recount to 40 is CORRECT and audit
   5 re-derived it independently.** `FORGE_SLOTS` has **7** entries,
   `DISPLACING_OPTS` has **4**, the loop passes all four only to
   `ApiDisplacedEndpoint` and `&DISPLACING_OPTS[..1]` to the other six,
   `UNREADABLE_SPELLINGS` has **2** entries and `FORGE_WRAPPER_DEPTHS` is **2**:
   `(6x1 + 1x4) x 2 x 2 = 40`. 19-16's floor of 50 was therefore unreachable
   against a maximum of 40 and no production change could have moved it. **No
   refusal assertion was weakened and no alphabet was narrowed** — verified by
   reading the property: every generated case still asserts a refusal AND an
   empty walk, the `slots_seen.len() == FORGE_SLOTS.len()` floor is intact, and
   the positive walk control precedes the loop. The corrected floor is **exactly
   equal to the generation count**, which makes it tighter than a slack floor,
   not looser: losing a single case now turns it red. Refusing to inflate the
   count with duplicate cases is also right.
4. **The stale handoff guard — dropping `NestedPayload` WOULD have been the
   worse trade, and the call is endorsed.** The diff is verifiable: 19-16's
   `assert_eq!(answer.code, 0)` for `rg "git status" {src,tests}` is gone, the
   pre-fix measurement survives as a print and a comment, and a NEW test
   `the_marked_payload_splice_is_refused_by_clause_2a_and_that_is_a_pinned_cost`
   pins the post-fix REFUSAL beside its permitted twin. **Net movement: one
   assertion about a PERMIT deleted, one assertion about a REFUSAL added.** The
   guard's own name, comment, failure message and `19-16-SUMMARY.md` all said it
   asserted nothing post-fix, so its contract really was discharged by the
   confirmed RED run. The decisive argument is directional: weakening a refusal
   clause to make a permit assertion green is the exact move five rounds of
   plan-check exist to prevent. **One honest caveat audit 5 records rather than
   suppresses:** audit 5 could not construct a row that the `NestedPayload` arm
   of clause 2(a) uniquely REFUSES — `bash -lc "git {push,--force} …"` is
   reached through the payload re-split, and the contrived alternatives are
   refused on other grounds — so the arm's demonstrated value today is
   conservatism rather than a measured harm. That is the fail-closed direction,
   its one cost is disclosed and pinned from both sides, and it does not change
   the verdict.

### A bookkeeping gap in `T-19-17r` — flagged, not counted as a threat

`19-17-SUMMARY.md` calls `T-19-17r` "the new over-refusal cost, **accepted** and
pinned from both sides". Audit 5 confirms the **measurement and the pins**:
`rg "git status" {src,tests}` → exit 2 `envelope_assertion_failed` and
`rg "git status" src/` → exit 0, both pinned in
`tests/envelope_literal_decision.rs:1355-1379` with the producing clause named.
But there is **no `AR-19-13` row in the Accepted Risks Log and no register row
for `T-19-17r`**, so it is described as accepted while being documented nowhere
an audit reads. Audit 5 does **not** add the acceptance: accepting a risk is a
human decision and this audit does not make it. Recorded here so the next round
either adds the log row or drops the word "accepted". The same applies to the
disclosed corpus limit `19-17-SUMMARY.md` records — `sh {-c,"git push --force …"}`
is refused because a quote inside an alternative is unenumerable, which audit 5
re-measured at exit 2 and confirms is correct and fail-closed, but the corpus
does not distinguish that refusal from an enumerated one.

---

## Audit 5 — what the round-5 control can and cannot fail on

### The principle rounds 3 and 4 established still holds

A decision region must be derived from the same scan the classifier runs, and
there is one walk. `first_unreadable_decision_word` still takes the git verb
from `scan_leading`, `config`'s key from `config_key_operand_index`, the forge's
first two words from `subcommand_word_indices` and the `gh api` endpoint from
`scan_gh_api`; `split_segments` is still defined over
`split_segments_with_heads`; `resolve_program_with_head` calls the step
machinery once and its match is now **exhaustive with no wildcard arm**, closing
the fail-open seam audit 4 flagged. Audit 5 probed the adjacent glob cells of
every slot and found none one slot over.

### Where the boundary now is, in one paragraph

**Round 5 decides on one INVARIANT instead of one bit and one geometry.** A
decision word must be provably LITERAL, evidenced positively while it is
consumed; and a simple command a brace expansion splices into is unresolvable
when a segment resolves `Governed`/`NestedPayload` or when a word the splice can
produce has a governed basename or cannot be enumerated. Between them these
cover **every way bash ASSEMBLES a word** — audit 5 enumerated the shell's
expansions against them one at a time and found no gap, including the ones no
alphabet in this phase has ever drawn.

**What they cannot fail on is the other half of the same boundary**: whether the
words the guard classifies are the words the program receives.

- **words the shell DELETES from argv** — a redirection (`T-19-97`), which is a
  fully literal token the guard reads and `execve` never sees;
- **characters the shell deletes that the tokenizer keeps** — a backslash-newline
  line continuation (`T-19-98`), where the guard's quote removal and bash's
  disagree;
- **a whole command line handed to a governed program as data** — `T-19-86`,
  unchanged, and the one residual that has survived every round;
- **a classifier arm's own operand or flag outside the decision region** —
  `T-19-91` (`reflog`, `symbolic-ref`, the bare `push $REF`) and `T-19-96` (a
  push flag), both registered.

**The one-sentence version for the next round.** Four rounds the gap was the
cell one slot over from what the corpus could draw; round 5 closed that class
outright and the gap moved AXIS instead — from *how a word is written* to
*which words arrive*. `T-19-99` is the mechanical form of it: the alphabets now
model "what makes a word unreadable" thoroughly and model nothing else, so a
control certified by them is certified about word assembly and about nothing
else.

### Suggested closure, in order — (d) FIRST, again

1. **(d) — widen the corpus BEFORE certifying anything.** Add an eighth and
   ninth class to `UNREADABLE_CLASSES` — *a word the shell removes from argv*
   and *a character pair the shell removes from a word* — with entries carrying
   `>`, `<`, `>>`, `<<<`, `2>` and a real backslash-newline in
   `REFUSED_BASES`, `WRAPPERS`, `DECOY_OPERANDS`, `SHELL_LAYERS` and
   `SEVERED_PREFIXES`, and assert the corpus generates them with the `MIN_*`
   floor pattern already in the file. **Listed first for the fifth round
   running**, and for the fifth round running it is the recommendation the
   previous audit also made and the next gap was in the cell it named.
2. **(a) — `T-19-98`, smallest change first.** Consume `\`+newline in
   `tokenize`'s backslash arm as a line continuation producing NO character, as
   bash does. One arm, no new concept, and its correctness is checkable against
   `od -c` output. The paired cost must be pinned: `\` before any OTHER
   character must keep producing that character and keep `literal` true, because
   escaping really is what makes a character literal.
3. **(b) — `T-19-97`.** Parse redirections the way bash does — an optional fd
   number, one of `<`, `>`, `>>`, `<<<`, `<&`, `>&`, and its target word — and
   remove both from the argv the classifiers read; or, fail-closed and cheaper,
   treat a redirection word standing before a governed program's decision word
   as making the command unresolvable. **Do NOT add `>` and `<` to
   `SEPARATORS`**: the existing doc's reason is right and splitting there hides
   the command being redirected. The paired cost must be pinned from both sides
   exactly as Rule B's and clause 2's were —
   `git push --force origin main >/dev/null` must keep reaching `exit 2`, and an
   ordinary `git status >/tmp/out` must keep reaching `exit 0`.
4. **(c)** — then `T-19-86`, `T-19-91` and `T-19-96`, which are three arms of
   one shape: a classifier arm answering `Allow` on an operand, flag or payload
   outside the decision region.

Then, and separately: add the `AR-19-13` row for `T-19-17r` or stop calling it
accepted.

---

## Audit 5 Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log — twelve, unchanged by
      round 5; audit 5 accepts nothing new
- [x] Every closure re-measured against the built binary at `ad847b4` with a
      fresh envelope root per row and the directory walked afterwards
- [x] Every new finding confirmed under `bash` against argv-printing shims;
      three candidates discarded because the shell did not run what the guard
      permitted (single-quoted `\`+newline, `\`+CR, `\`+TAB), plus alias
      expansion and process substitution, which reach no decision slot at all
- [x] `T-19-93` verified on the COUNT bar by a walked ledger listing, not by an
      exit code
- [x] Plans 19-13, 19-14, 19-15, 19-16 and 19-17's appended subsections left
      byte-identical, and every earlier audit's own tables untouched
- [x] Gate observed: `rtk proxy cargo test --no-fail-fast` → **1584 passed, 0
      failed, 13 ignored**, `passed + failed = 1584`, matching
      `19-17-SUMMARY.md` exactly; all eleven `envelope_*` binaries ran;
      per-binary `envelope_literal_decision` 43, `envelope_wrapper_class` 25,
      `envelope_expansion_slots` 32, `envelope_command_position` 18
- [ ] `threats_open: 0` confirmed — **4 open at `high`: `T-19-86`, `T-19-91`,
      `T-19-97`, `T-19-98`**
- [ ] `status: verified` set in frontmatter

**Approval: blocked, 2026-08-29 (audit 5).**

**Not accepted here.** `T-19-97` and `T-19-98` are high-severity, empirically
confirmed bypasses, each defeating layers 1, 2 and 3 on a single line with the
credential intact and the `-c` precedence re-measured on this machine, and each
carrying rows with **no second carrier** — `git stash`, `git update-ref`,
`git reflog delete` and a SAFE-06 cap bypassed uncounted on both calls in one
root. `T-19-86` and `T-19-91` remain open at `high` by scoping decision and by
round discipline respectively. Accepting any of the four is a human decision and
this audit does not make it.

**Progress is real and it is the best round of the five.** Round 5 is the first
that closed its predecessor's entire named class rather than the measured rows
of it, and the first whose corpus was written RED before the rule existed and
demonstrably failed on the class it certifies — 26 named failures in a commit
with zero `src/` hunks, all 26 green after. `T-19-93` was closed on the harder
bar of COUNT rather than refusal, in the tokenizer where the defect was rather
than by a tolerance in a consumer. Three execution-time judgements caught real
defects the plan did not have — a floor unreachable by construction, a
predicate the plan called unchanged that was one character class short, and a
stale guard pinning a permit the rule must refuse — and each was reported rather
than absorbed, with the one fork resolved in the direction that kept the
refusal. **The inversion is sound and audit 5 says so plainly.** What it did not
do is answer the second half of its own boundary question, and that is where the
two blocking findings are: not one slot over, but one axis over — from how a
word is written to which words arrive.

---

## Execution record — plan 19-18 (the corpus, RED). NOT an audit finding.

**This is an execution record made by plan 19-18, not an audit finding.** It
adds no threat, closes none, re-classifies none, and edits no audit table, no
Security Audit Trail entry, no Accepted Risks Log row, no sign-off and no
earlier appended subsection. Re-measuring and re-classifying these rows is
`/gsd-secure-phase 19`'s job.

**This plan CLOSES NOTHING.** `T-19-97`, `T-19-98` and `T-19-99` all stay OPEN
at this plan's end. `T-19-99` is closed only when `19-19`'s rule is certified by
the corpus written here, because a corpus is evidence about a control and there
is no control yet. **`/gsd-secure-phase 19` is NOT cleared by this plan, by
`19-19`, or by the two together**: `T-19-86` remains OPEN at `high` by explicit
user scoping decision and `T-19-91` remains OPEN at `high` with three arms.
Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.

### The invariant this round is about, stated once

> **The words the guard classifies must be exactly the words the program
> receives, in the same order — no more and no fewer.**

Round 4 established one half positively: a decision word must be provably
LITERAL, evidenced while it is consumed. `Token.literal` is that half. The other
half is that the word SURVIVES into argv, in its slot. Both `T-19-97` and
`T-19-98` are fully literal by round 4's own test **and the bit is RIGHT about
them** — measured directly against `policy::split_segments_with_heads`, which
reports `(">/dev/null", literal = true)` and `("push", literal = true)` for
`git >/dev/null push --force origin main`, against `("pus?", literal = false)`
for `git pus? --force origin main`. It is the model that is incomplete.

### Every audit-5 row reproduced

All twenty-four rows reproduced at the verdict audit 5 recorded, measured
against the BUILT BINARY at `fccb5de` — one fresh `GSD_MM_ENVELOPE_ROOT` per
row via `mktemp -d`, the whole root walked afterwards. **No audit-5 row failed
to reproduce**, so nothing was asserted over a measurement that disagreed with
the audit.

`T-19-97`, the sixteen rows (walk column is the whole walked root):

| Command | exit | walk |
|---|---|---|
| `git >/dev/null push --force origin main` | 0 | empty |
| `git 2>/dev/null push --force origin main` | 0 | empty |
| `git 1>/dev/null push --force origin main` | 0 | empty |
| `git >>/tmp/x push --force origin main` | 0 | empty |
| `git > /tmp/o push --force origin main` | 0 | empty |
| `git <<<x push --force origin main` | 0 | empty |
| `git push>/dev/null --force origin main` | 0 | empty |
| `git >/dev/null stash` | 0 | empty |
| `git >/dev/null update-ref -d refs/heads/main` | 0 | empty |
| `git >/dev/null config core.hooksPath /tmp/x` | 0 | empty |
| `gh >/dev/null pr create --title x` | 0 | **empty** |
| `gh >/dev/null api repos/o/r/pulls -f title=x` | 0 | **empty** |
| `glab >/dev/null mr create --title x` | 0 | **empty** |
| `bash -lc "git >/dev/null push --force origin main"` | 0 | empty |
| `echo hi && git >/dev/null push --force origin main` | 0 | empty |
| `git >/dev/null -c core.hooksPath=/dev/null push --force origin main` | 0 | empty |

`T-19-98`, the eight rows — every `<NL>` a REAL newline preceded by a REAL
backslash:

| Command | exit | walk |
|---|---|---|
| `git pu\<NL>sh --force origin main` | 0 | empty |
| `git \<NL>push --force origin main` | 0 | empty |
| `git sta\<NL>sh` | 0 | empty |
| `gh p\<NL>r create --title x` | 0 | **empty** |
| `git config core.hooks\<NL>Path /tmp/x` | 0 | empty |
| `git "pu\<NL>sh" --force origin main` | 0 | empty |
| `bash -lc "git pu\<NL>sh --force origin main"` | 0 | empty |
| `git -c core.hooks\<NL>Path=/dev/null pu\<NL>sh --force origin main` | 0 | empty |

### The bash-shim argv for every claimed bypass

Shims named `git`, `gh` and `glab` first on `PATH`. **Each shim writes its argv
to a side file rather than to stdout**, because a row whose whole point is
`>/dev/null` would otherwise swallow the very evidence it is driven for — the
first shim draft printed nothing for eleven rows for exactly that reason.

```
git >/dev/null push --force origin main        -> ARGV[git]: [push] [--force] [origin] [main]
git 2>/dev/null push --force origin main       -> ARGV[git]: [push] [--force] [origin] [main]
git 1>/dev/null push --force origin main       -> ARGV[git]: [push] [--force] [origin] [main]
git >>/tmp/x push --force origin main          -> ARGV[git]: [push] [--force] [origin] [main]
git > /tmp/o push --force origin main          -> ARGV[git]: [push] [--force] [origin] [main]
git <<<x push --force origin main              -> ARGV[git]: [push] [--force] [origin] [main]
git push>/dev/null --force origin main         -> ARGV[git]: [push] [--force] [origin] [main]
git >/dev/null stash                           -> ARGV[git]: [stash]
git >/dev/null update-ref -d refs/heads/main   -> ARGV[git]: [update-ref] [-d] [refs/heads/main]
git >/dev/null config core.hooksPath /tmp/x    -> ARGV[git]: [config] [core.hooksPath] [/tmp/x]
gh   >/dev/null pr create --title x            -> ARGV[gh]: [pr] [create] [--title] [x]
gh   >/dev/null api repos/o/r/pulls -f title=x -> ARGV[gh]: [api] [repos/o/r/pulls] [-f] [title=x]
glab >/dev/null mr create --title x            -> ARGV[glab]: [mr] [create] [--title] [x]
echo hi && git >/dev/null push --force origin main
                                               -> ARGV[git]: [push] [--force] [origin] [main]
git >/dev/null -c core.hooksPath=/dev/null push --force origin main
                    -> ARGV[git]: [-c] [core.hooksPath=/dev/null] [push] [--force] [origin] [main]
git pu\<NL>sh --force origin main              -> ARGV[git]: [push] [--force] [origin] [main]
git \<NL>push --force origin main              -> ARGV[git]: [push] [--force] [origin] [main]
git sta\<NL>sh                                 -> ARGV[git]: [stash]
gh p\<NL>r create --title x                    -> ARGV[gh]: [pr] [create] [--title] [x]
git config core.hooks\<NL>Path /tmp/x          -> ARGV[git]: [config] [core.hooksPath] [/tmp/x]
git "pu\<NL>sh" --force origin main            -> ARGV[git]: [push] [--force] [origin] [main]
git -c core.hooks\<NL>Path=/dev/null pu\<NL>sh --force origin main
                    -> ARGV[git]: [-c] [core.hooksPath=/dev/null] [push] [--force] [origin] [main]
```

### The `od -c` bytes for the backslash-newline rows

Every `T-19-98` row was written to a FILE, the file's bytes checked with
`od -c`, and bash driven over the file — which is the step audit 5 discarded
three candidates on.

```
git pu\<NL>sh --force origin main
  0000000   g   i   t       p   u   \  \n   s   h       -   -   f   o   r
  0000020   c   e       o   r   i   g   i   n       m   a   i   n  \n

git \<NL>push --force origin main
  0000000   g   i   t       \  \n   p   u   s   h       -   -   f   o   r
  0000020   c   e       o   r   i   g   i   n       m   a   i   n  \n

git sta\<NL>sh
  0000000   g   i   t       s   t   a   \  \n   s   h  \n

gh p\<NL>r create --title x
  0000000   g   h       p   \  \n   r       c   r   e   a   t   e       -
  0000020   -   t   i   t   l   e       x  \n

git config core.hooks\<NL>Path /tmp/x
  0000000   g   i   t       c   o   n   f   i   g       c   o   r   e   .
  0000020   h   o   o   k   s   \  \n   P   a   t   h       /   t   m   p
  0000040   /   x  \n

git "pu\<NL>sh" --force origin main
  0000000   g   i   t       "   p   u   \  \n   s   h   "       -   -   f
  0000020   o   r   c   e       o   r   i   g   i   n       m   a   i   n
  0000040  \n

git -c core.hooks\<NL>Path=/dev/null pu\<NL>sh --force origin main
  0000000   g   i   t       -   c       c   o   r   e   .   h   o   o   k
  0000020   s   \  \n   P   a   t   h   =   /   d   e   v   /   n   u   l
  0000040   l       p   u   \  \n   s   h       -   -   f   o   r   c   e
  0000060       o   r   i   g   i   n       m   a   i   n  \n

git push \<NL> origin refs/heads/gsd-auto/alpha/w
  0000000   g   i   t       p   u   s   h       \  \n       o   r   i   g
  0000020   i   n       r   e   f   s   /   h   e   a   d   s   /   g   s
  0000040   d   -   a   u   t   o   /   a   l   p   h   a   /   w  \n
```

### The three-leg argument, each leg re-measured

* **Layer 1** — the line begins `git >`, which does not match the
  `Bash(git push:*)` prefix rule the agent CLI owns.
* **Layer 2** — defeated on the same line:
  `git >/dev/null -c core.hooksPath=/dev/null push --force origin main` → exit 0
  with an empty walk.
* **Layer 3** — disarmed. Re-measured on this machine at `fccb5de`:
  `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=/ENV_WINS git >/tmp/l3out -c core.hooksPath=/CLI_WINS config --get core.hooksPath`
  prints **`/CLI_WINS`**; the PAIRED control (the same line without the `-c`)
  prints **`/ENV_WINS`**. The command-line `-c` outranks the envelope's
  env-injected setting (D-09), so no `pre-push` hook runs.

**Three rows have NO SECOND CARRIER AT ALL** — `git stash`, `git update-ref` and
the SAFE-06 PR cap (`T-19-35`) — so for those the guard is the only control, and
`git config core.hooksPath` is the loss of the second carrier itself.

### The seven cells found while PLANNING round 6

Same provenance caveat `19-14` and `19-16` established: **found while planning,
not by an audit.** All seven measured at exit 0 with empty walks at `fccb5de`,
all seven confirmed under the shims, and all seven are **inside `T-19-97`'s
class rather than new threats** — they are folded into `T-19-97`'s entry in
`deferred-items.md` and no new threat ID is registered for them.

| Cell | Line | exit | bash argv | Why the audit's rows do not reach it |
|---|---|---|---|---|
| `&>` | `git &>/tmp/o push --force origin main` | 0 | `[push] [--force] [origin] [main]` | `&` is in `SEPARATORS`, so the guard's separator arm splits ONE simple command into TWO. A redirection parser running after that arm never sees `&>` |
| `{v}>` | `git {v}>/tmp/o push --force origin main` | 0 | `[push] [--force] [origin] [main]` | bash 4.1 fd allocation, carried into the verb slot by round 5's own literal-brace branch because `{v}` has no comma and no range. **Post-fix verdict left for `19-19`** |
| `>\|` | `git >\|/tmp/o push --force origin main` | 0 | `[push] [--force] [origin] [main]` | a two-character operator no single-`>` rule reaches |
| `<>` | `git <>/tmp/o push --force origin main` | 0 | `[push] [--force] [origin] [main]` | same, and the one operator whose first character is `<` and second is `>` |
| `<<` | `git <<EOF push --force origin main` | 0 | `[push] [--force] [origin] [main]` | bash runs it even as a single line, warning `here-document at line 1 delimited by end-of-file` and running the command anyway |
| `<file` | `touch input.txt && git <input.txt push --force origin main` | 0 | `[push] [--force] [origin] [main]` | **PRECONDITION row.** Without the file bash fails the redirection and runs NOTHING |
| `x2>` | `git x2>/tmp/o push --force origin main` | 0 | **`[x2] [push] [--force] [origin] [main]`** | **the OVER-DELETION control.** An IO_NUMBER is a digits-only run since the START of the word, so `x2` IS argv and git itself rejects it. **Permitted today and it must STAY permitted** |

Two further operators were measured because `19-19`'s production uniquely adds
them and a corpus without them cannot fail on them:
`git &>>/tmp/o push --force origin main` → exit 0, `[push] [--force] [origin]
[main]`; `git <<-EOF push --force origin main` → exit 0, same argv; and their
permitted twins `git &>>/tmp/o status` and `git <<-EOF status` → exit 0,
`ARGV[git]: [status]`.

### Where the hole actually is — the position controls

Measured at `fccb5de`, so the displacement alphabet splices in the right place
rather than in a position that is already closed:

```
exit=2  force_push_blocked   >/dev/null git push --force origin main       (already refused)
exit=2  force_push_blocked   2>/dev/null git push --force origin main      (already refused)
exit=2  force_push_blocked   env >/dev/null git push --force origin main   (already refused)
exit=2  force_push_blocked   git push --force origin main >/dev/null       (already refused)
exit=0  + ONE LEDGER LINE    >out gh pr create --title x                   (already COUNTED)
                             walked listing: alpha/pr-ledger.ndjson (103 bytes)
```

`resolve_program` walks past a leading redirection as it would any wrapper
operand and finds `git`. **The hole is a redirection standing BETWEEN the
governed program and its decision words**, and a corpus that drew the prefix
position would be green before the fix and would certify nothing.

### The two pre-existing FALSE REFUSALS — the round REMOVES over-refusal

Measured at `fccb5de` against a repository whose current branch is inside
`refs/heads/gsd-auto/alpha/` with a local bare upstream, each beside its
permitted one-line twin. **These are the same defect in the opposite direction:
the guard reads a word the program never receives, and answers about it.**

```
exit=2 push_outside_namespace   git push origin refs/heads/gsd-auto/alpha/w > log.txt
  "the refspec `>` resolves to `refs/heads/>`, which is outside `refs/heads/gsd-auto/alpha/`"
exit=0                          git push origin refs/heads/gsd-auto/alpha/w      (twin)

exit=2 push_outside_namespace   git push \<NL> origin refs/heads/gsd-auto/alpha/w
  "the refspec `origin` resolves to `refs/heads/origin`, which is outside
   `refs/heads/gsd-auto/alpha/`"
exit=0                          git push origin refs/heads/gsd-auto/alpha/w      (twin)
```

Both derivations are taken from the MEASURED message and they are **DIFFERENT
mechanisms**. The redirection word is read as an EXTRA REFSPEC. The
`\`+newline is flushed by the whitespace after it and becomes its OWN WORD in
the REMOTE slot, displacing every operand one slot right — which is `T-19-97`'s
displacement arriving through `T-19-98`'s mechanism. Under the shims both lines
print `ARGV[git]: [push] [origin] [refs/heads/gsd-auto/alpha/w]`.

They are pinned at their PRE-FIX verdicts in two `#[test]` fns plan 19-18 names,
which are the ONLY assertions in `tests/envelope_argv_deletion.rs` that `19-19`
is permitted to REPLACE rather than only add to:

* `the_redirected_in_namespace_push_is_falsely_refused_today_and_19_19_must_move_it`
* `the_continued_in_namespace_push_is_falsely_refused_today_and_19_19_must_move_it`

### The design question, answered, with the rejected option's measured cost

**The REJECTED option:** refuse any governed simple command containing a token
the shell deletes. Measured cost at `fccb5de` — every one of these is permitted
today and would be refused:

```
exit=0  git log > out
exit=0  git status > /tmp/s.txt
exit=0  git diff > /tmp/d.patch
exit=0  git commit -m "x" >> build.log
exit=0  gh pr list 2>/dev/null
exit=0  git fetch origin 2>&1 | tee log
exit=0  + ONE LEDGER LINE   gh pr create --title x > /tmp/o
```

The last one is the decisive measurement: it is **COUNTED today**, and the
rejected option would turn a correct COUNT into a false positive on ordinary
forge syntax — the exact trade `T-19-93`'s bar forbids. **`19-19` models the
deletion instead**, and this record states the choice before the rule exists so
it is auditable rather than retrofitted.

The same reasoning is why the four FORGE rows are written as a restored COUNT
and not as a refusal. Each unwrapped form is measured at exit 0 with exactly one
ledger file (`gh pr create --title x` → 103 bytes, `gh api repos/o/r/pulls -f
title=x` → 112 bytes, `glab mr create --title x` → 105 bytes), and after
deletion the surviving argv IS the counted one. **The SAFE-06 cap is bypassed
UNCOUNTED rather than exceeded**, and the fix it demands is a restored count.

### The rows whose POST-fix verdict `19-18` could not derive

Three rows are **RECORDED, never asserted** — `tests/envelope_argv_deletion.rs`
section 9 drives each and prints its verdict with no assertion about it. This
plan measures PRE-fix, so its measure-first discipline cannot catch a wrong
POST-fix expectation, and a row pinned at a verdict the rule cannot produce
lands RED with `19-19` forbidden to edit it. `19-16` handled its two
undeliverable rows exactly this way.

| Row | measured at `fccb5de` | why `19-19` must derive it |
|---|---|---|
| `git >$F push --force origin main` | exit 2 `envelope_assertion_failed` — *``>$F`` is the git verb for this command* | round 4's bit fires on the `$` in what the guard reads as the VERB. After deletion the surviving argv is `push --force origin main`, so it stays refused with a DIFFERENT identifier. Also a PRECONDITION row: bash answers `ambiguous redirect` when `F` is unbound |
| `git >*.log push --force origin main` | exit 2 `envelope_assertion_failed` — same message shape | same, through the glob. Also a PRECONDITION row when the glob matches more than one file |
| `git {v}>/tmp/o push --force origin main` | exit 0, empty walk | `19-19` deliberately does not model a `{name}` fd prefix and marks it unresolvable — a different identifier reached by a different clause from its six siblings. Its permitted-half twin `git {v}>/tmp/o status` is exit 0 and is recorded the same way |

For the same reason `{v}>/tmp/o` is kept OUT of `DISPLACING_REDIRECTIONS`: every
entry there is asserted VERDICT-PRESERVING by the property's permitted arm, and
this one would be STRICTER than its base.

### Audit 5's three DISCARDED rows were NOT re-added

Single-quoted `\`+newline, `\`+CR and `\`+TAB stay discarded and are not rows in
any file. The single-quoted spelling was re-run ONCE as a sanity check of this
round's shim harness — it printed `ARGV[git]: [pu\` / `sh] [--force] [origin]
[main]`, i.e. one mangled word and no force push, confirming audit 5's discard —
and it was not added as a row. Recorded here so audit 6 does not spend the
measurement again.

### The `T-19-17r` bookkeeping gap — RECORDED as OUTSTANDING, acceptance NOT made

`19-17-SUMMARY.md` calls `T-19-17r` "the new over-refusal cost, **accepted** and
pinned from both sides". Audit 5 confirmed the measurement and both pins —
`rg "git status" {src,tests}` → exit 2, `rg "git status" src/` → exit 0, both at
`tests/envelope_literal_decision.rs:1355-1379` with the producing clause named.
But there is **no `AR-19-13` row in the Accepted Risks Log and no register
row**, so it is described as accepted while documented nowhere an audit reads.

**This plan records the gap and leaves it OPEN. It adds no `AR-19-13` row and
makes no acceptance**, because accepting a risk is a human decision and audit 5
explicitly declined to make it on the developer's behalf. The next round either
adds the log row or drops the word from the summary — and this plan does
neither.

### Audit 5's disclosed corpus limit, forwarded to `19-19`

`sh {-c,"git push --force …"}` is refused because a quote inside an alternative
is unenumerable. That is CORRECT and fail-closed, but the corpus cannot
distinguish that refusal from an enumerated one. It is forwarded to `19-19`
rather than addressed here **because the only place the distinction is
observable is a unit assertion over the private whole-word product scan in
`policy.rs`'s own test module, and this plan may not touch `src/`.**

### The corpus, and what it is now capable of failing on

`tests/envelope_argv_deletion.rs` is the FIFTH evidence file — 17 `#[test]` fns,
13 green and 4 RED. `tests/envelope_wrapper_class.rs` gains section 14: a SECOND
named axis, `DELETION_CLASSES`, standing beside a byte-identical
`UNREADABLE_CLASSES` (the diff is 1010 insertions and **zero deletions**; no
existing floor, alphabet entry, property or assertion was lowered, deleted or
narrowed), with five degenerate-proof quoting-aware predicates, an
11-entry `DISPLACING_REDIRECTIONS` alphabet spliced between the governed program
and its decision words, two `CONTINUATION_SPLICES`, and floors stated as EXACT
equalities derived from the generation arithmetic — 330 cases over 15 slots,
per class 165 / 135 / 240 / 15 / 15.

**An execution-time finding, reported rather than absorbed.** The exact
per-class equality caught a real defect in the first draft of
`draws_an_attached_redirection`: reading the second `>` of `>>/tmp/x` as an
operator attached to a literal run `>` made six of the eleven entries satisfy
the ATTACHED class as well, over-counting it by 90 cases and collapsing the
class-1/class-2 split the axis turns on. A floor stated as `>=` rather than `==`
would not have caught it. The predicate now encodes bash's rule — the longest
operator match at the start of a redirection, everything after it is the TARGET
— and seven new degenerate-proofing assertions pin it.

**The gate, observed.** `rtk proxy cargo test --no-fail-fast` →
**1600 passed, 5 failed, 13 ignored**, `passed + failed = 1605`. Baseline was
1584. This plan adds 21 new `#[test]` fns (17 + 4), and a red test RAN, so
`1584 + 21 = 1605` exactly. Every failure name is one of this plan's five RED
names; the two flaky `driver_reattach` tests passed in this run and their
passing is not a change this plan made. `envelope_literal_decision` 43,
`envelope_expansion_slots` 32 and `envelope_command_position` 18 are FULLY GREEN
and byte-identical.

### What this record does NOT do

* It does not close `T-19-97`, `T-19-98` or `T-19-99`.
* It does not clear `/gsd-secure-phase 19`. `T-19-86` and `T-19-91` remain OPEN
  at `high`; `T-19-96`, `T-19-74`'s residual, `T-19-84`, `T-19-85` and
  `T-19-61` … `T-19-73` are untouched.
* It writes no unqualified "T-19-60 is closed". Only the WRAPPER-OPERAND
  sub-class of `T-19-60` is closed; `T-19-86` and `T-19-91` are both sub-classes
  of it and both remain open.
* It changes not one line of `src/`. All three commits show ZERO `src/` hunks
  under `git show --stat`, which is the evidence this plan/rule split exists to
  produce.

---

## Execution record — plan 19-19 (NOT an audit finding)

**Written by plan `19-19` as an execution record.** It is not an audit, it makes
no acceptance decision, and it edits nothing above it: no audit table, no
Security Audit Trail row, no Accepted Risks Log row, no sign-off, and none of
the appended records of plans 19-16, 19-17 or 19-18. Re-measuring and
re-classifying these rows is `/gsd-secure-phase 19`'s job.

**`/gsd-secure-phase 19` is NOT cleared by this plan.** `T-19-86` and `T-19-91`
remain OPEN at `high`.

### The invariant as implemented

> **The words the guard classifies must be exactly the words the program
> receives, in the same order — no more and no fewer.**

`Token::literal` is the FIRST half and proves a word's BYTES. Audit 5 verified
that half COMPLETE over word ASSEMBLY, and this plan does **not** replace it.
Word survival is the SECOND half, and it proves a word's SURVIVAL and its SLOT.
Both are needed, and `T-19-97` is why: `>/dev/null` is fully LITERAL by round 5's
own test and **the bit is right about it** — yet the program never receives it.
What was incomplete was the model, not the inversion.

### The design question, answered explicitly

**Chosen: (a) — `tokenize` MODELS the deletion, so decision-word indices are
computed over the SURVIVING argv. Rejected: (b) — refusing any governed simple
command containing a token the shell deletes.**

Option (b)'s cost, measured and pinned by `19-18`. All permitted today, all
refused under (b):

| command | verdict today |
|---|---|
| `git log > out` | exit 0 |
| `git status > /tmp/s.txt` | exit 0 |
| `git diff > /tmp/d.patch` | exit 0 |
| `git commit -m "x" >> build.log` | exit 0 |
| `gh pr list 2>/dev/null` | exit 0 |
| `git fetch origin 2>&1 \| tee log` | exit 0 |
| **`gh pr create --title x > /tmp/o`** | **exit 0 WITH ONE LEDGER LINE** |

That last row is decisive: (b) turns a correctly **COUNTED** pull-request
creation into a refusal — a false negative traded for a false positive on
ordinary syntax, which is exactly the trade `T-19-93`'s COUNT bar forbids.
Ordinary redirection is common and it keeps working.

### The production as implemented

```
[ IO_NUMBER ] OPERATOR WORD          operator AND target both DELETED, no token for either
OPERATOR ∈ { < > >> <> >| <& >& &> &>> << <<- <<< }
IO_NUMBER = a BARE digits-only run since the start of the word, and nothing else
```

* **`&>` and `&>>` are recognised BEFORE `&` reaches the separator arm**, by an
  earlier match arm. `&` is in `SEPARATORS`, so without that ordering the guard
  splits ONE simple command into TWO and a redirection parser running afterwards
  never sees it.
* **Fail-closed residue**: any unquoted `<`/`>` the production cannot complete
  marks the simple command UNRESOLVABLE and refuses through the arms
  `resolve_program_with_head` already has. A spelling nobody enumerated fails
  closed rather than falling through as an ordinary word.
* **`\`+newline** is consumed as a LINE CONTINUATION producing NO character, in
  the unquoted arm and in the double-quote loop, and it never STARTS a word. The
  SINGLE-quote loop is untouched — bash performs no continuation there
  (`git 'pu\<NL>sh'` gives `[pu\<NL>sh]`, bytes verified with `od -c`) — and
  audit 5's three discarded rows were not re-added.
* **`literal` is NOT cleared** for a redirection word, a redirection target or a
  continuation. A deletion is not a rewrite; the bit is right about all three,
  and clearing it to obtain a refusal would have turned round 5's verdict pins
  green while the inversion stopped being what produced them. `19-18`'s
  non-vacuity pin (`literal == false` for `pus?`, `true` for `>/dev/null` and
  `push`) is green and UNMODIFIED.

**`SEPARATORS` is BYTE-IDENTICAL** — `git diff` reports zero changed lines
matching `SEPARATORS: &[` — and **`policy::is_separator(">")` is still `false`**,
with `separators_are_named_in_one_list_that_the_predicate_reads` green and
unmodified. The doc's FIRST sentence was right and is kept; only its second
sentence's unexamined consequence ("it stays an ordinary word and travels into
the classifier with the rest") was corrected in place.

### Every `19-18` RED row, before and after, with its reason identifier

Confirmed still RED against `1d1229e` before any production line moved.

| row | before | after | derivation |
|---|---|---|---|
| `git >/dev/null push --force origin main` | exit 0 | 2 `force_push_blocked` | operator+target deleted; surviving argv is `push --force origin main` |
| `git 2>/dev/null push --force origin main` | exit 0 | 2 `force_push_blocked` | bare `2` is an IO_NUMBER, discarded with the redirection |
| `git 1>/dev/null push --force origin main` | exit 0 | 2 `force_push_blocked` | same |
| `git >>/tmp/x push --force origin main` | exit 0 | 2 `force_push_blocked` | `>>` matched before `>` |
| `git > /tmp/o push --force origin main` | exit 0 | 2 `force_push_blocked` | whitespace allowed between operator and target |
| `git <<<x push --force origin main` | exit 0 | 2 `force_push_blocked` | `<<<` matched before `<<` and `<` |
| `git push>/dev/null --force origin main` | exit 0 | 2 `force_push_blocked` | attached operator terminates the word; `push` survives |
| `bash -lc "git >/dev/null push --force origin main"` | exit 0 | 2 `force_push_blocked` | `NestedPayload` re-split carries the same fact |
| `echo hi && git >/dev/null push --force origin main` | exit 0 | 2 `force_push_blocked` | per-command reset at the real operator |
| `git >/dev/null stash` | exit 0 | 2 `force_push_blocked` | **no second carrier** |
| `git >/dev/null update-ref -d refs/heads/main` | exit 0 | 2 `force_push_blocked` | **no second carrier** |
| `git >/dev/null config core.hooksPath /tmp/x` | exit 0 | 2 `hook_bypass_blocked` | **no second carrier** — disarming the hook IS the loss of it |
| `git >/dev/null -c core.hooksPath=/dev/null push --force origin main` | exit 0 | 2 `hook_bypass_blocked` | layer 2 defeated on the same line; step 1's identifier is kept (D-24) |
| `git pu\<NL>sh --force origin main` | exit 0 | 2 `force_push_blocked` | continuation deleted; recovered word is `push` |
| `git \<NL>push --force origin main` | exit 0 | 2 `force_push_blocked` | continuation does not START a word — two words, not three |
| `git "pu\<NL>sh" --force origin main` | exit 0 | 2 `force_push_blocked` | the DOUBLE-QUOTE loop's own backslash branch |
| `bash -lc "git pu\<NL>sh --force origin main"` | exit 0 | 2 `force_push_blocked` | nested payload |
| `git sta\<NL>sh` | exit 0 | 2 `force_push_blocked` | **no second carrier** |
| `git config core.hooks\<NL>Path /tmp/x` | exit 0 | 2 `hook_bypass_blocked` | **no second carrier** |
| `git -c core.hooks\<NL>Path=/dev/null pu\<NL>sh --force origin main` | exit 0 | 2 `hook_bypass_blocked` | both words continued |
| `git &>/tmp/o push --force origin main` | exit 0 | 2 `force_push_blocked` | `&>` matched before the separator arm |
| `git >\|/tmp/o push --force origin main` | exit 0 | 2 `force_push_blocked` | two-character operator |
| `git <>/tmp/o push --force origin main` | exit 0 | 2 `force_push_blocked` | two-character operator |
| `git <<EOF push --force origin main` | exit 0 | 2 `force_push_blocked` | heredoc DELIMITER is the target; the body is not argv |
| `touch input.txt && git <input.txt push --force origin main` | exit 0 | 2 `force_push_blocked` | precondition row |

**The four FORGE rows assert a restored COUNT, not a refusal**, because the cap
is bypassed **UNCOUNTED rather than exceeded** and has no second carrier
(`T-19-35`). Each is evidenced by a **walked ledger listing** in a fresh
`GSD_MM_ENVELOPE_ROOT`, not by an exit code:

| row | before | after |
|---|---|---|
| `gh >/dev/null pr create --title x` | exit 0, **EMPTY WALK** | exit 0, **exactly ONE ledger line** |
| `gh >/dev/null api repos/o/r/pulls -f title=x` | exit 0, **EMPTY WALK** | exit 0, **exactly ONE ledger line** |
| `glab >/dev/null mr create --title x` | exit 0, **EMPTY WALK** | exit 0, **exactly ONE ledger line** |
| `gh p\<NL>r create --title x` | exit 0, **EMPTY WALK** | exit 0, **exactly ONE ledger line** |

Derivation: once the deletion is modelled the subcommand words are back in their
slots, so `pr_command_label` matches and the creation is COUNTED. Refusing these
would have traded a restored count for a false positive on ordinary forge syntax.

### The three rows `19-18` measured but could not DERIVE — now derived and pinned

| row | before | after | clause |
|---|---|---|---|
| `git >$F push --force origin main` | 2 `envelope_assertion_failed` | 2 `force_push_blocked` | **identifier change, not a verdict change.** The bit fired on what the guard read as the VERB; the target is now deleted whether or not its text is knowable, so `classify_git` answers about `push --force origin main` |
| `git >*.log push --force origin main` | 2 `envelope_assertion_failed` | 2 `force_push_blocked` | same |
| `git {v}>/tmp/o push --force origin main` | exit 0 | 2 `envelope_assertion_failed` | **a different clause from its six siblings** — the `{name}` fd prefix is not modelled, so the production does not complete and the command is UNRESOLVABLE |

### The two over-refusals REMOVED — the round's cost, measured in the direction nobody expected

| row | before | after | twin | derivation |
|---|---|---|---|---|
| `git push origin refs/heads/gsd-auto/alpha/w > log.txt` | **2 `push_outside_namespace`** | **exit 0** | one-line form, exit 0 | the redirection word was read as an EXTRA REFSPEC (*the refspec `>` resolves to `refs/heads/>`*) |
| `git push \<NL> origin refs/heads/gsd-auto/alpha/w` | **2 `push_outside_namespace`** | **exit 0** | one-line form, exit 0 | the whitespace after the continuation FLUSHED it into its own WORD in the REMOTE slot, displacing every operand one slot right (*the refspec `origin` resolves to `refs/heads/origin`*) — `T-19-97`'s displacement arriving through `T-19-98`'s mechanism |

`19-18` pinned both PRE-fix in two `#[test]` fns it NAMED for this purpose, and
**replacing those two bodies was this plan's ONLY test deletion.** Every other
`tests/` hunk is an addition and every other file under `tests/` is
byte-identical — verified by reading the diff HUNKS, not the numstat total. Each
replaced pin keeps its one-line twin and gains an **out-of-namespace control**
(`git push origin refs/heads/main > log.txt`, still refused), so what the rule
removed is the redirection being read as a decision word rather than the refspec
check itself.

### The controls that show this is a deletion MODEL and not a blanket refusal

* **Over-deletion control, still PERMITTED**:
  `git x2>/tmp/o push --force origin main`. Bash gives git
  `ARGV[git]: [x2] [push] [--force] [origin] [main]` — `x2` IS argv, and git
  itself answers `git: 'x2' is not a git command`. This is the axis's
  `ls {git,svn}-repo`: over-deletion displaces every decision word LEFT, the
  same defect mirrored.
* **A QUOTED digit run is not an IO_NUMBER either.** Measured while executing:
  `git "2">/tmp/o push --force origin main` gives bash `[2] [push] …`, so
  reading the DEQUOTED word text would have over-deleted a real argv word. The
  IO_NUMBER test is therefore over a *bare* digit run tracked while the word is
  consumed — the same "collect the evidence where the word is consumed"
  discipline `Token::literal` is built on. Pinned in the tokenizer table.
* **Quoted `>` rows, still permitted**: `git commit -m ">"`,
  `git log --grep='>'`, `git commit -m "a > b"`, `rg ">" src/`,
  `--push-option="a>b"` — an operator is recognised only OUTSIDE quotes.
* **The ordinary-redirection corpus, still permitted**, and
  `gh pr create --title x > /tmp/o` **still COUNTED** with one ledger line.

### What the rule NEWLY REFUSES, each beside its permitted twin

| newly refused | permitted twin | clause |
|---|---|---|
| `git >` (2 `envelope_assertion_failed`) | `ls >`, `cargo test >` (exit 0) | unresolvable; the mark refuses only when a GOVERNED program is reached. **Bash does not run `git >` either** — `syntax error near unexpected token 'newline'` — so the refusal costs nothing anyone could have run |
| `git {v}>/tmp/o push --force origin main` | `git >/dev/null push --force origin main` (modelled, classified) | `{name}` fd allocation deliberately not modelled |
| `git {v}>/tmp/o status` (was exit 0) | `git >/dev/null status` (exit 0) | same clause — the mark asks whether the argv is KNOWABLE, not whether it is dangerous. **This is a genuinely new refusal and is disclosed rather than hidden** |

Net: **two shapes newly refused, two measured false refusals removed.**

### Audit 5's disclosed corpus limit — ADDRESSED

`sh {-c,"git push --force …"}` is refused because a QUOTE inside an alternative
makes the word's product set unenumerable — correct and fail-closed — but the
corpus cannot tell that refusal apart from an ENUMERATED one, because both reach
the same `envelope_assertion_failed` identifier through the same clause at the
guard boundary. The distinction exists only inside the whole-word product scan,
which is why it is a UNIT assertion:
`an_unenumerable_brace_word_is_distinguishable_from_an_enumerated_governed_one`
in `policy.rs`'s own `#[cfg(test)] mod tests`. It pins `products == None` for the
quoted alternative and `products == Some(["git"])` for `{g..g}it`, with
`{git,svn}-repo` as the control that keeps the `true` answer non-vacuous.
`19-18` could not write it because it may not touch `src/`.

### The decision region did NOT move, and no second reading site was needed

`first_unreadable_decision_word` is **unchanged**, and so is its single
`at(index, role)` closure. So are `scan_leading`, `config_key_operand_index`,
`subcommand_word_indices` and `scan_gh_api`. A deleted word never becomes a
`Token`, so every decision index is over the SURVIVING argv automatically —
round 3's principle (*a decision region is derived from the same scan the
classifier runs, never a second scan*) is **discharged, not weakened**.

`resolve_program_with_head` keeps ONE arm per `ProgramResolution` variant and
**no wildcard**; the deletion fact went into the arms that already existed. There
is still exactly one post-filter.

`src/envelope/hooks.rs` needed **no change**: `guard_in`'s split and the
`NestedPayload` arm's re-split already go through `split_segments_with_heads` and
`resolve_program_with_head`, so the fact threads by construction. That is the
same property that made the second reading site unnecessary.

### A control that moved for a reason worth recording

`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
carries a POSITIVE CONTROL requiring that stripping comments and the
`#[cfg(test)]` module leave more than a QUARTER of `policy.rs` — otherwise an
absence assertion over what remains proves almost nothing. This plan's
documentation and its two new test fns pushed the ratio to **24.88%** and turned
that control red. **The assertion was not edited**; the redundant prose was
tightened until production code was back above the floor. **The margin is now
thin, and the next round that documents `policy.rs` heavily will trip it again** —
recorded here so it is met as a known threshold rather than rediscovered.

### What this record does NOT do

* It writes no unqualified "T-19-60 is closed". Only the **WRAPPER-OPERAND**
  sub-class of `T-19-60` is closed; `T-19-86` and `T-19-91` are both sub-classes
  of it and both remain OPEN at `high`.
* **The `T-19-17r` bookkeeping gap stays OUTSTANDING.** `19-17-SUMMARY.md` calls
  it accepted; the measurement and both pins are confirmed; there is still no
  `AR-19-13` row and no register row. **This plan does NOT accept it and adds no
  `AR-19-13` row** — accepting a risk is a human decision and audit 5 explicitly
  declined to make it.
* It does not clear `/gsd-secure-phase 19`. `T-19-86` (four rows still at exit 0,
  its control green and unmodified) and `T-19-91` (three arms — `reflog $S`,
  `reflog show $S`, `symbolic-ref $S` at exit 0 with no second carrier, and the
  bare `git push $REF` cwd-dependent and permitted in-namespace) remain OPEN at
  `high`. `T-19-96`, `T-19-74`'s residual, `T-19-84`, `T-19-85` and
  `T-19-61` … `T-19-73` are untouched and unaccepted.
* It adds no crate. `Cargo.toml` and `Cargo.lock` are unchanged (`T-19-SC`).

---

## Threats found by audit 6 (2026-08-29, after plans 19-18 and 19-19)

**Provenance, stated first.** The two subsections above this one were written by
the EXECUTORS of plans 19-18 and 19-19 and are deliberately outside the audit
tables; audit 6 left them, and every earlier appended subsection, byte-identical
(checksummed before and after writing). Everything from here to the end of the
file is **audit 6's own**, measured against the built binary at `9534198` with a
fresh `GSD_MM_ENVELOPE_ROOT` per row and the envelope directory walked
afterwards, and with every claimed bypass re-run under `bash` against
argv-printing `git`/`gh`/`glab` shims that **write to a side file rather than to
stdout** — the trap `19-18` recorded, and mandatory here because a row whose
whole point is `>/dev/null` otherwise swallows its own evidence.

### The question this audit was set, answered plainly

**Is there a THIRD thing the guard assumes about the relationship between the
command line it reads and the argv the program receives?**

**The command-line-to-argv boundary is now correctly modelled. Audit 6 found no
third shell mechanism, and says so plainly.** Word ASSEMBLY (round 5) and word
DELETION (round 6) are the two halves, and together they are complete over the
transformation bash performs between the string and `execve`. Audit 6
enumerated what stands between a command line and argv one mechanism at a time —
quote removal, every expansion, redirection, line continuation, assignment-prefix
removal, control-operator splitting, here-document delimiters, pipeline and group
nesting — and every one is either modelled or fails closed. There is **no
reordering or relocation step in a simple command** for a third rule to miss:
bash removes words (redirections, assignments) and rewrites words (expansions),
and it does nothing else to their order. The invariant `19-19` states —

> *the words the guard classifies must be exactly the words the program
> receives, in the same order — no more and no fewer*

— is **true of the implementation as measured**, on all 27 redirection rows, all
11 continuation rows, all four forge COUNT rows and every over-deletion control.

**And the class is still open, because the boundary was never the whole
question.** Having reconstructed argv correctly, the guard must still decide
**which word in it is the verb** — and it does that with a hand-maintained
enumeration of *git's own option grammar* that **fails OPEN** on any option it
does not know. That is the third assumption, and it is not about the shell at
all:

* `git --attr-source HEAD push --force origin main` → **exit 0**. Every word is
  literal, no word is deleted, the order is preserved, and the guard reads all
  six words. `Token.literal` is `true` for every one of them and **the bit is
  right**. The guard simply calls `HEAD` the verb where git calls `push` the
  verb, because `--attr-source` consumes its value and `GIT_GLOBAL_VALUE_OPTS`
  does not say so (`T-19-100`).

**So the pattern held for a sixth round, and it moved axis again** — from *how a
word is written* (rounds 1–5), to *which words arrive* (round 6), to **whose
grammar decides which arriving word is the verb**. Rounds 1–6 all modelled the
SHELL. Nothing in this phase models the CALLEE.

### The new rows

| Threat ID | Category | Component | Severity | Disposition | Finding | Status |
|---|---|---|---|---|---|---|
| **T-19-100** | **Elevation of Privilege** | **`scan_leading` / `leading_git_option` / `GIT_GLOBAL_VALUE_OPTS` (`policy.rs:460-489`) — an incomplete, unpinned enumeration of git's own global-option grammar, which fails OPEN** | **high** | **mitigate (unimplemented)** | **`scan_leading` finds the git verb by walking leading `-`-initial tokens and asking `leading_git_option` how many words each occupies. For an option it does not recognise the answer is `(None, 1)`, so the loop advances one word, hits the option's VALUE, sees it does not start with `-`, and **breaks with that value as the verb**. `classify_git` then finds a verb in no denylist arm and answers `Allow`. `GIT_GLOBAL_VALUE_OPTS` lists five options; git 2.43 has **two more** that consume a separate value, and the list also carries `--super-prefix`, which git 2.43 no longer accepts — it is **out of step in both directions**. Measured at `9534198`, one fresh root per row, walk after: `git --attr-source HEAD push --force origin main` → **exit 0**; `git --attr-source HEAD stash` → **0**; `git --attr-source HEAD update-ref -d refs/heads/main` → **0**; `git --attr-source HEAD config core.hooksPath /tmp/x` → **0**; `git --attr-source HEAD reflog delete HEAD@{0}` → **0**; `git --attr-source HEAD symbolic-ref HEAD refs/heads/x` → **0**; and the whole set again under `--shallow-file /tmp/s`: `push --force`, `stash`, `update-ref -d`, `config core.hooksPath`, `reflog delete` all → **0**. The ATTACHED spellings are correctly refused — `git --attr-source=HEAD push --force origin main` → **exit 2** `force_push_blocked` — so this is the separate-value form specifically. **Confirmed against the REAL git binary, not argued**: `git --attr-source XVALUE version` prints `git version 2.43.0` (the value was consumed and the verb was reached) against the control `git --bogus-opt XVALUE version` → `unknown option: --bogus-opt`; `git --shallow-file /tmp/s stash` really ran stash (`Saved working directory and index state WIP on rewritten`); `git --attr-source HEAD update-ref -d refs/heads/nonexistent` exits 0. Under the shims bash hands git `ARGV[git]: [--attr-source] [HEAD] [push] [--force] [origin] [main]` — every word literal, none deleted, order preserved. **A REAL destructive force push was performed**, which no predecessor row in this phase demonstrated: in a fixture with a bare upstream, `git --attr-source HEAD push --force origin rewritten:main` printed `+ fd70e8d...347b427 rewritten -> main (forced update)` and the remote `main` was rewritten to an unrelated history. **The three-leg argument, each leg measured.** Layer 1 — the line begins `git --attr-source`, which does not match the `Bash(git push:*)` prefix rule the agent CLI owns. Layer 2 — exit 0 above, empty walk. Layer 3 — `git --attr-source HEAD -c core.hooksPath=/dev/null push --force origin main` → **exit 0**, and re-measured on this machine `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=/ENV_WINS git --attr-source HEAD -c core.hooksPath=/CLI_WINS config --get core.hooksPath` prints **`/CLI_WINS`** with the paired control (same line without `-c`) printing **`/ENV_WINS`**, so no `pre-push` hook runs. Credentials — `GIT_ASKPASS`, `GIT_CONFIG_GLOBAL/SYSTEM` and `GIT_SSH_COMMAND` are untouched, so the push authenticates. Four of the rows have **no second carrier at all** — `git stash`, `git update-ref`, `git reflog delete` and `git config core.hooksPath`, where disarming the hook IS the loss of the carrier. **NOT covered by AR-19-03.** That acceptance is a denylist over *git verbs*, resting on "the pre-push and pre-commit hooks make the denylist's gaps non-fatal"; here the permitted word is not a git verb at all, it is an option's operand, and the same line disarms the hooks. **NOT the same class as `T-19-91`/`T-19-96`**, which are classifier arms answering `Allow` on an operand they cannot READ; here every word is read and readable and the verb INDEX is wrong. **Not word assembly and not word deletion**: `19-19`'s invariant holds on these lines exactly. **The asymmetry that names the fix**: `resolve_program`'s wrapper axis solved this identical problem by failing CLOSED — a governed candidate whose command position cannot be established structurally is `Refuse`, not mis-indexed (audit 3, `T-19-60`) — and audit 6 re-confirmed it holds (`env --chdir=/tmp`, `env -C /tmp`, `timeout --signal TERM 60`, `nice -n 5`, `sudo -u nobody`, `env -S "…"` all → exit 2). `scan_leading` is the one arm that was never given that treatment. **`GIT_GLOBAL_VALUE_OPTS` has NO drift pin and no test reference anywhere** — `grep -rn GIT_GLOBAL_VALUE_OPTS src/ tests/` returns exactly two hits, its definition and its single use — where the phase's other hand-maintained surface list, `ENVELOPE_ENV_KEYS`, was given a re-sourced pin with four named floors after being wrong twice (`T-19-82`, `T-19-90`). | **OPEN — BLOCKING** |
| T-19-101 | Spoofing | every generative alphabet, and both named class axes (`UNREADABLE_CLASSES`, `DELETION_CLASSES`) | medium | mitigate | **`T-19-76`'s failure mode for the SIXTH consecutive round, and this time the axis moved rather than the cell.** Round 6's corpus is real and it closed `T-19-99` outright — `DELETION_CLASSES` stands beside a byte-identical `UNREADABLE_CLASSES`, and audit 5's own two greps, which returned NOTHING at `ad847b4`, now return **35** and **69** hits for a redirection character inside a driven string and carry eleven real backslash-newline escapes. But **both axes are axes of the SHELL's grammar**: `UNREADABLE_CLASSES`' seven classes are ways bash REWRITES a word, `DELETION_CLASSES`' five are ways bash REMOVES one. **Nothing anywhere in `tests/` models the CALLEE's grammar** — verified mechanically: `grep -rn "attr-source\|shallow-file\|GIT_GLOBAL_VALUE_OPTS" tests/` returns **nothing at all**, and no alphabet carries a leading git option that consumes a separate word. The corpus is therefore structurally incapable of generating, and so of failing on, `T-19-100`. This is the same finding as `T-19-76`, `T-19-83`, `T-19-89`, `T-19-95` and `T-19-99`, one axis further out — and the axis is again the point: each round widened the alphabets along the axis its own control is about, so the corpus keeps modelling exactly the control it certifies and nothing beside it. | open — below `high` (non-blocking) |
| T-19-102 | Denial of Service | `push_operands` / `PUSH_VALUE_OPTS` (`policy.rs:509`) — the same enumeration defect, in the over-refusal direction | low | mitigate | `PUSH_VALUE_OPTS` is `["repo", "push-option", "receive-pack", "exec"]`. `git push` in git 2.43 also consumes a separate value for `--recurse-submodules`, which is absent, so its value is read as a push OPERAND and the real repository is then read as a refspec. Measured in a fixture repository on `refs/heads/gsd-auto/alpha/w` with an upstream configured: `git push --recurse-submodules on-demand origin refs/heads/gsd-auto/alpha/w` → **exit 2 `push_outside_namespace`** (*the refspec `origin` resolves to `refs/heads/origin`*), while its twin `git push origin refs/heads/gsd-auto/alpha/w` → **exit 0** and real git runs the line to completion (`Everything up-to-date`). A **false refusal of an ordinary in-namespace push**, the same class of cost `19-19` measured itself removing two of. Rated `low`: it is a usability cost inside driven runs, legible rather than silent, and it fails in the safe direction. Recorded separately from `T-19-100` because the component and the direction both differ, and this phase's discipline is not to let one row imply what it did not measure. The short spellings are **not** affected — `git push -o ci.skip origin refs/heads/gsd-auto/alpha/w` and `--push-option ci.skip` both → exit 0. | open — below `high` (non-blocking) |

*Status: open · closed · open — below `high` threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` count toward `threats_open`*

`T-19-101` and `T-19-102` are open below the `high` threshold and do **not**
count toward `threats_open`. `T-19-100` does.

### Audit 6's bookkeeping, re-derived from audit 5's group rows

| Group | Count | Closed | Open |
|---|---|---|---|
| Everything through audit 5 | 111 | 90 | 21 (4 at `high`) |
| Closed by plans 19-18 / 19-19, re-measured by audit 6 (`T-19-97`, `T-19-98`, `T-19-99`) | — | +3 | −3 |
| Found by audit 6 (`T-19-100` … `T-19-102`) | 3 | 0 | 3 (1 at `high`) |
| **Total after audit 6** | **114** | **93** | **21 (3 at `high`)** |

The three that count toward `threats_open`: `T-19-86`, `T-19-91`, `T-19-100`.
The eighteen that do not: `T-19-61` … `T-19-73` (13), `T-19-84`, `T-19-85`,
`T-19-96`, `T-19-101`, `T-19-102`.

### The closures, re-measured rather than accepted from the summaries

Driven as
`printf '<PreToolUse JSON>' | GSD_MM_ENVELOPE_ROOT=$(mktemp -d) ./target/debug/gsd-meta-manager envelope guard alpha`,
one fresh root per row, the envelope directory walked with `os.walk`
afterwards.

* **`T-19-97` — CLOSED, and the operator set is complete.** All sixteen audit-5
  rows and all seven planning cells are at `exit=2` with an empty walk, including
  the `&`-separator fragmentation cell `git &>/tmp/o push --force origin main`,
  `&>>`, `>|`, `<>`, `<<`, `<<-EOF`, `<<<`, `>>`, the space-separated
  `git > /tmp/o push …`, the attached `git push>/dev/null --force …`, the
  precondition row `git <input.txt push …`, the nested payload, the brace group
  `{ git >/dev/null push --force origin main; }` and the sequence
  `echo hi && git >/dev/null …`. **Audit 6 swept the redirection operators the
  corpus does not enumerate and every one is also modelled**: `22>` (multi-digit
  fd), `2>&1`, `>&2`, `2>&-`, `0<&-`, `3<&0`, `2>>`, and the pipeline position
  `echo x | git >/dev/null push --force origin main` — all `exit=2`. The
  fd-allocation forms fail CLOSED as disclosed: `git {v}>/tmp/o push …` and
  `git {v}<&1 push …` → `exit=2 envelope_assertion_failed`.
* **`T-19-98` — CLOSED.** All eight rows at `exit=2` with an empty walk, except
  the forge row, which is a restored COUNT (below). The double-quoted spelling,
  the nested `bash -lc` payload and the two-continuation
  `git -c core.hooks\`+NL+`Path=/dev/null pu\`+NL+`sh …` all refuse under the
  identifier `19-19` derives. **Audit 5's three discarded rows stay correctly
  permitted** and were re-measured to confirm the single-quote loop was not
  touched: `git 'pu\`+NL+`sh' …`, `git pu\`+CR+`sh …` and `git pu\`+TAB+`sh …`
  all → exit 0, which is right, because bash performs no continuation in any of
  them.
* **The four FORGE rows — CLOSED on the COUNT bar, evidenced by a walked ledger
  listing rather than an exit code.** In one fresh root, two calls each:
  `gh >/dev/null pr create --title x`, `gh >/dev/null api repos/o/r/pulls -f
  title=x`, `glab >/dev/null mr create --title x` and
  `gh p\`+NL+`r create --title x` are all **call 1 exit 0 with exactly one
  ledger line, call 2 exit 2 `pr_cap_exceeded` with two** — byte-for-byte the
  behaviour of the unwrapped controls. A refusal here would have been the
  regression and there is none.
* **`T-19-99` — CLOSED.** The two greps audit 5 ran to prove the corpus could
  not draw the class now return 35 and 69 hits, `DELETION_CLASSES`,
  `DISPLACING_REDIRECTIONS` and `CONTINUATION_SPLICES` all exist, and the corpus
  demonstrably failed on its own class: `19-18` committed 5 named RED tests with
  **zero `src/` hunks** and `19-19` turned all 5 green. **The corpus's NEW blind
  spot is `T-19-101`.**
* **`T-19-93` did not degrade under the new rule.** Re-measured: the unquoted
  placeholder form is still counted (call 2 `pr_cap_exceeded`, two ledger
  lines), the `-X POST` spelling is counted, and the `…/pulls/7` boundary still
  writes **zero** ledger lines in both the quoted and unquoted spellings.

### The two removed over-refusals — genuinely correct, and masking no real refusal

Measured in a purpose-built fixture on `refs/heads/gsd-auto/alpha/w` with a local
bare upstream, the repository passed as the guard's working directory:

```
exit=0  git push origin refs/heads/gsd-auto/alpha/w > log.txt      <- was 2, now correct
exit=0  git push \<NL> origin refs/heads/gsd-auto/alpha/w          <- was 2, now correct
exit=0  git push origin refs/heads/gsd-auto/alpha/w                <- the one-line twin
exit=2  push_outside_namespace   git push origin refs/heads/main > log.txt
exit=2  push_outside_namespace   git push origin refs/heads/main
exit=2  force_push_blocked       git push --force origin refs/heads/gsd-auto/alpha/w > log.txt
```

**Both removals are correct and neither masks a real refusal.** The
out-of-namespace control still refuses *through* a redirection, and a `--force`
behind a redirection still refuses — so what the rule removed is the redirection
being read as a decision word, not the refspec check and not the flag check.

### Ordinary redirection, and the deletion model's two-sided controls

Every ordinary row is still permitted and the counted one is still counted:
`git log > out`, `git status > /tmp/s.txt`, `git diff > /tmp/d.patch`,
`git commit -m "x" >> build.log`, `gh pr list 2>/dev/null`,
`git fetch origin 2>&1 | tee log` and `gh pr create --title x > /tmp/o`
(**one ledger line, cap fires on call 2**). The quoted-operator corpus is
unaffected: `git commit -m ">"`, `git log --grep='>'`, `git commit -m "a > b"`,
`rg ">" src/` and `--push-option="a>b"` all exit 0. The two disclosed new
refusals fire exactly as `19-19` records — `git >` and `git {v}>/tmp/o status`
at `exit=2`, beside `ls >` and `git >/dev/null status` at exit 0.

**Over-deletion: audit 6 probed the silent direction and found none, and the
argument is structural rather than a list.** Every row where bash KEEPS a word
the guard might have deleted is permitted, and each was confirmed under the
shims: `git "2" >/tmp/o push …` → `[2] [push] …`, `git 2 >/tmp/o push …` →
`[2] [push] …`, `git ">"/dev/null push …` and `git \>/dev/null push …` →
`[>/dev/null] [push] …`, `git 2x>/tmp/o push …` → `[2x] [push] …`, and the
mandated control `git x2>/tmp/o push --force origin main` → **exit 0** with
`[x2] [push] [--force] [origin] [main]`. Real git rejects every one of those
words as a verb (`git: 'x2' is not a git command`, and the same for `2`, `2x`
and `>/dev/null`), so each permit is correct. **Over-deletion on this axis
cannot produce a bypass by construction**: for the guard to delete the real verb
it must see an operator immediately before it that bash does not, in which case
that operator-shaped word becomes bash's own `argv[0]` — and `>`-initial and
digit-initial words are not git verbs. Over-deletion here can only over-refuse
or permit something harmless, which is why the mirror of `ls {git,svn}-repo`
holds on this axis too.

### The known-open set, as audit 6 verified it

- **`T-19-86` (high, OPEN, unaccepted, deferred by explicit user scoping
  decision).** All four rows re-measured at `9534198`, fresh root each, all at
  **exit 0**: `git submodule foreach git push --force origin main`,
  `git rebase -x "git push --force origin main" HEAD~3`,
  `git bisect run sh -c "git push --force origin main"`,
  `git -c alias.p='!git push --force origin main' p`. Exactly as registered.
  Counts toward `threats_open`.
- **`T-19-91` (high, OPEN, three arms).** `git reflog $S`, `git reflog show $S`
  and `git symbolic-ref $S` reproduce at **exit 0**; `git symbolic-ref HEAD $R`
  → 2 `force_push_blocked` and `git push origin $REF` → 2
  `push_outside_namespace` still fail closed; and the third arm reproduces in
  the in-namespace fixture — `git push $REF` → **exit 0**, cwd-dependent and
  permitted in the configuration the envelope exists for, **not** "already fails
  closed". The record is correct as `19-19` leaves it and the second-carrier
  asymmetry is unweakened. Counts toward `threats_open`.
- **`T-19-96` (medium, OPEN, registered, not fixed).** `git push --forc? origin
  refs/heads/gsd-auto/alpha/w` → **exit 0**, its literal twin → exit 2
  `force_push_blocked`. Unchanged by round 6. Audit 6 notes it is now one of
  THREE arms of the same shape as `T-19-91` and, with `T-19-100`, one of four
  places a `classify_push`/`scan_leading` decision is made on a word the
  decision region does not cover.
- **`T-19-74` (medium, closed/accepted — AR-19-10).** Core rows frozen and
  re-measured permitted: `env $X push --force origin main` → **0** and
  `X=git; env $X push --force origin main` → **0**. `T-19-84` remains open and
  unaccepted.
- **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85` — carried forward untouched**,
  open and unaccepted at their original severities, all below `high`. Plans
  19-18 and 19-19 touched `src/envelope/policy.rs` and one test file only;
  `cred.rs`, `mod.rs`, `advisory.rs`, `scan.rs`, `config.rs` and **`hooks.rs`**
  were not opened, so `T-19-61` … `T-19-73` cannot have moved.
- **`T-19-SC` — still holds.** Neither `Cargo.toml` nor `Cargo.lock` was touched
  by any plan-19-18 or plan-19-19 commit.
- **`T-19-17r` — still OUTSTANDING, and audit 6 does not resolve it either.**
  The measurement and both pins are re-confirmed (`rg "git status" {src,tests}`
  → exit 2 `envelope_assertion_failed`, `rg "git status" src/` → exit 0). There
  is still **no `AR-19-13` row and no register row**. `19-18` and `19-19` both
  correctly declined to make the acceptance and both avoided calling it
  accepted. **Accepting a risk is a human decision and this audit does not make
  it**, for the same reason audit 5 declined. The next round either adds the log
  row or drops the word from `19-17-SUMMARY.md`.

### The four execution-time judgements, assessed independently

1. **The IO_NUMBER over-deletion bug — the fix is CORRECT, and audit 6 probed
   the direction it guards and found nothing left.** The plan's production
   ("digits-only run since word start") was genuinely ambiguous about quoting,
   and tested over the *dequoted* text it over-deletes: bash gives
   `git "2">/tmp/o push --force origin main` the argv `[2] [push] [--force]
   [origin] [main]`, confirmed by audit 6 under the shims. The fix — tracking
   bare-ness while the word is consumed — is the same "collect the evidence
   where the word is consumed" discipline `Token::literal` is built on, and it
   is the right generalisation rather than a special case for the quoted digit.
   Both mandated controls hold: `git x2>/tmp/o push --force origin main` and
   `git "2">/tmp/o push --force origin main` are **permitted**, and audit 6
   added four more over-deletion probes (the space-separated `2`, the
   double-quoted, single-quoted and backslash-escaped `>`) which are all
   permitted and all confirmed harmless against real git. **Finding it by
   measurement rather than from the plan is the round's best moment**, and
   reporting it as a deviation rather than absorbing it is correct.
2. **`hooks.rs` needing no change — CORRECT, and verified rather than accepted.**
   `git diff --stat 1d1229e..HEAD -- src/` is exactly one file,
   `src/envelope/policy.rs`. The claim that the fact threads by construction is
   checkable and checks out: `redirection_unresolvable` is set in `tokenize`,
   carried on `Token` and `Segment`, and **read at exactly one decision site** —
   `resolve_program_with_head` (`policy.rs:3258`) — with the only other
   occurrence a unit test. **No second reading site was introduced**, and
   round 3's principle is discharged rather than weakened: a deleted word never
   becomes a `Token`, so every decision index is over the surviving argv
   automatically. The post-filter is exhaustive with one arm per variant
   (`Governed | NestedPayload`, `NoProgram | Ungoverned`, `Refuse`) and **no
   wildcard** — the only `other =>` in the function is inside a doc comment
   describing the form it replaced.
3. **The production/comment ratio control — the CALL was right, the CONTROL is
   no longer meaningful as written.** Not editing an assertion to go green is
   the correct instinct and the only acceptable one; tightening prose instead of
   lowering a floor is exactly the discipline five rounds of plan-check exist to
   enforce. **But audit 6 re-derived the ratio independently and the margin is
   ~0.09 percentage points** — about 250 bytes of headroom in a 262 KB file.
   At that margin the control has stopped measuring what it was written to
   measure. Its purpose is anti-vacuity: an absence assertion over
   `production_code` proves nothing if the stripper ate the file. With ~66 KB of
   stripped production code that purpose is served with enormous margin, and the
   floor now functions as a **documentation budget** whose red says "the
   stripper broke" while meaning "someone wrote comments". Worse, it now pulls
   directly against this phase's own doc-disclosure pins — controls such as
   `resolve_programs_own_doc_still_discloses_the_residual_it_does_not_cover`
   REQUIRE specific paragraphs to exist, so a ratio floor rewards deleting the
   prose another control requires. **Audit 6's judgement: keep the anti-vacuity
   property, re-express it as an absolute floor on stripped production bytes
   rather than a ratio against a file whose comment volume is itself a
   deliberate security artifact.** Recorded as an observation, not registered as
   a threat: nothing is unprotected today.
4. **The third flake — correctly classified, but it is not new.** Audit 6 ran
   `a_relocated_copy_of_the_stub_refuses_instead_of_acting` eight times in
   isolation: **8/8 green**, and green in the full gate. It does belong beside
   the two `driver_reattach` flakes — same kind of item, environmental rather
   than about the policy under test. **One correction:
   `deferred-items.md` has carried it since 19-07**, under
   "`tests/envelope_tracer.rs` — ETXTBSY when a just-copied stub is exec'd",
   with the same symptom string, the same write-then-exec diagnosis, the same
   "once in ~6 full-suite runs" frequency and a fix direction. `19-19-SUMMARY.md`
   presents it as newly observed. That is a provenance slip and not a defect,
   and the re-observation independently corroborates the 19-07 frequency
   estimate. It touches no threat's evidence: the test is `T-19-01`'s, and a
   flake that ERRORS rather than passing vacuously cannot mask a regression.

### Gates observed

`rtk proxy cargo test --no-fail-fast` redirected to a file and counted with
`rtk proxy grep` (a plain `grep` is rewritten by the RTK hook, which strips the
`test result:` lines a count is read from — the trap `19-19` hit and recorded):
**1610 passed, 0 failed, 13 ignored** over 41 result lines,
`passed + failed = 1610`, matching `19-19-SUMMARY.md` exactly. All twelve
`envelope_*` binaries ran. The two flaky `driver_reattach` tests passed in this
run. `rtk proxy cargo clippy -- -D warnings` exits 0;
`cargo clippy --all-targets` was already failing at the base commit on four
pre-existing lints in `src/browser.rs` and `src/project_creator.rs` and is out
of scope.

**Provenance discipline.** Plans 19-13 through 19-19's appended subsections and
every earlier audit's own tables are left **byte-identical**; audit 6 checksummed
the whole body below the frontmatter before and after writing. Audit 6's
corrections to statements made in those subsections are recorded as audit-6
findings BESIDE them rather than as edits to them.

---

## Audit 6 — what the round-6 control can and cannot fail on

### The principle rounds 3, 4 and 5 established still holds

A decision region must be derived from the same scan the classifier runs, and
there is one walk. `first_unreadable_decision_word` is unchanged, including its
single `at(index, role)` closure; `scan_leading`, `config_key_operand_index`,
`subcommand_word_indices` and `scan_gh_api` are untouched;
`split_segments` is still defined over `split_segments_with_heads`;
`SEPARATORS` is byte-identical — one commit in the whole phase (`84a9b05`,
plan 19-05) has ever touched that line — and `is_separator` is a bare
`SEPARATORS.contains`, so `is_separator(">")` is `false`. The deletion fact was
added to the arms `resolve_program_with_head` already had rather than to a new
reading site. **Adding `>` and `<` to `SEPARATORS` was prohibited and was not
done**; audit 6 re-affirms that the doc's reasoning is correct.

### Where the boundary now is, in one paragraph

**Rounds 5 and 6 together decide two halves of one invariant, and audit 6 finds
that invariant COMPLETE.** A decision word must be provably LITERAL, evidenced
positively while it is consumed; and the words the guard classifies must be
exactly the words the program receives, in the same order — modelled by deleting
what bash deletes, with a fail-closed residue for any redirection the production
cannot complete. Audit 6 enumerated bash's transformations between the command
string and `execve` against them one at a time and **found no third mechanism**:
there is no reordering step, splitting and rewriting are the assembly half,
removal is the deletion half, and every operator, fd form, heredoc spelling,
nesting and pipeline position probed is either modelled or refuses.

**What they cannot fail on is not a property of the shell at all.** Having built
argv correctly, the guard must locate the verb WITHIN it, and that requires
knowing the CALLEE's option grammar:

- **an option of git's own that consumes a separate word** — `--attr-source`,
  `--shallow-file` (`T-19-100`), where the guard fails OPEN on an option it does
  not know, and where the same list already carries an option (`--super-prefix`)
  that the installed git rejects;
- **a classifier arm's own operand or flag outside the decision region** —
  `T-19-91` (`reflog`, `symbolic-ref`, the bare `push $REF`) and `T-19-96` (a
  push flag), both registered;
- **a whole command line handed to a governed program as data** — `T-19-86`,
  unchanged, and the one residual that has survived every round.

**The one-sentence version for the next round.** Six rounds have modelled the
shell — how a word is written, then which words arrive — and none has modelled
the program the words are handed to; `T-19-101` is the mechanical form of it, in
that no alphabet in this phase contains a single leading option of git's own that
consumes a separate word.

### Suggested closure, in order — (d) FIRST, for the sixth round running

1. **(d) — widen the corpus BEFORE certifying anything.** Add a THIRD named
   axis beside `UNREADABLE_CLASSES` and `DELETION_CLASSES` — *a word whose SLOT
   the callee's own option grammar decides* — with a `GIT_GLOBAL_OPTIONS`
   alphabet carrying separate-value options (`-C`, `-c`, `--git-dir`,
   `--work-tree`, `--namespace`, `--config-env`, **`--attr-source`**,
   **`--shallow-file`**) and value-less ones (`--no-pager`, `--bare`,
   `--literal-pathspecs`) spliced between the governed program and its decision
   words, with the `MIN_*`-floor pattern asserting the corpus generates them.
   **Listed first for the sixth round running**, and for the sixth round running
   it is the recommendation the previous audit made in a different cell and the
   next gap was in the cell it did not name.
2. **(a) — `T-19-100`, and make it fail CLOSED rather than complete a list.**
   Completing `GIT_GLOBAL_VALUE_OPTS` closes today's two cells and will be wrong
   again at the next git release — this list is already wrong in both directions
   against git 2.43. The durable shape is the one `resolve_program`'s wrapper
   axis already uses: **a leading option `leading_git_option` does not recognise
   makes the verb slot unestablished, and an unestablished verb is `Refuse`, not
   the next non-`-` word.** The paired cost must be pinned from both sides
   exactly as Rule B's and clause 2's were: every value-less global option
   (`--no-pager`, `--bare`, `--literal-pathspecs`, `--no-optional-locks`,
   `--no-replace-objects`, the pathspec family) must keep reaching its ungrouped
   verdict, and `git --no-pager status` must stay exit 0. Whichever shape is
   chosen, give the list the drift pin `ENVELOPE_ENV_KEYS` was given after being
   wrong twice — sourced from a place that changes when git's grammar does, with
   a floor that turns red if it is narrowed.
3. **(b) — `T-19-102`**, the same enumeration in `push_operands`, closed by the
   same change and measured the same way: the in-namespace twin must keep
   reaching exit 0.
4. **(c)** — then `T-19-86`, `T-19-91` and `T-19-96`, which are three arms of
   one shape: a classifier arm answering `Allow` on an operand, flag or payload
   outside the decision region.

Then, and separately: add the `AR-19-13` row for `T-19-17r` or stop calling it
accepted; and re-express the production/comment ratio control as an absolute
floor on stripped production bytes.

---

## Audit 6 Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log — twelve, unchanged by
      round 6; audit 6 accepts nothing new
- [x] Every closure re-measured against the built binary at `9534198` with a
      fresh envelope root per row and the directory walked afterwards
- [x] Every new finding confirmed under `bash` against argv-printing shims
      **writing to a side file**, and `T-19-100` additionally confirmed against
      the REAL `git` binary, including a real force push that rewrote a bare
      remote's `main`
- [x] The four forge rows verified on the COUNT bar by a walked ledger listing
      with the cap firing on the second call, not by an exit code
- [x] Over-deletion probed as the silent direction and none found; the two
      mandated over-deletion controls (`x2>`, `"2">`) remain permitted
- [x] `SEPARATORS` byte-identical and `is_separator(">") == false` verified
      mechanically; `>` and `<` were not added
- [x] Plans 19-13 … 19-19's appended subsections left byte-identical, verified
      by checksum before and after writing
- [x] Gate observed: `rtk proxy cargo test --no-fail-fast` → **1610 passed, 0
      failed, 13 ignored**, `passed + failed = 1610`, matching
      `19-19-SUMMARY.md` exactly; all twelve `envelope_*` binaries ran;
      `cargo clippy -- -D warnings` exit 0
- [ ] `threats_open: 0` confirmed — **3 open at `high`: `T-19-86`, `T-19-91`,
      `T-19-100`**
- [ ] `status: verified` set in frontmatter

**Approval: blocked, 2026-08-29 (audit 6).**

**Not accepted here.** `T-19-100` is a high-severity, empirically confirmed
bypass which defeats layers 1, 2 and 3 on a single line with the credential
intact, carries four rows with **no second carrier**, and is the only finding in
six rounds demonstrated by performing a real destructive force push against a
real remote. `T-19-86` and `T-19-91` remain open at `high` by scoping decision
and by round discipline respectively. Accepting any of the three is a human
decision and this audit does not make it.

**Progress is real, and the boundary question is answered.** Round 6 closed
`T-19-97` and `T-19-98` structurally rather than row by row — a finite grammar
production with a fail-closed residue, not an enumeration of spellings — and
closed `T-19-99` with a second named axis that demonstrably failed on its own
class before the rule existed. It did so while **removing** two measured false
refusals, which is the first round in this phase whose net effect on over-refusal
was negative, and it modelled the deletion inside the one walk so no second
reading site was needed and `hooks.rs` was not opened. Audit 6 says plainly what
five rounds of this phase have been building toward: **the command-line-to-argv
boundary is now correctly modelled, and there is no third shell mechanism.** The
open class is one level past that boundary — having reconstructed argv, the guard
must decide which word in it is the verb, and it asks a hand-maintained list
about another program's grammar and fails open when the list is silent. Six
rounds have modelled the shell. The next one has to model git.

## Execution record — plan 19-20 (the corpus, RED). NOT an audit finding.

**Attribution.** This subsection is a planning-and-execution record made by plan
`19-20`. It is **not** an audit finding, it does not re-classify any row in an
audit's tables, and it changes nothing above it. Re-measuring and re-classifying
these rows is `/gsd-secure-phase 19`'s job; an audit's own tables are its
provenance, and a plan writing into them forges it. Everything below was measured
at base commit `c21c13f` against the built binary `./target/debug/gsd-meta-manager`
with one fresh `GSD_MM_ENVELOPE_ROOT` per row and the whole envelope root walked
afterwards, or against the real `git` binary (`git version 2.43.0`).

**This plan closes nothing, and it clears no gate.** `T-19-100`, `T-19-101` and
`T-19-102` all stay OPEN at its end; `T-19-101` is closed only when `19-21`'s rule
is certified by the corpus written here, because a corpus is evidence about a
control and there is no control yet. `T-19-86` and `T-19-91` both remain **OPEN at
`high`**, so **`/gsd-secure-phase 19` is NOT cleared by this plan, by `19-21`, or
by the two together.** Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.

### The invariant this round is about

> **The word the guard calls the verb must be the word git calls the verb.**

Rounds 5 and 6 closed the two halves of the boundary BELOW this one, and audit 6
verified that closure mechanically: a decision word must be provably LITERAL (no
gap in word ASSEMBLY), and the words the guard classifies must be exactly the
words the program receives in the same order (no reordering step in a simple
command for a third rule to miss). **The command-line-to-argv boundary is closed.**
This round is a layer above it, not a seventh spelling of the same class: given
the right words in the right order, *which one is the command?* — a question about
git's grammar, not bash's.

The mechanism, exactly: `scan_leading` (`policy.rs:325-423`) walks leading
`-`-initial tokens and asks `leading_git_option` how many words each occupies. Its
answer for an unrecognised option is `(None, 1)` at **`policy.rs:488`**, and that
single `1` is the whole of `T-19-100`. The loop advances one word, lands on the
option's VALUE, sees it does not start with `-`, and BREAKS with that value as the
verb. `classify_git` then finds a verb in no denylist arm and answers `Allow`.
Every word is literal and `Token.literal` is `true` for all of them — round 5's
bit is RIGHT. No word is deleted and the order is preserved — round 6's model is
RIGHT. What is wrong is the verb INDEX.

### The `T-19-100` rows — measured, with their walks

Every row: fresh envelope root, whole root walked after, **EMPTY in every one**.

| Command | exit | reason id | walk |
|---|---|---|---|
| `git --attr-source HEAD push --force origin main` | 0 | (permit answers nothing) | empty |
| `git --attr-source HEAD stash` | 0 | — | empty |
| `git --attr-source HEAD update-ref -d refs/heads/main` | 0 | — | empty |
| `git --attr-source HEAD config core.hooksPath /tmp/x` | 0 | — | empty |
| `git --attr-source HEAD reflog delete HEAD@{0}` | 0 | — | empty |
| `git --attr-source HEAD symbolic-ref HEAD refs/heads/x` | 0 | — | empty |
| `git --shallow-file /tmp/s push --force origin main` | 0 | — | empty |
| `git --shallow-file /tmp/s stash` | 0 | — | empty |
| `git --attr-source HEAD -c core.hooksPath=/dev/null push --force origin main` | 0 | — | empty |
| **CONTROL** `git --attr-source=HEAD push --force origin main` | 2 | `force_push_blocked` | empty |

All nine audit-6 rows reproduced at the recorded verdict; **none failed to
reproduce**. The ATTACHED spelling is correctly refused today, so this is the
separate-value form specifically — and that asymmetry is the shape of the fix,
because git's own grammar makes an attached value self-contained whatever the
option is.

**The three-leg argument.** Layer 1's `Bash(git push:*)` prefix rule does not
match a line beginning `git --attr-source`; layer 2 is defeated by the verb-index
defect itself; layer 3 is disarmed on the same line, because a command-line `-c`
outranks the envelope's env-injected `core.hooksPath` (D-09). **Four rows have no
second carrier at all** — `stash`, `update-ref -d`, `reflog delete` and
`config core.hooksPath`, where disarming the hook IS the loss of the carrier.
Audit 6 confirmed the class by performing a real destructive force push that
rewrote a bare remote's `main`.

**The post-consumption argv each derivation names was itself measured**, not
described — `git push --force origin main`, `git stash`,
`git update-ref -d refs/heads/main`, `git reflog delete HEAD@{0}` and
`git symbolic-ref HEAD refs/heads/x` all at exit 2 `force_push_blocked`;
`git config core.hooksPath /tmp/x` and
`git -c core.hooksPath=/dev/null push --force origin main` at exit 2
`hook_bypass_blocked`.

### The two-sided real-git probe, in full

`git <opt> version` (1W) versus `git <opt> XVALUE version` (2W) on
**git version 2.43.0**:

| Option | 1W | 2W | Classification |
|---|---|---|---|
| `-c` | usage | `git version 2.43.0` | consumes a separate word |
| `--git-dir` | usage | `git version 2.43.0` | consumes a separate word |
| `--work-tree` | usage | `git version 2.43.0` | consumes a separate word |
| `--namespace` | usage | `git version 2.43.0` | consumes a separate word |
| **`--attr-source`** | usage | `git version 2.43.0` | **consumes a separate word — ABSENT from `GIT_GLOBAL_VALUE_OPTS`** |
| **`--shallow-file`** | usage | `git version 2.43.0` | **consumes a separate word — ABSENT from `GIT_GLOBAL_VALUE_OPTS`** |
| `-C` | `fatal: cannot change to 'version'` | `git version 2.43.0` | consumes a separate word (variant probe: a valid directory is needed; the 1W error IS the proof the word was consumed) |
| `--config-env` | `fatal: invalid config format: version` | `fatal: invalid config format: XVALUE` | consumes a separate word (variant probe: needs a `KEY=ENVVAR` pair; the error names the word it swallowed) |
| `--no-pager` | `git version 2.43.0` | `git: 'XVALUE' is not a git command.` | self-contained |
| `-p` / `--paginate` / `-P` | `git version 2.43.0` | not-a-command | self-contained |
| `--bare` | `git version 2.43.0` | not-a-command | self-contained |
| `--no-replace-objects` | `git version 2.43.0` | not-a-command | self-contained |
| `--literal-pathspecs` | `git version 2.43.0` | not-a-command | self-contained |
| `--glob-pathspecs` / `--noglob-pathspecs` / `--icase-pathspecs` | `git version 2.43.0` | not-a-command | self-contained |
| `--no-optional-locks` | `git version 2.43.0` | not-a-command | self-contained |
| `--exec-path` | `/usr/lib/git-core` | `/usr/lib/git-core` | terminating (identical first lines) |
| `--html-path` | `/usr/share/doc/git/html` | identical | terminating |
| `--man-path` | `/usr/share/man` | identical | terminating |
| `--info-path` | `/usr/share/info` | identical | terminating |
| `--version` | `git version 2.43.0` | identical | terminating |
| **`--super-prefix`** | **`unknown option: --super-prefix`** | **same** | **NOT ACCEPTED — yet CARRIED by `GIT_GLOBAL_VALUE_OPTS`** |
| **`--no-lazy-fetch`** | **`unknown option: --no-lazy-fetch`** | **same** | **NOT ACCEPTED by this git (a real option in later releases)** |
| **`--no-advice`** | **`unknown option: --no-advice`** | **same** | **NOT ACCEPTED by this git (a real option in later releases)** |
| `--bogus-opt` | `unknown option: --bogus-opt` | same | not accepted (negative control) |
| `--help` | — | `No manual entry for gitXVALUE` | **UNPROBED** — no probe of this shape can classify it |
| `-h` | — | `No manual entry for gitXVALUE` | **UNPROBED** — same reason |

**The rejections are the point.** The list is wrong in BOTH directions against the
installed git, which is the evidence the LIST is the defect rather than a missing
row. Additional measured facts:

- **attached forms**: `--attr-source=HEAD`, `--git-dir=/tmp/g`, `--namespace=n`
  and `--work-tree=/tmp/w` all reach the verb, but
  **`git --shallow-file=/tmp/s version` answers `unknown option: --shallow-file=/tmp/s`**,
  and `--no-pager=1` / `--bare=1` are rejected outright. An attached spelling is
  therefore always *self-contained* and **not always *accepted***.
- **no short-option bundling**: `-pc user.name=x`, `-C/tmp`, `-pP` and a bare `-`
  all print `unknown option:`. So the over-consuming direction is not a live
  bypass on this git — it is a mis-index of a command that does not run.

### `T-19-102` — the same defect in the over-refusal direction

Driven with a fixture repository on `refs/heads/gsd-auto/alpha/w` with a local
bare upstream as the guard's project root:

| Command | exit | reason id |
|---|---|---|
| `git push --recurse-submodules on-demand origin refs/heads/gsd-auto/alpha/w` | **2** | `push_outside_namespace` — *"the refspec `origin` resolves to `refs/heads/origin`, which is outside `refs/heads/gsd-auto/alpha/`"* |
| `git push origin refs/heads/gsd-auto/alpha/w` (twin) | 0 | — |
| `git push --signed no origin refs/heads/gsd-auto/alpha/w` | **2** | `push_outside_namespace` — **and this one is CORRECT** |
| `git push -o ci.skip origin refs/heads/gsd-auto/alpha/w` | 0 | — |
| `git push --push-option ci.skip origin refs/heads/gsd-auto/alpha/w` | 0 | — |
| `git push --recurse-submodules=on-demand origin refs/heads/gsd-auto/alpha/w` | 0 | — |

**Fixture confirmation against real git.** `git push --recurse-submodules on-demand
origin refs/heads/gsd-auto/alpha/w` runs to completion — `Everything up-to-date` —
so the refusal is of an ordinary in-namespace push. But
`git push --dry-run --signed no origin refs/heads/gsd-auto/alpha/w` answers
`error: src refspec origin does not match any` / `error: failed to push some refs
to 'no'`: real git reads `no` as the REPOSITORY and `origin` as a refspec.

`git push -h` spells the two as `--recurse-submodules (check|on-demand|only|no)`
and `--signed[=(yes|no|if-asked)]` — a required value and an attached-only
optional one. **A fix that completed `PUSH_VALUE_OPTS` from that help text would
add `signed` and introduce a real mis-parse.** Both rows are in the corpus; the
false refusal is asserted at its POST-fix exit 0 and the `--signed` row is pinned
REFUSED.

Also confirmed in the fixture: `git --attr-source HEAD status` prints
`On branch gsd-auto/alpha/w` and `git --shallow-file /tmp/s log --oneline` prints
`7347130 init`, so the value really is consumed and the verb really is reached.

### The cells found while PLANNING round 7

Same provenance caveat `19-14`, `19-16` and `19-18` established: **found while
planning, not by an audit**. All measured at `c21c13f`, all inside `T-19-100`'s
class rather than new threat IDs.

| Cell | Line | exit | What it adds |
|---|---|---|---|
| the stale entry, over-consuming | `git --super-prefix push --force origin main` | **0** | the list wrong in the BYPASS direction: the guard swallows the real verb `push` as the option's value and reads `origin` as the verb. **Real git: `unknown option: --super-prefix` — a MIS-INDEX OF A COMMAND THAT DOES NOT RUN, not a live bypass.** Twin `git --super-prefix x push --force origin main` → exit 2 `force_push_blocked` |
| the unknown option after a known one | `git -c a=b --attr-source HEAD push --force origin main` | **0** | the scan is a LOOP, so the gap is not confined to the first slot |
| deletion then callee grammar | `git >/dev/null --attr-source HEAD push --force origin main` | **0** | round 6's model deletes the redirection and hands this gap a clean argv |
| callee grammar then deletion | `git --attr-source HEAD >/dev/null push --force origin main` | **0** | the other order, same class |
| a continuation INSIDE the option name | ``git --attr-so`\`+NL+`urce HEAD push --force origin main`` | **0** | **the pin that proves round 6 is load-bearing here**: the continuation must be deleted before `--attr-source` is even spelled |
| the nested payload | `bash -lc "git --attr-source HEAD push --force origin main"` | **0** | reaches the same place through `NestedPayload`'s re-split |
| behind a sequence | `echo hi && git --attr-source HEAD push --force origin main` | **0** | past the separator arm |
| the short bundle | `git -pc user.name=x status` | **0** | verb read as `user.name=x`. **Real git: `unknown option: -pc` — LABELLED a mis-index, not a bypass** |
| the bare dash | `git - push --force origin main` | **0** | `scan_leading` breaks on `-` before any option check. **Real git: `unknown option: -`.** A control the rule must NOT widen into: pinned PERMITTED before AND after |
| the attached short `-C` | `git -C/tmp push --force origin main` | 2 `force_push_blocked` | already refused; real git rejects `-C/tmp` too, so there is no attached short form to model |
| `--` end of options | `git -- push --force origin main` | 2 `force_push_blocked` | already correct, pinned as a control |
| the hooks key ahead of the gap | `git -c core.hooksPath=/dev/null --attr-source HEAD push` | 2 `hook_bypass_blocked` | the `-c` key check fires before the verb is reached |

### The three rows RECORDED rather than asserted

All three at exit 2 `envelope_assertion_failed` at `c21c13f`:

| Command | Current reason (verbatim fragment) |
|---|---|
| `git --attr-source $T push --force origin main` | *"`$T` is the git verb for this command, and the shell may rewrite it before the program sees it…"* |
| `git --attr-source *.x push --force origin main` | *"`*.x` is the git verb for this command…"* |
| `git --attr-source HEAD {push,--force} origin main` | *"a brace expansion splices words back into this command after the guard has answered…"* |

**Why the post-fix verdict is not derivable.** After `19-21`, `--attr-source`
consumes its value, so `$T` and `*.x` are an option's OPERAND rather than a
decision word, and `{push,--force}` sits after a consumed value rather than in the
verb slot. Whether round 5's literalness bit still reaches those positions — and
therefore whether the identifier stays `envelope_assertion_failed` or becomes
`force_push_blocked` — is a `19-21` design outcome this plan cannot know.
Asserting merely that they are REFUSED is also forbidden: that would pin an exit
code whose mechanism this plan cannot name. `19-21` measures each and appends the
pin.

### The design question, answered, with every rejected option costed

| # | Option | Verdict |
|---|---|---|
| i | **Complete the list** | **REJECTED.** It closes today's two cells and is wrong again at the next git release, and the list is *already* wrong in both directions against the installed git — `--attr-source`/`--shallow-file` missing, `--super-prefix` present and rejected. Completing an enumeration does not change the failure DIRECTION, which is what the defect is |
| ii | **Derive the grammar from git at guard time** | **REJECTED on two independent grounds.** (a) The guard runs synchronously on the agent's `PreToolUse` critical path, and `push_needs_resolved_dests` exists precisely because a reproduced 180–240 second hang made shelling out per call unacceptable. (b) A guard that asks the program it is guarding to describe its own grammar can be lied to by a `git` earlier on `PATH` — the same attacker surface this phase's own shims demonstrate. There is also no machine-readable enumeration to read: git's global options are documented in prose, and `--list-cmds=` lists commands, not options |
| iii | **Derive it at build or envelope-construction time** | **REJECTED.** The probing binary is not the guarded binary; the result is non-hermetic; and a probe that fails would have to fail closed, which is a guard nobody can build |
| iv | **ADOPTED — invert the failure direction and pin the list against real git in a TEST** | The knowledge required is exactly **one bit per option**: does it consume the next word. The fix is to make the ABSENCE of that bit a **refusal** instead of a guess — the same fail-closed treatment `resolve_program`'s wrapper axis already has. The structural rules that need no knowledge (an attached `=` value, `--`, a non-`-` token) are applied FIRST, so the list has less to know. **The better source of truth is the installed git binary itself, consulted at TEST time rather than at guard time**, which catches drift in both directions and already catches `--super-prefix` today |

### The measured over-refusal cost, from both sides

**On the installed git 2.43.0 the cost is ZERO.** Every option this git accepts is
classified by the probe and enumerated; the only rows moving permitted → refused
are ones git ITSELF rejects (`--bogus-opt`, `--super-prefix`, `-pc`, `-C/tmp`), so
they are refusals of commands that already do nothing.

**On a FUTURE git the cost is one refusal per newly added global option until the
constant learns it.** `--no-advice` and `--no-lazy-fetch` are the measured
stand-ins: real global options in later releases, rejected by this git, and both
in the corpus so the future cost is checkable rather than argued.

**How a user recovers, in the order the refusal message should offer it:**

1. **Spell the option's value ATTACHED (`--option=value`) where git accepts that
   form** — this needs no list change at all, because git's own grammar makes an
   attached value self-contained. **But its limit is measured rather than assumed:
   `git --shallow-file=/tmp/s version` answers `unknown option:
   --shallow-file=/tmp/s` on this git, while `--attr-source=`, `--git-dir=`,
   `--namespace=` and `--work-tree=` all reach the verb.** The safety claim is
   unaffected — an attached value can never consume a following word — but the
   recovery step is not universally available, so it is offered FIRST and **not
   offered alone**.
2. Drop the option.
3. Add it to the constant, which the drift pin will name.

The refusal must name the **OPTION TOKEN**, on the same footing as the existing
`git -c {assignment}` refusals, so the message is actionable without quoting the
command back (SAFE-04).

### Why the `scan_leading` / `push_operands` asymmetry is deliberate

One gets the fail-closed treatment and the other does not, and the reason is the
failure DIRECTION, not inconsistency:

- **`scan_leading`'s** unknown-option failure is a **BYPASS** — the value becomes
  the verb, and the verb is what the denylist reads.
- **`push_operands`'** is an **OVER-REFUSAL** — an unconsumed value becomes an
  extra refspec, which refuses. A fail-closed `push_operands` would refuse
  `git push --dry-run origin <ref>` and every other one-word push flag: a control
  that fails into unusability and gets switched off (AR-19-11).

**But `PUSH_VALUE_OPTS` is fail-open in the OVER-consuming direction exactly as
`GIT_GLOBAL_VALUE_OPTS` is** — an entry git does not treat as value-taking makes
the guard skip a word git reads as a refspec — so it gets the same real-git drift
pin *without* the fail-closed default.

**Correction to a claim it would be easy to generalise wrongly:** "staleness costs
refusals rather than bypasses" is true for the constants' **silence** and **false
for their entries**. A self-contained entry the runtime git treats as value-taking,
or a value-taking entry the runtime git rejects, SHIFTS THE VERB — the
`--super-prefix` direction. The fail-closed default covers absence; both
constants' *contents* can still mis-index, which is why the drift pin must probe
**both directions**.

### The anti-vacuity control, re-expressed

`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
carried a 25% ratio floor as its fourth positive control. Re-derived with the
same stripper at `ec4c700`, **measuring in BYTES because Rust's `len()` is a byte
length and `policy.rs` carries multi-byte characters in its prose**:

| File | raw bytes | stripped bytes | ratio | headroom |
|---|---|---|---|---|
| `src/envelope/policy.rs` | 263,360 | 65,947 | **25.0406%** | **107 stripped bytes ≈ 428 comment bytes** |
| `src/envelope/hooks.rs` | 99,909 | 33,460 | 33.4905% | ample |

A `str`-CHARACTER measurement of `policy.rs` reads 262,748 — **612 bytes light**,
overstating the headroom by more than a factor of two.

**428 bytes of comment is less than `19-21`'s own doc additions.** The control had
stopped measuring what it names: its red would say *"the stripper broke"* while
meaning *"someone wrote comments"*, pulling directly against
`resolve_programs_own_doc_still_discloses_the_residual_it_does_not_cover`, which
REQUIRES specific paragraphs to exist. The ratio assertion is **DELETED and
replaced** — not supplemented, because keeping both would reproduce the collision
the change exists to remove, and not lowered, because a lower percentage is the
same control with a bigger budget:

```
POLICY_MIN_PRODUCTION_BYTES = 40_000   // 61% of the 65,947 BYTES measured at ec4c700
HOOKS_MIN_PRODUCTION_BYTES  = 20_000   // 60% of the 33,460 BYTES measured at ec4c700
```

Ordinary refactoring that deletes a third of either file still passes; a stripper
that ate the logic drops to near zero and turns red. The floor is deliberately
INDEPENDENT of comment volume, and its failure message forbids lowering it.

### The `glab --host` forge cell — recorded, NOT fixed, callee UNCONFIRMED

Measured at `1583d20`: `glab --host gitlab.com mr create --title x` → exit 0 with
**ZERO** ledger lines, while `glab --hostname gitlab.com mr create --title x` →
exit 0 with **one**. `FORGE_VALUE_OPTS` is `["-R", "--repo", "--hostname"]` — the
same hand-maintained enumeration of a callee's option grammar, in a third
component, failing in the UNDER-COUNTING direction.

**`glab` is not installed on this machine**, so whether glab accepts `--host` as a
separate-value global flag is **not confirmed against the callee**, and this is
**NOT claimed as a live bypass**. It is a planning-time cell needing callee
confirmation. `gh` was swept and is clean: `gh --repo o/r`, `gh -R o/r`,
`gh --hostname h.example`, `gh api --hostname h.example` and `gh --version` all
leave exactly one ledger line. Recorded in `deferred-items.md` as a candidate for
the round after this one. **Out of this round's three-item scope; deliberately not
fixed**, and `FORGE_VALUE_OPTS`, `GH_API_VALUE_OPTS` and `subcommand_word_indices`
are untouched.

### `T-19-17r` — the bookkeeping gap, OUTSTANDING, acceptance deliberately UNMADE

`19-17-SUMMARY.md` calls `T-19-17r` "accepted". Audits 5 and 6 both confirmed the
measurement and both pins (`rg "git status" {src,tests}` → exit 2,
`rg "git status" src/` → exit 0, pinned at
`tests/envelope_literal_decision.rs:1355-1379`), and **both deliberately declined
to make the acceptance, because accepting a risk is a human decision.** There is
still **no Accepted-Risks-Log row and no register row** for it — the
Accepted-Risks-Log row count for that risk ID is **zero**, and this plan did not
change it.

**This plan records the gap and makes no acceptance.** The next round either adds
the log row or drops the word; this plan does neither.

### The `envelope_tracer` provenance correction

`19-19-SUMMARY.md` presents the `a_relocated_copy_of_the_stub_refuses_instead_of_acting`
ETXTBSY / `ExecutableFileBusy` failure as newly observed. `deferred-items.md` has
carried it **since 19-07** under "`tests/envelope_tracer.rs` — ETXTBSY when a
just-copied stub is exec'd", with the same symptom string, the same
write-then-exec diagnosis and the same "once in ~6 full-suite runs" frequency.
It is a **provenance slip, not a defect**: audit 6 ran the test 8/8 green in
isolation, and it fails closed by ERRORING, so it cannot mask a regression.
`19-19-SUMMARY.md` is not edited; the correction is made once, here.

### What this plan produced

Two commits under `tests/` only, **zero `src/` hunks in each**:

1. `tests/envelope_callee_grammar.rs` — the SIXTH evidence file, 16 new `#[test]`
   fns, 5 RED.
2. `tests/envelope_wrapper_class.rs` — `CALLEE_GRAMMAR_CLASSES` as a THIRD named
   axis beside a byte-identical `UNREADABLE_CLASSES` and a byte-identical
   `DELETION_CLASSES`, plus the anti-vacuity re-expression. 5 new `#[test]` fns,
   2 RED.

Gate: `passed + failed` = **1631**, against a baseline of 1610 and exactly 21 new
`#[test]` fns — the identity holds. All thirteen `envelope_*` binaries ran.

---

## Execution record — plan 19-21 (the rule). NOT an audit finding.

**Written by plan 19-21's executor, appended after the plan-19-20 record and
touching nothing above this line.** No audit table, no Security Audit Trail
entry, no Accepted Risks Log row, no Sign-Off and no earlier appended subsection
was edited. Re-measuring and re-classifying these rows is `/gsd-secure-phase
19`'s job, not this record's.

### FIRST: `/gsd-secure-phase 19` is NOT cleared by this plan

- **`T-19-86` remains OPEN at `high`**, by explicit user scoping decision. Its
  four rows still exit 0 and its pin is green and unmodified.
- **`T-19-91` remains OPEN at `high`**, arms unweakened: `git reflog $S`,
  `git reflog show $S` and `git symbolic-ref $S` at exit 0 with **no second
  carrier**, and bare `git push $REF` at exit 0 in the in-namespace
  configuration. No decision-operand rule was added for `reflog`,
  `symbolic-ref` or `push`, and the denylist was not extended.
- **Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** `T-19-86` and
  `T-19-91` are both sub-classes of it and both remain open at `high`.
- `T-19-96` stays registered and unfixed. `T-19-74`'s core rows stay frozen.
  `T-19-61` … `T-19-73`, `T-19-84` and `T-19-85` are untouched.

### THE BLOCKER THIS ROUND FOUND AND DID NOT WORK AROUND

**`19-20`'s corpus contains two rows that no rule obeying this plan's
prohibitions can satisfy at once, and the executor reported it rather than
editing an assertion.**

| | Row | Pinned reason | Where |
|---|---|---|---|
| A | `git --super-prefix x status` | `envelope_assertion_failed` | `after_19_21_the_unknown_option_cost_rows_are_refused_beside_their_permitted_twins` (RED, must go green) |
| B | `git --super-prefix x push --force origin main` | `force_push_blocked` | `the_already_correct_planning_cells_keep_their_verdicts_as_controls` (green at `097dba2`, labelled "green today and after") |

**The proof that they are incompatible.** The two commands carry IDENTICAL
leading tokens — `--super-prefix x` — and `scan_leading` is a pure function of
argv that sees them identically up to the verb. Row A requires `scan_leading`
to return `Some(refusal)` for those tokens. `classify_git` returns that refusal
immediately, so row B is then `envelope_assertion_failed` too. For row B to be
`force_push_blocked` the CLASSIFIER's verdict must take precedence over
`scan_leading`'s refusal — which is a filter in `classify_git` after
`scan_leading` returned, a second reading site, and explicitly prohibited by
this plan.

**Row B's own comment shows it is a PRE-fix observation mis-labelled.** It reads
"the two-sided twin of the stale-entry cell, which is what proves the entry is
what moves the verb index" — it is `force_push_blocked` today precisely BECAUSE
`--super-prefix` is in `GIT_GLOBAL_VALUE_OPTS` and swallows `x`. Removing that
stale entry, which this round's own evidence requires, necessarily moves it.

**Safety is unaffected: both rows are REFUSED under both rules at exit 2, with
an empty walk.** Only the reason identifier differs, and
`envelope_assertion_failed` is the more honest of the two — naming
`force_push_blocked` for a command whose verb the guard admits it cannot
establish would attribute the refusal to a mechanism that did not produce it.
The row is left RED. Resolving it is a decision about `19-20`'s assertion, which
this plan may not make.

The sibling row `git -C/tmp push --force origin main`, in the same test, was the
same shape and WAS resolvable — see the deviation below.

### The rule as implemented

**An unestablished verb slot is a refusal, not the next non-`-` word.**
`leading_git_option` answered a word count, and for a spelling it did not
recognise the answer was one — so `scan_leading` advanced a single word, landed
on the option's VALUE, saw it did not begin with `-`, and broke with that value
as the verb; `classify_git` found it in no denylist arm and answered `Allow`.
The answer is now three-valued — self-contained, consumes a separate word, or
**grammar not established** — and the third produces a refusal at
`ParkReason::EnvelopeAssertionFailed` **inside the one scan**, returned through
the `(usize, Option<GitVerdict>)` channel that already carried the
unreadable-key refusal. That is the same fail-closed treatment
`resolve_program`'s wrapper axis has had since audit 3, applied to the one arm
of the same question that was never given it.

Mechanically checked: `git diff --stat` over `src/` for this plan is **exactly
one file**; no new `ParkReason` variant; `hooks.rs` not opened;
`first_unreadable_decision_word`, `config_key_operand_index`,
`subcommand_word_indices`, `scan_gh_api` and `resolve_program_with_head` have no
hunk; the post-filter gains no arm.

### The design question, answered — four options, three rejected on measured grounds

| # | Option | Verdict and cost |
|---|---|---|
| i | Complete `GIT_GLOBAL_VALUE_OPTS` | **REJECTED.** It closes today's two cells and is wrong again at the next git release. The list was already wrong in BOTH directions against the installed git — `--attr-source` and `--shallow-file` absent, `--super-prefix` present and answering `unknown option:`. Completing an enumeration does not change the failure DIRECTION, which is the defect |
| ii | Ask git for its grammar at GUARD time | **REJECTED on two independent grounds.** The guard runs synchronously on the agent's `PreToolUse` critical path and `push_needs_resolved_dests` exists precisely because a reproduced 180–240 second hang made per-call shelling out unacceptable; and a guard that asks the program it is guarding to describe its own grammar can be lied to by a `git` earlier on `PATH` — the same surface this phase's own argv-printing shims demonstrate. There is also nothing machine-readable to read: git's global options live in prose and `--list-cmds=` lists commands, not options |
| iii | Derive it at build or envelope-construction time | **REJECTED.** The probing binary is not the guarded binary, the result is non-hermetic, and a probe that failed would have to fail closed — a guard nobody can build |
| iv | **ADOPTED** — invert the failure direction in the guard, pin the constants against real git in a TEST | **The knowledge required is exactly ONE BIT per option: does it consume the next word.** Three structural rules supply it with no knowledge of git at all and run FIRST — a `--`-prefixed token containing `=` is self-contained whatever the option is, `--` ends the options, and a non-`-` token ends the scan — so only the remaining spellings need a constant, and the ABSENCE of the bit becomes a refusal |

### The two constants, and the removal that is part of the fix

`GIT_GLOBAL_VALUE_OPTS` **gained** `--attr-source` and `--shallow-file` and
**lost** `--super-prefix`. `GIT_GLOBAL_SELF_CONTAINED_OPTS` is entirely new
knowledge: before the inversion, silence meant "one word", so there was nothing
to enumerate. Every member of both came from `19-20`'s recorded two-sided probe,
re-run here against `git version 2.43.0`, not from a plan's text.

**`--super-prefix`'s removal is as much of the fix as the two additions.** A
stale entry is fail-open in the OVER-consuming direction, which is a **bypass**
and not an over-refusal: measured, `git --super-prefix push --force origin main`
exited **0**, because the scan swallowed the real verb `push` as the option's
value and read `origin` as the verb. It is inert only because git itself rejects
the option — **a stale entry for an option git ACCEPTS would be a live bypass.**

Two categories were considered and rejected in the constants' own docs rather
than silently not taken: a third `Terminates` answer, which would turn
`git --exec-path push --force origin main` into a permit (a control that ADDS a
permit needs stronger evidence than one that preserves a refusal); and a
`--no-*` convention rule, which would absorb `--no-advice` and `--no-lazy-fetch`
silently instead of surfacing them as the one refusal each that tells a
maintainer the constant needs a row.

### The drift pin, with its own limit stated correctly

Four new `#[test]` fns in `policy.rs`'s own `#[cfg(test)] mod tests` run the
two-sided probe against the installed `git` over **every entry of both
constants and of `PUSH_VALUE_OPTS`**. It exists because `GIT_GLOBAL_VALUE_OPTS`
had **no pin and no test reference anywhere** — audit 6 measured exactly two
mentions of it in the whole repository — which is how it came to be wrong in
both directions. It **catches `--super-prefix` today**: the entry was verified
red before removal by re-adding a rejected option (`--no-advice`) to the
constant and observing the pin fail.

- Negative controls in both arms: `--bogus-opt` must classify as NOT ACCEPTED,
  and a value-taking spelling must FAIL the self-contained probe, so a probe
  answering the same for everything turns red.
- The two constants are asserted **disjoint**.
- The **UNPROBED** set is bounded at two and named: `--help` and `-h`, because
  `git --help XVALUE version` answers `No manual entry for gitXVALUE`, which
  neither reaches a verb nor names the following word as a value. They are in
  neither constant and take the fail-closed path.
- Floors with their arithmetic beside them: ≥ 6 value-taking (the six spellings
  the corpus's class-1 alphabet splices by name), ≥ 8 self-contained (the seven
  pinned by name elsewhere, plus one). Each failure message says the correct
  response is to RESTORE entries, never to lower the floor.
- It does **NOT** skip when git is absent; it panics. A skipped pin is a
  fail-open pin.

**Its limit, stated correctly rather than comfortably.** It pins the constants
against the **DEVELOPER's** git, not the runtime git, and **the fail-closed
default covers only the SILENT case.** A MISSING bit costs a refusal. A **WRONG**
bit costs a **shifted verb** — a self-contained entry a runtime git treats as
value-taking, or a value-taking entry it rejects, makes `scan_leading` step over
or land short of the real verb, which is exactly the `--super-prefix` mechanism
this round measured at exit 0. **So this pin is the only control over the
wrong-bit direction, and a constant that outruns the runtime git is a bypass
rather than an over-refusal.**

### The over-refusal cost, measured from both sides

**ZERO on git 2.43.0.** Every option this git accepts is classified by the probe
and enumerated, so the only rows moving permitted → refused are ones git ITSELF
rejects — refusals of commands that already do nothing.

**One refusal per newly added global option on a FUTURE git**, until the
constant learns it. `--no-advice` and `--no-lazy-fetch` are the measured
stand-ins: real global options in later releases, rejected by this one, both in
the corpus so the future cost is checkable rather than argued.

**The three-step recovery, in the order the refusal message offers it:** spell
the option with its value attached (`--option=value`), which needs no constant
change because git's own grammar makes an attached value self-contained —
**offered first and never alone, because an attached spelling is always
self-contained but is NOT always accepted**: `git --shallow-file=/tmp/s version`
answers `unknown option: --shallow-file=/tmp/s` on this git, while
`--attr-source=`, `--git-dir=`, `--namespace=` and `--work-tree=` all reach the
verb; or drop the option; or add the spelling to the constant, which the drift
pin will name. The message names the OPTION TOKEN and never quotes the command
back (SAFE-04).

### What the rule newly REFUSES, each beside its permitted twin

| Command | before | after | twin, still exit 0 |
|---|---|---|---|
| `git --bogus-opt status` | 0 | 2 `envelope_assertion_failed` | `git status` |
| `git --no-advice status` | 0 | 2 `envelope_assertion_failed` | `git status` |
| `git --no-lazy-fetch status` | 0 | 2 `envelope_assertion_failed` | `git log --oneline` |
| `git --super-prefix x status` | 0 | 2 `envelope_assertion_failed` | `git status` |
| `git -pc user.name=x status` | 0 | 2 `envelope_assertion_failed` | `git -c user.name="$NAME" commit -m x` |

### What the rule newly REFUSES that was a live bypass

| Command | before | after |
|---|---|---|
| `git --attr-source HEAD push --force origin main` | 0 | 2 `force_push_blocked` |
| `git --attr-source HEAD stash` | 0 | 2 `force_push_blocked` |
| `git --attr-source HEAD update-ref -d refs/heads/main` | 0 | 2 `force_push_blocked` |
| `git --attr-source HEAD config core.hooksPath /tmp/x` | 0 | 2 `hook_bypass_blocked` |
| `git --attr-source HEAD reflog delete HEAD@{0}` | 0 | 2 `force_push_blocked` |
| `git --attr-source HEAD symbolic-ref HEAD refs/heads/x` | 0 | 2 `force_push_blocked` |
| `git --shallow-file /tmp/s push --force origin main` | 0 | 2 `force_push_blocked` |
| `git --shallow-file /tmp/s stash` | 0 | 2 `force_push_blocked` |
| `git --attr-source HEAD -c core.hooksPath=/dev/null push --force origin main` | 0 | 2 `hook_bypass_blocked` |
| `git --super-prefix push --force origin main` | 0 | 2 `envelope_assertion_failed` |
| `git -c a=b --attr-source HEAD push --force origin main` | 0 | 2 `force_push_blocked` |
| `git >/dev/null --attr-source HEAD push --force origin main` | 0 | 2 `force_push_blocked` |
| `git --attr-source HEAD >/dev/null push --force origin main` | 0 | 2 `force_push_blocked` |
| `git --attr-so`+`\`+NL+`urce HEAD push --force origin main` | 0 | 2 `force_push_blocked` |
| `bash -lc "git --attr-source HEAD push --force origin main"` | 0 | 2 `force_push_blocked` |
| `echo hi && git --attr-source HEAD push --force origin main` | 0 | 2 `force_push_blocked` |

Four of these have **no second carrier at all** — `stash`, `update-ref -d`,
`reflog delete` and `config core.hooksPath`, where disarming the hook IS the
loss of the carrier.

### What the rule newly PERMITS, with its control beside it

| Command | before | after |
|---|---|---|
| `git push --recurse-submodules on-demand origin refs/heads/gsd-auto/alpha/w` | 2 `push_outside_namespace` | **0** |
| `git push --signed no origin refs/heads/gsd-auto/alpha/w` (**CONTROL**) | 2 `push_outside_namespace` | 2 `push_outside_namespace` |

`PUSH_VALUE_OPTS` gained `recurse-submodules` and **nothing else**. `git push -h`
spells `--recurse-submodules (check|on-demand|no)` and
`--signed[=(yes|no|if-asked)]` — a REQUIRED value and an ATTACHED-ONLY optional
one — and the two look alike while git treats them differently: `no` becomes the
repository. The `--signed` control stays REFUSED, so a list completed from the
help text lands red. `push_operands` deliberately does NOT get the fail-closed
default (its unknown-flag direction is an over-refusal, not a bypass, and a
fail-closed version would refuse `git push --dry-run origin <ref>` — AR-19-11);
its OVER-consuming direction is covered by the drift pin, whose negative
controls are `--signed` and `--dry-run`.

### The three rows `19-20` could not derive — MEASURED and pinned with their clauses

| Command | measured after | the clause that produced it |
|---|---|---|
| `git --attr-source $T push --force origin main` | 2 `force_push_blocked` | `--attr-source` now consumes `$T`, so `$T` is an option OPERAND outside the decision region — as `$MSG` is in `git commit -m "$MSG"`. Round 5's rule no longer reaches it; `classify_push`'s denied-flag arm answers. **The identifier MOVED while the exit code did not.** |
| `git --attr-source *.x push --force origin main` | 2 `force_push_blocked` | the same clause, through the glob half of round 5's rule rather than the parameter-expansion half |
| `git --attr-source HEAD {push,--force} origin main` | 2 `envelope_assertion_failed` | a DIFFERENT rule: `19-17`'s WHOLE-COMMAND brace rule in `resolve_program_with_head`, which is not a decision-word rule and does not care which slot the braces occupy. **The one of the three whose identifier did not move**, and the row that shows this round did not pay for its rule by narrowing an earlier one's |

**None became PERMITTED**, so the finding those rows were watched for — a value
slot that stops being guarded, over-consumption in the `--super-prefix`
direction — did not arise.

### The controls that show this is a grammar MODEL, not a blanket refusal

- **Twelve ordinary invocations still at exit 0**: `git --no-pager status`,
  `git --no-pager log --oneline`, `git -c user.name="$NAME" commit -m x`,
  `git --git-dir=/tmp/g status`, `git -C /tmp status`, `git --bare status`,
  `git --literal-pathspecs status`, `git --no-optional-locks status`,
  `git --exec-path status`, `git --version`, `git --attr-source HEAD status`,
  `git --shallow-file /tmp/s log --oneline`. **`git --no-pager status` is this
  axis's `ls {git,svn}-repo`**: a rule that refused it would be a blanket
  refusal of anything beginning with `-`, which is how a safety control gets
  switched off (AR-19-11).
- `git - push --force origin main` still at **exit 0** — the rule was not widened
  into non-`-`-prefixed words — and `git -- push --force origin main` still at
  exit 2 `force_push_blocked`.
- **The four mechanism pins, green and unmodified.** `Token.literal` is `false`
  for `pus?` and `true` for `--attr-source`, `HEAD` and `push` — round 5's bit is
  RIGHT about every word of the bypass line and was not falsified to obtain a
  refusal. `git >/dev/null push --force origin main` refused and
  `git x2>/tmp/o push --force origin main` PERMITTED — round 6's deletion model
  is non-dead. Rule B's severed-head geometry reports
  `head_is_command_position == false` for the severed spellings and `true` for
  `{ git status; }`. `is_separator(">") == false` with `SEPARATORS`
  byte-identical.
- `wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
  green under `19-20`'s absolute byte floors. **It fired once during this round**
  and correctly: a `#[cfg(test)]` constant declared beside
  `GIT_GLOBAL_VALUE_OPTS` truncated that control's view of `policy.rs` at line
  600, because its stripper treats the first `#[cfg(test)]` line as the end of
  production logic. The constant was moved into the test module and the reason
  is recorded in its doc.

### `T-19-17r` — the bookkeeping gap, still OUTSTANDING

`19-17-SUMMARY.md` calls it "accepted". Audits 5 and 6 both confirmed the
measurement and both pins and **both explicitly declined to make the acceptance,
because accepting a risk is a human decision.** This plan does not make it
either. **No Accepted-Risks-Log row was added, no register row was added, and
the word "accepted" is not applied to `T-19-17r` anywhere in this round's code,
test names, comments, SUMMARY or this subsection.** The gap is recorded as
OUTSTANDING for the third round running.

### What this record does NOT do

- It closes `T-19-100`, `T-19-102` and `T-19-101` **in this plan's own words**
  and leaves the re-measurement and re-classification to `/gsd-secure-phase 19`.
- It makes **no acceptance** of any risk.
- The **`glab --host` forge cell** stays carried forward, **unconfirmed against
  its callee** — `glab` is not installed on this machine, so it is not claimed as
  a live bypass — and unfixed. `FORGE_VALUE_OPTS`, `GH_API_VALUE_OPTS` and
  `subcommand_word_indices` were not touched.
- `T-19-86`, `T-19-91`, `T-19-96` and `T-19-74` are untouched and open, so
  **`/gsd-secure-phase 19` is not cleared.**

### What this plan produced

Three commits, `src/envelope/policy.rs` the only `src/` file in any of them:

1. `e592f38` — the inversion, the two constants, and four drift-pin `#[test]`
   fns.
2. `fe49142` — `T-19-102`'s one-entry fix and the three derived rows appended.
3. `077f5fe` — the spawn-allowlist declaration and its stricter local
   compensating control.

Gate: `passed + failed` = **1639**, against `19-20`'s **1631** and exactly **8**
new `#[test]` fns counted from `git show` — the identity holds. All thirteen
`envelope_*` binaries ran. `git diff --numstat` over `tests/` shows **zero
deletions**. `cargo clippy -- -D warnings` exits 0. Two failures: the documented
`driver_reattach` flake, and the blocker row recorded at the top of this
subsection.

---

## Execution record — the plan-19-21 blocker row, RESOLVED. NOT an audit finding.

**Attribution.** Written by a scoped follow-up executor on 2026-09-04, at the
user's direction, to resolve the ONE blocker row plan `19-21` left open. It is
**not** an audit, and it does **not** amend audit 6, the Security Audit Trail,
the Accepted Risks Log, the threat register tables or any Sign-Off block — none
of those were touched. It records one corpus-labelling correction and the
measurements that justify it. Commit: `989f21a`.

### The blocker, restated

`19-21` reported that `tests/envelope_callee_grammar.rs` carried two pins it
judged mutually incompatible:

| | Row | Pinned identifier | Test |
|---|---|---|---|
| **A** | `git --super-prefix x status` | `envelope_assertion_failed` | `after_19_21_the_unknown_option_cost_rows_are_refused_beside_their_permitted_twins` |
| **B** | `git --super-prefix x push --force origin main` | `force_push_blocked` | `the_already_correct_planning_cells_keep_their_verdicts_as_controls` |

It left row B RED and reported rather than working around it. That was the
correct call, and the reasoning was checked rather than taken on trust.

### The analysis HELD, and here is how it was verified

**Structurally.** `--super-prefix` is absent from `GIT_GLOBAL_VALUE_OPTS` and
from `GIT_GLOBAL_SELF_CONTAINED_OPTS` in `src/envelope/policy.rs` after `19-21`
(read directly), and PRESENT in `GIT_GLOBAL_VALUE_OPTS` at the pre-fix base
`097dba2` (`git show 097dba2:src/envelope/policy.rs`). `scan_leading` returns
`(index, Some(unestablished_verb_refusal(token)))` on `NotEstablished`, and
`classify_git`'s FIRST statement returns that refusal. Two different identifiers
from identical leading tokens is therefore impossible without a filter in
`classify_git` after `scan_leading` returned — **the prohibited second reading
site. It was not built.**

**By measurement, against the BUILT BINARY** (`target/debug/gsd-meta-manager
envelope guard alpha`), one fresh `GSD_MM_ENVELOPE_ROOT` per row and the
envelope root WALKED afterwards, so the empty walk is OBSERVED rather than
inferred:

| Command | exit | identifier | walk |
|---|---|---|---|
| `git --super-prefix x status` | 2 | `envelope_assertion_failed` | EMPTY |
| `git --super-prefix x push --force origin main` | 2 | `envelope_assertion_failed` | EMPTY |
| `git -C/tmp push --force origin main` (control) | 2 | `force_push_blocked` | EMPTY |
| `git -- push --force origin main` (control) | 2 | `force_push_blocked` | EMPTY |
| `git -c core.hooksPath=/dev/null --attr-source HEAD push` (control) | 2 | `hook_bypass_blocked` | EMPTY |
| `git - push --force origin main` (control) | 0 | — | EMPTY |

**Against the CALLEE.** git 2.43.0 rejects `--super-prefix` in every form the
grammar distinguishes — bare (`git --super-prefix version`), separate-value
(`git --super-prefix x version`) and attached (`git --super-prefix=x version`)
each print `unknown option:`. So the removal of the entry was required and the
row is a mis-index of a command that does not run, never a live bypass.

**The conclusion.** `force_push_blocked` was a **PRE-fix observation
mis-labelled as post-fix**. It held at `097dba2` only because the stale entry
consumed `x` and moved `push` into the verb slot — which is exactly what the
row's own comment says it proves. Removing `--super-prefix` was a deliberate,
audit-confirmed part of `19-21`'s fix (audit 6 measured
`git --super-prefix push --force origin main` at exit 0 with a stale entry
swallowing the real verb). **Both spellings refuse at exit 2 with an empty walk
under either rule; only the identifier differs, and `envelope_assertion_failed`
is the more accurate of the two** — naming `force_push_blocked` for a command
whose verb the guard admits it cannot establish attributes the refusal to a
mechanism that did not produce it (D-24).

### What was changed

**One test file. Zero `src/` bytes.** Row B's post-fix expectation was corrected
to `policy::REASON_ENVELOPE_ASSERTION_FAILED`, with a comment stating plainly
that the previous identifier was a pre-fix observation mis-labelled as post-fix,
that it moved because `--super-prefix` left the constant as part of the fix, and
that both spellings refuse with an empty walk either way. The test's section
header was corrected to say the label covers the first three cells only, and the
sibling pre-fix comment in `after_19_21_the_stale_and_bundled_planning_cells_are_refused`
gained the same caveat so the confusion is not repeated.

**Nothing was weakened.** `refuses()` still asserts exit 2, the D-24 identifier,
and an empty ledger walk over a freshly walked root. `SEPARATORS` is untouched
and `policy::is_separator(">")` is still `false`. Round 4's literalness bit,
round 5's inversion and round 6's deletion model are unmodified and their
mechanism pins are green. `POLICY_MIN_PRODUCTION_BYTES = 40_000` and
`HOOKS_MIN_PRODUCTION_BYTES = 20_000` are unchanged. `git diff --numstat` over
`tests/` for this commit is `58 3` on one file; the three deleted lines are the
two-line section header and the single identifier constant, all replaced in
place.

### The sweep for the same defect

Every identifier-asserting row in `tests/envelope_callee_grammar.rs` and
`tests/envelope_wrapper_class.rs` was checked against the constants `19-21`
changed — `--super-prefix` removed from `GIT_GLOBAL_VALUE_OPTS`;
`-c`, `--config-env`, `--attr-source`, `--shallow-file` added;
`GIT_GLOBAL_SELF_CONTAINED_OPTS` entirely new; `recurse-submodules` added to
`PUSH_VALUE_OPTS`.

**Row B was the ONLY row of the class.** Findings, reported even where nothing
needed changing:

| Row | Depends on | Verdict |
|---|---|---|
| `git -C/tmp push --force origin main` → `force_push_blocked` | `-C` in `GIT_GLOBAL_VALUE_OPTS` — **unchanged** by `19-21` (and preserved by that plan's deviation-1 short-attached arm) | correctly labelled, measured green |
| `git -- push --force origin main` → `force_push_blocked` | the `--` end-of-options termination rule, no constant | correctly labelled, measured green |
| `git -c core.hooksPath=/dev/null --attr-source HEAD push` → `hook_bypass_blocked` | the `-c` key check firing before any verb, no constant lookup | correctly labelled, measured green |
| `git --attr-source=HEAD push --force origin main` → `force_push_blocked` | the STRUCTURAL attached-`=` rule, which needs no constant entry at all | correctly labelled, green both sides |
| `git push --signed no …` → `push_outside_namespace` | `signed` staying OUT of `PUSH_VALUE_OPTS` — `19-21` added `recurse-submodules` and nothing else | correctly labelled; this is the discrimination control and it held |
| the four `PUSH_VALUE_OPTS` rows at exit 0 in `after_19_21_the_in_namespace_push_…` | `-o` / `--push-option` already present; the attached form structural | permits, no identifier to mislabel |
| `envelope_wrapper_class.rs` callee section | `an_option_the_installed_git_rejects_fails_closed_on_every_base` and the generative refused arm assert **exit 2 and an empty walk only**; the base floors assert taxonomy MEMBERSHIP (`REASON_IDENTIFIERS.contains`), never a specific identifier | no row of the class exists in that file |
| every `refuses_under(...)` call in `envelope_wrapper_class.rs` | wrapper-axis lines; none carries a leading git global option whose bit changed | no row of the class |

`grep -rln` over `tests/` for `attr-source`, `shallow-file`,
`recurse-submodules`, `config-env` and `super-prefix` returns **only those two
files**, so the sweep is complete across the suite.

### The gate

`rtk proxy cargo test --no-fail-fast`, counts read with `rtk proxy grep` over a
redirected log (a plain `grep` reads a log the RTK hook has already stripped the
`test result:` lines from, which is D-34):

| passed | failed | ignored | `passed + failed` |
|---|---|---|---|
| **1639** | **0** | 13 | **1639** |

`passed + failed` is unchanged at 1639 because turning a red test green moves no
total and no `#[test]` fn was added. **All THIRTEEN `envelope_*` binaries ran and
all thirteen are GREEN**: `advisory` 10, `argv_deletion` 20, **`callee_grammar`
19 (was 18 passed / 1 failed)**, `command_position` 18, `credential` 6,
`expansion_slots` 32, `hook_refusals` 7, `literal_decision` 43, `pr_cap` 11,
`tracer` 6, `wiring` 14, `wrapper_bypass` 13, `wrapper_class` 34 — all with 0
failures. `cargo clippy -- -D warnings` exits 0. The three documented flakes
(the two `driver_reattach` names and the `envelope_tracer` `ExecutableFileBusy`
stub-write race) did not fire and were not touched.

### What this resolution does NOT do

- **`/gsd-secure-phase 19` is NOT cleared.** `T-19-86` and `T-19-91` remain
  **OPEN at `high`**, arms unweakened and no remedy added.
- **Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** `T-19-86` and
  `T-19-91` are both sub-classes of it and both remain open.
- **`T-19-17r` stays OUTSTANDING.** No `AR-19-13` row and no Accepted-Risks-Log
  row was added; it is **not** accepted.
- `T-19-96`, `T-19-74`, `T-19-84`, `T-19-85` and `T-19-61` … `T-19-73` are
  untouched and open.
- The `glab --host` forge cell keeps its **unconfirmed-callee** caveat: `glab`
  is not installed on this machine, so it is not claimed as a live bypass.
  `FORGE_VALUE_OPTS`, `GH_API_VALUE_OPTS` and `subcommand_word_indices` were not
  touched.

---

## Threats found by audit 7 (2026-09-03, after plans 19-20 and 19-21)

**Provenance, stated first.** The three subsections above this one were written by
the EXECUTORS of plans 19-20 and 19-21 and by the scoped follow-up that resolved
`19-21`'s blocker row; they are deliberately outside the audit tables. Audit 7
left them, and every earlier appended subsection and every earlier audit's own
tables, **byte-identical** — the body below the frontmatter was checksummed
before writing (`sha256 07cf5ef3…4b1f5fe6` over the 350,244 bytes below the
frontmatter of the 350,596-byte file), and `git diff` for this audit's write
reports **zero deleted lines** outside the frontmatter block, so every line every
earlier round wrote survives unaltered. Everything from here to the end of the
file is **audit 7's own**, measured against the built binary at
`3110d4a` with a fresh `GSD_MM_ENVELOPE_ROOT` per row and the whole envelope root
walked afterwards, and with every claimed bypass confirmed against the **real
`git` binary** (`git version 2.43.0`) in a fixture with a bare upstream — this
round being entirely about the callee.

### The question this audit was set, answered plainly

**With the shell boundary modelled and the callee's LEADING-OPTION grammar now
fail-closed, is there a further layer between the guard's decision and what git
actually does?**

**Yes. There is, and it is not the option grammar.** Round 7 is genuinely
complete on the axis it set out to close — all nine `T-19-100` rows, every
composition cell and every over-refusal twin re-measured below — and audit 6's
one-sentence brief, *"six rounds have modelled the shell; the next one has to
model git"*, was answered for exactly one of the three things git does with a
command line. Git turns a command line into behaviour in **three** stages, and
this phase now models the first:

1. **which arriving word is the verb** — git's *option grammar*. Closed by
   `19-21`, fail-closed on an unestablished slot, pinned against the installed
   git in both directions.
2. **what configuration is in effect while that verb runs** — git's *config
   resolution*. **Unmodelled.** The guard checks one config KEY (`core.hooksPath`,
   at `is_hooks_path_key`, `policy.rs:750`) at one carrier (`-c` / `--config-env`
   on argv). Git's config resolution has an **indirection** — `include.path` and
   `includeIf.<condition>.path` pull an arbitrary file's contents in **at the
   precedence of the directive that named them** — so a command-line
   `-c include.path=<file>` sets `core.hooksPath` at command-line precedence
   without the string `core.hooksPath` appearing anywhere on the command line.
   That is `T-19-103`.
3. **what the process it inherits tells it** — git's *environment as config*.
   **Unmodelled at one key.** `GIT_CONFIG_PARAMETERS` is git's own internal
   carrier for `-c` (it is how `-c` reaches child git processes) and it outranks
   the `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n` triplet the envelope injects. It is
   **absent from `ENVELOPE_ENV_KEYS`**. That is `T-19-104`.

**The measurement that settles it, made against real git rather than argued:**

```
GIT_CONFIG_COUNT=1 KEY_0=core.hooksPath VALUE_0=/ENV_WINS  git config --get core.hooksPath
  → /ENV_WINS                                              (the envelope's injection, control)
… git -c include.path=<file with [core] hooksPath=…>  config --get core.hooksPath
  → /INCLUDE_WINS                                          (the injection is OUTRANKED)
… GIT_CONFIG_PARAMETERS="'core.hooksPath=…'"          git config --get core.hooksPath
  → /PARAM_WINS                                            (the injection is OUTRANKED)
```

**So the pattern held for a seventh round, and the axis moved again** — from *how
a word is written* (rounds 1–5), to *which words arrive* (round 6), to *which
arriving word is the verb* (round 7), to **what the verb runs under**. Rounds 1–6
modelled the shell. Round 7 modelled git's option grammar. **Nothing in this phase
models git's config resolution or the environment git reads as config**, and both
reach `core.hooksPath` — the single setting D-09 names as the one that outranks
the envelope, and the setting `T-19-09` (high, closed) exists to deny.

**The disclosure that is now measurably wrong, quoted rather than paraphrased.**
`cred::hooks_path_env`'s own doc (`cred.rs:241-248`) states the limit as:

> *"the one form that outranks this injection is `git -c core.hooksPath=… push` —
> which plan 19-02 denies **by name** at the tool boundary for exactly that
> reason."*

There are at least **five** forms that outrank it, four of them permitted:
`-c core.hooksPath=` (denied), `-c include.path=`, `-c includeIf.<cond>.path=`,
`--config-env=include.path=<VAR>` and `GIT_CONFIG_PARAMETERS`. Same shape as
`T-19-84`: a residual disclosure pinned to exist rather than to be true.

### The new rows

| Threat ID | Category | Component | Severity | Disposition | Finding | Status |
|---|---|---|---|---|---|---|
| **T-19-103** | **Elevation of Privilege** | **git's own CONFIG RESOLUTION — `scan_leading`'s single-key check `is_hooks_path_key` (`policy.rs:750`), reached through `include.path` / `includeIf.<cond>.path`** | **high** | **mitigate (unimplemented)** | **`scan_leading` decides one question about a `-c` assignment: is the KEY half `core.hooksPath` (D-09). Git's config resolution has an INDIRECTION the check does not model: a config key `include.path` names a file whose contents are spliced in **at the precedence of the directive that named it**, so a command-line `-c include.path=<file>` sets `core.hooksPath` at command-line precedence while the string `core.hooksPath` never appears on the command line. Measured at `3110d4a`, one fresh root per row, walk EMPTY on every one: `git -c include.path=/tmp/evil.cfg commit -m x` → **exit 0**; `git -c include.path=/tmp/evil.cfg push origin refs/heads/gsd-auto/alpha/w` → **0**; `git -c include.path=… status` → **0**; `git -c includeIf.gitdir:/tmp/.path=/tmp/evil.cfg status` → **0**; `git --config-env=include.path=EVILVAR status` → **0**; `git --config-env include.path=EVILVAR status` → **0**; `git -c INCLUDE.PATH=/tmp/evil.cfg push …` → **0** (git resolves the key case-insensitively, exactly as `T-19-09`'s own `CORE.HOOKSPATH` row records for the direct spelling). The paired discriminators still refuse, so the mechanism is present and this is a gap in its reach, not its absence: `git -c core.hooksPath=/dev/null commit -m x` → 2 `hook_bypass_blocked`, `git --config-env=core.hooksPath=EVILVAR status` → 2, `git --config-env core.hooksPath=EVILVAR status` → 2. **Confirmed against the REAL git binary in all four resolution spellings**, with the envelope's own delivery mechanism (`cred::hooks_path_env` → `config_env` → `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n`) as the control: the control prints `/ENV_WINS`, and `-c include.path=`, `-c includeIf.gitdir:…path=`, `--config-env=include.path=VAR` and `-c INCLUDE.PATH=` each print the INCLUDED file's value. **A real push carrying a planted credential was performed end to end.** In a fixture on `refs/heads/gsd-auto/alpha/w` with a local bare upstream and a `pre-push` hook installed exactly as the envelope installs one: the plain in-namespace push is **refused by the hook** and the remote ref does not move (`6acc770`), and `git -c include.path=<evil> push origin refs/heads/gsd-auto/alpha/w` — a line the guard permits at exit 0 — **completes, moving the remote ref to `6ef47ef`**. The `pre-commit` enforcement point falls the same way: the control refuses, `git -c include.path=<evil> commit -m probe` commits. **A real DESTRUCTIVE force push was also performed**: `git config alias.q '!git -c include.path=<evil> push --force origin HEAD:refs/heads/main'` (permitted, exit 0) followed by `git q` (permitted, exit 0) **rewrote the bare remote's `main`**, while the same alias without the include is refused by the hook and moves nothing. **The three-leg argument, each leg measured.** Layer 1 — the line begins `git -c`, which does not match the `Bash(git push:*)` prefix rule the agent CLI owns. Layer 2 — exit 0 above, empty walk. Layer 3 — the hook does not run, measured. Credentials — `GIT_ASKPASS`, `GIT_CONFIG_GLOBAL/SYSTEM` and `GIT_SSH_COMMAND` are untouched, so the push authenticates. **What is lost is larger than one command.** `pre-push` is the ONLY carrier of the worktree credential scan (`hooks.rs:342`, SAFE-05, `T-19-15`/`T-19-16`/`T-19-18`) — layer 2 never scans — and it is the second carrier `AR-19-03` explicitly rests on ("the pre-push and pre-commit hooks make the denylist's gaps non-fatal"), so disarming it widens **every** open layer-2 row at once, `T-19-86`, `T-19-91` and `T-19-96` included. **Not word assembly, not word deletion, not the verb index**: every word is literal, none is deleted, the order is preserved, and `scan_leading` locates the verb correctly on every one of these lines. Rounds 4, 5, 6 and 7 are all RIGHT about them. **Not covered by `AR-19-04`** (`T-19-29`, the agent rewriting the generated git config): that acceptance rests on env-injected configuration outranking repository configuration, which audit 7 re-confirmed holds — a repo-local `core.hooksPath` and a repo-local `include.path` both lose to the injection. This is the opposite direction: a **command-line** carrier, which outranks it. | **OPEN — BLOCKING** |
| **T-19-104** | **Elevation of Privilege** | **`ENVELOPE_ENV_KEYS` / `resolve_program` step 1 `tampers_with_envelope_env` (`policy.rs:2817-2851`) — `GIT_CONFIG_PARAMETERS` is absent** | **high** | **mitigate (unimplemented)** | **`GIT_CONFIG_PARAMETERS` is git's own internal carrier for `-c`: it is the variable git EXPORTS so that a `-c` setting reaches child git processes, and setting it directly injects configuration at command-line precedence. It is not in `ENVELOPE_ENV_KEYS`, so no assignment to it is refused. Measured at `3110d4a`, fresh root per row, walk EMPTY: `GIT_CONFIG_PARAMETERS="'core.hooksPath=/tmp/nohooks'" git commit -m x` → **exit 0**; the same prefix before `git push origin refs/heads/gsd-auto/alpha/w` → **0**; the `export …; git push …` two-segment spelling → **0**; the `env GIT_CONFIG_PARAMETERS=… git push …` spelling → **0**; and the composed `GIT_CONFIG_PARAMETERS="'include.path=/tmp/evil.cfg'" git push …` → **0**. The paired discriminator is refused, which is what makes this a gap in the LIST rather than in the mechanism: `GIT_CONFIG_COUNT=0 git push origin refs/heads/gsd-auto/alpha/w` → **2 `hook_bypass_blocked`**. **Confirmed against the REAL git binary**: with the envelope's own injection in place (`GIT_CONFIG_COUNT=1 … VALUE_0=/ENV_WINS`), `GIT_CONFIG_PARAMETERS="'core.hooksPath=/PARAM_WINS'" git config --get core.hooksPath` prints **`/PARAM_WINS`** against the paired control printing `/ENV_WINS`; and in the bare-upstream fixture the in-namespace push that the `pre-push` hook refuses **completes and moves the remote ref** under this prefix. `-c` PROPAGATION was measured too, because it is what makes the disarm inherited rather than per-process: a child of git run through `git -c include.path=… rebase -x …` sees `GIT_CONFIG_PARAMETERS=['include.path'='/tmp/evil.cfg']` in its own environment, so one prefix disarms every git subprocess of the command — which is precisely how it composes with `T-19-86`. **Third omission of the same list.** `ENVELOPE_ENV_KEYS` was wrong at `GIT_SSH_COMMAND` (fixed by 19-11), wrong at `SSH_AUTH_SOCK`/`SSH_AGENT_PID` (`T-19-82`) and wrong at `GSD_MM_RUN_ID` (`T-19-90`), and was then given a re-sourced drift pin with four named floors. **That pin structurally cannot see this one**: it is sourced from `cred::EnvelopeEnv::with_run_id(build_env_in(…))`, i.e. from the keys the envelope SETS or REMOVES, and `GIT_CONFIG_PARAMETERS` is a key the envelope neither sets nor removes but which DEFEATS one it sets. Every floor the pin carries is a floor over the envelope's own entries. | **OPEN — BLOCKING** |
| T-19-105 | Spoofing | every generative alphabet, and all THREE named class axes (`UNREADABLE_CLASSES`, `DELETION_CLASSES`, `CALLEE_GRAMMAR_CLASSES`) | medium | mitigate | **`T-19-76`'s failure mode for the SEVENTH consecutive round, and for the second round running the axis moved rather than the cell.** Round 7's corpus is real and it closed `T-19-101` outright: `CALLEE_GRAMMAR_CLASSES` (`tests/envelope_wrapper_class.rs:4915`) names five classes beside a byte-identical `UNREADABLE_CLASSES` and a byte-identical `DELETION_CLASSES`, `GIT_GLOBAL_OPTIONS` carries 15 spellings against a floor of 15, and audit 6's own grep — `attr-source|shallow-file|GIT_GLOBAL_VALUE_OPTS` over `tests/`, which returned NOTHING at `9534198` — now returns two files. The corpus demonstrably failed on its own class before the rule existed: `a5404f7` and `fcdb9f7` are `tests/`-only commits with **zero `src/` hunks** carrying 5 and 2 RED tests, and `19-21`'s three `src/` commits turned them green. **But all five classes are classes of git's OPTION GRAMMAR** — consumes a word, consumes none, attached value, unaccepted, end-of-options — and the axis's own doc says so: *"ways GIT's own option grammar decides which arriving word is the verb"*. **Nothing anywhere in `tests/` or `src/` models git's CONFIG RESOLUTION or the environment git reads as config** — verified mechanically rather than read: `grep -rn "include\.path\|includeIf" src/ tests/` returns **nothing at all**, and `grep -rn "GIT_CONFIG_PARAMETERS" src/ tests/ .planning/` returns **nothing at all**, in the whole repository including seven rounds of planning documents. The corpus is therefore structurally incapable of generating, and so of failing on, `T-19-103` and `T-19-104`. Same finding as `T-19-76`, `T-19-83`, `T-19-89`, `T-19-95`, `T-19-99` and `T-19-101`, one axis further out — and the axis is again the point: each round widens the alphabets along the axis its own control is about, so the corpus keeps modelling exactly the control it certifies and nothing beside it. | open — below `high` (non-blocking) |
| T-19-106 | Spoofing | the callee-grammar drift pin's REACH — three of six callee enumerations pinned, none of the four structural arms | medium | mitigate | **The drift pin `19-21` built is the right control and it covers half its own surface.** It runs the two-sided probe over `GIT_GLOBAL_VALUE_OPTS`, `GIT_GLOBAL_SELF_CONTAINED_OPTS` and `PUSH_VALUE_OPTS`. Audit 7 re-ran that probe independently against `git version 2.43.0` over every entry of both global constants and confirms **all 24 entries are correctly classified in both directions**, `--super-prefix`/`--no-advice`/`--no-lazy-fetch`/`--bogus-opt` are all rejected by this git, and `git push -h` confirms `PUSH_VALUE_OPTS` is complete for 2.43 (`--repo`, `--receive-pack`, `--exec`, `-o/--push-option`, `--recurse-submodules` — all five present, `signed` correctly absent). **Three sibling enumerations of the same shape are unpinned**: `CONFIG_VALUE_OPTS` (`policy.rs:997`), `FORGE_VALUE_OPTS` (`:3977`) and `GH_API_VALUE_OPTS` (`:4023`). `CONFIG_VALUE_OPTS` **already carries a stale entry of exactly the `--super-prefix` shape**: `--comment` is not accepted by git 2.43 (`error: unknown option 'comment'`), and the guard consequently skips a word for it — `git config --comment core.hooksPath /dev/null` → **exit 0**, the key operand read as `/dev/null` — inert only because git rejects the option, which is the identical "mis-index of a command that does not run" the round just removed one component over. `FORGE_VALUE_OPTS` is the `glab --host` cell, reproduced by audit 7 at exit 0 with **zero** ledger lines beside `--hostname` at one, and `glab` is confirmed **not installed** on this machine so the callee caveat stands unchanged. **The four STRUCTURAL arms of `leading_git_option` are also unpinned, and two of them assert a grammar the installed git contradicts**: the `-c<rest>` arm's doc says *"git's short-option parser accepts `-ckey=value` with no space"* — measured, `git -cuser.name=x version` answers **`unknown option: -cuser.name=x`**; and deviation-1's fourth arm treats `-C/tmp` as `-C` with an attached value — measured, `git -C/tmp version` answers **`unknown option: -C/tmp`**. Both are inert in the safe direction today (they decide the shape of a command git will not run) and neither is reachable as a bypass, because git 2.43 accepts **no** short-option bundling or attachment at all. They are recorded because this round's whole thesis is that a grammar claim about the callee must be measured against the callee and pinned, and these two are neither. | open — below `high` (non-blocking) |
| T-19-107 | Repudiation | the "ZERO over-refusal cost on git 2.43.0" claim | low | mitigate | `19-21`'s record, `19-21-SUMMARY.md` and the constants' own docs all state: *"**ZERO on git 2.43.0.** Every option this git accepts is classified by the probe and enumerated, so the only rows moving permitted → refused are ones git ITSELF rejects."* **False by one measured row.** `git -v` is accepted by git 2.43.0 (`git -v` → `git version 2.43.0`; `git -v XVALUE version` → `git version 2.43.0`, so it is terminating exactly as `--version` is), it is in **neither** constant, and it is **not** in the disclosed UNPROBED set — which the pin bounds at two and names as `--help` and `-h`. Measured: `git -v` → **exit 2 `envelope_assertion_failed`**, `git -v status` → 2, beside `git --version` → 0. The cost is one refusal of an informational command, it fails in the safe direction, and it is legible; the defect is the CLAIM, not the refusal. Same class as `T-19-84` — a residual disclosure pinned to exist rather than to be true. The cheap fix is one entry in `GIT_GLOBAL_SELF_CONTAINED_OPTS` beside `--version`, which the pin's `>= 8` floor and its own two-sided probe would then cover. | open — below `high` (non-blocking) |

*Status: open · closed · open — below `high` threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` count toward `threats_open`*

`T-19-105`, `T-19-106` and `T-19-107` are open below the `high` threshold and do
**not** count toward `threats_open`. `T-19-103` and `T-19-104` do.

### Audit 7's bookkeeping, re-derived from audit 6's group rows

| Group | Count | Closed | Open |
|---|---|---|---|
| Everything through audit 6 | 114 | 93 | 21 (3 at `high`) |
| Closed by plans 19-20 / 19-21, re-measured by audit 7 (`T-19-100`, `T-19-101`, `T-19-102`) | — | +3 | −3 |
| Found by audit 7 (`T-19-103` … `T-19-107`) | 5 | 0 | 5 (2 at `high`) |
| **Total after audit 7** | **119** | **96** | **23 (4 at `high`)** |

The four that count toward `threats_open`: `T-19-86`, `T-19-91`, `T-19-103`,
`T-19-104`. The nineteen that do not: `T-19-61` … `T-19-73` (13), `T-19-84`,
`T-19-85`, `T-19-96`, `T-19-105`, `T-19-106`, `T-19-107`.

### The closures, re-measured rather than accepted from the summaries

Driven as
`printf '<PreToolUse JSON>' | GSD_MM_ENVELOPE_ROOT=$(mktemp -d) ./target/debug/gsd-meta-manager envelope guard alpha`,
one fresh root per row, the envelope root walked with `os.walk` afterwards. The
walk is proved non-blind on every pass by a positive control: `gh pr create
--title x` leaves `alpha/pr-ledger.ndjson` with one line on call 1 and the cap
fires on call 2 with two — so an empty walk beside a refusal means something.

* **`T-19-100` — CLOSED, all nine rows plus every composition cell.**

  ```
  exit=2 [force_push_blocked]        git --attr-source HEAD push --force origin main
  exit=2 [force_push_blocked]        git --attr-source HEAD stash
  exit=2 [force_push_blocked]        git --attr-source HEAD update-ref -d refs/heads/main
  exit=2 [hook_bypass_blocked]       git --attr-source HEAD config core.hooksPath /tmp/x
  exit=2 [force_push_blocked]        git --attr-source HEAD reflog delete HEAD@{0}
  exit=2 [force_push_blocked]        git --attr-source HEAD symbolic-ref HEAD refs/heads/x
  exit=2 [force_push_blocked]        git --shallow-file /tmp/s push --force origin main
  exit=2 [force_push_blocked]        git --shallow-file /tmp/s stash
  exit=2 [hook_bypass_blocked]       git --attr-source HEAD -c core.hooksPath=/dev/null push --force origin main
  exit=2 [force_push_blocked]        git --attr-source=HEAD push --force origin main      (attached control)
  ```

  Every walk EMPTY. The planning cells go with them:
  `git --super-prefix push --force origin main` → 2 `envelope_assertion_failed`,
  `git -c a=b --attr-source HEAD push …` → 2 `force_push_blocked`,
  `git >/dev/null --attr-source HEAD push …` → 2, the reverse order → 2, the
  real backslash-newline **inside the option name**
  ``git --attr-so`\`+NL+`urce HEAD push …`` → 2, `bash -lc "…"` → 2,
  `echo hi && …` → 2, `git -pc user.name=x status` → 2
  `envelope_assertion_failed`. The two pinned controls the rule must not widen
  into hold: `git - push --force origin main` still **exit 0** and
  `git -- push --force origin main` still 2 `force_push_blocked`.
* **`T-19-102` — CLOSED, and confirmed against the callee from both sides.**
  In a purpose-built fixture on `refs/heads/gsd-auto/alpha/w` with a local bare
  upstream: `git push --recurse-submodules on-demand origin
  refs/heads/gsd-auto/alpha/w` → **exit 0** (was 2), its twin → 0, and real git
  runs the line to completion (`Everything up-to-date` / `ok (up-to-date)`).
  **`git push --signed no origin refs/heads/gsd-auto/alpha/w` stays REFUSED** at
  `push_outside_namespace`, and real git confirms why: `git push --dry-run
  --signed no …` answers `error: src refspec origin does not match any` /
  `failed to push some refs to 'no'` — `no` becomes the repository. `git push -h`
  spells the two identically enough that a list completed from the help text
  would add `signed`; it would land red on this control. `-o ci.skip`,
  `--push-option ci.skip`, `--recurse-submodules=on-demand` and `--dry-run` all
  at exit 0; `git push origin refs/heads/main` → 2 `push_outside_namespace` and
  `git push --force origin refs/heads/gsd-auto/alpha/w` → 2 `force_push_blocked`,
  so the namespace and flag checks are untouched.
* **`T-19-101` — CLOSED as scoped, and the scope is stated rather than implied.**
  The third axis exists with five degenerate-proofed classes, a
  `MIN_CALLEE_GRAMMAR_CLASSES` floor of 5 and a `MIN_GIT_GLOBAL_OPTIONS` floor of
  15, and the corpus failed on its own class before the rule (`a5404f7`,
  `fcdb9f7`: `tests/`-only, zero `src/` hunks, RED). It is closed **for the
  callee's option grammar**. **The corpus's NEW blind spot is `T-19-105`** — the
  axis models git's option grammar and nothing of git's config resolution or of
  the environment git reads as config.
* **The `--super-prefix` removal — verified against the callee, both directions.**
  git 2.43.0 answers `unknown option: --super-prefix` for the bare, the
  separate-value and the attached spellings, so the removal was required; and the
  guard now refuses `git --super-prefix push --force origin main` (a live
  over-consuming mis-index at `9534198`) at `envelope_assertion_failed` with an
  empty walk.
* **The two-sided real-git probe, re-run independently by audit 7 over every
  constant entry.** All eight `GIT_GLOBAL_VALUE_OPTS` entries reach the verb only
  in the two-word form (`-C` and `--config-env` through their documented variant
  probes); all sixteen `GIT_GLOBAL_SELF_CONTAINED_OPTS` entries reach the verb in
  the one-word form, with the five terminating spellings producing identical
  first lines in both; and the four negative controls are all rejected. **The
  constants match the installed git exactly.**

### The mechanism pins, verified non-dead

Rounds 4, 5 and 6 are all still load-bearing and all still RIGHT about the lines
round 7 refuses:

```
exit=2 [envelope_assertion_failed]  git pus? --force origin main            <- round 5's literalness bit
exit=2 [force_push_blocked]         git >/dev/null push --force origin main <- round 6's deletion model
exit=0                              git x2>/tmp/o push --force origin main  <- round 6's over-deletion control
exit=0                              git >/dev/null status                   <- ordinary redirection
exit=0                              { git status; }   ( git status )        <- grouping unaffected
exit=2 [force_push_blocked]         { git push --force origin main; }
exit=0                              git commit -m ">"    rg ">" src/        <- quoted-operator corpus
```

`SEPARATORS` is **byte-identical** — `const SEPARATORS: &[&str] = &[";", "&&",
"||", "|", "&", "\n", "(", ")", "{", "}"]`, and `git log -L` over that line
returns exactly one commit in the whole phase (`84a9b05`, plan 19-05) — and
`is_separator` is a bare `SEPARATORS.contains`, so `is_separator(">") == false`
by construction. The composition cells compose in both directions:
`git --attr-source $T push --force origin main` → 2 `force_push_blocked`,
`git --attr-source *.x push …` → 2 `force_push_blocked`,
`git --attr-source HEAD {push,--force} origin main` → 2
`envelope_assertion_failed`, `git --attr-source HEAD $V --force origin main` → 2,
`V=push; git --no-pager $V --force origin main` → 2, and
`git --no-pager >/dev/null push --force origin main` → 2 `force_push_blocked`.

### The known-open set, as audit 7 verified it

- **`T-19-86` (high, OPEN, unaccepted, deferred by explicit user scoping
  decision).** All four rows re-measured at `3110d4a`, fresh root each, all at
  **exit 0**: `git submodule foreach git push --force origin main`,
  `git rebase -x "git push --force origin main" HEAD~3`,
  `git bisect run sh -c "git push --force origin main"`,
  `git -c alias.p='!git push --force origin main' p`. Exactly as registered.
  Counts toward `threats_open`. **One arm audit 7 measured that the register does
  not name, recorded here rather than as a new ID because the class and the
  component are the same:** the alias need not be on the command line at all.
  `git config alias.p '!git push --force origin HEAD:refs/heads/main'` → exit 0
  and `git p` → exit 0, in two separately-permitted tool calls the stateless
  guard cannot correlate. With the hooks live, layer 3 catches the inner push and
  the remote ref does not move — which is `AR-19-03` working — and that is
  precisely what `T-19-103` removes.
- **`T-19-91` (high, OPEN, three arms).** `S=refs/heads/main; git reflog $S`,
  `git reflog show $S` and `git symbolic-ref $S` all reproduce at **exit 0** with
  no second carrier; `git symbolic-ref HEAD $R` → 2 `force_push_blocked` and
  `git push origin $REF` → 2 `push_outside_namespace` still fail closed; and the
  bare `git push $REF` reproduces at **exit 0** in the in-namespace fixture.
  Unweakened by round 7. Counts toward `threats_open`.
- **`T-19-96` (medium, OPEN, registered, not fixed).** In the fixture,
  `git push --forc? origin refs/heads/gsd-auto/alpha/w` → **exit 0**, its literal
  twin → 2 `force_push_blocked`. Unchanged.
- **`T-19-74` (medium, closed/accepted — AR-19-10).** Core rows frozen and
  re-measured permitted: `env $X push --force origin main` → **0**,
  `X=git; env $X push --force origin main` → **0**. `T-19-84` remains open and
  unaccepted.
- **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85` — carried forward untouched**,
  open and unaccepted at their original severities, all below `high`. Plans 19-20
  and 19-21 touched `src/envelope/policy.rs` and three test files only;
  `cred.rs`, `mod.rs`, `advisory.rs`, `scan.rs`, `config.rs` and **`hooks.rs`**
  were not opened (`git diff --stat c21c13f..3110d4a -- src/` is one file), so
  `T-19-61` … `T-19-73` cannot have moved.
- **The `glab --host` forge cell — reproduced, and its caveat is unchanged.**
  `glab --host gitlab.com mr create --title x` → exit 0 with **ZERO** ledger
  lines on both calls; `glab --hostname …` → one line then `pr_cap_exceeded`.
  `glab` is confirmed **not installed** on this machine, so audit 7 does **not**
  upgrade it: it is a planning-time cell needing callee confirmation, not a
  claimed bypass. `gh` re-swept clean — `gh pr create`, `gh --repo o/r pr create`
  and `gh api repos/o/r/pulls -f title=x` each leave exactly one ledger line and
  the cap fires on call 2.
- **`T-19-SC` — still holds.** Neither `Cargo.toml` nor `Cargo.lock` appears in
  any commit between `c21c13f` and `3110d4a`.
- **`T-19-17r` — still OUTSTANDING, and audit 7 does not resolve it either.**
  `19-17-SUMMARY.md` calls it "accepted"; there is still **no `AR-19-13` row and
  no register row**, and the Accepted-Risks-Log row count for that risk ID is
  **zero**. Plans `19-20` and `19-21` and the follow-up all correctly declined to
  make the acceptance and all avoided applying the word. **Accepting a risk is a
  human decision and this audit does not make it**, for the third audit running.
  The next round either adds the log row or drops the word from
  `19-17-SUMMARY.md`.

### The four execution-time judgements, assessed independently

1. **The blocker reported rather than worked around — the CALL was right, the
   CORRECTION is right, and audit 7 extended the sweep and agrees it was the only
   row of its class.** Verified three ways. *Structurally*: `--super-prefix` is in
   neither constant at `3110d4a` and was in `GIT_GLOBAL_VALUE_OPTS` at `097dba2`;
   `scan_leading` returns the refusal through its one channel and `classify_git`
   acts on it first, so two different identifiers from identical leading tokens
   is impossible without the prohibited second reading site, and no such site was
   built. *By measurement*: `git --super-prefix x status` and
   `git --super-prefix x push --force origin main` are **both** exit 2
   `envelope_assertion_failed` with an EMPTY walk, and the four sibling controls
   keep their identifiers (`git -C/tmp push …` → `force_push_blocked`,
   `git -- push …` → `force_push_blocked`,
   `git -c core.hooksPath=/dev/null --attr-source HEAD push` →
   `hook_bypass_blocked`, `git - push …` → exit 0). *Against the callee*: git
   2.43.0 rejects `--super-prefix` bare, separate-value and attached, so the row
   is a mis-index of a command that does not run and `envelope_assertion_failed`
   is the more honest identifier — naming `force_push_blocked` for a command
   whose verb the guard admits it cannot establish attributes the refusal to a
   mechanism that did not produce it (D-24). **On the sweep's completeness, audit
   7 widened the method rather than repeating it.** The follow-up grepped for the
   five option NAMES; that is sufficient only if no row elsewhere spells a leading
   git option whose bit changed. Audit 7 checked the stronger predicate — every
   identifier-asserting row in the suite (227 across 11 files) that spells a
   leading `git -`/`git --` token — and **every such row outside the two swept
   files spells `-c`**, which was already in `GIT_GLOBAL_VALUE_OPTS` and which
   `19-21` did not touch. The conclusion holds and it now holds for the right
   reason.
2. **The fourth arm — CORRECT, its grammar restriction does what the summary
   claims, and audit 7 probed for a bundling spelling it mishandles and found
   none — but its reach is narrower than the doc reads.** Because the `-c<rest>`
   arm is ordered first and intercepts every `-c…` token, arm 4's
   `GIT_GLOBAL_VALUE_OPTS.contains(&head)` check can only ever match **`-C`** —
   the two constants hold exactly two entries of length two and one is taken. The
   restriction to value-taking heads is therefore doing real work
   (`-pc user.name=x` stays unestablished and is pinned so) but it is a
   one-element test today; it generalises only if the constant gains another
   two-character value-taking spelling. **Bundling probe, against the callee**:
   git 2.43.0 accepts **no** short-option bundling or attachment at all —
   `-pc user.name=x`, `-pP`, `-C/tmp`, `-cuser.name=x` and a bare `-` each answer
   `unknown option:` — so there is no bundle for the arm to mishandle, and both
   the arm's premise and the older `-c<rest>` arm's stated premise are grammar
   claims the installed git contradicts. Inert in the safe direction, unpinned,
   and recorded as `T-19-106` rather than smoothed over.
3. **The spawn-allowlist compensating control — the INSTINCT was right and the
   control is genuinely stricter in the dimension that matters, but it does NOT
   fully cover what the file-level entry gave up, in two measurable ways.** The
   file-level entry makes `every_process_spawn_site_in_src_is_on_the_allowlist`
   stop looking at `src/envelope/policy.rs` entirely; the local
   `the_guards_own_path_shells_out_to_nothing` re-asserts absence over the
   production half only, which is the right granularity and the right property.
   The two gaps: **(a) the marker set is narrower.** The global control carries
   `["Command::new(", "CommandWrap::with_new(", "process_group("]`; the local one
   carries only `["Command::new(", "process_group("]`. `CommandWrap::with_new(`
   is a spelling **this repository actually uses** (`src/executor/claude.rs:480`),
   so a production spawn written that way in `policy.rs` would now be invisible to
   both controls. **(b) the truncation guard is thin at exactly the seam that
   already failed once this round.** The local control's positive controls are
   `fn scan_leading` (line 374) and a 40,000-byte floor over the RAW production
   half, which runs to 207,358 bytes; 40,000 raw bytes is reached at **line 778**,
   so a stray `#[cfg(test)]` anywhere after line 778 in a 4,262-line production
   half silently truncates the control's view with both positive controls still
   green. This round's own incident put such a line at ~607 — **below 778, which
   is the only reason a floor caught it at all**. Neither gap is a threat today
   (no spawn marker of any spelling appears in the production half), so this is
   recorded as an observation on the control rather than registered.
4. **The anti-vacuity control's mid-round fire — CONFIRMED, and the byte floors
   are well-founded; audit 7 additionally finds the replacement was NECESSARY
   rather than tidy.** `GIT_GLOBAL_UNPROBED_OPTS` is now at `policy.rs:6145`,
   inside `mod tests` (line 4264), so the first `#[cfg(test)]` in the file is the
   module boundary at 4263 and the stripper's view is the whole production half.
   Nothing in the control was weakened. **The floors, re-derived with the file's
   own `production_code` at `3110d4a` and measured in BYTES**: `policy.rs` raw
   307,496 → stripped **68,785** against a floor of 40,000 (41% headroom);
   `hooks.rs` raw 99,909 → stripped **33,460** against 20,000 (40%). Both are
   comfortably above their floors, both floors are independent of comment volume,
   and a stripper that ate the logic drops to near zero. **And the deleted 25%
   ratio would be RED today**: `policy.rs`'s current ratio is **22.37%**, so
   `19-21`'s own doc additions would have failed the assertion `19-20` removed.
   That is the strongest possible corroboration of `19-20`'s judgement — the
   control it deleted was a documentation budget, and this round would have spent
   it.

### Gates observed

`rtk proxy cargo test --no-fail-fast` redirected to a file and counted with
**`rtk proxy grep`** (a plain `grep` is rewritten by the RTK hook, which strips
the `test result:` lines a count is read from — D-34): **1639 passed, 0 failed,
13 ignored** over 42 result lines, `passed + failed = 1639`, matching
`19-21-SUMMARY.md` and the follow-up record exactly. **All THIRTEEN `envelope_*`
binaries ran** — `advisory`, `argv_deletion`, `callee_grammar`,
`command_position`, `credential`, `expansion_slots`, `hook_refusals`,
`literal_decision`, `pr_cap`, `tracer`, `wiring`, `wrapper_bypass`,
`wrapper_class` — so `19-20`'s own evidence file executed. None of the three
documented flakes fired; their absence is not evidence they are fixed.
`rtk proxy cargo clippy -- -D warnings` exits 0. `cargo clippy --tests` was
already failing at the base on four pre-existing lints in `src/browser.rs` and
`src/project_creator.rs` and is out of scope.

**Provenance discipline.** Plans 19-13 … 19-21's appended subsections, the
plan-19-21 blocker-row resolution record and every earlier audit's own tables are
left **byte-identical**; audit 7 checksummed the whole body below the frontmatter
before writing. Audit 7's corrections to statements made in those subsections —
the "ZERO over-refusal cost" claim, the two structural arms' grammar claims and
`cred.rs`'s "the one form that outranks this injection" — are recorded as audit-7
findings BESIDE them rather than as edits to them.

---

## Audit 7 — what the round-7 control can and cannot fail on

### The principle rounds 3 through 6 established still holds

A decision region must be derived from the same scan the classifier runs, and
there is one walk. `19-21` added the third answer **inside** `scan_leading` and
returned it through the `(usize, Option<GitVerdict>)` channel that already carried
the unreadable-key refusal, so no second reading site was created: `git diff
--stat c21c13f..3110d4a -- src/` is exactly one file, `hooks.rs` was not opened,
no `ParkReason` variant was added, and
`first_unreadable_decision_word`, `config_key_operand_index`,
`subcommand_word_indices`, `scan_gh_api` and `resolve_program_with_head` have no
hunk. `SEPARATORS` is byte-identical and `is_separator(">")` is `false`. The
structural rules that need no knowledge of git run FIRST and are what keep the
constants small — an attached `=` value, `--`, a non-`-` token — which is the
correct reduction and is why `git --git-dir=/tmp/g status` and
`git --namespace=n log` keep working through options the constants never learn.

### Where the boundary now is, in one paragraph

**Rounds 5, 6 and 7 together decide three questions and audit 7 finds all three
answered.** A decision word must be provably LITERAL; the words the guard
classifies must be exactly the words the program receives in the same order; and
the word the guard calls the verb must be the word git calls the verb, with an
unestablished slot refusing rather than guessing. **What none of them asks is what
the verb will RUN UNDER.** Having built argv correctly and located the verb
correctly, the guard makes exactly one statement about git's configuration — *is
this `-c` assignment's key `core.hooksPath`* — and git's config resolution reaches
that setting by at least four further routes the guard does not model:

- **an indirection in the config graph** — `include.path`, `includeIf.<cond>.path`,
  and `--config-env=include.path=<VAR>`, which splice an arbitrary file in at the
  precedence of the directive that named them (`T-19-103`);
- **git's own environment carrier for `-c`** — `GIT_CONFIG_PARAMETERS`, absent
  from `ENVELOPE_ENV_KEYS`, and exported by git itself so one prefix disarms every
  git subprocess of the command (`T-19-104`);
- **a classifier arm's own operand or flag outside the decision region** —
  `T-19-91` and `T-19-96`, both registered and both widened by the two above;
- **a whole command line handed to a governed program as data** — `T-19-86`,
  unchanged, and now measured through a *persisted* alias as well as a
  command-line one.

**The one-sentence version for the next round.** Seven rounds have modelled how a
command line becomes an argv and which word in it is the verb; none has modelled
what that verb runs under, and the single setting the whole envelope's third layer
depends on is reachable without ever being named.

### Suggested closure, in order — (d) FIRST, for the seventh round running

1. **(d) — widen the corpus BEFORE certifying anything.** Add a FOURTH named axis
   beside `UNREADABLE_CLASSES`, `DELETION_CLASSES` and `CALLEE_GRAMMAR_CLASSES` —
   *a carrier that changes the configuration the verb runs under* — with an
   alphabet of config-resolution spellings (`-c include.path=`,
   `-c includeIf.gitdir:<p>.path=`, `--config-env=include.path=<VAR>`, the
   case-varied `INCLUDE.PATH`) and environment carriers
   (`GIT_CONFIG_PARAMETERS`, and the `export …;` and `env …` spellings of it),
   spliced ahead of a decision word, with the `MIN_*`-floor pattern asserting the
   corpus generates them. **Listed first for the seventh round running**, and for
   the seventh round running the previous audit's recommendation was right about
   its own cell and the next gap was in a cell it did not name.
2. **(a) — `T-19-103`, and make it a PROPERTY of the key rather than a longer
   list.** The durable shape is not "add `include.path` to
   `is_hooks_path_key`" — `includeIf.<arbitrary condition>.path` is an open family
   and the next release may add another. It is: **a `-c` / `--config-env`
   assignment whose key the guard cannot prove is inert must make the command
   unresolvable**, which is the same fail-closed treatment `scan_leading` just
   received for the verb slot and `resolve_program`'s wrapper axis has had since
   audit 3. The paired cost must be pinned from both sides exactly as clause 2's
   and Rule B's were: `git -c user.name="$NAME" commit -m x`,
   `git -c a=b --attr-source HEAD push …` and every ordinary `-c` in the suite
   must keep reaching their present verdicts.
3. **(b) — `T-19-104`**, one entry in `ENVELOPE_ENV_KEYS`, and — because that list
   has now been wrong four times — a floor in its drift pin for a key the envelope
   **does not set** but which **defeats one it does**. The pin's current source
   makes that class structurally invisible; the fix is a second source, not a
   wider filter.
4. **(c) — `T-19-106`'s unpinned siblings**: extend the two-sided real-git probe
   to `CONFIG_VALUE_OPTS` (which would fail today on `--comment`) and to the four
   structural arms of `leading_git_option`; then `T-19-107`'s one entry.
5. **(e)** — then `T-19-86`, `T-19-91` and `T-19-96`, which are three arms of one
   shape: a classifier arm answering `Allow` on an operand, flag or payload
   outside the decision region.

Then, and separately: add the `AR-19-13` row for `T-19-17r` or stop calling it
accepted; add `CommandWrap::with_new(` to the local spawn control's marker set and
give its stripper a floor proportional to the file it guards; and get a machine
with `glab` installed to settle the `--host` cell against its callee.

---

## Audit 7 Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log — twelve, unchanged by
      round 7; audit 7 accepts nothing new
- [x] Every closure re-measured against the built binary at `3110d4a` with a
      fresh envelope root per row and the root walked afterwards
- [x] Every new finding confirmed against the **REAL `git` binary**
      (`git version 2.43.0`), including a real in-namespace push carrying a
      planted credential that the `pre-push` hook refuses and that completed
      through `-c include.path=`, and a real DESTRUCTIVE force push that rewrote a
      bare remote's `main` through a persisted alias carrying the same
- [x] The envelope's own delivery mechanism used as the control in every
      precedence measurement (`GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n`, the triplet
      `cred::hooks_path_env` emits), not a stand-in
- [x] The PR-cap walk proved non-blind by a positive control on every pass
- [x] `SEPARATORS` byte-identical (one commit in the phase has ever touched that
      line) and `is_separator(">") == false`; rounds 4, 5 and 6's mechanism pins
      re-measured non-dead
- [x] The byte floors re-derived with the file's own stripper: `policy.rs`
      68,785 / 40,000 and `hooks.rs` 33,460 / 20,000
- [x] Plans 19-13 … 19-21's appended subsections and the blocker-row resolution
      record left byte-identical, verified by checksum before writing
- [x] Gate observed: `rtk proxy cargo test --no-fail-fast` → **1639 passed, 0
      failed, 13 ignored**, `passed + failed = 1639`; **all thirteen** `envelope_*`
      binaries ran; `cargo clippy -- -D warnings` exit 0
- [ ] `threats_open: 0` confirmed — **4 open at `high`: `T-19-86`, `T-19-91`,
      `T-19-103`, `T-19-104`**
- [ ] `status: verified` set in frontmatter

**Approval: blocked, 2026-09-03 (audit 7).**

**Not accepted here.** `T-19-103` and `T-19-104` are high-severity, empirically
confirmed bypasses of layer 3 — the layer `AR-19-03` explicitly rests on and the
only carrier of the worktree credential scan — reached on single command lines the
guard permits, with the credential intact, and each demonstrated by a real push
that moved a bare remote's ref. `T-19-86` and `T-19-91` remain open at `high` by
scoping decision and by round discipline respectively. Accepting any of the four
is a human decision and this audit does not make it.

**Progress is real, and the round did what it said.** `19-20` wrote a corpus that
demonstrably failed on its own class before any production line moved — two
`tests/`-only commits with zero `src/` hunks — and `19-21` closed `T-19-100` by
inverting a failure direction rather than completing a list, gave the constants
the drift pin `ENVELOPE_ENV_KEYS` was given after being wrong twice, removed a
stale entry that was itself a live over-consuming mis-index, and did it in one
production file without opening `hooks.rs`, without a second reading site and
without a new park reason. It closed `T-19-102` with a one-entry change whose
discrimination control (`--signed`) proves the list was not completed from the
help text, and its executor reported a blocker rather than editing an assertion —
which was the right call and, verified three ways, the right analysis. **The
callee's option grammar is now modelled and fail-closed, and audit 7 says so
plainly.** The remaining risk is not the disclosed residual set. Seven rounds have
modelled how a command line becomes an argv and which word in it is the verb; none
has modelled **what that verb runs under**, and `core.hooksPath` — the one setting
the envelope's whole third layer is, and the one `T-19-09` denies by name — is
reachable through git's own config resolution and through git's own environment
without ever being named. Seven rounds have modelled the shell and then git's
front door. The next one has to model git's config.

## Execution record — plan 19-22 (the corpus, RED). NOT an audit finding.

**This is a record made by plan 19-22, not a finding made by an audit.** Nothing
above this line was edited: no audit table, no Security Audit Trail row, no
Accepted Risks Log row, no Sign-Off, and no earlier appended subsection.
Re-measuring and re-classifying these rows is `/gsd-secure-phase 19`'s job, and
an audit's own tables are its provenance.

**This plan closes NOTHING.** `T-19-103`, `T-19-104`, `T-19-105`, `T-19-106` and
`T-19-107` all stay open at its end. **`T-19-86` and `T-19-91` remain OPEN at
`high`, so `/gsd-secure-phase 19` is NOT cleared** — not by this plan, not by
`19-23`, and not by the two together.
Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.

### The invariant this round is about, stated once

**The configuration the guard assumes is in effect must be the configuration git
will actually resolve.**

Rounds 5, 6 and 7 closed the three questions below it, and audit 7 verified all
three answered: a decision word must be provably LITERAL; the words the guard
classifies must be exactly the words the program receives in the same order; the
word the guard calls the verb must be the word git calls the verb. Audit 7's
framing is adopted verbatim — git turns a command line into behaviour in THREE
stages and this phase now models one — so this round is a stage further into what
git does with a command line, not an eighth spelling of the same class.

### The finding that shapes the whole corpus, stated FIRST

**A `T-19-103` reproducer must be built on a base LAYER 2 PERMITS.** The whole
content of an indirection is that it disarms layer 3 on a line layer 2 lets
through, so a reproducer whose base is already refused for its verb proves nothing
about config resolution. Re-measured against the built binary at `d0eb738`, one
fresh `GSD_MM_ENVELOPE_ROOT` per row, envelope directory walked afterwards and
EMPTY on every one:

| Command | exit | reason id | walk |
|---|---|---|---|
| `git -c include.path=/tmp/evil.cfg push --force origin main` | 2 | `force_push_blocked` | EMPTY |
| `bash -lc "git -c include.path=/tmp/evil.cfg push --force origin main"` | 2 | `force_push_blocked` | EMPTY |
| `echo hi && git -c include.path=/tmp/evil.cfg push --force origin main` | 2 | `force_push_blocked` | EMPTY |
| `git >/dev/null -c include.path=/tmp/evil.cfg push --force origin main` | 2 | `force_push_blocked` | EMPTY |
| `git --attr-source HEAD -c include.path=/tmp/evil.cfg push --force origin main` | 2 | `force_push_blocked` | EMPTY |

**All five are CONTROLS.** A corpus built out of them would be green before the
fix and would certify nothing — the eighth consecutive instance of `T-19-76`'s
failure mode, produced by the corpus rather than found by the next audit. The
reproducers use the three layer-2-PERMITTED bases audit 7 itself used —
`commit -m x`, `status`, and the in-namespace `push origin
refs/heads/gsd-auto/alpha/w` — and `tests/envelope_wrapper_class.rs` asserts that
constraint MECHANICALLY for the generative arm rather than leaving it to care.

### `T-19-103` — the ten rows, all measured, all walks EMPTY

| Command | exit BEFORE | reason id | pinned AFTER |
|---|---|---|---|
| `git -c include.path=/tmp/evil.cfg commit -m x` | 0 | (permit) | 2 `envelope_assertion_failed` |
| `git -c include.path=/tmp/evil.cfg status` | 0 | (permit) | 2 `envelope_assertion_failed` |
| `git -c include.path=/tmp/evil.cfg push origin refs/heads/gsd-auto/alpha/w` | 0 | (permit) | 2 `envelope_assertion_failed` |
| `git -c includeIf.gitdir:/tmp/.path=/tmp/evil.cfg status` | 0 | (permit) | 2 `envelope_assertion_failed` |
| `git --config-env=include.path=EVILVAR status` | 0 | (permit) | 2 `envelope_assertion_failed` |
| `git --config-env include.path=EVILVAR status` | 0 | (permit) | 2 `envelope_assertion_failed` |
| `git -c INCLUDE.PATH=/tmp/evil.cfg push origin refs/heads/gsd-auto/alpha/w` | 0 | (permit) | 2 `envelope_assertion_failed` |
| `git -c "include.path=/tmp/evil.cfg" status` | 0 | (permit) | 2 `envelope_assertion_failed` |
| `git -c include.pathx=/tmp/evil.cfg status` | 0 | (permit) | 2 `envelope_assertion_failed` (disclosed cost) |
| `git -c include.path=… -c core.hooksPath=/dev/null push --force origin main` | 2 | `hook_bypass_blocked` | 2 `envelope_assertion_failed` (ordering) |

**Every audit-7 row reproduced at its recorded verdict. None failed to
reproduce.**

### `T-19-104` — the six rows, all measured, all walks EMPTY

| Command | exit BEFORE | pinned AFTER |
|---|---|---|
| `GIT_CONFIG_PARAMETERS="'core.hooksPath=/tmp/nohooks'" git commit -m x` | 0 | 2 `hook_bypass_blocked` |
| `GIT_CONFIG_PARAMETERS="'core.hooksPath=/tmp/nohooks'" git push origin refs/heads/gsd-auto/alpha/w` | 0 | 2 `hook_bypass_blocked` |
| `export GIT_CONFIG_PARAMETERS="'core.hooksPath=/tmp/nohooks'"; git push origin refs/heads/gsd-auto/alpha/w` | 0 | 2 `hook_bypass_blocked` |
| `env GIT_CONFIG_PARAMETERS="'core.hooksPath=/tmp/nohooks'" git push origin refs/heads/gsd-auto/alpha/w` | 0 | 2 `hook_bypass_blocked` |
| `GIT_CONFIG_PARAMETERS="'include.path=/tmp/evil.cfg'" git push origin refs/heads/gsd-auto/alpha/w` | 0 | 2 `hook_bypass_blocked` |
| `echo GIT_CONFIG_PARAMETERS` | 0 | 2 `hook_bypass_blocked` (disclosed bare-word cost) |

### The paired discriminators — the eleven rows that prove this a gap in REACH

All measured at exit 2 with EMPTY walks and asserted **UNCHANGED**:

| Command | reason id |
|---|---|
| `git -c core.hooksPath=/dev/null commit -m x` | `hook_bypass_blocked` |
| `git --config-env=core.hooksPath=EVILVAR status` | `hook_bypass_blocked` |
| `git --config-env core.hooksPath=EVILVAR status` | `hook_bypass_blocked` |
| `git -c CORE.HOOKSPATH=/dev/null status` | `hook_bypass_blocked` |
| `GIT_CONFIG_COUNT=0 git push origin refs/heads/gsd-auto/alpha/w` | `hook_bypass_blocked` |
| `export GIT_CONFIG_COUNT=0; git push origin refs/heads/gsd-auto/alpha/w` | `hook_bypass_blocked` |
| `env GIT_CONFIG_COUNT=0 git push origin refs/heads/gsd-auto/alpha/w` | `hook_bypass_blocked` |
| `GIT_CONFIG_COUNT=0 git status` | `hook_bypass_blocked` |
| `export GIT_CONFIG_COUNT=0; git status` | `hook_bypass_blocked` |
| `env GIT_CONFIG_COUNT=0 git status` | `hook_bypass_blocked` |
| `echo GIT_CONFIG_COUNT` | `hook_bypass_blocked` |

**These are what make every `T-19-104` post-fix verdict DERIVABLE rather than
guessed**: all three environment spellings already refuse for a key that IS in
`ENVELOPE_ENV_KEYS`, on BOTH a refused and a permitted base, so adding one entry
derives all five rows and the bare-word cost twin.

### The discrimination controls, the disclosed cost, the permitted half

| Command | exit BEFORE | pinned AFTER | what it is |
|---|---|---|---|
| `git -c includepath=/tmp/evil.cfg status` | 0 | **0** | this round's `--signed no` — no `.`, so no section |
| `git -c notinclude.path=/tmp/evil.cfg status` | 0 | **0** | the same control from the other side — the SECTION decides |
| `git -c include.pathx=/tmp/evil.cfg status` | 0 | 2 `envelope_assertion_failed` | the disclosed cost, in the safe direction |
| `git -c user.name="$NAME" commit -m x` | 0 | **0** | this axis's `ls {git,svn}-repo` |
| `git -c core.pager=cat log` | 0 | **0** | permitted half |
| `git -c a=b status` | 0 | **0** | the dotless key — `CALLEE_KNOWN_LEADING_PREFIX` |
| `git --git-dir=/tmp/g status` | 0 | **0** | permitted half |
| `git -C /tmp status` | 0 | **0** | permitted half |
| `git --no-pager status` | 0 | **0** | round 7's own control, carried here |
| `git -c a=b push --force origin main` | 2 `force_push_blocked` | unchanged | verdict-preserving refusal |
| `git -c a=b --attr-source HEAD push --force origin main` | 2 `force_push_blocked` | unchanged | verdict-preserving refusal |
| `git -c includeIf.gitdir:~/p/.path=/tmp/evil.cfg status` | 2 `envelope_assertion_failed` | unchanged | **CONTROL** — the existing rewriting-character clause, not this round's rule |
| `git -c core.hooksPath=/dev/null -c include.path=… push --force origin main` | 2 `hook_bypass_blocked` | unchanged | ordering pin, hooks key first |
| `git >/dev/null -c include.path=/tmp/evil.cfg status` | 0 | 2 `envelope_assertion_failed` | composition with round 6 |
| `git --attr-source HEAD -c include.path=/tmp/evil.cfg status` | 0 | 2 `envelope_assertion_failed` | composition with round 7 |
| `bash -lc "git -c include.path=/tmp/evil.cfg status"` | 0 | 2 `envelope_assertion_failed` | composition with the wrapper axis |
| `echo hi && git -c include.path=/tmp/evil.cfg status` | 0 | 2 `envelope_assertion_failed` | composition with the segment axis |
| `gh pr create --title x` | 0, **one** ledger line | unchanged | the walk's non-blindness control (cap fires with two on call 2) |

### The real-git precedence table, measured with the envelope's OWN injection as the control

`git version 2.43.0`. The control is the exact triplet `cred::hooks_path_env`
emits — count DERIVED from one pair, `GIT_CONFIG_KEY_0`, `GIT_CONFIG_VALUE_0` —
not a stand-in. **A carrier that does not outrank the injection is not an
indirection and is not a reproducer.**

```
control:  GIT_CONFIG_COUNT=1 KEY_0=core.hooksPath VALUE_0=/ENV_WINS
          git config --get core.hooksPath                     -> /ENV_WINS
+ -c include.path=<file>                                      -> /INCLUDE_WINS
+ -c INCLUDE.PATH=<file>                                      -> /INCLUDE_WINS
+ -c includeIf.gitdir:<p>.path=<file>                         -> /INCLUDE_WINS
+ -c INCLUDEIF.gitdir:<p>.PATH=<file>                         -> /INCLUDE_WINS
+ --config-env=include.path=EVILVAR                           -> /INCLUDE_WINS
+ --config-env include.path=EVILVAR                           -> /INCLUDE_WINS
+ GIT_CONFIG_PARAMETERS="'core.hooksPath=/PARAM_WINS'"        -> /PARAM_WINS
+ GIT_CONFIG_PARAMETERS="'include.path=<file>'"               -> /PARAM_INCLUDE_WINS
+ -c include.pathx=<file>                                     -> /ENV_WINS   (git IGNORES it)
+ -c notinclude.path=<file>                                   -> /ENV_WINS
persisted `git config include.path <file>`, alone            -> /INCLUDE_WINS
persisted `git config include.path <file>`, under injection  -> /ENV_WINS   (INERT)
git -c a=b version                                            -> git version 2.43.0, rc 0
git -c a=b config --get a  -> error: key does not contain a section: a, rc 1
```

**The persisted-`git config` row is the reason the `git config` subcommand needs
no clause**: a repo-level include splices at REPOSITORY precedence and LOSES to
the injection, so it is not a second escape. Measured, not assumed.

### The case-insensitivity question, ANSWERED EXPLICITLY

**Yes: key comparison must be ASCII-case-insensitive on the SECTION and on the
VARIABLE, and the SUBSECTION must not be read at all.** Git folds the section and
the variable to lower case and leaves the subsection case-sensitive — measured in
both halves of the key: `INCLUDE.PATH` resolves, and so does
`INCLUDEIF.gitdir:<p>.PATH`.

**No existing pin changes.** `is_hooks_path_key` (`policy.rs:750-752`) is already
`eq_ignore_ascii_case("core.hookspath")`, and `git -c CORE.HOOKSPATH=/dev/null
status` is measured at `hook_bypass_blocked` today. The guard's whole-key
case-fold is MORE permissive than git for a subsectioned key, which is the safe
direction.

### The design question, stated with what the corpus can now fail on

**The guard must not ask "does this assignment spell `core.hooksPath`" but "can I
establish what `core.hooksPath` will be."** The costing belongs to the plan that
can measure the rule, so it is not answered here. What IS recorded is which shapes
this corpus now turns red:

- **a substring match on `include`** — red at `git -c includepath=…` and
  `git -c notinclude.path=…`, both pinned PERMITTED before and after;
- **a blanket refusal of every `-c`** — red at the permitted half, and
  `git -c user.name="$NAME" commit -m x` is named in the corpus as this axis's
  `ls {git,svn}-repo`;
- **a refusal of a key that cannot be decomposed into a section** — red at
  `git -c a=b status` AND at round 7's entire generative property, which is
  spliced behind `CALLEE_KNOWN_LEADING_PREFIX = "-c a=b"`
  (`tests/envelope_wrapper_class.rs:5197`);
- **a refusal raised in a SECOND PASS over the leading tokens rather than inside
  the same left-to-right walk** — red at one of the two ordering pins, which are
  deliberately pinned at DIFFERENT post-fix identifiers.

And the cost the corpus pins in the other direction: `git -c include.pathx=…`,
which real git IGNORES, is pinned REFUSED. Reading the SECTION and deliberately
not reading the variable covers `[include]`'s single variable and `includeIf`'s
open condition family by construction, and this row is its whole price.

**The residue has NO automated control.** A future git that added a THIRD
indirection section would not be covered and the rule would fail OPEN on it. The
precedence pin cannot observe a section it does not name — it holds only the two
known sections' behaviour, in the reverse direction (a git that stopped honouring
a known include). **Re-audit is the compensating control, and there is no other.**

### The `GIT_CONFIG_NOSYSTEM` cell — a measured DEFEAT with an INERT harm

Found while PLANNING, with the `19-14` provenance caveat: not an audit finding.
**Folded into `T-19-104`'s class, not registered as a new threat ID, and never
called a bypass.**

The defeat is real and measured against real git:

```
GIT_CONFIG_SYSTEM=<file with credential.helper=evil> git config --get credential.helper -> evil
  + GIT_CONFIG_NOSYSTEM=1                                                               -> rc 1, nothing read
```

Guard-side it is permitted: `GIT_CONFIG_NOSYSTEM=1 git push origin
refs/heads/gsd-auto/alpha/w` -> **exit 0**, walk EMPTY.

**The harm is INERT and must not be claimed.** `cred::write_gitconfig`'s own doc
(`cred.rs:253-271`) records that the generated helper-free file is pointed at by
**both** `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM`, so suppressing the system
read removes a deny the global pointer duplicates. Whether it earns an
`ENVELOPE_ENV_KEYS` entry is `19-23`'s design decision; this plan asserts no
post-fix verdict for it and records the guard-side row without pinning it.

### The class confirmed END TO END, reproduced rather than cited

A claim in this codebase about which forms outrank the injection is already known
FALSE — `cred.rs:241-248` says *"the one form that outranks this injection is
`git -c core.hooksPath=… push`"*, and four measured forms outrank it — so this
round does not inherit a behavioural claim it has not reproduced. The bare-remote
fixture was rebuilt and all four legs re-measured, with the hook delivered exactly
as the envelope delivers it and the remote's SHA recorded before and after each:

```
leg 1  plain in-namespace push        rc=1, hook REFUSED   11b417c -> 11b417c  UNMOVED
leg 2  -c include.path=<evil>         rc=0                 11b417c -> 0e482a9  MOVED
leg 3  GIT_CONFIG_PARAMETERS carrier  rc=0                 0e482a9 -> ba3c923  MOVED
leg 4  pre-commit: control rc=1 (refused), carrier rc=0     ba3c923 -> 79a7c23  MOVED
```

**All four legs reproduced. None failed.** They are asserted as a test, not merely
recorded, so a later git that changed this behaviour turns the file red rather
than leaving a stale sentence. `pre-push` is the ONLY carrier of the worktree
credential scan (`hooks.rs:342`, SAFE-05, `T-19-15`/`T-19-16`/`T-19-18`) and the
second carrier `AR-19-03` rests on, so disarming it widens `T-19-86`, `T-19-91`
and `T-19-96` at once.

### The `T-19-86` persisted-alias arm — RECORDED, and `T-19-86` NOT closed

Measured at `d0eb738`, two separately-permitted tool calls the stateless guard
cannot correlate:

```
exit=0  git config alias.p "!git push --force origin HEAD:refs/heads/main"
exit=0  git p
```

**Layer 3 catches the inner push TODAY** — which is `AR-19-03` working — **and
that is precisely what `T-19-103` removes.** So closing `T-19-103` is a
**RESTORATION of layer 3's catch, never a closure of `T-19-86`.** `T-19-86`
remains OPEN at `high` by explicit user scoping decision, its four registered rows
still at exit 0, and recording this fifth arm does not close, narrow or re-scope
it.

### Why the existing `ENVELOPE_ENV_KEYS` drift pin STRUCTURALLY CANNOT SEE `T-19-104`

The pin is sourced from `cred::EnvelopeEnv::with_run_id(build_env_in(…))` — i.e.
from the keys the envelope **SETS or REMOVES** — and every floor it carries is a
floor over the envelope's own entries. Both cells here are keys the envelope
**neither sets nor removes but which DEFEAT ones it does**. **The fix is a SECOND
SOURCE, not a wider filter**, and it is `19-23`'s. The third-omission history is
`GIT_SSH_COMMAND`, then `SSH_AUTH_SOCK`/`SSH_AGENT_PID` (`T-19-82`), then
`GSD_MM_RUN_ID` (`T-19-90`).

### `T-19-106` and `T-19-107` — registered here, not fixed

- **`T-19-106`** (medium) — the callee-grammar drift pin's REACH.
  `git config --comment core.hooksPath /dev/null` measured at **exit 0**, while
  real git answers ``error: unknown option `comment'``. `CONFIG_VALUE_OPTS`
  (`policy.rs:997`) carries a stale entry. Registered; the rule is `19-23`'s.
- **`T-19-107`** (low) — the "ZERO over-refusal cost on git 2.43.0" claim.
  `git -v` measured at **exit 2 `envelope_assertion_failed`** against real git's
  `git version 2.43.0` at rc 0. Registered; the rule is `19-23`'s.

### The `glab --host` forge cell — carried forward UNCHANGED and deliberately not fixed

`glab` is confirmed **NOT INSTALLED** on this machine, so whether it accepts
`--host` as a separate-value global flag is **not confirmed against the callee**.
Audit 7 explicitly declined to upgrade it and this plan declines too. Not claimed
as a live bypass; `FORGE_VALUE_OPTS`, `GH_API_VALUE_OPTS` and
`subcommand_word_indices` are untouched.

### `T-19-17r` — the bookkeeping gap, OUTSTANDING, and the acceptance deliberately NOT made

`19-17-SUMMARY.md` calls it "accepted". Audits 5, 6 and 7 all confirmed the
measurement and both pins at `tests/envelope_literal_decision.rs:1355-1379`, and
all three explicitly declined to make the acceptance. There is still **no
Accepted-Risks-Log row, no `AR-19-13` and no register row**.

**This plan does not make the acceptance either, because accepting a risk is a
human decision and a plan making it would be forging the same signature.** The
next round either adds the log row or drops the word; this plan does neither.

### The byte floors, re-measured with the file's own `production_code`

| File | raw bytes | stripped bytes | floor | ratio |
|---|---|---|---|---|
| `src/envelope/policy.rs` | 307,496 | 68,785 | 40,000 | **22.37%** |
| `src/envelope/hooks.rs` | 99,909 | 33,460 | 20,000 | 33.49% |

**`policy.rs`'s ratio is 22.37%, so the 25% ratio assertion `19-20` deleted would
be RED TODAY** — `19-21`'s own doc additions would have spent it. The absolute
floors are what this file's anti-vacuity control now rests on, they are
load-bearing rather than cosmetic, and `19-23` adds more prose to `policy.rs` than
`19-21` did. Both floors are unchanged by this plan.

### The handoff numbers

| | `passed + failed` | failed | ignored | `envelope_*` binaries | result lines |
|---|---|---|---|---|---|
| before (`d0eb738`) | **1639** | 0 | 13 | 13 | 42 |
| after (this plan) | **1673** | 9 | 13 | **14** | 43 |

`1673 = 1639 + 34`, and 34 is exactly the number of new `#[test]` fns this plan
adds — 28 in `tests/envelope_config_resolution.rs` and 6 in
`tests/envelope_wrapper_class.rs`, counted from `git show`. **A red test RAN, so
red→green leaves the total unchanged and every increase comes ONLY from new fns**;
the identity holds with no residual. All FOURTEEN `envelope_*` binaries ran.
`--test envelope_expansion_slots`, `--test envelope_command_position`,
`--test envelope_literal_decision`, `--test envelope_argv_deletion` and
`--test envelope_callee_grammar` are fully GREEN and unmodified. None of the three
documented flakes fired, and their absence is not evidence they are fixed.

Zero `src/` hunks in all three commits.

## Execution record — plan 19-23 (the rules). NOT an audit finding.

Written by plan 19-23's executor. It appends to the record and edits nothing
before it: no audit table, no Security Audit Trail row, no Accepted Risks Log
row, no Sign-Off, and no earlier appended subsection — including 19-20's,
19-21's, the blocker-row resolution's and 19-22's. Re-measuring and
re-classifying these rows is `/gsd-secure-phase 19`'s job.

### FIRST: `/gsd-secure-phase 19` is NOT cleared by this plan

**`T-19-86` and `T-19-91` both remain OPEN at `high`.** `T-19-96`, `T-19-74`,
`T-19-84`, `T-19-85` and `T-19-61` … `T-19-73` remain open and unaccepted by
explicit user decision. **Only the WRAPPER-OPERAND sub-class of `T-19-60` is
closed**; no unqualified "T-19-60 is closed" appears in this plan's output.

`T-19-103`, `T-19-104`, `T-19-105`, `T-19-106` and `T-19-107` are closed by this
plan **in this plan's own words**, and the re-measurement and re-classification
are left to the gate.

### The rules as implemented

**One.** An assignment the guard cannot BOUND to the key it names makes the
command unresolvable. `scan_leading` stops asking *does this assignment spell
`core.hooksPath`* and asks *can I establish what `core.hooksPath` will be*. The
refusal is raised inside the ONE scan, in the existing `if let Some(assignment)`
block, and returned through the `(usize, Option<GitVerdict>)` channel that
already carried two refusals. No new `ParkReason`, no second reading site, no
classifier change, no arm added to `resolve_program_with_head`, `hooks.rs`
unopened. The rule reads the SECTION — the text before the first `.`, compared
`eq_ignore_ascii_case` — and **deliberately reads neither the subsection nor the
variable**, so `includeIf`'s open condition family and any future variable in
either section are covered by construction.

**Two.** `GIT_CONFIG_PARAMETERS` joins `ENVELOPE_ENV_KEYS`, beside a SECOND pin
source for the class the first pin structurally cannot see.

### The RED set, confirmed STILL RED before any production line moved

All nine names `19-22-SUMMARY.md` records were re-run against `b9d8eca` and were
still failing. Verbatim, from the confirmation run:

```
tests/envelope_config_resolution.rs
  test result: FAILED. 21 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out
tests/envelope_wrapper_class.rs
  test result: FAILED. 38 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

```
AN ENVIRONMENT CARRIER OF CONFIGURATION WAS PERMITTED.
  command : "GIT_CONFIG_PARAMETERS=\"'core.hooksPath=/tmp/nohooks'\" git status"
  got     : exit 0 reason permit(the guard answered nothing)
  left: 0
 right: 2
```

```
`git -c include.path=/tmp/evil.cfg -c core.hooksPath=/dev/null push --force origin main`
must be refused UNDER `envelope_assertion_failed`. … Got: gsd-meta-manager
envelope: REFUSED (reason: hook_bypass_blocked)
```

**Nothing was already green.** Had any row been, the rules would have been rules
nobody had shown were needed, and that was to be reported rather than completed.

| Row (abbreviated) | before | after | walk |
|---|---|---|---|
| `git -c include.path=$F commit -m x` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git -c include.path=$F status` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git -c include.path=$F push origin refs/heads/gsd-auto/alpha/w` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git -c includeIf.gitdir:/tmp/.path=$F status` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git --config-env=include.path=EVILVAR status` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git --config-env include.path=EVILVAR status` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git -c INCLUDE.PATH=$F push origin refs/heads/…` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git -c include.pathx=$F status` | 0 permit | 2 `envelope_assertion_failed` | EMPTY |
| `git -c include.path=$F -c core.hooksPath=/dev/null push --force …` | 2 `hook_bypass_blocked` | 2 `envelope_assertion_failed` | EMPTY |
| `GIT_CONFIG_PARAMETERS=… git commit -m x` | 0 permit | 2 `hook_bypass_blocked` | EMPTY |
| `GIT_CONFIG_PARAMETERS=… git push origin refs/heads/…` (3 spellings) | 0 permit | 2 `hook_bypass_blocked` | EMPTY |
| `echo GIT_CONFIG_PARAMETERS` | 0 permit | 2 `hook_bypass_blocked` | EMPTY |
| the four composition rows (round 6, round 7, wrapper, segment) | 0 permit | 2 `envelope_assertion_failed` | EMPTY |

Every walk was observed by walking a fresh `GSD_MM_ENVELOPE_ROOT` after the row.

### A BLOCKER was hit, reported rather than worked around, and its resolution recorded

`19-22`'s `the_force_push_compositions_are_already_refused_and_are_controls_not_reproducers`
asserted `force_push_blocked` for five `-c include.path=` `--force` compositions.
That row and section 7's ordering pin —
`git -c include.path=$F -c core.hooksPath=/dev/null push --force origin main` at
`envelope_assertion_failed` — **differ only in tokens AFTER the first unbounded
assignment, which `scan_leading` never reads.** No rule raised inside the one
left-to-right scan can produce different identifiers for them, and a rule that
could would have to read past the first unbounded assignment, which the ordering
pin exists to forbid. **The two assertions were unsatisfiable together.**

The executor halted and reported rather than editing an evidence row, which
`19-23`'s prohibitions forbid. The defect is `19-22`'s: the demotion of those five
rows to controls was stated in **four** places and implemented in none —
the control's own comment (*"Their verdict does not move; only their IDENTIFIER
may, and section 9 records that separately without asserting it"*),
`19-22-SUMMARY.md` §"The two rows RECORDED rather than asserted",
`19-22-PLAN-CHECK.md` Check 4 (*"correctly demoted to controls"*) and Check 5
(*"No replacement exception needed or granted"*), and `19-23-PLAN.md:668`
predicting the identifier moves.

**The orchestrator authorized the correction as a named, narrow exception**,
scoped to the identifier constant on those five rows, committed FIRST and on its
own, RED at base with zero `src/` hunks. `tests/envelope_wrapper_class.rs` was not
touched and no other assertion was weakened.

**CARRIER BEFORE VERB was decided explicitly rather than by an edit.** An
unbounded config assignment means the guard cannot establish what configuration
the command will run under, so every downstream classification —
`force_push_blocked` included — describes a command whose behaviour the guard
cannot bound. Refusing at the carrier and saying so is the honest verdict;
reporting `force_push_blocked` would name a specific hazard while the guard is in
fact unable to see the command at all (D-24). That reasoning is written into the
rule's doc, the corrected control and the two derived-row pins.

### The design question, answered, with the three rejected options costed

**(i) Add `include.path` to `is_hooks_path_key`** — REJECTED.
`includeIf.<arbitrary condition>.path` is an open family, so the list is wrong
the moment a condition type is used; and it would attribute the refusal to
`HookBypassBlocked` on a line where the guard established no hooks-path write,
which is D-24's own prohibition.

**(ii) Read the included file and resolve the config at guard time** — REJECTED
on three independent measured grounds: the guard runs synchronously on the
`PreToolUse` critical path where a reproduced 180-240 second hang is why
`push_needs_resolved_dests` exists; the file may not exist at guard time or may
change between guard and exec (TOCTOU); and `includeIf`'s conditions depend on
the repository the command will run in, which a pure argv function does not know.

**(iii) Refuse every `-c`** — REJECTED and pinned red by the permitted half.
`git -c user.name="$NAME" commit -m x`, `git -c core.pager=cat log` and
`git -c a=b status` are all measured PERMITTED and stay so; a control that
refuses ordinary configuration fails into unusability and gets switched off
(AR-19-11).

**(iv) ADOPTED — read the SECTION and read nothing else.** git's config graph is
spliced from elsewhere by exactly one mechanism, identified by the section half
of the key. Reading the section and deliberately reading neither the subsection
nor the variable covers **both** open families by construction.

### The residue, stated plainly and NOT handed to any control

**This is NOT a fifth inversion.** Rounds 3, 5, 6 and 7 each made the guard's
silence a refusal. **This rule cannot, and its SILENCE IS A PERMIT**, because the
complement is unbounded: a fail-closed default over config sections would have to
refuse every section the guard has not enumerated, and users legitimately set
arbitrary ones (AR-19-11). What is available instead is a closed grammatical fact
about git's config graph.

> **A future git that adds a THIRD indirection section is not covered, this rule
> fails OPEN on it, and there is NO automated control over that direction.** The
> pin holds the REVERSE direction — it turns red if the installed git stops
> honouring a section the constant already names — and **it cannot observe a
> section it does not name, because it iterates the entries and an entry that
> does not exist is never probed.** A third indirection section reaches
> `core.hooksPath` silently until a human adds it.

That whole sentence, including its second half, is in `INDIRECTION_SECTIONS`'s
own doc, in the pin's doc, and here.

**PROVENANCE, recorded rather than smoothed over.** An earlier draft of this plan
attributed the residue to the real-git pin — *"the only control over that
direction is the pin"* — and a plan-check caught it as **`T-19-107`'s own failure
mode, false reassurance in a control's own doc, inside the round that registers
`T-19-107`**. A round that made and corrected that mistake should say so rather
than present the corrected text as the only text there ever was.

### Case-insensitivity, answered by measurement in BOTH halves — and no existing pin changed

Measured against `git version 2.43.0` with the exact triplet
`cred::hooks_path_env` emits as the control:

```
control:  GIT_CONFIG_COUNT=1 KEY_0=core.hooksPath VALUE_0=/ENV_WINS   -> /ENV_WINS
+ -c include.path=<file>                                              -> /INCLUDE_WINS
+ -c INCLUDE.PATH=<file>                                              -> /INCLUDE_WINS   <- CASE
+ -c includeIf.gitdir:<p>.path=<file>                                 -> /INCLUDE_WINS
+ -c INCLUDEIF.gitdir:<p>.PATH=<file>                                 -> /INCLUDE_WINS   <- CASE
+ --config-env=include.path=EVILVAR                                   -> /INCLUDE_WINS
+ --config-env include.path=EVILVAR                                   -> /INCLUDE_WINS
+ GIT_CONFIG_PARAMETERS="'core.hooksPath=/PARAM_WINS'"                -> /PARAM_WINS
+ GIT_CONFIG_PARAMETERS="'include.path=<file>'"                       -> /PARAM_INCLUDE_WINS
+ -c include.pathx=<file>                                             -> /ENV_WINS  (git IGNORES it)
+ -c notinclude.path=<file>                                           -> /ENV_WINS
+ -c user.name=x / -c core.pager=cat                                  -> /ENV_WINS
persisted `git config include.path <file>`, alone                     -> /INCLUDE_WINS
persisted `git config include.path <file>`, under injection           -> /ENV_WINS  (INERT)
git -c a=b version         -> git version 2.43.0, rc 0
git -c a=b config --get a  -> error: key does not contain a section: a, rc 1
```

Git folds the SECTION and the VARIABLE and leaves the SUBSECTION case-sensitive,
so the clause compares the section with `eq_ignore_ascii_case`. **No existing pin
changed**: `is_hooks_path_key` was ALREADY `eq_ignore_ascii_case("core.hookspath")`
and was not touched; `git -c CORE.HOOKSPATH=/dev/null status` is still
`hook_bypass_blocked`. One doc sentence was added recording that the guard's
whole-key fold is MORE permissive than git for a subsectioned key — the safe
direction, now stated rather than implicit.

The persisted-`git config` row is why the `git config` subcommand is **not** a
second escape and correctly needs no clause.

### The class reproduced END TO END, this round's own measurement

Not cited from `19-22` — re-measured, because a claim in this codebase about which
forms outrank the injection is already known false. Fixture rebuilt, hook
delivered exactly as the envelope delivers it, remote SHA recorded before and
after each leg:

```
leg 1  plain in-namespace push        rc=1, hook REFUSED   70af3d7 -> 70af3d7  UNMOVED
leg 2  -c include.path=<evil>         rc=0                 70af3d7 -> cea98c6  MOVED
leg 3  GIT_CONFIG_PARAMETERS carrier  rc=0                 cea98c6 -> 18102fc  MOVED
leg 4  pre-commit: control rc=1 (refused), carrier rc=0     18102fc -> a941712  MOVED
```

**All four legs reproduced independently. None failed.**

### The SECOND PIN SOURCE, with the structural reason it exists

`ENVELOPE_ENV_KEYS` has now been wrong FOUR times — `GIT_SSH_COMMAND` (19-11),
`SSH_AUTH_SOCK`/`SSH_AGENT_PID` (`T-19-82`), `GSD_MM_RUN_ID` (`T-19-90`) and
`GIT_CONFIG_PARAMETERS` (`T-19-104`). Its existing drift pin caught the first
three and **structurally cannot see the fourth**: its source is
`cred::EnvelopeEnv::with_run_id(build_env_in(…))` — the keys the envelope SETS or
REMOVES — and every floor it carries is a floor over the envelope's own entries.
A key the envelope neither sets nor removes is not in its source at all. **The fix
is a second SOURCE, not a wider filter.**

`ENVELOPE_ENV_DEFEATING_KEYS` carries each defeating key WITH the envelope key it
defeats, in the DATA. Two measured members:

| Defeating key | Defeats | Measured |
|---|---|---|
| `GIT_CONFIG_PARAMETERS` | the `GIT_CONFIG_COUNT` triplet | control `/ENV_WINS` → `/PARAM_WINS` |
| `GIT_CONFIG_NOSYSTEM` | `GIT_CONFIG_SYSTEM` | `credential.helper` reads `evil`; with the key set, rc 1, nothing read |

**`GIT_CONFIG_NOSYSTEM`'s harm is INERT and it is NEVER called a bypass.**
`cred::write_gitconfig` points BOTH `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM`
at the same helper-free file, so suppressing the system read removes a deny the
global pointer duplicates. **That duplication is now itself under test** — the pin
asserts the two variables carry the SAME path and that the file they name still
resolves no credential helper with the system read suppressed — so a later change
cannot spend the inertness silently.

The pin asserts every defeating key covered in both spellings, every defeated key
covered, MEASURES each defeat against real git with the envelope's own mechanism
as the control, and carries a non-vacuity negative control. It does not skip when
git is absent.

### The `cred.rs` correction — A NAMED, NARROW, DELIBERATE EXCEPTION

The doc at `cred.rs:241-248` claimed:

> *the one form that outranks this injection is `git -c core.hooksPath=… push` —
> which plan 19-02 denies by name at the tool boundary for exactly that reason.*

**Measured, there are FIVE and four were permitted until this round:**

| Form | Resolves to | Closed by |
|---|---|---|
| `git -c core.hooksPath=… push` | command-line precedence | plan 19-02, by name |
| `git -c include.path=<file>` | `/INCLUDE_WINS` | 19-23's confinement clause |
| `git -c includeIf.<cond>.path=<file>` | `/INCLUDE_WINS` | the same clause, by SECTION |
| `git --config-env=include.path=<VAR>` | `/INCLUDE_WINS` | the same clause, second carrier |
| `GIT_CONFIG_PARAMETERS="'core.hooksPath=…'"` | `/PARAM_WINS` | 19-23's `ENVELOPE_ENV_KEYS` entry |

The server-side-branch-protection sentence is KEPT, and a sentence was added that
the closures are client-side and the confinement clause fails OPEN on a future
third indirection section.

**This is the ONLY line of `cred.rs` this round opened.** `git diff --unified=0`
shows ONE hunk at `:244-248` and **zero non-doc lines changed** in the file.
`config_env`, `hooks_path_env`'s body, `write_gitconfig`, `build_env_in` and
`EnvelopeEnv` are untouched. **`T-19-61` … `T-19-73` were NOT taken on.** Leaving a
claim in the code that this round's own evidence contradicts would have been the
`T-19-84` failure mode — a residual disclosure pinned to exist rather than to be
true — which is why the exception was granted, and it is recorded as an exception
rather than presented as ordinary scope.

### The over-refusal cost, measured from BOTH sides

**The one row this round's own rule moves permitted → refused** is
`git -c include.pathx=/tmp/evil.cfg status`: a key real git IGNORES (the control
still prints `/ENV_WINS`), refused because the rule reads the SECTION and
deliberately not the variable. Safe direction. Named here rather than left for
audit 8. The correct response to it is NOT to start reading the variable.

Measured at exit 0 after the rules, all walks EMPTY:
`git -c user.name="$NAME" commit -m x`, `git -c core.pager=cat log`,
`git -c a=b status`, `git -c includepath=/tmp/evil.cfg status`,
`git -c notinclude.path=/tmp/evil.cfg status`, `git --git-dir=/tmp/g status`,
`git -C /tmp status`, `git --no-pager status`.

`git -c a=b push --force origin main` and
`git -c a=b --attr-source HEAD push --force origin main` stay at
`force_push_blocked`. `git -c CORE.HOOKSPATH=/dev/null status` stays at
`hook_bypass_blocked`. `echo GIT_CONFIG_PARAM` stays permitted beside
`echo GIT_CONFIG_PARAMETERS` refused.

**`git -c a=b status` is the DOTLESS fence.** A key with no `.` names no section
and is provably not an indirection, so it is CONFINED and reaches the hooks-path
clause exactly as before. Refusing it would have turned round 7's entire
callee-grammar generative property permanently red behind
`CALLEE_KNOWN_LEADING_PREFIX`.

### `T-19-107` — the correction, recorded BESIDE plan 19-21's record

`19-21-SUMMARY.md` and the appended plan-19-21 record are **NOT edited**. The
correction is recorded here, which is the provenance discipline audit 7 used for
this exact finding.

The claim *"ZERO over-refusal cost on git 2.43.0"* was **false by one measured
row**. `git -v` is accepted by this git — `git -v` prints `git version 2.43.0`
and `git -v XVALUE version` prints it too, so it terminates exactly as
`--version` does — it was in NEITHER grammar constant, it was NOT in the disclosed
UNPROBED set, and it was measured at exit 2 `envelope_assertion_failed` beside
`git --version` at exit 0.

**What is true instead**: the cost was ONE measured row before this round and that
row is removed; the remaining commands moving permitted → refused are ones git
ITSELF rejects. `-v` now sits in `GIT_GLOBAL_SELF_CONTAINED_OPTS`, where the
`>= 8` floor and the two-sided probe cover it, and the probe classified it before
it was written there. `git -v` and `git -v status` are now exit 0.

### `T-19-106`'s two halves

**(a) `CONFIG_VALUE_OPTS` LOST `--comment`**, and the removal is part of the fix
rather than tidying. Real git answers ``error: unknown option `comment'``, so the
option does not exist — yet the guard skipped a word for it. The measured twin:

| Command | before | after |
|---|---|---|
| `git config --comment core.hooksPath /dev/null` | **0** (`/dev/null` read as the key) | 2 `hook_bypass_blocked` |
| `git config core.hooksPath /dev/null` | 2 `hook_bypass_blocked` | 2 `hook_bypass_blocked` |

That is the identical over-consuming mis-index `19-21` removed `--super-prefix`
over, in a second constant, inert only because git rejects the option. A new
two-sided pin covers every remaining entry and **reads git's own classification
rather than inferring one**: `git config <opt>` with no value answers
``option `X' requires a value``, ``switch `X' requires a value``, or
``unknown option `X'``. **No per-entry variant value is needed at all** — the
round-7 pins need variants because their probe must REACH a verb; this one does
not — and that is stated rather than left implicit. Two negative controls:
`--bogus-config-opt` (must not exist) and `--list` (must exist and take no value).

**(b) The four STRUCTURAL arms of `leading_git_option` are pinned**, and two of
them asserted a grammar the installed git contradicts:

| Arm | Stated premise | Measured |
|---|---|---|
| arm 3, `-c<rest>` | *"git's short-option parser accepts `-ckey=value` with no space"* | `git -cuser.name=x version` → `unknown option: -cuser.name=x` |
| deviation-1's arm 4, `-C/tmp` | `-C` with an attached value | `git -C/tmp version` → `unknown option: -C/tmp` |

Both are **inert in the SAFE direction** — they decide the shape of commands git
will not run — and **neither arm's behaviour was changed**. The docs were
corrected to state what git actually does, why the arms are kept (arm 4's removal
regressed `git -C/tmp push --force origin main` from `force_push_blocked` to
`envelope_assertion_failed` and broke a `19-20` pin), and that arm 4's
`GIT_GLOBAL_VALUE_OPTS.contains(&head)` check is a **ONE-ELEMENT test today** —
the constant holds exactly two entries of length two and `-c` is taken by the
earlier arm — so it generalises only if the constant gains another two-character
value-taking spelling. The bundle half (`git -pc user.name=x version` →
`unknown option: -pc`) is pinned too.

**`FORGE_VALUE_OPTS` and `GH_API_VALUE_OPTS` are left UNPINNED, with the reason
stated**: `glab` is confirmed NOT INSTALLED, so a two-sided pin over them cannot
run, and a pin that skips is a fail-open pin. The `glab --host` forge cell is
carried forward **unfixed**, with its unconfirmed-callee caveat intact, and was
not upgraded without evidence — audit 7 explicitly declined to, and so does this.
`subcommand_word_indices` was not touched.

### The spawn compensating control's two repairs, with their measured line numbers

**(a) The marker set was NARROWER than the control it replaces.** It carried
`["Command::new(", "process_group("]` while `tests/spawn_seam_guard.rs:108`
carries three. **`CommandWrap::with_new(` is a spelling this repository actually
uses, at `src/executor/claude.rs:480`**, so a production spawn written that way in
`policy.rs` would have been invisible to BOTH controls — to the global one because
this file is on its `SPAWN_ALLOWLIST`, and to the local one because the marker was
absent. Added, with the self-invalidation hazard named: the marker constant lives
inside `#[cfg(test)] mod tests` and **must not be hoisted above the sentinel**.

A bare `contains` is kept rather than the global control's word-boundary matcher,
and the reason is stated: `calls_marker` exists there to avoid FALSE POSITIVES,
and this control asserts an ABSENCE, so a false positive here fails CLOSED.

**(b) The truncation guard was thin at exactly the seam that failed once this
round.** Measured at `b9d8eca`: `fn scan_leading` at line **374**, 40,000 raw
bytes reached at line **778**, of a production half running to line **4,262** and
**207,358 bytes**, with **one** `#[cfg(test)]` sentinel at line 4,263. **So a
stray `#[cfg(test)]` anywhere after line 778 truncated the control's view with
both positive controls still green.** This round's own incident put such a line at
**~607 — below 778, which is the only reason a floor caught it.**

Three repairs, not one, every figure re-measured after this round's additions
(production half now **4,613 lines / 228,785 bytes**):

1. a floor **PROPORTIONAL** to the file — 180,000 bytes, **78.7%, margin BELOW
   the measurement**, reached at line ~3,622, so a truncating sentinel must now
   land in the last fifth to go unnoticed. A deliberate refactor removing that
   much production logic is a fact to state in a commit message before the number
   moves;
2. a **DEEP anchor**, `fn forbidden_repo_path`, at line ~4,592 of 4,613 — the one
   `fn scan_leading` at 374 could not be, because a shallow anchor is satisfied by
   a view truncated anywhere after it;
3. an **EXACTLY-ONE `#[cfg(test)]` sentinel-count assertion**, so the stripper's
   premise stops being an assumption and becomes a fact under test.

**Neither control was weakened.** The marker set only grew, the truncation guard
only got stricter, `tests/spawn_seam_guard.rs` is UNEDITED, and `policy.rs`'s
`SPAWN_ALLOWLIST` entry is intact with its compensating-control comment.

### The two rows `19-22` could not derive — MEASURED and pinned, with their clauses

| Row | before | after | the clause that produced it |
|---|---|---|---|
| `GIT_CONFIG_NOSYSTEM=1 git push origin refs/heads/gsd-auto/alpha/w` | 0 permit | 2 `hook_bypass_blocked` | Task 2's `ENVELOPE_ENV_KEYS` entry — `tampers_with_envelope_env` now covers the assignment prefix. `T-19-104`'s class, and the identifier says so |
| `git -c include.path=$F push --force origin main` | 2 `force_push_blocked` | 2 `envelope_assertion_failed` | Task 1's confinement clause, raised at the first unbounded assignment before any verb is classified — verdict preserved, identifier moved |

**Neither became PERMITTED.** Had either done so it would have been a finding
about the rules and not a row to pin, because a carrier that stops being read is
the direction `T-19-103` is about.

### The controls that show these are MODELS, not blanket refusals

- the eight ordinary invocations above, all still exit 0;
- the two DISCRIMINATION controls — `git -c includepath=…` and
  `git -c notinclude.path=…` at exit 0 — which turn a `key.contains("include")`
  fix red;
- `echo GIT_CONFIG_PARAM` permitted beside `echo GIT_CONFIG_PARAMETERS` refused;
- the five MECHANISM pins, all green and non-vacuous: round 5's literalness bit
  (`Token.literal` FALSE for `pus?`, TRUE for `-c`, `include.path=…` and
  `status`); round 6's deletion model (`git >/dev/null push --force origin main`
  refused, `git x2>/tmp/o push --force origin main` PERMITTED); `SEPARATORS`
  **byte-identical** with zero hunks in this plan's diff and
  `policy::is_separator(">")` still `false`; round 7's fail-closed grammar
  (`git --attr-source HEAD push --force origin main` at `force_push_blocked`,
  `git --bogus-opt status` at `envelope_assertion_failed`, `git --no-pager status`
  and `git - push --force origin main` at exit 0); and Rule B's severed-head
  geometry.

### The decision region did NOT move, checked mechanically

- `git diff --stat` over `src/` is **exactly TWO files** — `policy.rs` and the one
  `cred.rs` doc block;
- `src/envelope/hooks.rs`, `tests/spawn_seam_guard.rs`, `Cargo.toml` and
  `Cargo.lock` have **no diff at all** (`T-19-SC` holds);
- **no new `ParkReason` variant** was added;
- `first_unreadable_decision_word`, `config_key_operand_index`,
  `subcommand_word_indices`, `scan_gh_api` and `resolve_program_with_head` have no
  behavioural hunk and the post-filter gained no arm;
- the guard's path shells out to nothing: every new `Command` use is inside
  `#[cfg(test)]`, and `the_guards_own_path_shells_out_to_nothing` is green with
  its widened marker set, proportional floor, deep anchor and sentinel count.

### The byte floors, unchanged

`POLICY_MIN_PRODUCTION_BYTES = 40_000` and `HOOKS_MIN_PRODUCTION_BYTES = 20_000`
are at their values in `tests/envelope_wrapper_class.rs`, which this plan did not
touch. **`policy.rs`'s comment ratio was 22.37% before this plan wrote a line**,
so the 25% ratio assertion `19-20` deleted would have been RED at base — the
absolute floors are what the anti-vacuity control rests on, they are independent
of comment volume by construction, and this plan wrote more prose to that file
than `19-21` did. Neither floor was lowered and none was economised around.

### The gate

```
rtk proxy cargo test --no-fail-fast          exit 0
passed + failed = 1680  (1680 passed, 0 failed, 13 ignored, 43 result lines)
19-22's recorded total 1673 + 7 NEW #[test] fns = 1680        identity holds, no residual
```

A red test RAN, so red→green leaves the total unchanged and every increase comes
ONLY from new fns. The seven, counted from `git diff b9d8eca..HEAD`: five in
`policy.rs`'s own test module
(`every_indirection_section_the_guard_names_really_outranks_the_envelopes_own_injection`,
`the_section_helper_reads_the_section_and_neither_the_subsection_nor_the_variable`,
`every_key_that_defeats_the_envelope_is_covered_and_its_defeat_is_measured`,
`every_config_value_opt_really_takes_a_separate_value_on_the_installed_git`,
`the_four_structural_arms_of_leading_git_option_are_pinned_against_the_installed_git`)
and two in `tests/envelope_config_resolution.rs`. **Zero `#[test]` fns removed.**

**`failed` is 0** — none of the three documented flakes fired, and their absence
is not evidence they are fixed. **All FOURTEEN `envelope_*` binaries RAN.**

| Binary | passed | failed | Binary | passed | failed |
|---|---|---|---|---|---|
| `envelope_advisory` | 10 | 0 | `envelope_literal_decision` | 43 | 0 |
| `envelope_argv_deletion` | 20 | 0 | `envelope_pr_cap` | 11 | 0 |
| `envelope_callee_grammar` | 19 | 0 | `envelope_tracer` | 6 | 0 |
| `envelope_command_position` | 18 | 0 | `envelope_wiring` | 14 | 0 |
| `envelope_config_resolution` | **30** | 0 | `envelope_wrapper_bypass` | 13 | 0 |
| `envelope_credential` | 6 | 0 | `envelope_wrapper_class` | **40** | 0 |
| `envelope_expansion_slots` | 32 | 0 | `envelope_hook_refusals` | 7 | 0 |

`cargo build` and `cargo clippy -- -D warnings` both exit 0. `cargo clippy --tests`
is NOT the gate: it fails at base on four pre-existing lints in `src/browser.rs`
and `src/project_creator.rs`, which were not touched.

`git diff --numstat` over `tests/` shows **zero deletions in three of the four
commits**. The exception is the authorized corpus correction, which necessarily
replaced lines; it is the one deviation from that verification item and it is
recorded as such rather than reported as clean.

### The `T-19-86` interaction — a RESTORATION, never a closure

`T-19-86` remains **OPEN at `high`** by explicit user scoping decision. Its four
registered rows are still at exit 0, its pin is green and UNMODIFIED, and the
PERSISTED-ALIAS arm `19-22` recorded is still at exit 0 on both calls —
`git config alias.p "!git push --force origin HEAD:refs/heads/main"` and `git p`,
walks EMPTY, re-measured this round. `env $X push --force origin main` and
`X=git; env $X push --force origin main` are still exit 0.

**Closing `T-19-103` RESTORES layer 3's catch of that arm. That is a restoration
and it is not a closure of `T-19-86`.** Recording the interaction does not close
it.

### `T-19-91`, `T-19-96`, `T-19-74` — untouched

`T-19-91` remains OPEN at `high` with its arms unweakened: `reflog $S`,
`reflog show $S` and `symbolic-ref $S` at exit 0 with no second carrier, and bare
`git push $REF` at exit 0 in the in-namespace configuration — recorded as
permitted rather than "already fails closed". No decision-operand rule was added
and no denylist was extended. `T-19-96` is left exactly as pinned. `T-19-74`'s
core rows are frozen.

### `T-19-17r` — still OUTSTANDING, and this plan did NOT accept it

The bookkeeping gap is recorded as **OUTSTANDING for the fourth round running**.
**No Accepted-Risks-Log row was added, `AR-19-13` was not created, and the word
"accepted" is not applied to `T-19-17r` anywhere in this plan's output.** Audits
5, 6 and 7 all confirmed the measurement and both pins and all three deliberately
declined to make the acceptance, because accepting a risk is a human decision.

### What this record does NOT do

It closes `T-19-103`, `T-19-104`, `T-19-105`, `T-19-106` and `T-19-107` in this
plan's own words and leaves the re-measurement and re-classification to
`/gsd-secure-phase 19`. It makes no acceptance. **It does not clear the gate.**

---

## Threats found by audit 8 (2026-09-03, after plans 19-22 and 19-23)

**Provenance, stated first.** The two subsections above this one were written by
the EXECUTORS of plans 19-22 and 19-23; they are deliberately outside the audit
tables. Audit 8 left them, every earlier appended subsection and every earlier
audit's own tables **byte-identical** — the body below the frontmatter was
checksummed before writing (`sha256 90fbf53e…02e83199` over the 452,080 bytes
below the frontmatter of the 452,972-byte file), and this audit's write adds a
frontmatter block, one Security-Audit-Trail row, one method subsection and
everything from here to the end of the file, deleting nothing. Everything below
is **audit 8's own**, measured against the built binary at `fb43577` with a fresh
`GSD_MM_ENVELOPE_ROOT` per row and the whole envelope root walked afterwards, and
with every claimed bypass confirmed against the **real `git` binary**
(`git version 2.43.0`) in a fixture with a bare upstream.

### The question this audit was set, answered plainly

**With word assembly, word deletion, leading-option grammar, config resolution
and the config-bearing environment all now modelled, is there a further layer
between the guard's decision and what git actually does?**

**Yes, and it is one position over from where round 8 looked.** Round 8 is
genuinely right about the layer it set out to close — every `T-19-103` and
`T-19-104` row re-measured below, every discrimination control and every
over-refusal twin holding — and the confinement clause is the right SHAPE of
rule: it asks whether an assignment can be bounded rather than whether it spells
a name. **But it reads config assignments in ONE region — the leading-option
region `scan_leading` walks — and git resolves configuration from a second place
the guard hands it: a config VALUE the clause itself CONFINES.**

`alias.<name>` is the demonstrated member. Git re-parses an alias body as a
command line **including its leading options**, so an alias whose body begins
`-c include.path=<file>` splices the include at command-line precedence. The key
`alias.q` names the section `alias`, which is not an indirection, so
`config_key_names_an_indirection_section` correctly answers `false` and the
assignment is CONFINED — and the carrier rides inside its value into a position
`scan_leading` never reads.

**The measurement that settles it, made against real git rather than argued.**
Fixture rebuilt from scratch: a local bare remote, a work repo on
`refs/heads/gsd-auto/alpha/w`, and `pre-push`/`pre-commit` hooks delivered
exactly as the envelope delivers them (`GIT_CONFIG_COUNT=1
GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=<hooks dir>`), remote SHA
recorded before and after every leg:

```
control  git config alias.qc '!git push --force origin HEAD:refs/heads/main'; git qc
                                       rc=1, hook REFUSED   ac303dc -> ac303dc  UNMOVED
leg 1    plain in-namespace push       rc=1, hook REFUSED   3b84f30 -> 3b84f30  UNMOVED
leg 2    persisted NON-SHELL alias carrying the include
         git config alias.p '-c include.path=<evil> push origin refs/heads/gsd-auto/alpha/w'
         git p                         rc=0                 3b84f30 -> 1ab87f6  MOVED
leg 3    audit 7's OWN destructive pair, re-run at fb43577
         git config alias.q '!git -c include.path=<evil> push --force origin HEAD:refs/heads/main'
         git q                         rc=0                 ac303dc -> 9f62444  REMOTE `main` REWRITTEN
leg 4    pre-commit: control rc=1 (refused); alias carrying the include commits at rc=0
```

**Audit 7's own destructive demonstration still works, unchanged, at `fb43577`.**
Both guard calls in every leg are **exit 0 with an EMPTY walk**, and the control
one line above each proves the hook is live and refusing.

**And it is not only the include.** `git -c alias.z="-c core.hooksPath=/dev/null
push origin refs/heads/gsd-auto/alpha/w" z` → **exit 0**, walk EMPTY: the
by-name deny that plan 19-02 built for exactly this setting, that `T-19-09`
(high, closed) exists to enforce and that `cred.rs` names as the one form
19-02 closes, is itself reachable when the string sits inside an alias body. Its
paired discriminator is refused on the same line shape —
`git -c core.hooksPath=/dev/null status` → 2 `hook_bypass_blocked` — so this is a
gap in REACH, not in the mechanism.

**So the pattern held for an eighth round, and this time the axis did not move —
the CELL did.** Rounds 1–5 were about how a word is written, round 6 about which
words arrive, round 7 about which arriving word is the verb, round 8 about what
the verb runs under. Round 8 is on the right axis. It read the carrier in the
argv region and in the environment, and the third region — **a value the guard
permits and git later re-parses as a command line** — is the one it did not
reach.

### What audit 8 measured and found INERT, recorded because the question was asked

The mandate named several config-resolution paths the section test does not
reach. Every one was measured against real git 2.43.0 with the envelope's own
injection as the control, and **all of them lose to it**:

```
control:  GIT_CONFIG_COUNT=1 KEY_0=core.hooksPath VALUE_0=/ENV_WINS  -> /ENV_WINS
persisted repo-local  git config include.path <evil>                 -> /ENV_WINS   INERT
persisted repo-local  git config core.hooksPath /LOCAL_WINS          -> /ENV_WINS   INERT
WORKTREE config       git config --worktree core.hooksPath /W        -> /ENV_WINS   INERT
WORKTREE config       git config --worktree include.path <evil>      -> /ENV_WINS   INERT
GLOBAL                git config --global include.path <evil>        -> /ENV_WINS   INERT
GLOBAL                git config --global core.hooksPath /G          -> /ENV_WINS   INERT
GIT_EXEC_PATH=<dir> with a fake `git-push` in it   -> git runs its own BUILTIN; the fake never executes
```

The worktree and global rows are audit 8's own additions — `19-22` and `19-23`
measured only the repo-local one. **`AR-19-04`'s premise therefore holds at every
repository-side precedence level, including the two neither round tested**, and
`git config include.path …` correctly needs no clause. `GIT_DIR`, `GIT_PAGER`,
`GIT_EDITOR`, `GIT_SEQUENCE_EDITOR`, `GIT_TEMPLATE_DIR`, `GIT_ATTR_NOSYSTEM`,
`GIT_PROXY_COMMAND`, `GIT_SSH` and `GIT_ALLOW_PROTOCOL` are all permitted and all
inert for layer 3: each either loses to a key the envelope already sets
(`GIT_SSH` to `GIT_SSH_COMMAND`) or executes a child that **inherits the
injection**, so a `git push --force` inside it still meets the hook.
`git commit --no-verify` and `git commit -n` are permitted and are `T-19-17`'s
registered, tested residual — `pre_push`'s `refuse_swept_paths` is documented and
asserted as the backstop for exactly that.

`credential.helper` and `askpass` resolution were probed: a `-c
credential.helper='!…'` value **does** execute a shell (`HELPER RAN`, measured),
and `git config --global credential.helper '!…'` **does** write into the
generated helper-free file and resolve from it. Both are `AR-19-04`/`T-19-29`
territory — accepted, and audit 8 does not re-open an acceptance. Neither reaches
`core.hooksPath`.

**`include` and `includeIf` are the COMPLETE set of file-splicing sections in git
2.43.0**, confirmed rather than assumed, so `INDIRECTION_SECTIONS` is correct
today and the residue is genuinely about a FUTURE git.

### The new rows

| Threat ID | Category | Component | Severity | Disposition | Finding | Status |
|---|---|---|---|---|---|---|
| **T-19-108** | **Elevation of Privilege** | **the CONFINEMENT clause's REACH — `config_key_names_an_indirection_section` / `scan_leading` (`policy.rs:508`, `:976`) read config assignments in the leading-option region only; a config VALUE the clause CONFINES carries the same indirection into a region the scan never reads** | **high** | **mitigate (unimplemented)** | **Git re-parses an `alias.<name>` body as a command line INCLUDING its leading options, so an alias body beginning `-c include.path=<file>` splices the include at command-line precedence. The key `alias.q` names a section that is not an indirection, so the clause correctly CONFINES the assignment — and the carrier rides inside its value. Measured at `fb43577`, one fresh root per row, walk EMPTY on every one: `git -c alias.q="-c include.path=<evil> push --force origin HEAD:refs/heads/main" q` → **exit 0** on a SINGLE line; `git config alias.p '-c include.path=<evil> push origin refs/heads/gsd-auto/alpha/w'` → **0** and `git p` → **0** in two separately-permitted calls the stateless guard cannot correlate; `git -c alias.z="-c core.hooksPath=/dev/null push origin refs/heads/gsd-auto/alpha/w" z` → **0**, which reaches the by-name deny plan 19-02 built. The paired discriminators are all refused on the same line shapes, which is what makes this a gap in REACH: `git -c include.path=<evil> push --force origin main` → 2 `envelope_assertion_failed`, `git -c core.hooksPath=/dev/null status` → 2 `hook_bypass_blocked`. **Confirmed against the REAL git binary end to end, with the control beside every leg.** Bare remote, `pre-push` and `pre-commit` delivered exactly as the envelope delivers them: the plain in-namespace push is refused and leaves `refs/heads/gsd-auto/alpha/w` at `3b84f30`, the persisted non-shell alias carrying the include **moves it to `1ab87f6`**; the same alias WITHOUT the include is refused by the hook and leaves `main` at `ac303dc`, and **audit 7's exact destructive pair re-run at `fb43577` rewrote `main` to `9f62444`**. `pre-commit` falls the same way — control rc 1, carrier rc 0. **The round's own closure claim is what this falsifies.** `cred.rs:246`'s corrected table lists `git -c include.path=<file>` as "Closed by 19-23's confinement clause"; the `19-22` record, the `19-23` record and `19-23-SUMMARY.md` all state that closing `T-19-103` is a **RESTORATION of layer 3's catch** of `T-19-86`'s persisted-alias arm. Measured, layer 3 does not catch it when the alias body carries a carrier of its own — the restoration did not happen for the arm it is named for. **Registered separately from `T-19-86` rather than folded into it**, because `T-19-86` is about a command line the guard cannot SEE and this is about a carrier the guard DOES see, confines, and hands on: closing `T-19-86` as scoped would not by itself establish what the aliased command runs under, and this row is the direct falsifier of a closure this round claims. **Not the corpus's fault alone**: `CONFIG_RESOLUTION_CLASSES`'s five classes are all argv or environment carriers, `grep -rn "alias\\.[a-z]*=\\"-c\|alias.*include\\.path" src/ tests/` returns **nothing at all**, and the only mention of the shape anywhere in the phase is inside `T-19-103`'s own finding row, as evidence FOR it. | **OPEN — BLOCKING** |
| T-19-109 | Repudiation | `cred::hooks_path_env`'s corrected doc (`cred.rs:246`), and the "RESTORATION of layer 3's catch" claim in the 19-22 record, the 19-23 record and `19-23-SUMMARY.md` | low | mitigate | **`T-19-84`'s failure mode reproduced INSIDE the fix for it.** The round correctly identified that `cred.rs`'s *"the one form that outranks this injection"* was false, corrected it as a named narrow exception, and replaced it with **"FIVE forms outrank this injection, and four of them were PERMITTED until plan 19-23"** plus a five-row table. Measured at `fb43577`, there is at least a **sixth**: an `alias.<name>` value whose body begins `-c include.path=<file>`, which resolves `core.hooksPath` to the included file's value against the same control (`/ENV_WINS` → `/INCLUDE_WINS`) and which moved a bare remote's ref twice in this audit's fixture. The new claim is a **counted completeness claim**, which is strictly harder to keep true than the sentence it replaced, and it was wrong on the day it was written. Same for the three documents' "closing `T-19-103` is a RESTORATION of layer 3's catch" — true for an alias body that is a plain push, false for one carrying a carrier, and stated without the qualifier. **The defect is the CLAIM, not the closure**: the confinement clause does everything it says for the region it reads, and its own residue paragraph is exemplary. The fix is to state the REGION the clause covers rather than to count forms — a table of five is wrong the moment a sixth is found, which is the shape `T-19-84`, `T-19-107` and this row all share. | open — below `high` (non-blocking) |

*Status: open · closed · open — below `high` threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` count toward `threats_open`*

`T-19-109` is open below the `high` threshold and does **not** count toward
`threats_open`. `T-19-108` does.

### Audit 8's bookkeeping, re-derived from audit 7's group rows

| Group | Count | Closed | Open |
|---|---|---|---|
| Everything through audit 7 | 119 | 96 | 23 (4 at `high`) |
| Closed by plans 19-22 / 19-23, re-measured by audit 8 (`T-19-103` as scoped, `T-19-104`, `T-19-107`) | — | +3 | −3 |
| Found by audit 8 (`T-19-108`, `T-19-109`) | 2 | 0 | 2 (1 at `high`) |
| **Total after audit 8** | **121** | **99** | **22 (3 at `high`)** |

The three that count toward `threats_open`: `T-19-86`, `T-19-91`, `T-19-108`.
The nineteen that do not: `T-19-61` … `T-19-73` (13), `T-19-84`, `T-19-85`,
`T-19-96`, `T-19-105`, `T-19-106`, `T-19-109`.

### The closures, re-measured rather than accepted from the summaries

Driven as
`printf '<PreToolUse JSON>' | GSD_MM_ENVELOPE_ROOT=$(mktemp -d) ./target/debug/gsd-meta-manager envelope guard alpha`,
one fresh root per row, the envelope root walked with `os.walk` afterwards, the
walk proved non-blind on every pass by `gh pr create --title x` leaving exactly
one `alpha/pr-ledger.ndjson` line.

* **`T-19-103` — CLOSED AS SCOPED, and the scope is stated rather than implied.**
  All ten `19-22` rows, plus two case-varied spellings neither round pinned:

  ```
  exit=2 [envelope_assertion_failed]  git -c include.path=/tmp/evil.cfg commit -m x
  exit=2 [envelope_assertion_failed]  git -c include.path=/tmp/evil.cfg status
  exit=2 [envelope_assertion_failed]  git -c include.path=… push origin refs/heads/gsd-auto/alpha/w
  exit=2 [envelope_assertion_failed]  git -c includeIf.gitdir:/tmp/.path=… status
  exit=2 [envelope_assertion_failed]  git --config-env=include.path=EVILVAR status
  exit=2 [envelope_assertion_failed]  git --config-env include.path=EVILVAR status
  exit=2 [envelope_assertion_failed]  git -c INCLUDE.PATH=… push origin refs/heads/gsd-auto/alpha/w
  exit=2 [envelope_assertion_failed]  git -c "include.path=/tmp/evil.cfg" status
  exit=2 [envelope_assertion_failed]  git -c include.pathx=… status      (disclosed cost)
  exit=2 [envelope_assertion_failed]  git -c INCLUDEIF.gitdir:/tmp/.PATH=… status   (audit 8's own)
  exit=2 [envelope_assertion_failed]  git -c includeif.gitdir:/tmp/.path=… commit -m x (audit 8's own)
  ```

  Every walk EMPTY. **Both ordering pins hold at their DELIBERATELY DIFFERENT
  identifiers** — `git -c include.path=… -c core.hooksPath=/dev/null push --force
  origin main` → 2 `envelope_assertion_failed` and the reverse spelling → 2
  `hook_bypass_blocked` — which is the mechanical proof that the refusal is
  raised at the first unbounded assignment inside the ONE left-to-right walk and
  not in a second pass. **It is closed for the argv and `--config-env`
  carriers. It is NOT closed for a carrier inside a config value the clause
  confines — that is `T-19-108`.**
* **`T-19-104` — CLOSED, all six rows plus the second pin source verified
  non-vacuous.**

  ```
  exit=2 [hook_bypass_blocked]  GIT_CONFIG_PARAMETERS="'core.hooksPath=…'" git commit -m x
  exit=2 [hook_bypass_blocked]  GIT_CONFIG_PARAMETERS="'core.hooksPath=…'" git push origin refs/heads/gsd-auto/alpha/w
  exit=2 [hook_bypass_blocked]  export GIT_CONFIG_PARAMETERS=…; git push …
  exit=2 [hook_bypass_blocked]  env GIT_CONFIG_PARAMETERS=… git push …
  exit=2 [hook_bypass_blocked]  GIT_CONFIG_PARAMETERS="'include.path=…'" git push …
  exit=2 [hook_bypass_blocked]  echo GIT_CONFIG_PARAMETERS        (disclosed bare-word cost)
  exit=0 (permit)               echo GIT_CONFIG_PARAM             (the discriminating twin)
  exit=2 [hook_bypass_blocked]  GIT_CONFIG_NOSYSTEM=1 git push origin refs/heads/gsd-auto/alpha/w
  ```

  Every walk EMPTY. The `GIT_CONFIG_COUNT` discriminators still refuse in all
  three environment spellings, and `env -u GIT_CONFIG_COUNT git push …` and
  `unset GIT_CONFIG_COUNT; git push …` are both refused too.
* **`GIT_CONFIG_NOSYSTEM`'s INERTNESS — verified, and so is the pin that would
  catch the duplication changing.** The pin
  (`every_key_that_defeats_the_envelope_is_covered_and_its_defeat_is_measured`)
  does two things audit 8 read line by line and then reproduced independently: it
  asserts `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` are `Some` and **equal**,
  with a failure message that says in terms *"that duplication is the ONLY reason
  `GIT_CONFIG_NOSYSTEM`'s measured defeat has no demonstrated harm — if they
  diverge, the inertness has been spent"*; and it then runs real `git config
  --get credential.helper` against that file **with `GIT_CONFIG_NOSYSTEM=1`
  set**, requiring a non-zero status and empty stdout. Reproduced by hand: a
  helper-free file pointed at by both variables resolves nothing with the system
  read suppressed. **The inertness is real and the pin genuinely catches the
  duplication changing.** It does not skip when git is absent.
* **`T-19-106` — half closed, and NOT closed.** See its own row below.
* **`T-19-107` — CLOSED.** `git -v` → **exit 0**, `git -v status` → **exit 0**,
  `git --version` → 0, and `-v` is in `GIT_GLOBAL_SELF_CONTAINED_OPTS` where the
  `>= 8` floor and the two-sided probe cover it. Real git 2.43.0 accepts `git -v`
  at rc 0, so the removal was required.
* **`T-19-106` — the `--comment` half CLOSED and the probe verified strictly
  stronger; TWO of its five named items remain, and one reason for that is
  measurably false.** `git config --comment core.hooksPath /dev/null` → **2
  `hook_bypass_blocked`** (was exit 0 with `/dev/null` read as the key), beside
  `git config core.hooksPath /dev/null` → 2, and every other `git config`
  spelling audit 8 tried is caught: `--file=/tmp/x`, `--replace-all`, `--add`,
  `--global`, `--type=path` and `--` all reach `hook_bypass_blocked` on the
  hooks-path key. The four structural arms are pinned. **But**: `gh` 2.45.0 **is
  installed on this machine**, `GH_API_VALUE_OPTS` is a pure `gh api` constant,
  and audit 8 ran the two-sided probe over all seventeen entries — every one
  answers `flag needs an argument` and `--bogus-opt` answers `unknown flag`, so
  **that pin can run today** and the stated reason for its absence (*"`glab` is
  confirmed NOT INSTALLED, so a two-sided pin over them cannot run"*) is a fact
  about a different callee. And `FORGE_VALUE_OPTS` carries **`--hostname`**,
  which `gh pr create` answers `unknown flag: --hostname` for — the identical
  stale-entry shape the round removed `--comment` and `--super-prefix` over, in a
  third constant, inert only because `gh` rejects it. `T-19-106` therefore stays
  **open at `medium`**.
* **`T-19-105` — ADDRESSED for the config-resolution axis, and OPEN for the ninth
  consecutive round.** The fourth named axis exists and is real:
  `CONFIG_RESOLUTION_CLASSES` (`tests/envelope_wrapper_class.rs:6038`) names five
  degenerate-proofed classes with `MIN_CONFIG_RESOLUTION_CLASSES = 5`
  (`:6399`), beside byte-identical `UNREADABLE_CLASSES`, `DELETION_CLASSES` and
  `CALLEE_GRAMMAR_CLASSES`; and the corpus demonstrably failed on its own class
  before the rule — `e7f3358` and `de573cb` are `tests/`-only commits with **zero
  `src/` hunks** carrying the RED set, and `19-23`'s three `src/` commits turned
  them green. **But all five classes are carriers on ARGV or in the
  ENVIRONMENT** — "a command-line carrier of a config indirection", "a
  command-line carrier of a confined assignment", "an environment carrier of
  configuration", "a case-varied spelling of an indirection", "an option-carrier
  delivery of an indirection" — and none is a carrier inside a config VALUE.
  Verified mechanically rather than read:
  `grep -rn "alias\.[a-z]*=\"-c\|alias.*include\.path" src/ tests/` returns
  **nothing at all**. The corpus is therefore structurally incapable of
  generating, and so of failing on, `T-19-108`. Same finding as `T-19-76`,
  `T-19-83`, `T-19-89`, `T-19-95`, `T-19-99`, `T-19-101` and `T-19-105` itself —
  and for the second round running the gap is not one axis further out but one
  REGION over on the same axis.

### The over-refusal cost, re-measured from BOTH sides

Every row the mandate named, and several it did not, at **exit 0 with an EMPTY
walk**:

```
exit=0  git -c user.name=x commit -m x        exit=0  git -c core.pager=cat log
exit=0  git -c a=b status                     exit=0  git -c a=b version
exit=0  git -c includepath=/tmp/evil.cfg status
exit=0  git -c notinclude.path=/tmp/evil.cfg status
exit=0  git --git-dir=/tmp/g status           exit=0  git -C /tmp status
exit=0  git --no-pager status                 exit=0  git -v   /  git -v status
exit=0  git push -o ci.skip origin refs/heads/gsd-auto/alpha/w
exit=0  git push --push-option=ci.skip origin refs/heads/gsd-auto/alpha/w
exit=0  git push --recurse-submodules on-demand origin refs/heads/gsd-auto/alpha/w
```

and the refusals that must not have moved:

```
exit=2 [hook_bypass_blocked]     git -c CORE.HOOKSPATH=/dev/null status
exit=2 [force_push_blocked]      git -c a=b push --force origin main
exit=2 [force_push_blocked]      git -c a=b --attr-source HEAD push --force origin main
exit=2 [push_outside_namespace]  git push --signed no origin refs/heads/gsd-auto/alpha/w
exit=2 [push_outside_namespace]  git push --repo origin refs/heads/main
```

**`git -c a=b version` at exit 0 is the row that matters most** — round 7's entire
callee-grammar generative property is spliced behind
`CALLEE_KNOWN_LEADING_PREFIX = "-c a=b"`, so the dotless fence is load-bearing for
a whole earlier round's evidence, and it holds. `git -c includepath=…` and
`git -c notinclude.path=…` at exit 0 turn a `key.contains("include")` fix red from
both sides.

### The mechanism pins, verified non-dead

Rounds 4, 5, 6 and 7 are all still load-bearing and all still RIGHT about the
lines round 8 refuses:

```
exit=2 [envelope_assertion_failed]  git pus? --force origin main            <- round 5's literalness bit
exit=2 [force_push_blocked]         git >/dev/null push --force origin main <- round 6's deletion model
exit=0                              git x2>/tmp/o push --force origin main  <- round 6's over-deletion control
exit=2 [force_push_blocked]         git --attr-source HEAD push --force origin main  <- round 7
exit=2 [envelope_assertion_failed]  git --bogus-opt status                  <- round 7's inversion
exit=0                              git - push --force origin main          <- round 7's pinned control
exit=2 [force_push_blocked]         git -- push --force origin main         <- its twin
```

`SEPARATORS` is **byte-identical** — `const SEPARATORS: &[&str] = &[";", "&&",
"||", "|", "&", "\n", "(", ")", "{", "}"]` at `policy.rs:1786`, and `git log -L`
over that line returns exactly **one** commit in the whole phase (`84a9b05`, plan
19-05) — and `is_separator` is a bare `SEPARATORS.contains` (`policy.rs:3031-3033`),
so `is_separator(">") == false` by construction.

**The byte floors are load-bearing and they are all still met.** Re-derived at
`fb43577` by splitting each file at its `#[cfg(test)]` sentinel: `policy.rs`
production half **228,100 bytes** (raw file 372,920), `hooks.rs` production half
**74,357** (raw 99,753) — both far above `POLICY_MIN_PRODUCTION_BYTES = 40_000`
and `HOOKS_MIN_PRODUCTION_BYTES = 20_000` at their unchanged values in
`tests/envelope_wrapper_class.rs:831` and `:838`. The three round-8 repairs are
present and re-measured: the **proportional** floor `production.len() >= 180_000`
(`policy.rs:6643`) is 78.9% of the current production half, the **deep anchor**
`fn forbidden_repo_path` (`:6653`) sits at line **4,592** of a 4,613-line
production half, and `policy.rs` contains **exactly ONE** `#[cfg(test)]` line
(4,614) with that count itself asserted at `:6665`.

### The known-open set, as audit 8 verified it

- **`T-19-86` (high, OPEN, unaccepted, deferred by explicit user scoping
  decision).** All four rows re-measured at `fb43577`, fresh root each, all at
  **exit 0** with EMPTY walks: `git submodule foreach git push --force origin
  main`, `git rebase -x "git push --force origin main" HEAD~3`, `git bisect run
  sh -c "git push --force origin main"`, `git -c alias.p='!git push --force
  origin main' p`. The persisted-alias arm reproduces too — `git config alias.p
  "!git push --force origin HEAD:refs/heads/main"` → 0 and `git p` → 0. Counts
  toward `threats_open`. **It is now WIDER than the record credits**: the
  layer-3 catch three documents attribute to closing `T-19-103` is not there when
  the alias body carries a carrier of its own, which is `T-19-108`. Closing
  `T-19-103` was, for the plain-bodied arm, a genuine restoration; for the
  carrier-bearing arm it never happened.
- **`T-19-91` (high, OPEN, three arms).** `S=refs/heads/main; git reflog $S`,
  `git reflog show $S` and `git symbolic-ref $S` all reproduce at **exit 0** with
  no second carrier; `git symbolic-ref HEAD $R` → 2 `force_push_blocked` and
  `git push origin $REF` → 2 `push_outside_namespace` still fail closed.
  Unweakened by round 8. Counts toward `threats_open`.
- **`T-19-96` (medium, OPEN, registered, not fixed).** `git push --forc? origin
  refs/heads/gsd-auto/alpha/w` → **exit 0**, its literal twin
  `git push --force origin refs/heads/gsd-auto/alpha/w` → 2 `force_push_blocked`.
  Unchanged.
- **`T-19-74` (medium, closed/accepted — AR-19-10).** Core rows frozen and
  re-measured permitted: `env $X push --force origin main` → **0**,
  `X=git; env $X push --force origin main` → **0**. `T-19-84` remains open and
  unaccepted.
- **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85` — carried forward untouched**,
  open and unaccepted at their original severities, all below `high`.
  `git diff --stat c21c13f..fb43577 -- src/` is **exactly two files**, and the
  `cred.rs` half is the named narrow exception: `git diff --unified=0` over it,
  filtered for lines that are not `///`, returns **NOTHING** — **zero non-doc
  lines changed**, exactly as claimed. `hooks.rs`, `mod.rs`, `advisory.rs`,
  `scan.rs` and `config.rs` were not opened, so `T-19-61` … `T-19-73` cannot have
  moved.
- **The `glab --host` forge cell — reproduced, and its caveat is unchanged.**
  `glab --host gitlab.com mr create --title x` → exit 0 with **ZERO** ledger
  lines; `glab --hostname …` → one line. `command -v glab` finds **nothing** —
  glab is confirmed NOT installed — so audit 8 does **not** upgrade it either.
  `gh` re-swept clean: `gh pr create --title x`, `gh --repo o/r pr create --title
  x`, `gh api repos/o/r/pulls -f title=x` and `gh api --hostname h
  repos/o/r/pulls -f title=x` each leave exactly one ledger line.
- **`T-19-SC` — still holds.** Neither `Cargo.toml` nor `Cargo.lock` appears in
  any commit between `c21c13f` and `fb43577`.
- **`T-19-17r` — still OUTSTANDING, and audit 8 does not resolve it either.**
  The Accepted Risks Log runs `AR-19-01` … `AR-19-12`; there is **no `AR-19-13`
  row and no register row**, for the fourth audit running. Plans `19-22` and
  `19-23` both correctly declined to make the acceptance and both avoided
  applying the word. **Accepting a risk is a human decision and this audit does
  not make it.** The next round either adds the log row or drops the word from
  `19-17-SUMMARY.md`.

### The fail-open residue — the admission is complete, and the residual is acceptable

**Every place the mandate asked about carries it, and nothing claims the pin
covers it.** `INDIRECTION_SECTIONS`'s own doc (`policy.rs:873-888`) says the
constant *"is a recognition of a closed grammatical fact, NOT a fail-closed
default. Its SILENCE IS A PERMIT"* and that a third indirection section *"is not
covered, and this rule fails OPEN on it, and there is NO automated control over
that direction"*. The **pin's own doc** (`policy.rs:7387-7400`) opens by stating
the direction it holds **before anything else** and then says in terms *"It is
therefore NOT a control over the fail-open residue … Saying otherwise here would
be false reassurance in a control's own doc — `T-19-107`'s failure mode,
committed inside the round that registers `T-19-107`."* A third statement sits at
`policy.rs:380`. The `19-23` record and `19-23-SUMMARY.md` both carry it, both
quote the second half, and both record that an earlier draft attributed the
residue to the pin and that a plan-check caught it. **Audit 8 found no claim
anywhere that the pin covers this direction.**

**Judgement: the residual is acceptable and properly bounded**, with one gap in
the bounding. Acceptable, because the alternative was costed by measurement and
is worse: a fail-closed default over config sections would refuse
`git -c user.name="$NAME" commit -m x`, and `AR-19-11` already records what
happens to a control that refuses ordinary work. Bounded, because the residue is
a single closed proposition about one external dependency — *does a future git
add a third section that splices a file at the directive's precedence* — and
audit 8 confirmed against git 2.43.0 that `include` and `includeIf` are the
complete set today, so the constant is correct rather than merely current. **The
gap**: the compensating control is "a human reads a future git's release notes",
and nothing schedules it. There is no `AR-` row for it, no revisit condition of
the kind `AR-19-03` carries at `policy.rs:226-241`, and no assertion that goes red
when the installed git's version changes. Recorded as an observation on the
control rather than registered as a threat, because the residue itself is
correctly disclosed and correctly unclaimed.

### The four execution-time judgements, assessed independently

1. **The blocker halted mid-plan and resolved by orchestrator decision — the
   HALT was right, the SEMANTIC is right, and the correction was recorded
   honestly.** Verified four ways. *The unsatisfiability is real*: the five
   `--force` compositions and section 7's ordering pin differ only in tokens
   after the first unbounded assignment, and `scan_leading` returns at that
   assignment (`policy.rs:508-514`), so no rule inside the one left-to-right walk
   can give them different identifiers — and a rule that could would have to read
   past it, which the ordering pin exists to forbid. *The demotion really was
   stated in four places and implemented in none*: audit 8 read all four texts —
   the control's own comment, `19-22-SUMMARY.md` §"The two rows RECORDED rather
   than asserted", `19-22-PLAN-CHECK.md` Checks 4 and 5, and `19-23-PLAN.md:668`.
   *The correction is narrow*: `a8b8632` touches **one file**, `tests/` only, 53
   insertions and 6 deletions, and the non-comment diff is exactly one identifier
   constant (`REASON_FORCE_PUSH_BLOCKED` → `REASON_ENVELOPE_ASSERTION_FAILED`)
   plus its message; `tests/envelope_wrapper_class.rs` was not opened and no other
   assertion moved. *No security property moved*: the VERDICT on those rows is
   exit 2 with an empty walk before and after — only the ATTRIBUTION changed.
   **CARRIER BEFORE VERB is the correct semantic.** An unbounded config
   assignment means the guard cannot establish what configuration the command
   runs under, so `force_push_blocked` would name a specific hazard on a line the
   guard admits it cannot see — precisely D-24's prohibition, and the same
   reasoning `19-21` applied to `--super-prefix` one round earlier. The executor
   halting rather than editing an evidence row is the behaviour this phase has
   been building toward for eight rounds, and the deviation is recorded as a
   deviation in `19-23-SUMMARY.md` rather than reported as clean.
2. **`ENVELOPE_ENV_DEFEATING_KEYS` inside `#[cfg(test)] mod tests` — the
   reasoning is CORRECT and the stripper is genuinely protected.** `policy.rs`
   contains **exactly one** `#[cfg(test)]` line, at 4,614, so a second one placed
   beside `ENVELOPE_ENV_KEYS` (around line 3,100) really would have truncated the
   stripper's view at that point — and the same commit's own
   sentinel-count assertion (`policy.rs:6665`) now forbids exactly that, which is
   why the placement and the assertion belong to one change rather than two. The
   constant is test-only data with no production consumer, so `mod tests` is its
   natural home and `GIT_GLOBAL_UNPROBED_OPTS` (`:6496`) is the standing
   precedent. **The protection is three-layered and audit 8 measured all three**:
   the exactly-one sentinel count, the proportional floor `>= 180_000` over a
   228,100-byte production half (78.9%, margin BELOW the measurement), and the
   deep anchor `fn forbidden_repo_path` at line 4,592 of 4,613 — the one
   `fn scan_leading` at 374 could not be, because a shallow anchor is satisfied by
   a view truncated anywhere after it. A stray sentinel must now land in the last
   fifth of the file to go unnoticed. Nothing was weakened.
3. **The `CONFIG_VALUE_OPTS` probe redesign — the replacement is two-sided and
   strictly stronger, and the abandoned shape really did read backwards.**
   Verified by reading the probe: it classifies with git's own words —
   ``option `X' requires a value`` / ``switch `X' requires a value`` / ``unknown
   option `X'`` — and it carries **two negative controls that pin the classifier
   itself**: `--bogus-config-opt` must come back unknown-and-not-requires-value,
   and `--list` must come back existing-and-not-requires-value. That second
   control is the one that matters: without it the pin would pass on a probe that
   reported "requires a value" for everything, which is the vacuity the first
   shape could not exclude. The abandoned shape's defect is real — `--blob` and
   `--default` are both in the constant, and both make git name the value in its
   output for reasons that are the OPPOSITE of the property being asserted. And
   the probe needs no per-entry variant because it never has to reach a verb,
   which is stated in the code rather than left implicit. Right redesign, and
   right to redesign mid-task rather than ship the weaker shape.
4. **`FORGE_VALUE_OPTS` and `GH_API_VALUE_OPTS` left unpinned — the PRINCIPLE is
   right and an absent pin IS better than a skipping one; but the reason was
   applied to a constant it does not fit, and `T-19-106` is therefore not
   closed.** The principle is correct and audit 8 endorses it without
   reservation: a pin that silently skips when its callee is absent is fail-open,
   it reports green on a machine that measured nothing, and this phase has been
   punished six times for exactly that shape of false green. `glab` is confirmed
   NOT installed (`command -v glab` finds nothing), so `FORGE_VALUE_OPTS`'s
   `--host`/`--hostname` question genuinely cannot be settled here and neither
   audit 7 nor `19-22` nor `19-23` was willing to upgrade it without a callee —
   correct, and audit 8 does the same. **But `GH_API_VALUE_OPTS` is not a glab
   constant.** It is seventeen `gh api` flags, `gh` 2.45.0 **is** installed on
   this machine, and audit 8 ran the two-sided probe over every entry in about a
   second: all seventeen answer `flag needs an argument` and `--bogus-opt`
   answers `unknown flag`. The constant is CORRECT today — there is no live
   defect — but the pin that would keep it correct can run and was not written,
   for a reason that is a fact about a different tool. And `FORGE_VALUE_OPTS`
   carries `--hostname`, which `gh pr create` rejects outright, which is the
   third instance of the `--super-prefix` / `--comment` stale-entry shape and is
   inert only because `gh` rejects it. **Verdict: the call is half right, and the
   half that is wrong is a stated reason rather than a decision.**

### Gates observed

`rtk proxy cargo test --no-fail-fast` redirected to a file and counted with
**`rtk proxy grep`** (a plain `grep` is rewritten by the RTK hook, which strips
the `test result:` lines a count is read from — D-34): **1679 passed, 1 failed,
13 ignored** over **43** result lines, so `passed + failed = 1680`, matching
`19-23-SUMMARY.md` exactly. **All FOURTEEN `envelope_*` binaries ran** —
`advisory` 10, `argv_deletion` 20, `callee_grammar` 19, `command_position` 18,
**`config_resolution` 30**, `credential` 6, `expansion_slots` 32,
`hook_refusals` 7, `literal_decision` 43, `pr_cap` 11, `tracer` 6, `wiring` 14,
`wrapper_bypass` 13, **`wrapper_class` 40** — every one at **zero failures**, and
the per-binary counts match the SUMMARY's table row for row. So round 8's own
evidence file executed.

**The one failure is a documented flake and it is out of scope.**
`driver_reattach::a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step`
failed with *"exactly one project has a run to observe: left 0, right 1"* and
**passed on an immediate re-run** (`3 passed; 0 failed`). It is one of the two
`driver_reattach` flakes the mandate names. Audit 8 records that it FIRED this
time rather than reporting the run clean — the earlier rounds' "none fired" is a
weaker statement than it reads, and one firing is not evidence of a new defect
any more than an absence is evidence of a fix. `cargo clippy --tests -- -D
warnings` was already failing at the base on four pre-existing lints in
`src/browser.rs` and `src/project_creator.rs` and is out of scope.

**Provenance discipline.** Plans 19-13 … 19-23's appended subsections, the
plan-19-21 blocker-row resolution record and every earlier audit's own tables are
left **byte-identical**; audit 8 checksummed the whole body below the frontmatter
before writing. Audit 8's corrections to statements made in those subsections —
the "FIVE forms outrank this injection" table and the "RESTORATION of layer 3's
catch" claim — are recorded as audit-8 findings BESIDE them rather than as edits
to them.

---

## Audit 8 — what the round-8 control can and cannot fail on

### The principle rounds 3 through 7 established still holds

A decision region must be derived from the same scan the classifier runs, and
there is one walk. `19-23` added the third answer **inside** `scan_leading` and
returned it through the `(usize, Option<GitVerdict>)` channel that already
carried two refusals, so no second reading site was created: `git diff --stat
c21c13f..fb43577 -- src/` is two files, one of them a doc-only hunk; `hooks.rs`
was not opened; no `ParkReason` variant was added; and
`first_unreadable_decision_word`, `config_key_operand_index`,
`subcommand_word_indices`, `scan_gh_api` and `resolve_program_with_head` have no
behavioural hunk. `SEPARATORS` is byte-identical and `is_separator(">")` is
`false`. The two ordering pins at deliberately different identifiers are the
mechanical proof that the new clause is raised inside the one left-to-right walk.

### Where the boundary now is, in one paragraph

**Rounds 5, 6, 7 and 8 together decide four questions and audit 8 finds all four
answered.** A decision word must be provably LITERAL; the words the guard
classifies must be exactly the words the program receives in the same order; the
word the guard calls the verb must be the word git calls the verb; and an
assignment the guard cannot BOUND to the key it names makes the command
unresolvable. **What none of them asks is what happens to a value the guard
decided was safe.** The confinement clause reads config assignments in the
leading-option region and `ENVELOPE_ENV_KEYS` reads them in the environment; git
resolves configuration from a third place, which is a config VALUE the guard
itself confined and handed on:

- **a config value git re-parses as a command line with its own leading
  options** — `alias.<name>`, measured end to end twice, and the route through
  which the by-name `core.hooksPath` deny is itself reachable (`T-19-108`);
- **a whole command line handed to a governed program as data** — `T-19-86`,
  unchanged, and now measurably WIDER than the record credits, because the
  layer-3 catch attributed to `T-19-103`'s closure is absent for a
  carrier-bearing alias body;
- **a classifier arm's own operand or flag outside the decision region** —
  `T-19-91` and `T-19-96`, both registered, both still permitted;
- **a counted completeness claim in a control's own doc** — `T-19-109`, the
  eighth-round instance of `T-19-84`'s shape.

**The one-sentence version for the next round.** Eight rounds have modelled how a
command line becomes an argv, which word in it is the verb, and — as of round 8 —
what that verb runs under **in the two regions the guard reads**; none has asked
what git does with a value the guard read, confined, and let through.

### Suggested closure, in order — (d) FIRST, for the eighth round running

1. **(d) — widen the corpus BEFORE certifying anything.** `CONFIG_RESOLUTION_CLASSES`
   is the right axis and it is one class short. Add a **sixth** class beside its
   five — *a carrier delivered inside a config VALUE the guard confines* — with
   an alphabet of alias bodies (`-c include.path=<f> <verb>`,
   `!git -c include.path=<f> <verb>`, `-c core.hooksPath=<p> <verb>`) in both the
   `-c alias.<n>=` and the persisted `git config alias.<n>` deliveries, and let
   the `MIN_CONFIG_RESOLUTION_CLASSES` floor carry it. **Listed first for the
   eighth round running**, and for the eighth round running the previous audit's
   recommendation was right about its own cell while the next gap sat in a cell
   it did not name.
2. **(a) — `T-19-108`, and the durable shape is a REGION rule, not a longer list
   of sections.** The clause already asks the right question; it asks it of the
   wrong extent. Two shapes are available and both should be costed by
   measurement rather than argued: **refuse an `alias.*` assignment whose value
   the guard cannot classify as a bounded command**, reusing the classifier the
   guard already runs on the outer line; or **treat an `alias.*` assignment as an
   unbounded assignment** in the same sense the confinement clause already means,
   which is the smaller change and the one whose over-refusal cost is a single
   legible family. The paired cost must be pinned from both sides exactly as this
   round's was: `git -c a=b status`, `git -c user.name="$NAME" commit -m x` and
   every ordinary `-c` in the suite must keep reaching their present verdicts,
   and round 7's generative property must stay green behind
   `CALLEE_KNOWN_LEADING_PREFIX`.
3. **(b) — `T-19-109`**: replace the counted table in `cred.rs` with a statement
   of the REGION each closure covers. A table of five is wrong the moment a sixth
   is found, and this is the second time that sentence has been corrected; and
   remove or qualify "RESTORATION of layer 3's catch" wherever it appears without
   the carrier-bearing-body qualifier.
4. **(c) — `T-19-106`'s remainder**: write the `GH_API_VALUE_OPTS` two-sided pin,
   which the installed `gh` supports today, and either remove `--hostname` from
   `FORGE_VALUE_OPTS` or record why a `gh pr create`-rejected entry is kept.
5. **(e)** — then `T-19-86`, `T-19-91` and `T-19-96`, which remain three arms of
   one shape: a classifier arm answering `Allow` on an operand, flag or payload
   outside the decision region.

Then, and separately: add the `AR-19-13` row for `T-19-17r` or stop calling it
accepted; give the `INDIRECTION_SECTIONS` residue a revisit condition of the kind
`AR-19-03` carries, so "a human re-audits on a git upgrade" is scheduled by
something rather than remembered; and get a machine with `glab` installed to
settle the `--host` cell against its callee.

---

## Audit 8 Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log — twelve, unchanged by
      round 8; audit 8 accepts nothing new
- [x] Every closure re-measured against the built binary at `fb43577` with a
      fresh envelope root per row and the root walked afterwards
- [x] Every new finding confirmed against the **REAL `git` binary**
      (`git version 2.43.0`) in a rebuilt bare-remote fixture, with a CONTROL
      beside every leg — including **audit 7's own destructive command pair
      re-run at `fb43577`, which still rewrote the bare remote's `main`**
- [x] The envelope's own delivery mechanism used as the control in every
      precedence measurement (`GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n`, the triplet
      `cred::hooks_path_env` emits), not a stand-in
- [x] The repository-side precedence paths the mandate named — repo-local,
      `--worktree` and GLOBAL, persisted and included — measured and found INERT
- [x] `GIT_CONFIG_NOSYSTEM`'s inertness re-derived AND its new pin read line by
      line: the same-path assertion and the helper-free-under-suppression
      assertion both present, neither skipping
- [x] The PR-cap walk proved non-blind by a positive control on every pass
- [x] `SEPARATORS` byte-identical (one commit in the phase has ever touched that
      line) and `is_separator(">") == false`; rounds 4, 5, 6 and 7's mechanism
      pins re-measured non-dead
- [x] The byte floors re-derived: `policy.rs` production half 228,100 / 40,000
      and 180,000; `hooks.rs` 74,357 / 20,000; exactly ONE `#[cfg(test)]`
      sentinel, with the count itself asserted
- [x] `cred.rs`'s named narrow exception verified: **zero non-doc lines changed**
- [x] Plans 19-13 … 19-23's appended subsections and the blocker-row resolution
      record left byte-identical, verified by checksum before writing
- [x] Gate observed: `rtk proxy cargo test --no-fail-fast` → **1679 passed, 1
      failed, 13 ignored**, `passed + failed = 1680`; **all FOURTEEN**
      `envelope_*` binaries ran, all at zero failures; the one failure is the
      documented `driver_reattach` flake and it passed on re-run
- [ ] `threats_open: 0` confirmed — **3 open at `high`: `T-19-86`, `T-19-91`,
      `T-19-108`**
- [ ] `status: verified` set in frontmatter

**Approval: blocked, 2026-09-03 (audit 8).**

**Not accepted here.** `T-19-108` is a high-severity, empirically confirmed
bypass of layer 3 — the layer `AR-19-03` explicitly rests on and the only carrier
of the worktree credential scan — reached on a single command line the guard
permits, with the credential intact, and demonstrated by a real push that rewrote
a bare remote's `main`. `T-19-86` and `T-19-91` remain open at `high` by scoping
decision and by round discipline respectively. Accepting any of the three is a
human decision and this audit does not make it.

**Progress is real, and the round did what it said — with one qualification it
did not make about itself.** `19-22` wrote a fourth named axis that demonstrably
failed on its own class before any production line moved, and it found FIRST that
a `T-19-103` reproducer must be built on a layer-2-permitted base, which is the
insight that stopped the round certifying five controls as reproducers. `19-23`
closed `T-19-103` and `T-19-104` by changing the QUESTION rather than lengthening
a list — *can I bound this assignment to the key it names* — gave
`ENVELOPE_ENV_KEYS` a second pin SOURCE for a class its first pin structurally
could not see, removed a third stale over-consuming entry, corrected a false claim
in `cred.rs` as a named narrow exception with zero non-doc lines changed, and
stated its own fail-open residue in five places while explicitly refusing to hand
it to a control that cannot cover it. Its executor halted on an unsatisfiable
evidence row rather than editing an assertion, and the resulting semantic —
carrier before verb — is the right one. **The confinement clause is the right
shape of rule and audit 8 says so plainly.** The remaining risk is **not** the
disclosed residual set. It is that the clause reads config assignments in the
region `scan_leading` walks and in the environment, and git reads configuration
from a third place: a value the guard confined and let through. `alias.<name>` is
the demonstrated member, `core.hooksPath` is reachable through it by name, and
audit 7's own destructive demonstration — the one this round is built to
close — still rewrites a bare remote's `main` at `fb43577`. Eight rounds have
modelled what reaches git. The next one has to model what git does with what the
guard already approved.

---

## Execution record — plan 19-24 (the corpus, RED). NOT an audit finding.

**Attribution.** This is a record made by plan 19-24 while planning and executing
round 9's CORPUS. It is not an audit finding, it amends no audit table, and it is
appended after the plan-19-23 record without editing anything that precedes it.
Audit 8 checksummed the body below the frontmatter before writing; that provenance
is this file's value, and a plan writing into an audit's own tables would forge it.
**No audit table, the Security Audit Trail, the Accepted Risks Log, the Sign-Offs
and every earlier appended subsection are untouched.**

**This plan closes NOTHING and clears NO gate.** `T-19-105`, `T-19-106`, `T-19-108`
and `T-19-109` are all OPEN at its end; the rules are `19-25`'s. **`T-19-86` and
`T-19-91` remain OPEN at `high`, so `/gsd-secure-phase 19` is NOT cleared by this
plan, by `19-25`, or by the two together.** Only the WRAPPER-OPERAND sub-class of
`T-19-60` is closed. Re-measuring and re-classifying these rows is that command's
job rather than a plan's.

### The invariant this round is about, stated once

**A value the guard read, CONFINED and let through is still an input to git's
configuration.**

Round 8 closed the question below it and audit 8 verified every one of its rows:
an assignment the guard cannot BOUND to the key it names makes the command
unresolvable. **That is the right SHAPE of rule** — it asks whether an assignment
can be bounded, not whether it spells a name. But the guard reads config
assignments in TWO regions — the leading-option region `scan_leading` walks, and
the environment — while git resolves configuration from a THIRD: a value the guard
itself confined. **So this round is one REGION over on the SAME axis, not a ninth
spelling of the same class and not a fifth axis.**

### FINDING 1, STATED FIRST — git's shell-alias rule is a ONE-BYTE fact, and a blanket `alias.*` refusal is UNDISCHARGEABLE

Two evidence files `19-25` may not edit pin a `!`-bodied alias PERMITTED as a
registered `T-19-86` row:

```
tests/envelope_command_position.rs:550       permits("git -c alias.p='!git push --force origin main' p")
tests/envelope_config_resolution.rs:1539-43  permits("git config alias.p \"!git push --force origin HEAD:refs/heads/main\"")
```

A rule refusing every `alias.*` assignment turns **both permanently red**. The
carve-out is git's own documented rule, measured in NINE spellings against
`git version 2.43.0` with the envelope's own injection as the control:

| spelling | git resolves | kind |
|---|---|---|
| control, no carrier | `/ENV_WINS` | — |
| `-c alias.a='-c include.path=<f> config --get core.hooksPath' a` | `/INCLUDE_WINS` | **K1, in-process** |
| `-c alias.b='!git config --get core.hooksPath' b` | `/ENV_WINS` | **K2, shell child inherits** |
| `-c alias.g='config --get core.hooksPath !x' g` (`!` not first) | `/ENV_WINS` | in-process |
| `-c alias.d=' !git …' d` (SPACE first) | `expansion of alias 'd' failed; '' is not a git command` | refuses to expand |
| `-c alias.t='<TAB>!git …' t` | `expansion of alias 't' failed; '' is not a git command` | refuses to expand |
| `-c alias.q='"!git -c include.path=<f> …"' q` (QUOTED body) | `expansion of alias 'q' failed; '!git -c include.path=<f> …' is not a git command` | **in-process** |
| `-c alias.e='' e` (EMPTY body) | `expansion of alias 'e' failed` | refuses to expand |
| `-c alias.o='-c include.path=<f>' o` (option only, no verb) | `fatal: empty alias for o` | refuses |

**The QUOTED-body row is the one that shows reading ONE BYTE is not a loophole.**
Its first byte is `"`, so a first-byte rule REFUSES it — and that refusal is
CORRECT: git's error names the whole dequoted string as not a *git* command, i.e.
its own `split_cmdline` handled it in-process and it was never given to a shell.
The quoting that would smuggle a `!` past a naive check also takes the body out of
the shell path entirely.

**No spelling was found in which the first byte IS `!` and git nonetheless
re-parses in-process.** The whole rule leans on that; a counterexample is a FINDING
to report, not a row to add, and `tests/envelope_reparsed_value.rs` carries that as
a standing assertion.

**This is `19-22`'s `-c a=b` seam and `19-18`'s `{v}>` blocker one REGION over,
caught at PLAN time and fenced MECHANICALLY** in `tests/envelope_wrapper_class.rs`
rather than left to care.

### Every measured guard row, with its walk

Driven against the built binary at this plan's base commit, one fresh
`GSD_MM_ENVELOPE_ROOT` per row, the whole envelope directory WALKED afterwards.
**The walk's non-blindness is proved in the same run**: `gh pr create --title x` →
exit 0 with exactly ONE `alpha/pr-ledger.ndjson` line.

**The `-c`-delivery `T-19-108` rows** — all exit 0, walk EMPTY, all asserted at exit
2 `envelope_assertion_failed` after:

```
git -c alias.q="-c include.path=/tmp/evil.cfg push --force origin HEAD:refs/heads/main" q
git -c alias.z="-c core.hooksPath=/dev/null push origin refs/heads/gsd-auto/alpha/w" z
git -c alias.q="-c include.path=/tmp/evil.cfg status" q
git -c alias.q="-c include.path=/tmp/evil.cfg commit -m x" q
git -c ALIAS.q="-c include.path=/tmp/evil.cfg status" q
git --config-env=alias.q=EVILBODY status
```

**The PERSISTED-delivery rows** — all exit 0, walk EMPTY, all asserted at exit 2
`envelope_assertion_failed` after: `git config alias.p '<non-shell body>'` and its
`--global`, `--worktree`, `--file`, `--add` and `--replace-all` spellings, plus
`git config alias.z '-c core.hooksPath=/dev/null status'`.

**The paired discriminators** — asserted UNCHANGED; these are what make this a gap
in REACH and not in MECHANISM:

```
exit=2 [envelope_assertion_failed]  git -c include.path=/tmp/evil.cfg push --force origin main
exit=2 [hook_bypass_blocked]        git -c core.hooksPath=/dev/null status
exit=2 [hook_bypass_blocked]        git config core.hooksPath /dev/null
```

**The DISCRIMINATION controls** — measured exit 0 and asserted exit 0 AFTER. A rule
written as `key.starts_with("alias")` turns the first red; `key.contains("alias")`
turns the second red; only a SECTION comparison keeps both green:
`git -c aliasx.q="…" q`, `git -c notalias.q="…" q`. Beside them the dotless fence
`git -c a=b status` / `git -c a=b version`, behind which
`CALLEE_KNOWN_LEADING_PREFIX` splices round 7's whole generative property.

**The DISCLOSED OVER-REFUSAL family, every row beside its permitted twin** — the
round's whole cost, disclosed by the corpus that produced it rather than found by
audit 9. All four measured exit 0 today, all asserted exit 2 after; all four twins
asserted exit 0 before and after:

| refused after `19-25` (a DEFINITION) | permitted twin (does the same work) |
|---|---|
| `git -c alias.st=status st` | `git status` |
| `git -c alias.lg="log --oneline" lg` | `git log --oneline` |
| `git -c alias.co=checkout co` | `git checkout` |
| `git config alias.co checkout` | run the command the alias would have run |

**Ordinary aliases do NOT keep working, and saying so plainly is the point.** What
does NOT move is INVOCATION: `git p`, `git co`, `git st`, `git lg`, `git z` and
`git q` are asserted exit 0 before and after, because the guard is stateless and
argv-only and cannot see an alias it did not watch being defined. **Defining a
non-shell alias is refused; using one is not.**

**The two ORDERING rows**, both exit 2 `hook_bypass_blocked` today, pinned at
deliberately DIFFERENT post-fix identifiers so a second-pass implementation lands
red:

```
alias FIRST : -c alias.q="…" -c core.hooksPath=/dev/null push --force origin main -> envelope_assertion_failed  (RED, identifier moves)
hooks FIRST : -c core.hooksPath=/dev/null -c alias.q="…" push --force origin main -> hook_bypass_blocked        (UNCHANGED)
```

**The two rows RECORDED rather than asserted** — measured, printed, and NOT written
as assertions, because this plan measures PRE-fix and cannot catch a wrong POST-fix
expectation. `19-22` asserted such a row against its own comment, its own SUMMARY
and its own plan-check, and it halted `19-23` mid-plan.

- `git --config-env alias.q=BODYVAR status` (SEPARATE-WORD spelling) — exit 0, walk
  EMPTY. The value half is an environment variable NAME, so whether `19-25` can
  read a first byte at all is a design decision. Real git DOES resolve it
  (`/INCLUDE_WINS`), so the harm is real; only the identifier is undeliverable.
- `git -c alias.q status` (no `=` at all) — exit 0, walk EMPTY. `config_key_of`
  returns the whole token, so there is NO value half. Real git resolves the control
  value (`/ENV_WINS`), i.e. **no harm on git's side**.

**The `T-19-86` rows** — all exit 0, asserted UNCHANGED, recorded WIDER, NOT closed:
`git submodule foreach git push --force origin main`,
`git rebase -x "git push --force origin main" HEAD~3`,
`git bisect run sh -c "git push --force origin main"`,
`git -c alias.p='!git push --force origin main' p`, and the persisted-alias arm.

### FINDING 2 — the K1/K2/K3 enumeration, MEASURED rather than assumed

- **K1 — re-parsed as a GIT command line, IN-PROCESS, including its leading
  options.** `alias.<name>` with a non-`!` body. **Measurement says it is the ONLY
  member**, and it is the whole of `T-19-108`. Plan-check swept all 29 enumerated
  K2 keys plus `include.path` / `includeIf` / `core.hooksPath`; only the two
  already-closed keys came back non-inert.
- **K2 — re-parsed as a SHELL command line, run as a CHILD that INHERITS the
  injection.** Measured INERT for layer 3, and **the MECHANISM is recorded rather
  than the verdict alone**: the child's own environment still carries
  `GIT_CONFIG_COUNT=1` / `KEY_0=core.hooksPath` / `VALUE_0=/ENV_WINS`. FOUR
  representatives measured with the child's environment DUMPED — a `!` alias body,
  `diff.external`, `credential.helper` (via `git credential fill`, which needs
  stdin to invoke the child at all) and `filter.<n>.clean` — every one printing
  `CHILD_ENV COUNT=1 KEY_0=core.hooksPath VALUE_0=/ENV_WINS` and
  `CHILD_RESOLVES /ENV_WINS`. **No K2 member was found that does not inherit**; one
  would have been a FINDING rather than a row.
- **K3 — re-parsed as a FILE PATH spliced at the directive's precedence.**
  `include.path`, `includeIf.<cond>.path`. Closed by `19-23`.

### FINDING 3 — the PERSISTENCE ASYMMETRY with `include.path`

Audit 8's INERT include rows are carried forward as ESTABLISHED and were also
re-confirmed here rather than cited. Under the envelope's own config posture —
`GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` both pointed at a generated helper-free
file, the `core.hooksPath` triplet injected:

```
git config --global alias.g '-c include.path=<f> config --get core.hooksPath'; git g -> /INCLUDE_WINS   LIVE
git config          alias.l '-c include.path=<f> config --get core.hooksPath'; git l -> /INCLUDE_WINS   LIVE
git config          include.path <f>                                                  -> /ENV_WINS      INERT
git config --global include.path <f>                                                  -> /ENV_WINS      INERT
```

**An include must WIN a precedence contest and loses; an alias only has to EXIST.**
Both persistence levels are live, including the one the envelope itself controls.
**A clause for `git config include.path` would be INERT and must NOT be added** —
that is the likeliest way `19-25` wastes a rule.

### FINDING 4 — DEPTH and QUOTING, so `19-25` costs option (a) against measurement

```
depth 2: -c alias.d2='-c "alias.inner=-c include.path=<f> config --get core.hooksPath" inner' -> /INCLUDE_WINS
depth 3: -c alias.d3='-c "alias.d2=-c \"alias.inner=…\" inner" d2'                            -> /INCLUDE_WINS
dequote: -c alias.m='config --get "core.hooksPath"' m                                         -> /ENV_WINS (RESOLVES)
control: git config --get '"core.hooksPath"'                                                  -> error: invalid key
```

**Depth 3 is the row that matters**: the recursion is not a depth-2 curiosity, so
any stated depth bound in a recursive rule is a residue an attacker reaches by
adding one more nesting level. And git splits an alias body with its OWN
`split_cmdline` rules, not the shell's — the dequoting row resolves where the
literally-quoted key is an `invalid key` error.

**A note on measurement discipline.** The first depth-2 probe run while executing
reported an expansion failure. That was an error in the PROBE — the inner carrier
was unquoted inside the body, so git's own whitespace splitting broke it apart —
not a contradiction of the plan. Re-run with the inner carrier quoted, both depths
reproduce at `/INCLUDE_WINS`. Recorded because a probe error that looks like a
finding is exactly the shape that gets asserted by mistake.

### FINDING 5 — the class confirmed END TO END, with a CONTROL beside every leg

The bare-remote fixture was REBUILT and every leg re-measured independently rather
than cited; claims in this codebase about what outranks the injection have been
wrong twice. Local bare upstream, offline, `core.hooksPath` delivered exactly as
`cred::hooks_path_env` emits it, under the envelope's own config posture:

| leg | command | result | ref before → after |
|---|---|---|---|
| 1 CONTROL | plain in-namespace push | REFUSED by `pre-push` | `0084939` → `0084939` (unmoved) |
| 2 CONTROL | persisted NON-SHELL alias **without** the include | REFUSED by `pre-push` | `0084939` → `0084939` (unmoved) |
| 3 CARRIER | `git config alias.p '-c include.path=<evil> push …'` then `git p` | COMPLETED | `0084939` → **`4a77396`** |
| 4 CARRIER | `-c alias.q='-c include.path=<evil> push …' q`, ONE line | COMPLETED | `4a77396` → **`d833ba0`** |
| 5 CONTROL | `commit` | REFUSED by `pre-commit` | HEAD unmoved |
| 5 CARRIER | `-c include.path=<evil> commit` | COMPLETED | HEAD moved |
| **T-19-86** | `!`-bodied pair, **NOT CLOSED** | COMPLETED | `d833ba0` → **`9687d94`** |

**Leg 2 is the sharpest control in the fixture**: the same alias mechanism with no
carrier is still caught by the hook, which is what isolates the CARRIER from the
ALIAS. Without it the harm would be mis-attributed as "aliases evade hooks". Every
leg reproduced; none failed to reproduce.

### The design question `19-25` must answer, stated with what this corpus can fail on

**Is the fix (a) to scan a confined value that git will re-parse as a command line,
applying the same clause recursively with a stated depth bound, or (b) to treat any
assignment whose key names a re-parsed value as unbounded and refuse?** **The
costing is `19-25`'s; the discrimination is this plan's.** This corpus turns RED on:

- **a prefix or substring match on `alias`** — red at `-c aliasx.q=…` and
  `-c notalias.q=…`, both pinned exit 0 before and after;
- **a blanket `alias.*` refusal including `!` bodies** — red at both `T-19-86` rows,
  and mechanically red at the SHELL-ALIAS FENCE in
  `tests/envelope_wrapper_class.rs`, which names both pinning files and line
  numbers in its failure message;
- **a refusal of a key it cannot decompose into a section** — red at `-c a=b`, and
  through `CALLEE_KNOWN_LEADING_PREFIX` at round 7's entire generative property;
- **a rule written only inside `scan_leading`** — red at all five persisted rows,
  because `config` is the verb and that region never sees the assignment;
- **a rule raised in a SECOND PASS rather than in the one left-to-right walk** — red
  at the two ordering pins, which are deliberately at different identifiers;
- **a blanket refusal of everything spelled `-c`** — red at the invariance arm and
  at the seven-row permitted half.

And in the other direction it pins the disclosed cost: four alias DEFINITIONS
refused, each beside a permitted twin, and six INVOCATIONS unchanged.

### `T-19-108` closes only AS SCOPED — audit 7's `!`-bodied pair STILL WORKS

The rule `19-25` writes closes the **non-shell** alias body: audit 8's leg 1, its
leg 2 (the persisted non-shell form) and its `alias.z` variant that reaches the
by-name `core.hooksPath` deny. **It does NOT close audit 7's leg 3** —
`git config alias.q '!git -c include.path=<evil> push --force origin
HEAD:refs/heads/main'` then `git q` — because a `!` body is a whole command line
handed to a governed program as DATA, which is `T-19-86`: open at `high`, out of
scope by explicit user decision, and pinned PERMITTED in two files this round may
not edit. **Measured after `19-23` landed, that pair still moved the bare remote's
ref (`d833ba0` → `9687d94`), and this corpus asserts that it does** so a scoped
closure cannot silently take `T-19-86` on. This is audit 8's own boundary
paragraph, which lists the re-parsed config value and the command-line-as-data as
two SEPARATE bullets.

### `T-19-86` — RECORDED MEASURABLY WIDER, and NOT closed

Three documents attribute a layer-3 catch to the closure of `T-19-103`: the
`pre-push` hook fires when the alias body runs, so the inner push is caught.
**That catch is ABSENT when the alias body carries a carrier of its own** — the `!`
child inherits the injection and then applies its own command-line carrier *on top
of it inside that child*. Recorded, not closed, narrowed or re-scoped; no remedy
added; named FIRST in the SUMMARY's "what remains uncovered".

### `T-19-109` — registered, correction is `19-25`'s

`cred.rs:246-260`'s counted "FIVE forms" table and the "RESTORATION of layer 3's
catch" claim in the plan-19-22 record, the plan-19-23 record and `19-23-SUMMARY.md`
are incomplete by a sixth measured form. **The correction is recorded BESIDE those
subsections rather than as an edit to them**, which is the provenance discipline
audit 8 used for this exact shape. **This plan carries zero `src/` hunks and does
not correct `cred.rs`.**

### `T-19-106` — the `deferred-items.md` MIS-CLOSURE, corrected

`deferred-items.md` marked it CLOSED at round 8; audit 8 re-opened it at `medium`
because `GH_API_VALUE_OPTS` is a pure `gh api` constant, **`gh` 2.45.0 IS
installed**, and the stated reason for leaving it unpinned is a fact about a
different callee. The entry is CORRECTED to OPEN in `deferred-items.md`; the rule
is `19-25`'s.

Re-measured with an **ENDPOINT-LESS** probe (`gh api <opt>`), so **no row touches
the network**. All seventeen entries — `-X`, `--method`, `-f`, `--raw-field`, `-F`,
`--field`, `-H`, `--header`, `-q`, `--jq`, `-t`, `--template`, `--input`,
`--hostname`, `--cache`, `-p`, `--preview` — answer `flag needs an argument`.
Two negative controls: `gh api --bogus-opt` → `unknown flag: --bogus-opt`, and
**`gh api --paginate` → `accepts 1 arg(s), received 0`** — recorded as the measured
string rather than a paraphrase, because it is a POSITIONAL error and not a
flag-level one, which is exactly what makes it the right control: the flag was
ACCEPTED and consumed NO value. **The endpoint-bearing form `gh api repos/o/r
--paginate` makes a REAL HTTP REQUEST**, so a pin written that way would be
non-hermetic and would fail open on a machine without network or auth.

**And a correction to audit 8's own suggestion, measured.** Removing `--hostname`
from `FORGE_VALUE_OPTS` would be a **REGRESSION in the under-counting direction**:
`glab --hostname gitlab.com mr create --title x` leaves exactly ONE ledger line
today, while `glab --host …` — an option NOT in that constant — leaves ZERO.
**The disposition is "record why", not "remove".**

### The fail-open residue's missing REVISIT CONDITION

Recorded as audit 8 recorded it: the admission is complete and correctly unclaimed
in five places, the residual is acceptable and properly bounded, and the single gap
is that its only control is a human reading a future git's release notes and
**nothing schedules that**. `19-25` adds the revisit condition and a version
witness. **That witness is a SCHEDULE and not a CONTROL** — it says WHEN to look and
cannot say WHAT changed; it does not observe a new indirection or re-parsed section
appearing, so the residues stay uncovered by any automated control and audit 8's
judgement of them is unchanged. **This plan invents no acceptance and adds no `AR-`
row.**

### `policy.rs:6644`'s stale proportional-floor comment — DOCUMENTATION DRIFT

The comment cites "228,785 / 78.7%"; re-measured here, `policy.rs` is raw 372,920 /
production **228,101**, so 180,000 is **78.9%**. The floor itself is correct and
load-bearing; only the arithmetic in its own comment has drifted. **This plan
carries zero `src/` hunks and may not edit `policy.rs` at all**, so it is recorded
here and in `deferred-items.md` and left for a later round.

### The `glab --host` forge cell — carried forward UNFIXED

`command -v glab` was re-run at this plan's base commit and found nothing.
**`glab` is confirmed NOT INSTALLED**, so whether it accepts `--host` as a
separate-value global flag is unconfirmed against the callee and it is NOT claimed
as a live bypass. Audits 7 and 8 both explicitly declined to upgrade it; this plan
declines too.

### `T-19-17r` — the bookkeeping gap, still OUTSTANDING, deliberately NOT resolved

`19-17-SUMMARY.md` calls it "accepted". Audits 5, 6, 7 and 8 all confirmed the
measurement and both pins, and **all four deliberately declined to make the
acceptance**. The Accepted Risks Log runs `AR-19-01` … `AR-19-12`; there is still
no `AR-19-13` row and no register row. **This plan does not make the acceptance,
because accepting a risk is a human decision and four audits running have
deliberately left it unmade**, and the word "accepted" is not applied to
`T-19-17r` anywhere in this plan's artifacts. The next round either adds the log
row or drops the word; this plan does neither.

### The byte floors and the anti-vacuity stripper's three protections

Re-measured at this plan's base commit by splitting each file at its
`#[cfg(test)]` sentinel:

- `policy.rs` raw **372,920** / production **228,101**, against
  `POLICY_MIN_PRODUCTION_BYTES = 40_000` and against the proportional floor
  `production.len() >= 180_000` (78.9%);
- `hooks.rs` raw **99,753** / production **74,358**, against
  `HOOKS_MIN_PRODUCTION_BYTES = 20_000`;
- the deep anchor `fn forbidden_repo_path` at production line **4,592** of
  **4,613**;
- exactly **ONE** `#[cfg(test)]` line, at **4,614**, with the count itself asserted.

All three protections keep their present strictness and both byte floors keep their
values. `SEPARATORS` is byte-identical and `policy::is_separator(">")` is `false`.

### Gate

`rtk proxy cargo test --no-fail-fast`, counts read with `rtk proxy grep` over a
redirected log. **`passed + failed` = 1710** against the baseline of **1680**.

**The arithmetic is stated and CHECKED rather than assumed.** A red test RAN, so
red→green leaves the total unchanged and every increase comes ONLY from new
`#[test]` fns. Counted from `git show`: **29** new fns in
`tests/envelope_reparsed_value.rs` and **1** in `tests/envelope_wrapper_class.rs` =
**30**. `1680 + 30 = 1710`, which is what was observed. No disagreement.

**All FIFTEEN `envelope_*` binaries RAN** — the fourteen audit 8 observed, each at
its own recorded count, plus `envelope_reparsed_value` at 29.

**8 failures, every one a RED name this plan created**, which is `19-25`'s handoff
contract. **None of the three documented flakes fired in this run**; absence is not
evidence they are fixed, and one firing would not have been evidence of a new
defect. `cargo build` and `cargo clippy -- -D warnings` exit 0
(`cargo clippy --tests` is NOT the gate and still fails at base on four
pre-existing lints in `src/browser.rs` and `src/project_creator.rs`, which this
plan does not fix).

**Every commit shows ZERO `src/` hunks**, which is the evidence this split exists to
produce: the corpus was capable of failing before any production line moved.

## Execution record — plan 19-25 (the rules). NOT an audit finding.

**This subsection is appended by plan 19-25's executor. It edits nothing that
precedes it — no audit table, no Security Audit Trail row, no Accepted Risks Log
row, no Sign-Off, and no earlier appended subsection, including the plan-19-21
blocker-row resolution record and the plan-19-22, 19-23 and 19-24 records. It
re-measures and re-classifies nothing in an audit's tables; that is
`/gsd-secure-phase 19`'s job.**

### STATED FIRST

**`/gsd-secure-phase 19` is NOT cleared by this plan.** `T-19-86` remains **OPEN
at `high`** — recorded by audit 8 as measurably WIDER than the register credits —
and `T-19-91` remains **OPEN at `high`** with its arms unweakened. `T-19-96` is
registered open, `T-19-74`'s core rows are frozen, and `T-19-61` … `T-19-73`,
`T-19-84` and `T-19-85` are open and unaccepted. **Only the WRAPPER-OPERAND
sub-class of `T-19-60` is closed**; no unqualified "T-19-60 is closed" appears in
this plan's output.

**`T-19-108` closes only AS SCOPED, and audit 7's `!`-bodied destructive pair
STILL WORKS after this plan.**

```
git config alias.q '!git -c include.path=<evil> push --force origin HEAD:refs/heads/main'
git q
```

That pair — which rewrote a bare remote's `main` from `ac303dc` to `9f62444` at
`fb43577`, and which `19-24` re-measured moving the ref again (`d833ba0` →
`9687d94`) after `19-23` landed — is a whole command line handed to a governed
program as DATA, which is **`T-19-86`**: out of scope by explicit user decision,
with both its rows pinned PERMITTED in files this plan may not edit. This plan
does not close, narrow or re-scope it. It is asserted still working, mechanically,
by
`tests/envelope_reparsed_value.rs::audit_7s_shell_bodied_destructive_pair_still_works_after_this_round_and_is_not_closed`
and by the third arm of the `REPARSED_COMMAND_SECTIONS` real-git pin, which pins
that direction as MEASURED rather than describing it.

### `19-24`'s RED set — confirmed STILL RED before any production line moved

Re-run against the unmodified tree at `57d715b`, before the first fix commit.
**Verbatim:**

```
tests/envelope_reparsed_value.rs
failures:
    after_19_25_a_persisted_alias_carrying_an_indirection_is_refused_in_every_spelling
    after_19_25_a_reparsed_alias_value_carrying_an_indirection_is_refused
    after_19_25_a_reparsed_alias_value_reaching_the_by_name_hooks_deny_is_refused
    after_19_25_defining_an_ordinary_non_shell_alias_is_refused_and_its_twin_is_not
    after_19_25_the_case_varied_and_second_carrier_spellings_are_refused_too
    after_19_25_the_scan_order_decides_which_clause_names_a_line_carrying_both
    after_19_25_the_three_non_shell_boundary_spellings_are_refused

test result: FAILED. 22 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out

tests/envelope_wrapper_class.rs
failures:
    a_carrier_inside_a_confined_config_value_fails_closed_in_both_deliveries

test result: FAILED. 40 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
```

**Eight RED, matching `19-24-SUMMARY.md`'s recorded handoff set exactly. No row
expected RED came up green**, so the rules below are rules the corpus had already
shown were needed.

| RED name | before | after | reason id | clause that produced it |
|---|---|---|---|---|
| `…a_reparsed_alias_value_carrying_an_indirection_is_refused` | exit 0 | exit 2 | `envelope_assertion_failed` | region 1 re-parse clause |
| `…a_reparsed_alias_value_reaching_the_by_name_hooks_deny_is_refused` | exit 0 | exit 2 | `envelope_assertion_failed` | region 1 re-parse clause |
| `…the_case_varied_and_second_carrier_spellings_are_refused_too` | exit 0 | exit 2 | `envelope_assertion_failed` | region 1, `eq_ignore_ascii_case` section + `--config-env` unreadable-value arm |
| `…the_three_non_shell_boundary_spellings_are_refused` | exit 0 | exit 2 | `envelope_assertion_failed` | region 1, git's one-byte rule |
| `…the_scan_order_decides_which_clause_names_a_line_carrying_both` | exit 2 `hook_bypass_blocked` | exit 2 | `envelope_assertion_failed` (alias first) / `hook_bypass_blocked` (hooks first) | region 1, ordered before `is_hooks_path_key` inside the ONE left-to-right walk |
| `…a_persisted_alias_carrying_an_indirection_is_refused_in_every_spelling` | exit 0 | exit 2 | `envelope_assertion_failed` | **region 2** |
| `…defining_an_ordinary_non_shell_alias_is_refused_and_its_twin_is_not` | exit 0 | exit 2 | `envelope_assertion_failed` | regions 1 **and** 2 (its fourth row is persisted) |
| `a_carrier_inside_a_confined_config_value_fails_closed_in_both_deliveries` | exit 0 | exit 2 | (asserts exit code + empty walk, never the identifier) | both regions |

**Region 1 alone moved five of the seven.** The two that remained were exactly the
persisted rows — the mechanical confirmation of `19-24`'s deviation-1 finding that
the class-2 overlap holds per-delivery and that **a rule inside `scan_leading`
alone closes only one of the two deliveries.**

### The design question, ANSWERED: option (b), with option (a) costed by MEASUREMENT

**Option (a) — scan the confined value, applying the same clause recursively with
a stated depth bound — is REJECTED on three measured grounds, not on preference:**

1. **It needs a SECOND TOKENIZER.** Git splits an alias body with its OWN rules,
   not the shell's — measured, `git -c alias.m='config --get "core.hooksPath"' m`
   RESOLVES where the literally-quoted key is an `invalid key` error. Implementing
   a foreign grammar the guard cannot observe is the exact class of unmeasured
   claim `T-19-106` and `T-19-107` were both registered for.
2. **It needs UNBOUNDED RECURSION.** Measured at depth 2 **and depth 3**: both
   resolve `/INCLUDE_WINS`. Any stated depth bound therefore leaves a fail-open
   residue at depth N+1 that **an attacker reaches TODAY by adding one nesting
   level** — strictly worse than round 8's residue, which needs a future git.
3. **It cannot read a body it does not have.** `--config-env=alias.q=<VAR>`
   delivers the body through an environment variable NAME and `git -c alias.q` has
   no value half at all, so option (a) collapses to option (b) at exactly the
   carriers that matter.

**Option (b) — ADOPTED.** An assignment whose key names a value git re-parses as a
command line is one the guard cannot bound, on exactly the footing round 8's
confinement clause already means.

### K1 / K2 / K3 — three kinds, one reaches layer 3

Re-measured at this plan's base against `git version 2.43.0`, with
`cred::hooks_path_env`'s own triplet as the control:

```
control, no carrier                                              -> /ENV_WINS
-c alias.a='-c include.path=<f> config --get core.hooksPath' a   -> /INCLUDE_WINS   K1
-c alias.b='!git config --get core.hooksPath' b                  -> /ENV_WINS       K2
-c alias.g='config --get core.hooksPath !x' g   (`!` not first)  -> /ENV_WINS
-c alias.q='"!git config --get core.hooksPath"' q  (QUOTED)      -> expansion failed; '!git …'
                                                                    is not a git command
-c alias.t='<TAB>!git …' t                                       -> expansion of alias 't' failed
```

* **K1** — re-parsed as a GIT command line, IN-PROCESS, including its leading
  options. `alias.<name>` with a non-`!` body. **The only member**, and the whole
  of `T-19-108`.
* **K2** — re-parsed as a SHELL command line run as a CHILD that **INHERITS** the
  injection. `19-24` measured four representatives with the child's environment
  dumped; `core.pager` and `core.editor` were not exercised and are not claimed.
  **K2 needs no entry because the child inherits — a measurement, not a category
  argument.**
* **K3** — re-parsed as a FILE PATH spliced at the directive's precedence. Closed
  by `19-23`.

So `REPARSED_COMMAND_SECTIONS` names **one section**, and it names it because that
section is where K1 lives.

### The two-region rule, and why region 2 is REQUIRED rather than symmetric

Region 1 is `scan_leading`'s existing `if let Some(assignment)` block, ordered
after the confinement clause and **before** `is_hooks_path_key`, returned through
the `(usize, Option<GitVerdict>)` channel that already carried three refusals.
Region 2 is `classify_config`'s key operand — the operand `is_hooks_path_key`
already reads and already refuses `git config core.hooksPath /dev/null` at
`hook_bypass_blocked`.

**The persistence asymmetry with `include.path` is what makes region 2
load-bearing, and getting it backwards would have shipped an inert clause.** A
persisted include must WIN a precedence contest against the envelope's injection
and LOSES — audit 8 measured it INERT at repo-local, `--worktree` and GLOBAL. **An
alias does not have to win anything; it only has to EXIST**, and `19-24` measured
both a `--global` and a repo-local alias LIVE at `/INCLUDE_WINS` under the
envelope's own `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` redirect. **No `include.path`
or `includeIf` clause was added at region 2**, and
`git config include.path /tmp/evil.cfg` is pinned PERMITTED twice so that adding
one lands red.

**No third reading site was created, confirmed mechanically.** `git diff` over the
three fix commits shows no hunk in `first_unreadable_decision_word`,
`config_key_operand_index`, `subcommand_word_indices`, `scan_gh_api` or
`resolve_program_with_head`; `src/envelope/hooks.rs` has ZERO diff lines; and **no
`ParkReason` variant was added** — the refusal is
`ParkReason::EnvelopeAssertionFailed`, with a message naming THIS mechanism rather
than the include one (D-24). **The two ordering rows landed at their deliberately
DIFFERENT identifiers** (`envelope_assertion_failed` when the alias assignment
comes first, `hook_bypass_blocked` when the hooks key does), which is the
mechanical proof the clause is raised inside the ONE left-to-right walk and not in
a second pass.

### The three fail-open directions — stated, and handed to NO control

**This rule fails OPEN in three named directions and NOT ONE of them has an
automated control.** All three are written into `REPARSED_COMMAND_SECTIONS`'s doc,
the predicate's doc, the value helper's doc and this record, in these words:

1. **An `alias.*` already present in a config file the guard never saw a write
   to.** The guard is stateless and argv-only; `git co` is an unknown verb at exit
   0 and stays there. A repo-local `.git/config` alias predating the run is LIVE.
   `T-19-86`'s shape; stays open.
2. **A `!`-bodied body carrying its own carrier.** Audit 7's destructive pair,
   still working — see STATED FIRST. `T-19-86`.
3. **A future git that re-parses a SECOND config value as a git command line with
   its own leading options.** Same shape as `INDIRECTION_SECTIONS`'s residue, same
   absence of control. The real-git pin holds only the REVERSE direction and its
   own doc opens by saying so, because it iterates the constant's entries and an
   entry that does not exist is never probed.

**None of these is handed to the pin or to any other control.** Round 7's clause
hid exactly this behind a pin and a plan-check caught it; that is not repeated
here.

### The revisit condition and the version witness

Both `INDIRECTION_SECTIONS` and `REPARSED_COMMAND_SECTIONS` gain a revisit
condition in `AR-19-03`'s shape — a named, concrete trigger and what to do when it
fires: when the installed `git --version` differs from
`CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION` (`git version 2.43.0`),
**both constants are re-derived against the new git and the recorded version is
updated.** `the_config_section_constants_record_the_git_version_they_were_derived_against`
fires on that; it **cannot skip, cannot warn without failing, and does not pass
when `git` is absent**. Its cost is stated rather than hidden: **it fires on every
git upgrade, including harmless ones — and that IS the schedule**, the thing that
turns *a human reads the release notes* into *a test fires*.

**THE WITNESS IS A SCHEDULE, NOT A CONTROL.**

It observes exactly one bit — that the installed version string moved off the
recorded one — so it can say **WHEN to look**. **It cannot say WHAT changed.** It
does not observe a third indirection section appearing, it does not observe a
second re-parsed config value appearing, and **it stays GREEN on a git that adds
one without changing its version string.** The no-control claim above is therefore
**unchanged and stands beside it**: there is NO automated control over any of the
three fail-open directions, and the witness merely schedules the human re-audit
that is the only control there is.

**Provenance, recorded rather than presented as a sentence that was always
right.** An earlier draft of plan `19-25` stated the no-control claim and added
the witness in the same breath, in a way that read as the witness BEING the
control — **`T-19-107`'s own shape arriving in the round that inherited it** — and
a plan-check caught it, exactly as a plan-check caught round 7's residue hidden
behind a pin that could not observe it. That provenance is carried in both
constants' docs and in the witness's own doc.

**NO ACCEPTANCE WAS MADE.** No `AR-` row was added, no risk was accepted, and the
word "accepted" is applied to neither residue.

### The cost, measured from BOTH sides — ordinary aliases do NOT all keep working

**Defining a non-shell alias is refused; using one is not.**

| refused after `19-25` (a DEFINITION) | permitted twin (does the same work) |
|---|---|
| `git -c alias.st=status st` | `git status` |
| `git -c alias.lg="log --oneline" lg` | `git log --oneline` |
| `git -c alias.co=checkout co` | `git checkout` |
| `git config alias.co checkout` | `git checkout` |
| `git config alias.st status` | `git status` |
| `git config alias.lg 'log --oneline'` | `git log --oneline` |

**INVOCATION is untouched at exit 0, before and after**: `git p`, `git co`,
`git st`, `git lg`, `git z`, `git q`. Round 8's whole permitted half stays green,
and round 7's generative property stays green behind
`CALLEE_KNOWN_LEADING_PREFIX = "-c a=b"` because a dotless key names no section and
is CONFINED. The discrimination controls `git -c aliasx.q=…` and
`git -c notalias.q=…` stay at exit 0 — a prefix test reddens the first, a substring
test the second, and only a SECTION comparison keeps both.

**Two further disclosed over-refusals, stated rather than discovered later:**
`git -c alias.q status` (no `=`) is refused although real git resolves the control
value — **no harm behind that refusal**, and the corpus says so; and every
`--config-env` alias assignment is refused because its value half is a variable
NAME the guard does not read.

### A carrier-reading requirement that was MEASURED, not assumed

`git --config-env=alias.q='!EVIL'`, where the variable `!EVIL` holds a **non-`!`**
body, resolves **`/INCLUDE_WINS`**. **A first-byte test applied uniformly across
both carriers would read the `!` of a variable NAME as git's shell rule and fail
OPEN on exactly that row.** So `reparsed_command_assignment_is_a_shell_body` reads
the CARRIER — from `argv[index]`, the token `scan_leading`'s loop already holds —
and treats every `--config-env` value as unreadable. **No new reading site, no
second pass, and no arm added to `leading_git_option`.** The row is pinned from
both sides in `policy.rs`'s own real-git pin.

### `T-19-109` — the count replaced by REGIONS, as a NAMED NARROW exception

`cred.rs`'s `hooks_path_env` limit paragraph claimed **"FIVE forms outrank this
injection"** with a five-row table. Measured, there is a **SIXTH** — a non-`!`
`alias.<name>` body beginning `-c include.path=<file>` — so the counted claim was
wrong the day it was written, **for the second time** (it first claimed ONE). The
paragraph now states the REGIONS each closure covers — the argv leading-option
region, the environment, and the `git config` write operand — and what is NOT
covered, **counting nothing**. The client-side / branch-protection closing sentence
is kept.

**Recorded as an exception rather than presented as ordinary scope**: this is a
named, narrow, deliberate exception to this phase's `cred.rs` fence, granted on
`19-23`'s standard. **ZERO non-doc lines changed**, verified by
`git diff --unified=0` over the file filtered for lines that are not `///`
returning **0**. `T-19-61` … `T-19-73` are NOT taken on; `config_env`,
`hooks_path_env`'s body, `write_gitconfig`, `build_env_in` and `EnvelopeEnv` were
not opened.

### The `RESTORATION of layer 3's catch` correction — recorded BESIDE, never as an edit

The claim that closing `T-19-103` is a **RESTORATION of layer 3's catch** appears
in the plan-19-22 record, the plan-19-23 record and `19-23-SUMMARY.md`. **It is
true only for an alias body carrying no carrier of its own.** When the body carries
its own carrier the catch is ABSENT — measured in `policy.rs`'s own pin, where a
`!` body with no carrier resolves `/ENV_WINS` (the child inherits) while the same
body carrying `-c include.path=<f>` in its own command line resolves
`/INCLUDE_WINS`.

**Those three documents are NOT edited.** The correction is recorded here, beside
them, which is the provenance discipline audits 7 and 8 both used for this exact
shape. `19-24-SUMMARY.md` is likewise not edited.

### `T-19-106` — both halves closed

**The `gh api` pin, written and running.** All seventeen `GH_API_VALUE_OPTS`
entries answer ``flag needs an argument`` against real `gh` 2.45.0, probed
**ENDPOINT-LESSLY** (`gh api <opt>`) so **no row touches the network** — the
endpoint-bearing spelling `gh api repos/o/r --paginate` is measured to make a real
HTTP request, which would make the pin non-hermetic and fail-open without network
or auth. **Both negative controls, with their MEASURED strings pinned rather than
paraphrased:**

```
gh api --bogus-opt   -> unknown flag: --bogus-opt
gh api --paginate    -> accepts 1 arg(s), received 0
```

The second is a **POSITIONAL** error rather than a flag-level one, which is
precisely why it is the right control: `gh` **accepted** `--paginate`, consumed
**no** value for it, and got as far as complaining about the missing endpoint —
the *exists-and-requires-no-value* answer the classifier must be held to. Without
it the pin passes on a probe reporting *needs an argument* for everything, the
vacuity `19-23`'s `CONFIG_VALUE_OPTS` redesign exists to exclude. **Provenance:
both plans first described this as a flag-level answer and a plan-check measured
it and corrected them**, which is why the string is pinned and the description is
not. The pin does not skip when `gh` is absent, and it records that the reason for
its earlier absence was a fact about a **different callee**.

**`--hostname` is KEPT in `FORGE_VALUE_OPTS` — a correction to audit 8's own
suggestion, measured.** Re-measured against the built binary at this plan's base:

```
glab --hostname gitlab.com mr create --title x   -> exit 0, exactly ONE pr-ledger line
glab --host     gitlab.com mr create --title x   -> exit 0, ZERO pr-ledger lines
```

Removing the entry moves a **counted** creation form to **UNCOUNTED** — the
under-counting direction the cap exists to prevent (`T-19-35`). **The entry is
asymmetric by construction and the doc says so**: `gh pr create --hostname` answers
``unknown flag: --hostname``, so it is **INERT for `gh`** and **load-bearing for
`glab`**, whose callee is **UNCONFIRMED because `glab` is not installed**
(re-verified: `command -v glab` finds nothing). That is the whole difference from
`--comment` and `--super-prefix`, which were removed because the callee that does
run rejects them. **The `gh` half is pinned two-sided; the `glab` half CANNOT be
pinned on this machine and is NOT** — a pin that skips is a fail-open pin. The
`glab --host` cell is carried forward **UNFIXED**, and `subcommand_word_indices` is
not otherwise touched.

### A FINDING this plan MADE and did NOT fix — region 2's operand grammar

**It is NOT `T-19-108`, and it is registered rather than closed.**

`scan_config`'s walk treats any word beginning with `-` as an OPTION. **Git 2.43.0
does not agree**: once the KEY operand has been seen, the next word is the VALUE
whatever its first byte is. Measured against real git with both config pointers at
an empty file:

```
git config alias.x -q                    -> exit 0, alias.x=-q
git config alias.y --global              -> exit 0, alias.y=--global
git config alias.z -- -c foo             -> exit 0, alias.z=--
git config core.hooksPath -c             -> exit 0, core.hooksPath=-c
```

**Measured against the BUILT BINARY**, the consequence for the guard:

```
exit 2 hook_bypass_blocked   git config core.hooksPath /dev/null
exit 2 hook_bypass_blocked   git config core.hooksPath -
exit 0                       git config core.hooksPath -c        <- BYPASS
exit 0                       git config core.hooksPath --        <- BYPASS
```

So **plan 19-02's by-name `core.hooksPath` deny at region 2 is reachable past by
any value whose first byte is `-`, other than a bare `-`** (which `scan_config`
collects as an operand). This is a gap in region 2's **operand grammar**, not in
the re-parse question.

**It blocked this plan's mandated region-2 gate and forced a derivation.**
`T-19-108`'s HEADLINE persisted row is `git config alias.p '-c include.path=<f>
…'`, whose value begins with `-`, so `is_write` is FALSE for it and a clause gated
on `is_write` would have been blind to the one row it exists for. The re-parse
clause therefore gates on `!is_read` plus a **VALUE WORD**, read through the new
`ConfigScan::value_word`, indexed off `key_operand`'s own index into the same walk.
`config_key_operand_index` is unchanged.

**`is_write` itself was deliberately NOT widened.** Correcting it moves verdicts
for keys outside this round's class with **no corpus able to fail on them**, and
this phase's whole discipline is that a rule is written against a corpus observed
RED first. The rows are **RECORDED, never asserted** — in
`tests/envelope_reparsed_value.rs::the_region_2_operand_grammar_gap_is_recorded_by_19_25_and_is_not_closed_by_it`
— because asserting them PERMITTED would pin a bypass as correct and asserting
them REFUSED would pin a verdict this round did not produce, which is `19-22`'s
failure mode. Registered in `deferred-items.md` as **`T-19-110`, OPEN at `high`**.

### A probe error, recorded rather than hidden

The K2 arm of the `REPARSED_COMMAND_SECTIONS` real-git pin was first written with
the body `!git -c include.path=<f> config --get core.hooksPath` and measured
`/INCLUDE_WINS`. **That was not a contradiction of the `!` carve-out** — it is
fail-open direction (ii), a `!` body carrying its OWN carrier, which is `T-19-86`.
The INHERITANCE fact needs a body whose own command line carries nothing. Both are
now pinned as separate arms. Recorded because a probe error that looks like a
finding is exactly the shape that gets asserted by mistake, which is why `19-24`
recorded its own.

### The mechanism pins and the anti-vacuity floors, re-measured

Rounds 5, 6, 7 and 8's mechanisms are all non-dead and green. `SEPARATORS` is
**byte-identical** and `policy::is_separator(">")` is `false`. `Token.literal`,
`Token.expansion`, `Token.word_splitting_flush`, `Token.brace_splice`,
`Segment.head_is_command_position` and `Segment.redirection_unresolvable` are
unchanged. Round 8's confinement clause is non-dead with both discrimination
controls green.

| | measured at this plan's end | floor |
|---|---|---|
| `policy.rs` production half | **261,386** bytes | `POLICY_MIN_PRODUCTION_BYTES = 40_000`, proportional `>= 180_000` |
| `hooks.rs` production half | **74,357** bytes | `HOOKS_MIN_PRODUCTION_BYTES = 20_000` |
| deep anchor | `fn forbidden_repo_path` at production line **5,141** of **5,162** | present |
| sentinel | exactly **ONE** `#[cfg(test)]`, at line **5,163** | count asserted |

All three stripper protections keep their present strictness; both byte floors keep
their values; nothing was lowered. This plan only GREW the production half.

`policy.rs`'s stale proportional-floor comment is **not touched** — it was already
recorded as documentation drift by `19-24` and this plan was not assigned it; it is
now staler, since the production half grew from 228,101 to 261,386 bytes.

### The gate

`rtk proxy cargo test --no-fail-fast`, counts read with `rtk proxy grep` over a
redirected log (D-34).

| | |
|---|---|
| `19-24`'s recorded `passed + failed` | **1710** |
| observed `passed + failed` | **1720** (1720 passed, **0** failed, 13 ignored) |
| new `#[test]` fns, counted from `git show` | 3 + 5 + 2 = **10** |
| identity | 1710 + 10 = **1720** ✓ no disagreement |

**A red test RAN, so red→green leaves the total unchanged and every increase comes
only from new `#[test]` fns.** **ZERO failures** — none of the three documented
flakes fired, which is not evidence they are fixed. **All FIFTEEN `envelope_*`
binaries RAN.** `cargo build` and `cargo clippy -- -D warnings` exit 0
(`cargo clippy --tests` is NOT the gate and still fails at base on four
pre-existing lints).

`git diff --numstat` over `tests/` shows **294 insertions and ZERO deletions**
across all four commits, and `tests/envelope_command_position.rs` and
`tests/envelope_config_resolution.rs` show ZERO diff lines.

### `T-19-17r` — still OUTSTANDING, and this plan did NOT accept it

**No Accepted-Risks-Log row was added, no `AR-19-13` was created (verified: count
0), and the word "accepted" is not applied to `T-19-17r` anywhere in this plan's
artifacts.** Five audits and plans running have deliberately left the acceptance
unmade because it is a human decision. The next round either adds the log row or
drops the word from `19-17-SUMMARY.md`; this plan does neither.

---

## Threats found by audit 9 (2026-09-04, after plans 19-24 and 19-25)

**Provenance, stated first.** The two subsections above this one were written by
the EXECUTORS of plans 19-24 and 19-25; they are deliberately outside the audit
tables. Audit 9 left them, every earlier appended subsection and every earlier
audit's own tables **byte-identical** — the body below the frontmatter was
checksummed before writing (`sha256 54ccdbae18e55a28…` over the 553,534 bytes
below the frontmatter of the 554,476-byte file), and this audit's write changes
the frontmatter, adds one Security-Audit-Trail row, one method subsection and
everything from here to the end of the file, **deleting nothing**. Everything
below is **audit 9's own**, measured against the built binary at `cc65220` with a
fresh `GSD_MM_ENVELOPE_ROOT` per row and the whole envelope root walked
afterwards, and with every claimed bypass confirmed against the **real `git`
binary** (`git version 2.43.0`) in a rebuilt bare-remote fixture with a control
beside every leg.

### The question this audit was set, answered plainly

**With word assembly, word deletion, leading-option grammar, config resolution,
the config-bearing environment and now re-parsed config values all modelled, is
there a further layer — or is the modelled surface complete and the remaining
risk exactly the disclosed residual set?**

**There is a further layer, and for the first time in nine rounds it is not on
the axis.** Round 9 is right about the layer it set out to close. Every
`T-19-108` row re-measured below is refused, both regions are load-bearing and
measured separately, the `!` carve-out is git's own one-byte rule and not a
loophole, and `alias` really is the only K1 section — audit 9 re-derived that
independently by sweeping **twenty-one** config keys against real git with the
envelope's own injection as the control, and every one of the other twenty
resolved `/ENV_WINS`. The rule is correct, its cost is disclosed from both sides,
and its residue is stated in four places and handed to no control.

**But every one of the nine rounds models an ARGV.** Word assembly, word
deletion, leading-option grammar, config resolution, the config-bearing
environment, a re-parsed config value: each asks what a *command line* becomes.
The guard is stateless and argv-only, and **every control this phase installs has
a FILE carrier that no command line has to touch**:

- **`.git/config`** carries the alias. `printf '[alias]\n\tfp = -c
  include.path=<evil> push --force origin HEAD:refs/heads/main\n' >> .git/config`
  is exit 0 — it is not a git command and the guard has no opinion about it — and
  `git fp` is exit 0 because the guard cannot see an alias it did not watch being
  defined. Against real git the pair **rewrote the bare remote's `main`,
  `e86471e` → `3027440`**, while the same alias written the same way **without**
  the carrier is refused by the hook and moves nothing. **That body has no `!`.
  It is K1 — `T-19-108`'s own class — reached through a carrier outside argv**,
  and it is registered here as `T-19-111` rather than folded into `T-19-86`.
- **`<envelope>/<alias>/pr-ledger.ndjson`** carries SAFE-06. `rm -f` on it is
  exit 0, and so are `: >`, `truncate -s 0` and `cp /dev/null`. The path is
  reached through a read the guard permits: `git config --get core.hooksPath` is
  exit 0 and names `<envelope>/<alias>/hooks`. Measured through the guard: the
  cap fires on call 2, the deletion is permitted, and **the very next
  `gh pr create` is permitted with a fresh one-line ledger**. `T-19-112`.
- **`<envelope>/<alias>/hooks/pre-push`** carries layer 3, and with it SAFE-05's
  only carrier and `AR-19-03`'s stated backstop. `cp /bin/true $(git config --get
  core.hooksPath)/pre-push` is exit 0. `T-19-113`.

**So the pattern held for a ninth round, and this time it left the plane.**
Rounds 1–5 were about how a word is written, round 6 about which words arrive,
round 7 about which arriving word is the verb, round 8 about what the verb runs
under, round 9 about what git does with a value the verb was handed. **Round 10's
question is not about a word at all**: every carrier this phase reads is an argv,
and every carrier it *depends on* is a file.

### What audit 9 measured and found INERT or CORRECT, recorded because the question was asked

Every mechanism the mandate named, measured against real git 2.43.0 with the
envelope's own triplet as the control (`-> /ENV_WINS` means the envelope wins):

```
control, no carrier                                        -> /ENV_WINS
init.templateDir / GIT_TEMPLATE_DIR                        -> /ENV_WINS   INERT
url.<b>.insteadOf                                          -> /ENV_WINS   INERT
submodule.<n>.update  (a `!` body)                         -> /ENV_WINS   INERT
core.pager / core.editor / sequence.editor                 -> /ENV_WINS   INERT
diff.external / difftool.<d>.cmd / mergetool.<m>.cmd        -> /ENV_WINS   INERT
credential.helper / core.sshCommand / core.gitProxy        -> /ENV_WINS   INERT
core.fsmonitor / uploadpack.packObjectsHook / gpg.program  -> /ENV_WINS   INERT
trailer.<t>.command / filter.<f>.clean / browser.<b>.cmd   -> /ENV_WINS   INERT
interactive.diffFilter / man.viewer                        -> /ENV_WINS   INERT
alias.<n> with a NON-`!` body carrying `-c include.path`   -> /INCLUDE_WINS  <- K1, the ONLY one
```

**`core.pager` and `core.editor` are among the two `19-25` explicitly did NOT
exercise and did NOT claim; audit 9 exercised both and they are inert.** The
inheritance fact behind the whole K2 family was reproduced rather than cited: a
`!` alias body printed `CHILD_COUNT=1 KEY0=core.hooksPath VALUE0=<hooks dir>`
from inside the child. **So `REPARSED_COMMAND_SECTIONS = ["alias"]` is correct
for this git, independently derived.**

**The `git` global-option grammar is complete.** All thirty-eight spellings of
the documented global options — every one in both `--opt=value` and `--opt value`
form, plus `-C/tmp`, `-ca=b` and a three-option chain — were driven with
`push --force origin main` behind them, and **every one is exit 2**. The single
exit-0 row, `git --exec-path /tmp push --force origin main`, is CORRECT: real git
prints `/usr/lib/git-core` and exits without pushing, which audit 9 measured
rather than assumed. Round 7's grammar model holds.

**The push operand grammar is complete for its class.** Twenty-two destructive
spellings — `+refs/...`, `+main`, `--mirror`, `--all`, `--tags`, `--delete`,
`-d`, `:main`, `origin --delete main`, `--force` AFTER the refspec,
`--force-with-lease`, `--force-if-includes`, `--receive-pack -x`, `--repo=`,
`HEAD:main`, `--set-upstream` — are all exit 2, and the in-namespace control is
exit 0.

**`PATH` and `GIT_EXEC_PATH` change no verdict.** `PATH=/tmp/e:$PATH git push
--force origin main` is exit 2, and `GIT_EXEC_PATH=<dir>` reaches a child that
inherits the injection, so it is the K2 shape.

**`git push <arbitrary-url> refs/heads/gsd-auto/alpha/w` is exit 0 and the hook
allows it**, because `pre_push` judges the ref lines, the worktree scan and the
swept paths and never reads the remote name or URL (`hooks.rs:314-364`). Recorded
as an observation and **NOT a finding**: `SECTION_ENVELOPE` says in terms that
the hook judges *"the refs git hands it, not the command line asked about"*, D-05
is a namespace control, and `T-19-25`'s credential leg holds because the host is
baked at `cred.rs:395`. This phase does not claim destination control and does
not imply it.

### The new rows

| Threat ID | Category | Component | Severity | Disposition | Finding | Status |
|---|---|---|---|---|---|---|
| **T-19-111** | **Elevation of Privilege** | **the RE-PARSE clause's CARRIER SET — `config_key_names_a_reparsed_command_section` is reached only from `scan_leading` (`policy.rs:566`) and `classify_config` (`policy.rs:1942`), so it sees the `-c`, `--config-env` and `git config` deliveries; a `.git/config` written by any other command delivers the same K1 body unseen** | **high** | **mitigate (unimplemented)** | **Round 9's own class, reached through a carrier that is not a command line.** `printf '[alias]\n\tfp = -c include.path=<evil> push --force origin HEAD:refs/heads/main\n' >> .git/config` → **exit 0, walk EMPTY** (it is not a git command); `git fp` → **exit 0, walk EMPTY** (the guard is stateless and cannot see an alias it did not watch being defined). Seven further write spellings all exit 0: `echo >>`, `cat >>`, `tee -a`, `sed -i`, `python3 -c "open(...,'a').write(...)"`, `cp <file> .git/config`. **The body carries no `!`**, so it is not a command line handed to a governed program as data — it is exactly the non-shell K1 body `T-19-108` names and `19-25` closed at the two argv/`git config` carriers. **Confirmed end to end against real git 2.43.0 with a CONTROL beside every leg.** Bare remote, `pre-push`/`pre-commit` delivered exactly as the envelope delivers them: the plain `git push --force origin HEAD:refs/heads/main` is **REFUSED by the hook** and leaves `main` at `e86471e`; the `printf`-written alias carrying the include **COMPLETES and rewrites `main` to `3027440`**; and the **same alias, written the same way, carrying NO carrier is REFUSED by the hook and moves nothing** — the control that isolates the CARRIER from both the ALIAS and the file write. **The residue IS disclosed** — `REPARSED_COMMAND_SECTIONS`'s doc (`policy.rs:1177-1183`) says *"a repo-local `.git/config` alias predating the run, or one written by a means that is not `git config`, is LIVE"* — **but it is MIS-ATTRIBUTED**: that doc, the `19-25` record and `19-25-SUMMARY.md` all call it *"`T-19-86`'s shape"*, and `cred.rs:287-289`'s corrected reach paragraph narrows it further to *"predating the run"*, dropping the written-during-the-run half the policy doc has. `T-19-86`'s four registered rows all require a governed program to be handed a governed COMMAND as data; this is a plain git command line in a config value. **Crediting a live, non-shell, measured destructive bypass to a threat the user has explicitly scoped OUT is how it stops being tracked**, and that is why this is registered separately rather than folded in. **Not the corpus's fault alone**: `CONFIG_REPARSED_VALUE_CARRIERS` and every other alphabet in all nine rounds draw only governed COMMAND LINES — `grep -rn "\.git/config\|pr-ledger.ndjson\"\|hooks/pre-push" tests/envelope_wrapper_class.rs tests/envelope_reparsed_value.rs tests/envelope_command_position.rs` returns **nothing at all**, so the corpus is structurally incapable of generating, and so of failing on, this row. | **OPEN — BLOCKING** |
| **T-19-112** | **Tampering** | **the SAFE-06 PR-cap ledger as a FILE — `<envelope>/<alias>/pr-ledger.ndjson` (`ledger.rs:47`); layer 2 classifies only governed programs and layer 1 denies only `Write`/`Edit` on `.claude/**` (`policy.rs:2063-2064`)** | **high** | **mitigate (unimplemented)** | **The cap is resettable by one permitted tool call, and the path is reached through a read the guard permits.** `git config --get core.hooksPath` → **exit 0**, naming `<envelope>/<alias>/hooks`; the ledger is its sibling. Measured through the guard, all **exit 0** with EMPTY walks: `rm -f <root>/alpha/pr-ledger.ndjson`, `: > <ledger>`, `truncate -s 0 <ledger>`, `cp /dev/null <ledger>`, `rm -rf <root>/alpha`, `echo evil > <root>/alpha/askpass`, and the whole thing on one line — `D=$(git config --get core.hooksPath); rm -f $D/../pr-ledger.ndjson`. **The reset was then driven end to end in one persistent root**: `gh pr create --title x` permitted on call 1 (ledger 1 line), **refused on calls 2 and 3** with the cap firing, the deletion performed, and **call 4 permitted again with a fresh one-line ledger**; the `: >` spelling behaves identically. `T-19-32`'s declared mitigation — *"Ledger under `<envelope>/<alias>/`, asserted by a path test; `.claude/**` denied on both carriers"* — **is PRESENT and addresses only the repository side**; nothing addresses the file itself. **SAFE-06 has no second carrier** (`hooks.rs:1263-1270`, `T-19-35`), so the guard is the only observation point and its record is deletable by the process it observes. **`SECTION_ENVELOPE` lists this under `Guaranteed`** — *"Pull-request cap: an append-only ledger this repository does not contain"* — and its only stated caveat is a different mechanism (*"a settings file the agent's own CLI silently ignores"*), so an affirmative Guaranteed claim is falsified by measurement, which is the `envelope → operator` trust boundary `T-19-39` exists for. **The counter-argument, stated rather than suppressed**: the trust-boundary table already declares *"driven agent → filesystem — the agent can write any file the process can"*. That declaration is why `T-19-113` below is `medium`; it does not reach here, because the cap is claimed as guaranteed on a carrier that declaration makes writable. **No test anywhere considers a write to the envelope directory**: `grep -rn "pr-ledger" src/` finds only `ledger.rs`'s own path and doc rows. | **OPEN — BLOCKING** |
| T-19-113 | Tampering | the hook stubs and the askpass stub as FILES — `<envelope>/<alias>/hooks/{pre-push,pre-commit}` (`hooks.rs:127-146`) | medium | mitigate | Layer 3 is removable without touching `GIT_CONFIG_COUNT`. Measured through the guard at **exit 0**: `printf 'exit 0' > <hooks>/pre-push`, `cp /bin/true <hooks>/pre-push`, and `D=$(git config --get core.hooksPath); printf 'exit 0' > $D/pre-push` — the last only because the `printenv GIT_CONFIG_VALUE_0` spelling of the same discovery is refused at `hook_bypass_blocked` while `git config --get core.hooksPath` is not. `T-19-01`'s declared mitigation (`current_exe()` + `assert_provenance_in`) **is present and correct** for what it covers — a *relocated copy* of the stub — but the sanctioned path's CONTENTS are not defended, and a replaced stub never re-enters the binary that would assert provenance. Losing `pre-push` loses SAFE-05's only carrier and the backstop `AR-19-03` explicitly rests on (*"the pre-push and pre-commit hooks make the denylist's gaps non-fatal"*), which is the same argument audits 6, 7 and 8 each used to rate a layer-3 defeat `high`. **Rated `medium` rather than `high`, and the reason is stated rather than assumed**: `SECTION_ENVELOPE` says outright that *"client-side hooks, tool denies and env-injected git config are all defeatable by an agent that can spawn an unsupervised shell"*, and the trust-boundary table declares the filesystem write. **What is wrong is the ceiling's NARRATIVE, not the disclosure**: D-09 names the route the guard REFUSES (`unset GIT_CONFIG_COUNT` → `hook_bypass_blocked`, re-measured) while the route that works is refused by nothing, so a reader of D-09 would conclude the guard stands underneath layer 3 and it does not. | open — below `high` (non-blocking) |
| T-19-114 | Spoofing | the generative corpus's carrier alphabet, all six axes | low | mitigate | `T-19-76`'s failure mode for the ninth consecutive round, and for the first time it is not one cell over — it is off the plane. **Every alphabet in all nine rounds draws a governed COMMAND LINE.** `CONFIG_REPARSED_VALUE_CARRIERS` (`tests/envelope_wrapper_class.rs:6632`) carries five entries, all `-c alias.*=` or `--config-env=alias.*=`; `CONFIG_PERSISTED_REPARSED_VALUE_CARRIERS` carries five, all `git config alias.*`. Verified mechanically rather than read: no test file constructs a non-governed command touching `.git/config`, `pr-ledger.ndjson` or `hooks/pre-push`, and `grep -rn "pr-ledger" src/` finds no control. The corpus is therefore structurally incapable of generating, and so of failing on, `T-19-111`, `T-19-112` or `T-19-113`. Same finding as `T-19-76`, `T-19-83`, `T-19-89`, `T-19-95`, `T-19-99`, `T-19-101`, `T-19-105` — at the next radius out, which this round shows is a different KIND of carrier rather than a wider alphabet of the same one. | open — below `high` (non-blocking) |

*Status: open · closed · open — below `high` threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` count toward `threats_open`*

`T-19-113` and `T-19-114` are open below the `high` threshold and do **not**
count toward `threats_open`. `T-19-111` and `T-19-112` do.

### `T-19-110` — audit 9 RE-RATES it `high` → `medium`, on measurement, and it changes no gate

Plan 19-25 registered `T-19-110` at `high` in `deferred-items.md`. **Audit 9
confirms the reproducers exactly and finds the class WIDER than the plan
recorded**, then finds its only reachable write **INERT**:

```
exit=2 [hook_bypass_blocked]  git config core.hooksPath /dev/null   <- the deny, working
exit=2 [hook_bypass_blocked]  git config core.hooksPath -           <- collected as an operand
exit=0                        git config core.hooksPath -c          <- BYPASS, as 19-25 recorded
exit=0                        git config core.hooksPath --          <- BYPASS, as 19-25 recorded
exit=0                        git config core.hooksPath -q          <- audit 9's own
exit=0                        git config core.hooksPath --global    <- audit 9's own
exit=0                        git config core.hooksPath -c/dev/null <- audit 9's own
exit=0                        git config --global core.hooksPath -c <- audit 9's own
```

**But every one of those writes LOSES the precedence contest it has to win.**
Measured against real git under the envelope's own posture: `git config
core.hooksPath -c` succeeds (`git config --local --get core.hooksPath` reads back
`-c`) and **`git config --get core.hooksPath` still resolves the envelope's hooks
directory**; the same at `--global`, against the generated file both pointers
name; and a force push after both writes is still **refused by the hook, `main`
unmoved at `3027440`**. **That is exactly the reasoning `19-25` itself used to
refuse an `include.path` clause at region 2, and audit 8 used to rate the
persisted include INERT — applied to the neighbouring key on the same walk.**
The clause `19-25` DID write is not blind to the gap: `git config alias.p -c`,
`--`, `-q` and `-c include.path=…` are all **exit 2
`envelope_assertion_failed`**, because the re-parse clause reads
`ConfigScan::value_word` rather than `is_write`. **So region 2's grammar gap has
no measured harm today, and `medium` is what the evidence supports.** It stays
OPEN: the gap is real, the operand grammar disagrees with git's, and a future
region-2 clause gated on `is_write` would inherit it. **This re-rating clears
nothing** — audit 9 adds two `high` rows of its own and the gate stays blocked.

### Audit 9's bookkeeping, re-derived from audit 8's group rows

| Group | Count | Closed | Open |
|---|---|---|---|
| Everything through audit 8 | 121 | 99 | 22 (3 at `high`) |
| Closed by plans 19-24 / 19-25, re-measured by audit 9 (`T-19-108` as scoped, `T-19-106`, `T-19-109`) | — | +3 | −3 |
| Registered by plan 19-25, NOT fixed (`T-19-110`, re-rated `medium` by audit 9) | 1 | 0 | 1 (0 at `high`) |
| Found by audit 9 (`T-19-111` … `T-19-114`) | 4 | 0 | 4 (2 at `high`) |
| **Total after audit 9** | **126** | **102** | **24 (4 at `high`)** |

The four that count toward `threats_open`: `T-19-86`, `T-19-91`, `T-19-111`,
`T-19-112`. The twenty that do not: `T-19-61` … `T-19-73` (13), `T-19-84`,
`T-19-85`, `T-19-96`, `T-19-105`, `T-19-110`, `T-19-113`, `T-19-114`.

### The closures, re-measured rather than accepted from the summaries

Driven as
`printf '<PreToolUse JSON>' | GSD_MM_ENVELOPE_ROOT=$(mktemp -d) ./target/debug/gsd-meta-manager envelope guard alpha`,
one fresh root per row, the envelope root walked with `os.walk` afterwards, the
walk proved non-blind on every pass by `gh pr create --title x` leaving exactly
one `alpha/pr-ledger.ndjson` line.

* **`T-19-108` — CLOSED AS SCOPED, and audit 9 states the scope the SUMMARY did
  not.** All six `-c`/`--config-env` rows and all seven persisted rows, every
  walk EMPTY:

  ```
  exit=2 [envelope_assertion_failed]  git -c alias.q="-c include.path=… push --force origin HEAD:refs/heads/main" q
  exit=2 [envelope_assertion_failed]  git -c alias.z="-c core.hooksPath=/dev/null push origin refs/heads/gsd-auto/alpha/w" z
  exit=2 [envelope_assertion_failed]  git -c alias.q="-c include.path=… status" q
  exit=2 [envelope_assertion_failed]  git -c alias.q="-c include.path=… commit -m x" q
  exit=2 [envelope_assertion_failed]  git -c ALIAS.q="-c include.path=… status" q
  exit=2 [envelope_assertion_failed]  git --config-env=alias.q=EVILBODY status
  exit=2 [envelope_assertion_failed]  git config alias.p '-c include.path=… push origin refs/heads/gsd-auto/alpha/w'
  exit=2 [envelope_assertion_failed]  … and its --global, --worktree, --file, --add and --replace-all spellings
  exit=2 [envelope_assertion_failed]  git config alias.z '-c core.hooksPath=/dev/null status'
  ```

  All three of audit 8's own measured `T-19-108` rows are closed, so the row
  closes on the convention this file has used since `T-19-87`: closed for its
  measured rows, residual carried on a new row. **The SUMMARY's "closes only AS
  SCOPED" names the excluded BODY (`!`) and not the excluded CARRIER (a file).**
  That is `T-19-111`.
* **Both regions are load-bearing, and the split is mechanically confirmed.**
  `19-25` reports region 1 alone moving five of seven RED rows, the two remaining
  being exactly the persisted ones. Audit 9 confirms the mechanism directly
  rather than re-running the transition: all seven `git config alias.*` rows are
  refused while `config` is the verb, so `scan_leading` stops before the operand
  and **no rule inside region 1 can reach them** — region 2 (`policy.rs:1942`) is
  what refuses them. The two ordering pins land at their **deliberately different
  identifiers** — alias-first → `envelope_assertion_failed`, hooks-first →
  `hook_bypass_blocked` — which is the mechanical proof the clause is raised
  inside the ONE left-to-right walk.
* **The cost is exactly as disclosed, in both directions.** Six DEFINITIONS
  refused (`git -c alias.st=status st`, `-c alias.lg="log --oneline" lg`,
  `-c alias.co=checkout co`, `git config alias.co checkout`, `alias.st status`,
  `alias.lg 'log --oneline'`), each beside a permitted twin doing the same work
  (`git status`, `git log --oneline`, `git checkout`, all exit 0). Six
  INVOCATIONS untouched at exit 0: `git p`, `git co`, `git st`, `git lg`,
  `git z`, `git q`. The discrimination controls hold — `git -c aliasx.q=…` and
  `git -c notalias.q=…` both exit 0, so a prefix test reddens the first and a
  substring test the second. The dotless fence holds: `git -c a=b status` and
  `git -c a=b version` both exit 0, and round 7's whole generative property is
  spliced behind `CALLEE_KNOWN_LEADING_PREFIX = "-c a=b"`. Every `git config`
  READ stays permitted: `--get`, `--list`, `-l`, `--get-regexp`, and the bare
  `git config alias.p`.
* **`T-19-106` — CLOSED, both halves, verified by reading the pin.**
  `every_gh_api_value_opt_really_takes_a_separate_value_on_the_installed_gh`
  (`policy.rs:7559`) probes all seventeen `GH_API_VALUE_OPTS` entries
  ENDPOINT-LESSLY so no row touches the network, requires
  ``flag needs an argument`` for each, and carries **both** negative controls
  with their measured strings pinned rather than paraphrased —
  `--bogus-opt` → ``unknown flag``, and `--paginate` → ``accepts 1 arg(s),
  received 0``, asserted BOTH to contain the positional answer and NOT to contain
  ``flag needs an argument``. That second control is the one that excludes the
  vacuity. It does not skip when `gh` is absent. **`--hostname` is KEPT in
  `FORGE_VALUE_OPTS` and audit 8's own suggestion to remove it is correctly
  overturned on measurement**: `glab --hostname … mr create` leaves ONE ledger
  line and `glab --host …` leaves ZERO, both re-measured here, so removal is a
  regression in the under-counting direction (`T-19-35`).
* **`T-19-109` — CLOSED.** `cred.rs:246-291` now states the REGIONS each closure
  covers — the argv leading-option region, the environment, and the `git config`
  write operand — **and counts nothing**, recording that the paragraph was wrong
  twice before. Verified by reading it. **One qualification recorded here rather
  than as a re-opening**: its "what is not covered" bullet says *"an alias
  already persisted in a config file the guard never saw a write to … predating
  the run"*, dropping the *"or one written by a means that is not `git config`"*
  half that `policy.rs:1182` has — which is `T-19-111`, understated in the one
  paragraph whose purpose is stating reach.
* **`T-19-105` — ADDRESSED for the config-resolution axis, and OPEN for the tenth
  consecutive round.** The sixth class is real and the floor rose with it:
  `MIN_CONFIG_RESOLUTION_CLASSES` **5 → 6**, `CONFIG_REPARSED_VALUE_CARRIERS`
  with its own `MIN_CONFIG_REPARSED_VALUE_CARRIERS = 5`, generated cases
  **161 → 264**. The corpus demonstrably failed on its own class before the rule:
  `0176d3c` and `7388038` are `tests/`-only commits with **zero `src/` hunks**
  carrying the eight RED rows, and `19-25`'s three `src/` commits turned them
  green. The SHELL-ALIAS FENCE that keeps a `!` entry out of the alphabet is
  present and names both pinning files. **The successor is `T-19-114`.**

### The carrier-reading judgement, probed at the boundary

Seventeen spellings neither round pinned, to answer whether reading the CARRIER
opens anything a uniform first-byte rule closed. **It does not, and it closes one
row a first-byte rule fails open on.** Every `--config-env` alias spelling is
refused including `--config-env=alias.q=!EVIL` and `--config-env alias.q=!EVIL`,
which is the row `19-25` measured resolving `/INCLUDE_WINS` and the reason the
carrier must be read. `-calias.q=…` (attached), `Alias.q=…` (case-folded
section), `alias.'q'=…`, the leading-SPACE and leading-TAB bodies, the
QUOTED body `'"!git …"'`, and a second alias assignment behind a first `!` one
are **all refused**; `!`-bodied assignments in three quotings are permitted as
`T-19-86` rows. The leading-space, TAB and quoted rows are refusals with **no
harm behind them** — git itself answers `expansion of alias … failed` for all
three, measured in `19-24` — so the direction is fail-closed and the cost is the
disclosed one.

### The mechanism pins, verified non-dead

Rounds 4 through 8 are all still load-bearing and all still RIGHT about the lines
round 9 refuses:

```
exit=2 [envelope_assertion_failed]  git pus? --force origin main             <- round 5's literalness bit
exit=2 [force_push_blocked]         git >/dev/null push --force origin main  <- round 6's deletion model
exit=0                              git x2>/tmp/o push --force origin main   <- round 6's over-deletion control
exit=2 [force_push_blocked]         git --attr-source HEAD push --force origin main  <- round 7
exit=2 [envelope_assertion_failed]  git --bogus-opt status                   <- round 7's inversion
exit=0                              git - push --force origin main           <- round 7's pinned control
exit=2 [force_push_blocked]         git -- push --force origin main          <- its twin
exit=2 [envelope_assertion_failed]  git -c include.path=/tmp/evil.cfg status <- round 8's confinement clause
exit=2 [hook_bypass_blocked]        GIT_CONFIG_PARAMETERS="'core.hooksPath=…'" git commit -m x
exit=2 [hook_bypass_blocked]        env -u GIT_CONFIG_COUNT git push --force origin main
exit=2 [envelope_assertion_failed]  V=push; git $V --force origin main       <- round 4's Rule A
```

`SEPARATORS` is **byte-identical** — `const SEPARATORS: &[&str] = &[";", "&&",
"||", "|", "&", "\n", "(", ")", "{", "}"]` at `policy.rs:2281`, and
`git log -L 2281,2281` over that line returns exactly **one** commit in the whole
phase (`84a9b05`, plan 19-05) — and `is_separator` is a bare `SEPARATORS.contains`
(`policy.rs:3526-3528`), so `is_separator(">") == false` by construction.

**The byte floors and the stripper's three protections, re-derived at `cc65220`
by splitting each file at its `#[cfg(test)]` sentinel:** `policy.rs` raw 441,914
/ production **261,387**, `hooks.rs` raw 99,753 / production **74,358** — both
far above `POLICY_MIN_PRODUCTION_BYTES = 40_000` and
`HOOKS_MIN_PRODUCTION_BYTES = 20_000` at their unchanged values
(`tests/envelope_wrapper_class.rs:831` and `:838`), and the production half is
68.7% of `policy.rs` against the proportional floor `>= 180_000`. **Exactly ONE
line in each file is `#[cfg(test)]` after trimming** — `policy.rs:5163`,
`hooks.rs:1573` — which audit 9 checked against the stripper's own predicate
(`trimmed == "#[cfg(test)]"`, `tests/envelope_wrapper_class.rs:769`) rather than
by substring, because fourteen further occurrences sit inside `mod tests` as
prose and would make a naive count read 15. The deep anchor `fn
forbidden_repo_path` is present in the production half.

### The known-open set, as audit 9 verified it

- **`T-19-86` (high, OPEN, unaccepted, deferred by explicit user scoping
  decision).** All four rows re-measured at `cc65220`, fresh root each, all
  **exit 0** with EMPTY walks; the persisted-alias arm reproduces
  (`git config alias.p "!git push --force origin HEAD:refs/heads/main"` → 0 and
  `git p` → 0), and so does the carrier-bearing `!` pair in both the persisted
  and the `-c` delivery. **Confirmed still DESTRUCTIVE against real git**: the
  `!` pair rewrote the bare remote's `main`, `2061bb6` → `e86471e`, while the
  same `!` alias carrying **no** carrier is refused by the hook and moves nothing
  — the control that proves the K2 child inherits the injection and that the harm
  is the CARRIER. Counts toward `threats_open`. **Recorded WIDER than the
  register credits, as audit 8 and `19-24` both recorded, and NOT narrowed by
  round 9.**
- **`T-19-91` (high, OPEN, three arms).** `S=…; git reflog $S`, `git reflog show
  $S` and `git symbolic-ref $S` all reproduce at **exit 0**; `git symbolic-ref
  HEAD $R` → 2 `force_push_blocked` and `git push origin $REF` → 2
  `push_outside_namespace` still fail closed. Unweakened by round 9. Counts
  toward `threats_open`.
- **`T-19-96` (medium, OPEN, registered, not fixed).** `git push --forc? origin
  refs/heads/gsd-auto/alpha/w` → **exit 0**, its literal twin → 2
  `force_push_blocked`. Unchanged.
- **`T-19-74` (medium, closed/accepted — AR-19-10).** Core rows frozen and
  re-measured permitted: `env $X push --force origin main` → **0**,
  `X=git; env $X push --force origin main` → **0**. `T-19-84` remains open and
  unaccepted.
- **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85` — carried forward untouched.**
  `git diff --numstat fb43577..cc65220 -- src/` is **exactly two files**
  (`cred.rs`, `policy.rs`), and the `cred.rs` half is the named narrow exception:
  `git diff --unified=0` over it, filtered for lines that are not `///` and not
  blank, returns **NOTHING** — **zero non-doc lines changed**, exactly as
  claimed. `hooks.rs`, `mod.rs`, `advisory.rs`, `scan.rs` and `config.rs` were
  not opened, so `T-19-61` … `T-19-73` cannot have moved.
- **The two `T-19-86` rows in fenced files — GREEN with ZERO diff lines.**
  `git diff --numstat fb43577..cc65220 -- tests/envelope_command_position.rs
  tests/envelope_config_resolution.rs` returns **nothing**, and both binaries ran
  green in the gate (`envelope_command_position` 18, `envelope_config_resolution`
  30, both at zero failures). `tests/envelope_command_position.rs:520-551` still
  asserts all four `T-19-86` spellings PERMITTED, and
  `tests/envelope_config_resolution.rs:1539-1548` still asserts the persisted
  `!` pair PERMITTED in both calls.
- **The `glab --host` forge cell — reproduced, and its caveat is unchanged.**
  `glab --host gitlab.com mr create --title x` → exit 0 with **ZERO** ledger
  lines; `glab mr create --title x` → one line. `command -v glab` finds
  **nothing** — glab is confirmed NOT installed — so audit 9 does **not** upgrade
  it either. `gh` re-swept clean: `gh pr create --title x`, `gh --repo o/r pr
  create --title x`, `gh api repos/o/r/pulls -f title=x -X POST`,
  `gh api -X POST repos/o/r/pulls` and `gh api --hostname h repos/o/r/pulls -f
  title=x` each leave exactly one ledger line.
- **`T-19-SC` — still holds.** Neither `Cargo.toml` nor `Cargo.lock` appears in
  any commit between `fb43577` and `cc65220`.
- **`T-19-17r` — still OUTSTANDING, and audit 9 does not resolve it either.**
  The Accepted Risks Log runs `AR-19-01` … `AR-19-12`; there is **no `AR-19-13`
  row and no register row**, for the fifth audit running, and
  `grep -c "^| AR-19-13"` is **0**. Plans `19-24` and `19-25` both correctly
  declined and neither applied the word. **Accepting a risk is a human decision
  and this audit does not make it.** The next round either adds the log row or
  drops the word from `19-17-SUMMARY.md`.

### The fail-open residues — the admission is complete, nothing claims the witness is a control, and one direction is mis-labelled

**Nothing anywhere claims the version witness is a control, and audit 9 looked
for it.** `REPARSED_COMMAND_SECTIONS`'s doc (`policy.rs:1207-1215`), the witness's
own body (`policy.rs:8746-8776`), its failure message (`:8803-8807`) and
`INDIRECTION_SECTIONS`'s doc all open the point with **"a SCHEDULE, NOT A
CONTROL"**, state that it observes exactly one bit, state that it stays green on
a git that adds a section without changing its version string, and state that the
no-control claim stands unchanged beside it. The witness `.expect()`s a real
`git` and **cannot skip, warn-without-failing or pass when git is absent** —
verified by reading it. Its cost is stated in the code: it fires on every git
upgrade, including harmless ones, **and that is the schedule**. The provenance of
an earlier draft that read as the witness BEING the control, caught by a
plan-check, is carried in three places.

**Judgement: two of the three residues are acceptable and properly bounded; the
first is not a residue at all.**

- **Direction (iii) — a future git re-parsing a second config value — is
  acceptable and properly bounded.** It is a single closed proposition about one
  external dependency, audit 9 independently confirmed against git 2.43.0 that
  `alias` is the only member by sweeping twenty-one keys, the constant is
  therefore correct rather than merely current, and it now has the revisit
  condition and the schedule `AR-19-03`'s shape calls for — which is the one gap
  audit 8 named and round 9 closed.
- **Direction (ii) — a `!` body carrying its own carrier — is `T-19-86`, correctly
  attributed, measured still destructive, and open at `high` by user decision.**
- **Direction (i) is NOT a residue of a FUTURE git or of an out-of-scope threat.
  It is round 9's own class, reachable today, by the agent, during the run, on
  two permitted tool calls, and it rewrote a bare remote's `main`.** Labelling it
  *"`T-19-86`'s shape"* in the constant's doc, the record and the SUMMARY, and
  narrowing it to *"predating the run"* in `cred.rs`, is the fourth instance of
  the shape `T-19-84`, `T-19-107` and `T-19-109` all name — a residual
  disclosure whose words are pinned to exist rather than to be true. That is
  `T-19-111`, and it is why the remaining risk is **not** exactly the disclosed
  residual set.

### The four execution-time judgements, assessed independently

1. **The value test reads the CARRIER, not just the first byte — CORRECT, and
   the boundary is clean.** The measurement that forced it reproduces:
   `git --config-env=alias.q='!EVIL'` with a non-`!` body in the variable resolves
   `/INCLUDE_WINS` against real git, so a uniform first-byte rule reads the `!` of
   a variable NAME as git's shell rule and **fails open on exactly that row**.
   `reparsed_command_assignment_is_a_shell_body(token, assignment)`
   (`policy.rs:566`, `:1347`) takes the carrier from `argv[index]` — the token
   `scan_leading`'s loop already holds — so the claim of no new reading site is
   structurally true, and `git diff` shows no hunk in
   `first_unreadable_decision_word`, `config_key_operand_index`,
   `subcommand_word_indices`, `scan_gh_api` or `resolve_program_with_head`.
   **Probed at the boundary in seventeen spellings: reading the carrier opens
   nothing the first-byte rule closed**, and it closes one row the first-byte
   rule fails open on. Every refusal it adds beyond the first-byte rule
   (leading-space, TAB, quoted body) is a row real git refuses to expand anyway,
   so the direction is fail-closed with no harm behind it.
2. **Region 2 gates on `!is_read` plus a VALUE WORD rather than `is_write` — the
   substitute is SOUND and it is NOT wider than needed.** The unsatisfiability is
   real and audit 9 re-derived it rather than accepting it: `scan_config`'s walk
   collects only words that do not begin with `-` (`policy.rs:1778`), so
   `is_write`'s classic-form test `key_operand_count() >= 2` (`:1862`) is FALSE
   for `T-19-108`'s headline persisted row, whose value begins `-c` — a clause
   gated on `is_write` would have been blind to the one row it exists for. The
   substitute is narrow in both directions: `!is_read` keeps `git config --get
   alias.p`, `--list`, `-l` and `--get-regexp alias` permitted (all measured exit
   0), and requiring a VALUE WORD keeps the bare read `git config alias.p`
   permitted (measured exit 0) while catching every write spelling including
   `--global`, `--worktree`, `--file`, `--add` and `--replace-all`. **And it
   does not inherit `T-19-110`**: `git config alias.p -c`, `--`, `-q` and
   `-c include.path=…` are all exit 2, because `value_word` is indexed off
   `key_operand`'s own index into the same walk rather than off the option
   filter. Right substitute, right width.
3. **The probe error recorded rather than hidden — the arm is RIGHT.** A `!` body
   carrying its own `-c include.path=` resolving `/INCLUDE_WINS` is not a
   contradiction of the `!` carve-out; it is fail-open direction (ii), and audit 9
   reproduced BOTH halves independently in the fixture: the `!` alias with no
   carrier is **refused by the hook** and moves nothing (the child inherits, which
   is the K2 fact), and the `!` alias carrying its own carrier **rewrites the
   remote's `main`** (which is `T-19-86`). Splitting them into two pinned arms is
   the correct repair, and recording the error is the behaviour that stopped a
   fail-open direction being asserted as an inheritance fact.
4. **`T-19-110` registered rather than fixed — the DEFERRAL is right, the
   SEVERITY was not, and audit 9 confirms both reproducers and finds four more.**
   The deferral reasoning is exactly this phase's discipline and audit 9 endorses
   it without reservation: widening `is_write` moves verdicts for keys outside
   the round's class with **no corpus able to fail on them**, and this phase has
   been punished six times for certifying a claim its corpus could not have failed
   on. Recording the rows rather than asserting them — asserting PERMITTED pins a
   bypass as correct, asserting REFUSED pins a verdict the round did not produce —
   is `19-22`'s lesson correctly applied. **What the plan did not do is measure
   the harm**, and the harm is nil: every reachable write loses the precedence
   contest against the envelope's own injection, at repo-local and at global, and
   the force push after both is still refused by the hook. `high` was assigned by
   analogy to the by-name deny rather than by measurement; `medium` is what the
   evidence supports. See the re-rating above.

### Gates observed

`rtk proxy cargo test --no-fail-fast` redirected to a file and counted with
**`rtk proxy grep`** (a plain `grep` is rewritten by the RTK hook, which strips
the `test result:` lines a count is read from — D-34): **1720 passed, 0 failed,
13 ignored** over **44** result lines, matching `19-25-SUMMARY.md` exactly.
**All FIFTEEN `envelope_*` binaries ran** — `advisory` 10, `argv_deletion` 20,
`callee_grammar` 19, `command_position` 18, `config_resolution` 30, `credential`
6, `expansion_slots` 32, `hook_refusals` 7, `literal_decision` 43, `pr_cap` 11,
**`reparsed_value` 34**, `tracer` 6, `wiring` 14, `wrapper_bypass` 13,
**`wrapper_class` 41** — every one at **zero failures**, and the per-binary counts
match the SUMMARY's table row for row. So round 9's own evidence file executed.

**None of the three documented flakes fired in this run.** Audit 8 recorded one
firing; audit 9 records none. **Absence is not evidence they are fixed**, and one
firing would not have been evidence of a new defect. `cargo clippy --tests -- -D
warnings` was already failing at the base on four pre-existing lints in
`src/browser.rs` and `src/project_creator.rs` and is out of scope.

**Provenance discipline.** Plans 19-13 … 19-25's appended subsections, the
plan-19-21 blocker-row resolution record and every earlier audit's own tables are
left **byte-identical**; audit 9 checksummed the whole body below the frontmatter
before writing and verified afterwards that the write is append-only apart from
the frontmatter, one trail row and one method subsection. Audit 9's corrections
to statements made in those subsections — the `T-19-86` attribution of fail-open
direction (i) and `T-19-110`'s severity — are recorded as audit-9 findings BESIDE
them rather than as edits to them.

---

## Audit 9 — what the round-9 control can and cannot fail on

### The principle rounds 3 through 8 established still holds

A decision region must be derived from the same scan the classifier runs, and
there is one walk. `19-25` added region 1 **inside** `scan_leading`'s existing
assignment block and returned it through the `(usize, Option<GitVerdict>)`
channel that already carried three refusals, and region 2 **inside**
`classify_config`'s existing `!is_read` key-operand block beside the
`is_hooks_path_key` deny that already reads that operand. No third reading site
was created: `src/envelope/hooks.rs` has **zero** diff lines over the whole
round, no `ParkReason` variant was added, and none of the five index primitives
has a behavioural hunk. `SEPARATORS` is byte-identical and `is_separator(">")` is
`false`. The two ordering pins at deliberately different identifiers are the
mechanical proof the new clause is raised inside the one left-to-right walk.

### Where the boundary now is, in one paragraph

**Rounds 5 through 9 together decide five questions and audit 9 finds all five
answered.** A decision word must be provably LITERAL; the words the guard
classifies must be exactly the words the program receives in the same order; the
word the guard calls the verb must be the word git calls the verb; an assignment
the guard cannot BOUND to the key it names makes the command unresolvable; and a
value whose key names a section git re-parses as a command line is one the guard
cannot bound either. **What none of the five asks is how the value got there.**
All nine rounds read an argv. The controls do not:

- **a config value delivered by writing a FILE** — `.git/config`, written by
  `printf`, `sed`, `tee` or `python3`, carrying the identical non-`!` K1 body
  round 9 closed at the argv carriers; measured end to end, it rewrote a bare
  remote's `main` (`T-19-111`);
- **the cap's own record, which is a file** — deletable and truncatable by a
  permitted command whose path the guard hands over through a permitted read,
  with the reset driven end to end (`T-19-112`);
- **layer 3's own stub, which is a file** — replaceable without touching
  `GIT_CONFIG_COUNT`, which is the route D-09's ceiling does not name
  (`T-19-113`);
- **a whole command line handed to a governed program as data** — `T-19-86`,
  unchanged and still destructive;
- **a classifier arm's own operand or flag outside the decision region** —
  `T-19-91`, `T-19-96` and `T-19-110`, all registered, all still permitted.

**The one-sentence version for the next round.** Nine rounds have modelled what a
command line becomes; **not one has asked what happens when the carrier is not a
command line at all**, and all three of this phase's own controls — the alias
config, the cap ledger and the hook stub — are files the agent may write with a
command the guard has no opinion about.

### Suggested closure, in order — (d) FIRST, for the ninth round running

1. **(d) — widen the corpus BEFORE certifying anything, and this time widen its
   KIND rather than its alphabet.** Every alphabet in nine rounds draws a governed
   command line. Add a class whose entries are **non-governed commands whose
   OPERAND is a control carrier** — `printf … >> .git/config`, `rm -f
   <envelope>/<alias>/pr-ledger.ndjson`, `cp /bin/true <hooks>/pre-push`, in the
   `>>`, `sed -i`, `tee -a` and interpreter deliveries — with a floor beside
   `MIN_CONFIG_RESOLUTION_CLASSES`. **Listed first for the ninth round running**,
   and for the ninth round running the previous audit's recommendation was right
   about its own cell while the next gap sat somewhere it did not name.
2. **(a) — `T-19-112` FIRST among the fixes, because it is the cheapest and the
   claim it falsifies is an affirmative one.** The guard already refuses a command
   whose operand names an envelope key; refusing a command any of whose word
   operands resolves under the envelope root is the same shape one level over, and
   its over-refusal cost is a single legible family the agent has no reason to
   touch. Pin it from both sides: the ledger and hook paths refused, an ordinary
   `rm -f /tmp/x` and every existing permitted row unchanged. Then either the
   `Guaranteed` clause in `SECTION_ENVELOPE` gains the caveat or the cap gains the
   protection — the current pair cannot both stand.
3. **(b) — `T-19-111`, and the honest first step is the ATTRIBUTION, not the
   rule.** Move fail-open direction (i) out of `T-19-86` in
   `REPARSED_COMMAND_SECTIONS`'s doc, the `19-25` record and `19-25-SUMMARY.md`,
   restore the *"or one written by a means that is not `git config`"* half in
   `cred.rs`, and register it at its own severity — because a live non-shell
   bypass filed under a threat the user scoped out is a threat that stops being
   counted. The rule itself is genuinely hard for a stateless argv-only guard and
   may honestly end in an acceptance; **that acceptance is a human decision and
   this audit does not make it.**
4. **(c) — `T-19-113`**: either defend the stub's contents (re-assert the stub's
   own bytes at run start, or have the binary refuse to certify a `pre-push` it
   did not write) or correct D-09's ceiling narrative so it names the route that
   works rather than the route the guard refuses.
5. **(e)** — then `T-19-86`, `T-19-91`, `T-19-96` and `T-19-110`, which remain
   four arms of one shape: a classifier arm answering `Allow` on an operand, flag
   or payload outside the decision region.

Then, and separately: add the `AR-19-13` row for `T-19-17r` or stop calling it
accepted; fix `policy.rs`'s stale proportional-floor comment, now two rounds
stale (it cites 228,785 / 78.7%; the production half is 261,387); and get a
machine with `glab` installed to settle the `--host` cell against its callee.

---

## Audit 9 Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log — twelve, unchanged by
      round 9; audit 9 accepts nothing new
- [x] Every closure re-measured against the built binary at `cc65220` with a
      fresh envelope root per row and the root walked afterwards
- [x] Every new finding confirmed against the **REAL `git` binary**
      (`git version 2.43.0`) in a rebuilt bare-remote fixture, with a CONTROL
      beside every leg — including a `.git/config` alias written by `printf >>`
      that **rewrote the bare remote's `main` (`e86471e` → `3027440`)** beside
      the same alias without the carrier, which the hook REFUSED
- [x] The envelope's own delivery mechanism used as the control in every
      precedence measurement (`GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n`, the triplet
      `cred::hooks_path_env` emits), not a stand-in
- [x] `alias` re-derived independently as the ONLY K1 section by sweeping
      twenty-one config keys — including `core.pager` and `core.editor`, the two
      `19-25` explicitly did not claim — every other one INERT
- [x] The complete `git` global-option grammar swept in both spellings (38 rows,
      37 refused, the one permit measured CORRECT against real git) and the push
      operand grammar swept in 22 destructive spellings, all refused
- [x] The PR-cap reset driven END TO END: cap fires on call 2, the deletion is
      permitted, the next call is permitted again with a fresh ledger
- [x] `T-19-110`'s reproducers confirmed, four more found, and the harm measured
      INERT at repo-local and global — re-rated `high` → `medium` on measurement
- [x] The PR-cap walk proved non-blind by a positive control on every pass
- [x] `SEPARATORS` byte-identical (one commit in the phase has ever touched that
      line) and `is_separator(">") == false`; rounds 4 through 8's mechanism pins
      re-measured non-dead
- [x] The byte floors re-derived: `policy.rs` production half 261,387 / 40,000
      and 180,000; `hooks.rs` 74,358 / 20,000; exactly ONE `#[cfg(test)]`
      sentinel per file by the stripper's own trimmed-equality predicate
- [x] `cred.rs`'s named narrow exception verified: **zero non-doc lines changed**
- [x] The two `T-19-86` rows in fenced files GREEN with **zero diff lines** over
      `fb43577..cc65220`
- [x] Nothing anywhere claims the version witness is a control — four sites read,
      all four open with "a SCHEDULE, NOT A CONTROL"; the witness cannot skip
- [x] Plans 19-13 … 19-25's appended subsections and the blocker-row resolution
      record left byte-identical, verified by checksum before and after writing
- [x] Gate observed: `rtk proxy cargo test --no-fail-fast` → **1720 passed, 0
      failed, 13 ignored** over 44 result lines; **all FIFTEEN** `envelope_*`
      binaries ran, all at zero failures; no documented flake fired
- [ ] `threats_open: 0` confirmed — **4 open at `high`: `T-19-86`, `T-19-91`,
      `T-19-111`, `T-19-112`**
- [ ] `status: verified` set in frontmatter

**Approval: blocked, 2026-09-04 (audit 9).**

**Not accepted here.** `T-19-111` is round 9's own class reached through a
carrier outside argv, confirmed by a real push that rewrote a bare remote's
`main` with a control beside it. `T-19-112` is a complete reset of SAFE-06 — a
control with no second carrier, listed under `Guaranteed` in the honesty
statement — driven end to end through the guard on commands it permits.
`T-19-86` and `T-19-91` remain open at `high` by scoping decision and by round
discipline respectively. Accepting any of the four is a human decision and this
audit does not make it.

**Round 9 did what it said, and it did it well.** `19-24` wrote a sixth class on
the existing axis that demonstrably failed before any production line moved —
two `tests/`-only commits, zero `src/` hunks, eight RED rows — and it caught
git's one-byte shell-alias rule at PLAN time and fenced it mechanically rather
than leaving it to care, which is the difference between this round and the two
that halted mid-plan. `19-25` answered the design question with option (b) and
**costed option (a) by measurement rather than by preference**, including the
depth-3 row that turns any stated depth bound into a residue an attacker reaches
today. It found by measurement that the value test must read the carrier, that
the plan's mandated gate was unsatisfiable and why, and it corrected both without
editing an evidence row. It closed `T-19-106` and `T-19-109`, replaced a counted
completeness claim with a statement of REGIONS, gave both config-section
constants the revisit condition audit 8 asked for, and said in four places that
the witness is a schedule and not a control. It made a finding it did not fix and
registered it. **The rule is right and audit 9 says so plainly.**

**The remaining risk is NOT the disclosed residual set, and the reason is one
word.** The residues are disclosed — but direction (i) is filed as
*"`T-19-86`'s shape"*, and it is not. It is round 9's own non-shell K1 body,
reachable today, by the agent, during the run, on two permitted tool calls,
through a carrier the guard cannot see because **the carrier is a file**. The
same sentence is true of the PR-cap ledger and of the `pre-push` stub. Nine
rounds have modelled every way a command line becomes behaviour, and each one was
a real layer honestly closed. **The tenth question is not about a command line.
It is: what are this envelope's own carriers, and which of them can the thing it
governs simply overwrite?**

## Execution record — plan 19-26 (the corpus, round 10). NOT an audit finding.

**This subsection is appended by plan 19-26's executor. It edits nothing that
precedes it — no audit table, no Security Audit Trail row, no Accepted Risks Log
row, no Sign-Off, and no earlier appended subsection, including the plan-19-21
blocker-row resolution record and the plan-19-22 … 19-25 records.** Audit 9
checksummed the body below the frontmatter before writing; that provenance is
this file's value, and a plan writing into an audit's own tables forges it. **The
plan-19-25 record's `T-19-86` mis-attribution is corrected BESIDE it, here, and
never by editing it.**

**STATED FIRST: `/gsd-secure-phase 19` IS NOT CLEARED BY THIS PLAN, BY `19-27`,
OR BY THE TWO TOGETHER.** `T-19-86` remains OPEN at `high` by explicit user
scoping decision. `T-19-91` remains OPEN at `high` with its arms unweakened.
`T-19-111` and `T-19-112` are open at `high`. `T-19-96` and `T-19-110` are open
at `medium`. `T-19-61` … `T-19-73`, `T-19-84` and `T-19-85` are open and
unaccepted by explicit user decision. **This plan closes NOTHING**, and only the
WRAPPER-OPERAND sub-class of `T-19-60` is closed.

### The invariant this round is about, stated once

> **EVERY CONTROL THIS ENVELOPE INSTALLS HAS A *FILE* CARRIER THAT NO COMMAND
> LINE HAS TO TOUCH.**

Rounds 1-5 asked how a word is WRITTEN; round 6 which words ARRIVE; round 7 which
arriving word is the VERB; round 8 what the verb RUNS UNDER; round 9 what git
does with a VALUE the verb was handed. Each was a real layer honestly closed, and
audit 9 verified every row of round 9's. **All five read an ARGV.** Round 10's
question is what happens when the carrier is not a command line at all.

The architectural fact the whole round turns on is the guard's own, at
`hooks.rs:983-991`:

> **`Ungoverned` is the ANSWER here, not a fall-through** … The resolver has now
> looked at every command position in the segment and found nothing this envelope
> governs. Permitting is correct — this guard is registered against every Bash
> tool call, and a rule that denied what it did not recognise would deny `ls`,
> `cargo test` and `rg`, which is how a safety control gets switched off.

**That paragraph is right, and it is also the whole of `T-19-112` and
`T-19-113`.** `rm -f <ledger>`, `cp /bin/true <hooks>/pre-push` and
`printf … >> .git/config` all land on the `NoProgram | Ungoverned => continue`
arm at `hooks.rs:992-994`. Nine rounds have made the GOVERNED half of that
boundary airtight; not one has asked whether the UNGOVERNED half should be
unconditional. **The answer this round gives: unconditional in the PROGRAM it
names, but not in the PATHS it names — because some of those paths are this
envelope's own controls.**

### The EXHAUSTIVE carrier enumeration — the round's central product

Fifteen live carriers, two stated non-carriers, one covered ENV family. Derived
from the source and measured against the built binary at `406c48e`, one fresh
`GSD_MM_ENVELOPE_ROOT` per row with the whole root walked afterwards. Rows marked
**PD** are *planner-derived* — measured and **RECORDED**, never asserted.

| # | Carrier | Defining source | Layer / control | Measured guard verdict (write / read) | Walk | Observed consequence |
|---|---|---|---|---|---|---|
| C-01 | `<env>/<alias>/pr-ledger.ndjson` | `ledger.rs:47`, `:95-97` | 2 — SAFE-06, D-19/D-20 | `rm -f` 0, `truncate -s 0` 0, `cp /dev/null` 0, `shred -u` 0 / `cat` 0, `wc -l` 0 | EMPTY | **OBSERVED**: cap resets. Driven end to end over four calls below (`T-19-112`) |
| C-02 | `<env>/<alias>/hooks/pre-push` | `hooks.rs:55`, `:127-149` | 3 — SAFE-05, `AR-19-03`'s backstop | `cp /bin/true` 0 | EMPTY | **OBSERVED end to end**: a force push the hook refused COMPLETED and MOVED a bare remote's `main` `8ef079d → f2d230f`, with `GIT_CONFIG_COUNT` UNTOUCHED. Restoring the stub restored the refusal (`T-19-113`) |
| C-03 | `<env>/<alias>/hooks/pre-commit` | `hooks.rs:56` | 3 — D-22 sweep | `cp /bin/true` 0 | EMPTY | **UNOBSERVED** — same shape as C-02; the sweep boundary was not driven and is not inferred |
| C-04 | `<env>/<alias>/hooks/` | `hooks_dir_in`, `:83-85` | 3 | `rm -rf` 0 | EMPTY | **OBSERVED by construction**: both stubs at once |
| C-05 | `<env>/<alias>/gitconfig` — `GIT_CONFIG_GLOBAL` **and** `GIT_CONFIG_SYSTEM` | `cred.rs:81` | SAFE-05, D-16/D-18 | **PD** — `printf … >>` 0, `cp` 0 | EMPTY | **OBSERVED against real git**: control `/ENV_WINS`; a non-`!` `[alias]` body carrying `-c include.path=<f>` resolves **`/INCLUDE_WINS`** — `T-19-111`'s K1 body at the GLOBAL level, never touching `.git/config`; and `[credential] helper = /tmp/evil-helper` reads back through `git config --get credential.helper`, **falsifying `SECTION_ENVELOPE`'s FIRST `Guaranteed` clause** (`advisory.rs:245-247`) |
| C-06 | `<env>/<alias>/askpass` | `cred.rs:49` | SAFE-05, D-17 | `cp /dev/null` 0 / `echo evil >` 0 | EMPTY | *Reading* it is already disclosed by `SECTION_ENVELOPE`; **OVERWRITING it was unmeasured before this round and is measured permitted here.** The behavioural consequence of a substituted responder is **UNOBSERVED** |
| C-07 | `<env>/<alias>/gh/` | `cred.rs:83-91`, `GH_CONFIG_DIR` | SAFE-05 | **PD** — `printf … >` 0, `cp` 0 | EMPTY | **OBSERVED against the installed `gh` 2.45.0, offline**: with the directory empty (as `cred.rs` leaves it) `gh auth status` reports *"You are not logged into any GitHub hosts"*; with a `hosts.yml` written it reports the host `example.invalid` with *"Active account: true"*, reading the token from the written file. **UNOBSERVED, deliberately**: whether a VALID token would be honoured — a test may not supply one |
| C-08 | `<env>/<alias>/settings.json` | `hooks.rs:1326`, `:1492` | 1 + 2 — D-07 delivery | **PD** — `printf … >` 0, `cp` 0 | EMPTY | **MECHANICAL half OBSERVED, BEHAVIOURAL half UNMEASURED** — see its own subsection below |
| C-09 | `<env>/<alias>/` itself | `mod.rs:221-226` | 1, 2, 3 | `rm -rf` 0 | EMPTY | **OBSERVED by construction**: C-01 … C-08 in ONE call — **which is the argument for making the ROOT the boundary rather than naming eight files** |
| C-10 | the binary the stubs and `guard_command` exec | `hooks.rs:196-204`, `:1454-1460` | 2, 3 | `cp /bin/true ./target/…` 0 | EMPTY | **UNOBSERVED** — replacing the exec target's bytes was not driven. `T-19-01` covers a RELOCATED COPY of the stub, not the target's bytes. The `envelope_tracer` `ExecutableFileBusy` flake is this seam |
| C-11 | `<project>/.git/config` | git's own resolution | the alias carrier | all eight write spellings 0, `git fp` 0 | EMPTY | **OBSERVED against real git with a bare remote and a control beside each leg** — `T-19-111`, table below |
| C-12 | `<project>/.claude/settings.json` | `policy.rs:2063-2064` | 1 — `--setting-sources project` | **PD** — `printf … >>` 0, `sed -i` 0 | EMPTY | Layer 1 denies `Write(.claude/**)` and `Edit(.claude/**)` as **TOOL patterns**; layer 1's Bash patterns are git verbs and layer 2 permits a segment reaching no governed program, **so the Bash route is denied by NEITHER layer.** Whether the loaded project-tier file makes that reachable in practice is **UNMEASURED** (it needs a live agent run) |
| C-13 | `<project>/.git/info/exclude` D-23 block | `hooks.rs:68-69`, `:558-561` | the sweep's ignore block | `rm -f` 0 | EMPTY | **UNOBSERVED** — the block's removability is a filesystem fact; the sweep consequence was not driven |
| C-14 | `<project>/.planning/meta-manager/runs/<run_id>/` | D-24/D-25 | park **EVIDENCE** | `rm -rf` 0 | EMPTY | **REPUDIATION, never ELEVATION.** `ParkOutcome`'s own doc (`mod.rs:243-249`): *"this type describes the **evidence**, never the decision (D-25, SAFE-06)"*. Deleting it destroys the record of refusals (`T-19-56`) without softening one |
| C-15 | `~/.config/gsd-meta-manager/config.json` | `config.rs:330-335`, read by `resolve_policy` `hooks.rs:1239-1258` from `Config::default_path()` at `:837-840` on **every Bash tool call** | 2 — SAFE-01 namespace and **SAFE-06 caps** | **PD** — `printf … >` 0, `cp` 0 | EMPTY | **OBSERVED end to end** — the PR cap disarmed by RAISING A NUMBER, with **no ledger write and no envelope path named**. Its own subsection below |
| N-01 | `.git/hooks/` | — | — | — | — | **NOT a carrier BY CONSTRUCTION.** D-09 declines it, and that refusal is the whole point of the env-injected delivery. Stated so the enumeration is exhaustive rather than selective |
| N-02 | `--disallowedTools` | `claude.rs:286-289` | 1 | — | — | **NOT reachable** — it is argv in the PARENT's spawn. The one layer-1 carrier the child cannot reach |
| E-01 | the ENV handles — `GIT_CONFIG_*`, `GIT_ASKPASS`, `GIT_SSH_COMMAND`, `GH_CONFIG_DIR`, `SSH_AUTH_SOCK`, `SSH_AGENT_PID`, `GSD_MM_ENVELOPE_PROJECT_ROOT`, `GSD_MM_RUN_ID`, `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_NOSYSTEM` | `policy.rs:3595-3685` | all three | — | — | **ALREADY MODELLED** by `ENVELOPE_ENV_KEYS` and `tampers_with_envelope_env`. Not this round's plane. **`GSD_MM_ENVELOPE_ROOT` is deliberately NOT in that list** (`mod.rs:165-168`) and is measured **INERT**: `GSD_MM_ENVELOPE_ROOT=/tmp/fresh gh pr create --title x` is exit 0 and the ledger line lands under the root the **GUARD** was given, not under the one the prefix names — **the walk is the proof** |

### `T-19-111` — the ATTRIBUTION CORRECTION, required REGARDLESS of whether the fix lands

Audit 9's own suggested order puts it plainly: *"the honest first step is the
ATTRIBUTION, not the rule"*.

`printf '[alias]\n\tfp = -c include.path=<evil> push --force origin HEAD:refs/heads/main\n' >> .git/config`
is exit 0 with an EMPTY walk — it is not a git command and the guard has no
opinion about it — and `git fp` is exit 0, because the guard is stateless and
cannot see an alias it did not watch being defined.

**THE BODY CARRIES NO `!`.** This is `T-19-108`'s own K1 class reached through a
carrier OUTSIDE ARGV, **not** a governed command handed to a governed program as
data. `T-19-86`'s four registered rows — `git submodule foreach git push --force
origin main`, `git rebase -x "git push --force origin main" HEAD~3`, `git bisect
run sh -c "git push --force origin main"`, `git -c alias.p='!git push --force
origin main' p` — all require the latter. **A plain git command line sitting in a
config value is not that.**

**THE FIVE MIS-ATTRIBUTION SITES, ENUMERATED BY FILE AND LINE:**

1. `src/envelope/policy.rs:1177-1183` — `REPARSED_COMMAND_SECTIONS`'s fail-open
   direction (i). It DOES disclose the residue — *"a repo-local `.git/config`
   alias predating the run, **or one written by a means that is not `git
   config`**, is LIVE"* — and then calls it *"`T-19-86`'s shape"*. `19-27`'s.
2. `src/envelope/cred.rs:287-289` — the reach paragraph, which narrows it further
   to *"a repo-local `.git/config` alias **predating the run** is live"*,
   **dropping the written-during-the-run half entirely.** The half must be
   RESTORED. `19-27`'s.
3. `19-SECURITY.md:7430` — inside the plan-19-25 record. **Corrected BESIDE, here,
   never by editing it.**
4. `19-25-SUMMARY.md:265` — same. **Corrected BESIDE, here, never by editing it.**
5. `deferred-items.md:1619` — the `T-19-108` closure entry, whose *"CLOSED AS
   SCOPED"* names the excluded BODY (`!`) and not the excluded CARRIER (a file).
   Corrected in this plan's own `deferred-items.md` section.

**Crediting a live, non-shell, measured destructive bypass to a threat the user
has explicitly scoped OUT is how it stops being counted.** `T-19-111` is
registered SEPARATELY at `high` and **must be moved OUT of `T-19-86`, not into
it.** No acceptance is made — that is a human decision.

### The design answer — option (d), a SPLIT, with the direction each choice FAILS

**(c) — move a carrier out of reach — is DEAD for every one of them, and saying
that once is worth more than fifteen denials.** Every envelope carrier must be
readable, writable or executable by the same uid the driven process runs as,
because git must exec the hooks, the agent CLI must read the settings and the
guard must append the ledger. There is no location on a single-uid model the
driven process cannot also reach. A different uid, a container, or a remote-side
control are the only real (c)s; the first two are outside this phase and the
third is what `SECTION_ENVELOPE`'s own conclusion already says.

| Carrier group | Control | Why it differs | Direction the choice FAILS |
|---|---|---|---|
| **C-01 … C-09** — everything under `<envelope>/<alias>/` | **(a)** — refuse a segment any of whose ABSOLUTE LITERAL word operands resolves LEXICALLY under the envelope root | The guard already HOLDS the root: `classify_segments(segments, depth, root, alias, …)` (`hooks.rs:954-961`) takes both, and `envelope_dir_in` (`mod.rs:221-226`) is a validate-then-`join` with no filesystem probe. **No git, no repo, no `PATH` lookup, no `canonicalize`, no program-name list** — so it dodges `wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic` and the TOCTOU/latency prohibitions in one move. One rule covers nine carriers, which is why the ROOT is the boundary rather than eight filenames | **FAILS OPEN, FOUR times, and its SILENCE IS A PERMIT in every one.** See below |
| **C-01 specifically**, as an *additional* option costed and **NOT taken** | **(b)** — tamper-evidence, **deferred with its cost stated and NOT scheduled** | The ledger is **append-only by design** (`ledger.rs:1-8`), so a deletion or truncation is detectable at the NEXT APPEND rather than prevented. The material exists: entries carry `run_id` and `at`. **But it would create the second carrier `T-19-35`, `AR-19-05` and `hooks.rs:1421-1428` all say SAFE-06 does not have**, and it would break a stated latency rule — *"The ledger read is a single pass over one append-only file"* (`hooks.rs:789`) | n/a — deferred. **Recorded as the honest narrowing of (a)'s residue and explicitly NOT scheduled, promised or handed to a pin** |
| **C-02 … C-04** — the hook stubs | **(a)**, and **(b) is structurally UNAVAILABLE** | `assert_provenance_in` already certifies a *relocated copy*. **But a REPLACED stub never re-enters the binary that would assert it** — `cp /bin/true pre-push` means the certifying code is precisely the code that no longer runs. Detection at run start is TOCTOU; detection at guard time is a file read per tool call against the same latency rule. **So prevention is the only control that can fire here, and the ledger's option (b) is genuinely not available to the stub.** *This is the round's sharpest design asymmetry* | (a)'s four directions. **And D-09's narrative is corrected either way** |
| **C-11 … C-15** — the repo-side and policy-input carriers | **(e)** — **NO RULE THIS ROUND.** Attribution correction, corpus rows RECORDED, registered at their own severity, **unaccepted** | None is under the envelope root. Widening (a) to the repo would mean the guard deciding about `.git/**` on every tool call with `project_root` an `Option`. **`C-15` is further out still**: it is the guard's OWN registry under `~/.config`, a path no envelope root contains. Their headline delivery is a REDIRECTION TARGET the deletion model deliberately does not read, so a rule naming them would be silent at exactly the measured row. And layer 3 — the only place `.git/config` could be observed without TOCTOU — **is disabled by the very carrier in question** | **FAILS OPEN ENTIRELY, and that is STATED rather than mitigated.** Registered, disclosed, handed to no control, **and no acceptance made — that is a human decision** |
| **`SECTION_ENVELOPE`'s `Guaranteed` cap clause** | **Advisory correction — mandatory EITHER WAY** | Not a control: an honesty repair. (a) narrows `T-19-112` and does not close it | n/a. **Shipping (a) and leaving the claim unqualified would be `T-19-107`'s defect committed by the round that inherited the lesson** |

**The one-line version.** The three carriers differ because one of them is
append-only and outside the repo (so it can be made tamper-*evident*), one of them
is the thing that would do the detecting (so it can only be *prevented*), and one
of them is inside the repo behind a redirection the guard has spent five rounds
deliberately not reading (so this round can only *tell the truth about it*).

#### Rule (a)'s FOUR fail-open directions — its SILENCE IS A PERMIT in every one

**NOT ONE is handed to a pin, a schedule or a witness.** This phase has twice
written a fail-open residue and once nearly handed it to a control that could not
observe it; a third would be committed by the round that inherited the lesson.

1. **(i) A REDIRECTION TARGET is not an operand — and this is real by MECHANISM,
   not by choice.** `tokenize` (`policy.rs:2264-2279`, `T-19-97`) consumes bash's
   `[IO_NUMBER] OPERATOR WORD` production and emits **no token for the operator OR
   ITS TARGET**, because bash deletes both before `execve`. The carrier path after
   a `>` is not a word at all. Reading `>` would re-open a model five rounds have
   pinned shut: `SEPARATORS` (`policy.rs:2281`) has **ONE commit in the whole
   phase** (`84a9b05`, plan 19-05), `is_separator(">") == false` by construction,
   and round 6's over-deletion control pins `git x2>/tmp/o push --force origin
   main` PERMITTED. **Measured live: `: > <ledger>` is exit 0 and, driven in one
   persistent root, RESETS THE CAP.**
2. **(ii) An EXPANSION-BORNE operand cannot be resolved.** `D=$(git config --get
   core.hooksPath); rm -f $D/../pr-ledger.ndjson` is exit 0. Refusing every
   non-literal operand of an ungoverned command would deny `rm $TMPDIR/x` and
   `cp "$SRC" "$DST"`, which is how a safety control gets switched off.
   **Measured while building the corpus and worth recording: this composite's
   FIRST segment resolves `Governed { index: 0 }` — the substitution really does
   run a governed `git config --get`, which is PERMITTED — while the segment that
   ACTS on the carrier reaches nothing the envelope governs. The carrier's
   location is fetched by a permitted governed READ and then acted on by an
   ungoverned command; that composition IS direction (ii).**
3. **(iii) A SYMLINK is not followed** — rule (a) resolves LEXICALLY and touches
   no filesystem. `..` is collapsed textually (measured: the LITERAL
   `<ENV>/alpha/hooks/../pr-ledger.ndjson` IS caught), but a link is not, because
   following one means `readlink`/`canonicalize` on the guard's critical path,
   which is the I/O `hooks.rs:771-816` forbids and the TOCTOU `mod.rs:196-203`
   forbids. **NARROWED, not open in every spelling**: `ln -s <ENV>/alpha/pr-ledger.ndjson /tmp/l`
   names an envelope path as its OWN operand and is refused after `19-27`. A link
   must PREDATE the run or be made by a means naming no envelope path.
4. **(iv) A RELATIVE path is not resolved** — the guard has no cwd. `guard_in`
   takes a `project_root: Option<&Path>` from `PROJECT_ROOT_ENV`, which is the
   JOURNAL locator and not the shell's working directory. **NARROWED**: `cd
   <ENV>/alpha` names an envelope path as its own operand and is refused, and the
   two-segment composite `cd <ENV>/alpha && rm -f pr-ledger.ndjson` is refused
   **BY SEGMENT 1** — segment 2's relative operand stays unresolvable, so a
   spelling that reaches the directory by any other means leaves it permitted.

### `SECTION_ENVELOPE`'s `Guaranteed` cap clause CHANGES EITHER WAY

The requirement is that the `Guaranteed` list be TRUE when this round ends.

> `"Pull-request cap: an append-only ledger this repository does not contain."`
> — `advisory.rs:250`

**Measured, four calls in ONE persistent root:** permitted (1 line), REFUSED at
`pr_cap_exceeded` (2 lines — the ledger appends BEFORE the verdict, D-20),
REFUSED (3 lines), deletion at exit 0, **call 4 PERMITTED AGAIN with a fresh
one-line ledger.** `hooks.rs:1421-1428`, `T-19-35` and `AR-19-05` all say SAFE-06
has **no second carrier**: *"One control genuinely has no git-hook counterpart …
the pull-request cap"*, whose remaining mitigations are the argv delivery and
`write_settings_in`'s round-trip check — **neither of which observes the ledger
file at all.** The guard is the only observation point and its record is
deletable by the process it observes.

**With rule (a) in place, `rm -f <ledger>` is refused and `: > <ledger>` is not**
(direction (i)), **and `C-15` resets the cap through a file outside the envelope
root entirely, with no ledger write at all.** So an unqualified `Guaranteed` stays
falsified by measurement either way. **The repaired text must NOT present rule
(a)'s four directions as the complete residue for SAFE-06.**

**The pinned-phrase ordering constraint, carried forward so `19-27` repairs the
content without breaking the honesty statement's own pins** (`advisory.rs:206-217`,
the three rules for the next editor): indent with `\x20` escapes and never with
literal spaces; keep the opening sentence clear of the later pinned phrases; and
**wrap AROUND a pinned phrase, never THROUGH one** — the longest is `defeatable by
an agent that can spawn an unsupervised`, at 53 characters. The pin test compares
FIRST occurrences and asserts `cannot reach your ambient git credentials` <
`defeatable by an agent that can spawn an unsupervised` < `enable server-side
branch protection`. That ordering is asserted, unchanged, in
`tests/envelope_control_carrier.rs`'s
`the_guaranteed_cap_clause_is_pinned_at_its_current_text_and_19_27_must_change_it`,
**which is the ONE named, bounded exception `19-26` grants to the no-edit rule on
its own file.** `19-27` updates that one test function and nothing else there.

### D-09's NARRATIVE DEFECT — recorded, and its correction required even if the control is deferred

`cp /bin/true $(git config --get core.hooksPath)/pre-push` is exit 0 and removes
layer 3 — SAFE-05's only carrier and the backstop `AR-19-03` explicitly rests on
(*"The pre-push and pre-commit hooks make the denylist's gaps non-fatal"*,
`19-SECURITY.md:1116`) — **without touching `GIT_CONFIG_COUNT` at all.**

`T-19-01`'s `current_exe()` + `assert_provenance_in` mitigation is present and
correct for a RELOCATED COPY of the stub, and a REPLACED stub never re-enters the
binary that would assert provenance. Audit 9 rated this `medium` rather than
`high` and said why: `SECTION_ENVELOPE` discloses that client-side hooks are
defeatable, and the trust-boundary table declares the filesystem write. **This
plan does not re-rate it.**

**What is WRONG is that D-09's own ceiling paragraph (`cred.rs:241-244`,
`mod.rs:167`, `advisory.rs:254`) names the route the guard REFUSES** — `unset
GIT_CONFIG_COUNT`, re-measured green at exit 2 `hook_bypass_blocked` — **while the
route that works is refused by nothing.** A reader of D-09 concludes the guard
stands underneath layer 3, and it does not.

### `AR-19-04`'s REASONING GAP — recorded, and explicitly NOT un-accepted

Its mitigation, verbatim (`19-SECURITY.md:1117`):

> the envelope regenerates it at each run start

**That does not cover a write DURING the run**, and the C-05 measurement above is
the write. This is the same *"predating the run"* narrowing audit 9 found in
`cred.rs:287-289`, in an acceptance rather than a doc.

**A THIRD INSTANCE OF THE SAME SHAPE, and naming it once is worth more than three
separate notes.** `resolve_policy`'s own doc (`hooks.rs:1233-1238`):

> **An unreadable registry or an unregistered alias resolves to the defaults
> rather than to an error**, and the direction is what makes that safe: every
> default is the *tighter* value — the reserved namespace and the 3/1 caps — so a
> guard that cannot read configuration confines the run more, never less.

**True for an ABSENT or UNREADABLE config; silent about a PRESENT AND WRITABLE
one.** `AR-19-04`'s *"regenerates at each run start"*, `cred.rs`'s *"predating the
run"* and this are one shape: **a reassurance reasoning about the case that is not
the threat.**

**`AR-19-04` and `AR-19-05` are RECORDED and NOT un-accepted. No `AR-` row is
added, edited or renumbered. Un-accepting a risk is a human decision and no plan
makes it.**

### `C-15` — a DISTINCT ROUTE to SAFE-06, not a spelling of `T-19-112`

**The shape of the miss is worth recording beside the finding.** The first
enumeration walked the envelope directory exhaustively and stopped at its edge, so
the plan-check's pattern fired **one TIER over**: the file that FEEDS the policy
is as much a carrier as the file that RECORDS it.

**Mechanical half.** `guard` passes `Config::default_path()` into `guard_in`
(`hooks.rs:837-840`), which calls `resolve_policy(config_path, alias)`
(`hooks.rs:1239-1258`) on **every Bash tool call**. `EnvelopePolicy::resolve`
(`policy.rs:2116-2139`):

```text
namespace:       validate_namespace(...) with a warn, else default   <- DEGRADES SAFELY
pr_cap_per_24h:  opt_in.pr_cap_per_24h.unwrap_or(DEFAULT_PR_CAP_PER_24H)   <- NO CLAMP
pr_cap_per_run:  opt_in.pr_cap_per_run.unwrap_or(DEFAULT_PR_CAP_PER_RUN)   <- NO CLAMP
credential:      opt_in.credential.clone()                           <- COPIES THROUGH
```

**Behavioural half, driven end to end in ONE persistent envelope root** with
`pr_cap_per_24h` and `pr_cap_per_run` both raised to `9999`:

| Call | Before | Exit | After | Reason |
|---|---|---|---|---|
| 1 | MISSING | **0** | 1 | (permit) |
| 2 | 1 | **0** | 2 | (permit) |
| 3 | 2 | **0** | 3 | (permit) |
| 4 | 3 | **0** | 4 | (permit) |

**CONTROL, same drive with the DEFAULT caps (3/1):** call 1 exit 0, calls 2, 3 and
4 exit **2** `pr_cap_exceeded`. **MID-RUN REPLACEMENT:** two calls at the defaults
(0, then 2), the config file replaced, and calls 3 and 4 **exit 0** — **the
per-call re-read observes a config the driven process replaced after the run
started.**

**THE FINDING: four permits, four ledger lines, NO DELETION ANYWHERE, and NO
ENVELOPE PATH NAMED.** The cap is disarmed by RAISING A NUMBER. **So rule (a)
cannot reach it — the file is not under the envelope root — and it gets control
(e): none.** `19-27` writes no rule for it. **Do NOT clamp `pr_cap_*` here: that is
a product decision about what a user may configure, and deciding it inside a
security round would decide it silently.**

**`Config::default_path()`'s RELATIVE `.config` fallback — MEASURED and NOT
REPRODUCED.** `config.rs:330-335` falls back to a RELATIVE `.config` when
`dirs::config_dir()` is `None`, which would put the guard's own registry under the
process's working directory. Driven with `HOME` and `XDG_CONFIG_HOME` **both
unset** and a raised-cap `.config/gsd-meta-manager/config.json` in the process's
cwd: the caps did **not** take, and an `openat` trace showed the guard opening
`/home/<user>/.config/gsd-meta-manager/config.json` — **`dirs` resolves the home
directory from the PASSWD DATABASE when `HOME` is unset, so the relative branch is
UNREACHED in this environment.** Recorded as measured-and-not-reproduced rather
than inferred from the source. **`HOME` alone (no `XDG_CONFIG_HOME`) DOES steer
the path, and raised caps supplied there DO take** — measured by `openat` trace
and by three permits.

### `C-08` — the claimed second carrier that has no production caller

The D-07 second-carrier table (`hooks.rs:1405-1428`) is the mechanism by which no
control in the settings file is ever SOLE-CARRIED, *"because the consumer ignores
an invalid settings file silently"*. Its deny-list rows are TRUE and mechanically
checked: `policy::disallowed_tools()` is the single source and `claude.rs:286-289`
really does push `--disallowedTools` as argv, pinned by
`every_pattern_the_settings_file_denies_is_also_carried_on_argv`.

**But the row for *the file's own delivery* names `settings_json` rendering the
value *"inline on argv"*, and the exact `grep` hit set over `src/` is TWO:**

```text
src/envelope/hooks.rs:1419  /// | the file's own delivery | [`settings_json`] renders the identical value for `--settings` to take **inline on argv**, …
src/envelope/hooks.rs:1467  pub fn settings_json(binary: &Path, alias: &str) -> anyhow::Result<String> {
```

— **the DEFINITION and the DOC ROW CLAIMING IT.** Production pushes `--settings`
with a **PATH** (`claude.rs:290-293`; `ExecutionOptions::envelope_settings:
Option<PathBuf>` at `executor/mod.rs:439`, wired at `driver/run.rs:1657-1664`).

**BEHAVIOURAL HALF: UNMEASURED.** Whether the already-spawned agent CLI RE-READS
its settings file mid-run is a property of a closed-source binary and cannot be
driven without a live session and a network call. `write_settings_in`'s read-back
(`hooks.rs:1484-1499`) runs at run START, before the child is spawned, so it
cannot observe a mid-run overwrite — **the "regenerates at run start" shape
again.**

**CONSEQUENCE IF THE HARM HOLDS:** the `PreToolUse` guard REGISTRATION — layer 2's
entire delivery — would be single-carried by a file inside the directory
`T-19-112` proved writable at exit 0. **Registered at the severity the MECHANICAL
half alone supports, with the harm stated as UNCONFIRMED. Claims in this codebase
about a second carrier have been wrong twice, so nothing is asserted in either
direction.**

### The measured rows, with their walks

Every row driven against the built binary at `406c48e`, one fresh
`GSD_MM_ENVELOPE_ROOT` each, the whole root walked afterwards. The walk's
non-blindness control is `gh pr create --title x`, which leaves **exactly one**
ledger line in a walked fresh root and fires the cap on call 2.

**The ENVELOPE-ROOT OPERAND rows — measured exit 0, asserted at their DERIVED
post-fix verdict (exit 2 `envelope_assertion_failed`), RED at this plan's end:**

```text
exit=0  rm -f <ENV>/alpha/pr-ledger.ndjson                    walk EMPTY   T-19-112
exit=0  truncate -s 0 <ENV>/alpha/pr-ledger.ndjson            walk EMPTY   T-19-112
exit=0  cp /dev/null <ENV>/alpha/pr-ledger.ndjson             walk EMPTY   T-19-112
exit=0  shred -u <ENV>/alpha/pr-ledger.ndjson                 walk EMPTY   program absent from src/
exit=0  rm -rf <ENV>/alpha                                    walk EMPTY   C-09, NINE carriers, one call
exit=0  cp /bin/true <ENV>/alpha/hooks/pre-push               walk EMPTY   T-19-113
exit=0  cp /bin/true <ENV>/alpha/hooks/pre-commit             walk EMPTY   C-03
exit=0  rm -rf <ENV>/alpha/hooks                              walk EMPTY   C-04
exit=0  cp /dev/null <ENV>/alpha/askpass                      walk EMPTY   C-06
exit=0  rm -f <ENV>/alpha/hooks/../pr-ledger.ndjson           walk EMPTY   the LITERAL `..`, collapsed lexically
exit=0  ln -s <ENV>/alpha/pr-ledger.ndjson /tmp/l             walk EMPTY   (iii)'s partial mitigation
exit=0  cd <ENV>/alpha                                        walk EMPTY   (iv)'s partial mitigation
exit=0  cd <ENV>/alpha && rm -f pr-ledger.ndjson              walk EMPTY   refused by SEGMENT 1
```

**The DISCLOSED OVER-REFUSAL family — the READS, refused as well as the writes,
and that is a DECISION.** The guard cannot tell a read from a write without
knowing every program's grammar — is `dd if=X of=Y` a read of `X` or a write of
`Y`? is `tee F` a read? — and an enumeration that tried would be back to a
program-name list, which
`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
mechanically forbids. **The cost: a run cannot inspect its own envelope
directory.** The permitted twin is that the path can still be NAMED.

```text
exit=0  cat <ENV>/alpha/pr-ledger.ndjson    walk EMPTY   -> exit 2 after 19-27
exit=0  wc -l <ENV>/alpha/pr-ledger.ndjson  walk EMPTY   -> exit 2 after 19-27
exit=0  ls <ENV>/alpha/hooks                walk EMPTY   -> exit 2 after 19-27
exit=0  git config --get core.hooksPath     walk EMPTY   -> UNCHANGED, pinned exit 0
```

**The FOUR FAIL-OPEN DIRECTIONS — pinned PERMITTED, verdict-PRESERVING, exit 0
before AND after:**

```text
exit=0  : > <ENV>/alpha/pr-ledger.ndjson                                    (i)
exit=0  printf 'exit 0' > <ENV>/alpha/hooks/pre-push                        (i)
exit=0  echo evil > <ENV>/alpha/askpass                                     (i)
exit=0  cat /tmp/evil >> <ENV>/alpha/gitconfig                              (i)
exit=0  D=$(git config --get core.hooksPath); rm -f $D/../pr-ledger.ndjson  (ii)
exit=0  H=$(git config --get core.hooksPath); cp /bin/true $H/pre-push      (ii)
exit=0  rm -f /tmp/l          (a symlink into the envelope root)            (iii)
exit=0  : > /tmp/l                                                          (iii)
exit=0  rm -f pr-ledger.ndjson          (a bare RELATIVE operand)           (iv)
exit=0  cp /bin/true pre-push                                               (iv)
```

**The NEAR-MISS controls — this round's `--signed no`, pinned exit 0 before AND
after.** A rule written as `path.ends_with("pr-ledger.ndjson")`, as a substring
test, or as a string-prefix test that does not compare resolved COMPONENTS turns
both RED:

```text
exit=0  rm -f /tmp/pr-ledger.ndjson                              basename, wrong directory
exit=0  cat <ENV-with-one-character-changed>/alpha/pr-ledger.ndjson   root, one byte off
```

**The ORDINARY-OPERAND permitted half — pinned exit 0 before AND after:**

```text
exit=0  rm -f /tmp/x    rm -rf /tmp/scratch    cp /bin/true /tmp/t    truncate -s 0 /tmp/f
exit=0  cat /tmp/x      ls                     cargo test            rg 'pr-ledger.ndjson' src/
```

`rg 'pr-ledger.ndjson' src/` is the NEGATIVE CONTROL that keeps the distinction
stated rather than assumed: the carrier filename is QUOTED, so it is a search
PATTERN and not an operand the command acts on. **A rule that refused it would be
reading TEXT rather than OPERANDS.**

**The REPO-SIDE rows — RECORDED and NEVER asserted, in either direction**, because
`19-27` writes no rule for them:

```text
exit=0  printf '[alias]\n\tfp = -c include.path=<evil> push --force …\n' >> .git/config
exit=0  git fp
exit=0  echo evil >> .git/config          exit=0  cat /tmp/evil >> .git/config
exit=0  tee -a .git/config < /tmp/evil    exit=0  sed -i s/x/y/ .git/config
exit=0  python3 -c "open('.git/config','a').write('x')"     exit=0  cp /tmp/evil .git/config
exit=0  printf x >> .claude/settings.json exit=0  sed -i s/deny/allow/ .claude/settings.json
exit=0  rm -f <project>/.git/info/exclude
exit=0  rm -rf <project>/.planning/meta-manager/runs/<run_id>
exit=0  cp /bin/true ./target/debug/gsd-meta-manager
exit=0  GSD_MM_ENVELOPE_ROOT=/tmp/fresh gh pr create --title x   <- INERT; the line landed under
                                                                    the root the GUARD was given
exit=0  printf '{"pr_cap_per_run":9999}' > ~/.config/gsd-meta-manager/config.json
exit=0  printf '[credential]\n\thelper = …\n' >> <ENV>/alpha/gitconfig
exit=0  printf x > <ENV>/alpha/gh/hosts.yml    exit=0  printf x > <ENV>/alpha/settings.json
```

### `T-19-112` — the four-call cap reset, re-driven END TO END in ONE persistent root

| Step | Ledger before | Exit | Ledger after | Reason |
|---|---|---|---|---|
| call 1 `gh pr create --title x` | MISSING | **0** | 1 | (permit) |
| call 2 `gh pr create --title x` | 1 | **2** | 2 | `pr_cap_exceeded` |
| call 3 `gh pr create --title x` | 2 | **2** | 3 | `pr_cap_exceeded` |
| guard verdict on `rm -f <ledger>` | 3 | **0** | 3 | (permit) |
| deletion PERFORMED | 3 | — | MISSING | — |
| call 4 `gh pr create --title x` | MISSING | **0** | 1 | (permit) |

**The ledger appends BEFORE the verdict** (D-20, `ledger.rs:33-36`), which is why
a refused attempt still adds a line. **Call 4 is permitted again with a fresh
one-line ledger.**

**The `: >` spelling, same drive, same root discipline** — direction (i), the
spelling that SURVIVES `19-27`: call 1 exit 0 (1 line), call 2 exit **2**
`pr_cap_exceeded` (2 lines), the guard's verdict on `: > <ledger>` **exit 0**,
truncation performed (0 lines), call 3 **exit 0** again.

### `T-19-111` and `T-19-113` — the bare-remote fixture REBUILT, with a control beside every leg

Local bare upstream, `pre-push`/`pre-commit` delivered exactly as the envelope
delivers them (`core.hooksPath` through the `GIT_CONFIG_COUNT`/`KEY_n`/`VALUE_n`
triplet `cred::hooks_path_env` emits), offline. Shell-driven at `406c48e` and
re-driven inside the suite:

| Leg | Command | Completed? | remote `main` before → after |
|---|---|---|---|
| **A — CONTROL** | plain `git push --force origin HEAD:refs/heads/main` | **NO**, hook refused | `4cc4cdc` → `4cc4cdc` **UNMOVED** |
| **B — CARRIER** | `printf`-written `.git/config` alias `fp = -c include.path=<evil> push --force …`, then `git fp` | **YES** | `4cc4cdc` → **`ed58fb9`** **MOVED** |
| **C — CONTROL** | the SAME alias, written the SAME way, carrying **NO carrier**, then `git np` | **NO**, hook refused | `ed58fb9` → `ed58fb9` **UNMOVED** |
| **B — repeat** | `git fp` again on a new commit | **YES** | `ed58fb9` → **`8ef079d`** **MOVED** |
| **D — `T-19-113`** | `cp /bin/true <hooks>/pre-push`, then plain force push, `GIT_CONFIG_COUNT` UNTOUCHED | **YES** | `8ef079d` → **`f2d230f`** **MOVED** |
| **D — CONTROL** | stub RESTORED, plain force push | **NO**, hook refused | `f2d230f` → `f2d230f` **UNMOVED** |

**Leg C is the sharpest control in the fixture**: it isolates the CARRIER from
both the ALIAS and the FILE WRITE. Without it the harm would read as "aliases
evade hooks" or "writing `.git/config` evades hooks" rather than "a non-`!` alias
body carries a config indirection", **and the whole class would be
mis-attributed.** **Leg D's control on both sides is what makes it a fact about
the STUB rather than about the fixture drifting.** Every leg reproduced; **none
failed to reproduce.**

### `T-19-110` — carried forward at `medium`, with audit 9's re-rating recorded as a MEASUREMENT

Audit 9's reproducers confirmed and four more found; **every reachable write loses
the precedence contest at repo-local and at global, and the force push after it is
still refused.** It clears nothing and it is not this round's scope. Untouched.

### The byte floors and the anti-vacuity stripper's three protections, RE-MEASURED

Measured at `406c48e` with each file's own definition of "production half", and
**MY numbers are recorded rather than either cited number repeated**:

| Measurement | Method | This plan's number |
|---|---|---|
| `policy.rs` production half | everything above the first `#[cfg(test)]` (`policy.rs:7147-7151`'s own stripper) | **262,229 bytes**, 5,162 lines |
| `hooks.rs` production half | the same | **74,490 bytes**, 1,572 lines |
| `policy.rs` stripped | `tests/envelope_wrapper_class.rs:763`'s `production_code` (comments AND test module removed) | **72,840 bytes** of a 443,076-byte raw file |
| `hooks.rs` stripped | the same | **33,460 bytes** of a 99,909-byte raw file |

Audit 9 recorded 261,387 / 74,358 at `cc65220` and the round-10 mandate cited
261,386. **Neither is repeated; the numbers above are this plan's own measurement
at its own base commit.**

**The three protections, all confirmed present and at their present strictness:**

1. **The proportional floor** — `production.len() >= 180_000` at `policy.rs:7192`.
   **PASSES** at 262,229.
2. **The deep anchor** — `fn forbidden_repo_path` at `policy.rs:5141`, asserted at
   `policy.rs:7202`. Present.
3. **The sentinel count** — exactly ONE `#[cfg(test)]` line per file by the
   stripper's trimmed-equality predicate: `policy.rs:5163`, `hooks.rs:1573`.
   Confirmed, one each.

`POLICY_MIN_PRODUCTION_BYTES = 40_000` and `HOOKS_MIN_PRODUCTION_BYTES = 20_000`
are unchanged and both pass with wide margin.

**THE STALE PROPORTIONAL-FLOOR COMMENT, RECORDED AND DELIBERATELY NOT FIXED
HERE.** `policy.rs:7172`, `:7185` and `:7194` all cite *228,785 bytes, 78.7%*
against a production half now measured at **262,229** — the real ratio is
**68.6%**, not 78.7%. **This plan may not edit `policy.rs` at all**; `19-27`
carries `policy.rs` hunks anyway, so the correction is scheduled there and named
here. **The FLOOR ITSELF is correct and must not move**; it is the arithmetic in
the comment that has drifted.

### The `glab --host` forge cell — carried forward UNCHANGED

`glab` is confirmed **NOT installed** (`command -v glab` finds nothing), so the
callee's grammar is unconfirmed and the under-count is **NOT claimed as a live
bypass**; audits 7, 8 and 9 all declined to upgrade it and this plan declines too.
**`--hostname` STAYS in `FORGE_VALUE_OPTS`**, with audit 9's re-measurement
recorded: `glab --hostname gitlab.com mr create --title x` leaves ONE ledger line
and `glab --host …` leaves ZERO, so **removal is a REGRESSION in the
under-counting direction** (`T-19-35`). Audit 9 overturned audit 8's own
suggestion on measurement, and that overturning stands.

### `T-19-17r` — the bookkeeping gap, OUTSTANDING and deliberately NOT resolved

`19-17-SUMMARY.md` calls it "accepted". Audits 5, 6, 7, 8 and 9 all confirmed the
measurement and both pins and **all five explicitly declined to make the
acceptance**; plans `19-24` and `19-25` both correctly declined too. The Accepted
Risks Log runs `AR-19-01` … `AR-19-12`; `grep -cE '^\| AR-19-13 \|'` over this
file is **0**, verified after this subsection was written.

**Six agents have now deliberately left that acceptance unmade because it is a
human decision, and this plan is the seventh.** The next round either adds the log
row or drops the word. **This plan does neither, adds no `AR-19-13`, and does not
apply the word "accepted" to `T-19-17r` anywhere.**

### What this plan is

A corpus, observed RED, and a record. **It closes nothing.** `T-19-111`,
`T-19-112`, `T-19-113` and `T-19-114` all stay open at its end, and re-measuring
and re-classifying these rows is `/gsd-secure-phase 19`'s job rather than a
plan's. **Every commit shows ZERO `src/` hunks**, which is the evidence this split
exists to produce: the corpus was capable of failing before any production line
moved.

---

## Plan 19-27 execution record — the carrier-operand rule and the corrections

**A record made by plan 19-27, not an audit finding.** It follows the plan-19-26
record and edits nothing that precedes it — no audit table, no Security Audit
Trail row, no Accepted Risks Log row, no Sign-Off, and no earlier appended
subsection.

### STATED FIRST — `/gsd-secure-phase 19` is NOT cleared by this plan

- **`T-19-86`** — OPEN at `high` by explicit user scoping decision. Unchanged,
  unremediated, its four registered rows and its persisted-alias arm still at exit
  0 and its pins green and UNMODIFIED. **`T-19-111` is moved OUT of it, not into
  it.**
- **`T-19-91`** — OPEN at `high`, arms unweakened. No decision-operand rule for
  `reflog`, `symbolic-ref` or `push`, no denylist extension.
- **`T-19-111`** — OPEN at `high`. **This plan writes NO rule for it.** It gets the
  attribution correction and nothing else, and **no acceptance is made** — that is
  a human decision.
- **`T-19-112`** — **NARROWED, explicitly NOT closed, and narrowed in ONE ROUTE OF
  SEVERAL.** Rule (a) refuses an absolute literal operand under the envelope
  directory. It does not reach `C-15` (a number raised in a file outside the
  envelope root, no ledger write, no envelope path named), it does not reach the
  four fail-open directions, and the deferred option (b) leaves the guard as the
  cap's only observation point (`T-19-35`, `AR-19-05`).
- **`T-19-113`** — **NARROWED, NOT closed.** Same clause, plus D-09's narrative
  corrected — which was required whether or not any control landed.
- **`T-19-114`** — closed for the CONTROL-CARRIER axis by `19-26`'s fifth named
  axis, whose fail-closed property this plan turns green and whose
  verdict-preserving alphabets it leaves PERMITTED.
- **`T-19-96`**, **`T-19-110`** — open at `medium`, not fixed. **`T-19-74`** — core
  rows frozen. **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85`** — open and
  unaccepted by explicit user decision.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** No unqualified
"T-19-60 is closed" appears anywhere in this plan's output.

### The four commits, in order

| # | SHA | What |
|---|---|---|
| 1 | `6944b52` | `feat(19-27)`: the carrier-operand predicate, `T-19-111`'s attribution, the floor comment |
| 2 | `7bc86f2` | `feat(19-27)`: the ONE call site, raised before the resolution match |
| 3 | `ad4211b` | `docs(19-27)`: the honesty repairs — the cap claim, D-09's ceiling, `cred.rs`'s reach |
| 4 | this one | `docs(19-27)`: the record |

### `19-26`'s RED rows — confirmed still RED first, then GREEN

**Confirmed against the unmodified tree at `22bb5f7`, BEFORE any production line
moved.** Verbatim:

```text
tests/envelope_control_carrier.rs
  test result: FAILED. 24 passed; 9 failed; 0 ignored; 0 measured; 0 filtered out
failures:
    after_19_27_a_command_naming_the_whole_alias_directory_is_refused
    after_19_27_a_command_whose_operand_is_a_hook_stub_is_refused
    after_19_27_a_command_whose_operand_is_the_askpass_responder_is_refused
    after_19_27_a_command_whose_operand_is_the_pr_cap_ledger_is_refused
    after_19_27_reads_under_the_envelope_root_are_refused_too_and_that_is_a_decision
    after_19_27_the_carrier_rule_reads_a_path_and_not_a_program_name
    direction_ii_an_expansion_borne_operand_stays_permitted_and_its_literal_twin_is_refused
    direction_iii_a_symlink_stays_permitted_and_its_measured_partial_mitigation_is_refused
    direction_iv_a_relative_operand_stays_permitted_and_its_measured_partial_mitigation_is_refused

tests/envelope_wrapper_class.rs
  test result: FAILED. 49 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out
failures:
    a_command_whose_operand_resolves_under_the_envelope_root_is_refused_after_19_27
```

**All ten are the exact set `19-26-SUMMARY.md` records. Not one was already green**
— which would have been a finding to report rather than a task to complete.

After commit 2: `envelope_control_carrier` **33 passed; 0 failed**;
`envelope_wrapper_class` **50 passed; 0 failed**. Every one of the ten green, at
the reason identifier the corpus derived — `envelope_assertion_failed`, the general
unresolvable one, and never `hook_bypass_blocked`.

### And the CONVERSE check, new this round — `19-26`'s PERMITTED rows are still PERMITTED

**A rule that turned any of them red would be a rule that quietly widened past the
boundary it declares, and that would have been a finding about the RULE rather
than an assertion to edit.** Every one re-driven against the built binary after
the fix and still at exit 0:

| Row | Direction / role | Before | After |
|---|---|---|---|
| `: > <ENV>/alpha/pr-ledger.ndjson` | (i) redirection target | 0 | **0** |
| `printf 'exit 0' > <ENV>/alpha/hooks/pre-push` | (i) | 0 | **0** |
| `echo evil > <ENV>/alpha/askpass` | (i) | 0 | **0** |
| `cat /tmp/evil >> <ENV>/alpha/gitconfig` | (i) | 0 | **0** |
| `D=$(git config --get core.hooksPath); rm -f $D/../pr-ledger.ndjson` | (ii) expansion-borne | 0 | **0** |
| `H=$(…); cp /bin/true $H/pre-push` | (ii) | 0 | **0** |
| `rm -f /tmp/l`, `: > /tmp/l` | (iii) symlink | 0 | **0** |
| `rm -f pr-ledger.ndjson`, `cp /bin/true pre-push` | (iv) relative | 0 | **0** |
| `rm -f /tmp/pr-ledger.ndjson` | NEAR-MISS, this round's `--signed no` | 0 | **0** |
| `cat <ENV-one-char-changed>/alpha/pr-ledger.ndjson` | NEAR-MISS | 0 | **0** |
| the whole ordinary-operand half, and `git config --get core.hooksPath` | control | 0 | **0** |

**Not one config-resolution verdict moved**: `envelope_config_resolution` 30/0,
`envelope_reparsed_value` 34/0, `envelope_callee_grammar` 19/0 — all unchanged.

### The design answer — option (d), a SPLIT chosen by measurement rather than by symmetry

**Option (c) — move a carrier out of reach — is DEAD for every envelope carrier,
and it is stated ONCE.** Each must be readable, writable or executable by the same
uid the driven process runs as: git must exec the hooks, the agent CLI must read
the settings, and the guard must append the ledger. A different uid, a container
or a remote-side control are the only real (c)s; the first two are outside this
phase and the third is what `SECTION_ENVELOPE`'s own conclusion already says.

| Carrier group | Control taken | Direction it FAILS |
|---|---|---|
| the nine under `<envelope>/<alias>/` (`C-01` … `C-09`) | **(a)**, written here — one lexical prefix test, no I/O, no program names, one rule for nine carriers | **OPEN in four directions**, below |
| the ledger, additionally | **(b)** tamper-evidence — **COSTED AND DEFERRED**, with **NO schedule, NO pin and NO witness attached** | n/a — deferred |
| the hook stubs | **(a) only**; **(b) is structurally UNAVAILABLE** | (a)'s four, plus D-09's narrative corrected either way |
| `.git/config`, `.claude/settings.json`, `.git/info/exclude`, the run journal, **and `C-15`** | **(e)** — NO rule. Attribution correction, disclosure, registered, **unaccepted** | **OPEN entirely, stated and handed to nothing** |
| `SECTION_ENVELOPE`'s `Guaranteed` cap clause | **repaired** — an honesty change, not a control | n/a |

**Option (b) for the ledger, costed from both sides.** The ledger is append-only
by design and its entries carry `run_id` and `at`, so a deletion or truncation IS
detectable at the next append — the material exists. What it costs is a SECOND
CARRIER for SAFE-06, which `hooks.rs:1421-1428`, `T-19-35` and `AR-19-05` all say
in terms SAFE-06 does not have (*"One control genuinely has no git-hook
counterpart … the pull-request cap"*), and it breaks the stated latency rule *"the
ledger read is a single pass over one append-only file"*, because the cross-check
would read the run journal too. **That is a round of its own. Recording it as
costed is different from scheduling it, and nothing here schedules it — no pin, no
revisit condition, no witness.** (a)'s residue stays uncovered.

**Option (b) for the stub is STRUCTURALLY UNAVAILABLE, and the asymmetry is the
round's sharpest design fact**: the thing that would detect a replaced `pre-push`
is the binary the replacement removed from the path. `assert_provenance_in`
certifies a RELOCATED COPY — `T-19-01`'s mitigation, **which is correct and is not
weakened here** — but a replaced stub never re-enters the binary that would assert
it. Prevention is the only control that can fire there.

### The rule, exactly, and where it is raised

**A segment is refused when any of its words is ABSOLUTE, LITERAL, and — after
LEXICAL normalisation — equal to or under `envelope_dir_in(root, alias)`, compared
COMPONENT-WISE.** Each condition is a boundary rather than a convenience: ABSOLUTE
because the guard has no cwd; LITERAL because a word the shell may rewrite is a
word the guard cannot resolve, and refusing every non-literal operand of an
ungoverned command would deny `rm $TMPDIR/x`; LEXICALLY NORMALISED with `..`
collapsed textually and **no link followed**, because `canonicalize`/`read_link`
is I/O and TOCTOU on the guard's critical path; COMPONENT-WISE because a raw
`starts_with` over `<root>/alpha` also refuses a sibling `alpha2`.

**The boundary is the DIRECTORY and not a list of filenames**, because
`rm -rf <root>/<alias>` takes nine carriers in ONE call and a filename list would
need a tenth entry the day a tenth carrier lands.

**Raised ONCE**, at the top of `classify_segments`' existing per-segment loop, on
the segment the loop already holds, **BEFORE `resolve_program_with_head`**. One
hunk: `@@ -966,0 +967,64 @@ fn classify_segments(`, 64 insertions, **zero
deletions**. No new `ParkReason`, no second pass, no second scan, no third reading
site, and `resolve_program`, `resolve_program_with_head`,
`first_unreadable_decision_word`, `config_key_operand_index`,
`subcommand_word_indices`, `scan_gh_api` and `scan_leading` are untouched.

**The placement has three measured consequences, and all three are pinned:**

| Consequence | Row | Measured |
|---|---|---|
| runs BEFORE the resolution match, so a GOVERNED carrier is reached | `git config --file <ENV>/alpha/gitconfig --get user.name` | **exit 2** `envelope_assertion_failed`; control outside the envelope **exit 0** |
| the same, through a leading git option | `git --git-dir <ENV>/alpha/gh status` | **exit 2**; control **exit 0** |
| runs BEFORE the ledger write | no refused command consumes PR-cap budget | walk EMPTY on every refused row |
| re-raises at DEPTH through `NestedPayload` re-entry | `bash -lc "rm -f <ENV>/alpha/pr-ledger.ndjson"` | **exit 2**; control **exit 0** |

**The two ORDERING pins, at deliberately DIFFERENT identifiers** — the mechanical
proof the clause is raised in the one per-segment walk rather than in a second
pass, which would answer the same identifier for both:

| Command | Identifier |
|---|---|
| `rm -f <ENV>/alpha/pr-ledger.ndjson && git push --force origin main` | `envelope_assertion_failed` |
| `git push --force origin main && rm -f <ENV>/alpha/pr-ledger.ndjson` | **`force_push_blocked`** |
| CONTROL: `rm -f /tmp/x && git push --force origin main` | `force_push_blocked` |
| `git -c core.hooksPath=/dev/null config --file <ENV>/alpha/gitconfig --get x` | `envelope_assertion_failed` |
| CONTROL: `git -c core.hooksPath=/dev/null status` | **`hook_bypass_blocked`**, unperturbed |

### The FOUR fail-open directions — stated in four places and handed to NO control

They appear in the predicate's doc, the refusal's doc, `cred.rs`'s WHAT IS NOT
COVERED list, and this record, in these words:

1. **A REDIRECTION TARGET is not an operand.** `: > <ledger>`,
   `printf 'exit 0' > <hooks>/pre-push`. **Real by MECHANISM rather than by
   choice**: `tokenize` consumes bash's redirection production and emits no token
   for the operator OR ITS TARGET, because bash deletes both before `execve`.
   Reading `>` would re-open a model five rounds have pinned shut — `SEPARATORS`
   has one commit in the whole phase (`84a9b05`), `is_separator(">")` is `false`
   by construction, and round 6's over-deletion control pins
   `git x2>/tmp/o push --force origin main` PERMITTED.
2. **An EXPANSION-BORNE operand cannot be resolved.**
   `D=$(git config --get core.hooksPath); rm -f $D/../pr-ledger.ndjson`. Note what
   this composition IS: the carrier's location is fetched by a **PERMITTED
   GOVERNED READ** — that first segment resolves `Governed { index: 0 }` — and
   then acted on by an ungoverned command.
3. **A SYMLINK is not followed.** `..` is collapsed lexically; a link is not.
   **NARROWED, and the word is NARROWED**: `ln -s <env>/… /tmp/l` names an
   envelope path as its own operand and is refused, so a link must PREDATE the run
   or be made by a means naming no envelope path.
4. **A RELATIVE path is not resolved.** **NARROWED the same way and no further**:
   `cd <env>/<alias>` is refused, and the composite
   `cd <env>/<alias> && rm -f pr-ledger.ndjson` is refused **BY SEGMENT ONE** —
   segment two's relative operand stays unresolvable.

**NO revisit condition and NO version witness was created for any of them, and
that is a decision with a reason.** `19-25`'s witness schedules a human re-audit
against a FUTURE GIT, which is the right shape for a residue that arrives when
somebody else ships something. **These four are reachable by the driven agent
TODAY.** A schedule over them would observe the wrong thing entirely — a version
string that never moves while the residue is exercised — and attaching one would
be `T-19-107`'s shape arriving in the round that inherited the lesson.

### `C-15` — carried forward at its measured severity, control (e), NO rule

**A DISTINCT ROUTE to SAFE-06, not a spelling of `T-19-112`.**
`~/.config/gsd-meta-manager/config.json` is re-read by `resolve_policy` on every
Bash tool call, and `EnvelopePolicy::resolve` passes `pr_cap_per_24h` and
`pr_cap_per_run` through `unwrap_or(DEFAULT_…)` with **no clamp**. `19-26`
measured four permits, four ledger lines, **no deletion and no envelope path
named**. Rule (a) resolves against the envelope directory and cannot reach a file
outside it.

**`pr_cap_*` is deliberately NOT clamped.** Clamping a configured cap is a PRODUCT
decision about what a user may configure, not a guard rule, and making it inside a
security round would decide it silently. **No acceptance is made for `C-15`.**

### The `T-19-111` ATTRIBUTION CORRECTION — required regardless of any rule, performed at all five sites

`T-19-86`'s four registered rows all require a GOVERNED program to be handed a
GOVERNED COMMAND AS DATA on the line the guard reads. **A non-`!` alias body
sitting in a config VALUE is not that**: it is a plain git command line in a file
— `REPARSED_COMMAND_SECTIONS`' own K1 class, reached through a carrier outside
argv. **Crediting a live, non-shell, measured destructive bypass to a threat the
user has explicitly scoped OUT is how it stops being counted.**

| # | Site | Before | After |
|---|---|---|---|
| 1 | `src/envelope/policy.rs:1177-1183` | *"That is `T-19-86`'s shape and it stays OPEN."* | *"That is `T-19-111`'s shape and it stays OPEN"*, with the one-sentence reason, the *"or one written by a means that is not `git config`"* half KEPT, the measurement recorded, and the statement that no rule and no acceptance is made. **EDITED, commit `6944b52`.** |
| 2 | `src/envelope/cred.rs:287-289` | *"a repo-local `.git/config` alias predating the run is live"* | the *"or one written by a means that is not `git config`"* half **RESTORED**, re-attributed to `T-19-111`, with the reason and the measurement. **EDITED, commit `ad4211b`.** |
| 3 | `19-SECURITY.md:7430` (inside the plan-19-25 record) | *"`T-19-86`'s shape; stays open."* | **CORRECTED BESIDE, HERE, never by editing it.** That line's residue direction (i) is `T-19-111`, not `T-19-86`. Direction (i) as written there — *"a repo-local `.git/config` alias predating the run is LIVE"* — is also narrower than the truth: an alias written DURING the run by a means that is not `git config` is equally live, and that is the half `policy.rs:1182` already carried. Item 2 of that same list (*"A `!`-bodied body carrying its own carrier"*) IS `T-19-86` and is correct as written. |
| 4 | `19-25-SUMMARY.md:265` | *"Stateless, argv-only; a repo-local `.git/config` alias predating the run is LIVE. `T-19-86`'s shape."* | **CORRECTED BESIDE, HERE, never by editing it.** Same correction, same two halves: the attribution is `T-19-111`, and *"predating the run"* is narrower than the measured reach. Its item 2 is correctly `T-19-86`. |
| 5 | `deferred-items.md:1619` | `T-19-108` *"CLOSED AS SCOPED"* naming the excluded BODY (`!`) | **corrected in this plan's `deferred-items.md` section**, to name the excluded CARRIER (a file) as well as the excluded BODY. |

**`T-19-111` is moved OUT of `T-19-86` and never into it. No rule is written for
it and no acceptance is made for either.**

### D-09's corrected ceiling narrative

`cp /bin/true <hooks>/pre-push` removed layer 3 — SAFE-05's only carrier and
`AR-19-03`'s backstop — **at exit 0 and without touching `GIT_CONFIG_COUNT` at
all**, and a force push then MOVED a bare remote's `main` `8ef079d` → `f2d230f`;
restoring the stub restored the refusal. Meanwhile the route D-09's ceiling
paragraph names is the one the guard REFUSES: the `unset GIT_CONFIG_COUNT` route
is re-measured green at `hook_bypass_blocked`. **A reader of D-09 concluded the
guard stands underneath layer 3, and it did not.**

| Site | Before | After |
|---|---|---|
| `advisory.rs:254` | `An agent that unsets GIT_CONFIG_COUNT in a subshell is past the last layer.` | `An agent that unsets GIT_CONFIG_COUNT in a subshell, or rewrites the` / `hook stubs and ledger the envelope installed, is past the last layer.` |
| `cred.rs:241-244` | *"An agent that unsets `GIT_CONFIG_COUNT` in a subshell escapes this layer. That is not a hole this project can close client-side."* | the same, **plus** the measured file route, the note that point 3's *"cannot uninstall it by editing a file in the repository"* is true and is not the same claim, the statement that plan 19-27 NARROWS and does not close it, the four open directions, and `T-19-01`'s mitigation recorded as unweakened |
| `mod.rs:165-168` | *"That party can equally unset `GIT_CONFIG_COUNT`, which is D-09's stated ceiling"* | **READ, and NOT opened. Recorded as a documentation gap for a later round.** `src/envelope/mod.rs` is not in this plan's `files_modified`, and the plan's own verification requires that `git diff --stat` touch no file outside it. The plan offered exactly this fallback for a paragraph that needs company; the paragraph needs the same correction and a later round should make it. |

### The `SECTION_ENVELOPE` repair — the pin hit set enumerated BEFORE the text moved

`grep -rn 'SECTION_ENVELOPE\|append-only ledger\|Pull-request cap\|envelope_notice'
src/ tests/` returned **40 hits**, complete set, recorded before any character
moved:

```text
src/driver/run.rs                    :2639, :2652        (envelope_notice consumer)
src/driver/dry_run.rs                :38, :74, :499, :502, :549, :735, :796
src/envelope/advisory.rs             :169, :239, :250, :272, :273
src/envelope/hooks.rs                :974                 (this plan's own comment)
src/envelope/policy.rs               :5337, :5378         (this plan's own docs)
tests/envelope_wrapper_class.rs      :8811
tests/envelope_advisory.rs           :184, :198, :203, :209, :212, :215, :259, :266, :276, :302
tests/envelope_control_carrier.rs    :503, :504, :594, :1146, :1172, :1518, :1953, :1968, :1971, :1992
```

| Before | After |
|---|---|
| `Pull-request cap: an append-only ledger this repository does not contain.` | `The pull-request count lives in` / `an append-only ledger this repository does not contain.` |

**What changed is the KIND of statement, not the wording.** The old clause named a
CONTROL and then described the ledger, so a reader took the cap itself as
guaranteed — and audit 9 falsified that by driving the reset end to end over four
calls in one persistent root. The new clause states a LOCATION FACT and stops: the
count lives in a ledger this repository does not contain, so no checkout, reset or
clean reaches it. **It claims nothing about whether the cap can be reset**, and
the `Not guaranteed` half now says rewriting the ledger is past the last layer.

**It makes NO completeness claim and enumerates NOTHING**, deliberately. Rule (a)'s
four fail-open directions are not SAFE-06's whole residue — `C-15` is a fifth route
and the deferred option (b) a sixth — and a clause listing four directions and
stopping would imply a completeness the measurement denies, which is the
`T-19-107` shape in a shorter sentence.

**The mechanical constraints, all six, verified against the RENDERED constant
rather than the source:**

- **213 whitespace tokens against the 215 cap. The cap was NOT raised.** Nothing
  was shortened; two clauses were ADDED, and both are LIMITATIONS.
- widest line **74** of the 80 cap; **no pinned phrase wrapped through** (the
  longest is 53 characters).
- the three pinned phrases in their pinned ORDER: `cannot reach your ambient git
  credentials` < `defeatable by an agent that can spawn an unsupervised` < `enable
  server-side branch protection`.
- indentation is `\x20` escapes, never literal spaces — confirmed by rendering the
  constant and measuring each line.
- **no residual disclosure was deleted.**

**Assertions updated: exactly ONE**, and it is the one `19-26` named in advance —
`the_guaranteed_cap_clause_is_pinned_at_its_current_text_and_19_27_must_change_it`
in `tests/envelope_control_carrier.rs`. **No `tests/envelope_advisory.rs`
assertion needed updating**, because the repair kept the phrase `append-only
ledger this repository does not contain` VERBATIM; that file shows a **ZERO diff**
and all ten of its tests pass UNMODIFIED, including
`the_honesty_statement_carries_each_of_its_three_required_parts`, its own
phrase-ORDER assertion and `the_honesty_statement_stays_short_enough_that_a_reader_finishes_it`.
`driver/dry_run.rs`'s three consumers (`:549`, `:735`, `:796`) and `envelope_notice`'s
pin all pass UNMODIFIED — `driver_dry_run` 15/0, `envelope_advisory` 10/0.

### The NAMED FILE EXCEPTIONS, stated AS exceptions

**Three were taken, and a possible fourth was declined.**

| File | Bound | Held? |
|---|---|---|
| `src/envelope/hooks.rs` | ONLY the call site inside `classify_segments`' per-segment loop | **YES** — one hunk `@@ -966,0 +967,64 @@`, 64 insertions and **zero deletions**, so `pre_push`, `pre_commit`, `guard`, `guard_in`, `deny`, `read_guard_request`, `park_refusal`, `write_stub`, `stub_body`, `install_in`, `hooks_dir_in`, `assert_provenance_in`, `settings_value`, `settings_json`, `guard_command`, `write_settings_in`, the D-07 second-carrier table and `mod tests` all show ZERO diff lines. **The `settings_json` second-carrier row was NOT repaired.** |
| `src/envelope/cred.rs` | ONLY the `hooks_path_env` limit paragraph, **zero non-doc lines** | **YES** — `git diff --unified=0` filtered for lines that are not `///` and not blank returns **NOTHING**. `config_env`, `hooks_path_env`'s body, `write_gitconfig`, `write_askpass_stub_in`, `build_env_in` and `EnvelopeEnv` untouched. |
| `src/envelope/advisory.rs` | ONLY `SECTION_ENVELOPE`'s literal | **YES** — 4 insertions, 2 deletions, all inside the constant. `envelope_notice`, `protection_line`, `PROTECTION_WARNING`, `PROTECTED_CLAIM` and `ProtectionState` untouched. |
| `src/envelope/mod.rs` | the possible FOURTH | **DECLINED, and the decision is recorded** — not in `files_modified`; the gap is carried forward for a later round. |

`T-19-61` … `T-19-73`, `T-19-84` and `T-19-85` were **not taken on**. `scan.rs`,
`config.rs`, `mod.rs`, `ledger.rs` and the second-carrier table were not opened.

### The over-refusal cost, measured from BOTH sides

**Reads are refused as well as writes, and that is a DECISION with a stated
reason.** The guard cannot tell a read from a write without knowing every
program's grammar — is `dd if=X of=Y` a read of `X` or a write of `Y`? is `tee F`
a read? — and an enumeration that tried would be a program-name list again, which
`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
mechanically forbids and D-08's *"never asks what the wrapper is CALLED"* rules
out. So `cat <ledger>`, `wc -l <ledger>` and `ls <env>/<alias>/hooks` are refused,
and **a run cannot inspect its own envelope directory.**

**The permitted twin is that the path can still be NAMED.**
`git config --get core.hooksPath` stays at **exit 0** and is pinned UNCHANGED, and
the refusal message names the directory, so a human debugging the run loses
nothing the refusal does not already tell them (AR-19-11: a refusal a user cannot
act on is a control that gets switched off). **Every ordinary-operand row stays at
exit 0**, including `rm -f /tmp/x`, `rm -rf /tmp/scratch`, `cp /bin/true /tmp/t`,
`truncate -s 0 /tmp/f`, `cat /tmp/x`, `ls`, `cargo test` and
`rg 'pr-ledger.ndjson' src/`. **Not one config-resolution verdict moved.**

The refusal never quotes the command back (SAFE-04); it names the envelope
DIRECTORY, a path this binary generated for this run that carries no secret, on the
same footing as `scan_leading`'s refusals naming a key and a section. It carries
`ParkReason::EnvelopeAssertionFailed` and **not** `HookBypassBlocked`, which names
a config-KEY mechanism this refusal does not use (D-24).

### The corrected proportional-floor comment — with BOTH measured numbers, and why the recorded ones disagreed

`policy.rs:7172`, `:7185` and `:7194` cited *228,785 bytes, 78.7%* — a measurement
several rounds stale, which `19-24` and `19-26` both recorded as drift they were
forbidden to touch.

| Measurement | Number |
|---|---|
| production half at this plan's BASE (`22bb5f7`) | **262,229 bytes**, 5,162 lines |
| production half AFTER this plan's own additions | **277,570 bytes**, 5,414 lines |
| the ratio written into the comment | 180,000 / 277,570 = **64.8%** |
| audit 9's recorded number (`cc65220`) | 261,387 |
| the round-10 mandate's number | 261,386 |

**The discrepancy is NOT "one byte of newline convention", and the real cause is
worth recording.** `production.len()` is `String::len()`, which counts **BYTES**.
This file's prose carries **842 multi-byte UTF-8 characters** — em dashes, arrows
and curly quotes. Counted as CHARACTERS the production half at this base is
**261,387**, which is audit 9's number exactly; counted as BYTES, which is what the
assertion actually compares, it is **262,229**. Both recorded numbers were
character counts of the same file. `19-26`'s 262,229 is the byte count and is
correct.

**The FLOOR stays at `180_000` and its strictness is unchanged.** Only the
comment's arithmetic moved, along with the stale line references (`fn scan_leading`
416 not 374; 40,000 bytes at line ~752 not ~778; 180,000 at ~3,613 not ~3,622;
`fn forbidden_repo_path` at ~5,157 of 5,414 not ~4,592 of 4,613).
`POLICY_MIN_PRODUCTION_BYTES = 40_000` and `HOOKS_MIN_PRODUCTION_BYTES = 20_000`
are unchanged, both passing with wide margin. All three stripper protections keep
their present strictness: the proportional floor, the deep anchor
`fn forbidden_repo_path`, and the exactly-ONE `#[cfg(test)]` sentinel count in each
file (`policy.rs` 1, `hooks.rs` 1).

### `AR-19-04` and `AR-19-05` — carried forward, RECORDED, and NOT un-accepted

`AR-19-04`'s mitigation reads *"the envelope regenerates it at each run start"* —
**which does not cover a write DURING the run**, and `19-26`'s `C-05` measurement
is exactly such a write. **The same shape a fourth time now**: `cred.rs:287-289`'s
*"predating the run"* (repaired in this plan), `resolve_policy`'s *"a guard that
cannot read configuration confines the run more, never less"* (true for an ABSENT
or UNREADABLE config, silent about a PRESENT and WRITABLE one), and this. **A
reassurance reasoning about the case that is not the threat.**

**No `AR-` row was added, edited, renumbered, accepted or un-accepted.**
`AR-19-04`'s reasoning gap is recorded beside it and nothing more; `AR-19-05`
likewise. That is a human decision.

### The planner-derived carriers — carried forward UNFIXED at their measured severity

`C-05` (the generated `gitconfig`), `C-07` (the `gh` directory), `C-12`
(`.claude/settings.json`) and **`C-08` — `settings.json`'s claimed-but-unwired
second carrier — are carried forward and NOT repaired.** `19-26` measured and
registered `C-08`; **repairing a delivery is a change to the SPAWN SEAM and is out
of this round's scope**, and this record says so rather than leaving the omission
to be read as an oversight.

**`C-08`'s behavioural half remains UNMEASURED.** The mechanical half is settled —
`grep -rn 'settings_json' src/` returns two hits, the definition and the doc row
claiming otherwise, while production pushes `--settings <path>` — and the
behavioural half needs a live agent run. **It is not claimed either way here.**

### `T-19-17r` — OUTSTANDING for the ninth time, and this plan did NOT accept it

`grep -cE '^\| AR-19-13 \|'` over this file is **0**, verified after writing.
**This plan adds no Accepted-Risks-Log row, creates no `AR-19-13`, and does not
apply the word "accepted" to `T-19-17r` anywhere.** Eight agents have deliberately
left the acceptance unmade because it is a human decision; **this plan is the
ninth.**

### `glab --host` and `T-19-110` — carried forward unchanged

**`--hostname` STAYS in `FORGE_VALUE_OPTS`.** Audit 9 re-measured and overturned
audit 8's own suggestion: `glab --hostname …` leaves ONE ledger line and
`glab --host …` leaves ZERO, so removal is a REGRESSION in the under-counting
direction (`T-19-35`). **`glab` is confirmed NOT INSTALLED**, so a pin over that
cell cannot run against its real callee and a pin that skips is a fail-open pin.
`T-19-110` is carried forward at `medium`, unfixed.

### The gate — the arithmetic STATED and CHECKED

Command: `rtk proxy cargo test --no-fail-fast`, counts read with `rtk proxy grep`
over a redirected log (D-34), never a plain `grep`.

| | `19-26`'s recorded post-state | this plan |
|---|---|---|
| result lines | 45 | **45** |
| passed | 1752 | **1771** |
| failed | **10** | **0** |
| ignored | 13 | 13 |
| **`passed + failed`** | **1762** | **1771** |
| `envelope_*` binaries | 16 | **16** |

**The identity, checked against `git show` rather than assumed.** A red test RAN,
so red→green leaves the total unchanged and every increase comes ONLY from new
`#[test]` fns:

```text
src/envelope/policy.rs (mod tests)   :  5 new #[test] fns   (commit 6944b52)
tests/envelope_control_carrier.rs    :  4 new #[test] fns   (commit 7bc86f2)
TOTAL                                :  9
1762 + 9 = 1771  ==  observed passed + failed
```

**The identity holds exactly. No disagreement to report.**

**Per-binary counts — all SIXTEEN `envelope_*` binaries RAN.** A run reporting
fifteen would be a run in which round 10's evidence file did not execute.

| Binary | passed | failed | | Binary | passed | failed |
|---|---|---|---|---|---|---|
| `envelope_advisory` | 10 | 0 | | `envelope_literal_decision` | 43 | 0 |
| `envelope_argv_deletion` | 20 | 0 | | `envelope_pr_cap` | 11 | 0 |
| `envelope_callee_grammar` | 19 | 0 | | `envelope_reparsed_value` | 34 | 0 |
| `envelope_command_position` | 18 | 0 | | `envelope_tracer` | 6 | 0 |
| `envelope_config_resolution` | 30 | 0 | | `envelope_wiring` | 14 | 0 |
| **`envelope_control_carrier`** | **37** | 0 | | `envelope_wrapper_bypass` | 13 | 0 |
| `envelope_credential` | 6 | 0 | | **`envelope_wrapper_class`** | **50** | 0 |
| `envelope_expansion_slots` | 32 | 0 | | `envelope_hook_refusals` | 7 | 0 |

**A DOCUMENTED FLAKE FIRED, and it is recorded rather than smoothed away.** The
verification run after commit 2 reported ONE failure —
`tests/driver_reattach.rs::a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`,
panicking at `driver_reattach.rs:542` with *"the run record is on disk: Os { code:
2, kind: NotFound }"*. That is one of the two documented `driver_reattach` flakes.
**It was NOT fixed, worked around or investigated — it is out of scope.** It did
not fire on the runs after commits 1 and 3. **Absence on those runs is not
evidence it is fixed, and its firing here is not a regression this plan caused:
`driver_reattach` exercises no envelope guard path.**

`cargo build` exits 0. `cargo clippy -- -D warnings` exits 0.
**`cargo clippy --tests` is NOT the gate** — it fails at base on four pre-existing
lints in `src/browser.rs` and `src/project_creator.rs`, untouched.

**Mechanism pins, all green and unmodified.** `SEPARATORS` is byte-identical
(`const SEPARATORS: &[&str] = &[";", "&&", "||", "|", "&", "\n", "(", ")", "{",
"}"];`, one commit ever, `84a9b05`); `is_separator(">")` is `false`; round 5's
literalness bit, round 6's deletion model, round 7's fail-closed callee grammar,
round 8's confinement clause and round 9's re-parse clause are all non-dead, with
`aliasx.`/`notalias.` and both `T-19-86` `!`-bodied rows at exit 0. `Token.literal`
was neither cleared nor repurposed — **this round READS it for the first time
outside a governed program's decision words.**

**No row anywhere under `tests/` was edited or deleted** except the one
`SECTION_ENVELOPE` assertion `19-26` named in advance.
`git diff --numstat HEAD~4..HEAD -- tests/` shows **zero deletions**. Neither
`Cargo.toml` nor `Cargo.lock` was touched (`T-19-SC`).

### What this plan is

A rule at one existing decision point, three narrow doc corrections in fenced
files, a repaired `Guaranteed` claim, a five-site attribution correction, and a
record. **It closes `T-19-112` and `T-19-113` not at all — it NARROWS them, and
`T-19-112` in one route of several. `T-19-86`, `T-19-91` and `T-19-111` remain
OPEN at `high`. `/gsd-secure-phase 19` is not cleared.**

## Threats found by audit 10 (2026-09-04, after plans 19-26 and 19-27)

**Provenance, stated first.** The two subsections above this one were written by
the EXECUTORS of plans 19-26 and 19-27; they are deliberately outside the audit
tables. Audit 10 left them, every earlier appended subsection and every earlier
audit's own tables **byte-identical** — the body below the frontmatter was
checksummed before writing (`sha256 8e09d81bd0b0fa25…` over the 685,855 bytes
below the frontmatter of the 686,776-byte file), and this audit's write changes
the frontmatter, adds one Security-Audit-Trail row, one method subsection and
everything from here to the end of the file, **deleting nothing**. Everything
below is **audit 10's own**, measured against the built binary at `0092009` with
a fresh `GSD_MM_ENVELOPE_ROOT` per row and the whole envelope root walked
afterwards, and with every claimed bypass confirmed against the **real `git`
binary** (`git version 2.43.0`) in a rebuilt bare-remote fixture with a control
beside every leg.

### The question this audit was set, answered plainly

**With argv fully modelled through nine rounds and the envelope's own file
carriers now partly defended, is there a further layer — or is the modelled
surface complete and the remaining risk exactly the disclosed residual set?**

**There is NO further layer. The modelled surface is complete, and audit 10 says
so plainly because that is what the measurement supports.** Round 10 found the
right plane and it is the last one: every control this envelope installs is
reached either by an argv (nine rounds, all closed for their measured rows) or by
a FILE (round 10). Audit 10 swept every cell the mandate named and found no
eleventh plane —

* **carriers outside the envelope root that rule (a) structurally cannot reach**
  — `C-15` is **not** the only one. `C-10` … `C-15` are all outside it. `C-11` …
  `C-15` are given control (e) explicitly. **`C-10` is given nothing at all** —
  see `T-19-116`;
* **the guard's own binary and the hook path that execs it** — that is `C-10`,
  and audit 10 OBSERVED it end to end;
* **anything that changes WHEN the guard runs rather than what it sees** — only
  two carriers can stop the guard running (`C-08`, the settings file; `C-10`, the
  binary) and one can make it miss its deadline (`C-01`, the ledger — `T-19-117`).
  All three are files, all three already on round 10's plane;
* **state the guard carries between calls** — the guard is a fresh process per
  tool call. `guard_in` reads exactly three things that outlive it: the ledger
  (`C-01`), the registry (`C-15`) and the run journal (`C-14`). All files, all
  enumerated by `19-26`;
* **whether a permitted read can locate a carrier for an ungoverned write** —
  that is fail-open direction (ii), reproduced below at exit 0.

**And the non-file, non-argv candidates were checked rather than assumed.** The
ENVIRONMENT is `E-01`, modelled by `ENVELOPE_ENV_KEYS` and `tampers_with_envelope_env`
and drift-pinned; `GSD_MM_ENVELOPE_ROOT` is measured INERT again. TIME's only
lever on the 24 h window is the ledger, and APPENDING to it counts toward the cap
(`T-19-36`), so that direction fails CLOSED. The NETWORK probe runs once at run
start. The guard's own stdio is `T-19-61` and is not agent-reachable. And
`gsd-meta-manager envelope --help` exposes `pre-push`, `pre-commit`, `askpass`,
`guard` and `scan` and **no `install`**, so the envelope cannot be regenerated
from inside the run through its own CLI.

**So the answer to the tenth question is: stop looking for an eleventh plane and
finish dispositioning the tenth.** `19-26` enumerated fifteen carriers. Nine got
rule (a). Five got control (e). **One got nothing, and it is the one that takes
two layers in a single call.**

### The new rows

| Threat ID | Category | Component | Severity | Disposition | Finding | Status |
|---|---|---|---|---|---|---|
| **T-19-116** | **Elevation of Privilege** | **`C-10` — the BINARY the hook stubs and `guard_command` exec (`hooks.rs:196-204`, `:1454-1460`). Enumerated by `19-26` and marked UNOBSERVED; it has NO threat id, NO deferred-items row and NO control letter in `19-27`'s disposition table** | **high** | **mitigate (unimplemented — no rule, no disposition)** | **One permitted command removes LAYER 2 AND LAYER 3 TOGETHER, and audit 10 drove it end to end against real git with a control on both sides.** `cp /bin/true <binary>` is **exit 0** through the guard, names no envelope path, and never touches `GIT_CONFIG_COUNT`. Bare-remote fixture, `pre-push`/`pre-commit` delivered exactly as the envelope delivers them (`GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=core.hooksPath GIT_CONFIG_VALUE_0=<hooks dir>`), stubs generated in `stub_body`'s exact three-line shape: **LEG A (control)** — plain `git push --force origin HEAD:refs/heads/main` with the real binary in place → **REFUSED by the hook**, remote `main` UNMOVED at `01f9664`. **LEG B** — `cp /bin/true <binary>`, then the same push → **COMPLETED**, remote `main` `01f9664` → **`2537e97` MOVED**. **LEG B-control** — binary RESTORED, same push → **REFUSED**, remote UNMOVED. **LAYER 2, measured separately**: the replaced binary run as `envelope guard alpha` on `git push --force origin main` answers **exit 0 (PERMIT)**; restored, it answers **exit 2**. Discovery is trivial and permitted — `command -v gsd-meta-manager` and `cat /proc/self/cmdline` are both exit 0. **`T-19-01`'s declared mitigation (`current_exe()` + `assert_provenance_in`) is present and correct for what it covers — a RELOCATED COPY of the stub — and covers nothing here**, because a replaced exec TARGET never re-enters the binary that would assert provenance; the corpus's own `record_only("C-10 the exec target")` comment says exactly this and was never driven. **Rule (a) cannot reach it**: the binary is not under the envelope root. **Layer 1 does not backstop it**: `--disallowedTools` carries git-verb prefixes and `cp /bin/true …` matches none. Credentials survive untouched. **This is strictly WIDER than `C-02`** (the stub, `T-19-113`, `medium`), which removes layer 3 only, and wider than `env -u GIT_CONFIG_COUNT`, which removes layer 3 only and is REFUSED. **The register cannot count what it has not registered**, and this is the shape `T-19-111`'s attribution correction exists to prevent, arrived at by omission rather than by mis-filing. | **OPEN — BLOCKING** |
| **T-19-118** | **Information Disclosure / Elevation of Privilege** | **`C-05` — the generated `<env>/<alias>/gitconfig`, which `cred.rs:520-525` puts on BOTH `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM`. Reached by fail-open direction (i), which rule (a) does not touch** | **high** | **mitigate (unimplemented)** | **The run reaches the user's AMBIENT git credentials — the one thing `SECTION_ENVELOPE`'s FIRST `Guaranteed` clause says outright it cannot — and audit 10 measured it against real git with a control beside it.** `19-26` measured that `[credential] helper = /tmp/evil-helper` written into that file READS BACK, and recorded it as falsifying the clause. **Audit 10 carries it one step further, to the harm:** with a fake HOME holding `~/.git-credentials`, the CONTROL (the file exactly as `write_gitconfig_in` leaves it) answers `git config --get credential.helper` at **exit 1, no helper** — the clause holds; after `printf '[credential]\n\thelper = store\n' >> <env>/<alias>/gitconfig`, which is **exit 0 through the guard** (a REDIRECTION — fail-open direction (i)), the helper resolves and **`git credential fill` for `https://github.com` returns the ambient username and password from `~/.git-credentials`**. Both halves of the clause are then false: the generated file DOES name a credential helper, and the run DID reach the ambient credentials. That is `T-19-23` (**high**, `closed`) — whose declared mitigation is precisely *"the helper test plants `credential.helper = store` in a fake HOME and proves the control resolves it first"*, a control that consists of the generated file naming no helper — and it is D-16 and SAFE-05's whole subject. **What is and is not covered by an existing acceptance, stated rather than assumed:** `AR-19-04` accepts `T-19-29` (*"agent rewrites the generated git config"*, **medium**) with the rationale *"the envelope regenerates it at each run start"*, which `19-26`, `19-27` and audit 9 all record does NOT cover a write DURING the run. That acceptance is about TAMPERING with the config at medium; it says nothing about credential reachability, and the consequence measured here is `T-19-23`'s subject at high. **Audit 10 does NOT un-accept `AR-19-04` and does NOT move `T-19-23` to open — un-accepting a risk is a human decision and this audit does not make it.** It registers the measured consequence as its own row. **Rule (a) reaches four of the six write spellings** and the two that matter do not need them: `cp`, `sed -i`, `tee -a` and a `python3 -c` payload naming the absolute path are all **exit 2**; `printf … >>` and `cat … >>` are **exit 0**. **And `19-27` repaired the `Guaranteed` CAP clause while leaving this `Guaranteed` CREDENTIAL clause unqualified**, although its own predecessor recorded the falsification — one clause over from the repair. | **OPEN — BLOCKING** |
| T-19-115 | Spoofing / Elevation of Privilege | rule (a)'s `LITERAL` condition versus its own **"FOUR named directions"** enumeration (`policy.rs:5293-5325`), and the fifth axis's carrier alphabets (`tests/envelope_wrapper_class.rs:8057`, `:8094`) | medium | mitigate | **THREE FURTHER SPELLING CLASSES reach every one of `C-01` … `C-09`, none is among the four disclosed directions, and no alphabet on the new axis can draw one.** Measured with the ABSOLUTE LITERAL control at exit 2 beside each, and with `bash` confirming the shell really reaches the file: **TILDE** — `rm -rf ~/<env>/alpha` → **exit 0**, alias directory really removed (nine carriers, one call); `cp /bin/true ~/<env>/alpha/hooks/pre-push` → **0**, stub really replaced with an ELF binary; **and against the REAL envelope root the tilde spelling is the natural one** — `rm -rf ~/.local/share/gsd-meta-manager/envelope/alpha` is **exit 0** while `rm -rf /home/<user>/.local/share/gsd-meta-manager/envelope/alpha` is **exit 2**. **GLOB** — `rm -rf <env>/alph?` → **0**, directory really removed; `rm -f <env>/alpha/*` → **0**; `rm -f <env>/alpha/pr-ledger.ndjso?` → **0**. **BRACE EXPANSION** — `rm -f <env>/alpha/{pr-ledger.ndjson,x}` → **0**. **Mechanically these are the LITERAL condition failing, and that condition IS stated** — round 5's own literalness table names expansion, pathname, tilde and brace as the four classes that clear the bit, so a reader of the CONDITION is not misled. **A reader of the RESIDUE is.** The doc says *"It fails OPEN in FOUR named directions"* and names a redirection target, a `$(…)` operand, a symlink and a relative path; every cited spelling is `$`-shaped and the cited costs are `rm $TMPDIR/x` and `cp "$SRC" "$DST"`. Neither a tilde nor a glob nor a brace list is any of the four, and each needs **no prior read, no symlink and no cwd**. This is the `T-19-84`/`T-19-107`/`T-19-109` shape a fifth time: a residual disclosure whose words are pinned to exist rather than to be complete. **And it is `T-19-76`'s failure mode for the TENTH consecutive round**: `ENVELOPE_ROOT_OPERAND_CARRIERS` draws eleven entries and every one is an ABSOLUTE LITERAL path; `CONTROL_CARRIER_EXPANSION_BORNE` is exactly two `$(…)` entries; **not one entry of any of the seven classes carries a `~`, a `*`, a `?` or a `{a,b}` in a carrier path** — so the axis is structurally incapable of failing on the class, in the cell one over from the one it filled. The irony is worth recording: rule (a) DECIDES on `Token.literal`, round 5's own bit, and inherits round 5's whole class list by construction — while the round-10 corpus draws only the one class the rule catches. **`T-19-114`'s successor.** | open — below `high` (non-blocking) |
| T-19-117 | Denial of Service / Elevation of Privilege | `C-01`, the PR-cap ledger, as **unbounded work on the guard's critical path** — `ledger::tally` over a whole-file read, against `GUARD_TIMEOUT_SECS = 5` (`hooks.rs:1408`, `:1504`) | medium | mitigate | **The ledger is not only the cap's RECORD; it is work the guard must finish before its registered deadline, and its size is set by a command the guard permits.** Measured on this machine against the built binary, one root, entries dated outside the 24 h window so the cap itself is unaffected: empty ledger **5 ms**; 10,000 lines (0.9 MB) **49 ms**; 200,000 lines (19 MB) **0.95 s**; **2,000,000 lines (190 MB) 7.69 s — past the registered 5 s timeout.** The cost falls **only on the forge commands the cap governs** and audit 10 measured that rather than assuming it: against the same 190 MB root, `ls` 6 ms, `git status` 5 ms, `git push --force origin main` 4 ms (exit 2), while `gh pr create` 7.79 s, `gh api …/pulls` 7.58 s and `glab mr create` 7.65 s. **The inflation is delivered by fail-open direction (i)** — `yes … >> <ledger>`, `head -c … /dev/zero >> <ledger>` and `cat <big> >> <ledger>` are all **exit 0** through the guard after `19-27`, because a redirection target is not an operand. `ledger.rs` reads the file whole and applies no size bound. **The BEHAVIOURAL half is UNMEASURED and is claimed in NEITHER direction**: what the agent CLI does with a `PreToolUse` hook that exceeds its registered timeout is a property of a closed-source binary and cannot be driven here — the same discipline `C-08`'s behavioural half is held to. Rated `medium` on the mechanical half alone. Recorded because it is the one answer to *"what changes WHEN the guard runs"*, and because the three latency rules `hooks.rs:771-816` states — no network, one repository consultation, *"the ledger read is a single pass over one append-only file"* — assume the file's size is the envelope's own business, and after `19-27` it still is not. | open — below `high` (non-blocking) |

*Status: open · closed · open — below `high` threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` count toward `threats_open`*

`T-19-115` and `T-19-117` are open below the `high` threshold and do **not** count
toward `threats_open`. `T-19-116` and `T-19-118` do.

### `T-19-114` — CLOSED AS SCOPED by `CONTROL_CARRIER_CLASSES`, successor `T-19-115`

The mandate asks directly whether `T-19-114` — the corpus blind spot — is
addressed by the fifth axis. **It is, for the spelling the rule catches, and the
axis is a real one rather than a wider alphabet of the same kind.** Verified by
reading it: `CONTROL_CARRIER_CLASSES` carries seven classes against
`MIN_CONTROL_CARRIER_CLASSES = 7`, seven representatives, six verdict-preserving
alphabets with per-alphabet floors, and `tests/envelope_control_carrier.rs` is a
ninth evidence file at 37 tests. The axis-defining property is asserted from the
guard's OWN resolver rather than from a corpus predicate, in both directions, and
it demonstrably failed before the rule — `19-27` re-confirmed all ten RED names
against the unmodified tree at `22bb5f7` before a production line moved, and
audit 10 re-read that list against `19-26-SUMMARY.md`'s: the two sets are
identical.

**It closes on this file's own convention — closed for its measured rows, residual
carried on a new row — and the residual is `T-19-115`:** every entry of
`ENVELOPE_ROOT_OPERAND_CARRIERS` is an absolute literal path, and no entry of any
of the seven classes carries a `~`, a `*`, a `?` or a `{a,b}` in a carrier path.

### `T-19-112` and `T-19-113` — the NARROWING is real, and nothing anywhere calls it closure

Both verified, and both directions of the claim checked.

**The narrowing is real.** Nineteen envelope-root operand rows re-measured at
`0092009`, fresh root per row, walk after — every one **exit 2
`envelope_assertion_failed`** with an EMPTY walk: the four ledger spellings
(`rm -f`, `truncate -s 0`, `cp /dev/null`, `shred -u`), `rm -rf <env>/alpha`,
both hook stubs, `rm -rf <env>/alpha/hooks`, the askpass responder, the literal
`hooks/../pr-ledger.ndjson`, `ln -s` and `cd` (directions (iii)/(iv)'s measured
partial mitigations), the three disclosed over-refusal reads, the
`NestedPayload` re-entry `bash -lc "rm -f <ledger>"`, and both GOVERNED-carrier
spellings. The ordering pins land at their deliberately different identifiers —
`rm -f <ledger> && git push --force` at `envelope_assertion_failed`,
`git push --force && rm -f <ledger>` at `force_push_blocked`, the control
`rm -f /tmp/x && git push --force` at `force_push_blocked`,
`git -c core.hooksPath=/dev/null config --file <env>/alpha/gitconfig --get x` at
`envelope_assertion_failed` and its control `git -c core.hooksPath=/dev/null
status` at `hook_bypass_blocked`, unperturbed. Both near-miss controls
(`rm -f /tmp/pr-ledger.ndjson`, a root one character off) and every
ordinary-operand row stay at exit 0, and `git config --get core.hooksPath` is
pinned unchanged at exit 0.

**Nothing anywhere describes it as closure.** `rtk proxy grep` over `src/`,
`tests/` and the phase's `.md` files for `T-19-112` beside the word *closed*
returns only rows saying NARROWED, NOT CLOSED or open. `19-27`'s own record and
SUMMARY both open with *"NARROWED, explicitly NOT closed, and narrowed in ONE
ROUTE OF SEVERAL"* and name `C-15` and the deferred option (b) as the others.

**And the routes it does not reach were re-driven rather than accepted.**
`C-15` reproduces exactly as `19-26` recorded it: with `pr_cap_per_24h` and
`pr_cap_per_run` raised to `9999` in `~/.config/gsd-meta-manager/config.json`,
four `gh pr create` calls in one persistent root are **all permitted with four
ledger lines**, against a DEFAULT-caps control in which calls 2, 3 and 4 are
refused at `pr_cap_exceeded`; and the MID-RUN replacement is observed — two calls
at the defaults (0, then 2), the config replaced, calls 3 and 4 at **exit 0**.
The write itself is exit 0. **Audit 10 records one mechanical detail `19-26` did
not**: `envelope guard` ignores the global `--config` flag and always reads
`Config::default_path()` (`hooks.rs:837-840`), so `C-15` is steered by
`HOME`/`XDG_CONFIG_HOME` and not by argv — which is why the first drive of this
row against `--config` did not reproduce and had to be re-driven. That is a
narrowing of the route, not a closing of it.

### The four fail-open directions — re-measured, and handed to NO control

All ten rows pinned PERMITTED by `19-26` are still permitted at `0092009`:
`: > <ledger>`, `printf 'exit 0' > <hooks>/pre-push`, `echo evil > <askpass>`,
`cat /tmp/evil >> <gitconfig>` (i); `D=$(git config --get core.hooksPath); rm -f
$D/../pr-ledger.ndjson` and the `cp /bin/true $H/pre-push` twin (ii);
`rm -f /tmp/l` and `: > /tmp/l` (iii); `rm -f pr-ledger.ndjson` and
`cp /bin/true pre-push` (iv).

**Direction (ii)'s composition is exactly as the mandate states it, and audit 10
confirmed the resolution rather than the verdict.** The first segment
`D=$(git config --get core.hooksPath)` resolves `Governed { index: 0 }` — a
PERMITTED governed `git config --get`, the same read `CONTROL_CARRIER_GOVERNED_TWIN`
pins at exit 0 — while the segment that ACTS on the carrier reaches nothing
governed. The carrier's location is fetched by a permitted governed READ and
acted on by an ungoverned command.

**Direction (i) is real by MECHANISM**, and audit 10 re-derived that rather than
citing it: `SEPARATORS` is byte-identical at `policy.rs:2297` and
`git log -L` over that line returns exactly **one** commit in the whole phase
(`84a9b05`, plan 19-05); `is_separator` is a bare `SEPARATORS.contains`
(`policy.rs:3542-3544`) so `is_separator(">") == false` by construction; and
round 6's over-deletion control `git x2>/tmp/o push --force origin main` is still
exit 0.

**None of the four is handed to a pin, a schedule or a witness.** The four test
functions assert them **PERMITTED** — verdict-preserving, so a rule that quietly
widened turns red — and claim no coverage. `rtk proxy grep` for a revisit
condition or a version witness naming any carrier direction in `policy.rs`
returns nothing, and the predicate's doc states in terms that none was created
and why: `19-25`'s witness schedules a re-audit against a FUTURE GIT, and these
are reachable by the agent TODAY. **That reasoning is correct and audit 10
endorses it.**

**What audit 10 adds is that FOUR is not the count.** See `T-19-115`.

### `SECTION_ENVELOPE`'s repaired cap clause — TRUE as written, and it claims nothing more

Verified against the constant itself (`advisory.rs:250-251`):

> `The pull-request count lives in` / `an append-only ledger this repository does not contain.`

**Mechanically true**: the ledger is `<envelope>/<alias>/pr-ledger.ndjson`
(`ledger.rs:47`), the envelope root is `dirs::data_local_dir()` (`mod.rs:181`),
and `envelope_tracer.rs:163-167` asserts the directory is outside the worktree.
**It states a LOCATION FACT and stops.** It names no control, makes no
completeness claim, enumerates no direction, and says nothing about whether the
cap can be reset. The `Not guaranteed` half now carries *"An agent that unsets
GIT_CONFIG_COUNT in a subshell, **or rewrites the hook stubs and ledger the
envelope installed**, is past the last layer"*, which is D-09's corrected
narrative. **Decision endorsed** — a clause listing four directions and stopping
would have implied a completeness the measurement denies, and `C-15` alone is a
fifth route.

**One clause over, the same defect is unrepaired**, and audit 10 records it
beside the endorsement rather than as a qualification of it: the FIRST
`Guaranteed` clause — *"This run cannot reach your ambient git credentials or SSH
agent: … global and system git config is a generated file naming no credential
helper"* — is falsified by measurement, `19-26` recorded the falsification, and
`19-27` left the clause unchanged. That is `T-19-118`.

### `T-19-111` — the attribution correction, verified at all five sites

| # | Site | Verified |
|---|---|---|
| 1 | `src/envelope/policy.rs:1183` | **EDITED.** Reads *"That is `T-19-111`'s shape and it stays OPEN."* The *"or one written by a means that is not `git config`"* half is KEPT, and the paragraph states that no rule and no acceptance is made. Item 2 of the same list is still `T-19-86` and is **correct as written** — a `!` body carrying its own carrier IS a command line handed to a governed program as data |
| 2 | `src/envelope/cred.rs:316-317` | **EDITED.** The dropped half is RESTORED — *"predating the run, **or one written by a means that is not `git config`**, is live"* — and re-attributed: *"That is `T-19-111`, not `T-19-86`"* |
| 3 | `19-SECURITY.md:7430` | **CORRECTED BESIDE**, in `19-27`'s own record, quoted and never edited |
| 4 | `19-25-SUMMARY.md:265` | **CORRECTED BESIDE**, same |
| 5 | `deferred-items.md:1619` | **CORRECTED BESIDE**, in `19-27`'s own `deferred-items.md` section, under the heading *"the closure entry corrected (site 5 of the attribution correction)"*. The line-1619 entry itself is untouched, which is the right handling |

**`T-19-111` remains OPEN at `high` with NO rule**, and audit 10 re-measured its
rows: the `printf`-written `.git/config` alias, `git fp`, and all six further
write spellings (`echo >>`, `cat >>`, `tee -a`, `sed -i`, `python3 -c`, `cp`) are
**all exit 0 with EMPTY walks**. Its corpus rows are `record_only` and asserted in
neither direction — verified by reading
`no_repo_side_row_is_asserted_and_this_file_says_so_mechanically`, which reads
the file itself. **No acceptance was made for it anywhere**, and audit 10 makes
none either.

### The known-open set, as audit 10 verified it

* **`T-19-86` (high, OPEN, unaccepted, deferred by explicit user scoping
  decision).** All four registered rows re-measured at `0092009`, fresh root
  each, **all exit 0** with EMPTY walks; the persisted-alias arm reproduces
  (`git config alias.p "!git push --force origin HEAD:refs/heads/main"` → 0,
  `git p` → 0). Counts toward `threats_open`.
* **`T-19-91` (high, OPEN, three arms).** `S=x; git reflog $S`, `git reflog show
  $S` and `git symbolic-ref $S` all **exit 0**; `git symbolic-ref HEAD $R` → 2
  `force_push_blocked` and `git push origin $REF` → 2 `push_outside_namespace`
  still fail closed. Unweakened by round 10. Counts toward `threats_open`.
* **`T-19-96` (medium, OPEN).** `git push --forc? origin
  refs/heads/gsd-auto/alpha/w` → **0**, literal twin → 2 `force_push_blocked`.
* **`T-19-110` (medium, OPEN).** `git config core.hooksPath -c` and `--` → **0**;
  the by-name deny `git config core.hooksPath /dev/null` → 2
  `hook_bypass_blocked`. Audit 9's re-rating carried forward unchanged.
* **`T-19-74` (medium, closed/accepted — AR-19-10).** Core rows frozen.
  **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85`** — open, unaccepted by explicit
  user decision, untouched: `git diff --numstat 22bb5f7..HEAD -- src/` is exactly
  four files (`advisory.rs`, `cred.rs`, `hooks.rs`, `policy.rs`), and `mod.rs`,
  `scan.rs`, `config.rs` and `ledger.rs` were not opened.
* **`FORGE_VALUE_OPTS` keeps `--hostname`** — verified present with audit 9's
  measured reason recorded at `policy.rs:4840-4858`. **`glab` is confirmed NOT
  installed** (`command -v glab` finds nothing), so the `--host` cell is
  unconfirmed against its callee and audit 10 does not upgrade it either.
* **`C-08`'s second carrier.** The mechanical half is settled and audit 10
  re-derived it: `rtk proxy grep -rn 'settings_json' src/` is **exactly two
  hits** — `hooks.rs:1483` (the doc row claiming argv delivery) and
  `hooks.rs:1531` (the definition) — while production pushes `--settings <path>`.
  **The behavioural half is still UNMEASURED**; `19-27` did not repair it and
  said so. Audit 10 does not claim it in either direction.
* **`AR-19-04`.** Its mitigation *"regenerates it at each run start"* does not
  cover a write during the run — the same narrowing shape as `cred.rs`'s
  *"predating the run"* and `resolve_policy`'s own doc, three instances, now with
  a measured consequence beside it (`T-19-118`). **RECORDED and deliberately NOT
  un-accepted. Accepting or un-accepting a risk is a human decision and this
  audit does not make it.** No `AR-` row was added, edited or renumbered.
* **`T-19-17r` — OUTSTANDING for the SIXTH audit running.**
  `grep -cE '^\| AR-19-13 \|'` over this file is **0**. Plans `19-26` and `19-27`
  both correctly declined. **This audit is the tenth agent to leave the
  acceptance unmade.** The next round either adds the log row or drops the word
  from `19-17-SUMMARY.md`.
* **`T-19-SC` — still holds.** Neither `Cargo.toml` nor `Cargo.lock` appears in
  any commit between `cc65220` and `0092009`.

### The mechanism pins, verified non-dead

Rounds 4 through 9 are all still load-bearing and all still right about the lines
round 10 refuses:

```
exit=2 [envelope_assertion_failed]  git pus? --force origin main                     <- round 5's literalness bit
exit=2 [force_push_blocked]         git >/dev/null push --force origin main          <- round 6's deletion model
exit=0                              git x2>/tmp/o push --force origin main           <- round 6's over-deletion control
exit=2 [force_push_blocked]         git --attr-source HEAD push --force origin main  <- round 7
exit=2 [envelope_assertion_failed]  git --bogus-opt status                           <- round 7's inversion
exit=0                              git - push --force origin main                   <- round 7's pinned control
exit=2 [force_push_blocked]         git -- push --force origin main                  <- its twin
exit=2 [envelope_assertion_failed]  git -c include.path=/tmp/evil.cfg status         <- round 8's confinement clause
exit=2 [hook_bypass_blocked]        GIT_CONFIG_PARAMETERS="'core.hooksPath=…'" git commit -m x
exit=2 [hook_bypass_blocked]        env -u GIT_CONFIG_COUNT git push --force origin main
exit=2 [envelope_assertion_failed]  V=push; git $V --force origin main               <- round 4's Rule A
exit=2 [envelope_assertion_failed]  git {push,--force} origin main                   <- round 5's brace splice
exit=0  (ONE ledger line)           gh api repos/{owner}/{repo}/pulls -f title=x     <- T-19-93 on the COUNT bar
exit=2 [envelope_assertion_failed]  git -c alias.q="-c include.path=… status" q      <- round 9's re-parse clause
exit=2 [envelope_assertion_failed]  git config alias.p '-c include.path=… push …'    <- round 9, region 2
exit=0                              git -c aliasx.q=x status · git -c notalias.q=x status · git -c a=b status
exit=0                              { git status; } · ( git status )
exit=2 [force_push_blocked]         { git push --force origin main; }
```

`Token.literal` was neither cleared nor repurposed — audit 10 confirms round 10
READS it in a second place and changes it nowhere.

### The byte floors and the stripper's three protections, RE-DERIVED

Measured at `0092009` with each file's own definition of "production half", using
the stripper's own trimmed-equality predicate rather than a substring count:

| Measurement | Audit 10's number |
|---|---|
| `policy.rs` production half | **277,570 bytes**, 276,660 characters, 5,414 lines |
| `hooks.rs` production half | **78,465 bytes**, 1,636 lines |
| `#[cfg(test)]` sentinels by trimmed equality | `policy.rs` **1** (line 5,415); `hooks.rs` **1** (line 1,637) |
| deep anchor `fn forbidden_repo_path` | present, line **5,157** |
| `fn scan_leading` | line **416** |
| 40,000 bytes reached at | line **752** |
| 180,000 bytes reached at | line **3,613** |

`POLICY_MIN_PRODUCTION_BYTES = 40_000` and `HOOKS_MIN_PRODUCTION_BYTES = 20_000`
are unchanged and both pass with wide margin; the proportional floor
`production.len() >= 180_000` passes at 277,570 — **64.85%**, written as 64.8%.
**Every one of the corrected comment's numbers and line references is exactly
right, and audit 10 checked each independently rather than reading them.**

### Gates observed

`rtk proxy cargo test --no-fail-fast` redirected to a file and counted with
**`rtk proxy grep`** (D-34): **1771 passed, 0 failed, 13 ignored** over **45**
result lines, matching `19-27-SUMMARY.md` exactly. **All SIXTEEN `envelope_*`
binaries ran**, so round 10's evidence file executed —
`envelope_control_carrier` is present. **No documented flake fired in this run.**
`19-27` recorded one firing (`driver_reattach::a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`);
audit 10 records none. **Absence is not evidence they are fixed**, and one firing
would not have been evidence of a new defect. `cargo clippy --tests -- -D
warnings` was already failing at the base on four pre-existing lints in
`src/browser.rs` and `src/project_creator.rs` and is out of scope.

**Provenance discipline.** Plans 19-13 … 19-27's appended subsections, the
plan-19-21 blocker-row resolution record and every earlier audit's own tables are
left **byte-identical**; audit 10 checksummed the whole body below the
frontmatter before writing and verified afterwards that the write is append-only
apart from the frontmatter, one trail row and one method subsection. Audit 10's
corrections to statements made in those subsections — that FOUR is not the count
of rule (a)'s fail-open directions, and that `C-10` has no disposition — are
recorded as audit-10 findings BESIDE them rather than as edits to them.

### Audit 10's bookkeeping, re-derived from audit 9's group rows

| Group | Count | Closed | Open |
|---|---|---|---|
| Everything through audit 9 | 126 | 102 | 24 (4 at `high`) |
| Closed by plans 19-26 / 19-27, re-measured by audit 10 (`T-19-114`, as scoped) | — | +1 | −1 |
| Found by audit 10 (`T-19-115` … `T-19-118`) | 4 | 0 | 4 (2 at `high`) |
| **Total after audit 10** | **130** | **103** | **27 (6 at `high`)** |

The six that count toward `threats_open`: `T-19-86`, `T-19-91`, `T-19-111`,
`T-19-112`, `T-19-116`, `T-19-118`. The twenty-one that do not: `T-19-61` …
`T-19-73` (13), `T-19-84`, `T-19-85`, `T-19-96`, `T-19-105`, `T-19-110`,
`T-19-113`, `T-19-115`, `T-19-117`.

### The four execution-time judgements, assessed independently

1. **`19-26` refusing to write the converse degenerate-proof its own plan
   mandated — CORRECT, and the substitute is the real proof.** The plan's form is
   genuinely false and audit 10 re-derived that rather than accepting it:
   `DELETION_CLASSES`' `draws_a_separate_word_redirection` is a pure text
   function and `: > <ENV>/alpha/pr-ledger.ndjson` really does carry a
   separate-word redirection, so asserting the plan's form would have landed
   permanently red on something untrue. **The substitute is the property the axis
   is actually about**, asserted in three parts and read from the guard's OWN
   resolver rather than from a corpus predicate: (A) every control-carrier
   entry's **carrier-bearing** segment resolves `NoProgram`/`Ungoverned`; (B)
   every existing-axis representative reaches a governed program; (C) no
   existing-axis representative satisfies any of the SIX carrier classes, with
   class 7 excluded because it is the complement of the six and the reason
   stated. **And the overlaps are honestly recorded** — computed over every
   alphabet against all four existing class predicates and `println!`ed under the
   heading *"RECORDED (not asserted)"*, never asserted. The carrier-bearing
   refinement was itself found by measurement and is right: direction (ii)'s
   first segment really does resolve `Governed { index: 0 }`, which audit 10
   reproduced.
2. **`19-27` substituting two governed-program carrier rows for the plan's —
   CORRECT, and both substitutes are non-vacuous.** Measured: the plan's own row
   `git config --file <ENV>/alpha/gitconfig alias.x '…'` is **exit 2**, and its
   control OUTSIDE the envelope is **also exit 2 at the SAME identifier**
   (`envelope_assertion_failed`, by round 9's re-parse clause over `alias.x`) —
   so the row certifies nothing and would have stayed green under the mis-placed
   rule it was written to catch. The two substitutes both discriminate:
   `git config --file <ENV>/alpha/gitconfig --get user.name` → **2** with its
   outside control at **0**, and `git --git-dir <ENV>/alpha/gh status` → **2**
   with its control at **0**. The plan's spelling is `record_only`'d with its
   overlap stated, which is the right disposition — an assertion on the
   identifier alone could not tell the two mechanisms apart.
3. **The byte/character discrepancy — RESOLVED, not papered over, and audit 10
   verified it to the byte.** At `19-27`'s base `22bb5f7` the production half is
   **262,229 bytes** and **261,387 characters** — audit 9's 261,387 is exactly
   the character count, and `production.len()` is `String::len()`, which is
   bytes. After `19-27`'s own additions it is **277,570 bytes / 5,414 lines**,
   and `180,000 / 277,570 = 64.85%`. **The floor comment is now TRUE**, including
   every corrected line reference — `fn scan_leading` at 416, 40,000 bytes at
   752, 180,000 at 3,613, `fn forbidden_repo_path` at 5,157 of 5,414 — each
   re-derived independently here. **One editorial imprecision, recorded and not a
   finding**: the comment says the file carries *"842 multi-byte characters"*.
   842 is the byte-minus-character DIFFERENCE; the count of multi-byte characters
   is 421, each three-byte character contributing two extra bytes. Every number
   written into the assertion is correct and the conclusion is unaffected.
4. **`mod.rs` read and DECLINED — the right call, and it held mechanically.**
   `git diff --numstat 22bb5f7..HEAD -- src/` is exactly four files and
   `src/envelope/mod.rs` is not among them, so the plan's own verification —
   that `git diff --stat` touch no file outside `files_modified` — is satisfied
   rather than asserted. **Taking the plan's own stated fallback over silently
   widening the diff is correct**, and this phase has twice been damaged by a
   round that expanded its own scope. The cost is one round of a stale sentence
   in a residual-limit note about `GSD_MM_ENVELOPE_ROOT`, a variable audits 9 and
   10 both measured INERT — the smallest cost available. **Audit 10 adds one
   thing for whoever picks it up**: `mod.rs:165-168`'s argument that the override
   *"does not lower a boundary that was standing"* because the same party *"can
   equally unset `GIT_CONFIG_COUNT`"* is D-09's uncorrected narrative in its
   third home, and `T-19-116` is a route that lowers a standing boundary without
   controlling the environment the TUI starts in.

---

## Audit 10 — what the round-10 control can and cannot fail on

### The principle rounds 3 through 9 established still holds

A decision region must come from the same scan the classifier runs, and there is
one walk. Rule (a) is raised **once**, at the top of `classify_segments`' existing
per-segment loop, on the segment the loop already holds, before
`resolve_program_with_head` — one hunk, `@@ -966,0 +967,64 @@`, 64 insertions and
**zero deletions**. No second pass, no second scan, no third reading site, no new
`ParkReason`. The two ordering pins at deliberately different identifiers are the
mechanical proof, and audit 10 re-measured both pairs. `resolve_program`,
`resolve_program_with_head`, `first_unreadable_decision_word`,
`config_key_operand_index`, `subcommand_word_indices`, `scan_gh_api` and
`scan_leading` are untouched, and **not one config-resolution verdict moved**.

The predicate itself is a pure lexical path test: no `canonicalize`, no
`read_link`, no `Command::new`, no program name and no filename — which is what
`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic`
requires one level over. **The placement decision — before the resolution match
rather than inside the `Ungoverned` arm — is right and audit 10 measured why**:
`git config --file <ENV>/alpha/gitconfig --get user.name` resolves `Governed` and
never reaches that arm, and it is refused.

### Where the boundary now is, in one paragraph

**Rule (a) decides on one lexical fact: a word that is ABSOLUTE, LITERAL and,
lexically normalised, under `envelope_dir_in(root, alias)`.** Each of the four
conditions is a boundary rather than a convenience, and each is also a place the
rule is silent — and its silence is a permit. Round 10 names four of those
silences. **There are seven**, and the three it does not name are the three
spellings of a path that clear `Token.literal` without a `$`: a **tilde**, a
**glob**, and a **brace list** (`T-19-115`). Beyond the rule's own silences, the
plane it defends is dispositioned unevenly:

* **nine carriers got rule (a)** — `C-01` … `C-09`, verified refused;
* **five got control (e)**, no rule, registered, unaccepted — `C-11` … `C-15`;
* **one got nothing** — `C-10`, the binary every stub and the guard itself execs,
  and it is the widest of the fifteen (`T-19-116`);
* **one carrier's measured harm outran its record** — `C-05`, where `19-26`
  measured a helper reading back and audit 10 measured the ambient credential
  coming out (`T-19-118`);
* **one carrier has a second consequence nothing records** — `C-01`, whose SIZE
  is work the guard must finish inside a five-second deadline (`T-19-117`).

**The one-sentence version for the next round.** Ten rounds have modelled how a
command line becomes behaviour and how a file becomes a control, and there is no
eleventh plane to find — **what is left is that the tenth plane has fifteen
carriers and only nine of them have a control.**

### Suggested closure, in order — (d) FIRST, for the tenth round running

1. **(d) — widen the corpus BEFORE certifying anything, and this time widen the
   SPELLING rather than the kind.** Add `~`-bearing, `*`/`?`-bearing and
   `{a,b}`-bearing carrier paths to `ENVELOPE_ROOT_OPERAND_CARRIERS` and to the
   direction alphabets, with a floor beside `MIN_CONTROL_CARRIER_CLASSES`
   asserting the axis can draw each. **Listed first for the tenth round running.**
   Round 5 already owns the class list rule (a) decides on; the corpus simply does
   not draw from it.
2. **(a) — `T-19-116` FIRST among the fixes, because it is the widest and the
   cheapest to REGISTER.** The registration is the honest first step and it costs
   nothing: give `C-10` a threat id, a severity and a control letter, so the
   carrier that takes layer 2 and layer 3 in one call appears in `threats_open`.
   A rule is genuinely hard — the binary is outside the envelope root and its path
   is `current_exe()` — and may honestly end in an acceptance; **that acceptance
   is a human decision and this audit does not make it.** The cheapest real
   control is the one option (b) is structurally denied for the stub but not for
   the binary: the stub could carry the binary's expected digest, so a replaced
   target is a refusal rather than a silent `exit 0`.
3. **(b) — `T-19-118`.** Either the `Guaranteed` credential clause gains the same
   kind of repair the cap clause just got, or the generated `gitconfig` gains a
   protection; the current pair cannot both stand. Note that rule (a) already
   refuses four of its six write spellings, so the remaining gap is direction (i)
   — which means this row and `T-19-112` share a residue and a fix for one is a
   fix for the other.
4. **(c) — `T-19-115`**: correct the residue's own arithmetic. Say what the
   condition is (a word the shell may rewrite) rather than enumerating four
   spellings of it, or enumerate all seven.
5. **(e)** — then `T-19-86`, `T-19-91`, `T-19-96`, `T-19-110` and `T-19-111`,
   which remain five arms of two shapes: a classifier arm answering `Allow` on an
   operand outside the decision region, and a carrier that is not a command line.

Then, and separately: add the `AR-19-13` row for `T-19-17r` or stop calling it
accepted; measure `C-08`'s behavioural half in a live agent session; correct
`mod.rs:165-168`'s ceiling paragraph, now the last uncorrected home of D-09's old
narrative; and bound the ledger read (`T-19-117`).

---

## Audit 10 Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log — twelve, unchanged by
      round 10; **audit 10 accepts nothing and un-accepts nothing**
- [x] Every closure re-measured against the built binary at `0092009` with a
      fresh envelope root per row and the root walked afterwards, the walk proved
      non-blind on every pass by `gh pr create --title x` leaving exactly one
      `alpha/pr-ledger.ndjson` line
- [x] `T-19-112`/`T-19-113`'s narrowing verified REAL over nineteen refused rows
      with both ordering-pin pairs at their different identifiers, and **nothing
      anywhere describes it as closure**
- [x] All ten fail-open rows re-measured PERMITTED, direction (ii)'s
      `Governed { index: 0 }` first segment confirmed, and **none handed to a
      pin, a schedule or a witness**
- [x] `C-15` re-driven END TO END with a default-caps control — four permits at
      raised caps, the mid-run replacement observed
- [x] `T-19-111`'s attribution verified at ALL FIVE sites; two edited, three
      corrected BESIDE; the row stays OPEN at `high` with **no rule** and its
      corpus rows RECORDED not asserted
- [x] `SECTION_ENVELOPE`'s repaired cap clause verified TRUE as written, stating
      a location fact, **claiming nothing more and enumerating nothing**
- [x] Every new finding confirmed against the **REAL `git` binary** with a
      CONTROL beside every leg — including a replaced exec target that let a
      force push the hook had refused **MOVE a bare remote's `main`
      (`01f9664` → `2537e97`)** with `GIT_CONFIG_COUNT` UNTOUCHED, restored to a
      refusal by putting the binary back; and a `credential.helper = store`
      appended to the generated gitconfig that made `git credential fill` return
      the ambient `~/.git-credentials` secret, against a control in which the
      same file named no helper
- [x] `SEPARATORS` byte-identical (`policy.rs:2297`, ONE commit in the phase,
      `84a9b05`) and `is_separator(">") == false` by construction; rounds 4
      through 9's mechanism pins all re-measured non-dead
- [x] Byte floors re-derived by the stripper's own predicate: `policy.rs`
      production half **277,570** bytes / 5,414 lines against 40,000 and 180,000
      (64.85%); `hooks.rs` 78,465 against 20,000; exactly ONE `#[cfg(test)]`
      sentinel per file; deep anchor present at line 5,157
- [x] Plans 19-13 … 19-27's appended subsections left byte-identical, verified by
      checksum before and after writing
- [x] Gate observed: `rtk proxy cargo test --no-fail-fast` → **1771 passed, 0
      failed, 13 ignored** over 45 result lines; **all SIXTEEN** `envelope_*`
      binaries ran; no documented flake fired
- [ ] `threats_open: 0` confirmed — **6 open at `high`: `T-19-86`, `T-19-91`,
      `T-19-111`, `T-19-112`, `T-19-116`, `T-19-118`**
- [ ] `status: verified` set in frontmatter

**Approval: blocked, 2026-09-04 (audit 10).**

**Not accepted here.** `T-19-116` removes layer 2 and layer 3 with one permitted
command and moved a bare remote's `main` under a control. `T-19-118` puts the
user's own git credential back inside a driven run and falsifies the first thing
the honesty statement guarantees. `T-19-86` and `T-19-91` remain open at `high`
by scoping decision and by round discipline. `T-19-111` and `T-19-112` remain
open at `high` with no rule and a partial rule respectively. Accepting any of the
six is a human decision and this audit does not make it.

**Round 10 was right about the plane, and audit 10 says so plainly.** `19-26`
enumerated fifteen carriers exhaustively, refused to write a false assertion its
own plan mandated and wrote the true one instead, and left ten rows RED with zero
`src/` hunks — the evidence a corpus can fail before a control exists. `19-27`
confirmed every one still red before a production line moved, added the converse
check nobody had asked for, found by measurement that its plan's own certifying
row would have certified nothing, raised the rule at the one place that reaches a
governed program touching a carrier, moved a live bypass OUT of a threat the user
scoped out at five sites, and repaired a `Guaranteed` claim into a statement that
makes no completeness claim at all. **Every one of those is the behaviour this
phase has spent ten rounds learning.**

**And the answer to the tenth question is the plain one: there is no eleventh
layer.** Nine rounds modelled an argv and the tenth modelled a file, and audit 10
swept the environment, the clock, the network, the guard's own state, its
registration, its deadline and its CLI surface and found nothing on a new plane.
**What it found is on round 10's own plane, unfinished.** Fifteen carriers were
enumerated; nine have a control, five are disclosed with none, and one — the
binary that every hook stub and the guard itself execs — was written down, marked
UNOBSERVED, and then given no threat id, no severity and no control letter. It is
the widest carrier of the fifteen. **The tenth question was the right one. The
eleventh is not a new question at all: it is finishing the answer to the tenth.**

---

## Execution record — plan 19-28 (the corpus, round 11). NOT an audit finding.

**This section is a record made by plan 19-28. It is not an audit finding, it
amends no audit table, and it edits nothing above this line.** Audit 10's body
was checksummed before this append; the Security Audit Trail, the Accepted Risks
Log, the Sign-Offs and every subsection plans 19-13 … 19-27 appended are
untouched.

### FIRST, and in these words: this plan closes nothing, and `/gsd-secure-phase 19` is NOT cleared

- **`T-19-86` remains OPEN at `high`** by explicit user scoping decision. Its four
  registered rows are at exit 0 and its pins are green and UNMODIFIED. `T-19-111`
  is kept OUT of it.
- **`T-19-91` remains OPEN at `high`**, its three arms unweakened.
- **`T-19-111` remains OPEN at `high` with NO rule.** Its corpus rows are RECORDED
  and asserted in neither direction, and `19-27`'s five-site attribution
  correction stays as performed.
- **`T-19-112` is NARROWED further and not closed.** `T-19-113` likewise.
- **`T-19-115`, `T-19-116`, `T-19-117` and `T-19-118` are ALL OPEN at this plan's
  end.** This plan measured them, widened the corpus so the axis can fail on
  each, observed it RED and stopped. **Whether `19-29`'s rules close any of them
  is audit 11's judgement rather than either plan's claim.**
- **`T-19-96`, `T-19-110` and `T-19-74` are registered open**; `T-19-61` …
  `T-19-73`, `T-19-84` and `T-19-85` are open, unaccepted and untouched.
- **Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** No unqualified
  "`T-19-60` is closed" appears anywhere in this record.

**Because `T-19-86`, `T-19-91`, `T-19-111` and `T-19-112` all remain open at
`high`, this plan does not clear `/gsd-secure-phase 19`, and neither does
`19-29`, and neither do the two together.**

### The base, verified rather than assumed

`git diff --numstat 0092009..ca30c90 -- src/ tests/` returns **nothing**, and so
does `git diff --numstat ca30c90..c82f7d8 -- src/ tests/`. This plan's base
`c82f7d8` is therefore byte-identical to the tree audit 10 measured against for
every file that decides a verdict; the only changes between them are planning
documents.

### THE FIFTEEN-CARRIER DISPOSITION TABLE, WITH A CONTROL LETTER FOR EVERY ONE

**The round's central product.** `19-26` enumerated fifteen carriers; `19-27`'s
disposition table gave `C-01` … `C-09` control (a) and `C-11` … `C-15` control
(e), **and had no row for `C-10` at all — no threat id, no severity, no
`deferred-items.md` entry and no letter. That absence is how a carrier which
takes layer 2 and layer 3 in one permitted call survived a round that enumerated
it.**

| Carrier | What it is | Threat | Severity | Control |
|---|---|---|---|---|
| `C-01` | `<env>/<alias>/pr-ledger.ndjson` | T-19-112 | high | **(a)** rule (a), operand position |
| `C-02` | `<env>/<alias>/hooks/pre-push` | T-19-113 | medium | **(a)** |
| `C-03` | `<env>/<alias>/hooks/pre-commit` | T-19-113 | medium | **(a)** |
| `C-04` | `<env>/<alias>/hooks/` | T-19-113 | medium | **(a)** |
| `C-05` | `<env>/<alias>/gitconfig` | **T-19-118** | **high** | **(a) PARTIAL** — the four OPERAND spellings only; the two REDIRECTION spellings are open |
| `C-06` | `<env>/<alias>/askpass` | — | — | **(a)** |
| `C-07` | `<env>/<alias>/gh/` | — | — | **(a)** |
| `C-08` | `<env>/<alias>/settings.json` | — | planner-derived | **(a)** for the file; the BEHAVIOURAL half is **UNMEASURED** |
| `C-09` | `<env>/<alias>/` | — | — | **(a)** |
| **`C-10`** | **the binary every stub and `guard_command` exec** | **T-19-116** | **high** | **(a), EXACT PATH, PARTIAL — NEW IN ROUND 11.** Registered here with a threat id, a severity, a `deferred-items.md` row and a letter for the first time |
| `C-11` | `<project>/.git/config` | T-19-111 | high | **(e)** no rule, unaccepted |
| `C-12` | `<project>/.claude/settings.json` | — | planner-derived | **(e)** |
| `C-13` | `<project>/.git/info/exclude` | — | — | **(e)** |
| `C-14` | `<project>/.planning/meta-manager/runs/<run_id>/` | — | — | **(e)** |
| `C-15` | `~/.config/gsd-meta-manager/config.json` | — | planner-derived | **(e)** — a **DISTINCT route** to SAFE-06, not a spelling of `T-19-112` |

**Fifteen carriers, fifteen letters, nothing left undispositioned.** The letters
describe what a control WOULD be after `19-29`; nothing here is a claim that
`19-29` has landed.

### `T-19-116` — `C-10`, driven END TO END with a control beside every leg

The bare-remote fixture was **REBUILT** rather than cited: a local bare upstream,
a working clone on `refs/heads/gsd-auto/alpha/w`, and `pre-push`/`pre-commit`
written by `hooks::install_in` in `stub_body`'s exact three-line shape and
delivered exactly as the envelope delivers them — through the `GIT_CONFIG_COUNT`
/ `GIT_CONFIG_KEY_0` / `GIT_CONFIG_VALUE_0` triplet naming `core.hooksPath`.

```text
CONTROL  real binary in place    git push --force origin HEAD:refs/heads/main
                                 -> REFUSED by the hook, remote main 38b1ae0 UNMOVED
LEG B    binary replaced by /bin/true, same push
                                 -> COMPLETED, remote main 38b1ae0 -> 153e6c1 MOVED
CONTROL  binary restored, remote rewound, same push
                                 -> REFUSED, remote main 38b1ae0 UNMOVED
```

**LAYER 2, measured SEPARATELY** — which is what makes `C-10` strictly wider
than `C-02` rather than another spelling of it:

```text
<replaced binary> envelope guard alpha   on `git push --force origin main` -> exit 0 (PERMIT)
<real binary>     envelope guard alpha   on the same                       -> exit 2
```

**Layer 2 AND layer 3 in one permitted call.** Strictly wider than `C-02`
(`T-19-113`, `medium`), which removes layer 3 only, and wider than
`env -u GIT_CONFIG_COUNT`, which removes layer 3 only **and is REFUSED** —
re-measured at `hook_bypass_blocked` in this round's own mechanism pins.

**The guard-level rows, every one measured exit 0 with an EMPTY walk against a
fresh `GSD_MM_ENVELOPE_ROOT`:** `cp /bin/true <BINARY>`, `mv /bin/true <BINARY>`,
`install -m 0755 /bin/true <BINARY>`, `ln -f /bin/true <BINARY>` and
`printf 'x' > <BINARY>`. The `mv` row carries the NO-PROGRAM-NAMES fence one
level over: `mv` appears nowhere in either production half.

**The DISCOVERY rows, measured exit 0 and pinned PERMITTED:** `command -v
gsd-meta-manager` and `cat /proc/self/cmdline`. A carrier whose location is
trivially discoverable is part of the finding, and refusing those two would
refuse `command -v` and `cat` without helping — `stub_body` (`hooks.rs:196-204`)
writes the absolute path into a mode-0755 file under the envelope root and
`guard_command` (`hooks.rs:1518-1524`) writes it into `settings.json`.

**`T-19-01`'s declared mitigation is recorded as CORRECT FOR WHAT IT COVERS and
UNWEAKENED.** `assert_provenance_in` (`hooks.rs:242-260`) certifies a RELOCATED
COPY of a hook: it compares the invoked path against this alias's sanctioned
hooks directory and checks that the binary the stub names still exists on disk.
**A replaced exec TARGET never re-enters the binary that would assert
provenance** — `/bin/true` asserts nothing — so the check is not weakened, it is
not reached.

**`current_exe()` versus the baked path — MEASURED, not assumed.**
`std::env::current_exe()` reads `/proc/self/exe` on Linux, which the kernel
answers with the FULLY RESOLVED target: driving a symlink to `python3` and
reading `/proc/self/exe` from inside it reported `/usr/bin/python3.12`.
`hooks::install` (`hooks.rs:92`) resolves the value it BAKES with the same call,
**so the baked path and the guard's path agree by construction and there is no
drift between them.** The residue is different and is disclosed rather than
closed: **a write naming a SYMLINK to the binary is a different string from
either**, which is direction (iii) arriving over the binary. `19-29` writes no
rule for it — following a link means `readlink`/`canonicalize` on the guard's
critical path, which `hooks.rs:771-816` forbids.

**THE BOUNDARY KIND IS EXACT-PATH AND NOT A PREFIX, AND THE REASON IS MEASURED.**
The envelope DIRECTORY is a PREFIX boundary because this envelope owns every byte
under it — `rm -rf <ENV>/alpha` takes NINE carriers in one call. **The binary's
directory is shared with everything else the user installed.** A prefix over it
would refuse `ls ~/.cargo/bin` and every `cargo install`. The two near-miss
controls — `cp /bin/true <BINARY-PARENT>/some-other-file` and `ls
<BINARY-PARENT>` — are measured exit 0 and asserted exit 0 AFTER, and a clause
written as a directory prefix turns both red. **They are round 11's `--signed
no`, from the other side.**

### `T-19-118` — `C-05`'s credential half, with its control driven FIRST

**`T-19-118` IS `T-19-23` REOPENED**, and it falsifies BOTH halves of
`SECTION_ENVELOPE`'s FIRST `Guaranteed` clause.

```text
CONTROL  the generated file exactly as `write_gitconfig_in` leaves it
         git config --get credential.helper   -> exit 1, no helper named
         git credential fill  https://github.com -> exit 128, ambient secret ABSENT
HARM     printf '[credential]\n\thelper = store\n' >> <ENV>/alpha/gitconfig
         (exit 0 through the guard, EMPTY walk — fail-open direction (i))
         git config --get credential.helper   -> `store`, exit 0
         git credential fill  https://github.com -> exit 0, ambient username and
                                                  password PRESENT
```

The secret is recorded as **PRESENT/ABSENT and never transcribed** — SAFE-04's own
reasoning applies to this corpus exactly as it applies to the ledger.

**THE SIX WRITE SPELLINGS, so the record states exactly which four rule (a)
already reaches:**

```text
exit=2  cp /tmp/evil.cfg <ENV>/alpha/gitconfig                      OPERAND, reached
exit=2  sed -i 's/x/y/' <ENV>/alpha/gitconfig                       OPERAND, reached
exit=2  tee -a <ENV>/alpha/gitconfig                                OPERAND, reached
exit=2  shred -u <ENV>/alpha/gitconfig                              OPERAND, reached
exit=0  printf '[credential]\n\thelper = store\n' >> <ENV>/alpha/gitconfig   REDIRECTION
exit=0  cat /tmp/evil >> <ENV>/alpha/gitconfig                               REDIRECTION
```

A `python3 -c "open('<ENV>/alpha/gitconfig','a').write(…)"` payload naming the
absolute path was measured at exit 2 as well — the path is an operand of
`python3` regardless of what the payload does with it.

**The discriminating control** is the same append to a path OUTSIDE the envelope
root, measured exit 0 and asserted exit 0 AFTER. The pair differs in exactly one
property — whether the redirection target resolves under the envelope root — and
in nothing else: same program, same operator, same bytes.

**`AR-19-04`, RECORDED VERBATIM AND EXPLICITLY NOT UN-ACCEPTED.** It accepts
`T-19-29` — *"agent rewrites the generated git config"*, `medium` — on the
rationale *"the envelope regenerates it at each run start"*. **That rationale
does not cover a write DURING the run**, which three records now note. **The
acceptance is about TAMPERING at `medium` and says nothing about credential
REACHABILITY at `high`.** `T-19-118` is registered as its OWN row rather than
moved into `T-19-23` or into the acceptance. **No `AR-` row is added, edited,
renumbered or un-accepted by this plan; un-accepting a risk is a human
decision.**

### The EMPTY-`credential.helper` candidate control — MEASURED, RECORDED, asserted nowhere

The envelope already injects `core.hooksPath` through `GIT_CONFIG_COUNT`
(`cred::hooks_path_env`, `cred.rs:356-358`), and env-injected pairs are applied
AFTER every file. A second pair naming `credential.helper` with an EMPTY value
resets the helper list.

**THE CRITERION IS `git credential fill`, NOT `git config --get-all
credential.helper`, AND THE DISTINCTION IS LOAD-BEARING RATHER THAN A
PREFERENCE.** Both readings, measured side by side under the harm posture:

```text
                                     --get-all credential.helper   credential fill
no injected pair (control)           `store`, `store`   exit 0     exit 0, secret PRESENT
EMPTY-helper pair injected           `store`, `store`,            exit 128, secret ABSENT
                                     then an EMPTY line, exit 0
```

**Git's empty value resets the helper list that RUNS, not the list the config
query ENUMERATES.** A metric built on `--get-all` would have reported a working
control as broken, and this plan's check caught exactly that. The criterion is
the one that names the harm: `T-19-118`'s measured consequence is `git credential
fill` returning the ambient username and password, so the control's criterion is
the same call answering with nothing. **THE CONTROL REPRODUCES.**

**The COST, measured the same way and carried forward:**

1. **`GIT_ASKPASS` is UNTOUCHED.** With the empty pair injected, a responder still
   answers and `git credential fill` returns its value — so the envelope's ONLY
   token channel (D-17) is not broken by the control.
2. **A later `-c credential.helper=store` on the same command line OVERRIDES the
   reset and the secret comes back.** This is recorded as a **BOUNDED RESIDUE,
   and the reason it is bounded is that it is ARGV-VISIBLE and already
   GOVERNED**: `-c` is a word on the command line, so round 8's confinement
   clause and layer 2's whole grammar see it — unlike EVERY write spelling.
3. **`gh` still operates.** The empty pair is a GIT config key and reaches `gh`
   only through `gh`'s own credential helper, which `GH_CONFIG_DIR` already
   closes from the other side (`cred.rs:373-375`).

**This is the one control that defends `C-05`'s credential half against EVERY
spelling rather than one, because it reads no command line at all** — the four
operand spellings rule (a) already reaches, the two redirection spellings
`19-29`'s widened sight reaches, and the tilde, glob and brace spellings NOTHING
reaches. **That reason does not depend on this round's rule landing, which is why
`19-29` REQUIRES it rather than being offered it as an option.** It is asserted
nowhere in `19-28`: an assertion here would pin a control that does not exist yet
and would land permanently red in a file `19-29` may not edit.

### THE ANSWER TO THE DIRECTION-(i) QUESTION, STATED IN FULL

**Does closing direction (i) mean the DELETION MODEL must read redirection
TARGETS? NO.** `19-26`'s plan-check and audit 10 both called direction (i) *"real
by MECHANISM, not by choice"*, and about the TOKENIZER that is exactly right. It
is also now demonstrably harmful: a fail-open direction carrying a live
`high`-severity credential exploit is not an acceptable residue.

**Round 6's deletion model answers *which words ARRIVE at the program*. A
redirection target does not arrive, and that answer does not change.** Rule (a)
asks a different question about the same walk: *does this line NAME a path this
run's controls live in?* Those are two questions about one token stream.

Three mechanical facts, read out of the source rather than argued:

1. **`skip_redirection_target` (`policy.rs:3007-3057`) already walks the target's
   full extent, applying the target's own single-quote, double-quote and
   backslash rules, in order to skip it.** Its doc's sentence *"Its text is never
   needed, only its extent"* **is the sentence that becomes false** — a WR-02
   shape: a stated reason that went stale and must be CORRECTED rather than
   quietly outgrown. The text and the literalness are by-products of a walk that
   already happens.
2. **`redirection_operator_len` (`policy.rs:2968-2992`) is a FINITE, CLOSED
   grammar of twelve operators and already distinguishes them by name.** Only
   some take a PATHNAME target: `>`, `>>`, `>|`, `<`, `<>`, `&>` and `&>>`.
   **`<<` and `<<-` take a heredoc DELIMITER, `<<<` a here-STRING, and `>&`/`<&`
   an fd number** — none of them a path, and recording any of them as a carrier
   candidate would be wrong. The split is derived at the operator, in the same
   match that already computes its length. Measured and pinned: `git <<EOF push
   --force origin main` deletes the delimiter exactly like a target, the
   surviving argv is `[git, push, --force, origin, main]`, and the row is
   refused as a force push.
3. **`split_segments_with_heads` (`policy.rs:2444-2526`) EXCLUDES every operator
   token from `segment.tokens` and flushes `current` into a `Segment` the instant
   one arrives.** So a redirection-target token pushed into the stream would
   **split `git >/dev/null push --force origin main` into `[git]` and `[push,
   --force, origin, main]`**, and the first resolves `Governed` with an EMPTY
   argv, which `classify_git` answers `Allow` for. **A rule written the obvious
   way silently converts round 6's headline refusal into a permit.**

**The precedent the fix must follow is already in the file**:
`Segment::redirection_unresolvable` (`:2402-2418`, accumulated at `:2461-2505`)
is a fact carried on the SEGMENT, gathered in the operator arm of the ONE walk,
applied retroactively over `segments[command_start..]` and reset at each real
command operator — **with `segment.tokens` byte-for-byte unchanged**.

**THE COST, STATED PLAINLY:** one new field on `Token` and one on `Segment`, both
additive and both compiler-enforced across the six `Token { … }` construction
sites and two `Segment { … }` sites in `policy.rs`; a return-type change on
`tokenize`, private with four call sites; a corrected doc sentence on
`skip_redirection_target`; and a pathname/non-pathname split of a grammar the
file already enumerates. **`SEPARATORS` does not move, `is_separator(">")` stays
`false`, no token enters the stream, and round 6's over-deletion control stays
PERMITTED.** Round 3's principle is discharged rather than weakened: the targets
are found BY the walk that already finds them, and rule (a) is still raised at
ONE site reading ONE segment. **If the fix needs a second scan or a second
reading site, that is a finding to report, not a place to add one.**

#### THE SEGMENT-COUNT MEASUREMENTS THAT MAKE THE CLAIM FALSIFIABLE

Asserted over `policy::split_segments_with_heads` directly, in both directions,
and **GREEN today**:

```text
git >/dev/null push --force origin main   -> 1 segment, tokens EXACTLY
                                             [git, push, --force, origin, main]
git x2>/tmp/o push --force origin main    -> 1 segment, tokens EXACTLY
                                             [git, x2, push, --force, origin, main]
git 2>/dev/null push --force origin main  -> 1 segment, [git, push, --force, origin, main]
git <<EOF push --force origin main        -> 1 segment, [git, push, --force, origin, main]
git >                                     -> 1 segment, [git], redirection_unresolvable = true
policy::is_separator(">") == false        policy::is_separator("<") == false
```

**THE FAILURE MESSAGE NAMES BOTH WAYS A TARGET CAN REACH THE STREAM**, because
the message is what the next executor reads:

- **(a) THE SPLIT VARIANT** — the target entered as an OPERATOR token, so the
  segment was flushed at it, the count is TWO, and the leading `git` resolves
  `Governed` with an empty argv, which `classify_git` answers `Allow` for.
- **(b) THE DISPLACED VARIANT** — the target entered as an ORDINARY word, so the
  count is still ONE and the EXACT-TOKEN clause fires instead: the argv the
  classifier reads is no longer the argv the program receives and every decision
  index is shifted. **That is `T-19-98`'s registered shape**, which is why the pin
  asserts the exact token list rather than only the count.

**In either case the correct response is to move the target onto the `Segment`
the way `Segment::redirection_unresolvable` already travels, never to relax the
assertion.**

#### THE TWO ORDERING PINS, AT DELIBERATELY DIFFERENT IDENTIFIERS

Both orderings of both composites were measured before either was written.

**PIN A — WITHIN one segment, position does NOT move the identifier**, because
the clause sits at `hooks.rs:1018-1029`, before the resolution match. GREEN
today in operand position:

```text
git --git-dir <ENV>/alpha push --force origin main  -> envelope_assertion_failed
git push --force origin main <ENV>/alpha            -> envelope_assertion_failed
git --git-dir /tmp/other push --force origin main   -> force_push_blocked   CONTROL
git push --force origin main /tmp/other             -> force_push_blocked   CONTROL
```

The control lands at a DIFFERENT identifier, which is what makes the pair a
measurement of the CLAUSE rather than of the push. **`git --git-dir=<ENV>/alpha
push …` — the `=`-joined spelling — answers `force_push_blocked`, because the
token does not begin with `/` and `lexical_absolute_components` rejects it. That
is measured and recorded rather than smoothed: rule (a)'s literal-ABSOLUTE
condition applies to a governed program's operands exactly as it does to an
ungoverned one's.**

**PIN B — ACROSS segments, order DOES decide it**, because `classify_segments`
returns the FIRST refusal it reaches. GREEN today:

```text
cp <ENV>/alpha/pr-ledger.ndjson /tmp/x && git push --force origin main
                                                -> envelope_assertion_failed
git push --force origin main && cp <ENV>/alpha/pr-ledger.ndjson /tmp/x
                                                -> force_push_blocked
cp /tmp/other /tmp/x && git push --force origin main
                                                -> force_push_blocked   CONTROL
```

**What makes the PAIR discriminating is that its two halves differ from EACH
OTHER**; the outside control fixes the first half's identifier as carrier-caused.
The redirection-target twin of pin A and the binary twin of pin B are the RED
halves and are asserted at their derived post-fix identifiers.

### `T-19-115` — the SEVEN spellings, and the residue's CORRECTED ARITHMETIC

**`T-19-76`'S FAILURE MODE FOR THE TENTH CONSECUTIVE ROUND, AND FOR THE FIRST
TIME IT IS IN THE CELL THE PREVIOUS ROUND JUST FILLED.** Rule (a) decides on
`Token.literal` — round 5's own bit — and inherits round 5's whole class list by
construction. Round 5's literalness table names **EXPANSION, PATHNAME, TILDE and
BRACE** as the four classes that clear the bit. **Round 10's corpus drew ONE of
the four**: all eleven `ENVELOPE_ROOT_OPERAND_CARRIERS` entries are absolute
literals, `CONTROL_CARRIER_EXPANSION_BORNE` is exactly two `$(…)` entries, and
not one entry of any of the seven classes carried a `~`, a `*`, a `?` or a
`{a,b}` in a carrier path. **The axis was structurally incapable of failing on
three of the rule's own silences.**

**THE DOCS' ARITHMETIC IS WRONG.** `policy.rs:5293-5325` and `hooks.rs:1013-1017`
both say rule (a) *"fails OPEN in FOUR named directions"* and every cited
spelling is `$`-shaped. **There are SEVEN, and the three unnamed ones need no
prior read, no symlink and no cwd:**

```text
(i)   a REDIRECTION TARGET      the operator and its target are deleted before execve
(ii)  an EXPANSION-BORNE operand   the value comes from a substitution
(iii) a SYMLINK                 resolved lexically, links are not followed   NARROWED
(iv)  a RELATIVE path           the guard has no cwd                         NARROWED
(v)   a TILDE                   `~/…` is not `Token.literal`                 NEW
(vi)  a GLOB (`*`, `?`, `[`)    not `Token.literal`                          NEW
(vii) a BRACE LIST (`{a,b}`)    not `Token.literal`                          NEW
```

Measured against the guard, each beside its ABSOLUTE-LITERAL TWIN:

```text
exit=0  rm -rf ~/.local/share/gsd-meta-manager/envelope/alpha
exit=2    twin: rm -rf <REAL-ROOT>/alpha
exit=0  cp /bin/true ~/…/envelope/alpha/hooks/pre-push       exit=2  twin
exit=0  shred -u ~/…/envelope/alpha/pr-ledger.ndjson         exit=2  twin
exit=0  rm -rf <ENV>/alph?          (NINE carriers in one call)   exit=2  twin
exit=0  rm -f <ENV>/alpha/*                                       exit=2  twin
exit=0  unlink <ENV>/alpha/pr-ledger.ndjso?                       exit=2  twin
exit=0  rm -f <ENV>/alpha/{pr-ledger.ndjson,x}                    exit=2  twin
exit=0  rm -rf <ENV>/{alpha,beta}                                 exit=2  twin
exit=0  shred -u <ENV>/alpha/{askpass,x}                          exit=2  twin
```

**All three spellings were also drawn in REDIRECTION-TARGET position — the cell
nobody had drawn** — and stay permitted after `19-29` because a non-literal
target is outside the widened sight's four conditions exactly as a non-literal
operand is outside rule (a)'s.

**`19-29` WRITES NO RULE FOR ANY OF THE THREE, AND THE REASON IS MECHANICAL
RATHER THAN A PRIORITY CALL**: a tilde needs the ENVIRONMENT and a glob needs the
FILESYSTEM, and the guard is forbidden both (`hooks.rs:771-816`,
`mod.rs:196-203`). **The fix is the ARITHMETIC**: state the CONDITION, do not
enumerate spellings.

#### The `bash` leg, and the refinement it forced

**A spelling the guard permits but the shell does not resolve is not a bypass,
and this leg is what tells them apart.** Every spelling was run under a real
`bash` against a scratch tree:

```text
rm -rf ~/<REL>/alpha                    -> REACHED
rm -rf <ENV>/alph?                      -> REACHED
rm -f  <ENV>/alpha/*                    -> REACHED
rm -f  <ENV>/alpha/pr-ledger.ndjso?     -> REACHED
rm -f  <ENV>/alpha/{pr-ledger.ndjson,x} -> REACHED
: >    ~/<REL>/alpha/pr-ledger.ndjson   -> REACHED (11 bytes -> 0)
: >    <ENV>/alpha/pr-ledger.ndjso?     -> REACHED, but ONLY because the glob matches
                                           EXACTLY ONE file; with two matches bash answers
                                           `ambiguous redirect` and reaches nothing
: >    <ENV>/alpha/{pr-ledger.ndjson,x} -> **NOT REACHED.** Brace expansion produces TWO
                                           words and a redirection target must be ONE, so
                                           bash answers `ambiguous redirect`
```

**So the BRACE spelling is a real carrier in OPERAND position and NOT in
REDIRECTION-TARGET position.** Both rows stay in the corpus — they are
verdict-preserving either way and the axis must be able to DRAW the spelling —
and the record says which of them is a bypass and which is a permit that costs
nothing. Also measured and recorded because it bounds the direction: **`sh`
(dash) does not perform pathname expansion on a redirection target at all**; the
guard is registered against the agent's `Bash` tool, so bash's semantics are the
relevant ones.

### `T-19-117` — the ledger as WORK, the curve, and the derived bound

`ledger::record_and_check_in` reads the ledger WHOLE (`ledger.rs:225-227`, a bare
`std::fs::read`) and `tally` (`:338-377`) walks every line — **with no size bound
anywhere** — on the guard's registered critical path against `GUARD_TIMEOUT_SECS
= 5` (`hooks.rs:1408`, delivered at `:1504`).

Measured with entries dated OUTSIDE the 24 h window so the CAP itself is
unaffected, ONE persistent root per level, driving the BUILT BINARY as a
subprocess (≈ 45 ms of each figure is process start):

```text
level        bytes         ls   git status  git push --force  gh pr create  gh api
empty        0             47   48          44                49            48
10,000       1,010,000     45   46          47                96            96
200,000      20,200,000    46   45          46                1008          943
2,000,000    202,000,000   43   43          46                **7900**      7803
```

**7.90 s against a 5 s registered deadline.** **THE FORGE / NON-FORGE SPLIT IS
THE SHAPE OF THE FINDING**: the cost falls ONLY on the commands that reach
`record_and_check_in`, which are exactly the forge commands the cap governs.
Without that half the row would say "the guard is slow" rather than "the cap's
own record is the guard's own workload", and the remedy would be aimed at the
wrong function.

**The inflation is delivered by fail-open direction (i)**, and every spelling is
exit 0 through the guard: `yes '…' | head -2000000 >> <ENV>/alpha/pr-ledger.ndjson`,
`cat /tmp/big >> …` and `head -c 200000000 /dev/zero >> …`.

**THE CONTRAST IS THREE FUNCTIONS AWAY IN THE SAME FILE.** `ends_mid_line`
(`ledger.rs:288-303`) refuses a whole-file read for a one-byte question, in its
own words: *"a whole-file read to answer a one-byte question is the sort of thing
that turns a guard into a hang"*. **The read three functions down is unbounded.**

**THE DERIVED BOUND.** `19-29`'s mandated value is **8 MiB**, and it is
**DEADLINE-derived and must NOT be cap-derived**: `C-15` is that
`pr_cap_per_24h` and `pr_cap_per_run` are UNCLAMPED and read from a file the
driven agent can write (`envelope guard` ignores the global `--config` flag and
always reads `Config::default_path()`, `hooks.rs:837-840`), so a bound computed
from the caps is a bound the agent steers. From the measured curve `tally` costs
≈ 41 ms per megabyte, so 8 MiB is ≈ 340 ms — under 7 % of the deadline. For
scale rather than for derivation: at the default `pr_cap_per_24h = 3` and ~101
bytes per entry, 8 MiB is about seventy-five years of maximal legitimate
appending.

**IT MUST FAIL CLOSED, AND A TAIL READ IS THE WRONG SHAPE.** `tally`'s own doc
says *"a line this function cannot parse is COUNTED, not skipped … under-counting
is the one failure this file exists to prevent"*. A tail read, a line cap and
sampling all UNDER-COUNT by construction, which violates that invariant in the
direction that PERMITS. Refusing preserves it. **The identifier is
`envelope_assertion_failed` and NOT `PrCapExceeded`**, which names a mechanism
this refusal does not use — the ledger was never tallied (D-24).

**The corpus row carries its discriminating control**: a ledger of 24 MiB
refuses while one of 1 MiB **still permits AND still counts** (the walk finds the
appended line). Without the second half the row proves only that a large number
refuses, which a bound of zero would also have. The two sizes are an order of
magnitude either side of 8 MiB, so any value `19-29` chooses inside
`(1 MiB, 24 MiB)` keeps both halves correct — **the pair asserts the
DISCRIMINATION, which is the part a wrong constant cannot fake.**

**THE BEHAVIOURAL HALF IS UNMEASURED AND IS CLAIMED IN NEITHER DIRECTION.** What
the agent CLI does with a `PreToolUse` hook past its registered timeout is a
property of a closed-source binary. It is held to the same discipline as
`C-08`'s behavioural half.

**`glab` is confirmed NOT INSTALLED on this machine**, so its cell is recorded as
UNMEASURABLE AGAINST ITS CALLEE rather than driven or inferred, and **no pin that
would SKIP is written — a pin that skips is a fail-open pin.**
**`FORGE_VALUE_OPTS` keeps `--hostname`**, unchanged and unproposed for removal.

### The axis — from SEVEN classes to ELEVEN, with the arithmetic re-derived

**THREE class predicates were refined, not two**, and the third is the one whose
absence would have done more than fire a fence:

1. **class 1** `draws_an_envelope_root_operand` — excludes a pathname-expansion
   metacharacter or a brace list, handing the spelling to classes 9 and 10.
2. **class 2** `draws_a_redirection_target_carrier`
   (`tests/envelope_wrapper_class.rs:7926` before this plan) was
   `is_redirection_target && starts_with(ROOT)` **with NO literalness guard at
   all**, so this round's own `: > <ENV>/alpha/pr-ledger.ndjso?` and
   `: > <ENV>/alpha/{pr-ledger.ndjson,x}` entries satisfied it — **and class 2's
   alphabet is simultaneously the one MOVING to the fail-closed arm**, so the
   collision would have placed PERMITTED glob and brace targets inside a property
   that asserts REFUSAL.
3. **class 5** `draws_a_relative_carrier` — excludes a tilde, handing it to class
   8. A `~` spelling does not begin with `/`, so it satisfied class 5 before the
   refinement, but it is not a relative path — it is an absolute path the shell
   has not expanded yet, and the two fail open for different reasons.

**`CONTROL_CARRIER_REDIRECTION_TARGETS` MOVED out of the invariance arm** into a
fail-closed one, with the `19-18` `{v}>` blocker, `19-20`'s
`GIT_GLOBAL_UNKNOWN_OPTIONS` split, `19-22`'s indirection/confined split,
`19-24`'s re-parsed/confined split and `19-26`'s own envelope-root split named
as the precedent — **a SIXTH time that split has been forced by measurement.**
`CONTROL_CARRIER_REDIRECTION_TARGETS_PRESERVING` carries the four targets that
stay permitted.

**Four new classes**, each drawing in BOTH word positions: tilde-borne (8),
glob-borne (9), brace-list-borne (10) — all verdict-PRESERVING with no rule
written — and **the guard's own binary (11)**, fail-closed and compared as an
**EXACT PATH**, because `C-10` is outside the envelope root so class 1 does not
draw it and class 7, the complement, WOULD.

**THE ARITHMETIC, RE-DERIVED FROM THE THREE REFINEMENTS AS WELL AS THE FOUR NEW
CLASSES AND STATED AS EXACT EQUALITIES.** Audit 5 found `19-16` set a floor of 50
against a maximum of 40 by construction, so it is checked against the alphabets
rather than against the prose:

```text
envelope-root operands              11    (fail-closed)
redirection targets                  4    (fail-closed — MOVED)
binary                               5    (fail-closed — NEW)
expansion-borne                      2
symlinked                            2
relative                             2
repo-side                            6    (RECORDED, never asserted)
ordinary operands                   13    (11 + the 2 binary near-miss controls)
redirection targets, preserving      4    (NEW)
tilde-borne                          4    (NEW)
glob-borne                           4    (NEW)
brace-list-borne                     4    (NEW)
                            CASES = 61    SLOTS = 12    CLASSES = 11
```

Per-class counts over all 61, with every deliberate overlap RECORDED rather than
smoothed: class 1 = 11, class 2 = 4, class 3 = 3 (the two expansion entries plus
the expansion-borne redirection TARGET), class 4 = 3 (the `ln -s` mitigation,
which is the record that direction (iii) is NARROWED), class 5 = 2, class 6 = 6,
class 7 = 13, classes 8/9/10 = 5 each (four entries plus one target apiece),
class 11 = 5 — **and NOT the two near-miss controls, which count in class 7.**

**Three fences.** NO-PROGRAM-NAMES extended so the binary class and each new
spelling class draws at least one row under a program absent from BOTH production
halves (`mv`, `shred`, `unlink`). PATH-PREFIX-NOT-BASENAME gains the binary's two
controls by name and additionally asserts they do NOT draw class 11.
**NO-EXPANSION-SPELLING is new**: it counts ENTRIES, not prose, to prove the axis
can now DRAW a `~`, a `*`/`?` and a `{a,b}` inside a carrier path — because the
absence of exactly that is `T-19-115`, and a floor that cannot fail on a class is
how this failure mode reached its tenth consecutive round. Its message says the
correct response is to ADD the spelling, never to relax the fence.

**Nothing else moved.** The four earlier axes, all their predicates, all their
degenerate-proofing and all their floors are byte-identical;
`MIN_CONFIG_RESOLUTION_CLASSES` is still **6**; every one of the 44 pre-existing
section-17 alphabet literals is still present and every `MIN_*` either kept its
value or ROSE.

### A BLOCKING FINDING FOR `19-29`, FOUND BY READING AND VERIFIED MECHANICALLY

**`19-29`'s redirection-target rule turns
`direction_i_a_redirection_target_is_not_an_operand_and_stays_permitted`
(`tests/envelope_control_carrier.rs:633-663`) RED, and `19-29`'s
`files_modified` does not list that file.**

That test pins exactly three rows PERMITTED:

```text
: > <ENV>/alpha/pr-ledger.ndjson
printf 'exit 0' > <ENV>/alpha/hooks/pre-push
echo evil > <ENV>/alpha/askpass
```

which are three of the four entries of `CONTROL_CARRIER_REDIRECTION_TARGETS` —
the alphabet this plan moved into the fail-closed arm precisely because their
verdict the fix CHANGES. **This is the same shape as putting an entry whose
verdict the fix changes into an invariance-preserving arm — `19-18`'s `{v}>`
blocker — except that it sits in a file neither `19-28` nor `19-29` is currently
permitted to edit, which is the shape that halted `19-23` mid-plan.**

`19-28` does not fix it: editing `tests/envelope_control_carrier.rs` is
prohibited by this plan, and round 10's evidence stays attributable to the round
that produced it. **`19-29`'s first action should be to add
`tests/envelope_control_carrier.rs` to its `files_modified` and to move those
three rows into a `after_19_29_…` property with their reason stated**, or to
report the scope conflict rather than silently leaving the file red. The
remaining fail-open directions pinned in the same file — (ii) expansion, (iii)
symlink to `/tmp/l`, (iv) relative — are verdict-PRESERVING and are unaffected;
so is `: > /tmp/l`, whose target is outside the root.

Also checked and clear: no other test file names an envelope-root redirection
target as a pinned-permitted row, and no test outside this round's two names the
guard's own binary as an operand.

### The `421` correction, and the site corrected BESIDE rather than in place

The floor comment at **`src/envelope/policy.rs:7450`** says the file carries
*"842 multi-byte characters"*. **842 is the byte-minus-character DIFFERENCE; the
COUNT is 421**, each three-byte character contributing two extra bytes.
**`policy.rs:7450` is `19-29`'s to edit and this plan does not touch it.**

The same error appears at **`deferred-items.md:2172`**, inside the plan-19-27
section. **That section is not edited.** The sentence there reads:

> *"Audit 9's 261,387 and the mandate's 261,386 are **CHARACTER** counts —
> `production.len()` is `String::len()`, which is bytes, and the file carries 842
> multi-byte characters."*

**Corrected BESIDE, here: the file carries `421` multi-byte characters, and 842
is the byte-minus-character difference.** Every other number in that comment was
re-derived by audit 10 and is right — `policy.rs` production half **277,570
bytes / 276,660 characters / 5,414 lines**, `hooks.rs` **78,465 bytes / 1,636
lines**, exactly ONE `#[cfg(test)]` sentinel per file, deep anchor `fn
forbidden_repo_path` at line 5,157, `fn scan_leading` at 416, and the
proportional floor passing at **64.85 %**. **The floor's value and strictness do
not move.**

### `mod.rs:165-168` is put explicitly IN SCOPE for `19-29`

`ENVELOPE_ROOT_ENV`'s residual-limit paragraph says: *"a party who already
controls the environment the TUI starts in can relocate the envelope. That party
can equally unset `GIT_CONFIG_COUNT`, which is D-09's stated ceiling, **so this
override does not lower a boundary that was standing**."*

**`T-19-116` contradicts that argument.** Replacing the binary lowers layers 2
AND 3 **without controlling the environment the TUI starts in at all** — it is an
ordinary file write by the same uid, through a command layer 2 permits, and
`env -u GIT_CONFIG_COUNT` is REFUSED where it is not. `19-27` read that paragraph
and correctly declined to touch it because it was outside its `files_modified`.
**This round names it, and `19-29` lists `src/envelope/mod.rs` in its
`files_modified`.**

### Carried forward unchanged

- **`C-08`'s mechanical half is settled and unchanged** (`grep -rn 'settings_json'
  src/` is exactly two hits while production pushes `--settings <path>`);
  **repairing a delivery is a change to the SPAWN SEAM, and the BEHAVIOURAL half
  stays UNMEASURED and is claimed in NEITHER direction.**
- **`C-15` stays a DISTINCT route to SAFE-06** rather than a spelling of
  `T-19-112`, steered by `HOME`/`XDG_CONFIG_HOME` and not by argv. **No
  `pr_cap_*` clamp is written or proposed** — clamping a configured cap is a
  PRODUCT decision about what a user may configure, not a guard rule, and making
  it inside a security round would decide it silently.
- **`T-19-110` stays open at `medium`; `T-19-96` and `T-19-74` unchanged**, the
  latter's core rows frozen.
- **`T-19-86` is not fixed, narrowed, re-scoped or re-classified**, `T-19-111` is
  not folded into it, `T-19-91` gains no remedy, and `T-19-111` gains no rule.
- **`T-19-SC` holds**: no crate was added; `Cargo.toml` and `Cargo.lock` are
  untouched. Every probe shells out through `std::process::Command` inside
  `#[cfg(test)]`.

### The mechanism pins and the byte floors, RE-MEASURED at this plan's base

All re-asserted in the new evidence file over the same public functions rather
than moved, and all GREEN:

```text
round 5  git pus? --force origin main                       -> 2 envelope_assertion_failed
round 6  git >/dev/null push --force origin main            -> 2 force_push_blocked
         git x2>/tmp/o push --force origin main             -> 0   (over-deletion control)
         git 2>/dev/null push --force origin main           -> 2
         git >                                              -> 2 envelope_assertion_failed
         SEPARATORS byte-identical at policy.rs:2297, ONE commit in the phase (84a9b05)
         is_separator(">") == false   is_separator("<") == false   (by construction)
         is_separator("&&"/";"/"|") == true                 (the positive control)
round 7  git --attr-source HEAD push --force origin main    -> 2 force_push_blocked
         git --bogus-opt status                             -> 2 envelope_assertion_failed
         git - push --force origin main                     -> 0   (over-refusal control)
         git -- push --force origin main                    -> 2
round 8  git -c include.path=/tmp/evil.cfg status           -> 2
         git -c includepath=… / notinclude.path=… / a=b     -> 0
round 9  git -c alias.q="-c include.path=… status" q        -> 2
         git -c aliasx.q=… / notalias.q=…                   -> 0
         git -c alias.p='!git push --force origin main' p   -> 0   (T-19-86, may not move)
round 10 all nineteen envelope-root operand rows            -> 2
         rm -f /tmp/pr-ledger.ndjson                        -> 0   (the --signed no)
         git config --get core.hooksPath                    -> 0   (the permitted read)
         env -u GIT_CONFIG_COUNT git push --force origin main -> 2 hook_bypass_blocked
         env -u GSD_MM_ENVELOPE_ROOT ls                     -> 0   (E-01, measured INERT)
```

`POLICY_MIN_PRODUCTION_BYTES = 40_000` and `HOOKS_MIN_PRODUCTION_BYTES = 20_000`
are unchanged; the anti-vacuity stripper's three protections — the proportional
floor `>= 180_000`, the deep anchor `fn forbidden_repo_path` and the exactly-ONE
`#[cfg(test)]` sentinel count per file — keep their present strictness.

**`Token.literal` is READ in a SECOND place by round 10's rule and in a THIRD by
round 11's**, so a change that cleared or repurposed it would silently widen both
rules as well as turning rounds 5 and 6's pins vacuous. That is stated here
because it is the fact a later editor is most likely to be unaware of.

### `T-19-17r` is OUTSTANDING for the ELEVENTH time

`grep -cE '^\| AR-19-13 \|'` over this file is **0**. Audits 5 through 10 and
plans 19-24 … 19-27 all confirmed the measurement and both pins and all of them
explicitly declined. **This plan is the eleventh to decline, because accepting a
risk is a human decision.** No `AR-19-13` row is created, and the word "accepted"
is not applied to `T-19-17r` anywhere — not in a test name, not in a comment, not
in this record.

### The gate, with its arithmetic STATED and CHECKED

```text
command   rtk proxy cargo test --no-fail-fast   (counts read with `rtk proxy grep`, D-34)
baseline  1771  passed + failed   (19-27-SUMMARY.md, 45 result lines)
after     1808  passed + failed   (1802 passed, 6 failed, 13 ignored, 46 result lines)
delta     +37
check     35 new #[test] fns in tests/envelope_carrier_reach.rs
        +  2 new #[test] fns in tests/envelope_wrapper_class.rs
        = 37.  1771 + 37 = 1808.  **The identity holds exactly.**
```

A red test RAN, so red→green leaves the total unchanged and every increase comes
ONLY from new `#[test]` fns. **SEVENTEEN `envelope_*` binaries RAN** — sixteen
would have meant this plan's own evidence file did not execute.

**THE COMPLETE EXPECTED-RED SET, which is `19-29`'s handoff contract:**

```text
tests/envelope_carrier_reach.rs   (30 passed, 5 failed)
  after_19_29_a_command_whose_operand_is_the_guards_own_binary_is_refused
  after_19_29_a_redirection_that_writes_the_generated_gitconfig_is_refused
  after_19_29_ordering_pin_a_holds_for_a_redirection_target_too
  after_19_29_ordering_pin_b_holds_for_the_binary_carrier_too
  after_19_29_a_ledger_past_the_size_bound_refuses_while_one_just_under_it_still_counts

tests/envelope_wrapper_class.rs   (51 passed, 1 failed)
  a_command_that_names_a_carrier_rule_a_cannot_see_is_refused_after_19_29
```

**No other test may be red.** Per-binary counts for all seventeen:
`envelope_advisory` 10/0, `envelope_argv_deletion` 20/0, `envelope_callee_grammar`
19/0, `envelope_carrier_reach` 30/5, `envelope_command_position` 18/0,
`envelope_config_resolution` 30/0, `envelope_control_carrier` 37/0,
`envelope_credential` 6/0, `envelope_expansion_slots` 32/0,
`envelope_hook_refusals` 7/0, `envelope_literal_decision` 43/0, `envelope_pr_cap`
11/0, `envelope_reparsed_value` 34/0, `envelope_tracer` 6/0, `envelope_wiring`
14/0, `envelope_wrapper_bypass` 13/0, `envelope_wrapper_class` 51/1.

`cargo build` and `cargo clippy -- -D warnings` both exit 0. **`cargo clippy
--tests` is NOT the gate** — it already fails at base on four pre-existing lint
errors in `src/browser.rs` and `src/project_creator.rs`, which are not touched.

**NONE of the three documented flakes fired** in this run — the two
`tests/driver_reattach.rs` failures and the `tests/envelope_tracer.rs` ETXTBSY
stub-write race were all green. **Absence is not evidence they are fixed.** The
ETXTBSY race IS `C-10`'s own seam and this plan exercises it: the direct
`cp /bin/true <binary>` in the end-to-end drive **hit `Text file busy` during
measurement** and the fixture retries with a backoff and then falls back to the
atomic write-temp-then-rename idiom, printing which spelling succeeded. **That
firing is RECORDED and was not fixed.**

**Every commit of this plan shows ZERO `src/` hunks**, verified with
`git show --numstat <sha> -- src/`.

---

## Execution record — plan 19-29 (the rules, round 11). NOT an audit finding.

**This subsection is a record made by plan 19-29's executor. It is appended after
the plan-19-28 record and edits nothing that precedes it** — no audit table, no
Security Audit Trail row, no Accepted Risks Log row, no sign-off, and no earlier
appended subsection.

### FIRST, and in these words: `/gsd-secure-phase 19` is NOT cleared, and nothing here is described as a closure

- **`T-19-86` remains OPEN at `high`** by explicit user scoping decision, with
  `T-19-111` still moved OUT of it exactly as `19-27` moved it.
- **`T-19-91` remains OPEN at `high`.**
- **`T-19-111` remains OPEN at `high` with NO rule written for it.**
- **`T-19-112` and `T-19-113` are NARROWED FURTHER and are still NOT CLOSED.**
  `C-15` is a route this rule cannot reach and the deferred ledger option (b) is
  another.
- **`T-19-115` gets NO rule at all.**
- **Whether `T-19-116`, `T-19-117` and `T-19-118` close is audit 11's judgement
  rather than this plan's claim.** This plan says **NARROWED** and states what
  remains open.
- **Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.** No unqualified
  "T-19-60 is closed" appears anywhere in this plan's output.
- `T-19-96`, `T-19-110` and `T-19-74` are registered open and unfixed;
  `T-19-61` … `T-19-73`, `T-19-84` and `T-19-85` are open, unaccepted and
  untouched.

### `19-28`'s RED rows: confirmed still RED before any production line moved, and their after-verdicts

The complete RED set was re-run against the unmodified tree at `d5542c6` with
`--no-fail-fast` **before any production line moved**, and matched
`19-28-SUMMARY.md`'s recorded list exactly: 1802 passed / 6 failed / 13 ignored,
`passed + failed` = **1808**, 46 result lines, **17** `envelope_*` binaries, and
none of the three documented flakes fired.

| row | before | after | landed in |
|---|---|---|---|
| `after_19_29_a_redirection_that_writes_the_generated_gitconfig_is_refused` | RED | **GREEN** at `envelope_assertion_failed` | Task 1 |
| `after_19_29_ordering_pin_a_holds_for_a_redirection_target_too` | RED | **GREEN** at `envelope_assertion_failed` | Task 1 |
| `after_19_29_a_command_whose_operand_is_the_guards_own_binary_is_refused` | RED | **GREEN** at `envelope_assertion_failed` | Task 2 |
| `after_19_29_ordering_pin_b_holds_for_the_binary_carrier_too` | RED | **GREEN** at `envelope_assertion_failed` | Task 2 |
| `a_command_that_names_a_carrier_rule_a_cannot_see_is_refused_after_19_29` | RED | **GREEN** | Task 2 |
| `after_19_29_a_ledger_past_the_size_bound_refuses_while_one_just_under_it_still_counts` | RED | **GREEN** at `envelope_assertion_failed` | Task 3 |

**No row was already green**, so no rule here is a rule nobody showed was needed.

### `19-28`'s PERMITTED rows: re-checked, and THREE of them were not

The PERMITTED set was driven against the unmodified tree and re-checked after the
fix. **The seven spelling rows in both word positions, the expansion-borne,
symlinked and relative rows, the near-misses and both EXACT-PATH-not-PREFIX
binary controls are all still PERMITTED.** `envelope_control_carrier` is 37/0 and
`envelope_wrapper_class` is 52/0.

**THREE ROWS DID FLIP, and every one of them is a row whose own stated derivation
says the fix flips it.** Each was verified by measurement before it was touched,
each correction is recorded here, and none was made uniform to make a fix pass.

1. **`direction_i_a_redirection_target_is_not_an_operand_and_stays_permitted`**
   (`tests/envelope_control_carrier.rs`) — the blocking scope finding `19-28`
   raised, corrected under **explicit human authorisation** obtained before the
   work began. See the next subsection.
2. **`the_ledger_inflation_spellings_are_all_permitted_today`**
   (`tests/envelope_carrier_reach.rs`) — **NOT named by `19-28`'s blocking
   finding**, which checked for exactly this shape and scoped the check one file
   too narrowly: it reported *"no OTHER test file names an envelope-root
   redirection target as a pinned-permitted row"*, which was true of every file
   except the one it was written in. The row's own comment already said *"they
   are the ones `19-29`'s widened SIGHT turns to exit 2"* while its assertion was
   written at the pre-fix verdict — `19-28`'s own deviations 2 and 3, missed
   once.
3. **`t_19_117_the_ledgers_size_is_unbounded_work_on_the_guards_critical_path`**
   (`tests/envelope_carrier_reach.rs`) — **this row and the fix are MUTUALLY
   EXCLUSIVE BY CONSTRUCTION.** It asserts in the present tense that a
   202,000,000-byte ledger pushes a forge command past the deadline, while
   `after_19_29_a_ledger_past_the_size_bound_…` in the SAME file requires the
   bound to sit inside `(1 MiB, 24 MiB)`. No constant satisfies both. `19-28`'s
   own failure text anticipated it: *"If this does not reproduce, RECORD the
   measured curve and report it rather than asserting audit 10's number."*

A fourth row, `envelope_credential.rs`'s
`a_credential_helper_the_user_really_has_stops_being_resolvable_under_the_envelope`,
moved for a different reason and is recorded under `T-19-118` below. That file is
not in this plan's fenced list.

### THE AUTHORISED EXCEPTION — what it measured, and the two reasons it corrected

Measured against the BUILT BINARY with a fresh `GSD_MM_ENVELOPE_ROOT` per row and
the envelope directory walked after each, **before any assertion was edited**:

```text
: > <ENV>/alpha/pr-ledger.ndjson                exit 2  envelope_assertion_failed
printf 'exit 0' > <ENV>/alpha/hooks/pre-push    exit 2  envelope_assertion_failed
echo evil > <ENV>/alpha/askpass                 exit 2  envelope_assertion_failed
: > /tmp/l                          (CONTROL)   exit 0  permit
D=$(git config --get core.hooksPath); rm -f $D/../pr-ledger.ndjson   exit 0  permit  (ii)
rm -f /tmp/l                                    exit 0  permit  (iii)
rm -f pr-ledger.ndjson                          exit 0  permit  (iv)
```

**All three rows flip, each flip is the intended one at the intended identifier,
and NO FOURTH ROW in that file flips**: `envelope_control_carrier` measured
36 passed / 1 failed with the rule applied — one test, three rows — and
directions (ii), (iii) and (iv) are untouched.

**Two stated reasons were corrected, and neither was deleted.**

- **`"These rows will not flip"` is FALSE BY MEASUREMENT.** `T-19-118` is a live
  `high`-severity credential reach through direction (i), and it re-opens the
  closed `T-19-23`. The inference was from a mechanism that is real to a
  conclusion the mechanism does not support: *the rule cannot see it as a WORD*
  does not entail *the guard cannot establish it at all*.
- **`"A rule that read redirection targets would re-open a model five rounds have
  pinned shut"` is FALSE IN A MORE INTERESTING WAY**, and the interesting part is
  that it was falsifiable and was falsified. It assumed the only way to read a
  target is to make `>` a separator or to push a token into the stream. The
  `Segment`-borne channel does neither, and the pins prove it:

```text
SEPARATORS                   byte-identical at policy.rs:2297 (one commit ever, 84a9b05)
policy::is_separator(">")    false
segment.tokens               byte-identical (SEGMENT-COUNT pins GREEN)
git x2>/tmp/o push --force … exit 0  (round 6's over-deletion control)
git >/dev/null push --force … exit 2, ONE segment
```

**Both were sound when written and were overtaken by evidence, and both say so in
place.** This phase has three disclosures that described a control's coverage in
terms which quietly excluded the live case; this round did not add a fourth.

### THE DESIGN ANSWER — BOTH, plus a third thing that is neither

| cell | widen the PATH SET | widen what the rule SEES | neither |
|---|---|---|---|
| `T-19-116` (`C-10`) | **YES** — the guard's own binary, as an **EXACT PATH** | inherits every existing silence over a second path | — |
| `T-19-118` (`C-05` via (i)) | — | **YES** — pathname REDIRECTION TARGETS, on a channel `Segment::tokens` never holds | the `Guaranteed` clause repair **and the REQUIRED injected empty-`credential.helper` control** |
| `T-19-115` | — | **NO** — a rule would need expansions the guard is forbidden to perform | the residue's **ARITHMETIC**: four becomes seven, stated as a CONDITION |
| `T-19-117` | — | — | a size bound in `ledger.rs` that fails **CLOSED**, derived from the DEADLINE |

**The two boundary KINDS differ for a measured reason.** The envelope directory
is a PREFIX because this envelope owns every byte under it and
`rm -rf <root>/<alias>` takes nine carriers in one call. **The binary is an EXACT
PATH because its directory is shared with everything else the user installed** —
a prefix over that parent would refuse `ls <parent>` and every `cargo install`.
`19-28` pinned `cp /bin/true <BINARY-PARENT>/some-other-file` and
`ls <BINARY-PARENT>` PERMITTED as this round's `--signed no` from the other side,
and both are still exit 0. **A clause written as a directory prefix turns both
red, and that is the point of them.**

### THE DIRECTION-(i) QUESTION, ANSWERED IN FULL

**Closing direction (i) does NOT require the deletion model to read redirection
targets as argv.** Round 6's model answers *which words ARRIVE at the program*; a
redirection target does not arrive, and that answer did not change. Rule (a) asks
a different question about the same walk: *does this line NAME a path this run's
controls live in?*

Three facts, read out of the source:

1. **`skip_redirection_target` already walked the target's full extent**, applying
   the target's own quoting rules, in order to skip it. Its text and its
   literalness are BY-PRODUCTS of a walk that already happens. Its doc's *"Its
   text is never needed, only its extent"* is the sentence this round made false,
   and it is **corrected in place with the reason it was right when written** —
   the WR-02 discipline this codebase already names.
2. **`redirection_operator_len` was a CLOSED twelve-operator grammar**, and only
   SEVEN take a PATHNAME: `<`, `>`, `>>`, `>|`, `<>`, `&>`, `&>>`. The other five
   take something that is not a file — `<<` and `<<-` a heredoc DELIMITER, `<<<`
   a here-STRING, `>&` and `<&` an fd NUMBER. **The split is derived AT THE
   OPERATOR, in the match that already computes the length**, and the function is
   renamed `redirection_operator` because a name saying "len" returning a
   pathname-ness would be the same stale-reason defect one function over.
3. **`split_segments_with_heads` excludes every operator token from
   `segment.tokens` and FLUSHES at one.** A redirection-target token pushed into
   the stream would split `git >/dev/null push --force origin main` into `[git]`
   and `[push, --force, origin, main]`, and the leading `git` resolves `Governed`
   with an EMPTY argv, which `classify_git` answers `Allow` for.

**So the target travels on the SEGMENT**, in a shape already in the file:
accumulated in the operator arm of the ONE walk, applied retroactively over
`segments[command_start..]`, reset at each real command operator — exactly as
`Segment::redirection_unresolvable` travels.

**THE SEGMENT-COUNT MEASUREMENTS, BEFORE AND AFTER — which are what make the
claim falsifiable rather than asserted:**

```text
                                            BEFORE                       AFTER
git >/dev/null push --force origin main     1 segment, [git,push,        UNCHANGED
                                             --force,origin,main]
git x2>/tmp/o push --force origin main      1 segment, [git,x2,push,     UNCHANGED
                                             --force,origin,main]
git 2>/dev/null push --force origin main    1 segment, [git,push,        UNCHANGED
                                             --force,origin,main]
git <<EOF push --force origin main          1 segment                    UNCHANGED
git >                                       1 segment, unresolvable      UNCHANGED
is_separator(">") / is_separator("<")       false / false                UNCHANGED
is_separator("&&")                          true                         UNCHANGED
```

**THE COST, stated rather than discovered:** ONE additive `Segment` field
(`redirection_targets`), a private return-type change on `tokenize` with FOUR
call sites (two production, two in-module tests), a corrected stale doc sentence,
and a pathname/non-pathname split of a grammar the file already enumerated.
**The plan projected TWO additive fields; ONE was enough**, because `tokenize`'s
return type carries the attribution index and no `Token` field was needed. Fewer
fields is strictly less surface, and it is recorded rather than glossed.

**NO SECOND READING SITE AND NO SECOND SCAN WAS NEEDED.** The clause is still
raised once, at the top of `classify_segments`' existing per-segment loop, on the
segment that loop already holds, before the resolution match, reading TWO FIELDS
of that one segment.

### THE SEVEN FAIL-OPEN DIRECTIONS, over TWO word classes and TWO paths, handed to NO control

**Stated as a CONDITION rather than as an enumeration, in the predicate's doc, the
call site's comment, `cred.rs`'s WHAT IS NOT COVERED and here.** The rule is
silent about **a word the SHELL MAY REWRITE**, about **a word that IS NOT
ABSOLUTE**, and about **a word that reaches a protected path ONLY THROUGH A
LINK** — and it is silent about all three **in EITHER word class and over BOTH
paths.** Round 5's own literalness table names EXPANSION, PATHNAME, TILDE and
BRACE as the classes that clear `Token.literal`, so the rule inherits that whole
class list by construction rather than by enumeration.

Seven spellings are measured as INSTANCES, not as a complete list:

1. **A REDIRECTION whose target is any of the six below.** **NARROWED by this
   round and explicitly NOT CLOSED.** Direction (i) stops being a direction of its
   own and becomes a **second WORD CLASS the other six apply over.**
2. **An EXPANSION-BORNE word**, in either class and over either path.
3. **A SYMLINK is not followed.** NARROWED over the DIRECTORY by a measured
   partial mitigation; **there is NO such mitigation over the binary.**
4. **A RELATIVE word.** NARROWED over the DIRECTORY; **not over the binary**,
   because `cd <binary-parent>` names a directory the exact-path clause
   deliberately does not cover.
5. **A TILDE.** No rule, and none is possible: resolving one needs the
   ENVIRONMENT, which `19-25`'s standing prohibition forbids at guard time.
6. **A GLOB.** No rule: resolving one needs the FILESYSTEM, which the latency and
   TOCTOU rules forbid.
7. **A BRACE LIST.** No rule, for the same reasons — and in redirection-target
   position bash answers `ambiguous redirect` and reaches no file at all, so that
   one is a permit that costs nothing.

**The path set grew, the word set grew, and the silences did NOT shrink.** Each
now applies over two word classes and two paths.

**NO REVISIT CONDITION, NO PIN, NO SCHEDULE AND NO VERSION WITNESS was created
for any of the seven, and that is a decision with a reason.** `19-25`'s witness
schedules a human re-audit against a FUTURE GIT, which is the right shape for a
residue that arrives when someone else ships something. **All seven are reachable
by the driven agent TODAY**, so a schedule over them would observe the wrong
thing entirely — a version string that never moves while the residue is
exercised — and attaching one would be `T-19-107`'s shape arriving in the round
that inherited the lesson.

### `C-10` — a PARTIAL rule, with what it does not reach stated at the same weight

**What it refuses:** `cp /bin/true <binary>`, `mv`, `install -m 0755`, `ln -f`
and `printf 'x' > <binary>` at the binary's ABSOLUTE LITERAL spelling — the
spelling audit 10 drove end to end, the one that moved a bare remote's `main`
`01f9664` → `2537e97` and made the guard itself answer exit 0.

**What it does NOT reach, at the same weight:** an expansion-borne spelling
(`cp /bin/true $(command -v gsd-meta-manager)`), a tilde spelling, a relative
spelling after a `cd`, and **a link on `PATH` whose target `current_exe()` reports
instead** — a third string neither half of the path set covers. All are pinned
PERMITTED and all are still exit 0.

**`current_exe()`'s agreement, measured by `19-28` and unchanged:** `/proc/self/exe`
is fully resolved and `hooks::install` bakes the same call's answer, **so the
baked path and the guard's path agree by construction.** The residue is the
symlink case above.

**`current_exe()` is resolved ONCE per invocation and threaded explicitly.** An
unresolvable exe path makes the binary half **SILENT**, which is **fail-open**
and is stated on the predicate's signature rather than left to be discovered.
A new pin asserts the silence with a positive control beside it.

**`T-19-01` is recorded correct-for-what-it-covers and UNWEAKENED.**
`assert_provenance_in` certifies a RELOCATED COPY; a replaced exec target never
re-enters the binary that would assert provenance, so the check is not weakened —
it is not reached. **It was never a defence of the exec target's bytes.**

**`C-10`'s control letter is updated to (a), EXACT PATH, PARTIAL**, so the
fifteen-carrier table still has a letter for every one.

### `T-19-115` — carried forward with NO rule, at its measured severity

`medium`, OPEN, **no rule written and no acceptance made.** The reason is
mechanical rather than a preference: a tilde needs the ENVIRONMENT and a glob
needs the FILESYSTEM, and the guard is forbidden both at guard time. **The remedy
taken is the residue's ARITHMETIC** — four becomes seven, stated as a CONDITION
with the spellings as instances — corrected in the predicate's doc, the call
site's comment, `cred.rs` and this record.

### The `SECTION_ENVELOPE` repair

**THE COMPLETE PIN HIT SET, ENUMERATED WITH `grep -rn` BEFORE THE TEXT MOVED**,
over `src/` and `tests/` for `SECTION_ENVELOPE`, `envelope_notice`,
`ambient git credentials`, `credential helper` and `socket is removed`:

| site | what it pins | outcome |
|---|---|---|
| `tests/envelope_advisory.rs:182-221` | all three phrases present, in order | **passed UNMODIFIED** |
| `tests/envelope_advisory.rs:239-289` | the 215-token cap, the 80-column cap | **passed UNMODIFIED, cap NOT raised** |
| `tests/envelope_advisory.rs:302` | `envelope_notice` renders it | **passed UNMODIFIED** |
| `tests/envelope_control_carrier.rs:2069` | the CAP clause verbatim (`19-27`'s repair) | **untouched, passed UNMODIFIED** |
| `tests/envelope_control_carrier.rs:2090-2092` | the phrase-ORDER assertion | **passed UNMODIFIED** |
| `src/driver/dry_run.rs:549, 735, 796` | three consumers of the constant | **passed UNMODIFIED** |
| `src/envelope/advisory.rs:328` | `envelope_notice`'s own composition | **untouched** |

**NO existing assertion pins the credential clause literally**, so the "one
pre-authorised edit" the plan reserved **was not needed and was not taken.** The
repair is covered by a NEW assertion in `tests/envelope_carrier_reach.rs`
instead, which is an ADDITION.

```text
BEFORE  This run cannot reach your ambient git credentials or SSH agent: the
        socket is removed, not emptied, and global and system git config is a
        generated file naming no credential helper.

AFTER   As started, this run cannot reach your ambient git credentials or
        SSH agent: the socket is removed and the git config it is given
        runs no credential helper.
```

**What changed is the KIND of statement**, exactly as `19-27`'s repair one clause
over did. *"As started"* states what the envelope ESTABLISHES and stops; it
claims nothing about what a command issued during the run can change. **It makes
no completeness claim and enumerates nothing.** `runs no credential helper` is a
measured phrase: with the injected pair, `--get-all` still LISTS the helper while
`credential fill` fails closed, so *"names no helper"* would now be false where
*"runs no credential helper"* is true.

**THE TOKEN ARITHMETIC, STATED:**

```text
before          213 whitespace tokens of a 215 cap   (two tokens of headroom)
Guaranteed      31 -> 28   by dropping a CLAIM  ("not emptied"; both config scopes named)
Not guaranteed  23 -> 24   by GENERALISING a LIMITATION, not dropping one:
                           "the hook stubs and ledger the envelope installed"
                        -> "the files and the binary this envelope runs on"
after           211 tokens.   **THE CAP WAS NOT RAISED.**
```

The generalisation is one token longer and covers a route the old wording did
not — `T-19-116`, the binary the stubs and the guard registration both exec.
`cannot reach your ambient git credentials` survives **verbatim, first and
unwrapped** on one 69-character line; all three pinned phrases keep their order;
widest line 74 of an 80 cap; `\x20` indentation checked on the **RENDERED**
constant. **No residual disclosure was deleted.**

### `T-19-118` — the branch taken, and why BOTH halves were required

**BOTH the clause repair AND the mechanism control were taken, and the plan was
right that neither alone is enough.** The clause repair alone leaves a live
`high`-severity credential reach admitted and uncontrolled; the rule alone closes
the absolute-literal redirection spelling and leaves six others.

**THE CONTROL: one `credential.helper` pair with an EMPTY value, added through
the existing `config_env` triplet builder** — never a second construction site.
Env-injected pairs are applied after every file, so **it defends `C-05`'s
credential half against EVERY write spelling — redirection, tilde, glob, brace,
expansion-borne, relative and symlinked alike — because it reads no command line
at all.** That reason does not depend on this round's rule landing, which is why
it is strictly wider than any path rule.

**MEASURED AGAINST REAL GIT 2.43.0 BEFORE THE ASSERTION WAS WRITTEN**, quoting
`19-28`'s recorded measurement and reproducing it:

```text
                            --get-all credential.helper       credential fill
no injected pair            `store`              exit 0       ambient secret PRESENT
EMPTY-helper pair injected  `store`, then an     exit 0       secret ABSENT
                            EMPTY line
+ a later -c credential.helper=…                              secret PRESENT
```

**THE `--get-all` READING IS RECORDED BESIDE THE CRITERION, and the distinction
is measured rather than stylistic.** The config query still LISTS the helper
while the fill fails closed: git's empty value resets the helper list that
**RUNS**, not the list the query **ENUMERATES**. **A gate built on `--get-all`
reports a working control as broken.** The criterion is the call that names the
harm.

**AT LEAST ONE ROW DRIVES A SPELLING RULE (a) DOES NOT REACH.** The asserting row
is built on a **TILDE** write to the generated `gitconfig` — one of the three
spellings that get no rule and never will — so the row proves the control is
**wider than the rule** rather than duplicating it.

**MEASURED COST:** `GIT_ASKPASS` UNTOUCHED, `gh` unaffected, and **a later
`-c credential.helper=store` on the same line OVERRIDES the reset** — recorded as
a **BOUNDED RESIDUE, bounded because that spelling is ARGV-VISIBLE and already
governed**, unlike every write spelling.

**A FOURTH ROW MOVED, and it is the plan's own false negative in reverse.**
`tests/envelope_credential.rs`'s
`a_credential_helper_the_user_really_has_stops_being_resolvable_under_the_envelope`
asserted that `git config --get credential.helper` FAILS. With the empty pair the
query SUCCEEDS and prints an empty line — **a gate on the query's exit status
reports a control that got STRONGER as one that broke.** The criterion is
corrected to the EFFECTIVE VALUE: what git would run must be empty and must not
be the user's `store` helper, with the file's own control unchanged. That file is
not in this plan's fenced list.

**`AR-19-04` is RECORDED and NOT un-accepted, not re-rated and not renumbered.**
Its rationale — *"the envelope regenerates it at each run start"* — covers
tampering BETWEEN runs and says nothing about credential REACHABILITY DURING one;
that reasoning gap is recorded beside it and nothing more. **`T-19-23` is NOT
marked closed or re-closed** — whether the repair closes it is audit 11's
judgement. **`AR-19-05` is untouched. No `AR-` row was added, edited, renumbered
or un-accepted, and `grep -cE '^\| AR-19-13 \|'` over this file is 0.**

### `T-19-117` — the ledger bound, NOT severed, with its derivation

**This plan's ONE named severable item was NOT severed.** It was executed in
full.

**`MAX_LEDGER_BYTES = 8 MiB`, DEADLINE-derived and NOT cap-derived.** From
`19-28`'s measured curve, `tally` costs ≈ 41 ms per megabyte; 8 MiB is ≈ 340 ms,
**under 7% of `GUARD_TIMEOUT_SECS = 5`**, leaving the rest of the budget for the
rest of the guard. **A cap-derived bound would be a bound the agent steers** —
`pr_cap_per_24h` and `pr_cap_per_run` are UNCLAMPED and read from a file the
driven agent can write (`C-15`), and the caps are not clamped here.

**IT REFUSES RATHER THAN READING LESS**, against `tally`'s own no-under-count
invariant: a tail read, a line cap and sampling all UNDER-COUNT by construction,
which violates that invariant **in the direction that permits**. The contrast is
quoted three functions above, where `ends_mid_line` already refuses a whole-file
read for a one-byte question — *"the sort of thing that turns a guard into a
hang"* — and the read beneath it was unbounded until now.

**The check is a `stat` placed BEFORE the read and BEFORE the append**, so the
refusal is fast: a bound that refused only after reading would answer the harm
with the harm. It is an `Err` rather than a `CapVerdict::Refuse`, so the guard
parks at **`ParkReason::EnvelopeAssertionFailed`** and never at `PrCapExceeded`,
which names a cap that FIRED (**D-24**). The refusal names the ledger's own path
and says it can be archived (**AR-19-11**).

**MEASURED, AFTER:**

```text
   ledger bytes    gh pr create        git push --force    ls / git status
   0                       0 ms exit 0            0 ms exit 2        0 ms
   940,000                41 ms exit 0            0 ms exit 2        0 ms   <- under the bound
   18,800,000             10 ms exit 2            0 ms exit 2        0 ms   <- past it
   188,000,000             2 ms exit 2            0 ms exit 2        0 ms   <- past it
   BASE (19-28, no bound): 7900 ms at the last level, PAST the 5 s deadline.
```

**THE OVER-REFUSAL, DISCLOSED FROM BOTH SIDES.** A ledger past the bound refuses
**the forge commands the cap governs and nothing else** — every `git` command and
every ordinary shell call at the same level is unaffected and still reaches its
own verdict. `19-28`'s just-under-the-bound control at 1 MiB **still permits AND
still counts**, which is the half that keeps this a size bound rather than a
disarmed cap.

**THE BEHAVIOURAL HALF — what the agent CLI does past its registered timeout — is
UNMEASURED and is claimed in NEITHER direction.** It is a property of a
closed-source binary.

### `mod.rs:165-168` corrected — D-09's LAST uncorrected home

```text
BEFORE  "…That party can equally unset `GIT_CONFIG_COUNT`, which is D-09's stated
         ceiling, so this override does not lower a boundary that was standing."

AFTER   the conclusion is KEPT and re-grounded: the override reaches no further
        than that because the variable is measured INERT as a guard lever — and
        the paragraph now names the FILE route and the BINARY route as ceilings
        the environment route does not bound, with `env -u GIT_CONFIG_COUNT git
        push --force` recorded as REFUSED beside them.
```

**`T-19-116` is a route that lowers layer 2 AND layer 3 without controlling the
environment the TUI starts in at all**, so the old ground is gone: **the
narrative named the route that is caught while the routes that work were caught
by nothing.** `19-27` read this paragraph, reached the same conclusion and
**correctly DECLINED** to edit it because `mod.rs` was outside that plan's stated
file set, taking its own stated fallback rather than silently widening its diff;
audit 10 endorsed the decline and named the paragraph for whoever picked it up.
**This round picked it up as a named exception with ZERO non-doc lines changed**,
verified by `git diff --unified=0` filtered for non-`///`, non-blank lines
returning nothing. `GSD_MM_ENVELOPE_ROOT`'s value and behaviour are unaltered.

### THE SIX NAMED FILE EXCEPTIONS, stated AS exceptions, with the verification each held

| file | bound | verification |
|---|---|---|
| `policy.rs` | the redirection channel, the `Segment` field, the widened predicate and its doc, the refusal, `842`→`421`, additions inside the existing `mod tests` | `resolve_program`, `resolve_program_with_head`, `first_unreadable_decision_word`, `config_key_operand_index`, `subcommand_word_indices`, `scan_gh_api`, `scan_leading`, `forbidden_repo_path`, `SEPARATORS` and `is_separator` show **ZERO** diff lines; ONE `#[cfg(test)]` line |
| `hooks.rs` | `guard_in`'s doc and preamble, `classify_segments`' signature and recursion, the ONE call site and its comment | `pre_push`, `pre_commit`, `deny`, `read_guard_request`, `park_refusal`, `write_stub`, `stub_body`, `install`, `install_in`, `hooks_dir_in`, `assert_provenance`, `assert_provenance_in`, `settings_value`, `settings_json`, `guard_command`, `write_settings`, `write_settings_in`, the D-07 second-carrier table and `mod tests` show **ZERO**; the `settings_json` second-carrier row is **NOT repaired** |
| `ledger.rs` | `record_and_check_in`'s new guard and `MAX_LEDGER_BYTES` with its doc — **the first plan in the phase to open this file** | `tally`, `append_entry`, `ends_mid_line`, `parse_stamp`, `saturating_bump`, `ledger_path`, `ledger_path_in`, `LedgerEntry`, `Tally`, `CapVerdict`, `WINDOW_SECS` show **ZERO** |
| `cred.rs` | the reach and limit paragraphs, `write_gitconfig`'s doc, and the ONE injected pair with its doc | `write_gitconfig_in`, `gitconfig_body`, `write_askpass_stub_in`, `configured_remote_host`, `EnvelopeEnv`, `build_env_in` untouched; **one in-file unit assertion widened and renamed**, recorded below |
| `advisory.rs` | `SECTION_ENVELOPE`'s literal and its doc comment | `envelope_notice`, `protection_line`, `PROTECTION_WARNING`, `PROTECTED_CLAIM`, `ProtectionState` show **ZERO** |
| `mod.rs` | `ENVELOPE_ROOT_ENV`'s residual-limit paragraph | **ZERO non-doc lines**, verified by the filtered `git diff --unified=0` |

**`T-19-61` … `T-19-73`, `T-19-84` and `T-19-85` were NOT taken on, and `scan.rs`
and `config.rs` were NOT opened.** `Cargo.toml` and `Cargo.lock` are untouched
(`T-19-SC` holds phase-wide).

**ASSERTIONS UPDATED, BY TEST FUNCTION NAME, each a named and bounded exception:**

| test function | file | why |
|---|---|---|
| `direction_i_a_redirection_target_is_not_an_operand_and_stays_permitted` → `after_19_29_direction_i_a_redirection_target_is_read_and_the_control_outside_the_root_is_not` | `envelope_control_carrier.rs` | the **human-authorised** exception |
| `the_ledger_inflation_spellings_are_all_permitted_today` → `after_19_29_the_ledger_inflation_spellings_are_reached_and_the_outside_controls_are_not` | `envelope_carrier_reach.rs` | its own comment already stated the flip |
| `t_19_117_the_ledgers_size_is_unbounded_work_on_the_guards_critical_path` → `after_19_29_t_19_117s_unbounded_work_is_bounded_and_the_forge_split_is_unchanged` | `envelope_carrier_reach.rs` | mutually exclusive with the fix it demands |
| `a_credential_helper_the_user_really_has_stops_being_resolvable_under_the_envelope` | `envelope_credential.rs` | criterion moved from the QUERY to the EFFECTIVE VALUE |
| `the_triplet_names_the_count_the_key_and_the_directory` → `the_injected_pairs_name_the_count_the_keys_and_their_values` | `cred.rs`'s own `mod tests` | the injection carries a second pair; the pin is WIDENED, still asserting the whole vector exactly |

**`git diff --numstat` over `tests/` across all four commits does NOT show zero
deletions, and that is stated plainly rather than claimed away.** It shows
`384/35` for `envelope_carrier_reach.rs`, `99/26` for `envelope_control_carrier.rs`
and `37/4` for `envelope_credential.rs`. **Every deleted line belongs to one of
the FIVE corrected test functions named in the table above, and to nothing
else.** No row was deleted; each was renamed and re-aimed with its reasoning
rewritten in place, and the total test count is unchanged by any of them —
the +12 comes entirely from new `#[test]` fns. **`19-29`'s plan asserted zero
deletions as a success criterion; that criterion is not met, and the reason is
that three rows pinned a pre-fix verdict the fix necessarily changes.** One of
those was authorised by a human in advance; the other two were not, and both are
reported as findings above rather than absorbed into the authorisation.

### The `421` correction

`policy.rs:7450` read *"842 multi-byte characters"*. **842 is the
byte-minus-character DIFFERENCE and the count is 421**: each of these is a
THREE-byte character contributing TWO extra bytes, and 421 × 2 = 842. The
comment now says so. **Every other number in that comment was re-derived
independently by audit 10 and is right, and not one of them was touched**;
`POLICY_MIN_PRODUCTION_BYTES`, `HOOKS_MIN_PRODUCTION_BYTES`, the proportional
floor's value and strictness, the deep anchor and the sentinel counts do not
move. This plan adds prose to `policy.rs`, which only grows the production half.

### Carried forward, unfixed and stated

`C-05`'s alias half, `C-07`, `C-08` and `C-12` are carried forward unfixed;
**`C-08`'s behavioural half stays UNMEASURED and is claimed in NEITHER
direction** — repairing a delivery is a change to the SPAWN SEAM and was out of
scope. `C-15` keeps control (e) with **no clamp**: clamping a configured cap is a
PRODUCT decision, not a guard rule. `C-11` … `C-15` keep control (e) entirely.
The `glab --host` forge cell is unfixed, **`FORGE_VALUE_OPTS` keeps
`--hostname`**, and **`glab` is confirmed NOT INSTALLED**, so no pin was written
that would skip. `T-19-110` stays at `medium`.

### `T-19-17r` is OUTSTANDING for the TWELFTH time

`grep -cE '^\| AR-19-13 \|'` over this file is **0**. **This plan did NOT accept
`T-19-17r`, added no Accepted-Risks-Log row, created no `AR-19-13`, and did not
apply the word "accepted" to it anywhere** — not in a test name, not in a
comment, not in either record. Accepting a risk is a human decision, and twelve
agents have now deliberately left this one unmade.

### The mechanism pins and the byte floors, RE-MEASURED

`SEPARATORS` is **byte-identical** at `policy.rs:2297` (one commit in the whole
phase, `84a9b05`) and `policy::is_separator(">")` is `false`, both asserted by a
new pin with a positive control. Round 5's literalness bit is non-vacuous and is
now read in a **THIRD** place. Round 6's deletion model is unchanged in behaviour
and its over-deletion control still permits. Round 7's four callee-grammar
directions, round 8's confinement clause with both `--signed no` controls, round
9's re-parse clause with `aliasx.` / `notalias.` and both `T-19-86` `!`-bodied
rows at exit 0, and round 10's clause over all nineteen envelope-root operand
rows are green. **Not one config-resolution verdict moved**
(`envelope_config_resolution` 30/0). The stripper's three protections keep their
strictness and `policy.rs` still has exactly ONE `#[cfg(test)]` line.

### The gate, with its arithmetic STATED and CHECKED

```text
command   rtk proxy cargo test --no-fail-fast   (counts read with `rtk proxy grep`, D-34)
before    1808  passed + failed   (1802 passed, 6 failed, 13 ignored, 46 result lines)
after     1820  passed + failed   (1818 passed, 2 failed, 13 ignored, 46 result lines)
delta     +12
check     12 new `#[test]` fns across all four commits, 0 removed (git diff HEAD~4..HEAD)
          1808 + 12 = 1820.   **The identity holds exactly.**
```

**A red test RAN, so red→green leaves the total unchanged and every increase
comes ONLY from new `#[test]` fns.** **The RED set from `19-28` is now EMPTY.**

**SEVENTEEN `envelope_*` binaries RAN** — sixteen would have meant an evidence
file did not execute:

| binary | passed | failed |
|---|---|---|
| `envelope_advisory` | 10 | 0 |
| `envelope_argv_deletion` | 20 | 0 |
| `envelope_callee_grammar` | 19 | 0 |
| **`envelope_carrier_reach`** | **39** | **0** |
| `envelope_command_position` | 18 | 0 |
| `envelope_config_resolution` | 30 | 0 |
| `envelope_control_carrier` | 37 | 0 |
| `envelope_credential` | 6 | 0 |
| `envelope_expansion_slots` | 32 | 0 |
| `envelope_hook_refusals` | 7 | 0 |
| `envelope_literal_decision` | 43 | 0 |
| `envelope_pr_cap` | 11 | 0 |
| `envelope_reparsed_value` | 34 | 0 |
| `envelope_tracer` | 6 | 0 |
| `envelope_wiring` | 14 | 0 |
| `envelope_wrapper_bypass` | 13 | 0 |
| **`envelope_wrapper_class`** | **52** | **0** |

`cargo build` and `cargo clippy -- -D warnings` both exit 0. **`cargo clippy
--tests` is NOT the gate** — it already fails at base on four pre-existing lints
in `src/browser.rs` and `src/project_creator.rs`, which were not touched.

**DOCUMENTED FLAKES — TWO FIRED AND ARE RECORDED RATHER THAN REPORTED CLEAN.**
Both `tests/driver_reattach.rs` failures fired in the gate run:
`a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` and
`a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`.
**Re-run in isolation they are 3 passed / 0 failed**, which is the documented
signature. They were not fixed and are out of scope. **The `envelope_tracer`
ETXTBSY race did NOT fire** in the gate run or in this plan's measurements —
`envelope_tracer` is 6/0. **That race is `C-10`'s own seam and this round changed
what the guard does about that seam**, so its absence is RECORDED and its
relation to this round's change is stated in NEITHER direction: absence is not
evidence it is fixed, and it is not evidence this round affected it.

---

## Threats found by audit 11 (2026-09-04, after plans 19-28 and 19-29)

**Provenance, stated first.** The two subsections above this one were written by
the EXECUTORS of plans 19-28 and 19-29; they are deliberately outside the audit
tables. Audit 11 left them, every earlier appended subsection and every earlier
audit's own tables **byte-identical** — the body below the frontmatter was
checksummed before writing (`sha256 8f65ad89f70afb72d2…` over the 821,703 bytes
below the frontmatter of the 822,705-byte file), and this audit's write changes
the frontmatter, adds one Security-Audit-Trail row, one method subsection and
everything from here to the end of the file, **deleting nothing**. Everything
below is **audit 11's own**, measured against the built binary at `1e56389` with
a fresh `GSD_MM_ENVELOPE_ROOT` per row and the whole envelope root walked
afterwards, and with every claimed bypass confirmed against the **real `git`
binary** in a rebuilt bare-remote fixture with a control beside every leg.

### The question this audit was set, answered plainly

**Audit 10 said the modelled surface is complete and the remaining work was
disposition, not discovery. Now that the disposition is done: is the plane
finished, are the residues true as stated, and does anything this round installed
create a new reachable state?**

**Three answers, and they are not the same answer.**

1. **The plane is finished. Audit 10 was right and audit 11 says so plainly.**
   Fifteen carriers, fifteen letters, and after `19-29` ten of them have a rule
   rather than nine. Audit 11 swept the cells adjacent to all six named axes and
   found **no twelfth plane** — no carrier that is neither an argv nor a file, no
   new way to change *when* the guard runs, no state it carries between calls
   that `19-26` did not enumerate. **The eleventh question was the right one and
   it is answered.**

2. **Nothing this round INSTALLED creates a new reachable state, and each of the
   three new mechanisms was probed as attack surface in its own right.** The
   `Segment`-borne redirection channel did not undo round 5 or round 6 — verified
   below at the byte, the token and the verdict. The empty-`credential.helper`
   injection adds no env-reachable state, because `ENVELOPE_ENV_KEYS` covers
   `GIT_CONFIG_KEY_`/`GIT_CONFIG_VALUE_` by PREFIX and therefore covered the new
   pair by construction — eight env spellings measured, all exit 2. The
   exact-path binary clause refuses nothing it should not: both near-miss
   controls stay exit 0. **The ledger bound cannot be turned into a bypass** — it
   is a `stat` before both the read and the append, and it fails closed.

3. **But the residues are NOT true as stated, and the plane's own arithmetic is
   wrong for the third consecutive round.** `19-29` corrected the residue from
   FOUR directions to SEVEN and stated it as a CONDITION rather than a list —
   which is the right remedy and audit 11 endorses the form. **The condition it
   states is still not the condition the code implements.** The docs say the rule
   is silent about *a word the SHELL MAY REWRITE*, *a word that IS NOT ABSOLUTE*
   and *a word that reaches a protected path ONLY THROUGH A LINK*. There is a
   fourth silence, it belongs to none of those three, and it is the widest of all
   of them: **a word that is fully literal, needs no expansion, no cwd and no
   link, and names an absolute path — carried as an OPTION-ATTACHED value.**
   `dd if=/bin/true of=<BINARY>` is **exit 0**, and audit 11 drove it end to end
   until a bare remote's `main` moved.

**So the honest one-sentence answer: the plane is finished, the round's three
mechanisms are sound, and the round's own residue statement is incomplete in the
cell one CHARACTER over from the one it just corrected.**

### The new rows

| Threat ID | Category | Component | Severity | Disposition | Finding | Status |
|---|---|---|---|---|---|---|
| **T-19-119** | **Elevation of Privilege / Tampering** | **`lexical_absolute_components` (`policy.rs:5457-5475`), the normaliser BOTH halves of `protected_carrier_named`'s path set are built on. It returns `None` unless the WORD ITSELF begins with `/`** | **high** | **mitigate (unimplemented — not among the seven declared directions, named nowhere)** | **AN OPTION-ATTACHED PATH IS INVISIBLE TO BOTH HALVES OF THE RULE, IT REACHES EVERY ONE OF `C-01` … `C-10`, AND AUDIT 11 DROVE IT END TO END AGAINST REAL GIT.** A word like `of=/abs/path` or `--target-directory=/abs/path` carries an absolute path inside a token that does not START with `/`, so `lexical_absolute_components` answers `None` and both `word_is_within` and `word_is_exactly` answer `false`. `Token.literal` is TRUE for it — there is no `$`, no `~`, no glob and no brace — so **it is none of the seven declared directions**, and it needs no prior read, no symlink, no cwd and no expansion. **THE BINARY, DRIVEN END TO END** in a rebuilt bare-remote fixture with `pre-push`/`pre-commit` in `stub_body`'s exact three-line shape delivered through the envelope's own `GIT_CONFIG_COUNT`/`KEY_0`/`VALUE_0` triplet: **LEG A (control)** — real binary in place, `git push --force origin HEAD:refs/heads/main` → **REFUSED by the hook**, remote `main` UNMOVED at `835b6be`. **LEG B** — `dd if=/bin/true of=<BINARY>` is **exit 0 through the guard** (against `cp /bin/true <BINARY>` at **exit 2**, the same program-free pair differing only in the `=` join), then the same push → **COMPLETED, remote `main` `835b6be` → `40ad7f7` MOVED**. **LEG B-control** — binary restored, remote rewound, same push → **REFUSED, UNMOVED**. **LAYER 2 separately**: the replaced binary answering `envelope guard alpha` on `git push --force origin main` → **exit 0 (PERMIT)**. **THE LEDGER, DRIVEN END TO END**: four `gh pr create` calls in one persistent root → 1 permit then 3 `pr_cap_exceeded`, four ledger lines; `dd if=/dev/null of=<ENV>/alpha/pr-ledger.ndjson` → **exit 0**, ledger truncated to 0; call 5 → **PERMITTED again**; the twin `cp /dev/null <ENV>/alpha/pr-ledger.ndjson` → **exit 2**. **THE CLASS IS BROAD AND EVERY ROW WAS CONFIRMED TO REACH THE FILE UNDER REAL `bash`**: `dd of=`, `tar --directory=`, `cp --target-directory=`, `rsync --temp-dir=`, `chmod --reference=`, `python3 --out=` and `git --git-dir=` are all **exit 0** over the envelope root, while every space-separated twin is **exit 2**. **AND THE PREDICATE'S OWN DOC CITES THIS SPELLING AS ONE IT REFUSES**: `policy.rs:5743` argues the disclosed read over-refusal is unavoidable because *"is `dd if=X of=Y` a read of `X` or a write of `Y`?"* — measured, `dd if=<ENV>/alpha/pr-ledger.ndjson of=/tmp/stolen` is **exit 0** and is neither. **This re-opens `T-19-116` and re-opens `T-19-112`'s cap route by a spelling neither round drew**, and it is `T-19-76`'s failure mode for the ELEVENTH consecutive round — the round widened the corpus to eleven classes and every carrier path in all eleven is either a bare absolute word or one of the six declared rewrites; **not one entry anywhere carries a path after an `=`.** | **OPEN — BLOCKING** |
| **T-19-121** | **Information Disclosure / Elevation of Privilege** | **`git -c credential.helper=<value>` on one permitted command line. `scan_leading`'s leading-option region READS the word; no rule REFUSES it** | **high** | **mitigate (unimplemented)** | **THE RUN REACHES THE USER'S AMBIENT GIT CREDENTIAL ON A SINGLE PERMITTED COMMAND LINE, WITH NO FILE WRITE AT ALL — the same harm `T-19-118` registers and `T-19-23`'s declared subject.** Measured against real git in a fake HOME holding `~/.git-credentials`, with the envelope's FULL posture applied (the generated `gitconfig` on both `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM`, and `19-29`'s own empty-`credential.helper` pair injected): `git -c credential.helper=store credential fill` for `https://github.com` returns the **ambient username and password**, against the same call without the `-c` at **exit 128, secret ABSENT**. Through the guard, `git -c credential.helper=store` is **exit 0** on `credential fill`, on `fetch origin`, on `ls-remote origin` and on an in-namespace `push`. **IT IS PRE-EXISTING AND THIS ROUND DID NOT CREATE IT** — audit 11 re-measured it at round 10's exact posture (generated file, no injected credential pair) and it returns the ambient secret there too, so it is a gap ten audits did not find rather than a regression. **WHAT THIS ROUND DID IS DESCRIBE IT WRONGLY.** `cred.rs:420-424` introduces it as *"a BOUNDED residue rather than a reason to decline the control, and the bound is stated rather than assumed: that spelling is ARGV-VISIBLE and is already GOVERNED by `scan_leading`'s leading-option region"*. **Argv-VISIBLE it is; GOVERNED it is not.** `scan_leading` parses the word and no rule acts on it — the by-name deny covers `core.hooksPath`, round 8's confinement clause covers `include.path`, and **nothing covers `credential.helper`**, which is measured. A residue whose stated bound is that another layer governs it, where that layer does not, is `T-19-84`/`T-19-107`/`T-19-109`/`T-19-115`'s shape a SIXTH time. `T-19-23` is left as this file records it and audit 11 **does not un-close it** — that is a human decision — but its declared mitigation (*"the generated file naming no helper"*) is now the weaker half of a two-part control and **this route defeats both halves.** | **OPEN — BLOCKING** |
| T-19-120 | Denial of Service | `ledger::MAX_LEDGER_BYTES` (`ledger.rs:114`, checked at `:291-311`) — the bound is over the file's SIZE and not over its KIND | medium | mitigate | **`T-19-117`'s bound closes unbounded WORK and leaves unbounded WAIT, which is strictly worse than the case it was written for.** The check is `std::fs::metadata(&path).map(|meta| meta.len())`, and **a FIFO stats at length 0**, so it passes the bound; `std::fs::read` three lines below then blocks until a writer that never comes. Measured against the built binary: with `<ENV>/alpha/pr-ledger.ndjson` created by `mkfifo`, `gh pr create --title x` **timed out at 20 s (exit 124)** and never returned — against the **7.90 s** `19-28` recorded for a 202 MB regular file and the **18 ms** the same file now answers in. **REACHABILITY, measured with the absolute-literal twin beside it**: `mkfifo <ENV>/alpha/pr-ledger.ndjson` is **exit 2**, while `mkfifo <ENV>/alph?/…` (glob), `mkfifo <ENV>/alpha/{pr-ledger.ndjson,x}` (brace), `mkfifo $R/alpha/…` (expansion-borne) and `mkfifo ~/.local/share/gsd-meta-manager/envelope/alpha/…` against the REAL root (tilde) are **all exit 0** — four of the seven declared fail-open directions, plus `T-19-119`. **The comment beside the check states the fail-open and gives its reason** — *"A file that cannot be stat'd is not a file this check can refuse on"* — and that reasoning is sound for a stat that FAILS; the FIFO case is a stat that SUCCEEDS and answers 0. **The BEHAVIOURAL half is UNMEASURED and claimed in NEITHER direction**, on the same discipline `C-08`'s and `T-19-117`'s behavioural halves are held to: what the agent CLI does with a `PreToolUse` hook that never returns is a property of a closed-source binary. Rated `medium` on the mechanical half alone, exactly as `T-19-117` was. | open — below `high` (non-blocking) |

*Status: open · closed · open — below `high` threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` count toward `threats_open`*

`T-19-120` is open below the `high` threshold and does **not** count toward
`threats_open`. `T-19-119` and `T-19-121` do.

### `T-19-117` and `T-19-118` — CLOSED AS SCOPED, each with its successor named

**This audit closes two rows and says plainly why each closes, because a phase
that has been blocked for eleven rounds needs its true closures as much as its
true findings.**

**`T-19-118` — CLOSED AS SCOPED, successor `T-19-121`.** Its declared harm is the
run reaching the ambient credential through `C-05`, the generated `gitconfig`.
Audit 11 re-drove the whole thing against real git 2.43.0 in a fake HOME holding
`~/.git-credentials`:

```text
CONTROL   generated file as written, no injected pair
          --get-all credential.helper -> exit 1, no helper
          git credential fill         -> exit 128, ambient secret ABSENT
HARM      `[credential] helper = store` appended (T-19-118's own row)
          --get-all credential.helper -> `store`, `store`, exit 0
          git credential fill         -> exit 0, ambient username and password PRESENT
CONTROL   the same file, with 19-29's injected empty-helper pair
          --get-all credential.helper -> `store`, `store`, then an EMPTY line, exit 0
          git credential fill         -> exit 128, ambient secret ABSENT
```

**The `--get-all` false negative is real and the criterion choice was right.** The
query still LISTS the helper at exit 0 while the fill fails closed; a gate built
on the query would have reported this working control as broken, and `19-29`'s
own deviation 3 records it catching exactly that in `envelope_credential.rs`.

**The control is wider than the rule, and audit 11 probed the cell beside it
rather than accepting the claim.** A **URL-SCOPED** helper —
`[credential "https://github.com"] helper = store`, a different config key that an
empty `credential.helper` might not have reset — was appended to the generated
file and measured: **exit 128, secret ABSENT with the pair injected, and the
ambient username and password PRESENT without it.** So the reset covers the
URL-scoped spelling too. Combined with the redirection rule, **every one of the
six write spellings `19-28` enumerated now fails closed**, and so do the tilde,
glob and brace spellings that have no rule and never will — because the control
reads no command line.

**And the mechanism adds no new state.** Eight environment spellings that would
strip or override the new pair were driven: `GIT_CONFIG_COUNT=1`,
`GIT_CONFIG_KEY_1=credential.helper`, `GIT_CONFIG_KEY_2=credential.helper`,
`env GIT_CONFIG_VALUE_1=store`, `env -u GIT_CONFIG_KEY_1`,
`GIT_CONFIG_GLOBAL=/tmp/evil`, `GIT_ASKPASS=/tmp/evil` and
`SSH_AUTH_SOCK=/tmp/evil` — **all exit 2**. `ENVELOPE_ENV_KEYS` carries
`GIT_CONFIG_KEY_` and `GIT_CONFIG_VALUE_` as PREFIX entries and `envelope_env_key`
matches them with `starts_with`, so the second pair was covered **by construction
rather than by an edit** — which is why no drift pin had to move.

**The cost is measured and is nil where it matters.** `GIT_ASKPASS` still answers
— a responder stub returns its value through `git credential fill` with the pair
injected — so D-17's only token channel is intact, and `gh` is closed from the
other side by `GH_CONFIG_DIR`.

**What is NOT closed is the argv route, and it is registered as `T-19-121` rather
than folded into this closure.** `T-19-23` is **not** marked closed or re-closed
by this audit, and `AR-19-04` and `AR-19-05` are **untouched, not un-accepted and
not renumbered.**

**`T-19-117` — CLOSED AS SCOPED, successor `T-19-120`.** Its declared harm is
`tally` over a whole-file read crossing `GUARD_TIMEOUT_SECS = 5`. Re-measured at
`1e56389` against the built binary, one root per level:

```text
ledger bytes    gh pr create        ledger lines after
1,000,000        exit 0, 13 ms      counted   <- under the bound, still permits AND still counts
8,388,608        exit 0, 61 ms      counted   <- exactly MAX_LEDGER_BYTES
8,388,609        exit 2, 28 ms      0         <- one byte past it
25,000,000       exit 2, 18 ms      0
202,000,000      exit 2, 18 ms      0         <- 19-28 measured 7,900 ms here
```

**The discrimination is exact at the constant**, the refusal is 18 ms rather than
7.9 s, and the just-under control **still permits and still counts** — which is
what keeps this a size bound rather than a disarmed cap. **The over-refusal is
bounded exactly as disclosed, measured from both sides** against a 20 MB ledger:
`ls` exit 0, `git status` exit 0, `git push --force origin main` exit 2 at its own
`force_push_blocked`, and only `gh pr create`, `gh api …/pulls` and
`glab mr create` refused. **It is deadline-derived and not cap-derived**, which
matters because `C-15` leaves `pr_cap_*` unclamped and agent-writable, and it
**fails closed** — an `Err`, so the guard parks at `EnvelopeAssertionFailed` and
never at `PrCapExceeded`, which would name a cap that fired (D-24). **The bound
cannot be turned into a bypass**: the `stat` precedes both the read and the
append, so there is no window in which a large ledger is appended to but not
counted. **What it cannot do is bound a file whose length is not its size** —
that is `T-19-120`.

### The `Segment`-borne redirection channel — round 5 and round 6 are INTACT, verified three ways

**The mandate's sharpest question, and the answer is that the round got this
right.** The obvious implementation — pushing the target into the token stream —
would have split `git >/dev/null push --force origin main` into `[git]` and
`[push, --force, origin, main]`, and the leading `git` resolves `Governed` with an
empty argv, which `classify_git` answers `Allow` for. **Round 6's headline refusal
would have become a permit.** It did not.

| what | audit 11's measurement |
|---|---|
| `SEPARATORS` | **byte-identical** at `policy.rs:2297`; `git log -L 2297,2297` over the whole phase returns **exactly one** commit, `84a9b05` (plan 19-05) |
| `is_separator(">")` | `false` **by construction** — `is_separator` is a bare `SEPARATORS.contains` at `policy.rs:3811` and `>` is not in the list |
| `segment.tokens` | **byte-identical** — `git >/dev/null push --force origin main` is ONE segment carrying exactly `[git, push, --force, origin, main]` |
| round 6's headline | `git >/dev/null push --force origin main` → **exit 2 `force_push_blocked`** |
| round 6's over-deletion control | `git x2>/tmp/o push --force origin main` → **exit 0**, unmoved |

**The SEGMENT-COUNT pins are green AND they would catch both variants — audit 11
read the assertion rather than the name.**
`a_redirected_simple_command_is_exactly_one_segment_with_exactly_the_surviving_argv`
asserts `segments.len() == 1` **and** asserts the exact token vector. The first
catches the **SPLIT** variant (a target arriving as an operator token flushes the
segment, count becomes two); the second catches the **DISPLACED** variant
(`T-19-98`'s registered shape — a target admitted as an ordinary word leaves the
count at one and shifts every decision index, and the observed vector would carry
`/dev/null` where `push` belongs). **Neither variant can pass, and the failure
message names both.** The pin is falsifiable in both directions rather than
green-by-construction.

**The channel is COMPLETE over the operator grammar and correct on the
pathname/non-pathname split**, which audit 11 swept rather than accepted. All
seven pathname operators reach the carrier and are refused — `>`, `>>`, `>|`,
`<`, `<>`, `&>`, `&>>`, plus the IO_NUMBER form `1>`, the attached form
`>/path` with no space, and both the double- and single-QUOTED target. The
target is found in segment 1, in segment 2, and through the `NestedPayload`
recursion (`bash -lc '… > <ENV>/…'` → exit 2). **And the five non-pathname
operators are correctly NOT recorded as carriers**, so the round bought its rule
without an over-refusal: `cat <<X`, `cat <<<X`, `ls >&2`, `ls 2>&1` and `cat <&0`
are all **exit 0**. The outside-the-root control `echo x > /tmp/outside && ls`
stays **exit 0**.

**`Token.literal` is now read in a THIRD place and was neither cleared nor
repurposed.** Round 5's whole pin family is green, re-measured below.

### The four execution-time judgements, assessed independently

1. **The authorised cross-fence exception — CORRECT, and the rewritten reasoning
   is now TRUE.** Audit 11 re-drove every row rather than reading the record.
   **All three rows flip and no fourth does**: `: > <ENV>/alpha/pr-ledger.ndjson`,
   `printf 'exit 0' > <ENV>/alpha/hooks/pre-push` and
   `echo evil > <ENV>/alpha/askpass` are **exit 2**; the in-test control
   `: > /tmp/l` is **exit 0**; directions (ii) (`D=$(git config --get
   core.hooksPath); rm -f $D/../pr-ledger.ndjson`), (iii) (`rm -f /tmp/l`) and
   (iv) (`rm -f pr-ledger.ndjson`) are all **exit 0, unmoved**; and
   `envelope_control_carrier` is **37 passed / 0 failed** in audit 11's own gate
   run, so no fourth row in that file moved. **Both rewritten reasons are true.**
   *"These rows will not flip"* was false by measurement, and the record names the
   inference error precisely — *the rule cannot see it as a WORD* does not entail
   *the guard cannot establish it at all*. *"A rule that read redirection targets
   would re-open a model five rounds have pinned shut"* was false because it
   assumed the only channel is the token stream; the `Segment`-borne channel is a
   third option and the table above is the proof, not the argument. **Rewriting a
   stale reason in place with why it was right when written is the right handling
   and this is the phase's best instance of it.**

2. **Two unnamed rows reported as findings rather than absorbed — CORRECT, and
   the mutual exclusivity is real.** `t_19_117_the_ledgers_size_is_unbounded_work…`
   asserts in the present tense that a 202,000,000-byte ledger pushes a forge
   command past the deadline; `after_19_29_a_ledger_past_the_size_bound_…` in the
   **same file** requires the bound to sit inside `(1 MiB, 24 MiB)`. Audit 11
   confirms **no constant satisfies both**: any bound under 24 MiB refuses a 202 MB
   ledger, and audit 11 measured that refusal at **18 ms**, so the first row's
   assertion is necessarily false the moment the second row's is true. Correcting
   it was not optional. **Reporting both rows as findings rather than folding them
   into the human authorisation is the right call** — the authorisation named one
   row, and an executor that quietly extended it to three would have converted a
   bounded permission into an open one. `19-28`'s blocking finding was one file
   too narrow and the record says so in its own words.

3. **`current_exe()` resolved in `guard_in` rather than `guard` — SOUND, and it
   does not change what `C-10` means in production.** Verified mechanically:
   `guard_in` has **exactly one production caller**, `guard` at `hooks.rs:837`
   (the only other reference is inside `#[cfg(test)]` at `:2038`), so "once per
   invocation" holds either way and in production `current_exe()` returns the same
   path from either site. **The mandated form was genuinely unsatisfiable and also
   wrong on the merits**: `guard_in` is the explicit-roots entry point the suite
   drives directly, so a path resolved in `guard` would be `None` for every corpus
   row and the clause would be **silently untested** — the exact failure this
   phase has been blocked over. The claim is about *the process answering this
   call*, which for a test process is the test binary, so the substitute makes the
   subject right rather than convenient. **What it changes is only what `C-10`
   means in TEST**, and it changes it in the correct direction. The fail-open when
   the path is unresolvable is real, is stated on the predicate's signature, and
   is pinned with a positive control beside it —
   `each_half_of_the_path_set_is_silent_when_it_was_not_given_a_path`
   (`policy.rs:10287`) asserts both halves silent when unfed and both firing when
   fed, plus that a `file` normalising to `/` answers `false` rather than matching
   every absolute word.

4. **The unmet success criterion, reported rather than claimed — the
   characterisation is EXACT.** Audit 11 re-derived the numbers and then went
   past them. `git diff --numstat` over `tests/` across `19-29`'s five commits is
   **`384/35`, `99/26`, `37/4`** — the criterion *"ZERO deletions"* is indeed not
   met. **Every deletion is accounted for and NO assertion weakened**, which
   audit 11 established mechanically rather than by reading the table: deleted
   lines matching `assert_`, `assert!`, `refuses(` or `permits(` number
   **ZERO**; removed `#[test]` attributes number **ZERO** against **12** added;
   and the only deleted `fn` lines in `tests/` are the **three** renamed
   functions the record names, with the fourth (`envelope_credential.rs`) keeping
   its name and the fifth being `cred.rs`'s own `mod tests` rename
   `the_triplet_names_the_count_the_key_and_the_directory` →
   `the_injected_pairs_name_the_count_the_keys_and_their_values`. **Reporting a
   missed criterion plainly, with the reason and the accounting, is better
   behaviour than meeting it would have been** — three rows pinned a pre-fix
   verdict the fix necessarily changes, and the alternative to renaming them was
   leaving the suite red.

### The known-open set, as audit 11 verified it

Every row re-measured at `1e56389`, fresh root each, walk after.

* **`T-19-86` (high, OPEN, unaccepted, deferred by explicit user scoping
  decision).** `git -c alias.p='!git push --force origin HEAD:refs/heads/main' p`
  → **exit 0**, and the persisted-alias arm
  `git config alias.p '!git push --force …'` → **exit 0**. Unweakened.
  Counts toward `threats_open`.
* **`T-19-91` (high, OPEN, three arms).** `S=x; git reflog $S`,
  `git reflog show $S` and `git symbolic-ref $S` all **exit 0**, while
  `git symbolic-ref HEAD $R` and `git push origin $REF` still fail closed at
  **exit 2**. Unweakened. Counts toward `threats_open`.
* **`T-19-111` (high, OPEN, NO rule).** The `printf`-written `.git/config` alias
  is **exit 0** with an EMPTY walk. Its corpus rows stay `record_only`,
  `19-27`'s five-site attribution correction stands, and **attribution did NOT
  move back into `T-19-86`** — both plans state it is kept OUT and audit 11
  confirms neither record folds it back. Counts toward `threats_open`.
* **`T-19-112` (high) and `T-19-113` (medium) — NARROWED FURTHER, and nothing
  anywhere calls either closed.** Direction (i) over the envelope root is now
  refused, which is real. **`T-19-112` stays OPEN**: `C-15` is a route these
  rules do not reach, the deferred ledger option (b) is another, and audit 11
  drove a **fourth** — `T-19-119` reset the cap end to end. A sweep of `src/`,
  `tests/` and the phase `.md` files for these ids beside the word *closed*
  returns only rows saying NARROWED, NOT CLOSED, or deferring the judgement.
* **`T-19-115` (medium, OPEN, no rule, no acceptance).** All three spellings
  re-measured permitted with their absolute-literal twins refused. **Its
  corrected arithmetic is itself now incomplete** — four became seven and the
  true count is at least eight (`T-19-119`). Recorded there rather than as a
  second row.
* **`T-19-116` (high) — NARROWED, NOT CLOSED, and audit 11 does not close it.**
  Its five registered spellings (`cp`, `mv`, `install -m 0755`, `ln -f`,
  `printf 'x' >`) are all **exit 2** now, and both EXACT-PATH-not-PREFIX controls
  (`cp /bin/true <PARENT>/some-other-file`, `ls <PARENT>`) are still **exit 0**,
  so the rule is real and its boundary kind is right. **But the threat's declared
  harm — one permitted command removing layer 2 AND layer 3 — is still reachable
  and audit 11 moved a bare remote's `main` with it** (`T-19-119`). A threat is
  closed when its harm is unreachable, not when its listed spellings are. Its
  three disclosed residues are also re-measured at **exit 0**:
  `cp /bin/true $(command -v gsd-meta-manager)`, the tilde spelling, and
  `cd <parent> && cp /bin/true gsd-meta-manager`. Counts toward `threats_open`.
* **`T-19-96` (medium, OPEN).** `git push --forc? origin
  refs/heads/gsd-auto/alpha/w` → **exit 0**. **`T-19-110` (medium, OPEN).**
  `git config core.hooksPath -c` and `--` → **exit 0**; the by-name deny
  `git config core.hooksPath /dev/null` → **exit 2**. Audit 9's re-rating carried
  forward unchanged.
* **`T-19-74` (medium, closed/accepted — AR-19-10).** `env $X push --force` →
  **exit 0**, core rows frozen. **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85`** —
  open, unaccepted, untouched; `scan.rs` and `config.rs` were not opened by
  either plan.
* **`C-11` … `C-15` remain entirely control (e)**, no rule, unaccepted. **`C-15`
  keeps no `pr_cap_*` clamp**, deliberately — clamping a configured cap is a
  product decision and making it inside a security round would decide it
  silently. Audit 11 endorses that reasoning.
* **`C-08`'s behavioural half and `T-19-117`'s behavioural half are UNMEASURED
  and claimed in NEITHER direction**, and audit 11 claims neither either. The
  `settings_json` second-carrier row was **not** repaired and the record says so.
* **`FORGE_VALUE_OPTS` keeps `--hostname`**; **`glab` is confirmed NOT installed**
  (`command -v glab` finds nothing), so its cell stays unmeasurable against its
  callee and **no pin that would skip was written**.
* **`AR-19-04` and `AR-19-05` are untouched**, not un-accepted, not re-rated and
  not renumbered. **`T-19-23` is NOT marked closed or re-closed** by either plan
  or by this audit. Twelve `AR-` rows, unchanged.
* **`T-19-17r` — OUTSTANDING for the SEVENTH audit running.**
  `grep -cE '^\| AR-19-13 \|'` over this file is **0**. Plans `19-28` and `19-29`
  both declined, correctly. **This audit is the thirteenth agent to leave the
  acceptance unmade, and it is a human decision.**
* **`T-19-SC` — still holds.** `Cargo.toml` and `Cargo.lock` appear in no commit
  between `0092009` and `1e56389`.

### `C-10`'s registration, and `SECTION_ENVELOPE`'s first `Guaranteed` clause

**`C-10` now has all four things audit 10 asked for**, verified rather than
accepted: a **threat id** (`T-19-116`), a **severity** (`high`), a
**`deferred-items.md` row** (`deferred-items.md:2229`, under a heading that says
the absence of that row is itself the finding) and a **control letter**
(`(a), EXACT PATH, PARTIAL`). The fifteen-carrier table has a letter for every
one of the fifteen.

**`SECTION_ENVELOPE`'s FIRST `Guaranteed` clause is TRUE as repaired**, and audit
11 counted rather than read:

> `As started, this run cannot reach your ambient git credentials or` /
> `SSH agent: the socket is removed and the git config it is given` /
> `runs no credential helper.`

**211 whitespace tokens of an UNRAISED 215 cap**, widest line **74** of an 80 cap
— both re-derived independently from the constant with the `\x20` escapes
expanded. Both halves hold by measurement: the socket is removed, and *"the git
config it is given runs no credential helper"* is true against every file-write
spelling including tilde and URL-scoped, because of the injected empty pair.
*"As started"* states what the envelope ESTABLISHES and stops; **it makes no
completeness claim and enumerates nothing**, which is the same repair `19-27`
made one clause over. `cannot reach your ambient git credentials` survives
verbatim, first and unwrapped. **`runs` rather than `names` is load-bearing and
measured** — `--get-all` still lists the helper while `credential fill` fails
closed, so *"names no helper"* would now be false where *"runs"* is true.
**Decision endorsed.** The `Not guaranteed` half's generalisation to *"the files
and the binary this envelope runs on"* covers `T-19-116`'s route, which the old
wording did not.

**`mod.rs:165-168` — CORRECTED, and it is D-09's last uncorrected home.** The old
ground (*"that party can equally unset `GIT_CONFIG_COUNT`"*) is gone; the
conclusion is kept and re-grounded on the variable's measured inertness, and the
paragraph now names **both** the FILE route and the BINARY route as ceilings the
environment route does not bound, with `env -u GIT_CONFIG_COUNT git push --force`
recorded as REFUSED beside them. Audit 11 re-measured that refusal at
`hook_bypass_blocked`. **Zero non-doc lines changed**, and
`GSD_MM_ENVELOPE_ROOT`'s value and behaviour are unaltered.

### The mechanism pins, verified non-dead

Rounds 4 through 10 are all still load-bearing and all still right about the
lines round 11 refuses:

```
exit=2 [envelope_assertion_failed]  git pus? --force origin main                     <- round 5
exit=2 [force_push_blocked]         git >/dev/null push --force origin main          <- round 6
exit=0                              git x2>/tmp/o push --force origin main           <- round 6 control
exit=2 [force_push_blocked]         git 2>/dev/null push --force origin main
exit=2 [envelope_assertion_failed]  git >                                            <- unresolvable
exit=2 [force_push_blocked]         git <<EOF push --force origin main               <- heredoc
exit=2 [force_push_blocked]         git --attr-source HEAD push --force origin main  <- round 7
exit=2 [envelope_assertion_failed]  git --bogus-opt status                           <- round 7 inversion
exit=0                              git - push --force origin main                   <- round 7 control
exit=2 [force_push_blocked]         git -- push --force origin main
exit=2 [envelope_assertion_failed]  git -c include.path=/tmp/evil.cfg status         <- round 8
exit=2 [hook_bypass_blocked]        env -u GIT_CONFIG_COUNT git push --force origin main
exit=2 [envelope_assertion_failed]  V=push; git $V --force origin main               <- round 4 Rule A
exit=2 [envelope_assertion_failed]  git {push,--force} origin main                   <- round 5 brace
exit=0                              git -c alias.p='!git push …' p                   <- T-19-86, unmoved
exit=2 [envelope_assertion_failed]  : > <ENV>/alpha/pr-ledger.ndjson                 <- round 11
exit=0                              : > /tmp/l                                       <- round 11 control
```

`SEPARATORS` byte-identical at `policy.rs:2297` with ONE commit in the whole phase
(`84a9b05`); `is_separator(">") == false` by construction. **`envelope_config_resolution`
is 30/0 — not one config-resolution verdict moved.**

### Gates observed

`rtk proxy cargo test --no-fail-fast` redirected to a file and counted with
**`rtk proxy grep`** (D-34): **1820 passed, 0 failed, 13 ignored** over **46**
result lines, `passed + failed = 1820`, matching `19-29-SUMMARY.md` exactly. **All
SEVENTEEN `envelope_*` binaries ran**, so round 11's evidence file executed.
**No documented flake fired in audit 11's run** — neither `driver_reattach` row,
not the `envelope_tracer` `ExecutableFileBusy`, and not the ETXTBSY write-over-the-binary
race. `19-29` recorded two firings (both `driver_reattach`, green in isolation);
audit 11 records none. **Absence is not evidence any of them is fixed**, and the
ETXTBSY race is `C-10`'s own seam, which this round changed what the guard does
about — audit 11 claims its relation to this round's change in **neither
direction**, on the same discipline `19-29` used. `cargo clippy --tests -D
warnings` was already failing at the base on four pre-existing lints in
`src/browser.rs` and `src/project_creator.rs` and is out of scope.

**Provenance discipline.** Plans 19-13 … 19-29's appended subsections, the
plan-19-21 blocker-row resolution record and every earlier audit's own tables are
left **byte-identical**; audit 11 checksummed the whole body below the frontmatter
before writing (`sha256 8f65ad89f70afb72d2…`, 821,703 bytes) and this write is
append-only apart from the frontmatter, one trail row and one method subsection.
Audit 11's corrections to statements made in those subsections — that SEVEN is
not the count of the rule's silences, and that the `-c credential.helper=` residue
is argv-VISIBLE but not GOVERNED — are recorded as audit-11 findings BESIDE them
rather than as edits to them.

### Audit 11's bookkeeping, re-derived from audit 10's group rows

| Group | Count | Closed | Open |
|---|---|---|---|
| Everything through audit 10 | 130 | 103 | 27 (6 at `high`) |
| Closed by plans 19-28 / 19-29, re-measured by audit 11 (`T-19-117`, `T-19-118`, each as scoped) | — | +2 | −2 (1 at `high`) |
| Found by audit 11 (`T-19-119` … `T-19-121`) | 3 | 0 | 3 (2 at `high`) |
| **Total after audit 11** | **133** | **105** | **28 (7 at `high`)** |

The seven that count toward `threats_open`: `T-19-86`, `T-19-91`, `T-19-111`,
`T-19-112`, `T-19-116`, `T-19-119`, `T-19-121`. The twenty-one that do not:
`T-19-61` … `T-19-73` (13), `T-19-84`, `T-19-85`, `T-19-96`, `T-19-105`,
`T-19-110`, `T-19-113`, `T-19-115`, `T-19-120`.

---

## Audit 11 — what the round-11 control can and cannot fail on

### The principle rounds 3 through 10 established still holds

A decision region must come from the same scan the classifier runs, and there is
one walk. The redirection targets are accumulated **in the operator arm of the ONE
walk**, carried on the `Segment` exactly as `Segment::redirection_unresolvable`
already travels, and the clause is still raised **once**, at the top of
`classify_segments`' existing per-segment loop, reading two fields of the segment
that loop already holds. **No second scan, no second reading site, no new
`ParkReason`, and `segment.tokens` byte-for-byte unchanged.** `resolve_program`,
`resolve_program_with_head`, `first_unreadable_decision_word`,
`config_key_operand_index`, `subcommand_word_indices`, `scan_gh_api`,
`scan_leading`, `forbidden_repo_path`, `SEPARATORS` and `is_separator` show zero
diff lines. The plan projected two additive fields and one was enough, which is
strictly less surface and was recorded rather than glossed.

### Where the boundary now is, in one paragraph

**The rule decides on one lexical fact in two word classes over two paths: a word
that is ABSOLUTE, LITERAL and either under `envelope_dir_in(root, alias)` (a
PREFIX) or exactly `current_exe()` (an EQUALITY).** The two boundary kinds differ
for a measured reason and both near-miss controls prove it. Each of those
conditions is also a place the rule is silent, and its silence is a permit. **Round
11 names seven silences and states them as a CONDITION rather than a list, which
is the right form.** But the condition it states — *a word the shell may rewrite,
a word that is not absolute, a word that reaches a protected path only through a
link* — **omits the case where the word is none of those three and the path inside
it is absolute anyway.** `lexical_absolute_components` asks whether the WORD begins
with `/`, and an option-attached value never does. That is `T-19-119`, it reaches
all ten ruled carriers, and it is where the plane is still unfinished:

* **ten carriers now have a rule** — `C-01` … `C-10`, verified refused at their
  bare absolute spellings;
* **five have control (e)**, no rule, registered, unaccepted — `C-11` … `C-15`;
* **all ten of the ruled ones are reachable by an option-attached spelling**, and
  two of them were driven to harm — a bare remote's `main` moved, and a fired PR
  cap reset;
* **one control's own bound is over the wrong property** — the ledger's SIZE
  rather than its KIND (`T-19-120`);
* **one route to the ambient credential never needed a carrier at all** — a `-c`
  on the command line (`T-19-121`).

**The one-sentence version for whoever picks this up.** Eleven rounds have
modelled how a command line becomes behaviour and how a file becomes a control,
and audit 10 was right that there is no further plane — **what is left is that
every path rule this phase has written asks whether a WORD is a path, and a word
can carry a path without being one.**

### Suggested closure, in order — (d) FIRST, for the eleventh round running

1. **(d) — widen the corpus BEFORE certifying anything, and this time widen the
   WORD SHAPE rather than the spelling.** Add option-attached carrier paths
   (`of=<path>`, `--target-directory=<path>`, `--directory=<path>`) to
   `ENVELOPE_ROOT_OPERAND_CARRIERS`, to the binary class and to the redirection
   alphabets, with a floor beside `MIN_CONTROL_CARRIER_CLASSES` asserting the axis
   can draw one. **Listed first for the eleventh round running**, and this time
   the missing draw is one character wide.
2. **(a) — `T-19-119` FIRST among the fixes, and it is the cheapest real fix this
   phase has had in five rounds.** The whole gap is `lexical_absolute_components`
   requiring the WORD to start with `/`. Splitting an operand at its first `=` and
   testing the tail — a purely lexical operation needing no environment, no
   filesystem and no program-name list — reaches every one of the measured rows.
   The over-refusal cost must be measured from both sides before it lands
   (`FOO=/tmp/x cmd` is an assignment prefix, not an operand, and must stay
   permitted).
3. **(b) — `T-19-121`.** `credential.helper` belongs beside `core.hooksPath` in the
   by-name deny and beside `include.path` in round 8's confinement clause; it is a
   config key, on argv, in a region the guard already parses. Either that, or the
   residue's own sentence must stop saying *governed*.
4. **(c) — `T-19-120`.** Bound the ledger by KIND as well as size: a
   `metadata.file_type().is_file()` beside the length check, failing closed on
   anything else, in the same `stat` that is already there.
5. **(e)** — then `T-19-86`, `T-19-91`, `T-19-96`, `T-19-110`, `T-19-111`,
   `T-19-112` and `T-19-116`, which remain arms of two shapes: a classifier arm
   answering `Allow` on an operand outside the decision region, and a carrier that
   is not a command line.

Then, and separately: add the `AR-19-13` row for `T-19-17r` or stop calling it
accepted; measure `C-08`'s behavioural half and `T-19-117`'s behavioural half in a
live agent session; and correct the residue's arithmetic from seven to eight —
**or, better, stop counting and state the condition the code actually tests.**

---

## Audit 11 Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log — twelve, unchanged by
      round 11; **audit 11 accepts nothing and un-accepts nothing**
- [x] Every closure re-measured against the built binary at `1e56389` with a
      fresh envelope root per row and the root walked afterwards, the walk proved
      non-blind on every pass by `gh pr create --title x` leaving exactly one
      `alpha/pr-ledger.ndjson` line
- [x] **The plane IS finished and audit 11 says so plainly** — no twelfth plane
      exists; every remaining finding is on round 10's own plane
- [x] **Round 5 and round 6 are INTACT** — `SEPARATORS` byte-identical
      (`policy.rs:2297`, ONE commit in the phase, `84a9b05`), `is_separator(">")`
      `false` by construction, `segment.tokens` byte-identical, the SEGMENT-COUNT
      pins green **and verified falsifiable in BOTH the SPLIT and the DISPLACED
      variant**, and the over-deletion control still permitting
- [x] The redirection channel swept over all seven pathname operators, both quote
      forms, both segment positions and the `NestedPayload` recursion; the five
      non-pathname operators correctly NOT recorded, so no over-refusal was bought
- [x] `T-19-118` **CLOSED AS SCOPED** — the empty-`credential.helper` control
      re-driven against real git with `git credential fill` as the criterion, the
      `--get-all` false negative reproduced, the **URL-SCOPED** cell probed and
      held, `GIT_ASKPASS` and `gh` unaffected, and **eight env spellings that would
      strip the pair all exit 2**. Successor `T-19-121` registered
- [x] `T-19-117` **CLOSED AS SCOPED** — the bound discriminates exactly at
      `MAX_LEDGER_BYTES` (8,388,608 permits and counts; 8,388,609 refuses), 202 MB
      answers in **18 ms** against `19-28`'s 7,900 ms, the over-refusal is bounded
      to the forge commands measured from both sides, and it **cannot be turned
      into a bypass**. Successor `T-19-120` registered
- [x] The four execution-time judgements assessed independently; **all four are
      correct**, and judgement 4's characterisation verified mechanically — **ZERO
      deleted assertions, ZERO removed `#[test]`, 12 added**
- [x] `C-10` verified to have a threat id, a severity, a `deferred-items.md` row
      and a control letter; `SECTION_ENVELOPE`'s first `Guaranteed` clause
      verified TRUE at **211 tokens of an UNRAISED 215 cap**, widest line 74 of 80;
      `mod.rs:165-168` verified CORRECTED with zero non-doc lines changed
- [x] Every new finding confirmed against the **REAL `git` binary** with a CONTROL
      beside every leg — including an option-attached write that replaced the
      guard's own binary and let a force push the hook had refused **MOVE a bare
      remote's `main` (`835b6be` → `40ad7f7`)**, restored to a refusal by putting
      the binary back; a fired PR cap reset by the same spelling against its
      space-separated twin at exit 2; and a `-c credential.helper=store` that
      returned the ambient `~/.git-credentials` secret on a permitted line
- [x] Gate observed: `rtk proxy cargo test --no-fail-fast` → **1820 passed, 0
      failed, 13 ignored** over 46 result lines; **all SEVENTEEN** `envelope_*`
      binaries ran; **no documented flake fired**
- [x] Plans 19-13 … 19-29's appended subsections left byte-identical, verified by
      checksum before writing
- [ ] `threats_open: 0` confirmed — **7 open at `high`: `T-19-86`, `T-19-91`,
      `T-19-111`, `T-19-112`, `T-19-116`, `T-19-119`, `T-19-121`**
- [ ] `status: verified` set in frontmatter

**Approval: blocked, 2026-09-04 (audit 11).**

**Not accepted here.** `T-19-119` moved a bare remote's `main` and reset a fired
PR cap with one permitted command, and it reaches every carrier this phase has
written a rule for. `T-19-121` puts the user's own git credential back inside a
driven run with no file write at all. `T-19-86` and `T-19-91` remain open at
`high` by scoping decision and by round discipline. `T-19-111` remains open with
no rule, `T-19-112` with a partial one, and `T-19-116` is narrowed rather than
closed because its declared harm is still reachable. Accepting any of the seven is
a human decision and this audit does not make it. **`T-19-17r` is left unmade for
the thirteenth time, and `AR-19-04`, `AR-19-05` and `T-19-23` are untouched.**

**Round 11 did good work and audit 11 says so plainly, because that is what the
measurement supports.** `19-28` widened the axis from seven classes to eleven,
refined three predicates rather than two, moved an alphabet into a fail-closed arm
because its verdict the fix would change, found `19-29`'s blocking scope conflict
by reading, and left six rows RED with **zero `src/` hunks**. `19-29` confirmed
every one still red before a production line moved, answered the direction-(i)
question with a channel that reads redirection targets **without** making `>` a
separator or pushing a token into the stream — the thing five rounds of pins
existed to prevent, and the pins are still green — reported two unauthorised row
corrections as findings rather than absorbing them into a human's permission,
reported a success criterion it did not meet rather than claiming it, and shipped
a mechanism control that is genuinely wider than any path rule because it reads no
command line at all. **Two threats close on this audit's measurement, and both
closures are real.**

**And the answer to the eleventh question is the plain one: the plane IS
finished.** Audit 10 was right — there is no twelfth layer, and audit 11 swept for
one and found nothing. **What audit 11 found is the same shape this phase has
produced eleven times running, one cell over from the one the round just filled.**
Round 10 wrote a rule that asks whether a word is a path under a directory. Round
11 widened it to a second word class and a second path and corrected its residue
from four silences to seven. **Neither round asked what happens when the path is
not the word but is inside it.** The corpus that certifies both draws sixty-one
carrier paths across eleven classes and not one of them carries a path after an
`=` — so the axis was, for the eleventh consecutive round, structurally incapable
of failing on the class the round certified. **The residues are not true as
stated. They are one character short.**

---

## Execution record — plan 19-30 (the corpus, round 12). NOT an audit finding.

**This subsection is a record made by plan 19-30's executor.** It is appended
after the plan-19-29 record and edits nothing that precedes it — no audit table,
no Security Audit Trail row, no Accepted-Risks-Log row, no Sign-Off, and no
earlier appended subsection. **In particular the correction to `19-28`'s and
`19-29`'s classification of the `=`-joined spelling is written BESIDE, here, and
never as an edit to theirs** — the discipline audit 11 used for its own two
corrections to those same subsections.

### FIRST: this plan closes nothing, and `/gsd-secure-phase 19` is NOT cleared

**`T-19-86`, `T-19-91`, `T-19-111`, `T-19-112`, `T-19-116`, `T-19-119` and
`T-19-121` all remain OPEN at `high`.** `T-19-120` is open at `medium`. This plan
wrote the corpus and the reproducers and STOPPED; `19-31` writes the rules and
the honesty repairs. **Whether `19-31` closes any of them is audit 12's judgement
rather than either plan's claim, and neither plan clears `/gsd-secure-phase 19`.**

The base was confirmed before the first measurement:
`git diff --numstat 1e56389..b6eaafa -- src/ tests/` is **EMPTY**, and
`git diff b6eaafa..77b3b90` touches only `.planning/`, so every number below was
measured against the tree audit 11 measured.

### Audit 11's framing is adopted whole

**The plane is finished.** Audit 10 said the modelled surface was complete;
audit 11 swept the cells adjacent to all six named axes and found no twelfth
plane; and it verified that round 11's three new mechanisms create no new
reachable state. `T-19-117` and `T-19-118` both closed on measurement. **This
round therefore fixes the cell one character over rather than looking for a new
plane.** What is left is that **every path rule this phase has written asks
whether a WORD is a path, and a word can carry a path without being one.**

### THE DESIGN ANSWER — where the option-attachment boundary is

`lexical_absolute_components` (`policy.rs:5457-5476`) reads a word and asks
whether it starts with `/`. The obvious fix is to split at the first `=` and test
the tail. **That fix is one character short in exactly the way the current one
is.**

| Candidate boundary | Reaches | Misses |
|---|---|---|
| split at the FIRST `=` | `of=/p`, `--git-dir=/p` | `--opt=a=/p` (a second `=`); `-C/p`, `-t/p` (ATTACHED SHORT OPTIONS, no `=` at all); `host:/p` (a `:`) |
| split at EVERY `=` | the above plus `--opt=a=/p` | the attached short options and the `:` |
| a LIST of attachment characters `= : , @` | most spellings | whatever character the next program uses — **and it is a PROGRAM-GRAMMAR ENUMERATION, which is D-08's defect one level over** |
| **`/`-ANCHORED SUBSTRINGS of a LITERAL word** | **all of the above, every attachment character that exists, and none** | only what the four existing conditions already exclude |

**THE MANDATED BOUNDARY, STATED EXACTLY.** For a word `w` that passes the
existing `Token.literal` filter, every byte index `i` with
`w.as_bytes()[i] == b'/'` yields a candidate `&w[i..]`. Each candidate goes
through the EXISTING `lexical_absolute_components` normaliser and the EXISTING
two comparisons — `word_is_within` (component-wise PREFIX) and `word_is_exactly`
(EQUALITY). **The rule never asks what precedes the `/`**, which is what stops
the boundary being one character short a thirteenth time.

**CONTAINMENT, STATED PRECISELY — because this phase has been bitten by a claim
that was true in spirit.** It is NOT *"`i == 0` is today's rule"*:
`lexical_absolute_components` opens with `word.trim_start_matches("./")` **before**
it tests `starts_with('/')`, so `.//abs/p` is accepted today and `./abs` is not.
The exact statement is that **for every word today's rule answers `Some(v)` for,
the string it actually NORMALISES — `w` with its leading `./`s stripped — begins
with `/`, is therefore itself one of the candidates, and re-normalises to exactly
`v`.** The candidate set contains today's answer BYTE FOR BYTE; the `./` strip can
only ADD answers (`./abs` is `None` today and yields `[abs]`) and can never remove
one. **No refusal that exists today can be lost, and the reason is containment of
the NORMALISED STRING rather than of the index.** Re-derived mechanically in
`containment_holds_the_candidate_set_carries_todays_answer_byte_for_byte`.

**A path can therefore appear after MORE THAN ONE attachment character and after
NONE**, and both are reached: `--opt=a=/p` and `-C/p` yield the same candidate as
`of=/p` does.

### THE COST, PINNED FROM BOTH SIDES

The cost is bounded by the **PATH SET**, not by the split. A candidate is refused
only if it PREFIXES a directory of at least three components rooted at
`<root>/<alias>`, or EQUALS a full binary path; `word_is_within` returns early on
`dir.is_empty() || word.len() < dir.len()` over COMPONENT VECTORS, so a short
suffix cannot match a deep directory at all.

```text
--author=A <a@b.c>                 no `/` in the word              -> no candidate
--format=%H                        no `/`                          -> no candidate
sed s/x/y/                         [x, y], [y], []                 -> shorter than any envelope dir
https://github.com/o/r             [github.com, o, r], [o, r], [r] -> wrong FIRST component
HEAD:refs/heads/gsd-auto/alpha/w   [heads, gsd-auto, alpha, w], …  -> wrong FIRST component
git --git-dir=/tmp/g status        [tmp, g]                        -> not under <root>/<alias>
FOO=/tmp/x cmd                     [tmp, x]                        -> not protected: PERMITTED
rg pr-ledger.ndjson src/           `src/` -> the EMPTY component list
```

Every one of these was measured exit 0 and is **asserted exit 0 AFTER**.
`git --git-dir=/tmp/g status` alone is pinned PERMITTED in FOUR files
(`envelope_callee_grammar.rs:1025`, `envelope_reparsed_value.rs:1075`,
`envelope_config_resolution.rs:1169`, `envelope_wrapper_class.rs:5133`) and named
in FIVE `policy.rs` doc sites, **so a rule that turned it red would turn five
files red at once.**

### THE NEW OVER-REFUSAL SURFACE, IN ITS GENERAL FORM

**Any word whose text contains this run's own envelope directory or its own
binary path as a `/`-anchored substring is refused after `19-31` even where the
program would not have used that substring as a path** — as a value, a pattern, a
message, a URL fragment or a relative path that happens to contain it. It fails
CLOSED, it is bounded to words naming this run's own directory or binary, and it
is the same family as the already-disclosed refusal of `cat <ledger>`.

**Three instances were driven and RECORDED at both verdicts, never asserted:**

| instance | today | derived after `19-31` |
|---|---|---|
| a RELATIVE word carrying it — `rm -f ./tmp/<root>/alpha/pr-ledger.ndjson` | exit 0 | exit 2 |
| a NON-PATH VALUE carrying it — `git log --format=./tmp/<root>/alpha` | exit 0 | exit 2 |
| a URL-shaped operand carrying it after `#` | exit 0 | exit 2 |
| **the same URL with `?` instead of `#`** | exit 0 | **exit 0, UNCHANGED** |

**The last row is a finding worth its own line: the `?` spelling is a
pathname-expansion metacharacter, so `Token.literal` is FALSE and the interior
scan never runs on the word.** Two URLs that look identical get different
answers, which is why the surface is stated over what the predicate READS rather
than over what a URL looks like.

**The relative instance's reach was measured under real `bash`**: from a working
directory that is not `/`, the relative word deleted a DECOY under `$PWD` and
left the real ledger untouched — which is precisely why the refusal is an
over-refusal rather than a mitigation.

### THE RESTATED RESIDUE CONDITION, WITH NO COUNT

On audit 11's own recommendation — *"stop counting and state the condition the
code actually tests"*:

> **This predicate reads every `/`-ANCHORED SUBSTRING of every LITERAL word, in
> either word class, against both paths. It is silent about a word the SHELL MAY
> REWRITE, because the guard cannot know its final text; about a word whose text
> carries NO ABSOLUTE PATH ANYWHERE, because there is nothing in it to normalise;
> and about a word that reaches a protected path ONLY THROUGH A LINK, because no
> lexical reading of the text names it. The spellings below are INSTANCES and are
> not counted.**

**The one-character correction is that *"a word that IS NOT ABSOLUTE"* becomes *"a
word whose text carries no absolute path anywhere in it."*** Round 5's literalness
classes (expansion, pathname, tilde, brace) are instances of the first clause;
relative and interior-relative words of the second; symlinks of the third.
**None of it is handed to a pin, a schedule or a version witness** — `19-25`'s
witness schedules a re-audit against a FUTURE GIT and every one of these is
reachable by the driven agent TODAY.

### `T-19-119` DRIVEN END TO END, TWICE, WITH A CONTROL BESIDE EVERY LEG

The bare-remote fixture was **REBUILT**, not cited. `pre-push` and `pre-commit`
in `stub_body`'s exact three-line shape, delivered through the
`GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_0` / `GIT_CONFIG_VALUE_0` triplet naming
`core.hooksPath`.

```text
CONTROL  real binary                     push --force -> REFUSED,   main 0bd5d44 UNMOVED
GUARD    cp /bin/true <BINARY>                        -> exit 2
GUARD    dd if=/bin/true of=<BINARY>                  -> exit 0     <- the whole finding, two lines
LEG B    binary replaced through `of=`   push --force -> COMPLETED, main 0bd5d44 -> 6efde46 MOVED
LAYER 2  <replaced> envelope guard alpha              -> exit 0     (the real binary answers exit 2)
CONTROL  binary restored, remote rewound push --force -> REFUSED,   main 0bd5d44 UNMOVED
```

And the LEDGER, re-driven from a FIRED cap in **ONE persistent root**:

```text
call 1  gh pr create --title x  -> exit 0 (permit),        1 ledger line
call 2  gh pr create --title x  -> exit 2 pr_cap_exceeded, 2 lines
call 3  gh pr create --title x  -> exit 2 pr_cap_exceeded, 3 lines
call 4  gh pr create --title x  -> exit 2 pr_cap_exceeded, 4 lines (412 bytes)
GUARD   cp /dev/null <ledger>                  -> exit 2  <- the TWIN
GUARD   dd if=/dev/null of=<ledger>            -> exit 0
        (the permitted spelling performed)     -> ledger truncated to 0 bytes
call 5  gh pr create --title x  -> exit 0 PERMITTED AGAIN, 1 line
```

### THE ATTACHMENT SWEEP — the measurement that DECIDES where the boundary is

Every row driven at the guard AND probed under real `bash`. **A spelling the
guard permits but the shell does not reach is not a bypass, and the reach leg is
what tells them apart.**

| spelling | guard | space-separated twin | real `bash` reach |
|---|---|---|---|
| `dd if=/dev/null of=<ENV>/alpha/pr-ledger.ndjson` | exit 0 | `cp /dev/null <ledger>` exit 2 | REACHES (file truncated) |
| `tar --directory=<ENV>/alpha …` | exit 0 | `tar --directory <ENV>/alpha …` exit 2 | REACHES (`tar -tf` lists the dir) |
| `cp --target-directory=<ENV>/alpha /bin/true` | exit 0 | exit 2 | REACHES (`true` appeared) |
| `rsync --temp-dir=<ENV>/alpha …` | exit 0 | exit 2 | REACHES (accepted and used) |
| `chmod --reference=<ENV>/alpha/pr-ledger.ndjson F` | exit 0 | exit 2 | REACHES (F took 0644) — **a READ** |
| `git --git-dir=<ENV>/alpha …` | exit 0 / `force_push_blocked` | exit 2 `envelope_assertion_failed` | REACHES (git names the dir back) |
| **`tar -C<ENV>/alpha …`** — SHORT option, **NO `=`** | exit 0 | exit 2 | REACHES |
| **`cp -t<ENV>/alpha /bin/true`** — the same | exit 0 | exit 2 | REACHES |
| **`PATH=/usr/bin:<ENV>/alpha mytool`** — a `:` | exit 0 | — | REACHES (bash EXECS out of it) |
| `GSD_MM_ENVELOPE_ROOT=<ENV>/alpha gh pr create` | exit 0 | — | value delivered |
| `dd if=/bin/true of=<BINARY>` | exit 0 | `cp /bin/true <BINARY>` exit 2 | REACHES |
| `tar --directory=<BINARY> …` | exit 0 | — | REACHES |
| `python3 --out=<ENV>/alpha/x` | exit 0 | — | **DOES NOT** — `unknown option --out=…` |
| `mytool --opt=a=<ENV>/alpha/…` — TWO `=` | exit 0 | — | **DOES NOT** — no such program grammar found |
| `ssh -oControlPath=<ENV>/alpha/s host` | exit 0 | — | **DOES NOT** — connection never opened |
| `rsync host:<ENV>/alpha /tmp/x` | exit 0 | — | **DOES NOT** — no remote |

The four unreached spellings are RECORDED, not asserted.

**The two genuinely-reaching TWO-`=` spellings that WERE found are both already
refused today by OTHER clauses**, so neither discriminates on this axis and both
are recorded with their overlap stated:
`GIT_CONFIG_PARAMETERS='core.excludesFile=<ENV>/alpha/excl' git status` is exit 2
`hook_bypass_blocked` (and so is its unprotected control, at the same
identifier), and `git -c alias.q=--git-dir=<ENV>/alpha q` is exit 2
`envelope_assertion_failed` from round 9's re-parse clause — and real git refuses
to run the latter at all (`alias 'q' changes environment variables`), so no reach
is claimed for it either.

### WHAT CLOSING `T-19-119` DOES — AND DOES NOT DO — FOR `T-19-116`

**Does:** every option-attached spelling of the binary path becomes refused,
including the exact one that moved a bare remote's `main`.

**Does not:** `T-19-116`'s declared harm — one permitted command removing layer 2
AND layer 3 — stays reachable, and all four residues were re-measured at exit 0:

```text
exit=0  cp /bin/true $(command -v gsd-meta-manager)          <- expansion-borne
exit=0  cp /bin/true ~/.cargo/bin/gsd-meta-manager           <- tilde
exit=0  cd <binary-parent> && cp /bin/true gsd-meta-manager  <- relative
        a PATH symlink whose target current_exe() reports    <- link, neither half covers it
```

**None of the four is an option attachment. All four are the OTHER silences, no
rule is written for any of them this round, and `T-19-116` stays OPEN at
`high`.**

### THE PROCESS FINDING — written BESIDE, never as an edit

**`19-28` observed this and classified it wrong, and that is how it survived a
round that had already found it.** `19-28-SUMMARY.md:381-383` records that
`git --git-dir=<ENV>/alpha push --force origin main` answers `force_push_blocked`
rather than `envelope_assertion_failed` *"because the token does not begin with
`/` and `lexical_absolute_components` rejects it"* — **the mechanism, named
exactly right** — and `19-29-SUMMARY.md:641-644` carried it forward as *"Not a
defect; recorded because its space-separated twin answers differently."*

**The OBSERVATION was correct and the CLASSIFICATION was wrong.** An operand whose
verdict changes when you delete one space is not a curiosity about identifiers; it
is a word class the rule cannot see. **An observation you cannot derive is
recorded as UNRESOLVED, not classified.** Neither `19-28`'s nor `19-29`'s
subsection is edited to say so.

### `T-19-121` — the KEY-SHAPE SPACE, and `cred.rs:420-425` recorded FALSE

The claim, quoted VERBATIM from `cred.rs:416-425`:

> * **a later `-c credential.helper=<something>` on the same command line, which
>   OVERRIDES the reset and brings the secret back.** That is a BOUNDED residue
>   rather than a reason to decline the control, and the bound is stated rather
>   than assumed: **that spelling is ARGV-VISIBLE and is already governed** by
>   [`super::policy::scan_leading`]'s leading-option region and layer 2's whole
>   grammar — unlike every write spelling, which is not.

**ARGV-VISIBLE it is. GOVERNED it is not.** The by-name deny covers
`core.hooksPath` (measured `hook_bypass_blocked`), round 8's confinement clause
covers `include.path` (measured `envelope_assertion_failed`), and **nothing
covers `credential.helper`** — measured exit 0 on four separate surfaces.
**This is `T-19-84`/`T-19-107`/`T-19-109`/`T-19-115`'s shape a SIXTH time**, and
this phase has already shipped a COUNTED completeness claim ("FIVE forms") that
was wrong the day it was written. **The correction is required whether or not any
rule lands.**

Measured against real git under the envelope's full posture, with `19-29`'s
injected empty-`credential.helper` pair present and **the no-`-c` CONTROL driven
FIRST**:

```text
                                                   real git             guard
CONTROL  no `-c` at all                            exit 128, ABSENT     —
-c credential.helper=store                         secret PRESENT       exit 0
-c CREDENTIAL.HELPER=store                         secret PRESENT       exit 0
-c Credential.Helper=store                         secret PRESENT       exit 0
-c credential.https://github.com.helper=store      secret PRESENT       exit 0
-c credentialx.helper=store                        secret ABSENT        exit 0
-c notcredential.helper=store                      secret ABSENT        exit 0
-c credential.helperx=store                        secret ABSENT        exit 0
--config-env=credential.helper=<VAR>               secret PRESENT       exit 0
GIT_CONFIG_PARAMETERS="'credential.helper=store'"  secret PRESENT       exit 2 hook_bypass_blocked
```

The secret is recorded PRESENT/ABSENT and never transcribed (SAFE-04's own
reasoning). **The URL-SCOPED key is what a by-name equality on
`credential.helper` would MISS**, and the three near misses are what it must NOT
catch — git folds the config SECTION and the VARIABLE to lower case and leaves
the SUBSECTION case-sensitive. **`19-31` can therefore state a by-name clause's
partiality at the same weight rather than discover it.** No rule is asserted
anywhere; the four guard rows are RECORDED, because `19-31`'s clause is SEVERABLE
and an assertion in either direction lands permanently red in a file `19-31` may
only ADD to.

**MEASURED DIFFERENTLY FROM THE PLAN'S OWN EXPECTATION, and recorded as
measured:** the plan's RECORDED-rows list says `T-19-104`'s carrier
`GIT_CONFIG_PARAMETERS="'credential.helper=store'"` is *"open at exit 0"*. **At
the guard it is exit 2 `hook_bypass_blocked`** — the key is in
`ENVELOPE_ENV_KEYS`. Real git does resolve the helper from it, so the reach is
real and the guard already refuses the carrier.

### `T-19-120` — a bound over SIZE where the failure is KIND

```text
FIFO ledger                                 gh pr create --title x -> exit 124 after 20.02 s
8 366 000-byte regular ledger (just under)                         -> exit 0 in 0.42 s, still COUNTING
FRESH root, no ledger at all                                       -> exit 0 in 0.01 s, exactly 1 line
```

A FIFO **stats at length 0**, passes a bound written over SIZE, and the
`std::fs::read` below it blocks forever. The comment beside the check states the
fail-open and its reason — *"A file that cannot be stat'd is not a file this check
can refuse on"* — and that reasoning is SOUND for a stat that FAILS. **The FIFO
case is a stat that SUCCEEDS and answers 0.**

**The correction belongs INSIDE THE SAME `Ok` ARM and must not touch the `Err`
arm.** A fresh envelope root has no ledger at all and takes the `Err`
fall-through; **a kind check placed outside the `Ok` arm would refuse every first
forge call in every fresh root.** The fresh-root row is asserted exit 0 BEFORE AND
AFTER precisely so that mistake turns one evidence file red instead of the suite.

Reachability, with the absolute-literal twin beside it: `mkfifo <ENV>/alpha/…` is
exit 2 today, while the tilde, glob, brace and expansion-borne spellings are all
exit 0 — four of the declared silences.

**The BEHAVIOURAL half — what the agent CLI does with a `PreToolUse` hook that
never returns — is UNMEASURED, is a property of a closed-source binary, and is
claimed in NEITHER direction**, exactly as `C-08`'s behavioural half is.

### `policy.rs:5743`'s own example, recorded FALSE

The disclosed-cost paragraph argues the read over-refusal is unavoidable because
*"is `dd if=X of=Y` a read of `X` or a write of `Y`?"*. **Measured:
`dd if=<ENV>/alpha/pr-ledger.ndjson of=/tmp/stolen` is exit 0**, so the guard
refused it NEITHER way and the example illustrated nothing at the moment it was
written. **Both repairs are recorded and the correction is required regardless:**
if the fix lands the sentence becomes TRUE and needs a WR-02 note saying it was
false when written; if the fix is severed it must be replaced by an example the
guard actually refuses — `tee F`, already beside it.

### THE FENCED-FILE ENUMERATION, RE-DERIVED MECHANICALLY — the answer is ZERO

```text
grep -rnE '[^ "]<ENV>|[^ "]<BIN>|[^ "]<ENVX>|[^ "]<BINPAR>' tests/
  5 hits, EVERY ONE a `//` or `///` DOC line:
    envelope_wrapper_class.rs:7976, :8556, :8558
    envelope_carrier_reach.rs:1154
    envelope_control_carrier.rs:753

grep -rnE '[^ "(]\{\}/|[^ "(]\{root|[^ "(]\{binary' tests/
  8 hits: five `refs/heads/gsd-auto/{}/…` refspecs (candidates
  [heads, gsd-auto, <alias>, …] — WRONG FIRST COMPONENT), one `/proc/{}/stat`,
  and envelope_config_resolution.rs:1825/:1832's
  `includeIf.gitdir:{repo}/.path={evil}` where `{repo}` is a TEMP GIT REPO and
  the row is a REAL-GIT probe, not a guard row.

grep -rnoE '=/[^ ")]*' tests/ src/envelope/
  Every `=`-attached absolute path in the whole corpus is one of
  /tmp/evil.cfg, /dev/null, /tmp/nohooks, /tmp/g, /tmp/e, /tmp/x, /tmp/s,
  /tmp/fresh, /tmp/nowhere, /tmp/evil, /x, /ENV_WINS, /CLI_WINS, /PARAM_WINS,
  /ALIAS_WINS, /INCLUDE_WINS, /PARAM_INCLUDE_WINS, /SHOULD_NOT_WIN,
  /nonexistent-hooks-dir.
  NOT ONE names an envelope root and NOT ONE equals a binary.
```

**ZERO assertions move.** Every `policy.rs` unit pin was checked individually and
re-derived mechanically against the mandated design, including the two that look
most exposed: `word_is_within("/tmp/envroot/alpha/../other/x")` stays `false`
because `..` collapses PER CANDIDATE, and `word_is_within("./alpha/pr-ledger.ndjson")`
stays `false` because `[alpha, pr-ledger.ndjson]` is two components against three.

**ONE ARTEFACT MOVES AND IT IS A COMMENT, NOT AN ASSERTION.** `policy.rs:9719`'s
reason for the `./alpha/pr-ledger.ndjson` row — *"relative again, with the leading
`./` the normaliser strips: stripping it must not turn a relative word into an
absolute one"* — goes STALE, because under the new rule a relative word DOES yield
absolute candidates while that row still answers `false` for a different reason.
**That is `19-31`'s WR-02 correction. It is named here in advance and was not
edited.**

**One further planning-time claim corrected by measurement:** the plan says
`GSD_MM_ENVELOPE_ROOT=/tmp/fresh gh pr create --title x` is *"pinned PERMITTED at
`tests/envelope_control_carrier.rs:1106`"*. It is **RECORDED there, not asserted**
(`record_only`, `:1105-1107`, `E-01`'s inert-prefix row). This plan's own evidence
file ASSERTS it, because after `19-31` its verdict is load-bearing.

### THE AXIS — eleven classes to TWELVE, with the complement extended

* Class 12 decides on a `/`-anchored substring at a **NON-ZERO index** and on the
  carrier paths — **never on an attachment character and never on a program
  name**. It excludes `$`, a leading `~`, a pathname metacharacter and a brace
  list, which keeps it disjoint from classes 3, 8, 9 and 10; it is disjoint from
  class 1 by construction (that is index zero) and from class 11 by construction
  (that is an equality over the whole word).
* **`draws_an_ordinary_operand` — class 7, the COMPLEMENT — is EXTENDED to
  exclude class 12.** Without it every interior-path entry lands in the
  INVARIANCE arm and the axis asserts PERMITTED a row `19-31` refuses. This is
  `19-18`'s `{v}>` blocker, `19-20`'s split, `19-22`'s, `19-24`'s and `19-28`'s —
  **a SIXTH time, and the SECOND through the complement specifically.**
* `draws_a_relative_carrier` — class 5 — refined to exclude an interior carrier
  path, for the same reason round 11 refined it for the tilde.
* NEW fail-closed `CONTROL_CARRIER_INTERIOR_PATH`, **12 entries**, drawn only
  from measured rows, with at least one under a program absent from both
  production halves and **three attached by something other than `=`**. Class 12
  is deliberately NOT drawn in redirection-target position and the doc says why.
* `CONTROL_CARRIER_ORDINARY_OPERANDS` grown by **11** measured cost controls.
* NEW **NO-INTERIOR-PATH fence**: counts 12 interior carrier words, of which 3
  are attached by `-C`, `-t` and `:`. Its failure message names the `=`-only trap.

**The arithmetic, re-derived from the alphabets AND from the complement
extension, as exact equalities:**

```text
CONTROL_CARRIER_CASES            61 -> 84   (+12 interior, +11 ordinary)
CONTROL_CARRIER_SLOTS            12 -> 13
MIN_CONTROL_CARRIER_CLASSES      11 -> 12
MIN_CONTROL_CARRIER_ORDINARY_OPERANDS  13 -> 24
MIN_CONTROL_CARRIER_INTERIOR_PATH        —  -> 12
class 7 count                    13 -> 24   (naive without the extension: 36)
class 12 count                    —  -> 12
every other class count          UNCHANGED
MIN_CONFIG_RESOLUTION_CLASSES    6, unchanged
```

**Class 7's fall from a naive 36 to 24 is the arithmetic proof the complement
extension took effect.** Audit 5 found `19-16` set a floor of 50 against a maximum
of 40 by construction; the arithmetic here is checked against the alphabets rather
than against the prose, and the exact-equality assertions are what check it.

### The mechanism pins and byte floors, re-measured

`SEPARATORS` byte-identical at `policy.rs:2297` with ONE commit in the whole phase
(`84a9b05`); `policy::is_separator(">")`, `("<")` and `("=")` all `false` with the
real separators as the positive control; `segment.tokens` byte-identical, with the
SEGMENT-COUNT pins green over both the `>/dev/null` and the `x2>` rows, asserting
the count AND the exact token vector so both the SPLIT and the DISPLACED variant
are caught; round 5's literalness bit non-vacuous, read from the real tokenizer;
round 6's deletion model and its over-deletion control; round 7's four
callee-grammar directions; round 8's confinement clause with both `--signed no`
controls; round 9's re-parse clause with `aliasx.`/`notalias.` and the `T-19-86`
`!`-bodied row at exit 0; round 10's nine envelope-root operand rows; round 11's
redirection-target and exact-path clauses with both near-miss controls permitted;
`envelope_config_resolution` at **30/0**. The byte floors, the proportional floor,
the deep anchor and the one-`#[cfg(test)]`-sentinel count are untouched and the
floor comment stays as `19-29` corrected it.

### `SECTION_ENVELOPE` re-measured and NOT edited

**211 whitespace tokens of an UNRAISED 215 cap. Widest line 74 of 80.** The first
`Guaranteed` clause is still TRUE and **`T-19-121` does not falsify it**: *"As
started"* states what the envelope ESTABLISHES and stops, and makes no claim about
what a later command line can do. The `Not guaranteed` half's generalisation to
*"the files and the binary this envelope runs on"* already covers this round's
route. **`advisory.rs` shows zero diff lines, and `19-31` is prohibited from
opening it too. Headroom that is spent cannot be got back.**

### Carried forward unchanged

`C-08`'s behavioural half and `T-19-120`'s behavioural half stay UNMEASURED and
are claimed in neither direction. `C-11` … `C-15` keep control **(e)** with no
rule; **no `pr_cap_*` clamp is proposed or written** — clamping a configured cap
is a PRODUCT decision about what a user may configure, not a guard rule.
`T-19-110`, `T-19-96`, `T-19-74` (core rows frozen), `T-19-112`, `T-19-113` and
`T-19-115` are unchanged. **`FORGE_VALUE_OPTS` keeps `--hostname`** — audit 9
overturned audit 8's own removal suggestion because removal moves a counted
creation form to UNCOUNTED (`T-19-35`) — and **`glab` is confirmed NOT INSTALLED**,
so every `glab` row is recorded as unmeasurable against its callee rather than
driven or inferred, and no pin that would skip was written.

**`T-19-17r` is OUTSTANDING for the FOURTEENTH time.** `grep -cE '^\| AR-19-13 \|'`
over this file is **0**; no Accepted-Risks-Log row was added, edited, renumbered
or un-accepted; `AR-19-04`, `AR-19-05` and `T-19-23` are untouched; and the word
"accepted" is applied to `T-19-17r` nowhere. **Accepting a risk is a human
decision and this plan is the fourteenth to decline it.**

### The gate

`rtk proxy cargo test --no-fail-fast`, counted with `rtk proxy grep` over a
redirected log. **Baseline 1820** (`passed + failed`, 46 result lines, 13 ignored,
17 `envelope_*` binaries, zero failures). The complete RED and PERMITTED name
lists, the per-binary counts and the exact post-state arithmetic are in
`19-30-SUMMARY.md`, and together they are `19-31`'s handoff contract.

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.**

---

## Plan 19-31 execution record — the rules and the honesty repairs

**This subsection is a RECORD MADE BY PLAN 19-31, not an audit finding.** It is
appended after the plan-19-30 record and edits nothing that precedes it — no
audit table, no Security Audit Trail, no Accepted Risks Log, no Sign-Off, and no
earlier appended subsection.

### FIRST: `/gsd-secure-phase 19` is NOT cleared, and nothing here is a closure

**`T-19-86`, `T-19-91`, `T-19-111`, `T-19-112` and `T-19-116` all remain OPEN at
`high`.** `T-19-113` and `T-19-115` remain open; `T-19-115` gets NO rule and its
condition is RESTATED, not closed.

**Whether `T-19-119`, `T-19-120` and `T-19-121` close is audit 12's judgement
rather than this plan's claim. This plan says NARROWED-or-CORRECTED and states
what remains open.** No unqualified "T-19-60 is closed" is written anywhere:
**only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.**

### `19-30`'s RED rows were confirmed STILL RED before any production line moved

Driven against the unmodified tree at `dd17bfb`, `--no-fail-fast`, verbatim:

```text
tests/envelope_interior_path.rs
test result: FAILED. 28 passed; 8 failed; 0 ignored; 0 measured; 0 filtered out; finished in 12.02s
failures:
    after_19_31_a_colon_attachment_inside_an_assignment_prefix_is_reached_too
    after_19_31_a_ledger_that_is_not_a_regular_file_is_refused_rather_than_read
    after_19_31_an_assignment_prefix_naming_a_protected_value_is_refused_and_the_unprotected_twin_is_not
    after_19_31_an_attachment_that_is_not_an_equals_sign_is_reached_the_same_way
    after_19_31_an_equals_attached_carrier_path_is_refused_over_both_protected_paths
    after_19_31_ordering_pin_a_is_the_three_way_git_dir_pin_with_the_one_character_restored
    after_19_31_ordering_pin_b_across_segments_the_first_refusal_still_wins
    after_19_31_policy_rs_5743s_own_dd_example_is_refused_in_the_direction_it_names

tests/envelope_wrapper_class.rs
test result: FAILED. 54 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.96s
failures:
    a_command_carrying_a_carrier_path_inside_a_word_is_refused_after_19_31
```

**That is exactly `19-30-SUMMARY.md`'s recorded set — nine rows, no more and no
fewer. Not one was already green.** Each row's after-verdict:

| row | before | after | in commit |
|---|---|---|---|
| `…an_equals_attached_carrier_path…` | RED | **GREEN** | `f3406d5` |
| `…an_attachment_that_is_not_an_equals_sign…` | RED | **GREEN** | `f3406d5` |
| `…a_colon_attachment_inside_an_assignment_prefix…` | RED | **GREEN** | `f3406d5` |
| `…an_assignment_prefix_naming_a_protected_value…` | RED | **GREEN** | `f3406d5` |
| `…policy_rs_5743s_own_dd_example…` | RED | **GREEN** | `f3406d5` |
| `…ordering_pin_a…` | RED | **GREEN** | `f3406d5` |
| `…ordering_pin_b…` | RED | **GREEN** | `f3406d5` |
| `a_command_carrying_a_carrier_path_inside_a_word…` | RED | **GREEN** | `f3406d5` |
| `…a_ledger_that_is_not_a_regular_file…` | RED | **GREEN** | `72f6aaa` |

### `19-30`'s PERMITTED rows were re-checked BEFORE and AFTER, with ZERO moves

Driven against the unmodified tree first, then re-driven after every commit:

```text
the_cost_rows_carry_an_equals_and_a_slash_and_stay_permitted_before_and_after   ok -> ok
the_near_miss_and_exact_path_controls_stay_permitted_before_and_after           ok -> ok
the_verdict_preserving_spellings_from_rounds_10_and_11_stay_permitted           ok -> ok
a_regular_ledger_just_under_the_bound_still_permits_and_still_counts…           ok -> ok
a_fresh_envelope_root_with_no_ledger_at_all_still_permits_before_and_after      ok -> ok
the_ledger_kind_reachability_table_is_measured_with_its_four_silences_beside_it ok -> ok
containment_holds_the_candidate_set_carries_todays_answer_byte_for_byte         ok -> ok
every_fenced_unit_pin_and_fenced_corpus_row_keeps_its_answer_under_the_widened_rule  ok -> ok
the_verdict_preserving_control_carrier_alphabets_stay_permitted…  (wrapper)     ok -> ok
wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic        ok -> ok
```

**No row moved from permitted to refused.** Had one, it would have been reported
as a finding about the RULE rather than edited.

### The design, with the boundary stated exactly

The rule is **every `/`-ANCHORED SUBSTRING of a LITERAL word**, each fed through
the **EXISTING** `lexical_absolute_components` normaliser and the **EXISTING** two
comparisons. `lexical_absolute_components` is unchanged in behaviour and in
signature text.

**CONTAINMENT, STATED PRECISELY RATHER THAN AS THE `i == 0` APPROXIMATION.**
`lexical_absolute_components` strips leading `./`s **before** it tests
`starts_with('/')`, so the shorthand *"`i == 0` is today's rule"* is false — `.//abs/p`
is accepted today at no `/`-index of its own. The exact statement, and the one
written into the helper's doc and asserted mechanically by
`containment_holds_the_candidate_set_carries_todays_answer_byte_for_byte`: **for
every word today's rule answers `Some(v)` for, the string it actually NORMALISES —
the word with its leading `./`s stripped — begins with `/`, is therefore itself one
of the candidates, and re-normalises to exactly `v`.** Containment is of the
NORMALISED STRING, not of the index; the `./` strip can only ADD answers, never
remove one. **So no refusal that existed before this round can have been lost.**

**It is NOT a list of attachment characters, and that was forced rather than
preferred.** `dd`'s `of=` is an OPERAND grammar, not an option; `tar -C/p`
attaches with nothing; `rsync host:/p` attaches with a `:`; `--opt=a=/p` attaches
after a second `=`. A character list is a PROGRAM-GRAMMAR ENUMERATION and is
D-08's defect one level over — the same shape
`wrapper_names_the_fix_must_not_know_are_absent_from_the_production_logic` exists
to forbid one level over, and that test is green.

**ROUND 3'S PRINCIPLE IS DISCHARGED RATHER THAN WEAKENED.** The candidate scan is
a scan of ONE WORD, inside the predicate that already reads that word.
`protected_carrier_named` is still raised ONCE, at the top of `classify_segments`'
existing per-segment loop. **No second reading site was needed and none was
written**; ordering pin B's control row — `git push --force … && dd if=… of=<ENV>/alpha/x`
answering `force_push_blocked` — is the mechanical proof, and it is green.

**ZERO DIFF LINES, verified by reading the diff rather than asserted:**
`SEPARATORS` (byte-identical at `policy.rs:2297`, still ONE commit in the phase,
`84a9b05`), `is_separator`, `tokenize`, `skip_redirection_target`,
`redirection_operator_len`, `split_segments_with_heads`, `Token`, `Segment`,
`segment.tokens`, `resolve_program`, `resolve_program_with_head`,
`first_unreadable_decision_word`, `config_key_operand_index`,
`subcommand_word_indices`, `scan_gh_api`, `forbidden_repo_path`,
`forbidden_repo_prefixes`, `POLICY_MIN_PRODUCTION_BYTES` and
`HOOKS_MIN_PRODUCTION_BYTES`. `is_separator(">")` is still `false` and the
SEGMENT-COUNT pins are green in both the SPLIT and DISPLACED variants.

### The cost, disclosed from BOTH sides

Every shape audit 11 named, pinned exit 0 **before AND after** through the guard,
**and now at the unit level too** with its candidate component lists asserted:

```text
--author=A <a@b.c>                 no `/` in the word              -> NO CANDIDATE AT ALL
--format=%H                        no `/`                          -> NO CANDIDATE AT ALL
sed s/x/y/                         [x, y], [y], []                 -> shorter than any envelope dir
https://github.com/o/r             [github.com,o,r] x2, [o,r], [r] -> wrong FIRST component
HEAD:refs/heads/gsd-auto/alpha/w   [heads,gsd-auto,alpha,w], …     -> wrong FIRST component
git --git-dir=/tmp/g status        [tmp, g]                        -> not under <root>/<alias>
GSD_MM_ENVELOPE_ROOT=/tmp/fresh …  [tmp, fresh]                    -> not protected: PERMITTED
rm -f /tmp/pr-ledger.ndjson        [pr-ledger.ndjson]              -> shorter than the directory
rg pr-ledger.ndjson src/           `src/` -> the EMPTY component list
cp /bin/true <BINPAR>/some-other-file   never EQUALS the binary    -> the exact-path control holds
```

The mechanical reason is `word_is_within`'s `dir.is_empty() || word.len() < dir.len()`
early return over COMPONENT VECTORS, now applied PER CANDIDATE. **The cost is
bounded by the PATH SET, not by the split.** `git --git-dir=/tmp/g status` is
still permitted in all five of its sites.

**THE ONE NEW MEMBER OF THE DISCLOSED COST, written into the predicate's own cost
section in its GENERAL form rather than narrated by its easiest instance:**

> **Any word whose text contains THIS RUN'S OWN envelope directory or THIS RUN'S
> OWN binary path as a `/`-anchored substring is now refused — EVEN WHERE THE
> PROGRAM WOULD NOT HAVE USED THAT SUBSTRING AS A PATH.** As a value, a pattern, a
> commit message, a URL fragment, or a relative path that merely happens to
> contain it.

It fails CLOSED, it is bounded to this run's own two paths, and it is the same
family as the already-disclosed refusal of `cat <ledger>`. `19-30`'s measured
instances sit UNDER it: a RELATIVE word that textually contains the carrier path
(the most reachable, now pinned at the unit level beside the `./alpha/…` row it
corrects), and a URL whose `#`-fragment contains it — **whose `?`-spelling does
NOT, because `?` clears `Token.literal` and the scan never runs on the word.** That
difference is why the surface is described over what the predicate READS rather
than over what a URL looks like. **A cost that is written down is a cost; a cost
that is met later is a defect.**

### The RESTATED RESIDUE CONDITION, quoted as it now stands, with NO count

> **This predicate reads every `/`-ANCHORED SUBSTRING of every LITERAL word, in
> either word class, against both paths. It is silent about a word the SHELL MAY
> REWRITE, because the guard cannot know its final text; about a word whose TEXT
> CARRIES NO ABSOLUTE PATH ANYWHERE, because there is nothing in it to normalise;
> and about a word that reaches a protected path ONLY THROUGH A LINK, because no
> lexical reading of the text names it.**

**The one-character correction:** *"a word that IS NOT ABSOLUTE"* became *"a word
whose TEXT carries no absolute path anywhere in it"* — the old clause was true of
the code when written and false of it now. **NO COUNT is written.** The doc
previously said *"SEVEN spellings are MEASURED"* and before that *"four"*; each
number was correct only until the next round found a spelling it had not, which
is `T-19-107`'s shape in a shorter sentence. **The condition is stated over what
the predicate READS and is handed to NO pin, NO schedule and NO version witness** —
every clause of it is reachable by the driven agent TODAY, so a schedule against a
future git version string would observe the wrong thing entirely.

### `T-19-116` — NARROWED FURTHER, and what remains is named at the same weight

**Does:** every option-attached spelling of the binary is now refused, including
`dd if=/bin/true of=<BINARY>`, the spelling that moved a bare remote's `main`
`835b6be` → `40ad7f7`.

**Does not** — the declared harm, one permitted command removing layer 2 AND layer
3, stays reachable through all four of:

```text
cp /bin/true $(command -v gsd-meta-manager)          <- expansion-borne
cp /bin/true ~/.cargo/bin/gsd-meta-manager           <- tilde
cd <binary-parent> && cp /bin/true gsd-meta-manager  <- relative
a PATH symlink whose target current_exe() reports    <- link
```

**None of the four is an option attachment; all four are instances of the
condition above. `T-19-116` stays OPEN at `high`.**

### The three named file exceptions, stated AS exceptions

| file | bound | verification it held |
|---|---|---|
| `src/envelope/policy.rs` | the candidate helper inside the slice, the two path halves' bodies, the predicate's doc, `:5743`, the `:9719` comment, the severable clause, additions inside the existing `mod tests` | 100/93 lib policy pins green; the zero-diff item list above read out of the diff |
| `src/envelope/ledger.rs` | ONLY the KIND check inside `record_and_check_in`'s existing `Ok` arm, plus its doc and one actionable-message helper | `tally`, `append_entry`, `ends_mid_line`, `parse_stamp`, `saturating_bump`, `ledger_path_in`, `LedgerEntry`, `Tally`, `CapVerdict`, `WINDOW_SECS` and `MAX_LEDGER_BYTES`' VALUE all zero diff |
| `src/envelope/cred.rs` | ONLY the WHAT IS NOT COVERED bullet at `:420-425` | `hooks_path_env`'s pairs, the `git credential fill` criterion table at `:399-414`, `write_gitconfig`, `write_gitconfig_in`, `gitconfig_body`, `config_env`, `build_env_in` all zero diff; the only deletion is the five lines of the false claim |

**`src/envelope/hooks.rs`, `src/envelope/mod.rs`, `src/envelope/advisory.rs`,
`src/scan.rs` and `src/config.rs` were NOT OPENED AT ALL** — half what `19-29`
opened. `git diff --numstat dd17bfb..HEAD` over those five files is **empty**.
**`SECTION_ENVELOPE` shows zero diff lines and its headroom is unspent**: `19-30`
re-measured 211 whitespace tokens of an UNRAISED 215 cap, widest line 74 of 80,
and *"As started"* means `T-19-121` does not falsify the first `Guaranteed`
clause whether or not the severable clause landed.

### The slice placement, as a mechanical fact

`slash_anchored_candidates` sits **immediately after**
`fn lexical_absolute_components(word: &str)` — the literal slice anchor of
`the_carrier_predicate_asks_the_filesystem_nothing_and_its_own_source_says_so` —
so it falls INSIDE that assertion's region. **A FOURTH positive control naming it
was added**, beside the three that already prove the slice is the right region, so
a future slice that missed it fails loudly rather than certifying the head of the
region. The eight forbidden filesystem and process APIs still appear nowhere in
the sliced code, and the signature text of the anchor is unchanged.

### The ledger KIND check

`record_and_check_in`'s existing `Ok` arm now binds the whole `Metadata` and
refuses when `file_type().is_file()` is false, **before** the size comparison and
**inside the same `stat`** — one syscall already on this path, no second probe, no
TOCTOU window widened.

```text
FIFO ledger (stat size 0)     before: exit 124 after 20.02 s   after: exit 2 after 21 ms
8 366 000-byte regular ledger before: exit 0, 89000 -> 89001    after: exit 0, 89000 -> 89001
FRESH root, no ledger at all  before: exit 0, 1 ledger line     after: exit 0, 1 ledger line
```

**The `Err` arm keeps its behaviour**, so a fresh envelope root's first forge call
still falls through — `19-30` pinned that row exit 0 before and after precisely so
a check placed outside the arm would turn one evidence row red instead of a large
fraction of the suite, and it is green. **The link-following stat was kept and the
non-following variant was not substituted**, so a symlinked-to-regular ledger stays
permitted and counted; that variant's literal name is deliberately absent from
`ledger.rs`, because the verify step greps the file for it. `MAX_LEDGER_BYTES`'
VALUE, derivation and just-under/just-over discrimination are untouched — a bound
over the wrong property is not corrected by moving the number. The refusal carries
`ParkReason::EnvelopeAssertionFailed`, never `PrCapExceeded`, which would name a
cap that fired on a ledger that was never tallied (D-24), and it names the file and
the kind found (AR-19-11) without quoting the command back (SAFE-04).

**`T-19-120`'s BEHAVIOURAL HALF — what the agent CLI does with a hook that never
returns — stays UNMEASURED and is claimed in NEITHER direction.**

### Both claim corrections, quoted before and after

Each was required **regardless of whether any rule landed**, and each is in a task
that could land independently of the severable one.

**`policy.rs:5743`, before:**

> The guard cannot tell a read from a write without knowing every program's
> grammar — is `dd if=X of=Y` a read of `X` or a write of `Y`? is `tee F` a read?

**Measured FALSE:** `dd if=<ENV>/alpha/pr-ledger.ndjson of=/tmp/stolen` was **exit
0**, so the guard refused it NEITHER way and the example illustrated nothing.

**After** — the sentence is kept and a WR-02 note added beside it saying it was
right-SOUNDING because the ambiguity it describes is real, FALSE about this code
because the path sat after an `=` inside a word, and TRUE now because the same
word's `/`-anchored candidates are read so both the `if=` and the `of=` spelling
reach the path set. `tee F`, beside it, was refused throughout. (The severance
fallback — replacing the example with `tee F` — was not needed.)

**`cred.rs:420-425`, before:**

> That is a BOUNDED residue rather than a reason to decline the control, and the
> bound is stated rather than assumed: **that spelling is ARGV-VISIBLE and is
> already governed** by `scan_leading`'s leading-option region and layer 2's whole
> grammar — unlike every write spelling, which is not.

**Argv-VISIBLE it was; GOVERNED it was not.** `scan_leading` parsed the word and no
rule acted on it: the by-name deny covered `core.hooksPath`, round 8's clause
covered `include.path`, and nothing covered `credential.helper`, measured returning
the ambient secret on ONE permitted line.

**After** — the bullet says the spelling is argv-visible, then **NAMES the rule
that acts on it** (`policy::config_key_names_the_credential_helper`) and states at
the same weight what that rule does not reach. **This is recorded in place as the
SIXTH instance in this phase of a residue whose stated bound was a layer that did
not enforce it** — `T-19-84`, `T-19-107`, `T-19-109` and `T-19-115` are the others
— and the repaired text states the discipline explicitly: **prefer a claim that
stays true to one that must be maintained.** A sentence asserting that another
layer governs something must be kept true by every future editor of that layer; a
sentence naming the rule that acts, or plainly naming its absence, is checkable at
a glance. The text says that if the named rule is ever removed, the honest edit is
to say the residue is UNBOUNDED — not to reach for another layer.

**The claim correction and the severable rule are in DIFFERENT COMMITS**, and the
correction was ordered AFTER the rule so that it RECORDS an observed outcome. **How
it was checked:** `git log --oneline` showed Task 2's commit `835d6c4`, and `grep`
found `config_key_names_the_credential_helper` defined at `policy.rs:1500` and
raised at `:596`. **Observed branch: the clause LANDED.**

### The severable clause LANDED, and its shape was derived rather than chosen

`config_key_names_the_credential_helper` is a predicate over **SECTION
`credential` and FINAL COMPONENT `helper`**, folded case-insensitively the way git
folds them and never reading the subsection. That shape comes from `19-30`'s
measured key-shape space and from nothing else: git folds a section and a final
name and keeps a subsection case-sensitive, so this predicate reaches the
URL-SCOPED `credential.<url>.helper` — whose subsection is any URL, an OPEN family
no enumeration could close — and reaches none of the near misses.

It is raised in the leading-option region `scan_leading` already parses, beside the
`core.hooksPath` deny, at `ParkReason::EnvelopeAssertionFailed`. **No new park
reason, no new reading site, no change to `scan_leading`'s walk, to
`config_key_operand_index` or to `subcommand_word_indices`, and `hooks.rs` was not
opened.** **It is NOT a substring or `contains` test** — `contains` refuses
`credential.helperx`, a key real git IGNORES.

```text
                                                guard before   guard after
-c credential.helper=store                      exit 0         exit 2  envelope_assertion_failed
-c CREDENTIAL.HELPER=store                      exit 0         exit 2
-c Credential.Helper=store                      exit 0         exit 2
-c credential.https://github.com.helper=store   exit 0         exit 2   <- URL-SCOPED
--config-env=credential.helper=EVILVAR          exit 0         exit 2   <- see below
-c credentialx.helper=store                     exit 0         exit 0   <- CONTROL
-c notcredential.helper=store                   exit 0         exit 0   <- CONTROL
-c credential.helperx=store                     exit 0         exit 0   <- CONTROL
-c credential.helper.x=store                    exit 0         exit 0   <- CONTROL
-c credential=store                             exit 0         exit 0   <- CONTROL
```

**Every verdict was driven as a `record_only` print and converted to an assertion
only after it was read.** The four near-miss controls are what make this a by-name
clause rather than a substring test, and they are permitted rather than refused at
the same identifier — `19-27`'s measured failure mode avoided.

**A GAIN BEYOND WHAT THE PLAN PREDICTED, recorded as a correction rather than
claimed as an aim.** `19-30` measured `--config-env=credential.helper=<VAR>` at
exit 0 and recorded it as a shape a by-name clause would NOT reach. **It IS
reached**, in both `--config-env` grammars, because `leading_git_option` yields an
assignment for it and this clause reads the KEY half, which `--config-env` spells
identically. Real git was measured resolving the helper from it, so this is a
genuine reach closed rather than an over-refusal.

**What it does NOT reach, stated at the same weight in its own doc:**

* **`GIT_CONFIG_PARAMETERS`** — an environment variable, not argv, so this clause
  is silent about it. It is refused today by a **DIFFERENT** mechanism, the
  envelope's own env-key deny at `hook_bypass_blocked`. **This re-confirms `19-30`'s
  correction of the plan's own text**, which called it *"open at exit 0"*.
  `T-19-104` stays registered, because a refusal by another mechanism is not a
  statement about this one. Recorded in the evidence file in NEITHER direction.
* **The `git config` WRITING form** — an operand of the `config` verb, not a
  leading option. It writes a FILE, which is the family `cred.rs`'s injected empty
  pair covers instead. Recorded, not asserted.
* **Any spelling that reaches the helper without naming this key on a leading
  option.** The clause is a by-name deny in one region and is not a statement
  about the credential layer as a whole.

**`cred.rs`'s injected empty-helper pair and this clause are two DIFFERENT
controls covering two different families, and neither is a substitute for the
other** — the pair reads no command line and so defends every write spelling; this
clause defends one argv spelling. Both are said so in both docs.

**No revisit condition and no version witness was created for any residue.**

**Over-refusal disclosed:** reading the KEY half only means `-c credential.helper=`
with an EMPTY value — a reset rather than a set — is refused too. It costs nothing
reachable, because the envelope already injects exactly that empty pair, and
reading the VALUE half is what `scan_leading`'s own cost containment forbids.

**`envelope_config_resolution` is 30/0 and NOT ONE of its thirty verdicts moved**,
confirmed before the commit and again at the gate.

### The stale-comment correction at `policy.rs:9717-9721`

`word_is_within("./alpha/pr-ledger.ndjson")` still answers `false` — **the
assertion did not move.** Its stated reason did: *"stripping the leading `./` must
not turn a relative word into an absolute one"* was true of the code that read only
the whole word, and is false of the code now, because that relative word DOES yield
the absolute candidates `/alpha/pr-ledger.ndjson` and `/pr-ledger.ndjson`. It
answers `false` for a DIFFERENT reason: two components against the envelope
directory's three. The comment carries that WR-02 correction, and **a NEW row was
added beside it** — `./tmp/envroot/alpha/pr-ledger.ndjson` answering `true`, with
its one-character control `./tmp/envrooz/…` answering `false` — pinning the
disclosed over-refusal at its measured verdict. `19-30` named this site in advance;
it is the only comment corrected and no assertion moved.

### Mechanism pins and byte floors, re-measured

`SEPARATORS` byte-identical (ONE commit in the phase, `84a9b05`);
`is_separator(">")` `false`; `segment.tokens` byte-identical with the SEGMENT-COUNT
pins green in both variants; round 5's literalness bit non-vacuous; round 6's
deletion model unchanged including its over-deletion control at exit 0; round 7's
four callee-grammar directions; round 8's confinement clause; round 9's re-parse
clause with `aliasx.`/`notalias.` at exit 0 and both `T-19-86` `!`-bodied rows at
exit 0; round 10's clause; round 11's redirection and exact-path clauses with both
near-miss controls permitted. **`every_fenced_unit_pin_and_fenced_corpus_row_keeps_its_answer_under_the_widened_rule`
— nine unit pins, twenty fenced corpus words and eight positive controls — is
GREEN**, which is the mechanical proof the fenced-file enumeration is zero.
`POLICY_MIN_PRODUCTION_BYTES`, `HOOKS_MIN_PRODUCTION_BYTES`, the proportional
floor, the deep anchor and the one-`#[cfg(test)]`-sentinel counts are untouched,
and the floor comment stays as `19-29` corrected it. This plan adds prose to
`policy.rs`, which only grows the production half.

### Carried forward unchanged

`C-11` … `C-15` keep control (e); **no `pr_cap_*` clamp was written**, because
clamping a configured cap is a product decision. **`C-08`'s behavioural half stays
UNMEASURED and is claimed in NEITHER direction.** `T-19-96`, `T-19-110`, `T-19-74`
(core rows frozen), `T-19-84`, `T-19-85` and `T-19-61` … `T-19-73` are unchanged;
`scan.rs` and `config.rs` were not opened. **`FORGE_VALUE_OPTS` keeps
`--hostname`** and **`glab` is confirmed NOT INSTALLED**, so the `glab --host`
forge cell is unfixed and no pin that would skip was written. `T-19-86` keeps
`19-27`'s five-site attribution correction and `T-19-111` stays OUT of it.

**`T-19-17r` is OUTSTANDING for the FIFTEENTH time.** `grep -cE '^\| AR-19-13 \|'`
over this file is **0**; no Accepted-Risks-Log row was added, edited, renumbered or
un-accepted; `AR-19-04`, `AR-19-05` and `T-19-23` are untouched; and the word
"accepted" is applied to `T-19-17r` nowhere. **Accepting a risk is a human decision
and this plan is the fifteenth to decline it.**

### The gate, with its arithmetic stated and CHECKED

`rtk proxy cargo test --no-fail-fast`, counted with `rtk proxy grep` over a
redirected log (D-34). `cargo build` and `cargo clippy -- -D warnings` both exit 0;
`cargo clippy --tests` was NOT the gate (it fails at base on four pre-existing
lints in `src/browser.rs` and `src/project_creator.rs`).

| | `19-30` | `19-31` |
|---|---|---|
| `passed + failed` | **1859** | **1871** |
| failures | 9, by design | **0** |
| result lines | 47 | 47 |
| ignored | 13 | 13 |
| `envelope_*` binaries | 18 | **18** |

**1871 − 1859 = 12, and 12 is exactly the number of new `#[test]` fns**, counted
from `git show` over the three commits: **7** in `policy.rs`'s own `mod tests` and
**5** in `tests/envelope_interior_path.rs`. **A red test RAN, so red→green leaves
the total unchanged and every increase came ONLY from new tests. The arithmetic
holds exactly.**

Per-binary counts, all eighteen `envelope_*` binaries RAN:

```text
envelope_advisory.rs          ok  10 passed  0 failed
envelope_argv_deletion.rs     ok  20 passed  0 failed
envelope_callee_grammar.rs    ok  19 passed  0 failed
envelope_carrier_reach.rs     ok  39 passed  0 failed
envelope_command_position.rs  ok  18 passed  0 failed
envelope_config_resolution.rs ok  30 passed  0 failed   <- the 30/0 pin, unmoved
envelope_control_carrier.rs   ok  37 passed  0 failed
envelope_credential.rs        ok   6 passed  0 failed
envelope_expansion_slots.rs   ok  32 passed  0 failed
envelope_hook_refusals.rs     ok   7 passed  0 failed
envelope_interior_path.rs     ok  41 passed  0 failed   <- was 28/8
envelope_literal_decision.rs  ok  43 passed  0 failed
envelope_pr_cap.rs            ok  11 passed  0 failed
envelope_reparsed_value.rs    ok  34 passed  0 failed
envelope_tracer.rs            ok   6 passed  0 failed
envelope_wiring.rs            ok  14 passed  0 failed
envelope_wrapper_bypass.rs    ok  13 passed  0 failed
envelope_wrapper_class.rs     ok  55 passed  0 failed   <- was 54/1
                                 435 tests over 18 binaries
```

`git diff --name-only dd17bfb..HEAD -- src/` lists exactly `policy.rs`,
`ledger.rs` and `cred.rs`. `git diff --numstat dd17bfb..HEAD -- tests/` shows
**164 additions and ZERO deletions**. Neither `Cargo.toml` nor `Cargo.lock`
appears — `T-19-SC` holds phase-wide.

**NONE of the four documented flakes fired** — not the two `driver_reattach`
failures, not `envelope_tracer`'s `ExecutableFileBusy` stub-write race, and not the
ETXTBSY race over the binary. **Absence is not evidence any of them is fixed**, and
nothing here was done to them. **The ETXTBSY race is `C-10`'s own seam and this
round changes what the guard does about that seam; its relation to this round is
claimed in NEITHER direction.**

**Only the WRAPPER-OPERAND sub-class of `T-19-60` is closed.**

---

## Threats found by audit 12 (2026-09-04, after plans 19-30 and 19-31)

**Provenance, stated first.** The two subsections above this one were written by
the EXECUTORS of plans 19-30 and 19-31; they are deliberately outside the audit
tables. Audit 12 left them, every earlier appended subsection and every earlier
audit's own tables **byte-identical** — the body below the frontmatter was
checksummed before writing (`sha256
d76681753f0cd393fdbc46c9a0aa3a274b8d6b5e50e605264b6393b93a4213cd` over the
931,948 bytes below the frontmatter of the 933,073-byte file), and this audit's
write changes the frontmatter, adds one Security-Audit-Trail row, one method
subsection and everything from here to the end of the file, **deleting nothing**.
Everything below is **audit 12's own**, measured against the built binary at
`f06d153` with a fresh `GSD_MM_ENVELOPE_ROOT` per row and the whole envelope root
walked afterwards, with every claimed bypass confirmed against the **real `git`
binary** in a rebuilt bare-remote fixture with a control beside every leg, and
with every "this round changed X" claim measured against a **rebuilt
pre-round-12 control binary** at `dd17bfb` rather than inferred from a diff.

### The question this audit was set, answered plainly

**Rounds 1–9 modelled an argv, round 10 a file, and audits 10 and 11 both swept
for a further layer and found none. Round 12 was a NARROWING round. The question
is whether the narrowing is now true — is the residue condition finally
complete?**

**Three answers.**

1. **The narrowing is REAL and two threats genuinely close.** `T-19-119` closes:
   audit 12 swept the whole class rather than its listed spellings — fourteen
   attachment characters over both protected paths, including the four `19-30`
   recorded as unreached — and **every one is exit 2**. `T-19-120` closes on its
   mechanical half: the FIFO ledger answers **exit 2 in 41 ms** where it hung for
   20 s, a fresh root still permits and counts, and a symlinked-to-**regular**
   ledger is still permitted and counted, which is the control that keeps the
   check a KIND bound rather than a disarmed cap. **The boundary the round chose
   — `/`-anchored substrings rather than an `=`-split or a character list — is
   the right one and audit 12 endorses it**, and the containment claim holds by
   measurement: not one pre-existing refusal was lost.

2. **BUT THE RESIDUE CONDITION IS STILL NOT COMPLETE, AND THIS TIME THERE ARE
   TWO CASES, NOT ONE.** Round 11 stated the residue as a condition and it
   omitted a case; round 12 restated it and it omits two more. The condition
   says the predicate *"reads every `/`-anchored substring of every LITERAL word,
   **in either word class**, against both paths"* and is silent about three
   things. **Both new cases are words that are LITERAL, carry an ABSOLUTE path
   DIRECTLY, need no cwd, no expansion and no link — so they belong to none of
   the three clauses:**
   * **a word in NEITHER WORD CLASS.** `xargs rm -rf <<< <ENV>/alpha` is **exit
     0**, and under real `bash` it **deleted the entire envelope directory**. A
     here-string word is consumed by `consume_redirection` and emits no `Token`;
     `<<<` is not a pathname operator so it is not recorded in
     `Segment::redirection_targets` either. The word is right there in the text
     the guard read and is in neither of the two classes the predicate iterates.
   * **a word naming an ANCESTOR of the protected path.** `rm -rf <root>` is
     **exit 0** while `rm -rf <root>/alpha` is exit 2. The boundary is a PREFIX
     over `<root>/<alias>`, so a word naming its PARENT is shorter and
     `word_is_within`'s `word.len() < dir.len()` early return answers `false`.

   **Audit 12 drove BOTH to harm, twice each, with a control beside every leg.**

3. **AND ONE OF THE ROUND'S THREE NEW MECHANISMS DOES CREATE NEW REACHABLE
   STATE.** The `/`-anchored candidate scan turned a LINEAR normalisation into a
   QUADRATIC one — `lexical_absolute_components` used to run ONCE per word and
   now runs ONCE PER `/` — and it runs on the guard's `PreToolUse` critical path.
   Measured on both trees against the same input: **a 16 KB command crosses
   `GUARD_TIMEOUT_SECS = 5`, and a 40 KB command takes 24.6 s where the
   pre-round-12 binary answers in 47 ms.** `MAX_GUARD_REQUEST_BYTES` bounds the
   INPUT at 1 MiB and bounds no work at all — which is `T-19-120`'s own shape, a
   bound over the wrong property, arriving in the round that fixed `T-19-120`.

**The honest one-sentence answer: the boundary round 12 chose is right, two
threats really close, and the residue condition is still not the condition the
code implements — this time because it enumerates the silences of a READING and
the code's silences are also in its WORD SET and in its COMPARISON.**

### The new rows

| Threat ID | Category | Component | Severity | Disposition | Finding | Status |
|---|---|---|---|---|---|---|
| **T-19-122** | **Elevation of Privilege / Tampering** | **`policy::tokenize`'s `consume_redirection` (`policy.rs:3532-3609`) and `protected_carrier_named`'s two word classes (`policy.rs:6118-6128`). A NON-PATHNAME redirection operator's word is DELETED from the token stream and is NOT recorded in `Segment::redirection_targets`, so it is in NEITHER class** | **high** | **mitigate (unimplemented — named in no residue clause, no threat row, no corpus class)** | **A LITERAL, ABSOLUTE WORD NAMING THE PROTECTED DIRECTORY EXACTLY IS INVISIBLE TO A PREDICATE WHOSE OWN CONDITION SAYS IT READS "EVERY LITERAL WORD, IN EITHER WORD CLASS", AND AUDIT 12 DELETED THE WHOLE ENVELOPE WITH IT.** `xargs rm -rf <<< <ENV>/alpha` is **exit 0** through the guard against `rm -rf <ENV>/alpha` at **exit 2** — the same program-free pair differing only in whether the path stands as an operand or after a `<<<`. The word is fully `Token.literal` (no `$`, no `~`, no glob, no brace), it is ABSOLUTE at index zero, it needs no cwd and no link, and it names the directory EXACTLY — **so it is none of the three declared silences.** It is silent because `consume_redirection` consumes the target word and emits no `Token`, and records it on the `Segment` only when `pathname_target && target.literal`; `<<<` sets `pathname_target = false`. **DRIVEN END TO END, TWICE.** *(i) LAYER 3.* Rebuilt bare-remote fixture, `pre-push`/`pre-commit` in `stub_body`'s exact three-line shape through the envelope's own `GIT_CONFIG_COUNT`/`KEY_0`/`VALUE_0` triplet: **CONTROL** — hooks present, `git push --force origin HEAD:refs/heads/main` → **REFUSED**, remote `main` UNMOVED at `c13f9ea`. **GUARD** — `xargs rm -rf <<< <ENV>/alpha` **exit 0** against the twin at exit 2. **LEG B** — command performed under real `bash`, envelope directory GONE, same push → **COMPLETED, remote `main` `c13f9ea` → `371a2f5` MOVED**. **CONTROL** — hooks restored, remote rewound, same push → **REFUSED, UNMOVED**. *(ii) THE PR CAP.* Four `gh pr create` calls in one persistent root → 1 permit then 3 `pr_cap_exceeded`, four ledger lines; `rm -f <ledger>` **exit 2**, `xargs rm -f <<< <ledger>` **exit 0**; command performed; call 5 → **PERMITTED again, 1 line**. **THE BINARY HALF IS REACHED TOO**: `cp /bin/true <BINARY>` is exit 2 while `xargs cp /bin/true <<< <BINARY>` and `xargs -I{} cp /bin/true {} <<< <BINARY>` are **exit 0**. **THE HEREDOC FORM IS THE SAME**: `xargs rm -rf <<EOF\\n<ENV>/alpha\\nEOF` deleted the directory under real `bash`. **AUDIT 11 LOOKED AT THIS CELL AND CLASSIFIED IT AS A COST AVOIDED**, recording the five non-pathname operators as *"correctly NOT recorded as carriers, so the round bought its rule without an over-refusal"* with `cat <<<X` as the witness. That is right about `cat` and wrong as a general statement: a here-string is a DATA channel, and `xargs` is a program that turns data into argv. **This re-opens `T-19-112`'s cap route and `T-19-116`'s layer-3 route by a spelling no round has drawn.** It is PRE-EXISTING — the pre-round-12 control binary at `dd17bfb` permits it too — so it is a gap twelve audits did not find rather than a regression. | **OPEN — BLOCKING** |
| **T-19-123** | **Elevation of Privilege / Tampering** | **`policy::word_is_within` (`policy.rs:5685-5711`) — a PREFIX boundary over `<root>/<alias>`, whose `word.len() < dir.len()` early return answers `false` for every ANCESTOR of the protected directory. Pinned PERMITTED by name at `policy.rs:10022-10026` and `tests/envelope_interior_path.rs:2341`** | **high** | **mitigate (unimplemented — the PERMIT is a deliberate, pinned design decision; the HARM it admits is modelled nowhere and named in no residue clause)** | **THE PREDICATE'S WHOLE REASON FOR BEING A PREFIX IS THAT `rm -rf <root>/<alias>` TAKES NINE CARRIERS IN ONE CALL — AND `rm -rf <root>` TAKES THE SAME NINE, ONE COMPONENT UP, AT EXIT 0.** `rm -rf <root>`, `rm -rf <root>/`, `mv <root> /tmp/gone`, `tar -C<root> …`, `find <root> -delete` and `chmod 000 <root>` are all **exit 0**, against `rm -rf <root>/alpha` at **exit 2**. The word is LITERAL, ABSOLUTE, names a real path DIRECTLY with no link and no cwd — **none of the three declared silences** — and the root is trivially derivable: `git config --get core.hooksPath` is **permitted and reports `<root>/<alias>/hooks`**, and **the refusal message itself names `<root>/<alias>`**, so one `dirname` away from any refusal is a permit. **DRIVEN END TO END, TWICE, CONTROL BESIDE EVERY LEG.** *(i)* **CONTROL** hooks present → force push **REFUSED**, remote `main` UNMOVED at `0658c51`; **GUARD** `rm -rf <root>` **exit 0** / `rm -rf <root>/alpha` **exit 2**; **LEG B** performed → same push **COMPLETED**, `0658c51` → `140421f` **MOVED**; **CONTROL** hooks restored, remote rewound → **REFUSED, UNMOVED**. *(ii)* a fired PR cap (4 calls, 3 `pr_cap_exceeded`) **reset to PERMITTED** by `rm -rf <root>` with `rm -f <ledger>` at exit 2 beside it. **THE COMPOSITE THE DOC CLAIMS AS A NARROWING IS DEFEATED BY IT**: `protected_carrier_named`'s own doc says `cd <env>/<alias> && rm -f pr-ledger.ndjson` *"is refused BY SEGMENT ONE"* — measured true — but `cd <root> && rm -rf alpha` is **exit 0**, because segment one names the ancestor. **THE BINARY HALF HAS THE SAME SHAPE**: `rm -rf <binary-parent>` is exit 0, and there the permit is explicitly reasoned (`ls <parent>` and `cargo install` must stay permitted) — **but the reasoning argues only that a PREFIX would be wrong; it never states that the parent's own DELETION removes the binary.** PRE-EXISTING since round 10, permitted by the pre-round-12 control binary too. **A PREFIX BOUNDARY IS THE RIGHT KIND AND IT IS ONE COMPONENT TOO DEEP**; whether the fix is an ancestor clause or an accepted risk is a decision this audit does not make. | **OPEN — BLOCKING** |
| T-19-124 | Denial of Service | `policy::slash_anchored_candidates` (`policy.rs:5658-5662`) applied per candidate inside `word_is_within` (`:5697`) and `word_is_exactly` (`:5760`) — `lexical_absolute_components` used to run ONCE per word and now runs ONCE PER `/` | medium | mitigate | **ROUND 12'S OWN MECHANISM TURNED A LINEAR SCAN ON THE GUARD'S CRITICAL PATH INTO A QUADRATIC ONE, AND THE ROUND'S COST ANALYSIS IS SILENT ABOUT IT.** Measured on the built binary at `f06d153` against a **REBUILT PRE-ROUND-12 CONTROL BINARY at `dd17bfb`**, same input, same machine, one word of the form `/a/a/a…`: <br>`slashes  HEAD      dd17bfb`<br>`  1 000     97 ms      —`<br>`  5 000   1 763 ms    —`<br>`  8 000   5 873 ms   39 ms`<br>` 20 000  24 591 ms   47 ms`<br>` 50 000  >116 000 ms 72 ms`<br>**The 5-second `GUARD_TIMEOUT_SECS` is crossed at about 8 000 slashes — a 16 KB command line.** The decisive control that attributes it to the scan rather than to length: **a 100 000-character word with NO slashes answers in 66 ms.** `MAX_GUARD_REQUEST_BYTES = 1 MiB` (`hooks.rs:733`) bounds the INPUT and bounds no work; extrapolating the measured quadratic, a 1 MiB slash-dense word is hours. **THE ROUND'S COST CLAIM IS TRUE AND IS ABOUT SOMETHING ELSE**: *"the cost is bounded by the PATH SET, not by the split"* is a statement about which candidates can be REFUSED — verified true — and says nothing about how many are COMPUTED. **This is `T-19-120`'s own shape — a bound over the wrong property — arriving in the round that fixed `T-19-120`.** The BEHAVIOURAL half — what the agent CLI does with a `PreToolUse` hook that overruns its timeout — is **UNMEASURED and claimed in NEITHER direction**, on the same discipline `C-08`'s, `T-19-117`'s and `T-19-120`'s behavioural halves are held to. Rated `medium` on the mechanical half alone, exactly as `T-19-117` and `T-19-120` were. | open — below `high` (non-blocking) |
| T-19-125 | Information Disclosure (documentation accuracy) | `protected_carrier_named`'s disclosed cost (`policy.rs:6044-6048`) — *"any word whose text contains THIS RUN'S OWN envelope directory or binary path **as a `/`-anchored substring**"* | low | mitigate | **THE DISCLOSED OVER-REFUSAL SURFACE IS NARROWER THAN THE REAL ONE, because the comparison normalises and the sentence describes a SUBSTRING.** Measured: `dd of=<ENV>/./alpha/x` and `dd of=<ENV>/zzz/../alpha/x` are both **exit 2**, and **neither word contains `<ENV>/alpha` as a substring at all**. The true surface is *any word one of whose `/`-anchored substrings NORMALISES to a path under this run's directory or equal to its binary*. It fails CLOSED and the gap is small, but this phase has now shipped a *"FIVE forms"* claim, a *"four directions"* claim and a *"SEVEN spellings"* claim that were each wrong when written, and the round that removed the count replaced it with a wording that is one operation short. | open — below `high` (non-blocking) |
| T-19-126 | Tampering (evidence quality) | `ledger.rs:355-361`'s forbidden-API grep gate, and `CONTROL_CARRIER_INTERIOR_PATH` entry 7 (`tests/envelope_wrapper_class.rs:8450`) | low | mitigate | **TWO EVIDENCE ARTEFACTS DO NOT CARRY THE WEIGHT PUT ON THEM.** *(i)* The `symlink_metadata` gate is a **plan-time verify grep, not a standing test**: nothing in `tests/` or `src/` re-runs it, and audit 12 found **no standing pin** that a symlinked-to-**regular** ledger is permitted and counted — that property had to be measured by hand (it holds: exit 0, counted). A substitution in a later round would be caught by nothing. The comment's stated reason is also inverted: writing the literal to forbid it would turn the gate **RED**, not make it *"pass vacuously"*. *(ii)* `CONTROL_CARRIER_INTERIOR_PATH` entry 7, re-spelled to `tar --create --file /tmp/t -C<ENV>/alpha` to dodge the `-c`-bearing `NestedPayload` branch, **does not reach the file under real `tar`** — GNU tar answers *"Cowardly refusing to create an empty archive"* because the re-spelling moved `-C` last and dropped the `.` member, where `19-30`'s swept spelling `tar -C<ENV>/alpha …` reached. The alphabet's own doc says every entry was drawn from `19-30`'s measured sweep. **The no-`=` sub-class is NOT vacuous** — entry 8 `cp /bin/true -t<ENV>/alpha` reaches (verified: `true` appeared in the directory) and entry 9's `:` attachment reaches — so this is one row's evidentiary value, not the class's. | open — below `high` (non-blocking) |

*Status: open · closed · open — below `high` threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above `workflow.security_block_on` count toward `threats_open`*

`T-19-124`, `T-19-125` and `T-19-126` are open below the `high` threshold and do
**not** count toward `threats_open`. `T-19-122` and `T-19-123` do.

### `T-19-119` and `T-19-120` — CLOSED, each on a CLASS sweep rather than a spelling list

**Audit 12 closes two rows and says plainly why, because a phase blocked for
twelve rounds needs its true closures as much as its true findings.**

**`T-19-119` — CLOSED.** Its declared harm is an option-attached path being
invisible to both halves of the path set. Audit 11 closed threats on the standard
*"a threat is closed when its harm is unreachable, not when its listed spellings
are"*, so audit 12 swept the CLASS: every attachment character, both protected
paths, including the four spellings `19-30` recorded as UNREACHED and therefore
never fenced.

```text
exit=2  dd if=/dev/null of=<ENV>/alpha/pr-ledger.ndjson    <- the row that reset a fired cap
exit=2  dd if=<ENV>/alpha/pr-ledger.ndjson of=/tmp/stolen  <- policy.rs:5743's own example
exit=2  git --git-dir=<ENV>/alpha status
exit=2  tar --directory=<ENV>/alpha …        exit=2  cp --target-directory=<ENV>/alpha …
exit=2  chmod --reference=<ENV>/…            exit=2  rsync --temp-dir=<ENV>/alpha …
exit=2  tar -C<ENV>/alpha …                  exit=2  cp -t<ENV>/alpha …        <- NO `=` at all
exit=2  make -C<ENV>/alpha                   exit=2  install -D /bin/true -t<ENV>/alpha
exit=2  PATH=/usr/bin:<ENV>/alpha mytool     exit=2  R=<ENV>/alpha             <- `:` and bare
exit=2  rsync host:<ENV>/alpha /tmp/x        exit=2  ssh -oControlPath=<ENV>/alpha/s host
exit=2  python3 --out=<ENV>/alpha/x          exit=2  mytool --a=b=c=<ENV>/alpha/x
exit=2  gh api -f path=<ENV>/alpha/x …       exit=2  awk -v d=<ENV>/alpha 'BEGIN{}'
exit=2  find / -path <ENV>/alpha -delete     exit=2  GSD_MM_ENVELOPE_ROOT=<ENV>/alpha gh pr create
exit=2  dd if=/bin/true of=<BINARY>          exit=2  tar --directory=<BINARY> …
exit=2  cp /bin/true --target-directory=<BINARY>        exit=2  ln -sf /bin/true <BINARY>
```

**The boundary the round chose is the right one and audit 12 says so.** The four
candidate boundaries `19-30` costed are real: a first-`=` split misses
`--opt=a=/p`, `-C/p`, `-t/p` and `host:/p`; an every-`=` split misses the
attached short options and the `:`; a character list is a program-grammar
enumeration and is D-08's defect one level over. **`/`-anchored substrings of a
literal word is the only one of the four that reaches all of them, and it asks
nothing about what precedes the `/`, which is what stops it being one character
short a thirteenth time.**

**THE CONTAINMENT CLAIM, RE-DERIVED AS STATED RATHER THAN AS THE `i == 0`
SHORTHAND, AND NO PRE-EXISTING REFUSAL WAS LOST.** `lexical_absolute_components`
strips leading `./`s **before** testing `starts_with('/')`, so containment is of
the NORMALISED STRING, not of the index. Audit 12 verified this three ways: the
source (`policy.rs:5591-5594`, the strip precedes the test); the mechanical pin
`containment_holds_the_candidate_set_carries_todays_answer_byte_for_byte`, which
asserts both directions including the ADD control (`./abs` is `None` today and
yields `[abs]` now); and **behaviourally, by re-driving every refusal this file
records** — rounds 4 through 11's whole pin family is still exit 2 at its own
identifier and every one of their controls is still exit 0 (table below). The
`./` strip can only ADD answers. **No refusal that existed before this round was
lost.**

**`T-19-120` — CLOSED on its mechanical half.** The KIND check sits inside
`record_and_check_in`'s existing `Ok` arm, binds the whole `Metadata`, and
refuses before the size comparison in the same `stat`.

```text
FIFO ledger                        exit 2 envelope_assertion_failed in 41 ms  (was exit 124 after 20.02 s)
DIRECTORY at the ledger path       exit 2 envelope_assertion_failed
SYMLINK -> /dev/null (char device) exit 2 envelope_assertion_failed
SYMLINK -> a REGULAR file          exit 0, PERMITTED AND COUNTED   <- the control that matters
FRESH root, no ledger at all       exit 0, exactly 1 ledger line   <- the `Err` arm, unchanged
```

**The symlink-to-regular row is the control that keeps this a KIND bound rather
than a link ban**, and audit 12 had to measure it by hand because no standing
test asserts it — see `T-19-126`. The refusal carries
`ParkReason::EnvelopeAssertionFailed` and never `PrCapExceeded` (D-24), verified
at the identifier. **The behavioural half is UNMEASURED and audit 12 claims it in
neither direction**, as `19-31` did.

### `T-19-121` — NARROWED HARD, NOT CLOSED, and the residue is MEASURED rather than argued

**The clause is real, correctly shaped, and reaches more than the plan predicted.**

```text
                                                 guard      real git (fake HOME + ~/.git-credentials)
CONTROL  no `-c` at all                          —          exit 128, secret ABSENT
-c credential.helper=store                       exit 2     secret PRESENT   <- the reach is real
-c CREDENTIAL.HELPER=store                       exit 2
-c Credential.Helper=store                       exit 2
-c credential.https://github.com.helper=store    exit 2     <- URL-SCOPED, an OPEN family
--config-env=credential.helper=EVILVAR           exit 2     <- BOTH grammars, see below
--config-env credential.helper=EVILVAR           exit 2
-c credentialx.helper=store                      exit 0     <- CONTROL
-c notcredential.helper=store                    exit 0     <- CONTROL
-c credential.helperx=store                      exit 0     <- CONTROL
-c credential.helper.x=store                     exit 0     <- CONTROL
-c credential=store                              exit 0     <- CONTROL
```

**All five near-miss controls stay permitted at the same identifier**, which is
what makes this a by-name deny rather than a `contains` test — `19-27`'s measured
failure mode avoided. The clause also survives its own adjacent cells:
`bash -lc '… -c credential.helper=store …'` (exit 2, through the `NestedPayload`
recursion), `env FOO=1 git -c credential.helper=store …` (exit 2), an
expansion-borne key `K=credential.helper; git -c $K=store …` (exit 2, at round
4's Rule A), a quoted spelling and an empty-subsection spelling (both exit 2).

**BUT THE THREAT'S DECLARED HARM IS STILL REACHABLE, AND AUDIT 12 MEASURED THE
SECRET COMING BACK.** `T-19-121`'s harm is *"the run reaches the user's ambient
git credential on a single permitted command line, with no file write at all."*

```text
GUARD     git -c alias.q='!git -c credential.helper=store credential fill' q   -> exit 0
REAL GIT  the same line, envelope's FULL posture, injected empty pair present  -> SECRET PRESENT
```

**That is `T-19-86`'s `!`-bodied alias route, and audit 12 does not fold it into
`T-19-86`.** `T-19-86`'s declared harm is a force push; widening it to carry a
credential-reach harm would be the attribution move both `19-27` and audit 11
established must not be made. **`19-31` is honest about this** — its own doc lists
*"any spelling that reaches the helper without naming this key on a leading
option"* as not reached, and says the clause *"is not a statement about the
credential layer as a whole"*. **The DECLARED MITIGATION is present and verified;
the THREAT does not close, on the standard this file has used since audit 11.**

**`cred.rs:420-425` — the honesty repair WROTE WHAT IT OBSERVED, and the claim is
of the kind that stays true.** Audit 12 verified the observation rather than the
prediction: commit `835d6c4` exists and is `feat(19-31): the credential.helper
clause, beside the core.hooksPath deny` (+289 lines, `policy.rs` and
`tests/envelope_interior_path.rs`, **zero deletions**);
`config_key_names_the_credential_helper` is defined at **`policy.rs:1500`** and
raised at **`policy.rs:596`**, both exactly as reported. **The repaired text names
a RULE and states what that rule does not reach**, where the old text asserted
that another LAYER governed the spelling — and the difference is the one the
bullet claims: *"a sentence asserting that another layer governs something is a
sentence a future editor must keep true as that layer changes; a sentence naming
the rule that acts is checkable at a glance and fails loudly when the rule is
deleted."* **Audit 12 endorses the form.** **One reservation, recorded rather than
smoothed:** the bullet's `WHAT IT DOES NOT COVER` list has two items and does not
name the alias-body route, which is measured, reachable today, and defeats both
halves of the two-part control. The general clause that covers it lives in
`policy.rs`, not here, and names no instance. That is not the old failure mode —
nothing false is asserted — but it is a *"not covered"* list that is not the
residue.

### The three mechanisms as attack surface in their own right

| mechanism | does it create new reachable state? | audit 12's measurement |
|---|---|---|
| the `/`-anchored candidate scan | **YES — CPU.** `T-19-124` | quadratic; 16 KB crosses `GUARD_TIMEOUT_SECS`; pre-round-12 control answers the same input in 39 ms |
| the `credential.helper` by-name clause | **no** | five near-miss controls permitted at the same identifier; the only over-refusal is a `-c credential.helper=` with an EMPTY value, which the envelope already injects itself; `git config credential.helper store` (the writing form) stays exit 0 and is defeated by `cred.rs`'s injected pair, measured against real git |
| the ledger KIND check | **no** | fresh root still permits and counts; symlink-to-regular still permits and counts; `Err` arm untouched; refusal at `EnvelopeAssertionFailed`, never `PrCapExceeded` |

**The cost claim, checked from both sides.** Every row `19-30` and `19-31`
recorded as PERMITTED is still permitted, re-driven at `f06d153`:
`git commit --author='A <a@b.c>' -m x`, `git log --format=%H`, `sed s/x/y/ f`,
`git clone https://github.com/o/r`,
`git push origin HEAD:refs/heads/gsd-auto/alpha/w`, `git --git-dir=/tmp/g status`,
`FOO=/tmp/x cmd`, `rm -f /tmp/pr-ledger.ndjson`, `rg pr-ledger.ndjson src/` and
`GSD_MM_ENVELOPE_ROOT=/tmp/fresh gh pr create --title x` — **all exit 0.**

**The recorded over-refusals are exactly three, and `19-30`'s count is CORRECT.**
Re-measured: the relative word (`rm -f ./<ENV>/alpha/pr-ledger.ndjson`) **exit
2**, the non-path value (`git log --format=./<ENV>/alpha`) **exit 2**, the
`#`-fragment URL **exit 2**, and **the `?` spelling exit 0, UNCHANGED** because
`?` clears `Token.literal`. `19-30`'s table lists FOUR rows under a *"three
instances"* heading precisely because the fourth is the CONTROL, labelled
`exit 0, UNCHANGED` in its own cell. **The count was never two; the record is
right and the reading that made it two is not.** The one thing the general-form
statement does not cover is `T-19-125`.

### The four execution-time judgements, assessed independently

1. **The four corpus re-spellings — the CALL WAS RIGHT, the VERDICTS never
   depended on it, and ONE re-spelling lost its reach.** The branch is real:
   `policy.rs:4688` resolves any single-dash word (not `-`, not `--`) whose tail
   contains a `c` as `NestedPayload` when a word follows it, so
   `cp -t<TempDir>/alpha /bin/true` would have taken that branch whenever a
   random temp path carried a lowercase `c`. **Audit 12 built an envelope root
   whose path contains `c` and drove both forms**: the four re-spellings AND the
   two original `c`-triggering spellings are **all exit 2
   `envelope_assertion_failed`** — the carrier clause is raised at the top of
   `classify_segments`' per-segment loop, above program resolution, so the
   verdict never depended on which branch resolved the program. **Re-spelling to
   remove a source of flakiness rather than leaving it to fire in CI was the
   right call.** **The reach leg is where it costs something**: `cp /bin/true
   -t<ENV>/alpha` REACHES (verified — `true` appeared in the directory),
   `tar --directory=<ENV>/alpha … .` REACHES, `rsync --temp-dir=<ENV>/alpha …`
   REACHES, but `tar --create --file /tmp/t -C<ENV>/alpha` — entry 7 as
   re-spelled — **does NOT**, because moving `-C` last also dropped the `.`
   member and GNU tar answers *"Cowardly refusing to create an empty archive"*.
   Recorded as `T-19-126`(ii): the class stays non-vacuous through entries 8 and
   9, but one row no longer carries what the alphabet's doc says every row
   carries.

2. **`--config-env` IS reached, in BOTH grammars — VERIFIED, and reporting it as
   a CORRECTION rather than as an aim is exactly right.** Measured
   independently: `git --config-env=credential.helper=EVILVAR fetch origin` and
   `git --config-env credential.helper=EVILVAR fetch origin` are **both exit 2**,
   and `19-30` had recorded the shape as one a by-name clause would MISS. The
   mechanism is as `19-31` states it — `leading_git_option` yields an assignment
   and the clause reads the KEY half, which `--config-env` spells identically —
   and real git does resolve the helper from it, so this is **a genuine reach
   closed, not an over-refusal**. **A gain a plan did not aim at, reported as a
   correction to the plan's own prediction rather than claimed as a design
   intention, is the behaviour this file has been asking for for twelve rounds.**

3. **The forbidden API's literal name kept out of `ledger.rs` — the DECISION was
   cheap and harmless, but the GATE IS NOT MEANINGFUL, and the stated reason is
   inverted.** Audit 12 looked for the gate and found no standing form of it:
   `symlink_metadata` is asserted absent from `policy.rs`'s carrier-predicate
   slice and from its tokenizer slice (`policy.rs:10245`, `:11007`), and **from
   `ledger.rs` by nothing at all** — the only `ledger.rs` gate is a plan-verify
   grep, which is one-shot and will not re-run. Worse, the property it stands in
   for has no pin either: **no test asserts that a symlinked-to-regular ledger is
   still permitted and counted**, which is the behaviour a substitution would
   break. Audit 12 measured it by hand (exit 0, counted). **The comment's reason
   is also backwards**: writing the literal to forbid it would make a
   zero-occurrence gate go **RED**, not *"pass vacuously"*. Keeping the name out
   was harmless; the honest repair is a standing behavioural pin, and it is
   recorded as `T-19-126`(i) rather than fixed here.

4. **Extending `envelope_carrier_refusal`'s message under Rule 2 — CORRECT, and
   the grep verification holds.** Audit 12 confirmed mechanically that **no test
   anywhere asserts the wording**: the phrases `stands alone as a word`,
   `carried INSIDE a longer word` and `attached to a short option` appear only at
   `policy.rs:6166-6167` and `:6178-6179`, in the two production strings
   themselves, and nowhere in `tests/`. The old *"stands as an operand"* would
   have been actively misleading after widening — a user running `tar -C<dir>`
   and reading it would have concluded the guard had matched the wrong thing —
   and **a refusal a user cannot act on is a control that gets switched off
   (AR-19-11), which is this phase's own stated reason for naming the path in the
   first place.** The extension is accurate: `-C` is a short option, `of=` is an
   `=`, `host:` is *"any other character"*. **One line of it is now slightly
   stale in the other direction and is recorded rather than fixed**: *"To
   proceed: name a path outside that directory"* is not actionable for
   `T-19-125`'s case, where the word merely CONTAINS the directory and the
   program would never have opened it.

### The known-open set, as audit 12 verified it

Every row re-measured at `f06d153`, fresh root each, walk after.

* **`T-19-86` (high, OPEN, unaccepted, deferred by explicit user scoping
  decision).** `git -c alias.p='!git push --force origin HEAD:refs/heads/main' p`
  → **exit 0**, and `git config alias.p '!git push --force origin main'` →
  **exit 0**. Unweakened. **And it now carries a second harm**: the same route
  returns the ambient credential (see `T-19-121`), recorded THERE and **not**
  folded into this row. Counts toward `threats_open`.
* **`T-19-91` (high, OPEN, three arms).** `S=x; git reflog $S`,
  `git reflog show $S` and `git symbolic-ref $S` all **exit 0**, while
  `git symbolic-ref HEAD $S` still fails closed at **exit 2**. Unweakened.
* **`T-19-111` (high, OPEN, NO rule).** The `printf`-written `.git/config` alias
  is **exit 0** with an EMPTY walk. **Attribution stayed OUT of `T-19-86`** —
  verified by sweep, neither round folds it back.
* **`T-19-112` (high, OPEN).** The cap route this round closed (`T-19-119`) is
  genuinely closed, and **audit 12 reset a fired cap TWICE by two other
  spellings** (`T-19-122`, `T-19-123`), plus `C-15` and the deferred ledger
  option (b) remain. **`T-19-113` (medium)** unchanged. Nothing anywhere calls
  either closed.
* **`T-19-115` (medium, OPEN, no rule, no acceptance).** All three spellings
  re-measured permitted with their absolute-literal twins refused: the glob
  `rm -rf <ENV>/alph?`, the tilde, the brace and the expansion-borne composite
  `D=$(git config --get core.hooksPath); rm -f $D/../pr-ledger.ndjson` are all
  **exit 0**. **The round replaced its arithmetic with a condition, which is the
  right form and which audit 12 endorses — and the condition is still not
  complete** (`T-19-122`, `T-19-123`). Recorded there rather than as a fourth
  row.
* **`T-19-116` (high) — NARROWED FURTHER, NOT CLOSED, and audit 12 does not close
  it.** Every option-attached spelling of the binary is now refused, including
  `dd if=/bin/true of=<BINARY>`, the one that moved a bare remote's `main`. **All
  four declared residues are still exit 0**, re-measured:
  `cp /bin/true $(command -v gsd-meta-manager)` (expansion-borne),
  `cp /bin/true ~/.cargo/bin/gsd-meta-manager` (tilde),
  `cd <binary-parent> && cp /bin/true gsd-meta-manager` (relative), and the
  `PATH`-symlink case, which neither half covers. **None of the four is an option
  attachment**, exactly as `19-30` and `19-31` both state. **And audit 12 adds
  two more routes to the same harm** — `xargs cp /bin/true <<< <BINARY>` and
  `rm -rf <binary-parent>`, both exit 0. Counts toward `threats_open`.
* **`T-19-96` (medium, OPEN).** `git push --forc? origin
  refs/heads/gsd-auto/alpha/w` → **exit 0**. **`T-19-110` (medium, OPEN).**
  `git config core.hooksPath -c` and `--` → **exit 0**; the by-name deny
  `git config core.hooksPath /dev/null` → **exit 2 `hook_bypass_blocked`**.
* **`T-19-74` (medium, closed/accepted — AR-19-10).** `env $X push --force` →
  **exit 0**, core rows frozen. **`T-19-61` … `T-19-73`, `T-19-84`, `T-19-85`** —
  open, unaccepted, untouched; `scan.rs` and `config.rs` were not opened by
  either plan (`git diff --name-only dd17bfb..HEAD -- src/` lists exactly
  `cred.rs`, `ledger.rs`, `policy.rs`).
* **`C-11` … `C-15` remain entirely control (e)**, no rule, unaccepted, and
  **`C-15` keeps no `pr_cap_*` clamp** — clamping a configured cap is a product
  decision. Audit 12 endorses that reasoning for the third audit running.
* **`C-08`'s behavioural half and `T-19-120`'s behavioural half are UNMEASURED
  and claimed in NEITHER direction**, and audit 12 claims neither either — and
  adds `T-19-124`'s behavioural half to that list on the same discipline.
* **`FORGE_VALUE_OPTS` keeps `--hostname`** (`policy.rs:5266`, byte-identical);
  **`glab` is confirmed NOT installed** (`command -v glab` finds nothing), so its
  cell stays unmeasurable against its callee and **no pin that would skip was
  written**.
* **`AR-19-04` and `AR-19-05` are untouched**, not un-accepted, not re-rated and
  not renumbered. **`T-19-23` is NOT marked closed or re-closed** by either plan
  or by this audit. **Twelve `AR-` rows**, counted, unchanged.
* **`T-19-17r` — OUTSTANDING for the SIXTEENTH time.** `grep -cE '^\| AR-19-13 \|'`
  over this file is **0**. Plans `19-30` and `19-31` both declined, correctly.
  **This audit is the sixteenth agent to leave the acceptance unmade, and it is a
  human decision.**
* **`T-19-SC` — still holds.** `git diff --name-only 1e56389..HEAD` names neither
  `Cargo.toml` nor `Cargo.lock`.

### The mechanism pins, verified non-dead

Rounds 4 through 11 are all still load-bearing and all still right about the
lines round 12 refuses. Every row re-driven at `f06d153`:

```
exit=2 [envelope_assertion_failed]  git pus? --force origin main                     <- round 5
exit=2 [force_push_blocked]         git >/dev/null push --force origin main          <- round 6
exit=0                              git x2>/tmp/o push --force origin main           <- round 6 control
exit=2 [force_push_blocked]         git 2>/dev/null push --force origin main
exit=2 [envelope_assertion_failed]  git >                                            <- unresolvable
exit=2 [force_push_blocked]         git --attr-source HEAD push --force origin main  <- round 7
exit=2 [envelope_assertion_failed]  git --bogus-opt status                           <- round 7 inversion
exit=0                              git - push --force origin main                   <- round 7 control
exit=2 [force_push_blocked]         git -- push --force origin main
exit=2 [envelope_assertion_failed]  git -c include.path=/tmp/evil.cfg status         <- round 8
exit=0                              git -c aliasx.q='!git push --force' q            <- round 9 control
exit=2 [hook_bypass_blocked]        env -u GIT_CONFIG_COUNT git push --force origin main
exit=2 [envelope_assertion_failed]  V=push; git $V --force origin main               <- round 4 Rule A
exit=2 [envelope_assertion_failed]  git {push,--force} origin main                   <- round 5 brace
exit=2 [envelope_assertion_failed]  cp /bin/true <ENV>/alpha/hooks/pre-push          <- round 10
exit=2 [envelope_assertion_failed]  : > <ENV>/alpha/pr-ledger.ndjson                 <- round 11
exit=0                              : > /tmp/l                                       <- round 11 control
exit=0                              echo x > /tmp/outside && ls                      <- outside control
exit=2 [envelope_assertion_failed]  ( rm -rf <ENV>/alpha )                           <- subshell segment
exit=2 [envelope_assertion_failed]  git status && dd if=/dev/null of=<ENV>/alpha/x   <- ordering pin A
exit=2 [force_push_blocked]         git push --force … && dd if=… of=<ENV>/alpha/x   <- ordering pin B
```

**`SEPARATORS` is BYTE-IDENTICAL and audit 12 re-derived that at its CURRENT
line rather than at the one the record cites.** The constant has moved from
`policy.rs:2297` to **`policy.rs:2422`**, because round 12 added prose above it —
so `git log -L 2297,2297` now returns `af72137` and says nothing about
`SEPARATORS`. Re-derived correctly: `git log -L 2422,2422:src/envelope/policy.rs`
returns **exactly one** commit for the whole phase, **`84a9b05`** (plan 19-05),
and the constant's text is byte-for-byte identical to its form at `84a9b05`,
`1e56389` and `dd17bfb`. **The claim is TRUE; the line number in the round-12
record is stale, which is a citation to re-derive rather than a mechanism that
moved.** `is_separator(">")` is `false` **by construction** — `is_separator` is a
bare `SEPARATORS.contains` at `policy.rs:3935-3937`.

**`segment.tokens` byte-identical**, and audit 12 verified the whole zero-diff
list by extracting each function from `dd17bfb` and from `HEAD` and comparing
digests: `is_separator`, `tokenize`, `skip_redirection_target`,
`split_segments_with_heads`, `resolve_program`, `config_key_operand_index`,
`subcommand_word_indices`, `scan_gh_api` and `forbidden_repo_path` are **all
SAME**. `git diff --numstat dd17bfb..HEAD -- tests/` is **164 additions, ZERO
deletions**.

**The byte floors and the fenced enumeration.** `POLICY_MIN_PRODUCTION_BYTES`
(40 000) and `HOOKS_MIN_PRODUCTION_BYTES` (20 000) are unchanged in value and
green. The two doc-stripping source pins each assert `///` is gone before
asserting the eight forbidden APIs are absent, and both regions are green.
**`every_fenced_unit_pin_and_fenced_corpus_row_keeps_its_answer_under_the_widened_rule`
is GREEN — 9 fenced unit pins, 8 positive controls and 20 fenced corpus words**,
read rather than accepted: the positive controls include the round's OWN class
(`of=…`, `-C…`, `PATH=/usr/bin:…`, `--opt=a=…`), so the absences are not vacuous.
**And one of its nine fenced rows is `("/tmp/envroot", "the PARENT of the
envelope directory is not under it")` — which is `T-19-123` pinned PERMITTED, by
name, with a stated reason. The code does exactly what the pin says. What is
missing is the threat row and the residue clause, not the implementation.**

### Gates observed

`rtk proxy cargo test --no-fail-fast` redirected to a file and counted with
**`rtk proxy grep`** (D-34): **1871 passed, 0 failed, 13 ignored** over **47**
result lines, `passed + failed = 1871`, matching `19-31-SUMMARY.md` exactly. **All
EIGHTEEN `envelope_*` binaries ran**, and every per-binary count matches `19-31`'s
table to the test, including **`envelope_config_resolution` at 30/0 — not one
config-resolution verdict moved** — `envelope_interior_path` at 41/0 (was 28/8)
and `envelope_wrapper_class` at 55/0 (was 54/1). **No documented flake fired in
audit 12's run** — neither `driver_reattach` row, not `envelope_tracer`'s
`ExecutableFileBusy`, and not the ETXTBSY write-over-the-binary race. **Absence is
not evidence any of them is fixed.** `cargo clippy --tests -- -D warnings` was
already failing at the base on four pre-existing lints in `src/browser.rs` and
`src/project_creator.rs` and is out of scope.

**Provenance discipline.** Plans 19-13 … 19-31's appended subsections, the
plan-19-21 blocker-row resolution record and every earlier audit's own tables are
left **byte-identical**; audit 12 checksummed the whole body below the
frontmatter before writing (`sha256 d76681753f0cd393fd…`, 931,948 bytes) and
re-verified the digest after rewriting the frontmatter. This write is append-only
apart from the frontmatter, one trail row and one method subsection. Audit 12's
corrections to statements made in those subsections — that the `SEPARATORS` line
citation is stale, that entry 7 of `CONTROL_CARRIER_INTERIOR_PATH` does not
reach, and that the `ledger.rs` grep gate is not a standing control — are
recorded as audit-12 findings BESIDE them rather than as edits to them.

### Audit 12's bookkeeping, re-derived from audit 11's group rows

| Group | Count | Closed | Open |
|---|---|---|---|
| Everything through audit 11 | 133 | 105 | 28 (7 at `high`) |
| Closed by plan 19-31, re-measured by audit 12 (`T-19-119` at `high`, `T-19-120` at `medium`) | — | +2 | −2 (1 at `high`) |
| Found by audit 12 (`T-19-122` … `T-19-126`) | 5 | 0 | 5 (2 at `high`) |
| **Total after audit 12** | **138** | **107** | **31 (8 at `high`)** |

The eight that count toward `threats_open`: `T-19-86`, `T-19-91`, `T-19-111`,
`T-19-112`, `T-19-116`, `T-19-121`, `T-19-122`, `T-19-123`. The twenty-three that
do not: `T-19-61` … `T-19-73` (13), `T-19-84`, `T-19-85`, `T-19-96`, `T-19-105`,
`T-19-110`, `T-19-113`, `T-19-115`, `T-19-124`, `T-19-125`, `T-19-126`.

---

## Audit 12 — what the round-12 control can and cannot fail on

### The principle rounds 3 through 11 established still holds

A decision region must come from the same scan the classifier runs, and there is
one walk. **The candidate scan is a scan of ONE WORD, inside the predicate that
already reads that word** — `slash_anchored_candidates` is an iterator over the
word's own byte indices, applied inside `word_is_within` and `word_is_exactly`,
and `protected_carrier_named` is still raised ONCE at the top of
`classify_segments`' existing per-segment loop. **No second reading site, no new
`ParkReason`, and `segment.tokens` byte-for-byte unchanged.** Ordering pin B is
the mechanical proof and it is green: `git push --force … && dd if=… of=<ENV>/…`
answers `force_push_blocked`, the FIRST refusal, not the carrier one. Round 3's
principle is discharged rather than weakened, for the second round running.

### Where the boundary now is, in one paragraph

**The rule decides on one lexical fact over two word classes and two paths: a
word that is LITERAL and one of whose `/`-ANCHORED SUBSTRINGS normalises to a
path under `envelope_dir_in(root, alias)` (a PREFIX) or equal to `current_exe()`
(an EQUALITY).** The interior scan is the right generalisation and the four
candidate boundaries were costed honestly. **But a boundary has three ways to be
silent, and the round's condition names only the first.**

1. **The word may not be READ** — the condition's own clause: not literal, no
   absolute path in the text, or reachable only through a link. This is the
   clause round 12 corrected, and it is now correct.
2. **The word may not be in the WORD SET.** The predicate iterates
   `segment.tokens` filtered by `!operator && literal`, chained with
   `segment.redirection_targets`. A here-string word is in neither: it is deleted
   by `consume_redirection` and recorded only when the operator takes a pathname.
   **That is `T-19-122`, and a `<<<` word deleted the whole envelope.**
3. **The word may be read, be in the set, and fail the COMPARISON while still
   naming the harm.** The PREFIX is over `<root>/<alias>`, so every ANCESTOR of
   it is shorter and answers `false`. **That is `T-19-123`, and `rm -rf <root>`
   took the same nine carriers the prefix boundary exists to protect.**

* **twelve carriers now have a rule** — `C-01` … `C-10` plus the interior and
  attachment spellings of both, verified refused at fourteen attachment
  characters;
* **five have control (e)**, no rule, registered, unaccepted — `C-11` … `C-15`;
* **all of the ruled ones are reachable by a here-string word and by an ancestor
  word**, and both were driven to harm — a bare remote's `main` moved twice, and
  a fired PR cap reset twice;
* **the mechanism that closed `T-19-119` opened a denial-of-service** on the
  guard's own critical path (`T-19-124`).

**The one-sentence version for whoever picks this up.** Twelve rounds have asked
*which words name a protected path*, and round 12 finally got the WORD reading
right — **what is left is that a path can arrive in a word the guard deletes, and
a directory can be destroyed by naming its parent.**

### Suggested closure, in order — (d) FIRST, for the twelfth round running

1. **(d) — widen the corpus BEFORE certifying anything, and this time widen the
   WORD SET and the COMPARISON rather than the word's text.** Add a here-string
   and heredoc carrier class (`xargs rm -rf <<< <path>`) and an ANCESTOR class
   (`rm -rf <root>`, `cd <root> && rm -rf <alias>`) to
   `CONTROL_CARRIER_INTERIOR_PATH`'s neighbours, each with the space-separated /
   one-component-deeper twin beside it, and a floor asserting the axis can draw
   one of each. **Listed first for the twelfth round running.**
2. **(a) — `T-19-122` FIRST among the fixes, and it is cheap.** The whole gap is
   `consume_redirection` recording a target only when
   `pathname_target && target.literal`. A here-string word is `literal` and is
   already parsed by `skip_redirection_target`; carrying it on a SECOND `Segment`
   field — a data-word class, distinct from `redirection_targets` so the
   pathname/non-pathname split stays intact — reaches it without touching
   `SEPARATORS`, `is_separator` or `segment.tokens`. The over-refusal must be
   measured from both sides before it lands (`cat <<< <ENV>/alpha` would become
   refused, which is the same family as `cat <ledger>`).
3. **(b) — `T-19-123`.** Either an ANCESTOR clause beside the PREFIX one — a
   candidate that is a proper prefix OF the envelope directory, which is a
   component-vector comparison the other way round and costs one more `if` — or
   an explicit accepted risk saying the envelope root's own parent is out of
   scope. **The unit pin at `policy.rs:10022` and
   `tests/envelope_interior_path.rs:2341` must MOVE either way**, and moving it
   is a design change, not a test edit.
4. **(c) — `T-19-124`.** Bound the candidate scan, not the input: a cap on the
   number of `/`-anchored candidates per word (or an early return once a
   candidate is shorter than the shortest protected path, which is the same
   `word.len() < dir.len()` test hoisted to the byte level) restores linearity
   without changing one verdict. Measure it against the pre-round-12 curve.
5. **(e)** — then `T-19-86`, `T-19-91`, `T-19-96`, `T-19-110`, `T-19-111`,
   `T-19-112`, `T-19-116` and `T-19-121`, which remain arms of two shapes: a
   classifier arm answering `Allow` on an operand outside the decision region,
   and a carrier that is not a command line.

Then, and separately: add the `AR-19-13` row for `T-19-17r` or stop calling it
accepted; measure `C-08`'s, `T-19-120`'s and `T-19-124`'s behavioural halves in a
live agent session; give the symlinked-to-regular ledger a standing pin; and
re-derive the `SEPARATORS` citation to `policy.rs:2422`.

---

## Audit 12 Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log — **twelve, counted**,
      unchanged by round 12; **audit 12 accepts nothing and un-accepts nothing**
- [x] Every closure re-measured against the built binary at `f06d153` with a
      fresh envelope root per row and the root walked afterwards
- [x] `T-19-119` **CLOSED on a CLASS sweep** — fourteen attachment characters
      over both protected paths, including the four spellings `19-30` recorded as
      unreached and never fenced; **the containment claim re-derived as stated
      rather than as the `i == 0` shorthand, and NO pre-existing refusal was
      lost**, verified from the source, from the mechanical pin and behaviourally
      over rounds 4–11's whole pin family with every control still permitting
- [x] `T-19-120` **CLOSED on its mechanical half** — FIFO exit 2 in 41 ms against
      exit 124 after 20.02 s, directory and character-device refused,
      **symlink-to-REGULAR still permitted and counted**, fresh root's `Err` arm
      unchanged, refusal at `EnvelopeAssertionFailed` and never `PrCapExceeded`
- [x] `T-19-121` **NARROWED HARD, NOT CLOSED** — the clause verified real and
      correctly shaped (five refused spellings, **five near-miss controls
      permitted at the same identifier**, both `--config-env` grammars reached,
      the reach re-confirmed against real git), and its declared harm
      **re-measured REACHABLE** through the `!`-bodied alias body, which returned
      the ambient `~/.git-credentials` secret on a permitted line
- [x] **THE RESIDUE TRICHOTOMY IS NOT COMPLETE**, and the two further cases are
      **driven to harm rather than argued** — a here-string word and an ancestor
      word, each moving a bare remote's `main` and resetting a fired PR cap, each
      with its refused twin and its restore control beside it
- [x] **The round's three new mechanisms probed as attack surface**; two create
      no new reachable state and **one does** — the candidate scan is QUADRATIC,
      measured against a **REBUILT PRE-ROUND-12 CONTROL BINARY** on the same input
- [x] The four execution-time judgements assessed independently; **all four calls
      were right**, and two carry a measured correction — one re-spelled corpus
      entry no longer reaches the file, and the `ledger.rs` grep gate is not a
      standing control
- [x] Mechanism pins verified non-dead over rounds 4–11 with every control still
      permitting; **`SEPARATORS` byte-identical, re-derived at its CURRENT line
      `policy.rs:2422` (ONE commit in the phase, `84a9b05`)** because the record's
      `:2297` citation is now stale; `is_separator(">")` `false` by construction;
      nine functions digest-compared `dd17bfb`..`HEAD` and all SAME;
      `tests/` **164 additions, ZERO deletions**; byte floors untouched;
      `every_fenced_unit_pin_and_fenced_corpus_row_…` **GREEN** and read for
      non-vacuity
- [x] Gate observed: `rtk proxy cargo test --no-fail-fast` → **1871 passed, 0
      failed, 13 ignored** over 47 result lines; **all EIGHTEEN** `envelope_*`
      binaries ran with every per-binary count matching `19-31`'s table; **no
      documented flake fired**
- [x] Plans 19-13 … 19-31's appended subsections left byte-identical, verified by
      checksum before AND after writing
- [ ] `threats_open: 0` confirmed — **8 open at `high`: `T-19-86`, `T-19-91`,
      `T-19-111`, `T-19-112`, `T-19-116`, `T-19-121`, `T-19-122`, `T-19-123`**
- [ ] `status: verified` set in frontmatter

**Approval: blocked, 2026-09-04 (audit 12).**

**Not accepted here.** `T-19-122` deleted an entire envelope directory with a
here-string and `T-19-123` did the same by naming the parent — each moved a bare
remote's `main` and reset a fired PR cap on one permitted command line, and both
are pre-existing rather than regressions. `T-19-121` puts the user's own git
credential back inside a driven run through an alias body. `T-19-86` and
`T-19-91` remain open at `high` by scoping decision and by round discipline;
`T-19-111` with no rule, `T-19-112` with a partial one, and `T-19-116` narrowed
rather than closed because its declared harm is still reachable. Accepting any of
the eight is a human decision and this audit does not make it. **`T-19-17r` is
left unmade for the sixteenth time, and `AR-19-04`, `AR-19-05` and `T-19-23` are
untouched.**

**Round 12 did good work and audit 12 says so plainly.** `19-30` costed four
candidate boundaries and picked the only one that is not a program-grammar
enumeration; it corrected `19-28`'s and `19-29`'s misclassification of the
`=`-joined spelling BESIDE them rather than as an edit; it found latent flakiness
in its own corpus before CI did; and it recorded `policy.rs:5743`'s `dd` example
FALSE and its own plan's `GIT_CONFIG_PARAMETERS` prediction wrong, both by
measurement. `19-31` confirmed every RED row still red before a production line
moved, landed the rule with **ZERO test deletions**, gained a reach the plan had
recorded as unreachable and reported it as a correction rather than an aim,
replaced a residue COUNT with a residue CONDITION on audit 11's own
recommendation, and repaired a false honesty claim by naming the rule that acts
instead of asserting that a layer governs. **Two threats close on this audit's
measurement, and both closures are real.**

**And the answer to the twelfth question is a plain no.** Audit 11 asked whether
the residue condition was finally complete and this audit probed the trichotomy
for a fourth case, as it was told to. **There are two, and neither is a spelling
— they are the other two ways a boundary can be silent.** Round 11 stated the
residue over what the predicate reads and omitted a word class. Round 12
corrected the reading and inherited the omission, then stated the condition over
what the predicate reads AGAIN — *"in either word class"* — while the code's word
set has a hole in it and the code's comparison has a floor under it. **The
corpus that certifies the round draws eighty-four rows across twelve classes and
not one of them is a here-string or an ancestor** — so the axis was, for the
twelfth consecutive round, structurally incapable of failing on the class beside
the one the round certified. **The residue is not one character short this time.
It is one CLAUSE short, and the clause is not about reading.**

**And the last thing this audit can usefully say: the open set is NOT yet the
human-decision items plus the deliberately-unruled residues.** Five of the eight
blocking rows are that — `T-19-86` (scoping decision), `T-19-91`, `T-19-111`,
`T-19-112` and `T-19-116` are all registered, disclosed, and open by decision or
by round discipline, and `T-19-121` is a bounded residue of a control that
genuinely landed. **But `T-19-122` and `T-19-123` are neither.** They are
unmodelled classes with reachable, driven harm, absent from every residue
statement, every corpus alphabet and every threat row until this audit. **When
those two are ruled or accepted, the sentence becomes true — and it will be worth
writing then.**
