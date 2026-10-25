---
phase: 31-retire-superseded-envelope-goal-layer-code
reviewed: 2026-10-25T00:00:00Z
depth: standard
files_reviewed: 57
files_reviewed_list:
  - Cargo.toml
  - CLAUDE.md
  - docs/CONFIGURATION.md
  - docs/GSD-CORE-SYNC.md
  - .github/workflows/release.yml
  - README.md
  - scripts/install-conformance-oracle.sh
  - scripts/pre-tag-check.Dockerfile
  - scripts/pre-tag-check.sh
  - src/action.rs
  - src/app.rs
  - src/bounded_output.rs
  - src/cli.rs
  - src/driver/kill.rs
  - src/driver/liveness.rs
  - src/driver/lock.rs
  - src/driver/mod.rs
  - src/driver/rate_limit.rs
  - src/driver/router.rs
  - src/envelope/ledger.rs
  - src/envelope/mod.rs
  - src/envelope/policy.rs
  - src/error.rs
  - src/executor/claude.rs
  - src/executor/codex.rs
  - src/executor/mod.rs
  - src/experimental.rs
  - src/journal/mod.rs
  - src/journal/reader.rs
  - src/journal/redact.rs
  - src/launcher/push.rs
  - src/main.rs
  - src/next_cli.rs
  - src/registry.rs
  - src/state_reader/git_ops.rs
  - src/state_reader/mod.rs
  - src/state_reader/roadmap_md.rs
  - src/state_reader/state_md.rs
  - src/text.rs
  - src/ui/glyphs.rs
  - src/ui/mod.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/driver_run.rs
  - src/ui/screens/help.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/preferences.rs
  - src/ui/screens/render_escape_guard.rs
  - src/watcher.rs
  - tests/async_blocking_guard.rs
  - tests/common/mod.rs
  - tests/envelope_pr_cap.rs
  - tests/launcher_push.rs
  - tests/spawn_seam_guard.rs
findings:
  critical: 0
  warning: 5
  info: 6
  total: 11
status: issues_found
---

# Phase 31: Code Review Report

**Reviewed:** 2026-10-25
**Depth:** standard
**Files Reviewed:** 57
**Status:** issues_found

## Summary

The review used the real Phase 31 span (`81c2ad7..HEAD`, since `73b3971` predates the phase and pulls in
earlier phases' code) and read the surviving files in full where they carry logic: `launcher/push.rs`,
`envelope/{policy,mod,ledger}.rs`, `journal/{mod,reader,redact}.rs`, `driver/{lock,kill,mod}.rs`,
`ui/screens/{driver_confirm,delete_confirm,preferences}.rs` and the `app.rs` tick path. `cargo clippy
--all-targets` is clean.

The mechanical retirement is sound. I found no dangling references to deleted modules, no behavioural
regression in the push step's namespace refusal or secret scan, and no regression in the PR ledger's tally
logic. The SAFE-04 rewiring (`redact_captured`) is applied to both child-stderr persistence sites in
`push.rs` and is covered by a unit test and an end-to-end test.

The defects are almost all of one kind: **user-facing security text and one safety guard that the deletion
left describing, or depending on, a control that no longer exists.** The opt-in disclosure still presents a
consent flow for retired model seams and a "re-confirmation" nothing performs. The opt-in prompts claim a
branch-namespace control the push step ignores. The project-delete guard against abandoning a live
autonomous run was deleted and not re-aimed at the launcher run that replaced it. There is also one narrow
bypass of the new redaction.

## Warnings

### WR-01: Opt-in disclosure still describes retired model seams and a re-confirmation that nothing performs

**File:** `src/ui/screens/driver_confirm.rs:36-76` (constants `SECTION_PROMPT_INPUTS`, `SECTION_MODEL_SEAM`,
`SECTION_RESIDUAL_EXPOSURE`), rendered at `:352-366`; also `src/error.rs:512` (`PromptInputsDrifted`)
**Issue:** Granting a driver opt-in (Driver tab `o`) still shows this disclosure verbatim:

- "Files whose bytes can reach a model prompt" with `STATE.md`, `ROADMAP.md` and `HANDOFF` labelled
  `[model seam]`.
- "The goal-decomposition and ambiguity seams run with an empty tool set... CLAUDE.md does NOT enter their
  prompts."
- "The executor profile... DOES still load this project's CLAUDE.md... that exposure is the reason the
  digest and the re-confirmation above exist."

Phase 31 deleted the goal layer (the goal-decomposition seam) and `drive`. `DrivableProject::from_registry`
is now the only caller of `registry::check_prompt_input_drift`, and `from_registry` has no production caller
left (its callers were `drive` and the dry-run preview). So nothing consults the recorded digests, nothing
re-confirms on a `CLAUDE.md` change, and the launcher does not gate on opt-in at all (as
`docs/CONFIGURATION.md` now says). The popup is the last thing an operator reads before recording consent,
and it describes a control that does not exist. It also contradicts the prompt text directly above it, which
says the record only feeds PR caps and the branch namespace. The `prompt_text` doc cites T-17-48 ("no
safety claim the code does not back"), so the disclosure breaks the rule that same file states.
**Fix:** Drop `SECTION_MODEL_SEAM`, `SECTION_RESIDUAL_EXPOSURE` and the prompt-input file list from the
grant popup, or replace them with one sentence naming what the record actually feeds. Retire or
re-document `current_prompt_inputs` and the digest recording. Update
`the_disclosure_names_the_file_set_the_seams_and_what_is_not_closed` and the `PromptInputsDrifted` message
to match. If the digest is kept deliberately for plan 31-07, mark the popup text "not enforced".

### WR-02: Opt-in prompts and docs claim the push step reads a branch namespace from the opt-in; it does not

**File:** `src/ui/screens/driver_confirm.rs:14, 209, 237, 252`; `docs/CONFIGURATION.md` (the "`driver_opt_in`
is not a launch gate" bullet); code at `src/launcher/push.rs:162-177, 198-209`
**Issue:** The grant and withdraw prompts say "Its PR caps and branch namespace stop applying" and "The
push step reads its PR caps and branch namespace from it". `docs/CONFIGURATION.md` says the record "carries
the per-project PR caps and branch namespace the push step reads". In `push.rs`, `policy_for`'s own doc says
"Only the PR caps are read from it; the destination is always the run's own branch". `dest_ref` hard-codes
`DEFAULT_NAMESPACE_ROOT/<alias>/<run-id>` and `check_destination` uses `default_namespace(alias)`.
`ctx.policy.namespace` is never read; `grep` finds no consumer outside `envelope/policy.rs`, `config.rs` and
`registry.rs`. A user who sets `branch_namespace` is told it applies and it silently does not. The failure
direction is safe (the default namespace is the stricter one), but the text is exactly the unbacked claim
the Phase 31 rewrite set out to remove.
**Fix:** Say "PR caps" only in both prompts, in `docs/CONFIGURATION.md` and in the `driver_confirm.rs`
module doc. Either remove `EnvelopePolicy.namespace` and `DriverOptIn.branch_namespace` handling, or make
`check_destination` honour `ctx.policy.namespace` (after `validate_namespace`).

### WR-03: Unregistering a project is no longer guarded against a live launcher run

**File:** `src/ui/screens/delete_confirm.rs:121` (`do_remove_project`; the deleted block was CR-06/D-11/D-27)
**Issue:** The removed guard refused `d` + `y` while the project had an observed run whose liveness was not
`Dead`. The stated reason was that a removal is irreversible from the TUI and leaves an autonomous agent
with git and push rights running with no way back. Plan 31-02 deleted it on the grounds that the TUI "can no
longer observe a legacy `drive` run". The launcher run that replaced `drive` is observable: it has
`launcher_marker.any_live` and `MarkerClass::Driven`, which `detail.rs` and `normal.rs` already use for
"driven and live". The same hazard now has no guard. After removal the project drops out of the
launcher-marker scan (`launcher_markers_for` iterates `config.projects`), so the Driven badge and the Driver
tab vanish while the detached launcher and its sandbox scope keep running. `driver stop <path>` still works,
but the TUI offers no hint that a run is live.
**Fix:** Re-aim the guard at the replacement signal, so it stays gated on `ctx.experimental` like the other
driver surfaces:

```rust
if ctx.experimental
    && ctx.view_cache.get(alias)
        .and_then(|c| c.launcher_marker)
        .is_some_and(|m| m.any_live)
{
    ctx.error_message = Some(format!(
        "'{}' has a live driver run. Stop it from its Driver tab (x) first.",
        crate::text::render_for_terminal(alias)
    ));
    ctx.needs_redraw = true;
    return;
}
```

Restore the matching refusal and control-arm tests, or record an explicit decision that unregistering a
project with a live run is allowed.

### WR-04: PR-ledger refusal message cites a "5-second deadline" the push step no longer has

**File:** `src/envelope/ledger.rs:51-54` (`COUNT_DEADLINE_SECS`), `:434-443` (user-visible error); module
docs `:56-132`, `:553-559`
**Issue:** When the ledger exceeds `MAX_LEDGER_BYTES` the error shown to the operator reads "past the
8388608-byte bound the push step can count inside its 5-second deadline". The constant's own doc says it is
"the budget the retired Phase 19 guard allowed, kept as the push step's own bound". Nothing enforces a 5 s
deadline on `record_and_check`: `open_pull_request` calls it synchronously with no timeout, and the only
budgets in `push.rs` are `GIT_QUICK`/`GIT_FETCH`/`GIT_PUSH`/`GH_BUDGET`. The size and kind bounds still do
their job, but the message tells the operator something false about the system. The module docs also
describe an agent shell command that "may inflate" the ledger via redirection "under the retired guard" and
a `crate::journal::reader` disposition that no longer exists, so they now read as live threat-model
statements.
**Fix:** Reword the error to the true reason ("too large to count safely on the push step's path") and drop
or rename `COUNT_DEADLINE_SECS`. Trim the module docs to what still holds.

### WR-05: SAFE-04 redaction can be bypassed when the captured stderr line exceeds the 4 KiB tail window

**File:** `src/launcher/process.rs:185-203` (`run_bounded_with_stderr`), consumed by
`src/launcher/push.rs:247-249` (`redact_captured`)
**Issue:** Only the last 4096 bytes of the sink are read back. If the final stderr line is longer than
4096 bytes, the window begins mid-line and `lines().rev().find(non-empty)` returns that fragment. A token
cut at the window start has lost its prefix (`ghp_`, `github_pat_`, `sk-ant-...`), so the redactor's
prefix-anchored regexes no longer match. The raw body then goes into LOG.md via `log_event("pr-failed" |
"push-failed")` and into `PushOutcome::Failed.detail`. The new `redact_captured` is only as strong as the
bytes it is handed. The line is also persisted with no length cap. Likelihood is low (it needs a
single-line stderr over 4 KiB), but this is the control Plan 31-01 added.
**Fix:** If the window did not start at a line boundary (`len > STDERR_TAIL_BYTES` and the first selected
line is the first line in the window), drop that partial first line, or redact it with a conservative
long-run-of-token-characters pattern. Cap the returned line, for example 512 chars, before logging:

```rust
let line: String = line.chars().take(512).collect();
```

## Info

### IN-01: Orphaned comment block above the `Driverctl` arm describes arms that no longer exist

**File:** `src/main.rs:321-336`
**Issue:** The block ("This arm is BEFORE `tui::init()`... A git hook's stdout and stderr... The envelope
re-entry arms judge their alias and FAIL CLOSED... 1 for the hooks and askpass, 2 for the guard") now sits
on the `Commands::Driverctl` arm. The hook, askpass and guard arms were deleted. It describes
`judged_alias_or_exit`, which is gone.
**Fix:** Delete the block. The surviving `Envelope::Scan` arm already carries its own accurate comments.

### IN-02: Slow-tick comments still reference the deleted reconciliation probe, inbox and journal

**File:** `src/app.rs:1855-1871, 1920`; doc comments at `:106-110`, `:149`
**Issue:** The 20-tick block says the Driver tab's "run list, inbox and journal" ride the counter, that "a
queued injection never reaches the four-state widget", and refers to "the reconciliation probe above" three
times and "three blocking reads". Only `rescan_driver_tab_if_open` (one `schedule_launcher_refresh`) and
the launcher-marker scan remain. The `launcher_heartbeat_redraw_wanted` doc has a broken hand-wrapped line,
and `launcher_markers_for` says it runs "inside the reconciliation `spawn_blocking` task".
**Fix:** Rewrite the comments to describe the two scans that exist.

### IN-03: Opt-in toggle popup dims the dashboard even though it is only reachable from the Driver tab

**File:** `src/ui/screens/driver_confirm.rs:305-313`
**Issue:** The comment says "the opt-in toggle keeps the dashboard, as it did when the dashboard also opened
it". Phase 31 removed the dashboard `o` key, so the only entry is `detail.rs:5991` (Driver tab). The popup
renders `NormalScreen` behind it while the user is in a detail view, so the background changes under them.
**Fix:** Render `DetailScreen::render_main_only` for both actions and drop the branch.

### IN-04: User-visible Preferences text names an internal phase number

**File:** `src/ui/screens/preferences.rs:86-89`
**Issue:** The screen shows "legacy driver, removed in Phase 31". End users have no frame of reference for
it, and it will go stale.
**Fix:** "(none: this build has no manager-wide settings)", or drop the manager section entirely.

### IN-05: Leftover naming and attributes from retired code

**File:** `src/ui/screens/driver.rs:98` (`DRY_RUN_LOADING`, still used as the generic "Loading..." string in
`driver_run.rs:990, 1724, 2101`), `src/journal/redact.rs:434, 453` (`#[allow(dead_code)]` on a `pub struct`
and a `pub(crate)` accessor; the `pub` item cannot warn in a lib crate, so the attribute is dead), and
`src/cli.rs:530` (a test message still saying "the TUI's `drive` respawn").
**Fix:** Rename the constant to `LOADING_TEXT`, remove the redundant `allow` on the `pub` struct, reword
the test message.

### IN-06: Release docs and packaging comments retain stale claims

**File:** `CLAUDE.md:135`, `Cargo.toml:27-28`
**Issue:** `CLAUDE.md` says a bare pre-tag run is unreliable because the machine has "a different ambient
git than the runner". That was the reason for the retired git-version witness, and the Dockerfile now says
no test pins a git version. `Cargo.toml` says "any fixture CLAUDE.md under tests/ ships with the crate", but
no `CLAUDE.md` exists under `tests/` any more (the injection-corpus fixtures were deleted).
**Fix:** Drop the "different ambient git" clause and the second `Cargo.toml` sentence.

---

_Reviewed: 2026-10-25_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
