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
//! * the project checkout read-write (the driver commits), every sibling run
//!   directory hidden by an empty tmpfs over the drivers dir, the run directory
//!   re-bound read-only on top of that, and the two lock files inside it masked
//!   with `/dev/null` (Phase 27 hand-off 2: a sandboxed process must not take
//!   either `flock`);
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

use crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL;

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

/// Further credential channels stripped by exact name (WR-02): cloud, registry
/// and git/ssh helper variables that name a credential or a program that fetches one.
pub const MORE_STRIP_ENV: &[&str] = &[
    "GIT_ASKPASS",
    "SSH_ASKPASS",
    "GIT_SSH",
    "GIT_SSH_COMMAND",
    "KUBECONFIG",
    "DOCKER_CONFIG",
    "DOCKER_AUTH_CONFIG",
    "NETRC",
    "GNUPGHOME",
    "NPM_CONFIG_USERCONFIG",
    "PIP_INDEX_URL",
    "PIP_EXTRA_INDEX_URL",
    "DBUS_SESSION_BUS_ADDRESS",
];

/// Variable-name prefixes stripped (WR-02): the cloud and forge families.
pub const STRIP_ENV_PREFIXES: &[&str] = &[
    "AWS_",
    "AZURE_",
    "ARM_",
    "GOOGLE_",
    "GCP_",
    "GCLOUD_",
    "CLOUDSDK_",
    "GH_",
    "GITHUB_",
    "GITLAB_",
    "GLAB_",
    "CI_JOB_",
];

/// Variable-name suffixes stripped (WR-02): anything named like a secret.
pub const STRIP_ENV_SUFFIXES: &[&str] = &[
    "_TOKEN",
    "_SECRET",
    "_PASSWORD",
    "_PASSWD",
    "_PASSPHRASE",
    "_API_KEY",
    "_APIKEY",
    "_SECRET_KEY",
    "_ACCESS_KEY",
    "_PRIVATE_KEY",
    "_CREDENTIALS",
    "_CREDS",
];

/// True when the launcher removes `key` from the driver's environment: the exact
/// lists [`STRIP_ENV`], [`EXTRA_STRIP_ENV`], [`MORE_STRIP_ENV`] plus the
/// [`STRIP_ENV_PREFIXES`] and [`STRIP_ENV_SUFFIXES`] families.
///
/// This stays a denylist because the driver legitimately needs a long tail of
/// ordinary variables (`PATH`, proxies, toolchain homes, `ANTHROPIC_BASE_URL`, ...)
/// that an allowlist would have to guess `[inferred — audit]`. The launcher's own
/// values (`CLAUDE_CODE_OAUTH_TOKEN`, ...) are applied AFTER this filter by
/// [`child_env`], so a family match never removes what the launcher sets.
pub fn is_stripped_env(key: &std::ffi::OsStr) -> bool {
    let Some(name) = key.to_str() else {
        // A non-UTF-8 name matches no family; it is not a credential convention.
        return false;
    };
    let upper = name.to_ascii_uppercase();
    STRIP_ENV
        .iter()
        .chain(EXTRA_STRIP_ENV)
        .chain(MORE_STRIP_ENV)
        .any(|exact| name == *exact)
        || STRIP_ENV_PREFIXES.iter().any(|p| upper.starts_with(p))
        || STRIP_ENV_SUFFIXES.iter().any(|x| upper.ends_with(x))
}

/// Directories under `$HOME` the sandbox replaces with an empty tmpfs.
pub const TMPFS_MASK_DIRS: &[&str] = &[
    ".ssh",
    ".gnupg",
    ".config/gh",
    ".aws",
    ".docker",
    ".kube",
    ".azure",
    ".config/gcloud",
    ".config/glab-cli",
    ".local/share/keyrings",
    ".terraform.d",
];

/// Files under `$HOME` the sandbox masks with `/dev/null`. The git entries close
/// git's own credential stores for D-27 (a push from inside must find no
/// credential) `[inferred — audit]`; the package-registry and launcher-token files
/// close WR-02 (the network is open, so anything readable is exfiltratable).
pub const NULL_MASK_FILES: &[&str] = &[
    ".claude/.credentials.json",
    ".claude.json",
    ".git-credentials",
    ".config/git/credentials",
    ".netrc",
    ".npmrc",
    ".pypirc",
    ".cargo/credentials",
    ".cargo/credentials.toml",
    ".vault-token",
    ".config/gsd-meta-manager/driver-oauth-token",
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
    /// `<project>/.planning/meta-manager/drivers/<run-id>`: read-only inside, its
    /// siblings hidden.
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
        #[cfg_attr(not(any(target_os = "linux", target_os = "macos")), allow(unused_mut))]
        let mut null_masks: Vec<PathBuf> = NULL_MASK_FILES
            .iter()
            .map(|rel| home.join(rel))
            .filter(|p| p.exists())
            .collect();
        // The stored run token lives under `dirs::config_dir()`, which an
        // `XDG_CONFIG_HOME` moves away from `~/.config` (WR-02).
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if let Some(token) = super::auth::token_path() {
            if token.exists() && !null_masks.contains(&token) {
                null_masks.push(token);
            }
        }
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
///
/// The drivers dir is covered by an empty tmpfs right after the project bind, so
/// sibling runs are hidden; the run's own ro-bind and lock masks follow it and are
/// re-established on top.
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
    // Sibling runs vanish: an empty tmpfs over the drivers dir, then this run's own
    // directory is re-bound on top. The tmpfs must follow the project bind (which
    // would otherwise re-mount the host drivers dir over it) and precede the run-dir
    // bind and lock masks (which a later tmpfs would hide). Only emitted when the run
    // dir sits directly under the project's drivers dir; `validate_run_dir` guarantees
    // that in production.
    let drivers = paths.project.join(DRIVER_RUNS_REL);
    if paths.run_dir.parent() == Some(drivers.as_path()) {
        argv.push(os("--tmpfs"));
        argv.push(drivers.into_os_string());
    }
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

/// The most bytes the rendered master prompt may have. Linux caps ONE argv string
/// at `MAX_ARG_STRLEN` (128 KiB); a larger prompt makes `execve` fail with `E2BIG`
/// after the launch was already recorded (WR-03), so it is refused up front.
pub const MAX_PROMPT_BYTES: usize = 120 * 1024;

/// The headless driver's own command line (spike `claude` list).
///
/// The prompt is the LAST argument, after a `--`, so a template that starts with
/// `-` (a `---` front-matter fence, a `- ` bullet) is never read as an option
/// (WR-03). Verified against the real `claude` 2.1.304 (commit 6ffda4c, Phase 28
/// `28-REVIEW-FIX.md` WR-03): a `---`-leading prompt after `--` succeeds, and the same
/// prompt without `--` is rejected as an unknown option.
pub fn claude_argv(prompt: &str, model: &str, max_budget_usd: f64) -> Vec<OsString> {
    let mut driver_argv: Vec<OsString> = vec![os("claude"), os("-p")];
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
    driver_argv.push(os("--"));
    driver_argv.push(os(prompt));
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

/// The child's whole environment: `host` minus [`is_stripped_env`] (the
/// [`STRIP_ENV`] lists and the secret-name families), then `GIT_TERMINAL_PROMPT=0` (a push from inside must
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
        .filter(|(key, _)| !is_stripped_env(key))
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
    fn child_env_strips_cloud_forge_and_secret_named_variables() {
        let secrets = [
            "AWS_ACCESS_KEY_ID",
            "AWS_SESSION_TOKEN",
            "AZURE_CLIENT_SECRET",
            "GOOGLE_APPLICATION_CREDENTIALS",
            "GCP_PROJECT_KEY_ALIAS_TOKEN",
            "GITLAB_TOKEN",
            "GH_ENTERPRISE_TOKEN",
            "GITHUB_PAT",
            "NPM_TOKEN",
            "CARGO_REGISTRY_TOKEN",
            "CARGO_REGISTRIES_X_TOKEN",
            "OPENAI_API_KEY",
            "DB_PASSWORD",
            "my_secret",
            "GIT_ASKPASS",
            "SSH_ASKPASS",
            "GIT_SSH_COMMAND",
            "KUBECONFIG",
        ];
        let mut host: Vec<(OsString, OsString)> = secrets
            .iter()
            .map(|k| (OsString::from(k), OsString::from("v")))
            .collect();
        for keep in ["PATH", "HOME", "TERM", "LANG", "HTTPS_PROXY", "CARGO_HOME"] {
            host.push((OsString::from(keep), OsString::from("v")));
        }
        host.push((
            OsString::from("CLAUDE_CODE_OAUTH_TOKEN"),
            OsString::from("host"),
        ));
        let env = child_env(
            host,
            &[("CLAUDE_CODE_OAUTH_TOKEN", OsString::from("launcher"))],
        );
        for key in secrets {
            assert!(!env.contains_key(&OsString::from(key)), "{key} survived");
        }
        for keep in ["PATH", "HOME", "TERM", "LANG", "HTTPS_PROXY", "CARGO_HOME"] {
            assert!(env.contains_key(&OsString::from(keep)), "{keep} was lost");
        }
        assert_eq!(env[&OsString::from("CLAUDE_CODE_OAUTH_TOKEN")], "launcher");
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
            "--tmpfs",
            "/work/proj/.planning/meta-manager/drivers",
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

    /// The index of the adjacent `flag, path` pair (or `flag, path, path` bind).
    fn pos(argv: &[String], flag: &str, path: &str) -> usize {
        let hits: Vec<usize> = (0..argv.len().saturating_sub(1))
            .filter(|&i| argv[i] == flag && argv[i + 1] == path)
            .collect();
        assert_eq!(hits.len(), 1, "{flag} {path} must appear exactly once");
        hits[0]
    }

    const DRIVERS: &str = "/work/proj/.planning/meta-manager/drivers";

    #[test]
    fn sibling_runs_are_hidden_by_a_tmpfs_before_the_run_dir_is_re_bound() {
        let argv = strings(&bwrap_argv(&spec(), &[OsString::from("claude")]));
        let run = format!("{DRIVERS}/r1");
        let project = pos(&argv, "--bind", "/work/proj");
        let tmpfs = pos(&argv, "--tmpfs", DRIVERS);
        let run_bind = pos(&argv, "--ro-bind", &run);
        let lock = argv
            .iter()
            .position(|a| a == &format!("{run}/launcher.lock"))
            .unwrap();
        let direction = argv
            .iter()
            .position(|a| a == &format!("{run}/DIRECTION.lock"))
            .unwrap();
        let cfg = pos(&argv, "--bind", "/data/driver/ab12cd34/r1/claude-config");
        let settings = pos(
            &argv,
            "--ro-bind",
            "/data/driver/ab12cd34/r1/claude-config/settings.json",
        );
        let bin = pos(
            &argv,
            "--ro-bind",
            "/data/driver/ab12cd34/r1/claude-config/bin",
        );
        let runtime = pos(&argv, "--tmpfs", "/run/user/1000");
        let sock = pos(
            &argv,
            "--bind",
            "/run/user/1000/gsdmm-driverctl/r1-ab12cd34.sock",
        );
        assert_eq!(
            tmpfs,
            project + 3,
            "the tmpfs follows the project bind triple"
        );
        assert_eq!(run_bind, tmpfs + 2, "the run-dir ro-bind follows the tmpfs");
        assert!(run_bind < lock && lock < direction && direction < cfg);
        assert!(cfg < settings && settings < bin && bin < runtime && runtime < sock);
    }

    #[test]
    fn a_config_dir_under_the_drivers_dir_is_bound_after_the_hiding_tmpfs() {
        let mut s = spec();
        let cfg = format!("{DRIVERS}/cfg");
        s.paths.config_dir = PathBuf::from(&cfg);
        let argv = strings(&bwrap_argv(&s, &[OsString::from("claude")]));
        let tmpfs = pos(&argv, "--tmpfs", DRIVERS);
        assert!(pos(&argv, "--bind", &cfg) > tmpfs);
        assert!(pos(&argv, "--ro-bind", &format!("{cfg}/settings.json")) > tmpfs);
        assert!(pos(&argv, "--ro-bind", &format!("{cfg}/bin")) > tmpfs);
    }

    #[test]
    fn no_drivers_tmpfs_unless_the_run_dir_sits_directly_under_the_projects_drivers_dir() {
        for run in [
            "/work/proj/r1",
            "/other/.planning/meta-manager/drivers/r1",
            "/work/proj/.planning/meta-manager/drivers/a/r1",
        ] {
            let mut s = spec();
            s.paths.run_dir = PathBuf::from(run);
            let argv = strings(&bwrap_argv(&s, &[OsString::from("claude")]));
            for i in 0..argv.len() - 1 {
                if argv[i] == "--tmpfs" {
                    assert!(
                        !argv[i + 1].ends_with(".planning/meta-manager/drivers")
                            && argv[i + 1] != "/work/proj",
                        "{run}: unexpected tmpfs {}",
                        argv[i + 1]
                    );
                }
            }
            // Exactly the pre-change profile: the golden minus the pair.
            let base = strings(&bwrap_argv(&spec(), &[OsString::from("claude")]));
            let mut want: Vec<String> = base.clone();
            let t = pos(&base, "--tmpfs", DRIVERS);
            want.drain(t..t + 2);
            for e in &mut want {
                if e.starts_with("/work/proj/.planning/meta-manager/drivers/r1") {
                    *e = e.replacen("/work/proj/.planning/meta-manager/drivers/r1", run, 1);
                }
            }
            assert_eq!(argv, want, "{run}");
        }
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
                "10",
                "--",
                "do it"
            ]
        );
    }

    #[test]
    fn claude_argv_keeps_a_dash_leading_prompt_after_the_double_dash() {
        let argv = strings(&claude_argv("---\ntitle: x\n---", "opus", 10.0));
        let n = argv.len();
        assert_eq!(argv[n - 2], "--");
        assert_eq!(argv[n - 1], "---\ntitle: x\n---");
        assert_eq!(argv.iter().filter(|a| *a == "--").count(), 1);
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
