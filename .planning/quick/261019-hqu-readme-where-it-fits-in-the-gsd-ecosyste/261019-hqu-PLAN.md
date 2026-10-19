---
quick_id: 261019-hqu
mode: quick
phase: quick-261019-hqu
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - README.md
autonomous: true
requirements: [QUICK-261019-hqu]

estimate:
  tokens: 30000
  raw_tokens: 30000
  tasks: 2
  confidence: low

must_haves:
  truths:
    - "README.md has a `### Where it fits in the GSD ecosystem` subsection where the old claude-orchestration subsection was (under `## Why GSD Meta Manager?`, right before `## Features`), and the old subsection heading is gone."
    - "The subsection has a short intro, then a `| GSD piece | Relationship |` table with exactly 7 rows: gsd-core (`.planning/`), `/gsd-progress` and `/gsd-manager`, the claude-orchestration backend, Runtimes, gsd-tools, Capabilities, gsd-pi."
    - "Every claim in the subsection matches the Evidence table in this plan. Nothing in it promises future work (no Phase 32 capability management, no launcher, no push step)."
    - "The 'What it writes' list names every production write class found during planning: the GSD config saves, the queue file, the app's own config and log, new-project creation, and the experimental driver."
    - "The 'What it never touches' list claims only what the Evidence table proves."
    - "No link or anchor in the repo points at the removed heading. Historical `.planning/` records that quote it stay unchanged."
    - "`rtk proxy cargo test --no-fail-fast` fails only `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`."
  artifacts:
    - path: README.md
      provides: "The 'Where it fits in the GSD ecosystem' subsection"
      contains: "### Where it fits in the GSD ecosystem"
  key_links:
    - from: "README.md gsd-core table row"
      to: "README.md `### Compatibility` (anchor `#compatibility`, README.md:403 at HEAD)"
      via: "markdown link `[Compatibility](#compatibility)`"
      pattern: "\\]\\(#compatibility\\)"
---

<objective>
Replace the README subsection at README.md:52-61 (the claude-orchestration
comparison under `## Why GSD Meta Manager?`) with `### Where it fits in the GSD
ecosystem`. The new subsection has an intro, a 7-row "GSD piece | Relationship"
table and two short lists: what the app writes and what it never touches. Every
claim must be backed by the Evidence table below, which was verified against the
source during planning.

Purpose: tell readers how this app relates to each part of the GSD ecosystem,
and give them a truthful account of what it writes on their machine.
Output: an edited README.md (one commit), plus the quick-task SUMMARY.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@README.md

**The human is unavailable.** Decide from this plan. Record every judgement call
you make as `[inferred]` in the SUMMARY.

**Another session is running Phase 28 on master at the same time.** Touch only
README.md and this quick task's SUMMARY. Stage by path only (`git add README.md`),
never `git add -A` or `git add .`. Do not push.

## Evidence (verified at planning time, HEAD efcd622)

Line numbers are from production code, before each file's `#[cfg(test)] mod`.

| # | Claim | Proof |
|---|-------|-------|
| E1 | The `#compatibility` anchor exists | README.md:403 `### Compatibility`. README.md:17 and :117 already link it |
| E2 | The app is synced to gsd-core releases (currently 1.18.0) | README.md:15-19; docs/GSD-CORE-SYNC.md |
| E3 | Sessions are detected for both `claude` and `codex` by `pgrep -x` plus `/proc` (Linux only) | src/session_detector.rs:77-93, :130-134; README.md:390-401 |
| E4 | Only Claude sessions can be resumed. The app launches or resumes `claude` in a tmux window or a GUI terminal | src/ui/screens/detail.rs:4637-4640 (resume), :5154-5157 (launch); src/terminal_switch.rs:210-214 (`tmux new-window`), :281-310 (`list-panes`/`select-window`/`select-pane`); README.md:275 |
| E5 | The experimental driver runs `claude -p --output-format stream-json`, or `codex exec` when `runtime: codex` | src/executor/claude.rs:250-254; src/executor/codex.rs:55, :104; README.md:37-45 |
| E6 | The driver picks the next GSD command deterministically from project state, then runs it through that agent | src/driver/router.rs:1-12 |
| E7 | gsd-tools is invoked in exactly one way: `smart-entry --json`, cwd = the project, `GSD_RUNTIME=claude` set on that child only | src/state_reader/queue_md.rs:177-183, :189-198 (the only production `gsd-tools` spawn; all other `gsd-tools` hits are path resolution or tests) |
| E8 | `smart-entry` itself is read-only | ~/.claude/gsd-core/bin/lib/smart-entry.cjs (gsd-core 1.18.0): no `fs.write*/rename/mkdir/unlink/rm` calls; its only subprocess is `git`, under the comment "Read-only git signals" (:214-218). `[inferred]`: modules it imports were not audited |
| E9 | Config-tab saves of `.planning/config.json` take gsd-core's `.planning/.lock`. Saves of `$GSD_HOME/.gsd/defaults.json` take the installer's `gsd-install-migration.lock`. Both are read-modify-write with a same-dir O_EXCL temp file, fsync, the target's mode bits kept, then rename (quick 261019-guw) | src/state_reader/config_write.rs:1-47 (protocol), :105-143 (targets and lock paths), :494-545 (`atomic_write`), :561-582 (`update_gsd_config`); the only production caller is src/ui/screens/detail.rs:12587-12608 |
| E10 | Saving the global template when it is absent creates it, including the `.gsd/` dir | src/state_reader/config_write.rs:454 (`create_dir_all(target.dir())`), :569 (absent means fallback base); detail.rs:7963 ("saving will create it") |
| E11 | `$GSD_HOME` falls back to `$HOME` when unset or empty, mirroring gsd-core `resolveGsdHome` | src/state_reader/config_json.rs:720-760 |
| E12 | The Config tab's Capabilities rows: `capabilities.strict_known_registries` is read-only, `capabilities.auto_update` is an editable bool | src/ui/screens/detail.rs:11936-11954, :11108-11137 (read-only), :13121-13126 (auto_update toggle) |
| E13 | Unmodelled keys in every block, including `capabilities.*`, render read-only under a "Not modelled" category and survive a save | src/state_reader/config_json.rs:60-73 (`ExtraKeys`, flatten); src/ui/screens/detail.rs:12105-12112, :12181-12182, :12233-12250 (`ConfigValueKind::ReadOnly`) |
| E14 | The app never reads installed capability packs. The only `capabilities` handling in src/ is the config keys | `grep -rn -i 'capabilit' src` finds only config keys, plus Claude's stream-json `init.capabilities` in src/executor/*. Nothing references `~/.gsd/capabilities/` |
| E15 | Queue writes go to `.planning/meta-manager/QUEUE.md` (tmp then rename). An empty queue deletes it. A legacy `.planning/QUEUE.md` is removed | src/state_reader/queue_md.rs:225-291 |
| E16 | The app's own registry and preferences live at `~/.config/gsd-meta-manager/config.json` (tempfile + persist), in the platform config dir | src/config.rs:405-409, :472-487; README.md:357-368 |
| E17 | Daily rolling log at `<data-local-dir>/gsd-meta-manager/gsd-meta-manager.log.*` | src/main.rs:75-92 |
| E18 | New-project creation (`c`) creates the directory, runs `git init` and runs the user's own `pre_create`/`post_create` hooks via `sh -c`. It does NOT create `.planning/` | src/project_creator.rs:33-90 |
| E19 | Experimental driver run records go under `.planning/meta-manager/runs/` (`.gitignore`, `active`, `run.lock`, `<run-id>/run.json`, `journal.jsonl`, `inbox.jsonl`) | src/journal/mod.rs:93, :194-196, :380-395; src/journal/writer.rs:236-305, :438-451, :545-567; src/journal/inbox.rs:217-221; src/driver/lock.rs:201-268 |
| E20 | The hidden `driverctl` subcommand (Phase 27) writes `state.json`, `LOG.md` and the `DIRECTION.md` frontmatter under `.planning/meta-manager/drivers/<run-id>/`. It is NOT gated by `GSDMM_EXPERIMENTAL_FEATURES`: it is dispatched before anything else | src/driverctl/store.rs:1-3, :66-69, :480-493; src/driverctl/serve.rs:101, :184-235; src/driver/direction.rs:470-570; src/main.rs:103-107 |
| E21 | When a driver run starts, it writes a marker block into the driven repo's `.git/info/exclude` | src/driver/run.rs:197-199; src/envelope/hooks.rs:555-622 |
| E22 | Per-project envelope files (hook stubs, agent settings, gitconfig, gh dir, askpass, `pr-ledger.ndjson`) go under `<data-local-dir>/gsd-meta-manager/envelope/<alias>/` | src/envelope/mod.rs:209-215, :255-259; src/envelope/hooks.rs:113-149, :1645-1680; src/envelope/cred.rs:575-584, :666, :788-812; src/envelope/ledger.rs:47, :165-174, :448-502 |
| E23 | `driverctl serve` creates its socket under `$XDG_RUNTIME_DIR/gsdmm-driverctl/` | src/driverctl/serve.rs:29, :78, :242-270 |
| E24 | Session transcripts are never opened. Claude subagent `agent-<id>.jsonl` is only stat'ed for its mtime, and only `*.meta.json` is read (capped at 64 KiB). Codex session files are not read at all | src/agents/adapters/claude.rs:12-20, :54-57, :215-229, :377-384, :544-567; no `.codex/sessions` reference in src/ |
| E25 | Nothing in src/ writes under a gsd-core install. The app only reads `VERSION` files and runs the `smart-entry` call in E7 | Full production write-site enumeration (E9, E15-E23 are every site); README.md:427-430 |
| E26 | The app never writes `ROADMAP.md`, `STATE.md` or phase artifacts. Its only `.planning/` writes are E9, E15, E19 and E20. Those files change only when the user opens them in `$EDITOR` (`e`) | src/main.rs:810 (`$EDITOR` spawn); the write-site enumeration; src/state_reader/mod.rs:1065+ writes are `#[cfg(test)]` |
| E27 | Every git read goes through `git --no-optional-locks -C <root>` with `GIT_OPTIONAL_LOCKS=0`. The app's only writing git call is `git init` (E18). No production code runs `git commit` or `git push` | src/state_reader/git_ops.rs:200-222; production git subcommands are log, rev-parse, status, show, diff-tree, diff, config --get, remote, rev-list, worktree reads and check-ignore (src/journal/writer.rs:362-368). The `"commit"`/`"push"` literals in src/envelope/* are policy labels, not invocations |
| E28 | gsd-pi keeps project state in `.gsd/` with a local database, not `.planning/` | Upstream README github.com/open-gsd/gsd-pi (fetched 2026-10-19): "GSD stores project planning and runtime state in `.gsd/`" and "Tracks project state in a local database with markdown projections". npm `gsd-pi` 3.0.0 points at gsd-build/gsd-2, which moved to open-gsd/gsd-pi. Nothing in this repo references gsd-pi |
| E29 | No link or anchor points at the old heading | `grep -rn -i 'not-the-same-as'` and a grep for the heading text over README.md, docs/, src/ and the repo `*.md`: the only hits are README.md:52 itself and historical `.planning/` records (260722-emn, 260923-lra SUMMARY/PLAN, MILESTONES.md:9), which are records and stay unchanged. docs/*.md has no `README.md#` links |
| E30 | No test reads the root README.md | `grep -rn README tests/ src/`: tests/driver_rate_limit.rs and tests/driver_injection_corpus.rs read fixture READMEs only. src/ui/screens/detail.rs:16250-16270 reads docs/GSD-CORE-SYNC.md and only asserts that it mentions README's "GSD compatibility" note (README.md:15, untouched) |

## Corrections vs the task draft

1. **The writes list is incomplete.** Beyond the two GSD config files and
   driver-launched commands, the app also writes: `.planning/meta-manager/QUEUE.md`
   (E15), its own config and log (E16, E17), a new project's directory plus
   `git init` and the user's create hooks (E18), and the experimental driver's
   files: `.planning/meta-manager/runs/` and `drivers/` (E19, E20), a block in
   `.git/info/exclude` (E21), envelope files in the data dir (E22) and a runtime
   socket (E23).
2. **"GSD's own lock" covers two different locks.** `config.json` uses
   `.planning/.lock` (`withPlanningLock`). `defaults.json` uses the installer's
   `gsd-install-migration.lock`, because only gsd-core's installer writes that
   file (E9). Saving the global template can create it (E10).
3. **Capabilities are not display-only.** `capabilities.auto_update` is an
   editable toggle and `strict_known_registries` is read-only (E12). The
   lossless read-only round-trip applies to unmodelled keys in every block,
   `capabilities.*` included (E13). Installed packs are never read (E14).
4. **The driver is not entirely behind `GSDMM_EXPERIMENTAL_FEATURES`.** The
   hidden `driverctl` helper runs without the flag (E20). The README bullet must
   say "the experimental driver" and must not claim the flag gates every driver
   write.
5. **The draft's gsd-pi phrase "its own `.gsd/` database" is imprecise.**
   Upstream says state lives in a `.gsd/` directory plus a local database (E28).
   The README already uses `$GSD_HOME/.gsd/` for gsd-core's own user store, so
   the gsd-pi row must refer to the project's `.gsd/` to avoid confusing the two.
6. **Session transcripts:** a more exact claim is possible. Claude subagent
   transcripts are stat'ed for liveness but never opened, and Codex session
   files are never read (E24).
7. **More never-touches the evidence supports:** `ROADMAP.md`, `STATE.md` and
   phase artifacts, except through the user's `$EDITOR` (E26), and git: reads
   use `--no-optional-locks` and the app itself never commits or pushes (E27).
8. **No anchor fix is needed.** Nothing links to the old heading. The historical
   `.planning/` records that quote it must not be edited (E29).
9. **Runtimes row:** resume is Claude-only, and Codex detection is best-effort
   and covers interactive sessions only (E3, E4; README.md:397-401).

## Reference draft (start from this; adjust wording, keep every fact)

Use it as the basis. You may rephrase, but every fact must stay traceable to
an E-row, and you must not add facts that have no E-row. Prose lines wrap at 80
columns or fewer, dashes are `--`, paths are in backticks. Table rows are single
lines and may exceed 80 columns, as README.md:207-222 already do.

    ### Where it fits in the GSD ecosystem

    GSD Meta Manager sits beside GSD rather than inside it: it reads what GSD
    leaves on disk and writes only the few files listed below.

    | GSD piece | Relationship |
    |-----------|--------------|
    | gsd-core (`.planning/`) | What the app manages: each project's `.planning/` state, read straight from disk. Regularly synced to gsd-core releases -- see [Compatibility](#compatibility) |
    | `/gsd-progress`, `/gsd-manager` | One project, from inside a Claude session. This app covers every registered project from outside any session |
    | `claude-orchestration` backend | Runs one project's phase waves in parallel inside a single Claude session, leaving the same commits and `SUMMARY.md` files as normal execution -- complementary, and read like any other run |
    | Runtimes | Claude Code and Codex: live sessions are detected (Linux), and the experimental driver runs either (`claude -p` or `codex exec`). Resume is Claude-only |
    | gsd-tools | Called read-only, as `gsd-tools smart-entry --json`, for next-command suggestions |
    | Capabilities | Their `capabilities.*` config keys are on the Config tab; keys this build does not model round-trip losslessly as read-only rows. Installed packs are not read |
    | gsd-pi | Not supported: it keeps project state in its own `.gsd/` directory and database, not `.planning/` |

    What it writes:

    - `.planning/config.json` and `$GSD_HOME/.gsd/defaults.json`, on a Config-tab
      save -- under GSD's own lock, replaced atomically.
    - `.planning/meta-manager/QUEUE.md` (Queue tab), its own config and log, and
      a new project's directory with `git init` (`c`).
    - The experimental driver: run records under `.planning/meta-manager/`, a
      block in `.git/info/exclude`, files in its data directory -- plus whatever
      the GSD commands it launches write.

    What it never touches:

    - Session transcripts (a subagent's is checked only for its timestamp, never
      opened), the GSD install, or installed capabilities.
    - `ROADMAP.md`, `STATE.md` or phase artifacts, except through your `$EDITOR`
      (`e`). Git is read with `--no-optional-locks`; the app never commits or
      pushes.

(The draft is indented four spaces here only to keep it out of the plan's XML.
Remove that indent in README.md.)
</context>

<tasks>

<task type="tracer">
  <name>Task 1: Replace the README.md:52-61 subsection with "Where it fits in the GSD ecosystem"</name>
  <files>README.md</files>
  <read_first>README.md (lines 25-63, to match tone; lines 357-368 and 388-430, so the new facts agree with Configuration, Platform support and Compatibility); this plan's Evidence table, Corrections list and Reference draft</read_first>
  <action>
  In README.md, replace the whole `###` subsection that starts at line 52 (the
  one comparing the app with GSD 1.8.0's experimental claude-orchestration
  backend) and runs through line 61, the paragraph ending "...across your whole
  portfolio.". Put the new `### Where it fits in the GSD ecosystem` subsection
  in its place, starting from this plan's Reference draft with the four-space
  indent removed. Leave lines 1-51 and everything from `## Features` on
  byte-identical, and keep exactly one blank line before `## Features`.

  Content rules:
  (a) The claude-orchestration row condenses the old paragraph: one Claude
  session on one project, waves run in parallel, the same commits and
  `SUMMARY.md` output, complementary. Drop the "GSD 1.8.0" and "Workflow tool"
  detail, which the Compatibility section already covers at README.md:408-410.
  (b) Every fact must trace to an Evidence row (E1-E30). Write only today's
  state: nothing from future roadmap phases, such as upcoming capability
  management or the upcoming driver launch, isolation or push work.
  (c) The driver bullet must not say the experimental flag gates it (Correction
  4). It must say the commands the driver launches write whatever GSD writes
  (E6).
  (d) The gsd-pi row stays neutral, one sentence, with no judgement of gsd-pi.
  Use "its own `.gsd/` directory and database" (Correction 5, E28).
  (e) In the never-touches list, do not claim the app never reads Claude
  credentials or `~/.claude`. Only transcripts, the GSD install, installed
  capabilities, the GSD planning documents (with the `$EDITOR` caveat) and
  git commits/pushes are proven.
  (f) Length: the Reference draft runs 32 lines from the heading through the
  last bullet, and that is the ceiling. Shorter is fine if no fact is lost.
  The task draft asked for about 20-25 lines, but the verified writes list is
  longer than the draft assumed. Record this as `[inferred]` in the SUMMARY.
  Prose lines are at most 80 columns. Table rows are exempt.

  Anchor sweep: re-run the anchor grep from the verify block below over README.md,
  docs/ and src/. It must find nothing. Do NOT edit any `.planning/` file that
  quotes the old heading, because those are historical records (E29).

  Commit README.md alone: `git add README.md`, then `git commit` with subject
  `docs(quick-261019-hqu): README "Where it fits in the GSD ecosystem"`. End the
  body with the attribution trailer lines from the session's system reminder.
  Do not push.
  </action>
  <verify>
    <!-- Use `rtk proxy grep` so the rtk hook does not reformat grep output. Baselines were measured at HEAD efcd622. -->
    <automated>rtk proxy grep -c '^### Where it fits in the GSD ecosystem$' README.md   # baseline 0 at HEAD; must print 1</automated>
    <automated>rtk proxy grep -c '^### Not the same as' README.md   # baseline 1 at HEAD; must print 0 (exit 1 on 0 matches is the pass case)</automated>
    <automated>rtk proxy grep -c '^| GSD piece | Relationship |$' README.md   # baseline 0; must print 1</automated>
    <automated>awk '/^### Where it fits in the GSD ecosystem$/{f=1} /^## Features$/{f=0} f && /^\| /' README.md | wc -l   # must print 9 (header + separator + 7 rows)</automated>
    <automated>rtk proxy grep -cF '](#compatibility)' README.md   # baseline 2 at HEAD; must print 3</automated>
    <automated>rtk proxy grep -cF 'smart-entry --json' README.md   # baseline 0; must print 1</automated>
    <automated>rtk proxy grep -cF '.planning/meta-manager/QUEUE.md' README.md   # baseline 0; must print >= 1</automated>
    <automated>awk '/^### Where it fits in the GSD ecosystem$/{f=1} /^## Features$/{f=0} f' README.md | wc -l   # section incl. trailing blank: must be <= 33</automated>
    <automated>awk '/^### Where it fits in the GSD ecosystem$/{f=1} /^## Features$/{f=0} f && length > 80 && !/^\|/' README.md | wc -l   # must print 0</automated>
    <automated>awk '/^### Where it fits in the GSD ecosystem$/{f=1} /^## Features$/{f=0} f && tolower($0) ~ /phase 3[0-9]|launcher|sandbox|bwrap/' README.md | wc -l   # must print 0</automated>
    <automated>rtk proxy grep -rn -i 'not-the-same-as' README.md docs/ src/   # must print nothing</automated>
    <automated>git show --stat --format='%h %s' $(git log -1 --format=%h --grep='quick-261019-hqu')   # README.md only, docs(quick-261019-hqu) subject; robust to concurrent Phase 28 commits</automated>
  </verify>
  <done>README.md has the new subsection in place of the old one, with a 7-row table, a writes list and a never-touches list. Every fact traces to an E-row. Gates: 1 new heading, 0 old heading, 9 table lines, 3 `#compatibility` links, at most 33 section lines, 0 overlong prose lines, 0 forward-looking terms, no anchors to the old heading. Committed alone as `docs(quick-261019-hqu): ...` and not pushed.</done>
</task>

<task type="auto">
  <name>Task 2: Run the full test suite and write the SUMMARY with corrections, inferred decisions and Phase-28-affected claims</name>
  <files>.planning/quick/261019-hqu-readme-where-it-fits-in-the-gsd-ecosyste/261019-hqu-SUMMARY.md</files>
  <read_first>this plan's Evidence, Corrections and "Phase-28-affected claims" sections</read_first>
  <action>
  Run the full suite exactly as given in verify, without piping it through grep
  or awk (rtk filters downstream of `proxy`, so a pipe would fake results). No
  test reads README.md (E30), so the suite should be unaffected. The one
  expected local failure is
  `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
  (a machine-local git-version witness). Any other failure is not caused by a
  README edit. Most likely it is the concurrent Phase 28 session's work in the
  shared tree. Record it in the SUMMARY with the test name and do not try to fix
  it.

  Write 261019-hqu-SUMMARY.md in the quick-task directory using the summary
  template. It MUST contain these sections, each as a `##` heading with exactly
  the names `## Corrections vs draft`, `## Inferred decisions` and
  `## Phase-28-affected claims`:
  (1) "Corrections vs draft": copy this plan's Corrections list (items 1-9),
  saying for each how the README handled it.
  (2) "Inferred decisions": every `[inferred]` call you made, plus these
  carried from planning. I-hqu-1: the section exceeds the ~20-25 line target
  so the writes list can be complete. I-hqu-2: the experimental-driver writes
  are summarised in one bullet ("files in its data directory" stands for the
  envelope files of E22, the socket of E23 is omitted, and the `drivers/` dir
  of E20 is folded into ".planning/meta-manager/"). I-hqu-3: "gsd-tools is
  read-only" rests on E8, and smart-entry's imported modules were not audited.
  I-hqu-4: the gsd-pi wording rests on its upstream README fetched 2026-10-19
  (E28), with no in-repo evidence. I-hqu-5: historical `.planning/` records
  that quote the old heading were left unchanged. I-hqu-6: the SUMMARY is left
  uncommitted for the orchestrator's docs commit. I-hqu-7: the never-touches
  claim for ROADMAP/STATE carries the `$EDITOR` caveat.
  (3) "Phase-28-affected claims": copy the list from this plan's section below.
  (4) The test result line (passed/failed counts and failing test names), and
  the commit sha of the README commit.

  Do not commit the SUMMARY or any other `.planning/` file. Do not touch
  STATE.md.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast   # expected: one failure, envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against</automated>
    <automated>rtk proxy grep -c -e '^## Corrections vs draft' -e '^## Inferred decisions' -e '^## Phase-28-affected claims' .planning/quick/261019-hqu-readme-where-it-fits-in-the-gsd-ecosyste/261019-hqu-SUMMARY.md   # must print 3</automated>
  </verify>
  <done>The test suite was run with `--no-fail-fast`, and its only failure is the known git-version witness (any other failure is recorded by name). 261019-hqu-SUMMARY.md exists with the Corrections vs draft, Inferred decisions and Phase-28-affected claims sections, the test result and the README commit sha. Nothing besides README.md was committed.</done>
</task>

</tasks>

## Phase-28-affected claims (copy into the SUMMARY)

Source: .planning/phases/28-sandboxed-driver-launcher/28-CONTEXT.md. Phase 28
has no plans yet, so these come from its decisions.

1. **"What it writes" / driver bullet.** Phase 28 adds a stored OAuth token at
   `~/.config/gsd-meta-manager/driver-oauth-token` (D-21). That is in the
   config dir, which the bullet's "data directory" does not cover. It also adds
   a per-run `CLAUDE_CONFIG_DIR` and a launch log under the data dir (D-18,
   D-04), which the bullet already covers, and `launcher.lock` plus state under
   `.planning/meta-manager/drivers/<run-id>/` (D-02, D-06), also covered. The
   bullet needs the token added when Phase 28 ships.
2. **"The app never commits or pushes."** Phase 28 adds a host-side `git push`
   of `refs/heads/gsd-auto/<alias>/<run-id>` at run end and through
   `driver push` (D-24, D-25), plus an optional `gh` PR (D-26, off by default).
   This claim must change when Phase 28 ships.
3. **Runtimes row ("the experimental driver runs either").** The Phase 28
   launcher is headless `claude -p` only (D-04). Codex driving stays on the
   legacy Phase 17 `drive` path, which Phase 31 deletes. The row may need to
   change at Phase 28 or 31.
4. **Never-touches: transcripts, GSD install.** These still hold under Phase 28.
   The host's `gsd-core`, agents, commands, skills and hooks are symlinked
   read-only into the per-run config (D-18), and the host
   `.credentials.json` access token is only read, as the fallback (D-22). Do
   not add a "never reads Claude credentials" claim.
5. **CLI surface.** Phase 28 adds a `driver launch|stop|push|auth` subcommand
   group (D-01). The README Usage block and any "experimental" qualifier may
   need revisiting. This section is not affected unless item 2 or 3 applies.

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| README to reader | Users decide what to trust the tool with based on the "writes" and "never touches" claims |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-hqu-01 | Repudiation / misleading assurance | README "never touches" list | medium | mitigate | Each claim is limited to an Evidence row (E24-E27). Rule (e) in Task 1 forbids an unproven "never reads credentials" claim. Phase-28-affected claims are listed so the next README pass revisits them |
| T-hqu-02 | Information disclosure (omission) | README "What it writes" list | medium | mitigate | The write-site enumeration (E9, E15-E23) feeds the list. Correction 1 names every class, and a grep gate pins the queue path |
| T-hqu-SC | Tampering | npm/pip/cargo installs | low | accept | No packages are installed. This is a docs-only change |
</threat_model>

<verification>
- Every gate in Task 1's verify block passes. The section is at most 33 lines,
  with no prose line over 80 columns.
- `rtk proxy cargo test --no-fail-fast` fails only the git-version witness.
- `git log -1 --stat` shows only README.md under a `docs(quick-261019-hqu): ...`
  subject, and nothing was pushed.
</verification>

<success_criteria>
- README.md:52 onward reads `### Where it fits in the GSD ecosystem`, with the
  7-row table and the two lists, and every claim traces to the Evidence table.
- There are no dangling anchors, and no historical records were edited.
- The SUMMARY records the corrections vs draft, the inferred decisions and the
  Phase-28-affected claims.
</success_criteria>

<output>
Create `.planning/quick/261019-hqu-readme-where-it-fits-in-the-gsd-ecosyste/261019-hqu-SUMMARY.md` when done (leave it uncommitted for the orchestrator).
</output>
