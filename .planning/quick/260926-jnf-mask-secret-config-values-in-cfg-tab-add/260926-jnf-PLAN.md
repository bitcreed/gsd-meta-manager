---
phase: quick-260926-jnf
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/config_secrets.rs
  - src/state_reader/mod.rs
  - src/state_reader/config_json.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/detail.rs
  - docs/GSD-CORE-SYNC.md
  - docs/CONFIGURATION.md
  - README.md
autonomous: true
requirements: [QUICK-260926-jnf]

estimate:
  tokens: 90000
  raw_tokens: 90000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "No raw value of a secret-classified key ever reaches a terminal cell. This holds for: Defaults typed rows; pass-through (`Not modelled`) rows; secret members nested inside a read-only JSON value; values inherited from ~/.gsd/defaults.json (the ` *` rows); and the Global (`d`) view. It is proven by rendering into a ratatui TestBackend and asserting that neither the full secret nor its last 4 characters appear, while the row key and the mask marker DO appear."
    - "A set secret renders as `•••••••• (set)`: exactly eight bullets, with no suffix and no length. A null, missing or empty-string value renders as `(unset)`. A boolean in a search-provider slot renders as `true`/`false` (INFERRED I-1)."
    - "Enter on a secret row opens the text prompt with an EMPTY buffer, never the old value. Typed characters are drawn as one `•` each. Typing exactly `true`/`false` stores a JSON boolean; any other non-empty (trimmed) input stores the key string. An empty Enter changes nothing. `x` still clears. No status message contains the value (INFERRED I-6)."
    - "A project whose `brave_search`/`firecrawl`/`exa_search` holds an API-key STRING now parses. Before this change it was a whole-file parse failure that drew no rows. Serialize -> parse -> serialize is byte-identical."
    - "`parse_gsd_config`'s tracing warning carries only the serde error category plus line/column, never the serde Display (which quotes the offending value)."
    - "The heuristic masks an unknown `foo_api_key` pass-through row and a nested `integrations.github_token` member. It does NOT mask the four measured upstream `*_tokens` budget keys (`review.max_prompt_tokens`, `review.max_prompt_tokens_per_reviewer[.<slug>]`, `statusline.show_context_tokens`, `workflow.smart_zone_tokens`)."
    - "`tavily_search`, `ref_search`, `perplexity` and `jina` (secret, since v1.4.0), `runtime` (string, since v1.01.0), `context_profile` (enum dev/research/review, since v1.01.0) and `agent_skills` (read-only, since v1.01.0) each have exactly one typed Defaults row with a ConfigHelp. `DEFAULTS_OPTION_COUNT` is 139. `kg_backend` gets no row (INFERRED I-2)."
    - "docs/GSD-CORE-SYNC.md is rewritten IN THE SAME COMMIT as the count bump. It lists all 139 Modelled keys, the secret-key list with provenance, the kg_backend correction and the recounted step-3 buckets."
    - "`rtk proxy cargo test --no-fail-fast` has no failure other than the src/envelope/policy.rs git-version witness, and `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0."
  artifacts:
    - path: "src/state_reader/config_secrets.rs"
      provides: "SECRET_CONFIG_KEYS (7), NON_SECRET_CONFIG_KEYS (4 measured exemptions), SECRET_NAME_MARKERS, is_secret_key, MASKED_SECRET, mask_json_value, redact_json_for_display"
      contains: "pub const SECRET_CONFIG_KEYS"
    - path: "src/state_reader/config_json.rs"
      provides: "ApiKeySetting untagged enum {Flag(bool), Key(String)} with redacting Debug; 7 search-provider slots typed with it; runtime/context_profile/agent_skills fields; value-free parse-error log"
      contains: "pub enum ApiKeySetting"
    - path: "src/ui/screens/detail.rs"
      provides: "ConfigValueKind::Secret, secret layered accessor, set_secret_value, masked edit popup, masked pass-through/read-only JSON display, 7 new rows, PROMOTED_TOP_LEVEL_KEYS, DEFAULTS_OPTION_COUNT 139, TestBackend secret-leak tests"
      contains: "ConfigValueKind::Secret"
    - path: "src/ui/screens/mod.rs"
      provides: "EditBuffer::char_count and EditBuffer::masked (bullets, no content)"
      contains: "fn masked"
    - path: "docs/GSD-CORE-SYNC.md"
      provides: "Rewritten sync record: 139 Modelled rows, promoted top-level keys, secret-bearing keys section, kg_backend reclassification"
      contains: "260926-jnf"
    - path: "docs/CONFIGURATION.md"
      provides: "User-facing description of secret masking and secret editing in the Config tab"
      contains: "•••••••• (set)"
  key_links:
    - from: "src/ui/screens/detail.rs build_defaults_entries / append_passthrough_entries / opt_json_readonly"
      to: "src/state_reader/config_secrets.rs"
      via: "ConfigEntry.value is built ONLY from masked text for secret keys, so no render, prefill or status path can reach the raw value"
      pattern: "config_secrets::"
    - from: "src/ui/screens/detail.rs handle_text_input_key (Enter)"
      to: "set_secret_value"
      via: "Secret-kind branch: empty input -> no-op, true/false -> Flag, else Key"
      pattern: "set_secret_value\\("
    - from: "src/ui/screens/detail.rs the_sync_record_names_every_modelled_key_and_the_measured_baseline"
      to: "docs/GSD-CORE-SYNC.md"
      via: "reads the committed doc through CARGO_MANIFEST_DIR; every row key must appear backticked under ## Modelled"
      pattern: "## Modelled"
---

<objective>
Stop the Config (7:Cfg) tab from ever displaying a secret config value, and give the remaining real top-level gsd-core keys typed Defaults rows. Secrets must be masked everywhere the TUI draws a config value: typed rows, pass-through rows, nested JSON, inherited defaults, the Global view, the edit prompt, status messages and tracing logs. The newly typed search-provider rows must themselves be masked secret rows.

Purpose:
- Today, a search-provider API key stored in `.planning/config.json` fails in one of two ways:
  - It is drawn verbatim on a `Not modelled` row: `tavily_search`, `ref_search`, `perplexity`, `jina`.
  - It makes the whole file unparseable and is quoted into the log: `brave_search`, `firecrawl`, `exa_search` are typed `Option<bool>` here, but upstream they are `string | boolean | null`.
- The README ships screenshots of this TUI, so any value it draws should be assumed to end up in one.

Output: the new `src/state_reader/config_secrets.rs`; modified config_json.rs, screens/mod.rs and detail.rs; a rewritten docs/GSD-CORE-SYNC.md; updated docs/CONFIGURATION.md and README.md; and `260926-jnf-SUMMARY.md`.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/quick/260926-gtk-config-re-sync-to-gsd-core-1-15-0-release-1-15-0-ec81d0d-add/260926-gtk-SUMMARY.md
@docs/GSD-CORE-SYNC.md
@src/state_reader/config_json.rs

Run context: quick task, sequential on the primary checkout. No worktrees, never push. Commit with the executor's normal atomic TDD flow: a failing `test(quick-260926-jnf): …` commit first, then `feat(…)`/`docs(…)`. The human is unavailable. Decide from the facts below and record every inferred decision with an `INFERRED:` marker in the SUMMARY; the I-n list below is the starting set.

<upstream_facts>
The planner MEASURED these on 2026-09-26. Re-verify cheaply; never re-derive them from memory.
- The upstream checkout ~/projects/node/gsd-core is READ-ONLY. Use only `git -C ~/projects/node/gsd-core show|log|grep|tag|describe <ref>` against REF=`upstream/release-1.15.0` (ec81d0d10f545dd5aea2cc893863a542bc49029d). Never check out a branch there.
- Its working-tree HEAD is not the sync target. `GSD_CORE_SYNCED_COMMIT` and the other constants DO NOT change in this task.
- **Upstream secret list, in code:** `git show $REF:src/secrets.cts` gives `SECRET_CONFIG_KEYS = {brave_search, firecrawl, exa_search}`.
  - `maskSecret`: null/undefined/'' -> `(unset)`; fewer than 8 chars -> `****`; otherwise `****` + last 4.
- **Upstream secret list, in docs:** `git show $REF:docs/CONFIGURATION.md`, section `### Search API keys` (lines ~274-288), documents SEVEN keys as "Masked in display":
  - `brave_search`, `firecrawl`, `exa_search`, `tavily_search`, `ref_search`, `perplexity`, `jina`.
  - Each has type `string | boolean | null`, default `null`.
  - A string is the API key. `true`/`false`/`null` override auto-detection from `<X>_API_KEY` env or `~/.gsd/<x>_api_key`.
  - So the code and the docs disagree by 4 keys. The docs' promise is the one this build honours (INFERRED I-3).
- **Heuristic false positives.** Upstream key cells matching the markers `api_key|apikey|api-key|token|secret|password|passwd`, i.e. `git show $REF:docs/CONFIGURATION.md | grep -oP '^\| \`\K[a-z0-9_.<>-]+(?=\`)' | sort -u | grep -iE '<markers>'`, are exactly four, all numeric/boolean budget keys. None of them is a secret:
  - `review.max_prompt_tokens`
  - `review.max_prompt_tokens_per_reviewer`
  - `statusline.show_context_tokens`
  - `workflow.smart_zone_tokens`

  The schema manifest's `validKeys` has the same four matches.
- **The 8 keys from 260926-gtk I-8.** Only SEVEN are config keys:
  - `runtime`: string; `claude`, `codex` or any string; no default. In validKeys.
  - `context_profile`: string enum `dev`/`research`/`review`; no default. Not in validKeys, but documented at CONFIGURATION.md:252.
  - `agent_skills`: object, default `{}`, a map of agent types to arrays of skill entries. The defaults manifest has `"agent_skills": {}`; dynamic key pattern `agent_skills.<agent-type>`.
  - `tavily_search`, `ref_search`, `perplexity`, `jina`: see "in docs" above. Upstream `jina`'s effective default is "available".
  - **`kg_backend` is NOT a key.** It is an enum VALUE of the capability-registered `mempalace.memory_mode` (`capabilities/mempalace/capability.json:36`). The step-3 grep caught it in CONFIGURATION.md:1098's "Memory modes in detail" value table (INFERRED I-2).
- **Measured `since`**, using the sync record's step-5 recipe `git log $REF --reverse --format=%H -S"$K" -- docs/CONFIGURATION.md src/ capabilities/ | head -1` then `git tag --contains <c> --sort=v:refname | head -1`:
  - `runtime`: d478e7f48, v1.01.0.
  - `context_profile`: 641ea8ad4, v1.01.0.
  - `agent_skills`: db3eeb8fe, v1.01.0.
  - `tavily_search`, `ref_search`, `perplexity`, `jina`: 11afca296, "feat(#656): Research module — content-addressed cache + provider seam", v1.4.0. Their CONFIGURATION.md table rows arrived later, at fd576528a (v1.7.0). The recipe's earliest-commit rule wins (INFERRED I-7).
  - `v1.01.0` is gsd-core's oddly-spelled first tag, which the record already documents.
- **The manager ignores gsd-core's own `runtime` key (ID-2).** `tests/driver_codex_runtime.rs:285-295` pins that. The manager's per-project runtime lives in its own registry (`src/config.rs` `RUNTIME_KEY`). Nothing reads `GsdConfig.extra["runtime"]` (grep: only test code touches `.extra.get(`).
- **Expected step-3 recount** after this task: 169 -> 162 lines, still 113 dotted. The buckets become:
  - undotted value rows 48 -> 49 (+`kg_backend`);
  - undotted top-level pass-through 8 -> 0;
  - dotted value rows 3;
  - unprefixed labels 30;
  - pass-through families 80.

  RE-MEASURE this; do not copy it.
</upstream_facts>

<code_anchors>
Line numbers are pre-edit; locate by identifier if they drift. detail.rs is 22.5k lines, so read only the ranges named here.
- **config_json.rs**
  - 77-155: `GsdConfig`. Lines 91-96 hold `brave_search`/`firecrawl`/`exa_search: Option<bool>` with `#[serde(default)]`, and no `skip_serializing_if`. Keep that attribute shape for those three.
  - 538-548: `parse_gsd_config`. Line 544 is the ONLY tracing call in the tree that touches a config value (`tracing::warn!` with the serde error's Display).
  - ~600-615: the fixture carrying `"brave_search": false`.
- **detail.rs, types and render**
  - 9425-9455: `ConfigValueKind` and `editable()`.
  - 9683-9707: `ConfigEntry` (`value: String` is the ONE field every render reads).
  - 9709-9773: the `opt_*_layered` helpers, `json_display`, `opt_json_readonly`.
  - 7016-7340: `render_defaults_tab`.
    - 7105-7122: `val_style` match.
    - 7136: `val_span`.
    - 7238-7294: the String edit popup, which renders `cache.defaults_text_buffer.shown()`.
- **detail.rs, key handling**
  - 1141-1151: `config_text_edit_index`, which gates the text intercept on `ConfigValueKind::String`.
  - 4265-4328: the Defaults Enter handler.
    - 4301-4316: the String branch that SEEDS the buffer from `entry.value`.
  - 4460-4497: the `x` clear handler. It echoes the key only.
  - 5420-5486: `handle_text_input_key`; the Enter commit is at 5446.
- **detail.rs, row builders and mutation arms**
  - 9775 onwards: `build_defaults_entries`.
    - `push` closure: 9785.
    - Features `brave_search`/`firecrawl`/`exa_search` rows: 10232-10243.
    - Model & Pipeline starts at 10246; `model_profile` push at 10262.
  - 10612-10763: `append_passthrough_entries`; pass-through values come from `json_display(&value)` at 10755.
  - 10980 `set_config_value`: the bool arms for the three search keys are at 10998-11000; the enum string arms are at ~11060-11122.
  - 11128 `set_string_value`.
  - 11178 `clear_config_value`: 11192-11194.
  - 11333 `mutate_config_entry`:
    - Bool toggles: 11352-11354.
    - Enum `cycle` closure: ~11434. Note that `unwrap_or(0)` makes an unknown current advance to option 1.
    - ReadOnly -> false: 11612.
- **detail.rs, tests**
  - 11641-11804 `populated_gsd_config`: a JSON fixture with nothing unset and NO unmodelled keys.
  - 11824 `DEFAULTS_OPTION_COUNT = 132`, with the doc comment above it.
  - 11864 `every_choice_bearing_entry_documents_exactly_its_dropdown_options`, which pins the Enum-row count (12 today).
  - 11984 `render_defaults_config_to_text(config, w, h, selected)`.
  - 12597 `RESYNCED_KEYS`, 12672 `RESYNCED_KEYS_1_15_0`.
  - 12689 `the_sync_record_names_every_modelled_key_and_the_measured_baseline`.
  - 12789 `the_resync_covers_every_key_the_drift_measurement_found`.
  - 12809 `kind_name` (exhaustive).
  - 12826 and 12872: the two per-key loops. The second has per-kind arms bool/enum/string/integer/readonly.
  - 18322 `ctx_on_config_row(config, key)`, 18482 `draw_config_tab(screen, ctx, w, h)`.
  - 18521 `config_filter_enter_on_a_filtered_unset_row_opens_that_rows_chooser`: the handle_key-driven test shape to copy.
- **src/ui/screens/mod.rs**
  - 865-924: `EditBuffer` (a wrapper over `crate::text::Untrusted`).
- **src/ui/screens/render_escape_guard.rs**
  - `hostile_gsd_config` (1424) uses `..Default::default()`, so new `GsdConfig` fields compile there.
  - `first_string_entry` matches `ConfigValueKind::String` exactly. Secret rows must NOT be String kind, or the probe would seed a masked value.
- **Surfaces searched:**
  - No config "diff" view exists.
  - Status messages on the Config tab echo only keys (`Cleared {key}`, `{key} cannot be cleared`) or static labels.
  - The `/` filter matches key and category only.
  - So `ConfigEntry.value` plus the edit buffer are the whole render surface, along with the one tracing call.
</code_anchors>

<inferred_decisions>
The executor must copy these into the SUMMARY under `INFERRED`, including any it revises.
- **I-1 Masking format** is `•••••••• (set)`: a fixed eight bullets, no suffix, no length.
  - null / missing / `""` -> `(unset)`. A boolean in a secret slot is shown verbatim, since it is the documented auto-detection override and carries no secret. Any other non-null value under a secret key (string, number, array, object) -> `•••••••• (set)`.
  - This diverges on purpose from upstream's `****<last-4>`:
    - The last 4 characters leak 16-24 bits of key entropy into screenshots.
    - A fixed width also hides the key's length and provider format.
    - No action in this TUI needs to tell two keys apart. Replacing a key means typing the new one.
- **I-2** `kg_backend` is a `mempalace.memory_mode` value, not a key. It gets no row, and gtk's I-8 is corrected in the sync record. That makes 7 new rows, so `DEFAULTS_OPTION_COUNT` goes 132 -> 139.
- **I-3** The explicit secret list is 7 keys: upstream code (3) plus upstream docs' "Search API keys" table (4). The code/docs gap upstream is recorded in the sync record.
- **I-4 Heuristic.**
  - Markers: `api_key`, `apikey`, `api-key`, `token`, `secret`, `password`, `passwd`. It is a case-insensitive substring match on the FULL dotted path.
  - It applies to keys this build does not model: pass-through rows, and member paths inside any read-only JSON value.
  - Measured exemptions are `NON_SECRET_CONFIG_KEYS` (the 4 keys above), matched as the exact path or as a `<key>.` prefix.
  - Authored rows are classified explicitly. A census test makes it impossible to add an authored row whose key trips the heuristic unless it is Secret-kind or exempt.
- **I-5** All 7 search-provider slots are typed with one untagged `ApiKeySetting { Flag(bool), Key(String) }`, and the 3 existing Bool rows become Secret rows.
  - A non-bool, non-string value (number, object) in a slot is a whole-file parse failure, the same accepted divergence as gtk I-5. Untagged-enum errors carry no value.
  - `Key("")` displays `(unset)`, matching upstream `maskSecret('')`.
- **I-6 Secret prompt.**
  - Enter opens an empty buffer, and each typed character is echoed as one `•`.
  - On commit the input is trimmed. Empty -> no-op, with the status `<key> unchanged — type a new value, or x to clear` (key via `shown()`).
  - Exactly `true`/`false` -> `Flag`. Anything else -> `Key(trimmed)`.
  - `x` clears as before. The existing "Config saved" status is kept, and no status carries the value.
  - The popup title keeps `DEFAULTS_EDIT_BRANCH_TOKEN`.
- **I-7 `since` values:**
  - `runtime`, `context_profile`, `agent_skills`: v1.01.0.
  - The four search keys: v1.4.0.
  - `brave_search`/`firecrawl`/`exa_search` keep `""` (they predate the record and are not retro-measured, the existing policy).
- **I-8 Placement:**
  - `tavily_search`, `ref_search`, `perplexity`, `jina` go in Features directly after `exa_search`.
  - `runtime`, `context_profile`, `agent_skills` go in Model & Pipeline directly after `model_profile`.
- **I-9** `runtime` is a free-form String row. Its help names `claude`/`codex` and says this manager's own agent driver does not read it.
- **I-10** `context_profile` is an Enum row with choices dev/research/review (with_choices, one clause each). Upstream has no default, so its toggle arm maps unset -> `dev` (the first option) rather than through `cycle`, which would skip to `research`.
- **I-11** `agent_skills` is a ReadOnly row via `opt_json_readonly`, like `review.reviewer_instances`: no set/clear/toggle arm, and the per-key loop's readonly arm asserts the refusal. Its JSON display goes through the nested redaction.
- **I-12** Heuristic-masked pass-through rows stay ReadOnly. Masking does not make them editable.
</inferred_decisions>
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1 (tracer): `brave_search` holding an API key, end to end — parses, renders masked, edits through an empty masked prompt, persists</name>
  <files>src/state_reader/config_secrets.rs, src/state_reader/mod.rs, src/state_reader/config_json.rs, src/ui/screens/mod.rs, src/ui/screens/detail.rs</files>
  <behavior>
    - config_secrets.rs unit tests (names contain `secret`):
      - `is_secret_key` is true for each of the 7 listed keys, for `foo_api_key`, `FOO_API_KEY`, `x.apiKey`, `github_token`, `db_password` and `client_secret`.
      - It is false for `mode`, `workflow.smart_zone_tokens`, `statusline.show_context_tokens`, `review.max_prompt_tokens` and `review.max_prompt_tokens_per_reviewer.claude`.
      - `mask_json_value`: null -> `(unset)`, `""` -> `(unset)`, `true` -> `true`, `"sk-anything"` -> `MASKED_SECRET`, `42` -> `MASKED_SECRET`.
      - `SECRET_CONFIG_KEYS.len() == 7` and `NON_SECRET_CONFIG_KEYS.len() == 4` are pinned.
    - config_json.rs `a_search_provider_slot_holding_an_api_key_parses_and_round_trips_secret`:
      - `{"brave_search":"BSA-SECRET-Q7Z9","firecrawl":true,"exa_search":null}` parses.
      - It gives `brave_search == Some(ApiKeySetting::Key(..))`, `firecrawl == Some(Flag(true))` and `exa_search == None`.
      - serialize -> reparse -> serialize is byte-identical.
      - `format!("{config:?}")` contains neither `BSA-SECRET-Q7Z9` nor `Q7Z9`.
    - detail.rs `a_secret_search_key_never_reaches_a_rendered_cell` (TestBackend via `render_defaults_config_to_text` with height >= the row count, so every row draws):
      - The config holds `brave_search` = `"BSA-SECRET-Q7Z9"`.
      - The text contains `brave_search` and `MASKED_SECRET` (arrival).
      - It contains neither `SECRET-` nor `Q7Z9`.
    - detail.rs `editing_a_secret_row_never_prefills_and_masks_typed_input`, driven through the REAL `handle_key` on `ctx_on_config_row(config, "brave_search")`:
      - Enter -> `defaults_editing == Some(idx)` and `defaults_text_buffer.char_count() == 0`.
      - Type `NEW-SECRET-X4K8` -> `draw_config_tab` lines contain `•` x15, and contain neither `NEW-SECRET` nor `X4K8` nor `Q7Z9`.
      - Enter -> `defaults_config.brave_search == Some(Key("NEW-SECRET-X4K8"))`, and `ctx.status_message` (if any) does not contain `X4K8`.
      - Enter then an immediate Enter (empty) -> the value is unchanged and the status names `brave_search unchanged`.
      - Enter, type `false`, Enter -> `Some(Flag(false))`, and the row value is `false`.
    - The existing tests that assumed `brave_search`/`firecrawl`/`exa_search` are Bool rows are updated to the Secret kind (and to `ApiKeySetting::Flag`), with the reason recorded in the SUMMARY. Any pinned Bool-kind count moves by exactly -3, and the new value is re-measured, not guessed.
  </behavior>
  <action>
    RED first:
    - Write the tests above. The new-type references will fail to compile (E0412/E0599), and that is the expected red.
    - Commit `test(quick-260926-jnf): add failing secret-slot parse, render and edit tests`.

    Then GREEN, in this order:
    1. Create src/state_reader/config_secrets.rs and register it with `pub mod config_secrets;` in src/state_reader/mod.rs, in alphabetical order after `config_json`. It holds pure functions only, and its module doc cites upstream `src/secrets.cts` and the docs table (I-3). Contents:
       - `SECRET_CONFIG_KEYS` (the 7 keys);
       - `NON_SECRET_CONFIG_KEYS` (the 4 measured exemptions);
       - `SECRET_NAME_MARKERS` (I-4);
       - `MASKED_SECRET` = `•••••••• (set)`;
       - `UNSET_LABEL` = `(unset)`;
       - `is_secret_key(path)`: explicit list first, then exemption (exact or `key.` prefix), then the lowercase marker substring;
       - `mask_json_value(&serde_json::Value) -> String` (I-1);
       - `redact_json_for_display(path, &Value) -> Value`. It walks objects recursively, building member paths `path.member` (`member` when the path is empty). Arrays keep the parent path. Any member whose path `is_secret_key` is replaced by `Value::String(mask_json_value(member))`. Task 2 wires this in; unit-test it here.
    2. config_json.rs: add `ApiKeySetting` as `#[serde(untagged)]` with variants `Flag(bool)` and `Key(String)` (Flag first, so JSON booleans never become keys).
       - Derive Clone, PartialEq, Serialize, Deserialize, but implement Debug BY HAND so `Key` prints `Key(<redacted>)` (T-jnf-04).
       - Retype `brave_search`, `firecrawl`, `exa_search` as `Option<ApiKeySetting>`, keeping their existing serde attributes.
       - The doc comment on the type states the upstream type `string | boolean | null` and the parse-failure bug it fixes.
       - Update the in-file fixture assertions.
    3. screens/mod.rs `EditBuffer`: add `char_count(&self) -> usize` and `masked(&self) -> String`, which returns one `•` per character and never content. Document that `masked` is the secret prompt's only render route. Neither method exposes raw text.
    4. detail.rs:
       - Add the `ConfigValueKind::Secret` variant, documented as an editable text-prompt kind whose `ConfigEntry.value` is ALWAYS the output of the masking helpers, never the raw value.
       - Add the helper `opt_secret_layered(project: Option<&ApiKeySetting>, defaults: Option<&ApiKeySetting>)`, which mirrors `opt_bool_layered`:
         - Flag -> `true`/`false`;
         - Key("") -> `(unset)`;
         - Key(_) -> `MASKED_SECRET`;
         - None in both layers -> `Unset(Box::new(Secret))`.
       - Rebuild the three Features rows with it. Their help (ConfigHelp::new, no choices) says the slot holds the provider API key (hidden) or true/false to override auto-detection of `<X>_API_KEY` / `~/.gsd/<x>_api_key`, and that `x` clears.
       - Add `set_secret_value(config, key, input) -> bool`, which implements the I-6 classification for the three keys (Task 3 adds four more).
       - Remove the three keys' arms from `set_config_value` and from `mutate_config_entry`'s Bool block. The type change forces this, and a Secret row has no dropdown and no toggle. Keep and retype their `clear_config_value` arms.
       - Add a `Secret => false` arm in `mutate_config_entry`.
       - Add a helper `opens_text_prompt(kind)` (String or Secret, via `editable()`) and use it in `config_text_edit_index` and at the popup dispatch in `render_defaults_tab`.
       - In the Enter handler, add a Secret branch beside the String one. It sets `defaults_editing` and seeds `EditBuffer::default()`, NEVER `entry.value`.
       - In the popup, render `cache.defaults_text_buffer.masked()` instead of `.shown()` when the kind is Secret, so the width arithmetic counts bullets.
       - In `handle_text_input_key`'s Enter, branch on `entry.kind.editable()`. Secret goes to the I-6 path, calling `persist_active_config` only when `set_secret_value` applied. String is unchanged.
       - Give `val_style` a Secret arm with its own dim style; the choice is the executor's.
       - Add the `kind_name` test arm `Secret => "secret"`.
    Keep `first_string_entry` matching `ConfigValueKind::String` only.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib secret</automated>
    <automated>rtk proxy cargo test --lib config</automated>
  </verify>
  <done>
    - A `brave_search` API key parses, renders only as `•••••••• (set)` and edits through an empty masked prompt that persists `Key`/`Flag` correctly. An empty Enter keeps the key.
    - The filtered test runs are green.
    - The RED commit precedes the `feat(quick-260926-jnf): …` commit.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: mask every other surface — heuristic pass-through rows, nested JSON members, inherited defaults and the Global view, the parse-error log; document the behaviour</name>
  <files>src/ui/screens/detail.rs, src/state_reader/config_json.rs, src/state_reader/config_secrets.rs, docs/CONFIGURATION.md, README.md</files>
  <behavior>
    - detail.rs `the_secret_heuristic_masks_unknown_pass_through_keys`:
      - The config carries `"foo_api_key":"FOO-SECRET-J2W6"`, `"My_Service_Token":"TOK-SECRET-V5M3"`, `"integrations":{"github_token":"GH-SECRET-B9R1","url":"https://example.invalid"}` and `"review":{"max_prompt_tokens":5000}`.
      - The pass-through rows' `value` is `MASKED_SECRET` for `foo_api_key` and `My_Service_Token`.
      - The `integrations` row value contains `MASKED_SECRET` and `https://example.invalid`, and not `B9R1`.
      - The `review.max_prompt_tokens` row value is `5000`: the exemption holds.
      - A TestBackend render of that config (tall enough for every row) contains none of `SECRET-`, `J2W6`, `V5M3`, `B9R1`, and does contain `foo_api_key` (arrival).
    - detail.rs `a_secret_inherited_from_global_defaults_is_masked_in_both_views`:
      - The project config lacks `brave_search`; `defaults_user_config` has `"brave_search":"GLB-SECRET-H3T7"`.
      - In the Project target, the row value is `MASKED_SECRET` with `from_defaults == true`, and the render shows ` *` but not `H3T7`.
      - After switching to the Global target (set `defaults_edit_target = Global`), the render still contains no `H3T7`.
    - detail.rs `a_read_only_json_row_redacts_nested_secret_members`: a `review.reviewer_instances` value carrying `{"a":{"cli":"x","api_key":"RI-SECRET-N8P2"}}` renders its row without `N8P2`.
    - detail.rs `no_authored_row_trips_the_secret_heuristic_unmasked` (census): for every row of `all_config_entries()` whose key `is_secret_key`, the kind (via `editable()`) is Secret. The 4 exempt keys are already non-secret by construction.
    - config_json.rs `a_config_parse_error_summary_never_quotes_the_value_secret`:
      - Parse `{"workflow":{"research":"sk-SECRET-F6Y4"}}` into `GsdConfig` and keep the error.
      - Assert `e.to_string()` DOES contain `F6Y4` (proof the raw Display leaks).
      - Assert `parse_error_summary(&e)` does not, and does contain the line number.
  </behavior>
  <action>
    RED first: write the tests above and commit `test(quick-260926-jnf): add failing heuristic, nested, inherited and log secret tests`.

    GREEN:
    1. detail.rs `append_passthrough_entries`: compute each row's value through a single helper, `display_config_value(path, &Value) -> String` (put it in config_secrets.rs):
       - If `is_secret_key(path)`, return `mask_json_value(value)`.
       - Otherwise return the old `json_display` rendering of `redact_json_for_display(path, value)`.

       Rows stay ReadOnly (I-12).
    2. Give `opt_json_readonly` a `path: &str` parameter and route it through the same helper. Pass each call site its row's JSON path; the label is right for the existing callers. `json_display` stays for non-config uses only; grep it, and if nothing else uses it, fold it into the helper.
    3. config_json.rs:
       - Add `pub fn parse_error_summary(e: &serde_json::Error) -> String`, built ONLY from `e.classify()` (Io/Syntax/Data/Eof as a word), `e.line()` and `e.column()`.
       - Make `parse_gsd_config`'s warning log that summary instead of the error's Display, with a comment naming T-jnf-03.
    4. No code change is needed for the Global view and the inherited-defaults layer: Task 1's `opt_secret_layered` and step 1 already build their `ConfigEntry.value`. The tests prove it. If one fails, fix it at the builder, never at the render.
    5. docs/CONFIGURATION.md: add a section `## Secret values in GSD project config (Config tab)`, placed before `## Related files in the repository`, stating:
       - which keys are secret (the 7 explicit ones plus the name heuristic and its markers, with the 4 exempt budget keys);
       - what is displayed (`•••••••• (set)`, `(unset)`, `true`/`false` for the override flags) and why there is no last-4 suffix;
       - how editing works: an empty prompt, typed input shown as bullets, `true`/`false` as the override, an empty Enter keeps the value, `x` clears;
       - that the value is still written in plaintext to `.planning/config.json` / `~/.gsd/defaults.json`, so the file's permissions are the security boundary, as upstream says;
       - that logs never carry config values.

       Add `src/state_reader/config_secrets.rs` to the Related-files list.
    6. README.md Config key table: extend the `Enter` row (or add one `Config` row directly after it) to say that on a secret row (API keys) Enter opens an empty prompt with hidden input, and an empty Enter keeps the current key. Link to the new CONFIGURATION.md section.

    Commit the code as `feat(quick-260926-jnf): …` and the docs as `docs(quick-260926-jnf): …`.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib secret</automated>
    <automated>rtk proxy cargo test --lib passthrough</automated>
    <automated>rtk proxy cargo test --lib pass_through</automated>
  </verify>
  <done>
    - Unknown secret-named keys, nested secret members, inherited and Global-view secrets all render masked.
    - The four budget keys still show their numbers.
    - The log line is value-free.
    - The user docs describe masking and editing.
    - The existing pass-through census and hostile-key tests stay green.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: typed rows for the 7 real top-level keys, count 132 -> 139, sync record rewritten in the SAME commit</name>
  <files>src/state_reader/config_json.rs, src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md</files>
  <behavior>
    - A new detail.rs table `PROMOTED_TOP_LEVEL_KEYS` (quick 260926-jnf) holds `tavily_search`/`ref_search`/`perplexity`/`jina` -> "secret", `runtime` -> "string", `context_profile` -> "enum" and `agent_skills` -> "readonly". It is chained into BOTH per-key loops (12826, 12872), and `the_resync_covers_every_key_the_drift_measurement_found` pins its length at 7.
      - The first loop then asserts: exactly one row each, the right kind, a `since` starting with `v`, and not a pass-through row.
      - The second loop gains a `"secret"` arm:
        - `set_secret_value(key, "K-SECRET-Z1Z1")` sets `Key`;
        - `set_secret_value(key, "true")` sets `Flag(true)`;
        - `set_secret_value(key, "  ")` returns false;
        - then clear.
    - detail.rs `every_promoted_top_level_key_writes_its_own_json_path`, per key: the since value (I-7) is exact, and `serde_json::to_value(&config).pointer("/<key>")` reflects each set and clear.
      - For `context_profile`, the toggle from unset gives `dev` (I-10).
      - For `agent_skills`, parse `{"agent_skills":{"gsd-planner":["skills/x"]}}`. It yields a ReadOnly row, no top-level pass-through row named `agent_skills`, and survives a toggle-of-another-key-then-save byte-for-byte.
    - `a_secret_search_key_never_reaches_a_rendered_cell` is extended to the four new keys, each holding a distinct `*-SECRET-<L4>`. None of the last-4 tokens appears in the render.
    - config_json.rs `the_promoted_top_level_keys_are_typed_not_extra`: `{"runtime":"codex","context_profile":"review","agent_skills":{},"tavily_search":"T-SECRET-M1M1","jina":true}` parses into the typed fields, and `config.extra` is empty. `{}` serialises with none of the 7 keys present.
    - detail.rs `the_sync_record_names_every_secret_key_and_exemption`: docs/GSD-CORE-SYNC.md, read via `CARGO_MANIFEST_DIR`, contains every `SECRET_CONFIG_KEYS` and `NON_SECRET_CONFIG_KEYS` entry backticked.
  </behavior>
  <action>
    RED commit: add the table, the tests above, the fixture entries and the extended render test. Leave `DEFAULTS_OPTION_COUNT` at 132 in this commit.
    - `populated_gsd_config` gains `"runtime":"claude"`, `"context_profile":"dev"`, `"agent_skills":{}` and `"tavily_search"`/`"ref_search"`/`"perplexity"`/`"jina"` as booleans. The fixture must stay free of unmodelled keys, and nothing may be unset.
    - Commit `test(quick-260926-jnf): add failing promoted top-level key tests`. It fails to compile on the missing fields, which is the expected red.

    GREEN, ONE commit containing all of the following, because `the_sync_record_names_every_modelled_key_and_the_measured_baseline` ties the rows to the doc:
    1. config_json.rs `GsdConfig`: under a `// --- top-level key promotion (quick task 260926-jnf) ---` marker, add the fields below, each with `#[serde(default, skip_serializing_if = "Option::is_none")]`:
       - `tavily_search`, `ref_search`, `perplexity`, `jina`: `Option<ApiKeySetting>`;
       - `runtime`: `Option<String>`;
       - `context_profile`: `Option<String>`;
       - `agent_skills`: `Option<serde_json::Value>`.

       Doc-comment `runtime` with the ID-2 note and `agent_skills` with its shape.
    2. detail.rs `build_defaults_entries`: add the 7 rows at the I-8 positions.
       - Use `opt_secret_layered` for the four search keys, with the same help pattern as Task 1's rows and `.since("v1.4.0")`.
       - Use `str_l` for `runtime`, with ConfigHelp::new naming claude/codex and the "this manager's driver does not read it" note, `.since("v1.01.0")`.
       - Use `enum_l` with `&["dev","research","review"]` for `context_profile`: ConfigHelp::with_choices with one explanatory clause per value, drawn from upstream's "execution context preset" wording, `.since("v1.01.0")`.
       - Use `opt_json_readonly` for `agent_skills` (path `agent_skills`), help "map of agent types to the skill entries each gets…", `.since("v1.01.0")`.
       - Every summary is at least 20 chars.
       - Update the `push` closure comment's ordinal to 140th.
    3. Arms:
       - `set_secret_value` gains the four new keys.
       - `set_string_value` gains `runtime`.
       - `set_config_value`'s enum-string block gains `context_profile`.
       - `mutate_config_entry`'s Enum block gains `context_profile` (unset -> `dev`, else `cycle`).
       - `clear_config_value` gains `runtime`, `context_profile` and the four search keys.
       - `agent_skills` gets no arm (I-11).
    4. Set `DEFAULTS_OPTION_COUNT` to 139, and extend its doc comment with "132 -> 139 at quick task 260926-jnf (7 top-level keys; kg_backend is a value, not a key)". Re-measure and update any pinned Enum-row or choice-bearing counts in `every_choice_bearing_entry_documents_exactly_its_dropdown_options`. They are expected to go +1 for `context_profile`; record the measured values.
    5. docs/GSD-CORE-SYNC.md: rewrite the affected sections, keeping the `## Modelled` and `## Pass-through` headings and all three constants' values unchanged.
       - Add a `## Top-level key promotion (quick task 260926-jnf)` section: a table of the 7 keys with type/default/since/introducing commit from `<upstream_facts>`, plus the kg_backend correction citing `capabilities/mempalace/capability.json:36` and CONFIGURATION.md:1098.
       - Add a `## Secret-bearing keys` section:
         - the 7 keys with provenance (upstream `src/secrets.cts` = 3, the upstream docs table = 7, and the gap);
         - the markers;
         - the 4 exemptions, with the measuring command;
         - the masking format and why it diverges from upstream's `****<last-4>`;
         - pointers to `config_secrets.rs`.
       - Recount the step-3 buckets by RE-RUNNING steps 2-3 (expected 162 lines / 113 dotted), and fix the "Undotted top-level keys, pass-through" row and its prose.
       - Update the Modelled intro to 139 rows (57 + 2 + 7 with a measured `since`, 73 without), and list the 7 keys in their categories.
       - Remove the 8-key entry from Pass-through.
       - Add to the next-sync list: "re-diff upstream `src/secrets.cts` SECRET_CONFIG_KEYS, the CONFIGURATION.md Search API keys table and the heuristic-matching key cells".

    Commit `feat(quick-260926-jnf): type the seven remaining top-level gsd-core keys; mask search-provider keys`, including the sync record.

    Finally run the two full gates below. Never pipe their output into grep: rtk filters downstream of the proxy and can fake a count.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast</automated>
    <automated>rtk proxy cargo clippy --all-targets -- -D warnings</automated>
  </verify>
  <done>
    - The 7 keys are typed, editable where their kind allows, masked where secret, and `since`-marked.
    - `DEFAULTS_OPTION_COUNT == 139` and GSD-CORE-SYNC.md change in the same commit.
    - The full test run's only failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. If a known flake (the ETXTBSY race in tests/envelope_carrier_reach.rs) appears, re-run that suite in isolation and record it.
    - Clippy exits 0.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| project `.planning/config.json` / `~/.gsd/defaults.json` -> TUI | Values, including API keys, cross into terminal cells, status messages and the log file |
| operator keystrokes -> secret prompt -> config file | Typed secret material is echoed on screen and written back |
| TUI screen -> screenshots / screen shares | Anything drawn is assumed to be captured (the README ships screenshots) |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-jnf-01 | I | detail.rs build_defaults_entries / append_passthrough_entries / opt_json_readonly | high | mitigate | `ConfigEntry.value` for any secret-classified path is built ONLY from `mask_json_value` / `opt_secret_layered` / `redact_json_for_display`, so every render reads masked text by construction. TestBackend tests cover typed, pass-through, nested, inherited and Global cases. |
| T-jnf-02 | I | detail.rs Enter handler + edit popup | high | mitigate | The Secret branch seeds `EditBuffer::default()`, never `entry.value`, and the popup renders `EditBuffer::masked()`. This is proven through real `handle_key` plus a TestBackend draw. |
| T-jnf-03 | I | config_json.rs parse_gsd_config tracing::warn | high | mitigate | Log `parse_error_summary` (category + line/column), never the serde Display, which quotes the offending value. Pinned by a test proving the raw Display leaks and the summary does not. |
| T-jnf-04 | I | derived `Debug` on GsdConfig | medium | mitigate | `ApiKeySetting` has a hand-written Debug that prints `Key(<redacted>)`. Pinned by the Task 1 `{config:?}` test. |
| T-jnf-05 | D/T | config_json.rs search-provider slots | medium | mitigate | Before this change, a string key in `brave_search`/`firecrawl`/`exa_search` made the whole file unparseable: no rows, and no save. The untagged `ApiKeySetting` parses it, and a byte-identical round-trip test pins lossless save. |
| T-jnf-06 | T | secret prompt empty Enter | medium | mitigate | Because the prompt is never prefilled, an empty Enter is a no-op for Secret rows (it clears only via explicit `x`), so a stray Enter cannot wipe a key. |
| T-jnf-07 | I | secrets stored under non-secret-looking names or embedded in other values (e.g. inside `workflow.test_command`) | low | accept | Detection is name-based by specification (explicit list + heuristic). Value-content scanning would need guesswork about key formats. The residual is documented in docs/CONFIGURATION.md. |
| T-jnf-08 | I | per-character bullet echo reveals typed length | low | accept | This is standard password-field behaviour and is needed to confirm keystrokes and Backspace. The stored value is shown only as a fixed-width `•••••••• (set)`. |
| T-jnf-09 | I | plaintext at rest in config.json | low | transfer | This matches upstream's stated boundary: the file is the security boundary, and its permissions belong to the operator. It is documented, not changed. |
</threat_model>

<verification>
- `rtk proxy cargo test --no-fail-fast`: only the src/envelope/policy.rs git-version witness may fail. Compare the pass count with the gtk baseline (2684 passed) plus the new tests. A green run with an unchanged count means suites never ran.
- `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0.
- `git log --oneline` shows, for each task, a `test(quick-260926-jnf)` commit before its `feat`/`docs` commit.
- Task 3's GREEN commit touches detail.rs (the count) AND docs/GSD-CORE-SYNC.md: `git show --stat <sha>` lists both.
- The upstream checkout is untouched: `git -C ~/projects/node/gsd-core status --short` and its HEAD branch are unchanged from before the run.
</verification>

<success_criteria>
- No secret value or its last 4 characters appears in any TestBackend render of the Config tab, across typed, pass-through, nested, inherited and Global cases, while the rows themselves visibly render.
- Secret editing never prefills, masks input, and persists `Key`/`Flag` correctly. An empty Enter keeps the key.
- Seven typed rows are added with measured `since` values, and `DEFAULTS_OPTION_COUNT` is 139. The sync record is rewritten in the same commit, with the kg_backend correction and the secret-key provenance.
- docs/CONFIGURATION.md and README.md describe masking and secret editing.
- Both gates pass, with only the permitted witness failing.
</success_criteria>

<output>
Create `.planning/quick/260926-jnf-mask-secret-config-values-in-cfg-tab-add/260926-jnf-SUMMARY.md` when done. It must include commits, TDD red evidence per task, gate results with counts, the re-measured step-3 buckets, every INFERRED decision (I-1..I-12 plus any the executor adds), and the threat-model disposition outcomes.
</output>
