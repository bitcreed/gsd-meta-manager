#!/usr/bin/env bash
# Launch the master driver headless against a scratch project.
# Usage: run.sh <project> <mode: loop|fresh-per-item> <sandbox: none|bwrap> [master-model]
set -euo pipefail
HERE="$(cd "$(dirname "$0")" && pwd)"
P="$(cd "$1" && pwd)"; MODE="$2"; SANDBOX="$3"; MODEL="${4:-sonnet}"
RUN_DIR="$P/.planning/meta-manager/drivers/spike-001"
PROMPT="$(sed -e "s#{RUN_DIR}#$RUN_DIR#g" -e "s#{PROJECT_ROOT}#$P#g" -e "s#{MODE}#$MODE#g" \
  "$HERE/../MASTER-PROMPT.md")
Begin now: run Startup, then the Loop, for RUN_DIR=$RUN_DIR."
CLAUDE=(env -u CLAUDECODE -u CLAUDE_CODE_SESSION_ID -u CLAUDE_CODE_BRIDGE_SESSION_ID -u CLAUDE_CODE_ENTRYPOINT -u CLAUDE_CODE_CHILD_SESSION -u CLAUDE_CODE_MESSAGING_SOCKET -u CLAUDE_CODE_MESSAGING_TOKEN -u CLAUDE_PID claude -p "$PROMPT" --model "$MODEL" --output-format json --max-budget-usd 2)
case "$SANDBOX" in
  none)
    # Host run: restricted tool allowlist instead of a sandbox.
    cd "$P"
    exec "${CLAUDE[@]}" --permission-mode acceptEdits \
      --allowedTools "Read Write Edit Glob Grep Agent Bash(git:*) Bash(cat:*) Bash(python3:*) Bash(grep:*) Bash(date:*) Bash(kill:*) Bash(rm:*) Bash(echo:*) Bash(ls:*) Bash(test:*) Bash([:*)"
    ;;
  bwrap)
    # Sandboxed run: read-only host, writable project + Claude state, private /tmp, own cgroup scope.
    exec systemd-run --user --scope --quiet --unit="${SCOPE_UNIT:-gsdmm-spike-$$}" -p MemoryMax=4G -p TasksMax=512 \
      bwrap --ro-bind / / --dev /dev --proc /proc --tmpfs /tmp \
        --bind "$P" "$P" --bind "$HOME/.claude" "$HOME/.claude" --bind "$HOME/.claude.json" "$HOME/.claude.json" \
        --unshare-pid --die-with-parent --chdir "$P" \
        "${CLAUDE[@]}" --permission-mode bypassPermissions
    ;;
esac
