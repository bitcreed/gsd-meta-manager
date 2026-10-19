# gsd-core config sync record

**Config surface synced against gsd-core `v1.18.0`**
(`35f5a452239b897b8d6352c973e1e4efd683370f`, `package.json` 1.18.0, tagged
2026-10-19) **on 2026-10-19. Conformance oracle / `GSD_CORE_SYNCED_VERSION`
pinned at `1.18.0`**, the latest version published on npm.

`src/state_reader/config_json.rs` carries the machine-readable copy of those
values as three constants, which at this sync all name the same release:

| Constant | Value | What it means |
|---|---|---|
| `GSD_CORE_SYNCED_VERSION` | `1.18.0` | The **conformance-oracle pin**: the latest gsd-core release published on npm at or below the synced commit. `scripts/install-conformance-oracle.sh` greps this declaration and runs `npm install @opengsd/gsd-core@<version>` — in the publish job (`.github/workflows/release.yml`) and inside `./scripts/pre-tag-check.sh --container`. The floor of the installed-GSD comparison's in-sync range. |
| `GSD_CORE_SYNCED_COMMIT` | `v1.18.0` | The tree the config surface was diffed against, as `git describe --tags --always` spells it. |
| `GSD_CORE_SYNCED_TREE_VERSION` | `1.18.0` | The `package.json` version of that tree. The ceiling of the installed-GSD comparison's in-sync range (quick task 260926-j0a). |

This file is the human-readable half: the full key inventory, per-key status,
and the exact commands that produced it, so the *next* sync starts from a
measured point instead of a blank slate.

Before this record existed the drift was silent in both directions. The sync
before quick task 260916-vqw was at GSD 1.8.0 (quick task 260722-emn), nothing
in the tree said so, and by 1.14.0 gsd-core documented **57 scalar keys this
build modelled nowhere** — including `workflow.compact_content`, the one a human
happened to notice. The 1.14.0 re-sync (260916-vqw, at `v1.14.0-52-g651511d1e`)
closed that gap; the release-1.15.0 re-sync (260926-gtk, at the untagged
`v1.14.0-111-gec81d0d10`) was the first delta sync made FROM this record. Quick
task 260926-jnf then promoted the seven real top-level keys gtk had left
pass-through and made every secret-bearing key display masked — see
[Top-level key promotion](#top-level-key-promotion-quick-task-260926-jnf) and
[Secret-bearing keys](#secret-bearing-keys). The v1.16.0 sync was three quick
tasks: 261012-jqb (the key delta — six `gates.*` keys retired, `audit.enabled`
added), 261012-jqc (`$GSD_HOME` for the global defaults file) and 261012-kee
(the v1.16.0 baseline). The v1.17.0 sync (quick task 261016-dx3) was
baseline-only, with no config key, default or loader change — see
[The v1.17.0 delta](#the-v1170-delta-quick-task-261016-dx3). The v1.18.0 sync
(quick task 261019-foc) was baseline-only as well, with no config key or default
change and a loader refactor that keeps the `GSD_HOME` rule — see
[The v1.18.0 delta](#the-v1180-delta-quick-task-261019-foc).

> **Keep this file, `config_json.rs`'s three constants and README.md's
> `**GSD compatibility:**` note in step, in the SAME commit.** A record that
> outlives its subject tells the next reader a surface is covered when it is
> not; `the_sync_record_names_every_modelled_key_and_the_measured_baseline` and
> `the_readme_compatibility_note_names_the_synced_baseline` go red otherwise.

**When the three disagree.** The oracle pin can only name a version that is on
npm. If a sync targets an untagged tree past the last published release (as the
release-1.15.0 sync did: `package.json` 1.15.0, describe
`v1.14.0-111-gec81d0d10`, pin `1.14.0`), keep the pin at the last published
version and bump it — re-describing the commit and replacing any provisional
`since` values with step 5's measured tag — once the tag ships on npm.
`the_gsd_core_sync_baseline_is_recorded` requires COMMIT to CONTAIN VERSION and
the pin to be at or below the tree version, which is what flags a half-done
bump.

**The release-1.15.0 tree is not an ancestor of the tags.** `ec81d0d10` (the
previous `GSD_CORE_SYNCED_COMMIT`) is reachable from neither `v1.15.0`
(`b10ab3fde`) nor `v1.16.0` — upstream re-cut the release branch. Diff from a
TAG (`v1.15.0`), not from that sha, when measuring the v1.15.0..v1.16.0 delta.

---

## How to re-measure (run these, do not trust this file's numbers)

The sibling checkout is **read-only** for this purpose. Nothing below writes to
it, and nothing below checks out a branch there.

**Measure against a REF, never the working tree.** The local checkout's HEAD is
whatever branch someone last worked on (at the time of writing: `next`, at
`v1.18.0-6-g8d3d29c46` with `package.json` 1.18.0) — not the sync target. A
plain `cat "$CORE/docs/CONFIGURATION.md"` or `node -p require(package.json)`
measures the wrong tree.

```bash
CORE=~/projects/node/gsd-core
REF=v1.19.0                          # the sync target: the next release tag
PREV=v1.18.0                         # the previous GSD_CORE_SYNCED_COMMIT

# 1. The baseline constants.
git -C "$CORE" describe --tags --always "$REF"   # -> GSD_CORE_SYNCED_COMMIT
git -C "$CORE" show "$REF:package.json" | grep '"version"'   # -> GSD_CORE_SYNCED_TREE_VERSION; the tree's version
npm view @opengsd/gsd-core version               # -> GSD_CORE_SYNCED_VERSION

# 2. The two key sets. Pipe RAW output — rtk filters piped command output and
#    can make a count pass vacuously; use `rtk proxy` when a count matters.
git -C "$CORE" show "$REF:docs/CONFIGURATION.md" \
  | grep -oP '^\| `\K[a-z0-9_.<>-]+(?=`)' \
  | LC_ALL=C sort -u > /tmp/core-keys.txt
grep -oP 'push\(cat, "\K[^"]+' src/ui/screens/detail.rs \
  | LC_ALL=C sort -u > /tmp/gmm-keys.txt

# 3. What gsd-core documents and this build does not push a row for.
LC_ALL=C comm -23 /tmp/core-keys.txt /tmp/gmm-keys.txt

# 4. The delta since the previous baseline — keys, defaults, capability-
#    registered keys, and loader semantics.
git -C "$CORE" diff "$PREV" "$REF" -- docs/CONFIGURATION.md \
  gsd-core/bin/shared/config-schema.manifest.json \
  gsd-core/bin/shared/config-defaults.manifest.json \
  'capabilities/*/capability.json' src/config-loader.cts

# 5. Per-key introduction version (ID-4 — MEASURED, never recalled).
K=workflow.compact_content
C=$(git -C "$CORE" log "$REF" --reverse --format=%H -S"$K" -- docs/CONFIGURATION.md src/ capabilities/ | head -1)
git -C "$CORE" tag --contains "$C" --sort=v:refname | head -1
git -C "$CORE" describe --tags "$C"
```

**Step 5's needle is the DOTTED key, not the leaf name.** A leaf like `enabled`
or `auto_update` matches dozens of unrelated commits across `src/`, and the
first of those is a `since` that is wrong by several releases. Every key below
resolved on the dotted form; the leaf-name fallback was never reached.

**When `tag --contains` prints nothing**, the key is not in any release tag
yet. The `since` is then `v` + the REF's `package.json` version, and it is
**provisional** until that tag exists — re-run step 5 at
the next sync and replace it with the measured tag.

**gsd-core's early tags are spelled oddly and that is not a typo here.**
`v1.01.0` (2026-05-24) and `v1.03.0` sit between `v1.0.0` and `v1.2.0` in that
repository's own tag list. A `since: v1.01.0` below is the measured tag name,
reproduced verbatim.

**Capability-registered keys are not in the schema manifest.** A key declared in
`capabilities/<name>/capability.json` (like `workflow.ui_interaction_capture`)
shows up in CONFIGURATION.md and in step 4's capability diff, but not in
`config-schema.manifest.json`'s `validKeys`. Diff both.

---

## The v1.18.0 delta (quick task 261019-foc)

Measured on 2026-10-19 between `v1.17.0` and `v1.18.0`
(`35f5a452239b897b8d6352c973e1e4efd683370f`), against the tags, not the
working tree:

| Measurement | v1.17.0 | v1.18.0 | Change |
|---|---:|---:|---|
| CONFIGURATION.md key cells (step 2) | 255 | 255 | **+0 / -0** |
| `config-schema.manifest.json` | — | — | unchanged (no diff) |
| `config-defaults.manifest.json` | — | — | unchanged |
| `src/config.cts`, `src/secrets.cts` | — | — | unchanged |
| `src/config-loader.cts` | — | — | global-defaults home lookup now `resolveGsdHome()` (#5017), same rule |
| Existing keys whose default changed | | | **none** |
| `capabilities/*/capability.json` | | | 45 files bump `version` 1.17.0 -> 1.18.0; 3 also gain a runtime-only `hostBehaviors` field |

The sorted key-cell sets are identical, so the Modelled count stays 135 and
`DEFAULTS_OPTION_COUNT` stays 135.

**`GSD_HOME` and `resolveGsdHome()` (#5017).** The config loader's two inline
`process.env['GSD_HOME'] || os.homedir()` lookups became calls to the new
`resolveGsdHome()` in `src/runtime-homes.cts`. Its body is the same `||` rule,
now shared by every reader and writer of the store. `GSD_HOME` moves only
`$GSD_HOME/.gsd/`: capability overlays (`capabilities/<id>/capability.json`),
`consent.json`, `defaults.json` and the `*_api_key` files. It does not move the
install root. This build's `user_defaults_path` already implements that rule,
so only its docstring citation moved. The install detection in
`src/state_reader/gsd_install.rs` never consulted `GSD_HOME`.

**Config-adjacent diffs in the range that are NOT new keys:**

- The 45 capability `version` bumps.
- `runtime.hostBehaviors.defaultHomeIncludeRoot` was added to antigravity
  (`.gemini/antigravity`) and copilot (`.copilot`) (#5123).
- kimi gained `runtime.hostBehaviors.retargetsGsdCoreProsePath: true`, another
  runtime-only field.
- `.planning` writer-side changes this build does not read:
  - the ROADMAP Plans line pending-tail reconcile in `update-plan-progress`
    (#4947);
  - the STATE body Completed Phases counter ratchet in `phase complete`
    (#4901). This build reads the frontmatter `progress.completed_phases`;
  - `phase add` writing `Depends on: Nothing` (or the nearest declared lower
    phase) instead of `N-1` (#5113). No `Depends on` parser exists here;
  - `/gsd-pause-work` handoff commits typed `docs(pause):` instead of `wip:`
    (#4943).
- smart-entry's `situation` text fix (#4890). Only the summary wording changed
  (`phaseSummary` replaces `phasePosition`); the JSON shape is unchanged, and
  `parse_smart_entry_json` reads only `actions`.
- The Community Capability Registry (`docs/registries/`) gained its
  `ponytail-everywhere` entry (#5380). It is docs-only and ships no code.

**Third-party capability keys.** `ponytail.*` (that registry entry's
`configKeys`: `ponytail.enabled`, `ponytail.enforcement`, `ponytail.level`) and
`musk.*` (the `config` block of a locally installed third-party capability, not
part of gsd-core) are not gsd-core keys. When a project's config.json carries
them they land in the top-level `extra` map and show as read-only `Not modelled`
pass-through rows that round-trip on save, with no code change. They are
recorded under Pass-through.

The secret-bearing re-diff found no change: `src/secrets.cts` is unchanged and
the heuristic still matches only the four exempt budget keys.
`gsd-core/references/gsd-run-resolver.md` is unchanged, so the resolver-order
comments in `src/state_reader/queue_md.rs` stay accurate.

---

## The v1.17.0 delta (quick task 261016-dx3)

Measured on 2026-10-16 between `v1.16.0` and `v1.17.0`
(`962c1192e062b5278281cd3a7bfd671a67a4db44`), against the tags, not the
working tree:

| Measurement | v1.16.0 | v1.17.0 | Change |
|---|---:|---:|---|
| CONFIGURATION.md key cells (step 2) | 254 | 255 | **+1 / -0** (`review.effort.opencode`) |
| `config-schema.manifest.json` | — | — | unchanged (no diff) |
| `config-defaults.manifest.json` | — | — | unchanged |
| `src/config-loader.cts`, `src/config.cts` | — | — | unchanged |
| Existing keys whose default changed | | | **none** |
| `capabilities/*/capability.json` | | | 45 files bump `version` 1.16.0 -> 1.17.0; 8 also change runtime-only fields |

**`review.effort.opencode`** is the one added key cell, and it is not a new
family. The `review.effort.<slug>` family has been capability-registered since
v1.13.0 (5ad9a36f3, #4255) through the `effortConfigKey` entries of the
claude, codex and opencode `capability.json` files; v1.17.0 is merely the first
release whose CONFIGURATION.md gives one of its members a leading key cell, so
step 2 sees it. Accepted values: `minimal`, `low`, `medium`, `high`, `xhigh`,
`max`, `inherit`. This build parses it into `ReviewConfig.extra`; it shows as a
read-only `Not modelled` row and round-trips on save with no code change. It
is recorded under Pass-through, so the Modelled count stays 135 and
`DEFAULTS_OPTION_COUNT` stays 135.

**Config-adjacent diffs in the range that are NOT new keys:**

- `runtime.hostBehaviors.unsupportedFeatures: ["context-warnings"]` was added
  to the antigravity, augment, codebuddy, hermes, kimi-code, kimi and qwen
  capability manifests.
- opencode's `dispatch.nested` went from `"undocumented"` to `false` and
  `dispatch.maxDepth` from `"undocumented"` to `1`.
- opencode gained a `nativeCliPlugin` block (the OpenCode statusline TUI
  plugin, `plugins/gsd-statusline/tui.tsx`).
- CONFIGURATION.md gained an "OpenCode 1.x and 2.x" subsection. Its new names
  are environment variables (`GSD_OPENCODE_GENERATION`, `GSD_HOOK_RUNTIME`,
  `GSD_HOOK_SHELL`), not config keys; its `review.models.opencode` row
  re-documents a key already documented at v1.16.0.
- The 45 capability `version` bumps.

The secret-bearing re-diff found no change: `src/secrets.cts` is unchanged and
the heuristic still matches only the four exempt budget keys.
`gsd-core/references/gsd-run-resolver.md` is unchanged, so the resolver-order
comments in `src/state_reader/queue_md.rs` that cite 1.16.0 stay accurate.

---

## The release-1.15.0 delta (quick task 260926-gtk)

Measured on 2026-09-26 between the previous baseline `651511d1e`
(`v1.14.0-52-g651511d1e`, an ancestor of the target) and `ec81d0d`:

| Measurement | 651511d1e | ec81d0d | Change |
|---|---:|---:|---|
| CONFIGURATION.md key cells (step 2) | 258 | 260 | **+2 / -0** |
| `config-schema.manifest.json` `validKeys` | 120 | 121 | + `planner.stall_detection_enabled` (121 -> 122 counting the one `runtimeStateKeys` entry) |
| `config-defaults.manifest.json` | — | — | one new top-level block, `"planner": { "stall_detection_enabled": true }` |
| Existing keys whose default changed | | | **none** |

The two added keys, both now modelled as editable Bool rows with `since`
`v1.15.0` — `git tag --contains` resolved both introducing commits to v1.15.0
once the tag existed, re-measured at quick task 261012-jqb:

| Key | Type / default | Introduced by | Notes |
|---|---|---|---|
| `planner.stall_detection_enabled` | boolean, `true` | 1e3e1f7cd (#4570 / PR #4585, `v1.14.0-98`) | New top-level `planner` block. `resolvePlannerStallDetectionEnabled` in `src/config-loader.cts` lets only a real JSON boolean override the default; a string, number or null fails safe to `true`. CONFIGURATION.md warns that `false` gives up bounded recovery if the runtime loses the completion handoff. |
| `workflow.ui_interaction_capture` | boolean, `false` | 88b5775dc (#4223 / PR #4477, `v1.14.0-80`) | Declared in `capabilities/ui/capability.json:43`, not in the schema manifest. Lets `gsd-ui-auditor` add post-interaction captures (hover, focus ring, open menus, filled forms) through the chrome-devtools CLI run from Bash; needs an installed Chrome (`CHROME_BIN` overrides); read by `/gsd-ui-review`. |

**A deliberate divergence (INFERRED I-5).** gsd-core fails a non-boolean
`planner.stall_detection_enabled` safe to `true`; this build types the key as
`Option<bool>`, so a hand-edited string or number is a whole-file parse failure
— the same as every other typed key here. A failed parse draws no Defaults rows
and therefore cannot save over the file, and gsd-core's own `config-set` coerces
`"false"` to a real boolean, so only a hand edit can produce the case.

**Config-adjacent diffs in the range that are NOT new keys:**

- `src/config-loader.cts` runtime-identity fill (#4717, 9a41a9521): fills the
  existing `runtime` key from `GSD_RUNTIME` / the per-install marker. Loader
  semantics, not a key — handled by the batch's Codex-awareness item.
- Every `capabilities/*/capability.json` `version` bumped `1.14.0` -> `1.15.0`
  (45 files).
- `capabilities/opencode` drops the installer flag `skipCodexSkillsManifest`
  (installer metadata, not a config key).
- CONFIGURATION.md prose edits to the `phase_id_convention`, `worktree.baseRef`
  and `resolve_model_ids` rows, and a new graphify-CLI prose section (no key).

### Step 3's output is not a gap list — read it with these buckets

RE-MEASURED at quick task 260926-jnf, at `ec81d0d` against this build: **162
lines, 113 of them dotted** (171 before gtk modelled its two keys, 169 after,
162 after jnf promoted seven top-level keys):

| Bucket | Count | What it is |
|---|---:|---|
| Undotted rows that are not config keys | 48 | The grep matches any leading table cell, so enum *values* (`balanced`, `codex`, `true`, `v1`, and `kg_backend` — see below), table headings, and the field names of tables that describe something other than config (`heading`/`source`/`template`/`fallback`/`enabled` of a custom PR-body section entry; `active_phase`/`next_action`/`next_phases`/`progress` of STATE.md frontmatter) come through as if they were keys. |
| Undotted top-level keys, pass-through | 1 | `context` — see "Still pass-through at the top level" below. |
| Dotted value rows, not keys | 3 | `gpt-5.6-luna`, `gpt-5.6-sol`, `gpt-5.6-terra` — model ids from a value table. |
| Modelled under an unprefixed row label | 30 | `git.base_branch` is this tab's `base_branch` row, `intel.enabled` is `intel_enabled`, and 21 `workflow.*` keys are spelled bare. The JSON path each row reads is correct; only the label is unprefixed. See "Known inconsistency" below. |
| Pass-through families | 80 | The nested and templated keys in the [Pass-through](#pass-through) section, which get no typed row **by design**. |

**Two corrections to the gtk bucketing.** gtk counted 48 value rows and 8
undotted top-level keys. `kg_backend` was one of the 8, but it is not a key: it
is an enum VALUE of the capability-registered `mempalace.memory_mode`
(`capabilities/mempalace/capability.json:36`), and step 2's grep caught it in
CONFIGURATION.md:1098's "Memory modes in detail" value table. It now sits in
the value-row bucket. Conversely `context` (CONFIGURATION.md:255, and in
`config-schema.manifest.json`'s `validKeys`) was counted as a value row, but it
IS a real top-level key. The seven real keys of the 8 are promoted below;
`context` is the only undotted top-level key still pass-through.

### Still pass-through at the top level: `context`

Upstream contradicts itself on its type. CONFIGURATION.md:255 documents it as a
free-form string ("Custom context string injected into every agent prompt"),
while `src/config.cts:903-904` makes `config-set context` an enum over
`dev`/`research`/`review` — the same values as `context_profile`. A typed row
would have to pick one, so quick task 260926-jnf left it as a read-only `Not
modelled` row (visible, lossless on save) rather than guess (INFERRED I-13).

---

## Top-level key promotion (quick task 260926-jnf)

The seven real top-level keys the release-1.15.0 re-sync left pass-through, now
typed rows. `since` is step 5's recipe (earliest commit touching the key in
`docs/CONFIGURATION.md src/ capabilities/`, then `tag --contains`), MEASURED
against `upstream/release-1.15.0`:

| Key | Type / default upstream | Row kind here | since | Introducing commit |
|---|---|---|---|---|
| `tavily_search` | `string \| boolean \| null`, `null` | Secret | v1.4.0 | 11afca296 ("feat(#656): Research module — content-addressed cache + provider seam") |
| `ref_search` | `string \| boolean \| null`, `null` | Secret | v1.4.0 | 11afca296 |
| `perplexity` | `string \| boolean \| null`, `null` | Secret | v1.4.0 | 11afca296 |
| `jina` | `string \| boolean \| null`, `null` (effective default: available) | Secret | v1.4.0 | 11afca296 |
| `runtime` | string, `claude` / `codex` / any; no default | String | v1.01.0 | d478e7f48 |
| `context_profile` | enum `dev` / `research` / `review`; no default | Enum | v1.01.0 | 641ea8ad4 |
| `agent_skills` | object, `{}` — agent type to an array of skill entries | ReadOnly | v1.01.0 | db3eeb8fe |

- The four search keys' CONFIGURATION.md table rows arrived later, at fd576528a
  (v1.7.0); step 5's earliest-commit rule wins (INFERRED I-7). `brave_search`,
  `firecrawl` and `exa_search` predate this record and keep an empty `since`.
- `runtime` is gsd-core's own selector. This manager does not read it (ID-2):
  its per-project agent runtime lives in its own registry (`src/config.rs`
  `RUNTIME_KEY`), and `tests/driver_codex_runtime.rs` pins that a project whose
  GSD config says `codex` still spawns the unchanged Claude argv. gsd-core's
  loader also fills it from `GSD_RUNTIME` / the install marker (#4717), so the
  file value is not the whole story.
- `context_profile` has no upstream default. From unset, the first toggle writes
  `dev` (the first option) rather than stepping through `cycle`, which would
  skip to `research` (INFERRED I-10).
- `agent_skills` is read-only, like `review.reviewer_instances`: its value is a
  map of arrays, edited in the file. Its `agent_skills.<agent-type>` members are
  shown inside that value, with any secret-named member masked.
- `kg_backend` gets no row: it is a `mempalace.memory_mode` value, not a key
  (see the correction above). So the promotion is 7 rows, 132 -> 139.

---

## Secret-bearing keys

The Config tab never draws a secret value (quick task 260926-jnf). The
classification and masking live in `src/state_reader/config_secrets.rs`; every
`ConfigEntry.value` for a secret-classified path is built from its helpers, so
no render, prompt prefill, status message or log line can reach the raw value.

**The explicit list — 7 keys, and upstream disagrees with itself.**

| Source at `ec81d0d` | Keys |
|---|---|
| Code: `src/secrets.cts` `SECRET_CONFIG_KEYS` | `brave_search`, `firecrawl`, `exa_search` (3) |
| Docs: CONFIGURATION.md "Search API keys" table, each "Masked in display" | the 3 above plus `tavily_search`, `ref_search`, `perplexity`, `jina` (7) |

This build honours the documented promise: all seven are secret here
(INFERRED I-3). Each is typed `string | boolean | null`: a string is the API
key; `true`/`false` override auto-detection from `<X>_API_KEY` or
`~/.gsd/<x>_api_key`. Before 260926-jnf, `brave_search`/`firecrawl`/`exa_search`
were typed `Option<bool>`, so a project that stored its key the documented way
failed to parse as a whole (and the serde error quoted the key into the log).
All seven now share one untagged `ApiKeySetting { Flag(bool), Key(String) }`
with a hand-written `Debug` that prints `Key(<redacted>)`.

**The name heuristic** covers keys this build does not model — pass-through
rows and members nested inside read-only JSON values. A full dotted path is
secret when it contains, case-insensitively, one of `api_key`, `apikey`,
`api-key`, `token`, `secret`, `password`, `passwd`. A census test makes it
impossible to add an authored row whose key trips the heuristic unless the row
is Secret-kind.

**The measured exemptions — 4 budget keys.** Every upstream key cell matching
the markers holds a number or a boolean, not a secret. They are exempt as the
exact path or as a `<key>.` prefix:

- `review.max_prompt_tokens`
- `review.max_prompt_tokens_per_reviewer` (and its `.<slug>` entries)
- `statusline.show_context_tokens`
- `workflow.smart_zone_tokens`

Measured with (`validKeys` in the schema manifest has the same four matches):

```bash
git -C "$CORE" show "$REF:docs/CONFIGURATION.md" \
  | grep -oP '^\| `\K[a-z0-9_.<>-]+(?=`)' | sort -u \
  | grep -iE 'api_key|apikey|api-key|token|secret|password|passwd'
```

**The masking format diverges from upstream on purpose (INFERRED I-1).** A set
secret renders as `•••••••• (set)` — always eight bullets, no suffix, no
length. null, missing and `""` render `(unset)`; a boolean renders `true` /
`false`. Upstream's `maskSecret` renders `****<last-4>` (and `****` below 8
characters). This build drops the last four because the README ships
screenshots of this TUI: four characters leak 16-24 bits of key entropy, the
length reveals the provider's key format, and nothing in the TUI needs to tell
two keys apart. Editing opens an empty prompt that echoes one `•` per typed
character; an empty Enter keeps the key; `x` clears.

Detection is by NAME only. A secret stored under an innocent name, or embedded
in another value, is displayed like any other value (T-jnf-07, accepted).

---

## Modelled

Every key `build_defaults_entries` pushes a row for, in display order. **135
rows**: **51** carry a `since` measured at the 1.14.0 re-sync (260916-vqw
measured 57; six were retired at 261012-jqb), **2** carry `v1.15.0` (added by
260926-gtk, measured against the v1.15.0 tag at 261012-jqb), **7** carry a
`since` measured when quick task 260926-jnf promoted them, **2** carry
`v1.16.0` (261012-jqb), **1** carries `v1.6.0`, measured at 261019-guw, and the
**72** without one predate this record and were not retro-measured.

`DEFAULTS_OPTION_COUNT` in `src/ui/screens/detail.rs` pins this count at 135
(132 -> 139 at 260926-jnf, 139 -> 135 at 261012-jqb), `RESYNCED_KEYS` /
`RESYNCED_KEYS_1_15_0` / `RESYNCED_KEYS_1_16_0` / `PROMOTED_TOP_LEVEL_KEYS` pin
the four key lists, `REMOVED_KEYS_1_16_0` pins the six retired ones, and three
coverage assertions refuse a row with no `ConfigHelp`.

### Planning

| Key | since |
|---|---|
| `research` | |
| `plan_check` | |
| `pattern_mapper` | |
| `nyquist_validation` | |
| `ui_phase` | |
| `ui_safety_gate` | |
| `ai_integration_phase` | |
| `subagent_timeout` | |
| `workflow.specless_probe_fallback` | |
| `workflow.assumption_delta` | |
| `workflow.plan_drift_precheck` | |
| `workflow.plan_chunked` | |
| `workflow.context_guard_mode` | |
| `workflow.context_coverage_gate` | v1.01.0 |
| `workflow.context_drift_precheck` | v1.13.0 |
| `workflow.context_drift_action` | v1.13.0 |
| `workflow.drift_threshold` | v1.01.0 |
| `workflow.drift_action` | v1.01.0 |
| `workflow.inline_plan_threshold` | v1.01.0 |
| `workflow.max_discuss_passes` | v1.01.0 |
| `workflow.smart_zone_tokens` | v1.9.0 |
| `workflow.post_planning_gaps` | v1.01.0 |
| `workflow.plan_bounce` | v1.01.0 |
| `workflow.plan_bounce_passes` | v1.01.0 |
| `workflow.plan_bounce_script` | v1.01.0 |
| `workflow.plan_review_convergence` | v1.01.0 |
| `planning.chunked_parallel` | v1.13.0 |
| `planning.pr_strict` | v1.12.0 |
| `planner.stall_detection_enabled` | v1.15.0 |
| `plan_review.source_grounding` | v1.2.0 |
| `plan_review.source_grounding_authority` | v1.2.0 |
| `features.global_learnings` | v1.01.0 |
| `features.thinking_partner` | v1.01.0 |

### Execution

| Key | since |
|---|---|
| `verifier` | |
| `tdd_mode` | |
| `code_review` | |
| `code_review_depth` | |
| `ui_review` | |
| `node_repair` | |
| `node_repair_budget` | |
| `workflow.api_coverage_gate` | |
| `workflow.windows_enforce` | |
| `workflow.mvp_mode` | |
| `workflow.test_gate_timeout` | |
| `workflow.code_review_command` | |
| `workflow.security_asvs_level` | |
| `workflow.security_block_on` | |
| `workflow.security_enforcement` | v1.01.0 |
| `workflow.agent_hint_routing` | v1.11.0 |
| `workflow.code_review_point` | v1.13.0 |
| `workflow.code_review_depth_overrides` | v1.12.0 |
| `workflow.human_verify_mode` | v1.01.0 |
| `workflow.build_command` | v1.01.0 |
| `workflow.test_command` | v1.01.0 |
| `workflow.worktree_skip_hooks` | v1.01.0 |
| `workflow.live_dom_uat` | v1.12.0 |
| `workflow.ui_interaction_capture` | v1.15.0 |
| `workflow.cross_ai_execution` | v1.01.0 |
| `workflow.cross_ai_command` | v1.01.0 |
| `workflow.cross_ai_timeout` | v1.01.0 |

### Docs & Output

| Key | since |
|---|---|
| `commit_docs` | |
| `planning.commit_docs` | v1.01.0 |
| `skip_discuss` | |
| `use_worktrees` | |
| `text_mode` | |
| `workflow.compact_content` | v1.14.0 |
| `response_language` | |

### Features

| Key | since |
|---|---|
| `intel_enabled` | |
| `graphify_enabled` | |
| `graphify_build_timeout` | |
| `graphify.graph_path` | |
| `graphify.auto_update` | v1.01.0 |
| `brave_search` | |
| `firecrawl` | |
| `exa_search` | |
| `tavily_search` | v1.4.0 |
| `ref_search` | v1.4.0 |
| `perplexity` | v1.4.0 |
| `jina` | v1.4.0 |

### Model & Pipeline

| Key | since |
|---|---|
| `mode` | |
| `granularity` | |
| `model_profile` | |
| `runtime` | v1.01.0 |
| `context_profile` | v1.01.0 |
| `agent_skills` | v1.01.0 |
| `parallelization` | |
| `auto_advance` | |
| `auto_chain_active` | |
| `branching_strategy` | |
| `base_branch` | |
| `phase_branch_template` | |
| `milestone_branch_template` | |
| `quick_branch_template` | |
| `git.create_tag` | v1.01.0 |
| `git.allow_default_branch_commits` | v1.13.0 |
| `git.protected_branches` | v1.12.0 |

### Misc

| Key | since |
|---|---|
| `context_warnings` | |
| `hooks.context_warning_threshold` | v1.14.0 |
| `hooks.context_critical_threshold` | v1.14.0 |
| `hooks.workflow_guard` | v1.01.0 |
| `context_window` | v1.01.0 |
| `research_before_questions` | |
| `discuss_mode` | |
| `search_gitignored` | |
| `planning.search_gitignored` | v1.01.0 |
| `project_code` | |
| `phase_naming` | |
| `phase_id_convention` | |
| `claude_md_path` | |
| `sub_repos` | |
| `planning.sub_repos` | v1.01.0 |
| `workflow.auto_prune_state` | v1.01.0 |

### Orchestration

| Key | since |
|---|---|
| `claude_orchestration.enabled` | |
| `claude_orchestration.execution_backend` | |
| `claude_orchestration.min_agent_sdk_version` | |

### Statusline

| Key | since |
|---|---|
| `statusline.show_context_tokens` | |
| `statusline.state_format` | |
| `statusline.show_git` | |
| `statusline.context_position` | v1.01.0 |
| `statusline.show_last_command` | v1.01.0 |
| `statusline.show_state_freshness` | v1.12.0 |

### Routing

| Key | since |
|---|---|
| `dynamic_routing.provider_escalation` | |
| `dynamic_routing.max_escalations` | |
| `dynamic_routing.enabled` | v1.01.0 |
| `dynamic_routing.escalate_on_failure` | v1.01.0 |

### External Job

| Key | since |
|---|---|
| `external_job.submit_timeout_ms` | |
| `external_job.poll_timeout_ms` | |
| `external_job.artifact_dir` | |

### Capabilities

| Key | since |
|---|---|
| `capabilities.strict_known_registries` | v1.6.0 |
| `capabilities.auto_update` | |

`capabilities.strict_known_registries` is read-only because upstream types it
`array | null` (CONFIGURATION.md:1035; `null` permits any source, `[]` blocks
every external install). Since 261019-guw a bool written by an earlier build is
preserved verbatim and flagged as malformed rather than making the whole
`config.json` unreadable.

### Review

| Key | since |
|---|---|
| `review.reviewer_instances` | |

### Gates

| Key | since |
|---|---|
| `gates.execute_next_plan` | v1.01.0 |
| `gates.confirm_transition` | v1.01.0 |
| `gates.confirm_milestone_scope` | v1.16.0 |

gsd-core v1.16.0 (ded8b6259, #4974) retired six never-read gates; they have no
typed row here and are listed under [Pass-through](#pass-through).

### Observability

| Key | since |
|---|---|
| `audit.enabled` | v1.16.0 |

`since` here is the release that made the key EFFECTIVE: af464656e (#4975,
first tag v1.16.0) registered it in the schema and wired the dispatch seams to
read it. Step 5's first-mention recipe answers v1.2.0 (2d14eb887), but that
commit only documented a key `config-set` rejected and nothing read — a
prose-only promise, so v1.2.0 would advertise an inert key (INFERRED I-1).

---

## Pass-through

These gsd-core keys deliberately get **no typed row**. They are not missing:
each one still appears in the Defaults tab under the **`Not modelled`**
category, is rendered read-only, and round-trips byte-for-byte through a save.
Recording them here is what makes "deliberately deferred" distinguishable from
"nobody noticed".

Every key of a parsed block that this build has no field for lands in that
block's `#[serde(flatten)] extra` map, so the list below is a description of
what is *currently* in there — not a filter. A key gsd-core adds tomorrow, under
a name nobody here has seen, is handled by the same mechanism without an edit.

| Family | Why no typed row |
|---|---|
| `review.models.<cli>`, `review.effort.<slug>`, `review.reviewer_instances.<name>.*` | Templated by CLI / reviewer-lane slug / instance name; a typed field per unknown name is not expressible. The effort family is owned by each lane's capability manifest (`effortConfigKey`). |
| `review.default_reviewers`, `review.*_host`, `review.max_prompt_tokens*`, `review.parallel_lanes` | Would be typeable; deferred because the whole `review.*` block is one editing surface and splitting it half-and-half reads worse than leaving it whole. |
| `model_policy.*`, `model_profile_overrides.<runtime>.<tier>` | Keyed by runtime and tier; templated. |
| `effort.*`, `fast_mode.*` | Both carry `agent_overrides.<agent-id>` and `routing_tier_defaults.*`; templated. |
| `dynamic_routing.tier_models.<tier>` | Templated by tier name. |
| `models.<phase_type>`, `granularities.<phase_type>` | Templated by phase type. |
| `agent_tools.<selector>`, `agent_skills_security.trusted_global_roots` | Templated by selector; the security roots key is a list. (`agent_skills` itself is a typed read-only row since 260926-jnf; its `agent_skills.<agent-type>` members are shown inside that row's value.) |
| `code_quality.fallow.*` | Four keys of one opt-in subsystem this TUI surfaces nowhere else. |
| `mempalace.*` (11 keys) | An optional integration this build has no other awareness of. |
| `parallelization.*` (6 keys) | gsd-core's namespaced expansion of the top-level `parallelization` boolean this tab already edits; promoting them needs a decision about which wins. |
| `refactor.*`, `security.injection_blocking` | Small coherent blocks, no typed row yet. |
| `gates.confirm_project`, `gates.confirm_phases`, `gates.confirm_roadmap`, `gates.confirm_breakdown`, `gates.confirm_plan`, `gates.issues_review`, `safety.always_confirm_destructive`, `safety.always_confirm_external_services` | Removed upstream at v1.16.0 (ded8b6259, #4974): no longer read by anything and rejected by gsd-core's `config-set`. They never get a typed row; a pre-1.16.0 file keeps them (`GatesConfig.extra` / the top-level extra map) and shows them read-only. The six `gates.*` keys had typed rows from 260916-vqw until 261012-jqb retired them. |
| `executor.stall_*`, `planner.stall_detect_interval_minutes`, `planner.stall_threshold_minutes`, `manager.flags.*` | Runtime tuning knobs, rarely edited by hand. Since 260926-gtk `planner` is a TYPED block (`planner.stall_detection_enabled` is modelled), so its two remaining knobs render as separate `planner.<key>` rows; before it, the whole `planner` object was a single top-level `planner` row. |
| `context` | The one undotted top-level key still pass-through: upstream's docs call it a free-form string, its `config-set` an enum. See "Still pass-through at the top level". |
| `claude_md_assembly.mode`, `learnings.max_inject` | Singletons that did not fit an existing category. |
| `ponytail.*`, `musk.*` (third-party capability `configKeys`) | Owned by community or locally installed capabilities rather than by gsd-core; open-ended by capability name, so a typed row is not expressible. They reach the tab through the top-level extra map. |

### Known inconsistency — unprefixed row labels

Thirty rows read a namespaced JSON path but are LABELLED without its prefix:
`base_branch` for `git.base_branch`, `intel_enabled` for `intel.enabled`,
`research` for `workflow.research`, and so on. The path each row reads is
correct; only the label a human sees is short. The newer rows all use the full
dotted spelling, so the tab currently mixes both conventions.

This is recorded rather than fixed here because renaming a label changes what
`set_config_value`, `clear_config_value` and `mutate_config_entry` match on, and
those three tables are keyed by the label. Doing it properly means moving all
three to the dotted spelling at once — a change with no user-visible benefit
beyond consistency, and one that would have been buried inside a key re-sync.

---

## Next sync

Start from step 1 above. **Every sync, whatever else it does:** when any of
`GSD_CORE_SYNCED_VERSION`, `GSD_CORE_SYNCED_COMMIT` or
`GSD_CORE_SYNCED_TREE_VERSION` moves, update README.md's `**GSD compatibility:**`
paragraph in the same commit — `the_readme_compatibility_note_names_the_synced_baseline`
fails otherwise. The app's installed-GSD comparison uses the oracle pin as the
floor and the tree version as the ceiling of the in-sync range, so bumping
either one moves the boundary at which users see the "newer than synced"
warning.

Then, in priority order:

1. **Re-measure the baseline trio** with step 1 against the next release tag
   (`REF=v1.19.0`, `PREV=v1.18.0`). If `GSD_CORE_SYNCED_COMMIT` already
   matches the new REF, there is nothing else to do. If the target is an
   untagged tree past the last npm release, see "When the three disagree"
   above. After moving the oracle pin, re-run
   `./scripts/pre-tag-check.sh --container vX.Y.Z` so the new oracle is
   certified.
   **Re-diff the secret-bearing keys every sync** (quick task 260926-jnf):
   upstream `src/secrets.cts` `SECRET_CONFIG_KEYS`, the CONFIGURATION.md
   "Search API keys" table, and the heuristic-matching key cells (the command
   under [Secret-bearing keys](#secret-bearing-keys)). A new secret key goes
   into `SECRET_CONFIG_KEYS` in `config_secrets.rs`; a new non-secret match
   goes into `NON_SECRET_CONFIG_KEYS`.
   **Decide `context`'s type** once upstream's docs and `config-set` agree,
   then promote it.
2. **Promote `parallelization.*` (6 keys).** The highest-value pass-through
   family: this tab already edits a top-level `parallelization` boolean, so a
   project carrying the namespaced block sees a row that disagrees with the file.
   Decide which path wins before adding rows.
3. **Promote the flat `review.*` keys** (`default_reviewers`, the three `*_host`
   keys, `max_prompt_tokens`, `max_prompt_tokens_per_reviewer`,
   `parallel_lanes`). All are scalars; only the `models.<cli>`,
   `effort.<slug>` and `reviewer_instances.<name>.*` families have to stay
   pass-through.
4. **Then `refactor.*` and `safety.*`**, which are small and fully scalar.
5. **Retro-measure `since` for the 73 rows that have none**, using step 5's
   command, if a reader ever needs the full history rather than the delta.
6. **Consider the label rename** described under "Known inconsistency" — as its
   own change, never folded into a sync.
