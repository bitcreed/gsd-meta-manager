# Phase 31: Retire Superseded Envelope & Goal-Layer Code - Pattern Map

**Mapped:** 2026-10-24
**Files analyzed:** 27 modified or receiving files (pure whole-file deletions are listed in 31-RESEARCH.md §Deletion/Keep Inventory and are not repeated here)
**Analogs found:** 26 / 27

Phase 31 mostly deletes code. Most "analogs" below are the surviving half of the same file, or a sibling
that already does what the moved code needs to do. Line numbers are at HEAD `036426d`.
**`src/launcher/*` is being edited concurrently by the 28.1 code-review loop.** For those files this map cites
function names and signatures, not line numbers. Re-locate them with `rtk proxy grep -n` at execution time.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/envelope/ledger.rs` (decouple `GUARD_TIMEOUT_SECS`; receive 8 bound tests) | service | file-I/O | own `mod tests` (ledger.rs:620-700) + the corpora rows being moved | exact |
| `tests/envelope_pr_cap.rs` (retarget onto `record_and_check_in`) | test | file-I/O | `src/envelope/ledger.rs` `mod tests` + this file's own `seed`/`ledger_lines` helpers | exact |
| `tests/common/mod.rs` (drop hooks) | test utility | — | itself, reduced to `git()` | exact |
| `src/launcher/push.rs` (SAFE-04 routing, D-09/C11) | service | request-response (child stderr to LOG.md) | `redact_urls` + its test `credentials_in_a_url_never_survive_redaction` (same file) | exact |
| `src/next_cli.rs` (local refusal, C8) | CLI | request-response | `resolve_root`'s `shown()` refusal pattern (next_cli.rs:68-95) + `DriveError::TargetPhaseInvalid` text (error.rs:998-1006) | exact |
| `src/app.rs` 20-tick task + `RunsReconciled` handler (C7 rewire) | event loop | event-driven | `Action::AgentsScanned` (action.rs:216) / `AppContext::apply_launcher_markers` (ui/screens/mod.rs:1966) | exact |
| `src/action.rs` (drop/rename `RunsReconciled`) | model (message) | event-driven | `AgentsScanned { per_project }` (action.rs:199-218) | exact |
| `src/journal/mod.rs` `classify_change` (runs/** to `Ignored`) | utility | transform | the `drivers` arm (journal/mod.rs:1181-1196) + test `classify_change_ignores_every_other_drivers_path` (:2355) | exact |
| `src/driver/rate_limit.rs` (receive classify test; drop `bounds::REASON_*`) | service | transform | own `mod tests` (rate_limit.rs:386-400, 853-880) | exact |
| `src/ui/screens/driver_confirm.rs` (trim, C1/C12) | component | request-response | surviving `StopLauncher` arm in itself (:256-262, :344-353) | exact |
| `src/ui/screens/driver.rs` (trim to shared helpers, C2) | component | render | itself; consumer list in `driver_run.rs:35-39` | exact |
| `src/ui/screens/driver_run.rs` (legacy section removal) | component | render | itself (Phase 30, concurrent 28.1 WR-09 edits) | exact |
| `src/driver/mod.rs` (module decls + doc) | config (module root) | — | its own decl block (driver/mod.rs:60-120) | exact |
| `src/driver/kill.rs`, `src/driver/lock.rs` (drop legacy fns) | service | file-I/O | kept `signal_group`/`acquire_at` siblings in the same files | exact |
| `src/envelope/policy.rs` (keep 15-189 + 2418-2490) | service | transform | itself | exact |
| `src/envelope/mod.rs` (drop `park*`, three `pub mod`, doc) | config | — | itself (:131-136) | exact |
| `src/cli.rs` / `src/main.rs` (Envelope keeps only `Scan`, C3) | CLI | request-response | the existing `Scan` arm | exact |
| `src/error.rs` (drop `DriveError`, goal errors) | model | — | itself; `registry.rs:2106-2200` census arms | exact |
| `tests/spawn_seam_guard.rs`, `tests/async_blocking_guard.rs` (census edits) | test | batch (source scan) | their own `SPAWN_ALLOWLIST` (:48) / `REQUIRED_ASYNC_FILES` (:276) | exact |
| `src/ui/mod.rs` `MEASURED_REACH` (:357-361), `driver.rs` `CENSUS_FILES` (:2015) | test | batch | themselves | exact |
| `scripts/pre-tag-check.sh`, `scripts/pre-tag-check.Dockerfile`, `.github/workflows/release.yml`, `scripts/install-conformance-oracle.sh`, `Cargo.toml` comment, `CLAUDE.md` step 3 | config / tooling | batch | themselves (excerpts below) | exact |
| `tests/no_container_crate.rs` (NEW, optional, RETIRE-02) | test | file-I/O | `tests/launcher_acceptance.rs:25-53` (`repo()` + skip-reason) | role-match |
| `.planning/phases/19-.../deferred-items.md` (Phase 31 closure section) | doc | — | its own "Still OPEN and unchanged" sections (:3186-3228) | exact |
| `.planning/phases/19-.../19-SECURITY.md` (frontmatter + closure table) | doc | — | own frontmatter (:1-25); table shape from `28.1-SECURITY.md:44-47` | exact |
| `.planning/phases/21-.../deferred-items.md` (criterion-4 closure) | doc | — | its "RE-SURFACED, UNCHANGED — ROADMAP success criterion 4" sections (:469, :649) | exact |
| `.planning/phases/22-.../22-0{1..4}-PLAN.md` frontmatter + NEW `22-SUPERSEDED.md` | doc | — | `superseded_by:` precedent in `.planning/quick/260915-f4n-.../260915-f4n-SUMMARY.md:74-82`; ROADMAP §Phase 22 banner (ROADMAP.md:705-709) | exact |
| `.planning/todos/pending/<date>-session-detector-resume-optional-arity.md` (NEW, CR-01; maybe CR-02) | doc | — | `.planning/todos/pending/2026-10-18-dashboard-verification-status-quote-divergence.md` | exact |
| `.planning/REQUIREMENTS.md` traceability | doc | — | rows at REQUIREMENTS.md:336-343, 409-411 | exact |

## Pattern Assignments

### `src/envelope/ledger.rs` (service, file-I/O)

**Analog:** its own test module (ledger.rs:620-669) for the receiving shape. The rows being moved come from
`tests/envelope_carrier_reach.rs:1884,2082`, `tests/envelope_interior_path.rs:1985,2029,2078`, and
`tests/envelope_word_set.rs:2154,2201,2276`.

**Decoupling site** (ledger.rs:434-443; docs at :6, :60, :171, :258, :345):
```rust
return Err(anyhow!(
    "the pull-request ledger at {} is {size} bytes, past the {MAX_LEDGER_BYTES}-byte \
     bound this guard can count inside its registered {}-second deadline, ...",
    path.display(),
    super::hooks::GUARD_TIMEOUT_SECS        // <- replace with a ledger-owned const
));
```
Add the new constant beside `WINDOW_SECS` (ledger.rs:49-51), using the same doc style: one `///` line saying why the
value exists. Reword "guard" to "push step" in the message and in the doc comments at :58-60, :171 (`hooks::install_in`
reference), :248, :330-345 and :430-433. `MAX_LEDGER_BYTES` and its value stay unchanged.

**Receiving test-module shape** (ledger.rs:620-669):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, SecondsFormat, Utc};

    fn policy(cap_24h: u32, cap_run: u32) -> EnvelopePolicy { EnvelopePolicy { namespace: "refs/heads/gsd-auto/alpha/".to_string(), pr_cap_per_24h: cap_24h, pr_cap_per_run: cap_run, credential: None } }
    fn entry(at: &str, run_id: &str) -> LedgerEntry { ... }
    fn now_stamp() -> String { Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true) }

    #[test]
    fn the_first_attempt_in_an_empty_ledger_is_permitted() {
        let root = tempfile::tempdir().unwrap();
        let verdict = record_and_check_in(root.path(), "alpha", &entry(&now_stamp(), "run-1"), &policy(3, 1)).unwrap();
        assert_eq!(verdict, CapVerdict::Permit { used_24h: 1, used_run: 1 });
    }
```
**How to translate each corpus row.** The corpora go through `ask(envelope, "gh pr create --title x")` and check
`answer.code`. Translate that as follows:
- `code == 0` becomes `record_and_check_in(...)` returning `Ok(CapVerdict::Permit { .. })`.
- `code == 2` with `REASON_ENVELOPE_ASSERTION_FAILED` becomes `record_and_check_in(...)` returning `Err(e)`. Assert
  on the error text (`"not a regular file"`, `"bytes, past the"`). Since D-24 the kind and size bounds are `Err`, not
  `CapVerdict::Refuse`.
- The "counted through the link" check stays a file read: `std::fs::read_to_string(&real).unwrap().lines().count()`
  (envelope_word_set.rs:2177).
- **FIFO rows** (`envelope_interior_path.rs:1985`, `envelope_word_set.rs:2208-2226`) used a subprocess with a 12 s
  timeout. A direct call blocks forever if the bound regresses. Run the call on a `std::thread::spawn` and wait with
  `mpsc::Receiver::recv_timeout(Duration::from_secs(12))`, so a regression fails the test instead of hanging it.
- Fixtures use `std::process::Command::new("mkfifo")` and `std::os::unix::fs::symlink(...)`. Put those tests under
  `#[cfg(unix)]`, because ledger.rs already pairs `#[cfg(unix)]` with portable arms (:287-303).

**Pitfall: moving the source-slice pin into ledger.rs** (envelope_word_set.rs:2276-2355). That test asserts that
`LEDGER_SOURCE.matches("#[cfg(test)]").count() == 1` and slices at the first `#[cfg(test)]`. If it moves into ledger.rs:
- Read the file with `include_str!("ledger.rs")`. That is the same self-include idiom as `envelope/mod.rs:449` and
  `journal/mod.rs:3283`.
- Build the sentinel without writing it literally, e.g. `concat!("#[cfg", "(test)]")`. A literal
  `"#[cfg(test)]"` in the test body would add a second occurrence.
- `"symlink_metadata"` may appear only below the sentinel. The moved symlink test calls `std::fs::symlink_metadata`,
  which is fine because it sits below the sentinel.

The alternative is to keep this one pin as an integration test in `tests/envelope_pr_cap.rs` with
`include_str!("../src/envelope/ledger.rs")`, exactly as written today. That is the lower-risk option.

---

### `tests/envelope_pr_cap.rs` (test, file-I/O)

**Analog:** its own helpers. Keep `seed` (:135-155), `ledger_lines` (:158-166) and `ALIAS` (:24) unchanged.
Delete `BIN`, `Answer`, `ask` (:26-128) and `registry_with_caps` (:39-76). Under `hooks::guard_in` the caps came from
a registry file. Now they come from an `EnvelopePolicy` literal built like ledger.rs:625-632:
```rust
use gsd_meta_manager::envelope::{ledger, policy};
fn policy_with_caps(cap_24h: u32, cap_run: u32) -> policy::EnvelopePolicy {
    policy::EnvelopePolicy { namespace: policy::default_namespace(ALIAS), pr_cap_per_24h: cap_24h, pr_cap_per_run: cap_run, credential: None }
}
fn attempt(root: &Path, run_id: &str, caps: &policy::EnvelopePolicy) -> ledger::CapVerdict {
    let entry = ledger::LedgerEntry { at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true), run_id: run_id.into(), command: "gh pr create".into(), platform: "github".into() };
    ledger::record_and_check_in(root, ALIAS, &entry, caps).expect("a plain alias in a fresh root has a usable ledger")
}
```
Check `default_namespace`'s exact signature in policy.rs 15-189 before using it. Its shape at
ledger.rs:627 is `"refs/heads/gsd-auto/alpha/"`.
- **Retarget** (5 tests): :173, :205, :222, :243, :271. Map `third.permitted()` to `matches!(v, CapVerdict::Permit{..})`,
  and `fourth.reason().contains(REASON_PR_CAP_EXCEEDED)` to `CapVerdict::Refuse { reason, .. }` with
  `reason.as_str() == policy::REASON_PR_CAP_EXCEEDED`. The `"per-run cap"` and `"over-count"` text checks move to
  `verdict.refusal_detail().unwrap()` (ledger.rs:215-241). For the paired `gh pr list` negative at :297-304: under
  direct calls nothing non-creating reaches the ledger. Either drop that half, or restate it as "the push step only
  records `gh pr create`", which `push.rs::open_pull_request` hard-codes.
- **Delete** (5 tests): :311, :345, :366, :406, :452. They cover settings, `GUARD_TIMEOUT_SECS` and
  `disallowed_tools`.
- **Split** :474. Keep only `assert!(ledger::ledger_path_in(envelope.path(), hostile).is_none())`. Add
  `record_and_check_in(...).is_err()` for each hostile alias.
- Rewrite the file header (:1-16): the subject is now the ledger the push step calls, not the `PreToolUse` guard. Keep
  the D-35 "offline, agent-free, clock-free" and D-32 paired-allow paragraphs.

---

### `tests/common/mod.rs` (test utility)

**Correction to research C6:** no kept suite calls `common::fixture()`. All eight `agents_*` suites and
`registry_worktree_guard.rs` import only `use common::git;`, and each header says "never `common::fixture()`"
(e.g. agents_claude.rs:15, registry_worktree_guard.rs:12,21). `fixture`, `Fixture`, `BIN` and `remote_refs` are used
only by `envelope_tracer`, `envelope_credential` and `envelope_advisory`, all of which are deleted.

**Rewrite:** keep `#![allow(dead_code)]` (:15) and `pub fn git` (:61-70) unchanged. Delete `use ...envelope::hooks`
(:20), `BIN`, `Fixture`, `fixture()` and `remote_refs` (:23-59, :72-129). Rewrite the header (:1-14) so it describes a
shared git helper rather than "the `file://` bare-remote harness, shared by every envelope integration test". Do this
in the same commit that deletes `hooks.rs`, or nine kept binaries fail to compile.

---

### `src/launcher/push.rs` (service, request-response; D-09 / C11) — CONCURRENT EDIT ZONE

**Analog:** `pub fn redact_urls(text: &str) -> String` and its unit test
`credentials_in_a_url_never_survive_redaction` in the same file's `mod tests`.

**Sites, by function** (re-locate at execution):
1. `fn git_with_env(args: Vec<OsString>, budget: Duration, more_env: &[(&str, &str)]) -> GitRun`: the
   `Some((out, line)) => GitRun { ..., last_error: redact_urls(&line) }` arm. `last_error` feeds every
   `PushOutcome::Failed { detail }` (fetch, worktree add, push) and the `push-failed` `log_event`.
2. In the push body, after `if !pushed.success {`, `let detail = redact_urls(&pushed.last_error);` goes to
   `log_event(store, "push-failed", vec![("dest", ...), ("detail", Some(detail.clone()))])`. It is already
   URL-redacted at site 1, so after site 1 changes this is a double pass. Either is harmless, because `redact` is
   idempotent by design (redact.rs:297-309).
3. `fn open_pull_request(ctx: &PushContext, store: &mut RunStore, url: &str, dest: &str, base_branch: &str)`:
   `Some((_, line)) => (None, redact_urls(&line))` goes to `log_event(store, "pr-failed", vec![("detail", Some(detail))])`.

**Pattern to apply.** Add one private helper so the two capture sites cannot drift:
```rust
/// A captured child stderr line, made safe to persist: URL userinfo first
/// ([`redact_urls`]), then the SAFE-04 token/path table ([`crate::journal::redact::redact`]).
fn redact_captured(line: &str) -> String {
    crate::journal::redact::redact(&redact_urls(line))
}
```
Callee signature: `pub fn redact(s: &str) -> String` (journal/redact.rs:310). It also rewrites home paths to
`/home/[REDACTED:user]`. No test today asserts `push-failed` or `pr-failed` detail text (grep across `tests/` and
`src/` returned none), so nothing existing breaks. **Add a test** beside `credentials_in_a_url_never_survive_redaction`
in the same `assert_eq!`/`assert!(!...contains(...))` style. Use a bare token taken from redact.rs's own fixtures
(redact.rs:532, `"token ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345 end"`) and assert that `ghp_` does not survive
`redact_captured`.

**Not persisted by push.rs itself:** `eprintln!("driver launch: push: {outcome}")` in `src/launcher/run.rs` and
`out_line`/`err_line` in `src/driver_cli.rs` (`push_finished_run` match). These go to stdio, not LOG.md. Record that
finding in the summary. Do not add redaction there unless the launcher's stderr is shown to be captured to disk.

Schedule after the 28.1 review loop commits (research Precondition).

---

### `src/next_cli.rs` (CLI, request-response; C8)

**Analog:** the refusals already in this file, which use `render_for_terminal` through a `shown()` closure
(next_cli.rs:68-95), and the `err_line(&format!("next: {..}")); return 1;` exit pattern (:154-155, :161-162).

**Current** (next_cli.rs:149-157):
```rust
if !gsd_meta_manager::journal::is_plain_path_component(target) {
    let refusal = DriveError::TargetPhaseInvalid {
        target_phase: Untrusted::from_untrusted_source(target.to_string()),
    };
    err_line(&format!("next: {refusal}"));
    return 1;
}
```
**Replacement:** a local message with the same text as error.rs:1000-1006: `"the target phase {:?} is not a single
plain path component, so it is refused before anything is created. A target phase is a phase number such as `20`;
it may not contain a path separator, `..`, or a leading `/`"`. Interpolate `render_for_terminal(target)`. Error.rs used
`escaped(..)` under `{:?}`. Check which form `Rendered` supports, and keep the escaping so T-29-10/T-29-13 still hold.
Then drop `use gsd_meta_manager::error::DriveError;` (:32) and, if it is now unused, `Untrusted` from the :35 import.
That avoids a clippy unused-import failure in this binary target. The pinning test is
`tests/next_cli.rs:249 a_traversing_target_phase_is_refused` (exit 1, via `refused(..)`), and it must stay green
unchanged.

---

### `src/app.rs` + `src/action.rs` 20-tick marker scan (event-driven; C7)

**Analog:** `Action::AgentsScanned { per_project }` (action.rs:199-218) is a whole-scan replacement with no
reconcile payload. The handler side is `AppContext::apply_launcher_markers(&mut self, markers: &HashMap<String,
driver_run::LauncherMarker>) -> bool` (ui/screens/mod.rs:1966-1983).

**Current task body** (app.rs:2375-2405):
```rust
if let Some(ref tx) = self.ctx.event_tx {
    let tx: UnboundedSender<Action> = tx.clone();
    let projects = self.ctx.config.projects.clone();
    let experimental = self.ctx.experimental;
    tokio::task::spawn_blocking(move || {
        let runs = crate::driver::reconcile::reconcile_all(&projects);              // delete
        let last_outcomes = crate::driver::reconcile::last_ended_outcomes(&projects); // delete
        let launcher_markers = if experimental { launcher_markers_for(&projects, std::time::SystemTime::now()) } else { HashMap::new() };
        let _ = tx.send(Action::RunsReconciled { runs, last_outcomes, launcher_markers });
    });
}
```
**Target:** keep the `spawn_blocking`, the counter and the `experimental` short-circuit. Send only the markers, e.g.
`Action::LauncherMarkersScanned { launcher_markers }`, or keep the name `RunsReconciled` with one field. Keep the long
comment at :2360-2374 that forbids a second timer. The handler (app.rs:2943-2955) keeps only its first block:
```rust
if self.ctx.apply_launcher_markers(&launcher_markers) {
    self.ctx.recompute_filtered_aliases();
    self.needs_redraw = true;
}
```
Delete `observed`/`last_outcomes` (:2956-2967 onward). Move the variant doc for `launcher_markers` (action.rs:186-197)
to the new variant unchanged. Retarget the tests at app.rs:4958, :8956-8960 and :9008-9020 (they build
`RunsReconciled { runs, last_outcomes, launcher_markers }` and expect "the slow tick sends RunsReconciled"). Rename them,
but keep their assertions about the marker. Also delete the startup `reconcile_all`/`last_ended_outcomes` in
`src/main.rs:756-765`.

---

### `src/journal/mod.rs` `classify_change` (utility, transform)

**Analog:** the `drivers` arm and its exhaustive-shape test, in the same file.

**Current runs arm** (journal/mod.rs:1171-1179):
```rust
if parts.len() >= 5 && parts[0] == ".planning" && parts[1] == "meta-manager" && parts[2] == "runs" {
    return ChangeKind::DriverJournal { run_id: parts[3].to_string() };
}
```
**Target:** `return ChangeKind::Ignored;`, then delete the `DriverJournal` variant (:1093-1098). Decide whether
`>= 3` (research example: `runs/.gitignore`, `active` and the run dir itself also go `Ignored`) or `>= 5` (only deep
files; direct children stay `Planning`, as the doc at :1140-1142 says today). Whichever you choose, update the doc
paragraph at :1133-1142 and the `Ignored` variant doc (:1109-1116, which currently names only `drivers/`).
**Test pattern to copy** (journal/mod.rs:2355-2379): a `cases` array of path strings, each
`assert_eq!(classify_change(root, &root.join(&case)), ChangeKind::Ignored, "{case} must be ignored")`. Replace the
`DriverJournal` test at ~:2282 with this shape. Keep `classify_change_keeps_a_project_owned_drivers_dir_as_planning`
(:2382) and add a `.planning/runs/x` twin to it, so the three-component anchor (T-16-07) stays pinned.

---

### `src/driver/rate_limit.rs` (receives one test; drops the `bounds` reason ids)

**Analog:** its own `mod tests` (rate_limit.rs:386-400), which already `include_str!`s transcripts by relative path:
```rust
const T01: &str = include_str!("../../tests/fixtures/transcripts/01-success-textonly.ndjson");
```
**Move** `the_synthesised_fixtures_quota_event_classifies_as_a_rejection` (tests/driver_rate_limit.rs:790-830).
Add `const T09: &str = include_str!("../../tests/fixtures/transcripts/09-rate-limit-rejected.ndjson");` and the
one-line `is_rate_limit_line` filter (driver_rate_limit.rs:97-99). Inside the module the names lose their prefix:
`rate_limit::classify` becomes `classify`, `rate_limit::RATE_LIMIT_INFO_FIELD` becomes `RATE_LIMIT_INFO_FIELD`, and so
on. It already uses `serde_json::Value` via the `json` import, so add `Value`.
**Edit** `the_quota_reason_is_greppable_and_collides_with_no_sibling_taxonomy` (:853-880). Remove the four
`crate::driver::bounds::REASON_*` siblings (:863-866) and keep the three `router::REASON_*`. Update the message
"four taxonomies ride the one `JournalEvent::Parked.reason` field", because `JournalEvent` goes with the journal trim.

---

### `src/ui/screens/driver_confirm.rs` (component; C1 + C12)

**Analog:** the surviving `StopLauncher` path in this file. `new_launcher_stop` (:257-262) builds on `Self::new`.
After the trim, `new` (:245-254) must stop setting `command: DEFAULT_DRIVE_COMMAND` and `goal`. Drop both fields
(:222-232) once `Start` is gone. `DriverAction` (:195-205) keeps `ToggleOptIn` and `StopLauncher`. Update the doc
"Which of the three driver actions" (:193) and the `ConfirmPopup` comment (:207-218).
**Reword (C12)**, `prompt_text` arms (:340-343, :354-357). Keep the `render_for_terminal(alias)` composition at :327
exactly as it is:
```rust
DriverAction::ToggleOptIn if opted_in => format!("Withdraw the driver opt-in for \"{alias}\"? No NEW run can start afterwards; ...")
DriverAction::ToggleOptIn => format!("Allow \"{alias}\" to be driven? This is the record the spawn gate checks ...")
```
Both claims become false (`policy_for` in push.rs falls back to defaults without an opt-in). State what the record
does now: it carries the PR caps and the branch namespace that the push step reads. Follow the T-17-48 rule in the
:288-291 doc: make no safety claim the code does not back. Update the
`MEASURED_REACH` entry `("src/ui/screens/driver_confirm.rs", 4)` (ui/mod.rs:359) and `CENSUS_FILES` (driver.rs:2015)
in the same commit.

---

### `src/driver/mod.rs` (module root)

**Analog:** its own decl block (driver/mod.rs:60-120). Every `pub mod` there carries a `//` paragraph saying why it is
inside or outside `#[cfg(unix)]`. Keep that convention for the survivors: `direction`, `steer`, `router`,
`rate_limit` (portable), `liveness` (portable), and `#[cfg(unix)] kill` / `#[cfg(unix)] lock`. Delete
`dry_run` (:66), `bounds` (:81), `goal`/`untrusted` (:96-97), `escalate` (:103), `reconcile` (:115), `run` (:117),
`spawn` (:119) and their comment paragraphs. Several surviving comments say "Third/Fourth/Sixth in the same block" and
"both modules". Re-number or re-word them so they do not reference deleted siblings. Pitfall 5: removing one side of a
`cfg(unix)`/`cfg(not(unix))` pair breaks only the cross-target clippy, which pre-tag-check does not run.

**`kill.rs` keeps** `pub const DRIVER_TEARDOWN_GRACE` (:57) and `pub(crate) fn signal_group(pgid: u32, sig: Signal)`
(:183). It **drops** `pub enum ReapArm` (:79), `pub enum StopOutcome` (:117) and `pub async fn stop_run` (:315).
**`lock.rs` keeps** `RUN_LOCK_FILE`, `ACQUIRE_RETRY_BUDGET`, `run_lock_path`, `LockHolder`, `RunLock`, `acquire_at`,
`read_holder_at`, `acquire_at_with_retry`, `LockProbe`, `probe_at`. It **drops** `lock_path` (:103), `acquire` (:183)
and `read_holder` (:296), plus `use crate::journal::{runs_root, writer};` (:63).

---

### `src/cli.rs` / `src/main.rs` (CLI; C3)

**Analog:** the existing `EnvelopeAction::Scan` arm, which resolves with `Alias::new`, not `judged_alias_or_exit`.
Delete the `PrePush`, `PreCommit`, `Askpass` and `Guard` variants and arms, and `fn judged_alias_or_exit`
(main.rs:34). That function is private in a bin target, so an orphan fails `-D warnings` (Pitfall 4). `main.rs:562,588`
are the only `journal::redact` uses in main and live in the deleted arms. Recheck the import afterwards.
`scripts/launcher-acceptance.sh:940` and `scripts/prompt-acceptance.sh:1134` call `envelope scan`; they stay unchanged.

---

### Census / bidirectional pins (test, batch)

Edit each one in the **same commit** as the file it names (Pitfall 1):
- `tests/spawn_seam_guard.rs` `SPAWN_ALLOWLIST` (:48-120). Every entry has a `//` line saying why it may spawn
  (e.g. :49-60 for `executor/claude.rs`, `codex.rs`, `outcome.rs`, `driver/kill.rs`). The `vanished` check fails on a
  stale entry, so remove the entry together with its comment.
- `tests/async_blocking_guard.rs:276` currently reads
  `const REQUIRED_ASYNC_FILES: &[&str] = &["src/driver/mod.rs", "src/driver/run.rs"];`. Repoint it at surviving async
  files (`src/app.rs`, `src/main.rs`, `src/main_loop.rs`). Update the message at :687 ("one of the two").
- `src/ui/mod.rs:357-361` `MEASURED_REACH: [(&str, usize); 3]`. This is a Vec equality that fails in both
  directions. Re-measure it, and change the array length if an entry drops out.

---

### Release tooling (D-12; one atomic commit with the constant deletion)

**`scripts/pre-tag-check.sh`.** Delete as one unit:
- vars `POLICY_FILE`/`CONST_NAME`/`WITNESS_TEST` (:572-574);
- state vars (:598-601);
- the summary ADVISORY block (:630-646);
- the extraction and the HARD ERROR on a missing constant (:662-705), which **exits 1 if the constant goes first**;
- the banner (:706-738);
- the DEC-5 witness grep (:886-891);
- the header sentence (:23 "and the git-version witness");
- the `--container` hint "read the reconciliation banner it prints" (:300).

Keep every container-mode line.
**`scripts/pre-tag-check.Dockerfile`:** keep the `add-apt-repository -y ppa:git-core/ppa` RUN (:29-38) and rewrite
only its comment (:19-27). Also adjust the "two places" rationale at :4-9 so the oracle remains the named reason and git
becomes runner fidelity.
**`.github/workflows/release.yml:11-16`:** drop the parenthetical about the witness and keep "environment-sensitive" for
the oracle. **`scripts/install-conformance-oracle.sh:231`:** "Mirrors the git-version advisory at
scripts/pre-tag-check.sh lines 207-235" needs a new reference. **`Cargo.toml:27-29`:** the exclude comment names
`tests/fixtures/injection-corpus/`, which is deleted in the drive/goal plan. **`CLAUDE.md` Release Process step 3:**
drop "and the git-version witness" from the bare-vs-container paragraph and the whole "The git reconciliation banner"
paragraph.

---

### `tests/no_container_crate.rs` (NEW, optional; test, file-I/O)

**Analog:** `tests/launcher_acceptance.rs:25-53`. It uses `PathBuf::from(env!("CARGO_MANIFEST_DIR"))` as `repo()` and
returns a `skip_reason()` instead of failing when a file is absent, as it is in a packaged checkout. Read the lock at
runtime (`std::fs::read_to_string(repo().join("Cargo.lock"))`), never through `include_str!`, so gate 6
(`cargo package --locked`) cannot be stranded. Assert `!lock.contains("name = \"bollard\"")` with a message that names
Phase 22's supersession.

---

### Phase 19 closure (`deferred-items.md` + `19-SECURITY.md`)

**Section analog:** the per-id bullet lists in `19-.../deferred-items.md:3186-3228` ("Still OPEN and unchanged by this
plan"). Each is one `- **\`T-19-NN\`** — <status>. <one-line reason>.` bullet, append-only, and never rewrites earlier
text. The ROADMAP asks for one row per id, so write the Phase 31 closure as a dated `## Phase 31 closure (2026-MM-DD)`
section holding a **table**. Copy the column shape from `28.1-SECURITY.md:44`:
```markdown
| Threat ID | Severity | Class | Superseded by | Pointer | Status |
|---|---|---|---|---|---|
| T-19-86 | high | G (+N) | capability removal | LNCH-04, LNCH-06, LNCH-09 (28.1 D-02..D-06); reopens against Phase 28.1 if the D-11 live probe fails | closed (superseded) |
```
**Frontmatter analog:** `19-SECURITY.md:1-25`. Change `status: blocked` to `superseded`, set `threats_open:` to the
post-closure count, and bump `updated:`. Add a `superseded_by: phase-31` key, as the precedent
`260915-f4n-SUMMARY.md:76` (`superseded_by: quick-260915-hae`) does. Append to `register_totals`; never rewrite it.
Keep the comment lines (:22-24). Put the closure table beside the audit trail, never in place of it. Before writing
the 27th row, enumerate the open set from the register (research A1: T-19-105 or T-19-17r).

### Phase 21 criterion 4 (`21-.../deferred-items.md`)

**Analog:** the recurring `## RE-SURFACED, UNCHANGED — ROADMAP success criterion 4` sections (:469, :649). They quote
`21-VERIFICATION.md` verbatim rather than paraphrasing it. Append a `# Phase 31 closure (date)` top-level heading,
matching the `# Round-N ...` top-level style at :180, :206, :240. Under it, put one `## CLOSED (superseded) — ROADMAP
success criterion 4` section whose pointer is LNCH-04, the 28.1 egress/`$HOME` controls, and 28.1-VERIFICATION SC6
(cite the VERIFICATION, not the REQUIREMENTS checkbox, per research). Add one `## NOT superseded — CR-01` section that
points at the new todo.

### NEW todo for CR-01 (and CR-02 if it is still live)

**Analog:** `.planning/todos/pending/2026-10-18-dashboard-verification-status-quote-divergence.md`. It has
frontmatter `created`, `title`, `area`, `severity`, `files:` (with `path:lines`), then `## Problem` and
`## Solution direction`. For CR-01: `area: session_detector`, `files: [src/session_detector.rs:728-734, :425, :469]`.
Quote the arity bug evidence from research §CR-01.

### Phase 22 archive (`22-0{1..4}-PLAN.md` + `22-SUPERSEDED.md`)

**Frontmatter analog:** `22-01-PLAN.md:1-21` (keys `phase`, `plan`, `type`, `wave`, ...). Add `status: superseded` and
`superseded_by: phase-28` after `plan:`, following the `superseded_by:` key convention of
`260915-f4n-SUMMARY.md:76`. Change nothing else.
**`22-SUPERSEDED.md` body analog:** the blockquote banner of `260915-f4n-SUMMARY.md:79-82` ("> **No longer
blocking.** ... See `<path>`. The measurements below are unchanged") plus ROADMAP.md:707-709. Name bwrap +
`systemd-run --user --scope` (Phase 28), "0/4 plans executed, no code landed, `bollard` never added", and the fact that
the directory stays in place for ROADMAP links and `gsd-tools` lookup.

### `.planning/REQUIREMENTS.md`

**Analog:** the traceability rows at :336-343. Each is `| ID | Phase NN (superseded) | <disposition>; <pointer> |`.
SAFE-02 already reads "...; guard retired in Phase 31" (:337). Bring SAFE-05/07/08 (:340, :342, :343) into the same
"guard retired in Phase 31" form. Flip RETIRE-01..03 (:409-411, and the checkboxes at :283-291) only at verification.
Do not touch the LNCH-13/14 or 28.1 rows (D-17).

## Shared Patterns

### SAFE-04 capture-time redaction
**Source:** `src/journal/redact.rs:310` `pub fn redact(s: &str) -> String` (idempotent, generic pass then host pass).
**Apply to:** every child-output string the push step persists through `RunStore::log_launcher_event`, via one
`redact_captured` helper in `push.rs`. `envelope/scan.rs:48` keeps consuming `credential_alternation()`, unchanged.

### Untrusted text in refusals and prompts
**Source:** `crate::text::render_for_terminal` (text.rs:487), as used at `next_cli.rs:69` and
`driver_confirm.rs:327, :347`.
**Apply to:** the new `next_cli` refusal, the reworded opt-in prompts, and any surviving message that interpolates an
alias, run id or phase.

### Census edits travel with the deletion
**Source:** `tests/spawn_seam_guard.rs:48`, `tests/async_blocking_guard.rs:276`, `src/ui/mod.rs:357`,
`src/ui/screens/driver.rs:2015`, `render_escape_guard.rs` path list, and the self-`include_str!` tests
(`envelope/policy.rs:9071,11022-11024,11147,11542,12568`, `envelope/mod.rs:449`, `journal/mod.rs:3283`).
**Apply to:** every plan that deletes a file. Before each delete, run
`rtk proxy grep -rn '"src/<file>' src tests`.

### Module docs must not reference deleted siblings
**Source:** the doc-link sweep in research §D-07 (`rtk proxy grep -rn "driver::\(goal\|escalate\|untrusted\|run\|spawn\|dry_run\|bounds\|reconcile\)" src`).
**Apply to:** `driver/mod.rs`, `envelope/{mod,policy,ledger}.rs`, `journal/mod.rs`, `text.rs:21,49`, `router.rs:388`,
`app.rs:491`, `registry.rs:784`, `state_md.rs`, `cli.rs:217,293`.

### Test-count verification
**Source:** user CLAUDE.md and memory notes. Run `rtk proxy cargo test --no-fail-fast > "$LOG" 2>&1` and count
`test result:` lines with a script run under `rtk proxy`, never through a downstream `| grep`. Baseline: 4411/2/30
over 94 result lines.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| ledger.rs FIFO-bound unit test with a thread timeout | test | file-I/O | Every existing FIFO row ran through a subprocess with a hard timeout (`ask_binary_under_timeout`), and that harness is deleted with the corpora. Use `std::thread::spawn` + `mpsc::recv_timeout` (std-only); there is no in-repo precedent to copy |

## Metadata

**Analog search scope:** `src/envelope/`, `src/launcher/push.rs`, `src/journal/{mod,redact}.rs`, `src/app.rs`,
`src/action.rs`, `src/next_cli.rs`, `src/driver/{mod,kill,lock,rate_limit}.rs`, `src/ui/screens/{driver_confirm,mod}.rs`,
`src/ui/mod.rs`, `tests/{common/mod,envelope_pr_cap,envelope_word_set,envelope_interior_path,envelope_carrier_reach,driver_rate_limit,next_cli,spawn_seam_guard,async_blocking_guard,launcher_acceptance,agents_*,registry_worktree_guard}.rs`,
`scripts/`, `.github/workflows/release.yml`, `Cargo.toml`, `.planning/phases/{19,21,22,28.1}-*`, `.planning/quick/260915-f4n-*`,
`.planning/todos/pending/`, `.planning/{REQUIREMENTS,ROADMAP}.md`.
**Tracked-source gate:** every analog path above was verified with `git ls-files` (all tracked; no mirror paths).
**Files scanned:** ~40
**Pattern extraction date:** 2026-10-24
