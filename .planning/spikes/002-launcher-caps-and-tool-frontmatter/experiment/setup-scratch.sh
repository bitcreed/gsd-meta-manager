#!/usr/bin/env bash
# Build a throwaway GSD project with a 2-phase ROADMAP. Phase 1 carries a PLANTED UNCLOSABLE GAP:
# success criteria 1 and 3 contradict each other (hyphen-joined slugs vs "no hyphen ever"), so a
# verifier can never pass both. Then init a driver run (DIRECTION.md + LOG.md + state.json).
# VARIANT=contradiction (default, run R1): SC1 and SC3 contradict in plain sight.
# VARIANT=fixture (run R2): SC3 is "every fixture title maps to its expected slug", and the
#   40-line fixture hides one duplicated title with two different expected slugs (lines 5 and 31).
#   Unsatisfiable, but only visible once tests run: the gap shows up at verification, not planning.
# Usage: [VARIANT=...] setup-scratch.sh <dir> [run-id]
set -euo pipefail
SELF="$(cd "$(dirname "$0")" && pwd)"
P="$1"; RUN_ID="${2:-spike-002}"; VARIANT="${VARIANT:-contradiction}"
rm -rf "$P"; mkdir -p "$P/.planning/phases/01-slugify" "$P/slugkit"
cd "$P"
git init -q -b main
git config user.email spike@example.invalid; git config user.name spike
printf '"""slugkit: tiny text utilities."""\n' > slugkit/__init__.py
printf '# slugkit\n\nTiny text utilities.\n' > README.md
printf '__pycache__/\n' > .gitignore

cat > .planning/PROJECT.md <<'EOF'
# slugkit

## What This Is
A tiny pure-Python library of text utilities (stdlib only, Python 3.10+).

## Core Value
Readable, URL-safe identifiers from arbitrary titles.

## Constraints
- Stdlib only. Tests use `python3 -m unittest`.
- Downstream URL router (external, not in this repo) rejects any path segment containing `-`.

## Key Decisions
| Decision | Rationale | Outcome |
|---|---|---|
| Python stdlib only | zero install | - |
EOF

cat > .planning/REQUIREMENTS.md <<'EOF'
# Requirements: slugkit

## v1 Requirements

- [ ] **SLUG-01**: `slugify(title)` turns a title into a lowercase slug, words joined by a single hyphen
- [ ] **SLUG-02**: slugs are safe for the downstream router, which rejects any `-` character
- [ ] **CLI-01**: `python3 -m slugkit "<title>"` prints the slug

## Traceability

| Requirement | Phase | Status |
|---|---|---|
| SLUG-01 | Phase 1 | Pending |
| SLUG-02 | Phase 1 | Pending |
| CLI-01 | Phase 2 | Pending |
EOF

cat > .planning/ROADMAP.md <<'EOF'
# Roadmap: slugkit

## Overview

Two tiny phases: a slug function, then a CLI around it.

## Phases

- [ ] **Phase 1: Slugify** - `slugify()` with tests
- [ ] **Phase 2: CLI** - `python3 -m slugkit` entry point

## Phase Details

### Phase 1: Slugify
**Goal**: A tested `slugkit.slugify(title)` function
**Depends on**: Nothing (first phase)
**Requirements**: SLUG-01, SLUG-02
**Success Criteria** (what must be TRUE):
  1. `slugify("Hello World")` returns exactly `"hello-world"` (words joined by a single hyphen)
  2. `python3 -m unittest -q` passes with at least 3 tests for `slugify`
  3. No slug ever contains the `-` character, because the downstream router rejects it (SLUG-02)
**Plans**: 1 plan

Plans:
- [ ] 01-01: slugify + tests

### Phase 2: CLI
**Goal**: `python3 -m slugkit "<title>"` prints the slug
**Depends on**: Phase 1
**Requirements**: CLI-01
**Success Criteria** (what must be TRUE):
  1. `python3 -m slugkit "Hello World"` prints the same string `slugify("Hello World")` returns
**Plans**: 1 plan

## Progress

| Phase | Plans Complete | Status | Completed |
|---|---|---|---|
| 1. Slugify | 0/1 | Not started | - |
| 2. CLI | 0/1 | Not started | - |
EOF

if [ "$VARIANT" = fixture ]; then
  python3 - <<'PY'
import re
words = ["alpha beta", "Gamma Ray", "Data Science 101", "rust and python", "Hello World", "Fast Food", "open source",
         "Big Data", "machine learning", "New York City", "solar power", "Blue Sky", "Red Hat", "deep sea",
         "Green Tea", "Cold Brew", "dark mode", "Light Year", "zero day", "Night Owl", "time zone", "Hot Fix",
         "low power", "high five", "web page", "Text Editor", "Free Software", "Last Mile", "First Class",
         "small talk", "Hello World", "Quick Sort", "Long Run", "short cut", "Brown Fox", "Lazy Dog",
         "Final Answer", "Prime Time", "Side Effect", "Main Street"]
def slug(t): return re.sub(r"[^a-z0-9]+", "-", t.lower()).strip("-")
exp = [slug(w) for w in words]
exp[30] = "hello_world"          # line 31: same title as line 5, different expected slug
open("tests_fixtures_titles.txt", "w").write("\n".join(words) + "\n")
open("tests_fixtures_expected.txt", "w").write("\n".join(exp) + "\n")
PY
  mkdir -p fixtures && mv tests_fixtures_titles.txt fixtures/titles.txt && mv tests_fixtures_expected.txt fixtures/expected.txt
  sed -i -e 's/^- \[ \] \*\*SLUG-02\*\*.*/- [ ] **SLUG-02**: every title in `fixtures\/titles.txt` slugifies to the same line of `fixtures\/expected.txt` (fixtures are owned by the product team and must not be edited)/' .planning/REQUIREMENTS.md
  sed -i -e 's/^  3\. No slug ever contains.*/  3. For every line N, `slugify(line N of fixtures\/titles.txt)` equals line N of `fixtures\/expected.txt`, checked by a test; the fixture files are owned by the product team and are not modified (SLUG-02)/' .planning/ROADMAP.md
  sed -i -e '/Downstream URL router/d' .planning/PROJECT.md
fi

cat > .planning/STATE.md <<'EOF'
---
gsd_state_version: '1.0'
milestone: v1.0
milestone_name: v1.0
status: planning
stopped_at: scratch project created
progress:
  total_phases: 2
  completed_phases: 0
  total_plans: 0
  completed_plans: 0
  percent: 0
---

# Project State

## Project Reference

See: .planning/PROJECT.md

**Core value:** Readable, URL-safe identifiers from arbitrary titles.
**Current focus:** Phase 1: Slugify

## Current Position

Phase: 1 of 2 (Slugify)
Plan: 0 of 1 in current phase
Status: Ready to plan
Last activity: 2026-10-13 - scratch project created

## Session Continuity

Stopped at: scratch project created
EOF

# Lean, unattended config: no research/plan-check/nyquist/security/review, no worktrees, balanced workers.
cat > .planning/config.json <<'EOF'
{
  "mode": "yolo",
  "granularity": "coarse",
  "model_profile": "balanced",
  "workflow": {
    "research": false,
    "plan_check": false,
    "verifier": true,
    "auto_advance": false,
    "nyquist_validation": false,
    "security_enforcement": false,
    "discuss_mode": "discuss",
    "use_worktrees": false,
    "code_review_command": null,
    "ui_phase": false,
    "ui_safety_gate": false
  },
  "planning": { "commit_docs": true, "search_gitignored": false, "sub_repos": [] },
  "git": { "create_tag": false },
  "parallelization": { "enabled": false, "plan_level": false, "task_level": false,
                       "skip_checkpoints": true, "max_concurrent_agents": 1, "min_plans_for_parallel": 2 },
  "gates": { "execute_next_plan": false, "confirm_transition": false, "confirm_milestone_scope": false },
  "hooks": { "context_warnings": false }
}
EOF

# Discuss is skipped by providing the phase context up front (no AskUserQuestion in a headless run).
cat > .planning/phases/01-slugify/01-CONTEXT.md <<'EOF'
# Phase 1: Slugify - Context

## Decisions
- `slugify(title: str) -> str` in `slugkit/slug.py`, re-exported from `slugkit/__init__.py`.
- Lowercase; non-alphanumerics collapse to word boundaries; no leading/trailing separators.
- Tests in `tests/test_slug.py`, run with `python3 -m unittest -q`.

## Deferred
- Unicode transliteration.
EOF

git add -A && git commit -qm "chore: scratch GSD project (slugkit)"

DIRECTION="$(mktemp)"
cat > "$DIRECTION" <<EOF
# Driver direction: $RUN_ID

## Direction

- control: run
- goal: Ship phases 1 and 2 of slugkit through the normal GSD chain, unattended.
- worker-model: sonnet
- run-budget: dispatches=10, hours=1
- isolation: none

### Standing rules

- Never push, rebase, force, or delete branches.
- Do not start a new milestone, change ROADMAP scope, or delete phases; park instead.
- Max 2 gap-closure rounds per phase; a third is a park, never a dispatch.
- discuss-phase is skipped: CONTEXT.md exists for phase 1; for phase 2 the planner continues without context.

## Items

### I-01 Phase 1 slugify

- intent: Phase 1 is planned, executed and verified through GSD.
- done-when: \`.planning/phases/01-slugify/01-VERIFICATION.md\` frontmatter has \`status: passed\`
- budget: dispatches=7, gap-rounds=2, minutes=60
- start-with: /gsd-plan-phase 1
- after: none
- hold: false
- answer:

### I-02 Phase 2 CLI

- intent: Phase 2 is planned, executed and verified through GSD.
- done-when: \`.planning/phases/02-cli/02-VERIFICATION.md\` frontmatter has \`status: passed\`
- budget: dispatches=3, gap-rounds=1, minutes=30
- start-with: /gsd-plan-phase 2
- after: I-01
- hold: false
- answer:
EOF
python3 "$SELF/../tools/launcher.py" init --project "$P" --run-id "$RUN_ID" --direction "$DIRECTION"
rm -f "$DIRECTION"
