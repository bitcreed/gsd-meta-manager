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
//!   `apiKeyHelper`, minus any `env` entry the process scrub strips, with the
//!   driverctl hooks added around the user's own. The PreToolUse `Agent|Task` entry goes FIRST and carries
//!   `"onFailure": "block"` (hand-off 1), so a dead socket or a missing binary
//!   fails closed;
//! * a minimal `.claude.json` (`hasCompletedOnboarding`), only when absent;
//! * a `bin/driverctl` shim that execs this binary (hand-off 8), so the master
//!   prompt's `driverctl log ...` runs unchanged.
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

use super::sandbox::{EXTRA_STRIP_ENV, STRIP_ENV};

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
    /// The run directory (the Stop-guard entry of 28-06 names it).
    pub run_dir: PathBuf,
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
fn write_atomic(path: &Path, contents: &[u8], mode: u32) -> io::Result<()> {
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

/// The per-run settings from the host's: a pure function of the host settings
/// value and the quoted `<cfg>/bin/driverctl hook` command line.
///
/// * a non-object input is an empty object;
/// * [`DROPPED_SETTINGS_KEYS`] are removed, and so is every `env` entry named in
///   `sandbox::STRIP_ENV` or `sandbox::EXTRA_STRIP_ENV` (the settings `env` block
///   would otherwise re-inject what the process environment scrub removed
///   `[inferred — audit]`);
/// * the dispatch gate (`PreToolUse`, `Agent|Task`, `onFailure: block`) goes
///   FIRST, so it runs before any host `PreToolUse` hook; the `SessionStart`,
///   `SessionEnd`, `Stop` and `PostToolUse` entries go last. User entries (GSD,
///   rtk) keep their relative order.
pub fn settings_transform(host: Value, hook_command: &str) -> Value {
    let mut settings = match host {
        Value::Object(map) => map,
        _ => Map::new(),
    };
    for key in DROPPED_SETTINGS_KEYS {
        settings.remove(*key);
    }
    if let Some(Value::Object(env)) = settings.get_mut("env") {
        env.retain(|name, _| {
            !STRIP_ENV
                .iter()
                .chain(EXTRA_STRIP_ENV)
                .any(|stripped| name == stripped)
        });
    }
    let hooks = settings
        .entry("hooks".to_owned())
        .or_insert_with(|| json!({}));
    if !hooks.is_object() {
        *hooks = json!({});
    }
    if let Value::Object(hooks) = hooks {
        hook_list(hooks, "PreToolUse")
            .insert(0, hook_entry(Some("Agent|Task"), hook_command, true));
        for event in ["SessionStart", "SessionEnd", "Stop"] {
            hook_list(hooks, event).push(hook_entry(None, hook_command, false));
        }
        hook_list(hooks, "PostToolUse").push(hook_entry(Some("*"), hook_command, false));
    }
    Value::Object(settings)
}

/// Create and fill the per-run config directory. Idempotent: `settings.json` and
/// the shim are rewritten on every launch, the links and `.claude.json` only when
/// absent.
pub fn seed(spec: &SeedSpec) -> io::Result<()> {
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
    let command = hook_command(&shim, "hook");

    let settings = settings_transform(
        Value::Object(read_host_settings(&spec.host_claude_dir)),
        &command,
    );
    let rendered = serde_json::to_vec_pretty(&settings)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    write_atomic(&spec.config_dir.join("settings.json"), &rendered, 0o600)?;

    let onboarding = spec.config_dir.join(".claude.json");
    if fs::symlink_metadata(&onboarding).is_err() {
        write_atomic(&onboarding, br#"{"hasCompletedOnboarding": true}"#, 0o600)?;
    }

    let script = format!(
        "#!/bin/sh\nexec {} driverctl \"$@\"\n",
        shell_quote(spec.binary.as_os_str())
    );
    write_atomic(&shim, script.as_bytes(), 0o755)?;
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
        assert_eq!(settings["hooks"]["Stop"].as_array().unwrap().len(), 1);
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
            "#!/bin/sh\nexec '/opt/gsd mm/gsd-meta-manager' driverctl \"$@\"\n"
        );
        let mode = fs::metadata(&shim).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o755);
        let dir_mode = fs::metadata(&spec.config_dir).unwrap().permissions().mode() & 0o777;
        assert_eq!(dir_mode, 0o700);
    }
}
