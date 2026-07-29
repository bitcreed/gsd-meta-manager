#!/bin/sh
# A transcript-replaying stand-in for the `claude` CLI.
#
# The executor is pointed at this script instead of the real binary, so the
# whole transport — argv construction, process-group spawn, the three pipe
# tasks, the first-init gate, framing and parsing — is exercised with zero
# subscription, network or quota dependency (D-24).
#
# Usage (as leading arguments, before the generated claude argv):
#
#     fake-claude.sh <transcript.ndjson> <exit-code> [ignored claude flags...]
#
# Every argument after the first two is ignored on purpose: the executor
# appends the real `claude` argv, and this stand-in must tolerate it exactly
# the way a forward-compatible CLI would.

set -u

TRANSCRIPT="${1:?usage: fake-claude.sh <transcript> <exit-code>}"
EXIT_CODE="${2:?usage: fake-claude.sh <transcript> <exit-code>}"

# Drain stdin in the background so the executor's writer task never takes a
# broken pipe. The real CLI reads stdin for the whole run; a stand-in that does
# not would make every send() look like a transport failure.
cat >/dev/null &
DRAIN_PID=$!

# Replay the transcript line by line with a small delay, so the reader task
# genuinely frames a stream rather than a single buffered blob.
while IFS= read -r line; do
    [ -z "$line" ] && continue
    printf '%s\n' "$line"
    sleep 0.01
done < "$TRANSCRIPT"

kill "$DRAIN_PID" 2>/dev/null
wait "$DRAIN_PID" 2>/dev/null

exit "$EXIT_CODE"
