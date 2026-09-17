#!/usr/bin/env bash
#
# pre-tag-check.sh — a LOCAL dry-run of .github/workflows/release.yml, runnable
# BEFORE the `v*` tag exists.
#
# It reproduces the gates that workflow runs on a tag push (tag/version match,
# MSRV compile, release build, test, clippy) so a release can be rehearsed
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
# Usage: ./scripts/pre-tag-check.sh [vX.Y.Z]

# `set -u` only, on purpose:
#   - no `set -e`: gates 2-5 must ALL run and report, so the operator gets the
#     whole picture from one invocation rather than one failure at a time.
#   - no `pipefail`: the constant-extraction pipelines end in `head -1`, whose
#     SIGPIPE would be misread as a pipeline failure.
set -u

PROG="${0##*/}"

usage() {
	cat <<'USAGE'
Usage: ./scripts/pre-tag-check.sh [TAG]

Runs, locally, the gates that .github/workflows/release.yml runs on a tag push.

  TAG   optional, e.g. v1.7.1. When given, gate 1 asserts it matches the
        Cargo.toml version and hard-stops on disagreement. When omitted, gate 1
        just reports the crate version and is marked SKIPPED.

Exit status: 0 when no gate FAILED, 1 when any gate FAILED, 2 on bad usage.
USAGE
}

TAG_ARG=""
HAVE_TAG=0
for arg in "$@"; do
	case "$arg" in
	-h | --help)
		usage
		exit 0
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

RULE="================================================================================"
THIN="--------------------------------------------------------------------------------"

POLICY_FILE="src/envelope/policy.rs"
CONST_NAME="CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION"
WITNESS_TEST="the_config_section_constants_record_the_git_version_they_were_derived_against"

# ---------------------------------------------------------------------------
# State consumed by the summary. Everything the EXIT trap reads must be
# initialised BEFORE the trap is installed, because the trap also fires on the
# constant-extraction hard error and on gate 1's hard stop.
#
# "NOT REACHED" is the honest initial value: an early exit must not report a
# gate as skipped (a judgement) when it was simply never run.
# ---------------------------------------------------------------------------
G1_STATUS="NOT REACHED"
G2_STATUS="NOT REACHED"
G3_STATUS="NOT REACHED"
G4_STATUS="NOT REACHED"
G5_STATUS="NOT REACHED"
G1_NOTE=""
G2_NOTE=""
G3_NOTE=""
G4_NOTE=""
G5_NOTE=""

LOG_DIR=""
INSTALLED_GIT=""
DERIVED_GIT_VERSION=""
GIT_ADVISORY=0
WITNESS_IN_FAILURES=0

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
	echo "$THIN"
	if [ -n "$LOG_DIR" ]; then
		echo "  gate logs: $LOG_DIR"
	else
		echo "  gate logs: none created (exited before any gate ran)"
	fi

	if [ "$GIT_ADVISORY" -eq 1 ]; then
		echo "$THIN"
		echo "  ADVISORY: the installed git and the derived-against constant DISAGREE."
		echo "  ADVISORY:   installed git            : $INSTALLED_GIT"
		echo "  ADVISORY:   derived-against constant : $DERIVED_GIT_VERSION"
		echo "  ADVISORY:     ($CONST_NAME"
		echo "  ADVISORY:      in $POLICY_FILE)"
		echo "  ADVISORY: the GitHub ubuntu-latest runner's ambient git is the AUTHORITY"
		echo "  ADVISORY: for that witness test. A GREEN LOCAL GATE DOES NOT GUARANTEE"
		echo "  ADVISORY: the publish job passes."
		if [ "$G4_STATUS" = "FAILED" ] && [ "$WITNESS_IN_FAILURES" -eq 1 ]; then
			echo "$THIN"
			echo "  NOTE: gate 4's failing set includes the version witness"
			echo "        $WITNESS_TEST,"
			echo "        which is EXPECTED to fail locally while the advisory above stands."
			echo "        Read the gate-4 log for any OTHER failing test before concluding"
			echo "        that something regressed."
		fi
	fi
	echo "$THIN"
	echo "  The exit code reflects GATE FAILURES ONLY. The git-version advisory, on"
	echo "  its own, never fails this run. SKIPPED is not a failure."
	echo "$RULE"
}

trap 'rc=$?; print_summary; exit "$rc"' EXIT

# ---------------------------------------------------------------------------
# CONSTANT EXTRACTION
#
# The derived-against version is READ OUT OF THE SOURCE at runtime. The version
# string must never be spelled in this script — a copy here would go stale
# silently and the reconciliation would then compare the constant against
# itself-from-last-year.
#
# Match the DECLARATION line only: the constant is also named in several doc
# comment lines above it, and any of those would extract the wrong literal (or
# no literal at all).
# ---------------------------------------------------------------------------
if [ ! -f "$POLICY_FILE" ]; then
	echo "$RULE" >&2
	echo "HARD ERROR: $POLICY_FILE does not exist." >&2
	echo "This script exists to reconcile the installed git against the constant" >&2
	echo "  $CONST_NAME" >&2
	echo "declared in that file. Without it the script has lost its subject and" >&2
	echo "cannot honestly report anything. Run from the repository root." >&2
	echo "$RULE" >&2
	exit 1
fi

CONST_DECL=$(grep -E "^[[:space:]]*pub const[[:space:]]+${CONST_NAME}[[:space:]]*:[[:space:]]*&str[[:space:]]*=" "$POLICY_FILE" | head -1)
if [ -n "$CONST_DECL" ]; then
	DERIVED_GIT_VERSION=$(printf '%s\n' "$CONST_DECL" | sed -E 's/.*"(.*)".*/\1/')
	# sed leaves the line untouched when it does not match; treat that as empty.
	if [ "$DERIVED_GIT_VERSION" = "$CONST_DECL" ]; then
		DERIVED_GIT_VERSION=""
	fi
fi

if [ -z "$DERIVED_GIT_VERSION" ]; then
	echo "$RULE" >&2
	echo "HARD ERROR: could not extract $CONST_NAME" >&2
	echo "from $POLICY_FILE." >&2
	if [ -z "$CONST_DECL" ]; then
		echo "Reason: no matching 'pub const ... : &str = ...' declaration line found." >&2
	else
		echo "Reason: the declaration line carries no quoted literal:" >&2
		echo "  $CONST_DECL" >&2
	fi
	echo "This script has lost its subject — the constant it exists to reconcile" >&2
	echo "against — so it refuses to continue and report a reconciliation it cannot" >&2
	echo "actually perform. Fix the declaration, or this script's matcher." >&2
	echo "$RULE" >&2
	exit 1
fi

INSTALLED_GIT=$(git --version 2>/dev/null)
[ -n "$INSTALLED_GIT" ] || INSTALLED_GIT="(git not on PATH)"

# ---------------------------------------------------------------------------
# RECONCILIATION BANNER — printed on EVERY run, BEFORE gate 1, so it is still
# on screen when gate 1 hard-stops.
#
# A MISMATCH HERE IS AN ADVISORY, NEVER A FAILURE. On a correctly-prepared
# release the constant is deliberately re-derived against the git version the
# CI runner carries, which is routinely NEWER than the one installed locally.
# Hard-failing on that would paint this script red on a GOOD tree and train the
# operator to ignore it — the exact outcome the banner exists to prevent. So it
# shouts, and the exit code stays reserved for real gate failures.
# ---------------------------------------------------------------------------
echo "$RULE"
echo "!!! GIT VERSION RECONCILIATION !!!"
echo "$THIN"
echo "  installed git (this machine) : $INSTALLED_GIT"
echo "  derived-against constant     : $DERIVED_GIT_VERSION"
echo "    from $POLICY_FILE"
echo "    ($CONST_NAME)"
echo "$THIN"
if [ "$INSTALLED_GIT" = "$DERIVED_GIT_VERSION" ]; then
	echo "  MATCH: this tree can validate the version witness test the way CI will."
else
	GIT_ADVISORY=1
	echo "  MISMATCH — read this:"
	echo "    1. THIS LOCAL RUN CANNOT VALIDATE THAT TEST THE WAY CI WILL."
	echo "    2. The GitHub ubuntu-latest runner's ambient git is the AUTHORITY for it."
	echo "    3. A GREEN LOCAL GATE THEREFORE DOES NOT GUARANTEE THE PUBLISH JOB PASSES."
fi
echo "$RULE"
echo

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
# DELIBERATELY DIFFERS FROM CI: CI runs a plain `cargo test`, this runs
# `cargo test --no-fail-fast`. Fail-fast stops at the first failing test BINARY
# and hides every later suite behind it — which is exactly how the v1.7.0
# breakage stayed invisible locally. The point of a dry-run is the whole
# picture, so the whole suite runs.
# ---------------------------------------------------------------------------
section "GATE 4 — cargo test --no-fail-fast"
if run_gate "gate4-test.log" cargo test --no-fail-fast; then
	G4_STATUS="PASS"
else
	G4_STATUS="FAILED"
	G4_NOTE="see gate4-test.log"
fi
# DEC-5: detect the version witness in the output so the summary can EXPLAIN it.
# The gate's verdict is NOT touched — no filter, no skip, no carve-out. A filter
# here could hide a real regression behind an expected failure. If the test is
# ever renamed this hint simply goes quiet and the gate still fails correctly.
if [ -f "$LOG_DIR/gate4-test.log" ] && grep -qF "$WITNESS_TEST" "$LOG_DIR/gate4-test.log"; then
	WITNESS_IN_FAILURES=1
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

OVERALL_RC=0
for st in "$G1_STATUS" "$G2_STATUS" "$G3_STATUS" "$G4_STATUS" "$G5_STATUS"; do
	if [ "$st" = "FAILED" ]; then
		OVERALL_RC=1
	fi
done
exit "$OVERALL_RC"
