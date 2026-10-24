#!/bin/bash
# usage: rcap.sh NAME PROMPT  -- sandboxed claude -p (haiku) through cap.py; managed settings via tmpfs-/etc fallback
R=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/110e6e69-666b-4cc0-8133-142b5fa27a61/scratchpad/r
E=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/110e6e69-666b-4cc0-8133-142b5fa27a61/scratchpad/egress
NAME=$1; PROMPT=$2
D=$R/$NAME; rm -rf $D; mkdir -p $D/cfg $D/work $D/managed; : > $D/proxy.log; : > $D/hook.log
printf '{"skipWebFetchPreflight":true,"disableClaudeAiConnectors":true}' > $D/cfg/settings.json
cat > $D/managed/managed-settings.json <<J
{"hooks":{"PreToolUse":[{"matcher":"*","hooks":[{"type":"command","command":"sh -c 'cat > /dev/null; echo managed-pre >> $D/hook.log'"}]}]},
 "permissions":{"deny":["Cron*","RemoteTrigger","SendMessage","ListAgents","ScheduleWakeup","DesignSync","mcp__claude_ai_*"]}}
J
SOCK=$XDG_RUNTIME_DIR/gsdmm-cap-$NAME.sock
python3 -I $R/cap.py $SOCK $D/proxy.log & PP=$!; sleep 0.5
args=(--ro-bind / / --dev /dev --proc /proc --tmpfs /etc)
for e in /etc/* /etc/.[!.]*; do [ -e "$e" ] || [ -L "$e" ] || continue; if [ -L "$e" ]; then args+=(--symlink "$(readlink "$e")" "$e"); else args+=(--ro-bind "$e" "$e"); fi; done
args+=(--ro-bind $D/managed /etc/claude-code --remount-ro /etc)
bwrap "${args[@]}" --tmpfs /tmp --tmpfs /run --tmpfs $XDG_RUNTIME_DIR --ro-bind $E /tmp/e --bind $SOCK /tmp/proxy.sock \
  --bind $D $D --unshare-pid --unshare-net --unshare-ipc --unshare-uts --hostname sbx --new-session --die-with-parent \
  --unsetenv CLAUDECODE --unsetenv CLAUDE_CODE_MESSAGING_SOCKET --unsetenv CLAUDE_CODE_MESSAGING_TOKEN --unsetenv TMUX --unsetenv TMUX_PANE \
  --unsetenv CLAUDE_CODE_SESSION_ID --unsetenv CLAUDE_CODE_BRIDGE_SESSION_ID --unsetenv CLAUDE_CODE_CHILD_SESSION \
  --unsetenv CLAUDE_PID --unsetenv CLAUDE_CODE_ENTRYPOINT --unsetenv CLAUDE_CODE_EXECPATH --unsetenv CLAUDE_CODE_SESSION_ATTENDED \
  --setenv CLAUDE_CONFIG_DIR $D/cfg --setenv CLAUDE_CODE_OAUTH_TOKEN "sk-ant-oat01-DUMMY-placeholder-not-a-real-token" \
  --setenv ANTHROPIC_BASE_URL http://127.0.0.1:3128 --setenv NO_PROXY 127.0.0.1,localhost \
  --setenv DISABLE_TELEMETRY 1 --setenv DISABLE_ERROR_REPORTING 1 --setenv DISABLE_AUTOUPDATER 1 \
  --setenv CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC 1 --setenv ENABLE_CLAUDEAI_MCP_SERVERS false \
  --chdir $D/work -- /tmp/e/run2.sh /home/blk/.local/bin/claude -p --model haiku --output-format stream-json --verbose \
     --permission-mode bypassPermissions --max-budget-usd 0.08 -- "$PROMPT" > $D/out.jsonl 2> $D/err.txt < /dev/null
echo "rc=$?"; sleep 1; kill $PP; rm -f $SOCK
