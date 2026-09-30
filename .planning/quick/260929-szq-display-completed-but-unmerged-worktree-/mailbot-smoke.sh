#!/usr/bin/env bash
# Quick 260929-szq, I-14: read-only real-world smoke of the unmerged-work
# display against mailbot, whose phase-05 SUMMARYs lived only in the locked
# worktree agent-a486395e992a58454 when this task was planned.
#
# Read-only by construction:
#   * the precondition is `test -e` / `ls` only;
#   * the app runs with a temp --config and temp XDG_* dirs (HOME untouched,
#     so the Claude adapter can read ~/.claude transcripts read-only);
#   * this script runs no git command against mailbot (the app's own reads go
#     through git_ops with --no-optional-locks);
#   * only navigation keys are sent; a trap kills the tmux session and
#     removes the temp dir.
#
# Run from the repo root. Exits 0 with `SKIP: <reason>` when mailbot's state
# no longer matches (then fixture (h) in tests/agents_unmerged.rs carries the
# proof). MAILBOT_DIR overrides the project path (e.g. a synthetic copy).
set -euo pipefail

MAILBOT_DIR="${MAILBOT_DIR:-$HOME/projects/python/mailbot}"
AGENT="agent-a486395e992a58454"
REF="@a486395"
HALF=$'◐'
SESSION="szq-smoke-$$"
BIN="target/release/gsd-meta-manager"

skip() {
    echo "SKIP: $*"
    exit 0
}

# (i) Precondition — read-only checks only.
command -v tmux >/dev/null 2>&1 || skip "tmux is not on PATH"
test -d "$MAILBOT_DIR/.planning" || skip "$MAILBOT_DIR has no .planning/"
# shellcheck disable=SC2086
if ! ls "$MAILBOT_DIR"/.claude/worktrees/$AGENT/.planning/phases/05-*/05-01-SUMMARY.md >/dev/null 2>&1; then
    skip "the $AGENT worktree no longer holds phase 05's SUMMARYs (mailbot state changed)"
fi
# shellcheck disable=SC2086
if ls "$MAILBOT_DIR"/.planning/phases/05-*/05-0?-SUMMARY.md >/dev/null 2>&1; then
    skip "main already holds phase 05's SUMMARYs (merged since planning)"
fi

# (ii) Build.
cargo build --release --quiet

# (iii) Temp config and XDG dirs.
T=$(mktemp -d)
cleanup() {
    tmux kill-session -t "$SESSION" >/dev/null 2>&1 || true
    rm -rf "$T"
}
trap cleanup EXIT
mkdir -p "$T/config" "$T/data" "$T/state" "$T/cache"
ABS_MAILBOT=$(cd "$MAILBOT_DIR" && pwd -P)
printf '{"version":2,"projects":{"mailbot":{"path":"%s","added":"2026-09-29T00:00:00Z"}},"preferences":{}}\n' \
    "$ABS_MAILBOT" >"$T/config.json"

# (iv) Launch headless.
tmux new-session -d -s "$SESSION" -x 200 -y 50 \
    "env XDG_CONFIG_HOME=$T/config XDG_DATA_HOME=$T/data XDG_STATE_HOME=$T/state XDG_CACHE_HOME=$T/cache $PWD/$BIN --config $T/config.json"

capture() { tmux capture-pane -p -t "$SESSION" >"$T/$1.txt"; }
send() { tmux send-keys -t "$SESSION" "$@"; }
FAILED=0
fail() {
    echo "FAIL: $1"
    echo "----- $2 -----"
    cat "$T/$2.txt"
    FAILED=1
}

# (v) Dashboard: the mailbot row gains ◐ within 30 s (the scan runs every ~5 s).
# Enter is sent as C-m (a carriage return): a bare `Enter` can arrive as a
# line feed, which the TUI reads as Ctrl+J and types `j` into the filter.
sleep 2
send / m a i l b o t C-m
dash_ok=0
for _ in $(seq 1 30); do
    sleep 1
    capture dash
    if grep 'mailbot' "$T/dash.txt" | grep -q "$HALF"; then
        dash_ok=1
        break
    fi
done
if [ "$dash_ok" = 1 ]; then
    echo "PASS: dashboard mailbot row: $(grep 'mailbot' "$T/dash.txt" | grep "$HALF" | head -1 | tr -s ' ')"
else
    fail "the dashboard mailbot row never showed $HALF" dash
fi

# (vi) Detail → 1:Roadmap: phase 05's row carries ◐.
send C-m
sleep 1
send 1
sleep 1
capture roadmap
if grep -E '(^|[^0-9.])0?5([^0-9.]|$)' "$T/roadmap.txt" | grep -q "$HALF"; then
    echo "PASS: roadmap phase 05: $(grep -E '(^|[^0-9.])0?5([^0-9.]|$)' "$T/roadmap.txt" | grep "$HALF" | head -1 | tr -s ' ')"
else
    fail "no Roadmap line for phase 05 carries $HALF" roadmap
fi

# (vii) 2:Phases: walk the phase cursor until the Waves pane shows phase 05.
send 2
sleep 1
found=0
for key in j j j j j j j j j j j j j j j j j j j j k k k k k k k k k k k k k k k k k k k k k k k k k k k k k k k k k k k k k k k k; do
    capture phases
    if grep -q -- "$REF" "$T/phases.txt"; then
        found=1
        break
    fi
    send "$key"
    sleep 0.3
done
if [ "$found" = 1 ] && grep -q 'unmerged' "$T/phases.txt"; then
    echo "PASS: Waves rows read unmerged with $REF"
    grep -- "$REF" "$T/phases.txt" | head -5 | tr -s ' '
    if grep -q $'▶ active' "$T/phases.txt"; then
        echo "INFO: ▶ active present (the worktree is locked by a live owner)"
    else
        echo "INFO: ▶ active absent (the worktree awaits merge)"
    fi
else
    fail "the Phases tab never showed 'unmerged' with $REF" phases
fi

# (viii) Quit.
send Escape
sleep 0.3
send q
sleep 0.5
if tmux has-session -t "$SESSION" 2>/dev/null; then
    send q
    sleep 0.5
fi

if [ "$FAILED" = 1 ]; then
    exit 1
fi
echo "PASS: mailbot smoke"
