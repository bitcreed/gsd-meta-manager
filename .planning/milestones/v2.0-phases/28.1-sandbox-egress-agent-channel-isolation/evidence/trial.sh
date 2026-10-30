#!/bin/bash
# usage: trial.sh NAME ALLOWLIST [extra env assignments...] -- claude args
S=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/110e6e69-666b-4cc0-8133-142b5fa27a61/scratchpad/egress
NAME=$1; ALLOWL=$2; shift 2
EXTRA=(); while [ "$1" != "--" ]; do EXTRA+=(--setenv "${1%%=*}" "${1#*=}"); shift; done; shift
D=$S/$NAME; rm -rf $D; mkdir -p $D/cfg $D/work; : > $D/proxy.log
ALLOW=$ALLOWL python3 -I $S/proxy.py $XDG_RUNTIME_DIR/egp-$NAME.sock $D/proxy.log & PP=$!
sleep 0.5
TOK=$(python3 -I -c "import json;print(json.load(open('/home/blk/.claude/.credentials.json'))['claudeAiOauth']['accessToken'])")
bwrap --ro-bind / / --dev /dev --proc /proc --tmpfs /tmp $PRE --ro-bind $S /tmp/e --bind $XDG_RUNTIME_DIR/egp-$NAME.sock /tmp/proxy.sock \
  --bind $D $D --unshare-pid --unshare-net --unshare-ipc --unshare-uts --hostname sbx --new-session --die-with-parent \
  --unsetenv CLAUDECODE --unsetenv CLAUDE_CODE_MESSAGING_SOCKET --unsetenv CLAUDE_CODE_MESSAGING_TOKEN \
  --unsetenv CLAUDE_CODE_SESSION_ID --unsetenv CLAUDE_CODE_BRIDGE_SESSION_ID --unsetenv CLAUDE_CODE_CHILD_SESSION \
  --unsetenv CLAUDE_PID --unsetenv CLAUDE_CODE_ENTRYPOINT --unsetenv CLAUDE_CODE_EXECPATH --unsetenv CLAUDE_CODE_SESSION_ATTENDED \
  --setenv CLAUDE_CONFIG_DIR $D/cfg --setenv CLAUDE_CODE_OAUTH_TOKEN "$TOK" "${EXTRA[@]}" \
  --chdir $D/work -- /tmp/e/run.sh $WRAP /home/blk/.local/bin/claude "$@" > $D/out.txt 2> $D/err.txt
echo "rc=$?"
unset TOK
sleep 3; kill $PP
