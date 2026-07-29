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
#   interval    seconds between heartbeats, as `sleep` understands it. The exact
#               string `0` is a **fast path**: the `sleep` call is skipped
#               entirely rather than invoked with a zero argument, because
#               `sleep` is an external program and forking one per heartbeat
#               caps the emission rate far below what a flood test needs.
#               `printf` and the arithmetic are shell builtins, so with the fork
#               removed this loop emits tens of thousands of lines per second —
#               enough to fill a bounded 8192-slot channel and keep it full,
#               which is the stalled-consumer condition CR-01 is about.
#   ending      what happens once the heartbeats are done:
#                 `silent` — sleep effectively forever, emitting nothing. The
#                           idle cap must fire.
#                 `result` — emit a terminal `result` envelope and exit 0. A run
#                           that chattered past the idle cap must still reach
#                           this.
#                 `denied` — emit a terminal `result` envelope that says
#                           `success` while carrying a POPULATED
#                           `permission_denials[]`, then exit 0. This is the
#                           `--permission-mode dontAsk` shape: the CLI's own
#                           verdict fields look clean and the denials array is
#                           the only thing that says the run was blocked. The
#                           denial record is entirely synthetic and carries a
#                           relative filename — no host path in any form.
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
    # The zero-interval fast path: never fork a `sleep` just to sleep for no
    # time. Every other interval is passed through unchanged, so no existing
    # caller's pacing moves.
    case "$INTERVAL" in
    0) ;;
    *) sleep "$INTERVAL" ;;
    esac
    printf '{"type":"system","subtype":"thinking_tokens","estimated_tokens":%s,"estimated_tokens_delta":1,"session_id":"s","uuid":"beat-%s"}\n' \
        "$emitted" "$emitted"
done

if [ "$ENDING" = "result" ]; then
    printf '{"type":"result","subtype":"success","is_error":false,"terminal_reason":"completed","session_id":"s","num_turns":1,"total_cost_usd":0.01,"result":"done","uuid":"result-1"}\n'
    exit 0
fi

# `denied`: the envelope's own verdict fields say the turn succeeded. Only the
# denials array says otherwise, and it is the array that decides the run.
if [ "$ENDING" = "denied" ]; then
    printf '{"type":"result","subtype":"success","is_error":false,"terminal_reason":"completed","session_id":"s","num_turns":1,"total_cost_usd":0.01,"result":"done","permission_denials":[{"tool_name":"Write","tool_use_id":"toolu_denied_1","tool_input":{"file_path":"README.md"}}],"uuid":"result-1"}\n'
    exit 0
fi

# `silent`: alive, holding stdout open, and saying nothing at all. Long enough
# that no plausible test idle cap outlasts it.
sleep 600
exit 0
