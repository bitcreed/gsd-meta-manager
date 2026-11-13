#![cfg(target_os = "linux")]
// ============================================================================
// LIVE proof of the sandbox's isolation (Phase 28, plan 28-03; SC3, D-19, D-20,
// D-27). Opt-in: the test is `#[ignore]` because it needs a working `bwrap`, a
// user systemd manager and the network, none of which a CI runner has.
//
//   cargo test --test launcher_live_isolation -- --ignored --nocapture
//
// 28.1-08 (D-16): `$HOME` is a size-capped tmpfs with a derived allowlist re-bound, so
// the probe plants an unlisted file in the REAL home (`~/.gsdmm-probe-<pid>/env`, the
// `~/.claudear/env` lookalike) and a home symlink to a project `.env`, and proves both
// unreadable while the toolchain, GSD and git keep working. Both are removed on every
// exit path.
//
// Run it OUTSIDE any tool sandbox. A test that cannot run prints a `SKIP:` line
// and returns; a `SKIP:` line is no evidence, it means this host proved nothing.
//
// A sibling run dir is probed too: it must be invisible inside the sandbox and
// byte-identical on the host after a tampering attempt (quick 261020-f80).
//
// The probe uses the REAL `HOME` read-only: it prints exit codes, entry counts
// and byte counts, never the content of a host file. It writes nothing to the
// host except under `CARGO_TARGET_TMPDIR`.
// ============================================================================

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::symlink;
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use gsd_meta_manager::driverctl::serve::socket_path;
use gsd_meta_manager::launcher::config_dir::{seed, SeedSpec, HOST_MANAGED_DIR};
use gsd_meta_manager::launcher::egress::socket_path_for as egress_socket_path_for;
use gsd_meta_manager::launcher::home::{HomeBind, HomeSetup};
use gsd_meta_manager::launcher::host::detect;
use gsd_meta_manager::launcher::sandbox::{
    bwrap_argv, child_env, systemd_run_argv, LaunchPaths, SandboxSpec,
};
use tempfile::TempDir;

/// The in-sandbox probe. `$1` run dir, `$2` config dir, `$3` host home, `$4` a
/// sibling run dir, `$5` the planted directory under the host home, `$6` the planted
/// symlink under the host home.
/// Every line is `KEY=VALUE`; no host file content is printed.
const PROBE: &str = r#"
run_dir=$1; cfg=$2; home=$3; sib=$4; planted_dir=$5; planted_link=$6
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
echo "npmrc_read=$(read_rc "$home/.npmrc")"
echo "cargo_credentials_read=$(read_rc "$home/.cargo/credentials.toml")"
echo "run_token_read=$(read_rc "$home/.config/gsd-meta-manager/driver-oauth-token")"
for d in .aws .kube .azure .config/gcloud; do
  if [ -d "$home/$d" ]; then echo "entries_$(echo "$d" | tr './' '__')=$(ls -A "$home/$d" 2>/dev/null | wc -l | tr -d ' ')"; fi
done
echo "env_aws_access_key_id=${AWS_ACCESS_KEY_ID:-}"
echo "env_npm_token=${NPM_TOKEN:-}"
echo "env_gitlab_token=${GITLAB_TOKEN:-}"
echo "env_ssh_auth_sock=${SSH_AUTH_SOCK:-}"
echo "env_gh_token=${GH_TOKEN:-}"
echo "env_anthropic_api_key=${ANTHROPIC_API_KEY:-}"
echo "env_anthropic_auth_token=${ANTHROPIC_AUTH_TOKEN:-}"
echo "env_git_terminal_prompt=${GIT_TERMINAL_PROMPT:-}"
if (: < /dev/tty) 2>/dev/null; then echo "tty_open=OPEN"; else echo "tty_open=NONE"; fi
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
echo "drivers_entries=$(ls -A "$(dirname "$run_dir")" | tr '\n' ' ' | sed 's/ *$//')"
if [ -e "$sib" ]; then echo "sibling_visible=present"; else echo "sibling_visible=absent"; fi
echo "sibling_log_read=$(read_rc "$sib/LOG.md")"
mkdir -p "$sib" 2>/dev/null
echo tampered >> "$sib/LOG.md" 2>/dev/null
touch "$sib/sandbox-probe" 2>/dev/null
echo "sibling_write_attempted=1"
echo "planted_env_read=$(cat "$planted_dir/env" > /dev/null 2>&1; echo $?)"
echo "planted_link_read=$(cat "$planted_link" > /dev/null 2>&1; echo $?)"
if [ -e "$home/.cargo/credentials.toml" ]; then echo "cargo_credentials=present"; else echo "cargo_credentials=absent"; fi
echo "claude_version_rc=$(claude --version > /dev/null 2>&1; echo $?)"
echo "node_version_rc=$(node --version > /dev/null 2>&1; echo $?)"
echo "cargo_version_rc=$(cargo --version > /dev/null 2>&1; echo $?)"
if command -v rtk > /dev/null 2>&1; then
  echo "rtk_version_rc=$(rtk --version > /dev/null 2>&1; echo $?)"
else
  echo "rtk_version_rc=absent"
fi
scratch=$(mktemp -d)
( cd "$scratch" && git init -q && git commit -q --allow-empty -m probe ) > /dev/null 2>&1
echo "git_commit_rc=$?"
echo "gsd_tools_rc=$(node "$home/.claude/gsd-core/bin/gsd-tools.cjs" runtime-identity --raw > /dev/null 2>&1; echo $?)"
echo "git_alias_count=$(git config --get-regexp '^alias\.' 2>/dev/null | wc -l | tr -d ' ')"
echo "claude_agents_touch=$(touch_state "$home/.claude/agents/sandbox-probe")"
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

/// The two files planted in the real home, removed when this drops (every exit
/// path, a failed assertion included).
struct Planted {
    dir: PathBuf,
    link: PathBuf,
}

impl Planted {
    fn plant(home: &Path, project_env: &Path) -> Planted {
        let pid = std::process::id();
        let dir = home.join(format!(".gsdmm-probe-{pid}"));
        let link = home.join(format!(".gsdmm-probe-{pid}-link"));
        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_file(&link);
        let planted = Planted { dir, link };
        fs::create_dir_all(&planted.dir).expect("plant the probe directory");
        fs::write(planted.dir.join("env"), "PLANTED-SECRET-ENV\n").expect("plant the probe file");
        fs::write(project_env, "PLANTED-SECRET-PROJECT-ENV\n").expect("plant the project .env");
        symlink(project_env, &planted.link).expect("plant the probe link");
        planted
    }
}

impl Drop for Planted {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
        let _ = fs::remove_file(&self.link);
    }
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
    let sibling = project.join(".planning/meta-manager/drivers/probe28-sibling");
    fs::create_dir_all(&sibling).unwrap();
    fs::write(sibling.join("LOG.md"), "sibling-log-sentinel\n").unwrap();
    git_ok(&project, &["init", "-q"]);
    git_ok(&project, &["commit", "-q", "--allow-empty", "-m", "probe"]);
    // D-16: an unlisted file in the real home and a home symlink to a project `.env`.
    let planted = Planted::plant(&home, &project.join(".env"));

    let home_setup = HomeSetup::for_host(
        &home,
        Path::new(env!("CARGO_BIN_EXE_gsd-meta-manager")),
        detect().overlay,
        &root,
    )
    .expect("derive the $HOME setup");
    for bind in &home_setup.binds {
        let (tag, path) = match bind {
            HomeBind::Ro(p) => ("ro", p),
            HomeBind::Overlay(p) => ("overlay", p),
            HomeBind::Tmpfs { path, .. } => ("tmpfs", path),
        };
        println!(
            "LIVE allowlist {tag} ~/{}",
            path.strip_prefix(&home).unwrap_or(path).display()
        );
    }
    match home_setup.dangling_notice(&home) {
        Some(line) => println!("LIVE {line}"),
        None => println!("LIVE dangling: none"),
    }
    let sock = socket_path("probe28", &run_dir).expect("the run's socket path");
    seed(&SeedSpec {
        config_dir: config_dir.clone(),
        host_claude_dir: home.join(".claude"),
        host_home: home.clone(),
        binary: std::env::current_exe().unwrap(),
        run_dir: run_dir.clone(),
        socket: sock.clone(),
        managed: true,
        host_managed_dir: PathBuf::from(HOST_MANAGED_DIR),
        keep_roots: home_setup.keep_roots.clone(),
    })
    .expect("seed the per-run config dir");

    let _ = fs::remove_file(&sock);
    let _listener = UnixListener::bind(&sock).expect("bind the probe socket");
    // The egress socket is re-bound next to it, so it must exist (a dummy listener:
    // the egress path itself is proved by `launcher_live_egress`).
    let egress_sock = egress_socket_path_for(&sock, &run_dir);
    let _ = fs::remove_file(&egress_sock);
    let _egress_listener = UnixListener::bind(&egress_sock).expect("bind the egress socket");

    // The host environment, with the four secrets the launcher must keep out.
    let mut host_env: Vec<(OsString, OsString)> = std::env::vars_os().collect();
    for (key, value) in [
        ("SSH_AUTH_SOCK", "/run/probe/fake-agent.sock"),
        ("GH_TOKEN", "gh-token-probe-sentinel"),
        ("ANTHROPIC_API_KEY", "api-key-probe-sentinel"),
        ("ANTHROPIC_AUTH_TOKEN", "auth-token-probe-sentinel"),
        ("AWS_ACCESS_KEY_ID", "aws-probe-sentinel"),
        ("NPM_TOKEN", "npm-probe-sentinel"),
        ("GITLAB_TOKEN", "gitlab-probe-sentinel"),
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
            egress_socket: egress_sock.clone(),
        },
        &home,
        &home_setup,
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
    let probe = in_sandbox(
        PROBE,
        &[
            &run_dir,
            &config_dir,
            &home,
            &sibling,
            &planted.dir,
            &planted.link,
        ],
    );
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

    // ---- sibling run dirs are hidden (quick 261020-f80) --------------------
    // Placed before the push probe: its host-control SKIP returns early.
    assert!(probe.success, "the probe run failed: {}", probe.stderr);
    let sib_kv: BTreeMap<&str, &str> = probe
        .stdout
        .lines()
        .filter_map(|l| l.split_once('='))
        .collect();
    let sib_get = |key: &str| -> &str {
        sib_kv
            .get(key)
            .copied()
            .unwrap_or_else(|| panic!("no `{key}=` line in the probe output"))
    };
    assert_eq!(sib_get("drivers_entries"), "probe28");
    assert_eq!(sib_get("sibling_visible"), "absent");
    assert_eq!(sib_get("sibling_log_read"), "absent");
    assert_eq!(sib_get("sibling_write_attempted"), "1");
    let entry_names = |dir: &Path| -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };
    assert_eq!(
        fs::read(sibling.join("LOG.md")).unwrap(),
        b"sibling-log-sentinel\n",
        "the host sibling LOG.md changed"
    );
    assert_eq!(entry_names(&sibling), ["LOG.md"]);
    assert_eq!(
        entry_names(run_dir.parent().unwrap()),
        ["probe28", "probe28-sibling"]
    );

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

    // D-16: `~/.claude` is the sandbox's own tmpfs now, so the write lands there (and
    // `leaked` above proved the host path stayed absent); only the re-bound
    // subdirectories are read-only.
    assert_eq!(get("claude_touch"), "WRITABLE");
    if home.join(".claude/agents").is_dir() {
        assert_eq!(get("claude_agents_touch"), "EROFS");
    }
    for key in ["planted_env_read", "planted_link_read"] {
        let value = get(key);
        assert!(
            value.parse::<i32>().is_ok_and(|rc| rc != 0),
            "{key}={value}: an unlisted file in the real home must be unreadable"
        );
    }
    assert_eq!(get("cargo_credentials"), "absent");
    for key in [
        "claude_version_rc",
        "node_version_rc",
        "cargo_version_rc",
        "git_commit_rc",
        "gsd_tools_rc",
        "git_alias_count",
    ] {
        assert_eq!(
            get(key),
            "0",
            "{key}: the toolchain, GSD and git must work on the allowlist alone"
        );
    }
    let host_has_rtk = std::env::var_os("PATH")
        .is_some_and(|path| std::env::split_paths(&path).any(|dir| dir.join("rtk").is_file()));
    if host_has_rtk {
        assert_eq!(get("rtk_version_rc"), "0");
    }
    for key in [
        "creds_read",
        "claude_json_read",
        "git_credentials_read",
        "netrc_read",
        "npmrc_read",
        "cargo_credentials_read",
        "run_token_read",
    ] {
        let value = get(key);
        assert!(
            value == "absent" || value.parse::<i32>().is_ok_and(|rc| rc != 0),
            "{key}={value}: a masked credential file must not be readable"
        );
    }
    assert_eq!(get("ssh_entries"), "0");
    for (key, value) in &kv {
        if key.starts_with("entries_") {
            assert_eq!(*value, "0", "{key}: a masked credential directory is empty");
        }
    }
    let gh = get("gh_entries");
    assert!(gh == "0" || gh == "absent", "gh_entries={gh}");
    for key in [
        "env_ssh_auth_sock",
        "env_gh_token",
        "env_anthropic_api_key",
        "env_anthropic_auth_token",
        "env_aws_access_key_id",
        "env_npm_token",
        "env_gitlab_token",
    ] {
        assert_eq!(get(key), "", "{key} must be unset inside");
    }
    assert_eq!(get("env_git_terminal_prompt"), "0");
    // 28 IN-02: without a controlling terminal the probe fails with or without
    // --new-session, so asserting it would prove nothing (CI, a detached shell).
    if std::fs::OpenOptions::new()
        .read(true)
        .open("/dev/tty")
        .is_ok()
    {
        assert_eq!(
            get("tty_open"),
            "NONE",
            "the controlling terminal must not be reachable inside (--new-session)"
        );
    } else {
        eprintln!(
            "SKIP: tty_open proves nothing here: this process has no controlling terminal, \
             so the probe fails with or without --new-session (the bwrap argv golden still \
             pins --new-session)"
        );
    }
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

// ============================================================================
// 28.1-09, D-24: the final sweep over the COMPLETE profile (network isolation, the
// `$HOME` allowlist, the planted-file guard). Opt-in, like the probe above.
//
//   cargo test --test launcher_live_isolation -- --ignored --nocapture
//
// Settles A1, A2, A3 and B6 with captured evidence and D-21's ro half:
//  * the pruned `find` of RESEARCH Pitfall 8 lists EXACTLY the driverctl socket and
//    the egress socket (both present, so the sweep is not vacuous), and
//    `$XDG_RUNTIME_DIR` holds only `gsdmm-driverctl`, so the host's `cc-socks`, `bus`,
//    Wayland and ssh/gpg agent sockets, whatever the host has, are absent (A1, A2);
//  * a write to `~/.claude/daemon/dispatch` lands in the sandbox's own `~/.claude`
//    and never reaches the host (A3);
//  * `.git/config` and `.claude/settings.json` cannot be written, `.git` and
//    `.claude` cannot be renamed, while a commit and new dirs under `.claude/skills`,
//    `.claude/worktrees` and `.planning/` work (D-21).
//
// Concurrency (LNCH-13): the fixture is its own directory under
// `CARGO_TARGET_TMPDIR`, with its own sockets and unit name, and every host-side plant
// is removed by a drop guard on every path.
// ============================================================================

use gsd_meta_manager::launcher::egress::upstream::Upstream;
use gsd_meta_manager::launcher::egress::{EgressServer, EgressSpec, TokenProvider};
use gsd_meta_manager::launcher::sandbox::{bridged_command, egress_env_sets, BRIDGE_PORT};

/// The in-sandbox probe of the final sweep. `$1` the project, `$2` the host home.
/// Every line is `KEY=VALUE`; no host file content is printed.
const FINAL_PROBE: &str = r#"
project=$1; home=$2
cd "$project" || exit 1
# `-perm -000` (always true) comes first: it makes find lstat() each entry. Without it
# GNU find 4.10 trusts the directory entry's d_type, and a socket bind-mounted by bwrap
# sits on a regular-file mount point whose d_type is DT_REG, so `-type s` alone prints
# nothing for the run's own two sockets (measured, 28.1-09): RESEARCH Pitfall 8's form is
# vacuous for bind-mounted sockets.
echo "sockets=$(find / \( -path /proc -o -path /sys -o -path /dev \) -prune -o -perm -000 -type s -print 2>/dev/null | sort | paste -sd, -)"
echo "runtime_entries=$(ls -A "$XDG_RUNTIME_DIR" | tr '\n' ' ' | sed 's/ *$//')"
for n in cc-socks bus wayland-0 gnupg ssh-agent.socket; do
  if [ -e "$XDG_RUNTIME_DIR/$n" ]; then echo "runtime_has_$n=present"; else echo "runtime_has_$n=absent"; fi
done
mkdir -p "$home/.claude/daemon" 2>/dev/null && touch "$home/.claude/daemon/dispatch" 2>/dev/null; echo "daemon_write=$?"
{ echo '[core]' >> .git/config; } 2>/dev/null; echo "git_config_append=$?"
touch .claude/settings.json 2>/dev/null; echo "claude_settings_touch=$?"
mv .git .gitx 2>/dev/null; echo "mv_git=$?"
[ -d .gitx ] && mv .gitx .git
mv .claude .claudex 2>/dev/null; echo "mv_claude=$?"
[ -d .claudex ] && mv .claudex .claude
git -c user.name=probe -c user.email=probe@localhost -c commit.gpgsign=false commit -q --allow-empty -m probe > /dev/null 2>&1; echo "git_commit_rc=$?"
mkdir -p .claude/skills/probe-skill 2>/dev/null; echo "skills_mkdir_rc=$?"
mkdir -p .claude/worktrees/probe-tree 2>/dev/null; echo "worktrees_mkdir_rc=$?"
mkdir -p .planning 2>/dev/null && touch .planning/probe-note 2>/dev/null; echo "planning_touch_rc=$?"
"#;

/// Undoes every host-side change of the final sweep when dropped, on every path.
struct FinalSweepGuard {
    dispatch: PathBuf,
    daemon_dir: PathBuf,
    dispatch_existed: bool,
    daemon_dir_existed: bool,
    project: PathBuf,
}

impl Drop for FinalSweepGuard {
    fn drop(&mut self) {
        // A write that wrongly reached the host is removed, a pre-existing file is not.
        if !self.dispatch_existed {
            let _ = fs::remove_file(&self.dispatch);
            if !self.daemon_dir_existed {
                let _ = fs::remove_dir(&self.daemon_dir);
            }
        }
        // A rename that wrongly succeeded is undone.
        for (moved, original) in [(".gitx", ".git"), (".claudex", ".claude")] {
            let moved = self.project.join(moved);
            let original = self.project.join(original);
            if moved.exists() && !original.exists() {
                let _ = fs::rename(&moved, &original);
            }
        }
    }
}

/// `(exists, mtime)` of a host path, for the before/after comparison.
fn existence_and_mtime(path: &Path) -> (bool, Option<std::time::SystemTime>) {
    match fs::metadata(path) {
        Ok(meta) => (true, meta.modified().ok()),
        Err(_) => (false, None),
    }
}

#[test]
#[ignore = "needs bwrap with namespaces and a user systemd; run with -- --ignored --nocapture"]
fn the_final_profile_closes_every_audited_channel() {
    if let Err(reason) = bwrap_runs_here() {
        println!("SKIP: {reason}");
        return;
    }
    let manager = user_manager_state();
    if manager != "running" && manager != "degraded" {
        println!("SKIP: the user systemd manager is not usable (is-system-running: {manager:?})");
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

    // Its own fixture dir (bwrap's `--tmpfs /tmp` hides /tmp), its own unit name.
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("launcher-final-sweep");
    fs::create_dir_all(&base).unwrap();
    let scratch = TempDir::new_in(&base).unwrap();
    let root = scratch.path().canonicalize().unwrap();
    let project = root.join("project");
    let run_name = format!("final{}", std::process::id());
    let run_dir = project
        .join(".planning/meta-manager/drivers")
        .join(&run_name);
    let config_dir = root.join("config");
    for dir in [
        &run_dir,
        &config_dir.join("bin"),
        &config_dir.join("managed"),
        &project.join(".claude/skills"),
        &project.join(".claude/worktrees"),
    ] {
        fs::create_dir_all(dir).unwrap();
    }
    fs::write(config_dir.join("settings.json"), "{}").unwrap();
    fs::write(run_dir.join("launcher.lock"), "").unwrap();
    fs::write(run_dir.join("DIRECTION.lock"), "").unwrap();
    fs::write(project.join(".claude/settings.json"), "{\"fixture\":1}\n").unwrap();
    git_ok(&project, &["init", "-q"]);
    git_ok(&project, &["commit", "-q", "--allow-empty", "-m", "probe"]);
    assert!(
        project.join(".git/hooks").is_dir(),
        "git init creates .git/hooks"
    );

    // Host state to compare afterwards.
    let git_config = project.join(".git/config");
    let claude_settings = project.join(".claude/settings.json");
    let git_config_before = fs::read(&git_config).unwrap();
    let claude_settings_before = fs::read(&claude_settings).unwrap();
    let dispatch = home.join(".claude/daemon/dispatch");
    let (dispatch_existed, dispatch_mtime) = existence_and_mtime(&dispatch);
    let _guard = FinalSweepGuard {
        dispatch: dispatch.clone(),
        daemon_dir: home.join(".claude/daemon"),
        dispatch_existed,
        daemon_dir_existed: home.join(".claude/daemon").exists(),
        project: project.clone(),
    };

    // A1/A2 non-vacuity control: which of those services the host really has.
    let host_runtime: Vec<String> = {
        let mut names: Vec<String> = fs::read_dir(&runtime)
            .map(|d| {
                d.filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        names.sort();
        names
    };
    let interesting: Vec<&String> = host_runtime
        .iter()
        .filter(|n| {
            n.starts_with("cc-socks")
                || n.as_str() == "bus"
                || n.starts_with("wayland")
                || n.starts_with("gnupg")
                || n.contains("ssh-agent")
                || n.starts_with("gcr")
        })
        .collect();
    println!("LIVE host runtime-dir names (A1/A2 control): {host_runtime:?}");
    println!("LIVE host runtime-dir agent/service entries: {interesting:?}");
    println!(
        "LIVE host ~/.claude/daemon/dispatch before: exists={dispatch_existed} mtime={dispatch_mtime:?}"
    );

    let home_setup = HomeSetup::for_host(
        &home,
        Path::new(env!("CARGO_BIN_EXE_gsd-meta-manager")),
        detect().overlay,
        &root,
    )
    .expect("derive the $HOME setup");

    // The run's two sockets: a dummy driverctl listener and a real egress server.
    let driverctl_sock = socket_path(&run_name, &run_dir).expect("the run's driverctl socket");
    let _ = fs::remove_file(&driverctl_sock);
    let _driverctl = UnixListener::bind(&driverctl_sock).expect("bind the driverctl socket");
    let egress_sock = egress_socket_path_for(&driverctl_sock, &run_dir);
    let _ = fs::remove_file(&egress_sock);
    let egress_listener = UnixListener::bind(&egress_sock).expect("bind the egress socket");
    // A plain upstream nobody connects to: the sweep only needs the server on its path.
    let upstream = std::net::TcpListener::bind("127.0.0.1:0").expect("bind the stub upstream");
    let egress = EgressServer::start(
        egress_listener,
        EgressSpec {
            allow_hosts: Vec::new(),
            token: TokenProvider::Static("LIVE-FINAL-SWEEP-TOKEN-SENTINEL".to_owned()),
            upstream: Upstream::Plain(upstream.local_addr().expect("upstream addr")),
            log_path: root.join("egress.log"),
        },
    )
    .expect("start the egress server");

    let spec = SandboxSpec::from_host(
        LaunchPaths {
            project: project.clone(),
            run_dir: run_dir.clone(),
            config_dir: config_dir.clone(),
            socket: driverctl_sock.clone(),
            egress_socket: egress_sock.clone(),
        },
        &home,
        &home_setup,
    );
    assert!(
        !spec.guard_binds.is_empty(),
        "the fixture has .git and .claude, so the guard binds exist"
    );

    let inner: Vec<OsString> = vec![
        "sh".into(),
        "-c".into(),
        FINAL_PROBE.into(),
        "probe".into(),
        project.clone().into_os_string(),
        home.clone().into_os_string(),
    ];
    let command = bridged_command(
        Path::new(env!("CARGO_BIN_EXE_gsd-meta-manager")),
        BRIDGE_PORT,
        &egress_sock,
        &inner,
    );
    let unit = format!("gsdmm-final-{}", std::process::id());
    let argv = systemd_run_argv(&unit, Some(180), &bwrap_argv(&spec, &command));
    let mut sets: Vec<(&str, OsString)> = egress_env_sets(BRIDGE_PORT);
    sets.push(("CLAUDE_CONFIG_DIR", config_dir.clone().into_os_string()));
    sets.push(("DRIVERCTL_SOCK", driverctl_sock.clone().into_os_string()));
    let env = child_env(std::env::vars_os(), &sets);
    let (program, rest) = argv.split_first().unwrap();
    let out = Command::new(program)
        .args(rest)
        .env_clear()
        .envs(&env)
        .current_dir(&project)
        .stdin(Stdio::null())
        .output()
        .expect("run systemd-run");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
    print!("{stdout}");
    if !stderr.trim().is_empty() {
        println!("LIVE probe stderr: {}", stderr.trim());
    }
    let stats = egress.stop();
    println!("LIVE egress server stopped: {stats:?}");

    // ---- host side, after the run ------------------------------------------
    let (dispatch_exists_after, dispatch_mtime_after) = existence_and_mtime(&dispatch);
    println!(
        "LIVE host ~/.claude/daemon/dispatch after:  exists={dispatch_exists_after} mtime={dispatch_mtime_after:?}"
    );
    let git_config_after = fs::read(&git_config).unwrap();
    let claude_settings_after = fs::read(&claude_settings).unwrap();

    assert!(out.status.success(), "the probe run failed: {stderr}");
    let kv: BTreeMap<&str, &str> = stdout.lines().filter_map(|l| l.split_once('=')).collect();
    let get = |key: &str| -> &str {
        kv.get(key)
            .copied()
            .unwrap_or_else(|| panic!("no `{key}=` line in the probe output"))
    };

    // D-24 / B6, RESEARCH Pitfall 8: exactly the run's two sockets, both present.
    let mut expected = [
        driverctl_sock.display().to_string(),
        egress_sock.display().to_string(),
    ];
    expected.sort();
    assert_eq!(
        get("sockets"),
        expected.join(","),
        "the sweep must list exactly the driverctl socket and the egress socket"
    );
    // A1, A2: nothing else of the host's runtime dir is visible.
    assert_eq!(get("runtime_entries"), "gsdmm-driverctl");
    for name in ["cc-socks", "bus", "wayland-0", "gnupg", "ssh-agent.socket"] {
        assert_eq!(get(&format!("runtime_has_{name}")), "absent", "{name}");
    }
    // A3: the write landed in the sandbox's own `~/.claude`, the host is unchanged.
    assert_eq!(
        (dispatch_exists_after, dispatch_mtime_after),
        (dispatch_existed, dispatch_mtime),
        "~/.claude/daemon/dispatch changed on the host"
    );

    // D-21: unwritable and unrenamable where present.
    for key in [
        "git_config_append",
        "claude_settings_touch",
        "mv_git",
        "mv_claude",
    ] {
        assert_ne!(get(key), "0", "{key} must fail inside");
    }
    assert_eq!(git_config_before, git_config_after, ".git/config changed");
    assert_eq!(
        claude_settings_before, claude_settings_after,
        ".claude/settings.json changed"
    );
    assert!(project.join(".git").is_dir() && project.join(".claude").is_dir());
    // ... and the legitimate writes still work.
    for key in [
        "git_commit_rc",
        "skills_mkdir_rc",
        "worktrees_mkdir_rc",
        "planning_touch_rc",
    ] {
        assert_eq!(get(key), "0", "{key} must succeed inside");
    }
}
