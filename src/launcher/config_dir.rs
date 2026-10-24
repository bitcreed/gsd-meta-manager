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
//!   `apiKeyHelper`, minus any `env` entry the process scrub strips. The PreToolUse
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
//!   `/etc/claude-code` policy is merged in, never hidden ([`read_host_managed`]);
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

/// Managed-settings keys the launcher pins; a host value never replaces them. Each
/// is a lock whose launcher value is already the strictest one.
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

/// STUB (28.1-08 RED).
pub fn hook_is_kept(command: &str, home: &Path, roots: &[PathBuf]) -> bool {
    let _ = (command, home, roots);
    true
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
    let _ = (home, keep_roots);
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
/// strictest of each); every other host key is copied as written, so a host
/// `allowManagedHooksOnly` stays in force. `allowManagedHooksOnly` is never set by
/// the launcher: it would unload the GSD/rtk user hooks (D-16).
pub fn managed_settings(gate: &GateCommands, host: &[Value]) -> Value {
    let mut hooks = Map::new();
    insert_gate_hooks(&mut hooks, gate);
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
                    for (event, entries) in events {
                        match entries {
                            Value::Array(entries) => append_unique(ours, event, entries),
                            other => {
                                ours.entry(event.clone()).or_insert_with(|| other.clone());
                            }
                        }
                    }
                }
                ("permissions", Value::Object(lists)) => {
                    let Some(Value::Object(ours)) = settings.get_mut("permissions") else {
                        continue;
                    };
                    for (list, entries) in lists {
                        match entries {
                            Value::Array(entries) => append_unique(ours, list, entries),
                            other => {
                                ours.insert(list.clone(), other.clone());
                            }
                        }
                    }
                }
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
