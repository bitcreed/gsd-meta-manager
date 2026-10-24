#!/usr/bin/env bash
# In-container stress harness for
# tests/driverctl_race.rs::two_writers_under_direction_lock_lose_nothing
# (quick 261023-qs8). Shared by the BEFORE and AFTER runs, so both are measured
# under the identical environment.
#
# Usage: bash stress-two-writers.sh LABEL PROFILE MODE N
#   LABEL    before | after | probe
#   PROFILE  0 | A | B | C   (external load, see LOAD PROFILES below)
#   MODE     binary (whole test binary, default 4 test threads, as gate 4)
#            exact  (only "$QS8_FILTER" --exact)
#   N        iterations
#
# Environment overrides:
#   QS8_TARGET    test target (default driverctl_race)
#   QS8_FILTER    test name (default two_writers_under_direction_lock_lose_nothing)
#   QS8_TMPFS=1   TMPDIR=/dev/shm inside the container (overlay-vs-tmpfs check)
#   QS8_KEEP_ALL=1 keep every iteration's log, not only the failing ones (in
#                 binary mode this also passes --nocapture, so probe output of
#                 passing iterations is kept; used only for probe runs)
#
# The container is started with the same flags as scripts/pre-tag-check.sh
# section 5 (--rm, host uid:gid, --cpuset-cpus 0-3, the HOME / CARGO_* env, the
# repo at /work, the two cache volumes) on image gsdmm-pre-tag-check:latest.
# It never mounts the docker socket and never runs as root. Raw logs go to
# ${TMPDIR}/qs8-stress/..., never into the repo.
#
# LOAD PROFILES
#   0  none: the binary's own concurrently running tests are the load (gate 4)
#   A  4 CPU spinners + 1 loop rerunning driverctl_race editor_race_smoke --exact
#   B  8 spinners + 2 such loops
#   C  12 spinners + 3 such loops
set -u

if [ "$#" -ne 4 ]; then
	echo "usage: bash $0 LABEL PROFILE MODE N" >&2
	exit 2
fi
LABEL=$1 PROFILE=$2 MODE=$3 N=$4
case "$LABEL" in before | after | probe) ;; *) echo "bad LABEL: $LABEL" >&2; exit 2 ;; esac
case "$PROFILE" in 0 | A | B | C) ;; *) echo "bad PROFILE: $PROFILE" >&2; exit 2 ;; esac
case "$MODE" in binary | exact) ;; *) echo "bad MODE: $MODE" >&2; exit 2 ;; esac
case "$N" in '' | *[!0-9]*) echo "bad N: $N" >&2; exit 2 ;; esac

QS8_TARGET=${QS8_TARGET:-driverctl_race}
QS8_FILTER=${QS8_FILTER:-two_writers_under_direction_lock_lose_nothing}
QS8_TMPFS=${QS8_TMPFS:-0}
QS8_KEEP_ALL=${QS8_KEEP_ALL:-0}

HERE=$(cd "$(dirname "$0")" && pwd)
REPO=$(git -C "$HERE" rev-parse --show-toplevel)
RESULTS="$HERE/stress-results.txt"
IMAGE=gsdmm-pre-tag-check:latest
TARGET_VOL=gsdmm-pretag-target
CARGO_VOL=gsdmm-pretag-cargo-home

# Tree identity, computed on the host BEFORE the run starts.
HEAD_SHORT=$(git -C "$REPO" rev-parse --short HEAD)
DIFF_HASH=$(git -C "$REPO" diff HEAD -- src tests Cargo.toml Cargo.lock | sha256sum | cut -c1-12)
TREE="$HEAD_SHORT+$DIFF_HASH"

TS=$(date -u +%Y%m%dT%H%M%SZ)
OUT="${TMPDIR:-/tmp}/qs8-stress/$LABEL-$PROFILE-$MODE-$TS"
mkdir -p "$OUT"
echo "qs8: label=$LABEL profile=$PROFILE mode=$MODE n=$N target=$QS8_TARGET tmpfs=$QS8_TMPFS tree=$TREE"
echo "qs8: output dir $OUT"

# Writable cache volumes? (named volumes are created root-owned; pre-tag-check
# section 4 chowns them). Probe as the host uid; chown only if needed.
if ! docker run --rm -u "$(id -u):$(id -g)" -v "$TARGET_VOL:/cargo-target" -v "$CARGO_VOL:/cargo-home" \
	"$IMAGE" bash -c 'touch /cargo-target/.qs8-w /cargo-home/.qs8-w && rm -f /cargo-target/.qs8-w /cargo-home/.qs8-w' 2>/dev/null; then
	echo "qs8: cache volumes not writable as $(id -u):$(id -g); chowning (mirrors pre-tag-check section 4)"
	docker run --rm -u 0:0 -v "$TARGET_VOL:/cargo-target" -v "$CARGO_VOL:/cargo-home" "$IMAGE" \
		bash -c 'chown -R "$1" /cargo-target /cargo-home' chown-script "$(id -u):$(id -g)" || exit 1
fi

# The inner script is a SINGLE-QUOTED CONSTANT; host values arrive as
# positional arguments. It writes one machine-readable tally to
# /stress-out/tally and prints a human-readable report.
INNER='
set -u
PROFILE=$1 MODE=$2 N=$3 TARGET=$4 FILTER=$5 TMPFS=$6 KEEP_ALL=$7
out=/stress-out

build_json=$(cargo test --test driverctl_race --test "$TARGET" --no-run --message-format=json 2>"$out/build.stderr") || {
	echo "qs8: BUILD FAILED"; tail -40 "$out/build.stderr"; exit 3; }
exe_of() {
	printf "%s\n" "$build_json" | jq -r --arg t "$1" \
		"select(.reason == \"compiler-artifact\" and .profile.test == true and .target.name == \$t) | .executable" | tail -1
}
EXE=$(exe_of "$TARGET")
LOAD_EXE=$(exe_of driverctl_race)
if [ -z "$EXE" ] || [ ! -x "$EXE" ] || [ -z "$LOAD_EXE" ]; then
	echo "qs8: could not resolve test executable (target=$TARGET exe=$EXE load=$LOAD_EXE)"; exit 3
fi
echo "qs8: exe $EXE"
if [ "$TMPFS" = 1 ]; then export TMPDIR=/dev/shm; fi
echo "qs8: nproc=$(nproc) uid=$(id -u) TMPDIR=${TMPDIR:-/tmp} fs=$(df -T "${TMPDIR:-/tmp}" | awk "NR==2{print \$2}")"

case "$PROFILE" in
	0) spin=0 loops=0 ;;
	A) spin=4 loops=1 ;;
	B) spin=8 loops=2 ;;
	C) spin=12 loops=3 ;;
esac
pids=""
cleanup() {
	for p in $pids; do kill "$p" 2>/dev/null; done
	for p in $pids; do wait "$p" 2>/dev/null; done
}
trap cleanup EXIT
trap "exit 130" INT TERM
i=0
while [ "$i" -lt "$spin" ]; do
	bash -c "while :; do :; done" &
	pids="$pids $!"
	i=$((i + 1))
done
i=0
while [ "$i" -lt "$loops" ]; do
	bash -c "while :; do \"\$0\" editor_race_smoke --exact -q >/dev/null 2>&1; done" "$LOAD_EXE" &
	pids="$pids $!"
	i=$((i + 1))
done
echo "qs8: load profile $PROFILE: $spin spinners, $loops smoke loops"

fail=0 other=0 tmo=0
it=1
while [ "$it" -le "$N" ]; do
	log="$out/iter-$it.log"
	if [ "$MODE" = binary ] && [ "$KEEP_ALL" = 1 ]; then
		# Keeping every log is for probes: show passing tests stdout too.
		timeout 120 "$EXE" --nocapture >"$log" 2>&1
	elif [ "$MODE" = binary ]; then
		timeout 120 "$EXE" >"$log" 2>&1
	else
		timeout 120 "$EXE" "$FILTER" --exact --nocapture >"$log" 2>&1
	fi
	rc=$?
	keep=$KEEP_ALL
	if [ "$rc" -eq 124 ]; then
		tmo=$((tmo + 1)); keep=1
		echo "qs8: iter $it TIMEOUT"
	elif [ "$rc" -ne 0 ]; then
		keep=1
		if grep -q "^test $FILTER \.\.\. FAILED" "$log"; then
			fail=$((fail + 1))
			echo "qs8: iter $it TARGET FAIL: $(grep -m1 "panicked at" -A1 "$log" | tr "\n" " ")"
		fi
		if grep "^test .* \.\.\. FAILED" "$log" | grep -vq "^test $FILTER \.\.\. FAILED"; then
			other=$((other + 1))
			echo "qs8: iter $it OTHER FAIL: $(grep "^test .* \.\.\. FAILED" "$log" | grep -v "^test $FILTER " | tr "\n" " ")"
		elif ! grep -q "^test $FILTER \.\.\. FAILED" "$log"; then
			other=$((other + 1))
			echo "qs8: iter $it NONZERO rc=$rc without a FAILED line (counted as other)"
		fi
	fi
	[ "$keep" = 1 ] || rm -f "$log"
	it=$((it + 1))
done
echo "fail=$fail other=$other timeout=$tmo" >"$out/tally"
echo "qs8: done fail=$fail other=$other timeout=$tmo"
'

docker run --rm \
	-u "$(id -u):$(id -g)" \
	--cpuset-cpus 0-3 \
	-e HOME=/home/runner \
	-e CARGO_INCREMENTAL=0 \
	-e CARGO_TARGET_DIR=/cargo-target \
	-e CARGO_HOME=/cargo-home \
	-v "$REPO:/work" \
	-w /work \
	-v "$TARGET_VOL:/cargo-target" \
	-v "$CARGO_VOL:/cargo-home" \
	-v "$OUT:/stress-out" \
	"$IMAGE" \
	bash -c "$INNER" stress-inner "$PROFILE" "$MODE" "$N" "$QS8_TARGET" "$QS8_FILTER" "$QS8_TMPFS" "$QS8_KEEP_ALL"
RC=$?

if [ ! -s "$OUT/tally" ]; then
	echo "qs8: no tally produced (container rc=$RC); nothing appended" >&2
	exit 1
fi
TALLY=$(cat "$OUT/tally")
LINE="$LABEL profile=$PROFILE mode=$MODE n=$N $TALLY tree=$TREE date=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
if [ "$QS8_TARGET" != driverctl_race ] || [ "$QS8_TMPFS" = 1 ]; then
	LINE="$LINE target=$QS8_TARGET tmpfs=$QS8_TMPFS"
fi
echo "$LINE" >>"$RESULTS"
echo "qs8: appended: $LINE"
echo "qs8: output dir $OUT"
