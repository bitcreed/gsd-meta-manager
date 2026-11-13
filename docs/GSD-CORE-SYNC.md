# gsd-core config sync record

**Config surface synced against gsd-core `v1.20.1`**
(`9385f4618b52ef68ed8a5e8bdd93cfb25bd87765`, `package.json` 1.20.1, tagged
2026-11-05) **on 2026-11-05. Conformance oracle / `GSD_CORE_SYNCED_VERSION`
pinned at `1.20.1`**, the latest version published on npm.

`src/state_reader/config_json.rs` carries the machine-readable copy of those
values as three constants, which at this sync all name the same release:

| Constant | Value | What it means |
|---|---|---|
| `GSD_CORE_SYNCED_VERSION` | `1.20.1` | The **conformance-oracle pin**: the latest gsd-core release published on npm at or below the synced commit. `scripts/install-conformance-oracle.sh` greps this declaration and runs `npm install @opengsd/gsd-core@<version>` — in the publish job (`.github/workflows/release.yml`) and inside `./scripts/pre-tag-check.sh --container`. The floor of the installed-GSD comparison's in-sync range. |
| `GSD_CORE_SYNCED_COMMIT` | `v1.20.1` | The tree the config surface was diffed against, as `git describe --tags --always` spells it. |
| `GSD_CORE_SYNCED_TREE_VERSION` | `1.20.1` | The `package.json` version of that tree. The ceiling of the installed-GSD comparison's in-sync range (quick task 260926-j0a). |

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
[The v1.18.0 delta](#the-v1180-delta-quick-task-261019-foc). The v1.19.0 sync
(quick task 261027-184) modelled one new key, `git.repo_tagging` — see
[The v1.19.0 delta](#the-v1190-delta-quick-task-261027-184). The v1.20.0 sync
was two quick tasks: 261103-em6 re-ported the `.planning` formats (the ROADMAP
phase-entry grammar, the Progress-table owner and the milestone slice), and
261103-f5i moved the baseline, promoted the two planner stall knobs to typed
rows and corrected three help texts — see
[The v1.20.0 delta](#the-v1200-delta-quick-task-261103-f5i). The v1.20.1 sync
(quick task 261105-e7u) was baseline-only, a hotfix release with no config key,
default or loader change; the milestone-prefixed phase-id readers it touches are
ported by quick task 261105-e7v — see
[The v1.20.1 delta](#the-v1201-delta-quick-task-261105-e7u).

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
`v1.20.1-24-g9d6b6c728` with `package.json` 1.20.1, i.e. 24 commits past the
last tag it was described from) — not the sync target. A
plain `cat "$CORE/docs/CONFIGURATION.md"` or `node -p require(package.json)`
measures the wrong tree.

```bash
CORE=~/projects/node/gsd-core
REF=v1.21.0                          # the sync target: the next release tag
PREV=v1.20.1                         # the previous GSD_CORE_SYNCED_COMMIT

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

## The v1.20.1 delta (quick task 261105-e7u)

Measured on 2026-11-05 between `v1.20.0` and `v1.20.1`
(`9385f4618b52ef68ed8a5e8bdd93cfb25bd87765`), against the tags, not the
working tree. v1.20.1 is a hotfix release: 20 commits, two of them
release-branch merges (#5894), and a CHANGELOG with a Fixed section only.

| Measurement | v1.20.0 | v1.20.1 | Change |
|---|---:|---:|---|
| CONFIGURATION.md key cells (step 2) | 263 | 263 | **+0 / -0** (the file has no diff) |
| `config-schema.manifest.json`, `config-defaults.manifest.json`, `gsd-core/templates/config.json` | — | — | unchanged (no diff) |
| `src/config.cts`, `src/secrets.cts`, `src/config-loader.cts` | — | — | unchanged (no diff) |
| Existing keys whose default changed | — | — | **none** |
| `capabilities/*/capability.json` | 1.20.0 | 1.20.1 | 45 files bump `version`; no other line changes |

The Modelled count stays 138 and `DEFAULTS_OPTION_COUNT` stays 138. No Defaults
row, typed field or reader changes in this build.

**Milestone-prefixed phase ids (#5675, #5676).** Under
`phase_id_convention: milestone-prefixed`, `src/phase-id.cts` now has one owner
per convention for the numeric phase token: `phaseNumberTokenSource(convention)`
returns the #5626 M-NN grammar `\d+(?:-\d+)?[A-Z]?(?:\.\d+)*` for
milestone-prefixed and the unchanged numeric source otherwise. #5676
(30322c042) covers `phase complete`, `state sync`, init progress, init
manager's heading scans and checked-phase fallback, `find-phase` and the
planning snapshot; #5675 (a195bbc38) covers `drift-guard phase-status` and
workstream progress. A `03-02-beta` directory is phase `3-02`, not `3`. For
every other convention the token source is byte-identical, so default-convention
`.planning` reads are unchanged. This build's port of the grammar is owned by
quick task 261105-e7v (same batch); this sync changes no reader.

**`<project>/.gsd/` tool state (#5682, #5757).** Both the dispatch-isolation
sentinel (`.gsd/dispatch-isolation-sentinel.json`) and the harness fork-probe
cache (`.gsd/harness-fork-probe.json`) already lived under `<project>/.gsd/` at
v1.20.0. v1.20.1 adds per-dispatch keyed records at
`.gsd/dispatch-isolation/<phase>--<plan>.json` (#5682, a8e0f1a1d), so two
orchestrators in one checkout stop overwriting each other's decision. It also
adds `src/gsd-tool-dir.cts` as the single owner of the directory (#5757,
3e2e1f331), which never writes through a `.gsd` symlink resolving outside the
project and seeds `.gsd/.gitignore` so the cache stays out of git. This build
reads neither. The only `.gsd` paths under `src/` are `$HOME/.gsd` (the sandbox
read-only bind list, `src/launcher/home.rs`) and `$GSD_HOME/.gsd` (the defaults
store), and the only dispatch-isolation hit is instruction prose in
`src/launcher/assets/MASTER-PROMPT.md`.

**Other areas.**

- **The CLI.** `commands/gsd` changes only `new-project.md` and
  `new-milestone.md`. The closing hand-off now recommends `/gsd:discuss-phase`,
  with `/gsd:plan-phase` to plan directly and `/gsd:execute-phase` to execute
  (#5723), and both `requires:` lists gain `discuss-phase` and `execute-phase`.
  No verb is added or removed, and this app calls no new verbs. Also:
  `effort sync --apply` records rewritten agent hashes in
  `gsd-file-manifest.json` (#5683), which this build does not read, and
  `phase insert` names the owning milestone in its error (#5679).
- **Routing.** `init.manager`'s output shape is unchanged. `src/init.cts` only
  threads the phase-id convention into its scans, byte-identical except under
  milestone-prefixed, and `src/smart-entry.cts` has no diff.
  `tests/driver_router_conformance.rs` is green against the installed gsd-core
  1.20.1 (see quick task 261105-e7u's SUMMARY for the run). Any
  milestone-prefixed conformance fixture belongs to 261105-e7v.
- **Agents/worktrees.** `agents/` has no diff. The isolation-guard hook change is
  the #5682 keyed record above.
- **Codex and installs.** `capabilities/codex/capability.json` changes only
  `version`, and the upstream install-tree fixtures (`tests/fixtures`) have no
  diff. #5715 (5a7cfecf7) rewrites each config-dir reference once in installed
  files (nested or `$&` config dirs, including the Codex agent TOML). The
  install roots `src/state_reader/gsd_install.rs` reads
  (`<config home>/gsd-core/VERSION` for Claude and Codex, global and
  project-local) are unchanged.
- **Secret-bearing keys.** `git diff` of `src/secrets.cts` and
  `gsd-core/references/gsd-run-resolver.md` over the range is empty: no change.
  The resolver-order comments in `src/state_reader/queue_md.rs` stay accurate.
- **Release pipeline only.** #5890, #5894, #5902, #5903 and #5904 (merge-commit
  release PRs, ancestry checks, changeset lint, issue-link and title gates,
  prompt-injection-scan fixtures) plus two CI timeout history updates. Nothing
  here reads them.

---

## The v1.20.0 delta (quick task 261103-f5i)

Measured on 2026-11-03 between `v1.19.0` and `v1.20.0`
(`b14172e38d02aeab483856949453264228886517`), against the tags, not the
working tree:

| Measurement | v1.19.0 | v1.20.0 | Change |
|---|---:|---:|---|
| CONFIGURATION.md key cells (step 2) | 262 | 263 | **+1 / -0**: `review.context_window_tokens` (object, default `{}`, each slug `-1`; first commit `a032f9fed`, #4559 / PR #5839, 2026-11-01; first tag `v1.20.0`) |
| `config-schema.manifest.json`, `config-defaults.manifest.json`, `gsd-core/templates/config.json` | — | — | unchanged (no diff) |
| `src/config.cts`, `src/secrets.cts` | — | — | unchanged (no diff) |
| `src/config-loader.cts` | — | — | an `env` parameter on `isGitIgnored` only (3 insertions, 3 deletions); no config semantics change |
| Existing keys whose default changed | | | **none**. The manifest already had `inline_plan_threshold` at `2`; only the DOC default was corrected, from `3` to `2` |
| `capabilities/*/capability.json` | | | 45 files bump `version` 1.19.0 -> 1.20.0; 10 of them register `review.context_window_tokens.<slug>`: antigravity, claude, codex, cursor, kimi-code, llama_cpp, lm_studio, ollama, opencode, qwen (the capability directories are `llama-cpp` and `lm-studio`; the key slugs use underscores) |

**How this build models the change.**

- **Planner stall knobs promoted (typed rows).** `planner.stall_threshold_minutes`
  and `planner.stall_detect_interval_minutes` were documented key cells at
  v1.19.0 already, so they are a promotion, not a key-cell delta; they are
  pinned by `PROMOTED_PLANNER_KEYS_1_20_0`, and there is no
  `RESYNCED_KEYS_1_20_0`. gsd-core 1.20.0 (#3923) lists both in
  `/gsd-config --advanced` (Planning Tuning) as whole minutes, minimum 1,
  defaults 10 and 5 (`src/config.cts`), which reverses 260926-gtk's
  pass-through decision (I-3) [inferred — audit]. `since` is `v1.10.0`,
  MEASURED by step 5: `git log v1.20.0 --reverse -S<dotted key>` gives
  `067a4d1c6` (#2650, PR #3015, 2026-08-03) for both, and `tag --contains`
  gives `v1.10.0`. The rows are `Option<u32>` Integer rows, skipped when unset
  so a clear removes the key instead of writing `null`. A cycle from unset
  writes 1, steps by 1, and wraps to 1 after 30 (threshold) or 15 (interval)
  — never to 0, because upstream's minimum is 1. There is no `set_config_value`
  arm, as for every Integer row in this build. A hand-edited non-integer
  value is a whole-file parse failure (the accepted divergence of
  `PlannerConfig`'s I-5), and a hand-edited `0` is shown verbatim. Modelled and
  `DEFAULTS_OPTION_COUNT` go 136 -> 138. `executor.stall_*` stays pass-through:
  `settings-advanced.md` does not expose it.
- **`review.context_window_tokens` stays pass-through.** See the Pass-through
  table; it joins `review.effort.<slug>` in the templated row.
- **Doc-only changes to existing keys:**
  - `workflow.inline_plan_threshold`: the doc default was corrected to `2` and
    its meaning is now inline execution — plans with that many tasks or fewer
    execute inline instead of spawning an executor subagent, and `0` always
    spawns one. The help text was fixed (the pre-1.20 help gave default 3 and
    "own file" semantics).
  - `phase_naming`: documented as `"sequential"` | `"custom"`
    (case-insensitive), default sequential, other values keep numeric
    allocation. The help text was fixed; the row stays a free-form String
    (inferred I-7).
  - `phase_id_convention`: the help gains `bracket`; the row stays a String
    row (inferred I-7).
  - `planner.stall_*`: the docs now point at `/gsd-config --advanced`.
  - `review.max_prompt_tokens*`: the cap now REFUSES an oversized prompt
    instead of trimming it, and the default is 65% of the lane's context
    window. These keys are pass-through here, so this is recorded only.

**Areas** (each from a measured command or file):

- **.planning formats.** Re-ported in quick task 261103-em6 (commits
  `e3b32a1`, `b2af8ac`, `785fae9`): the ROADMAP phase-entry grammar (#5763);
  the Progress-table owner (#5761), which is fence-aware, uses
  `## Progress Tracker` and has no fall-through; and the milestone slice
  (#5165). DEFERRED: custom phase ids and bracket ids (inferred I-8; see
  "Next sync" item 8). The bracket read paths predate 1.20; the write paths are
  new in 1.20 (`57942161d`, #5528 / PR #5750; custom directories `3befe44bd`,
  #5186 / PR #5756; #4304 in `src/smart-entry.cts`). A project that uses custom
  ids, `.phase-id` markers or bracket ids is not recognised: the readers
  abstain.
- **Routing.** The `init.manager` output shape is unchanged;
  `tests/init-manager.test.cjs` adds convention cases only, and
  `src/smart-entry.cts` only threads the custom/bracket directory context. The
  router conformance test `tests/driver_router_conformance.rs` is green
  against the installed gsd-core 1.20.0 (`~/.claude/gsd-core/VERSION`); see this
  task's SUMMARY for the run.
- **The CLI.** `git diff --stat v1.19.0 v1.20.0 -- commands/gsd` shows only
  `update.md` (`--reapply --interactive`). This app calls no new verbs.
  [inferred — audit]
- **Agents/worktrees.** `agents/` changed 5 files (`gsd-code-fixer`,
  `gsd-code-fixer.compact`, `gsd-code-reviewer`, `gsd-code-reviewer.compact`,
  `gsd-executor`); the executor now writes ROADMAP progress BEFORE STATE.
  Branch and worktree naming did not change. The `.compact` variants live under
  `agents/` in the source tree (29 of them at v1.20.0:
  `git ls-tree --name-only v1.20.0 agents/ | grep -c compact`) and under
  `~/.claude/gsd-core/agents/` when installed. This app takes the agent type
  verbatim from subagent meta `agentType` (`src/agents/adapters/claude.rs`) and
  enumerates no agent files, so `.compact` stems cannot be misclassified:
  `grep -rnE 'compact\.md|agents/gsd-' src` hits only the
  `workflow.compact_content` help string.
- **Codex.** `capabilities/codex/capability.json` changes only the `version`
  and the reviewer-lane context-window keys. `tests/fixtures/install-tree/codex.json`
  adds two references files (`cicd-review-patterns.md`, `necessity-review.md`).
- **Secret-bearing keys.** `git diff v1.19.0 v1.20.0 -- src/secrets.cts
  gsd-core/references/gsd-run-resolver.md` is empty: no change.

**Deferred.** Custom and bracket phase identity, which is scope-changing
(it generalises phase identity beyond the numeric `PhaseNum`). It has an owner
under "Next sync" item 8.

---

## The v1.19.0 delta (quick task 261027-184)

Measured on 2026-10-27 between `v1.18.0` and `v1.19.0`
(`9dc57a772195de423ab788e4d0d8734b162db35d`), against the tags, not the
working tree:

| Measurement | v1.18.0 | v1.19.0 | Change |
|---|---:|---:|---|
| CONFIGURATION.md key cells (step 2) | 255 | 262 | **+7 / -0**: one key, `git.repo_tagging`. The other six cells (`environment`, `reference`, `release`, `repo`, `status`, `tag`) are the columns of the new per-repository release-map table, not keys |
| `config-schema.manifest.json` | — | — | +`git.repo_tagging` |
| `config-defaults.manifest.json` | — | — | +`repo_tagging: "single"` |
| `gsd-core/templates/config.json` | — | — | +`repo_tagging` |
| `src/config-loader.cts` | — | — | `resolveRepoTagging`: only `per_repo` is honoured; anything else reads `single` and never throws |
| `src/config.cts` | — | — | `VALID_REPO_TAGGING_MODES = ['single','per_repo']` and the `config-set` enum assertion |
| `src/secrets.cts` | — | — | unchanged (no diff) |
| Existing keys whose default changed | | | **none** |
| `capabilities/*/capability.json` | | | 45 files bump `version` 1.18.0 -> 1.19.0; `pi` also changes runtime `bun` -> `node` and gains install surfaces; none adds or changes a config key |

**How this build models the key.** `GitConfig.repo_tagging` is an
`Option<String>`, skipped when unset so a clear removes the key instead of
writing `null`. The Defaults row is an Enum `[single, per_repo]` with built-in
default `single` and `since` `v1.19.0`, MEASURED by step 5: the first commit
naming it is `754e886bf` (#5706, PR #5720, 2026-10-25) and its first
containing tag is `v1.19.0`. Set, cycle and clear write `/git/repo_tagging`; a
cycle from unset starts at `single`, so the first press lands on `per_repo`.
Modelled and `DEFAULTS_OPTION_COUNT` go 135 -> 136 and Enum rows 13 -> 14 (Task
1, 261027-184). A hand-edited value outside the two choices is shown verbatim,
while gsd-core's loader reads it as `single` (inferred I-4).

**Areas that needed no change:**

- **.planning formats.** `milestone complete --release-map` writes
  `milestones/<ver>-RELEASE-MAP.json`, which this build does not read.
  `grep -rn 'RELEASE-MAP' src` is empty.
- **Routing.** The conformance oracle test `tests/driver_router_conformance.rs`
  is green against the installed gsd-core 1.19.0 (`~/.claude/gsd-core/VERSION`
  reads `1.19.0`); see this task's SUMMARY for the full-suite result line.
- **The CLI.** `git diff --shortstat v1.18.0 v1.19.0 -- commands/gsd` is
  empty: no slash-command file changed. `src/` changed in many modules
  (release-map, milestone-lock, config); none is a surface this build shells
  out to. [inferred — audit]
- **Agents/worktrees.** `agents/` changed in 3 files (+7 / -6:
  `gsd-code-fixer`, `gsd-code-fixer.compact`, `gsd-verifier`), and the
  `per-plan-worktree-gate` and `worktree-pre-dispatch-commit` workflow steps
  changed. This build reads no agent file and does not drive worktree dispatch.
  [inferred — audit]
- **Codex.** `capabilities/codex/capability.json` is a 1-line change, the
  `version` bump.
- **Secret-bearing keys.** The re-diff found no change:
  `git diff v1.18.0 v1.19.0 -- src/secrets.cts
  gsd-core/references/gsd-run-resolver.md` is empty.

**Deferred.** A TUI badge surfacing a milestone's per-repository release map is
not built in this sync. It has an owner under "Next sync".

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
  the manager's own per-project `runtime` key (`src/config.rs` `RUNTIME_KEY`) is
  kept and written back unchanged so an old config loads, but since Phase 31 retired the legacy
  `drive` subcommand it no longer selects a driven agent (the launcher runs
  Claude only), and a project whose GSD config says `codex` is not driven
  differently. gsd-core's loader also fills it from `GSD_RUNTIME` / the install marker (#4717), so the
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

**The measured exemptions — 5 keys.** Every upstream key cell matching
the markers holds a number, a boolean or a list of file paths, not a secret. They
are exempt as the exact path or as a `<key>.` prefix:

- `review.max_prompt_tokens`
- `review.max_prompt_tokens_per_reviewer` (and its `.<slug>` entries)
- `statusline.show_context_tokens`
- `workflow.smart_zone_tokens`
- `hooks.secret_read_exempt` (gsd-core 1.21.0, #4432; repo-relative paths of
  committed files the secret-read guard lets agents read)

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

Every key `build_defaults_entries` pushes a row for, in display order. **139
rows**: **51** carry a `since` measured at the 1.14.0 re-sync (260916-vqw
measured 57; six were retired at 261012-jqb), **2** carry `v1.15.0` (added by
260926-gtk, measured against the v1.15.0 tag at 261012-jqb), **7** carry a
`since` measured when quick task 260926-jnf promoted them, **2** carry
`v1.16.0` (261012-jqb), **1** carries `v1.6.0`, measured at 261019-guw, **1**
carries `v1.19.0` (261027-184), **2** carry `v1.10.0`, measured when quick task
261103-f5i promoted them, **1** carries `v1.21.0` (261113-fi0), and the **72** without one predate this record
and were not retro-measured.

`DEFAULTS_OPTION_COUNT` in `src/ui/screens/detail.rs` pins this count at 139
(132 -> 139 at 260926-jnf, 139 -> 135 at 261012-jqb, 135 -> 136 at 261027-184,
136 -> 138 at 261103-f5i, 138 -> 139 at 261113-fi0),
`RESYNCED_KEYS` / `RESYNCED_KEYS_1_15_0` / `RESYNCED_KEYS_1_16_0` /
`RESYNCED_KEYS_1_19_0` / `PROMOTED_PLANNER_KEYS_1_20_0` /
`RESYNCED_KEYS_1_21_0` / `PROMOTED_TOP_LEVEL_KEYS` pin the seven key lists,
`REMOVED_KEYS_1_16_0` pins the six retired ones, and three coverage assertions
refuse a row with no `ConfigHelp`.

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
| `planner.stall_threshold_minutes` | v1.10.0 |
| `planner.stall_detect_interval_minutes` | v1.10.0 |
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
| `git.repo_tagging` | v1.19.0 |
| `git.allow_default_branch_commits` | v1.13.0 |
| `git.protected_branches` | v1.12.0 |

### Misc

| Key | since |
|---|---|
| `context_warnings` | |
| `hooks.context_warning_threshold` | v1.14.0 |
| `hooks.context_critical_threshold` | v1.14.0 |
| `hooks.workflow_guard` | v1.01.0 |
| `hooks.secret_read_exempt` | v1.21.0 |
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

`hooks.secret_read_exempt` is read-only (a list of repo-relative paths, edited in
the file), and its name trips the secret heuristic, so it is a measured exemption
(see [Secret-bearing keys](#secret-bearing-keys)).

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
| `review.models.<cli>`, `review.effort.<slug>`, `review.context_window_tokens.<slug>`, `review.reviewer_instances.<name>.*` | Templated by CLI / reviewer-lane slug / instance name; a typed field per unknown name is not expressible. The effort family is owned by each lane's capability manifest (`effortConfigKey`). `review.context_window_tokens` (gsd-core 1.20.0, #4559) is a per-reviewer-slug object (10 capability-registered slugs at v1.20.0) that derives the default cap of `review.max_prompt_tokens*`; it round-trips losslessly through `ReviewConfig`'s flatten `extra`. |
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
| `executor.stall_*`, `manager.flags.*` | Runtime tuning knobs, rarely edited by hand. `executor.stall_*` is not exposed in `/gsd-config --advanced`, so it stays a runtime knob. `planner` has been a fully typed block since 261103-f5i: it became typed at 260926-gtk with `planner.stall_detection_enabled`, and its two minute knobs became typed rows when gsd-core 1.20.0 (#3923) exposed them in `/gsd-config --advanced` (reversing 260926-gtk I-3). Before 260926-gtk, the whole `planner` object was a single top-level `planner` row. |
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
   (`REF=v1.21.0`, `PREV=v1.20.1`). If `GSD_CORE_SYNCED_COMMIT` already
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
   `effort.<slug>`, `context_window_tokens.<slug>` and
   `reviewer_instances.<name>.*` families have to stay pass-through.
   `review.max_prompt_tokens*` now REFUSES an oversized prompt instead of
   trimming it (gsd-core 1.20.0), and its default is 65% of
   `review.context_window_tokens.<slug>` or of the lane's declared window. A
   promotion should therefore show that derived default.
   `review.context_window_tokens.<slug>` itself stays templated, like
   `effort.<slug>`.
4. **Then `refactor.*` and `safety.*`**, which are small and fully scalar.
5. **Retro-measure `since` for the 73 rows that have none**, using step 5's
   command, if a reader ever needs the full history rather than the delta.
6. **Consider the label rename** described under "Known inconsistency" — as its
   own change, never folded into a sync.
7. **Release-map TUI badge (deferred at 261027-184).** Surface a milestone's
   per-repository release map (`milestones/<ver>-RELEASE-MAP.json`, written
   when `git.repo_tagging` is `per_repo`) as a badge. This build reads no
   release map today.
8. **Custom and bracket phase identity (deferred at 261103-f5i).** gsd-core
   1.20.0 names a plurality in phase identity: custom ids
   (`phase_naming: "custom"`, `.phase-id` marker files, directories such as
   `CCM-PRODUCT-BOUNDARY-slug`, ROADMAP headings such as
   `### Phase PRODUCT-BOUNDARY:`) and bracket ids (`### [GSD.02] 05:` headings,
   `GSD.02-05-slug` directories). This build's phase identity is the numeric
   `PhaseNum`, so a project that uses either has its phases NOT recognised: the
   readers abstain rather than guess. The work is scope-changing — it
   generalises phase identity beyond `PhaseNum` — and so is not a sync task.
   **Owner:** next-milestone planning, as a dedicated phase. A sync task must
   not attempt it.
