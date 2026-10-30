#!/bin/bash
# usage: [FILTER=1 PATHS=..] rtrial.sh NAME -- cmd...   (runs cmd in sandbox via run.sh, base url = bridge)
S=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/110e6e69-666b-4cc0-8133-142b5fa27a61/scratchpad/egress
NAME=$1; shift 2
D=$S/$NAME; rm -rf $D; mkdir -p $D/cfg $D/work; : > $D/proxy.log; [ -n "$SETTINGS" ] && printf "%s" "$SETTINGS" > $D/cfg/settings.json; SOCK=$XDG_RUNTIME_DIR/egp-$NAME.sock
python3 -I $S/rproxy.py $SOCK $D/proxy.log & PP=$!; sleep 0.5
bwrap --ro-bind / / --dev /dev --proc /proc --tmpfs /tmp --tmpfs /run --tmpfs $XDG_RUNTIME_DIR --ro-bind $S /tmp/e --bind $SOCK /tmp/proxy.sock \
  --bind $D $D --unshare-pid --unshare-net --unshare-ipc --unshare-uts --hostname sbx --new-session --die-with-parent \
  --unsetenv CLAUDECODE --unsetenv CLAUDE_CODE_MESSAGING_SOCKET --unsetenv CLAUDE_CODE_MESSAGING_TOKEN --unsetenv TMUX \
  --unsetenv CLAUDE_CODE_SESSION_ID --unsetenv CLAUDE_CODE_BRIDGE_SESSION_ID --unsetenv CLAUDE_CODE_CHILD_SESSION \
  --unsetenv CLAUDE_PID --unsetenv CLAUDE_CODE_ENTRYPOINT --unsetenv CLAUDE_CODE_EXECPATH --unsetenv CLAUDE_CODE_SESSION_ATTENDED \
  --setenv CLAUDE_CONFIG_DIR $D/cfg --setenv CLAUDE_CODE_OAUTH_TOKEN "sk-ant-oat01-DUMMY-placeholder-not-a-real-token" \
  --setenv ANTHROPIC_BASE_URL http://127.0.0.1:3128 --setenv NO_PROXY 127.0.0.1,localhost \
  --setenv DISABLE_TELEMETRY 1 --setenv DISABLE_ERROR_REPORTING 1 --setenv DISABLE_AUTOUPDATER 1 \
  --setenv CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC 1 --setenv ENABLE_CLAUDEAI_MCP_SERVERS false \
  --chdir $D/work -- /tmp/e/run2.sh "$@" > $D/out.txt 2> $D/err.txt < /dev/null
echo "rc=$?"; sleep 2; kill $PP; rm -f $SOCK
