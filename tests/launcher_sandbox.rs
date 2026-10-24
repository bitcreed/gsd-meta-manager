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
use gsd_meta_manager::launcher::sandbox::{
    bridged_command, bwrap_argv, child_env, claude_argv, egress_env_sets, etc_entries_in, EtcEntry,
    LaunchPaths, SandboxSpec, BRIDGE_PORT, DUMMY_OAUTH_TOKEN, EXTRA_STRIP_ENV, STRIP_ENV,
};
use tempfile::TempDir;

const RUN: &str = "/work/proj/.planning/meta-manager/drivers/r1";
const CFG: &str = "/data/driver/ab12cd34/r1/claude-config";
const SOCK: &str = "/run/user/1000/gsdmm-driverctl/r1-ab12cd34.sock";
const EGRESS: &str = "/run/user/1000/gsdmm-driverctl/egress-ab12cd34.sock";

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
        tmpfs_masks: [".ssh", ".gnupg", ".config/gh", ".aws", ".docker"]
            .iter()
            .map(|rel| PathBuf::from("/home/u").join(rel))
            .collect(),
        null_masks: [
            ".claude/.credentials.json",
            ".claude.json",
            ".git-credentials",
            ".config/git/credentials",
            ".netrc",
        ]
        .iter()
        .map(|rel| PathBuf::from("/home/u").join(rel))
        .collect(),
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
        // home masks: directories, then files
        "--tmpfs",
        "/home/u/.ssh",
        "--tmpfs",
        "/home/u/.gnupg",
        "--tmpfs",
        "/home/u/.config/gh",
        "--tmpfs",
        "/home/u/.aws",
        "--tmpfs",
        "/home/u/.docker",
        "--ro-bind",
        "/dev/null",
        "/home/u/.claude/.credentials.json",
        "--ro-bind",
        "/dev/null",
        "/home/u/.claude.json",
        "--ro-bind",
        "/dev/null",
        "/home/u/.git-credentials",
        "--ro-bind",
        "/dev/null",
        "/home/u/.config/git/credentials",
        "--ro-bind",
        "/dev/null",
        "/home/u/.netrc",
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
fn masks_are_omitted_when_absent() {
    let home = TempDir::new().unwrap();
    let spec = SandboxSpec::from_host(paths(), home.path());
    assert!(spec.tmpfs_masks.is_empty());
    assert!(spec.null_masks.is_empty());
    let argv = strings(&bwrap_argv(&spec, &[OsString::from("true")]));
    let home_str = home.path().display().to_string();
    assert!(
        !argv.iter().any(|a| a.starts_with(&home_str)),
        "an empty home must add no mask flag: {argv:?}"
    );
    // The fixed prefix is exactly root, /dev, /proc, /tmp, then the /etc block.
    assert_eq!(
        argv[..10],
        [
            "bwrap",
            "--ro-bind",
            "/",
            "/",
            "--dev",
            "/dev",
            "--proc",
            "/proc",
            "--tmpfs",
            "/tmp"
        ]
    );
    // Nothing between the /etc block and the project bind: no mask flag at all.
    let remount = argv
        .windows(2)
        .position(|w| w == ["--remount-ro", "/etc"])
        .expect("the /etc block ends with --remount-ro /etc");
    assert_eq!(argv[remount + 2], "--bind");
    assert_eq!(argv[remount + 3], "/work/proj");
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
        tmpfs_masks: Vec::new(),
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
        tmpfs_masks: Vec::new(),
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
        tmpfs_masks: Vec::new(),
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
