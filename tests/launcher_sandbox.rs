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
use gsd_meta_manager::launcher::sandbox::{
    bwrap_argv, child_env, claude_argv, LaunchPaths, SandboxSpec, EXTRA_STRIP_ENV, STRIP_ENV,
};
use tempfile::TempDir;

const RUN: &str = "/work/proj/.planning/meta-manager/drivers/r1";
const CFG: &str = "/data/driver/ab12cd34/r1/claude-config";
const SOCK: &str = "/run/user/1000/gsdmm-driverctl/r1-ab12cd34.sock";

fn paths() -> LaunchPaths {
    LaunchPaths {
        project: PathBuf::from("/work/proj"),
        run_dir: PathBuf::from(RUN),
        config_dir: PathBuf::from(CFG),
        socket: PathBuf::from(SOCK),
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
        // project rw, run dir ro, the two lock masks
        "--bind",
        "/work/proj",
        "/work/proj",
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
        // runtime dir tmpfs, then only the driverctl socket
        "--tmpfs",
        "/run/user/1000",
        "--bind",
        SOCK,
        SOCK,
        "--unshare-pid",
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
    // The fixed prefix up to the project bind is exactly root, /dev, /proc, /tmp.
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
    assert_eq!(argv[10], "--bind");
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
    };
    let argv = strings(&bwrap_argv(&spec, &[OsString::from("true")]));
    assert!(argv.iter().any(|a| a == "/run/user/1000"));
    assert!(!argv.iter().any(|a| a == "/elsewhere/r1.sock"));
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
    assert_eq!(argv[..3], ["claude", "-p", "do it"]);
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
    for dir in [&run_dir, &config_dir.join("bin"), &home] {
        fs::create_dir_all(dir).unwrap();
    }
    fs::write(config_dir.join("settings.json"), "{}").unwrap();
    fs::write(run_dir.join("launcher.lock"), "").unwrap();
    fs::write(run_dir.join("DIRECTION.lock"), "").unwrap();

    let sock = socket_path("probe28", &run_dir).expect("the run's socket path");
    let _ = fs::remove_file(&sock);
    let listener = UnixListener::bind(&sock).expect("bind the probe socket");

    let spec = SandboxSpec::from_host(
        LaunchPaths {
            project: project.clone(),
            run_dir,
            config_dir,
            socket: sock.clone(),
        },
        &home,
    );
    let script = format!(
        "ls -A \"$XDG_RUNTIME_DIR\"; test -S '{}' && echo SOCK_OK",
        sock.display()
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
    let _ = fs::remove_file(&sock);

    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    println!("LIVE runtime dir inside the sandbox:\n{stdout}");
    assert!(
        out.status.success(),
        "bwrap failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(stdout, "gsdmm-driverctl\nSOCK_OK\n");
}
