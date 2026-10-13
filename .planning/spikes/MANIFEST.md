# Spike Manifest

## Ideas

### queue-driven-master-agent
Replace the v2.0 autonomy machinery (Phase 19 bash-parsing guard, Phase 21 LLM goal layer, Phase 22
containers, tmux/stdin injection) with a queue-driven master Claude session. A human writes direction
and work items into a file. The master dispatches one GSD step at a time to a worker subagent, records
a 15-line-or-shorter report, and decides against each item's done-when and budget. Steering means
editing the file, and gsd-mm is the editor, launcher, observer and pusher. Isolation comes from bwrap
plus `systemd-run --user --scope`.

**Requirements:**

- Never depend on tmux, stdin injection or AskUserQuestion. The only human channel is the files.
- The subagent's suggested next command is advisory. The driver decides from done-when, budget and
  evidence it has checked itself.
- Driver files are keyed by a gsd-mm-chosen run id, not by the Claude session id.
- Files have a single owner: DIRECTION.md is human-owned and LOG.md is driver-owned and append-only.
  Both live under `.planning/meta-manager/drivers/<run-id>/` (gitignored) in the main checkout.
- Pass `isolation: "worktree"` to nested GSD dispatches when the workflow's isolation config requires it.
- At most 2 gap-closure rounds per phase.

## Spikes

| # | Idea | Name | Type | Validates | Verdict | Tags |
|---|------|------|------|-----------|---------|------|
| 001 | queue-driven-master-agent | queue-driven-master-agent | standard | Given a direction file, when a headless master dispatches one worker per step and decides on done-when and budget, then items complete, file steering works, and interrupted runs reconcile, with no tmux | PARTIAL | driver, autonomy, v2.0, headless, bwrap, reconciliation |
