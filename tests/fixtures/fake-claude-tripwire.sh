#!/bin/sh
# A stand-in for the `claude` CLI that is NEVER SUPPOSED TO RUN.
#
# Unlike `fake-claude.sh`, this script replays nothing and proves nothing by
# succeeding. Its entire purpose is to leave evidence behind if it is executed:
# the dry-run path is supposed to construct no executor and spawn no agent
# (D-23), and the **absence** of the file this script writes is what proves it.
#
# It exits 1 as well as writing, so a caller that both spawned it and ignored
# its output would still fail loudly.
#
# Usage (as leading arguments, before any generated claude argv):
#
#     fake-claude-tripwire.sh <evidence-path> [ignored claude flags...]
#
# Everything after the first argument is ignored on purpose, exactly as
# `fake-claude.sh` tolerates the executor's appended argv.
#
# A tripwire that has never been seen to fire proves nothing, so
# `tests/driver_dry_run.rs` runs this script directly as a positive control and
# asserts the evidence file appears.

set -u

EVIDENCE="${1:?usage: fake-claude-tripwire.sh <evidence-path>}"

printf 'tripwire fired: the agent program was executed\n' > "$EVIDENCE"

exit 1
