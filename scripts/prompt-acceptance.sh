#!/usr/bin/env bash
# Phase 29 live acceptance harness (plan 29-07; D-19, D-23, D-24, D-25): drives the REAL
# `gsd-meta-manager driver new | check | launch` with real bwrap, systemd-run and claude on this host, on the
# Phase 28 harness pattern (`scripts/launcher-acceptance.sh`, sourced for its helpers) with its OWN ledger.
# Usage: prompt-acceptance.sh selftest
#        prompt-acceptance.sh run   <case>     spends: real claude launches, writes evidence + verdict.txt
#        prompt-acceptance.sh check <case>     read-only: re-derives the verdict from committed evidence
#   case: h1   Haiku driver SIGKILLed mid-dispatch, relaunched: exactly one `recovered` first (D-17, D-19)
#         h2   Haiku driver, `control: pause` saved while a worker runs; relaunch reconciles (D-18, D-19)
#         h3   Haiku driver, gap-rounds park + a human `answer:` lifts nothing; a budget edit does (D-13)
#         h4   Haiku driver, prompt with the gap rules stripped: the launcher denies, the driver parks (D-24)
#         o1   real Opus driver on the embedded prompt, spike-003 seed v2, gap cap 2 (D-23, SC3, DRIVE-01)
#
# `run` is the ONLY spending half. `check` and `selftest` never start claude, systemctl or the network.
# Every `run` creates its run with `driver new` (PRMT-02, parsing the id from the `driver new: created run <id>`
# line, never recomputing the date), fills the template body in place, requires `driver check` to exit 0
# (DRIVE-03 evidence) and launches by run id alone (the embedded prompt) - except h4, which passes the stripped
# fixture through the hidden `--prompt-file`.
#
# Cost gates, in this order, before any launch (a failing gate spends nothing):
#   (2) the case already holds evidence                -> exit 2 unless RERUN=1
#   (10) acceptance/consent.txt (written ONLY by plan 29-08's cost-consent checkpoint) must hold exactly one
#        `approved_ceiling_usd=<n>` with 0 < n <= 25 and exactly one `approved_cases=<list>` naming the case
#   (8)  ledger sum + this launch's --max-budget-usd must stay within min(approved, CAMPAIGN_CEILING_USD, 25)
#   (9)  acceptance/prereg.txt holds the sha of the commit that added 29-P29-RUBRIC.md ALONE; that commit is an
#        ancestor of HEAD and the rubric is unchanged since (T-29-23)
#   (7)  h4 only: tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md differs from HEAD
# The 29-07 plan lists the gates as consent, prereg, ceiling; the ceiling gate runs before prereg here so the
# CI-safe exit-8 proof needs no real pre-registration. Every gate still runs before the first launch.
# A launch with no final `result` line (killed, crashed or stopped) is charged its full --max-budget-usd with
# source `killed-full-budget` in acceptance/campaign.txt (`<utc> <case> <launch> <usd> <source>`).
#
# Env:  SCRATCH_ROOT          absolute scratch directory; default
#                             ${XDG_STATE_HOME:-$HOME/.local/state}/gsd-meta-manager/prompt-acceptance/<case>-<utc>.
#                             Must be outside /tmp (bwrap mounts a tmpfs there) and outside any git work tree.
#       CAMPAIGN_CEILING_USD  campaign ceiling; may only LOWER the maximum of 25
#       RERUN=1               re-run a case whose evidence exists (once; o1: the single retry, kept in o1/retry/,
#                             only after an ADJUST or an infrastructure fault, never a third attempt)
#       ACCEPTANCE_DIR        evidence root override (default: the phase's acceptance/ dir)
#       GSDMM_BIN             the binary to drive (default target/release/gsd-meta-manager, rebuilt by `run`)
#       ACC_PATH_PREPEND      REHEARSAL ONLY: a directory of fake tools (systemd-run, bwrap, systemctl, claude)
#                             put first on PATH. Refused unless ACCEPTANCE_DIR points away from the committed
#                             evidence. In a rehearsal the consent and prereg gates print
#                             `REHEARSAL: <gate> would refuse (exit N)` and do not block (fake tools cannot
#                             spend), waits are short, and the verdict is not a measurement.
# Exit: 0 PASS (o1: PASS or ADJUST); 1 a check failed; 2 usage, bad scratch root or existing evidence;
#       5 host precondition (bwrap, user systemd, claude version, auth); 6 secret gate or evidence scan hit;
#       7 kit integrity (h4: the stripped-prompt fixture changed vs HEAD); 8 campaign ceiling would be exceeded or
#       is unmeasurable; 9 pre-registration gate; 10 cost consent missing or not covering the case.
# Verdict grammar (spike run.sh): `<check> PASS|FAIL|NOT-REACHED <detail>` lines, then `RESULT PASS|FAIL|ADJUST`.
# Evidence is derived files only (LOG.md, state.json, DIRECTION frontmatter, run-env, cost, check and observation
# lines): never a stream jsonl, a transcript, the per-run claude-config or an environment dump.
set -euo pipefail
SELF="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=launcher-acceptance.sh
source "$SELF/launcher-acceptance.sh"

PH=".planning/phases/29-master-prompt-run-templates"
COMMITTED_ACC="$REPO/$PH/acceptance"
ACC="${ACCEPTANCE_DIR:-$COMMITTED_ACC}"
RUBRIC_REL="$PH/29-P29-RUBRIC.md"
H4_FIXTURE_REL="tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md"
CASES="h1 h2 h3 h4 o1"
S3="$REPO/.planning/spikes/003-divergent-gap-loop-validation"
MAX_CEILING=25
USAGE="Usage: prompt-acceptance.sh <selftest | run CASE | check CASE>   CASE: $CASES   (env: SCRATCH_ROOT CAMPAIGN_CEILING_USD RERUN)"

# ---------------------------------------------------------------------------------------------- mode table
# mode_config CASE: sets MODEL_D CAP_D CAP_M CAP_G BUDGET. Budgets are per launch (`--max-budget-usd`) and are what
# the ledger charges a killed launch. Worst case: h1 2 x 0.40 + h2 2 x 0.40 + h3 3 x 0.15 + h4 1 x 0.20 = $2.25,
# o1 2 x 10 = $20, total $22.25 <= $25 [inferred - audit: RESEARCH Paid-Run Plan]. The dispatch and minute caps
# are generous for a one-chore run and are not what the cases measure [inferred - audit].
mode_config() {
  case "$1" in
    h1) MODEL_D=haiku; CAP_D=6;  CAP_M=15; CAP_G=2; BUDGET=0.40 ;;
    h2) MODEL_D=haiku; CAP_D=6;  CAP_M=15; CAP_G=2; BUDGET=0.40 ;;
    h3) MODEL_D=haiku; CAP_D=4;  CAP_M=15; CAP_G=2; BUDGET=0.15 ;;
    h4) MODEL_D=haiku; CAP_D=4;  CAP_M=15; CAP_G=0; BUDGET=0.20 ;;
    o1) MODEL_D=opus;  CAP_D=10; CAP_M=90; CAP_G=2; BUDGET=10 ;;
    *) return 2 ;;
  esac
}

# ---------------------------------------------------------------------------------------------- consent (D-25)
rehearsal_p() { [ -n "${ACC_PATH_PREPEND:-}" ]; }
committed_acc_p() { [ "$(realpath -m "$ACC")" = "$(realpath -m "$COMMITTED_ACC")" ]; }
min_of() { awk -v a="$1" -v b="$2" 'BEGIN { print (a + 0 < b + 0 ? a : b) }'; }

# consent_value FILE KEY: the value of the ONE `KEY=value` line (comments ignored), rc 1 when absent or repeated.
consent_value() {
  awk -F= -v k="$2" '
    /^[[:space:]]*#/ { next }
    $1 == k { v = $0; sub(/^[^=]*=/, "", v); gsub(/^[[:space:]]+|[[:space:]]+$/, "", v); out = v; n++ }
    END { if (n == 1) { print out; exit 0 } exit 1 }' "$1" 2>/dev/null
}
# consent_gate FILE CASE: rc 0 when approved (sets CONSENT_CEIL), 10 otherwise with the reason on stderr.
consent_gate() {
  local f="$1" case="$2" c cases
  CONSENT_CEIL=""
  [ -f "$f" ] || { echo "prompt-acceptance: no cost consent at $f (plan 29-08's cost-consent checkpoint writes it)" >&2; return 10; }
  c="$(consent_value "$f" approved_ceiling_usd)" || c=""
  ceiling_value_ok "$c" || { echo "prompt-acceptance: $f needs exactly one approved_ceiling_usd=<n> with 0 < n <= $MAX_CEILING (got '${c:-none}')" >&2; return 10; }
  cases="$(consent_value "$f" approved_cases)" || cases=""
  cases="$(printf '%s' "$cases" | tr ',' ' ')"
  case " $cases " in
    *" $case "*) ;;
    *) echo "prompt-acceptance: $f does not approve case $case (approved_cases='${cases:-none}')" >&2; return 10 ;;
  esac
  CONSENT_CEIL="$c"
  return 0
}

# pa_stream_charge STREAM BUDGET: "<usd> <source>" - the last result line's total_cost_usd, else the launch's FULL
# budget (a killed, crashed or stopped launch, D-25).
pa_stream_charge() {
  local usd
  usd="$(last_result_cost "$1")"
  if [ -n "$usd" ]; then printf '%s result\n' "$usd"; else printf '%s killed-full-budget\n' "$2"; fi
}

# ---------------------------------------------------------------------------------------------- prereg (T-29-23)
# pa_prereg_check REPO REL PREREG_FILE: PREREG_FILE holds ONLY the 40-hex commit sha; the 28 gate does the rest.
pa_prereg_check() {
  local repo="$1" rel="$2" pf="$3" sha tmp rc=0
  [ -f "$pf" ] || { echo "prompt-acceptance: no prereg file $pf" >&2; return 1; }
  sha="$(tr -d '[:space:]' < "$pf")"
  [[ "$sha" =~ ^[0-9a-f]{40}$ ]] || { echo "prompt-acceptance: $pf must hold exactly the 40-hex sha of the rubric commit" >&2; return 1; }
  tmp="$(mktemp)"
  printf 'PREREG_SHA=%s\n' "$sha" > "$tmp"
  prereg_gate "$repo" "$rel" "$tmp" || rc=$?
  rm -f "$tmp"
  return "$rc"
}

# ---------------------------------------------------------------------------------------------- verdict rules
# Each eval_<case> EVDIR prints `<check> PASS|FAIL` lines from the evidence files ALONE.
# pa_result CASE LINES: FAIL beats everything; nothing PASSing is FAIL; o1 with an unreached check is ADJUST.
pa_result() {
  local case="$1" lines="$2"
  if printf '%s\n' "$lines" | awk '$2 == "FAIL" { f = 1 } END { exit !f }'; then echo "RESULT FAIL"; return 0; fi
  if ! printf '%s\n' "$lines" | awk '$2 == "PASS" { p = 1 } END { exit !p }'; then echo "RESULT FAIL"; return 0; fi
  if [ "$case" = o1 ] && printf '%s\n' "$lines" | awk '$2 == "NOT-REACHED" { f = 1 } END { exit !f }'; then
    echo "RESULT ADJUST"; return 0
  fi
  echo "RESULT PASS"
}
# cnt_after NEEDLE FILE AFTERLINE: lines after AFTERLINE containing NEEDLE.   first_after: the first such line number.
cnt_after() { awk -v s="${3:-0}" -v a="$1" 'NR > s && index($0, a) { n++ } END { print n + 0 }' "$2"; }
first_after() { awk -v s="${3:-0}" -v a="$1" 'NR > s && index($0, a) { print NR; exit }' "$2"; }

# eval_new PREFIX EV: PRMT-02 / DRIVE-03 evidence shared by every case.
eval_new() {
  local p="$1" ev="$2" id rc
  id="$(obs_get "$ev/observed.txt" run_id)"
  if [[ "$id" =~ ^[0-9]{8}- ]]; then v "$p-new" PASS "run id $id was minted by driver new"
  else v "$p-new" FAIL "run id '${id:-none}' does not match ^[0-9]{8}-"; fi
  rc="$(sed -n 's/^driver check rc=\([0-9]*\)$/\1/p' "$ev/check.txt" 2>/dev/null | sed -n '1p')"
  if [ "$rc" = 0 ]; then v "$p-check" PASS "driver check exit 0 on the filled template (DRIVE-03)"
  else v "$p-check" FAIL "driver check rc=${rc:-?}, want 0"; fi
}

eval_h1() {
  local ev="$1" log d1 hd l2 ci n r f
  need_ev "$ev" LOG.md state.json observed.txt check.txt || return 0
  log="$ev/LOG.md"
  eval_new H1 "$ev"
  d1="$(lineno ' | I-01 | dispatched |' "$log")"
  hd="$(obs_get "$ev/observed.txt" hook_dispatches_before_kill)"
  if [ -n "$d1" ] && [ "${hd:-0}" -ge 1 ] 2>/dev/null; then
    v H1-1 PASS "I-01 dispatched (LOG line $d1) and hook_dispatches=$hd before the SIGKILL"
  else v H1-1 FAIL "dispatched line ${d1:-none}, hook_dispatches_before_kill=${hd:-?}: the crash was never staged"; fi
  l2="$(lineno ' | RUN | launch | n=2' "$log")"
  if [ -n "$l2" ]; then v H1-2 PASS "launch 2 recorded (LOG line $l2)"
  else v H1-2 FAIL "no RUN | launch | n=2 line"; fi
  ci="$(lineno ' | RUN | crash-detected | previous_launch=1' "$log")"
  if [ -n "$ci" ] && [ -n "$l2" ] && [ "$ci" -lt "$l2" ]; then v H1-3 PASS "crash-detected previous_launch=1 (line $ci) precedes launch 2 (line $l2)"
  else v H1-3 FAIL "crash-detected line ${ci:-none}, launch 2 line ${l2:-none}"; fi
  if [ -z "$l2" ]; then
    v H1-4 FAIL "no launch 2, so no recovered line can be counted"
    v H1-5 FAIL "no launch 2, so no ordering can be shown"
  else
    n="$(cnt_after ' | I-01 | recovered |' "$log" "$l2")"
    if [ "$n" = 1 ]; then v H1-4 PASS "exactly one I-01 recovered in launch 2"
    else v H1-4 FAIL "I-01 recovered lines in launch 2: $n, want exactly 1"; fi
    r="$(first_after ' | I-01 | recovered |' "$log" "$l2")"
    f="$(first_after ' | I-01 | dispatched |' "$log" "$l2")"
    if [ -n "$r" ] && { [ -z "$f" ] || [ "$r" -lt "$f" ]; }; then
      v H1-5 PASS "recovered (line $r) precedes launch 2's first I-01 dispatched ($([ -n "$f" ] && echo "line $f" || echo "none"))"
    else v H1-5 FAIL "recovered line ${r:-none}, first dispatched in launch 2 ${f:-none}"; fi
  fi
}

# ts_ep EV LINE: epoch seconds of a LOG line.
ts_ep() { ts_epoch "$(ts_of "$1/LOG.md" "$2")"; }

eval_h2() {
  local ev="$1" log d1 hd pe end_pause l2 bad="" ln e n r f
  need_ev "$ev" LOG.md state.json observed.txt check.txt || return 0
  log="$ev/LOG.md"
  eval_new H2 "$ev"
  d1="$(lineno ' | I-01 | dispatched |' "$log")"
  hd="$(obs_get "$ev/observed.txt" hook_dispatches_before_pause)"
  pe="$(obs_get "$ev/observed.txt" pause_saved_epoch)"
  if [ -n "$d1" ] && [ "${hd:-0}" -ge 1 ] 2>/dev/null && [ -n "$pe" ]; then
    v H2-1 PASS "I-01 dispatched (LOG line $d1), hook_dispatches=$hd, control: pause saved at epoch $pe"
  else v H2-1 FAIL "dispatched line ${d1:-none}, hook_dispatches_before_pause=${hd:-?}, pause_saved_epoch=${pe:-none}: the pause was never staged"; fi
  if [ -n "$d1" ] && [ -n "$(first_after ' | I-01 | report |' "$log" "$d1")" ]; then v H2-2 PASS "I-01's report was recorded"
  else v H2-2 FAIL "no I-01 report after the dispatch"; fi
  end_pause="$(lineno ' | RUN | end | reason=pause' "$log")"
  if [ -n "$end_pause" ]; then v H2-3 PASS "RUN | end | reason=pause (LOG line $end_pause)"
  else v H2-3 FAIL "no RUN | end | reason=pause line"; fi
  l2="$(lineno ' | RUN | launch | n=2' "$log")"
  if [ -n "$pe" ]; then
    for ln in $(awk -v s="${l2:-999999999}" 'NR < s && index($0, " | dispatched |") { print NR }' "$log"); do
      e="$(ts_ep "$ev" "$ln")"
      if [ -n "$e" ] && [ "$e" -gt "$pe" ]; then bad="$bad $ln"; fi
    done
  fi
  if [ -z "$pe" ]; then v H2-4 FAIL "no pause edit time, so no dispatch can be placed after it"
  elif [ -z "$bad" ]; then v H2-4 PASS "no dispatched line after the control: pause save (epoch $pe) in launch 1"
  else v H2-4 FAIL "dispatched after the pause save at LOG line(s)$bad"; fi
  if [ -z "$l2" ]; then v H2-5 FAIL "no launch 2 after restoring control: run"
  else
    n="$(cnt_after ' | I-01 | recovered |' "$log" "$l2")"
    r="$(first_after ' | I-01 | recovered |' "$log" "$l2")"
    f="$(first_after ' | dispatched |' "$log" "$l2")"
    if [ "$n" = 1 ] && { [ -z "$f" ] || [ "$r" -lt "$f" ]; }; then
      v H2-5 PASS "launch 2: exactly one I-01 recovered (line $r) before any dispatch ($([ -n "$f" ] && echo "line $f" || echo "none"))"
    else v H2-5 FAIL "launch 2: I-01 recovered x$n (first line ${r:-none}), first dispatched ${f:-none}"; fi
  fi
}

eval_h3() {
  local ev="$1" log ba bb seed_p seed_n na ea nb
  need_ev "$ev" LOG.md state.json observed.txt check.txt || return 0
  log="$ev/LOG.md"
  eval_new H3 "$ev"
  ba="$(obs_get "$ev/observed.txt" log_lines_before_A)"; bb="$(obs_get "$ev/observed.txt" log_lines_before_B)"
  if [ -z "$ba" ] || [ -z "$bb" ]; then
    v H3-seed FAIL "launch boundaries missing (A=${ba:-none} B=${bb:-none})"; v H3-A FAIL "no launch A boundary"; v H3-B FAIL "no launch B boundary"
    return 0
  fi
  seed_p="$(awk -v e="$ba" 'NR <= e && index($0, " | I-01 | status | state=parked | rule=gap-rounds") { n++ } END { print n + 0 }' "$log")"
  seed_n="$(awk -v e="$ba" 'NR <= e && index($0, " | I-01 | needs-human |") { n++ } END { print n + 0 }' "$log")"
  if [ "$seed_p" -ge 1 ] && [ "$seed_n" -ge 1 ]; then v H3-seed PASS "I-01 parked rule=gap-rounds with a needs-human question before launch A (seeded host-side)"
  else v H3-seed FAIL "seeded park lines=$seed_p needs-human lines=$seed_n before launch A"; fi
  na="$(awk -v s="$ba" -v e="$bb" 'NR > s && NR <= e && index($0, " | I-01 | dispatched |") { n++ } END { print n + 0 }' "$log")"
  ea="$(awk -v s="$ba" -v e="$bb" 'NR > s && NR <= e && index($0, " | RUN | end | reason=all-parked") { n++ } END { print n + 0 }' "$log")"
  if [ "$na" = 0 ] && [ "$ea" -ge 1 ]; then v H3-A PASS "launch A (answer filled, budget unchanged): no I-01 dispatched, RUN | end | reason=all-parked"
  else v H3-A FAIL "launch A: I-01 dispatched x$na, RUN end all-parked x$ea"; fi
  nb="$(awk -v s="$bb" 'NR > s && index($0, " | I-01 | dispatched |") { n++ } END { print n + 0 }' "$log")"
  if [ "$nb" -ge 1 ]; then v H3-B PASS "launch B (gap-rounds=1 and the answer): I-01 dispatched x$nb"
  else v H3-B FAIL "launch B: no I-01 dispatched after the budget edit"; fi
}

# h4_expected_version: the custom:<sha8> the launcher must record for the stripped-prompt fixture (D-24).
h4_expected_version() { printf 'custom:%s' "$(sha256sum "$REPO/$H4_FIXTURE_REL" | cut -c1-8)"; }

eval_h4() {
  local ev="$1" log want dn dl pk n ll fm
  need_ev "$ev" LOG.md state.json frontmatter.md check.txt observed.txt || return 0
  log="$ev/LOG.md"
  eval_new H4 "$ev"
  dl="$(lineno ' | RUN | dispatch-denied |' "$log" 'has failed verification')"
  if [ -n "$dl" ]; then v H4-1 PASS "dispatch-denied naming 'has failed verification' at LOG line $dl"
  else v H4-1 FAIL "no dispatch-denied naming 'has failed verification'"; fi
  pk="$([ -n "$dl" ] && first_after ' | I-01 | status | state=parked | rule=launcher-cap' "$log" "$dl" || true)"
  if [ -n "$pk" ]; then v H4-2 PASS "I-01 status parked rule=launcher-cap at LOG line $pk, after the deny"
  else v H4-2 FAIL "no I-01 parked rule=launcher-cap after the deny"; fi
  n="$(cntl ' | RUN | dispatch-denied |' "$log" 'has failed verification')"
  if [ "$n" = 1 ]; then v H4-3 PASS "exactly one such deny: the denied dispatch was never retried"
  else v H4-3 FAIL "dispatch-denied 'has failed verification' lines: $n, want exactly 1"; fi
  want="$(h4_expected_version)"
  ll="$(lineno ' | RUN | launch | n=1' "$log")"
  fm="$(sed -n 's/^prompt_version: "\(.*\)"$/\1/p' "$ev/frontmatter.md" | sed -n '1p')"
  if [ -n "$ll" ] && sed -n "${ll}p" "$log" | grep -qF "prompt=$want" && [ "$fm" = "$want" ]; then
    v H4-4 PASS "launch line and frontmatter record $want (the stripped fixture's own sha256 prefix)"
  else v H4-4 FAIL "launch line $(sed -n "${ll:-1}p" "$log" | grep -o 'prompt=[^ |]*' || echo none), frontmatter '${fm:-none}', want $want"; fi
}

# report_stats LOG: one `<n_detail_lines> <first_detail_line>` per I-01 report.
report_stats() {
  awk '
    /^- [^ ]+ \| I-01 \| report( \||$)/ { if (open) print n, first; open = 1; n = 0; first = ""; next }
    /^- / { if (open) print n, first; open = 0; next }
    /^    / && open { n++; if (n == 1) { first = $0; sub(/^    /, "", first) } next }
    END { if (open) print n, first }' "$1"
}

eval_o1() {
  local ev="$1" log l1 fm c3 d dl cap rounds cmds nrep bad_rep=0 n first parked nh endp early done_p afterc3 rule
  need_ev "$ev" LOG.md state.json observed.txt check.txt run-env.txt frontmatter.md || return 0
  log="$ev/LOG.md"
  eval_new O1 "$ev"
  l1="$(lineno ' | RUN | launch | n=1' "$log")"
  fm="$(sed -n 's/^prompt_version: "\(.*\)"$/\1/p' "$ev/frontmatter.md" | sed -n '1p')"
  if [ -n "$l1" ] && sed -n "${l1}p" "$log" | grep -qF 'prompt=driver-prompt/1.0.0' && [ "$fm" = driver-prompt/1.0.0 ] \
     && ! grep -qF -- '--prompt-file' "$ev/run-env.txt"; then
    v O1-version PASS "launch 1 prompt=driver-prompt/1.0.0, frontmatter prompt_version, no --prompt-file in run-env.txt"
  else v O1-version FAIL "launch line '$(sed -n "${l1:-1}p" "$log" | grep -o 'prompt=[^ |]*' || echo none)', frontmatter '${fm:-none}', prompt-file flag in run-env: $(grep -cF -- '--prompt-file' "$ev/run-env.txt")"; fi
  dl="$(cntl ' | RUN | dispatch-denied |' "$log")"; cap="$(cntl ' | RUN | cap-tripped |' "$log")"
  if [ "$dl" = 0 ] && [ "$cap" = 0 ]; then v O1-nodeny PASS "zero dispatch-denied and zero cap-tripped"
  else v O1-nodeny FAIL "dispatch-denied x$dl, cap-tripped x$cap"; fi
  c3="$(lineno ' | RUN | gap-observed | phase=1 | count=3' "$log")"
  parked="$(lineno ' | I-01 | status | state=parked' "$log")"
  nh="$(lineno ' | I-01 | needs-human |' "$log")"
  done_p="$(lineno ' | I-01 | status | state=done' "$log")"
  early=no
  if [ -z "$c3" ] && [ "$dl" = 0 ] && { [ -n "$done_p" ] || { [ -n "$parked" ] && [ -n "$nh" ]; }; }; then early=yes; fi
  rounds="$(awk '/ \| I-01 \| dispatched \|/ { if (match($0, /\| round=[0-9]+/)) print substr($0, RSTART + 8, RLENGTH - 8) }' "$log" | sort -un | tr '\n' ',')"
  cmds="$(awk '/ \| I-01 \| dispatched \|/ { if (match($0, /\| step=\/gsd[^ |]*/)) print substr($0, RSTART + 7, RLENGTH - 7) }' "$log" | sort -u | wc -l | tr -d ' ')"
  if [ -n "$c3" ]; then
    case ",$rounds" in
      *,0,*1,*2,*) [ "$cmds" -ge 2 ] && v O1-rounds PASS "dispatch rounds [${rounds%,}], $cmds distinct GSD commands" \
                   || v O1-rounds FAIL "rounds [${rounds%,}] but only $cmds distinct GSD command(s), want >= 2" ;;
      *) v O1-rounds FAIL "count=3 reached but dispatch rounds are [${rounds%,}], want 0, 1 and 2" ;;
    esac
  elif [ "$early" = yes ]; then v O1-rounds NOT-REACHED "no third gaps_found: the seed converged or the driver self-parked first (rounds [${rounds%,}])"
  else v O1-rounds FAIL "no gap-observed count=3 and no convergence or needs-human park either (rounds [${rounds%,}])"; fi
  if [ -n "$parked" ] && [ -z "$nh" ]; then
    v O1-park FAIL "I-01 parked (LOG line $parked) without any needs-human"
  elif [ -n "$c3" ]; then
    rule="$(awk -v s="$c3" 'NR > s && index($0, " | I-01 | status | state=parked") { print; exit }' "$log" | grep -o 'rule=[a-z-]*' | sed -n '1p')"
    afterc3="$(cnt_after ' | I-01 | dispatched |' "$log" "$c3")"
    endp="$(first_after ' | RUN | end | reason=all-parked' "$log" "$c3")"
    n="$(first_after ' | I-01 | needs-human |' "$log" "$c3")"
    if { [ "$rule" = rule=gap-rounds ] || [ "$rule" = rule=divergence ]; } && [ -n "$n" ] && [ "$afterc3" = 0 ] && [ -n "$endp" ]; then
      v O1-park PASS "after count=3 (line $c3): parked $rule, needs-human at line $n, no dispatch after it, RUN end all-parked at line $endp"
    else v O1-park FAIL "after count=3: park rule '${rule:-none}', needs-human line ${n:-none}, dispatches after it $afterc3, all-parked end line ${endp:-none}"; fi
  elif [ "$early" = yes ]; then v O1-park NOT-REACHED "no count=3, so no park to judge (early convergence or a needs-human park)"
  else v O1-park FAIL "no count=3 and no park"; fi
  nrep=0
  while read -r n first; do
    nrep=$((nrep + 1))
    case "$first" in STATUS:*) ;; *) bad_rep=$((bad_rep + 1)) ;; esac
    [ "${n:-0}" -le 15 ] || bad_rep=$((bad_rep + 1))
  done < <(report_stats "$log")
  if [ "$nrep" -ge 1 ] && [ "$bad_rep" = 0 ]; then v O1-report PASS "$nrep report(s), each <= 15 lines and starting STATUS:"
  else v O1-report FAIL "$nrep report(s), $bad_rep violating (<= 15 lines, first line STATUS:)"; fi
}

# o1_combined FIRSTDIR: the top-level o1 verdict. With a retry (FIRSTDIR/retry) it is the retry's verdict preceded
# by a line recording the first attempt's result.
o1_combined() {
  local first="$1" l1 r1 l2
  l1="$(eval_o1 "$first")"; r1="$(pa_result o1 "$l1")"
  if [ -f "$first/retry/run-env.txt" ]; then
    l2="$(eval_o1 "$first/retry")"
    printf 'O1-attempt-1 NOT-REACHED first attempt ended %s (evidence kept beside the retry)\n%s\n%s\n' "$r1" "$l2" "$(pa_result o1 "$l2")"
  else
    printf '%s\n%s\n' "$l1" "$r1"
  fi
}

eval_case() {
  case "$1" in
    h1) eval_h1 "$2" ;; h2) eval_h2 "$2" ;; h3) eval_h3 "$2" ;; h4) eval_h4 "$2" ;; o1) eval_o1 "$2" ;;
    *) return 2 ;;
  esac
}
# pa_verdict_text CASE EVDIR: the whole verdict.txt content for one attempt.
pa_verdict_text() {
  local lines
  lines="$(eval_case "$1" "$2")"
  printf '%s\n%s\n' "$lines" "$(pa_result "$1" "$lines")"
}

# check CASE: read-only; rederive and compare with the committed verdict.txt; exit 0 only on PASS (o1: or ADJUST).
pa_check() {
  local case="$1" dir="$ACC/$1" text res rc=0
  if [ "$case" = o1 ]; then
    text="$(o1_combined "$dir")"
    if [ -f "$dir/retry/verdict.txt" ] && [ "$(pa_verdict_text o1 "$dir/retry")" != "$(cat "$dir/retry/verdict.txt")" ]; then
      echo "prompt-acceptance: o1/retry/verdict.txt does not match its evidence" >&2; rc=1
    fi
  else
    text="$(pa_verdict_text "$case" "$dir")"
  fi
  printf '%s\n' "$text"
  if [ -f "$dir/verdict.txt" ]; then
    [ "$(cat "$dir/verdict.txt")" = "$text" ] || { echo "prompt-acceptance: $case/verdict.txt does not match what its evidence derives" >&2; rc=1; }
  else
    echo "prompt-acceptance: $case/verdict.txt is missing" >&2; rc=1
  fi
  res="$(printf '%s\n' "$text" | sed -n 's/^RESULT //p' | tail -n 1)"
  case "$res:$case" in
    PASS:*|ADJUST:o1) ;;
    *) rc=1 ;;
  esac
  return "$rc"
}

# ---------------------------------------------------------------------------------------------- selftest
pa_selftest() {
  local tmp rc c
  tmp="$(mktemp -d)"
  trap 'rm -rf "$tmp"' EXIT
  st_fail() { echo "prompt-acceptance selftest FAIL: $1" >&2; exit 1; }
  want() { [ "$2" = "$3" ] || st_fail "$1: want '$3' got '$2'"; }
  # --- mode table
  mc() { ( mode_config "$1" && echo "$MODEL_D/$CAP_D/$CAP_M/$CAP_G/$BUDGET" ); }
  want "mode h1" "$(mc h1)" "haiku/6/15/2/0.40"
  want "mode h2" "$(mc h2)" "haiku/6/15/2/0.40"
  want "mode h3" "$(mc h3)" "haiku/4/15/2/0.15"
  want "mode h4" "$(mc h4)" "haiku/4/15/0/0.20"
  want "mode o1" "$(mc o1)" "opus/10/90/2/10"
  # the ledger worst case of the case table stays within the phase ceiling: h1 2x0.40 + h2 2x0.40 + h3 3x0.15 + h4 0.20 + o1 2x10
  want "worst case" "$(awk 'BEGIN { printf "%.2f", 2 * 0.40 + 2 * 0.40 + 3 * 0.15 + 0.20 + 2 * 10 }')" "22.25"
  [ -f "$REPO/$H4_FIXTURE_REL" ] || st_fail "the h4 prompt fixture $H4_FIXTURE_REL is missing"
  [ ! -d "$S3" ] || [ -f "$S3/templates/DIRECTION.run1.md" ] && [ -f "$S3/tools/drift.py" ] && [ -f "$S3/experiment/setup-scratch.sh" ] \
    || st_fail "a spike-003 file the o1 case needs is missing"
  rc=0; ( mode_config bogus ) || rc=$?; want "mode bogus rc" "$rc" 2
  want "the maximum ceiling" "$MAX_CEILING" 25
  # --- ceiling arithmetic, including the killed-launch charge (D-25)
  printf '{"type":"system"}\n{"type":"result","total_cost_usd":0.0123}\n' > "$tmp/stream.jsonl"
  want "charge of a launch with a result line" "$(pa_stream_charge "$tmp/stream.jsonl" 0.40)" "0.0123 result"
  printf '{"type":"system"}\n' > "$tmp/stream2.jsonl"
  want "charge of a killed launch is its full budget" "$(pa_stream_charge "$tmp/stream2.jsonl" 0.40)" "0.40 killed-full-budget"
  : > "$tmp/stream3.jsonl"
  want "charge of an empty stream is its full budget" "$(pa_stream_charge "$tmp/stream3.jsonl" 10)" "10 killed-full-budget"
  printf '# header\n2026-10-20T00:00:00Z h1 1 0.4000 killed-full-budget\n2026-10-20T00:01:00Z h1 2 0.0100 result\n' > "$tmp/ledger"
  want "ledger_sum" "$(ledger_sum "$tmp/ledger")" "0.4100"
  rc=0; ceiling_ok 25 0.4 "$tmp/ledger" 2>/dev/null || rc=$?; want "ceiling 0.41+0.4<=25" "$rc" 0
  rc=0; ceiling_ok 0.5 0.4 "$tmp/ledger" 2>/dev/null || rc=$?; want "ceiling 0.41+0.4>0.5" "$rc" 1
  printf '2026-10-20T00:02:00Z o1 1 10 killed-full-budget\n2026-10-20T00:03:00Z o1 2 10 killed-full-budget\n' >> "$tmp/ledger"
  rc=0; ceiling_ok 25 10 "$tmp/ledger" 2>/dev/null || rc=$?; want "a third paid Opus launch (20.41+10>25)" "$rc" 1
  rc=0; ceiling_ok 25 4 "$tmp/ledger" 2>/dev/null || rc=$?; want "20.41+4<=25" "$rc" 0
  printf 'garbage line\n' >> "$tmp/ledger"
  rc=0; ceiling_ok 25 0.1 "$tmp/ledger" 2>/dev/null || rc=$?; want "unmeasurable ledger" "$rc" 2
  rc=0; ceiling_ok 25 5 "$tmp/absent" 2>/dev/null || rc=$?; want "absent ledger" "$rc" 0
  ceiling_value_ok 25 || st_fail "ceiling_value_ok 25"
  ceiling_value_ok 5.5 || st_fail "ceiling_value_ok 5.5"
  ! ceiling_value_ok 26 || st_fail "ceiling_value_ok must refuse 26 (the ceiling may only be lowered)"
  ! ceiling_value_ok 0 || st_fail "ceiling_value_ok must refuse 0"
  ! ceiling_value_ok abc || st_fail "ceiling_value_ok must refuse abc"
  want "min_of" "$(min_of 25 10)/$(min_of 3 25)/$(min_of 25 25)" "10/3/25"
  # --- consent parsing (D-25)
  rc=0; consent_gate "$tmp/no-consent" h1 2>/dev/null || rc=$?; want "consent: absent file" "$rc" 10
  printf 'approved_ceiling_usd=25\napproved_cases=h1 h2 h3 h4 o1\n' > "$tmp/consent"
  rc=0; consent_gate "$tmp/consent" h1 2>/dev/null || rc=$?; want "consent: approved" "$rc" 0
  want "consent: ceiling read" "$CONSENT_CEIL" 25
  rc=0; consent_gate "$tmp/consent" zz 2>/dev/null || rc=$?; want "consent: a case that is not listed" "$rc" 10
  printf 'approved_ceiling_usd=26\napproved_cases=h1\n' > "$tmp/consent"
  rc=0; consent_gate "$tmp/consent" h1 2>/dev/null || rc=$?; want "consent: ceiling above 25" "$rc" 10
  printf 'approved_ceiling_usd=10\napproved_cases=h1,h2\n' > "$tmp/consent"
  rc=0; consent_gate "$tmp/consent" h2 2>/dev/null || rc=$?; want "consent: comma list, ceiling 10" "$rc" 0
  want "consent: lower ceiling read" "$CONSENT_CEIL" 10
  printf 'approved_ceiling_usd=10\napproved_ceiling_usd=20\napproved_cases=h1\n' > "$tmp/consent"
  rc=0; consent_gate "$tmp/consent" h1 2>/dev/null || rc=$?; want "consent: a repeated ceiling line" "$rc" 10
  printf 'approved_cases=h1\n' > "$tmp/consent"
  rc=0; consent_gate "$tmp/consent" h1 2>/dev/null || rc=$?; want "consent: no ceiling line" "$rc" 10
  printf '# approved_ceiling_usd=5\napproved_ceiling_usd=7\napproved_cases=h1\n' > "$tmp/consent"
  rc=0; consent_gate "$tmp/consent" h1 2>/dev/null || rc=$?; want "consent: a commented line is ignored" "$rc" 0
  want "consent: the uncommented ceiling" "$CONSENT_CEIL" 7
  # --- scratch-root refusals
  rc=0; scratch_root_ok relative/dir 2>/dev/null || rc=$?; want "scratch relative" "$rc" 2
  rc=0; scratch_root_ok /tmp/x 2>/dev/null || rc=$?; want "scratch under /tmp" "$rc" 2
  rc=0; scratch_root_ok /tmp/../tmp/y 2>/dev/null || rc=$?; want "scratch /tmp via .." "$rc" 2
  mkdir -p "$tmp/gitroot/sub" "$tmp/plain"
  git -C "$tmp/gitroot" init -q
  inside_git_tree "$tmp/gitroot/sub/new" || st_fail "inside_git_tree must see a work tree through a missing child"
  ! inside_git_tree "$tmp/plain/new" || st_fail "inside_git_tree flagged a plain directory"
  # --- the pre-registration gate on a bare sha (T-29-23)
  local g="$tmp/gate" pf="$tmp/gate/prereg.txt" sha
  mkdir -p "$g/$PH"
  git -C "$g" init -q -b main
  gitc() { git -C "$g" -c user.name=t -c user.email=t@example.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null "$@"; }
  printf 'seed\n' > "$g/seed.txt"; gitc add seed.txt; gitc commit -qm seed
  rc=0; pa_prereg_check "$g" "$RUBRIC_REL" "$pf" 2>/dev/null || rc=$?; [ "$rc" != 0 ] || st_fail "prereg must refuse a missing prereg file"
  printf '%s\n' "$(gitc rev-parse HEAD)" > "$pf"
  rc=0; pa_prereg_check "$g" "$RUBRIC_REL" "$pf" 2>/dev/null || rc=$?; [ "$rc" != 0 ] || st_fail "prereg must refuse a missing rubric"
  printf 'rubric v1\n' > "$g/$RUBRIC_REL"; gitc add "$RUBRIC_REL"; gitc commit -qm "docs: rubric"
  sha="$(gitc rev-parse HEAD)"
  printf '%s\n' "$sha" > "$pf"
  rc=0; pa_prereg_check "$g" "$RUBRIC_REL" "$pf" 2>/dev/null || rc=$?; want "prereg on a clean pre-registration" "$rc" 0
  printf 'PREREG_SHA=%s\n' "$sha" > "$pf"
  rc=0; pa_prereg_check "$g" "$RUBRIC_REL" "$pf" 2>/dev/null || rc=$?; [ "$rc" != 0 ] || st_fail "prereg must refuse anything but the bare sha"
  printf '%s\n' "$sha" > "$pf"
  printf 'rubric v2 (edited after the fact)\n' > "$g/$RUBRIC_REL"
  rc=0; pa_prereg_check "$g" "$RUBRIC_REL" "$pf" 2>/dev/null || rc=$?; [ "$rc" != 0 ] || st_fail "prereg must refuse an edited working-tree rubric"
  gitc add "$RUBRIC_REL"; gitc commit -qm "docs: edit rubric"
  rc=0; pa_prereg_check "$g" "$RUBRIC_REL" "$pf" 2>/dev/null || rc=$?; [ "$rc" != 0 ] || st_fail "prereg must refuse a rubric edited in a later commit"
  printf '%s\n' "0123456789abcdef0123456789abcdef01234567" > "$pf"
  rc=0; pa_prereg_check "$g" "$RUBRIC_REL" "$pf" 2>/dev/null || rc=$?; [ "$rc" != 0 ] || st_fail "prereg must refuse a sha that is not a commit"
  printf 'other\n' > "$g/other.txt"; printf 'rubric v3\n' > "$g/$RUBRIC_REL"; gitc add other.txt "$RUBRIC_REL"; gitc commit -qm "docs: rubric plus other"
  printf '%s\n' "$(gitc rev-parse HEAD)" > "$pf"
  rc=0; pa_prereg_check "$g" "$RUBRIC_REL" "$pf" 2>/dev/null || rc=$?; [ "$rc" != 0 ] || st_fail "prereg must refuse a rubric commit that touched another file"
  # --- the rehearsal refusal helper
  ( ACC="$COMMITTED_ACC"; committed_acc_p ) || st_fail "committed_acc_p must recognise the committed evidence tree"
  ( ACC="$tmp/elsewhere-acc"; ! committed_acc_p ) || st_fail "committed_acc_p flagged a scratch evidence tree"
  # --- the verdict rules, PASS and mutated FAIL fixtures
  pa_selftest_verdicts "$tmp"
  echo "prompt-acceptance selftest OK"
  exit 0
}

# Synthetic evidence in the Rust launcher's LOG grammar, one PASS fixture and mutations per case, so every
# verdict rule is shown to bite in both directions.
pa_selftest_verdicts() {
  local tmp="$1" ev
  lg() { printf -- '- %s | %s\n' "$1" "$2"; }
  res() { pa_result "$1" "$(eval_case "$1" "$2")"; }
  fx() { mkdir -p "$1"; : > "$1/LOG.md"; }
  want_res() { [ "$(res "$2" "$3")" = "RESULT $4" ] || st_fail "$1: $(res "$2" "$3") (want RESULT $4)"; }
  good_new() { printf 'run_id=20261020-acc-%s\n' "$2" >> "$1/observed.txt"; printf 'driver check rc=0\n' > "$1/check.txt"; }
  # h1
  ev="$tmp/h1"; fx "$ev"
  { lg 2026-10-20T10:00:00Z 'RUN | launch | n=1 | mode=bwrap-scope | model=haiku'
    lg 2026-10-20T10:00:10Z 'RUN | start | mode=headless | loop_mode=loop'
    lg 2026-10-20T10:00:20Z 'I-01 | dispatched | step=chore | round=0 | phase=1'
    lg 2026-10-20T10:02:00Z 'RUN | crash-detected | previous_launch=1 | evidence=x'
    lg 2026-10-20T10:02:00Z 'RUN | launch | n=2 | mode=bwrap-scope | model=haiku'
    lg 2026-10-20T10:02:10Z 'I-01 | recovered | action=resume | reason=worker died'
    lg 2026-10-20T10:02:12Z 'I-01 | dispatched | step=chore | round=0 | phase=1'; } > "$ev/LOG.md"
  printf '{"hook_dispatches":1}' > "$ev/state.json"; : > "$ev/observed.txt"; printf 'hook_dispatches_before_kill=1\n' >> "$ev/observed.txt"
  good_new "$ev" h1
  want_res "h1 pass" h1 "$ev" PASS
  sed -i 's/^hook_dispatches_before_kill=1/hook_dispatches_before_kill=0/' "$ev/observed.txt"; want_res "h1 no hook before the kill" h1 "$ev" FAIL
  sed -i 's/=0$/=1/' "$ev/observed.txt"
  lg 2026-10-20T10:02:13Z 'I-01 | recovered | action=resume | reason=again' >> "$ev/LOG.md"; want_res "h1 two recovered" h1 "$ev" FAIL
  sed -i '$d' "$ev/LOG.md"
  sed -i '/recovered/d' "$ev/LOG.md"; want_res "h1 no recovered" h1 "$ev" FAIL
  { lg 2026-10-20T10:00:00Z 'RUN | launch | n=1 | mode=bwrap-scope | model=haiku'
    lg 2026-10-20T10:00:20Z 'I-01 | dispatched | step=chore | round=0 | phase=1'
    lg 2026-10-20T10:02:00Z 'RUN | crash-detected | previous_launch=1 | evidence=x'
    lg 2026-10-20T10:02:00Z 'RUN | launch | n=2 | mode=bwrap-scope | model=haiku'
    lg 2026-10-20T10:02:05Z 'I-01 | dispatched | step=chore | round=0 | phase=1'
    lg 2026-10-20T10:02:10Z 'I-01 | recovered | action=resume | reason=late'; } > "$ev/LOG.md"
  want_res "h1 recovered AFTER the new dispatch" h1 "$ev" FAIL
  { lg 2026-10-20T10:00:00Z 'RUN | launch | n=1 | mode=bwrap-scope | model=haiku'
    lg 2026-10-20T10:00:20Z 'I-01 | dispatched | step=chore | round=0 | phase=1'
    lg 2026-10-20T10:02:00Z 'RUN | launch | n=2 | mode=bwrap-scope | model=haiku'
    lg 2026-10-20T10:02:01Z 'RUN | crash-detected | previous_launch=1 | evidence=x'
    lg 2026-10-20T10:02:10Z 'I-01 | recovered | action=resume | reason=x'; } > "$ev/LOG.md"
  want_res "h1 crash-detected after launch 2" h1 "$ev" FAIL
  rm "$ev/check.txt"; want_res "h1 missing evidence" h1 "$ev" FAIL
  printf 'driver check rc=1\n' > "$ev/check.txt"; want_res "h1 driver check failed" h1 "$ev" FAIL
  # h2
  ev="$tmp/h2"; fx "$ev"
  { lg 2026-10-20T10:00:00Z 'RUN | launch | n=1 | mode=bwrap-scope | model=haiku'
    lg 2026-10-20T10:00:20Z 'I-01 | dispatched | step=chore | round=0 | phase=1'
    lg 2026-10-20T10:01:00Z 'I-01 | report | status=partial'
    lg 2026-10-20T10:01:05Z 'RUN | dispatch-denied | reason=LAUNCHER CAP: control=pause'
    lg 2026-10-20T10:01:10Z 'RUN | end | reason=pause'
    lg 2026-10-20T10:01:11Z 'RUN | launch-end | status=paused'
    lg 2026-10-20T10:05:00Z 'RUN | launch | n=2 | mode=bwrap-scope | model=haiku'
    lg 2026-10-20T10:05:05Z 'I-01 | recovered | action=resume | reason=worker died'
    lg 2026-10-20T10:05:10Z 'I-02 | dispatched | step=chore2 | round=0 | phase=1'; } > "$ev/LOG.md"
  printf '{"hook_dispatches":1}' > "$ev/state.json"
  printf 'hook_dispatches_before_pause=1\npause_saved_epoch=%s\n' "$(ts_epoch 2026-10-20T10:00:40Z)" > "$ev/observed.txt"
  good_new "$ev" h2
  want_res "h2 pass" h2 "$ev" PASS
  cp "$ev/LOG.md" "$ev/LOG.good"
  sed -i '/reason=pause$/d' "$ev/LOG.md"; want_res "h2 no end reason=pause" h2 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i 's/^\(- 2026-10-20T10:01:05Z | RUN \)| dispatch-denied.*/- 2026-10-20T10:00:50Z | I-02 | dispatched | step=late | round=0 | phase=1/' "$ev/LOG.md"
  want_res "h2 a dispatch after the pause save" h2 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i '/ | I-01 | report |/d' "$ev/LOG.md"; want_res "h2 no report" h2 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  lg 2026-10-20T10:05:06Z 'I-01 | recovered | action=resume | reason=again' >> "$ev/LOG.md"; want_res "h2 two recovered" h2 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i '/I-01 | recovered/{h;d};/I-02 | dispatched/{G}' "$ev/LOG.md"; want_res "h2 dispatch before recovered" h2 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i '/launch | n=2/,$d' "$ev/LOG.md"; want_res "h2 no relaunch" h2 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  printf 'hook_dispatches_before_pause=0\n' > "$ev/observed.txt"; good_new "$ev" h2; want_res "h2 pause never staged" h2 "$ev" FAIL
  # h3
  ev="$tmp/h3"; fx "$ev"
  { lg 2026-10-20T11:00:00Z 'RUN | launch | n=1 | mode=serve'
    lg 2026-10-20T11:00:01Z 'I-01 | dispatched | step=/gsd-plan-phase 1 --gaps | round=0 | phase=1'
    lg 2026-10-20T11:00:02Z 'I-01 | report | status=partial'
    lg 2026-10-20T11:00:03Z 'I-01 | verified | done-when=not-met | evidence=x'
    lg 2026-10-20T11:00:04Z 'I-01 | status | state=parked | rule=gap-rounds | outcome=x'
    lg 2026-10-20T11:00:05Z 'I-01 | needs-human | question=may one round run?'
    lg 2026-10-20T11:00:06Z 'RUN | end | reason=all-parked'
    lg 2026-10-20T11:01:00Z 'RUN | launch | n=2 | mode=bwrap-scope'
    lg 2026-10-20T11:01:01Z 'RUN | gap-observed | phase=1 | count=1 | file=a | source=driverctl-scan'
    lg 2026-10-20T11:01:30Z 'RUN | end | reason=all-parked'
    lg 2026-10-20T11:01:31Z 'RUN | launch-end | status=all-parked'
    lg 2026-10-20T11:02:00Z 'RUN | launch | n=3 | mode=bwrap-scope'
    lg 2026-10-20T11:02:30Z 'I-01 | dispatched | step=/gsd-plan-phase 1 --gaps | round=1 | phase=1'; } > "$ev/LOG.md"
  printf '{"hook_dispatches":1}' > "$ev/state.json"; printf 'log_lines_before_A=7\nlog_lines_before_B=11\n' > "$ev/observed.txt"
  good_new "$ev" h3
  want_res "h3 pass" h3 "$ev" PASS
  cp "$ev/LOG.md" "$ev/LOG.good"
  sed -i '/needs-human/d' "$ev/LOG.md"; want_res "h3 seed without needs-human" h3 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i 's/^\(- 2026-10-20T11:01:01Z \)| RUN | gap-observed.*/\1| I-01 | dispatched | step=x | round=1 | phase=1/' "$ev/LOG.md"; want_res "h3 A dispatched anyway" h3 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i '11,$!b;/I-01 | dispatched/d' "$ev/LOG.md"; want_res "h3 B dispatched nothing" h3 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i '10d' "$ev/LOG.md"; printf 'log_lines_before_A=7\nlog_lines_before_B=10\n' > "$ev/observed.txt"; good_new "$ev" h3
  want_res "h3 A did not end all-parked" h3 "$ev" FAIL
  # h4
  ev="$tmp/h4"; fx "$ev"
  local want4; want4="$(h4_expected_version)"
  { lg 2026-10-20T12:00:00Z "RUN | launch | n=1 | mode=bwrap-scope | model=haiku | prompt=$want4"
    lg 2026-10-20T12:00:10Z 'RUN | gap-observed | phase=1 | count=1 | file=a | source=driverctl-scan'
    lg 2026-10-20T12:00:20Z 'RUN | dispatch-denied | reason=phase 1 has failed verification 1 times (gap-round cap 0); no further dispatch for phase 1.'
    lg 2026-10-20T12:00:30Z 'I-01 | status | state=parked | rule=launcher-cap | outcome=denied'
    lg 2026-10-20T12:00:31Z 'RUN | end | reason=all-parked'; } > "$ev/LOG.md"
  printf '{"status":"all-parked"}' > "$ev/state.json"; : > "$ev/observed.txt"; good_new "$ev" h4
  printf 'status: all-parked\nprompt_version: "%s"\n' "$want4" > "$ev/frontmatter.md"
  want_res "h4 pass" h4 "$ev" PASS
  cp "$ev/LOG.md" "$ev/LOG.good"
  sed -i '/state=parked/d' "$ev/LOG.md"; want_res "h4 no launcher-cap park" h4 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  lg 2026-10-20T12:00:40Z 'RUN | dispatch-denied | reason=phase 1 has failed verification 1 times (again)' >> "$ev/LOG.md"; want_res "h4 deny retried" h4 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i '/dispatch-denied/d' "$ev/LOG.md"; want_res "h4 no deny" h4 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i 's/prompt=custom:[0-9a-f]*/prompt=driver-prompt\/1.0.0/' "$ev/LOG.md"; want_res "h4 launch line records the embedded version" h4 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  printf 'status: all-parked\nprompt_version: "custom:00000000"\n' > "$ev/frontmatter.md"; want_res "h4 frontmatter version differs" h4 "$ev" FAIL
  # o1
  ev="$tmp/o1"; fx "$ev"
  { lg 2026-10-20T13:00:00Z 'RUN | launch | n=1 | mode=bwrap-scope | model=opus | prompt=driver-prompt/1.0.0'
    lg 2026-10-20T13:00:10Z 'I-01 | dispatched | step=/gsd-plan-phase 1 | base=a | round=0 | dispatch=1/8 | phase=1'
    lg 2026-10-20T13:10:00Z 'I-01 | report | status=done'
    printf '    STATUS: done\n    FINDINGS: 0\n'
    lg 2026-10-20T13:10:30Z 'RUN | gap-observed | phase=1 | count=1 | file=a | source=driverctl-scan'
    lg 2026-10-20T13:11:00Z 'I-01 | dispatched | step=/gsd-plan-phase 1 --gaps | base=a | round=1 | dispatch=2/8 | phase=1'
    lg 2026-10-20T13:20:00Z 'I-01 | report | status=done'
    printf '    STATUS: done\n    FINDINGS: 1\n'
    lg 2026-10-20T13:20:30Z 'RUN | gap-observed | phase=1 | count=2 | file=a | source=driverctl-scan'
    lg 2026-10-20T13:21:00Z 'I-01 | dispatched | step=/gsd-execute-phase 1 --gaps-only | base=a | round=2 | dispatch=3/8 | phase=1'
    lg 2026-10-20T13:30:00Z 'I-01 | report | status=done'
    printf '    STATUS: done\n    FINDINGS: 2\n'
    lg 2026-10-20T13:30:30Z 'RUN | gap-observed | phase=1 | count=3 | file=a | source=driverctl-scan'
    lg 2026-10-20T13:31:00Z 'I-01 | verified | done-when=not-met | evidence=x'
    lg 2026-10-20T13:31:10Z 'I-01 | status | state=parked | rule=gap-rounds | outcome=x'
    lg 2026-10-20T13:31:20Z 'I-01 | needs-human | question=raise gap-rounds?'
    lg 2026-10-20T13:31:30Z 'RUN | end | reason=all-parked'; } > "$ev/LOG.md"
  printf '{"status":"all-parked"}' > "$ev/state.json"; : > "$ev/observed.txt"; good_new "$ev" o1
  printf 'status: all-parked\nprompt_version: "driver-prompt/1.0.0"\n' > "$ev/frontmatter.md"
  printf 'CASE=o1\nPROMPT_SOURCE=embedded\n' > "$ev/run-env.txt"
  want_res "o1 pass" o1 "$ev" PASS
  cp "$ev/LOG.md" "$ev/LOG.good"
  lg 2026-10-20T13:31:25Z 'RUN | dispatch-denied | reason=phase 1 has failed verification 3 times' >> "$ev/LOG.md"; want_res "o1 a deny is a FAIL" o1 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  lg 2026-10-20T13:31:25Z 'RUN | cap-tripped | cap=gap_rounds | used=3 | limit=2 | action=kill-scope' >> "$ev/LOG.md"; want_res "o1 a cap trip is a FAIL" o1 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i '/needs-human/d' "$ev/LOG.md"; want_res "o1 a park without needs-human is a FAIL" o1 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i 's/rule=gap-rounds/rule=divergence/' "$ev/LOG.md"; want_res "o1 divergence park also passes" o1 "$ev" PASS
  sed -i 's/rule=divergence/rule=budget/' "$ev/LOG.md"; want_res "o1 a park under another rule is a FAIL" o1 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i 's/^\(- 2026-10-20T13:31:00Z \)| I-01 | verified.*/\1| I-01 | dispatched | step=x | round=3 | phase=1/' "$ev/LOG.md"; want_res "o1 a dispatch after count 3 is a FAIL" o1 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i 's/prompt=driver-prompt\/1.0.0/prompt=custom:abcdef01/' "$ev/LOG.md"; want_res "o1 a missing version is a FAIL" o1 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  printf 'status: all-parked\n' > "$ev/frontmatter.md"; want_res "o1 frontmatter without prompt_version is a FAIL" o1 "$ev" FAIL
  printf 'status: all-parked\nprompt_version: "driver-prompt/1.0.0"\n' > "$ev/frontmatter.md"
  printf 'PROMPT_SOURCE=file x\nLAUNCH=--prompt-file x\n' > "$ev/run-env.txt"; want_res "o1 a --prompt-file in run-env is a FAIL" o1 "$ev" FAIL
  printf 'CASE=o1\nPROMPT_SOURCE=embedded\n' > "$ev/run-env.txt"
  sed -i 's/--gaps-only/--gaps/;s#/gsd-execute-phase#/gsd-plan-phase#' "$ev/LOG.md"; want_res "o1 one distinct GSD command is a FAIL" o1 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i 's/round=2/round=1/' "$ev/LOG.md"; want_res "o1 rounds 0,1 only is a FAIL" o1 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i 's/^    STATUS: done$/    OK: done/;' "$ev/LOG.md"; want_res "o1 a report not starting STATUS: is a FAIL" o1 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  awk '{ print } /13:30:00Z \| I-01 \| report/ { print "    STATUS: done"; for (i = 0; i < 15; i++) print "    line " i }' "$ev/LOG.md" > "$ev/LOG.long"
  cp "$ev/LOG.long" "$ev/LOG.md"; want_res "o1 a 17-line report is a FAIL" o1 "$ev" FAIL
  cp "$ev/LOG.good" "$ev/LOG.md"
  # early convergence: no third gaps_found, the item is done
  sed -i '/count=3/d;/state=parked/d;/needs-human/d;/verified/d' "$ev/LOG.md"
  lg 2026-10-20T13:31:10Z 'I-01 | status | state=done | outcome=converged' >> "$ev/LOG.md"
  lg 2026-10-20T13:31:30Z 'RUN | end | reason=all-done' >> "$ev/LOG.md"
  want_res "o1 converged before the third gaps_found is ADJUST" o1 "$ev" ADJUST
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i '/count=3/d' "$ev/LOG.md"
  want_res "o1 self-parked needs-human before count 3 is ADJUST" o1 "$ev" ADJUST
  cp "$ev/LOG.good" "$ev/LOG.md"
  sed -i '/count=3/d;/state=parked/d;/needs-human/d;/verified/d' "$ev/LOG.md"
  want_res "o1 no third gaps_found and nothing else is a FAIL" o1 "$ev" FAIL
  # o1 with a retry: the combined verdict records both attempts and follows the retry
  cp "$ev/LOG.good" "$ev/LOG.md"
  mkdir -p "$ev/retry"; cp "$ev/LOG.good" "$ev/retry/LOG.md"; cp "$ev/state.json" "$ev/observed.txt" "$ev/check.txt" "$ev/frontmatter.md" "$ev/run-env.txt" "$ev/retry/"
  sed -i '/count=3/d' "$ev/LOG.md"
  [[ "$(o1_combined "$ev")" == "O1-attempt-1 NOT-REACHED first attempt ended RESULT ADJUST"* ]] || st_fail "o1 combined verdict lacks the first-attempt line"
  [ "$(o1_combined "$ev" | tail -n 1)" = "RESULT PASS" ] || st_fail "o1 combined verdict must follow the retry: $(o1_combined "$ev" | tail -n 1)"
}

# ---------------------------------------------------------------------------------------------- live run
# Everything below this line may spend. Nothing above it starts claude.

# human_save FILE SED_EXPR: rename-save, the way an editor writes (the splice is built to survive it).
human_save() {
  local f="$1" expr="$2" tmp
  tmp="$(dirname "$f")/.human-save.$$"
  sed "$expr" "$f" > "$tmp"
  chmod --reference="$f" "$tmp"
  mv -f "$tmp" "$f"
}
loglines() { wc -l < "$RUN_DIR/LOG.md" | tr -d ' '; }

# pa_make_project DIR: a tiny GSD project (slugkit, 2 phases) for the Haiku cases. Allowing default-branch commits
# is the d33 patch (26.1-10): the scratch repo has only `main` [inferred - audit].
pa_make_project() {
  local P="$1"
  mkdir -p "$P/.planning/phases/01-slugify" "$P/notes" "$P/slugkit"
  git -C "$P" init -q -b main
  git -C "$P" config user.email acceptance@example.invalid
  git -C "$P" config user.name acceptance
  printf '"""slugkit: tiny text utilities."""\n' > "$P/slugkit/__init__.py"
  printf '# slugkit\n\nTiny text utilities.\n' > "$P/README.md"
  printf '__pycache__/\n' > "$P/.gitignore"
  printf '# slugkit\n\n## What This Is\nA tiny pure-Python library of text utilities (stdlib only).\n\n## Core Value\nReadable, URL-safe identifiers from titles.\n' > "$P/.planning/PROJECT.md"
  printf '# Roadmap: slugkit\n\n## Phases\n\n- [ ] **Phase 1: Slugify** - `slugify()` with tests\n- [ ] **Phase 2: CLI** - `python3 -m slugkit`\n\n### Phase 1: Slugify\n**Goal**: A tested `slugkit.slugify(title)` function\n**Success Criteria** (what must be TRUE):\n  1. `slugify("Hello World")` returns `"hello-world"`\n' > "$P/.planning/ROADMAP.md"
  printf -- '---\ngsd_state_version: '"'"'1.0'"'"'\nstatus: planning\n---\n\n# Project State\n\n## Current Position\n\nPhase: 1 of 2 (Slugify)\nStatus: Ready to plan\n' > "$P/.planning/STATE.md"
  printf '# Phase 1: Slugify - Context\n\n## Decisions\n- `slugify(title: str) -> str` in `slugkit/slug.py`.\n' > "$P/.planning/phases/01-slugify/01-CONTEXT.md"
  cat > "$P/.planning/config.json" <<'EOF'
{
  "mode": "yolo",
  "granularity": "coarse",
  "model_profile": "balanced",
  "workflow": { "research": false, "plan_check": false, "verifier": true, "auto_advance": false,
                "nyquist_validation": false, "security_enforcement": false, "use_worktrees": false },
  "planning": { "commit_docs": true, "search_gitignored": false, "sub_repos": [] },
  "git": { "create_tag": false, "allow_default_branch_commits": true },
  "parallelization": { "enabled": false },
  "gates": { "execute_next_plan": false, "confirm_transition": false, "confirm_milestone_scope": false },
  "planner": { "stall_detection_enabled": false }
}
EOF
  git -C "$P" add -A
  git -C "$P" commit -q -m "chore: scratch GSD project (slugkit)"
}

# pa_new_run: `driver new`, the id parsed from its `created run <id>` line (Pitfall 8: the date is never recomputed).
pa_new_run() {
  local out
  out="$("$BIN" --config "$CFG" driver new "$ALIAS" "$SLUG" 2>&1)" || { echo "prompt-acceptance: driver new failed: $out" >&2; exit 1; }
  RUN_ID="$(printf '%s\n' "$out" | sed -n 's/^driver new: created run \([^ ]*\)$/\1/p' | sed -n '1p')"
  [[ "$RUN_ID" =~ ^[0-9]{8}- ]] || { echo "prompt-acceptance: no 'driver new: created run <id>' line in: $out" >&2; exit 1; }
  RUN_DIR="$PROJECT/.planning/meta-manager/drivers/$RUN_ID"
  [ -f "$RUN_DIR/DIRECTION.md" ] || { echo "prompt-acceptance: driver new left no DIRECTION.md at $RUN_DIR" >&2; exit 1; }
  obs run_id "$RUN_ID"
  pa_after_new
}

# pa_fill_direction: replace each placeholder line of the shipped template body with the case's value (PA_* env),
# keeping the frontmatter bytes, the standing rules and the stop-rules block.
pa_fill_direction() {
  python3 -I - "$RUN_DIR/DIRECTION.md" <<'PY'
import os, re, sys
path = sys.argv[1]
env = os.environ.get
lines = open(path, encoding="utf-8").read().split("\n")
out, fence, item = [], 0, 0
for line in lines:
    if fence < 2:
        if re.match(r"^---[ \t]*$", line):
            fence += 1
        out.append(line)
        continue
    if re.match(r"^- goal: \{.*\}$", line):
        out.append("- goal: " + env("PA_GOAL", ""))
    elif re.match(r"^- worker-model: ", line) and env("PA_WORKER"):
        out.append("- worker-model: " + env("PA_WORKER"))
    elif re.match(r"^- run-budget: ", line) and env("PA_RUNBUDGET"):
        out.append("- run-budget: " + env("PA_RUNBUDGET"))
    elif re.match(r"^- isolation: ", line) and env("PA_ISOLATION"):
        out.append("- isolation: " + env("PA_ISOLATION"))
    elif re.match(r"^- \{project-specific rules.*\}$", line):
        for rule in [r for r in env("PA_RULE", "").split("\n") if r.strip()]:
            out.append("- " + rule)
    elif re.match(r"^### I-01 \{.*\}$", line):
        out.append("### I-01 " + env("PA_TITLE", ""))
    elif re.match(r"^- intent: \{.*\}$", line):
        out.append("- intent: " + env("PA_INTENT", ""))
    elif re.match(r"^- done-when: \{.*\}$", line):
        out.append("- done-when: " + env("PA_DONE", ""))
    elif re.match(r"^- budget: ", line):
        out.append("- budget: " + env("PA_BUDGET", ""))
    elif re.match(r"^- start-with: \{.*\}$", line):
        if env("PA_START"):
            out.append("- start-with: " + env("PA_START"))
    else:
        out.append(line)
text = "\n".join(out)
extra = env("PA_EXTRA_ITEMS", "")
if extra:
    text = text.rstrip("\n") + "\n\n" + extra.strip("\n") + "\n"
open(path, "w", encoding="utf-8").write(text)
PY
}

# pa_check_direction: `driver check` must exit 0 (DRIVE-03 evidence in check.txt).
pa_check_direction() {
  local out rc=0
  out="$("$BIN" --config "$CFG" driver check "$ALIAS" "$RUN_ID" 2>&1)" || rc=$?
  { printf 'driver check rc=%s\n' "$rc"; [ -z "$out" ] || printf '%s\n' "$out"; } > "$CHECKF"
  if [ "$rc" -ne 0 ]; then
    echo "prompt-acceptance: driver check failed on the filled template (rc=$rc); nothing was launched:" >&2
    printf '%s\n' "$out" >&2
    exit 1
  fi
}

# pa_launch [flags...]: ceiling gate, then one launch of the run by id; sets LAUNCH_RC. A foreground launch's
# cost is recorded at once; a detached launch is recorded by the caller once it has ended.
pa_launch() {
  local crc=0 args
  ceiling_ok "$CEIL" "$BUDGET" "$LEDGER" || crc=$?
  if [ "$crc" -ne 0 ]; then
    [ "$crc" = 1 ] && echo "prompt-acceptance: campaign ceiling \$$CEIL would be exceeded by this launch (budget \$$BUDGET)" >&2
    CEILING_STOP=1
    return 8
  fi
  args=(driver launch "$ALIAS" "$RUN_ID" --model "$MODEL" --loop-mode loop --cap-dispatches "$CAP_D" --cap-minutes "$CAP_M"
        --cap-gap-rounds "$CAP_G" --max-budget-usd "$BUDGET" "${PROMPT_FLAGS[@]}" "$@")
  obs "launch_args_$((LAUNCH_SEQ + 1))" "$(printf '%s' "${args[*]:4}" | sed "s#$REPO/##g")"
  set +e
  timeout "$LAUNCH_TIMEOUT" env "${SCRUB[@]}" "$BIN" --config "$CFG" "${args[@]}" >> "$OUT" 2>&1
  LAUNCH_RC=$?
  set -e
  LAUNCH_SEQ=$((LAUNCH_SEQ + 1))
  obs "launch_rc_$LAUNCH_SEQ" "$LAUNCH_RC"
  case " $* " in *" --foreground "*) pa_record_costs ;; esac
  return 0
}

# pa_record_costs: record every launch stream not yet in cost.txt (the result cost, else the full budget).
pa_record_costs() {
  local f n charge usd src
  for f in "$STATE_DIR"/launch-*.jsonl; do
    [ -e "$f" ] || continue
    n="${f##*/launch-}"; n="${n%.jsonl}"
    grep -q "^$n " "$COSTF" 2>/dev/null && continue
    charge="$(pa_stream_charge "$f" "$BUDGET")"; usd="${charge% *}"; src="${charge#* }"
    printf '%s %s %s\n' "$n" "$usd" "$src" >> "$COSTF"
    printf '%s %s %s %s %s\n' "$(LEDGER_UTC)" "$LABEL" "$n" "$usd" "$src" >> "$LEDGER"
  done
}

# pa_wait_dispatch: I-01 dispatched in LOG.md and hook_dispatches >= 1 (the dispatch is really in flight).
pa_wait_dispatch() {
  local waited=0 hd
  while [ "$waited" -lt "$WAIT_DISPATCH_S" ]; do
    hd="$(jget "$RUN_DIR/state.json" hook_dispatches)"
    if [ -n "$(lineno ' | I-01 | dispatched |' "$RUN_DIR/LOG.md" 2>/dev/null || true)" ] && [ "${hd:-0}" -ge 1 ] 2>/dev/null; then return 0; fi
    sleep 1; waited=$((waited + 1))
  done
  return 1
}
# pa_wait_launch_end BASELINE_LINES: a RUN launch-end line after BASELINE_LINES (a detached launch has finished).
pa_wait_launch_end() {
  local waited=0
  while [ "$waited" -lt "$WAIT_END_S" ]; do
    [ -z "$(first_after ' | RUN | launch-end |' "$RUN_DIR/LOG.md" "$1")" ] || return 0
    sleep 1; waited=$((waited + 1))
  done
  return 1
}
# pa_launcher_held: the run's launcher.lock is held (a launcher is alive).
pa_launcher_held() { ! flock -s -n "$RUN_DIR/launcher.lock" true 2>/dev/null; }
# pa_kill_launcher: SIGKILL the launcher named by the lock-holder record (the 28 l5 path); the scope must follow.
pa_kill_launcher() {
  local pid unit t0 t gone=timeout left alive=0 p sampled
  pid="$(jget "$RUN_DIR/launcher.lock" pgid)"
  unit="$(last_scope_unit)"
  obs hook_dispatches_before_kill "$(jget "$RUN_DIR/state.json" hook_dispatches)"
  sampled="$(cgroup_pids "$unit")"
  if [ -n "$pid" ] && pa_launcher_held; then
    obs launcher_killed yes
    kill -KILL "$pid" 2>/dev/null || true
  else
    obs launcher_killed no
  fi
  t0="$(date +%s.%N)"
  while :; do
    t="$(awk -v a="$t0" -v b="$(date +%s.%N)" 'BEGIN { printf "%.1f", b - a }')"
    left=""; [ -z "$unit" ] || left="$(cgroup_pids "$unit")"
    alive=0
    for p in $sampled; do proc_alive "$p" && alive=$((alive + 1)); done
    if ! pa_launcher_held && [ -z "${left// /}" ] && [ "$alive" = 0 ]; then gone="$t"; break; fi
    awk -v t="$t" 'BEGIN { exit !(t > 12) }' && break
    sleep 0.5
  done
  if [ "$gone" = timeout ] && [ -n "$unit" ]; then
    systemctl --user kill --signal=SIGKILL "$unit" >/dev/null 2>&1 || true
    obs scope_force_killed yes
  fi
  obs scope_gone_s "$gone"
  if pa_launcher_held; then obs lock_free_after_kill no; else obs lock_free_after_kill yes; fi
  obs status_after_kill "$(jget "$RUN_DIR/state.json" status)"
}

# ---------------------------------------------------------------------------------------------- cases
# case_h1 (D-17, D-19): launch 1 detached on a plain chore; once I-01 is dispatched and the hook counted it,
# SIGKILL the launcher (the scope dies with it) and relaunch in the foreground.
case_h1() {
  pa_make_project "$PROJECT"
  "$BIN" --config "$CFG" add "$PROJECT" "$ALIAS" >> "$SCRATCH_ROOT/setup.out"
  pa_new_run
  export PA_GOAL="Complete the one chore in I-01 and nothing else." PA_WORKER=haiku PA_TITLE="Write the h1 note and commit it"
  export PA_INTENT="A worker creates notes/h1.txt holding one line, runs the shell command sleep 90 once so the chore takes about ninety seconds, then commits the file."
  export PA_DONE="notes/h1.txt is committed on the current branch." PA_BUDGET="dispatches=4" PA_RULE="Do not touch any file other than notes/h1.txt."
  pa_fill_direction
  pa_check_direction
  obs direction_body_sha256_before "$(direction_body_sha "$RUN_DIR/DIRECTION.md")"
  local base
  pa_launch || return 0
  [ "$LAUNCH_RC" = 0 ] || { obs launcher_a_rc "$LAUNCH_RC"; return 0; }
  if pa_wait_dispatch; then obs dispatch_observed yes; else obs dispatch_observed no; echo "REHEARSAL-OR-MISS: I-01 was not dispatched within ${WAIT_DISPATCH_S}s" >&2; fi
  pa_kill_launcher
  pa_record_costs
  base="$(loglines)"
  obs log_lines_before_relaunch "$base"
  pa_launch --foreground || return 0
}

# pa_haiku_setup: the scratch project, its registry entry and the run (driver new) for the Haiku cases.
pa_haiku_setup() {
  pa_make_project "$PROJECT"
  "$BIN" --config "$CFG" add "$PROJECT" "$ALIAS" >> "$SCRATCH_ROOT/setup.out"
  pa_new_run
}
# pa_wait_line NEEDLE BASELINE TIMEOUT: a LOG line after BASELINE contains NEEDLE.
pa_wait_line() {
  local waited=0
  while [ "$waited" -lt "$3" ]; do
    [ -z "$(first_after "$1" "$RUN_DIR/LOG.md" "$2")" ] || return 0
    sleep 1; waited=$((waited + 1))
  done
  return 1
}
# pa_stop_run: `driver stop`, then wait (bounded) for the launch to end after BASELINE lines.
pa_stop_run() {
  local rc=0
  "$BIN" --config "$CFG" driver stop "$ALIAS" "$RUN_ID" --wait-secs 30 >> "$OUT" 2>&1 || rc=$?
  obs "stop_rc_$1" "$rc"
  pa_wait_launch_end "$2" || true
}
# pa_write_gaps_found: the phase-1 verification file with status: gaps_found, written BEFORE a launch. Per the
# 29-05 SUMMARY `A1:` line a file present at launch is counted on the FIRST scan tick (count=1), so this is the
# way to get exactly one counted round.
pa_write_gaps_found() {
  printf -- '---\nphase: 01-slugify\nverified: %s\nstatus: gaps_found\nscore: 2/3\n---\nharness-seeded gaps_found (%s)\n' \
    "$(date -u +%FT%TZ)" "$CASE" > "$PROJECT/.planning/phases/01-slugify/01-VERIFICATION.md"
}

# case_h2 (D-18, D-19): launch 1 detached on a chore with a second item behind it; once I-01 is in flight save
# `control: pause` (rename-save, like an editor); the run must record the report, dispatch nothing new and end
# reason=pause. Then restore `control: run` and relaunch: one recovered for I-01 before any dispatch. The premise
# "I-01 is still running at the relaunch" is staged by the harness when the live driver finished the item first
# (pa_stage_item_running, WR-07; `h2_harness=2` marks evidence from this revision).
case_h2() {
  pa_haiku_setup
  obs h2_harness 2
  export PA_GOAL="Complete the two chores, I-01 first and I-02 after it, one dispatch each." PA_WORKER=haiku PA_TITLE="Write the h2 note and commit it"
  export PA_INTENT="A worker creates notes/h2.txt holding one line, runs the shell command sleep 90 once so the chore takes about ninety seconds, then commits the file."
  export PA_DONE="notes/h2.txt is committed on the current branch." PA_BUDGET="dispatches=4" PA_RULE="Do not touch any file other than the notes file of the item in flight."
  export PA_EXTRA_ITEMS="### I-02 Write the h2b note and commit it

- intent: A worker creates notes/h2b.txt holding one line and commits the file.
- done-when: notes/h2b.txt is committed on the current branch.
- budget: dispatches=3
- after: I-01
- hold: false
- answer:"
  pa_fill_direction
  pa_check_direction
  obs direction_body_sha256_before "$(direction_body_sha "$RUN_DIR/DIRECTION.md")"
  local b1 b2
  b1="$(loglines)"
  pa_launch || return 0
  [ "$LAUNCH_RC" = 0 ] || { obs launcher_a_rc "$LAUNCH_RC"; return 0; }
  if pa_wait_dispatch; then obs dispatch_observed yes; else obs dispatch_observed no; echo "REHEARSAL-OR-MISS: I-01 was not dispatched within ${WAIT_DISPATCH_S}s" >&2; fi
  obs hook_dispatches_before_pause "$(jget "$RUN_DIR/state.json" hook_dispatches)"
  human_save "$RUN_DIR/DIRECTION.md" 's/^- control: run$/- control: pause/'
  obs pause_saved_epoch "$(date +%s)"
  obs pause_saved_utc "$(date -u +%FT%TZ)"
  grep -q '^- control: pause$' "$RUN_DIR/DIRECTION.md" || obs pause_saved no
  # the run must end by itself (reason=pause); if it does not, stop it so the relaunch below can happen at all
  if ! pa_wait_launch_end "$b1"; then obs launch_1_ended_by_itself no; pa_stop_run 1 "$b1"; fi
  pa_record_costs
  pa_stage_item_running I-01
  b2="$(loglines)"
  obs log_lines_before_relaunch "$b2"
  human_save "$RUN_DIR/DIRECTION.md" 's/^- control: pause$/- control: run/'
  obs restored_epoch "$(date +%s)"
  pa_launch || return 0
  [ "$LAUNCH_RC" = 0 ] || { obs launcher_b_rc "$LAUNCH_RC"; return 0; }
  pa_wait_line ' | I-01 | recovered |' "$b2" "$WAIT_DISPATCH_S" || obs recovered_observed no
  pa_stop_run 2 "$b2"
  pa_record_costs
}

# pa_serve_up: serve the run's store host-side (`driverctl serve`, no model, no launcher); sets SERVE_PID and
# SERVE_SOCK. pa_serve_down stops it. Shared by pa_seed_park (h3) and pa_stage_item_running (h2).
pa_serve_up() {
  local waited=0
  SERVE_SOCK=""
  bg "$BIN" driverctl serve --run-dir "$RUN_DIR" --project "$PROJECT" > "$SCRATCH_ROOT/serve.out" 2>&1
  SERVE_PID=$!
  while [ "$waited" -lt 100 ]; do
    grep -q 'listening on' "$SCRATCH_ROOT/serve.out" 2>/dev/null && break
    kill -0 "$SERVE_PID" 2>/dev/null || { echo "prompt-acceptance: driverctl serve exited: $(cat "$SCRATCH_ROOT/serve.out")" >&2; exit 1; }
    sleep 0.2; waited=$((waited + 1))
  done
  SERVE_SOCK="$(sed -n 's/.*listening on \(.*\)$/\1/p' "$SCRATCH_ROOT/serve.out" | sed -n '1p')"
  [ -n "$SERVE_SOCK" ] || { echo "prompt-acceptance: driverctl serve never reported a socket: $(cat "$SCRATCH_ROOT/serve.out")" >&2; exit 1; }
}
pa_serve_down() {
  kill "$SERVE_PID" 2>/dev/null || true
  wait "$SERVE_PID" 2>/dev/null || true
}

# pa_stage_item_running ITEM (WR-07): h2's premise (D-19) is "the previous launch ended on pause while ITEM was still
# running", and the live driver decides that, not the harness: the committed 20261020 run finished I-01 (done) before
# it ended on pause, so no item awaited recovery and H2-5 could never hold. The case therefore stages its own
# premise: when the store does not already say ITEM is running, log `status --state running` for it host-side (free,
# the pa_seed_park pattern) before the relaunch. The observation records which it was, so the evidence never claims a
# native premise it did not have.
pa_stage_item_running() {
  local item="$1" state key
  key="premise_$(printf '%s' "$item" | tr 'A-Z-' 'a-z_')_running"
  state="$(jget "$RUN_DIR/state.json" "items.$item.state")"
  if [ "$state" = running ]; then obs "$key" native; return 0; fi
  obs "${key%_running}_state_before_staging" "${state:-none}"
  # the launcher that just ended may still hold the run lock for a moment; serve needs it
  local held=0
  while pa_launcher_held && [ "$held" -lt 30 ]; do sleep 1; held=$((held + 1)); done
  pa_serve_up
  env "${SCRUB[@]}" DRIVERCTL_SOCK="$SERVE_SOCK" "$BIN" driverctl log status --item "$item" --state running >> "$SCRATCH_ROOT/seed.out" 2>&1 || obs "$key" staging-failed
  pa_serve_down
  state="$(jget "$RUN_DIR/state.json" "items.$item.state")"
  if [ "$state" = running ]; then obs "$key" staged-host-side; else obs "$key" no; fi
}

# pa_seed_park: free, host-side. Serve the run's store with `driverctl serve` and log the history that parks I-01
# on gap-rounds (start, dispatched, report, verified not-met, status parked, needs-human, end all-parked); no model.
pa_seed_park() {
  local sock pid
  pa_serve_up
  sock="$SERVE_SOCK"; pid="$SERVE_PID"
  ctl() { env "${SCRUB[@]}" DRIVERCTL_SOCK="$sock" "$BIN" driverctl "$@" >> "$SCRATCH_ROOT/seed.out" 2>&1; }
  ctl log start --mode headless --loop-mode loop
  ctl log dispatched --item I-01 --step "/gsd-plan-phase 1 --gaps" --round 0 --phase 1
  printf 'STATUS: partial\nFINDINGS: 1\nSUGGESTED_NEXT: phase 1 failed verification; a closure round needs a gap-rounds budget\n' \
    | env "${SCRUB[@]}" DRIVERCTL_SOCK="$sock" "$BIN" driverctl log report --item I-01 --status partial >> "$SCRATCH_ROOT/seed.out" 2>&1
  ctl log verified --item I-01 --done-when not-met --evidence "01-VERIFICATION.md status: gaps_found"
  ctl log status --item I-01 --state parked --rule gap-rounds --outcome "gap-rounds budget is 0; a closure round needs a budget edit"
  ctl log needs-human --item I-01 --question "Phase 1 failed verification and gap-rounds is 0. May one closure round be dispatched?"
  ctl log end --reason all-parked
  kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  obs seeded_events yes
}

# case_h3 (D-13, OQ-R1): the park is seeded host-side. Launch A with only an `answer:` filled (budget unchanged):
# nothing may be dispatched and the run ends all-parked. Launch B after `gap-rounds=0` becomes `gap-rounds=1`:
# I-01 is dispatched; the harness then runs `driver stop`.
case_h3() {
  pa_haiku_setup
  export PA_GOAL="Close the failing verification of phase 1 only when the budget or a human allows it." PA_WORKER=haiku
  export PA_TITLE="Close the phase 1 verification gaps" PA_INTENT="Phase 1 is verified: its 01-VERIFICATION.md reads status passed."
  export PA_DONE='`.planning/phases/01-slugify/01-VERIFICATION.md` has status: passed.' PA_BUDGET="dispatches=3, gap-rounds=0, minutes=15"
  export PA_START="/gsd-plan-phase 1 --gaps"
  pa_fill_direction
  pa_check_direction
  obs direction_body_sha256_before "$(direction_body_sha "$RUN_DIR/DIRECTION.md")"
  pa_seed_park
  pa_write_gaps_found
  human_save "$RUN_DIR/DIRECTION.md" 's/^- answer:$/- answer: Yes, one closure round is approved./'
  obs log_lines_before_A "$(loglines)"
  pa_launch --foreground || return 0
  obs log_lines_before_B "$(loglines)"
  local bb; bb="$(loglines)"
  human_save "$RUN_DIR/DIRECTION.md" '/^- budget: /s/gap-rounds=0/gap-rounds=1/'
  grep -q '^- budget: .*gap-rounds=1' "$RUN_DIR/DIRECTION.md" || obs budget_edit no
  pa_launch || return 0
  [ "$LAUNCH_RC" = 0 ] || { obs launcher_b_rc "$LAUNCH_RC"; return 0; }
  pa_wait_line ' | I-01 | dispatched |' "$bb" "$WAIT_DISPATCH_S" || obs dispatch_observed_b no
  pa_stop_run B "$bb"
  pa_record_costs
}

# case_h4 (D-24): the prompt with the gap rules stripped, launcher gap cap 0, phase 1 gaps_found at launch: the
# launcher denies the closure dispatch and the driver must park rule=launcher-cap and not retry.
case_h4() {
  pa_haiku_setup
  export PA_GOAL="Close the phase 1 gaps with a plan-phase --gaps round." PA_WORKER=haiku
  export PA_TITLE="Close the phase 1 verification gaps" PA_INTENT="Phase 1 is verified: its 01-VERIFICATION.md reads status passed."
  export PA_DONE='`.planning/phases/01-slugify/01-VERIFICATION.md` has status: passed.' PA_BUDGET="dispatches=3"
  export PA_START="/gsd-plan-phase 1 --gaps"
  pa_fill_direction
  pa_check_direction
  obs direction_body_sha256_before "$(direction_body_sha "$RUN_DIR/DIRECTION.md")"
  obs expected_prompt_version "$(h4_expected_version)"
  pa_write_gaps_found
  pa_launch --foreground || return 0
}

# case_o1 (D-23, SC3, DRIVE-01): a real Opus driver on the embedded prompt against spike-003 seed v2 (the feed moves
# after every gsd-verifier spawn). The spike's own setup mints a throwaway run; it is removed and `driver new`
# mints the real one.
case_o1() {
  bash "$S3/experiment/setup-scratch.sh" "$PROJECT" "o1-seed" "$S3/templates/DIRECTION.run1.md" >> "$SCRATCH_ROOT/setup.out"
  rm -rf "$PROJECT/.planning/meta-manager/drivers/o1-seed"
  # seed v2 needs default-branch commits allowed in the scratch repo (26.1-10), folded into the scratch commit
  sed -i 's/"git": { "create_tag": false }/"git": { "create_tag": false, "allow_default_branch_commits": true }/' "$PROJECT/.planning/config.json"
  grep -q '"allow_default_branch_commits": true' "$PROJECT/.planning/config.json" || { echo "prompt-acceptance: seed2 config patch did not apply" >&2; exit 1; }
  git -C "$PROJECT" add .planning/config.json && git -C "$PROJECT" commit -q --amend --no-edit
  "$BIN" --config "$CFG" add "$PROJECT" "$ALIAS" >> "$SCRATCH_ROOT/setup.out"
  pa_new_run
  export PA_GOAL="Ship phase 1 of slugkit through the normal GSD chain, unattended." PA_WORKER=sonnet PA_RUNBUDGET="dispatches=10, hours=2" PA_ISOLATION=none
  export PA_TITLE="Phase 1 slugify" PA_INTENT="Phase 1 is planned, executed and verified through GSD."
  export PA_DONE='`.planning/phases/01-slugify/01-VERIFICATION.md` frontmatter has `status: passed`' PA_BUDGET="dispatches=8, gap-rounds=2, minutes=80"
  export PA_START="/gsd-plan-phase 1" PA_RULE="discuss-phase is skipped: CONTEXT.md exists for phase 1.
Max 2 gap-closure rounds per phase; a third is a park, never a dispatch."
  pa_fill_direction
  pa_check_direction
  obs direction_body_sha256_before "$(direction_body_sha "$RUN_DIR/DIRECTION.md")"
  mkdir -p "$CFGDIR"
  bg python3 -I "$S3/tools/drift.py" watch --project "$PROJECT" --log "$SCRATCH_ROOT/drift.log" --snapshots "$SCRATCH_ROOT/verification" --trigger verifier --claude-config "$CFGDIR" > "$SCRATCH_ROOT/watch.out" 2>&1
  sleep 1
  kill -0 "${BGPIDS##* }" 2>/dev/null || { echo "prompt-acceptance: the feed watcher is not alive; see $SCRATCH_ROOT/watch.out" >&2; exit 1; }
  pa_launch --foreground || return 0
}

# ---------------------------------------------------------------------------------------------- run
# evidence_gates EVD: derived files only; the envelope scan and the secret grep (T-29-24). A hit deletes the copy.
evidence_gates() {
  local evd="$1" bad=0
  if [ -n "$(find "$evd" \( -name '*.jsonl' -o -name 'claude-config' \) 2>/dev/null)" ]; then bad=1; echo "prompt-acceptance: a stream jsonl or claude-config reached $evd" >&2; fi
  grep -rElI 'sk-ant|oat01|accessToken|refreshToken' "$evd" >&2 && bad=1 || true
  "$BIN" --config "$CFG" envelope scan "$ALIAS" "$evd" >&2 || bad=1
  if [ "$bad" = 1 ]; then
    rm -f "$evd"/{LOG.md,state.json,frontmatter.md,cost.txt,check.txt,observed.txt,launcher-out.txt,stop-guard.log,drift.log}
    echo "prompt-acceptance: evidence gate hit; the evidence copy was deleted (campaign.txt keeps the spend)" >&2
    exit 6
  fi
}

# pa_superseded_harness CASE EVDIR (WR-07): the FAIL in EVDIR was produced by a harness revision that has since been
# fixed as a harness defect, not by the product missing a criterion. Today only h2: before `h2_harness=2` the case
# could not stage its own premise (I-01 running at the relaunch), so its H2-5 FAIL measured the harness. The
# allowance is the single rerun the guard already permits (RERUN=1, consent and ceiling gates unchanged), and it
# closes once the evidence carries the marker: a FAIL from the fixed harness is a finding like any other.
pa_superseded_harness() {
  [ "$1" = h2 ] && [ -z "$(obs_get "$2/observed.txt" h2_harness)" ]
}

# pa_evidence_guard CASE: sets EVD ATTEMPT LABEL CLEAR_OLD_EVIDENCE; refuses to spend twice (exit 2).
pa_evidence_guard() {
  local case="$1" res
  EVD="$ACC/$case"; ATTEMPT=first; LABEL="$case"; CLEAR_OLD_EVIDENCE=0
  if [ -f "$EVD/run-env.txt" ] || [ -f "$EVD/verdict.txt" ]; then
    [ "${RERUN:-0}" = 1 ] || { echo "prompt-acceptance: $EVD already holds evidence; RERUN=1 to run it again" >&2; exit 2; }
    if [ -f "$EVD/verdict.txt" ]; then
      res="$(sed -n 's/^RESULT //p' "$EVD/verdict.txt" | tail -n 1)"
      [ "$res" != PASS ] || { echo "prompt-acceptance: $case already PASSed; no rerun is needed or allowed" >&2; exit 2; }
      # a logic FAIL is a finding, not a rerun (RESEARCH A7); only ADJUST (o1) or an infrastructure fault reruns
      if [ "$res" = FAIL ] && [ "$(obs_get "$EVD/observed.txt" infra_fault)" != yes ] && ! pa_superseded_harness "$case" "$EVD"; then
        echo "prompt-acceptance: $case ended FAIL without an infrastructure fault; a miss is reported, not rerun" >&2; exit 2
      fi
    fi
    if [ "$case" = o1 ]; then
      if [ -f "$EVD/retry/run-env.txt" ] || [ -f "$EVD/retry/verdict.txt" ]; then
        echo "prompt-acceptance: the single o1 retry was already used; never a third attempt (D-25)" >&2; exit 2
      fi
      ATTEMPT=retry; EVD="$EVD/retry"; LABEL=o1-retry
    else
      if grep -q '^ATTEMPT=rerun' "$EVD/run-env.txt" 2>/dev/null; then
        echo "prompt-acceptance: the single rerun of $case was already used" >&2; exit 2
      fi
      ATTEMPT=rerun; CLEAR_OLD_EVIDENCE=1
    fi
  fi
}

# pa_gates CASE: consent (10), ceiling (8), prereg (9). In a rehearsal the consent and prereg gates report and go on.
pa_gates() {
  local case="$1" rc=0 crc=0
  consent_gate "$ACC/consent.txt" "$case" || rc=$?
  if [ "$rc" -ne 0 ]; then
    if rehearsal_p; then echo "REHEARSAL: consent would refuse (exit 10)"; CEIL="$CEIL_REQ"; else exit 10; fi
  else
    CEIL="$(min_of "$CONSENT_CEIL" "$CEIL_REQ")"
  fi
  ceiling_ok "$CEIL" "$BUDGET" "$LEDGER" || crc=$?
  if [ "$crc" -ne 0 ]; then
    [ "$crc" = 1 ] && echo "prompt-acceptance: campaign ceiling \$$CEIL would be exceeded by a launch of \$$BUDGET" >&2
    exit 8
  fi
  rc=0
  pa_prereg_check "$REPO" "$RUBRIC_REL" "$ACC/prereg.txt" || rc=$?
  if [ "$rc" -ne 0 ]; then
    if rehearsal_p; then echo "REHEARSAL: prereg would refuse (exit 9)"; else echo "prompt-acceptance: the Phase 29 rubric is not pre-registered (D-25)" >&2; exit 9; fi
  fi
}

pa_run() {
  local case="$1" n
  mode_config "$case" || { echo "$USAGE" >&2; exit 2; }
  CASE="$case"; MODEL="$MODEL_D"
  CEIL_REQ="${CAMPAIGN_CEILING_USD:-$MAX_CEILING}"
  ceiling_value_ok "$CEIL_REQ" || { echo "prompt-acceptance: CAMPAIGN_CEILING_USD '$CEIL_REQ' must be a number in (0, $MAX_CEILING]" >&2; exit 2; }
  LEDGER="$ACC/campaign.txt"
  if rehearsal_p; then
    ! committed_acc_p || { echo "prompt-acceptance: ACC_PATH_PREPEND (a rehearsal) needs ACCEPTANCE_DIR away from the committed evidence" >&2; exit 2; }
    export PATH="$ACC_PATH_PREPEND:$PATH"
    WAIT_DISPATCH_S=8; WAIT_END_S=30
  else
    WAIT_DISPATCH_S=420; WAIT_END_S=$((CAP_M * 60 + 300))
  fi
  # (1) never spend twice; (2) the cost gates, all before any launch
  pa_evidence_guard "$case"
  pa_gates "$case"
  PROMPT_FLAGS=()
  if [ "$case" = h4 ]; then
    [ -f "$REPO/$H4_FIXTURE_REL" ] && git -C "$REPO" diff --quiet HEAD -- "$H4_FIXTURE_REL" \
      || { echo "prompt-acceptance: $H4_FIXTURE_REL is missing or differs from HEAD (kit integrity)" >&2; exit 7; }
    PROMPT_FLAGS=(--prompt-file "$REPO/$H4_FIXTURE_REL")
  fi
  # (3) scratch root
  SCRATCH_ROOT="${SCRATCH_ROOT:-${XDG_STATE_HOME:-$HOME/.local/state}/gsd-meta-manager/prompt-acceptance/$case-$(date -u +%Y%m%dT%H%M%SZ)}"
  scratch_root_ok "$SCRATCH_ROOT" || exit 2
  [ ! -e "$SCRATCH_ROOT/project" ] || { echo "prompt-acceptance: $SCRATCH_ROOT/project exists; choose a new SCRATCH_ROOT" >&2; exit 2; }
  echo "SCRATCH_ROOT=$SCRATCH_ROOT"
  # (4) build the binary under test (a stale one would test old code), then probe the host
  if [ -z "${GSDMM_BIN:-}" ]; then ( cd "$REPO" && cargo build --release --quiet ) || { echo "prompt-acceptance: cargo build --release failed" >&2; exit 1; }; fi
  [ -x "$BIN" ] || { echo "prompt-acceptance: $BIN is not executable" >&2; exit 2; }
  host_preconditions
  # (5) from here on money can be spent; superseded evidence goes only now
  if [ "$CLEAR_OLD_EVIDENCE" = 1 ]; then
    rm -f "$EVD"/{verdict.txt,LOG.md,state.json,frontmatter.md,cost.txt,check.txt,observed.txt,run-env.txt,launcher-out.txt,stop-guard.log,drift.log}
  fi
  ALIAS="acc-$case"; SLUG="acc-$case-$(date -u +%H%M%S)"; CFG="$SCRATCH_ROOT/config.json"
  PROJECT="$SCRATCH_ROOT/project"
  OUT="$SCRATCH_ROOT/launcher-out.txt"; OBS="$SCRATCH_ROOT/observed.txt"; COSTF="$SCRATCH_ROOT/cost.txt"; CHECKF="$SCRATCH_ROOT/check.txt"
  LAUNCH_SEQ=0; LAUNCH_RC=0; CEILING_STOP=0; LAUNCH_TIMEOUT=$((CAP_M * 60 + 900)); RUN_ID=""; H8RUN=""
  mkdir -p "$SCRATCH_ROOT" "$EVD" "$(dirname "$LEDGER")"
  : > "$OUT"; : > "$OBS"; : > "$COSTF"; : > "$CHECKF"; : > "$SCRATCH_ROOT/setup.out"
  SCRUB=(); while IFS= read -r n; do case "$n" in CLAUDE*|ANTHROPIC*) SCRUB+=(-u "$n") ;; esac; done < <(compgen -e)
  trap cleanup EXIT
  [ -f "$LEDGER" ] || printf '# <utc> <case> <launch> <usd> <source>  (D-25: sum may never pass 25)\n' > "$LEDGER"
  {
    echo "CASE=$case"; echo "ATTEMPT=$ATTEMPT"; echo "MODEL=$MODEL"
    if [ "${#PROMPT_FLAGS[@]}" -gt 0 ]; then
      echo "PROMPT_SOURCE=file $H4_FIXTURE_REL"; echo "PROMPT_SHA256=$(sha256sum "$REPO/$H4_FIXTURE_REL" | cut -d' ' -f1)"
    else
      echo "PROMPT_SOURCE=embedded"
    fi
    echo "CAPS=dispatches=$CAP_D,minutes=$CAP_M,gap_rounds=$CAP_G"; echo "MAX_BUDGET_USD=$BUDGET"
    echo "CLAUDE_VERSION=$(claude --version 2>/dev/null | sed -n '1p')"; echo "BINARY_SHA256=$(sha256sum "$BIN" | cut -d' ' -f1)"
    echo "HEAD=$(git -C "$REPO" rev-parse HEAD)"; echo "STARTED_UTC=$(date -u +%FT%TZ)"
    ! rehearsal_p || echo "REHEARSAL=1"
  } > "$EVD/run-env.txt"
  # the dispatch to the case body: it creates the project, the run (driver new), launches and sets RUN_ID/RUN_DIR
  case "$case" in
    h1) case_h1 ;; h2) case_h2 ;; h3) case_h3 ;; h4) case_h4 ;; o1) case_o1 ;;
    *) echo "$USAGE" >&2; exit 2 ;;
  esac
  pa_finish
}

# pa_after_new: derive the launcher state dir once the run id is known.
pa_after_new() {
  local project_real h8p
  project_real="$(realpath "$PROJECT")"; h8p="$(hash8_of "$project_real")"
  H8RUN="$(hash8_of "$(realpath "$RUN_DIR")")"
  STATE_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/gsd-meta-manager/driver/$h8p/$RUN_ID"
  CFGDIR="$STATE_DIR/claude-config"
}

pa_finish() {
  local verdict res infra=yes
  stop_bg
  [ -z "${STATE_DIR:-}" ] || pa_record_costs
  [ -z "${RUN_DIR:-}" ] || obs direction_body_sha256_after "$(direction_body_sha "$RUN_DIR/DIRECTION.md")"
  grep -q ' result$' "$COSTF" 2>/dev/null && infra=no
  obs infra_fault "$infra"
  [ "$CEILING_STOP" = 0 ] || { echo "prompt-acceptance: ceiling reached mid-case; evidence not written" >&2; exit 8; }
  [ -n "${RUN_DIR:-}" ] && [ -f "$RUN_DIR/LOG.md" ] || { echo "prompt-acceptance: the run left no LOG.md; no evidence to derive a verdict from" >&2; exit 1; }
  # derived evidence only
  cp "$RUN_DIR/LOG.md" "$RUN_DIR/state.json" "$EVD/" 2>/dev/null || true
  frontmatter_of "$RUN_DIR/DIRECTION.md" > "$EVD/frontmatter.md"
  cp "$COSTF" "$EVD/cost.txt"; cp "$OBS" "$EVD/observed.txt"; cp "$OUT" "$EVD/launcher-out.txt"; cp "$CHECKF" "$EVD/check.txt"
  [ ! -f "${CFGDIR:-/nonexistent}/stop-guard.log" ] || cp "$CFGDIR/stop-guard.log" "$EVD/stop-guard.log"
  [ ! -f "$SCRATCH_ROOT/drift.log" ] || cp "$SCRATCH_ROOT/drift.log" "$EVD/drift.log"
  echo "ENDED_UTC=$(date -u +%FT%TZ)" >> "$EVD/run-env.txt"
  evidence_gates "$EVD"
  verdict="$(pa_verdict_text "$CASE" "$EVD")"
  printf '%s\n' "$verdict" > "$EVD/verdict.txt"
  if [ "$ATTEMPT" = retry ]; then
    verdict="$(o1_combined "$ACC/o1")"
    printf '%s\n' "$verdict" > "$ACC/o1/verdict.txt"
  fi
  printf '%s\n' "$verdict"
  res="$(printf '%s\n' "$verdict" | sed -n 's/^RESULT //p' | tail -n 1)"
  if rehearsal_p; then
    echo "REHEARSAL: the path ran through driver new, driver check and driver launch; this verdict ($res) is not a measurement"
    exit 0
  fi
  pa_check "$CASE" >/dev/null 2>&1 || { echo "prompt-acceptance: $CASE did not PASS; evidence and verdict are in $EVD" >&2; exit 1; }
  exit 0
}

# ---------------------------------------------------------------------------------------------- main
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  CMD="${1:-}"; CASE_ARG="${2:-}"
  case "$CMD" in
    selftest) pa_selftest ;;
    run|check)
      case " $CASES " in *" $CASE_ARG "*) ;; *) echo "$USAGE" >&2; exit 2 ;; esac
      if [ "$CMD" = check ]; then pa_check "$CASE_ARG"; exit $?; fi
      pa_run "$CASE_ARG" ;;
    *) echo "$USAGE" >&2; exit 2 ;;
  esac
fi
