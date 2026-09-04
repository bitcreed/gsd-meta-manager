---
phase: 19
slug: gitsafe-git-blast-radius-envelope
status: blocked
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (high)
threats_open: 4
asvs_level: 1
block_on: high
created: 2026-08-29
updated: 2026-09-03
register_authored_at_plan_time: true
audited_against: HEAD (3110d4a) — the SEVENTH audit judges the tree after plans
  19-20 (the callee-grammar corpus, RED) and 19-21 (the fail-closed rule), plus
  the scoped follow-up 989f21a that resolved 19-21's blocker row. Audit 6 judged
  9534198 (after 19-18/19-19); audit 5 judged ad847b4 (after 19-16/19-17); audit
  4 judged b72237e (after 19-14/19-15); audit 3 judged 228e4bc (after 19-13);
  audit 2 judged b8605ef (after 19-11/19-12); audit 1 judged 0ec1fcb.
register_totals: 119 total / 96 closed / 23 open / 4 at or above `high`
# the four blocking: T-19-86, T-19-91, T-19-103, T-19-104
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
