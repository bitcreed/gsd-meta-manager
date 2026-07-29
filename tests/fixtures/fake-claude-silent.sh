#!/bin/sh
# A **silent** stand-in for the `claude` CLI: it starts, it spawns a grandchild,
# and it never says `system/init`.
#
# That omission is the entire fixture. `ClaudeExecutor::start` blocks on the
# capability gate — a refusal must be a start-time error, not a mid-run surprise
# — and the gate opens only on the first `system/init` envelope. A stand-in that
# never emits one parks the driver *inside* `Executor::start()`, which is the
# window CR-01 is about: a real `claude` sits there for minutes whenever the
# documented `SessionStart` hook hang reproduces
# (`src/executor/mod.rs:225-239`), and a stop issued during it was swallowed.
#
# `fake-claude-spawner.sh` cannot exercise this. It emits `system/init`
# immediately, so every test built on it lands in the driver's drain loop, where
# the terminate signal was already raced correctly.
#
# Usage (as leading arguments, before the generated claude argv):
#
#     fake-claude-silent.sh <pidfile> [ignored...]
#
# **The pid file is the only channel this fixture has.** The journal cannot carry
# these two pids: the journal is fed from the executor's event drain, and the
# drain does not run until `start` returns — which, by construction, it never
# does here. So the two pids the test cannot otherwise see are written to `$1`,
# space separated: this script's own `$$` (which is the agent's process group id,
# because the executor spawns it as a group leader) and its backgrounded
# grandchild's.
#
# Every argument after the first is ignored on purpose: the executor appends the
# real `claude` argv, and this stand-in must tolerate it exactly the way a
# forward-compatible CLI would.

set -u

PIDFILE="$1"

# Drain stdin in the background so the executor's writer task never takes a
# broken pipe. The prompt is never released — the gate never opens — but the
# writer task exists from the moment of spawn.
cat >/dev/null &

# The grandchild: a child of THIS script, which is itself the executor's child.
# Backgrounded from a non-interactive shell it inherits this process group, so a
# group-wide signal reaches it and a child-only signal does not.
sleep 600 &
GRANDCHILD=$!

printf '%s %s\n' "$$" "$GRANDCHILD" > "$PIDFILE"

# Stay alive as the group leader, saying nothing. The test tears the group down
# from outside — or, when it is doing its job, the driver does.
wait "$GRANDCHILD"
exit 0
