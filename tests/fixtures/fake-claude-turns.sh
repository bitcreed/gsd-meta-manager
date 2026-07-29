#!/bin/sh
# A **multi-turn**, stdin-reactive stand-in for the `claude` CLI.
#
# `fake-claude-echo.sh` is the duplex stand-in the Phase 15 transport tests use.
# It emits its single terminal `result` only *after* stdin EOF, which was exactly
# right while the driver closed stdin immediately after spawn — and is wrong for
# a steerable run. Under D-11 the driver closes stdin at a `result` boundary with
# an empty inbox, so a stand-in that never emits a `result` before EOF and a
# driver that never closes stdin before a `result` wait for each other forever.
#
# This file is that one behavioural difference and nothing else: **one `result`
# per turn, emitted as soon as the turn's user message has been echoed**, which
# is what the real CLI does. `fake-claude-echo.sh` is left untouched, because the
# transport tests pin its exact event sequence and changing it under them would
# trade one phase's convenience for another phase's regression guard.
#
# Usage (as leading arguments, before the generated claude argv):
#
#     fake-claude-turns.sh <capabilities-csv> <version> <api-key-source> <stdin-log> [ignored...]
#
#   capabilities-csv  comma-separated capability names; EMPTY emits `[]`
#   version           `claude_code_version`; `-` omits the field entirely
#   api-key-source    `apiKeySource`; `-` omits the field entirely
#   stdin-log         every line the driver writes to stdin is appended here.
#                     The file is TRUNCATED at startup, before the init is
#                     emitted, so a zero-length log proves the driver wrote
#                     nothing rather than proving the child never ran.
#
# Every argument after the fourth is ignored on purpose: the executor appends the
# real `claude` argv, and this stand-in must tolerate it exactly the way a
# forward-compatible CLI would.
#
# Behaviour, matching the 2.1.220 observations this phase is built on:
#
#   * a `user` message is echoed back with `"isReplay":true` — the replay echo,
#     emitted after a short delay to stand in for dequeue latency;
#   * the echo is followed by that turn's `result`, because `result` closes a
#     TURN and not the run (Phase 15 D-29);
#   * a `control_request` is answered with a `control_response` carrying the SAME
#     `request_id` and a doubly-nested `still_queued` array;
#   * stdin EOF is "no more input", not "stop": the loop simply ends and the
#     process exits 0. There is no extra terminal envelope at EOF — the last turn
#     already wrote its own.

set -u

CAPABILITIES="${1?usage: fake-claude-turns.sh <capabilities-csv> <version> <api-key-source> <stdin-log>}"
VERSION="${2?usage: fake-claude-turns.sh <capabilities-csv> <version> <api-key-source> <stdin-log>}"
API_KEY_SOURCE="${3?usage: fake-claude-turns.sh <capabilities-csv> <version> <api-key-source> <stdin-log>}"
STDIN_LOG="${4?usage: fake-claude-turns.sh <capabilities-csv> <version> <api-key-source> <stdin-log>}"

# Truncate before anything is emitted: the gate can only refuse after it has read
# the init below, so by the time a refusal happens this file exists.
: >"$STDIN_LOG"

capabilities_json=""
if [ -n "$CAPABILITIES" ]; then
    capabilities_json=$(printf '%s' "$CAPABILITIES" | sed 's/[^,]\{1,\}/"&"/g')
fi

version_field=""
if [ "$VERSION" != "-" ]; then
    version_field=$(printf ',"claude_code_version":"%s"' "$VERSION")
fi

auth_field=""
if [ "$API_KEY_SOURCE" != "-" ]; then
    auth_field=$(printf ',"apiKeySource":"%s"' "$API_KEY_SOURCE")
fi

printf '{"type":"system","subtype":"init","session_id":"s","permissionMode":"dontAsk"%s%s,"capabilities":[%s]}\n' \
    "$version_field" "$auth_field" "$capabilities_json"

seq=0
while IFS= read -r line; do
    printf '%s\n' "$line" >>"$STDIN_LOG"
    seq=$((seq + 1))

    case "$line" in
    *'"type":"control_request"'*)
        request_id=$(printf '%s' "$line" | sed -n 's/.*"request_id":"\([^"]*\)".*/\1/p')
        # `success` acknowledges the REQUEST. `still_queued` is the
        # authoritative statement of what remains queued (D-31).
        printf '{"type":"control_response","response":{"subtype":"success","request_id":"%s","response":{"still_queued":["queued-turn-1"]}}}\n' \
            "$request_id"
        ;;
    *'"type":"user"'*)
        # The echo is emitted at dequeue, not at receipt — the delay stands in
        # for that. Reusing the received body keeps the echo honest.
        sleep 0.02
        printf '%s,"session_id":"s","isReplay":true,"uuid":"echo-%s"}\n' \
            "$(printf '%s' "$line" | sed 's/}$//')" "$seq"
        # And the turn closes. `total_cost_usd` accumulates across turns the way
        # the real CLI's does; `num_turns` is per-turn and resets (D-12).
        printf '{"type":"result","subtype":"success","is_error":false,"terminal_reason":"completed","session_id":"s","num_turns":1,"total_cost_usd":0.0%s,"result":"turn %s done","uuid":"result-%s"}\n' \
            "$seq" "$seq" "$seq"
        ;;
    esac
done

exit 0
