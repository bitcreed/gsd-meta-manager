#!/usr/bin/env bash
#
# pre-tag-check.sh — a LOCAL dry-run of .github/workflows/release.yml, runnable
# BEFORE the `v*` tag exists.
#
# It reproduces the gates that workflow runs on a tag push (tag/version match,
# MSRV compile, release build, test, clippy, package verify, and the
# cross-target check + clippy matrix) so a release can be rehearsed
# locally instead of discovered broken after the tag is already pushed.
#
# GATE ORDER IS DELIBERATELY INVERTED relative to the CI job graph. In
# release.yml the `msrv` job is a `needs:` prerequisite of `publish`, so CI only
# reaches the tag/version check after a full MSRV compile. Locally the tag check
# costs one `cargo metadata` call, so it runs FIRST and hard-stops: there is no
# point spending minutes of compilation to validate a tag the manifest does not
# carry. Do not "fix" this back to the CI ordering.
#
# .github/workflows/release.yml IS THE AUTHORITY. Wherever this script and that
# workflow disagree, the workflow is right and this script is stale.
#
# --container RUNS THESE SAME GATES INSIDE AN ubuntu-24.04 LOOKALIKE. A BARE
# local run cannot certify the publish job: this machine has GSD installed, so
# the test that has actually blocked publication — the router-conformance oracle
# test, which fails by design when the oracle is absent, as it is on a bare
# runner — passes here on precisely the trees CI rejects. Container mode builds
# scripts/pre-tag-check.Dockerfile, provisions the oracle in there with the
# same scripts/install-conformance-oracle.sh the publish job calls, and re-runs
# this script inside it.
#
# Environment:
#   PRE_TAG_CONTAINER_RUNTIME   force a specific container runtime instead of
#                               probing docker then podman. It exists for two
#                               reasons: unusual rootless setups, and so the
#                               no-usable-runtime failure path is TESTABLE
#                               (point it at a name that does not exist).
#   PRE_TAG_CHECK_IN_CONTAINER  set by the dispatch on the INNER run. Asking
#                               for --container while it is set is a hard error.
#   PRE_TAG_CONTAINER_CACHE_MAX_GB
#                               cap on EACH cache volume, in decimal GB (10^9
#                               bytes, as `docker system df` reports). Default
#                               20. Whole numbers only; 0 wipes both volumes on
#                               every run. A volume over the cap is wiped before
#                               the run, so that run is cold.
#   PRE_TAG_CONTAINER_CLEAN     1 is equivalent to --clean (0, empty or unset is
#                               off; anything else is a usage error).
#   CARGO_NET_OFFLINE           cargo's own variable, honoured by every cargo gate
#                               in a BARE run; set true to run gate 6 offline on a
#                               warm cache. It is not passed into the container
#                               (which forwards only explicit -e variables and
#                               needs the network anyway). Gate 6 deliberately has
#                               no --offline flag and no offline fallback: see the
#                               comment above it.
#
# STORAGE IS BOUNDED. --container used to grow Docker storage without limit
# (one cache volume reached 115 GB). Now:
#   - Ownership is the label gsdmm.pre-tag-check=1 PLUS the fixed volume names
#     gsdmm-pretag-target and gsdmm-pretag-cargo-home. Deletions go by exact name,
#     by the image repository, or by that label, never global.
#   - The cap is checked before EVERY run, inside the privileged prep step; a
#     volume over it is wiped, making that run cold.
#   - The inner run sets CARGO_INCREMENTAL=0, as CI does, so debug/incremental
#     stops growing.
#   - After each build the script prunes only its own DANGLING labelled images.
#   - The BuildKit builder cache cannot be scoped by label, so the script leaves
#     it alone; pruning it is the operator's global decision.
#   - gsdmm-target and gsdmm-cargo are NOT this script's. It never touches them.
#   - --clean (or PRE_TAG_CONTAINER_CLEAN=1) removes both volumes and every image
#     of the repository or carrying the label, then exits (or, with --container,
#     goes on to a cold run).
#
# Usage: ./scripts/pre-tag-check.sh [--container] [--clean] [vX.Y.Z]

# `set -u` only, on purpose:
#   - no `set -e`: gates 2-7 must ALL run and report, so the operator gets the
#     whole picture from one invocation rather than one failure at a time.
#   - no `pipefail`: the constant-extraction pipelines end in `head -1`, whose
#     SIGPIPE would be misread as a pipeline failure.
set -u

PROG="${0##*/}"

usage() {
	cat <<'USAGE'
Usage: ./scripts/pre-tag-check.sh [--container] [--clean] [TAG]

Runs, locally, the gates that .github/workflows/release.yml runs on a tag push.

Gates (run order):
  1  tag / Cargo.toml version   hard-stops on disagreement
  2  MSRV check                 cargo check on the declared rust-version floor
  3  cargo build --release
  4  cargo test --no-fail-fast
  5  cargo clippy -- -D warnings
  6  cargo package --locked     builds and verifies the packaged .crate the way
                                the publish job's `cargo publish --locked` does
                                before uploading. It never publishes and needs no
                                token.
  7  cross-target check+clippy  runs release.yml's cross-target-check job for
                                every target its matrix lists: `cargo check
                                --locked --target T <check_args>` and, when the
                                entry says clippy: true, `cargo clippy --locked
                                --target T --all-targets -- -D warnings`. The
                                target list is READ from release.yml at run time
                                (never copied here), and the gate refuses
                                (FAILED) a matrix or job it cannot model. A bare
                                run needs each target installed with rustup
                                (`rustup target add T`); a missing one is
                                SKIPPED, never installed for you. Inside
                                --container a missing target is FAILED, because
                                the image is built with exactly that list.

  TAG          optional, e.g. v1.7.1. When given, gate 1 asserts it matches the
               Cargo.toml version and hard-stops on disagreement. When omitted,
               gate 1 just reports the crate version and is marked SKIPPED.

  --container  run the gates inside an ubuntu-24.04 lookalike built from
               scripts/pre-tag-check.Dockerfile, after provisioning the GSD
               conformance oracle in there with the same
               scripts/install-conformance-oracle.sh the publish job calls.
               A green container run certifies the publish job in a way a bare
               local run cannot. Composes with TAG, in either order.

               If no container runtime is usable this FAILS LOUDLY and exits
               non-zero. There is deliberately NO fallback to a local run.

  --clean      remove the container storage this script created: the volumes
               gsdmm-pretag-target and gsdmm-pretag-cargo-home (by exact name)
               and every image in the gsdmm-pre-tag-check repository or carrying
               the label gsdmm.pre-tag-check=1. Equivalent to
               PRE_TAG_CONTAINER_CLEAN=1. On its own it exits 0 without running
               any gate (and takes no TAG). With --container it cleans first and
               then performs a COLD run. It never touches anything this script
               did not create.

Environment:
  PRE_TAG_CONTAINER_RUNTIME   force a runtime instead of probing docker,
                              then podman.
  PRE_TAG_CONTAINER_CACHE_MAX_GB
                              cap on EACH cache volume, in decimal GB. Default
                              20. Whole numbers only; 0 wipes both volumes on
                              every --container run. A volume over the cap is
                              wiped before the run, so that run is cold.
  PRE_TAG_CONTAINER_CLEAN     1 is equivalent to --clean (0, empty or unset is
                              off).
  CARGO_NET_OFFLINE           cargo's own variable, bare runs only: set true to
                              run gate 6 offline on a warm cache. Gate 6 has no
                              --offline flag and no offline fallback.

Exit status: 0 when no gate FAILED, 1 when any gate FAILED, 2 on bad usage.
USAGE
}

TAG_ARG=""
HAVE_TAG=0
CONTAINER=0
CLEAN=0
for arg in "$@"; do
	case "$arg" in
	-h | --help)
		usage
		exit 0
		;;
	--container)
		CONTAINER=1
		;;
	--clean)
		CLEAN=1
		;;
	-*)
		echo "$PROG: unknown option: $arg" >&2
		usage >&2
		exit 2
		;;
	*)
		if [ "$HAVE_TAG" -eq 1 ]; then
			echo "$PROG: too many arguments (expected at most one tag, got a second: $arg)" >&2
			usage >&2
			exit 2
		fi
		TAG_ARG="$arg"
		HAVE_TAG=1
		;;
	esac
done

case "${PRE_TAG_CONTAINER_CLEAN:-0}" in
0) ;;
1) CLEAN=1 ;;
*)
	echo "$PROG: PRE_TAG_CONTAINER_CLEAN must be 0 or 1, got: '${PRE_TAG_CONTAINER_CLEAN}'" >&2
	usage >&2
	exit 2
	;;
esac
# A bare run never touches container storage, so a TAG alongside a clean-only
# request is a mistake worth refusing rather than silently ignoring.
if [ "$CLEAN" -eq 1 ] && [ "$CONTAINER" -eq 0 ] && [ "$HAVE_TAG" -eq 1 ]; then
	echo "$PROG: --clean without --container takes no TAG (got: $TAG_ARG)" >&2
	usage >&2
	exit 2
fi

RULE="================================================================================"
THIN="--------------------------------------------------------------------------------"

# ===========================================================================
# release.yml cross-target-check MATRIX PARSER (gate 7, and the --container
# build arg)
#
# WHY THIS EXISTS. The v2.2.0 and v2.3.0 tags both died in release.yml's
# `cross-target-check` job: on x86_64-pc-windows-gnu clippy rejected a test
# helper as dead code, and no local gate could see it, because every other gate
# lints the HOST target only.
#
# DERIVE, NEVER COPY. The target list is READ from release.yml at run time. A
# copy kept in this script (or in the Dockerfile) would be a second source of
# truth that drifts the first time someone adds a target, and a drifted gate is
# a false certificate. The --container dispatch hands the same parsed list to
# the image build as a build arg, so the image cannot drift either.
#
# THE SYNC GUARD. Deriving the list is not enough: if CI's COMMANDS change (a
# new flag, a dropped --all-targets) or the matrix grows a key this gate does not
# model (exclude:, features:, os:), then the gate keeps passing while it no longer
# mirrors the job. So the parser REFUSES unless the job still contains, as fixed
# non-comment lines, exactly the three lines gate 7 models (Check run, the
# `if: matrix.clippy` condition, Clippy run), and REFUSES any matrix key other
# than the `include` entries' target / check_args / clippy. The gate is then
# updated deliberately, never silently stale. Comment lines are skipped
# everywhere, so a quoted copy in a comment can never satisfy the guard.
#
# Resolved from this script's OWN location, never cwd: the container dispatch
# runs before any cwd check, and the inner run may be started from anywhere.
#
# awk/grep/sed only: yq and python are not guaranteed in the image. Structure is
# checked in awk (portable to mawk, so no regex intervals there); value syntax is
# checked in bash below, where [[ =~ ]] has intervals.
# ===========================================================================
XT_SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
XT_WORKFLOW="$XT_SCRIPT_DIR/../.github/workflows/release.yml"
XT_CHECK_LINE='run: cargo check --locked --target "${{ matrix.target }}" ${{ matrix.check_args }}'
XT_IF_LINE='if: matrix.clippy'
XT_CLIPPY_LINE='run: cargo clippy --locked --target "${{ matrix.target }}" --all-targets -- -D warnings'

# Prints one `target<TAB>check_args<TAB>clippy` line per include entry on stdout.
# On refusal prints the reason on stderr and returns non-zero. An absent
# check_args is empty and an absent clippy is false: exactly how CI's
# `${{ matrix.check_args }}` and `if: matrix.clippy` treat a missing key.
xt_parse_matrix() {
	local yml="$1"
	if [ ! -f "$yml" ]; then
		echo "cannot read $yml" >&2
		return 1
	fi
	awk -v CHECK="$XT_CHECK_LINE" -v IFL="$XT_IF_LINE" -v CLIP="$XT_CLIPPY_LINE" '
function trim(s) { gsub(/^[ \t\r]+|[ \t\r]+$/, "", s); return s }
function unq(s) {
	s = trim(s)
	if (length(s) >= 2 && substr(s, 1, 1) == "\"" && substr(s, length(s), 1) == "\"") {
		s = substr(s, 2, length(s) - 2)
	}
	return s
}
function refuse(msg) { print "release.yml cross-target-check: " msg > "/dev/stderr"; failed = 1; exit 1 }
BEGIN { injob = 0; seen = 0; inmatrix = 0; inc = 0; n = 0; nchk = 0; nif = 0; nclip = 0; mchild = -1; failed = 0 }
{
	t = trim($0)
	if (t == "" || substr(t, 1, 1) == "#") next
	match($0, /^ */); ind = RLENGTH
	if (!injob) {
		if (!seen && $0 ~ /^  cross-target-check:[ \t\r]*$/) { injob = 1; seen = 1 }
		next
	}
	# The job ends at the next 2-space-indent (or shallower) non-comment key.
	if (ind <= 2) { injob = 0; inmatrix = 0; inc = 0; next }

	if (t == CHECK) nchk++
	else if (t == IFL) nif++
	else if (t == CLIP) nclip++

	if (t == "matrix:") { inmatrix = 1; mind = ind; mchild = -1; inc = 0; next }
	if (inmatrix && ind <= mind) { inmatrix = 0; inc = 0 }
	if (inmatrix) {
		if (mchild < 0) mchild = ind
		if (ind == mchild) {
			k = t; sub(/:.*$/, "", k)
			if (k != "include") refuse("matrix key \"" k "\" is not modelled by gate 7 (only include is)")
			inc = 1
			next
		}
		if (inc) {
			if (substr(t, 1, 2) == "- ") { body = trim(substr(t, 3)); fresh = 1 } else { body = t; fresh = 0 }
			c = index(body, ":")
			if (c == 0) refuse("unparseable include line: " t)
			k = trim(substr(body, 1, c - 1)); v = unq(substr(body, c + 1))
			if (fresh) {
				if (k != "target") refuse("include entry starts with key \"" k "\", expected \"- target:\"")
				n++; tgt[n] = v; chk[n] = ""; clp[n] = "false"
			} else {
				if (n == 0) refuse("key \"" k "\" appears before any \"- target:\" entry")
				if (k == "check_args") chk[n] = v
				else if (k == "clippy") clp[n] = v
				else refuse("matrix key \"" k "\" is not modelled by gate 7 (only target, check_args, clippy are)")
			}
		}
	}
}
END {
	if (failed) exit 1
	if (!seen) refuse("no cross-target-check job found")
	if (n == 0) refuse("zero matrix include entries found")
	if (nchk != 1) refuse("the job no longer has exactly one Check line gate 7 models: " CHECK)
	if (nif != 1) refuse("the job no longer has exactly one condition line gate 7 models: " IFL)
	if (nclip != 1) refuse("the job no longer has exactly one Clippy line gate 7 models: " CLIP)
	for (i = 1; i <= n; i++) printf "%s\t%s\t%s\n", tgt[i], chk[i], clp[i]
}
' "$yml"
}

# Parses AND validates, filling XT_TARGETS / XT_ARGS / XT_CLIPPY (parallel
# arrays). Values reach cargo argv and a docker build arg, so each one is
# validated against a strict shape; a refusal is a refusal, never a fallback.
xt_load() {
	XT_TARGETS=()
	XT_ARGS=()
	XT_CLIPPY=()
	local out line tgt rest chk clp tok toks
	local tgt_re='^[a-z0-9_]+(-[a-z0-9_.]+){2,3}$'
	local flag_re='^--?[A-Za-z0-9][A-Za-z0-9_-]*$'
	if ! out=$(xt_parse_matrix "$XT_WORKFLOW"); then
		return 1
	fi
	while IFS= read -r line; do
		[ -n "$line" ] || continue
		# Split by parameter expansion, not `read`: an empty check_args between two
		# tabs would collapse under IFS=tab and shift the fields.
		tgt=${line%%$'\t'*}
		rest=${line#*$'\t'}
		chk=${rest%%$'\t'*}
		clp=${rest#*$'\t'}
		if ! [[ $tgt =~ $tgt_re ]]; then
			echo "release.yml cross-target-check: invalid target \"$tgt\"" >&2
			return 1
		fi
		toks=()
		read -ra toks <<<"$chk"
		for tok in "${toks[@]}"; do
			if ! [[ $tok =~ $flag_re ]]; then
				echo "release.yml cross-target-check: invalid check_args token \"$tok\" for $tgt" >&2
				return 1
			fi
		done
		if [ "$clp" != "true" ] && [ "$clp" != "false" ]; then
			echo "release.yml cross-target-check: clippy must be true or false for $tgt, got \"$clp\"" >&2
			return 1
		fi
		XT_TARGETS+=("$tgt")
		XT_ARGS+=("$chk")
		XT_CLIPPY+=("$clp")
	done <<<"$out"
	if [ "${#XT_TARGETS[@]}" -eq 0 ]; then
		echo "release.yml cross-target-check: zero matrix entries" >&2
		return 1
	fi
	return 0
}

# ===========================================================================
# --container DISPATCH
#
# PLACEMENT IS DELIBERATE: after RULE/THIN (this block shouts with them) and
# BEFORE the EXIT trap is installed further down. That trap prints a seven-gate
# summary describing gates that never ran in THIS process — on a container
# dispatch every line of it would read NOT REACHED, a confusing lie stapled to
# the end of a real run. The INNER run prints its own banner and its own
# summary, streamed straight through to this terminal, and that is the only
# summary a --container invocation should show. So this block must exit before
# the trap exists.
#
# Nothing below this block is reachable in container mode, and nothing in it
# runs at all in bare mode.
# ===========================================================================
if [ "$CONTAINER" -eq 1 ] || [ "$CLEAN" -eq 1 ]; then
	# --- 1. RECURSION GUARD -------------------------------------------------
	# The dispatch never passes --container inward, so this is belt-and-braces.
	# It is explicit anyway because an accidental recursion in a script that
	# builds images and runs ~2100 tests is expensive enough to be worth a
	# named variable and a hard stop.
	if [ -n "${PRE_TAG_CHECK_IN_CONTAINER:-}" ]; then
		echo "$RULE" >&2
		if [ "$CONTAINER" -eq 1 ]; then
			RECURSION_ASKED="--container"
		else
			RECURSION_ASKED="--clean"
		fi
		echo "HARD ERROR: $RECURSION_ASKED requested while PRE_TAG_CHECK_IN_CONTAINER is set." >&2
		echo "That variable means this process is ALREADY the in-container run, so" >&2
		echo "honouring the flag would drive the container runtime from inside a" >&2
		echo "container (build an image, start the whole gate suite again, or delete" >&2
		echo "the cache volumes under a live run). Drop the flag: the inner run is a" >&2
		echo "bare run." >&2
		echo "$RULE" >&2
		exit 2
	fi

	# --- 1b. CACHE CAP VALIDATION ------------------------------------------
	# A USAGE error, so it runs before ANY side effect (runtime probe, build,
	# volume operation). The value ends up in a script that runs as root and
	# deletes files, so it is validated down to 1-9 decimal digits here and is
	# later handed over as a positional argument, never spliced into script text.
	#
	# Consumed only by --container: a bare run never touches container storage.
	if [ "$CONTAINER" -eq 1 ]; then
		CACHE_MAX_GB_RAW="${PRE_TAG_CONTAINER_CACHE_MAX_GB:-}"
		[ -n "$CACHE_MAX_GB_RAW" ] || CACHE_MAX_GB_RAW=20
		case "$CACHE_MAX_GB_RAW" in
		*[!0-9]*)
			CACHE_MAX_GB_BAD=1
			;;
		*)
			CACHE_MAX_GB_BAD=0
			[ "${#CACHE_MAX_GB_RAW}" -le 9 ] || CACHE_MAX_GB_BAD=1
			;;
		esac
		if [ "$CACHE_MAX_GB_BAD" -eq 1 ]; then
			echo "$PROG: PRE_TAG_CONTAINER_CACHE_MAX_GB must be a whole number of GB (1-9 decimal digits), got: '$CACHE_MAX_GB_RAW'" >&2
			usage >&2
			exit 2
		fi
		# 10# forces base 10: a value like 08 must never be read as (invalid) octal.
		# GB is 10^9 bytes, the unit `docker system df` reports in. 0 falls out as
		# "wipe every run": du of even an empty volume is more than 0 bytes.
		CACHE_MAX_GB=$((10#$CACHE_MAX_GB_RAW))
		CACHE_CAP_BYTES=$((CACHE_MAX_GB * 1000000000))
	fi

	# --- 1c. CROSS-TARGET MATRIX -------------------------------------------
	# Gate 7 runs inside the container against rustup targets baked into the
	# image, and the image gets its target list from release.yml through a build
	# arg. So the matrix must be derived (and found modelable) BEFORE any side
	# effect: a refusal here is a HARD ERROR, because building an image around a
	# matrix this script cannot model would hand back a green run that certifies
	# nothing about the cross-target-check job (the v2.2.0/v2.3.0 failure).
	if [ "$CONTAINER" -eq 1 ]; then
		if ! xt_load; then
			echo "$RULE" >&2
			echo "HARD ERROR: could not derive the cross-target matrix from release.yml (see above)." >&2
			echo "THIS RUN HAS CERTIFIED NOTHING. No gate ran." >&2
			echo "$THIN" >&2
			echo "Gate 7 refuses a matrix or job it cannot model: update gate 7 in" >&2
			echo "scripts/pre-tag-check.sh together with release.yml, then re-run." >&2
			echo "$RULE" >&2
			exit 1
		fi
		CONTAINER_CROSS_TARGETS="${XT_TARGETS[*]}"
	fi

	# --- 2. RUNTIME RESOLUTION ---------------------------------------------
	# A runtime BINARY whose daemon does not answer is not a usable runtime, so
	# probe `<runtime> info` and not merely `command -v`.
	if [ -n "${PRE_TAG_CONTAINER_RUNTIME:-}" ]; then
		RUNTIME_CANDIDATES="$PRE_TAG_CONTAINER_RUNTIME"
	else
		RUNTIME_CANDIDATES="docker podman"
	fi
	RUNTIME=""
	for candidate in $RUNTIME_CANDIDATES; do
		if command -v "$candidate" >/dev/null 2>&1 && "$candidate" info >/dev/null 2>&1; then
			RUNTIME="$candidate"
			break
		fi
	done

	if [ -z "$RUNTIME" ]; then
		echo "$RULE" >&2
		echo "HARD ERROR: no usable container runtime." >&2
		echo "$THIN" >&2
		echo "Tried: $RUNTIME_CANDIDATES" >&2
		echo "(a runtime counts as usable only when its binary is on PATH AND" >&2
		echo " \`<runtime> info\` succeeds — an unreachable daemon is not a runtime)." >&2
		echo "$THIN" >&2
		if [ "$CONTAINER" -eq 0 ]; then
			echo "--clean REMOVED NOTHING: there is no runtime to remove anything with." >&2
			echo "Start a container runtime, or set PRE_TAG_CONTAINER_RUNTIME to one that" >&2
			echo "works, and re-run." >&2
		else
			echo "THIS RUN HAS CERTIFIED NOTHING. No gate ran." >&2
			echo >&2
			echo "There is deliberately NO FALLBACK to a local run. A silent fallback" >&2
			echo "would recreate the exact blind spot --container exists to close: a" >&2
			echo "green local gate on the one thing CI is red on. This machine has GSD" >&2
			echo "installed, so the router-conformance oracle test, which has actually" >&2
			echo "blocked publication, passes here on the trees CI rejects." >&2
			echo >&2
			echo "Start a container runtime, or set PRE_TAG_CONTAINER_RUNTIME to one that" >&2
			echo "works, and re-run. To knowingly run the weaker local gates instead, drop" >&2
			echo "--container; a green bare run does not certify the publish job." >&2
		fi
		echo "$RULE" >&2
		exit 1
	fi

	# Resolve the repo root from this script's OWN location. The container
	# dispatch runs BEFORE the run-from-the-repo-root check further down, so it
	# cannot lean on cwd to decide what to bind-mount.
	CONTAINER_SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
	CONTAINER_REPO_ROOT=$(cd -- "$CONTAINER_SCRIPT_DIR/.." && pwd)
	CONTAINER_IMAGE_REPO="gsdmm-pre-tag-check"
	CONTAINER_IMAGE="$CONTAINER_IMAGE_REPO:latest"
	CONTAINER_TARGET_VOL="gsdmm-pretag-target"
	CONTAINER_CARGO_VOL="gsdmm-pretag-cargo-home"
	# OWNERSHIP. Every image this script builds and every volume it creates
	# carries this one label, held in one variable so build, volume create and
	# prune cannot drift apart. The two volumes are ALSO owned by their FIXED
	# NAMES: volumes created before the label existed carry none, and a volume
	# cannot be relabelled. So every deletion goes by exact name or by label,
	# never global. gsdmm-target and gsdmm-cargo are NOT ours; never touch them.
	CONTAINER_LABEL="gsdmm.pre-tag-check=1"

	# --- 2b. CLEAN ----------------------------------------------------------
	# Remove ONLY what this script owns: the two volumes by EXACT name, and every
	# image of the gsdmm-pre-tag-check repository (all tags) or carrying the
	# label. Never a global prune, never any other volume name (gsdmm-target and
	# gsdmm-cargo are NOT ours). Anything absent is reported and tolerated;
	# anything PRESENT that cannot be removed (e.g. in use by a running container)
	# is a HARD ERROR and no run follows. Runs AFTER cap validation, so a bad cap
	# never wipes and then fails.
	if [ "$CLEAN" -eq 1 ]; then
		echo "$RULE"
		echo "!!! CLEAN !!!"
		echo "$THIN"
		echo "  runtime : $RUNTIME"
		echo "$THIN"
		CLEAN_FAILED=""
		for vol in "$CONTAINER_TARGET_VOL" "$CONTAINER_CARGO_VOL"; do
			if ! "$RUNTIME" volume inspect "$vol" >/dev/null 2>&1; then
				echo "  volume $vol: already absent"
			elif "$RUNTIME" volume rm "$vol" >/dev/null; then
				echo "  volume $vol: removed"
			else
				echo "  volume $vol: REMOVAL FAILED" >&2
				CLEAN_FAILED="$CLEAN_FAILED volume:$vol"
			fi
		done
		CLEAN_IMAGE_IDS=$(
			{
				"$RUNTIME" image ls -q "$CONTAINER_IMAGE_REPO"
				"$RUNTIME" image ls -q --filter "label=$CONTAINER_LABEL"
			} 2>/dev/null | sort -u
		)
		if [ -z "$CLEAN_IMAGE_IDS" ]; then
			echo "  images: no image present"
		else
			for img in $CLEAN_IMAGE_IDS; do
				# -f: one ID may carry several tags, which a plain rm refuses.
				if "$RUNTIME" image rm -f "$img" >/dev/null; then
					echo "  image $img: removed"
				else
					echo "  image $img: REMOVAL FAILED" >&2
					CLEAN_FAILED="$CLEAN_FAILED image:$img"
				fi
			done
		fi
		if [ -n "$CLEAN_FAILED" ]; then
			echo "$RULE" >&2
			echo "HARD ERROR: could not remove:$CLEAN_FAILED" >&2
			echo "Something may still be using it (a running container?). No run follows." >&2
			echo "$RULE" >&2
			exit 1
		fi
		if [ "$CONTAINER" -eq 0 ]; then
			echo "$THIN"
			echo "  clean complete; no gate was run (see --help)."
			echo "$RULE"
			exit 0
		fi
		echo "$THIN"
		echo "  clean complete; continuing to a COLD container run."
		echo "$RULE"
		echo
	fi

	echo "$RULE"
	echo "!!! CONTAINER MODE !!!"
	echo "$THIN"
	echo "  runtime    : $RUNTIME"
	echo "  image      : $CONTAINER_IMAGE"
	echo "  dockerfile : $CONTAINER_SCRIPT_DIR/pre-tag-check.Dockerfile"
	echo "  repo       : $CONTAINER_REPO_ROOT -> /work"
	echo "  cache cap  : $CACHE_MAX_GB GB per volume (PRE_TAG_CONTAINER_CACHE_MAX_GB)"
	echo "  cross tgts : $CONTAINER_CROSS_TARGETS (from release.yml cross-target-check)"
	echo "$THIN"
	echo "  The gates below run inside an ubuntu-24.04 lookalike, after the SAME"
	echo "  scripts/install-conformance-oracle.sh the publish job calls has"
	echo "  provisioned the conformance oracle in there. What you are about to read"
	echo "  is the inner run's own banner and summary."
	echo "$RULE"
	echo

	# --- 3. BUILD ----------------------------------------------------------
	# Build on EVERY invocation. The layer cache makes an unchanged Dockerfile
	# near-instant, and a stale image is a lie about what was tested.
	# The build context is scripts/ and the Dockerfile copies nothing from it:
	# the repo arrives as a bind mount at run time, not as an image layer.
	#
	# PRE_TAG_CROSS_TARGETS is the list step 1c derived from release.yml. The
	# Dockerfile keeps no copy of it, so the image installs exactly the targets
	# gate 7 will ask for; the layer cache keys on the arg value.
	echo "+ $RUNTIME build --label $CONTAINER_LABEL --build-arg PRE_TAG_CROSS_TARGETS=\"$CONTAINER_CROSS_TARGETS\" -t $CONTAINER_IMAGE -f pre-tag-check.Dockerfile $CONTAINER_SCRIPT_DIR"
	if ! "$RUNTIME" build \
		--label "$CONTAINER_LABEL" \
		--build-arg "PRE_TAG_CROSS_TARGETS=$CONTAINER_CROSS_TARGETS" \
		-t "$CONTAINER_IMAGE" \
		-f "$CONTAINER_SCRIPT_DIR/pre-tag-check.Dockerfile" \
		"$CONTAINER_SCRIPT_DIR"; then
		echo "$RULE" >&2
		echo "HARD ERROR: the image build failed. THIS RUN HAS CERTIFIED NOTHING." >&2
		echo "No fallback to a local run; see --help." >&2
		echo "$RULE" >&2
		exit 1
	fi
	echo

	# Reclaim OUR superseded images: a rebuild after a Dockerfile edit re-tags
	# :latest and leaves the previous image dangling. Scoped to dangling images
	# carrying our label (no -a), so nothing of anyone else's can match.
	# The BuildKit builder cache is deliberately NOT pruned: its prune has no
	# label filter and would hit every other project's cache on this machine.
	# Reclaiming it is the operator's global decision, not this script's.
	echo "+ $RUNTIME image prune -f --filter label=$CONTAINER_LABEL"
	if ! "$RUNTIME" image prune -f --filter "label=$CONTAINER_LABEL"; then
		echo "WARNING: pruning this script's dangling images failed; continuing." >&2
	fi
	echo

	# --- 4. VOLUME BOOTSTRAP + CACHE CAP -----------------------------------
	# Two stable named volumes so the cargo cache survives between runs — a cold
	# run is three profiles compiled from scratch. Nothing ever reclaimed them, so
	# they are now BOUNDED: each is measured before every run and wiped when over
	# the cap (the next build is then cold).
	#
	# Inspect-then-create (not a blind create): podman's create errors on an
	# existing name, while docker silently ignores labels on an existing one.
	for vol in "$CONTAINER_TARGET_VOL" "$CONTAINER_CARGO_VOL"; do
		if ! "$RUNTIME" volume inspect "$vol" >/dev/null 2>&1; then
			if ! "$RUNTIME" volume create --label "$CONTAINER_LABEL" "$vol" >/dev/null; then
				echo "$RULE" >&2
				echo "HARD ERROR: could not create the cache volume $vol." >&2
				echo "THIS RUN HAS CERTIFIED NOTHING; there is no fallback to a local run." >&2
				echo "$RULE" >&2
				exit 1
			fi
		fi
	done

	# Named volumes are created ROOT-OWNED, and the gate run is unprivileged, so
	# the privileged step also chowns them. It runs the SAME freshly-built image
	# (deliberately: no second image to pull, audit or keep current), mounts ONLY
	# the two volumes and never the repo.
	#
	# The script is a SINGLE-QUOTED CONSTANT. Host values (cap, uid:gid, the
	# name=mountpoint pairs) arrive as positional arguments and are never spliced
	# into its text. Report mode never reaches the delete path, and the post-run
	# report additionally mounts the volumes :ro as a backstop.
	CONTAINER_PREP_SCRIPT='
set -eu
mode=$1
cap=$2
owner=$3
shift 3
cap_h=$(numfmt --to=si "$cap")
for pair in "$@"; do
	name=${pair%%=*}
	mount=${pair#*=}
	bytes=$(du -sb "$mount" | cut -f1)
	human=$(numfmt --to=si "$bytes")
	if [ "$mode" = report ]; then
		echo "post-run cache size: $name  $human ($bytes bytes)"
		if [ "$bytes" -gt "$cap" ]; then
			echo "  over the cap of $cap_h: it will be wiped at the start of the next --container run"
		fi
		continue
	fi
	if [ "$bytes" -gt "$cap" ]; then
		echo "cache volume $name: $human ($bytes bytes), cap $cap_h -> WIPING"
		echo "  !!! contents exceed the cap, so this volume is wiped and the next build is COLD !!!"
		find "$mount" -mindepth 1 -delete
		after=$(du -sb "$mount" | cut -f1)
		echo "  after wipe: $(numfmt --to=si "$after") ($after bytes)"
	else
		echo "cache volume $name: $human ($bytes bytes), cap $cap_h -> KEPT"
	fi
done
if [ "$mode" = prep ]; then
	for pair in "$@"; do
		chown -R "$owner" "${pair#*=}"
	done
fi
'
	if ! "$RUNTIME" run --rm -u 0:0 \
		-v "$CONTAINER_TARGET_VOL:/cargo-target" \
		-v "$CONTAINER_CARGO_VOL:/cargo-home" \
		"$CONTAINER_IMAGE" \
		bash -c "$CONTAINER_PREP_SCRIPT" prep-script prep "$CACHE_CAP_BYTES" "$(id -u):$(id -g)" \
		"$CONTAINER_TARGET_VOL=/cargo-target" "$CONTAINER_CARGO_VOL=/cargo-home"; then
		echo "$RULE" >&2
		echo "HARD ERROR: the cache-volume prep step (size check, cap wipe, chown to $(id -u):$(id -g)) failed." >&2
		echo "The gate run is unprivileged and would fail to write its build cache." >&2
		echo "THIS RUN HAS CERTIFIED NOTHING; there is no fallback to a local run." >&2
		echo "$RULE" >&2
		exit 1
	fi

	# --- 5. RUN ------------------------------------------------------------
	# The inner command mirrors the publish job INCLUDING its provisioning step.
	# That is precisely what makes a green container run mean a green publish
	# job: same script, same order, same absent-until-provisioned oracle.
	CONTAINER_INNER="./scripts/install-conformance-oracle.sh && ./scripts/pre-tag-check.sh"
	if [ "$HAVE_TAG" -eq 1 ]; then
		CONTAINER_INNER="$CONTAINER_INNER $(printf '%q' "$TAG_ARG")"
	fi

	# --cpuset-cpus 0-3 so `nproc` reports 4 in there, like the runner: test
	# parallelism is one of the few things that changes which flakes surface.
	#
	# A DEDICATED CARGO_TARGET_DIR IS REQUIRED. Sharing the host's target/ would
	# thrash the developer's build cache between two different environments,
	# with every subsequent local build paying for this run.
	#
	# The /work bind mount is READ-WRITE, so gates 3-5 can touch Cargo.lock if
	# it is stale. That surfaces in `git status`, and it is arguably what you
	# want to find out before tagging — CI would fail on it too. Read-only
	# would simply break the three gates that legitimately compile.
	#
	# CARGO_INCREMENTAL=0 mirrors CI: release.yml installs its toolchains with
	# dtolnay/rust-toolchain (lines 25, 84, 108), which exports it into the job.
	# It also stops debug/incremental (3.5 GB measured) from regrowing.
	"$RUNTIME" run --rm \
		-u "$(id -u):$(id -g)" \
		--cpuset-cpus 0-3 \
		-e HOME=/home/runner \
		-e CARGO_INCREMENTAL=0 \
		-e CARGO_TARGET_DIR=/cargo-target \
		-e CARGO_HOME=/cargo-home \
		-e PRE_TAG_CHECK_IN_CONTAINER=1 \
		-v "$CONTAINER_REPO_ROOT:/work" \
		-w /work \
		-v "$CONTAINER_TARGET_VOL:/cargo-target" \
		-v "$CONTAINER_CARGO_VOL:/cargo-home" \
		"$CONTAINER_IMAGE" \
		bash -c "$CONTAINER_INNER"
	CONTAINER_RC=$?

	# Evidence only: report each volume's size after the run. Read-only mounts,
	# report mode, and a failure here is a WARNING that never alters CONTAINER_RC.
	echo
	if ! "$RUNTIME" run --rm -u 0:0 \
		-v "$CONTAINER_TARGET_VOL:/cargo-target:ro" \
		-v "$CONTAINER_CARGO_VOL:/cargo-home:ro" \
		"$CONTAINER_IMAGE" \
		bash -c "$CONTAINER_PREP_SCRIPT" prep-script report "$CACHE_CAP_BYTES" - \
		"$CONTAINER_TARGET_VOL=/cargo-target" "$CONTAINER_CARGO_VOL=/cargo-home"; then
		echo "WARNING: could not measure the cache volumes after the run." >&2
	fi

	echo
	echo "$RULE"
	echo "!!! CONTAINER MODE — inner run exited $CONTAINER_RC !!!"
	echo "  The summary above is the inner run's. This process ran no gates of its"
	echo "  own and propagates that status verbatim."
	echo "$RULE"
	exit "$CONTAINER_RC"
fi

# ---------------------------------------------------------------------------
# State consumed by the summary. Everything the EXIT trap reads must be
# initialised BEFORE the trap is installed, because the trap also fires on
# gate 1's hard stop.
#
# "NOT REACHED" is the honest initial value: an early exit must not report a
# gate as skipped (a judgement) when it was simply never run.
# ---------------------------------------------------------------------------
G1_STATUS="NOT REACHED"
G2_STATUS="NOT REACHED"
G3_STATUS="NOT REACHED"
G4_STATUS="NOT REACHED"
G5_STATUS="NOT REACHED"
G6_STATUS="NOT REACHED"
G7_STATUS="NOT REACHED"
G1_NOTE=""
G2_NOTE=""
G3_NOTE=""
G4_NOTE=""
G5_NOTE=""
G6_NOTE=""
G7_NOTE=""

LOG_DIR=""

gate_line() {
	# $1 = number, $2 = short name, $3 = status, $4 = note
	if [ -n "$4" ]; then
		printf '  gate %s  %-26s %-11s (%s)\n' "$1" "$2" "$3" "$4"
	else
		printf '  gate %s  %-26s %s\n' "$1" "$2" "$3"
	fi
}

print_summary() {
	echo
	echo "$RULE"
	echo "=== PRE-TAG SUMMARY ==="
	echo "$THIN"
	gate_line 1 "tag / Cargo.toml version" "$G1_STATUS" "$G1_NOTE"
	gate_line 2 "MSRV check" "$G2_STATUS" "$G2_NOTE"
	gate_line 3 "cargo build --release" "$G3_STATUS" "$G3_NOTE"
	gate_line 4 "cargo test" "$G4_STATUS" "$G4_NOTE"
	gate_line 5 "cargo clippy -D warnings" "$G5_STATUS" "$G5_NOTE"
	gate_line 6 "cargo package --locked" "$G6_STATUS" "$G6_NOTE"
	gate_line 7 "cross-target check+clippy" "$G7_STATUS" "$G7_NOTE"
	echo "$THIN"
	if [ -n "$LOG_DIR" ]; then
		echo "  gate logs: $LOG_DIR"
	else
		echo "  gate logs: none created (exited before any gate ran)"
	fi

	echo "$THIN"
	echo "  The exit code reflects GATE FAILURES ONLY. SKIPPED is not a failure."
	echo "$RULE"
}

trap 'rc=$?; print_summary; exit "$rc"' EXIT

LOG_DIR=$(mktemp -d "${TMPDIR:-/tmp}/pre-tag-check.XXXXXXXX")
echo "Gate logs will be written under: $LOG_DIR"
echo

run_gate() {
	# $1 = log file basename, $2.. = command to run.
	# The exit status comes from PIPESTATUS[0] — the command's own status — and
	# never from tee, which succeeds whatever the command did.
	local logfile="$LOG_DIR/$1"
	shift
	echo "+ $*"
	"$@" 2>&1 | tee "$logfile"
	return "${PIPESTATUS[0]}"
}

section() {
	echo
	echo "$THIN"
	echo ">>> $*"
	echo "$THIN"
}

# ---------------------------------------------------------------------------
# GATE 1 — tag / Cargo.toml version
# Mirrors the publish job's "Check tag matches Cargo.toml version" step,
# including the same ${VAR#v} leading-v strip.
# ---------------------------------------------------------------------------
section "GATE 1 — tag vs Cargo.toml version"
CARGO_META=$(cargo metadata --no-deps --format-version 1 2>/dev/null)
CRATE_VERSION=$(printf '%s' "$CARGO_META" | jq -r '.packages[0].version' 2>/dev/null)

if [ -z "$CRATE_VERSION" ] || [ "$CRATE_VERSION" = "null" ]; then
	G1_STATUS="FAILED"
	G1_NOTE="could not read a version from cargo metadata"
	echo "FAILED: cargo metadata did not yield .packages[0].version."
elif [ "$HAVE_TAG" -eq 0 ]; then
	G1_STATUS="SKIPPED"
	G1_NOTE="no tag argument"
	echo "Crate version (Cargo.toml): $CRATE_VERSION"
	echo "SKIPPED: no tag argument given, so there is nothing to compare it to."
	echo "         Re-run as: ./scripts/pre-tag-check.sh v$CRATE_VERSION"
else
	TAG_VERSION="${TAG_ARG#v}"
	echo "Tag argument      : $TAG_ARG  (version part: $TAG_VERSION)"
	echo "Crate version     : $CRATE_VERSION"
	if [ "$TAG_VERSION" = "$CRATE_VERSION" ]; then
		G1_STATUS="PASS"
		echo "PASS: the tag names the version this manifest carries."
	else
		G1_STATUS="FAILED"
		G1_NOTE="tag $TAG_ARG != crate $CRATE_VERSION"
		echo
		echo "FAILED: tag $TAG_ARG does not match Cargo.toml version $CRATE_VERSION."
		echo "HARD STOP. Everything after this gate is minutes of compilation that"
		echo "cannot mean anything while the tag names a version the manifest does not"
		echo "carry. Fix Cargo.toml (release step 1) or the tag, then re-run."
		exit 1
	fi
fi

# ---------------------------------------------------------------------------
# GATE 2 — MSRV
# Mirrors the `msrv` job. The floor is DERIVED from cargo metadata's
# rust_version — the same field that job reads — so it cannot go stale when the
# floor moves.
# ---------------------------------------------------------------------------
section "GATE 2 — MSRV (declared floor compiles)"
MANIFEST_MSRV=$(printf '%s' "$CARGO_META" | jq -r '.packages[0].rust_version' 2>/dev/null)

if [ -z "$MANIFEST_MSRV" ] || [ "$MANIFEST_MSRV" = "null" ]; then
	# CI errors out in this case too: a floor that does not exist cannot be
	# certified. Not a skip.
	G2_STATUS="FAILED"
	G2_NOTE="Cargo.toml declares no rust-version"
	echo "FAILED: Cargo.toml declares no [package] rust-version; there is no floor"
	echo "        to certify. The msrv job in release.yml errors out on this too."
elif ! command -v rustup >/dev/null 2>&1; then
	G2_STATUS="SKIPPED"
	G2_NOTE="rustup not on PATH"
	echo "Declared floor: $MANIFEST_MSRV"
	echo "SKIPPED: rustup is not on PATH, so the floor toolchain cannot be selected."
else
	echo "Declared floor (Cargo.toml rust-version): $MANIFEST_MSRV"
	# Select the toolchain BY NAME from the installed list. Match the declared
	# floor followed by '.', '-', or end-of-name, because the floor may be
	# declared two-component ("1.88") while the installed toolchain may be
	# spelled either "1.88-<triple>" or "1.88.0-<triple>". Take the first field
	# of each line: rustup annotates the active/default entry.
	#
	# NEVER probe with `cargo +<floor> --version` to test for presence: rustup
	# AUTO-DOWNLOADS AND INSTALLS a missing toolchain on that probe. That both
	# lies about the skip condition (it can no longer be reached) and mutates
	# the developer's machine as a side effect of a read-only-looking check.
	# Detection here is read-only: `rustup toolchain list` and nothing else.
	FLOOR_ESCAPED=$(printf '%s' "$MANIFEST_MSRV" | sed -E 's/\./\\./g')
	MSRV_TOOLCHAIN=$(rustup toolchain list 2>/dev/null | awk '{print $1}' | grep -E "^${FLOOR_ESCAPED}([.-]|$)" | head -1)
	if [ -z "$MSRV_TOOLCHAIN" ]; then
		G2_STATUS="SKIPPED"
		G2_NOTE="no installed toolchain for floor $MANIFEST_MSRV"
		echo
		echo "  >>> SKIPPED — THIS GATE DID NOT PASS. <<<"
		echo "  No installed rustup toolchain matches the declared floor $MANIFEST_MSRV."
		echo "  Nothing was installed (deliberately). To actually run this gate:"
		echo "      rustup toolchain install $MANIFEST_MSRV"
		echo "  Until then CI's msrv job is the only thing checking the floor."
	else
		echo "Using installed toolchain: $MSRV_TOOLCHAIN"
		if run_gate "gate2-msrv.log" cargo +"$MSRV_TOOLCHAIN" check --all-targets --locked; then
			G2_STATUS="PASS"
		else
			G2_STATUS="FAILED"
			G2_NOTE="cargo +$MSRV_TOOLCHAIN check failed"
		fi
	fi
fi

# ---------------------------------------------------------------------------
# GATE 3 — release build (publish job's "Build" step)
# ---------------------------------------------------------------------------
section "GATE 3 — cargo build --release"
if run_gate "gate3-build.log" cargo build --release; then
	G3_STATUS="PASS"
else
	G3_STATUS="FAILED"
	G3_NOTE="see gate3-build.log"
fi

# ---------------------------------------------------------------------------
# GATE 4 — tests (publish job's "Test" step)
#
# `--no-fail-fast`, and CI NOW AGREES. Fail-fast stops at the first failing
# test BINARY and hides every later suite behind it — which is exactly how the
# v1.7.0 breakage stayed invisible locally, and how this crate went on to burn
# three tags discovering one environment-dependent failure at a time. This gate
# diverged from CI on that point for as long as CI ran a plain `cargo test`;
# quick task 260917-r4c moved the publish job's Test step to `--no-fail-fast`
# too, so the divergence is closed and this line now mirrors the workflow
# rather than deliberately departing from it.
# ---------------------------------------------------------------------------
section "GATE 4 — cargo test --no-fail-fast"
if run_gate "gate4-test.log" cargo test --no-fail-fast; then
	G4_STATUS="PASS"
else
	G4_STATUS="FAILED"
	G4_NOTE="see gate4-test.log"
fi

# ---------------------------------------------------------------------------
# GATE 5 — clippy (publish job's "Clippy" step)
# ---------------------------------------------------------------------------
section "GATE 5 — cargo clippy -- -D warnings"
if run_gate "gate5-clippy.log" cargo clippy -- -D warnings; then
	G5_STATUS="PASS"
else
	G5_STATUS="FAILED"
	G5_NOTE="see gate5-clippy.log"
fi

# ---------------------------------------------------------------------------
# GATE 6 — cargo package (publish job's "Publish" step, minus the upload)
#
# This mirrors the publish job's Publish step, `cargo publish --locked`, up to
# and EXCLUDING the upload. `cargo package` assembles the .crate from the
# manifest's file list and then builds the unpacked tarball; that build is the
# step that failed v1.15.0 (`failed to verify package tarball`), when an
# unanchored `assets/` exclude dropped three `include_str!` targets.
#
# The subcommand has no upload path, needs no token and no crates.io account.
# This script neither reads nor forwards CARGO_REGISTRY_TOKEN.
#
# It sits after clippy because Publish follows Clippy in release.yml.
#
# Its output (the .crate and the unpacked tree) lands under the target dir's
# package/ subtree: target/ in bare mode, /cargo-target/ in container mode,
# inside the capped cache volume. The script does not delete it: release.yml
# drops target/package only for the rust-cache cleaner, which never runs here;
# cargo overwrites it on the next run; and the .crate is useful evidence.
#
# WHY --allow-dirty, ALWAYS. CLAUDE.md Release Process step 3 runs BEFORE the
# step-5 release commit, so the step-1 version bump and the step-2 `cargo update`
# are uncommitted BY DESIGN. Without the flag this gate would fail on every
# correctly run release. The container dispatch bind-mounts the same working
# tree at /work and does not copy it, so the dirty state is identical in both
# modes and one flag serves both. Every other gate also tests the working tree,
# not HEAD. The dirty-tree advisory below is what keeps the flag from hiding a
# real problem.
#
# WHY NO --offline AND NO OFFLINE FALLBACK. Measured: `cargo package` contacts
# the crates.io index even on a warm cache, unlike the check, build, test and
# clippy gates, and the publish job runs online. A retry-offline fallback would
# be a second code path that green CI never takes. The escape hatch for a bare
# offline run on a warm cache is cargo's own CARGO_NET_OFFLINE=true, which every
# cargo gate honours. The container passes only explicit `-e` variables and
# needs the network anyway for its image build and oracle install.
# ---------------------------------------------------------------------------
section "GATE 6 — cargo package --locked (the publish step's tarball verify; never uploads)"
if run_gate "gate6-package.log" cargo package --locked --allow-dirty; then
	G6_STATUS="PASS"
else
	G6_STATUS="FAILED"
	G6_NOTE="see gate6-package.log"
fi

# DIRTY-TREE ADVISORY. Runs whatever the verdict is, and NEVER alters it — a
# hint must not be able to hide, or invent, a failure. The pathspec is exactly the packaged paths whose
# content changes what compiles.
if DIRTY_PATHS=$(git status --porcelain --untracked-files=all -- src tests Cargo.toml Cargo.lock 2>/dev/null); then
	if [ -n "$DIRTY_PATHS" ]; then
		DIRTY_COUNT=$(printf '%s\n' "$DIRTY_PATHS" | wc -l | tr -d ' ')
		echo
		echo "ADVISORY: --allow-dirty packaged these uncommitted paths. The publish job"
		echo "          packages only what is COMMITTED at the tag. An untracked (??) file"
		echo "          the crate needs passes this gate and FAILS the publish job."
		printf '%s\n' "$DIRTY_PATHS" | sed 's/^/  /'
		if [ "$G6_STATUS" = "PASS" ]; then
			G6_NOTE="packaged $DIRTY_COUNT uncommitted path(s); see gate output"
		fi
	fi
else
	echo
	echo "ADVISORY: dirty state unknown — git status failed (no repository, or a linked"
	echo "          worktree seen from inside the container). Verdict unchanged."
fi

# ---------------------------------------------------------------------------
# GATE 7 — cross-target check + clippy (release.yml `cross-target-check` job)
#
# WHY. v2.2.0 and v2.3.0 both died in that job, on a Windows-only dead-code lint
# in a test helper whose callers were gated to linux/macos. Gates 3-5 lint the
# HOST target only, so nothing local could see it. This gate runs the job's two
# commands verbatim for every target in its matrix.
#
# DERIVED, NOT COPIED, AND GUARDED. The target list, each entry's check_args and
# its clippy flag come from release.yml through xt_load (see the parser comment
# above), which also refuses any matrix or job it cannot model. A refusal is
# FAILED: a gate that cannot say what CI runs must not say PASS.
#
# NEVER INSTALLS. Like gate 2, detection is read-only (`rustup target list
# --installed`); `rustup target add` mutates the developer's toolchain, and an
# auto-install would also hide an image that drifted from release.yml.
#   - bare run, target missing   -> SKIPPED, with the exact `rustup target add`.
#   - --container, target missing -> FAILED: the image is built with exactly the
#     release.yml list, so a gap means the image drifted and the certificate
#     would be a lie. (PRE_TAG_CHECK_IN_CONTAINER is the dispatch's marker.)
#
# CLIPPY IS SKIPPED AFTER A FAILED CHECK. CI's Clippy step carries `if:
# matrix.clippy`, which has an implicit success(), so a failed Check skips it
# there too; the same rule here keeps one root cause from being reported twice.
# check_args is split with `read -ra` (no globbing, no eval) and every value is
# passed to cargo as its own argv element.
# ---------------------------------------------------------------------------
section "GATE 7 — cross-target check + clippy (release.yml cross-target-check matrix)"
if ! xt_load; then
	G7_STATUS="FAILED"
	G7_NOTE="could not derive the matrix from release.yml"
	echo "FAILED: release.yml's cross-target-check job could not be modelled (see the"
	echo "        reason above), so this gate cannot say what CI runs and certifies nothing."
elif ! command -v rustup >/dev/null 2>&1; then
	G7_STATUS="SKIPPED"
	G7_NOTE="rustup not on PATH"
	echo "SKIPPED: rustup is not on PATH, so installed targets cannot be listed."
else
	XT_INSTALLED=$(rustup target list --installed 2>/dev/null)
	G7_FAILED=""
	G7_SKIPPED=""
	for i in "${!XT_TARGETS[@]}"; do
		xt="${XT_TARGETS[$i]}"
		echo
		echo ">>> target $xt (check_args: ${XT_ARGS[$i]:-<none>}; clippy: ${XT_CLIPPY[$i]})"
		if ! printf '%s\n' "$XT_INSTALLED" | grep -Fxq -- "$xt"; then
			if [ -n "${PRE_TAG_CHECK_IN_CONTAINER:-}" ]; then
				G7_FAILED="$G7_FAILED $xt(not installed: image drifted from release.yml)"
				echo "FAILED: $xt is not installed in this image. The image is built with"
				echo "        exactly release.yml's target list, so it has drifted; rebuild it."
			else
				G7_SKIPPED="$G7_SKIPPED $xt"
				echo "  >>> SKIPPED — THIS TARGET DID NOT PASS. <<<"
				echo "  $xt is not installed. Nothing was installed (deliberately). To run it:"
				echo "      rustup target add $xt"
			fi
			continue
		fi
		xt_args=()
		read -ra xt_args <<<"${XT_ARGS[$i]}"
		if ! run_gate "gate7-$xt-check.log" cargo check --locked --target "$xt" ${xt_args[@]+"${xt_args[@]}"}; then
			G7_FAILED="$G7_FAILED $xt(check)"
			echo "  (clippy not run for $xt: CI skips its Clippy step after a failed Check)"
		elif [ "${XT_CLIPPY[$i]}" = "true" ]; then
			if ! run_gate "gate7-$xt-clippy.log" cargo clippy --locked --target "$xt" --all-targets -- -D warnings; then
				G7_FAILED="$G7_FAILED $xt(clippy)"
			fi
		fi
	done
	if [ -n "$G7_FAILED" ]; then
		G7_STATUS="FAILED"
		G7_NOTE="failed:${G7_FAILED}"
	elif [ -n "$G7_SKIPPED" ]; then
		G7_STATUS="SKIPPED"
		G7_NOTE="not installed, rustup target add:${G7_SKIPPED}"
	else
		G7_STATUS="PASS"
		G7_NOTE="${#XT_TARGETS[@]} targets"
	fi
fi

OVERALL_RC=0
for st in "$G1_STATUS" "$G2_STATUS" "$G3_STATUS" "$G4_STATUS" "$G5_STATUS" "$G6_STATUS" "$G7_STATUS"; do
	if [ "$st" = "FAILED" ]; then
		OVERALL_RC=1
	fi
done
exit "$OVERALL_RC"
