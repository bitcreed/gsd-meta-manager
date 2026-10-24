#![cfg(target_os = "linux")]
// ============================================================================
// The hardened sandbox profile (Phase 28, plan 28-03; D-19, D-20, D-27,
// RESEARCH §1 and §3).
//
// The goldens are pure (no bwrap, no systemd) and run in CI. The one live smoke
// at the bottom is opt-in:
//
//   cargo test --test launcher_sandbox -- --ignored --nocapture
//
// Run it OUTSIDE any tool sandbox; a `SKIP:` line is no evidence.
// ============================================================================

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use gsd_meta_manager::driverctl::serve::socket_path;
use gsd_meta_manager::launcher::egress::socket_path_for;
use gsd_meta_manager::launcher::home::{HomeBind, HomeSetup};
use gsd_meta_manager::launcher::sandbox::{
    bridged_command, bwrap_argv, child_env, claude_argv, common_env_sets, egress_env_sets,
    etc_entries_in, EtcEntry, LaunchPaths, SandboxSpec, BRIDGE_PORT, DUMMY_OAUTH_TOKEN,
    EXTRA_STRIP_ENV, ROUTING_STRIP_ENV, STRIP_ENV,
};
use tempfile::TempDir;

const RUN: &str = "/work/proj/.planning/meta-manager/drivers/r1";
const CFG: &str = "/data/driver/ab12cd34/r1/claude-config";
const SOCK: &str = "/run/user/1000/gsdmm-driverctl/r1-ab12cd34.sock";
const EGRESS: &str = "/run/user/1000/gsdmm-driverctl/egress-ab12cd34.sock";
const GITCONFIG: &str = "/data/driver/ab12cd34/r1/gitconfig";

fn paths() -> LaunchPaths {
    LaunchPaths {
        project: PathBuf::from("/work/proj"),
        run_dir: PathBuf::from(RUN),
        config_dir: PathBuf::from(CFG),
        socket: PathBuf::from(SOCK),
        egress_socket: PathBuf::from(EGRESS),
    }
}

fn strings(argv: &[OsString]) -> Vec<String> {
    argv.iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect()
}

#[test]
fn golden_bwrap_argv() {
    let spec = SandboxSpec {
        home: PathBuf::from("/home/u"),
        paths: paths(),
        home_binds: vec![
            HomeBind::Ro(PathBuf::from("/home/u/.local/share/claude")),
            HomeBind::Ro(PathBuf::from("/home/u/.claude/hooks")),
            HomeBind::Overlay(PathBuf::from("/home/u/.cargo/registry")),
            HomeBind::Tmpfs {
                path: PathBuf::from("/home/u/.cache"),
                size: 1_073_741_824,
            },
        ],
        gitconfig: Some(PathBuf::from(GITCONFIG)),
        null_masks: vec![PathBuf::from("/home/u/.claude/hooks/token")],
        runtime_dir: Some(PathBuf::from("/run/user/1000")),
        etc_entries: vec![
            EtcEntry::Bind(PathBuf::from("/etc/hosts")),
            EtcEntry::Symlink {
                target: PathBuf::from("../run/systemd/resolve/stub-resolv.conf"),
                path: PathBuf::from("/etc/resolv.conf"),
            },
        ],
    };
    let command = vec![OsString::from("claude"), OsString::from("-p")];
    let expected: Vec<String> = [
        "bwrap",
        // root ro, /dev, /proc, /tmp
        "--ro-bind",
        "/",
        "/",
        "--dev",
        "/dev",
        "--proc",
        "/proc",
        "--tmpfs",
        "/tmp",
        // D-12: /etc is a tmpfs of read-only re-binds, the per-run managed dir sits at
        // /etc/claude-code, and the whole of /etc is remounted read-only
        "--tmpfs",
        "/etc",
        "--ro-bind-try",
        "/etc/hosts",
        "/etc/hosts",
        "--symlink",
        "../run/systemd/resolve/stub-resolv.conf",
        "/etc/resolv.conf",
        "--ro-bind",
        &format!("{CFG}/managed"),
        "/etc/claude-code",
        "--remount-ro",
        "/etc",
        // D-03: no host filesystem socket under /run (docker, system bus, snapd, ...)
        "--tmpfs",
        "/run",
        // D-16: $HOME is a size-capped tmpfs, then the derived allowlist, then the
        // per-run gitconfig (D-17), then the null masks (D-20)
        "--size",
        "268435456",
        "--tmpfs",
        "/home/u",
        "--ro-bind",
        "/home/u/.local/share/claude",
        "/home/u/.local/share/claude",
        "--ro-bind",
        "/home/u/.claude/hooks",
        "/home/u/.claude/hooks",
        "--overlay-src",
        "/home/u/.cargo/registry",
        "--tmp-overlay",
        "/home/u/.cargo/registry",
        "--size",
        "1073741824",
        "--tmpfs",
        "/home/u/.cache",
        "--ro-bind",
        GITCONFIG,
        "/home/u/.gitconfig",
        "--ro-bind",
        "/dev/null",
        "/home/u/.claude/hooks/token",
        // project rw
        "--bind",
        "/work/proj",
        "/work/proj",
        // drivers dir hidden: siblings vanish, own run re-bound below
        "--tmpfs",
        "/work/proj/.planning/meta-manager/drivers",
        // run dir ro, the two lock masks
        "--ro-bind",
        RUN,
        RUN,
        "--ro-bind",
        "/dev/null",
        &format!("{RUN}/launcher.lock"),
        "--ro-bind",
        "/dev/null",
        &format!("{RUN}/DIRECTION.lock"),
        // config dir rw, its settings.json and bin ro
        "--bind",
        CFG,
        CFG,
        "--ro-bind",
        &format!("{CFG}/settings.json"),
        &format!("{CFG}/settings.json"),
        "--ro-bind",
        &format!("{CFG}/bin"),
        &format!("{CFG}/bin"),
        // the managed file is not rewritable through the writable config dir
        "--ro-bind",
        &format!("{CFG}/managed"),
        &format!("{CFG}/managed"),
        // runtime dir tmpfs, then the driverctl socket, then the egress socket
        "--tmpfs",
        "/run/user/1000",
        "--bind",
        SOCK,
        SOCK,
        "--bind",
        EGRESS,
        EGRESS,
        // D-02: own pid, network, ipc and uts namespaces under a fixed hostname
        "--unshare-pid",
        "--unshare-net",
        "--unshare-ipc",
        "--unshare-uts",
        "--hostname",
        "gsdmm-sandbox",
        "--new-session",
        "--die-with-parent",
        "--chdir",
        "/work/proj",
        "--",
        "claude",
        "-p",
    ]
    .iter()
    .map(|s| (*s).to_owned())
    .collect();
    assert_eq!(strings(&bwrap_argv(&spec, &command)), expected);
}

#[test]
fn null_masks_are_a_second_layer_inside_binds_only() {
    let home = TempDir::new().unwrap();
    let home_path = home.path().to_owned();
    fs::create_dir_all(home_path.join(".ssh")).unwrap();
    fs::write(home_path.join(".claude.json"), "{}").unwrap();
    fs::write(home_path.join(".netrc"), "x").unwrap();
    fs::create_dir_all(home_path.join(".cargo/bin")).unwrap();
    fs::write(home_path.join(".cargo/credentials.toml"), "x").unwrap();
    fs::write(home_path.join(".cargo/credentials"), "x").unwrap();

    // No bind: every credential file is hidden by the tmpfs alone, so no mask, and no
    // flag at all names a path under the host home except the tmpfs itself.
    let bare = SandboxSpec::from_host(paths(), &home_path, &HomeSetup::default());
    assert!(bare.null_masks.is_empty());
    assert!(bare.home_binds.is_empty());
    let argv = strings(&bwrap_argv(&bare, &[OsString::from("true")]));
    let home_str = home_path.display().to_string();
    let remount = argv
        .windows(2)
        .position(|w| w == ["--remount-ro", "/etc"])
        .expect("the /etc block ends with --remount-ro /etc");
    assert_eq!(argv[remount + 2..remount + 4], ["--tmpfs", "/run"]);
    assert_eq!(
        argv[remount + 4..remount + 7],
        ["--size", "268435456", "--tmpfs"]
    );
    assert_eq!(argv[remount + 7], home_str);
    assert_eq!(argv[remount + 8], "--bind");
    assert_eq!(argv[remount + 9], "/work/proj");

    // A read-only `.cargo` bind pulls the two cargo credential files in as masks;
    // `.netrc` and `.claude.json` sit outside every bind and get none.
    let setup = HomeSetup {
        binds: vec![HomeBind::Ro(home_path.join(".cargo"))],
        gitconfig: None,
        ..HomeSetup::default()
    };
    let spec = SandboxSpec::from_host(paths(), &home_path, &setup);
    assert_eq!(
        spec.null_masks,
        [
            home_path.join(".cargo/credentials"),
            home_path.join(".cargo/credentials.toml")
        ]
    );
    let argv = strings(&bwrap_argv(&spec, &[OsString::from("true")]));
    let bind = argv
        .windows(3)
        .position(|w| {
            w == [
                "--ro-bind",
                &home_str_join(&home_path, ".cargo"),
                &home_str_join(&home_path, ".cargo"),
            ]
        })
        .expect("the .cargo bind");
    let mask = argv
        .windows(3)
        .position(|w| {
            w[0] == "--ro-bind" && w[1] == "/dev/null" && w[2].ends_with(".cargo/credentials.toml")
        })
        .expect("the credentials mask");
    assert!(bind < mask, "a mask follows the bind that holds the file");
    // The credential directories are no longer emptied one by one (D-20).
    assert!(
        !argv
            .windows(2)
            .any(|w| w[0] == "--tmpfs" && w[1].ends_with("/.ssh")),
        "{argv:?}"
    );
}

fn home_str_join(home: &std::path::Path, rel: &str) -> String {
    home.join(rel).display().to_string()
}

#[test]
fn the_etc_block_precedes_remount_and_skips_host_claude_code() {
    let etc = TempDir::new().unwrap();
    fs::write(etc.path().join("hosts"), "127.0.0.1 localhost\n").unwrap();
    fs::write(etc.path().join(".hidden"), "x").unwrap();
    fs::create_dir(etc.path().join("ssl")).unwrap();
    fs::create_dir(etc.path().join("claude-code")).unwrap();
    fs::write(etc.path().join("claude-code/managed-settings.json"), "{}").unwrap();
    std::os::unix::fs::symlink("../run/resolv.conf", etc.path().join("resolv.conf")).unwrap();

    let entries = etc_entries_in(etc.path());
    let p = |name: &str| etc.path().join(name);
    // Sorted by file name (dot-names included), the host claude-code skipped, a
    // symlink kept as a symlink.
    assert_eq!(
        entries,
        [
            EtcEntry::Bind(p(".hidden")),
            EtcEntry::Bind(p("hosts")),
            EtcEntry::Symlink {
                target: PathBuf::from("../run/resolv.conf"),
                path: p("resolv.conf")
            },
            EtcEntry::Bind(p("ssl")),
        ]
    );
    assert!(etc_entries_in(&etc.path().join("absent")).is_empty());

    let spec = SandboxSpec {
        home: PathBuf::from("/home/u"),
        paths: paths(),
        home_binds: Vec::new(),
        gitconfig: None,
        null_masks: Vec::new(),
        runtime_dir: None,
        etc_entries: entries,
    };
    let argv = strings(&bwrap_argv(&spec, &[OsString::from("true")]));
    let at = |flag: &str, a: &str| {
        argv.windows(2)
            .position(|w| w[0] == flag && w[1] == a)
            .unwrap_or_else(|| panic!("{flag} {a} missing: {argv:?}"))
    };
    let tmpfs = at("--tmpfs", "/etc");
    let managed = at("--ro-bind", &format!("{CFG}/managed"));
    let remount = at("--remount-ro", "/etc");
    assert_eq!(argv[managed + 2], "/etc/claude-code");
    assert!(tmpfs < at("--ro-bind-try", &p("hosts").display().to_string()));
    assert!(at("--ro-bind-try", &p("hosts").display().to_string()) < managed);
    assert!(managed < remount);
    // The host's own claude-code is never re-bound; the only /etc/claude-code
    // argument is the managed dir's destination.
    let host_cc = p("claude-code").display().to_string();
    assert!(!argv.contains(&host_cc), "{argv:?}");
    // The /etc block precedes the project bind and the config-dir bind, and the
    // config-dir bind is followed by the managed re-bind at the managed path.
    let cfg_bind = at("--bind", CFG);
    let managed_rebind = argv
        .windows(3)
        .position(|w| {
            w[0] == "--ro-bind"
                && w[1] == format!("{CFG}/managed")
                && w[2] == format!("{CFG}/managed")
        })
        .expect("managed re-bound at its own path");
    assert!(remount < cfg_bind && cfg_bind < managed_rebind);
}

#[test]
fn a_socket_outside_the_runtime_dir_is_not_re_bound() {
    let mut p = paths();
    p.socket = PathBuf::from("/elsewhere/r1.sock");
    let spec = SandboxSpec {
        home: PathBuf::from("/home/u"),
        paths: p,
        home_binds: Vec::new(),
        gitconfig: None,
        null_masks: Vec::new(),
        runtime_dir: Some(PathBuf::from("/run/user/1000")),
        etc_entries: Vec::new(),
    };
    let argv = strings(&bwrap_argv(&spec, &[OsString::from("true")]));
    assert!(argv.iter().any(|a| a == "/run/user/1000"));
    assert!(!argv.iter().any(|a| a == "/elsewhere/r1.sock"));
}

#[test]
fn the_egress_socket_is_re_bound_after_the_driverctl_socket_and_only_inside_the_runtime_dir() {
    let spec = SandboxSpec {
        home: PathBuf::from("/home/u"),
        paths: paths(),
        home_binds: Vec::new(),
        gitconfig: None,
        null_masks: Vec::new(),
        runtime_dir: Some(PathBuf::from("/run/user/1000")),
        etc_entries: Vec::new(),
    };
    let argv = strings(&bwrap_argv(&spec, &[OsString::from("true")]));
    let at = |flag: &str, a: &str| {
        argv.windows(2)
            .position(|w| w[0] == flag && w[1] == a)
            .unwrap_or_else(|| panic!("{flag} {a} missing: {argv:?}"))
    };
    let runtime = at("--tmpfs", "/run/user/1000");
    let driverctl = at("--bind", SOCK);
    let egress = at("--bind", EGRESS);
    assert_eq!(driverctl, runtime + 2);
    assert_eq!(
        egress,
        driverctl + 3,
        "the egress bind follows the driverctl bind"
    );

    let mut elsewhere = paths();
    elsewhere.egress_socket = PathBuf::from("/elsewhere/egress.sock");
    let spec = SandboxSpec {
        paths: elsewhere,
        ..spec
    };
    let argv = strings(&bwrap_argv(&spec, &[OsString::from("true")]));
    assert!(!argv.iter().any(|a| a == "/elsewhere/egress.sock"));
}

#[test]
fn the_run_tmpfs_follows_the_etc_block_and_precedes_the_project_and_the_runtime_dir() {
    let spec = SandboxSpec {
        home: PathBuf::from("/home/u"),
        paths: paths(),
        home_binds: Vec::new(),
        gitconfig: None,
        null_masks: Vec::new(),
        runtime_dir: Some(PathBuf::from("/run/user/1000")),
        etc_entries: Vec::new(),
    };
    let argv = strings(&bwrap_argv(&spec, &[OsString::from("true")]));
    let at = |flag: &str, a: &str| {
        argv.windows(2)
            .position(|w| w[0] == flag && w[1] == a)
            .unwrap_or_else(|| panic!("{flag} {a} missing: {argv:?}"))
    };
    let remount = at("--remount-ro", "/etc");
    let run = at("--tmpfs", "/run");
    let project = at("--bind", "/work/proj");
    let runtime = at("--tmpfs", "/run/user/1000");
    assert_eq!(run, remount + 2, "/run follows the /etc block");
    assert!(
        run < project && project < runtime,
        "/run precedes the project and the runtime dir"
    );
    assert_eq!(
        argv.iter().filter(|a| a.as_str() == "/run").count(),
        1,
        "/run is mounted exactly once"
    );
}

#[test]
fn golden_bridged_command() {
    let inner = [OsString::from("claude"), OsString::from("-p")];
    let command = bridged_command(
        std::path::Path::new("/x/gsdmm"),
        BRIDGE_PORT,
        std::path::Path::new(EGRESS),
        &inner,
    );
    assert_eq!(
        strings(&command),
        ["/x/gsdmm", "bridge", "--port", "3128", "--sock", EGRESS, "--", "claude", "-p"]
    );
}

#[test]
fn golden_egress_env_sets() {
    let url = "http://127.0.0.1:3128";
    let sets: Vec<(String, String)> = egress_env_sets(3128)
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_string_lossy().into_owned()))
        .collect();
    let want = |k: &str, v: &str| (k.to_owned(), v.to_owned());
    assert_eq!(
        sets,
        [
            want("ANTHROPIC_BASE_URL", url),
            want("HTTPS_PROXY", url),
            want("https_proxy", url),
            want("HTTP_PROXY", url),
            want("http_proxy", url),
            want("NO_PROXY", "127.0.0.1,localhost"),
            want("no_proxy", "127.0.0.1,localhost"),
            want("CLAUDE_CODE_GZIP_REQUEST_BODIES", "0"),
            want("CLAUDE_CODE_OAUTH_TOKEN", DUMMY_OAUTH_TOKEN),
        ]
    );
    assert_eq!(BRIDGE_PORT, 3128);
    assert!(egress_env_sets(8080)
        .iter()
        .all(|(_, v)| !v.to_string_lossy().contains("3128")));
    // The egress socket sits beside the driverctl socket, in the same private dir.
    assert_eq!(
        socket_path_for(
            std::path::Path::new(SOCK),
            std::path::Path::new("/nonexistent/run")
        )
        .parent(),
        std::path::Path::new(SOCK).parent()
    );
}

#[test]
fn golden_child_env() {
    let mut host: Vec<(OsString, OsString)> = STRIP_ENV
        .iter()
        .map(|k| (OsString::from(k), OsString::from("host-value")))
        .collect();
    host.push(("ANTHROPIC_AUTH_TOKEN".into(), "host-auth".into()));
    host.push(("PATH".into(), "/usr/bin".into()));
    host.push(("HOME".into(), "/home/u".into()));
    // A host value must not be able to switch the prompt back on.
    host.push(("GIT_TERMINAL_PROMPT".into(), "1".into()));
    let env = child_env(
        host,
        &[
            ("CLAUDE_CONFIG_DIR", OsString::from("/cfg")),
            ("CLAUDE_CODE_OAUTH_TOKEN", OsString::from("tok")),
        ],
    );
    let expected: BTreeMap<OsString, OsString> = [
        ("CLAUDE_CODE_OAUTH_TOKEN", "tok"),
        ("CLAUDE_CONFIG_DIR", "/cfg"),
        ("GIT_TERMINAL_PROMPT", "0"),
        ("HOME", "/home/u"),
        ("PATH", "/usr/bin"),
    ]
    .iter()
    .map(|(k, v)| (OsString::from(k), OsString::from(v)))
    .collect();
    assert_eq!(env, expected);
    assert_eq!(EXTRA_STRIP_ENV, ["ANTHROPIC_AUTH_TOKEN"]);
}

const ROUTING_NAMES: [&str; 9] = [
    "TMUX",
    "TMUX_PANE",
    "ALL_PROXY",
    "all_proxy",
    "CLAUDE_CODE_USE_BEDROCK",
    "CLAUDE_CODE_USE_VERTEX",
    "CLAUDE_CODE_USE_FOUNDRY",
    "ANTHROPIC_CUSTOM_HEADERS",
    "ANTHROPIC_BASE_URL",
];

#[test]
fn routing_variables_are_stripped_but_host_proxies_are_kept() {
    assert_eq!(ROUTING_STRIP_ENV, ROUTING_NAMES);
    let mut host: Vec<(OsString, OsString)> = ROUTING_NAMES
        .iter()
        .map(|k| (OsString::from(k), OsString::from("host-routing")))
        .collect();
    host.push(("PATH".into(), "/usr/bin".into()));
    host.push(("HTTPS_PROXY".into(), "http://host-proxy:8080".into()));
    let env = child_env(host, &[]);
    let keys: Vec<String> = env
        .keys()
        .map(|k| k.to_string_lossy().into_owned())
        .collect();
    // The degraded posture keeps the host's own proxies; nothing else survives.
    assert_eq!(keys, ["GIT_TERMINAL_PROMPT", "HTTPS_PROXY", "PATH"]);
    assert_eq!(
        env[&OsString::from("HTTPS_PROXY")],
        "http://host-proxy:8080"
    );
}

#[test]
fn golden_env_sets() {
    let common: Vec<(String, String)> = common_env_sets()
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_string_lossy().into_owned()))
        .collect();
    let want = |k: &str, v: &str| (k.to_owned(), v.to_owned());
    assert_eq!(
        common,
        [
            want("DISABLE_TELEMETRY", "1"),
            want("DISABLE_ERROR_REPORTING", "1"),
            want("DISABLE_AUTOUPDATER", "1"),
            want("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1"),
            want("ENABLE_CLAUDEAI_MCP_SERVERS", "false"),
        ]
    );

    // The bwrap posture: the host's routing values are gone and the launcher's win.
    let mut host: Vec<(OsString, OsString)> = ROUTING_NAMES
        .iter()
        .map(|k| (OsString::from(k), OsString::from("host-routing")))
        .collect();
    host.push(("HTTPS_PROXY".into(), "http://host-proxy:8080".into()));
    host.push(("NO_PROXY".into(), "internal.example".into()));
    host.push(("DISABLE_TELEMETRY".into(), "0".into()));
    let sets = [common_env_sets(), egress_env_sets(3128)].concat();
    let env = child_env(host, &sets);
    let url = "http://127.0.0.1:3128";
    let expected: BTreeMap<OsString, OsString> = [
        ("ANTHROPIC_BASE_URL", url),
        ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1"),
        ("CLAUDE_CODE_GZIP_REQUEST_BODIES", "0"),
        ("CLAUDE_CODE_OAUTH_TOKEN", DUMMY_OAUTH_TOKEN),
        ("DISABLE_AUTOUPDATER", "1"),
        ("DISABLE_ERROR_REPORTING", "1"),
        ("DISABLE_TELEMETRY", "1"),
        ("ENABLE_CLAUDEAI_MCP_SERVERS", "false"),
        ("GIT_TERMINAL_PROMPT", "0"),
        ("HTTPS_PROXY", url),
        ("HTTP_PROXY", url),
        ("NO_PROXY", "127.0.0.1,localhost"),
        ("http_proxy", url),
        ("https_proxy", url),
        ("no_proxy", "127.0.0.1,localhost"),
    ]
    .iter()
    .map(|(k, v)| (OsString::from(k), OsString::from(v)))
    .collect();
    assert_eq!(env, expected);
}

#[test]
fn claude_argv_restricts_setting_sources() {
    let argv = strings(&claude_argv("do it", "opus", 10.0));
    let at = argv
        .iter()
        .position(|a| a == "--setting-sources")
        .expect("--setting-sources is present");
    assert_eq!(argv[at + 1], "user");
    assert_eq!(argv[..2], ["claude", "-p"]);
    assert_eq!(argv[argv.len() - 2..], ["--", "do it"]);
}

// ---------------------------------------------------------------------------
// Live smoke
// ---------------------------------------------------------------------------

fn bwrap_runs_here() -> Result<(), String> {
    match Command::new("bwrap")
        .args(["--ro-bind", "/", "/", "--dev", "/dev", "--", "true"])
        .stdin(Stdio::null())
        .output()
    {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(format!(
            "bwrap cannot run a trivial command here ({}): {}",
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Err(e) => Err(format!("bwrap is not runnable: {e}")),
    }
}

#[test]
#[ignore = "needs bwrap; run with -- --ignored --nocapture"]
fn runtime_dir_holds_only_the_driverctl_socket() {
    if let Err(reason) = bwrap_runs_here() {
        println!("SKIP: {reason}");
        return;
    }
    let runtime = match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) if !dir.is_empty() => PathBuf::from(dir),
        _ => {
            println!("SKIP: XDG_RUNTIME_DIR is not set");
            return;
        }
    };
    assert!(runtime.is_dir(), "XDG_RUNTIME_DIR names no directory");

    // bwrap's `--tmpfs /tmp` hides /tmp, so the fixture lives under the target dir.
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("launcher-sandbox");
    fs::create_dir_all(&base).unwrap();
    let scratch = TempDir::new_in(&base).unwrap();
    let root = scratch.path().canonicalize().unwrap();
    let project = root.join("project");
    let run_dir = project.join(".planning/meta-manager/drivers/probe28");
    let config_dir = root.join("config");
    let home = root.join("home");
    for dir in [
        &run_dir,
        &config_dir.join("bin"),
        &config_dir.join("managed"),
        &home,
    ] {
        fs::create_dir_all(dir).unwrap();
    }
    fs::write(config_dir.join("settings.json"), "{}").unwrap();
    fs::write(run_dir.join("launcher.lock"), "").unwrap();
    fs::write(run_dir.join("DIRECTION.lock"), "").unwrap();

    let sock = socket_path("probe28", &run_dir).expect("the run's socket path");
    let _ = fs::remove_file(&sock);
    let listener = UnixListener::bind(&sock).expect("bind the probe socket");
    let egress_sock = socket_path_for(&sock, &run_dir);
    let _ = fs::remove_file(&egress_sock);
    let egress_listener = UnixListener::bind(&egress_sock).expect("bind the probe egress socket");

    let spec = SandboxSpec::from_host(
        LaunchPaths {
            project: project.clone(),
            run_dir,
            config_dir,
            socket: sock.clone(),
            egress_socket: egress_sock.clone(),
        },
        &home,
        &HomeSetup::default(),
    );
    let script = format!(
        "ls -A \"$XDG_RUNTIME_DIR\"; ls -A \"$(dirname '{sock}')\"; \
         test -S '{sock}' && echo SOCK_OK; test -S '{egress}' && echo EGRESS_OK",
        sock = sock.display(),
        egress = egress_sock.display()
    );
    let argv = bwrap_argv(
        &spec,
        &[
            OsString::from("sh"),
            OsString::from("-c"),
            OsString::from(script),
        ],
    );
    let (program, args) = argv.split_first().unwrap();
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("run bwrap");
    drop(listener);
    drop(egress_listener);
    let _ = fs::remove_file(&sock);
    let _ = fs::remove_file(&egress_sock);

    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    println!("LIVE runtime dir inside the sandbox:\n{stdout}");
    assert!(
        out.status.success(),
        "bwrap failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // `ls -A` sorts: the runtime dir holds only `gsdmm-driverctl`, and that holds
    // exactly the two sockets of the run (the egress one next to the driverctl one).
    let mut names = [
        sock.file_name().unwrap().to_string_lossy().into_owned(),
        egress_sock
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned(),
    ];
    names.sort();
    assert_eq!(
        stdout,
        format!(
            "gsdmm-driverctl\n{}\n{}\nSOCK_OK\nEGRESS_OK\n",
            names[0], names[1]
        )
    );
}
