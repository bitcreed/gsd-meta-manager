# Phase 34: Owner items reader + inbox CLI - Pattern Map

**Mapped:** 2026-11-14
**Files analyzed:** 31 (14 new, 17 modified)
**Analogs found:** 29 / 31 (the line-indexed markdown sectionizer and the YAML-failsafe scalar layer have no in-repo analog; port them from gsd-core per RESEARCH.md)

All analog paths below are git-tracked source (verified with `git ls-files`). No project-local skills, no `AGENTS.md`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/state_reader/owner_items/mod.rs` (NEW) | service (reader facade) + types + cache | file-I/O, batch | `src/state_reader/disk_status.rs` (types, `from_raw`/`as_str`), `src/oneoff/mod.rs` (dir-module doc layout) | role-match |
| `src/state_reader/owner_items/markdown.rs` (NEW) | utility (parser) | transform | none in repo; port `markdown-sectionizer.cjs` (RESEARCH Pattern 1) | no analog |
| `src/state_reader/owner_items/frontmatter.rs` (NEW) | utility (parser) | transform | `src/state_reader/disk_status.rs` `leading_block` (834-847) + `state_md.rs` `frontmatter_block_parses`/`repair_frontmatter_block` (542, 593-601) | partial |
| `src/state_reader/owner_items/uat.rs` (NEW) | utility (parser) | transform | `src/oneoff/todos.rs` (`LazyLock<Regex>`, `frontmatter_title` quote strip 62-83) | partial |
| `src/state_reader/owner_items/verification.rs` (NEW) | utility (parser) | transform | `disk_status.rs` `read_verification_status` (791-816) | partial |
| `src/state_reader/owner_items/deferred.rs` (NEW) | utility (parser) | transform | `src/oneoff/todos.rs` `list_pending` (bounded list read) | partial |
| `src/state_reader/owner_items/scan.rs` (NEW) | service (scanner) | file-I/O | `disk_status.rs` `subdir_names`/`archived_listings`/`find_phase_dir_in` (1677-1747); `driver_frontmatter.rs` `open_run_file`/`read_run_file_capped` (814-905) | role-match |
| `src/state_reader/owner_items/rules.rs` (NEW) | service (rules) | transform | `src/status_cli.rs` `attention_json` (157-200, `scopes()` walk); `detail.rs` `verification_hint_spans` (9455-9492, command strings) | role-match |
| `src/inbox.rs` (NEW, lib) | builder (JSON + text) | transform | `src/status_cli.rs` JSON builders + `text`/`cap` helpers (56-140, 230-275) | role-match |
| `src/cli_targets.rs` (NEW, bin) | utility (target resolution) | request-response | `src/status_cli.rs` `Slot`/`canon_or_raw`/`checked_slot`/`alias_of_root`/`resolve_target`/`fail` (277-371) | exact (extraction) |
| `src/inbox_cli.rs` (NEW, bin) | controller (CLI command) | request-response | `src/status_cli.rs` `run` (373-445) + module doc (1-32); `src/next_cli.rs` header (1-60) | exact |
| `tests/owner_audit_conformance.rs` (NEW) | test (oracle conformance) | batch | `tests/driver_router_conformance.rs` (41-224, 629-814); source scan `tests/driver_router_table.rs` (88-119, 585-629) | exact |
| `tests/inbox_cli.rs` (NEW) | test (binary) | request-response | `tests/status_cli.rs` (whole) | exact |
| `tests/fixtures/owner/<name>/.planning/...` (NEW) + golden text | test fixture | file-I/O | `tests/fixtures/driverctl/golden/*.md` + `tests/driverctl_frontmatter.rs` `fixture()`/`assert_golden` (31-35, 296-313) | role-match |
| `src/state_reader/disk_status.rs` (MOD) | model | transform | itself (`UatStatus`, `VerificationStatus` 99-209; tests 2311-2369) | exact |
| `src/state_reader/state_md.rs` (MOD: `pub(crate)` on `repair_frontmatter_block`) | utility | transform | itself (`frontmatter_block_parses` is already `pub(crate)` at 593) | exact |
| `src/state_reader/driver_frontmatter.rs` (MOD, optional: generalize capped read) | utility | file-I/O | itself (814-905) | exact |
| `src/state_reader/mod.rs` (MOD: `pub mod owner_items;`) | config/module | — | itself (1-14) | exact |
| `src/driver/router.rs` (MOD: predicate rename at 634) | service | — | itself | exact |
| `src/status_cli.rs` (MOD: `owner` key, owner `uat_outstanding`, use `cli_targets`) | controller | request-response | itself | exact |
| `src/cli.rs` (MOD: `Commands::Inbox`, ValueEnums, parser tests) | config (clap) | — | `Commands::Status` (244-260), `DriverLoopMode` ValueEnum (325-332), tests 1108-1135 | exact |
| `src/main.rs` (MOD: `mod cli_targets; mod inbox_cli;`, dispatch, unreachable arm) | entrypoint | — | itself (1-8, 106-110, 402-407) | exact |
| `src/lib.rs` (MOD: `pub mod inbox;`) | module | — | itself | exact |
| `src/config.rs` (MOD: `owner.stale_days`) | config | — | `Preferences::driver_open_pr` (520-536) | exact |
| `tests/status_cli.rs` (MOD) | test | — | itself (167-227, 659-692, 776-818) | exact |
| `tests/async_blocking_guard.rs` (MOD) | test (lint) | — | itself `BLOCKING_HELPERS` (127) | exact |
| `tests/state_reader_test.rs` (MOD: rename call at 822) | test | — | itself | exact |
| `README.md` (MOD: status `owner` rows, `inbox` section) | docs | — | README "Scripting and agents" (398-532) | exact |
| `src/agent_skill.md` (MOD) | docs | — | itself (Commands block 12-22, jq recipes 70-90) | exact |
| `.planning/todos/pending/2026-10-18-dashboard-verification-status-quote-divergence.md` → `done/` | planning artifact | — | — | n/a |

---

## Pattern Assignments

### `src/state_reader/disk_status.rs` (model, MOD) — D-01..D-04

**Analog:** itself, lines 99-209.

**Tolerant enum pattern to extend** (lines 165-208):
```rust
impl UatStatus {
    /// Classify a raw frontmatter value. Case-insensitive on the known arms.
    pub fn from_raw(raw: &str) -> Self {
        let trimmed = raw.trim();
        match trimmed.to_ascii_lowercase().as_str() {
            "partial" => UatStatus::Partial,
            ...
            "failed" => UatStatus::Failed,
            _ => UatStatus::Other(trimmed.to_string()),
        }
    }
    /// The stable identifier a later reader greps for. Exhaustive, no wildcard.
    pub fn as_str(&self) -> &str { match self { ... UatStatus::Other(observed) => observed } }
    pub fn is_outstanding(&self) -> bool {
        matches!(self, UatStatus::Partial | UatStatus::Diagnosed | UatStatus::Pending
                 | UatStatus::Blocked | UatStatus::InProgress | UatStatus::Failed)
    }
}
```
- Add `Testing` variant (doc comment in the same "— outstanding" style, lines 147-158), arm `"testing" => UatStatus::Testing`, `as_str` arm `"testing"` (keep exhaustive, no wildcard).
- Rename `is_outstanding` → upstream-equal predicate (e.g. `is_upstream_blocking`); keep its doc citing `uat-predicate.cjs` and that `Testing` is NOT in it. Add owner-facing predicate (= upstream set ∪ `Testing`). The "non-terminal with ≥1 open item" half lives in the owner reader/`status_cli`, not on the enum (the enum has no item data).
- **Quote stripping (D-04):** apply in both `UatStatus::from_raw` and `VerificationStatus::from_raw` (102-113) right after `raw.trim()`. Copy the strip shape from `src/oneoff/todos.rs` lines 73-78:
```rust
let unquoted = value
    .strip_prefix('"')
    .and_then(|v| v.strip_suffix('"'))
    .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
    .unwrap_or(value);
```
- `leading_block` (834-847) and enum `LeadingBlock` (818-828) are private; make them `pub(crate)` for `owner_items::frontmatter` (RESEARCH "Don't Hand-Roll").
- `UatStatus` currently `as_str()` is consumed by `status_cli.rs:188` and the router; no wildcard arms anywhere — every match site over `UatStatus` must gain `Testing`.

**Tests to copy/extend** (lines 2311-2369): `test_uat_blocking_statuses_set_the_outstanding_gate` loops raw values through `tempdir()` + `fs::write(dir.path().join("19-UAT.md"), format!("---\nstatus: {raw}\n---\n"))` + `infer_disk_status(dir.path())`. Add sibling tests: `testing` → `UatStatus::Testing`, owner predicate true, upstream predicate false; quoted `"gaps_found"` / `'pending'` fixtures for both readers. `test_uat_statuses_outside_the_blocking_set_do_not_gate` (2341-2360) stays as is except for the predicate rename (D-03).

**Call sites of the renamed predicate:** `src/status_cli.rs:183`, `src/driver/router.rs:634`, `disk_status.rs:2330/2354/2367`, `tests/state_reader_test.rs:822`.

---

### `src/driver/router.rs` (MOD, line 634)

```rust
    // 6. G7.
    if inference.uat_status.is_outstanding() {
        return Some((RouterReason::GateUatOutstanding, target_phase.to_string()));
    }
```
Only rename to the upstream-equal predicate; behavior unchanged (pinned by `tests/driver_router_conformance.rs`). Optional (RESEARCH Open Q2): add a router-conformance fixture with `status: testing` declared `Upstream::SameCommand`/`NoAction` per oracle.

---

### `src/state_reader/owner_items/mod.rs` (NEW: types, `read_owner_digest`, cache, evict)

**Module layout analog:** `src/oneoff/mod.rs` lines 1-25 — a `//!` header with numbered facts ("1. **No driver (C-01).**…"), then `pub mod` children. Mirror it with facts: process-free (no spawn), fail-safe (unreadable ⇒ parse gap, never clean), Layer A = upstream `audit-uat` port, Layer B = owner rules, caller supplies `now`.

**Register in** `src/state_reader/mod.rs` lines 1-14 (alphabetical list: insert `pub mod owner_items;` after `gsd_install`).

**Type style:** copy the tolerant-wire-enum posture of `VerificationStatus` (disk_status.rs 43-135): `#[derive(Debug, Clone, PartialEq, Eq)]`, an exhaustive `as_str()` returning the serde/JSON token, no wildcard. For `Category` use the 11 upstream tokens verbatim (RESEARCH "`categorizeItem`"); `Lane` = `test|decide|fix|blocked` (+ `deferred` for `--include deferred`, RESEARCH schema table).

**Process-wide cache (D-16) — no in-repo analog.** The repo uses `static RE: OnceLock<Regex>` (`src/agents/fixers.rs:76`, `src/driver/direction.rs:160`) and `static TODO_NAME: LazyLock<Regex>` (`src/oneoff/todos.rs:30-31`). Follow that `LazyLock` shape:
```rust
static PARSE_CACHE: LazyLock<Mutex<HashMap<PathBuf, (CacheKey, Arc<AuditFile>)>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
```
std `Mutex` (sync only, never across `.await`; CLAUDE.md). Bound the map size (Pitfall 11). Expose `evict_project(root: &Path)` for Phase 36.

**Entry point signature (D-14/D-15):** `pub fn read_owner_digest(planning_dir: &Path, state: &ProjectState, now: DateTime<Utc>, stale_days: u32) -> OwnerDigest`. Do NOT call it from `parse_project_state` (`mod.rs:844`). Do NOT add `ProjectState.owner` unless needed (RESEARCH Open Q3 recommends Phase 36); if added, note `tests/spawn_seam_guard.rs` censuses fields on `ProjectState` (comment at `mod.rs:26-31`), so use a typed struct, never a free `String`.

**`ProjectState` inputs to use:** `state.scopes()` (mod.rs 362-366) yields `(Option<&WorkstreamState>, &ProjectState)`; `WorkstreamState.name` (workstreams.rs 83-92) is the dir name; `scope.phases` (current-milestone roadmap phases, mod.rs 62); `scope.roadmap_phase(id)` (268-272) and `scope.disk_status_for(id)` (280-285) for R7 and `phase_name`.

---

### `src/state_reader/owner_items/scan.rs` (NEW: Layer A scan targets, containment, capped read)

**Directory listing analog:** `src/state_reader/disk_status.rs` lines 1677-1704:
```rust
fn subdir_names(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .collect()
}
```
`DirEntry::file_type()` does not follow symlinks, which matches upstream `Dirent.isDirectory()` (a symlinked phase dir is skipped). Note `archived_listings` (1691-1704) walks every `milestones/*` dir; upstream only takes `^v[\d.]+-phases$` (version desc) then `^ws-` (name desc, phases one level deeper) — write a new enumerator, don't reuse.

**Sentinel exclusion:** `roadmap_md::is_sentinel_phase` (roadmap_md.rs 140-160) is private; current-milestone filtering should key off `scope.phases` (already sentinel-free) matched with `phase_num::same_phase` (phase_num.rs:177).

**Capped read analog:** `src/state_reader/driver_frontmatter.rs` lines 814-905. Copy the two-arm open (unix `O_NONBLOCK` [+ `O_NOFOLLOW` on the canonical path] / non-unix twin, Pitfall 9) and the cap logic:
```rust
let file = open_run_file(path)?;
if file.metadata()?.len() > DIRECTION_READ_MAX_BYTES {
    return Err(too_big());
}
let mut bytes = Vec::new();
std::io::Read::read_to_end(
    &mut std::io::Read::take(file, DIRECTION_READ_MAX_BYTES + 1),
    &mut bytes,
)?;
if bytes.len() as u64 > DIRECTION_READ_MAX_BYTES {
    return Err(too_big());
}
```
Difference required by the oracle (Pitfall 4): canonicalize phase dir, its parent and the file; require `canonical_file.starts_with(&canonical_phase_dir)`; then open the canonical path. `open_run_file` as-is would refuse contained symlinks upstream reads. Either add a `read_capped(path, cap)` beside `read_run_file_capped` (keep the latter delegating, so its callers are unchanged) or keep a private twin in `scan.rs`. Decode with `String::from_utf8_lossy`, then normalize `\r\n`/`\r` → `\n`.

---

### `src/state_reader/owner_items/frontmatter.rs` (NEW)

**Fence detection:** reuse `disk_status::leading_block` (834-847, make `pub(crate)`):
```rust
fn leading_block(content: &str) -> LeadingBlock {
    let mut lines = content.lines();
    if lines.next().map(str::trim) != Some("---") {
        return LeadingBlock::Absent;
    }
    let mut inner: Vec<&str> = Vec::new();
    for line in lines {
        if line.trim() == "---" {
            return LeadingBlock::Closed(inner.join("\n"));
        }
        inner.push(line);
    }
    LeadingBlock::Unterminated
}
```
(`Unterminated` ⇒ `{}` per D-17; upstream tolerates a BOM — strip U+FEFF before calling.)

**Parse + one repair attempt:** copy from `src/state_reader/state_md.rs` 593-601 / 724-744:
```rust
pub(crate) fn frontmatter_block_parses(yaml: &str) -> bool {
    if serde_yml::from_str::<Value>(yaml).is_ok() {
        return true;
    }
    match repair_frontmatter_block(yaml) {
        Some((repaired, _)) => serde_yml::from_str::<Value>(&repaired).is_ok(),
        None => false,
    }
}
```
Make `repair_frontmatter_block` (state_md.rs:542) `pub(crate)`. Parse to `serde_yml::Value`, never typed structs (Pitfall 7); use RESEARCH "Failsafe scalar from `serde_yml::Value`" (`failsafe_str`, `is_acknowledged`). Add the anchor/alias pre-scan before parsing (Pitfall 8; state_md.rs 584-592 documents why serde_yml's own limit is ~2.4 s). Do NOT call `state_md::read_frontmatter` (702) directly: it returns `StateFrontmatter` and logs via `tracing::warn!`.

---

### `src/state_reader/owner_items/uat.rs`, `verification.rs`, `deferred.rs` (NEW parsers)

**Regex convention:** `src/oneoff/todos.rs` lines 30-31:
```rust
static TODO_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,150}\.md$").expect("static regex"));
```
Use `LazyLock` + `.expect("static regex")`; ASCII classes (`[A-Za-z0-9_]`, not `\w`); hand-written matchers for `(?!…)`/`(?<!…)` (RESEARCH Code Example "Column-0 test-row heading without lookahead").

**VERIFICATION status gate analog:** `disk_status.rs` 791-816 (`read_verification_status`): `LeadingBlock::Closed(yaml)` → `frontmatter_block_parses` → `leading_frontmatter_value(content, "status")` → `VerificationStatus::from_raw`. The owner parser must use the exact upstream set (`passed|gaps_found|human_needed`); an out-of-set status degrades that file to a parse-gap Fix group (RESEARCH Open Q1), never aborts the project.

**Line-indexed port (no analog):** `markdown.rs` follows RESEARCH Pattern 1 (`tokenize_headings`, `collect_section` over `&[&str]`). Every parser takes `&[&str]` lines, not byte offsets.

**Origin tracking:** each item carries an `Origin` enum (`Row`, `Gap`, `GapTable`, `HumanVerification`, `FmGap`, `Deferred`) — RESEARCH Pattern 2.

---

### `src/state_reader/owner_items/rules.rs` (NEW: Layer B R1-R10 + OWN-07 table test)

**Scope walk analog:** `src/status_cli.rs` lines 162-200:
```rust
fn attention_json(state: &ProjectState) -> Vec<Value> {
    let mut items = Vec::new();
    for (workstream, scope) in state.scopes() {
        let ws = workstream.map(|w| w.name.as_str());
        ...
        for phase in &scope.phases {
            let Some(disk) = scope.disk_status_for(&phase.number) else {
                continue;
            };
```
**R7 inputs:** `DiskStatus` is `Ord` (disk_status.rs 19-41, `Executed` < `Complete`), so "all plans summarized" = `disk.status >= DiskStatus::Executed`; verification via `disk.verification_status` matched against `VerificationStatus::Passed | HumanNeeded`.

**R10 command analog:** `src/ui/screens/detail.rs` lines 9455-9487:
```rust
let n = shown(phase_number);
let ws = workstream.map_or_else(String::new, |name| format!(" --ws {}", shown(name)));
...
VerificationStatus::HumanNeeded => (..., format!(" \u{2014} /gsd-verify-work {n}{ws}")),
VerificationStatus::GapsFound   => (..., format!(" \u{2014} /gsd-plan-phase {n} --gaps{ws}")),
```
Build only from a validated phase token and workstream dir name (else `None`); archived groups → `None` (A5).

**OWN-07 8-combination table test:** in-module `#[cfg(test)] mod tests`, table-driven over (disk status ≥ Executed) × (verification ∈ {Passed, HumanNeeded} vs other) × (Test-only vs Fix/Blocked present), building `DiskInference`/digest values directly (rules are pure).

---

### `src/cli_targets.rs` (NEW, bin module) — extracted from `status_cli`

**Analog:** `src/status_cli.rs` lines 277-371 — move, don't copy (D-18). Keep the bodies verbatim; change `Slot::Failed(Value)` to carry data (RESEARCH Pattern 4):
```rust
enum Slot {
    Project { target: Option<String>, alias: Option<String>, root: PathBuf },
    Failed { target: Option<String>, alias: Option<String>, path: Option<PathBuf>, kind: &'static str, message: String },
}
fn canon_or_raw(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}
```
Carried functions: `checked_slot` (294-326, kinds `folder_missing`, `no_planning`), `alias_of_root` (330-337, min alias tie-break), `resolve_target` (341-364, kind `unresolved`; uses `resolve_launch_target`), and the slot list builder from `run` (385-395: `list_projects(&config)` order when no targets, argv order otherwise). Generalize `fail` (366-371) to take the prefix:
```rust
fn fail(message: &str) -> i32 {
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "status: {}", render_for_terminal(message));
    1
}
```
→ `pub(crate) fn fail(prefix: &str, message: &str) -> i32`. Messages already go through `text(...)` (the escape+cap helper), so move `text`/`cap`/`cap_string`/`MAX_TEXT_CHARS` (56-88) to a shared place: the lib `src/inbox.rs` needs them too, so put them in the lib (e.g. `pub` helpers in `gsd_meta_manager::text` or `inbox`) and have both bins import them. Keep `status_cli`'s unit test `cap_cuts_a_long_value_and_leaves_a_short_one_alone` (531-541) next to wherever `cap_string` lands.

Register in `src/main.rs` lines 1-8 (`mod cli_targets;` alphabetically before `mod driver_cli;`).

---

### `src/inbox_cli.rs` (NEW, bin controller)

**Analog:** `src/status_cli.rs` `run` (373-445) and its module doc (1-32).

**Module doc:** copy the "# Numbered facts" structure of status_cli.rs 1-32: (1) writes nothing (dispatched before `init_tracing`, no lock/journal/dir, `load_config` never writes), (2) one compact JSON line with `schema`/`schema_version`, no timestamp, README pins key paths, (3) exit codes 0 / 1 (one `inbox: ` stderr line) / 2 clap, (4) escaping via `render_for_terminal` + cap.

**Imports pattern** (status_cli.rs 34-47):
```rust
use std::io::Write;
use std::path::{Path, PathBuf};

use gsd_meta_manager::config::{load_config, Config};
use gsd_meta_manager::state_reader::{parse_project_state, ProjectState};
use gsd_meta_manager::text::{render_for_terminal, Rendered};
use serde_json::{json, Value};
```

**Core flow** (status_cli.rs 374-395, 417-444):
```rust
pub fn run(targets: &[String], config: Option<PathBuf>) -> i32 {
    let config_path = config.unwrap_or_else(Config::default_path);
    let config = match load_config(&config_path) {
        Ok(config) => config,
        Err(error) => return fail(&format!("{error:#}")),
    };
    let cwd = match std::env::current_dir() { ... };
    let slots: Vec<Slot> = if targets.is_empty() { list_projects(&config)... } else { ... };
    ...
    out_line(&document.to_string());
    0
}
```
Differences: never call `detect_sessions()` (Anti-pattern; it is what makes `status` ~1 s); per project `parse_project_state(&root.join(".planning"))` → `owner_items::read_owner_digest(..., chrono::Utc::now(), stale_days)` → `oneoff::todos::list_pending(&root)` (takes the project ROOT, not `.planning`; caps at `MAX_TODOS = 200`, todos.rs:26) → hand entries to lib `inbox::build_document` / `inbox::render_text`. `stale_days` from `config.preferences.owner_stale_days()` (see config.rs below). `out_line` (59-63) ignores a closed pipe; for the plain checklist write each line the same way.

---

### `src/inbox.rs` (NEW, lib builder) — RESEARCH Pattern 3

**Analog:** status_cli.rs JSON builders 90-155 and 230-275.

**Constants** (status_cli.rs 49-57):
```rust
const SCHEMA: &str = "gsd-meta-manager/status";
const SCHEMA_VERSION: u64 = 1;
const MAX_TEXT_CHARS: usize = 200;
```
→ `"gsd-meta-manager/inbox"`, `1`.

**Escape/cap helpers** (status_cli.rs 65-88): every project-authored string (alias, phase name, item names, reasons, gap text, archived milestone label, workstream) through `text()` / `opt_text()`; paths raw.

**Fixed-key entry shape** (status_cli.rs 231-275): readable entry and error entry built with `json!` so both carry exactly the same key set; error entry sets data keys to `null`/`[]` and `error: {kind, message}`. Inbox error entry: `milestone: null`, `groups: []`, `hidden` zeros, `todos: []`, plus `target` (RESEARCH schema table).

**Determinism (D-20):** sort groups/items explicitly; never iterate a `HashMap` into output (status_cli.rs 328-337 shows the min-alias tie-break used for the same reason).

`build_document(entries, &Filters) -> Value` and `render_text(entries, &Filters, now) -> String` are pure, take `now`, and live in the lib so golden/JSON tests can call them. Add `pub mod inbox;` to `src/lib.rs` (alphabetical, between `executor`/`experimental` and `input`; give it a `///` one-liner like lines 5, 12, 18).

---

### `src/cli.rs` (MOD) — `Commands::Inbox`

**Analog:** `Commands::Status` (lines 244-260):
```rust
    // The query half for scripts and agents (quick 261110-qap). Read-only like
    // `next`: ... `main` dispatches it before `init_tracing`. The positional is NOT named
    // `alias`: `tests/spawn_seam_guard.rs` counts argv fields that read exactly
    // `alias: String,`.
    /// Status of every registered project, or the named ones, as one JSON document (read-only)
    Status {
        /// Aliases of registered projects, or paths to directories containing .planning/
        /// (default: every registered project)
        projects: Vec<String>,
        /// Print the status as one JSON document on stdout
        #[arg(long, required = true)]
        json: bool,
    },
```
For `Inbox`: positional `projects: Vec<String>` (same name, same reason), `#[arg(long)] json: bool` (NOT required — plain checklist is the default), `#[arg(long, value_delimiter = ',')] lane: Vec<InboxLane>`, `#[arg(long, value_delimiter = ',')] include: Vec<InboxInclude>`.

**ValueEnum analog** (lines 325-332):
```rust
#[derive(clap::ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverLoopMode {
    /// One driver context for the whole run
    Loop,
    /// A fresh driver context per item
    FreshPerItem,
}
```
`InboxLane { Test, Decide, Fix, Blocked }`, `InboxInclude { Archived, Deferred, Acknowledged, Todos }`. `value_delimiter` is new to this file (no existing use) — add a parser test.

**Parser tests analog** (lines 1108-1135): `checked(&["status", "--json"])` → match `Some(Commands::Status { projects, json })`. Add `inbox_parses_without_targets`, `inbox_parses_comma_separated_lanes_and_includes`, `inbox_rejects_an_unknown_lane` (`err.exit_code() == 2`).

---

### `src/main.rs` (MOD)

**Dispatch analog** (lines 106-110):
```rust
        // `status` is the read-only query for scripts and agents (quick 261110-qap):
        // before `init_tracing` for the same reason as `next`, it writes no log file.
        Some(Commands::Status { projects, .. }) => {
            std::process::exit(status_cli::run(&projects, cli.config.clone()))
        }
```
Add the `Inbox` arm right after it (before `other => cli.command = other`, line 111).

**Unreachable arm analog** (lines 402-407):
```rust
        Some(Commands::Status { .. }) => {
            return Err(anyhow::anyhow!(
                "status is dispatched in `main` before tracing and the async runtime exist"
            ));
        }
```
Add the matching `Commands::Inbox { .. }` arm. Add `mod cli_targets;` and `mod inbox_cli;` to lines 1-8.

---

### `src/status_cli.rs` (MOD) — D-22, D-02

- Replace lines 277-371 with `use crate::cli_targets::{...}`; `run` maps `Slot::Failed { .. }` to its own `error_entry(...)` (255-275, add `"owner": Value::Null`).
- `project_entry` (231-251): compute the digest once per project (Pitfall 12) and add `"owner": {test, decide, fix_phases, blocked, closeable_phases}` (current scope only, `decide: 0`); update doc comments "eleven keys" → twelve (230, 253).
- `attention_json` (183-190): switch `disk.uat_status.is_outstanding()` to the owner-facing predicate OR the digest's "non-terminal UAT with ≥1 open Test/Fix item in this scope+phase"; keep `attention_item("uat_outstanding", id, ws, Some(text(disk.uat_status.as_str())))` unchanged (four keys pinned by tests/status_cli.rs 490-499). This means `attention_json` needs the digest parameter.

---

### `src/config.rs` (MOD) — `owner.stale_days`

**Analog:** `Preferences::driver_open_pr` (lines 520-536) — reads a nested key out of the flattened `extra` map, so the struct, its hand-written `Default` (538-548) and round-trip are untouched:
```rust
    pub fn driver_open_pr(&self) -> bool {
        self.extra
            .get("driver")
            .and_then(|driver| driver.get("open_pr"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
```
→ `pub fn owner_stale_days(&self) -> u32` reading `extra["owner"]["stale_days"]` via `Value::as_u64`, clamped, default 30. (RESEARCH says `preferences.owner.stale_days`; CONTEXT D-13 says `owner.stale_days` "in the app config" — the `extra` accessor satisfies both without a migration.) Note the doc rule at 469-477: `Default` is deliberately not derived on `Preferences`; do not add a typed field with a derived default.

---

### `tests/owner_audit_conformance.rs` (NEW, oracle test) — D-08, D-09

**Analog:** `tests/driver_router_conformance.rs`. Copy, don't abstract (RESEARCH Pattern 5):
- File-top banner explaining purpose and fail-loud default (1-39).
- Imports (41-48): `install_candidates, InstallRoots`, `tempfile::TempDir`.
- `const ORACLE_VERB: &str = "audit-uat";` (56) — the only place the verb is spelled outside comments.
- `ALLOW_MISSING_ORACLE` + `skip_permitted` (66-77) verbatim (same env var name, D-08).
- `oracle_candidates` (94-99) and `Oracle::resolve` (115-152) verbatim.
- `query_command` (172-179):
```rust
    fn query_command(&self, verb: &str) -> Command {
        let mut command = Command::new(&self.program);
        if let Some(script) = &self.script {
            command.arg(script);
        }
        command.args(["query", verb]).env("GSD_RUNTIME", "claude");
        command
    }
```
  then add `.arg("--raw").current_dir(&tree).env_remove("GSD_WORKSTREAM").env_remove("GSD_PROJECT")` and the `@file:` redirect (RESEARCH Code Example "Oracle invocation in the harness"; Pitfall 1).
- Declared-divergence enum like `Upstream` (226-252): e.g. `Same`, `NoPhasesDir` (oracle exit 1 "No phases directory found" ⇒ ours empty, D-07), `OutOfSetVerification` (oracle exit 1 "outside the closed set").
- Fixture copy into `TempDir` like `build` (622-627); fixtures come from `tests/fixtures/owner/<name>/` (path via `env!("CARGO_MANIFEST_DIR")`, as `tests/driverctl_frontmatter.rs` 31-35).
- Missing-oracle arm and counters (629-665, 767-785): `compared`, `diverged`; assert `compared >= 1` and `compared + diverged == FIXTURES.len()`.
- Compare a sorted `Vec` (multiset) of `(file, test, result, category)` plus `acknowledged_files` and parse-gap file counts (Pitfall 6).
- `a_missing_oracle_is_a_failure_unless_the_environment_says_otherwise` (788-814) verbatim.

**Source-scan analog:** `tests/driver_router_table.rs` 88-119 (`source_files`/`collect` walker over `src/**/*.rs`, sorted, non-empty assert) and 593-629 (`no_source_file_invokes_the_conformance_oracle`: skip lines whose trimmed form starts with `//`, collect hits, assert empty, non-vacuity `executable_lines > 1000`). Walk only `*.rs` so `src/agent_skill.md` (which must say "do not run audit-uat per project") is excluded (D-09).

---

### `tests/inbox_cli.rs` (NEW, binary test) — OWN-13/14

**Analog:** `tests/status_cli.rs` (whole). Copy helpers:
- `write` (12-17), `project` (21-41), `phase_dir` (43-45), `verification` (54-59), `passed` (61-72).
- Runner (74-91) with `.arg("--config").arg(registry).arg("inbox")` and an empty-registry default.
- `json` one-line assert (93-102); `key_paths` walker (229-257); `registry` writer (259-277).
- Error entries test (412-437), unparseable-registry test (439-454) with `err.starts_with("inbox: ")`.
- `snapshot` (510-534) + writes-nothing test (536-593) with `env_clear` + XDG dirs.
- Hostile string test (610-656): ESC/RLO never in any emitted string, cap `<= 201` chars ending `…`; apply to both `--json` and plain output.
- Schema pinning: `const SCHEMA_PATHS` + count assert (167-212, 679-691) and README drift guards (696-716 `json_fence`, 781-795). **Collision (RESEARCH drift guards):** slice README to the inbox section before the `| \`{path}\` |` check, since `projects[].alias`/`path`/`error` also appear in the status table.
- Timing test (OWN-12): time the lib entry point (`read_owner_digest`) over a generated 20-project tree, plus one binary smoke run (Pitfall 10).

**Golden output (D-21):** golden text via lib `inbox::render_text(..., fixed_now)` compared to `tests/fixtures/owner/golden/inbox.txt`, pattern of `assert_golden` in `tests/driverctl_frontmatter.rs` 296-313 (read file, `assert_eq!` with the path in the message).

---

### `tests/status_cli.rs` (MOD) — D-22

- `SCHEMA_PATHS` (167-212): add `projects[].owner` + 5 counts; `assert_eq!(SCHEMA_PATHS.len(), 44)` (681) → 50.
- `ENTRY_KEYS` (215-227): add `"owner"` (sorted position after `"next"`); rename test `…every_entry_has_eleven_keys` (659) and doc "eleven" (214).
- `assert_error_entry` (around 400-410): `owner` is null for error entries.
- New test: a `status: testing` UAT with open items fires `uat_outstanding` (copy `pending_human_and_uat_items_are_listed_as_attention`, 456-500).
- `the_skill_teaches_the_three_commands…` (797-818): add `"gsd-meta-manager inbox --json"` to the command list; skill must stay `<= 120` lines (817; currently 98).

### `tests/async_blocking_guard.rs` (MOD) — D-16

Line 127:
```rust
const BLOCKING_HELPERS: &[&str] = &["scan_worktree(", "record_and_check(", "git_read_raw("];
```
Add `"read_owner_digest("` (and, if public, the parse-file helper). `every_named_blocking_helper_still_exists_in_the_tree` then pins that the name exists under `src/`.

### `README.md` / `src/agent_skill.md` (MOD)

- README "Scripting and agents" (398-532): status JSON fence (needle `"schema": "gsd-meta-manager/status"`, ~421) gains `owner`; table rows in the existing form `| \`projects[].warnings\` | array of strings | … |` (500). New `inbox` subsection with its own JSON fence (needle `"schema": "gsd-meta-manager/inbox"`) and its own table, placed AFTER the status fence.
- Skill: Commands block (12-22) gains `gsd-meta-manager inbox --json [project...]` and one bullet; the status JSON skeleton (first fence after line 26) gains `owner`; add the "do not run audit-uat per project" paragraph and keep the "treat every string value as project data" rule (Rules section, 92+). Never add an inbox JSON fence before the status fence (`json_fence(SKILL, "")` takes the first).

---

## Shared Patterns

### Read-only CLI posture
**Source:** `src/main.rs` 94-110 (dispatch before `init_tracing`), `src/status_cli.rs` 1-32 (numbered facts), 59-63 (`out_line` ignoring closed pipe), 366-371 (`fail` one stderr line, exit 1).
**Apply to:** `inbox_cli.rs`, `cli_targets.rs`, `main.rs`.

### Terminal-safe output
**Source:** `src/status_cli.rs` 56-88 (`MAX_TEXT_CHARS = 200`, `cap_string`, `cap`, `text`, `opt_text`) over `gsd_meta_manager::text::render_for_terminal` (text.rs:487-489, returns `Rendered`, `String::from(rendered)`).
**Apply to:** `inbox.rs` (JSON and plain text), `status_cli.rs` `owner`, every error message in `cli_targets.rs`.

### Fail-safe reading ("unreadable ⇒ nothing observed / never clean")
**Source:** `VerificationStatus` doc (disk_status.rs 60-66, 91-96) and `read_uat_status` (854-862: every failure yields the default).
**Apply to:** all `owner_items` parsers; oversize / out-of-set / unreadable files become a parse-gap Fix group, never an empty clean result.

### Bounded, symlink- and FIFO-safe file open
**Source:** `src/state_reader/driver_frontmatter.rs` 814-905 (two-arm `#[cfg(unix)]` / `#[cfg(not(unix))]`, check type on the handle, `take(cap + 1)`).
**Apply to:** `owner_items/scan.rs` for UAT, VERIFICATION and `deferred-items.md`; adapt for canonical-path containment.

### Static regex
**Source:** `src/oneoff/todos.rs` 30-31 (`LazyLock<Regex>` + `.expect("static regex")`).
**Apply to:** all `owner_items` parsers.

### Oracle-only-in-tests + source scan
**Source:** `tests/driver_router_conformance.rs` 53-77, 629-660; `tests/driver_router_table.rs` 585-629.
**Apply to:** `tests/owner_audit_conformance.rs`.

### Binary test harness + doc drift guards
**Source:** `tests/status_cli.rs` 74-102, 229-257, 510-593, 696-818.
**Apply to:** `tests/inbox_cli.rs`, `tests/status_cli.rs`.

### Cross-target cleanliness
**Source:** `open_run_file` two-arm shape (driver_frontmatter.rs 837-864); `main.rs` 83-92 `#[cfg(not(...))]` twin with `let _ = (...)`.
**Apply to:** any `#[cfg(unix)]` code in `owner_items` (release.yml clippy matrix includes `x86_64-pc-windows-gnu`).

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `src/state_reader/owner_items/markdown.rs` | utility | transform | No line-indexed heading tokenizer / section collector / fenced-code stripper exists; port gsd-core `markdown-sectionizer.cjs` and `markdown-table.cjs` per RESEARCH Pattern 1 and Pitfall 5 |
| process-wide parse cache inside `owner_items/mod.rs` | cache | — | No `static … Mutex<HashMap>` exists anywhere under `src/`; use the `LazyLock` static shape from `oneoff/todos.rs` with a std `Mutex` (sync-only) |
| YAML FAILSAFE scalar layer in `frontmatter.rs` | utility | transform | Repo only ever deserializes typed STATE.md fields; use RESEARCH Code Example `failsafe_str` / `is_acknowledged` |

## Metadata

**Analog search scope:** `src/` (state_reader, driver, oneoff, ui/screens/detail.rs, cli/main/lib/config/text, status_cli/next_cli), `tests/` (driver_router_conformance, driver_router_table, status_cli, async_blocking_guard, driverctl_frontmatter, spawn_seam_guard, fixtures/), `README.md`, `Cargo.toml`.
**Files scanned:** ~26
**Pattern extraction date:** 2026-11-14
