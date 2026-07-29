# OQ1 Spike — Does `claude -p` reliably run a multi-step GSD skill headlessly?

**Verdict:** PASS

**Run date:** 2026-07-29
**Claude CLI:** 2.1.220
**Spike owner:** Phase 15 Plan 01, Task 1 (D-27, D-28)

---

## Scope

Research (`15-RESEARCH.md` § *Spike Outcomes*) already CONFIRMED at **single-tool-call**
scale that `--setting-sources project` suppresses the `PreToolUse` hook hang: exit 0 in 8s
with the flag versus exit 124 at a 90s cap without it. This spike is the **multi-step
generalisation** the whole v2.0 milestone rests on — a real GSD skill that itself spawns
subagent waves, run headlessly to completion.

Two specific questions the bounded research probe could not speak to:

- **(a)** Do subagent waves re-introduce the hook hang — i.e. does the `--setting-sources
  project` mitigation hold for work performed inside subagents, not just the top-level turn?
- **(b)** Does a long skill run cross the silent 10-minute `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS`
  background-subagent ceiling (D-14)?

Both are answered below.

---

## Safety fences (D-28)

**Scratch directory:** `/tmp/oq1-scratch-T6XQLOYo`

This directory is **neither this repository nor any project registered in the user's
gsd-meta-manager config** — it was created fresh by `mktemp -d -t oq1-scratch-XXXXXXXX`
under the system temp root, and the registered set
(`~/.config/gsd-meta-manager/config.json`) contains eleven paths, all under
`/home/blk/projects/`, none under `/tmp`.

| Fence | How it was honoured |
|-------|---------------------|
| Disposable scratch target | `mktemp -d`, `git init`, one seed commit, a minimal `.planning/` tree with a single phase and a single plan of two trivial `type="auto"` tasks |
| Never this repository | cwd for every invocation was the scratch dir; the repo was never the target |
| Never a registered project | verified against `~/.config/gsd-meta-manager/config.json` before the first run |
| No permission bypass | `--permission-mode dontAsk` only. `--dangerously-skip-permissions` and `--permission-mode bypassPermissions` were **never** used |
| External wall-clock bound | every invocation ran under `timeout -s TERM` (60s pre-flight, **900s** main run, 300s A3 probe) |
| Environment hygiene (T-15-04) | every inherited `CLAUDE*` variable was scrubbed from the child env via a generated `env -u …` prefix; only `CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=600000` was set deliberately (D-14) |
| Never `--bare` | not used — it breaks subscription auth (D-08) |

---

## Step 2 — Pre-flight of the skill surface

**This step was necessary, and it produced two findings that change what later phases must
do.** A spike that silently ran against a Claude with no GSD skills would prove nothing, so
the surface was probed before the main run.

### Finding P1 — `--setting-sources project` **does** strip user-level skills and agents

First pre-flight (60s, text-only, exit 0, wall 2.5s), inspecting the first `system/init`:

```
n_skills   = 15        (built-ins only)
gsd_skills = []
gsd_slash  = []
agents     = ["claude","Explore","general-purpose","Plan","statusline-setup"]
```

Zero `gsd*` skills and zero `gsd*` slash commands. `15-RESEARCH.md` states
*"`--setting-sources project` does not strip skills or agents"* — that observation was
about the **built-in** skill set, which is indeed preserved. **User-level skills and agents
installed under `~/.claude/skills/` and `~/.claude/agents/` are not.** The mitigation the
entire transport depends on therefore also removes the GSD install from a default
(user-scope) GSD installation.

Remediation applied for the spike, exactly as Task 1 prescribes: the user-level GSD install
was symlinked into the scratch project's own `.claude/` — **72** `gsd-*` skill directories
into `.claude/skills/`, **34** `gsd-*.md` agent definitions into `.claude/agents/`
(including `gsd-executor`, which `/gsd-execute-phase` spawns), and `gsd-core` into
`.claude/gsd-core`. Second pre-flight:

```
n_skills   = 87
gsd_skills = 72 entries, incl. gsd-execute-phase
gsd_slash  = 72 entries, incl. gsd-execute-phase
agents     = 39 entries, incl. gsd-executor
```

### Finding P2 — an untrusted workspace silently voids project `permissions.allow`

The first pre-flight's stderr (273 bytes — the only non-empty stderr of the whole spike)
read:

```
Ignoring 11 permissions.allow entries from .claude/settings.json: this workspace has not
been trusted. Run Claude Code interactively here once and accept the trust dialog, or set
projects["/tmp/oq1-scratch-T6XQLOYo"].hasTrustDialogAccepted: true in /home/blk/.claude.json.
```

Under `--permission-mode dontAsk`, an ignored allow-list means every write and every `Bash`
call is denied, and the run fails for **sandbox** reasons that look nothing like a transport
failure. The scratch workspace was marked trusted via a single scoped key in
`~/.claude.json` (`projects["/tmp/oq1-scratch-T6XQLOYo"].hasTrustDialogAccepted = true`).
After that, stderr was **0 bytes** for every subsequent run.

Both findings are carried into `## What this means` below — neither is a spike artefact,
both are host-configuration preconditions the driver must own.

---

## Step 3 — The main run

```bash
env -u CLAUDE… CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS=600000 \
timeout -s TERM 900 \
claude -p --input-format stream-json --output-format stream-json --verbose \
  --replay-user-messages --session-id c1679680-e8fa-4ae0-bff5-87b108b6e08a \
  --setting-sources project --permission-mode dontAsk --strict-mcp-config \
  < msg.ndjson
```

stdin carried exactly one NDJSON user message: `/gsd-execute-phase 1`.
Transcript: `transcripts-raw/08-oq1-multistep.ndjson` (474 lines, 902 KB).

### Headline signals

| Signal | Value | Reading |
|--------|-------|---------|
| **exit code** | **0** | The process exited **on its own**, not on the external bound. Not 124. |
| **wall clock** | **774.6 s** (12 min 55 s) | Well inside the 900s cap |
| **`duration_ms`** | **773 293** | **Not pinned** to the 900 000 ms bound — it tracks real work |
| **`duration_api_ms`** | 766 772 | 99.2 % of `duration_ms`; the process was working, not waiting |
| `subtype` | `success` | |
| `terminal_reason` | `completed` | **not** `aborted_tools` |
| `stop_reason` | `end_turn` | |
| `is_error` | `false` | |
| `num_turns` | 61 | a genuinely multi-step agentic run |
| `permission_denials` | `[]` (empty) | `dontAsk` never blocked anything the run needed |
| `total_cost_usd` | 7.428 | notional under subscription auth (D-16) |
| **stderr** | **0 bytes** | corroborates the research note that a clean run writes nothing to stderr |

Contrast with the reproduced hang: there, `duration_ms` **equalled** the wall-clock cap
(89 134 ms against a 90s bound) while `duration_api_ms` was 3 447 ms. Here the two are
within 0.8 % of each other and both are far below the cap. That is the decisive shape
difference.

### `system/hook_started` — the causal tell

```
hook_started  = 0
hook_response = 0
```

**No hook event of any kind appeared anywhere in the 474-line transcript** — not before the
first `system/init`, and not at any point during 61 turns of subagent work. In the hung arm
captured during research, `system/hook_started` ×2 and `system/hook_response` ×2 appeared
*before* `system/init`. The mitigation holds for the full multi-step run, which is question
(a) answered.

### Full `type`/`subtype` inventory (474 lines, every one valid JSON)

| count | `type`/`subtype` |
|-------|------------------|
| 161 | `assistant/-` |
| 132 | `system/thinking_tokens` |
| 115 | `user/-` |
| 51 | `system/task_progress` |
| 3 | `system/task_updated` |
| 3 | `system/task_started` |
| 3 | `system/task_notification` |
| 2 | `system/vcs_state_changed` |
| 2 | `rate_limit_event/-` |
| 1 | `system/init` |
| 1 | `result/success` |

**Five `system` subtypes here are documented in no research file:** `task_progress`,
`task_started`, `task_updated`, `task_notification`, `vcs_state_changed`. Together with
`thinking_tokens` they account for **194 of 474 lines — 41 % of the stream is
subtypes the parser has never been told about.** This is the strongest available evidence
for D-09 (tolerant parsing, catch-all variant, never fatal) and for D-17 (bounded-batch
drain): a parser with an exhaustive `subtype` match would have failed this run outright.

### Subagent evidence

Three distinct non-null `parent_tool_use_id` values appear in the stream — subagent traffic
is multiplexed into the same NDJSON stream as the orchestrator's own, tagged by parent tool
use. The `system/task_started` ×3 / `task_updated` ×3 / `task_progress` ×51 events are the
subagent lifecycle surfacing on the wire.

**An important structural clarification for question (a).** Claude Code's subagents run
**in-process** (`Task`/`Agent` tool), not as nested `claude` CLI child processes. There is
therefore no separate process for `--setting-sources project` to propagate *to*; the
setting-source resolution happens once, at session start, and governs the whole process
including all subagent work. That is why zero hook events appear across 61 turns. The
concern as originally phrased ("subagent waves spawn nested `claude` processes") does not
describe how 2.1.220 works, and the answer is better than the question feared.

### Proof the run did real work (not a fast, empty success)

Both files the scratch plan promised exist on disk with exactly the specified contents:

```
/tmp/oq1-scratch-T6XQLOYo/hello.txt    -> "hello from the scratch project"
/tmp/oq1-scratch-T6XQLOYo/goodbye.txt  -> "goodbye from the scratch project"
```

and the scratch repo carries **nine** new commits made by the headless run on top of the two
seed commits:

```
7cf56fc docs(phase-01): evolve PROJECT.md after phase completion
b5b513d chore(phase-01): clear ephemeral auto-chain flag (manual invocation)
600f7a3 docs(phase-01): complete phase execution
f5ed3eb docs(01): add code review report
d403ad8 docs(01-01): correct deviation commit references in summary
e59c4a1 docs(01-01): correct stale planning artifact writes
f56470c docs(01-01): complete hello plan
19eeccc feat(01-01): create goodbye.txt farewell artifact
128bbda feat(01-01): create hello.txt greeting artifact
```

The run executed the plan, committed per task, wrote a SUMMARY, ran a verifier subagent and
a code-review subagent, and updated the roadmap — the full `/gsd-execute-phase` workflow,
unattended, from a single line on stdin.

### Question (b) — the background-subagent ceiling

`CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS` was set **explicitly** to `600000` (10 min) rather
than inherited (D-14). The run's wall clock was **774.6 s — 29 % beyond that ceiling — and
it completed cleanly with `terminal_reason: "completed"` and a full `result` envelope.**

The ceiling therefore did **not** truncate the run. Reading the observed behaviour together
with the documented semantics: the ceiling bounds how long the CLI *waits for background
subagents after the final result*, not the total duration of foreground agentic work. A
`/gsd-execute-phase` run whose subagents are foreground `Task` calls is not exposed to it.
The value must still be set explicitly rather than inherited, because a silent default is
exactly the kind of invisible bound that produces an unexplained truncated tail — but it is
not the run-length cap it was feared to be.

A liveness sampler recorded (epoch, line-count) every 5 s for the whole run. The stream
advanced continuously from t+5 s to the end with no silent gap approaching the idle cap —
direct empirical support for D-13's choice of an **idle** cap (time since last stream line)
over a time-since-spawn cap. A 774-second legitimate run and a 90-second hang are
indistinguishable by elapsed time and trivially distinguishable by stream liveness.

---

## Step 4 — The A3 sub-probe (tool-using queued turn)

RESEARCH Assumption **A3** asked whether the per-turn `system/init` + `result` behaviour
observed for a *text-only* queued turn also holds when the queued turn **uses tools**.

Driver: a FIFO held open across the run. `msg1` (a 400-word essay, text-only) at t=0;
`msg2` (`"IGNORE the essay task. Read the file ./hello.txt … and reply with its exact
contents"`) written **mid-turn at t=12 s**; stdin closed at t=14 s. `timeout -s TERM 300`.
Transcript: `transcripts-raw/09-oq1-tooluse-two-turns.ndjson` (19 lines).

**A3 finding, as counts:**

| Envelope | Count |
|----------|-------|
| **`system/init`** | **2** |
| **`type:"result"`** | **2** |

Ordered walk:

| # | event | note |
|---|-------|------|
| 1 | `system/init` | first init |
| 2 | `rate_limit_event` | |
| 3-6 | `system/thinking_tokens` ×4 | |
| 7 | `user` `isReplay:true` @09:29:30.382 | msg1 echo |
| 8-9 | `assistant` ×2 | the essay |
| 10 | **`result` `success`** | **turn 1 closes** |
| 11 | **`system/init`** | **second init** |
| 12-13 | `system/thinking_tokens` ×2 | |
| 14 | `user` `isReplay:true` @09:29:52.688 | msg2 echo — **8 ms after turn 1's `result`**, ~41 s after it was written |
| 15 | `assistant` | contains a **`tool_use` block, tool `Read`** |
| 16 | `assistant` | |
| 17 | `user` @09:29:57.288 | tool result (not a replay) |
| 18 | `assistant` | |
| 19 | **`result` `success`** | **turn 2 closes**, `result: "hello from the scratch project"` |

Process exit **0**, wall 31.6 s, stderr 0 bytes, `hook_started` = 0.

**A3 is settled: the behaviour is identical for a tool-using queued turn.** Two `init`s, two
`result`s, the tool genuinely ran (one `Read` `tool_use` block, and the returned value is
the real file content). D-29 and D-30 hold without a tool-use carve-out.

Two secondary details worth carrying:

- `num_turns` was **1** on turn 1 and **2** on turn 2 — it counts *internal* agentic turns
  within the queued turn (the tool round-trip added one) and **resets per envelope**. A
  run-level turn count must be summed, never read off the last envelope (D-29).
- `duration_api_ms` (31 230) again **exceeds** `duration_ms` (5 740) on the second envelope,
  because it is cumulative across the run while `duration_ms` is per-turn. Confirms the D-13
  gotcha: `duration_api_ms` is not a wall clock and must never drive the idle timer.

---

## PASS conditions, checked

| Condition (D-27) | Observed | Met |
|------------------|----------|-----|
| Process exited on its own with a code other than 124 | exit **0** | ✅ |
| Last `result`'s `duration_ms` NOT pinned to the wall-clock bound | 773 293 ms against a 900 000 ms bound | ✅ |
| No `terminal_reason` of `aborted_tools` caused by silence | `terminal_reason: "completed"` | ✅ |
| The scratch project's task files exist on disk | `hello.txt` + `goodbye.txt`, correct contents, committed | ✅ |

**All four met. Verdict: PASS.**

---

## What this means

The milestone's premise holds. `claude -p` over duplex `stream-json` **does** reliably run
a real multi-step GSD skill headlessly to completion — 61 turns, three subagent waves, 13
minutes, nine commits, a clean `result` envelope and a self-driven exit 0 — provided
`--setting-sources project` is passed. Phase 15 may proceed to build the executor on this
foundation, and plans 15-02 … 15-06 may treat their `**Verdict:** PASS` precondition as met.

Three things this spike learned that the plans must absorb:

1. **`--setting-sources project` strips the user-scope GSD install (P1).** The flag is
   load-bearing for the hang mitigation and simultaneously removes `~/.claude/skills/gsd-*`
   and `~/.claude/agents/gsd-*` from the session. A target project with only a *user-scope*
   GSD install will show zero `gsd*` entries in `system/init.skills[]` and the driver's
   prompt will not resolve. **This is a pre-flight gate the driver must own**, and
   `system/init` already carries everything needed to check it: assert that
   `skills[]`/`slash_commands[]` contains the GSD command about to be sent, and refuse up
   front with a typed error naming the missing skill — the same D-06 shape as the capability
   gate, on the same event, at the same moment, costing zero quota. Phase 17's `driver_opt_in`
   record is the natural home for "this project has a project-scope GSD install".

2. **An untrusted workspace silently voids project `permissions.allow` (P2).** Combined with
   `--permission-mode dontAsk` this denies every write, and the resulting failure looks like
   a capability problem rather than a configuration one. The tells are cheap and both
   already on the wire: a **non-empty stderr** (this is the only non-empty stderr the entire
   spike produced — vindicating D-04's insistence that the pipes stay separable) and a
   populated `permission_denials[]` in the `result` envelope. Surface both.

3. **41 % of a real run's stream is `subtype`s no research document lists.** `task_progress`,
   `task_started`, `task_updated`, `task_notification` and `vcs_state_changed` join
   `thinking_tokens` as high-volume undocumented events. D-09's tolerant parsing is not
   defensive hygiene here, it is a hard requirement — and D-17's bounded-batch drain has a
   concrete load figure to design against: **474 lines / 775 s** average, but bursty, with
   `task_progress` arriving in clusters.

---

## Residue

- **Scratch project** `/tmp/oq1-scratch-T6XQLOYo` — left in place under the system temp root
  as the evidence trail for this report. Disposable; nothing references it.
- **`~/.claude.json`** retains one scoped key,
  `projects["/tmp/oq1-scratch-T6XQLOYo"].hasTrustDialogAccepted = true`. Deliberately **not**
  reverted: `~/.claude.json` is live-written by the running Claude Code session, so a
  read-modify-write to remove one key risks clobbering concurrent writes. The key is inert
  once the scratch directory is gone. The full-file backup the setup script took was deleted
  rather than left in `$HOME`.
- **Raw transcripts** staged in the gitignored
  `.planning/phases/15-transport-foundation/transcripts-raw/`. Per the plan, only the A3
  capture is promoted to a redacted golden fixture
  (`tests/fixtures/transcripts/08-tooluse-queued-two-turns.ndjson`); the 902 KB multi-step
  capture is not promoted — its value is this report.
