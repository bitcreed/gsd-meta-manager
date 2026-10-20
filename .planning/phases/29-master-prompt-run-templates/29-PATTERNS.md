# Phase 29: Master Prompt & Run Templates - Pattern Map

**Mapped:** 2026-10-19
**Files analyzed:** 22 (new + modified)
**Analogs found:** 21 / 22

Line numbers are taken from RESEARCH.md (verified by the researcher this session) and spot-checked here
(sandbox.rs, store.rs, driver_frontmatter.rs, gate.rs, main.rs, cli.rs, driver_cli.rs). All analog paths are git-tracked source.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/launcher/assets/MASTER-PROMPT.md` (new) | config/asset | transform (compile-time) | `.planning/spikes/003-divergent-gap-loop-validation/MASTER-PROMPT.md` (v0.4 content) | exact (content port) |
| `src/launcher/prompt.rs` (new) | utility | transform | `src/launcher/sandbox.rs:370-383` (`render_prompt`), `src/driverctl/serve.rs:248-251` (`hash8`) | role-match |
| `src/launcher/mod.rs` (mod) | config | — | itself (module list) | exact |
| `src/launcher/run.rs` (mod: optional prompt, preflight) | service | request-response | itself, `:497`, `:1078-1079`, `:1155-1171` | exact |
| `src/launcher/gate.rs` (mod: control deny) | service (pure) | request-response | itself `:56-88` `decide_dispatch` | exact |
| `src/driverctl/store.rs` (mod: prompt_version, D-17, gate input) | service/store | CRUD + event-driven | itself `:837-849` (sandbox key), `:894-901` (RUN launch kv), `:925-950` (`handle_log`), `:219-236` (`RUN_STATE_FIELDS`) | exact |
| `src/state_reader/driver_frontmatter.rs` (mod: `prompt_version`) | model | transform | itself `:379-386`, `:597-605` (`sandbox`) | exact |
| `src/driver_templates/mod.rs` (new) | service | file-I/O | `src/driver_cli.rs` + `src/driverctl/serve.rs` (`valid_run_id`, `validate_run_dir`, `DRIVERS_REL`) | role-match |
| `src/driver_templates/lint.rs` (new) | utility (pure) | transform | `src/driver/direction.rs` (`parse_direction` regexes `:146`, `:151`, `:156`, `:183-189`; `body_sane` `:122-124`) | role-match |
| `src/driver_templates/assets/DIRECTION.md` (new) | config/asset | — | `.planning/spikes/002-launcher-caps-and-tool-frontmatter/templates/DIRECTION.md` | exact (port) |
| `src/driver_templates/assets/LOG.md` (new) | config/asset | — | `.planning/spikes/002-launcher-caps-and-tool-frontmatter/templates/LOG.md` | exact (port) |
| `src/cli.rs` (mod: `DriverAction::New/Check`, `Commands::Next`, hidden `--prompt-file`) | config (clap) | — | itself `:439-455` (`DriverAction`), `:494-502` (`LaunchArgs`) | exact |
| `src/driver_cli.rs` (mod: `new_main`, `check_main`) | controller | request-response | itself `:328-345` (`stop_main`) | exact |
| `src/next_cli.rs` (new) | controller | request-response (read-only) | `src/driver_cli.rs` (`run`/`stop_main`) + `src/driverctl_cli.rs` dispatch | role-match |
| `src/main.rs` (mod: dispatch `Next` before tracing) | config/entry | — | itself `:104-112` (`Driverctl` early dispatch) | exact |
| `src/launcher/config_dir.rs` (mod: `bin/gsd-meta-manager` shim) | utility | file-I/O | itself `:226-254` (`seed()` driverctl shim) | exact |
| `tests/driver_prompt.rs` (new) | test | — | `tests/driverctl_client.rs:611-724` (prompt block lifter) | exact |
| `tests/driver_new.rs` (new) | test (binary e2e) | file-I/O | `tests/launcher_e2e.rs` + `tests/launcher_support/mod.rs` `Fixture` | role-match |
| `tests/launcher_reconcile.rs` (new) | test (fake-driver e2e) | event-driven | `tests/launcher_e2e.rs`, `tests/driverctl_store.rs:603-643` | exact |
| `tests/next_cli.rs` (new) | test | request-response | `tests/driver_router_table.rs` + `tests/launcher_e2e.rs` binary-invocation style | role-match |
| `tests/prompt_acceptance.rs` + `tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md` (new) | test | batch | `tests/launcher_acceptance.rs` | exact |
| `scripts/prompt-acceptance.sh` (new) + `scripts/launcher-acceptance.sh` (mod: source-able guard) | script | batch | `scripts/launcher-acceptance.sh` (`MAX_CEILING=20` `:42`, `prereg_gate` `:151`) | exact |
| `.planning/phases/29-…/29-P29-RUBRIC.md` (new) | doc | — | `.planning/phases/28-sandboxed-driver-launcher/28-D33-RUBRIC.md` | exact |

## Pattern Assignments

### `src/launcher/prompt.rs` + `assets/MASTER-PROMPT.md` (utility, transform)

**Analog:** `src/launcher/sandbox.rs` lines 370-383 — the render seam stays; only the template source changes.
```rust
pub fn render_prompt(template: &str, run_dir: &Path, project: &Path, mode: &str) -> String {
    let run_dir = run_dir.display().to_string();
    let mut prompt = template
        .replace("{RUN_DIR}", &run_dir)
        .replace("{PROJECT_ROOT}", &project.display().to_string())
        .replace("{MODE}", mode);
    prompt.push_str(&format!(
        "\n\nBegin now: run Startup, then the Loop, for RUN_DIR={run_dir}."
    ));
    prompt
}
```
New module shape (RESEARCH Pattern 1):
```rust
pub const DRIVER_PROMPT_VERSION: &str = "driver-prompt/1.0.0";
pub const MASTER_PROMPT: &str = include_str!("assets/MASTER-PROMPT.md");
// None => (MASTER_PROMPT, DRIVER_PROMPT_VERSION); Some(p) => (read(p), format!("custom:{}", hash8(bytes)))
```
- `include_str!` precedent: `src/state_reader/mod.rs:1071` (path relative to the source file).
- sha8: reuse `serve::hash8` (`src/driverctl/serve.rs:248-251`, `Sha256::digest`) rather than re-implementing.
- Run `check_prompt` / `MAX_PROMPT_BYTES` (`sandbox.rs:339`, 120 KiB) on the embedded asset too.
- Asset header: `<!-- driver-prompt/1.0.0 … -->`; stop-rules block between `<!-- stop-rules:begin -->` / `<!-- stop-rules:end -->`.
- Keep the v0.4 `## Recording events: \`driverctl\` (the only way)` heading and placeholder spellings byte-identical (the lifter in `tests/driverctl_client.rs` depends on them).

### `src/launcher/run.rs` (service, request-response)

**Analog:** itself.
- `launch_argv` (`:1078-1079`) today pushes unconditionally:
  ```rust
  argv.push("--prompt-file".into());
  argv.push(opts.prompt_file.as_os_str().to_owned());
  ```
  → wrap in `if let Some(p) = &opts.prompt_file`. Update `launch_argv_round_trips_through_the_real_parser` and `launch_argv_carries_every_flag_and_the_daemon_markers` (`:1584-1642`) for both shapes.
- `launch_detached` (`:1155-1171`) canonicalizes/reads the file in the parent → skip for `None`.
- Add `lint_direction` preflight in **both** `launch()` and `launch_detached()` before any write; map to new `LauncherError::DirectionInvalid { problems }` (exit 1 via `driver_cli::report`).
- Pass `prompt_version` into `LaunchSpec`.

### `src/launcher/gate.rs` (pure service)

**Analog:** itself, lines 56-88 — ordered early returns: tripped → (NEW: control) → gap rule → dispatch cap.
```rust
pub fn decide_dispatch(payload: &HookPayload, state: &RunState, limits: &CapLimits, tripped: Option<&str>) -> GateDecision {
    if let Some(cap) = tripped {
        return GateDecision::Deny(format!("run stopped by launcher cap {cap}"));
    }
    // NEW (D-18): if let Some(c @ ("pause" | "stop")) = control { return GateDecision::Deny(format!("control={c}; ...")); }
    ...
    GateDecision::Allow
}
```
Add `control: Option<&str>` parameter (recommended option (a): store passes `self.direction_info().control`; gate closure type at `store.rs:291`). Hook client already prefixes `LAUNCHER CAP: ` (`src/driverctl_cli.rs:457-461`). Unit tests sit in the same file's `#[cfg(test)]` block.

### `src/driverctl/store.rs` (store, CRUD + event-driven)

**Per-launch additive key — copy `sandbox` exactly** (lines 837-849):
```rust
// D-30: the posture belongs to THIS launch. A key left by an earlier launch
// (a degraded one, then a standalone `driverctl serve`) must not describe it.
match &spec.sandbox {
    Some(sandbox) => { next.extra.insert(EXTRA_SANDBOX.to_owned(), Value::String(header_safe(sandbox))); }
    None => { next.extra.remove(EXTRA_SANDBOX); }
}
```
**RUN launch kv** (lines 894-901) — add `("prompt", spec.prompt_version.as_deref().map(header_safe))`:
```rust
entries.push(LogEntry::run("launch", vec![
    ("n", Some(next.launches.to_string())),
    ("mode", Some(launcher_mode)),
    ("model", model),
]));
```
**Rejection point for D-17** — `handle_log` (lines 925-950): insert checks after `validate_log`, before `apply_event`, returning `Response::rejected(message)` (same form as line 930). New `RunState` fields `awaiting_recovery`, `recovered_this_launch` with `#[serde(default)]`; set/clear in `begin_launch`; append both names to `RUN_STATE_FIELDS` (`:219-236`, array length 16 → 18). Do NOT reject a first `recovered` on a non-running item (`tests/driverctl_store.rs:603-643`). `LaunchSpec` (`:264-277`, `#[derive(Default)]`) gains `prompt_version: Option<String>`; `frontmatter()` (`:715-731`) reads it from `extra` like `sandbox`. Factor `frontmatter()` into pure `frontmatter_for(state, direction)` for `driver new`.

### `src/state_reader/driver_frontmatter.rs` (model)

**Analog:** itself, lines 379-386:
```rust
/// ... Additive: absent from every frontmatter written before the launcher
/// existed, and emitted only when set so those stay byte-identical.
#[serde(default)]
pub sandbox: Option<String>,
```
Add `#[serde(default)] pub prompt_version: Option<String>` after `sandbox_missing`; emit only when `Some` in `emit()` next to the sandbox emission (`:597-605`). No schema version bump (header rule `:38-44`). No `deny_unknown_fields` (spawn_seam_guard).

### `src/driver_templates/mod.rs` (service, file-I/O) + assets

**Analogs:** `driverctl::serve::{valid_run_id, validate_run_dir, DRIVERS_REL}` (`RUN_ID_RE` at `serve.rs:98`), store `open_with_clock` NotFound fallback (`store.rs:460-477`, so do not write `state.json`).
Core: `run_id = format!("{}-{slug}", Utc::now().format("%Y%m%d"))`; symlink/containment refusal; `fs::create_dir(run_dir)` (EEXIST = collision); files via `OpenOptions::new().write(true).create_new(true)`; DIRECTION.md = `DriverFrontmatter::emit()` of `frontmatter_for(&RunState::new(..), &direction)` + template body. Body must satisfy `body_sane` (`direction.rs:122-124`: contains `## Direction` and `## Items`).

### `src/driver_templates/lint.rs` (pure utility)

**Analog:** `src/driver/direction.rs` regexes — reuse the same shapes but be strict (do NOT tighten `parse_direction`):
- control `(?m)^- control:\s*(\w+)` (`:146`), item `(?m)^### (I-\d+)\b` (`:151`), budget `(?m)^- budget:\s*(.+)$` + `v.trim().parse::<i64>()` (`:156`, `:183-189`).
Output `Vec<Problem{line, message}>`, rendered as `DIRECTION.md:<line>: <message>`. Strip HTML comments before the placeholder scan.

### `src/cli.rs` (clap)

**Analog:** `DriverAction` (`:439-455`) — doc-comment-per-variant, positional `project: String` ("Alias or path of a registered project"), `run_id: String`. `LaunchArgs.prompt_file` (`:498-500`):
```rust
/// The master prompt template; `{RUN_DIR}`, `{PROJECT_ROOT}` and `{MODE}` are filled in
#[arg(long)]
pub prompt_file: PathBuf,
```
→ `#[arg(long, hide = true)] pub prompt_file: Option<PathBuf>`. Add `New { project, slug }`, `Check { project, run_id }`, and top-level `Commands::Next { project, #[arg(long)] target_phase: Option<String>, #[arg(long)] json: bool }`.

### `src/driver_cli.rs` (controller)

**Analog:** `stop_main` (lines 328-345):
```rust
fn stop_main(target: &str, run_id: &str, wait_secs: u64, config: Option<PathBuf>) -> i32 {
    let (_alias, project, _registry) = match resolve_project(target, config) {
        Ok(resolved) => resolved,
        Err(message) => { err_line(&format!("driver stop: {message}")); return 1; }
    };
    match stop_run(&project, run_id, ...) {
        Ok(report) => { out_line(&format!("driver stop: run {run_id}: {report}")); ... }
        Err(error) => report("stop", &error),
    }
}
```
`new_main`/`check_main` follow it: `err_line`/`out_line` helpers (`:23-34`), exit code returned as `i32`, dispatched from `run()` (`:38`). Note the `#[cfg(target_os = "linux")]` / non-linux stub pairs (`:351`/`:393`, `:328`/`:398`) — `new`/`check` are not launcher-bound and need not be linux-only.

### `src/next_cli.rs` + `src/main.rs` (controller, read-only)

**Analog for dispatch:** `src/main.rs:104-112`:
```rust
let mut cli = Cli::parse_checked();
match cli.command.take() {
    Some(Commands::Driverctl { action }) => std::process::exit(driverctl_cli::run(action)),
    other => cli.command = other,
}
// Initialize tracing subscriber with file appender before anything else
init_tracing();
```
Add `Some(Commands::Next { .. }) => std::process::exit(next_cli::run(..))` in that first match (before `init_tracing`, D-21). Body: path-with-`.planning/` else `load_config` + `resolve_launch_target`; `parse_project_state(&root.join(".planning"))` (`state_reader/mod.rs:754-757`) → `router::decide(&state, target)` (`router.rs:901`) → `serde_json::json!({"advisory": true, ...})`. Ungated (driver group is ungated, `main.rs:117-119`).

### `src/launcher/config_dir.rs` (shim)

**Analog:** `seed()` lines 226-254 writes `bin/driverctl` as `exec <binary> driverctl "$@"`; add `bin/gsd-meta-manager` with a `case "$1" in next) …;; *) exit 2;; esac` guard.

### Tests

- `tests/driver_prompt.rs` → copy the lifter from `tests/driverctl_client.rs:611-724`; assert enums against `protocol::{RULE_ENUM (:46-55), END_REASONS (:59-66), RECOVER_ACTIONS (:75), MAX_REPORT_LINES (:43)}`; `FINDINGS:` at line start (`protocol.rs:441-452`).
- `tests/driver_new.rs`, `tests/launcher_reconcile.rs` → header/imports of `tests/launcher_e2e.rs:1-30`:
  ```rust
  #![cfg(target_os = "linux")]
  #[path = "launcher_support/mod.rs"]
  mod launcher_support;
  use launcher_support::{wait_until, Fixture, RUN_ID, TOKEN, WAITING_SCENARIO, WELL_MANNERED_SCENARIO};
  ```
  `Fixture` (`launcher_support/mod.rs:159-264`: `project()`, `registry()`, `prompt_file()`, `scenario_path()`) — add new fake-claude scenarios (crash mid-dispatch, pause mid-dispatch) there. `driver_new.rs` needs no fake bwrap; it can drop the linux cfg.
- `tests/prompt_acceptance.rs` → mirror `tests/launcher_acceptance.rs` (CI-safe harness proofs, no spend).
- LOG template test: `driverctl::log::parse_log` yields zero events.

## Shared Patterns

### Additive frontmatter/state keys
**Source:** `driver_frontmatter.rs:379-386`, `store.rs:837-849`. **Apply to:** `prompt_version`, `awaiting_recovery`, `recovered_this_launch`. `#[serde(default)]`, emit only when set, cleared per launch, no schema bump, never `deny_unknown_fields`.

### Typed refusals / exit codes
**Source:** `driver_cli.rs` (`err_line`, `report(verb, &error)`, `1` refusal / `3` already launched); store `Response::rejected` (rc 2). **Apply to:** `driver new` collision, `driver check`, launch preflight, D-17 rejections.

### Pure decision + thin I/O shell
**Source:** `gate.rs::decide_dispatch`, `router::decide`. **Apply to:** lint, gate control check, `next` — keep logic pure and unit-tested; CLI only resolves/reads/prints.

### Paid-run discipline
**Source:** `scripts/launcher-acceptance.sh` (`MAX_CEILING` `:42`, `prereg_gate` `:151`), `28-D33-RUBRIC.md`. **Apply to:** `scripts/prompt-acceptance.sh` (ceiling 25, own ledger), rubric committed by sha before spend, cost-consent checkpoint.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md` (derivation check) | test fixture | transform | Only analog is spike-003 `make_variant.py --check` (Python, out of tree); implement the transform in Rust inside `tests/driver_prompt.rs` per RESEARCH Pattern 3 item 9 |

## Metadata

**Analog search scope:** `src/launcher/`, `src/driverctl/`, `src/driver/`, `src/state_reader/`, `src/cli.rs`, `src/driver_cli.rs`, `src/main.rs`, `tests/`, `scripts/`
**Files scanned:** ~20
**Pattern extraction date:** 2026-10-19
