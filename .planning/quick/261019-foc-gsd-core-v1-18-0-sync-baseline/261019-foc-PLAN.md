---
quick_id: 261019-foc
mode: quick
phase: quick-261019-foc
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/config_json.rs
  - docs/GSD-CORE-SYNC.md
  - README.md
autonomous: true
requirements: [QUICK-261019-foc]

estimate:
  tokens: 45000
  raw_tokens: 45000
  tasks: 1
  confidence: low

must_haves:
  truths:
    - "src/state_reader/config_json.rs declares GSD_CORE_SYNCED_VERSION = \"1.18.0\", GSD_CORE_SYNCED_COMMIT = \"v1.18.0\" and GSD_CORE_SYNCED_TREE_VERSION = \"1.18.0\". Their doc comments name the v1.18.0 sync (261019-foc), the full tag sha 35f5a452239b897b8d6352c973e1e4efd683370f and REF=v1.18.0. No 1.17.0 string remains between the top of the file and the TREE_VERSION declaration. The extraction scripts/install-conformance-oracle.sh performs (declaration regex, `head -1`, then the sed that pulls out the quoted literal) yields `1.18.0`, and it still matches exactly one line."
    - "The user_defaults_path docstring cites gsd-core `src/runtime-homes.cts` `resolveGsdHome` (v1.18.0, #5017) as the rule's home, with the same rule `process.env.GSD_HOME || <home>`. It keeps the 'non-empty `GSD_HOME`' wording, and the previous config-loader commit citation is gone. No code in that function or in user_defaults_path_from changes."
    - "README.md has exactly one `**GSD compatibility:**` paragraph. It says `synced to gsd-core 1.18.0 (`v1.18.0`), with oracle pin `1.18.0``, still links docs/GSD-CORE-SYNC.md, and README.md contains no 1.17.0 anywhere."
    - "In docs/GSD-CORE-SYNC.md, the header and the constants table name `v1.18.0` (sha 35f5a452239b897b8d6352c973e1e4efd683370f, `package.json` 1.18.0, tagged 2026-10-19), the sync date 2026-10-19 and the npm pin `1.18.0`. The history paragraph records the v1.18.0 sync as quick task 261019-foc and links `#the-v1180-delta-quick-task-261019-foc`."
    - "docs/GSD-CORE-SYNC.md has a `## The v1.18.0 delta (quick task 261019-foc)` section placed before the v1.17.0 delta section. It records the MEASURED v1.17.0..v1.18.0 delta: 255 -> 255 key cells (+0 / -0); unchanged schema and defaults manifests; DEFAULTS_OPTION_COUNT stays 135; config-loader home lookup now resolveGsdHome() (#5017); GSD_HOME moves only `$GSD_HOME/.gsd`, not the install root; 45 capability `version` bumps; antigravity/copilot `defaultHomeIncludeRoot`; the .planning writer-side changes the app does not read; the smart-entry `situation` text fix; the Community Capability Registry entry; and the `ponytail.*` / `musk.*` read-only pass-through handling."
    - "The re-measure recipe reads `REF=v1.19.0` / `PREV=v1.18.0`, both in its bash block and in the Next sync step 1 parenthetical. The Pass-through table gains one third-party-capability row naming `ponytail.*` and `musk.*`. The `## Modelled` section is untouched and names neither."
    - "Gate: the three tie tests pass, `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0, and `rtk proxy cargo test --no-fail-fast` fails only `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (expected on this machine). All three files land in ONE commit."
  artifacts:
    - path: src/state_reader/config_json.rs
      provides: "The baseline trio at 1.18.0 / v1.18.0 / 1.18.0 with re-measured doc comments, and the GSD_HOME resolver docstring citing runtime-homes.cts resolveGsdHome (v1.18.0)"
      contains: "pub const GSD_CORE_SYNCED_COMMIT: &str = \"v1.18.0\";"
    - path: docs/GSD-CORE-SYNC.md
      provides: "The v1.18.0 header, the constants table, the v1.18.0 delta section, the third-party-capability pass-through row and the v1.19.0/v1.18.0 recipe"
      contains: "## The v1.18.0 delta (quick task 261019-foc)"
    - path: README.md
      provides: "The GSD compatibility note naming 1.18.0"
      contains: "gsd-core 1.18.0 (`v1.18.0`), with oracle pin `1.18.0`"
  key_links:
    - from: src/state_reader/config_json.rs
      to: README.md
      via: "the_readme_compatibility_note_names_the_synced_baseline (src/ui/screens/detail.rs:16282) asserts the README paragraph contains all three constant values"
      pattern: "GSD_CORE_SYNCED_(VERSION|COMMIT|TREE_VERSION)"
    - from: src/state_reader/config_json.rs
      to: docs/GSD-CORE-SYNC.md
      via: "the_sync_record_names_every_modelled_key_and_the_measured_baseline (src/ui/screens/detail.rs:16209) asserts the record names the three values and lists every modelled key in the ## Modelled .. ## Pass-through span"
      pattern: "GSD_CORE_SYNCED_TREE_VERSION"
    - from: src/state_reader/config_json.rs
      to: scripts/install-conformance-oracle.sh
      via: "the script greps the single `pub const GSD_CORE_SYNCED_VERSION: &str =` declaration (line 100) and npm-installs @opengsd/gsd-core@<pin> in release.yml and pre-tag-check --container; there is no second literal pin to edit"
      pattern: "pub const GSD_CORE_SYNCED_VERSION"
---

<objective>
Move this build's gsd-core sync baseline from v1.17.0 to v1.18.0. The baseline lives in three places that tests tie together: config_json.rs's three constants, the docs/GSD-CORE-SYNC.md record, and README.md's compatibility note. All three move in ONE commit. In the same commit, record the measured v1.17.0..v1.18.0 delta and refresh the GSD_HOME resolver docstring to cite the rule's new upstream home. The gap analysis found no model gap, so no config key, default or loader behaviour changes in this build.

Purpose: this machine and every user on gsd-core 1.18.0 (on npm since 2026-10-19T00:09Z) currently see a false "GSD 1.18.0 newer than synced 1.17.0" warning. After this change 1.18.0 is in the in-sync range (floor and ceiling both 1.18.0). The CI conformance oracle also moves to the current release, and the sync record tells the next syncer exactly where this sync stopped.

Output: one commit touching src/state_reader/config_json.rs, docs/GSD-CORE-SYNC.md and README.md.

Measured by the planner on 2026-10-19 against ~/projects/node/gsd-core. The record's own motto is "run these, do not trust this file's numbers", so the executor re-runs the measurements:

- Baseline values:
  - `git describe --tags --always v1.18.0` = `v1.18.0`.
  - The full tag sha is 35f5a452239b897b8d6352c973e1e4efd683370f (the tag is lightweight: the tag and commit shas are equal).
  - The tag's creatordate is 2026-10-19.
  - `v1.18.0:package.json` version is 1.18.0.
  - `npm view @opengsd/gsd-core version` = 1.18.0, published 2026-10-19T00:09:12Z.
  - This machine's installed `~/.claude/gsd-core/VERSION` is 1.18.0.
- Step 2 key cells: 255 at v1.17.0 and 255 at v1.18.0. The sorted sets are identical (+0 / -0).
- `git diff v1.17.0 v1.18.0` shows no change to:
  - `gsd-core/bin/shared/config-schema.manifest.json` and `config-defaults.manifest.json`
  - `src/config.cts`
  - `src/secrets.cts`
  - `gsd-core/references/gsd-run-resolver.md`
- Files that did change:
  - `docs/CONFIGURATION.md` (+13/-5, prose only, no key cell changed).
  - `src/config-loader.cts` (+3/-3): both global-defaults lookups changed from `process.env['GSD_HOME'] || os.homedir()` to `resolveGsdHome()`, and the `os` import was dropped.
  - `src/runtime-homes.cts` (+18): new export `resolveGsdHome(fallbackHome = os.homedir())`, whose body is `return process.env['GSD_HOME'] || fallbackHome;`.
  - Upstream refs: issue #5017, fix commit 7cf70ada0, merged as PR #5463 (381d94e5d).
- The rule is identical to the one the app already implements:
  - resolveGsdHome's JSDoc says "set and non-blank", but its body is JS `||`, so a whitespace-only `GSD_HOME` is used verbatim.
  - That is exactly what `user_defaults_path_from` does. The Rust docstring's "non-empty" wording matches the CODE and stays.
- What `$GSD_HOME/.gsd/` holds at v1.18.0 (per CONFIGURATION.md):
  - defaults.json, consent.json
  - capability overlays (`$GSD_HOME/.gsd/capabilities/<id>/capability.json`, about line 1202)
  - the `*_api_key` files (about lines 251-257 and 272)
- `GSD_HOME` does not move the install root. Consistent with that, the app's install detection (src/state_reader/gsd_install.rs) never reads GSD_HOME.
- Capability manifests:
  - 45 `capabilities/*/capability.json` files changed, and all 45 bump `version` 1.17.0 -> 1.18.0.
  - Runtime-only additions (all under `runtime.hostBehaviors`), none of them config keys:
    - `defaultHomeIncludeRoot` in antigravity (`.gemini/antigravity`) and copilot (`.copilot`), from d65fc122f (#5123).
    - `retargetsGsdCoreProsePath: true` in kimi. The requester did not list it; see I-foc-3.
- `.planning` writer-side changes that the app does not read:
  - `update-plan-progress` reconciles the pending-plan tail of the ROADMAP Plans line (47aa62030, #4947).
  - `phase complete` never lowers the STATE body Completed Phases counter (18a6189ad, #4901). The app reads the frontmatter `progress.completed_phases`, not the body counter.
  - `phase add` writes `**Depends on:** Nothing` (or the nearest declared lower phase) instead of `N-1` (ce4249845, #5113). No `Depends on` parser exists anywhere in src/.
  - `/gsd-pause-work` handoff commits use `docs(pause):` instead of `wip:` (4d8c0ae79, #4943).
- smart-entry (8fba5096e, #4890): `phaseSummary` replaces `phasePosition` and only changes the `situation` text (for example "Phase 13 of 5" becomes a roadmap-scoped label). The JSON shape is unchanged. src/state_reader/queue_md.rs's parse_smart_entry_json reads only `actions`.
- Third-party keys:
  - The Community Capability Registry (`docs/registries/`, already present at v1.17.0) gained its `ponytail-everywhere` entry, which declares `configKeys` `ponytail.enabled` / `ponytail.enforcement` / `ponytail.level` (a59deb77c, #5380). It is docs-only.
  - `musk.*` is not in gsd-core at all. It is the `config` block of a locally installed third-party capability (`$GSD_HOME/.gsd/capabilities/musk`, version 0.3.1).
  - Both kinds land in GsdConfig's top-level `extra` map. `append_passthrough_entries` (src/ui/screens/detail.rs about line 12114) shows them read-only under `Not modelled`, and they round-trip on save.
- Other `1.17.0` strings in src/ are historical "since" citations and must NOT change: queue_md.rs:39/140, executor/runtime.rs:170 and detail.rs:8735 describe the `/gsd-<name>` form introduced IN 1.17.0. No script, CI workflow or test fixture pins 1.17.0 literally:
  - install-conformance-oracle.sh derives its pin from the constant.
  - release.yml has no version literal.
  - The test fixtures in state_reader/mod.rs:1515, app.rs:2598 and normal.rs:3395 read the constants.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@./CLAUDE.md
@docs/GSD-CORE-SYNC.md (lines 1-180: header, constants table, history paragraph, recipe, the v1.17.0 delta section to model on; then the `## Pass-through` table and `## Next sync`)
@src/state_reader/config_json.rs (lines 1-60 for the constants; lines 677-700 for the GSD_HOME_ENV / user_defaults_path docstring; the test `the_gsd_core_sync_baseline_is_recorded` near line 1577)
@README.md (lines 15-19, the `**GSD compatibility:**` paragraph)
@src/ui/screens/detail.rs (tie tests near lines 16209 and 16282; DEFAULTS_OPTION_COUNT at line 15035; read only, do not edit)
@scripts/install-conformance-oracle.sh (the VERSION_DECL grep near line 100; read only)

Analog: commit a254c32 "chore(sync): baseline gsd-core v1.17.0 (quick 261016-dx3)". It is the previous baseline bump of the same three files and shows the expected shape. Its plan is .planning/quick/261016-dx3-sync-baseline-to-gsd-core-v1-17-0-set-gs/261016-dx3-PLAN.md.

Inferred decisions, recorded for later audit (the human was unavailable):
- [INFERRED I-foc-1] One task, one commit, because the requester and the record's own rule both say so. The tracer-first slice is therefore this whole item.
- [INFERRED I-foc-2] Files: only the three listed. The requester asked to grep for other 1.17.0 pins tied to these constants, and the grep found none. The oracle script derives from the constant, release.yml carries no literal, and the fixtures read the constants. The remaining 1.17.0 strings in src/ are "introduced in 1.17.0" citations that stay true.
- [INFERRED I-foc-3] The kimi `runtime.hostBehaviors.retargetsGsdCoreProsePath: true` addition was measured but not in the requester's list. It is recorded beside `defaultHomeIncludeRoot` as another runtime-only field, so the record matches the measured diff.
- [INFERRED I-foc-4] The Pass-through table gains one row for third-party capability `configKeys` (`ponytail.*`, `musk.*`). The requester described them as "handled as read-only pass-through rows". The table is where the record distinguishes "deliberately pass-through" from "nobody noticed". This follows dx3's precedent of extending the table for a newly seen family. The `## Modelled` section is not touched.
- [INFERRED I-foc-5] The recipe moves to `REF=v1.19.0` / `PREV=v1.18.0`, and its "at the time of writing" parenthetical is refreshed. Left alone, the recipe's REF would equal the new baseline. dx3 did the same.
- [INFERRED I-foc-6] `./scripts/pre-tag-check.sh --container` is NOT run here; it needs a release tag and is release-process step 3. The SUMMARY records that the next release must run it to certify the 1.18.0 oracle. Meanwhile tests/driver_router_conformance.rs runs against this machine's installed gsd-core 1.18.0 in the full test run.
- [INFERRED I-foc-7] The GSD_HOME docstring keeps the word "non-empty". resolveGsdHome's JSDoc says "non-blank", but its code is `||`, and the Rust resolver mirrors the code. Writing the JSDoc's word would describe a trim that neither side performs.
</context>

<tasks>

<task type="tracer">
  <name>Task 1: Move the gsd-core baseline to v1.18.0 across constants, README and the sync record, refresh the GSD_HOME citation, and record the measured delta, in one commit</name>
  <files>src/state_reader/config_json.rs, docs/GSD-CORE-SYNC.md, README.md</files>
  <action>
**Step 0: re-measure. Do not copy the planner's numbers blind.**

Run the record's own recipe from docs/GSD-CORE-SYNC.md "How to re-measure" with CORE=~/projects/node/gsd-core, REF=v1.18.0 and PREV=v1.17.0. Run steps 1, 2 and 4. Add:
- the tag's full sha (`git rev-parse v1.18.0^{commit}`) and its creatordate;
- a no-change diff over src/config.cts, src/secrets.cts and gsd-core/references/gsd-run-resolver.md;
- `git diff v1.17.0 v1.18.0 -- src/runtime-homes.cts src/config-loader.cts`, to confirm the resolveGsdHome body;
- `git diff v1.17.0 v1.18.0 -- 'capabilities/*/capability.json'`, filtered to non-`version` lines, to confirm the runtime-only additions;
- the secret-heuristic key-cell command from the record's "Secret-bearing keys" section.

Run pipe-sensitive counts inside `rtk proxy bash -c '...'` so rtk cannot filter the pipeline. Never measure the CORE working tree: HEAD there is `next` at `v1.18.0-6-g8d3d29c46`. If any value differs from the objective's measured list, use the value you measured and say so in the SUMMARY.

**Step 1: src/state_reader/config_json.rs**

1. Set the three constants:
   - `GSD_CORE_SYNCED_VERSION` = "1.18.0"
   - `GSD_CORE_SYNCED_COMMIT` = "v1.18.0"
   - `GSD_CORE_SYNCED_TREE_VERSION` = "1.18.0"

   Keep each declaration on one line with the exact `pub const NAME: &str = "...";` shape. scripts/install-conformance-oracle.sh greps the VERSION declaration, and that grep must keep matching exactly one line.
2. Update the doc comments above the three constants, following the same pattern dx3 used:
   - Append 261019-foc to the "(quick tasks ...)" list in the VERSION comment.
   - Reword "At the v1.17.0 sync (261016-dx3) the target is a published tag" to name the v1.18.0 sync and 261019-foc.
   - Set the `text` recipe block's REF to v1.18.0.
   - In the COMMIT comment, name the `v1.18.0` release tag and its full sha 35f5a452239b897b8d6352c973e1e4efd683370f.
   - No 1.17.0 string may remain from the top of the file through the TREE_VERSION declaration. Release history belongs in docs/GSD-CORE-SYNC.md.
3. GSD_HOME docstring on `user_defaults_path` (about lines 682-691). Replace the parenthetical that cites the v1.16.0 config-loader commit with a citation of gsd-core `src/runtime-homes.cts` `resolveGsdHome` (v1.18.0, #5017). Name the rule as `process.env.GSD_HOME || os.homedir()`; upstream's `fallbackHome` parameter defaults to `os.homedir()`. Optionally add one clause noting that since v1.18.0 the config loader and every other defaults-store reader and writer call that one function.
   - Keep "a non-empty `GSD_HOME` roots the store" exactly as worded (I-foc-7).
   - Do not change the `user_defaults_path_from` docstring or any code. This is docstring-only, with no behaviour change.
4. No other line in this file changes.

**Step 2: README.md**

In the single `**GSD compatibility:**` paragraph, change the last sentence to "build is synced to gsd-core 1.18.0 (`v1.18.0`), with oracle pin `1.18.0`." Keep the paragraph's line wrapping (the sentence stays on line 19), the `docs/GSD-CORE-SYNC.md` link and the single marker. Touch nothing else in README.md.

**Step 3: docs/GSD-CORE-SYNC.md**

Make these edits in the same working tree as Steps 1 and 2.

(a) Header (lines 3-6). Rewrite it to say the config surface is synced against gsd-core `v1.18.0`, with:
- the full sha in backticks
- `package.json` 1.18.0
- tagged 2026-10-19
- synced on 2026-10-19
- `GSD_CORE_SYNCED_VERSION` pinned at `1.18.0`, the latest version published on npm

(b) Constants table (lines 13-15). Set the Value column to `1.18.0` / `v1.18.0` / `1.18.0`, keeping each row's leading `| \`CONSTANT\` | \`value\` |` cell shape. Nothing from the top of the file through the "This file is the human-readable half" line may name 1.17.0.

(c) History paragraph (ends about line 36 with the v1.17.0 delta link). Keep the v1.17.0 sentence. Append one sentence: the v1.18.0 sync (quick task 261019-foc) was baseline-only, with no config key or default change and a loader refactor that keeps the `GSD_HOME` rule. Link it as [The v1.18.0 delta](#the-v1180-delta-quick-task-261019-foc).

(d) Recipe (about lines 66-75).
- In the bash block, set `REF=v1.19.0` (comment: the sync target, the next release tag) and `PREV=v1.18.0` (comment: the previous GSD_CORE_SYNCED_COMMIT). Keep the column alignment of the trailing comments.
- Refresh the "(at the time of writing: ...)" parenthetical with the re-measured branch, `git describe --tags HEAD` and `package.json` version of the local checkout (planner saw `next`, `v1.18.0-6-g8d3d29c46`, 1.18.0). This is I-foc-5.

(e) New delta section. Insert `## The v1.18.0 delta (quick task 261019-foc)` immediately BEFORE `## The v1.17.0 delta (quick task 261016-dx3)`, so the newest delta comes first. Follow it with a `---` separator line.
- The section must contain no `## Modelled` or `## Pass-through` heading, because the tie test splits on those. Use bold lead-ins, not `###` headings.
- Match the v1.17.0 section's style: an opening "Measured on 2026-10-19 between `v1.17.0` and `v1.18.0` (`<full sha>`), against the tags, not the working tree:" line, then:
   1. A measurement table with columns Measurement | v1.17.0 | v1.18.0 | Change, and these rows:
      - CONFIGURATION.md key cells (step 2): 255 / 255, **+0 / -0**.
      - `config-schema.manifest.json`: unchanged (no diff).
      - `config-defaults.manifest.json`: unchanged.
      - `src/config.cts`, `src/secrets.cts`: unchanged.
      - `src/config-loader.cts`: global-defaults home lookup now `resolveGsdHome()` (#5017), same rule.
      - Existing keys whose default changed: **none**.
      - `capabilities/*/capability.json`: 45 files bump `version` 1.17.0 -> 1.18.0; 3 also gain a runtime-only `hostBehaviors` field.

      Follow the table with one sentence: the Modelled count stays 135 and `DEFAULTS_OPTION_COUNT` stays 135.
   2. A **`GSD_HOME` and `resolveGsdHome()` (#5017)** paragraph:
      - The config loader's two inline `process.env['GSD_HOME'] || os.homedir()` lookups became calls to the new `resolveGsdHome()` in `src/runtime-homes.cts`. Its body is the same `||` rule, now shared by every reader and writer of the store.
      - `GSD_HOME` moves only `$GSD_HOME/.gsd/`: capability overlays (`capabilities/<id>/capability.json`), `consent.json`, `defaults.json` and the `*_api_key` files. It does not move the install root.
      - This build's `user_defaults_path` already implements that rule, so only its docstring citation moved. The install detection in `src/state_reader/gsd_install.rs` never consulted `GSD_HOME`.
   3. A bullet list headed "Config-adjacent diffs in the range that are NOT new keys":
      - The 45 capability `version` bumps.
      - `runtime.hostBehaviors.defaultHomeIncludeRoot` added to antigravity (`.gemini/antigravity`) and copilot (`.copilot`) (#5123).
      - kimi's `runtime.hostBehaviors.retargetsGsdCoreProsePath: true` (I-foc-3).
      - The `.planning` writer-side changes this build does not read:
        - the ROADMAP Plans line pending-tail reconcile in `update-plan-progress` (#4947);
        - the STATE body Completed Phases counter ratchet in `phase complete` (#4901). This build reads the frontmatter `progress.completed_phases`;
        - `phase add` writing `Depends on: Nothing` (or the nearest declared lower phase) instead of `N-1` (#5113). No `Depends on` parser exists here;
        - `/gsd-pause-work` handoff commits typed `docs(pause):` instead of `wip:` (#4943).
      - smart-entry's `situation` text fix (#4890). Only the summary wording changed; the JSON shape is unchanged, and `parse_smart_entry_json` reads only `actions`.
      - The Community Capability Registry (`docs/registries/`) gained its `ponytail-everywhere` entry (#5380). It is docs-only and ships no code.
   4. A **Third-party capability keys** paragraph:
      - `ponytail.*` (that registry entry's `configKeys`) and `musk.*` (a locally installed third-party capability's `config` block, not part of gsd-core) are not gsd-core keys.
      - When a project's config.json carries them, they land in the top-level `extra` map and show as read-only `Not modelled` pass-through rows that round-trip on save, with no code change.
      - They are recorded under Pass-through (I-foc-4).
   5. One sentence: the secret-bearing re-diff found no change. `src/secrets.cts` is unchanged, and the heuristic still matches only the four exempt budget keys. Use the re-measured result.
   6. One sentence: `gsd-core/references/gsd-run-resolver.md` is unchanged, so the resolver-order comments in `src/state_reader/queue_md.rs` stay accurate.

(f) Pass-through table. Append one row at the END of the table, after the `claude_md_assembly.mode` row (I-foc-4).
- Family cell: `ponytail.*`, `musk.*` (third-party capability `configKeys`).
- Reason: owned by community or locally installed capabilities rather than by gsd-core; open-ended by capability name, so a typed row is not expressible; they reach the tab through the top-level extra map.

(g) Next sync. In step 1, replace the current REF/PREV parenthetical with (`REF=v1.19.0`, `PREV=v1.18.0`), so it agrees with the bash block from (d).

(h) Do NOT edit the `## Modelled` section or its 135-row inventory. Do NOT add any ponytail or musk key there: listing one would claim a typed row that does not exist. Do not edit the existing v1.17.0 or older delta sections.

**Step 4: gate, then ONE commit**

1. Run the verify block below.
2. Stage the three files BY PATH. Do not use `git add -A`: an untracked `.planning/state.json` is in the tree and must stay out of this commit.
3. Commit with message `chore(sync): baseline gsd-core v1.18.0 (quick 261019-foc)`. The body summarises the delta and lists I-foc-1..7, followed by the session's attribution lines.
4. Do not edit src/ui/screens/detail.rs, scripts/, .github/, Cargo.toml or any file outside files_modified. No Cargo.toml version bump, no tag, no push.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib -- the_gsd_core_sync_baseline_is_recorded the_sync_record_names_every_modelled_key_and_the_measured_baseline the_readme_compatibility_note_names_the_synced_baseline</automated>
    <automated>F=src/state_reader/config_json.rs && test "$(grep -E '^[[:space:]]*pub const[[:space:]]+GSD_CORE_SYNCED_VERSION[[:space:]]*:[[:space:]]*&str[[:space:]]*=' $F | head -1 | sed -E 's/.*"(.*)".*/\1/')" = "1.18.0" && test "$(grep -cE '^[[:space:]]*pub const[[:space:]]+GSD_CORE_SYNCED_VERSION[[:space:]]*:[[:space:]]*&str[[:space:]]*=' $F)" -eq 1 && test "$(grep -E '^pub const GSD_CORE_SYNCED_COMMIT: &str =' $F | head -1 | sed -E 's/.*"(.*)".*/\1/')" = "v1.18.0" && test "$(grep -E '^pub const GSD_CORE_SYNCED_TREE_VERSION: &str =' $F | head -1 | sed -E 's/.*"(.*)".*/\1/')" = "1.18.0" && test "$(grep -c '35f5a452239b897b8d6352c973e1e4efd683370f' $F)" -ge 1 && test "$(awk '{print} /^pub const GSD_CORE_SYNCED_TREE_VERSION/{exit}' $F | grep -c '1\.17\.0')" -eq 0 && test "$(awk '{print} /^pub const GSD_CORE_SYNCED_TREE_VERSION/{exit}' $F | grep -c '261019-foc')" -ge 1 && R="$(awk '/^pub const GSD_HOME_ENV/{f=1} f{print} /^pub fn user_defaults_path\(\)/{exit}' $F)" && printf '%s\n' "$R" | grep -qF 'resolveGsdHome' && printf '%s\n' "$R" | grep -qF 'runtime-homes.cts' && printf '%s\n' "$R" | grep -qF 'v1.18.0' && printf '%s\n' "$R" | grep -qF '#5017' && printf '%s\n' "$R" | grep -qF 'non-empty' && test "$(grep -c 'fd1bfd09e' $F)" -eq 0 && echo CONSTANTS-OK</automated>
    <automated>test "$(grep -cF 'gsd-core 1.18.0 (`v1.18.0`), with oracle pin `1.18.0`' README.md)" -ge 1 && test "$(grep -c '1\.17\.0' README.md)" -eq 0 && test "$(grep -cF '**GSD compatibility:**' README.md)" -eq 1 && echo README-OK</automated>
    <automated>D=docs/GSD-CORE-SYNC.md && H="$(awk '/^This file is the human-readable half/{exit} {print}' $D)" && test "$(printf '%s\n' "$H" | grep -c '1\.17\.0')" -eq 0 && printf '%s\n' "$H" | grep -qF '35f5a452239b897b8d6352c973e1e4efd683370f' && printf '%s\n' "$H" | grep -qF '2026-10-19' && grep -qE '^\| `GSD_CORE_SYNCED_VERSION` \| `1\.18\.0` \|' $D && grep -qE '^\| `GSD_CORE_SYNCED_COMMIT` \| `v1\.18\.0` \|' $D && grep -qE '^\| `GSD_CORE_SYNCED_TREE_VERSION` \| `1\.18\.0` \|' $D && grep -qE '^REF=v1\.19\.0 ' $D && grep -qE '^PREV=v1\.18\.0 ' $D && test "$(grep -cE '^(REF=v1\.18\.0|PREV=v1\.17\.0) ' $D)" -eq 0 && grep -qF '(`REF=v1.19.0`, `PREV=v1.18.0`)' $D && test "$(grep -cF '(`REF=v1.18.0`, `PREV=v1.17.0`)' $D)" -eq 0 && grep -qF '#the-v1180-delta-quick-task-261019-foc' $D && N18="$(grep -nxF '## The v1.18.0 delta (quick task 261019-foc)' $D | cut -d: -f1)" && N17="$(grep -nxF '## The v1.17.0 delta (quick task 261016-dx3)' $D | cut -d: -f1)" && test -n "$N18" && test -n "$N17" && test "$N18" -lt "$N17" && for t in resolveGsdHome runtime-homes.cts '#5017' consent.json defaults.json _api_key defaultHomeIncludeRoot antigravity copilot retargetsGsdCoreProsePath 'Depends on' 'docs(pause):' situation docs/registries ponytail musk 255 135; do awk '/^## The v1\.18\.0 delta/{f=1;next} /^## /{f=0} f' $D | grep -qF -- "$t" || { echo "DELTA-MISSING $t"; exit 1; }; done && test "$(awk '/^## Modelled/{f=1} /^## Pass-through/{f=0} f' $D | grep -cE 'ponytail|musk')" -eq 0 && test "$(awk '/^## Pass-through/{f=1} /^### /{f=0} f' $D | grep -E '^\| ' | grep -c 'ponytail')" -ge 1 && test "$(grep -c '^## Modelled' $D)" -eq 1 && test "$(grep -c '^## Pass-through' $D)" -eq 1 && grep -qF 'const DEFAULTS_OPTION_COUNT: usize = 135;' src/ui/screens/detail.rs && echo RECORD-OK</automated>
  </verify>
  <done>
- The three tie tests pass, shown as `test result: ok. 3 passed` in raw output.
- CONSTANTS-OK, README-OK and RECORD-OK are all printed. Planner baselines at HEAD 084f1fa:
  - every deliverable check fails: VERSION decl 1.17.0; 6 `1.17.0` hits above the TREE decl; resolveGsdHome 0 in the docstring region; 1 old loader-sha hit; README 1.17.0 count 1; 6 `1.17.0` hits in the doc header; 2 old REF/PREV lines; 1 old step-1 parenthetical; 0 ponytail pass-through rows.
  - the invariant checks pass by design: single VERSION decl, single README marker, `non-empty` wording present, one `## Modelled` and one `## Pass-through` heading, DEFAULTS_OPTION_COUNT 135.
- `git show --stat --format= HEAD` lists exactly src/state_reader/config_json.rs, docs/GSD-CORE-SYNC.md and README.md, in one commit.
- The SUMMARY records:
  - the re-measured values, and any deviation from the planner's numbers;
  - I-foc-1..7;
  - the release-time obligation to run `./scripts/pre-tag-check.sh --container vX.Y.Z` to certify the 1.18.0 oracle.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| npm registry / upstream gsd-core repo -> this build's oracle pin | `GSD_CORE_SYNCED_VERSION` decides which `@opengsd/gsd-core` version CI npm-installs as the conformance oracle (install-conformance-oracle.sh, in release.yml and pre-tag-check --container). |
| config_json.rs declaration -> CI shell script | install-conformance-oracle.sh greps the source text of one `pub const` line, so the Rust source is input to a shell parser. |
| sync record -> future syncer | docs/GSD-CORE-SYNC.md is trusted as the measured baseline the next sync diffs from. |
| `GSD_HOME` env -> defaults-file path | user_defaults_path roots `$GSD_HOME/.gsd/defaults.json`; this plan changes only the docstring that cites the upstream rule. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-foc-01 | Spoofing | GSD_CORE_SYNCED_VERSION oracle pin | medium | mitigate | The pin is MEASURED with `npm view @opengsd/gsd-core version` (1.18.0) and cross-checked against `git show v1.18.0:package.json` and the tag sha in the upstream clone. Step 0 re-runs both. `the_gsd_core_sync_baseline_is_recorded` asserts that COMMIT contains VERSION and that pin <= tree version. |
| T-foc-02 | Tampering | install-conformance-oracle.sh declaration grep | medium | mitigate | The single-line `pub const GSD_CORE_SYNCED_VERSION: &str = "1.18.0";` shape is kept. The CONSTANTS gate replays the script's own extraction (regex, `head -1`, sed), requires `1.18.0`, and requires the regex to match exactly one line. A reformat or a doc-comment copy therefore cannot make CI install the wrong version or fail to parse. |
| T-foc-03 | Repudiation | docs/GSD-CORE-SYNC.md delta claims | low | mitigate | Every delta number is re-measured in Step 0 against the REF tag, never the working tree, and deviations are logged in the SUMMARY. The Modelled section is gated to stay free of ponytail/musk, so the record never claims a typed row that does not exist. |
| T-foc-04 | Information disclosure | user_defaults_path resolver (GSD_HOME) | low | accept | Docstring-only change. The resolver code and its tests are untouched, so which defaults file is read cannot change. The CONSTANTS gate keeps the "non-empty" wording, so the doc cannot drift toward a trim the code does not perform. |
| T-foc-SC | Tampering | npm install of @opengsd/gsd-core in CI | medium | mitigate | No package-manager install task in this plan and no new package name. The existing oracle package moves to a version verified as published (`npm view`, 2026-10-19) and matching the upstream `v1.18.0` tag. The release-time `pre-tag-check.sh --container` run certifies it before any tag (I-foc-6). |
</threat_model>

<verification>
Full regression, run after the commit. Wrap whole pipelines in `rtk proxy bash -c` so rtk cannot filter the counts.

- `rtk proxy bash -c 'cargo test --no-fail-fast 2>&1 | grep -E "^test result:|^    [a-z_:]+$|FAILED"'`
  - Every `test result:` line is `ok` except one, which reports exactly 1 failed: `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. That failure is expected on this machine, because the local git differs from the pinned 2.55.0. Do not fix it.
  - No other failure is acceptable.
  - tests/driver_router_conformance.rs runs against this machine's installed gsd-core 1.18.0.
- `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0.
- `rtk proxy git show --stat --format=%s HEAD` shows the single sync commit with exactly the three files.
</verification>

<success_criteria>
- The three constants read 1.18.0 / v1.18.0 / 1.18.0. A user on gsd-core 1.18.0 is now in the in-sync range (floor and ceiling both 1.18.0).
- The GSD_HOME resolver docstring cites `runtime-homes.cts` `resolveGsdHome` (v1.18.0, #5017), and behaviour is unchanged.
- README's compatibility note names 1.18.0 (`v1.18.0`) with oracle pin `1.18.0`.
- The sync record carries the v1.18.0 header, the measured v1.18.0 delta section (ahead of v1.17.0's), the third-party-capability pass-through row and the REF=v1.19.0 / PREV=v1.18.0 recipe.
- The Modelled count and DEFAULTS_OPTION_COUNT stay 135.
- One commit. The full suite fails only the known envelope::policy git-version test, and clippy is clean.
</success_criteria>

<output>
Create `.planning/quick/261019-foc-gsd-core-v1-18-0-sync-baseline/261019-foc-SUMMARY.md` when done.
</output>
