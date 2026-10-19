#![cfg(target_os = "linux")]
// ============================================================================
// The per-run CLAUDE_CONFIG_DIR (Phase 28, plan 28-03; D-18, D-22, RESEARCH §3):
// the settings transform, the host links, the non-intrusive guarantee and the
// no-refresh-token invariant. Fixture HOME / XDG dirs are TempDirs.
// ============================================================================

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use chrono::Utc;
use gsd_meta_manager::launcher::auth::{resolve_launch_token, TokenSource};
use gsd_meta_manager::launcher::config_dir::{
    hook_command, run_state_dir_in, seed, settings_transform, SeedSpec, DROPPED_SETTINGS_KEYS,
    LINK_FROM_HOST,
};
use gsd_meta_manager::launcher::sandbox::child_env;
use serde_json::{json, Value};
use tempfile::TempDir;

const HOOK: &str = "'/cfg/bin/driverctl' hook";

/// Serialises the one test that points `HOME` / `XDG_CONFIG_HOME` at a fixture.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn command_of(entry: &Value) -> &str {
    entry["hooks"][0]["command"].as_str().unwrap_or_default()
}

fn host_fixture() -> Value {
    json!({
        "model": "opus",
        "statusLine": {"type": "command", "command": "x"},
        "enabledPlugins": {"p@m": true},
        "remoteControlAtStartup": true,
        "agentPushNotifEnabled": true,
        "apiKeyHelper": "/usr/bin/print-key",
        "env": {"ANTHROPIC_API_KEY": "k", "GH_TOKEN": "g", "FOO": "bar"},
        "hooks": {
            "PreToolUse": [
                {"matcher": "Bash", "hooks": [{"type": "command", "command": "gsd-guard"}]},
                {"matcher": "Bash", "hooks": [{"type": "command", "command": "rtk-rewrite"}]}
            ],
            "Stop": [{"hooks": [{"type": "command", "command": "user-stop"}]}]
        }
    })
}

#[test]
fn settings_transform_keeps_user_hooks_in_order_and_drops_the_auth_keys() {
    let out = settings_transform(host_fixture(), HOOK);

    for key in [
        "statusLine",
        "enabledPlugins",
        "remoteControlAtStartup",
        "agentPushNotifEnabled",
        "apiKeyHelper",
    ] {
        assert!(out.get(key).is_none(), "{key} must be dropped");
        assert!(DROPPED_SETTINGS_KEYS.contains(&key), "{key} is listed");
    }
    assert_eq!(out["env"], json!({"FOO": "bar"}));
    assert_eq!(out["model"], "opus");

    let pre = out["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(pre.len(), 3);
    assert_eq!(pre[0]["matcher"], "Agent|Task");
    assert_eq!(pre[0]["hooks"][0]["onFailure"], "block");
    assert_eq!(command_of(&pre[0]), HOOK);
    assert_eq!(command_of(&pre[1]), "gsd-guard");
    assert_eq!(command_of(&pre[2]), "rtk-rewrite");

    let stop = out["hooks"]["Stop"].as_array().unwrap();
    assert_eq!(stop.len(), 2);
    assert_eq!(command_of(&stop[0]), "user-stop");
    assert_eq!(command_of(&stop[1]), HOOK);
    for event in ["SessionStart", "SessionEnd"] {
        let list = out["hooks"][event].as_array().unwrap();
        assert_eq!(list.len(), 1, "{event}");
        assert_eq!(command_of(&list[0]), HOOK);
    }
    let post = out["hooks"]["PostToolUse"].as_array().unwrap();
    assert_eq!(post.len(), 1);
    assert_eq!(post[0]["matcher"], "*");
}

#[test]
fn settings_transform_of_nothing_is_exactly_the_five_driverctl_entries() {
    for host in [json!({}), json!([1, 2]), json!("text"), Value::Null] {
        let out = settings_transform(host.clone(), HOOK);
        let hooks = out["hooks"].as_object().expect("a hooks object");
        let mut events: Vec<&str> = hooks.keys().map(String::as_str).collect();
        events.sort_unstable();
        assert_eq!(
            events,
            [
                "PostToolUse",
                "PreToolUse",
                "SessionEnd",
                "SessionStart",
                "Stop"
            ],
            "{host}"
        );
        for list in hooks.values() {
            let list = list.as_array().unwrap();
            assert_eq!(list.len(), 1);
            assert_eq!(command_of(&list[0]), HOOK);
        }
        assert_eq!(out.as_object().unwrap().len(), 1, "only hooks: {out}");
    }
}

#[test]
fn settings_transform_replaces_a_hooks_value_that_is_not_an_object() {
    let out = settings_transform(json!({"hooks": "nope", "env": 3}), HOOK);
    assert_eq!(out["hooks"]["PreToolUse"][0]["matcher"], "Agent|Task");
    assert_eq!(out["env"], 3, "a non-object env is left alone");
}

fn spec_in(root: &Path) -> SeedSpec {
    SeedSpec {
        config_dir: root.join("cfg"),
        host_claude_dir: root.join("host"),
        binary: PathBuf::from("/opt/gsd-meta-manager"),
        run_dir: root.join("run"),
    }
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}

#[test]
fn seed_links_only_existing_host_entries_and_never_claude_md() {
    let root = TempDir::new().unwrap();
    let spec = spec_in(root.path());
    fs::create_dir_all(spec.host_claude_dir.join("agents")).unwrap();
    fs::create_dir_all(spec.host_claude_dir.join("skills")).unwrap();
    fs::write(spec.host_claude_dir.join("CLAUDE.md"), "global rules").unwrap();
    fs::create_dir_all(&spec.config_dir).unwrap();
    // A name the host also has, already present in the config dir: left alone.
    fs::write(spec.config_dir.join("skills"), "mine").unwrap();

    seed(&spec).unwrap();

    let agents = spec.config_dir.join("agents");
    assert!(fs::symlink_metadata(&agents)
        .unwrap()
        .file_type()
        .is_symlink());
    assert_eq!(
        fs::read_link(&agents).unwrap(),
        spec.host_claude_dir.join("agents")
    );
    assert_eq!(
        fs::read_to_string(spec.config_dir.join("skills")).unwrap(),
        "mine",
        "an existing entry is never replaced or merged"
    );
    for absent in [
        "commands",
        "gsd-core",
        "get-shit-done",
        "hooks",
        "CLAUDE.md",
    ] {
        assert!(
            fs::symlink_metadata(spec.config_dir.join(absent)).is_err(),
            "{absent} must not exist in the config dir"
        );
    }
    assert!(!LINK_FROM_HOST.contains(&"CLAUDE.md"));
}

/// Every file under `dir`, relative path -> content, symlinks recorded by target.
fn snapshot(dir: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let rel = path.strip_prefix(base).unwrap().to_owned();
            let kind = entry.file_type().unwrap();
            if kind.is_symlink() {
                out.insert(
                    rel,
                    fs::read_link(&path)
                        .unwrap()
                        .to_string_lossy()
                        .into_owned()
                        .into_bytes(),
                );
            } else if kind.is_dir() {
                out.insert(rel, b"<dir>".to_vec());
                walk(base, &path, out);
            } else {
                out.insert(rel, fs::read(&path).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

#[test]
fn seed_leaves_the_host_claude_dir_byte_identical() {
    let root = TempDir::new().unwrap();
    let spec = spec_in(root.path());
    fs::create_dir_all(spec.host_claude_dir.join("agents/sub")).unwrap();
    fs::write(spec.host_claude_dir.join("agents/sub/a.md"), "agent").unwrap();
    fs::create_dir_all(spec.host_claude_dir.join("hooks")).unwrap();
    fs::write(spec.host_claude_dir.join("CLAUDE.md"), "global").unwrap();
    fs::write(
        spec.host_claude_dir.join("settings.json"),
        host_fixture().to_string(),
    )
    .unwrap();
    fs::write(spec.host_claude_dir.join(".credentials.json"), "{}").unwrap();
    // A pre-existing symlink in the config dir that points INTO the host dir: the
    // fresh write must replace it, not write through it.
    fs::create_dir_all(&spec.config_dir).unwrap();
    std::os::unix::fs::symlink(
        spec.host_claude_dir.join("settings.json"),
        spec.config_dir.join("settings.json"),
    )
    .unwrap();

    let before = snapshot(&spec.host_claude_dir);
    seed(&spec).unwrap();
    seed(&spec).unwrap();
    assert_eq!(snapshot(&spec.host_claude_dir), before);

    let seeded = spec.config_dir.join("settings.json");
    assert!(
        !fs::symlink_metadata(&seeded)
            .unwrap()
            .file_type()
            .is_symlink(),
        "the seeded settings.json is a file of its own"
    );
    assert!(read_json(&seeded)["env"].get("GH_TOKEN").is_none());
}

#[test]
fn seed_without_a_host_settings_file_holds_only_the_driverctl_hooks() {
    let root = TempDir::new().unwrap();
    let spec = spec_in(root.path());
    fs::create_dir_all(&spec.host_claude_dir).unwrap();
    seed(&spec).unwrap();
    let settings = read_json(&spec.config_dir.join("settings.json"));
    assert_eq!(settings.as_object().unwrap().len(), 1);
    assert_eq!(settings["hooks"].as_object().unwrap().len(), 5);
    for link in LINK_FROM_HOST {
        assert!(
            fs::symlink_metadata(spec.config_dir.join(link)).is_err(),
            "{link}: a missing link target is skipped"
        );
    }
    let shim = spec.config_dir.join("bin/driverctl");
    assert_eq!(
        command_of(&settings["hooks"]["PreToolUse"][0]),
        hook_command(&shim, "hook")
    );
}

#[test]
fn run_state_dirs_of_two_projects_with_one_run_id_differ() {
    let root = Path::new("/data/driver");
    let a = run_state_dir_in(root, Path::new("/work/a"), "r1");
    let b = run_state_dir_in(root, Path::new("/work/b"), "r1");
    assert_ne!(a, b);
    assert_eq!(a.file_name(), b.file_name());
    assert_eq!(a.parent().unwrap().parent().unwrap(), root);
    assert_eq!(
        a,
        run_state_dir_in(root, Path::new("/work/a"), "r1"),
        "deterministic"
    );
}

fn regular_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            regular_files(&entry.path(), out);
        } else if kind.is_file() {
            out.push(entry.path());
        }
    }
}

#[test]
fn no_refresh_token_material_reaches_the_sandbox() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let home = TempDir::new().unwrap();
    let host_claude = home.path().join(".claude");
    fs::create_dir_all(&host_claude).unwrap();
    let expires_ms = (Utc::now() + chrono::Duration::hours(1)).timestamp_millis();
    fs::write(
        host_claude.join(".credentials.json"),
        json!({"claudeAiOauth": {
            "accessToken": "AT-SENTINEL-0123456789",
            "refreshToken": "RT-SENTINEL-0123456789",
            "expiresAt": expires_ms,
            "scopes": ["user:inference"]
        }})
        .to_string(),
    )
    .unwrap();
    fs::write(
        host_claude.join("settings.json"),
        host_fixture().to_string(),
    )
    .unwrap();

    std::env::set_var("HOME", home.path());
    std::env::remove_var("XDG_CONFIG_HOME");
    let token = resolve_launch_token(Utc::now()).expect("the host token resolves");
    assert_eq!(token.source, TokenSource::Host);
    assert_eq!(token.value, "AT-SENTINEL-0123456789");

    let spec = SeedSpec {
        config_dir: home.path().join("data/cfg"),
        host_claude_dir: host_claude.clone(),
        binary: PathBuf::from("/opt/gsd-meta-manager"),
        run_dir: home.path().join("run"),
    };
    seed(&spec).unwrap();
    let env = child_env(
        std::env::vars_os(),
        &[
            (
                "CLAUDE_CONFIG_DIR",
                spec.config_dir.clone().into_os_string(),
            ),
            ("CLAUDE_CODE_OAUTH_TOKEN", OsString::from(&token.value)),
        ],
    );

    for (key, value) in &env {
        let value = value.to_string_lossy();
        assert!(
            !value.contains("RT-SENTINEL"),
            "{key:?} carries the refresh token"
        );
        if value.contains("AT-SENTINEL") {
            assert_eq!(
                key,
                &OsString::from("CLAUDE_CODE_OAUTH_TOKEN"),
                "the access token may only be CLAUDE_CODE_OAUTH_TOKEN"
            );
        }
    }
    assert_eq!(
        env[&OsString::from("CLAUDE_CODE_OAUTH_TOKEN")],
        OsString::from("AT-SENTINEL-0123456789")
    );

    let mut files = Vec::new();
    regular_files(&spec.config_dir, &mut files);
    assert!(!files.is_empty());
    for file in files {
        let body = fs::read_to_string(&file).unwrap_or_default();
        assert!(!body.contains("RT-SENTINEL"), "{}", file.display());
        assert!(!body.contains("AT-SENTINEL"), "{}", file.display());
    }
}
