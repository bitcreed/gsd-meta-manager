//! The pure command-line builders of the launcher (Phase 28; D-08, D-16, D-19, D-20).
//!
//! Nothing here touches the file system, the clock or the environment: every
//! host fact is a parameter (the one exception is [`SandboxSpec::from_host`],
//! which says so). That is what lets the `systemd-run`, `bwrap` and `claude`
//! command lines be golden-tested on a runner with no bwrap and no user systemd
//! (D-20, the router-conformance / git-witness lesson).
//!
//! The profile is the spike-002/003 one (`launcher_s003.py`), validated live in
//! L1 step 0:
//!
//! * host root read-only, a private `/dev`, `/proc` and `/tmp`;
//! * the project checkout read-write (the driver commits), the run directory
//!   read-only on top of it, and the two lock files inside it masked with
//!   `/dev/null` (Phase 27 hand-off 2: a sandboxed process must not take either
//!   `flock`);
//! * the host's credential directories replaced by an empty tmpfs and the
//!   credential files masked, each only when it exists (a mask over a missing
//!   path aborts bwrap);
//! * the per-run config directory read-write;
//! * `--unshare-pid --new-session --die-with-parent`.
//!
//! 28-03 hardened this profile without changing a signature in this file:
//!
//! * `$XDG_RUNTIME_DIR` is replaced by an empty tmpfs and only the run's own
//!   driverctl socket is re-bound into it, so the SSH agent, the D-Bus session
//!   bus and the user systemd manager are unreachable (RESEARCH §1, D-19);
//! * the per-run `settings.json` (the gate hooks) and `bin/` (the shim they call)
//!   are read-only inside the writable config dir;
//! * git's other credential stores are masked as well as `~/.ssh` (D-27);
//! * `claude` reads user settings only (`--setting-sources user`), and
//!   `ANTHROPIC_AUTH_TOKEN`, which outranks the run token, is stripped.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// The first word of every scope unit name (D-08).
pub const SCOPE_PREFIX: &str = "gsdmm-driver";

/// The environment variables removed from the child before the launcher sets its
/// own (D-19): the spike list verbatim. Session identity of the launching
/// Claude Code, and every credential channel the host exposes.
pub const STRIP_ENV: &[&str] = &[
    "CLAUDECODE",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_BRIDGE_SESSION_ID",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN",
    "CLAUDE_PID",
    "SSH_AUTH_SOCK",
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "CLAUDE_CONFIG_DIR",
    "ANTHROPIC_API_KEY",
    "CLAUDE_ENV_FILE",
];

/// Environment variables stripped in addition to [`STRIP_ENV`] (RESEARCH §3):
/// `ANTHROPIC_AUTH_TOKEN` outranks `CLAUDE_CODE_OAUTH_TOKEN`, so a host value
/// would silently replace the run token.
pub const EXTRA_STRIP_ENV: &[&str] = &["ANTHROPIC_AUTH_TOKEN"];

/// Directories under `$HOME` the sandbox replaces with an empty tmpfs.
pub const TMPFS_MASK_DIRS: &[&str] = &[".ssh", ".gnupg", ".config/gh", ".aws", ".docker"];

/// Files under `$HOME` the sandbox masks with `/dev/null`. The last two close
/// git's own credential stores for D-27 (a push from inside must find no
/// credential) `[inferred — audit]`.
pub const NULL_MASK_FILES: &[&str] = &[
    ".claude/.credentials.json",
    ".claude.json",
    ".git-credentials",
    ".config/git/credentials",
    ".netrc",
];

/// The scope unit name of launch `launch_n` of a run (D-08):
/// `gsdmm-driver-<run-id>-<hash8>-<n>`. systemd appends `.scope`; the name the
/// store records carries the suffix.
///
/// `hash8` is the digest of the canonical run directory
/// ([`crate::driverctl::serve::hash8`]), so two projects that use one run id get
/// two units.
pub fn scope_unit_name(run_id: &str, hash8: &str, launch_n: u32) -> String {
    format!("{SCOPE_PREFIX}-{run_id}-{hash8}-{launch_n}")
}

/// The paths of one launch, all canonical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchPaths {
    /// The driven checkout: read-write inside.
    pub project: PathBuf,
    /// `<project>/.planning/meta-manager/drivers/<run-id>`: read-only inside.
    pub run_dir: PathBuf,
    /// The per-run `CLAUDE_CONFIG_DIR`: read-write inside.
    pub config_dir: PathBuf,
    /// The run's driverctl socket: re-bound inside the runtime-dir tmpfs.
    pub socket: PathBuf,
}

/// What [`bwrap_argv`] builds the sandbox from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxSpec {
    /// The user's home directory the masks are relative to.
    pub home: PathBuf,
    pub paths: LaunchPaths,
    /// Directories replaced by an empty tmpfs (every one exists on the host).
    pub tmpfs_masks: Vec<PathBuf>,
    /// Files masked with `/dev/null` (every one exists on the host).
    pub null_masks: Vec<PathBuf>,
    /// The host's `$XDG_RUNTIME_DIR`, replaced by an empty tmpfs inside with only
    /// [`LaunchPaths::socket`] re-bound. `None` leaves it unmasked (no such dir).
    pub runtime_dir: Option<PathBuf>,
}

impl SandboxSpec {
    /// The spec for this host: the mask candidates under `home`, kept only when
    /// they exist, and `$XDG_RUNTIME_DIR` when it is an absolute path. **The one
    /// function here that reads the file system and the environment.**
    pub fn from_host(paths: LaunchPaths, home: &Path) -> Self {
        let tmpfs_masks = TMPFS_MASK_DIRS
            .iter()
            .map(|rel| home.join(rel))
            .filter(|p| p.is_dir())
            .collect();
        let null_masks = NULL_MASK_FILES
            .iter()
            .map(|rel| home.join(rel))
            .filter(|p| p.exists())
            .collect();
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute());
        SandboxSpec {
            home: home.to_owned(),
            paths,
            tmpfs_masks,
            null_masks,
            runtime_dir,
        }
    }
}

fn os(s: &str) -> OsString {
    OsString::from(s)
}

/// The `bwrap` command line that runs `command` in the sandbox, in a fixed
/// order (the order is the contract: a later bind covers an earlier one).
pub fn bwrap_argv(spec: &SandboxSpec, command: &[OsString]) -> Vec<OsString> {
    let paths = &spec.paths;
    let mut argv: Vec<OsString> = ["bwrap", "--ro-bind", "/", "/", "--dev", "/dev", "--proc"]
        .iter()
        .map(|s| os(s))
        .collect();
    argv.extend(["/proc", "--tmpfs", "/tmp"].iter().map(|s| os(s)));
    for dir in &spec.tmpfs_masks {
        argv.push(os("--tmpfs"));
        argv.push(dir.as_os_str().to_owned());
    }
    for file in &spec.null_masks {
        argv.push(os("--ro-bind"));
        argv.push(os("/dev/null"));
        argv.push(file.as_os_str().to_owned());
    }
    let bind = |argv: &mut Vec<OsString>, flag: &str, path: &Path| {
        argv.push(os(flag));
        argv.push(path.as_os_str().to_owned());
        argv.push(path.as_os_str().to_owned());
    };
    bind(&mut argv, "--bind", &paths.project);
    bind(&mut argv, "--ro-bind", &paths.run_dir);
    for lock in ["launcher.lock", "DIRECTION.lock"] {
        argv.push(os("--ro-bind"));
        argv.push(os("/dev/null"));
        argv.push(paths.run_dir.join(lock).into_os_string());
    }
    bind(&mut argv, "--bind", &paths.config_dir);
    // The gate's settings and the shim its hooks call stay unwritable for the driver.
    bind(
        &mut argv,
        "--ro-bind",
        &paths.config_dir.join("settings.json"),
    );
    bind(&mut argv, "--ro-bind", &paths.config_dir.join("bin"));
    if let Some(runtime) = &spec.runtime_dir {
        argv.push(os("--tmpfs"));
        argv.push(runtime.as_os_str().to_owned());
        if paths.socket.starts_with(runtime) {
            bind(&mut argv, "--bind", &paths.socket);
        }
    }
    argv.push(os("--unshare-pid"));
    // A new session drops the operator's controlling terminal, so the model cannot
    // `ioctl(TIOCSTI)` keystrokes into the shell that launched it (WR-01).
    argv.push(os("--new-session"));
    argv.push(os("--die-with-parent"));
    argv.push(os("--chdir"));
    argv.push(paths.project.as_os_str().to_owned());
    argv.push(os("--"));
    argv.extend(command.iter().cloned());
    argv
}

/// The `systemd-run` command line that runs `inner` in a transient user scope.
///
/// `--collect` so a failed scope does not stay loaded (RESEARCH §2). The `--`
/// before `inner` is what the fake shims of the integration tests key on.
/// `runtime_max_sec` is the wall-clock backstop (28-05 supplies it).
pub fn systemd_run_argv(
    unit: &str,
    runtime_max_sec: Option<u64>,
    inner: &[OsString],
) -> Vec<OsString> {
    let mut argv: Vec<OsString> = ["systemd-run", "--user", "--scope", "--quiet", "--collect"]
        .iter()
        .map(|s| os(s))
        .collect();
    argv.push(OsString::from(format!("--unit={unit}")));
    for property in ["TimeoutStopSec=15", "MemoryMax=6G", "TasksMax=1024"] {
        argv.push(os("-p"));
        argv.push(os(property));
    }
    if let Some(secs) = runtime_max_sec {
        argv.push(os("-p"));
        argv.push(OsString::from(format!("RuntimeMaxSec={secs}")));
    }
    argv.push(os("--"));
    argv.extend(inner.iter().cloned());
    argv
}

/// The headless driver's own command line (spike `claude` list).
pub fn claude_argv(prompt: &str, model: &str, max_budget_usd: f64) -> Vec<OsString> {
    let mut driver_argv: Vec<OsString> = vec![os("claude"), os("-p"), os(prompt)];
    // Project settings are reloaded mid-session; only the per-run user settings
    // (the gate hooks) may apply (RESEARCH §3, T-28-11).
    driver_argv.extend(["--setting-sources", "user"].iter().map(|s| os(s)));
    driver_argv.extend(["--model", model].iter().map(|s| os(s)));
    driver_argv.extend(
        [
            "--output-format",
            "stream-json",
            "--verbose",
            "--permission-mode",
            "bypassPermissions",
            "--max-budget-usd",
        ]
        .iter()
        .map(|s| os(s)),
    );
    driver_argv.push(OsString::from(max_budget_usd.to_string()));
    driver_argv
}

/// The master prompt for one launch: `template` with `{RUN_DIR}`,
/// `{PROJECT_ROOT}` and `{MODE}` substituted, then the spike's `Begin now:` line
/// (`launcher_s003.py` L317-320).
pub fn render_prompt(template: &str, run_dir: &Path, project: &Path, mode: &str) -> String {
    let run_dir = run_dir.display().to_string();
    let mut prompt = template
        .replace("{RUN_DIR}", &run_dir)
        .replace("{PROJECT_ROOT}", &project.display().to_string())
        .replace("{MODE}", mode);
    prompt.push_str(&format!(
        "\n\nBegin now: run Startup, then the Loop, for RUN_DIR={run_dir}."
    ));
    prompt
}

/// The child's whole environment: `host` minus [`STRIP_ENV`] and
/// [`EXTRA_STRIP_ENV`], then `GIT_TERMINAL_PROMPT=0` (a push from inside must
/// fail, not wait on a prompt, D-27), then each of `sets` applied in order (a
/// later set of the same key wins).
///
/// The child is started with `env_clear()` and exactly this map
/// ([`super::process::spawn_launch_child`]), so the scrub is decided here, by a
/// pure function, and not by a loop around a command builder.
pub fn child_env(
    host: impl IntoIterator<Item = (OsString, OsString)>,
    sets: &[(&str, OsString)],
) -> BTreeMap<OsString, OsString> {
    let mut env: BTreeMap<OsString, OsString> = host
        .into_iter()
        .filter(|(key, _)| {
            !STRIP_ENV
                .iter()
                .chain(EXTRA_STRIP_ENV)
                .any(|stripped| key == stripped)
        })
        .collect();
    env.insert(OsString::from("GIT_TERMINAL_PROMPT"), OsString::from("0"));
    for (key, value) in sets {
        env.insert(OsString::from(key), value.clone());
    }
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths() -> LaunchPaths {
        LaunchPaths {
            project: PathBuf::from("/work/proj"),
            run_dir: PathBuf::from("/work/proj/.planning/meta-manager/drivers/r1"),
            config_dir: PathBuf::from("/data/driver/ab12cd34/r1/claude-config"),
            socket: PathBuf::from("/run/user/1000/gsdmm-driverctl/r1-ab12cd34.sock"),
        }
    }

    fn spec() -> SandboxSpec {
        SandboxSpec {
            home: PathBuf::from("/home/u"),
            paths: paths(),
            tmpfs_masks: vec![PathBuf::from("/home/u/.ssh")],
            null_masks: vec![PathBuf::from("/home/u/.claude/.credentials.json")],
            runtime_dir: Some(PathBuf::from("/run/user/1000")),
        }
    }

    fn strings(argv: &[OsString]) -> Vec<String> {
        argv.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn the_scope_unit_name_is_prefix_run_hash_and_launch_number() {
        assert_eq!(
            scope_unit_name("r28", "0123abcd", 3),
            "gsdmm-driver-r28-0123abcd-3"
        );
    }

    #[test]
    fn render_prompt_substitutes_the_three_tokens_and_appends_the_begin_line() {
        let out = render_prompt(
            "dir={RUN_DIR} root={PROJECT_ROOT} mode={MODE} again={RUN_DIR}",
            Path::new("/p/.planning/meta-manager/drivers/r1"),
            Path::new("/p"),
            "loop",
        );
        assert_eq!(
            out,
            "dir=/p/.planning/meta-manager/drivers/r1 root=/p mode=loop \
             again=/p/.planning/meta-manager/drivers/r1\n\nBegin now: run Startup, then the \
             Loop, for RUN_DIR=/p/.planning/meta-manager/drivers/r1."
        );
    }

    #[test]
    fn child_env_drops_the_stripped_keys_and_applies_the_sets_last() {
        let host = vec![
            (OsString::from("PATH"), OsString::from("/usr/bin")),
            (OsString::from("SSH_AUTH_SOCK"), OsString::from("/s")),
            (OsString::from("GH_TOKEN"), OsString::from("g")),
            (OsString::from("ANTHROPIC_API_KEY"), OsString::from("a")),
            (OsString::from("CLAUDE_CONFIG_DIR"), OsString::from("/host")),
            (OsString::from("KEEP"), OsString::from("1")),
        ];
        let env = child_env(
            host,
            &[
                ("CLAUDE_CONFIG_DIR", OsString::from("/run-cfg")),
                ("PATH", OsString::from("/cfg/bin:/usr/bin")),
            ],
        );
        let keys: Vec<String> = env
            .keys()
            .map(|k| k.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            keys,
            ["CLAUDE_CONFIG_DIR", "GIT_TERMINAL_PROMPT", "KEEP", "PATH"]
        );
        assert_eq!(env[&OsString::from("CLAUDE_CONFIG_DIR")], "/run-cfg");
        assert_eq!(env[&OsString::from("PATH")], "/cfg/bin:/usr/bin");
    }

    #[test]
    fn strip_env_is_the_spike_list() {
        assert_eq!(STRIP_ENV.len(), 14);
        for key in [
            "SSH_AUTH_SOCK",
            "GH_TOKEN",
            "GITHUB_TOKEN",
            "ANTHROPIC_API_KEY",
        ] {
            assert!(STRIP_ENV.contains(&key), "{key}");
        }
    }

    #[test]
    fn bwrap_argv_has_the_fixed_order() {
        let command = vec![OsString::from("claude"), OsString::from("-p")];
        let argv = strings(&bwrap_argv(&spec(), &command));
        let run = "/work/proj/.planning/meta-manager/drivers/r1";
        let expected: Vec<String> = [
            "bwrap",
            "--ro-bind",
            "/",
            "/",
            "--dev",
            "/dev",
            "--proc",
            "/proc",
            "--tmpfs",
            "/tmp",
            "--tmpfs",
            "/home/u/.ssh",
            "--ro-bind",
            "/dev/null",
            "/home/u/.claude/.credentials.json",
            "--bind",
            "/work/proj",
            "/work/proj",
            "--ro-bind",
            run,
            run,
            "--ro-bind",
            "/dev/null",
            &format!("{run}/launcher.lock"),
            "--ro-bind",
            "/dev/null",
            &format!("{run}/DIRECTION.lock"),
            "--bind",
            "/data/driver/ab12cd34/r1/claude-config",
            "/data/driver/ab12cd34/r1/claude-config",
            "--ro-bind",
            "/data/driver/ab12cd34/r1/claude-config/settings.json",
            "/data/driver/ab12cd34/r1/claude-config/settings.json",
            "--ro-bind",
            "/data/driver/ab12cd34/r1/claude-config/bin",
            "/data/driver/ab12cd34/r1/claude-config/bin",
            "--tmpfs",
            "/run/user/1000",
            "--bind",
            "/run/user/1000/gsdmm-driverctl/r1-ab12cd34.sock",
            "/run/user/1000/gsdmm-driverctl/r1-ab12cd34.sock",
            "--unshare-pid",
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
        assert_eq!(argv, expected);
    }

    #[test]
    fn systemd_run_argv_wraps_the_inner_command_after_a_double_dash() {
        let inner = vec![OsString::from("bwrap"), OsString::from("--x")];
        let argv = strings(&systemd_run_argv(
            "gsdmm-driver-r1-ab12cd34-2",
            None,
            &inner,
        ));
        assert_eq!(
            argv[..6],
            [
                "systemd-run",
                "--user",
                "--scope",
                "--quiet",
                "--collect",
                "--unit=gsdmm-driver-r1-ab12cd34-2"
            ]
        );
        assert_eq!(argv[argv.len() - 3..], ["--", "bwrap", "--x"]);
        assert!(argv.iter().any(|a| a == "TimeoutStopSec=15"));
        assert!(!argv.iter().any(|a| a.starts_with("RuntimeMaxSec")));

        let timed = strings(&systemd_run_argv("u", Some(3900), &inner));
        assert!(timed.iter().any(|a| a == "RuntimeMaxSec=3900"));
    }

    #[test]
    fn claude_argv_is_the_headless_spike_command() {
        let argv = strings(&claude_argv("do it", "opus", 10.0));
        assert_eq!(
            argv,
            [
                "claude",
                "-p",
                "do it",
                "--setting-sources",
                "user",
                "--model",
                "opus",
                "--output-format",
                "stream-json",
                "--verbose",
                "--permission-mode",
                "bypassPermissions",
                "--max-budget-usd",
                "10"
            ]
        );
    }

    #[test]
    fn from_host_keeps_only_masks_that_exist() {
        let home = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(home.path().join(".ssh")).unwrap();
        std::fs::write(home.path().join(".claude.json"), "{}").unwrap();
        let spec = SandboxSpec::from_host(paths(), home.path());
        assert_eq!(spec.tmpfs_masks, [home.path().join(".ssh")]);
        assert_eq!(spec.null_masks, [home.path().join(".claude.json")]);
    }
}
