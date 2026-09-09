#!/bin/sh
# A **late-announcing** stand-in for the `claude` CLI: it prints NOTHING at
# startup and emits its `system/init` envelope only after it has read its first
# stdin line.
#
# That single difference from `fake-claude-echo.sh` is the entire fixture, and
# it is not hypothetical. It stands in for the measured behaviour of the
# installed CLI **2.1.266** (260908-uqq CONTEXT, "Root cause"):
#
#   * `claude -p --input-format stream-json --output-format stream-json
#     --verbose --session-id <uuid> --setting-sources project --permission-mode
#     dontAsk --strict-mcp-config` with stdin held open and empty produced
#     **0 bytes on stdout and 0 bytes on stderr**, stayed alive for as long as
#     stdin stayed open, and exited 0 on EOF;
#   * the same invocation with a user message written at +6 s produced
#     `system/init` at +6 s, the assistant message at +7 s and the result at
#     +7 s.
#
# So on that CLI `system/init` is emitted *because of* the first user message,
# not at startup. An executor that withholds the prompt until the gate has
# judged an init deadlocks against it: the driver waits for `init`, the CLI
# waits for the prompt, and only the run-wide idle cap breaks the tie. The
# repository's other stand-ins all announce eagerly (2.1.220's shape), so none
# of them can express this and none of them could have caught the deadlock.
#
# The behaviour this fixture pins is the `prompt_release_grace` release: the
# supervisor writes the prompt anyway once the grace expires, which is the only
# thing that gets a run past startup here.
#
# Usage (as leading arguments, before the generated claude argv):
#
#     fake-claude-late-init.sh <capabilities-csv> <version> <api-key-source> <stdin-log> [ignored...]
#
#   capabilities-csv  comma-separated capability names; EMPTY emits `[]`
#   version           `claude_code_version`; `-` omits the field entirely
#   api-key-source    `apiKeySource`; `-` omits the field entirely
#   stdin-log         every line the driver writes to stdin is appended here.
#                     The file is TRUNCATED at startup — before anything is
#                     read and before the init is emitted — so a zero-length
#                     log proves the driver wrote nothing rather than proving
#                     the child never ran, and a NON-empty log proves the
#                     prompt was released before the gate could rule.
#
# Every argument after the fourth is ignored on purpose: the executor appends
# the real `claude` argv, and this stand-in must tolerate it exactly the way a
# forward-compatible CLI would.
#
# Everything downstream of the announcement matches `fake-claude-echo.sh`
# exactly — replay echo with `"isReplay":true`, a correlated `control_response`,
# a terminal `result` at stdin EOF, exit 0 — so a test can swap the two
# stand-ins and change only *when* the init arrives.

set -u

CAPABILITIES="${1?usage: fake-claude-late-init.sh <capabilities-csv> <version> <api-key-source> <stdin-log>}"
VERSION="${2?usage: fake-claude-late-init.sh <capabilities-csv> <version> <api-key-source> <stdin-log>}"
API_KEY_SOURCE="${3?usage: fake-claude-late-init.sh <capabilities-csv> <version> <api-key-source> <stdin-log>}"
STDIN_LOG="${4?usage: fake-claude-late-init.sh <capabilities-csv> <version> <api-key-source> <stdin-log>}"

# Truncate before anything is read and before anything is emitted. Unlike the
# eager stand-in, here the gate cannot possibly have refused yet — nothing has
# been announced — so this file existing proves only that the child ran.
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

# NOTHING is printed here. That omission is the fixture.

announced=0
seq=0
while IFS= read -r line; do
    printf '%s\n' "$line" >>"$STDIN_LOG"

    # The announcement is triggered by the first stdin line and by nothing
    # else, exactly as 2.1.266 was measured to behave. It is emitted AFTER the
    # line has been logged, so a test that finds an init on the stream can
    # always also find the message that provoked it in the log.
    if [ "$announced" -eq 0 ]; then
        announced=1
        printf '{"type":"system","subtype":"init","session_id":"s","permissionMode":"dontAsk"%s%s,"capabilities":[%s]}\n' \
            "$version_field" "$auth_field" "$capabilities_json"
    fi

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
        ;;
    esac
done

printf '{"type":"result","subtype":"success","is_error":false,"terminal_reason":"completed","session_id":"s","num_turns":1,"total_cost_usd":0.0125,"result":"done","uuid":"result-1"}\n'

exit 0
