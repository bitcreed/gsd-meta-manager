#!/bin/sh
# An **orphaning** stand-in for the `claude` CLI.
#
# It exists to prove the one condition stdout EOF cannot escape: a descendant
# that outlives the leader and keeps the stdout pipe open. `reader_rx` yields
# `None` only on stdout EOF, and stdout reaches EOF only once EVERY process
# holding the write end is gone. The real `claude` routinely backgrounds Bash
# grandchildren that inherit stdout, so a leader that exits while one is still
# alive leaves the supervisor with no escape at all — and with the exited flag
# set, every deadline guarded on it is switched off too (CR-02).
#
# Usage (as leading arguments, before the generated claude argv):
#
#     fake-claude-orphan.sh [ignored...]
#
# Every argument is ignored on purpose: the executor appends the real `claude`
# argv, and this stand-in must tolerate it exactly the way a forward-compatible
# CLI would.
#
# The descendant announces its pid on a line whose message `type` no CLI version
# emits, in the same shape the spawner stand-in uses, so the tolerant parser
# carries it as a forward-compatible unknown **with its raw text intact** and
# the test's existing pid helper can read it.
#
# Nothing here carries host state: the file is fully synthetic (T-15-54).

set -u

# Drain stdin in the background so the executor's writer task never takes a
# broken pipe when the gate releases the prompt.
cat >/dev/null &

printf '{"type":"system","subtype":"init","session_id":"s","permissionMode":"dontAsk","claude_code_version":"2.1.220","apiKeySource":"none","capabilities":["interrupt_receipt_v1","interrupt_cancel_queued_v1","msg_lifecycle_v1"]}\n'

# The descendant. Backgrounded from a non-interactive shell it inherits this
# process group AND this stdout, so a group-wide signal reaches it and, until it
# is gone, stdout never reaches EOF.
#
# It chatters briefly before falling silent, and that is load-bearing rather
# than decorative. `ProcessGroupChild::wait()` awaits the leader, caches its
# status, then blocks reaping the rest of the group — so against a descendant
# that is silent from the very first instant, the supervisor's exit arm stays
# pending inside that group reap and the exited flag never flips at all; the run
# would then end on the idle cap and prove nothing about the post-exit drain.
# A few lines arriving after the leader is gone let another arm win, dropping
# that partially-polled future, and the next poll returns the CACHED leader
# status immediately — which is exactly the "exited but NOT reaped" state CR-02
# is about.
#
# It then holds the pipe open in SILENCE for longer than any bound the test
# carries — longer than the test's own 60-second hard timeout — so that the
# post-exit drain bound is demonstrably the only thing that can end the run. A
# shorter hold would let the descendant expire on its own and the test would
# pass without the bound existing at all.
#
# On a PASSING run the executor's own teardown reaches the whole process group
# and this descendant is gone within a few seconds — which is precisely what the
# test asserts. On a FAILING run nothing else reaps it: libtest ends the process
# with `exit`, so no destructor runs and `KillOnDrop` never fires. The
# self-terminating bound is therefore the real backstop, and it is deliberately
# short enough that repeated runs cannot accumulate strays.
{
    beat=0
    while [ "$beat" -lt 10 ]; do
        beat=$((beat + 1))
        sleep 0.2
        printf '{"type":"system","subtype":"thinking_tokens","estimated_tokens":%s,"estimated_tokens_delta":1,"session_id":"s","uuid":"orphan-%s"}\n' \
            "$beat" "$beat"
    done
    sleep 90
} &
DESCENDANT=$!

printf '{"type":"descendant_announcement","pid":%s}\n' "$DESCENDANT"

printf '{"type":"result","subtype":"success","is_error":false,"terminal_reason":"completed","session_id":"s","num_turns":1,"total_cost_usd":0.01,"result":"done","permission_denials":[],"uuid":"result-1"}\n'

# And now the entire point of this stand-in: the leader exits WITHOUT ever
# reaping or joining the descendant it started. Adding a join here would turn
# this script into the spawner stand-in and the condition under test would
# disappear.
exit 0
