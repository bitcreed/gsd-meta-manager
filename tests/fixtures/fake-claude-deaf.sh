#!/bin/sh
# A **signal-ignoring** stand-in for the `claude` CLI.
#
# `trap '' TERM` installs SIG_IGN, which is inherited across `exec` and by every
# child this script starts — so the whole group ignores the terminate signal.
# That forces the escalation half of the teardown: the ten-second grace elapses,
# the uncatchable signal follows, and the wait still has to complete or a zombie
# survives (D-14, T-15-34).
#
# Usage (as leading arguments, before the generated claude argv):
#
#     fake-claude-deaf.sh [ignored...]
#
# Every argument is ignored on purpose: the executor appends the real `claude`
# argv, and this stand-in must tolerate it exactly the way a forward-compatible
# CLI would.

set -u

trap '' TERM

# Drain stdin in the background so the executor's writer task never takes a
# broken pipe when the gate releases the prompt.
cat >/dev/null &

printf '{"type":"system","subtype":"init","session_id":"s","permissionMode":"dontAsk","claude_code_version":"2.1.220","apiKeySource":"none","capabilities":["interrupt_receipt_v1","interrupt_cancel_queued_v1","msg_lifecycle_v1"]}\n'

# Alive, deaf, and holding stdout open. Only the uncatchable signal ends this.
sleep 600
exit 0
