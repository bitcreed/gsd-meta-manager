#!/bin/sh
# A transcript-replaying stand-in that first PLANTS an artifact in the project.
#
# Identical to `fake-claude.sh` in every respect except that its first act is to
# copy a caller-supplied body to a caller-supplied path. That is what makes a
# multi-iteration claim checkable without a real subscription: the driver's
# router reads project state between iterations, so a stand-in that changes
# nothing can only ever demonstrate the no-progress and command-repeat
# detectors. This one lets iteration one leave a fact behind for iteration two's
# router to read — which is how "the run reached its declared goal" is proved as
# an outcome rather than as a state the fixture was born in.
#
# Both halves are caller-supplied on purpose: this script decides nothing about
# what is planted, so the fixture that uses it states its own premise in Rust,
# where the assertion is.
#
# Usage (as leading arguments, before the generated claude argv):
#
#     fake-claude-planting.sh <artifact-path> <body-file> <transcript.ndjson> <exit-code> [ignored...]
#
# Every argument after the fourth is ignored on purpose: the executor appends
# the real `claude` argv, and this stand-in must tolerate it exactly the way a
# forward-compatible CLI would.

set -u

ARTIFACT="${1:?usage: fake-claude-planting.sh <artifact-path> <body-file> <transcript> <exit-code>}"
BODY="${2:?usage: fake-claude-planting.sh <artifact-path> <body-file> <transcript> <exit-code>}"
TRANSCRIPT="${3:?usage: fake-claude-planting.sh <artifact-path> <body-file> <transcript> <exit-code>}"
EXIT_CODE="${4:?usage: fake-claude-planting.sh <artifact-path> <body-file> <transcript> <exit-code>}"

# The plant is idempotent: a second iteration re-copies the same bytes, so the
# artifact a later router reads is the same one the first iteration wrote.
mkdir -p "$(dirname "$ARTIFACT")"
cp "$BODY" "$ARTIFACT"

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
