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
use gsd_meta_manager::launcher::auth::{resolve_launch_token_in, TokenSource};
use gsd_meta_manager::launcher::config_dir::{
    hook_command, hook_is_kept, managed_settings, read_host_managed, run_state_dir_in, seed,
    settings_transform, GateCommands, SeedSpec, DENY_TOOLS, DROPPED_SETTINGS_KEYS, LINK_FROM_HOST,
    MANAGED_FILE, MANAGED_SUBDIR,
};
use gsd_meta_manager::launcher::sandbox::child_env;
use serde_json::{json, Value};
use tempfile::TempDir;

const HOOK: &str = "'/cfg/bin/driverctl' hook";
const GUARD: &str = "'/cfg/bin/driverctl' stop-guard --run-dir '/runs/r1'";

/// Serialises the two shim tests' write -> spawn -> wait sequence.
///
/// A test thread's `fork` inherits another thread's still-open write descriptor on a
/// just-written script until that child execs, so exec of the script fails with
/// `ETXTBSY` ("Text file busy"). A temp-plus-rename does not help: the rename keeps the
/// inode, and the inherited descriptor points at that same inode. So the fake binary
/// is written and every process that runs it exits inside this lock, and
/// [`run_shim`] retries the rare residual `ETXTBSY` (a fork from outside the file).
static EXEC_LOCK: Mutex<()> = Mutex::new(());

/// The host home the hook filter reasons about (D-19).
const HOME: &str = "/home/u";

/// The `$HOME` entries the sandbox re-binds in these tests.
fn roots() -> Vec<PathBuf> {
    [
        "/home/u/.claude/hooks",
        "/home/u/.claude/gsd-core",
        "/home/u/.nvm/versions/node/v24",
    ]
    .iter()
    .map(PathBuf::from)
    .collect()
}

fn transform(host: Value, gate: Option<&GateCommands>) -> Value {
    settings_transform(host, gate, Path::new(HOME), &roots())
}

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
                {"matcher": "Bash", "hooks": [{"type": "command", "command": "/home/u/.claude/hooks/gsd-guard.sh"}]},
                {"matcher": "Bash", "hooks": [{"type": "command", "command": "/home/u/.claude/hooks/rtk-rewrite.sh"}]}
            ],
            "Stop": [{"hooks": [{"type": "command", "command": "rtk hook stop"}]}]
        }
    })
}

#[test]
fn settings_transform_keeps_user_hooks_in_order_and_drops_the_auth_keys() {
    let out = transform(host_fixture(), Some(&gate()));

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
    assert_eq!(command_of(&pre[1]), "/home/u/.claude/hooks/gsd-guard.sh");
    assert_eq!(command_of(&pre[2]), "/home/u/.claude/hooks/rtk-rewrite.sh");

    // D-17: the user's entry, then the heartbeat, then the Stop guard LAST.
    let stop = out["hooks"]["Stop"].as_array().unwrap();
    assert_eq!(stop.len(), 3);
    assert_eq!(command_of(&stop[0]), "rtk hook stop");
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
    let out = transform(host_fixture(), None);
    let pre = out["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(pre.len(), 2);
    assert_eq!(command_of(&pre[0]), "/home/u/.claude/hooks/gsd-guard.sh");
    assert_eq!(command_of(&pre[1]), "/home/u/.claude/hooks/rtk-rewrite.sh");
    assert!(!out.to_string().contains("driverctl"), "{out}");
    assert!(out.get("permissions").is_none(), "the deny list is managed");
    assert_eq!(out["skipWebFetchPreflight"], true);
    assert_eq!(out["disableClaudeAiConnectors"], true);
    let bare = transform(json!({}), None);
    assert!(bare.get("hooks").is_none(), "{bare}");
}

#[test]
fn degraded_deny_list_is_appended_to_the_hosts_own_without_duplicates() {
    let host = json!({"permissions": {"deny": ["Bash(rm:*)", "Cron*"], "allow": ["Read"]}});
    let out = transform(host, Some(&gate()));
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
        let out = transform(host.clone(), Some(&gate()));
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
    let out = transform(json!({"hooks": "nope", "env": 3}), Some(&gate()));
    assert_eq!(out["hooks"]["PreToolUse"][0]["matcher"], "Agent|Task");
    assert_eq!(out["env"], 3, "a non-object env is left alone");
}

fn hooks_of(out: &Value, event: &str) -> Vec<String> {
    out["hooks"][event]
        .as_array()
        .map(|list| {
            list.iter()
                .flat_map(|entry| entry["hooks"].as_array().cloned().unwrap_or_default())
                .filter_map(|hook| hook["command"].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn the_claudear_hooks_are_dropped() {
    let host = json!({"hooks": {
        "PermissionRequest": [{"hooks": [{"type": "command", "command":
            "/home/u/.local/share/pipx/venvs/claudear/bin/python -m claudear.hooks.permission"}]}],
        "Notification": [{"hooks": [{"type": "command", "command": "curl -s localhost:8787/x"}]}],
        "PreToolUse": [
            {"matcher": "Bash", "hooks": [{"type": "command", "command": "echo hi"}]},
            {"matcher": "Bash", "hooks": [{"type": "command", "command":
                "bash /home/u/.claude/hooks/gsd-validate-commit.sh"}]},
            {"matcher": "Read", "hooks": [{"type": "command", "command":
                "node /home/u/.claude/hooks/gsd-x.js /home/u/.secret/x"}]}
        ],
    }});
    let out = transform(host, None);
    assert!(out["hooks"].get("PermissionRequest").is_none(), "{out}");
    assert!(out["hooks"].get("Notification").is_none(), "{out}");
    assert_eq!(
        hooks_of(&out, "PreToolUse"),
        ["bash /home/u/.claude/hooks/gsd-validate-commit.sh"]
    );
    assert!(!out.to_string().contains("claudear"), "{out}");
}

#[test]
fn gsd_and_rtk_hooks_survive_in_order_and_an_emptied_matcher_disappears() {
    let node = "\"$(for n in \"/home/u/.nvm/versions/node/v24/bin/node\" \"$(command -v node)\" /usr/bin/node; do [ -x \"$n\" ] && echo \"$n\" && break; done)\" \"/home/u/.claude/hooks/gsd-context-monitor.js\"";
    let host = json!({"hooks": {"PostToolUse": [
        {"matcher": "*", "hooks": [{"type": "command", "command": node}]},
        {"matcher": "Edit", "hooks": [
            {"type": "command", "command": "echo dropped"},
            {"type": "command", "command": "/home/u/.claude/hooks/rtk-rewrite.sh"}
        ]},
        {"matcher": "Bash", "hooks": [{"type": "command", "command": "rtk hook claude"}]},
        {"matcher": "Write", "hooks": [{"type": "command", "command": "echo nope"}]},
        {"matcher": "Task", "hooks": [{"type": "prompt", "prompt": "be careful"}]}
    ]}});
    let out = transform(host, None);
    let list = out["hooks"]["PostToolUse"].as_array().unwrap();
    let matchers: Vec<&str> = list
        .iter()
        .map(|e| e["matcher"].as_str().unwrap())
        .collect();
    assert_eq!(matchers, ["*", "Edit", "Bash"]);
    assert_eq!(command_of(&list[0]), node);
    assert_eq!(list[1]["hooks"].as_array().unwrap().len(), 1);
    assert_eq!(command_of(&list[1]), "/home/u/.claude/hooks/rtk-rewrite.sh");
    assert_eq!(command_of(&list[2]), "rtk hook claude");
}

#[test]
fn hook_is_kept_decides_on_the_program_and_on_every_home_path_it_names() {
    let home = Path::new(HOME);
    let roots = roots();
    for kept in [
        "bash /home/u/.claude/hooks/gsd-validate-commit.sh",
        "/home/u/.claude/hooks/rtk-rewrite.sh",
        "rtk hook claude",
        "/usr/local/bin/rtk hook claude",
        "node /home/u/.claude/gsd-core/bin/gsd-tools.cjs query init",
        "FOO=1 node /home/u/.claude/hooks/a.js",
        // GSD's node wrapper: a stale installer node among the probed candidates.
        "\"$(for n in \"/home/u/.nvm/versions/node/v99.0.0/bin/node\" \"$(command -v node)\" /usr/bin/node; do [ -x \"$n\" ] && printf '%s' \"$n\" && break; done)\" \"/home/u/.claude/hooks/gsd-x.js\"",
        // `~`, `$HOME`, `${HOME}` and `"$HOME"` spellings, expanded before the root check.
        "node ~/.claude/hooks/a.js",
        "node $HOME/.claude/hooks/a.js",
        "node ${HOME}/.claude/hooks/a.js",
        "node \"$HOME\"/.claude/hooks/a.js",
        "node \"$HOME/.claude/hooks/a.js\"",
    ] {
        assert!(hook_is_kept(kept, home, &roots), "{kept}");
    }
    for dropped in [
        "/home/u/.local/share/pipx/venvs/claudear/bin/python -m claudear.hooks.permission",
        "curl -s localhost:8787/x",
        "curl -s http://127.0.0.1:8787/x /home/u/.claude/hooks/a.js",
        "rtk hook claude --notify localhost:8787",
        "echo hi",
        "",
        "node /home/u/.claude/hooks/a.js /home/u/.secret/x",
        // Only the `for ... in` candidate list is exempt, not the rest of the line.
        "for n in /usr/bin/node; do node /home/u/.secret/x /home/u/.claude/hooks/a.js; done",
        "node ~/.secret/x.js .claude/hooks/",
        "node $HOME/.secret/x.js .claude/hooks/",
        "node ${HOME}/.secret/x.js .claude/hooks/",
        "node \"$HOME\"/.secret/x.js .claude/hooks/",
        "node /home/u/.claude/scripts/a.js",
        "node /home/u",
    ] {
        assert!(!hook_is_kept(dropped, home, &roots), "{dropped}");
    }
    // A path outside `$HOME` needs no root: `/usr/bin/node` is visible through `/`.
    assert!(hook_is_kept(
        "/usr/bin/node /home/u/.claude/hooks/a.js",
        home,
        &roots
    ));
    // With no roots no home path is allowed, but a bare rtk call still is.
    assert!(!hook_is_kept("node /home/u/.claude/hooks/a.js", home, &[]));
    assert!(hook_is_kept("rtk hook claude", home, &[]));
}

#[test]
fn seed_filters_the_host_hooks_by_the_keep_roots_in_both_postures() {
    let root = TempDir::new().unwrap();
    let home = root.path().join("home");
    let hooks = home.join(".claude/hooks");
    fs::create_dir_all(&hooks).unwrap();
    let keep = format!("bash {}/gsd-ok.sh", hooks.display());
    fs::write(
        home.join(".claude/settings.json"),
        json!({"hooks": {"PreToolUse": [
            {"matcher": "Bash", "hooks": [{"type": "command", "command": keep}]},
            {"matcher": "Bash", "hooks": [{"type": "command", "command":
                format!("{}/.local/share/pipx/venvs/claudear/bin/python -m claudear.hooks.permission", home.display())}]}
        ]}})
        .to_string(),
    )
    .unwrap();
    for managed in [true, false] {
        let mut spec = spec_in(root.path());
        spec.config_dir = root.path().join(format!("cfg-{managed}"));
        spec.host_claude_dir = home.join(".claude");
        spec.host_home = home.clone();
        spec.managed = managed;
        spec.keep_roots = vec![hooks.clone()];
        seed(&spec).unwrap();
        let settings = read_json(&spec.config_dir.join("settings.json"));
        let commands = hooks_of(&settings, "PreToolUse");
        assert!(commands.contains(&keep), "managed={managed}: {commands:?}");
        assert!(
            !settings.to_string().contains("claudear"),
            "managed={managed}: {settings}"
        );
        // And with no roots at all, the GSD hook is gone as well.
        spec.keep_roots = Vec::new();
        seed(&spec).unwrap();
        let settings = read_json(&spec.config_dir.join("settings.json"));
        assert!(!hooks_of(&settings, "PreToolUse").contains(&keep));
    }
}

fn spec_in(root: &Path) -> SeedSpec {
    SeedSpec {
        config_dir: root.join("cfg"),
        host_claude_dir: root.join("host"),
        host_home: root.to_path_buf(),
        binary: PathBuf::from("/opt/gsd-meta-manager"),
        run_dir: root.join("run"),
        socket: root.join("driverctl.sock"),
        managed: false,
        host_managed_dir: root.join("no-host-managed"),
        keep_roots: Vec::new(),
    }
}

#[test]
fn seed_expands_home_against_host_home_not_the_claude_dirs_parent() {
    // quick 261113-dck: `host_claude_dir` is `$CLAUDE_CONFIG_DIR` (a profile), so its
    // parent is not the host `$HOME` the hook's `$HOME` expands against.
    let root = TempDir::new().unwrap();
    let home = root.path().join("home");
    let profile = root.path().join("profiles/work");
    let hooks = home.join(".claude/hooks");
    fs::create_dir_all(&hooks).unwrap();
    fs::create_dir_all(&profile).unwrap();
    let keep = "bash $HOME/.claude/hooks/gsd-x.sh".to_owned();
    fs::write(
        profile.join("settings.json"),
        json!({"hooks": {"PreToolUse": [
            {"matcher": "Bash", "hooks": [{"type": "command", "command": keep}]}
        ]}})
        .to_string(),
    )
    .unwrap();
    for managed in [true, false] {
        let mut spec = spec_in(root.path());
        spec.config_dir = root.path().join(format!("cfg-{managed}"));
        spec.host_claude_dir = profile.clone();
        spec.host_home = home.clone();
        spec.managed = managed;
        spec.keep_roots = vec![hooks.clone()];
        seed(&spec).unwrap();
        let settings = read_json(&spec.config_dir.join("settings.json"));
        assert!(
            hooks_of(&settings, "PreToolUse").contains(&keep),
            "managed={managed}: {settings}"
        );
    }
}

#[test]
fn a_gsd_hook_under_a_relocated_claude_root_is_kept() {
    let root = TempDir::new().unwrap();
    let home = root.path().join("home");
    let profile = home.join("profiles/work");
    fs::create_dir_all(profile.join("hooks")).unwrap();
    let gsd = format!("node {}/hooks/gsd-y.js", profile.display());
    let other = format!("bash {}/bin/other.sh", profile.display());
    fs::write(
        profile.join("settings.json"),
        json!({"hooks": {"PreToolUse": [
            {"matcher": "Bash", "hooks": [
                {"type": "command", "command": gsd},
                {"type": "command", "command": other}
            ]}
        ]}})
        .to_string(),
    )
    .unwrap();
    let mut spec = spec_in(root.path());
    spec.host_claude_dir = profile.clone();
    spec.host_home = home.clone();
    spec.keep_roots = vec![profile.join("hooks"), profile.join("bin")];
    seed(&spec).unwrap();
    let commands = hooks_of(
        &read_json(&spec.config_dir.join("settings.json")),
        "PreToolUse",
    );
    assert!(commands.contains(&gsd), "{commands:?}");
    assert!(!commands.contains(&other), "{commands:?}");
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
    assert_eq!(command_of(&stop[0]), "rtk hook stop");
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

    let no_stored = home.path().join("no-stored-token");
    let token = resolve_launch_token_in(Some(&no_stored), Some(&host_claude), Utc::now())
        .expect("the host token resolves");
    assert_eq!(token.source, TokenSource::Host);
    assert_eq!(token.value, "AT-SENTINEL-0123456789");

    let spec = SeedSpec {
        config_dir: home.path().join("data/cfg"),
        host_claude_dir: host_claude.clone(),
        host_home: home.path().to_path_buf(),
        binary: PathBuf::from("/opt/gsd-meta-manager"),
        run_dir: home.path().join("run"),
        socket: home.path().join("run.sock"),
        managed: true,
        host_managed_dir: home.path().join("no-host-managed"),
        keep_roots: Vec::new(),
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
        host_home: root.path().to_path_buf(),
        binary: fake,
        run_dir: root.path().join("run"),
        socket: root.path().join("run.sock"),
        managed: false,
        host_managed_dir: root.path().join("no-host-managed"),
        keep_roots: Vec::new(),
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
    assert_eq!(
        command_of(&settings["hooks"]["PreToolUse"][0]),
        "/home/u/.claude/hooks/gsd-guard.sh"
    );
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

// 28-REVIEW WR-08 / 29-REVIEW WR-02: a host `hooks` or `permissions` that is not a
// JSON object (here an array, a string, null and a number, across the file and a
// drop-in) must never replace the launcher's gate hooks or deny list in the managed
// file, and must not stop the rest of the host policy from merging.
#[test]
fn a_managed_seed_keeps_the_gate_when_the_hosts_hooks_or_permissions_is_not_an_object() {
    let root = TempDir::new().unwrap();
    let mut spec = spec_in(root.path());
    spec.managed = true;
    seed(&spec).unwrap();
    let managed_path = spec.config_dir.join(MANAGED_SUBDIR).join(MANAGED_FILE);
    let baseline = read_json(&managed_path);

    spec.host_managed_dir = root.path().join("etc-claude-code");
    fs::create_dir_all(spec.host_managed_dir.join("managed-settings.d")).unwrap();
    fs::write(
        spec.host_managed_dir.join(MANAGED_FILE),
        r#"{"hooks": [], "permissions": "x", "model": "admin"}"#,
    )
    .unwrap();
    fs::write(
        spec.host_managed_dir
            .join("managed-settings.d")
            .join("10-null.json"),
        r#"{"hooks": null, "permissions": 7}"#,
    )
    .unwrap();
    seed(&spec).unwrap();

    let managed = read_json(&managed_path);
    assert_eq!(managed["hooks"], baseline["hooks"]);
    assert_eq!(managed["permissions"], baseline["permissions"]);
    assert_eq!(
        managed["permissions"]["deny"].as_array().unwrap().len(),
        DENY_TOOLS.len()
    );
    assert_eq!(managed["model"], "admin");
}

// 30-REVIEW WR-13: the nested counterpart of 28 WR-08 / 29 WR-02. A host hook event
// that is not a list, a malformed matcher entry, a permissions list that is not a
// list of strings and a permissions setting that is neither a string nor a bool
// (across the file and a drop-in) must never reach the managed file beside the gate
// hooks and the deny list, and must not stop the rest of the host policy.
#[test]
fn a_managed_seed_drops_wrong_shape_nested_host_values_and_keeps_the_gate() {
    let root = TempDir::new().unwrap();
    let mut spec = spec_in(root.path());
    spec.managed = true;
    seed(&spec).unwrap();
    let managed_path = spec.config_dir.join(MANAGED_SUBDIR).join(MANAGED_FILE);
    let baseline = read_json(&managed_path);

    spec.host_managed_dir = root.path().join("etc-claude-code");
    fs::create_dir_all(spec.host_managed_dir.join("managed-settings.d")).unwrap();
    fs::write(
        spec.host_managed_dir.join(MANAGED_FILE),
        r#"{"hooks": {"Notification": "x", "PreToolUse": {"matcher": "Bash"}, "Stop": [7, {"hooks": "x"}]}, "permissions": {"allow": "x", "deny": "Edit", "ask": [7, null]}, "model": "admin"}"#,
    )
    .unwrap();
    fs::write(
        spec.host_managed_dir
            .join("managed-settings.d")
            .join("10-nested.json"),
        r#"{"hooks": {"SessionStart": [{"matcher": 7, "hooks": []}, {"hooks": [7]}, {"hooks": [{"command": "x"}]}]}, "permissions": {"additionalDirectories": {}, "defaultMode": 7, "disableBypassPermissionsMode": "disable"}}"#,
    )
    .unwrap();
    seed(&spec).unwrap();

    let managed = read_json(&managed_path);
    assert_eq!(managed["hooks"], baseline["hooks"]);
    assert_eq!(
        managed["permissions"]["deny"],
        baseline["permissions"]["deny"]
    );
    assert_eq!(
        managed["permissions"]["deny"].as_array().unwrap().len(),
        DENY_TOOLS.len()
    );
    let mut keys: Vec<&String> = managed["permissions"].as_object().unwrap().keys().collect();
    keys.sort();
    assert_eq!(keys, ["deny", "disableBypassPermissionsMode"]);
    assert_eq!(
        managed["permissions"]["disableBypassPermissionsMode"],
        "disable"
    );
    assert_eq!(managed["model"], "admin");
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
