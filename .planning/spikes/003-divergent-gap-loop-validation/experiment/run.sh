#!/usr/bin/env bash
# Spike 003 runner: one scratch project + host feed watcher + the unmodified spike-002 launcher.
# Usage: run.sh <mode>        mode: l7
#   l7 = scripted Haiku driver (cap test L7): feed-visibility probe, gap writes 1-4, allow at the
#        gap cap, deny at cap+1, kill at cap+2. (More modes are added by the next task.)
# Env:  SCRATCH_ROOT  default ${XDG_STATE_HOME:-$HOME/.local/state}/gsd-meta-manager/spike-003/<label>-<utc>
#       RESULTS_LABEL default = mode; the run id is s003-<label>
#       MODEL         overrides the mode table's model
#       RELAUNCH=1    reuse the label's recorded SCRATCH_ROOT and skip setup
# Exit: 0 launcher ran to completion; 1 launcher failed to start; 2 bad mode or scratch root inside a
#       git work tree; 5 host token too short; 6 secret gate hit; 7 make_variant.py --check drift.
# Stdout line 1 is SCRATCH_ROOT=<abs path>. On exit $SCRATCH_ROOT/DONE is touched.
# Results (derived files only, never *.jsonl / claude-config / transcripts) go to
# experiment/results/<label>/. Spike-002 files are executed in place and never edited.
set -euo pipefail
SELF="$(cd "$(dirname "$0")" && pwd)"
S3="$(cd "$SELF/.." && pwd)"
S2="$(cd "$S3/../002-launcher-caps-and-tool-frontmatter" && pwd)"
USAGE="Usage: run.sh <l7>   (env: SCRATCH_ROOT RESULTS_LABEL MODEL RELAUNCH)"

MODE="${1:-}"
STANDIN=0
case "$MODE" in
  l7)
    MODEL_D=haiku; PROMPT="$S3/experiment/cap-tests/L7-drift-gap-cap.md"; DIRECTION="$S3/templates/DIRECTION.run1.md"
    CAP_D=5; CAP_M=5; CAP_G=2; BUDGET=1 ;;
  *) echo "$USAGE" >&2; exit 2 ;;
esac
MODEL="${MODEL:-$MODEL_D}"
LABEL="${RESULTS_LABEL:-$MODE}"
RUN_ID="s003-$LABEL"
RES="$S3/experiment/results/$LABEL"
RELAUNCHES=0

# (a) scratch root and the git-work-tree guard
if [ "${RELAUNCH:-0}" = 1 ]; then
  [ -f "$RES/run-env.txt" ] || { echo "RELAUNCH=1 but $RES/run-env.txt is missing" >&2; exit 2; }
  SCRATCH_ROOT="$(sed -n 's/^SCRATCH_ROOT=//p' "$RES/run-env.txt" | head -1)"
else
  SCRATCH_ROOT="${SCRATCH_ROOT:-${XDG_STATE_HOME:-$HOME/.local/state}/gsd-meta-manager/spike-003/$LABEL-$(date -u +%Y%m%dT%H%M%SZ)}"
fi
echo "SCRATCH_ROOT=$SCRATCH_ROOT"
anc="$(dirname "$SCRATCH_ROOT")"
while [ ! -d "$anc" ] && [ "$anc" != "/" ]; do anc="$(dirname "$anc")"; done
if git -C "$anc" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "run.sh: SCRATCH_ROOT's parent ($anc) is inside a git work tree; refusing" >&2
  exit 2
fi
PROJECT="$SCRATCH_ROOT/project"; STATE_DIR="$SCRATCH_ROOT/state"; RUN_DIR="$PROJECT/.planning/meta-manager/drivers/$RUN_ID"

# (b) host token life
MINUTES_LEFT="$(python3 -I - <<'PY'
import json, os, time
c = json.load(open(os.path.expanduser("~/.claude/.credentials.json")))["claudeAiOauth"]
print(int((c["expiresAt"] / 1000.0 - time.time()) / 60))
PY
)"
if [ "$MINUTES_LEFT" -lt $((CAP_M + 15)) ]; then
  echo "token too short: $MINUTES_LEFT min"
  exit 5
fi

# environment scrub list: every CLAUDE*/ANTHROPIC* variable name in the current environment
SCRUB=(); SCRUB_NAMES=""
while IFS= read -r n; do
  case "$n" in CLAUDE*|ANTHROPIC*) SCRUB+=(-u "$n"); SCRUB_NAMES+="${SCRUB_NAMES:+,}$n" ;; esac
done < <(compgen -e)

WATCH_PID=""; STANDIN_PID=""
cleanup() {
  [ -n "$WATCH_PID" ] && kill "$WATCH_PID" 2>/dev/null || true
  [ -n "$STANDIN_PID" ] && kill "$STANDIN_PID" 2>/dev/null || true
  systemctl --user stop "gsdmm-$RUN_ID-*.scope" >/dev/null 2>&1 || true
  [ -d "$SCRATCH_ROOT" ] && touch "$SCRATCH_ROOT/DONE" || true
}
trap cleanup EXIT

mkdir -p "$SCRATCH_ROOT" "$RES"
if [ "${RELAUNCH:-0}" != 1 ]; then
  rm -rf "$RES"; mkdir -p "$RES"
  # (c) run-env.txt before launching
  {
    echo "MODE=$MODE"; echo "LABEL=$LABEL"; echo "RUN_ID=$RUN_ID"; echo "MODEL=$MODEL"
    echo "SCRATCH_ROOT=$SCRATCH_ROOT"; echo "PROJECT=$PROJECT"; echo "STATE_DIR=$STATE_DIR"; echo "RUN_DIR=$RUN_DIR"
    echo "PROMPT_FILE=$PROMPT"; echo "PROMPT_SHA256=$(sha256sum "$PROMPT" | cut -d' ' -f1)"
    echo "DIRECTION_FILE=$DIRECTION"
    echo "CAPS=dispatches=$CAP_D,minutes=$CAP_M,gap_rounds=$CAP_G"; echo "MAX_BUDGET_USD=$BUDGET"
    echo "CLAUDE_VERSION=$(claude --version 2>/dev/null | head -1)"
    echo "LAUNCHER_SHA256=$(sha256sum "$S2/tools/launcher.py" | cut -d' ' -f1)"
    echo "DRIVERCTL_SHA256=$(sha256sum "$S2/tools/driverctl.py" | cut -d' ' -f1)"
    echo "ENV_SCRUBBED=$SCRUB_NAMES"; echo "STARTED_UTC=$(date -u +%FT%TZ)"
  } > "$RES/run-env.txt"
  # (d) scratch project
  bash "$SELF/setup-scratch.sh" "$PROJECT" "$RUN_ID" "$DIRECTION" > "$SCRATCH_ROOT/setup.out"
fi

# (e) host feed watcher for the life of the launch
python3 -I "$S3/tools/drift.py" watch --project "$PROJECT" --log "$SCRATCH_ROOT/drift.log" \
  --snapshots "$SCRATCH_ROOT/verification" > "$SCRATCH_ROOT/watch.out" 2>&1 &
WATCH_PID=$!

# (f) launch under a CLAUDE*/ANTHROPIC*-scrubbed environment
mkdir -p "$STATE_DIR"
N=$(( $(find "$STATE_DIR" -maxdepth 1 -name "launch-*.jsonl" | wc -l) + 1 ))
set +e
env "${SCRUB[@]}" python3 "$S2/tools/launcher.py" run --project "$PROJECT" --run-id "$RUN_ID" --model "$MODEL" \
  --loop-mode loop --cap-dispatches "$CAP_D" --cap-minutes "$CAP_M" --cap-gap-rounds "$CAP_G" \
  --max-budget-usd "$BUDGET" --state-dir "$STATE_DIR" --prompt-file "$PROMPT" \
  > "$SCRATCH_ROOT/launcher-result-$N.json" 2> "$SCRATCH_ROOT/launcher-stderr-$N.txt"
LRC=$?
set -e
sleep 2    # let the watcher take its last poll before it is stopped

# (g) derived outputs only
cp "$RUN_DIR/LOG.md" "$RUN_DIR/state.json" "$RUN_DIR/DIRECTION.md" "$RES/" 2>/dev/null || true
cp "$SCRATCH_ROOT/drift.log" "$RES/" 2>/dev/null || true
[ -d "$SCRATCH_ROOT/verification" ] && cp -r "$SCRATCH_ROOT/verification" "$RES/verification" || true
cp "$SCRATCH_ROOT/launcher-result-$N.json" "$RES/" 2>/dev/null || true
[ -f "$SCRATCH_ROOT/standin.log" ] && cp "$SCRATCH_ROOT/standin.log" "$RES/" || true
git -C "$PROJECT" log --oneline --stat > "$RES/scratch-git-log.txt" 2>&1 || true
for f in "$STATE_DIR"/launch-*.jsonl; do
  [ -e "$f" ] || continue
  k="${f##*/launch-}"; k="${k%.jsonl}"
  python3 -I "$S2/experiment/cost_split.py" "$STATE_DIR/claude-config" "$f" > "$RES/cost-launch-$k.txt" 2>&1 || true
done

# (h) secret gate
if hits="$(grep -rElI 'sk-ant|oat01|accessToken|refreshToken' "$RES")" && [ -n "$hits" ]; then
  echo "run.sh: secret-pattern hit in results; nothing deleted:" >&2
  echo "$hits" >&2
  { echo "ENDED_UTC=$(date -u +%FT%TZ)"; echo "LAUNCHER_RC=$LRC"; echo "RELAUNCHES=$RELAUNCHES"; } >> "$RES/run-env.txt"
  exit 6
fi

# (i) close out
{ echo "ENDED_UTC=$(date -u +%FT%TZ)"; echo "LAUNCHER_RC=$LRC"; echo "RELAUNCHES=$RELAUNCHES"; } >> "$RES/run-env.txt"
[ "$LRC" -eq 0 ] || exit 1
exit 0
