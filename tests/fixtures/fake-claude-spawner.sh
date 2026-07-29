#!/bin/sh
# A **grandchild-spawning** stand-in for the `claude` CLI.
#
# This is the only stand-in that can prove the thing D-14 actually claims. The
# real `claude` spawns Bash grandchildren, and a signal delivered to the direct
# child alone leaves them running — holding files, ports and quota a user cannot
# see and did not consent to (T-15-30). Killing the direct child therefore looks
# identical to killing the tree unless something in the tree outlives the leader
# and can be checked afterwards. That is what the grandchild here is for.
#
# Usage (as leading arguments, before the generated claude argv):
#
#     fake-claude-spawner.sh [ignored...]
#
# It emits its `system/init`, backgrounds a long-lived grandchild, and prints
# that grandchild's pid on a line the test reads off the event stream. The line
# uses a message `type` no CLI version emits, so the tolerant parser carries it
# as a forward-compatible unknown **with its raw text intact** — a known message
# type would be parsed into a model that has nowhere to put a pid.
#
# Every argument is ignored on purpose: the executor appends the real `claude`
# argv, and this stand-in must tolerate it exactly the way a forward-compatible
# CLI would.

set -u

# Drain stdin in the background so the executor's writer task never takes a
# broken pipe when the gate releases the prompt.
cat >/dev/null &

printf '{"type":"system","subtype":"init","session_id":"s","permissionMode":"dontAsk","claude_code_version":"2.1.220","apiKeySource":"none","capabilities":["interrupt_receipt_v1","interrupt_cancel_queued_v1","msg_lifecycle_v1"]}\n'

# The grandchild: a child of THIS script, which is itself the executor's child.
# Backgrounded from a non-interactive shell it inherits this process group, so a
# group-wide signal reaches it and a child-only signal does not.
sleep 600 &
GRANDCHILD=$!

printf '{"type":"grandchild_announcement","pid":%s}\n' "$GRANDCHILD"

# Stay alive as the group leader; the test tears the group down from outside.
wait "$GRANDCHILD"
exit 0
