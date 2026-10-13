# Spike Conventions

These are the patterns and stack choices shared by the spike sessions. New spikes follow them unless the question
needs something else.

## Stack

- Spike tooling is bash plus Python 3 using only the stdlib (it runs as `python3 -I`, with no
  venv). Nothing goes into `src/`. A Rust port happens only in a real phase.
- Driven agents run as headless `claude -p` with `--output-format json` or `stream-json`. Each
  experiment gets its own `--max-budget-usd`.

## Structure

- `NNN-name/{README.md, MASTER-PROMPT.md, templates/, tools/, experiment/{setup*.sh, results/}}`.
- Each spike builds its scratch projects with its own `experiment/setup*.sh`, in a temporary
  directory or the session scratchpad. They are never built inside this repo.
- Each spike's results are copied into `experiment/results/`: the logs, the final state, a cost
  file per run, and any measurement output. Secrets are never copied.

## Patterns

- Sandbox: `systemd-run --user --scope` wrapped around
  `bwrap --ro-bind / / --tmpfs /tmp --bind <project> --unshare-pid --die-with-parent`.
  - From spike 002: mask `~/.ssh`, `~/.claude/.credentials.json` and `~/.claude.json`.
  - From spike 002: use a per-run `CLAUDE_CONFIG_DIR`, with auth from an env access token.
- Before a launch, unset the parent `CLAUDE_CODE_*`, `CLAUDECODE` and `CLAUDE_PID` environment
  variables, so the child does not attach to the coordinator's session.
- Test risky control paths with a cheap Haiku "misbehaving driver" prompt before spending on Opus.
- Measure cost per transcript (`experiment/cost_split.py` in spike 002). Per-model totals cannot
  separate driver from workers.

## Tools & Libraries

- Claude Code 2.1.296: `Agent` calls are asynchronous ("Async agent launched"), and hook payloads
  carry `agent_id` only for nested calls.
- bubblewrap 0.11.1 and systemd 259.
- `serde_yml` 0.0.13 parses the driver frontmatter.
