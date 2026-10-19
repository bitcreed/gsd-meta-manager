#![cfg(target_os = "linux")]
// ============================================================================
// LIVE proof of the sandbox's isolation (Phase 28, plan 28-03; SC3, D-19, D-20,
// D-27). Opt-in: the test is `#[ignore]` because it needs a working `bwrap`, a
// user systemd manager and the network, none of which a CI runner has.
//
//   cargo test --test launcher_live_isolation -- --ignored --nocapture
//
// Run it OUTSIDE any tool sandbox. A test that cannot run prints a `SKIP:` line
// and returns; a `SKIP:` line is no evidence, it means this host proved nothing.
//
// The probe uses the REAL `HOME` read-only: it prints exit codes, entry counts
// and byte counts, never the content of a host file. It writes nothing to the
// host except under `CARGO_TARGET_TMPDIR`.
// ============================================================================

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use gsd_meta_manager::driverctl::serve::socket_path;
use gsd_meta_manager::launcher::config_dir::{seed, SeedSpec};
use gsd_meta_manager::launcher::sandbox::{
    bwrap_argv, child_env, systemd_run_argv, LaunchPaths, SandboxSpec,
};
use tempfile::TempDir;

/// The in-sandbox probe. `$1` run dir, `$2` config dir, `$3` host home.
/// Every line is `KEY=VALUE`; no host file content is printed.
const PROBE: &str = r#"
run_dir=$1; cfg=$2; home=$3
touch_state() {
  out=$(touch "$1" 2>&1) && { echo WRITABLE; rm -f "$1"; return; }
  case "$out" in
    *"Read-only file system"*) echo EROFS ;;
    *) echo "FAILED:$out" ;;
  esac
}
append_state() {
  out=$( { : >> "$1"; } 2>&1 ); rc=$?
  if [ $rc -eq 0 ]; then echo WRITABLE; return; fi
  case "$out" in
    *"Read-only file system"*) echo EROFS ;;
    *) echo "FAILED:$out" ;;
  esac
}
read_rc() {
  if [ -e "$1" ]; then cat "$1" > /dev/null 2>&1; echo $?; else echo absent; fi
}
echo "claude_touch=$(touch_state "$home/.claude/sandbox-probe")"
echo "creds_read=$(read_rc "$home/.claude/.credentials.json")"
echo "claude_json_read=$(read_rc "$home/.claude.json")"
echo "git_credentials_read=$(read_rc "$home/.git-credentials")"
echo "netrc_read=$(read_rc "$home/.netrc")"
echo "ssh_entries=$(ls -A "$home/.ssh" 2>/dev/null | wc -l | tr -d ' ')"
if [ -d "$home/.config/gh" ]; then
  echo "gh_entries=$(ls -A "$home/.config/gh" 2>/dev/null | wc -l | tr -d ' ')"
else
  echo "gh_entries=absent"
fi
echo "env_ssh_auth_sock=${SSH_AUTH_SOCK:-}"
echo "env_gh_token=${GH_TOKEN:-}"
echo "env_anthropic_api_key=${ANTHROPIC_API_KEY:-}"
echo "env_anthropic_auth_token=${ANTHROPIC_AUTH_TOKEN:-}"
echo "env_git_terminal_prompt=${GIT_TERMINAL_PROMPT:-}"
echo "run_dir_touch=$(touch_state "$run_dir/sandbox-probe")"
echo "lock_bytes=$(stat -c %s "$run_dir/launcher.lock")"
echo "lock_read=$(read_rc "$run_dir/launcher.lock")"
echo "direction_lock_bytes=$(stat -c %s "$run_dir/DIRECTION.lock")"
echo "cfg_touch=$(touch_state "$cfg/sandbox-probe")"
echo "settings_touch=$(append_state "$cfg/settings.json")"
echo "bin_touch=$(touch_state "$cfg/bin/sandbox-probe")"
echo "runtime_entries=$(ls -A "$XDG_RUNTIME_DIR" | tr '\n' ' ' | sed 's/ *$//')"
if [ -S "$XDG_RUNTIME_DIR/bus" ]; then echo "bus=present"; else echo "bus=absent"; fi
echo "user_manager=$(systemctl --user is-system-running 2>&1 | head -n 1)"
if [ -S "$DRIVERCTL_SOCK" ]; then echo "sock=present"; else echo "sock=absent"; fi
"#;

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

fn user_manager_state() -> String {
    Command::new("systemctl")
        .args(["--user", "is-system-running"])
        .stdin(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default()
}

/// `(owner, repo)` of a GitHub remote URL in either the scp, ssh or https form.
fn github_owner_repo(url: &str) -> Option<(String, String)> {
    let rest = url
        .strip_prefix("git@github.com:")
        .or_else(|| url.strip_prefix("ssh://git@github.com/"))
        .or_else(|| url.strip_prefix("https://github.com/"))?;
    let rest = rest.trim_end_matches('/');
    let rest = rest.strip_suffix(".git").unwrap_or(rest);
    let (owner, repo) = rest.split_once('/')?;
    (!owner.is_empty() && !repo.is_empty() && !repo.contains('/'))
        .then(|| (owner.to_owned(), repo.to_owned()))
}

fn git_ok(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=probe", "-c", "user.email=probe@localhost"])
        .args(["-c", "commit.gpgsign=false"])
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("run git");
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// What a sandboxed or host command produced.
struct Ran {
    success: bool,
    stdout: String,
    stderr: String,
}

/// The ssh options that make a failure mean "no credential", not "unknown host".
const GIT_SSH: &str =
    "ssh -o StrictHostKeyChecking=accept-new -o BatchMode=yes -o UserKnownHostsFile=/dev/null";

fn push_command(url: &str) -> String {
    format!("timeout 60 git push --dry-run '{url}' HEAD:refs/heads/gsd-auto/probe")
}

fn last_lines(text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(3)..].join(" / ")
}

#[test]
#[ignore = "needs bwrap, a user systemd and network; run with -- --ignored --nocapture"]
fn sandbox_isolates_host_credentials_and_services() {
    if let Err(reason) = bwrap_runs_here() {
        println!("SKIP: {reason}");
        return;
    }
    let state = user_manager_state();
    if state != "running" && state != "degraded" {
        println!("SKIP: the user systemd manager is not usable (is-system-running: {state:?})");
        return;
    }
    let Some(home) = dirs::home_dir() else {
        println!("SKIP: no home directory");
        return;
    };
    let Some(runtime) = std::env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from) else {
        println!("SKIP: XDG_RUNTIME_DIR is not set");
        return;
    };
    let origin = Command::new("git")
        .args([
            "-C",
            env!("CARGO_MANIFEST_DIR"),
            "remote",
            "get-url",
            "origin",
        ])
        .stdin(Stdio::null())
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
        .unwrap_or_default();
    let Some((owner, repo)) = github_owner_repo(&origin) else {
        println!("SKIP: origin ({origin:?}) is not a GitHub URL");
        return;
    };
    let ssh_url = format!("git@github.com:{owner}/{repo}.git");
    let https_url = format!("https://github.com/{owner}/{repo}.git");

    // bwrap's `--tmpfs /tmp` hides /tmp, so the fixture lives under the target dir.
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("launcher-isolation");
    fs::create_dir_all(&base).unwrap();
    let scratch = TempDir::new_in(&base).unwrap();
    let root = scratch.path().canonicalize().unwrap();
    let project = root.join("project");
    let run_dir = project.join(".planning/meta-manager/drivers/probe28");
    let config_dir = root.join("config");
    fs::create_dir_all(&run_dir).unwrap();
    fs::write(
        run_dir.join("launcher.lock"),
        "launcher-lock-holder-record\n",
    )
    .unwrap();
    fs::write(
        run_dir.join("DIRECTION.lock"),
        "direction-lock-holder-record\n",
    )
    .unwrap();
    git_ok(&project, &["init", "-q"]);
    git_ok(&project, &["commit", "-q", "--allow-empty", "-m", "probe"]);

    seed(&SeedSpec {
        config_dir: config_dir.clone(),
        host_claude_dir: home.join(".claude"),
        binary: std::env::current_exe().unwrap(),
        run_dir: run_dir.clone(),
    })
    .expect("seed the per-run config dir");

    let sock = socket_path("probe28", &run_dir).expect("the run's socket path");
    let _ = fs::remove_file(&sock);
    let _listener = UnixListener::bind(&sock).expect("bind the probe socket");

    // The host environment, with the four secrets the launcher must keep out.
    let mut host_env: Vec<(OsString, OsString)> = std::env::vars_os().collect();
    for (key, value) in [
        ("SSH_AUTH_SOCK", "/run/probe/fake-agent.sock"),
        ("GH_TOKEN", "gh-token-probe-sentinel"),
        ("ANTHROPIC_API_KEY", "api-key-probe-sentinel"),
        ("ANTHROPIC_AUTH_TOKEN", "auth-token-probe-sentinel"),
    ] {
        host_env.retain(|(k, _)| k != key);
        host_env.push((key.into(), value.into()));
    }
    let mut path = config_dir.join("bin").into_os_string();
    if let Some(host_path) = std::env::var_os("PATH") {
        path.push(":");
        path.push(host_path);
    }
    let env = child_env(
        host_env,
        &[
            ("CLAUDE_CONFIG_DIR", config_dir.clone().into_os_string()),
            ("DRIVERCTL_SOCK", sock.clone().into_os_string()),
            ("PATH", path),
            ("GIT_SSH_COMMAND", GIT_SSH.into()),
        ],
    );

    let spec = SandboxSpec::from_host(
        LaunchPaths {
            project: project.clone(),
            run_dir: run_dir.clone(),
            config_dir: config_dir.clone(),
            socket: sock.clone(),
        },
        &home,
    );

    let pid = std::process::id();
    let mut n = 0u32;
    let mut in_sandbox = |script: &str, args: &[&Path]| -> Ran {
        n += 1;
        let mut command: Vec<OsString> =
            vec!["sh".into(), "-c".into(), script.into(), "probe".into()];
        command.extend(args.iter().map(|a| a.as_os_str().to_owned()));
        let argv = systemd_run_argv(
            &format!("gsdmm-probe-{pid}-{n}"),
            Some(120),
            &bwrap_argv(&spec, &command),
        );
        let (program, rest) = argv.split_first().unwrap();
        let out = Command::new(program)
            .args(rest)
            .env_clear()
            .envs(&env)
            .current_dir(&project)
            .stdin(Stdio::null())
            .output()
            .expect("run systemd-run");
        Ran {
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }
    };

    // ---- SC3 probe ---------------------------------------------------------
    let probe = in_sandbox(PROBE, &[&run_dir, &config_dir, &home]);
    println!("LIVE host user manager: {state}");
    println!("LIVE host XDG_RUNTIME_DIR entries: {}", {
        let mut names: Vec<String> = fs::read_dir(&runtime)
            .map(|d| {
                d.filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names.len()
    });
    println!(
        "LIVE host ~/.ssh entries: {}",
        fs::read_dir(home.join(".ssh")).map_or(0, Iterator::count)
    );
    print!("{}", probe.stdout);
    if !probe.stderr.trim().is_empty() {
        println!("LIVE probe stderr: {}", probe.stderr.trim());
    }
    let wrongly_created = home.join(".claude/sandbox-probe");
    let leaked = wrongly_created.exists();
    if leaked {
        let _ = fs::remove_file(&wrongly_created);
    }

    // ---- D-27 push probe ---------------------------------------------------
    let host_push = |url: &str| -> Ran {
        let out = Command::new("sh")
            .args(["-c", &push_command(url)])
            .current_dir(&project)
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_SSH_COMMAND", GIT_SSH)
            .stdin(Stdio::null())
            .output()
            .expect("run git on the host");
        Ran {
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
        }
    };
    let host_ssh = host_push(&ssh_url);
    let host_https = host_push(&https_url);
    println!(
        "LIVE host control push --dry-run over ssh:   success={} ({})",
        host_ssh.success,
        last_lines(&host_ssh.stderr)
    );
    println!(
        "LIVE host control push --dry-run over https: success={} ({})",
        host_https.success,
        last_lines(&host_https.stderr)
    );
    if !host_ssh.success && !host_https.success {
        println!("SKIP: neither scheme can push (dry-run) from the host, so the inside failure would prove nothing");
        return;
    }
    let inside_ssh = in_sandbox(&push_command(&ssh_url), &[]);
    let inside_https = in_sandbox(&push_command(&https_url), &[]);
    println!(
        "LIVE inside push --dry-run over ssh:   success={} ({})",
        inside_ssh.success,
        last_lines(&inside_ssh.stderr)
    );
    println!(
        "LIVE inside push --dry-run over https: success={} ({})",
        inside_https.success,
        last_lines(&inside_https.stderr)
    );

    drop(_listener);
    let _ = fs::remove_file(&sock);

    // ---- assertions --------------------------------------------------------
    assert!(
        !leaked,
        "~/.claude/sandbox-probe was created: the host ~/.claude is writable inside"
    );
    assert!(probe.success, "the probe run failed: {}", probe.stderr);
    let kv: BTreeMap<&str, &str> = probe
        .stdout
        .lines()
        .filter_map(|l| l.split_once('='))
        .collect();
    let get = |key: &str| -> &str {
        kv.get(key)
            .copied()
            .unwrap_or_else(|| panic!("no `{key}=` line in the probe output"))
    };

    assert_eq!(get("claude_touch"), "EROFS");
    for key in [
        "creds_read",
        "claude_json_read",
        "git_credentials_read",
        "netrc_read",
    ] {
        let value = get(key);
        assert!(
            value == "absent" || value.parse::<i32>().is_ok_and(|rc| rc != 0),
            "{key}={value}: a masked credential file must not be readable"
        );
    }
    assert_eq!(get("ssh_entries"), "0");
    let gh = get("gh_entries");
    assert!(gh == "0" || gh == "absent", "gh_entries={gh}");
    for key in [
        "env_ssh_auth_sock",
        "env_gh_token",
        "env_anthropic_api_key",
        "env_anthropic_auth_token",
    ] {
        assert_eq!(get(key), "", "{key} must be unset inside");
    }
    assert_eq!(get("env_git_terminal_prompt"), "0");
    assert_eq!(get("run_dir_touch"), "EROFS");
    assert_eq!(get("lock_bytes"), "0");
    assert_eq!(get("direction_lock_bytes"), "0");
    assert_ne!(get("lock_read"), "0", "launcher.lock must not be readable");
    assert_eq!(get("cfg_touch"), "WRITABLE");
    assert_eq!(get("settings_touch"), "EROFS");
    assert_eq!(get("bin_touch"), "EROFS");
    assert_eq!(get("runtime_entries"), "gsdmm-driverctl");
    assert_eq!(get("bus"), "absent");
    let manager = get("user_manager");
    assert!(
        manager != "running" && manager != "degraded",
        "user_manager={manager}: the user systemd manager is reachable inside"
    );
    assert_eq!(get("sock"), "present");

    assert!(
        !inside_ssh.success,
        "a push over ssh from inside the sandbox succeeded"
    );
    assert!(
        !inside_https.success,
        "a push over https from inside the sandbox succeeded"
    );
}
