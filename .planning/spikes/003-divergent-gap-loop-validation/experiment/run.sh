#!/usr/bin/env bash
# Spike 003 runner: one scratch project + host feed watcher + the spike-003 fork of the spike-002 launcher
# (every launch is `tools/launcher_s003.py run`, after `launcher_s003.py selftest` passes).
# Usage: run.sh <mode>        mode: l7 | l8 | smoke1 | smoke2 | run1 | run2
#   l7     scripted Haiku driver (cap test L7): feed-visibility probe, gap writes 1-4 spaced past the
#          coalescing window, allow at the gap cap, deny at cap+1, kill at cap+2.
#   l8     scripted Haiku driver (liveness test L8): early-ending worker, Stop-guard block, SendMessage resume.
#   smoke1 Haiku, prompt v0.4, dispatch cap 0 (first dispatch is denied): proves the driver starts.
#   smoke2 as smoke1 with the Run-2 prompt/DIRECTION and the kill stand-in on.
#   run1   Opus, prompt v0.4 (gap-round and divergence rules on), run-1 DIRECTION.
#   run2   Opus, Run-2 prompt/DIRECTION (rules removed), kill stand-in on.
#   Kill stand-in (smoke2/run2): after the first `dispatch-denied` it waits for the item to park (or
#   60 s), then until the gap-round coalescing window (gapcount.py window) + 5 s has passed since the last
#   gap-observed / gap-coalesced LOG line, and runs spike 002's gap-writer.sh once, writing one more
#   gaps_found. That write therefore counts as a new round (VALID-02 kill at L+2). It stands in for "a
#   worker already in flight writes a fourth verification" and is labelled in standin.log.
# Env:  SCRATCH_ROOT  default ${XDG_STATE_HOME:-$HOME/.local/state}/gsd-meta-manager/spike-003/<label>-<utc>
#       RESULTS_LABEL default = mode; the run id is s003-<label>. Must match [A-Za-z0-9._-]+ and not start
#                     with '.'; results/<label> must not exist unless RELAUNCH=1 (nothing is ever wiped)
#       MODEL         overrides the mode table's model
#       RELAUNCH=1    reuse the label's recorded SCRATCH_ROOT and skip setup (never into a Phase 26 label)
#       CAMPAIGN_CEILING_USD  default 60, may only lower the $60 spike-003 ceiling (D-07)
# Exit: 0 launcher ran to completion; 1 launcher failed, feed watcher dead at start, or a results copy
#       failed; 2 bad mode, bad or existing label, ceiling above 60, scratch root inside a git work tree;
#       5 host token too short; 6 secret gate hit; 7 kit integrity check failed (make_variant.py --check
#       drift or launcher_s003.py selftest failure); 8 campaign ceiling would be exceeded (or unmeasurable).
# Stdout line 1 is SCRATCH_ROOT=<abs path>. On exit $SCRATCH_ROOT/DONE is touched.
# Results (derived files only, never *.jsonl / claude-config / transcripts) go to
# experiment/results/<label>/. Spike-002 files are executed in place and never edited.
set -euo pipefail
SELF="$(cd "$(dirname "$0")" && pwd)"
S3="$(cd "$SELF/.." && pwd)"
S2="$(cd "$S3/../002-launcher-caps-and-tool-frontmatter" && pwd)"
USAGE="Usage: run.sh <l7|l8|smoke1|smoke2|run1|run2>   (env: SCRATCH_ROOT RESULTS_LABEL MODEL RELAUNCH CAMPAIGN_CEILING_USD)"
export PYTHONDONTWRITEBYTECODE=1

MODE="${1:-}"
STANDIN=0
case "$MODE" in
  l7)
    MODEL_D=haiku; PROMPT="$S3/experiment/cap-tests/L7-drift-gap-cap.md"; DIRECTION="$S3/templates/DIRECTION.run1.md"
    CAP_D=5; CAP_M=10; CAP_G=2; BUDGET=1 ;;
  l8)
    MODEL_D=haiku; PROMPT="$S3/experiment/cap-tests/L8-early-worker.md"; DIRECTION="$S3/templates/DIRECTION.run1.md"
    CAP_D=3; CAP_M=10; CAP_G=2; BUDGET=1 ;;
  smoke1)
    MODEL_D=haiku; PROMPT="$S3/MASTER-PROMPT.md"; DIRECTION="$S3/templates/DIRECTION.run1.md"
    CAP_D=0; CAP_M=5; CAP_G=2; BUDGET=1 ;;
  smoke2)
    MODEL_D=haiku; PROMPT="$S3/experiment/MASTER-PROMPT.run2.md"; DIRECTION="$S3/templates/DIRECTION.run2.md"
    CAP_D=0; CAP_M=5; CAP_G=2; BUDGET=1; STANDIN=1 ;;
  run1)
    MODEL_D=opus; PROMPT="$S3/MASTER-PROMPT.md"; DIRECTION="$S3/templates/DIRECTION.run1.md"
    CAP_D=10; CAP_M=90; CAP_G=2; BUDGET=15 ;;
  run2)
    MODEL_D=opus; PROMPT="$S3/experiment/MASTER-PROMPT.run2.md"; DIRECTION="$S3/templates/DIRECTION.run2.md"
    CAP_D=10; CAP_M=90; CAP_G=2; BUDGET=15; STANDIN=1 ;;
  *) echo "$USAGE" >&2; exit 2 ;;
esac
MODEL="${MODEL:-$MODEL_D}"

# ---- pre-launch gate chain: every gate runs before any scratch root, results dir, systemctl or credential read
# (1) The Run-2 files are asserted derivations of the v0.4 prompt; refuse to launch on drift.
if [ "$STANDIN" = 1 ] && ! python3 -I "$S3/tools/make_variant.py" --check >&2; then
  echo "run.sh: make_variant.py --check reports drift" >&2
  exit 7
fi
# (2) WR-09: the label feeds rm/mkdir/sed/systemctl globs; allow only a plain, non-hidden name.
LABEL="${RESULTS_LABEL-$MODE}"
case "$LABEL" in
  ''|.*|*[!A-Za-z0-9._-]*)
    echo "run.sh: bad RESULTS_LABEL '$LABEL' (need [A-Za-z0-9._-]+, not starting with '.')" >&2
    exit 2 ;;
esac
RUN_ID="s003-$LABEL"
RES="$S3/experiment/results/$LABEL"
# (3) D-07: the ceiling can only be lowered from $60.
CEIL="${CAMPAIGN_CEILING_USD:-60}"
if ! [[ "$CEIL" =~ ^[0-9]+(\.[0-9]+)?$ ]] || awk -v c="$CEIL" 'BEGIN { exit !(c > 60) }'; then
  echo "run.sh: CAMPAIGN_CEILING_USD '$CEIL' must be a number at most 60" >&2
  exit 2
fi
# (4) WR-09: committed evidence is never wiped. An existing results dir needs RELAUNCH=1, and the six
# Phase 26 results dirs are never relaunched into.
case "$LABEL" in
  l7|smoke1|smoke2|run1|run1b|run2)
    if [ "${RELAUNCH:-0}" = 1 ]; then
      echo "run.sh: RELAUNCH=1 into Phase 26 results '$LABEL' refused; choose a new RESULTS_LABEL" >&2
      exit 2
    fi ;;
esac
if [ -e "$RES" ] && [ "${RELAUNCH:-0}" != 1 ]; then
  echo "run.sh: experiment/results/$LABEL exists; choose a new RESULTS_LABEL (or RELAUNCH=1 to continue it)" >&2
  exit 2
fi
# (5) D-07 campaign gate: measured spike-003 spend (Phase 26 + 26.1, Haiku included) + this launch's
# budget must stay within the ceiling. Applies to RELAUNCH=1 too.
CB_RC=0
CB_OUT="$(python3 -I "$S3/tools/campaign_budget.py" --ceiling "$CEIL" --next "$BUDGET" 2>&1)" || CB_RC=$?
printf '%s\n' "$CB_OUT" >&2
if [ "$CB_RC" -ne 0 ]; then
  [ "$CB_RC" -eq 1 ] && echo "run.sh: campaign ceiling \$$CEIL would be exceeded by this launch (budget \$$BUDGET)" >&2 \
    || echo "run.sh: campaign spend could not be measured (campaign_budget.py rc=$CB_RC); refusing" >&2
  exit 8
fi
SPENT_ALL="$(printf '%s\n' "$CB_OUT" | tail -1 | sed -n 's/^SPENT_ALL=\([0-9.]*\) .*/\1/p')"
[ -n "$SPENT_ALL" ] || { echo "run.sh: campaign_budget.py printed no SPENT_ALL; refusing" >&2; exit 8; }
# (6) kit integrity: the fork must differ from the pinned spike-002 launcher only inside its marked hunks.
FORK_OUT="$(python3 -I "$S3/tools/launcher_s003.py" selftest 2>&1)" || {
  printf '%s\n' "$FORK_OUT" >&2
  echo "run.sh: launcher_s003.py selftest failed" >&2
  exit 7
}
FORK_DIFF_LINES="$(printf '%s\n' "$FORK_OUT" | sed -n 's/^FORK_DIFF_LINES=//p' | head -1)"
[ -n "$FORK_DIFF_LINES" ] || { echo "run.sh: launcher_s003.py selftest printed no FORK_DIFF_LINES" >&2; exit 7; }

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

# The watcher and the stand-in are stopped (and waited for) before results are copied (WR-08); the EXIT
# trap tolerates processes that are already gone.
WATCH_PID=""; STANDIN_PID=""
stop_bg() {
  if [ -n "$WATCH_PID" ]; then kill "$WATCH_PID" 2>/dev/null || true; wait "$WATCH_PID" 2>/dev/null || true; WATCH_PID=""; fi
  if [ -n "$STANDIN_PID" ]; then kill "$STANDIN_PID" 2>/dev/null || true; wait "$STANDIN_PID" 2>/dev/null || true; STANDIN_PID=""; fi
}
cleanup() {
  stop_bg
  systemctl --user stop "gsdmm-$RUN_ID-*.scope" >/dev/null 2>&1 || true
  [ -d "$SCRATCH_ROOT" ] && touch "$SCRATCH_ROOT/DONE" || true
}
trap cleanup EXIT

mkdir -p "$SCRATCH_ROOT" "$RES"
if [ "${RELAUNCH:-0}" != 1 ]; then
  # (c) run-env.txt before launching (the results dir is new: gate (4) refused an existing one)
  {
    echo "MODE=$MODE"; echo "LABEL=$LABEL"; echo "RUN_ID=$RUN_ID"; echo "MODEL=$MODEL"
    echo "SCRATCH_ROOT=$SCRATCH_ROOT"; echo "PROJECT=$PROJECT"; echo "STATE_DIR=$STATE_DIR"; echo "RUN_DIR=$RUN_DIR"
    echo "PROMPT_FILE=$PROMPT"; echo "PROMPT_SHA256=$(sha256sum "$PROMPT" | cut -d' ' -f1)"
    echo "DIRECTION_FILE=$DIRECTION"
    echo "CAPS=dispatches=$CAP_D,minutes=$CAP_M,gap_rounds=$CAP_G"; echo "MAX_BUDGET_USD=$BUDGET"
    echo "CLAUDE_VERSION=$(claude --version 2>/dev/null | head -1)"
    echo "LAUNCHER_SHA256=$(sha256sum "$S2/tools/launcher.py" | cut -d' ' -f1)"
    echo "DRIVERCTL_SHA256=$(sha256sum "$S2/tools/driverctl.py" | cut -d' ' -f1)"
    echo "FORK_SHA256=$(sha256sum "$S3/tools/launcher_s003.py" | cut -d' ' -f1)"
    echo "FORK_DIFF_LINES=$FORK_DIFF_LINES"
    echo "CAMPAIGN_SPENT_USD=$SPENT_ALL"
    echo "ENV_SCRUBBED=$SCRUB_NAMES"; echo "STARTED_UTC=$(date -u +%FT%TZ)"
  } > "$RES/run-env.txt"
  # (d) scratch project
  bash "$SELF/setup-scratch.sh" "$PROJECT" "$RUN_ID" "$DIRECTION" > "$SCRATCH_ROOT/setup.out"
else
  { echo "RELAUNCH_STARTED_UTC=$(date -u +%FT%TZ)"; echo "RELAUNCH_CAMPAIGN_SPENT_USD=$SPENT_ALL"; } >> "$RES/run-env.txt"
fi

# Kill stand-in (smoke2/run2): see header. Runs as a background subshell; never touches launcher.py.
# The extra gaps_found is written no sooner than <window> + 5 s after the last gap-observed/gap-coalesced
# LOG line so the coalescing launcher counts it as a new round instead of folding it into the last write.
last_gap_epoch() {   # epoch seconds (fractional) of the last gap-observed / gap-coalesced LOG line, or empty
  local ts
  ts="$(grep -E '^- [0-9T:.Z-]+ \| RUN \| gap-(observed|coalesced) \|' "$RUN_DIR/LOG.md" 2>/dev/null | tail -1 | sed -n 's/^- \([^ ]*\) |.*/\1/p' || true)"
  [ -n "$ts" ] && date -u -d "$ts" +%s.%N 2>/dev/null || true
}
standin_loop() {
  local log="$RUN_DIR/LOG.md" ln="" t0 trig=timeout win last now
  while [ -z "$ln" ]; do
    ln="$(grep -nF '| RUN | dispatch-denied |' "$log" 2>/dev/null | head -1 | cut -d: -f1 || true)"
    [ -n "$ln" ] || sleep 0.5
  done
  t0="$(date +%s)"
  while [ $(( $(date +%s) - t0 )) -lt 60 ]; do
    if tail -n +"$((ln + 1))" "$log" | grep -F '| I-01 | status | state=parked' >/dev/null; then trig=parked; break; fi
    sleep 0.5
  done
  win="$(python3 -I "$S3/tools/gapcount.py" window)"
  while :; do
    last="$(last_gap_epoch)"
    [ -n "$last" ] || break
    now="$(date +%s.%N)"
    if awk -v n="$now" -v l="$last" -v w="$win" 'BEGIN { exit !(n - l >= w + 5) }'; then break; fi
    sleep 0.5
  done
  bash "$S2/experiment/cap-tests/gap-writer.sh" "$PROJECT" 1 0 > /dev/null
  echo "$(date -u +%FT%TZ) trigger=$trig wrote gaps_found via gap-writer.sh" >> "$SCRATCH_ROOT/standin.log"
}

cred_expiry() {
  python3 -I -c 'import json,os;print(json.load(open(os.path.expanduser("~/.claude/.credentials.json")))["claudeAiOauth"]["expiresAt"])'
}

# (e) host feed watcher for the life of the launch; WR-07: a dead watcher aborts the launch
python3 -I "$S3/tools/drift.py" watch --project "$PROJECT" --log "$SCRATCH_ROOT/drift.log" \
  --snapshots "$SCRATCH_ROOT/verification" > "$SCRATCH_ROOT/watch.out" 2>&1 &
WATCH_PID=$!
sleep 1
if kill -0 "$WATCH_PID" 2>/dev/null; then
  echo "WATCHER_ALIVE=yes" >> "$RES/run-env.txt"
else
  echo "WATCHER_ALIVE=no" >> "$RES/run-env.txt"
  echo "run.sh: feed watcher is not alive one second after start; last lines of watch.out:" >&2
  tail -n 20 "$SCRATCH_ROOT/watch.out" >&2 || true
  WATCH_PID=""
  exit 1
fi
if [ "$STANDIN" = 1 ]; then standin_loop & STANDIN_PID=$!; fi

# (f) launch under a CLAUDE*/ANTHROPIC*-scrubbed environment; relaunch after an auth failure
mkdir -p "$STATE_DIR"
N=$(( $(find "$STATE_DIR" -maxdepth 1 -name "launch-*.jsonl" | wc -l) + 1 ))
while :; do
  EXP0="$(cred_expiry)"
  set +e
  env "${SCRUB[@]}" python3 "$S3/tools/launcher_s003.py" run --project "$PROJECT" --run-id "$RUN_ID" --model "$MODEL" \
    --loop-mode loop --cap-dispatches "$CAP_D" --cap-minutes "$CAP_M" --cap-gap-rounds "$CAP_G" \
    --max-budget-usd "$BUDGET" --state-dir "$STATE_DIR" --prompt-file "$PROMPT" \
    > "$SCRATCH_ROOT/launcher-result-$N.json" 2> "$SCRATCH_ROOT/launcher-stderr-$N.txt"
  LRC=$?
  set -e
  if [ "$RELAUNCHES" -lt 2 ] && [ -f "$STATE_DIR/launch-$N.jsonl" ] \
     && grep -qE 'OAuth token revoked|Failed to authenticate' "$STATE_DIR/launch-$N.jsonl" \
     && grep -q '"tripped": null' "$SCRATCH_ROOT/launcher-result-$N.json" 2>/dev/null; then
    waited=0
    while [ "$(cred_expiry)" = "$EXP0" ] && [ "$waited" -lt 1800 ]; do sleep 60; waited=$((waited + 60)); done
    if [ "$(cred_expiry)" != "$EXP0" ]; then
      RELAUNCHES=$((RELAUNCHES + 1)); N=$((N + 1)); continue
    fi
  fi
  break
done
sleep 2    # let the watcher take its last poll before it is stopped

# WR-08: stop and reap the watcher and the stand-in before anything is copied, so no snapshot or
# standin.log line can appear after (or during) the copy.
if kill -0 "$WATCH_PID" 2>/dev/null; then echo "WATCHER_ALIVE_AT_END=yes" >> "$RES/run-env.txt"
else echo "WATCHER_ALIVE_AT_END=no" >> "$RES/run-env.txt"; fi
stop_bg

# (g) derived outputs only. WR-01: `cp -a src/. dst/` never nests on RELAUNCH. IN-07: a missing LOG.md or
# state.json is reported and fails the run (after the secret gate), instead of an empty results dir with rc 0.
COPY_FAILED=0
for f in LOG.md state.json; do
  cp "$RUN_DIR/$f" "$RES/" 2>/dev/null || { echo "run.sh: results copy failed: $f" >&2; COPY_FAILED=1; }
done
cp "$RUN_DIR/DIRECTION.md" "$RES/" 2>/dev/null || true
cp "$SCRATCH_ROOT/drift.log" "$RES/" 2>/dev/null || true
if [ -d "$SCRATCH_ROOT/verification" ]; then
  mkdir -p "$RES/verification"
  cp -a "$SCRATCH_ROOT/verification/." "$RES/verification/" || true
fi
cp "$SCRATCH_ROOT"/launcher-result-*.json "$RES/" 2>/dev/null || true
[ -f "$SCRATCH_ROOT/standin.log" ] && cp "$SCRATCH_ROOT/standin.log" "$RES/" || true
[ -f "$STATE_DIR/claude-config/stop-guard.log" ] && cp "$STATE_DIR/claude-config/stop-guard.log" "$RES/stop-guard.log" || true
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
[ "$COPY_FAILED" -eq 0 ] || exit 1
exit 0
