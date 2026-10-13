---
spike: 001
idea: queue-driven-master-agent
name: queue-driven-master-agent
type: standard
validates: "Given a human-owned direction file with work items, when a headless Opus-class master session dispatches one worker subagent per step and decides against each item's done-when and budget, then items complete, steering works by editing the file, and an interrupted run is reconciled on restart, with no tmux or stdin injection"
verdict: PARTIAL
related: []
tags: [driver, autonomy, v2.0, headless, bwrap, systemd-run, queue, reconciliation]
---

# Spike 001: Queue-driven master agent

## What This Validates

Given a human-owned `DIRECTION.md` (standing rules plus items, each with intent, done-when and budget),
when a `claude -p` master reads it, dispatches one worker subagent per step, appends the outcome to a
driver-owned `LOG.md`, and checks done-when itself (it does not follow the worker's suggestion),
then:

- the items complete;
- editing the file steers the run between dispatches;
- a killed run is reconciled on restart;
- it all works inside `bwrap` plus a `systemd-run --user --scope`.

The question behind it: can this replace the planned v2.0 autonomy machinery, namely the Phase 19
bash-parsing guard, the Phase 21 LLM goal layer, Phase 22 containers, and tmux/stdin injection?

Artifacts:

- `MASTER-PROMPT.md`: the driver prompt. It does not depend on a mode.
- `templates/DIRECTION.md` and `templates/LOG.md`: the chosen file format.
- `experiment/`: `setup.sh` builds the scratch project, `run.sh` launches the driver on the host or under bwrap, and `results/` holds the logs and costs.

## Research

Sources: `.planning/ROADMAP.md` (Phases 15-25), `.planning/meta-manager/CONCEPT.md`,
`.planning/milestones/v1.2-phases/13-*/QUEUE-EXECUTION-DESIGN.md`,
`src/state_reader/queue_md.rs`, `src/driver/router.rs`, `src/driver/dry_run.rs`, `src/cli.rs`,
`src/journal/{mod,writer}.rs`, `claude --help` (2.1.296), and the env of a live session.

| Approach | Mechanism | Pros | Cons | Status |
|---|---|---|---|---|
| Planned v2.0 | Rust supervisor, deterministic router (P20), LLM goal seam (P21), argv-classifying PreToolUse guard (P19), containers (P22), stdin injection (P18) | Mechanical guarantees and provable properties | P19 is on gap round 13 and P21 is on round 13 at a hard 4/5 ceiling. Each round finds a new carrier class. Large surface | In progress, stalled |
| **Queue-driven master** | Opus master session follows a prompt contract, with files as memory and steering. Workers are subagents, isolated by bwrap and a cgroup scope. gsd-mm pushes | Little new code. Uses GSD's own workflows unchanged. Steering cannot fail to deliver. Survives crashes by design | Stop rules and "don't follow suggested-next" are prompt-enforced (soft). Prompt injection is contained by the sandbox, not prevented. bwrap is Linux-only | **Chosen to spike** |
| Phase 13 Strategy A | One `claude -p` per QUEUE.md command | Simple | No judgement between steps. Every step is a cold start. No done-when | Superseded by this |

## Deliverable 1: file naming and layout decision

**Decision: key the files by a run id that gsd-mm picks, not by the Claude session id. Use two
files, each with a single owner, under `.planning/meta-manager/drivers/<run-id>/` in the main
checkout. The directory is gitignored. Driver and human always use the absolute main-checkout path.**

- **The Claude session id cannot be the key (verified).** `CLAUDE_CODE_SESSION_ID` is exported to
  the Bash tool. Headless runs logged it themselves: `session=e66f20dc…`. But every launch gets a
  new one. The five driver launches in this spike (A, B1, B2, B3, C) produced five different ids
  for two logical runs. Fresh-per-item mode is one launch per item by design, so it would split
  one run across N files. `--resume` keeps the id and `--fork-session` changes it (per `--help`).
  `--session-id <uuid>` can preset the id, but only for one launch. Subagents also see the
  *parent's* id: this spike's own subagent env showed the coordinator's session id. The session id
  is therefore recorded per launch, in the `RUN start` event, as metadata only.
  - The run id is `<yyyymmdd>-<slug>`. gsd-mm mints it, or the human does when launching by hand.
    It is stable across restarts, fresh-per-item, resume and compaction. Several concurrent
    drivers are simply several run dirs. `[inferred]`
- **Location:** `.planning/meta-manager/drivers/<run-id>/`, gitignored by a self-ignoring
  `drivers/.gitignore`. This matches the Phase 16 precedent, `RUNS_SUBDIR =
  "meta-manager/runs"` with its own `.gitignore` (`src/journal/mod.rs:93`,
  `src/journal/writer.rs:235`). The run dir is not in any branch, so worktrees never get a copy,
  merges never conflict on it, and the code merge-back after an interruption is plain git: worker
  branches and worktrees, which reconciliation inspects. The repo root was rejected because it
  clutters the tree, risks `git add -A`, and puts a copy in every worktree. Committing was
  rejected because the driver's appends would dirty the tree that workers commit from. The driver
  is given `RUN_DIR` as an absolute path and is told never to touch a worktree copy. `[inferred]`:
  archiving a finished run into a commit is left to gsd-mm as an explicit action.
- **Concurrency: split by owner.** The human, or the gsd-mm Queue tab acting for the human, is
  the only writer of `DIRECTION.md`, and the driver only reads it. The driver is the only writer
  of `LOG.md`, which is append-only via a heredoc `>>`, and the human only reads it. With one
  writer per file there are no lost updates. An editor's write-and-rename can at worst give the
  driver the previous version for one dispatch, and the driver re-reads before every dispatch.
  Item status is derived from the log: an item's state is its last `status` event. Answering a
  parked item is a DIRECTION edit (`answer:` plus `hold: false`). A single shared file was
  rejected: the driver's Edit/Write tools rewrite whole files, so they would silently clobber a
  human edit made during a dispatch, and recovering from an interruption would mean diffing two
  writers' changes.
- **QUEUE.md: coexist now, migrate later.** `parse_queue_md` (`src/state_reader/queue_md.rs:192`)
  turns every non-`#`, non-blank line into a command. A rich item format inside QUEUE.md would be
  misread as N bogus commands, so the format cannot be reused in place. Keep QUEUE.md as the
  lightweight per-project command list. Add a gsd-mm action "promote queue to driver run" that
  emits one item per `QueuedAction` (intent = the command, done-when = "command completed and
  STATE.md advanced"). Retire QUEUE.md once the Driver tab edits DIRECTION.md directly. `[inferred]`
- **Template:** `templates/DIRECTION.md` holds the direction and standing rules, and each item has
  id, intent, done-when, budget, start-with, after, hold and answer. `templates/LOG.md` holds the
  event grammar. Status, outcome and needs-human are log events.

## Deliverable 2: master prompt

`MASTER-PROMPT.md` covers the required cases as follows:

- **Interactive in tmux, headless `claude -p`, no tmux, and failed injection** all behave the
  same, because the prompt never depends on a terminal. AskUserQuestion is forbidden. The human
  channel is only `LOG.md`: on needing a human, the driver appends `needs-human` plus
  `state=parked` and moves to the next item or ends the run. Steering is `DIRECTION.md`, re-read
  before every dispatch and after every report, with `control: run|pause|stop`. A typed message
  in an interactive session is optional and never required, so a failed tmux injection loses
  nothing.
- **Startup reconciliation.** Every item still marked `running` gets evidence gathered (commits
  since `base` on HEAD and on every worktree branch, porcelain status, STATE.md), then exactly one
  of four actions:
  - done;
  - resume: re-dispatch with "continue from";
  - reset: nothing was produced, and the lost dispatch still counts;
  - park: dirty worktree or anything ambiguous. The driver never cleans this up itself.
- **Stop rules:**
  - per-item budget (dispatches, minutes, gap-rounds);
  - at most 2 gap-closure rounds per phase;
  - divergence: two consecutive rounds with new findings means park;
  - no progress: two dispatches with no new commit or evidence;
  - run budget.

  `SUGGESTED_NEXT` is explicitly advisory. Done-when is always re-checked by the driver.
- **Report schema:** 11 fields, at most 15 lines (STATUS, ITEM, STEP, COMMITS, WHERE, CHANGED,
  CHECKS, DONE_WHEN, FINDINGS, NEEDS_HUMAN, SUGGESTED_NEXT).
- **`MODE=fresh-per-item`:** handles one item and exits. The launcher loops.
- **Nested dispatch:** the user's rule about `isolation: "worktree"` is in the worker brief verbatim.

## Deliverable 3: `next --json` size

The equivalent does not exist as a JSON CLI. `drive <alias> --target-phase N --dry-run` already
calls `router::decide` (`src/driver/dry_run.rs:285`), but it needs an opted-in alias and prints
text. `decide(&ProjectState, &str) -> Decision` is pure (`src/driver/router.rs:901`), and
`parse_project_state` is public.

A `Next { project: alias|path, --phase <N> (default state.active_phase_number()), --json }`
subcommand would need:

- one `Commands` arm;
- about 40 lines of handler, mapping `Decision::{Run,Park,GoalMet,NoRule}` to JSON via
  `serde_json::json!` and the existing `RouterReason::as_str()`;
- 2 or 3 integration tests on fixture `.planning/` dirs.

**Size: S, about 100-150 LOC. One `/gsd-quick`.** It is gated like the other driver surfaces
(`GSDMM_EXPERIMENTAL_FEATURES`). The router's alphabet is only discuss, plan and execute, and it
parks at every verify gate, so it is a narrow hint. GSD's own `gsd-tools smart-entry --json`
already works (verified on this repo: `{"situation":…,"recommended":"progress",…}`) and is already
consumed by `queue_md.rs:175`. The master can use either today. Neither binds it, by rule 3.

## How to Run

```bash
S=$(mktemp -d)
experiment/setup.sh $S/proj                                    # scratch repo + DIRECTION.md + LOG.md
experiment/run.sh   $S/proj loop none                          # headless, host, tool allowlist
SCOPE_UNIT=x experiment/run.sh $S/proj fresh-per-item bwrap    # headless, bwrap + cgroup scope
systemctl --user kill --signal=SIGKILL x.scope                 # simulate a crash mid-item
```

## What to Expect

Two commits, one per item. `LOG.md` ends `RUN end reason=all-done` (loop mode), `fresh-exit`
(fresh-per-item mode) or `pause`. After a kill, the next launch logs `recovered action=…`.

## Investigation Trail

The master ran on Sonnet and the workers on Haiku to keep the spike cheap. `[inferred]`: Opus is
the intended master model, and the protocol is not model-specific. All runs used headless
`claude -p`, `--max-budget-usd 2`, and the parent `CLAUDE_CODE_*` env vars unset.

1. **Run A: host, loop, tool allowlist.** Both items were done in about 49s, for $0.229 (list
   price) across 2 dispatches. The driver verified done-when itself (grep, `git status`, unittest).
   Surprise: `LOG.md` only *pointed* to the grammar in `templates/LOG.md`, so the driver invented
   a different line format. Fix: `setup.sh` now inlines the grammar comment into LOG.md. Runs B
   and C then followed the pipe grammar exactly.
2. **Run B1: bwrap + `systemd-run --user --scope`, `bypassPermissions`.** Auth worked inside the
   sandbox, with `~/.claude` and `~/.claude.json` bound read-write. It was SIGKILLed via
   `systemctl --user kill` about 2s after `I-01 dispatched`. The scope went inactive, nothing was
   committed, and a stale `driver.lock` was left with `217 e66f20dc…`.
3. **Run B2: restart, fresh-per-item, bwrap.** Logged `I-01 recovered action=reset reason=stale
   lock pid 217 dead; no commits, clean tree…; lost dispatch still counts`, re-dispatched
   (`dispatch=2/2`), verified, `fresh-exit`. $0.187.
4. **Run B3: next fresh launch.** It found nothing to reconcile, did I-02, and did `fresh-exit`.
   $0.187. It reported a real side-finding as a finding, not an action: an untracked
   `__pycache__/`.
5. **Run C: steering by file.** While I-01's worker was in flight, `sed -i` flipped
   `control: run` to `control: pause` in DIRECTION.md. The driver finished I-01, re-read the file,
   logged `RUN end reason=pause`, and never dispatched I-02. $0.184.
6. **Surprise: PID-namespace reuse.** Runs B1 and C both ran as **pid 217** inside
   `bwrap --unshare-pid`. A pid-based stale-lock check across sandboxed launches is therefore
   unsound: it worked in B2 only because 217 happened to be free. The prompt now compares the
   pidns inode and prefers a launcher-held flock (`DRIVER_LOCK_HELD=1`). gsd-mm already has the
   `flock(2)` single-run lock from Phase 17 CTRL-05.
7. **Report fidelity:** the masters condensed the 11-line worker report into one detail line every
   time. The prompt now demands verbatim copying. gsd-mm should parse only the event header's
   key=values and treat the detail lines as display text.

Total spike spend was about $0.85 at list price (A, B2, B3, C, plus the killed B1, not metered).

**Untested:**

- interactive mode (tmux or not);
- real GSD chains (plan, execute, verify) with nested `gsd-executor` dispatch and worktree
  isolation under a worker subagent. Indirect evidence that nesting is possible: this spike ran as
  a subagent that itself had the Agent and Skill tools;
- the divergence, gap-round and no-progress stop rules, which never fired;
- the refusal to follow `SUGGESTED_NEXT` under pressure;
- parallel items;
- rate-limit exhaustion mid-run;
- macOS.

## Results

**Verdict: PARTIAL.** The mechanism is validated:

- headless dispatch, decision on done-when, and self-verification;
- steering by editing the file between dispatches;
- reconciliation after a crash;
- fresh-per-item mode with all memory in the files;
- sandboxing with bwrap and a cgroup scope.

All of this is with no tmux and no stdin injection, and the new code is a launcher plus two file
formats. What is **not** validated is what would make it a replacement rather than a demo: the
behaviour on real multi-step GSD phases, and whether prompt-level stop rules hold when a phase
keeps finding gaps. That second case is exactly the Phase 19 and 21 failure mode.

**What it would supersede, if a GSD-scale follow-up spike confirms it:**

| Phase | Fate |
|---|---|
| 18 Driver tab / live watch / durable injection (shipped) | Keep the Driver tab and live watch as the observer of LOG.md and the stream. Durable stdin injection and the queued, delivered, acted-on states are superseded by DIRECTION.md edits |
| 19 GITSAFE | The argv/bash-classifying PreToolUse guard (SAFE-02, rounds 9-13) is superseded: the sandbox has no push credentials (no `~/.ssh`, no agent socket, no gh token), so a push fails mechanically. Secret scan (SAFE-03), the PR cap (SAFE-06) and protected-branch refusal (SAFE-01) move to gsd-mm's own push step outside the sandbox. Scoped credentials (SAFE-05) become moot |
| 20 Router & run bounds | No longer the decider. It stays as the advisory `next --json` hint. Step and wall-clock caps become the launcher's hard caps (`--max-budget-usd`, `timeout`, scope `RuntimeMaxSec`). Rate-limit parking stays in the launcher (it reads the result subtype) |
| 21 LLM goal layer & injection hardening | The goal layer is superseded: the master is the goal layer, and DIRECTION.md is the approved plan. Injection hardening is not solved. It is reframed as blast-radius containment (sandbox, no creds, gsd-mm push gate). Criterion 4 was already agent-unclosable |
| 22 Container target | Superseded on Linux by bwrap plus `systemd-run --user --scope` (verified working, including auth). macOS and Windows would still need something else |
| 23 Gate policy & auto-validation | Mostly superseded: gate choice becomes per-item done-when plus standing rules ("auto-validate when tests pass") |

**Open risks:**

1. **Soft stop rules.** Budgets, gap-round caps and divergence detection live in a prompt, and none
   fired in this spike. A gap loop like Phase 21's could keep "making progress" by its own
   account. Mitigation: hard caps in the launcher (cost, wall clock, max launches) that do not rely
   on the model, plus a follow-up spike seeded with a deliberately unclosable item.
2. **Sandbox fidelity and portability.** The bwrap profile used here binds `~/.claude`
   read-write, so a driven agent could rewrite the user's global hooks and settings. That is a
   real escape from the "gsd-mm owns the boundary" model. It needs a dedicated `CLAUDE_CONFIG_DIR`
   (the Phase 22 two-file credential trap applies). bwrap and systemd are Linux-only, while
   CLAUDE.md's constraint is "any GSD user". The pid-namespace lock bug shows that sandbox
   semantics leak into the protocol.
3. **Cost and judgement at scale.** About $0.19 per trivial item with a Sonnet master. An Opus
   master that re-reads files and verifies every step of a real phase is unmeasured, and the
   5h/7d quota is shared across concurrent drivers (OQ10 from Phase 20).

Next: a follow-up spike that runs one real GSD phase (discuss through verify) on a scratch project
through this driver, including a planted unclosable gap, to settle risk 1 before any roadmap
change.
