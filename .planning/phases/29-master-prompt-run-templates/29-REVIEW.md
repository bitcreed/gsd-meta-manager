---
phase: 29-master-prompt-run-templates
reviewed: 2026-10-20T00:00:00Z
depth: standard
files_reviewed: 34
files_reviewed_list:
  - scripts/launcher-acceptance.sh
  - scripts/prompt-acceptance.sh
  - src/cli.rs
  - src/driver_cli.rs
  - src/driverctl/store.rs
  - src/driver_templates/assets/DIRECTION.md
  - src/driver_templates/assets/LOG.md
  - src/driver_templates/lint.rs
  - src/driver_templates/mod.rs
  - src/launcher/assets/MASTER-PROMPT.md
  - src/launcher/config_dir.rs
  - src/launcher/gate.rs
  - src/launcher/mod.rs
  - src/launcher/outcome.rs
  - src/launcher/prompt.rs
  - src/launcher/run.rs
  - src/lib.rs
  - src/main.rs
  - src/next_cli.rs
  - src/state_reader/driver_frontmatter.rs
  - tests/driverctl_client.rs
  - tests/driverctl_frontmatter.rs
  - tests/driverctl_reconcile.rs
  - tests/driverctl_store.rs
  - tests/driver_new.rs
  - tests/driver_prompt.rs
  - tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md
  - tests/launcher_caps.rs
  - tests/launcher_config_dir.rs
  - tests/launcher_e2e.rs
  - tests/launcher_reconcile.rs
  - tests/launcher_support/mod.rs
  - tests/next_cli.rs
  - tests/prompt_acceptance.rs
findings:
  critical: 0
  warning: 7
  info: 5
  total: 12
status: issues_found
---

# Phase 29: Code Review Report

**Reviewed:** 2026-10-20
**Depth:** standard
**Files Reviewed:** 34
**Status:** issues_found

## Summary

Scope note: `diff_base` 73b3971b pre-dates Phase 28, so most launcher files show as wholly new. The Phase 29 change set starts at 6ffda4c; review effort went to that range (`git diff 6ffda4c..HEAD`) plus the real defects visible in the touched files.

The Phase 29 additions are generally careful. `driver new` refuses symlinks and path escapes before creating anything, the `next` command is read-only with a closed-set output, the recovery bookkeeping survives a store restart, and the prompt/protocol vocabulary is pinned by tests. No data-loss or exploitable-security blocker was found.

The weaknesses are mostly places where two readers of the same file disagree (the DIRECTION lint versus the store's parser), fail-open behavior in the human pause/stop control, one false-positive in the gate's phase matcher, and acceptance evidence that records an unresolved FAIL.

No `<structural_findings>` or `<external_reviewer_evidence>` blocks were provided.

## Critical Issues

None.

## Warnings

### WR-01: DIRECTION lint and the store's parser read different plans, so `driver check` can certify a file the store reads differently

**File:** `src/driver_templates/lint.rs:96-115,216-309` vs `src/driver/direction.rs:144-196`
**Issue:** The lint blanks HTML comments and only recognizes `### I-n` headings under `## Items` and `- control:` under `## Direction`. The store's `parse_direction` (used for the gate's control word, for item validation in `driverctl log`, and for the frontmatter counts) does neither. It runs `(?m)^### (I-\d+)\b` and `(?m)^- control:\s*(\w+)` over the whole raw body.
A human who comments out an item with a multi-line `<!-- ... -->` block, or leaves a stray `### I-09` under `## Direction` or a later section, gets a clean `driver check` and a clean launch preflight. The store still counts that item (it appears as `pending` in the frontmatter, can be dispatched, and holds `awaiting_recovery`). Duplicate `- control:` lines are likewise not flagged by the lint, although only the first one is honored. This defeats the purpose of DRIVE-03 ("a plan the driver can follow") for the case it most needs to catch.
**Fix:** Make one reader authoritative. Either have `parse_direction` blank comments with the same `blank_comments` helper and scope headings by section, or have `lint_direction` cross-check against `parse_direction(body)`:
```rust
let store_view = crate::driver::direction::parse_direction(body);
let lint_ids: Vec<&str> = items.iter().filter(|i| !i.id.is_empty()).map(|i| i.id.as_str()).collect();
for it in &store_view.items {
    if !lint_ids.contains(&it.id.as_str()) {
        problems.push(Finding { line: 1,
            message: format!("{}: item heading outside `## Items` or inside a comment is still read by the driver", it.id) });
    }
}
if controls.len() > 1 {
    problems.push(Finding { line: controls[1].0, message: "more than one `- control:` line".to_owned() });
}
```

### WR-02: The human `pause`/`stop` control fails open on a read error and on any non-lowercase spelling

**File:** `src/driverctl/store.rs:645-652,1121`, `src/launcher/gate.rs:72`, `src/driverctl/store.rs:989-1010`
**Issue:** `direction_info()` returns `DirectionInfo::default()` (control `None`, no items) when `DIRECTION.md` cannot be read. The gate then treats that as "no control word" and allows the dispatch. `decide_dispatch` and `recovery_rejection` also match only the exact lowercase strings `"pause"` and `"stop"`. A human who saves `- control: Pause` or `PAUSE` mid-run, or who hits a transient read failure, gets no pause, and the run keeps spending. The lint rejects non-lowercase words, but only at launch; a mid-run edit is never linted. The gate is the only enforcement of D-18.
**Fix:** Fail closed for the control word and normalize case.
```rust
// store.rs direction_info: keep the previous good control word, or treat an unreadable file as "stop"
// gate.rs / recovery_rejection:
match control.map(str::to_ascii_lowercase).as_deref() {
    Some(word @ ("pause" | "stop")) => { /* deny */ }
    _ => {}
}
```
At minimum, log a `RUN | direction-unreadable` event when the read fails so the failure is visible.

### WR-03: DIRECTION.md content reaches the terminal unescaped through lint findings

**File:** `src/driver_templates/lint.rs:267-271,294-299,337-339,382-390,408-424` (message construction); `src/driver_cli.rs:440-445`; `src/launcher/run.rs:229-250` and `src/launcher/mod.rs:162` (preflight output)
**Issue:** `Finding` messages interpolate raw DIRECTION text (`line.trim()`, `value`, `piece`, `part`, `v`) and are printed with plain `writeln!`. A DIRECTION.md arriving with a cloned repository can carry escape sequences or bidi controls, and `driver check` or `driver launch` then writes them to the operator's terminal. The same change set escapes everything else it prints (`render_for_terminal` in `driver_templates/mod.rs` and `next_cli.rs`; the identity-census work around `text.rs`). This is the one path that skips that discipline. `new_main` (`src/driver_cli.rs:354`) also echoes the user-supplied `{target}` raw in its "next commands" lines, which is lower risk but unquoted for copy-paste.
**Fix:** Run every interpolated untrusted fragment through `crate::text::render_for_terminal(..)` when building the `Finding` message (or once in `Display for Finding`), and shell-quote `target` in the printed `check:` and `launch:` command lines.

### WR-04: `mentions_phase` attributes a decimal phase to its integer parent, so the gap-round gate denies the wrong dispatch

**File:** `src/launcher/gate.rs:47-55`
**Issue:** The pattern ends in `\b`, and a word boundary exists between `6` and `.`. For key `26`, the prompts `phase 26.1` and `/gsd-plan-phase 26.1 --gaps` both match (verified with the equivalent regex). For key `1`, `26.1 --gaps` matches through the `\b0*1\s+--gaps` alternative. This project uses decimal phases routinely (26.1, 21.x). Once phase 26 is over the gap cap, every dispatch for 26.1 is wrongly denied with "phase 26 has failed verification N times". The existing test `a_decimal_phase_key_is_matched_literally` only checks the opposite direction (key `26.1` against `phase 26`).
**Fix:** Replace the trailing `\b` with a lookahead-free equivalent that rejects a following `.digit`.
```rust
// regex crate has no lookahead; consume the next char instead:
let pattern = format!(r"(?i)(?:phase[\s_-]*0*{ph}|-phase\s+0*{ph}|\b0*{ph}(?:\s+--gaps))(?:$|[^0-9A-Za-z.]|\.(?:$|[^0-9]))");
```
Add tests for key `26` against `phase 26.1` and key `1` against `26.1 --gaps`.

### WR-05: DIRECTION template promises an `isolation:` knob the master prompt never reads

**File:** `src/driver_templates/assets/DIRECTION.md:23-24` vs `src/launcher/assets/MASTER-PROMPT.md:58-61,193-196`
**Issue:** The template ships `- isolation: worktree` with the comment "Passed through to workers for nested GSD dispatches that require it", and the o1 acceptance case sets `PA_ISOLATION=none`. The prompt never mentions the `isolation` key: hard rule 3 and the worker brief hard-code `isolation: "worktree"` whenever the workflow calls for it. A human who writes `isolation: none` is silently ignored, and the lint does not check the key either. The version-pinned prompt contract (driver-prompt/1.0.0) therefore diverges from the template contract.
**Fix:** Either drop the key from the template, or add to the Loop step and worker brief: "read `isolation:` from DIRECTION.md (`worktree` default); pass it through as the nested-dispatch isolation". If the prompt changes, bump `DRIVER_PROMPT_VERSION` and regenerate the no-gap-rules fixture. Optionally validate `isolation` in the lint (`worktree|none`).

### WR-06: The "reconcile first" refusal is enforced only on the `dispatched` log record, not on the dispatch itself

**File:** `src/driverctl/store.rs:989-1010` (`recovery_rejection`), `src/launcher/gate.rs:62-101`
**Issue:** Commit 4a86684 is titled "refuse dispatch until each running item is recovered once", but `awaiting_recovery` is consulted only when the driver logs `dispatched`. The gate that sees the real `PreToolUse Agent|Task` hook (`decide_dispatch`) receives `&RunState` containing `awaiting_recovery` and ignores it. A driver that calls the Agent tool without first logging `dispatched` dispatches un-reconciled work; the only trace is `observed > logged` in the dispatch audit. If D-17 is meant as a real dispatch refusal (the project's stance is that the driver is untrusted), this is a gap.
**Fix:** In `decide_dispatch`, deny while any `state.awaiting_recovery` id is still listed in DIRECTION (same filter as `recovery_rejection`), with a `reconcile first: ...` reason. If the intent really is audit-only, say so in the D-17 text and the commit/summary wording.

### WR-07: The H2 acceptance case cannot stage its premise, and the committed evidence is an unresolved FAIL

**File:** `scripts/prompt-acceptance.sh:196-231,948-986`; evidence `.planning/phases/29-master-prompt-run-templates/acceptance/h2/verdict.txt`
**Issue:** The committed H2 run ends `H2-5 FAIL launch 2: I-01 recovered x0`. The log shows why: the pause was saved at 13:29:09 while the worker ran `sleep 90`, but the driver recorded the worker's report, `verified`, and `status --state done` for I-01 before ending on `pause` at 13:30:59. I-01 was `done`, not `running`, so no item awaited recovery and `recovered` correctly never appeared. The case design assumes the item is still `running` at relaunch, which is true only if the driver dies or the worker is still in flight at `end`. `pa_evidence_guard` then refuses to rerun a logic FAIL (`infra_fault=no`). The result is that D-18/D-19's live "pause mid-dispatch then relaunch reconciles" claim is unproven live, and the harness cannot be corrected without editing the guard. (The offline `driverctl_reconcile` and `launcher_reconcile` tests do cover the store logic.)
**Fix:** Redesign H2 so that the item is still `running` when the run ends on pause (for example, a second item under `after: I-01` and a driver ending on a deny while I-01 is mid-worker, or SIGKILL the driver after the pause lands), update the rubric accordingly, and either record H2 as a known-unmet criterion in the phase verification or add a justified rerun path.

## Info

### IN-01: Prompt uses `$RUN_DIR` and `$PROJECT_ROOT` as if they were environment variables

**File:** `src/launcher/assets/MASTER-PROMPT.md:20-24,131`
**Issue:** The launcher substitutes `{RUN_DIR}`/`{PROJECT_ROOT}` textually, but the driver environment only gets `DRIVER_LOCK_HELD` (`src/launcher/run.rs:1367`). `cat $RUN_DIR/DIRECTION.md` or `git -C $PROJECT_ROOT log` run literally would expand to an empty string. Models usually substitute from the Inputs line, so this is a latent trap.
**Fix:** Export `RUN_DIR` and `PROJECT_ROOT` in `child_env`, or spell the paths in the prompt as `{RUN_DIR}/DIRECTION.md` so the substitution happens.

### IN-02: `pa_selftest` skip logic for a missing spike directory has the wrong precedence

**File:** `scripts/prompt-acceptance.sh:410-411`
**Issue:** `[ ! -d "$S3" ] || [ -f A ] && [ -f B ] && [ -f C ] || st_fail` parses as `((! -d || -f A) && -f B && -f C) || st_fail`. With no spike directory the guard is true, then `-f B` is false, so `st_fail` fires. The intended "skip when absent" never happens.
**Fix:** `if [ -d "$S3" ]; then [ -f A ] && [ -f B ] && [ -f C ] || st_fail "..."; fi`.

### IN-03: Preflight reorders refusals: a paused or invalid DIRECTION now masks "already launched"

**File:** `src/launcher/run.rs:229-253,551-553`
**Issue:** `preflight_direction` runs before the run lock. A second `driver launch` against a run whose live launcher is healthy, after the human set `control: pause`, now exits 1 with "control is pause" instead of exit 3 with "run is already launched". It also reads with `read_to_string`, so a non-UTF-8 DIRECTION.md (which the store reads leniently via `from_utf8_lossy`) is refused with a generic I/O error.
**Fix:** Probe the lock first (`probe_at`) and report `Locked` before linting, and read the file as bytes with `String::from_utf8_lossy` so the lint and the store see the same text.

### IN-04: `DEFAULT_GAP_CAP` duplicates the CLI default

**File:** `src/driver_templates/lint.rs:35`, `src/cli.rs` (`cap_gap_rounds` default)
**Issue:** The lint warns against a hand-copied `2`; only a test in `cli.rs` pins the pairing. Likewise `DRIVER_RUNS_REL` (state_reader) and `DRIVERS_REL` (serve) are two constants for one path, now both used by `driver check`/`driver new`.
**Fix:** Export one `DEFAULT_CAP_GAP_ROUNDS` constant used by both clap's `default_value_t` and the lint, and reuse a single path constant.

### IN-05: `shell_quote` is lossy for non-UTF-8 paths

**File:** `src/launcher/config_dir.rs:101-104`
**Issue:** `String::from_utf8_lossy` replaces invalid bytes with U+FFFD, so the `driverctl` and `gsd-meta-manager` shims would `exec` a different path than the real binary or run dir. Rare, but silent.
**Fix:** Build the quoted word from the raw bytes (`OsStrExt::as_bytes`, escaping `'`), or refuse to seed when the binary path is not valid UTF-8.

---

_Reviewed: 2026-10-20_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
