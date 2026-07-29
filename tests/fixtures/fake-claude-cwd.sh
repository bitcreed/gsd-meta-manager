#!/bin/sh
# A transcript-replaying stand-in that also records its own working directory.
#
# Identical to `fake-claude.sh` in every respect except that its first act is to
# write `$PWD` to a sentinel path. That is what makes success criterion #4
# — "a live run against one project provably performs no write, spawn, or git
# operation against any other registered project" — checkable without a real
# subscription: the test reads the sentinel and asserts the recorded cwd is
# under project A and never under project B (plan 17-03).
#
# Usage (as leading arguments, before the generated claude argv):
#
#     fake-claude-cwd.sh <cwd-sentinel> <transcript.ndjson> <exit-code> [ignored claude flags...]
#
# Every argument after the third is ignored on purpose: the executor appends the
# real `claude` argv, and this stand-in must tolerate it exactly the way a
# forward-compatible CLI would.

set -u

SENTINEL="${1:?usage: fake-claude-cwd.sh <cwd-sentinel> <transcript> <exit-code>}"
TRANSCRIPT="${2:?usage: fake-claude-cwd.sh <cwd-sentinel> <transcript> <exit-code>}"
EXIT_CODE="${3:?usage: fake-claude-cwd.sh <cwd-sentinel> <transcript> <exit-code>}"

# First act, before a single stream byte: record where we were started.
printf '%s\n' "$PWD" > "$SENTINEL"

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
