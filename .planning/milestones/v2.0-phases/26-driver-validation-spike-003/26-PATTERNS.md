# Phase 26: Driver Validation Spike 003 - Pattern Map

**Mapped:** 2026-10-16
**Files analyzed:** 17 (all new, under `.planning/spikes/003-divergent-gap-loop-validation/` plus one MANIFEST row)
**Analogs found:** 16 / 17 (all analogs git-tracked, verified via `git ls-files`)

Let `S2` = `.planning/spikes/002-launcher-caps-and-tool-frontmatter`, `S1` = `.planning/spikes/001-queue-driven-master-agent`, `S3` = `.planning/spikes/003-divergent-gap-loop-validation`.
Do NOT copy `S2/tools/launcher.py` or `driverctl.py`; invoke them in place (`SPIKE=002` template path).

## File Classification

| New file (under S3 unless noted) | Role | Data Flow | Closest Analog | Match |
|---|---|---|---|---|
| `README.md` | doc | n/a | `S2/README.md` (frontmatter L1-10) | exact |
| `.planning/spikes/MANIFEST.md` (append row 003) | doc | n/a | rows 001/002 in same file | exact |
| `MASTER-PROMPT.md` (v0.3) | prompt | n/a | `S2/MASTER-PROMPT.md` v0.2 | exact (edit of copy) |
| `experiment/MASTER-PROMPT.run2.md` | prompt (generated) | transform | same v0.2 | derived |
| `templates/DIRECTION.run1.md`, `DIRECTION.run2.md` | template | n/a | `S2/templates/DIRECTION.md` | exact |
| `tools/drift.py` | utility, host watcher | event-driven (poll git, atomic write) | `S2/experiment/cap-tests/gap-writer.sh` (host stand-in loop) + `S2/experiment/race_test.py` (atomic replace) | role-match |
| `tools/dup_check.py` | utility | batch / transform | `S2/experiment/cost_split.py` | role-match |
| `tools/stage_cost.py` | utility | batch / transform | `S2/experiment/cost_split.py` | exact |
| `tools/make_variant.py` | utility | transform | none (see below) | none |
| `experiment/acceptance.py` | utility/test | batch parse | `S2/experiment/race_test.py` (script w/ sys.path import of driverctl) | partial |
| `experiment/setup-scratch.sh` | script | file-I/O | `S2/experiment/setup-scratch.sh` | exact |
| `experiment/run-1.sh`, `run-2.sh` (opt. run-3-answer.sh) | script | request-response | `S1/experiment/run.sh` + README "How to Run" of S2 | role-match |
| `experiment/results/*` | derived outputs | file-I/O | `S2/experiment/results/R2-hidden-fixture/` | exact |

## Pattern Assignments

### `tools/stage_cost.py` and `tools/dup_check.py` (utility, batch)
**Analog:** `S2/experiment/cost_split.py` (tracked). Stdlib-only, module docstring with `Usage:`, `main(*sys.argv[1:N])`.

Weights and per-message dedupe (L14, L30-L40): keep the LAST line per `message.id`:
```python
W = {"in": 1.0, "out": 5.0, "cr": 0.1, "cw": 1.25}
def weighted(u): return sum(W[k] * u.get(k, 0) for k in W)
last = {}   # message id -> (model, usage); the LAST line carries final usage
...
if d.get("type") != "assistant": continue
m = d.get("message") or {}
last[m.get("id")] = (m.get("model", "?"), m.get("usage") or {})
```
Rate solving from `result.modelUsage` (L60-L64):
```python
wt = weighted({"in": u["inputTokens"], "out": u["outputTokens"], "cr": u["cacheReadInputTokens"], "cw": u["cacheCreationInputTokens"]})
rate[model] = u["costUSD"] / wt if wt else 0
```
Transcript locations (L56-L58):
```python
main_tx = glob.glob(os.path.join(cfg, "projects", "*", f"{sid}.jsonl"))[0]
subs = glob.glob(os.path.join(cfg, "projects", "*", sid, "subagents", "**", "*.jsonl"), recursive=True)
```
Required deviations (RESEARCH Pitfall 9): do not return on missing `result` event (L53-55 does); take sid from `RUN | connect | session=` in LOG.md; no bare `[0]` index (guard empty glob); bucket by LOG `dispatched`..`report` windows. `dup_check.py` reads `agent-*.meta.json` (`agentType`, `parentAgentId`, `toolUseId`, `spawnDepth`, `description`, `finished`) instead of token usage, exits 0 only if DUP==0.

### `tools/drift.py` (utility, event-driven)
**Analog:** `S2/experiment/cap-tests/gap-writer.sh` (host-side loop writing into the scratch project) for the CLI shape; `race_test.py` L37-L40 for temp-file + `os.replace` atomic write.
```bash
# gap-writer.sh: Usage: gap-writer.sh <project> <count> <interval-s>
printf -- '---\nphase: 01-slugify\nverified: %s\nstatus: gaps_found\n...' "$(date -u +%FT%TZ)" > "$F"
```
```python
tmp = path + ".human"; open(tmp,"w").write(data); os.replace(tmp, path)
```
Pattern: subcommands `watch|selftest|emit N`; `emit(gen)` pure function; trigger `git -C P ls-tree -r --name-only HEAD` counting `-SUMMARY.md`; log UTC-timestamped advances; also snapshot each distinct `*-VERIFICATION.md` (A8). Fixture generator style (python heredoc with word list and `slug()`, then writing `titles.txt`/`expected.txt`) is in `S2/experiment/setup-scratch.sh` L100-L110; reuse for batch 0.

### `experiment/setup-scratch.sh` (script, file-I/O)
**Analog:** `S2/experiment/setup-scratch.sh` (tracked).
```bash
#!/usr/bin/env bash
set -euo pipefail
SELF="$(cd "$(dirname "$0")" && pwd)"
P="$1"; RUN_ID="${2:-spike-002}"
rm -rf "$P"; mkdir -p "$P/.planning/phases/01-slugify" "$P/slugkit"
cd "$P"; git init -q -b main
git config user.email spike@example.invalid; git config user.name spike
printf '__pycache__/\n' > .gitignore          # add `feed/` here
```
Then heredocs for PROJECT.md, REQUIREMENTS.md, ROADMAP.md (L9+), STATE.md frontmatter (`gsd_state_version: '1.0'` ...), and the final `launcher.py init` call. Deviations: drop the `Downstream URL router` constraint and any `Deferred: unicode transliteration` (anti-patterns); SC3 text is the neutral `feed/` wording from RESEARCH Pattern 1; `feed/` gitignored; seed `feed/` with gen 0. Header comment documents variants, usage line.

### `templates/DIRECTION.run1.md` / `.run2.md`
**Analog:** `S2/templates/DIRECTION.md`: tool-written frontmatter block (`generated_by: driverctl ...`, `caps:`, `dispatch_audit:`), then HUMAN-OWNED body with `## Direction` / `## Items`, `- key: value` lines, `- budget: dispatches=8, gap-rounds=2`. Run2: delete the standing rule `Max 2 gap-closure rounds per phase...` and the `gap-rounds=2` key.

### `MASTER-PROMPT.md` v0.3 / `tools/make_variant.py`
**Analog:** `S2/MASTER-PROMPT.md` (spans: L61, L62-66, L67-69, L113; line 38 enum stays). v0.3 adds D1/D2 clauses (RESEARCH Pitfall 7) plus a diff note. `make_variant.py` has no analog: write as asserted-removal (`assert span in text` per span, then write and print `difflib.unified_diff`), stdlib only, same docstring-with-Usage style as `cost_split.py`.

### `experiment/acceptance.py` (utility, batch parse)
**Analog:** `S2/experiment/race_test.py` for importing the tool module (`sys.path.insert(0, .../"..","tools"); import driverctl as dc  # noqa: E402`) and using `dc.split_frontmatter`. Parse LOG grammar `- {ts} | {item} | {event} | k=v ...`; checks A1-A9 from RESEARCH table; exit non-zero on fail. Import path must point at `S2/tools`, not a copy.

### `experiment/run-1.sh`, `run-2.sh`
**Analogs:** `S1/experiment/run.sh`; invocation block in RESEARCH "Invocation". Add `trap 'kill $DRIFT_PID' EXIT`, `env -u` scrub of all `CLAUDE*`/`ANTHROPIC*`, `#!/usr/bin/env bash` (user shell is fish). Run launcher with `run_in_background`.

### `README.md`
**Analog:** `S2/README.md` L1-L10 frontmatter:
```yaml
---
spike: 003
idea: queue-driven-master-agent
name: divergent-gap-loop-validation
type: standard
validates: "Given ... "
verdict: PARTIAL|VALIDATED|INVALIDATED
related: [001, 002]
tags: [driver, ...]
---
```
Sections follow S2: "What Validates", "How to Run", results, "Not validated". Pre-register the GO/ADJUST/NO-GO rubric before Run 1; record Claude 2.1.302.

### `MANIFEST.md` row
Append after the 002 row, same pipe columns: `| 003 | queue-driven-master-agent | divergent-gap-loop-validation | standard | <validates> | <verdict> | <tags> |`.

## Shared Patterns

- **Conventions (`.planning/spikes/CONVENTIONS.md` L25-L35):** unset parent `CLAUDE_CODE_*`/`CLAUDECODE`/`CLAUDE_PID` before launch; Haiku misbehaving-driver test before Opus spend; measure cost per transcript; secrets never copied.
- **Results hygiene:** `experiment/results/<run>/` holds only derived files (LOG.md, state.json, DIRECTION final, cost*.txt, dup-report.txt, drift.log, VERIFICATION snapshots), mirroring `S2/experiment/results/R2-hidden-fixture/`. Gate: `grep -rE 'sk-ant|oat01|accessToken'` on results (use `rtk proxy`).
- **Python:** stdlib only, run `python3 -I`, shebang `#!/usr/bin/env python3`, docstring with `Usage:`.
- **Bash:** `#!/usr/bin/env bash`, `set -euo pipefail`, `SELF=` dirname idiom, header comment documenting variants and usage.

## No Analog Found

| File | Role | Reason |
|---|---|---|
| `tools/make_variant.py` | transform | No existing text-variant generator; use RESEARCH Pattern 2 |

## Metadata

**Analog search scope:** `.planning/spikes/` (tracked files only, `git ls-files`)
**Files scanned:** ~45 listed, 5 read in part
**Pattern extraction date:** 2026-10-16
