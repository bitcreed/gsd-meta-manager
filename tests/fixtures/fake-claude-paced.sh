#!/bin/sh
# A **paced** multi-turn stand-in for the `claude` CLI: `fake-claude-turns.sh`
# with two windows added, and nothing else changed.
#
# Both windows exist because the properties plan 18-02 has to prove live at
# *boundaries* — the moment before the driver closes stdin, and the moment after
# — and a boundary a test cannot stand on is a boundary a test can only race.
# The alternative was a sleep in the test, which would prove the same property on
# a fast machine and flake on a loaded one.
#
#   1. **The pre-close window.** The first `PACED_TURNS` turns' `result` is
#      delayed, which delays those turn boundaries and therefore the
#      `close_input()` that follows one of them. That is the window in which a
#      message appended while the run is live must still be delivered (D-10,
#      D-11). Every turn past `PACED_TURNS` answers immediately, so the delay is
#      paid a bounded number of times per run rather than once per message.
#
#      **Why two paced turns and not one** (CR-01). The first paced turn is the
#      *command's* turn, so a message injected during it is delivered before any
#      boundary has been crossed — which a driver that can be steered exactly
#      once passes just as well as one that can be steered N times. Proving the
#      second steering needs a window on the far side of a boundary, and turn two
#      is the first injected message's own turn. Pacing it is what lets a test
#      append a SECOND message at a moment when the broken driver has already
#      closed stdin and the correct one has not. Without it that window is the
#      ~20ms between turn two's echo and its `result`, which is a race, not a
#      boundary.
#
#   2. **The post-close window.** At stdin EOF the stand-in creates
#      `<stdin-log>.eof` and then lingers before exiting. The marker is what lets
#      a test observe "the driver has closed stdin" as a fact rather than infer
#      it from elapsed time, and the linger is the window in which a message
#      appended afterwards must be journaled `missed` rather than left in
#      `queued` forever.
#
# The marker path is DERIVED from the stdin-log argument rather than passed as a
# fifth argument, and that is not a shortcut: the executor appends the real
# `claude` argv after the leading arguments, so argument five onwards is the
# CLI's own and a stand-in that claimed it would be reading the wrong thing the
# moment the generated argv changed.
#
# Usage (as leading arguments, before the generated claude argv):
#
#     fake-claude-paced.sh <capabilities-csv> <version> <api-key-source> <stdin-log> [ignored...]
#
# Everything else — the `isReplay` echo, one `result` per turn, the
# `control_response` correlation, exit 0 at EOF — is `fake-claude-turns.sh`'s
# behaviour verbatim. `fake-claude-turns.sh` is left untouched because the tests
# that pace nothing should not pay for these windows.

set -u

CAPABILITIES="${1?usage: fake-claude-paced.sh <capabilities-csv> <version> <api-key-source> <stdin-log>}"
VERSION="${2?usage: fake-claude-paced.sh <capabilities-csv> <version> <api-key-source> <stdin-log>}"
API_KEY_SOURCE="${3?usage: fake-claude-paced.sh <capabilities-csv> <version> <api-key-source> <stdin-log>}"
STDIN_LOG="${4?usage: fake-claude-paced.sh <capabilities-csv> <version> <api-key-source> <stdin-log>}"

# How long the first turn is held open, and how long the process lingers after
# stdin EOF. Both are generous by orders of magnitude against the driver's
# 750ms inbox poll, because the cost of being wrong differs by direction: too
# short flakes, too long only makes the suite slower.
TURN_PACE=2
EOF_LINGER=2

# How many turns are held open. See window 1 in the header: two, because the
# property that needs a window is "a message injected AFTER a turn boundary is
# still delivered", and turn one ends at the first boundary.
PACED_TURNS=2

EOF_MARKER="${STDIN_LOG}.eof"

# Truncate before anything is emitted, and remove any marker a previous run in
# the same directory left: a stale marker would tell a test that stdin had
# closed before this process had even read from it.
: >"$STDIN_LOG"
rm -f "$EOF_MARKER"

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
turns=0
while IFS= read -r line; do
    printf '%s\n' "$line" >>"$STDIN_LOG"
    seq=$((seq + 1))

    case "$line" in
    *'"type":"control_request"'*)
        request_id=$(printf '%s' "$line" | sed -n 's/.*"request_id":"\([^"]*\)".*/\1/p')
        printf '{"type":"control_response","response":{"subtype":"success","request_id":"%s","response":{"still_queued":["queued-turn-1"]}}}\n' \
            "$request_id"
        ;;
    *'"type":"user"'*)
        turns=$((turns + 1))
        # The echo is emitted at dequeue, not at receipt — the short delay
        # stands in for that. Reusing the received body keeps the echo honest,
        # which is what makes exact-text correlation testable at all.
        sleep 0.02
        printf '%s,"session_id":"s","isReplay":true,"uuid":"echo-%s"}\n' \
            "$(printf '%s' "$line" | sed 's/}$//')" "$seq"
        # Window 1: the first `PACED_TURNS` turns. Held AFTER the echo so the
        # echo's own correlation is not delayed with it — which is what lets a
        # test synchronise on `interjection_acted_on` and still have the whole
        # window ahead of it.
        if [ "$turns" -le "$PACED_TURNS" ]; then
            sleep "$TURN_PACE"
        fi
        printf '{"type":"result","subtype":"success","is_error":false,"terminal_reason":"completed","session_id":"s","num_turns":1,"total_cost_usd":0.0%s,"result":"turn %s done","uuid":"result-%s"}\n' \
            "$seq" "$seq" "$seq"
        ;;
    esac
done

# Window 2. The marker is written before the linger, so a test that sees it
# knows the whole linger is still ahead of it.
: >"$EOF_MARKER"
sleep "$EOF_LINGER"

exit 0
