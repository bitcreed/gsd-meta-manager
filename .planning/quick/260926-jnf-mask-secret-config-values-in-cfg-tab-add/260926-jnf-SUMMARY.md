---
phase: quick-260926-jnf
plan: 01
subsystem: ui/config-tab, state_reader
status: complete
tags: [security, secrets, config, gsd-core-sync, tui]
requires: [quick-260926-gtk, quick-260926-j0a]
provides:
  - src/state_reader/config_secrets.rs (secret classification + masking)
  - ApiKeySetting typed search-provider slots
  - 7 promoted top-level Defaults rows
affects: [docs/GSD-CORE-SYNC.md, docs/CONFIGURATION.md, README.md]
tech-stack:
  added: []
  patterns:
    - "ConfigEntry.value for a secret path is built only from masking helpers (structural, not render-time)"
    - "untagged serde enum with hand-written redacting Debug"
key-files:
  created:
    - src/state_reader/config_secrets.rs
  modified:
    - src/state_reader/mod.rs
    - src/state_reader/config_json.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - docs/GSD-CORE-SYNC.md
    - docs/CONFIGURATION.md
    - README.md
decisions:
  - "I-1 mask is a fixed '•••••••• (set)' (no last-4, no length); booleans shown verbatim; null/''/missing -> (unset)"
  - "I-2 kg_backend is a mempalace.memory_mode value, not a key (re-verified); no row; 7 rows, 132 -> 139"
  - "I-3 explicit secret list = upstream code (3) + upstream docs Search API keys table (4) = 7"
  - "I-13 upstream `context` is a real top-level key but its type is self-contradictory upstream; left pass-through, count stays 139"
metrics:
  duration: "~19 min"
  completed: 2026-09-26
plan_head_before: f71a4e6fcb9eaaa93eb3d85e0114fe15ecb40738
actuals:
  tokens: 24000
  tasks: 3
  commits: 8
---

# Quick 260926-jnf Plan 01: Mask secret config values in the Cfg tab + typed rows for the remaining gsd-core top-level keys Summary

No secret config value reaches a terminal cell, the prompt, a status message or the log any more: search-provider API keys and heuristic-named keys render as a fixed `•••••••• (set)`, the secret prompt opens empty and echoes bullets, and the seven real top-level gsd-core keys that were still pass-through are now typed rows (`DEFAULTS_OPTION_COUNT` 132 -> 139).

## Commits

| # | Task | Commit | Type |
|---|------|--------|------|
| 1 | Task 1 RED | 8282c7b | test: failing secret-slot parse, render and edit tests |
| 2 | Task 1 GREEN | cade09d | feat: mask search-provider API keys end to end |
| 3 | Task 2 RED | f1a5809 | test: failing heuristic, nested, inherited and log tests |
| 4 | Task 2 GREEN | 85fa304 | feat: mask pass-through, nested and logged values |
| 5 | Task 2 docs | c3537b2 | docs: CONFIGURATION.md section + README Config Enter row |
| 6 | Task 3 RED | 4cdcbff | test: failing promoted top-level key tests (count left at 132) |
| 7 | Task 3 GREEN | 845a706 | feat: 7 typed keys, count 139, GSD-CORE-SYNC.md rewritten (same commit: `git show --stat` lists detail.rs AND docs/GSD-CORE-SYNC.md) |
| 8 | Gate fix | f660526 | fix: ApiKeySetting Debug without the guarded `.finish(` call name |

## What was built

- **`src/state_reader/config_secrets.rs`** (new, pure functions): `SECRET_CONFIG_KEYS` (7), `NON_SECRET_CONFIG_KEYS` (4 measured budget exemptions, exact or `key.` prefix), `SECRET_NAME_MARKERS`, `MASKED_SECRET`, `UNSET_LABEL`, `is_secret_key`, `mask_json_value`, `redact_json_for_display`, `display_config_value`.
- **`ApiKeySetting { Flag(bool), Key(String) }`** (untagged, `Flag` first, redacting `Debug` -> `Key(<redacted>)`) types all 7 search slots. A string key in `brave_search`/`firecrawl`/`exa_search` used to fail the whole file; it now parses and round-trips byte-identically.
- **`ConfigValueKind::Secret`**: rows built via `opt_secret_layered` (masked in the project layer, the inherited ` *` layer and the Global view). `opens_text_prompt()` gates the text intercept and popup; the Secret Enter branch seeds `EditBuffer::default()`; the popup draws `EditBuffer::masked()`; `set_secret_value` implements I-6. `Secret => false` in `mutate_config_entry`; no dropdown.
- **Masking wherever a value is shown**: pass-through rows and every read-only JSON row go through `display_config_value` (`opt_json_readonly` now takes the row's JSON path; `json_display` was only used by those two sites and is folded into the helper).
- **Log**: `parse_gsd_config` logs `parse_error_summary` (category + line/column) instead of serde's Display.
- **7 promoted rows**: `tavily_search`/`ref_search`/`perplexity`/`jina` (Secret, Features, since v1.4.0); `runtime` (String), `context_profile` (Enum dev/research/review), `agent_skills` (ReadOnly) in Model & Pipeline after `model_profile` (since v1.01.0).
- **Docs**: docs/CONFIGURATION.md `## Secret values in GSD project config (Config tab)` + Related-files entry; README Config `Enter` row; docs/GSD-CORE-SYNC.md rewritten (promotion table, `## Secret-bearing keys`, kg_backend correction, re-measured buckets, 139 Modelled rows, next-sync secret re-diff).

## TDD Gate Compliance / red evidence

- **Task 1 RED (8282c7b)**: did not compile, 26 errors: E0425 (`is_secret_key`, `mask_json_value`, `redact_json_for_display`, `MASKED_SECRET`, `SECRET_CONFIG_KEYS`, `NON_SECRET_CONFIG_KEYS`, `UNSET_LABEL`), E0432/E0433 (`ApiKeySetting`), E0599 (`EditBuffer::char_count`). GREEN: `cargo test --lib secret` 10/10.
- **Task 2 RED (f1a5809)**: did not compile, 1 error: E0425 `parse_error_summary`. After adding only `parse_error_summary`, two tests failed at runtime before the display wiring: `the_secret_heuristic_masks_unknown_pass_through_keys` and `a_read_only_json_row_redacts_nested_secret_members`. The inherited/Global test and the census passed already, as the plan's step 4 predicted (Task 1's `opt_secret_layered` covers them).
- **Task 3 RED (4cdcbff)**: did not compile: E0609 missing fields `tavily_search`, `ref_search`, `perplexity`, `jina`, `runtime`, `context_profile`, `agent_skills`. After GREEN code, and before the doc rewrite, three tests failed as expected: the Enum-row pin (measured 13 vs 12), `the_sync_record_names_every_modelled_key_and_the_measured_baseline`, and `the_sync_record_names_every_secret_key_and_exemption`. All three were fixed in the same GREEN commit.

## Gate results

- `rtk proxy cargo test --no-fail-fast`: 56 `test result` lines (all suites ran). **2773 passed, 1 failed, 15 ignored.** The only failure is the permitted witness `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (local git 2.53.0 vs pinned 2.55.0). The first full run also failed `tests/spawn_seam_guard.rs::every_terminal_write_in_the_driver_run_goes_through_the_stamped_helper`, because the Debug impl's `debug_tuple(..).finish()` tripped the source guard. Fixed in f660526; see Deviations. The ETXTBSY flake did not appear.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: exit 0.
- New tests: 17 in total. 5 in config_secrets, 4 in config_json (search slot, parse-error summary, promoted keys, plus a `Flag(false)` assertion added to an existing fixture test), and 8 in detail.rs.
- The upstream checkout is untouched: `git -C ~/projects/node/gsd-core status --short` is empty and HEAD is still `next`. Only `git show`/`log`/`grep`/`tag` were used against `upstream/release-1.15.0`.

## Re-measured step-3 buckets (upstream `ec81d0d` vs this build)

Step 2 finds 260 core key cells and 139 GMM row keys. Step 3 gives **162 lines, 113 of them dotted**. That matches the plan's expected 162/113, but the undotted split does not match the planner's expectation:

| Bucket | Count |
|---|---:|
| Undotted rows that are not config keys (values, headings, PR-body entry fields, STATE.md frontmatter fields; now includes `kg_backend`) | 48 |
| Undotted top-level keys, pass-through | **1 (`context`)** |
| Dotted value rows | 3 |
| Modelled under unprefixed labels | 30 |
| Pass-through families | 80 |

The planner expected 49 value rows and 0 top-level keys. It inherited gtk's bucketing, which had counted `context` as a value row.

## INFERRED decisions (for audit)

- **INFERRED I-1**: The mask is `•••••••• (set)`: eight bullets, no suffix, no length. null, missing and `""` show `(unset)`. A boolean shows verbatim. Any other value under a secret key shows the mask. This diverges from upstream's `****<last-4>` on purpose.
- **INFERRED I-2**: `kg_backend` is not a key. I re-verified this with `git grep` on `upstream/release-1.15.0`: it appears only as an enum value of `mempalace.memory_mode` (`capabilities/mempalace/capability.json:36`, CONFIGURATION.md:1083/1098, CONTEXT.md, mempalace docs) and never as a config key. It gets no row, gtk's I-8 is corrected in the sync record, and there are 7 new rows, so 132 -> 139.
- **INFERRED I-3**: The explicit list is 7 keys: upstream code `src/secrets.cts` (3) plus the upstream docs "Search API keys" table (4). The gap is recorded in the sync record.
- **INFERRED I-4**: The heuristic markers are `api_key`/`apikey`/`api-key`/`token`/`secret`/`password`/`passwd`, matched case-insensitively against the full dotted path. The 4 measured exemptions match as the exact path or a `key.` prefix. A census test (`no_authored_row_trips_the_secret_heuristic_unmasked`) guards authored rows.
- **INFERRED I-5**: One untagged `ApiKeySetting` types all 7 slots. A number or object in a slot is a whole-file parse failure, the accepted gtk I-5 divergence. `Key("")` displays `(unset)`.
- **INFERRED I-6**: The secret prompt opens with an empty buffer and shows one bullet per character. Input is trimmed on commit. Empty input is a no-op with the status `<key> unchanged — type a new value, or x to clear`. Exactly `true`/`false` stores a `Flag`; anything else stores `Key`. `x` clears. The value never appears in a status message. The title keeps `DEFAULTS_EDIT_BRANCH_TOKEN`.
- **INFERRED I-7**: `since` for `runtime`, `context_profile` and `agent_skills` is v1.01.0; for the four search keys it is v1.4.0 (11afca296). I re-measured both with the recipe, and they match the planner. `brave_search`/`firecrawl`/`exa_search` keep `""`.
- **INFERRED I-8**: Placement is the four search keys after `exa_search` (Features) and `runtime`/`context_profile`/`agent_skills` after `model_profile` (Model & Pipeline).
- **INFERRED I-9**: `runtime` is a free-form String row. Its help names claude/codex and says this manager's own driver does not read it.
- **INFERRED I-10**: `context_profile` is an Enum with one clause per choice, taken from upstream `docs/features/execution-context-profiles.md` REQ-CTX-01..03. From unset, a toggle gives `dev`, not `cycle`'s `research`.
- **INFERRED I-11**: `agent_skills` is a ReadOnly row via `opt_json_readonly("agent_skills", …)` with no set/clear/toggle arm. Nested members are redacted.
- **INFERRED I-12**: A heuristic-masked pass-through row stays ReadOnly (asserted in the heuristic test).
- **INFERRED I-13 (new, executor)**: Upstream `context` (CONFIGURATION.md:255, in `validKeys`) is a real top-level key that the plan and gtk treated as a value row. Upstream contradicts itself on its type: the docs say free-form string, while `src/config.cts:903-904` `config-set` enforces enum dev/research/review. I did not add a row, because that would mean guessing a type and moving the plan's pinned count. It stays a visible, lossless `Not modelled` row. It is recorded in GSD-CORE-SYNC.md ("Still pass-through at the top level", the Pass-through table, and a next-sync item). `DEFAULTS_OPTION_COUNT` stays 139.
- **INFERRED I-14 (new, executor)**: The secret value style (`val_style`) is Magenta, which is distinct from the Yellow string values.
- **INFERRED I-15 (new, executor)**: Help text for the search-provider rows names the upstream env var and `~/.gsd/<x>_api_key` file for each provider (read from upstream CONFIGURATION.md). `jina`'s help notes that unset counts as available.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `ApiKeySetting` Debug tripped the spawn-seam source guard**
- **Found during:** the full gate after Task 3.
- **Issue:** `tests/spawn_seam_guard.rs::every_terminal_write_in_the_driver_run_goes_through_the_stamped_helper` rejects any `.finish(` in `src/`, and `f.debug_tuple("Flag").field(b).finish()` matched it.
- **Fix:** `write!(f, "Flag({b})")`, which gives identical output, plus a comment naming the guard.
- **Files modified:** src/state_reader/config_json.rs
- **Commit:** f660526

### Other notes

- `display_config_value` landed in config_secrets.rs in the Task 1 GREEN commit, not Task 2. It was unused until Task 2 wired it in. This had no behavioural effect.
- **Existing tests that assumed Bool rows for `brave_search`/`firecrawl`/`exa_search`: none existed.** No test pinned a Bool-kind count, and every test that walks bool rows derives its set from `all_config_entries()`, so none needed updating. The populated fixture's `"brave_search": false` still parses, as `Flag(false)`. The only pinned kind count that moved is the Enum-row pin, 12 -> 13 (measured, for `context_profile`).

## Threat-model dispositions

| Threat | Disposition | Outcome |
|---|---|---|
| T-jnf-01 value in cells | mitigate | Done. Every secret `ConfigEntry.value` comes from masking helpers. TestBackend tests cover typed, 4 promoted, pass-through, nested, inherited and Global cases. |
| T-jnf-02 prompt prefill/echo | mitigate | Done. The prompt seeds `EditBuffer::default()` and renders `masked()`, proven through real `handle_key` plus a draw. |
| T-jnf-03 log quotes value | mitigate | Done. `parse_error_summary`; the test proves the raw Display leaks and the summary does not. |
| T-jnf-04 derived Debug | mitigate | Done. Hand-written Debug prints `Key(<redacted>)`, pinned by the `{config:?}` test. |
| T-jnf-05 parse failure / lossy save | mitigate | Done. The string key parses, and the byte-identical round trip is pinned. |
| T-jnf-06 empty Enter wipes key | mitigate | Done. Empty Enter is a no-op with an `unchanged` status. |
| T-jnf-07 innocent names | accept | Documented in CONFIGURATION.md and GSD-CORE-SYNC.md. |
| T-jnf-08 typed length visible | accept | One bullet per char while typing. The stored value is fixed-width. |
| T-jnf-09 plaintext at rest | transfer | Documented in CONFIGURATION.md. |

No new threat surface beyond the plan's register.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: src/state_reader/config_secrets.rs, docs/GSD-CORE-SYNC.md (contains `260926-jnf`, `## Secret-bearing keys`), docs/CONFIGURATION.md (contains `•••••••• (set)`)
- FOUND commits: 8282c7b, cade09d, f1a5809, 85fa304, c3537b2, 4cdcbff, 845a706, f660526
- No stray worktrees or branches (`git worktree list` shows the main tree only; branches are `dev` and `master`). Nothing was pushed.
