#!/bin/sh
# A transcript-replaying, argv/stdin/env-LOGGING stand-in for the `codex` CLI.
#
# It lets the driver's Codex path be exercised end to end — registry key to
# journal — with zero subscription, network or quota dependency, and it records
# exactly what the executor handed the child so a test can assert on it.
#
# Usage (as leading arguments, before the generated codex argv):
#
#     fake-codex.sh <transcript> <exit-code> <log-dir> [executor argv...]
#
#   transcript  a `codex exec --json` JSONL capture, printed verbatim to stdout
#               (see tests/fixtures/codex/README.md)
#   exit-code   the status this stand-in exits with after printing it
#   log-dir     an existing directory; three files are written into it:
#                 argv   every remaining argument, one per line
#                 stdin  what fd 0 is (`readlink /proc/$$/fd/0`), or `unknown`.
#                        Read from the link, never from the descriptor, so the
#                        stand-in can never block on an inherited stdin.
#                 env    the sorted NAMES — never the values — of every
#                        variable matching ^(CODEX_|CLAUDE|GSD_)
#
# Everything after the third argument is the executor's own argv and is logged,
# not interpreted: this stand-in replays the same transcript whatever it is
# asked, exactly as `fake-claude.sh` does.

set -u

TRANSCRIPT="${1?usage: fake-codex.sh <transcript> <exit-code> <log-dir>}"
EXIT_CODE="${2?usage: fake-codex.sh <transcript> <exit-code> <log-dir>}"
LOG_DIR="${3?usage: fake-codex.sh <transcript> <exit-code> <log-dir>}"
shift 3

: > "$LOG_DIR/argv"
for arg in "$@"; do
    printf '%s\n' "$arg" >> "$LOG_DIR/argv"
done

readlink "/proc/$$/fd/0" > "$LOG_DIR/stdin" 2>/dev/null || echo unknown > "$LOG_DIR/stdin"

env | sed -n 's/^\(CODEX_[^=]*\)=.*/\1/p; s/^\(CLAUDE[^=]*\)=.*/\1/p; s/^\(GSD_[^=]*\)=.*/\1/p' | sort > "$LOG_DIR/env"

cat "$TRANSCRIPT"
exit "$EXIT_CODE"
