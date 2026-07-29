#!/bin/sh
# A **paced** stand-in for the `claude` CLI, for the two deadline tests.
#
# It exists to make one distinction provable: a run that goes silent is torn
# down by the idle cap, and a run that keeps emitting is not — which is exactly
# the distinction time-since-spawn cannot make (D-13). Both behaviours come from
# one script because they are one behaviour with two settings.
#
# Usage (as leading arguments, before the generated claude argv):
#
#     fake-claude-slow.sh <heartbeats> <interval> <ending> [ignored...]
#
#   heartbeats  how many `system/thinking_tokens` lines to emit after the init.
#               `0` means the stand-in goes silent immediately after announcing
#               itself, which is the hang the idle cap exists to catch.
#   interval    seconds between heartbeats, as `sleep` understands it.
#   ending      what happens once the heartbeats are done:
#                 `silent` — sleep effectively forever, emitting nothing. The
#                           idle cap must fire.
#                 `result` — emit a terminal `result` envelope and exit 0. A run
#                           that chattered past the idle cap must still reach
#                           this.
#
# Every argument after the third is ignored on purpose: the executor appends the
# real `claude` argv, and this stand-in must tolerate it exactly the way a
# forward-compatible CLI would.

set -u

HEARTBEATS="${1:?usage: fake-claude-slow.sh <heartbeats> <interval> <ending>}"
INTERVAL="${2:?usage: fake-claude-slow.sh <heartbeats> <interval> <ending>}"
ENDING="${3:?usage: fake-claude-slow.sh <heartbeats> <interval> <ending>}"

# Drain stdin in the background so the executor's writer task never takes a
# broken pipe when the gate releases the prompt.
cat >/dev/null &

printf '{"type":"system","subtype":"init","session_id":"s","permissionMode":"dontAsk","claude_code_version":"2.1.220","apiKeySource":"none","capabilities":["interrupt_receipt_v1","interrupt_cancel_queued_v1","msg_lifecycle_v1"]}\n'

emitted=0
while [ "$emitted" -lt "$HEARTBEATS" ]; do
    emitted=$((emitted + 1))
    sleep "$INTERVAL"
    printf '{"type":"system","subtype":"thinking_tokens","estimated_tokens":%s,"estimated_tokens_delta":1,"session_id":"s","uuid":"beat-%s"}\n' \
        "$emitted" "$emitted"
done

if [ "$ENDING" = "result" ]; then
    printf '{"type":"result","subtype":"success","is_error":false,"terminal_reason":"completed","session_id":"s","num_turns":1,"total_cost_usd":0.01,"result":"done","uuid":"result-1"}\n'
    exit 0
fi

# `silent`: alive, holding stdout open, and saying nothing at all. Long enough
# that no plausible test idle cap outlasts it.
sleep 600
exit 0
