#!/usr/bin/env bash
#
# install-conformance-oracle.sh — put GSD's own `gsd-tools` where
# `tests/driver_router_conformance.rs` looks for it, and refuse to report
# success unless it actually answers.
#
# WHY THIS EXISTS. That test compares this crate's router rule table against
# GSD's own router, and it FAILS BY DESIGN when the oracle is absent — an
# absent oracle means nothing checked that the transcription still holds, and
# a test that passes because it checked nothing is worse than no test. A
# GitHub `ubuntu-latest` runner has Node but no GSD, so the publish job was
# running that test with its subject missing. The crate failed to publish
# three times, once on exactly this. THE FIX IS TO PROVIDE THE DEPENDENCY.
# Setting the test's missing-oracle opt-out would make the check vacuous,
# which is the fail-open shape this codebase repeatedly rejects.
#
# SINGLE SOURCE OF TRUTH. This script is the whole answer to "what does the
# publish job need on top of a bare ubuntu-latest so that `cargo test` tests
# something?", and it is invoked by BOTH:
#   - .github/workflows/release.yml, the publish job, before its Test step; and
#   - scripts/pre-tag-check.sh --container, inside the runner lookalike.
# That is deliberate: the dry run provisions BYTE-IDENTICALLY to CI, so a
# green container rehearsal is evidence about the publish job rather than
# evidence about the developer's laptop.
#
# THE DON'T-CLOBBER RULE, stated up front because it is the surprising part.
# If `$HOME/.claude/gsd-core` already exists — directory, symlink, or anything
# else — this script NEVER touches it. It installs nothing, verifies what is
# there, exits 0 if that answers and NON-ZERO if it does not. It does not
# repair, move, back up or overwrite. Developer machines run this by hand and
# a real GSD install lives at that path; deleting someone's GSD as a side
# effect of a release rehearsal is not this script's decision to make. When it
# is present-but-broken the operator is told, and fixes it themselves.
#
# `set -euo pipefail`, UNLIKE ITS SIBLING scripts/pre-tag-check.sh. That
# script runs `set -u` only because it owes the operator all five gate results
# from one invocation, so it must survive a failure to keep reporting. This
# script owes nothing of the kind: every step is a precondition for the next,
# so the first failure is the whole story and an early hard stop is the
# correct shape. The divergence is deliberate, not drift.
#
# Usage: ./scripts/install-conformance-oracle.sh
set -euo pipefail

PROG="${0##*/}"

RULE="================================================================================"
THIN="--------------------------------------------------------------------------------"

# Resolve the repo root from the script's OWN location, never from cwd: CI runs
# this from the workspace root and the container runs it from /work.
SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "$SCRIPT_DIR/.." && pwd)

VERSION_FILE="$REPO_ROOT/src/state_reader/config_json.rs"
VERSION_CONST="GSD_CORE_SYNCED_VERSION"
PACKAGE="@opengsd/gsd-core"

NPM_PREFIX="$HOME/.npm-global"
CLAUDE_DIR="$HOME/.claude"
ORACLE_DIR="$CLAUDE_DIR/gsd-core"
ORACLE_SHIM="$ORACLE_DIR/bin/gsd-tools.cjs"

# The verb tests/driver_router_conformance.rs:54 names. The probe below asks the
# oracle the SAME question the test asks, so a green probe means a green test.
ORACLE_VERB="init.manager"

hard_error() {
	echo "$RULE" >&2
	echo "HARD ERROR: $1" >&2
	shift
	for line in "$@"; do
		echo "$line" >&2
	done
	echo "$RULE" >&2
	exit 1
}

# ---------------------------------------------------------------------------
# CONSTANT EXTRACTION
#
# The GSD version is READ OUT OF THE SOURCE at runtime, mirroring the block at
# scripts/pre-tag-check.sh lines 155-202. Its reasoning applies here verbatim:
# a version literal spelled in this script would go stale silently, and the
# publish job would then install a GSD that has nothing to do with the one this
# build was synced against.
#
# Match the DECLARATION line only. The constant is also named in the doc
# comments above it and in GSD_CORE_SYNCED_COMMIT's doc comment, any of which
# would yield the wrong literal (or none).
# ---------------------------------------------------------------------------
if [ ! -f "$VERSION_FILE" ]; then
	hard_error \
		"$VERSION_FILE does not exist." \
		"This script installs the EXACT GSD version that file's $VERSION_CONST" \
		"records. Without it there is no version to pin, and pinning is not" \
		"optional here: this runs in the job that holds the crates.io token."
fi

VERSION_DECL=$(grep -E "^[[:space:]]*pub const[[:space:]]+${VERSION_CONST}[[:space:]]*:[[:space:]]*&str[[:space:]]*=" "$VERSION_FILE" | head -1 || true)
GSD_VERSION=""
if [ -n "$VERSION_DECL" ]; then
	GSD_VERSION=$(printf '%s\n' "$VERSION_DECL" | sed -E 's/.*"(.*)".*/\1/')
	# sed leaves a non-matching line untouched; treat that as "no literal".
	if [ "$GSD_VERSION" = "$VERSION_DECL" ]; then
		GSD_VERSION=""
	fi
fi

if [ -z "$GSD_VERSION" ]; then
	if [ -z "$VERSION_DECL" ]; then
		hard_error \
			"could not extract $VERSION_CONST from $VERSION_FILE." \
			"Reason: no matching 'pub const ... : &str = ...' declaration line found." \
			"This script refuses to guess a version, and refuses to install an" \
			"unpinned one. Fix the declaration, or this script's matcher."
	fi
	hard_error \
		"could not extract $VERSION_CONST from $VERSION_FILE." \
		"Reason: the declaration line carries no quoted literal:" \
		"  $VERSION_DECL" \
		"This script refuses to guess a version, and refuses to install an" \
		"unpinned one. Fix the declaration, or this script's matcher."
fi

# ---------------------------------------------------------------------------
# TOOL REQUIREMENTS
# ---------------------------------------------------------------------------
for tool in node npm; do
	if ! command -v "$tool" >/dev/null 2>&1; then
		hard_error \
			"\`$tool\` is not on PATH." \
			"The oracle is a Node program installed from npm. The publish job must" \
			"install Node (actions/setup-node) BEFORE calling this script, and the" \
			"container image must carry it." \
			"See scripts/pre-tag-check.Dockerfile for the version the runner has."
	fi
done

echo "$RULE"
echo "=== $PROG — provisioning the GSD conformance oracle ==="
echo "$THIN"
echo "  repo root        : $REPO_ROOT"
echo "  pinned version   : $PACKAGE@$GSD_VERSION"
echo "    from $VERSION_CONST in src/state_reader/config_json.rs"
echo "  oracle path      : $ORACLE_DIR"
echo "$RULE"
echo

# ---------------------------------------------------------------------------
# VERIFICATION — run AFTER installing, and INSTEAD of installing when something
# is already there.
#
# A provisioning step that "succeeds" while leaving the oracle unresolvable
# hands the fail-open shape straight back to the test: the run would be green
# and nothing would have compared the rule table to anything. That is the whole
# failure this script exists to close, so the probe's verdict is this script's
# exit status.
#
# The probe exercises the SAME call path the test uses — `query init.manager`
# against a tree shaped like the test's `skeleton()` fixture
# (tests/driver_router_conformance.rs:232-246) — so a green probe is evidence
# about the test rather than about the filesystem.
# ---------------------------------------------------------------------------
verify_oracle() {
	if [ ! -f "$ORACLE_SHIM" ]; then
		echo "  probe: FAILED — $ORACLE_SHIM is not a file." >&2
		return 1
	fi

	local fixture
	fixture=$(mktemp -d "${TMPDIR:-/tmp}/gsd-oracle-probe.XXXXXXXX")
	# shellcheck disable=SC2064 # expand $fixture now, deliberately.
	trap "rm -rf '$fixture'" RETURN

	mkdir -p "$fixture/.planning/phases"
	cat >"$fixture/.planning/ROADMAP.md" <<'ROADMAP'
# Roadmap

## Phases

- [ ] **Phase 01: Alpha** - the phase both routers are asked about

### Phase 01: Alpha

**Goal**: exercise the rule table
**Depends on**: Nothing
ROADMAP
	cat >"$fixture/.planning/STATE.md" <<'STATE'
---
status: in_progress
---

# State
STATE

	local out err rc
	out=$(mktemp "${TMPDIR:-/tmp}/gsd-oracle-out.XXXXXXXX")
	err=$(mktemp "${TMPDIR:-/tmp}/gsd-oracle-err.XXXXXXXX")
	rc=0
	# NOT `--version`: that is not a valid flag on this tool and exits non-zero,
	# so it would report a working oracle as broken.
	(cd "$fixture" && node "$ORACLE_SHIM" query "$ORACLE_VERB") >"$out" 2>"$err" || rc=$?

	if [ "$rc" -ne 0 ]; then
		echo "  probe: FAILED — the oracle exited $rc." >&2
		echo "    command : node $ORACLE_SHIM query $ORACLE_VERB" >&2
		echo "    cwd     : $fixture (a skeleton() lookalike)" >&2
		echo "    stdout  : $(head -c 200 "$out")" >&2
		echo "    stderr  : $(head -c 200 "$err")" >&2
		rm -f "$out" "$err"
		return 1
	fi

	if ! node -e 'let s="";process.stdin.on("data",d=>s+=d).on("end",()=>{JSON.parse(s)})' <"$out"; then
		echo "  probe: FAILED — the oracle exited 0 but its stdout is not JSON." >&2
		echo "    stdout  : $(head -c 200 "$out")" >&2
		echo "    stderr  : $(head -c 200 "$err")" >&2
		rm -f "$out" "$err"
		return 1
	fi

	rm -f "$out" "$err"
	echo "  probe: OK — \`query $ORACLE_VERB\` exited 0 and returned JSON."
	return 0
}

# ---------------------------------------------------------------------------
# VERSION ADVISORY, never a failure.
#
# Mirrors the git-version advisory at scripts/pre-tag-check.sh lines 207-235,
# for the same reason: a developer's GSD install legitimately runs AHEAD of the
# synced constant, and hard-failing there would paint this script red on a good
# tree and train the operator to ignore it. Absence is expected too — the npm
# package's gsd-core/ directory carries no VERSION file — so "unknown" is a
# normal answer and not a fault.
# ---------------------------------------------------------------------------
version_advisory() {
	local resolved found="" candidate
	resolved=$(cd -- "$ORACLE_DIR" 2>/dev/null && pwd -P) || return 0
	for candidate in "$resolved/package.json" "$resolved/../package.json"; do
		if [ -f "$candidate" ]; then
			found=$(node -e 'try{process.stdout.write(String(require(process.argv[1]).version||""))}catch(e){}' "$candidate" 2>/dev/null || true)
			[ -n "$found" ] && break
		fi
	done

	if [ -z "$found" ]; then
		echo "  installed version: unknown (no adjacent package.json carrying a version)"
		return 0
	fi

	echo "  installed version: $found"
	if [ "$found" != "$GSD_VERSION" ]; then
		echo "$THIN"
		echo "  ADVISORY: the pre-existing install and the synced constant DISAGREE."
		echo "  ADVISORY:   installed        : $found"
		echo "  ADVISORY:   synced constant  : $GSD_VERSION ($VERSION_CONST)"
		echo "  ADVISORY: this is NOT a failure. A developer install legitimately runs"
		echo "  ADVISORY: ahead of the constant. CI installs the pinned version, so what"
		echo "  ADVISORY: this run proved is that the oracle answers — not that it is"
		echo "  ADVISORY: the same build CI will use."
		echo "$THIN"
	fi
	return 0
}

ACTION=""

if [ -e "$ORACLE_DIR" ] || [ -L "$ORACLE_DIR" ]; then
	# THE DON'T-CLOBBER PATH. Verify only; never modify. See the header.
	ACTION="already present (left untouched)"
	echo ">>> $ORACLE_DIR already exists — installing NOTHING."
	echo "    This script never repairs, moves, backs up or overwrites an existing"
	echo "    GSD install. It only checks that it answers."
	echo
	if ! verify_oracle; then
		echo >&2
		echo "$RULE" >&2
		echo "HARD ERROR: $ORACLE_DIR exists but the oracle does not answer." >&2
		echo "Nothing was changed — deliberately. Removing or repairing an existing" >&2
		echo "GSD install is the operator's call, not this script's." >&2
		echo "Fix it, or remove $ORACLE_DIR and re-run this script to get a pinned" >&2
		echo "$PACKAGE@$GSD_VERSION installed in its place." >&2
		echo "$RULE" >&2
		exit 1
	fi
	version_advisory
else
	ACTION="installed $PACKAGE@$GSD_VERSION"
	echo ">>> No oracle at $ORACLE_DIR — installing."
	echo
	# PIN THE EXACT VERSION. Never a range, never @latest: this runs in the job
	# that holds the crates.io publishing token, so an unpinned install is an
	# unreviewed code path in that job (T-r4c-SC).
	npm install -g --prefix "$NPM_PREFIX" "$PACKAGE@$GSD_VERSION"
	mkdir -p "$CLAUDE_DIR"
	# The symlink target is the package's INNER gsd-core/ directory, not the
	# package root. That is what puts bin/gsd-tools.cjs at
	# $HOME/.claude/gsd-core/bin/gsd-tools.cjs — the FIRST path Oracle::resolve()
	# probes (tests/driver_router_conformance.rs:91-115) — so no PATH surgery is
	# needed. Which matters in CI: a PATH exported in one `run:` step does not
	# survive into the next without $GITHUB_PATH.
	ln -s "$NPM_PREFIX/lib/node_modules/$PACKAGE/gsd-core" "$ORACLE_DIR"
	echo
	if ! verify_oracle; then
		echo >&2
		echo "$RULE" >&2
		echo "HARD ERROR: the install completed but the oracle does not answer." >&2
		echo "Reporting success here would hand the fail-open shape straight back to" >&2
		echo "tests/driver_router_conformance.rs, which is the failure this script" >&2
		echo "exists to close. Exiting non-zero instead." >&2
		echo "$RULE" >&2
		exit 1
	fi
fi

echo
echo "$RULE"
echo "=== ORACLE READY ==="
echo "$THIN"
echo "  action     : $ACTION"
echo "  oracle     : $ORACLE_SHIM"
echo "  probe      : PASS (\`query $ORACLE_VERB\` returned JSON)"
echo "$THIN"
echo "  tests/driver_router_conformance.rs can now do real work. It still FAILS if"
echo "  the rule table and GSD's router disagree — providing the oracle is not the"
echo "  same as switching the check off, and nothing here sets its opt-out."
echo "$RULE"
