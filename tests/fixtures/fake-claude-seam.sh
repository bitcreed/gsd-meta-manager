#!/bin/sh
# A stand-in for the `claude` CLI that serves BOTH spawn profiles.
#
# The driver builds one executor per run and points it at one program, so a
# stand-in that could only answer GSD commands could never exercise the model
# seams, and one that could only answer seams could never exercise the loop the
# seams sit inside. This script decides which profile it was spawned under the
# same way a reader would: `--json-schema` appears on a model-seam argv and on
# nothing else.
#
# **It leaves evidence on disk for every seam spawn**, which is the tripwire
# pattern `tests/fixtures/fake-claude-tripwire.sh` established: a count kept
# inside the driver is a count of what the driver believes it did, while a line
# appended by a program that actually ran is a fact.
#
# Usage (as leading arguments, before the generated claude argv):
#
#     fake-claude-seam.sh <workdir> <transcript> [ignored claude flags...]
#
# Files it reads under <workdir>:
#
#     seam-payload.<N>    the structured_output to answer seam spawn N with
#     seam-payload.json   the fallback, used for any spawn with no numbered file
#
# Files it writes under <workdir>:
#
#     seam-spawns         one line per seam spawn, so the count is on disk
#     seam-stdin.<N>      every byte spawn N was sent, so a test can assert on
#                         what reached the model rather than on what was meant to
#     agent-spawns        one line per EXECUTOR-profile spawn, carrying the argv
#                         it was handed
#
# `agent-spawns` is the tripwire half, added by plan 21-06 and additive to every
# earlier caller: a test proving that a refused action was never executed needs
# evidence written by a program that actually ran, because a count kept inside
# the driver is a count of what the driver believes it did. The file's ABSENCE is
# what proves no GSD command was ever run, and `tests/driver_refusal_record.rs`
# pairs every such absence with a control arm in which a legal command IS routed
# and the file DOES appear — a tripwire that has never been seen to fire proves
# nothing.
#
# A spawn with no payload available answers with a result envelope carrying no
# `structured_output` at all — which is the transport's own shape for "the model
# produced nothing usable", and the state `escalation_output_unusable` names.

set -u

WORKDIR="${1:?usage: fake-claude-seam.sh <workdir> <transcript>}"
TRANSCRIPT="${2:?usage: fake-claude-seam.sh <workdir> <transcript>}"
shift 2

IS_SEAM=0
for arg in "$@"; do
    if [ "$arg" = "--json-schema" ]; then
        IS_SEAM=1
        break
    fi
done

if [ "$IS_SEAM" -eq 0 ]; then
    # The executor profile: replay the transcript, exactly as `fake-claude.sh`
    # does and for the same reasons.
    #
    # The tripwire line goes FIRST, before any replay, so a spawn that then
    # failed for an unrelated reason still leaves the evidence that it happened.
    mkdir -p "$WORKDIR"
    printf '%s\n' "$*" >> "$WORKDIR/agent-spawns"

    cat >/dev/null &
    DRAIN_PID=$!
    while IFS= read -r line; do
        [ -z "$line" ] && continue
        printf '%s\n' "$line"
        sleep 0.01
    done < "$TRANSCRIPT"
    kill "$DRAIN_PID" 2>/dev/null
    wait "$DRAIN_PID" 2>/dev/null
    exit 0
fi

mkdir -p "$WORKDIR"
touch "$WORKDIR/seam-spawns"
N=$(( $(wc -l < "$WORKDIR/seam-spawns") + 1 ))
printf 'seam spawn %s\n' "$N" >> "$WORKDIR/seam-spawns"

SESSION="00000000-0000-4000-8000-00000000000$N"

# The init envelope FIRST. The executor withholds the first user message until
# this passes the capability gate, so nothing arrives on stdin before it.
#
# `tools` is exactly the structured-output tool, which is what the real seam
# profile's `--tools ""` produces — a stand-in advertising more would let an
# arrival assertion pass against a seam that quietly had file access.
printf '%s\n' "{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"$SESSION\",\"tools\":[\"StructuredOutput\"],\"mcp_servers\":[],\"model\":\"claude-opus-5\",\"permissionMode\":\"dontAsk\",\"apiKeySource\":\"none\",\"claude_code_version\":\"2.1.220\",\"capabilities\":[\"interrupt_receipt_v1\",\"interrupt_cancel_queued_v1\",\"msg_lifecycle_v1\"],\"uuid\":\"11111111-1111-4111-8111-00000000000$N\"}"

# Everything the seam was sent, captured verbatim. The driver writes one NDJSON
# user message and then closes stdin, so this drains to EOF.
cat > "$WORKDIR/seam-stdin.$N"

PAYLOAD_FILE="$WORKDIR/seam-payload.$N"
[ -f "$PAYLOAD_FILE" ] || PAYLOAD_FILE="$WORKDIR/seam-payload.json"

if [ -f "$PAYLOAD_FILE" ]; then
    PAYLOAD=$(cat "$PAYLOAD_FILE")
    printf '%s\n' "{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"session_id\":\"$SESSION\",\"num_turns\":1,\"terminal_reason\":\"completed\",\"permission_denials\":[],\"result\":\"ok\",\"structured_output\":$PAYLOAD,\"uuid\":\"22222222-2222-4222-8222-00000000000$N\"}"
else
    printf '%s\n' "{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"session_id\":\"$SESSION\",\"num_turns\":1,\"terminal_reason\":\"completed\",\"permission_denials\":[],\"result\":\"ok\",\"uuid\":\"22222222-2222-4222-8222-00000000000$N\"}"
fi

exit 0
