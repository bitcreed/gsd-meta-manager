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
    hook_command, managed_settings, read_host_managed, run_state_dir_in, seed, settings_transform,
    GateCommands, SeedSpec, DENY_TOOLS, DROPPED_SETTINGS_KEYS, LINK_FROM_HOST, MANAGED_FILE,
    MANAGED_SUBDIR,
};
use gsd_meta_manager::launcher::sandbox::child_env;
use serde_json::{json, Value};
use tempfile::TempDir;

const HOOK: &str = "'/cfg/bin/driverctl' hook";
const GUARD: &str = "'/cfg/bin/driverctl' stop-guard --run-dir '/runs/r1'";

/// Serialises the one test that points `HOME` / `XDG_CONFIG_HOME` at a fixture.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Serialises the two shim tests' write -> spawn -> wait sequence.
///
/// A test thread's `fork` inherits another thread's still-open write descriptor on a
/// just-written script until that child execs, so exec of the script fails with
/// `ETXTBSY` ("Text file busy"). A temp-plus-rename does not help: the rename keeps the
/// inode, and the inherited descriptor points at that same inode. So the fake binary
/// is written and every process that runs it exits inside this lock, and
/// [`run_shim`] retries the rare residual `ETXTBSY` (a fork from outside the file).
static EXEC_LOCK: Mutex<()> = Mutex::new(());

fn gate() -> GateCommands {
    GateCommands {
        hook: HOOK.to_owned(),
        stop_guard: GUARD.to_owned(),
    }
}

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
    let out = settings_transform(host_fixture(), Some(&gate()));

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

    // D-17: the user's entry, then the heartbeat, then the Stop guard LAST.
    let stop = out["hooks"]["Stop"].as_array().unwrap();
    assert_eq!(stop.len(), 3);
    assert_eq!(command_of(&stop[0]), "user-stop");
    assert_eq!(command_of(&stop[1]), HOOK);
    assert_eq!(command_of(&stop[2]), GUARD);
    assert!(stop[2].get("matcher").is_none());
    assert_eq!(stop[2]["hooks"][0]["timeout"], 20);
    assert!(
        stop[2]["hooks"][0].get("onFailure").is_none(),
        "a broken guard must fail open, never block the driver"
    );
    for event in ["SessionStart", "SessionEnd"] {
        let list = out["hooks"][event].as_array().unwrap();
        assert_eq!(list.len(), 1, "{event}");
        assert_eq!(command_of(&list[0]), HOOK);
    }
    let post = out["hooks"]["PostToolUse"].as_array().unwrap();
    assert_eq!(post.len(), 1);
    assert_eq!(post[0]["matcher"], "*");
    // D-09a, and the deny list because no managed tier carries it here.
    assert_eq!(out["skipWebFetchPreflight"], true);
    assert_eq!(out["disableClaudeAiConnectors"], true);
    assert_eq!(out["permissions"]["deny"], json!(DENY_TOOLS));
}

#[test]
fn settings_transform_without_a_gate_adds_no_hook_and_keeps_the_user_hooks() {
    let out = settings_transform(host_fixture(), None);
    let pre = out["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(pre.len(), 2);
    assert_eq!(command_of(&pre[0]), "gsd-guard");
    assert_eq!(command_of(&pre[1]), "rtk-rewrite");
    assert!(!out.to_string().contains("driverctl"), "{out}");
    assert!(out.get("permissions").is_none(), "the deny list is managed");
    assert_eq!(out["skipWebFetchPreflight"], true);
    assert_eq!(out["disableClaudeAiConnectors"], true);
    let bare = settings_transform(json!({}), None);
    assert!(bare.get("hooks").is_none(), "{bare}");
}

#[test]
fn degraded_deny_list_is_appended_to_the_hosts_own_without_duplicates() {
    let host = json!({"permissions": {"deny": ["Bash(rm:*)", "Cron*"], "allow": ["Read"]}});
    let out = settings_transform(host, Some(&gate()));
    let deny: Vec<&str> = out["permissions"]["deny"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(deny[0], "Bash(rm:*)");
    for tool in DENY_TOOLS {
        assert_eq!(deny.iter().filter(|d| *d == tool).count(), 1, "{tool}");
    }
    assert_eq!(out["permissions"]["allow"], json!(["Read"]));
}

#[test]
fn settings_transform_of_nothing_is_exactly_the_driverctl_entries() {
    for host in [json!({}), json!([1, 2]), json!("text"), Value::Null] {
        let out = settings_transform(host.clone(), Some(&gate()));
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
        for (event, list) in hooks {
            let list = list.as_array().unwrap();
            if event == "Stop" {
                // The heartbeat, then the Stop guard: exactly two driverctl entries.
                assert_eq!(list.len(), 2);
                assert_eq!(command_of(&list[0]), HOOK);
                assert_eq!(command_of(&list[1]), GUARD);
            } else {
                assert_eq!(list.len(), 1, "{event}");
                assert_eq!(command_of(&list[0]), HOOK);
            }
        }
        // Hooks, the permissions (the deny list), and the two D-09a flags.
        let mut keys: Vec<&str> = out
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "disableClaudeAiConnectors",
                "hooks",
                "permissions",
                "skipWebFetchPreflight"
            ],
            "{out}"
        );
    }
}

#[test]
fn settings_transform_replaces_a_hooks_value_that_is_not_an_object() {
    let out = settings_transform(json!({"hooks": "nope", "env": 3}), Some(&gate()));
    assert_eq!(out["hooks"]["PreToolUse"][0]["matcher"], "Agent|Task");
    assert_eq!(out["env"], 3, "a non-object env is left alone");
}

fn spec_in(root: &Path) -> SeedSpec {
    SeedSpec {
        config_dir: root.join("cfg"),
        host_claude_dir: root.join("host"),
        binary: PathBuf::from("/opt/gsd-meta-manager"),
        run_dir: root.join("run"),
        socket: root.join("driverctl.sock"),
        managed: false,
        host_managed_dir: root.join("no-host-managed"),
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
    // Hooks, the deny list, and the two D-09a flags: nothing else.
    assert_eq!(settings.as_object().unwrap().len(), 4);
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
fn seed_adds_the_stop_guard_as_the_last_stop_entry_naming_the_quoted_run_dir() {
    let root = TempDir::new().unwrap();
    let mut spec = spec_in(root.path());
    spec.run_dir = root.path().join("it's a run");
    fs::create_dir_all(&spec.host_claude_dir).unwrap();
    fs::write(
        spec.host_claude_dir.join("settings.json"),
        host_fixture().to_string(),
    )
    .unwrap();
    seed(&spec).unwrap();
    seed(&spec).unwrap();

    let settings = read_json(&spec.config_dir.join("settings.json"));
    let shim = spec.config_dir.join("bin/driverctl");
    let stop = settings["hooks"]["Stop"].as_array().unwrap();
    assert_eq!(stop.len(), 3, "idempotent: two seeds do not stack entries");
    assert_eq!(command_of(&stop[0]), "user-stop");
    assert_eq!(command_of(&stop[1]), hook_command(&shim, "hook"));
    let quoted = format!(
        "'{}'",
        spec.run_dir.display().to_string().replace('\'', "'\\''")
    );
    assert_eq!(
        command_of(&stop[2]),
        hook_command(&shim, &format!("stop-guard --run-dir {quoted}"))
    );
    // Only the Stop event carries the guard.
    let text = settings.to_string();
    assert_eq!(text.matches("stop-guard").count(), 1, "{text}");
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
        socket: home.path().join("run.sock"),
        managed: true,
        host_managed_dir: home.path().join("no-host-managed"),
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

// ── The `gsd-meta-manager` shim runs `next` and nothing else (Phase 29, D-21) ──

/// Seed a config dir whose "binary" is a script that records each call and echoes
/// its argv, then return the shim path and the record file.
fn next_shim_with_fake_binary(root: &TempDir) -> (PathBuf, PathBuf) {
    use std::os::unix::fs::PermissionsExt;

    let record = root.path().join("fake-calls");
    let fake = root.path().join("fake-gsd-meta-manager");
    fs::write(
        &fake,
        format!(
            "#!/bin/sh\necho called >> '{}'\necho \"$@\"\n",
            record.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    let spec = SeedSpec {
        config_dir: root.path().join("cfg"),
        host_claude_dir: root.path().join("host"),
        binary: fake,
        run_dir: root.path().join("run"),
        socket: root.path().join("run.sock"),
        managed: false,
        host_managed_dir: root.path().join("no-host-managed"),
    };
    seed(&spec).unwrap();
    (spec.config_dir.join("bin/gsd-meta-manager"), record)
}

/// Run the shim and collect its output, retrying a bounded number of times while the
/// exec fails with `ETXTBSY` (`Text file busy`); any other error panics as before.
fn run_shim(shim: &Path, args: &[&str]) -> std::process::Output {
    const ATTEMPTS: usize = 5;
    let busy = rustix::io::Errno::TXTBSY.raw_os_error();
    for attempt in 1..=ATTEMPTS {
        match std::process::Command::new(shim).args(args).output() {
            Err(e) if e.raw_os_error() == Some(busy) && attempt < ATTEMPTS => {}
            Ok(out)
                if out.status.code() == Some(126)
                    && String::from_utf8_lossy(&out.stderr).contains("Text file busy")
                    && attempt < ATTEMPTS => {}
            other => return other.unwrap(),
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    unreachable!("the last attempt always returns")
}

#[test]
fn the_next_shim_forwards_next_and_its_arguments_to_the_binary() {
    let _exec = EXEC_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = TempDir::new().unwrap();
    let (shim, record) = next_shim_with_fake_binary(&root);
    let out = run_shim(&shim, &["next", "/p", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert_eq!(String::from_utf8_lossy(&out.stdout), "next /p --json\n");
    assert_eq!(fs::read_to_string(record).unwrap(), "called\n");
}

#[test]
fn the_next_shim_refuses_any_other_verb_without_running_the_binary() {
    let _exec = EXEC_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = TempDir::new().unwrap();
    let (shim, record) = next_shim_with_fake_binary(&root);
    for argv in [vec!["driver", "launch", "x", "y"], vec![], vec!["nextish"]] {
        let out = run_shim(&shim, &argv);
        assert_eq!(out.status.code(), Some(2), "{argv:?}: {out:?}");
        assert!(out.stdout.is_empty(), "{argv:?}");
        assert_eq!(
            String::from_utf8_lossy(&out.stderr),
            "gsd-meta-manager: only 'next' is available inside the driver sandbox\n",
            "{argv:?}"
        );
    }
    assert!(!record.exists(), "the fake binary must never have run");
}

// ── 28.1-02: the gate in the managed tier (D-12, D-13) ──

fn gate_entry(matcher: Option<&str>, command: &str, block: bool) -> Value {
    let mut hook = json!({"type": "command", "command": command, "timeout": 20});
    if block {
        hook["onFailure"] = json!("block");
    }
    match matcher {
        Some(m) => json!({"matcher": m, "hooks": [hook]}),
        None => json!({"hooks": [hook]}),
    }
}

#[test]
fn managed_settings_golden() {
    let out = managed_settings(&gate(), &[]);
    let expected = json!({
        "hooks": {
            "PreToolUse": [gate_entry(Some("Agent|Task"), HOOK, true)],
            "SessionStart": [gate_entry(None, HOOK, false)],
            "SessionEnd": [gate_entry(None, HOOK, false)],
            "Stop": [gate_entry(None, HOOK, false), gate_entry(None, GUARD, false)],
            "PostToolUse": [gate_entry(Some("*"), HOOK, false)]
        },
        "permissions": {"deny": [
            "Cron*", "RemoteTrigger", "SendMessage", "ListAgents", "ScheduleWakeup",
            "DesignSync", "mcp__claude_ai_*"
        ]},
        "skipWebFetchPreflight": true,
        "disableClaudeAiConnectors": true,
        "enableArtifact": false,
        "disableRemoteControl": true,
        "crossSessionInbound": "refuse"
    });
    assert_eq!(out, expected);
    assert!(out.get("allowManagedHooksOnly").is_none());
    assert_eq!(DENY_TOOLS.len(), 7);
}

#[test]
fn host_managed_policy_is_merged_not_hidden() {
    let admin_hook =
        json!({"matcher": "Bash", "hooks": [{"type": "command", "command": "admin-audit"}]});
    let host = json!({
        "permissions": {"deny": ["Bash(rm:*)"], "allow": ["Read"]},
        "hooks": {"PreToolUse": [admin_hook]},
        "model": "x",
        "disableRemoteControl": false,
        "allowManagedHooksOnly": true
    });
    let out = managed_settings(&gate(), &[host]);

    let mut deny: Vec<Value> = DENY_TOOLS.iter().map(|t| json!(t)).collect();
    deny.push(json!("Bash(rm:*)"));
    assert_eq!(out["permissions"]["deny"], Value::Array(deny));
    assert_eq!(out["permissions"]["allow"], json!(["Read"]));
    assert_eq!(
        out["hooks"]["PreToolUse"],
        json!([gate_entry(Some("Agent|Task"), HOOK, true), admin_hook])
    );
    assert_eq!(out["model"], "x");
    // The launcher's five locks are already the strictest value of each key.
    assert_eq!(out["disableRemoteControl"], true);
    // The administrator's own lock stays in force (the launcher never sets it).
    assert_eq!(out["allowManagedHooksOnly"], true);
    // Duplicates are not stacked, and a second doc layers after the first.
    let again = managed_settings(
        &gate(),
        &[
            json!({"permissions": {"deny": ["Bash(rm:*)"]}}),
            json!({"permissions": {"deny": ["Bash(rm:*)", "Edit"]}, "model": "y"}),
        ],
    );
    let deny: Vec<&str> = again["permissions"]["deny"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(&deny[DENY_TOOLS.len()..], ["Bash(rm:*)", "Edit"]);
    assert_eq!(again["model"], "y");
}

#[test]
fn read_host_managed_reads_the_file_then_the_dropins_in_name_order() {
    let root = TempDir::new().unwrap();
    let dir = root.path().join("etc-claude-code");
    fs::create_dir_all(dir.join("managed-settings.d")).unwrap();
    fs::write(dir.join(MANAGED_FILE), r#"{"a": 1}"#).unwrap();
    // Written out of order on purpose; a non-.json file is not a drop-in.
    fs::write(dir.join("managed-settings.d/20-b.json"), r#"{"c": 3}"#).unwrap();
    fs::write(dir.join("managed-settings.d/10-a.json"), r#"{"b": 2}"#).unwrap();
    fs::write(dir.join("managed-settings.d/README"), "not json").unwrap();
    let docs = read_host_managed(&dir).unwrap();
    assert_eq!(docs, [json!({"a": 1}), json!({"b": 2}), json!({"c": 3})]);

    assert_eq!(
        read_host_managed(&root.path().join("absent")).unwrap(),
        Vec::<Value>::new()
    );
}

#[test]
fn an_unparseable_host_managed_file_refuses() {
    for body in ["[1]", "{", "\"text\""] {
        let root = TempDir::new().unwrap();
        let host_dir = root.path().join("etc-claude-code");
        fs::create_dir_all(&host_dir).unwrap();
        let file = host_dir.join(MANAGED_FILE);
        fs::write(&file, body).unwrap();

        let err = read_host_managed(&host_dir).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData, "{body}");
        assert!(
            err.to_string().contains(&file.display().to_string()),
            "{err}"
        );

        let mut spec = spec_in(root.path());
        spec.managed = true;
        spec.host_managed_dir = host_dir;
        let err = seed(&spec).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidData, "{body}");
        assert!(
            !spec.config_dir.exists(),
            "{body}: a refused launch writes nothing"
        );
    }
    // A drop-in is judged the same way.
    let root = TempDir::new().unwrap();
    let host_dir = root.path().join("etc-claude-code");
    fs::create_dir_all(host_dir.join("managed-settings.d")).unwrap();
    fs::write(host_dir.join("managed-settings.d/10-bad.json"), "[]").unwrap();
    let err = read_host_managed(&host_dir).unwrap_err();
    assert!(err.to_string().contains("10-bad.json"), "{err}");
}

#[test]
fn managed_seed_is_idempotent_and_keeps_the_gate_out_of_settings_json() {
    use std::os::unix::fs::PermissionsExt;

    let root = TempDir::new().unwrap();
    let mut spec = spec_in(root.path());
    spec.managed = true;
    fs::create_dir_all(&spec.host_claude_dir).unwrap();
    fs::write(
        spec.host_claude_dir.join("settings.json"),
        host_fixture().to_string(),
    )
    .unwrap();
    seed(&spec).unwrap();
    seed(&spec).unwrap();

    let managed_path = spec.config_dir.join(MANAGED_SUBDIR).join(MANAGED_FILE);
    let managed = read_json(&managed_path);
    assert_eq!(managed["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
    assert_eq!(managed["hooks"]["Stop"].as_array().unwrap().len(), 2);
    assert_eq!(managed["hooks"]["PreToolUse"][0]["matcher"], "Agent|Task");
    assert_eq!(
        managed["hooks"]["PreToolUse"][0]["hooks"][0]["onFailure"],
        "block"
    );
    let mode = fs::metadata(&managed_path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);

    let text = fs::read_to_string(spec.config_dir.join("settings.json")).unwrap();
    assert!(
        !text.contains("driverctl"),
        "no gate entry in settings.json: {text}"
    );
    let settings: Value = serde_json::from_str(&text).unwrap();
    // The user's kept hooks stay, in order, and nothing fires twice.
    assert_eq!(settings["hooks"]["PreToolUse"].as_array().unwrap().len(), 2);
    assert_eq!(command_of(&settings["hooks"]["PreToolUse"][0]), "gsd-guard");
    assert_eq!(settings["hooks"]["Stop"].as_array().unwrap().len(), 1);
    assert_eq!(settings["skipWebFetchPreflight"], true);
    assert_eq!(settings["disableClaudeAiConnectors"], true);
    assert!(settings.get("permissions").is_none());
}

#[test]
fn a_managed_seed_merges_the_hosts_policy_into_the_per_run_file() {
    let root = TempDir::new().unwrap();
    let mut spec = spec_in(root.path());
    spec.managed = true;
    spec.host_managed_dir = root.path().join("etc-claude-code");
    fs::create_dir_all(&spec.host_managed_dir).unwrap();
    fs::write(
        spec.host_managed_dir.join(MANAGED_FILE),
        r#"{"permissions": {"deny": ["WebSearch"]}, "model": "admin"}"#,
    )
    .unwrap();
    seed(&spec).unwrap();
    let managed = read_json(&spec.config_dir.join(MANAGED_SUBDIR).join(MANAGED_FILE));
    assert_eq!(managed["model"], "admin");
    let deny = managed["permissions"]["deny"].as_array().unwrap();
    assert_eq!(deny.last().unwrap(), "WebSearch");
    assert_eq!(deny.len(), DENY_TOOLS.len() + 1);
    // The host's own directory was only read.
    assert_eq!(
        fs::read_to_string(spec.host_managed_dir.join(MANAGED_FILE)).unwrap(),
        r#"{"permissions": {"deny": ["WebSearch"]}, "model": "admin"}"#
    );
}

#[test]
fn degraded_seed_keeps_the_gate_and_the_deny_list_in_settings_json() {
    let root = TempDir::new().unwrap();
    let spec = spec_in(root.path());
    assert!(!spec.managed);
    seed(&spec).unwrap();
    assert!(
        !spec.config_dir.join(MANAGED_SUBDIR).exists(),
        "no managed file in the degraded posture"
    );
    let settings = read_json(&spec.config_dir.join("settings.json"));
    let pre = &settings["hooks"]["PreToolUse"][0];
    assert_eq!(pre["matcher"], "Agent|Task");
    assert_eq!(pre["hooks"][0]["onFailure"], "block");
    assert_eq!(settings["permissions"]["deny"], json!(DENY_TOOLS));
    assert_eq!(settings["skipWebFetchPreflight"], true);
    assert_eq!(settings["disableClaudeAiConnectors"], true);
}

#[test]
fn the_shim_pins_the_socket() {
    let _exec = EXEC_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let root = TempDir::new().unwrap();
    let mut spec = spec_in(root.path());
    spec.socket = PathBuf::from("/run/user/1000/it's a sock.sock");
    spec.binary = PathBuf::from("/opt/gsd mm/it's-bin");
    seed(&spec).unwrap();
    let shim = spec.config_dir.join("bin/driverctl");
    assert_eq!(
        fs::read_to_string(&shim).unwrap(),
        "#!/bin/sh\nDRIVERCTL_SOCK='/run/user/1000/it'\\''s a sock.sock'; export DRIVERCTL_SOCK\nexec '/opt/gsd mm/it'\\''s-bin' driverctl \"$@\"\n"
    );

    // Functionally: a caller that sets or unsets the variable cannot redirect it.
    use std::os::unix::fs::PermissionsExt;
    let fake = root.path().join("fake-bin");
    fs::write(&fake, "#!/bin/sh\necho \"sock=$DRIVERCTL_SOCK args=$*\"\n").unwrap();
    fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).unwrap();
    spec.binary = fake;
    spec.socket = root.path().join("real.sock");
    seed(&spec).unwrap();
    let out = std::process::Command::new(&shim)
        .args(["hook"])
        .env("DRIVERCTL_SOCK", "/attacker.sock")
        .output();
    let out = match out {
        Ok(out) => out,
        // ETXTBSY from a concurrent fork: fall back to the retrying runner.
        Err(_) => run_shim(&shim, &["hook"]),
    };
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!(
            "sock={} args=driverctl hook\n",
            root.path().join("real.sock").display()
        )
    );
}

#[test]
fn scripts_is_linked() {
    assert!(LINK_FROM_HOST.contains(&"scripts"));
    let root = TempDir::new().unwrap();
    let spec = spec_in(root.path());
    fs::create_dir_all(spec.host_claude_dir.join("scripts")).unwrap();
    seed(&spec).unwrap();
    let link = spec.config_dir.join("scripts");
    assert_eq!(
        fs::read_link(&link).unwrap(),
        spec.host_claude_dir.join("scripts")
    );
}
