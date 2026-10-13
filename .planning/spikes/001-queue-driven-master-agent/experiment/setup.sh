#!/usr/bin/env bash
# Create a throwaway GSD-like scratch project with a two-item driver run.
# Usage: setup.sh <dir>
set -euo pipefail
SELF="$(cd "$(dirname "$0")" && pwd)"
P="$1"; RUN_ID="spike-001"
rm -rf "$P"; mkdir -p "$P/.planning/meta-manager/drivers/$RUN_ID"
cd "$P"
git init -q -b main
git config user.email spike@example.invalid; git config user.name spike
cat > calc.py <<'EOF'
def add(a, b):
    return a + b
EOF
printf '# calc\n\nA tiny calculator.\n' > README.md
cat > .planning/STATE.md <<'EOF'
---
gsd_state_version: 1.0
milestone: v0.1
status: executing
stopped_at: scratch project created
---
# State
EOF
printf '# Roadmap\n\n- [ ] **Phase 1: Basics**\n' > .planning/ROADMAP.md
printf '*\n' > .planning/meta-manager/drivers/.gitignore
git add -A && git commit -qm "init scratch project"
D=".planning/meta-manager/drivers/$RUN_ID"
cat > "$D/DIRECTION.md" <<'EOF'
# Driver direction: spike-001

## Direction

- control: run
- goal: Two tiny chores on a scratch project, to exercise the driver loop.
- worker-model: haiku
- run-budget: dispatches=5, hours=1
- isolation: none

### Standing rules

- Never push, rebase, force, or delete branches.
- Use plain instructions, not GSD commands: this scratch project has no GSD workflow installed.
- Keep each worker task tiny; no refactors.

## Items

### I-01 README usage line

- intent: README.md tells a reader how to call add().
- done-when: `grep -q "add(" README.md` succeeds AND the change is committed (clean `git status --porcelain` outside .planning/meta-manager)
- budget: dispatches=2, minutes=10
- after: none
- hold: false
- answer:

### I-02 Trivial test

- intent: add() has one unit test.
- done-when: `python3 -m unittest -q test_calc` exits 0 AND test_calc.py is committed
- budget: dispatches=2, minutes=10
- after: I-01
- hold: false
- answer:
EOF
# LOG header = the template's grammar comment, inlined (run A showed a bare pointer is not enough)
{ printf '# Driver log: spike-001\n\n'; sed -n '/^<!--/,/^-->/p' "$SELF/../templates/LOG.md"; echo; } > "$D/LOG.md"
echo "$P/$D"
