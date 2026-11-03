//! The per-run `CLAUDE_CONFIG_DIR` and where a run's launcher state lives
//! (Phase 28; D-18, D-19, Phase 27 hand-offs 1 and 8).
//!
//! The driver runs with a Claude config directory of its own, outside the driven
//! repository, so the host's `~/.claude` can stay read-only (and its credential
//! file masked) inside the sandbox. [`seed`] ports the spike's `seed_config`:
//!
//! * symlinks to the host's `agents`, `commands`, `skills`, `gsd-core`,
//!   `get-shit-done` and `hooks` (read-only inside) -- each only when the host has
//!   it and the config dir does not. **The global `CLAUDE.md` is deliberately not
//!   linked** (OQ-R2, `[inferred]`): the driver follows the master prompt and the
//!   project's own `CLAUDE.md`, not the operator's personal rules;
//! * [`settings_transform`] of the host `settings.json`: minus `statusLine`,
//!   `enabledPlugins`, `remoteControlAtStartup`, `agentPushNotifEnabled` and
//!   `apiKeyHelper`, minus any `env` entry the process scrub strips, and **minus every
//!   user hook that is not a GSD or rtk hook whose `$HOME` paths all lie inside the
//!   sandbox's allowlist** (28.1-08, D-19: [`hook_is_kept`]; wrong-shape hook and
//!   permission entries are dropped too, 30 WR-13; the claudear hooks reach
//!   the operator's phone bridge, and under the allowlist they would fail on every
//!   event anyway). The PreToolUse
//!   `Agent|Task` entry goes FIRST and carries `"onFailure": "block"` (hand-off 1),
//!   so a dead socket or a missing binary fails closed;
//! * **28.1 (D-12, D-13): the gate lives in the managed tier.** For a bwrap-scope
//!   run the gate's hooks and the agent-tool deny list are written to
//!   `<cfg>/managed/managed-settings.json` ([`managed_settings`]) and bound
//!   read-only at `/etc/claude-code` inside the sandbox, so a nested `claude` with
//!   another config dir, `--settings` or `--setting-sources` cannot shed them. The
//!   per-run `settings.json` then carries no driverctl entry (no hook fires twice).
//!   A degraded run has no `/etc` to bind, so it keeps the gate in `settings.json`
//!   with the deny list and writes no managed file. The host administrator's own
//!   `/etc/claude-code` policy is merged in, never hidden ([`read_host_managed`]),
//!   except that a host `hooks` or `permissions` value of the wrong shape is dropped
//!   (a non-object block, or a nested event, matcher entry, permission list or
//!   permission setting of the wrong shape; 28 WR-08, 30 WR-13), so it cannot
//!   displace the gate;
//! * a minimal `.claude.json` (`hasCompletedOnboarding`), only when absent;
//! * a `bin/driverctl` shim that sets `DRIVERCTL_SOCK` to the run's socket and execs
//!   this binary (hand-off 8), so the master prompt's `driverctl log ...` runs
//!   unchanged and unsetting the variable cannot detach a hook from the gate;
//! * a `bin/gsd-meta-manager` shim that execs this binary with `next` and its
//!   arguments, and refuses every other first argument (Phase 29, D-21), so the
//!   prompt's citation `gsd-meta-manager next {PROJECT_ROOT} --json` resolves on
//!   `PATH` inside the sandbox.
//!
//! The data directory follows the envelope's rule (`src/envelope/mod.rs`): an
//! `Option` with no `/tmp` fallback, because a shared directory is where a
//! squatter pre-creates a path.

use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::driverctl::serve::hash8;

use super::sandbox::is_stripped_env;

/// The application's directory under the data dir.
const APP_SUBDIR: &str = "gsd-meta-manager";

/// The launcher's subdirectory (D-18).
const DRIVER_SUBDIR: &str = "driver";

/// Host `~/.claude` entries linked read-only into the per-run config dir.
pub const LINK_FROM_HOST: &[&str] = &[
    "agents",
    "commands",
    "skills",
    "gsd-core",
    "get-shit-done",
    "hooks",
    // D-16: the host's GSD hooks call helpers under `~/.claude/scripts`; without the
    // link they fail inside the sandbox (a latent Phase 28 bug).
    "scripts",
];

/// The per-run subdirectory that is bound at `/etc/claude-code` (D-12).
pub const MANAGED_SUBDIR: &str = "managed";

/// The managed settings file inside [`MANAGED_SUBDIR`].
pub const MANAGED_FILE: &str = "managed-settings.json";

/// Where Claude Code reads managed settings on Linux, on the host and inside the sandbox.
pub const HOST_MANAGED_DIR: &str = "/etc/claude-code";

/// The drop-in directory name under [`HOST_MANAGED_DIR`].
const MANAGED_DROPIN_DIR: &str = "managed-settings.d";

/// Tool names denied through `permissions.deny` (D-13): agent-to-agent and cloud
/// channels a sandboxed driver must not use. Wildcards are Claude Code's own syntax.
pub const DENY_TOOLS: &[&str] = &[
    "Cron*",
    "RemoteTrigger",
    "SendMessage",
    "ListAgents",
    "ScheduleWakeup",
    "DesignSync",
    "mcp__claude_ai_*",
];

/// The `permissions` sub-keys that are lists of rule or path strings (30-REVIEW
/// WR-13). Every other `permissions` sub-key (`defaultMode`,
/// `disableBypassPermissionsMode`, `blockReadsOutsideWorkingDirectories`) is a string
/// or bool setting `[inferred — cleanup 2026-10-28]`.
const PERMISSION_LISTS: &[&str] = &["allow", "ask", "deny", "additionalDirectories"];

/// Managed-settings keys the launcher pins; a host value never replaces them. Each
/// is a lock whose launcher value is already the strictest one.
///
/// `disableAllHooks` is deliberately NOT here (IN-03): a host administrator's value for
/// it stays in force, as every other host managed key does, and is never hidden. A host
/// that sets it to `true` therefore switches the gate hooks off for driver runs too; the
/// launcher's own caps (wall clock, gap rounds, the scope limits) do not depend on them.
const LOCK_KEYS: &[&str] = &[
    "skipWebFetchPreflight",
    "disableClaudeAiConnectors",
    "enableArtifact",
    "disableRemoteControl",
    "crossSessionInbound",
];

/// Host settings keys that never reach the driver: UI and plugin keys, and
/// `apiKeyHelper`, which outranks `CLAUDE_CODE_OAUTH_TOKEN` (RESEARCH §3).
pub const DROPPED_SETTINGS_KEYS: &[&str] = &[
    "statusLine",
    "enabledPlugins",
    "remoteControlAtStartup",
    "agentPushNotifEnabled",
    "apiKeyHelper",
];

/// The timeout of every driverctl hook entry, in seconds (spike value).
const HOOK_TIMEOUT_S: u64 = 20;

/// `dirs::data_local_dir()/gsd-meta-manager/driver`, or `None` when the platform
/// has no data dir. Never a `/tmp` fallback.
pub fn driver_root() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join(APP_SUBDIR).join(DRIVER_SUBDIR))
}

/// `<root>/<hash8(canonical project)>/<run-id>`: the launcher's state directory
/// for one run (the config dir, the launch logs).
///
/// `run_id` must already be a validated run id (one path component).
pub fn run_state_dir_in(root: &Path, canonical_project: &Path, run_id: &str) -> PathBuf {
    root.join(hash8(canonical_project)).join(run_id)
}

/// What [`seed`] needs to know.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedSpec {
    /// The per-run `CLAUDE_CONFIG_DIR` to create and fill.
    pub config_dir: PathBuf,
    /// The host's `~/.claude`: read for the links and the settings, never written.
    pub host_claude_dir: PathBuf,
    /// The binary the `driverctl` shim execs.
    pub binary: PathBuf,
    /// The run directory the Stop-guard entry (`driverctl stop-guard --run-dir`) names.
    pub run_dir: PathBuf,
    /// The run's driverctl socket: baked into the `bin/driverctl` shim as
    /// `DRIVERCTL_SOCK` (RESEARCH Pattern 6 `[inferred — audit]`).
    pub socket: PathBuf,
    /// True for a bwrap-scope run: the gate goes to `<cfg>/managed` (bound at
    /// `/etc/claude-code`) instead of `settings.json`.
    pub managed: bool,
    /// The host's managed-settings directory ([`HOST_MANAGED_DIR`] in production),
    /// merged into the per-run managed file.
    pub host_managed_dir: PathBuf,
    /// The `$HOME` entries the sandbox re-binds (`HomeSetup::keep_roots`): a user hook
    /// survives into the per-run settings only when every `$HOME` path it names lies
    /// inside one of these (D-19). Empty keeps no user hook.
    pub keep_roots: Vec<PathBuf>,
}

/// The two quoted shim command lines the gate's hooks run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateCommands {
    /// `'<shim>' hook`: the dispatch gate, heartbeat and post-tool entries.
    pub hook: String,
    /// `'<shim>' stop-guard --run-dir '<rd>'`: the Stop guard (D-17).
    pub stop_guard: String,
}

/// `text` as one single-quoted shell word.
fn shell_quote(text: &OsStr) -> String {
    let raw = String::from_utf8_lossy(text.as_bytes());
    format!("'{}'", raw.replace('\'', "'\\''"))
}

/// The hook command line: the shim path, single-quoted, then the verb.
pub fn hook_command(shim: &Path, verb: &str) -> String {
    format!("{} {verb}", shell_quote(shim.as_os_str()))
}

/// One hook entry running `command`.
fn hook_entry(matcher: Option<&str>, command: &str, on_failure_block: bool) -> Value {
    let mut hook = json!({"type": "command", "command": command, "timeout": HOOK_TIMEOUT_S});
    if on_failure_block {
        hook["onFailure"] = json!("block");
    }
    let mut entry = Map::new();
    if let Some(matcher) = matcher {
        entry.insert("matcher".to_owned(), json!(matcher));
    }
    entry.insert("hooks".to_owned(), json!([hook]));
    Value::Object(entry)
}

/// The `hooks.<event>` array of `settings`, created (and coerced from a
/// non-array) when absent.
fn hook_list<'a>(hooks: &'a mut Map<String, Value>, event: &str) -> &'a mut Vec<Value> {
    let slot = hooks.entry(event.to_owned()).or_insert_with(|| json!([]));
    if !slot.is_array() {
        *slot = json!([]);
    }
    match slot {
        Value::Array(list) => list,
        _ => unreachable!("the slot was just made an array"),
    }
}

/// The host `settings.json` as an object: absent, unparseable or not an object
/// is an empty object `[inferred — audit]` (a corrupt host file must not block
/// every launch; the driver then simply has no GSD/rtk hooks).
fn read_host_settings(host_claude_dir: &Path) -> Map<String, Value> {
    fs::read(host_claude_dir.join("settings.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|value| match value {
            Value::Object(map) => Some(map),
            _ => None,
        })
        .unwrap_or_default()
}

/// Write `contents` to `path` through a temp file and a rename, with `mode`.
pub(super) fn write_atomic(path: &Path, contents: &[u8], mode: u32) -> io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    io::Write::write_all(&mut tmp, contents)?;
    fs::set_permissions(tmp.path(), fs::Permissions::from_mode(mode))?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(())
}

/// The file name of the executable snapshot inside `<config-dir>/bin` ([`snapshot_exe`]).
pub const EXE_SNAPSHOT_NAME: &str = "gsd-meta-manager.exe";

/// The suffix Linux appends to `/proc/self/exe`'s target once the file is unlinked.
const DELETED_SUFFIX: &str = " (deleted)";

/// The path of this binary, checked to still exist (IN-05).
///
/// After an in-place upgrade (the file replaced while this process runs) Linux reports
/// the old path with a ` (deleted)` suffix. The suffix is stripped when the path
/// without it exists (the upgraded binary); otherwise the launch is refused with a
/// message, because a shim or bridge command that names a missing file would fail
/// every spawn with exit 127.
pub fn resolve_current_exe() -> io::Result<PathBuf> {
    resolve_exe_path(std::env::current_exe()?)
}

/// [`resolve_current_exe`] for a given reported path.
fn resolve_exe_path(reported: PathBuf) -> io::Result<PathBuf> {
    if reported.exists() {
        return Ok(reported);
    }
    if let Some(stripped) = reported
        .to_str()
        .and_then(|text| text.strip_suffix(DELETED_SUFFIX))
        .map(PathBuf::from)
    {
        if stripped.exists() {
            return Ok(stripped);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!(
            "this binary ({}) no longer exists on disk (replaced or removed while it was starting); start the launch again",
            reported.display()
        ),
    ))
}

/// The binary the shim, the gate hooks and the bridge execute (WR-05).
///
/// A binary that lies under the driven `project` (`target/debug/gsd-meta-manager` while
/// the tool drives its own repository) is writable by the driver through the read-write
/// project bind, and the host later runs it. It is copied to
/// `<config_dir>/bin/`[`EXE_SNAPSHOT_NAME`] instead (launcher-owned, outside the project,
/// read-only inside through the `<config_dir>/bin` bind) and that copy is returned. The
/// project's own `target/` stays read-write, so `cargo build` keeps working, and nothing
/// is bound read-only inside the project. Any other binary is returned unchanged.
///
/// `project` must be canonical. The copy goes through a temp file and a rename, so a
/// concurrent reader never sees a half-written executable.
pub fn snapshot_exe(exe: &Path, project: &Path, config_dir: &Path) -> io::Result<PathBuf> {
    let real = fs::canonicalize(exe).unwrap_or_else(|_| exe.to_owned());
    if !real.starts_with(project) && !exe.starts_with(project) {
        return Ok(exe.to_owned());
    }
    let bin = config_dir.join("bin");
    create_private_dir(&bin)?;
    let dest = bin.join(EXE_SNAPSHOT_NAME);
    let mut src = fs::File::open(&real)?;
    let mut tmp = tempfile::NamedTempFile::new_in(&bin)?;
    io::copy(&mut src, &mut tmp)?;
    fs::set_permissions(tmp.path(), fs::Permissions::from_mode(0o755))?;
    tmp.persist(&dest).map_err(|e| e.error)?;
    Ok(dest)
}

/// Create `dir` and its missing parents, each 0700.
fn create_private_dir(dir: &Path) -> io::Result<()> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
}

/// Insert the gate's hook entries into `hooks`: the dispatch gate
/// (`PreToolUse`, `Agent|Task`, `onFailure: block`) FIRST, so it runs before any
/// other `PreToolUse` hook; the `SessionStart`, `SessionEnd`, `Stop` and
/// `PostToolUse` entries go last, and the stop guard (D-17) is the very last `Stop`
/// entry. Shared by the degraded `settings.json` and the managed file, so the two
/// postures cannot drift apart.
fn insert_gate_hooks(hooks: &mut Map<String, Value>, gate: &GateCommands) {
    hook_list(hooks, "PreToolUse").insert(0, hook_entry(Some("Agent|Task"), &gate.hook, true));
    for event in ["SessionStart", "SessionEnd", "Stop"] {
        hook_list(hooks, event).push(hook_entry(None, &gate.hook, false));
    }
    // D-17: the Stop guard is the LAST Stop entry, after the heartbeat. It is
    // not fail-closed: a broken guard must never wedge the driver.
    hook_list(hooks, "Stop").push(hook_entry(None, &gate.stop_guard, false));
    hook_list(hooks, "PostToolUse").push(hook_entry(Some("*"), &gate.hook, false));
}

/// Append `items` to the array at `map[key]` (created, or coerced from a non-array),
/// skipping exact duplicates.
fn append_unique(map: &mut Map<String, Value>, key: &str, items: &[Value]) {
    let slot = map.entry(key.to_owned()).or_insert_with(|| json!([]));
    if !slot.is_array() {
        *slot = json!([]);
    }
    if let Value::Array(list) = slot {
        for item in items {
            if !list.contains(item) {
                list.push(item.clone());
            }
        }
    }
}

/// True when `entry` is a well-formed hook matcher entry (30-REVIEW WR-13): an object
/// whose `matcher` is absent or a string, whose `hooks` is an array, and whose every
/// hook item is an object with a string `type`.
///
/// Per-type fields (`command`, `url`, `prompt`, `timeout`) and event names are not
/// checked: they vary by Claude Code release `[inferred — cleanup 2026-10-28]`.
fn is_well_formed_hook_entry(entry: &Value) -> bool {
    let Value::Object(entry) = entry else {
        return false;
    };
    if entry.get("matcher").is_some_and(|m| !m.is_string()) {
        return false;
    }
    let Some(Value::Array(hooks)) = entry.get("hooks") else {
        return false;
    };
    hooks
        .iter()
        .all(|hook| hook.get("type").is_some_and(Value::is_string) && hook.is_object())
}

/// The well-formed part of a host `hooks` object (30-REVIEW WR-13): each event whose
/// value is an array, holding its [`is_well_formed_hook_entry`] entries in their
/// original order. An event whose value is not an array, or that keeps no entry, is
/// omitted, so nothing that survives needs coercing `[inferred — cleanup 2026-10-28]`.
fn well_formed_hooks(events: &Map<String, Value>) -> Map<String, Value> {
    let mut kept = Map::new();
    for (event, entries) in events {
        let Value::Array(entries) = entries else {
            continue;
        };
        let good: Vec<Value> = entries
            .iter()
            .filter(|entry| is_well_formed_hook_entry(entry))
            .cloned()
            .collect();
        if !good.is_empty() {
            kept.insert(event.clone(), Value::Array(good));
        }
    }
    kept
}

/// The well-formed part of a host `permissions` object (30-REVIEW WR-13).
///
/// A [`PERMISSION_LISTS`] key keeps its string items in order when its value is an
/// array, and is omitted when no string remains or the value is not an array. Any
/// other key is kept only when its value is a string or a bool, so the documented
/// settings (`defaultMode`, `disableBypassPermissionsMode`, ...) are not hidden
/// `[inferred — cleanup 2026-10-28]`.
fn well_formed_permissions(permissions: &Map<String, Value>) -> Map<String, Value> {
    let mut kept = Map::new();
    for (key, value) in permissions {
        if PERMISSION_LISTS.contains(&key.as_str()) {
            let Value::Array(items) = value else {
                continue;
            };
            let good: Vec<Value> = items.iter().filter(|i| i.is_string()).cloned().collect();
            if !good.is_empty() {
                kept.insert(key.clone(), Value::Array(good));
            }
        } else if value.is_string() || value.is_boolean() {
            kept.insert(key.clone(), value.clone());
        }
    }
    kept
}

/// Path markers that make a command a GSD hook (D-19): the directories the GSD
/// installer puts its hook and helper scripts in.
const GSD_HOOK_MARKERS: &[&str] = &[
    ".claude/hooks/",
    ".claude/gsd-core/",
    ".claude/get-shit-done/",
    ".claude/scripts/",
];

/// Host names a kept hook may not reach: the operator's local agents and phone
/// bridges listen there (T-28.1-35).
const LOCAL_HOSTS: &[&str] = &["localhost", "127.0.0.1", "[::1]"];

/// `text` without the candidate lists of `for <var> in <words>; do` loops.
///
/// GSD wraps `node` as `$(for n in "<installer's node>" "$(command -v node)"
/// /usr/bin/node; do [ -x "$n" ] && ... && break; done)` followed by the script. The
/// first candidate is the node the installer saw, which may be gone or outside the
/// allowlist on this host; the loop then falls through to the next. A candidate that
/// is probed with `-x` cannot leak anything, so it is not a path the hook "names"
/// `[inferred — audit]`.
fn without_probe_lists(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find("for ") {
        let after_for = &rest[start + 4..];
        let list = after_for.find(" in ").and_then(|in_at| {
            let list_start = in_at + 4;
            let end = after_for[list_start..]
                .find("; do")
                .or_else(|| after_for[list_start..].find(";do"))?;
            Some((list_start, list_start + end))
        });
        match list {
            Some((from, to)) => {
                out.push_str(&rest[..start + 4 + from]);
                rest = &after_for[to..];
            }
            None => {
                out.push_str(&rest[..start + 4]);
                rest = after_for;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The words of `command` that are absolute paths under `home`, after `~`, `$HOME`
/// and `${HOME}` are expanded and quotes removed.
///
/// A word is a run of characters outside whitespace and the shell's separators, so
/// `$(for n in "/a/b" ...)` and `FOO=/a/b` both yield `/a/b`. This is a conservative
/// scanner, not a shell parser `[inferred — audit]`: a variable other than `$HOME`
/// is not expanded.
fn home_paths_named(command: &str, home: &Path) -> Vec<PathBuf> {
    let home_text = home.to_string_lossy();
    let expanded = command
        .replace("${HOME}", &home_text)
        .replace("$HOME", &home_text)
        .replace(['"', '\''], "");
    without_probe_lists(&expanded)
        .split(|c: char| c.is_whitespace() || ";|&<>()`=,{}".contains(c))
        .filter_map(|word| {
            let path = if word == "~" {
                home.to_owned()
            } else if let Some(rest) = word.strip_prefix("~/") {
                home.join(rest)
            } else if word.starts_with('/') {
                PathBuf::from(word)
            } else {
                return None;
            };
            path.starts_with(home).then_some(path)
        })
        .collect()
}

/// True when the hook `command` may reach the driver (D-19): it is a GSD or rtk
/// hook, it does not reach a local service, and every path under `home` it names
/// lies inside one of `roots` (the `$HOME` entries the sandbox re-binds, so the hook
/// would run there).
///
/// A GSD hook names a file in [`GSD_HOOK_MARKERS`]; an rtk hook has `rtk` as the
/// basename of one of its words. Anything else (the claudear hooks that reach the
/// operator's phone bridge, a user's own scripts) is dropped, in both postures
/// `[inferred — audit]`.
pub fn hook_is_kept(command: &str, home: &Path, roots: &[PathBuf]) -> bool {
    if LOCAL_HOSTS.iter().any(|host| command.contains(host)) {
        return false;
    }
    let home_text = home.to_string_lossy();
    let expanded = command
        .replace("${HOME}", &home_text)
        .replace("$HOME", &home_text)
        .replace(['"', '\''], "");
    let names_gsd = GSD_HOOK_MARKERS
        .iter()
        .any(|marker| expanded.contains(marker));
    let names_rtk = expanded.split_whitespace().any(|word| {
        Path::new(word)
            .file_name()
            .is_some_and(|name| name == "rtk")
    });
    if !(names_gsd || names_rtk) {
        return false;
    }
    home_paths_named(command, home)
        .iter()
        .all(|path| roots.iter().any(|root| path.starts_with(root)))
}

/// Drop every user hook of `hooks` that [`hook_is_kept`] refuses, in place (D-19).
///
/// Only `command` hooks can be GSD or rtk hooks, so any other type is dropped. A
/// matcher entry whose hooks were all dropped disappears, and so does an event whose
/// entries all did; entries that were empty before keep their place, and kept
/// entries keep their order.
fn filter_user_hooks(hooks: &mut Map<String, Value>, home: &Path, roots: &[PathBuf]) {
    let keep = |hook: &Value| {
        hook.get("command")
            .and_then(Value::as_str)
            .is_some_and(|command| hook_is_kept(command, home, roots))
    };
    for entries in hooks.values_mut() {
        let Value::Array(list) = entries else {
            continue;
        };
        list.retain_mut(|entry| {
            let Some(Value::Array(inner)) = entry.get_mut("hooks") else {
                return true;
            };
            let had = inner.len();
            inner.retain(&keep);
            had == 0 || !inner.is_empty()
        });
    }
    hooks.retain(|_, entries| !matches!(entries, Value::Array(list) if list.is_empty()));
}

/// The per-run settings from the host's: a pure function of the host settings
/// value and, for the degraded posture, the gate's command lines.
///
/// * a non-object input is an empty object;
/// * [`DROPPED_SETTINGS_KEYS`] are removed, and so is every `env` entry named in
///   `sandbox::is_stripped_env` (the settings `env` block
///   would otherwise re-inject what the process environment scrub removed
///   `[inferred — audit]`);
/// * `skipWebFetchPreflight` and `disableClaudeAiConnectors` are always `true`
///   (D-09a: the preflight and the connector fetch would otherwise reach hosts the
///   egress proxy refuses);
/// * wrong-shape nested `hooks` and `permissions` values (an event that is not a list,
///   a malformed matcher entry, a permission list that is not a list of strings, a
///   permission setting that is neither a string nor a bool) are dropped by
///   [`well_formed_hooks`] / [`well_formed_permissions`] before the D-19 filter and the
///   gate (WR-13);
/// * the user's hooks are filtered by [`hook_is_kept`] against `home` and `keep_roots`
///   before the gate's are added (D-19, 28.1-08);
/// * `gate = None` is the bwrap posture: the gate lives in the managed file
///   ([`managed_settings`]), so no hook is added here and the user's GSD/rtk hooks
///   keep their order. `gate = Some(..)` is the degraded posture: the gate entries
///   are added as in Phase 28 (see [`insert_gate_hooks`]) and [`DENY_TOOLS`] is
///   appended to `permissions.deny`, because no managed tier exists to carry it.
pub fn settings_transform(
    host: Value,
    gate: Option<&GateCommands>,
    home: &Path,
    keep_roots: &[PathBuf],
) -> Value {
    let mut settings = match host {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    for key in DROPPED_SETTINGS_KEYS {
        settings.remove(*key);
    }
    if let Some(Value::Object(env)) = settings.get_mut("env") {
        env.retain(|name, _| !is_stripped_env(std::ffi::OsStr::new(name)));
    }
    settings.insert("skipWebFetchPreflight".to_owned(), json!(true));
    settings.insert("disableClaudeAiConnectors".to_owned(), json!(true));
    if settings
        .get("hooks")
        .is_some_and(|hooks| !hooks.is_object())
    {
        settings.insert("hooks".to_owned(), json!({}));
    }
    // 30-REVIEW WR-13: drop wrong-shape nested hooks and permissions values before the
    // D-19 filter and the gate, so a malformed entry never sits beside the gate's own
    // (whatever the posture, one code path). A non-object `permissions` stays as is.
    if let Some(Value::Object(hooks)) = settings.get_mut("hooks") {
        *hooks = well_formed_hooks(hooks);
    }
    if let Some(Value::Object(permissions)) = settings.get_mut("permissions") {
        *permissions = well_formed_permissions(permissions);
    }
    // D-19: only GSD and rtk hooks reach the driver, whatever the posture.
    if let Some(Value::Object(hooks)) = settings.get_mut("hooks") {
        filter_user_hooks(hooks, home, keep_roots);
    }
    if let Some(gate) = gate {
        let hooks = settings
            .entry("hooks".to_owned())
            .or_insert_with(|| json!({}));
        if let Value::Object(hooks) = hooks {
            insert_gate_hooks(hooks, gate);
        }
        let permissions = settings
            .entry("permissions".to_owned())
            .or_insert_with(|| json!({}));
        if !permissions.is_object() {
            *permissions = json!({});
        }
        if let Value::Object(permissions) = permissions {
            let deny: Vec<Value> = DENY_TOOLS.iter().map(|t| json!(t)).collect();
            append_unique(permissions, "deny", &deny);
        }
    }
    Value::Object(settings)
}

/// The managed settings (D-12, D-13): the gate's hooks in the Phase 28 order, the
/// agent-tool deny list and the five locks, then the host administrator's policy
/// merged in (`host`, in precedence order: the file, then the drop-ins).
///
/// Merge rule `[inferred — audit]` (RESEARCH Pattern 6): `hooks.<event>` and
/// `permissions.<list>` arrays are appended AFTER the launcher's entries, exact
/// duplicates dropped; the five lock keys keep the launcher's value (already the
/// strictest of each); a host `hooks` or `permissions` that is not a JSON object is
/// ignored, never copied (WR-08), and inside an object only the well-formed part is
/// merged: [`well_formed_hooks`] keeps array-valued events and their well-formed
/// matcher entries, [`well_formed_permissions`] keeps string items of the four rule
/// lists plus string and bool settings such as `defaultMode` and
/// `disableBypassPermissionsMode` (WR-13); every other host key is copied as written, so a
/// host `allowManagedHooksOnly` stays in force. `allowManagedHooksOnly` is never set by
/// the launcher: it would unload the GSD/rtk user hooks (D-16).
pub fn managed_settings(gate: &GateCommands, host: &[Value]) -> Value {
    let mut hooks = Map::new();
    insert_gate_hooks(&mut hooks, gate);
    managed_settings_with(hooks, host)
}

/// [`managed_settings`] without the gate (quick 261103-f5y one-off runs): the deny
/// list, the five locks and the host policy, and no launcher hook at all.
pub fn managed_settings_without_gate(host: &[Value]) -> Value {
    managed_settings_with(Map::new(), host)
}

/// The managed settings with `hooks` as the launcher's own hook block.
fn managed_settings_with(hooks: Map<String, Value>, host: &[Value]) -> Value {
    let mut settings = Map::new();
    settings.insert("hooks".to_owned(), Value::Object(hooks));
    settings.insert("permissions".to_owned(), json!({"deny": DENY_TOOLS}));
    settings.insert("skipWebFetchPreflight".to_owned(), json!(true));
    settings.insert("disableClaudeAiConnectors".to_owned(), json!(true));
    settings.insert("enableArtifact".to_owned(), json!(false));
    settings.insert("disableRemoteControl".to_owned(), json!(true));
    settings.insert("crossSessionInbound".to_owned(), json!("refuse"));

    for doc in host {
        let Value::Object(doc) = doc else { continue };
        for (key, value) in doc {
            if LOCK_KEYS.contains(&key.as_str()) {
                continue;
            }
            match (key.as_str(), value) {
                ("hooks", Value::Object(events)) => {
                    let Some(Value::Object(ours)) = settings.get_mut("hooks") else {
                        continue;
                    };
                    // WR-13: only well-formed events reach `append_unique`; every
                    // value is a non-empty array, so nothing is coerced.
                    for (event, entries) in well_formed_hooks(events) {
                        if let Value::Array(entries) = entries {
                            append_unique(ours, &event, &entries);
                        }
                    }
                }
                ("permissions", Value::Object(lists)) => {
                    let Some(Value::Object(ours)) = settings.get_mut("permissions") else {
                        continue;
                    };
                    // IN-03, WR-13: a host list that is not an array never reaches the
                    // managed file at all, and neither does a non-string rule; the
                    // safety tier must not depend on Claude Code refusing such a file.
                    for (key, value) in well_formed_permissions(lists) {
                        match value {
                            Value::Array(items) => append_unique(ours, &key, &items),
                            // A string or bool setting: the first doc that sets it
                            // wins (I-6). It can never touch `deny`, which only
                            // arrives as an array.
                            other => {
                                ours.entry(key).or_insert(other);
                            }
                        }
                    }
                }
                // 28-REVIEW WR-08 / 29-REVIEW WR-02: a host `hooks` or `permissions`
                // that is not a JSON object (string, array, null, number, bool) is
                // ignored. It never reaches the catch-all, so it can never replace
                // the gate hooks or the deny list; the safety tier must not depend on
                // Claude Code refusing such a file (the top-level counterpart of IN-03).
                ("hooks" | "permissions", _) => {}
                _ => {
                    settings.insert(key.clone(), value.clone());
                }
            }
        }
    }
    Value::Object(settings)
}

/// The host administrator's managed settings under `dir`: `managed-settings.json`
/// first, then `managed-settings.d/*.json` sorted by file name. An absent `dir` (or
/// an absent file) is no policy, `Ok(vec![])`.
///
/// A file that is not a JSON object is `InvalidData` naming the file: Claude Code
/// itself refuses to start on such a file, so the launch is refused here, before any
/// process exists, instead of silently running without the administrator's policy
/// `[inferred — audit]`.
pub fn read_host_managed(dir: &Path) -> io::Result<Vec<Value>> {
    let mut files: Vec<PathBuf> = Vec::new();
    let main = dir.join(MANAGED_FILE);
    if main.is_file() {
        files.push(main);
    }
    if let Ok(entries) = fs::read_dir(dir.join(MANAGED_DROPIN_DIR)) {
        let mut dropins: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.extension() == Some(OsStr::new("json")) && path.is_file())
            .collect();
        dropins.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        files.extend(dropins);
    }
    let mut docs = Vec::new();
    for path in files {
        let bytes = fs::read(&path)?;
        let invalid = |why: &str| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "host managed settings {} is not a JSON object ({why}); Claude Code refuses to start on such a file, so the launch is refused",
                    path.display()
                ),
            )
        };
        match serde_json::from_slice::<Value>(&bytes) {
            Ok(doc @ Value::Object(_)) => docs.push(doc),
            Ok(_) => return Err(invalid("another JSON value")),
            Err(e) => return Err(invalid(&e.to_string())),
        }
    }
    Ok(docs)
}

/// Create and fill the per-run config directory. Idempotent: `settings.json` and
/// the shim are rewritten on every launch, the links and `.claude.json` only when
/// absent.
pub fn seed(spec: &SeedSpec) -> io::Result<()> {
    // The host policy is read first: an unparseable file refuses the launch before
    // anything is created.
    let host_managed = if spec.managed {
        read_host_managed(&spec.host_managed_dir)?
    } else {
        Vec::new()
    };

    let bin = spec.config_dir.join("bin");
    create_private_dir(&bin)?;

    for name in LINK_FROM_HOST {
        let src = spec.host_claude_dir.join(name);
        let dst = spec.config_dir.join(name);
        if src.exists() && fs::symlink_metadata(&dst).is_err() {
            std::os::unix::fs::symlink(&src, &dst)?;
        }
    }

    let shim = bin.join("driverctl");
    let gate = GateCommands {
        hook: hook_command(&shim, "hook"),
        stop_guard: hook_command(
            &shim,
            &format!(
                "stop-guard --run-dir {}",
                shell_quote(spec.run_dir.as_os_str())
            ),
        ),
    };

    let host_settings = Value::Object(read_host_settings(&spec.host_claude_dir));
    // The host `$HOME` the keep roots are relative to: `host_claude_dir` is `~/.claude`.
    let home = spec.host_claude_dir.parent().unwrap_or(Path::new("/"));
    let settings = if spec.managed {
        let managed_dir = spec.config_dir.join(MANAGED_SUBDIR);
        create_private_dir(&managed_dir)?;
        let managed = managed_settings(&gate, &host_managed);
        let rendered = serde_json::to_vec_pretty(&managed)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        write_atomic(&managed_dir.join(MANAGED_FILE), &rendered, 0o600)?;
        settings_transform(host_settings, None, home, &spec.keep_roots)
    } else {
        settings_transform(host_settings, Some(&gate), home, &spec.keep_roots)
    };
    let rendered = serde_json::to_vec_pretty(&settings)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    write_atomic(&spec.config_dir.join("settings.json"), &rendered, 0o600)?;

    let onboarding = spec.config_dir.join(".claude.json");
    if fs::symlink_metadata(&onboarding).is_err() {
        write_atomic(&onboarding, br#"{"hasCompletedOnboarding": true}"#, 0o600)?;
    }

    // The socket is pinned here, in the read-only shim: a driver that unsets or
    // rewrites `DRIVERCTL_SOCK` cannot detach a hook or a `driverctl` call from the
    // gate (T-28.1-13, RESEARCH Pattern 6 `[inferred — audit]`).
    let script = format!(
        "#!/bin/sh\nDRIVERCTL_SOCK={}; export DRIVERCTL_SOCK\nexec {} driverctl \"$@\"\n",
        shell_quote(spec.socket.as_os_str()),
        shell_quote(spec.binary.as_os_str())
    );
    write_atomic(&shim, script.as_bytes(), 0o755)?;

    // D-21: `gsd-meta-manager next <project> --json` is the read-only router hint
    // the master prompt cites. The shim puts `next` on PATH (`<cfg>/bin` leads it)
    // and refuses any other first argument, so a typo cannot run `driver launch`
    // by accident.
    //
    // **A convenience, not a security boundary** (T-29-12, accepted). `/` is
    // ro-bound inside the sandbox (`--ro-bind / /`), so the binary's absolute path
    // is executable whether or not this shim exists, and a driver that wants
    // another verb can call it directly. The boundary is the sandbox itself: a
    // read-only run directory, no host secrets, and pushes only from the host.
    let next_shim = bin.join("gsd-meta-manager");
    let next_script = format!(
        "#!/bin/sh\ncase \"$1\" in\n  next) shift; exec {} next \"$@\" ;;\n  *) echo \"gsd-meta-manager: only 'next' is available inside the driver sandbox\" >&2; exit 2 ;;\nesac\n",
        shell_quote(spec.binary.as_os_str())
    );
    write_atomic(&next_shim, next_script.as_bytes(), 0o755)?;
    Ok(())
}

/// What [`seed_oneoff`] needs to know (quick 261103-f5y).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OneoffSeedSpec {
    /// The per-run config directory to create and fill.
    pub config_dir: PathBuf,
    /// The host's agent config dir: read for the links and the settings, never written.
    pub host_claude_dir: PathBuf,
    /// The binary the `bin/gsd-meta-manager` shim execs.
    pub binary: PathBuf,
    /// The run's report directory, pinned in the shim as `GSDMM_ONEOFF_REPORT_DIR`.
    pub report_dir: PathBuf,
    /// True for a bwrap-scope run: the deny list goes to `<cfg>/managed`.
    pub managed: bool,
    /// The host's managed-settings directory, merged into the per-run managed file.
    pub host_managed_dir: PathBuf,
    /// The `$HOME` entries the sandbox re-binds; empty keeps no user hook.
    pub keep_roots: Vec<PathBuf>,
}

/// The environment variable the one-off shim pins (`crate::oneoff::REPORT_DIR_ENV`).
const ONEOFF_REPORT_DIR_ENV: &str = "GSDMM_ONEOFF_REPORT_DIR";

/// Create and fill a one-off run's config directory (quick 261103-f5y): the host
/// links, the transformed settings and the onboarding file as [`seed`] writes them,
/// the managed deny list and locks with **no gate hook, no Stop-guard and no
/// `bin/driverctl`**, and a `bin/gsd-meta-manager` shim that runs only `taskctl`,
/// with the report directory pinned.
pub fn seed_oneoff(spec: &OneoffSeedSpec) -> io::Result<()> {
    let host_managed = if spec.managed {
        read_host_managed(&spec.host_managed_dir)?
    } else {
        Vec::new()
    };

    let bin = spec.config_dir.join("bin");
    create_private_dir(&bin)?;

    for name in LINK_FROM_HOST {
        let src = spec.host_claude_dir.join(name);
        let dst = spec.config_dir.join(name);
        if src.exists() && fs::symlink_metadata(&dst).is_err() {
            std::os::unix::fs::symlink(&src, &dst)?;
        }
    }

    let host_settings = Value::Object(read_host_settings(&spec.host_claude_dir));
    let home = spec.host_claude_dir.parent().unwrap_or(Path::new("/"));
    let mut settings = settings_transform(host_settings, None, home, &spec.keep_roots);
    if spec.managed {
        let managed_dir = spec.config_dir.join(MANAGED_SUBDIR);
        create_private_dir(&managed_dir)?;
        let managed = managed_settings_without_gate(&host_managed);
        let rendered = serde_json::to_vec_pretty(&managed)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        write_atomic(&managed_dir.join(MANAGED_FILE), &rendered, 0o600)?;
    } else if let Value::Object(map) = &mut settings {
        // Degraded: no managed tier exists to carry the deny list.
        let permissions = map
            .entry("permissions".to_owned())
            .or_insert_with(|| json!({}));
        if !permissions.is_object() {
            *permissions = json!({});
        }
        if let Value::Object(permissions) = permissions {
            let deny: Vec<Value> = DENY_TOOLS.iter().map(|t| json!(t)).collect();
            append_unique(permissions, "deny", &deny);
        }
    }
    let rendered = serde_json::to_vec_pretty(&settings)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    write_atomic(&spec.config_dir.join("settings.json"), &rendered, 0o600)?;

    let onboarding = spec.config_dir.join(".claude.json");
    if fs::symlink_metadata(&onboarding).is_err() {
        write_atomic(&onboarding, br#"{"hasCompletedOnboarding": true}"#, 0o600)?;
    }

    // The report dir is pinned in the read-only shim; like the driver's `next` shim it
    // is a convenience, not a boundary (the sandbox's writable binds are).
    let shim = bin.join("gsd-meta-manager");
    let script = format!(
        "#!/bin/sh\ncase \"$1\" in\n  taskctl) {ONEOFF_REPORT_DIR_ENV}={}; export {ONEOFF_REPORT_DIR_ENV}; exec {} \"$@\" ;;\n  *) echo \"gsd-meta-manager: only 'taskctl' is available inside the one-off sandbox\" >&2; exit 2 ;;\nesac\n",
        shell_quote(spec.report_dir.as_os_str()),
        shell_quote(spec.binary.as_os_str())
    );
    write_atomic(&shim, script.as_bytes(), 0o755)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn oneoff_spec(root: &TempDir, managed: bool) -> OneoffSeedSpec {
        OneoffSeedSpec {
            config_dir: root.path().join("cfg"),
            host_claude_dir: root.path().join("host"),
            binary: PathBuf::from("/opt/gsd mm/gsd-meta-manager"),
            report_dir: root.path().join("run/report"),
            managed,
            host_managed_dir: root.path().join("no-host-managed"),
            keep_roots: Vec::new(),
        }
    }

    #[test]
    fn seed_oneoff_managed_writes_the_links_a_gate_free_managed_file_and_a_taskctl_shim() {
        let root = TempDir::new().unwrap();
        let host = root.path().join("host");
        fs::create_dir_all(host.join("skills")).unwrap();
        fs::create_dir_all(host.join("gsd-core")).unwrap();
        let spec = oneoff_spec(&root, true);
        seed_oneoff(&spec).unwrap();
        let cfg = &spec.config_dir;
        assert!(fs::symlink_metadata(cfg.join("skills"))
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(fs::symlink_metadata(cfg.join("gsd-core"))
            .unwrap()
            .file_type()
            .is_symlink());
        let settings = fs::read_to_string(cfg.join("settings.json")).unwrap();
        assert!(!settings.contains("driverctl"), "{settings}");
        let managed: Value =
            serde_json::from_slice(&fs::read(cfg.join(MANAGED_SUBDIR).join(MANAGED_FILE)).unwrap())
                .unwrap();
        assert_eq!(managed["hooks"], json!({}));
        assert_eq!(managed["permissions"]["deny"], json!(DENY_TOOLS));
        for key in LOCK_KEYS {
            assert!(managed.get(*key).is_some(), "lock {key}");
        }
        assert!(!managed.to_string().contains("driverctl"));
        assert!(fs::symlink_metadata(cfg.join("bin/driverctl")).is_err());
        let shim = fs::read_to_string(cfg.join("bin/gsd-meta-manager")).unwrap();
        assert!(shim.contains("taskctl)"), "{shim}");
        assert!(shim.contains(&format!(
            "GSDMM_ONEOFF_REPORT_DIR='{}'",
            spec.report_dir.display()
        )));
        assert!(
            shim.contains("exec '/opt/gsd mm/gsd-meta-manager' \"$@\""),
            "{shim}"
        );
        assert!(shim.contains("exit 2"));
        assert!(!shim.contains("next"));
    }

    #[test]
    fn seed_oneoff_shim_runs_taskctl_only() {
        let root = TempDir::new().unwrap();
        let spec = oneoff_spec(&root, false);
        seed_oneoff(&spec).unwrap();
        let shim = spec.config_dir.join("bin/gsd-meta-manager");
        let script = fs::read_to_string(&shim).unwrap();
        // Parsed rather than executed (spawning is reserved to the launcher's process
        // seam): the case arms are exactly `taskctl)` and `*)`.
        let arms: Vec<&str> = script
            .lines()
            .map(str::trim_start)
            .filter(|l| l.contains(") "))
            .map(|l| l.split(')').next().unwrap())
            .collect();
        assert_eq!(arms, ["taskctl", "*"]);
    }

    #[test]
    fn seed_oneoff_degraded_puts_the_deny_list_in_settings_and_writes_no_managed_dir() {
        let root = TempDir::new().unwrap();
        let spec = oneoff_spec(&root, false);
        seed_oneoff(&spec).unwrap();
        let settings: Value =
            serde_json::from_slice(&fs::read(spec.config_dir.join("settings.json")).unwrap())
                .unwrap();
        let deny = settings["permissions"]["deny"].as_array().unwrap();
        for tool in DENY_TOOLS {
            assert!(deny.contains(&json!(tool)), "{tool}");
        }
        assert!(settings.get("hooks").is_none_or(|h| h == &json!({})));
        assert!(!settings.to_string().contains("driverctl"));
        assert!(fs::symlink_metadata(spec.config_dir.join(MANAGED_SUBDIR)).is_err());
        assert!(fs::symlink_metadata(spec.config_dir.join("bin/driverctl")).is_err());
    }

    fn spec(root: &TempDir) -> SeedSpec {
        SeedSpec {
            config_dir: root.path().join("cfg"),
            host_claude_dir: root.path().join("host"),
            binary: PathBuf::from("/opt/gsd mm/gsd-meta-manager"),
            run_dir: root.path().join("run"),
            socket: PathBuf::from("/run/user/1000/gsdmm-driverctl/r1-ab12cd34.sock"),
            managed: false,
            host_managed_dir: root.path().join("no-host-managed"),
            keep_roots: Vec::new(),
        }
    }

    fn settings_of(spec: &SeedSpec) -> Value {
        serde_json::from_slice(&fs::read(spec.config_dir.join("settings.json")).unwrap()).unwrap()
    }

    #[test]
    fn hook_command_single_quotes_the_shim_path() {
        assert_eq!(
            hook_command(Path::new("/a b/it's/driverctl"), "hook"),
            r"'/a b/it'\''s/driverctl' hook"
        );
    }

    #[test]
    fn run_state_dir_is_root_hash_and_run_id() {
        let dir = run_state_dir_in(Path::new("/r"), Path::new("/proj"), "r1");
        assert_eq!(
            dir,
            Path::new("/r").join(hash8(Path::new("/proj"))).join("r1")
        );
    }

    #[test]
    fn a_binary_under_the_project_is_copied_out_and_any_other_is_left_alone() {
        // WR-05
        let root = TempDir::new().unwrap();
        let base = fs::canonicalize(root.path()).unwrap();
        let project = base.join("proj");
        let target = project.join("target/debug");
        fs::create_dir_all(&target).unwrap();
        let exe = target.join("gsd-meta-manager");
        fs::write(&exe, b"#!/bin/sh\necho built\n").unwrap();
        fs::set_permissions(&exe, fs::Permissions::from_mode(0o755)).unwrap();
        let cfg = base.join("state/claude-config");
        let snap = snapshot_exe(&exe, &project, &cfg).unwrap();
        assert_eq!(snap, cfg.join("bin").join(EXE_SNAPSHOT_NAME));
        assert!(!snap.starts_with(&project));
        assert_eq!(fs::read(&snap).unwrap(), fs::read(&exe).unwrap());
        assert_eq!(
            fs::metadata(&snap).unwrap().permissions().mode() & 0o777,
            0o755
        );
        // The project's own copy is writable and replaceable (cargo build), the
        // snapshot does not follow it, and a second snapshot replaces the first.
        fs::write(&exe, b"rebuilt").unwrap();
        assert_ne!(fs::read(&snap).unwrap(), b"rebuilt");
        assert_eq!(snapshot_exe(&exe, &project, &cfg).unwrap(), snap);
        assert_eq!(fs::read(&snap).unwrap(), b"rebuilt");
        // A symlink outside the project that points into it is also copied.
        let link = base.join("link");
        std::os::unix::fs::symlink(&exe, &link).unwrap();
        assert_eq!(snapshot_exe(&link, &project, &cfg).unwrap(), snap);
        // A binary outside the project is returned unchanged and nothing is copied.
        let outside = base.join("installed/gsd-meta-manager");
        fs::create_dir_all(outside.parent().unwrap()).unwrap();
        fs::write(&outside, b"x").unwrap();
        let cfg2 = base.join("state2/claude-config");
        assert_eq!(snapshot_exe(&outside, &project, &cfg2).unwrap(), outside);
        assert!(!cfg2.exists());
    }

    #[test]
    fn a_deleted_suffix_is_stripped_when_the_file_exists_and_refused_otherwise() {
        // IN-05
        let root = TempDir::new().unwrap();
        let exe = root.path().join("gsd-meta-manager");
        fs::write(&exe, b"x").unwrap();
        let reported = PathBuf::from(format!("{} (deleted)", exe.display()));
        assert_eq!(resolve_exe_path(reported).unwrap(), exe);
        assert_eq!(resolve_exe_path(exe.clone()).unwrap(), exe);
        fs::remove_file(&exe).unwrap();
        let gone = PathBuf::from(format!("{} (deleted)", exe.display()));
        let err = resolve_exe_path(gone).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(err.to_string().contains("no longer exists"), "{err}");
    }

    #[test]
    fn a_non_array_host_deny_list_does_not_replace_the_launchers() {
        // IN-03
        let gate = GateCommands {
            hook: "h".to_owned(),
            stop_guard: "s".to_owned(),
        };
        let host = vec![json!({"permissions": {"deny": "x", "ask": "y"}})];
        let managed = managed_settings(&gate, &host);
        let deny = managed["permissions"]["deny"].as_array().expect("array");
        assert_eq!(deny.len(), DENY_TOOLS.len());
        assert!(deny.iter().any(|v| v == "RemoteTrigger"));
        // 30-REVIEW WR-13: a non-array host list is dropped even when the launcher
        // does not hold it, so it never reaches the managed file.
        assert!(managed["permissions"].get("ask").is_none());
    }

    /// Every non-object JSON shape. I-2: the bool and the non-empty array complete the
    /// string, empty array, null and number the finding names.
    fn non_object_shapes() -> Vec<Value> {
        vec![
            json!("x"),
            json!([]),
            json!([{"matcher": "Bash", "hooks": [{"type": "command", "command": "admin-audit"}]}]),
            json!(null),
            json!(7),
            json!(true),
        ]
    }

    #[test]
    fn a_non_object_host_hooks_or_permissions_never_replaces_the_managed_gate() {
        // 28-REVIEW WR-08 / 29-REVIEW WR-02
        let gate = GateCommands {
            hook: "h".to_owned(),
            stop_guard: "s".to_owned(),
        };
        let baseline = managed_settings(&gate, &[]);
        for key in ["hooks", "permissions"] {
            for shape in non_object_shapes() {
                let mut doc = Map::new();
                doc.insert(key.to_owned(), shape.clone());
                assert_eq!(
                    managed_settings(&gate, &[Value::Object(doc)]),
                    baseline,
                    "host {key} = {shape} changed the managed settings"
                );
            }
        }

        // A malformed value does not stop the rest of the merge, nor a later doc.
        let admin_hook =
            json!({"matcher": "Bash", "hooks": [{"type": "command", "command": "admin-audit"}]});
        let host = vec![
            json!({"hooks": "x", "permissions": null, "model": "admin"}),
            json!({"permissions": {"deny": ["Edit"]}, "hooks": {"PreToolUse": [admin_hook]}}),
        ];
        let managed = managed_settings(&gate, &host);
        assert_eq!(managed["model"], "admin");
        let mut want_deny: Vec<Value> = DENY_TOOLS.iter().map(|t| json!(t)).collect();
        want_deny.push(json!("Edit"));
        assert_eq!(managed["permissions"]["deny"], Value::Array(want_deny));
        let gate_entry = baseline["hooks"]["PreToolUse"][0].clone();
        assert_eq!(
            managed["hooks"]["PreToolUse"],
            json!([gate_entry, admin_hook])
        );
    }

    #[test]
    fn a_non_object_host_hooks_or_permissions_never_replaces_the_degraded_gate() {
        // 28-REVIEW WR-08 / 29-REVIEW WR-02; I-3: this already held at HEAD and is
        // now pinned.
        let gate = GateCommands {
            hook: "h".to_owned(),
            stop_guard: "s".to_owned(),
        };
        let home = Path::new("/home/u");
        let baseline = settings_transform(json!({}), Some(&gate), home, &[]);
        for key in ["hooks", "permissions"] {
            for shape in non_object_shapes() {
                let mut doc = Map::new();
                doc.insert(key.to_owned(), shape.clone());
                assert_eq!(
                    settings_transform(Value::Object(doc), Some(&gate), home, &[]),
                    baseline,
                    "host {key} = {shape} changed the degraded settings"
                );
            }
        }
    }

    /// Every wrong-shape nested host doc (30-REVIEW WR-13). Both posture tests loop
    /// over this one list, so the postures cannot drift (I-4).
    fn wrong_shape_nested_host_docs() -> Vec<Value> {
        let mut docs = Vec::new();
        for event in ["PreToolUse", "Stop", "Notification"] {
            for wrong in [
                json!("x"),
                json!({"matcher": "Bash", "hooks": []}),
                json!(null),
                json!(7),
                json!(true),
            ] {
                docs.push(json!({"hooks": {event: wrong}}));
            }
        }
        for event in ["PreToolUse", "Notification"] {
            for entry in [
                json!("x"),
                json!(7),
                json!(null),
                json!([]),
                json!({"matcher": "Bash"}),
                json!({"hooks": "x"}),
                json!({"hooks": {}}),
                json!({"hooks": [7]}),
                json!({"hooks": [{"command": "x"}]}),
                json!({"hooks": [{"type": 7, "command": "x"}]}),
                json!({"matcher": 7, "hooks": []}),
            ] {
                docs.push(json!({"hooks": {event: [entry]}}));
            }
        }
        for list in PERMISSION_LISTS {
            for wrong in [
                json!("x"),
                json!({"x": 1}),
                json!(null),
                json!(7),
                json!(true),
            ] {
                docs.push(json!({"permissions": {*list: wrong}}));
            }
            docs.push(json!({"permissions": {*list: [7, null, true, {}, ["Edit"]]}}));
        }
        for key in ["defaultMode", "disableBypassPermissionsMode", "x"] {
            for wrong in [json!([]), json!({}), json!(null), json!(7)] {
                docs.push(json!({"permissions": {key: wrong}}));
            }
        }
        docs
    }

    fn test_gate() -> GateCommands {
        GateCommands {
            hook: "h".to_owned(),
            stop_guard: "s".to_owned(),
        }
    }

    #[test]
    fn a_wrong_shape_nested_host_value_never_reaches_the_managed_file() {
        // 30-REVIEW WR-13; I-1, I-2, I-3
        let gate = test_gate();
        let baseline = managed_settings(&gate, &[]);
        for doc in wrong_shape_nested_host_docs() {
            assert_eq!(
                managed_settings(&gate, std::slice::from_ref(&doc)),
                baseline,
                "host doc {doc} changed the managed settings"
            );
        }
    }

    #[test]
    fn a_wrong_shape_nested_host_value_never_reaches_the_degraded_settings() {
        // 30-REVIEW WR-13; I-4
        let gate = test_gate();
        let home = Path::new("/home/u");
        let baseline = settings_transform(json!({}), Some(&gate), home, &[]);
        for doc in wrong_shape_nested_host_docs() {
            assert_eq!(
                settings_transform(doc.clone(), Some(&gate), home, &[]),
                baseline,
                "host doc {doc} changed the degraded settings"
            );
        }
    }

    #[test]
    fn well_formed_host_values_still_merge_beside_wrong_shape_ones() {
        // 30-REVIEW WR-13; I-2, I-6; both postures
        let gate = test_gate();
        let home = Path::new("/home/u");
        let h = json!({"matcher": "Bash", "hooks": [{"type": "command", "command": "rtk hook claude"}]});
        let host = json!({
            "hooks": {"PreToolUse": ["x", h, {"hooks": "x"}]},
            "permissions": {
                "deny": [7, "Edit"],
                "allow": "x",
                "defaultMode": "default",
                "disableBypassPermissionsMode": "disable",
                "blockReadsOutsideWorkingDirectories": true,
                "x": 7
            }
        });

        let managed = managed_settings(&gate, std::slice::from_ref(&host));
        let base_managed = managed_settings(&gate, &[]);
        assert_eq!(
            managed["hooks"]["PreToolUse"],
            json!([base_managed["hooks"]["PreToolUse"][0].clone(), h])
        );
        let mut want_deny: Vec<Value> = DENY_TOOLS.iter().map(|t| json!(t)).collect();
        want_deny.push(json!("Edit"));
        assert_eq!(managed["permissions"]["deny"], Value::Array(want_deny));

        let degraded = settings_transform(host, Some(&gate), home, &[]);
        let base_degraded = settings_transform(json!({}), Some(&gate), home, &[]);
        assert_eq!(
            degraded["hooks"]["PreToolUse"],
            json!([base_degraded["hooks"]["PreToolUse"][0].clone(), h])
        );
        let mut want_deny = vec![json!("Edit")];
        want_deny.extend(DENY_TOOLS.iter().map(|t| json!(t)));
        assert_eq!(degraded["permissions"]["deny"], Value::Array(want_deny));

        for out in [&managed, &degraded] {
            assert!(out["permissions"].get("allow").is_none());
            assert!(out["permissions"].get("x").is_none());
            assert_eq!(out["permissions"]["defaultMode"], "default");
            assert_eq!(
                out["permissions"]["disableBypassPermissionsMode"],
                "disable"
            );
            assert_eq!(
                out["permissions"]["blockReadsOutsideWorkingDirectories"],
                true
            );
            let hooks = out["hooks"].as_object().unwrap();
            assert_eq!(&well_formed_hooks(hooks), hooks);
            let permissions = out["permissions"].as_object().unwrap();
            assert_eq!(&well_formed_permissions(permissions), permissions);
        }
    }

    #[test]
    fn seed_without_a_host_dir_still_seeds_the_gate() {
        let root = TempDir::new().unwrap();
        let spec = spec(&root);
        seed(&spec).unwrap();
        let settings = settings_of(&spec);
        let pre = &settings["hooks"]["PreToolUse"][0];
        assert_eq!(pre["matcher"], "Agent|Task");
        assert_eq!(pre["hooks"][0]["onFailure"], "block");
        assert_eq!(pre["hooks"][0]["timeout"], 20);
        let shim = spec.config_dir.join("bin/driverctl");
        assert_eq!(
            pre["hooks"][0]["command"],
            format!("'{}' hook", shim.display())
        );
        for event in ["SessionStart", "SessionEnd", "Stop"] {
            let entry = &settings["hooks"][event][0];
            assert!(entry.get("matcher").is_none(), "{event}");
            assert!(entry["hooks"][0].get("onFailure").is_none(), "{event}");
        }
        assert_eq!(settings["hooks"]["PostToolUse"][0]["matcher"], "*");
        let onboarding = fs::read_to_string(spec.config_dir.join(".claude.json")).unwrap();
        assert!(onboarding.contains("hasCompletedOnboarding"));
    }

    #[test]
    fn seed_keeps_host_hooks_after_the_gate_and_drops_the_dropped_keys() {
        let root = TempDir::new().unwrap();
        let spec = spec(&root);
        fs::create_dir_all(spec.host_claude_dir.join("agents")).unwrap();
        fs::write(
            spec.host_claude_dir.join("settings.json"),
            json!({
                "statusLine": {"type": "command"},
                "enabledPlugins": {"x": true},
                "remoteControlAtStartup": true,
                "agentPushNotifEnabled": true,
                "model": "opus",
                "hooks": {"PreToolUse": [{"matcher": "Bash", "hooks": []}]}
            })
            .to_string(),
        )
        .unwrap();
        seed(&spec).unwrap();
        let settings = settings_of(&spec);
        for key in DROPPED_SETTINGS_KEYS {
            assert!(settings.get(*key).is_none(), "{key}");
        }
        assert_eq!(settings["model"], "opus");
        assert_eq!(settings["hooks"]["PreToolUse"][0]["matcher"], "Agent|Task");
        assert_eq!(settings["hooks"]["PreToolUse"][1]["matcher"], "Bash");
        assert!(fs::symlink_metadata(spec.config_dir.join("agents"))
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(fs::symlink_metadata(spec.config_dir.join("skills")).is_err());
    }

    #[test]
    fn seed_is_idempotent_and_does_not_stack_hooks() {
        let root = TempDir::new().unwrap();
        let spec = spec(&root);
        seed(&spec).unwrap();
        fs::write(spec.config_dir.join(".claude.json"), "{\"mine\": 1}").unwrap();
        seed(&spec).unwrap();
        let settings = settings_of(&spec);
        assert_eq!(settings["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
        // The heartbeat and the Stop guard (28-06): two entries, however many seeds.
        let stop = settings["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop.len(), 2);
        assert!(stop[1]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .contains("stop-guard --run-dir"));
        let onboarding = fs::read_to_string(spec.config_dir.join(".claude.json")).unwrap();
        assert!(
            onboarding.contains("mine"),
            "an existing .claude.json is kept"
        );
    }

    #[test]
    fn the_shim_execs_the_binary_with_driverctl() {
        let root = TempDir::new().unwrap();
        let spec = spec(&root);
        seed(&spec).unwrap();
        let shim = spec.config_dir.join("bin/driverctl");
        assert_eq!(
            fs::read_to_string(&shim).unwrap(),
            "#!/bin/sh\nDRIVERCTL_SOCK='/run/user/1000/gsdmm-driverctl/r1-ab12cd34.sock'; export DRIVERCTL_SOCK\nexec '/opt/gsd mm/gsd-meta-manager' driverctl \"$@\"\n"
        );
        let mode = fs::metadata(&shim).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o755);
        let dir_mode = fs::metadata(&spec.config_dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(dir_mode, 0o700);
    }

    const NEXT_SHIM: &str = "#!/bin/sh\ncase \"$1\" in\n  next) shift; exec '/opt/gsd mm/gsd-meta-manager' next \"$@\" ;;\n  *) echo \"gsd-meta-manager: only 'next' is available inside the driver sandbox\" >&2; exit 2 ;;\nesac\n";

    #[test]
    fn the_gsd_meta_manager_shim_execs_next_and_refuses_everything_else() {
        let root = TempDir::new().unwrap();
        let spec = spec(&root);
        seed(&spec).unwrap();
        let shim = spec.config_dir.join("bin/gsd-meta-manager");
        let text = fs::read_to_string(&shim).unwrap();
        assert_eq!(text, NEXT_SHIM);
        assert!(text.contains("case \"$1\" in"), "{text}");
        assert!(
            text.contains("only 'next' is available inside the driver sandbox"),
            "{text}"
        );
        let mode = fs::metadata(&shim).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o755);
    }

    #[test]
    fn both_shims_are_byte_identical_after_a_second_seed() {
        let root = TempDir::new().unwrap();
        let spec = spec(&root);
        seed(&spec).unwrap();
        let read = |name: &str| fs::read(spec.config_dir.join("bin").join(name)).unwrap();
        let (first_driverctl, first_next) = (read("driverctl"), read("gsd-meta-manager"));
        seed(&spec).unwrap();
        assert_eq!(first_driverctl, read("driverctl"));
        assert_eq!(first_next, read("gsd-meta-manager"));
    }
}
