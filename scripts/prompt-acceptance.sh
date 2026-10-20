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
CASES="h1"
MAX_CEILING=25
USAGE="Usage: prompt-acceptance.sh <selftest | run CASE | check CASE>   CASE: $CASES   (env: SCRATCH_ROOT CAMPAIGN_CEILING_USD RERUN)"

# ---------------------------------------------------------------------------------------------- mode table
# mode_config CASE: sets MODEL_D CAP_D CAP_M CAP_G BUDGET. Budgets are per launch (`--max-budget-usd`) and are what
# the ledger charges a killed launch. Worst case: h1 2 x 0.40 + h2 2 x 0.40 + h3 3 x 0.15 + h4 1 x 0.20 = $2.25,
# o1 2 x 10 = $20, total $22.25 <= $25 [inferred - audit: RESEARCH Paid-Run Plan]. The dispatch and minute caps
# are generous for a one-chore run and are not what the cases measure [inferred - audit].
mode_config() {
  case "$1" in
    h1) MODEL_D=haiku; CAP_D=6; CAP_M=15; CAP_G=2; BUDGET=0.40 ;;
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

eval_case() {
  case "$1" in
    h1) eval_h1 "$2" ;;
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
  text="$(pa_verdict_text "$case" "$dir")"
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
      if [ "$res" = FAIL ] && [ "$(obs_get "$EVD/observed.txt" infra_fault)" != yes ]; then
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
    h1) case_h1 ;;
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
  cat "$EVD/verdict.txt"
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
