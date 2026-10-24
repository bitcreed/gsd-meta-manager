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
// name read from stdin), so no real driverctl server is involved.
//
// 28.1-10 (D-04): the sandbox has no network and no real token. It holds the dummy
// `CLAUDE_CODE_OAUTH_TOKEN`, reaches the API only through the in-sandbox bridge and a
// launcher-side `EgressServer` that injects the real token over TLS, and the test
// proves both that the nested claude still completes and that A5 holds under that
// profile. The control is unchanged: the same command on the host, with the real
// token, where no managed file is reachable.
// ============================================================================

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use chrono::Utc;
use gsd_meta_manager::driverctl::serve::socket_path;
use gsd_meta_manager::launcher::auth::{resolve_launch_token, TokenSource};
use gsd_meta_manager::launcher::config_dir::{seed, SeedSpec, HOST_MANAGED_DIR};
use gsd_meta_manager::launcher::egress::upstream::Upstream;
use gsd_meta_manager::launcher::egress::{
    socket_path_for, EgressServer, EgressSpec, TokenProvider,
};
use gsd_meta_manager::launcher::home::HomeSetup;
use gsd_meta_manager::launcher::host::detect;
use gsd_meta_manager::launcher::sandbox::{
    bridged_command, bwrap_argv, child_env, common_env_sets, egress_env_sets, LaunchPaths,
    SandboxSpec, BRIDGE_PORT, DUMMY_OAUTH_TOKEN,
};
use tempfile::TempDir;

/// The nested claude. `CLAUDE_CONFIG_DIR` is foreign, project settings are the only
/// source, and `--settings` tries to turn every hook off. `claude` is resolved from
/// the environment `PATH`.
const CLAUDE_NESTED: &str = r#"CLAUDE_CONFIG_DIR=$(mktemp -d /tmp/foreign.XXXXXX) claude -p --model haiku --setting-sources project --settings '{"disableAllHooks":true}' --permission-mode bypassPermissions --max-budget-usd 0.05 -- 'Run the shell command true with the Bash tool, then reply DONE.'"#;

/// What runs inside the sandbox: says whether the token it holds is the dummy, then
/// the nested claude, then its exit code.
fn nested_script() -> String {
    format!(
        "if [ \"$CLAUDE_CODE_OAUTH_TOKEN\" = '{DUMMY_OAUTH_TOKEN}' ]; then echo token_is_dummy=yes; \
         else echo token_is_dummy=no; fi\n{CLAUDE_NESTED}\nrc=$?\necho claude_rc=$rc\nexit $rc\n"
    )
}

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
    match std::env::var_os("XDG_RUNTIME_DIR") {
        Some(dir) if !dir.is_empty() && Path::new(&dir).is_dir() => {}
        _ => {
            println!("SKIP: XDG_RUNTIME_DIR is not set to a directory");
            return;
        }
    }
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

    // The run's two sockets live in the runtime dir's private directory, as in a
    // launch: a dummy driverctl listener, and the egress server on its own path.
    let socket = socket_path("managed281", &run_dir).expect("the run's driverctl socket path");
    let _ = fs::remove_file(&socket);
    let _driverctl = UnixListener::bind(&socket).expect("bind the driverctl socket");
    let egress_sock = socket_path_for(&socket, &run_dir);
    let _ = fs::remove_file(&egress_sock);
    let egress_listener = UnixListener::bind(&egress_sock).expect("bind the egress socket");
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

    // The real token stays out here: only the egress server holds it.
    let provider = match token.source {
        TokenSource::Stored => TokenProvider::Static(token.value.clone()),
        TokenSource::Host => TokenProvider::Host {
            claude_dir: home.join(".claude"),
            initial: token.value.clone(),
        },
    };
    let egress_log = root.join("egress.log");
    let egress = EgressServer::start(
        egress_listener,
        EgressSpec {
            allow_hosts: Vec::new(),
            token: provider,
            upstream: Upstream::Tls {
                host: "api.anthropic.com".to_owned(),
                port: 443,
            },
            log_path: egress_log.clone(),
        },
    )
    .expect("start the egress server");

    let mut path = config_dir.join("bin").into_os_string();
    if let Some(host_path) = std::env::var_os("PATH") {
        path.push(":");
        path.push(host_path);
    }
    let sets = [
        vec![
            ("CLAUDE_CONFIG_DIR", config_dir.clone().into_os_string()),
            ("PATH", path),
        ],
        common_env_sets(),
        egress_env_sets(BRIDGE_PORT),
    ]
    .concat();
    let env = child_env(std::env::vars_os(), &sets);

    // ---- the managed run: nested claude inside the network-isolated sandbox ---
    let home_setup = HomeSetup::for_host(
        &home,
        Path::new(env!("CARGO_BIN_EXE_gsd-meta-manager")),
        detect().overlay,
        &root,
    )
    .expect("derive the $HOME setup");
    let spec = SandboxSpec::from_host(
        LaunchPaths {
            project: project.clone(),
            run_dir: run_dir.clone(),
            config_dir: config_dir.clone(),
            socket,
            egress_socket: egress_sock.clone(),
        },
        &home,
        &home_setup,
    );
    let inner = vec!["sh".into(), "-c".into(), nested_script().into()];
    let command = bridged_command(
        Path::new(env!("CARGO_BIN_EXE_gsd-meta-manager")),
        BRIDGE_PORT,
        &egress_sock,
        &inner,
    );
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
    let stats = egress.stop();
    println!("LIVE sandbox exit: {}", out.status);
    let sandbox_stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    println!(
        "LIVE sandbox stdout (last 200 bytes): {}",
        tail(&sandbox_stdout, 200)
    );
    let token_is_dummy = sandbox_stdout
        .lines()
        .find_map(|l| l.strip_prefix("token_is_dummy="))
        .unwrap_or("<missing>")
        .to_owned();
    let claude_rc = sandbox_stdout
        .lines()
        .find_map(|l| l.strip_prefix("claude_rc="))
        .unwrap_or("<missing>")
        .to_owned();
    println!("token_is_dummy={token_is_dummy}");
    println!("claude_rc={claude_rc}");
    let managed_lines = lines_of(&sandbox_log);
    for line in &managed_lines {
        println!("LIVE hook log: {line}");
    }
    let managed_hook_lines = managed_lines
        .iter()
        .filter(|l| l.contains("PostToolUse"))
        .count();
    println!("managed_hook_lines={managed_hook_lines}");
    println!("inference_forwarded={}", stats.inference_forwarded);
    let egress_lines = lines_of(&egress_log);
    for line in &egress_lines {
        println!("LIVE egress.log: {line}");
    }
    let inference_200 = egress_lines
        .iter()
        .filter(|l| {
            let f: Vec<&str> = l.split('|').collect();
            f.get(1) == Some(&"inference") && f.get(4) == Some(&"200")
        })
        .count();
    let direct_inference_connects = egress_lines
        .iter()
        .filter(|l| l.contains("api.anthropic.com:443"))
        .count();
    println!("egress_inference_200_lines={inference_200}");
    println!("egress_direct_inference_connects={direct_inference_connects}");

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
        .args(["-c", CLAUDE_NESTED])
        .env_clear()
        .envs(&control_env)
        .current_dir(&project)
        .stdin(Stdio::null())
        .output()
        .expect("run the control");
    println!("LIVE control exit: {}", control.status);
    let control_lines = lines_of(&control_log).len();
    println!("control_lines={control_lines}");

    assert_eq!(
        token_is_dummy, "yes",
        "the sandbox must hold the dummy token, never the real one"
    );
    assert_eq!(
        claude_rc,
        "0",
        "the nested claude did not complete through the bridge (stderr: {})",
        tail(&String::from_utf8_lossy(&out.stderr), 400)
    );
    assert!(
        managed_hook_lines >= 1,
        "the nested claude never reached the managed gate hook (stderr: {})",
        tail(&String::from_utf8_lossy(&out.stderr), 400)
    );
    assert!(
        stats.inference_forwarded >= 1,
        "no inference request went through the egress server"
    );
    assert!(
        inference_200 >= 1,
        "egress.log has no inference line with status 200"
    );
    assert_eq!(
        direct_inference_connects, 0,
        "the sandbox opened a CONNECT to the inference host"
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
