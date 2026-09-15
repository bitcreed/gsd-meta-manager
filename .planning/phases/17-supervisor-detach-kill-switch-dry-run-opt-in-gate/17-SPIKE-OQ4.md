# OQ4 Spike — Does the `claude` CLI have a `--worktree` flag, and does it behave as inferred?

**Verdict:** RESOLVED — the flag **exists**, is **documented**, and **works under `-p`**.
Its observed semantics are **wrong for this codebase**, so Phase 17 **declines** it (D-21) and
Phase 19 inherits the decision with the `git worktree add` fallback already identified as
strictly better.

**Run date:** 2026-09-15 (re-verification)
**Original probe:** 2026-07-29, during the Phase 17 context pass (`17-CONTEXT.md` § *OQ4 spike
result*)
**Claude CLI:** 2.1.272 this pass; 2.1.220 on the original probe
**Spike owner:** Phase 17 — ROADMAP *Phase risks* → **MUST-SPIKE (OQ4)**

---

## Scope

The ROADMAP risk reads:

> **MUST-SPIKE (OQ4)**: does the `--worktree` flag exist and behave as inferred? It was
> inferred from the statusline JSON schema, not a flag reference, and worktree isolation is
> load-bearing for driver-vs-human safety. Fallback: `git worktree add` manually and point the
> driver's cwd at it

Two distinct questions, and they have different answers:

- **(a)** *Does the flag exist at all?* — it was inferred from a statusline JSON field, never
  from a flag reference, so the whole risk was that ARCHITECTURE had invented it. **It exists.**
- **(b)** *Does it behave as inferred* — i.e. is it a usable isolation primitive for the
  driver's cwd? **No.** It works, but it writes inside the repo working directory and picks its
  own branch name, both of which collide with load-bearing invariants of this codebase.

**Prerequisites:** none outstanding. Phase 17 is `status: passed` (`17-VERIFICATION.md`,
8/8 plans), the probe needs nothing from Phase 18–21, and the only environmental requirement —
an installed `claude` CLI on a subscription auth path — is satisfied (`apiKeySource: "none"`).

---

## Safety fences

Identical to the Phase 15 spike discipline (D-28), and re-applied here:

- cwd was a **disposable scratch git repo** created for this probe under the session
  scratchpad — never this repo, never a registered GSD project.
- The scratch repo has a single commit and no `.planning/`, so nothing the probe created could
  be picked up by `watcher.rs::extract_project_root`.
- Run under an external `timeout 180`.
- No `--dangerously-skip-permissions`, no `--permission-mode bypassPermissions`; the observed
  `permissionMode` was `default` and `permission_denials` came back empty.
- Prompt was a single no-tool reply, so nothing was written by the agent itself; every artifact
  observed below was created by the **flag**, not by the turn.

---

## Step 1 — Flag surface (`claude --help`, 2.1.272)

Verbatim, two entries:

```
  --tmux                                Create a tmux session for the worktree
                                        (requires --worktree). Uses iTerm2
                                        native panes when available; use
                                        --tmux=classic for traditional tmux.

  -w, --worktree [name]                 Create a new git worktree for this
                                        session (optionally specify a name)
```

Also in `Commands:` — `rm <id>` *"Delete a background session, and its worktree when that is
safe."*

Three independent references (`-w`, `--tmux`'s dependency, `rm`'s cleanup clause) settle
question (a): `--worktree` is a **first-class, documented flag**, not an artifact of the
statusline schema. The optional-argument form `[name]` also matches ARCHITECTURE's inference.

---

## Step 2 — Execution probe under `-p`

```bash
claude -p --worktree oq4probe272 --output-format stream-json --verbose \
       --setting-sources project 'Reply with exactly: OK'
```

Observed — exit **0**, stderr **empty**, four envelopes:

```
system/init → rate_limit_event → assistant → result/success
```

```json
{"type":"result","subtype":"success","is_error":false,
 "terminal_reason":"completed","result":"OK",
 "duration_ms":1682,"num_turns":1,"permission_denials":[]}
```

`system/init.cwd` = `<repo>/.claude/worktrees/oq4probe272` — the flag **did** relocate the
session's working directory. Same envelope sequence and same `cwd` shape as the 2.1.220 probe.

### `system/init` key set, and the absence that matters

```
agents, analytics_disabled, apiKeySource, capabilities, claude_code_version, cwd,
fast_mode_disabled_reason, fast_mode_state, mcp_servers, memory_paths, messaging_socket_path,
model, output_style, permissionMode, plugins, product_feedback_disabled, session_id, skills,
slash_commands, subtype, terminal_slash_commands, tools, type, uuid
```

**There is no `worktree` key.** This is the same absence the original probe found; 2.1.272 adds
`messaging_socket_path` and `terminal_slash_commands` to the 2.1.220 set but still exposes no
worktree field. A driver therefore **cannot read the worktree path back off the `-p`
protocol** — it can only infer it from `cwd`, or compute it itself. That is a second,
independent reason the fallback (`git worktree add`, where the driver *chooses* the path) is
better than the flag.

Unchanged from Phase 15's recording: `capabilities` = `interrupt_receipt_v1`,
`interrupt_cancel_queued_v1`, `msg_lifecycle_v1`; `apiKeySource: "none"` (subscription path
alive, D-08's Phase-15 guard still valid).

Minor extra finding: `memory_paths.auto` was keyed to the **launch directory** (the main
checkout), not to the worktree cwd — so a `--worktree` session's memory anchor and its working
directory disagree.

---

## Step 3 — What the flag left on disk

```
$ git worktree list
<repo>                                   e7d0ace [main]
<repo>/.claude/worktrees/oq4probe272     e7d0ace [worktree-oq4probe272]  locked

$ git branch
* main
  + worktree-oq4probe272

$ git status --porcelain
?? .claude/                              ← untracked AND unignored

$ ls .gitignore                          → No such file or directory
$ grep -vc '^#\|^$' .git/info/exclude    → 0     (template comments only)

$ git worktree prune -v && git worktree list
<worktree still listed>                  ← locked, so prune is a no-op

$ cat .git/worktrees/oq4probe272/locked
claude session oq4probe272 (pid 349868 start 355441)
```

**All five of D-21's constraints reproduce verbatim on 2.1.272.** Nothing about the flag's
placement, branch naming, ignore behaviour, or lock behaviour changed across 52 patch versions.

---

## The SDK/tool-level `EnterWorktree` — related, but *not* evidence for question (a)

This Claude Code session exposes agent-SDK tools named `EnterWorktree` / `ExitWorktree`. It is
worth recording explicitly that these are **a different surface from the CLI flag**, and that
they would **not** have answered OQ4 on their own:

- They are **in-session tools an agent calls**, not process-launch arguments. A supervisor that
  spawns `claude -p` cannot invoke them; it can only pass flags.
- Their presence proves that *Claude Code the product* has worktree support. It says nothing
  about whether that support is reachable as `-w/--worktree` on the binary this machine has
  installed — which is precisely what OQ4 asked. Only `--help` and an actual `-p` run settle
  that.
- They do, however, **corroborate the disqualifying semantics**: `EnterWorktree`'s own
  description states it "creates a new git worktree inside `.claude/worktrees/` on a new
  branch", which is the same in-repo placement the probe observed, and confirms it is a
  product-wide convention rather than a `-p`-only quirk. It also documents a `worktree.baseRef`
  setting (`fresh` | `head`) — note that **base ref** is configurable while the **branch name**
  is not, so constraint #3 below is not settable away.

Treat the tool surface as *corroboration of behaviour*, never as *proof of the flag*.

---

## Why Phase 17 declines it anyway (D-21, unchanged)

1. It writes **inside the repo working directory**. `watcher.rs::extract_project_root` walks
   *up* to the nearest `.planning` at arbitrary depth, so a worktree copy of `.planning/`
   resolves the project root to the **worktree**, manufacturing a phantom project and
   mis-routing `FileChanged` and journal classification.
2. The worktree gets its **own `.planning/`**, so the journal the driver writes (main checkout)
   and the artifacts the agent writes (worktree) diverge — and `RunSnapshot::capture(project_root)`
   (`src/executor/outcome.rs:60`) would fingerprint the wrong tree, breaking Phase 15's
   disk-corroborated outcome derivation.
3. The branch name `worktree-<name>` is **chosen by the CLI and is not configurable**, colliding
   head-on with Phase 19's `gsd-auto/**` push-prefix allowlist.
4. `.claude/` is left **untracked and unignored** in the main checkout — a driven agent's own
   `git add -A` sweeps the entire worktree into a commit.
5. The worktree is **`locked`** (lock reason names the claude session pid), so `git worktree
   prune` will not clean it; abandoned worktrees accumulate and need explicit
   `git worktree remove --force`.

Plus the Step 2 finding: **no `worktree` key on the `-p` envelope**, so the driver cannot even
read back the path the CLI chose.

---

## Resolution conditions, checked

| Condition | Result |
|---|---|
| Flag exists on the installed CLI | **YES** — `-w, --worktree [name]`, 2.1.272 and 2.1.220 |
| Documented, not an artifact of the statusline schema | **YES** — three independent `--help` references |
| Works in headless `-p` mode | **YES** — exit 0, `result/success`, empty stderr |
| Behaves as inferred (usable driver-cwd isolation) | **NO** — five disqualifying constraints, all reproduced |
| Fallback required | **YES** — `git worktree add` outside the repo, driver-chosen path and branch |
| Adopted by Phase 17 | **NO** — deliberately declined; Phase 19 owns worktree isolation |

**OQ4 is closed.** Nothing remains for a later phase to re-probe; Phase 19 inherits the five
constraints as given.

---

## Residue

- **Scratch repo** under the session scratchpad
  (`…/scratchpad/oq4probe/repo`) with its `.claude/worktrees/oq4probe272` worktree still
  `locked` — session-scoped and disposable; nothing references it. Removing it needs
  `git worktree remove --force`, which is itself constraint #5 demonstrated.
- **`~/.claude.json`** gains one inert scoped key for the scratch path, per the Phase 15
  precedent — deliberately not reverted, since the file is live-written by the running session.
- No change to this repo's working tree was made by the probe; the probe never ran with cwd
  inside it.
