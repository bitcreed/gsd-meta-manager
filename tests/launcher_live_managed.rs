#![cfg(target_os = "linux")]
// ============================================================================
// LIVE proof that the gate's hooks live in Claude Code's managed tier (Phase 28.1,
// plan 28.1-02; D-12, A5, T-28.1-09). Opt-in: the test is `#[ignore]` because it
// needs a working `bwrap`, the `claude` CLI and a launch token, and it spends about
// $0.05 on Haiku.
//
//   cargo test --test launcher_live_managed -- --ignored --nocapture
//
// Run it OUTSIDE any tool sandbox. A test that cannot run prints a `SKIP:` line and
// returns; a `SKIP:` line is no evidence, it means this host proved nothing.
//
// What it shows: a nested `claude` inside the sandbox, started with a foreign
// `CLAUDE_CONFIG_DIR`, `--setting-sources project` and `--settings
// '{"disableAllHooks":true}'`, still fires the gate's managed `PostToolUse` hook,
// because the managed file reaches it through `/etc/claude-code`. The control runs
// the same command on the host, where no managed file is reachable: it must add
// zero lines to the hook log, so the lines of the first run come from the managed
// tier and from nothing else.
//
// The gate binary is a logger script (one line per call: argv and the hook event
// name read from stdin), so no real driverctl server is involved. This wave still
// has an open network; 28.1-10 reroutes the test through the egress proxy.
// ============================================================================

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use chrono::Utc;
use gsd_meta_manager::launcher::auth::resolve_launch_token;
use gsd_meta_manager::launcher::config_dir::{seed, SeedSpec, HOST_MANAGED_DIR};
use gsd_meta_manager::launcher::sandbox::{bwrap_argv, child_env, LaunchPaths, SandboxSpec};
use tempfile::TempDir;

/// The nested claude. `CLAUDE_CONFIG_DIR` is foreign, project settings are the only
/// source, and `--settings` tries to turn every hook off. `claude` is resolved from
/// the environment `PATH`.
const NESTED: &str = r#"CLAUDE_CONFIG_DIR=$(mktemp -d /tmp/foreign.XXXXXX) claude -p --model haiku --setting-sources project --settings '{"disableAllHooks":true}' --permission-mode bypassPermissions --max-budget-usd 0.05 -- 'Run the shell command true with the Bash tool, then reply DONE.'"#;

/// A logger standing in for the gate binary: appends one line per call holding its
/// argv and the `hook_event_name` it finds on stdin, and answers with an empty stdout
/// (an allow).
fn logger_script(log: &Path) -> String {
    format!(
        "#!/bin/sh\nev=$(cat | grep -o '\"hook_event_name\" *: *\"[A-Za-z]*\"' | head -n 1)\necho \"argv=$* event=$ev\" >> '{}'\nexit 0\n",
        log.display()
    )
}

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

fn claude_answers() -> Result<(), String> {
    match Command::new("claude")
        .arg("--version")
        .stdin(Stdio::null())
        .output()
    {
        Ok(out) if out.status.success() => Ok(()),
        Ok(out) => Err(format!("claude --version exited {}", out.status)),
        Err(e) => Err(format!("claude is not runnable: {e}")),
    }
}

fn lines_of(log: &Path) -> Vec<String> {
    fs::read_to_string(log)
        .unwrap_or_default()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
#[ignore = "needs bwrap, claude and a token; spends about $0.05 on Haiku; run with -- --ignored --nocapture"]
fn a_nested_claude_cannot_shed_the_managed_gate_hook() {
    if let Err(reason) = bwrap_runs_here() {
        println!("SKIP: {reason}");
        return;
    }
    if let Err(reason) = claude_answers() {
        println!("SKIP: {reason}");
        return;
    }
    let Some(home) = dirs::home_dir() else {
        println!("SKIP: no home directory");
        return;
    };
    let token = match resolve_launch_token(Utc::now()) {
        Ok(token) => token,
        Err(e) => {
            println!("SKIP: no launch token resolves ({e})");
            return;
        }
    };
    if Path::new(HOST_MANAGED_DIR)
        .join("managed-settings.json")
        .exists()
    {
        println!("SKIP: the host has its own managed settings, so the control would prove nothing");
        return;
    }

    // bwrap's `--tmpfs /tmp` hides /tmp, so the fixture lives under the target dir.
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("launcher-live-managed");
    fs::create_dir_all(&base).unwrap();
    let scratch = TempDir::new_in(&base).unwrap();
    let root = scratch.path().canonicalize().unwrap();
    let project = root.join("project");
    let run_dir = project.join(".planning/meta-manager/drivers/managed281");
    let config_dir = root.join("config");
    fs::create_dir_all(&run_dir).unwrap();
    fs::write(run_dir.join("launcher.lock"), "").unwrap();
    fs::write(run_dir.join("DIRECTION.lock"), "").unwrap();

    // The sandbox run logs into the writable config dir; the control logs elsewhere.
    let sandbox_log = config_dir.join("hook.log");
    let control_log = root.join("control-hook.log");
    let logger = root.join("logger.sh");
    fs::write(&logger, logger_script(&sandbox_log)).unwrap();
    fs::set_permissions(&logger, fs::Permissions::from_mode(0o755)).unwrap();

    let socket = root.join("driverctl.sock");
    seed(&SeedSpec {
        config_dir: config_dir.clone(),
        host_claude_dir: home.join(".claude"),
        binary: logger.clone(),
        run_dir: run_dir.clone(),
        socket: socket.clone(),
        managed: true,
        host_managed_dir: PathBuf::from(HOST_MANAGED_DIR),
    })
    .expect("seed the per-run config dir");

    let mut path = config_dir.join("bin").into_os_string();
    if let Some(host_path) = std::env::var_os("PATH") {
        path.push(":");
        path.push(host_path);
    }
    let env = child_env(
        std::env::vars_os(),
        &[
            ("CLAUDE_CONFIG_DIR", config_dir.clone().into_os_string()),
            ("PATH", path),
            ("CLAUDE_CODE_OAUTH_TOKEN", token.value.clone().into()),
        ],
    );

    // ---- the managed run: nested claude inside the sandbox -------------------
    let spec = SandboxSpec::from_host(
        LaunchPaths {
            project: project.clone(),
            run_dir: run_dir.clone(),
            config_dir: config_dir.clone(),
            socket,
            egress_socket: root.join("egress.sock"),
        },
        &home,
    );
    let command = vec!["sh".into(), "-c".into(), NESTED.into()];
    let argv = bwrap_argv(&spec, &command);
    let (program, rest) = argv.split_first().unwrap();
    let out = Command::new(program)
        .args(rest)
        .env_clear()
        .envs(&env)
        .current_dir(&project)
        .stdin(Stdio::null())
        .output()
        .expect("run bwrap");
    println!("LIVE sandbox exit: {}", out.status);
    println!(
        "LIVE sandbox stdout (last 200 bytes): {}",
        tail(&String::from_utf8_lossy(&out.stdout), 200)
    );
    let managed_lines = lines_of(&sandbox_log);
    for line in &managed_lines {
        println!("LIVE hook log: {line}");
    }
    let managed_hook_lines = managed_lines
        .iter()
        .filter(|l| l.contains("PostToolUse"))
        .count();
    println!("managed_hook_lines={managed_hook_lines}");

    // ---- the control: the same command on the host, no managed file ----------
    let control_bin = root.join("control-bin");
    fs::create_dir_all(&control_bin).unwrap();
    let control_logger = control_bin.join("driverctl");
    fs::write(&control_logger, logger_script(&control_log)).unwrap();
    fs::set_permissions(&control_logger, fs::Permissions::from_mode(0o755)).unwrap();
    let mut control_path = control_bin.into_os_string();
    if let Some(host_path) = std::env::var_os("PATH") {
        control_path.push(":");
        control_path.push(host_path);
    }
    let control_env = child_env(
        std::env::vars_os(),
        &[
            ("PATH", control_path),
            ("CLAUDE_CODE_OAUTH_TOKEN", token.value.clone().into()),
        ],
    );
    let control = Command::new("sh")
        .args(["-c", NESTED])
        .env_clear()
        .envs(&control_env)
        .current_dir(&project)
        .stdin(Stdio::null())
        .output()
        .expect("run the control");
    println!("LIVE control exit: {}", control.status);
    let control_lines = lines_of(&control_log).len();
    println!("control_lines={control_lines}");

    assert!(
        managed_hook_lines >= 1,
        "the nested claude never reached the managed gate hook (stderr: {})",
        tail(&String::from_utf8_lossy(&out.stderr), 400)
    );
    assert_eq!(
        control_lines, 0,
        "the control reached the gate without the managed tier, so the first count proves nothing"
    );
}

fn tail(text: &str, max: usize) -> String {
    let text = text.trim();
    let start = text.len().saturating_sub(max);
    let start = (start..=text.len())
        .find(|i| text.is_char_boundary(*i))
        .unwrap_or(text.len());
    text[start..].replace('\n', " / ")
}
