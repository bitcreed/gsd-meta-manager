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
//! * `$HOME` replaced by a size-capped tmpfs with only the derived allowlist re-bound
//!   (28.1-08, D-16; see [`super::home`]), a per-run gitconfig at `~/.gitconfig`, and
//!   the credential files inside an allowlisted bind masked as a second layer (D-20);
//! * the per-run config directory read-write;
//! * `--unshare-pid --new-session --die-with-parent` (28.1 adds the network, IPC and UTS
//!   namespaces, see below).
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
//!
//! 28.1 (D-12) moves the gate into Claude Code's managed tier: `/etc` is replaced
//! by a tmpfs, every host `/etc` entry is re-bound read-only (the host's own
//! `/etc/claude-code` skipped), the per-run `<cfg>/managed` is bound read-only at
//! `/etc/claude-code`, and `/etc` is remounted read-only. That works on every bwrap
//! version (overlay mounts need 0.11; Ubuntu 24.04 ships 0.9), RESEARCH Q2
//! `[inferred — audit]`. `<cfg>/managed` is also re-bound read-only at its own path
//! after the writable config-dir bind, so the driver cannot rewrite the file
//! through the config dir.
//!
//! 28.1 (D-03) also replaces `/run` with an empty tmpfs, right after the `/etc` block, so no
//! host filesystem socket under `/run` or `/var/run` is reachable; the runtime-dir
//! tmpfs and the two sockets are mounted inside it afterwards.
//!
//! 28.1 (D-02, D-04) cuts the profile off the host network: `--unshare-net` leaves
//! a fresh namespace with only `lo`, `--unshare-ipc` and `--unshare-uts` drop the
//! host's SysV IPC and hostname (the hostname is fixed to [`SANDBOX_HOSTNAME`]). The only
//! way out is the run's egress socket, re-bound next to the driverctl socket inside
//! the runtime-dir tmpfs and reached through `gsd-meta-manager bridge` (see
//! [`bridged_command`]), which listens on `127.0.0.1:`[`BRIDGE_PORT`]. Claude Code
//! is pointed at that port by [`egress_env_sets`] and holds only
//! [`DUMMY_OAUTH_TOKEN`]; the real token lives in the launcher-side egress server.
//!
//! Quick task 261103-f5y adds a second run-dir profile, [`RunDirMode::ReportWritable`],
//! for one-off runs (`crate::oneoff`): the "run dir" bound there is the run's `report/`
//! directory, bound read-write, with no lock masks and no driverctl socket. Everything
//! else is the driver profile, in the same order ([`bwrap_argv_with`]).
//!
//! Quick task 261103-hx7 (D-09) adds [`bwrap_argv_masking`]: the app's data and config
//! roots (`dirs::data_local_dir()` and `dirs::config_dir()` joined with
//! `gsd-meta-manager`) are masked with an empty tmpfs when they live outside `$HOME` and
//! `/tmp`, which the read-only root would otherwise leave readable (one-off inboxes, the
//! stored run token). [`masks_outside`] is the pure filter; the host-reading half is
//! `config_dir::app_root_masks`. With no masks the argv is unchanged.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use crate::state_reader::driver_frontmatter::DRIVER_RUNS_REL;

use super::home::{HomeBind, HomeSetup, HOME_TMPFS_SIZE};

/// The first word of every scope unit name (D-08).
pub const SCOPE_PREFIX: &str = "gsdmm-driver";

/// The loopback port of the in-sandbox bridge (D-08). Fixed: the sandbox's network
/// namespace is private and empty, so nothing else can hold the port
/// `[inferred — audit]`.
pub const BRIDGE_PORT: u16 = 3128;

/// The token the driver holds (D-04): well-formed enough for Claude Code to accept,
/// worth nothing upstream. The egress server replaces whatever credential arrives.
pub const DUMMY_OAUTH_TOKEN: &str = "sk-ant-oat01-DUMMY-placeholder-not-a-real-token";

/// The hostname inside the sandbox's UTS namespace (D-02): the host's name is not
/// leaked `[inferred — audit]`.
pub const SANDBOX_HOSTNAME: &str = "gsdmm-sandbox";

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
/// lists [`STRIP_ENV`], [`EXTRA_STRIP_ENV`], [`MORE_STRIP_ENV`] and
/// [`ROUTING_STRIP_ENV`] plus the [`STRIP_ENV_PREFIXES`] and [`STRIP_ENV_SUFFIXES`]
/// families.
///
/// This stays a denylist because the driver legitimately needs a long tail of
/// ordinary variables (`PATH`, toolchain homes, `HTTPS_PROXY` for a degraded
/// run, ...) that an allowlist would have to guess `[inferred — audit]`.
/// `ANTHROPIC_BASE_URL` and the catch-all proxy variable are no longer among them:
/// the bwrap posture sets its own base URL and proxy values
/// ([`egress_env_sets`]) and the host's must not survive in either posture, while
/// the host's ordinary `HTTP(S)_PROXY` still passes for degraded runs. The
/// launcher's own values (`CLAUDE_CODE_OAUTH_TOKEN`, ...) are applied AFTER this
/// filter by [`child_env`], so a family match never removes what the launcher sets.
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
        .chain(ROUTING_STRIP_ENV)
        .any(|exact| name == *exact)
        || STRIP_ENV_PREFIXES.iter().any(|p| upper.starts_with(p))
        || STRIP_ENV_SUFFIXES.iter().any(|x| upper.ends_with(x))
}

/// Routing and session variables stripped in BOTH postures (D-09a, RESEARCH
/// Pitfall 14) `[inferred — audit]`: the launching shell's `TMUX` session, a
/// catch-all `ALL_PROXY` that would bypass the bridge, the Bedrock/Vertex/Foundry
/// switches that send inference to another provider, extra request headers, and a
/// host `ANTHROPIC_BASE_URL`. The bwrap posture then sets its own base URL and
/// proxy values ([`egress_env_sets`]); the degraded posture sets none.
pub const ROUTING_STRIP_ENV: &[&str] = &[
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

/// The variables set in both postures (D-09a, RESEARCH Pitfall 14): no telemetry,
/// error reporting, auto-update or other non-essential traffic from the driver,
/// and the claude.ai MCP connectors off. In the bwrap posture they also keep the
/// driver quiet on a network that has only the egress allowlist.
pub fn common_env_sets() -> Vec<(&'static str, OsString)> {
    vec![
        ("DISABLE_TELEMETRY", os("1")),
        ("DISABLE_ERROR_REPORTING", os("1")),
        ("DISABLE_AUTOUPDATER", os("1")),
        ("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", os("1")),
        ("ENABLE_CLAUDEAI_MCP_SERVERS", os("false")),
    ]
}

/// Credential directories under `$HOME`. **No longer in the argv** (28.1-08, D-20): the
/// `$HOME` tmpfs hides them with every other unlisted path. The list survives as a test
/// invariant, `no_allowlist_entry_overlaps_a_mask_dir`: no allowlist entry may equal,
/// contain or lie inside one of these.
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

/// Files under `$HOME` the sandbox masks with `/dev/null` when they exist inside an
/// allowlisted read-only bind, as a second layer (D-20). The git entries close
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

/// Project directories bound onto themselves when they exist (D-21), so the driver
/// cannot rename them away (`rename(2)` on a mount point fails with `EBUSY`) and swap
/// in a directory of its own.
pub const GUARD_SELF_DIRS: &[&str] = &[".git", ".claude"];

/// The project-relative paths bound read-only inside the self-bound directories
/// (D-21), with whether each is a directory. A path that is absent at launch gets no
/// bind: placing a placeholder in the operator's project would leave junk behind
/// (LNCH-11 prohibition), so detection alone (D-22) covers it.
pub const GUARD_RO_REL: &[(&str, bool)] = &[
    (".git/config", false),
    (".git/hooks", true),
    (".claude/settings.json", false),
    (".claude/settings.local.json", false),
    (".claude/hooks", true),
    (".claude/agents", true),
    (".claude/commands", true),
];

/// One bind of the planted-file guard (D-21).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardBind {
    /// `--bind p p`: the directory stays writable but cannot be renamed or replaced.
    SelfBind(PathBuf),
    /// `--ro-bind p p`: the file or directory is unwritable.
    Ro(PathBuf),
}

/// The guard binds for `project`, existing paths only, each self-bind before its
/// children. A `.git` that is a file (a worktree checkout) gets no `.git` binds: its
/// pointer is covered by detection only. A symlinked `.git` or `.claude` gets none
/// either: binding through a link would bind the target, not the project's entry.
pub fn guard_binds_for(project: &Path) -> Vec<GuardBind> {
    let is_real_dir = |p: &Path| {
        std::fs::symlink_metadata(p)
            .map(|m| m.file_type().is_dir())
            .unwrap_or(false)
    };
    let mut binds = Vec::new();
    for dir in GUARD_SELF_DIRS {
        let root = project.join(dir);
        if !is_real_dir(&root) {
            continue;
        }
        binds.push(GuardBind::SelfBind(root));
        for (rel, is_dir) in GUARD_RO_REL {
            if !rel.starts_with(&format!("{dir}/")) {
                continue;
            }
            let path = project.join(rel);
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            let kind = meta.file_type();
            // Regular files and directories only: a symlink is detection-only.
            if (*is_dir && kind.is_dir()) || (!*is_dir && kind.is_file()) {
                binds.push(GuardBind::Ro(path));
            }
        }
    }
    binds
}

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
    /// The run's egress socket ([`crate::launcher::egress::socket_path_for`]): re-bound
    /// next to the driverctl socket, the bridge's only way out.
    pub egress_socket: PathBuf,
}

/// One host `/etc` entry re-created inside the sandbox's tmpfs `/etc` (D-12).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EtcEntry {
    /// A file or directory: bound read-only at its own path.
    Bind(PathBuf),
    /// A symlink, re-created as a symlink with the same target.
    Symlink { target: PathBuf, path: PathBuf },
}

/// What [`bwrap_argv`] builds the sandbox from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxSpec {
    /// The user's home directory the masks are relative to.
    pub home: PathBuf,
    pub paths: LaunchPaths,
    /// The planted-file guard (D-21): self-binds and read-only binds of the project's
    /// existing `.git` and `.claude` paths, emitted right after the project bind.
    pub guard_binds: Vec<GuardBind>,
    /// The `$HOME` entries re-bound over the tmpfs `$HOME` (D-16), parents first.
    pub home_binds: Vec<HomeBind>,
    /// The per-run gitconfig, bound read-only at `<home>/.gitconfig` (D-17).
    pub gitconfig: Option<PathBuf>,
    /// Files masked with `/dev/null` (every one exists on the host and lies inside
    /// a read-only entry of [`home_binds`](Self::home_binds)).
    pub null_masks: Vec<PathBuf>,
    /// The host's `$XDG_RUNTIME_DIR`, replaced by an empty tmpfs inside with only
    /// [`LaunchPaths::socket`] re-bound. `None` leaves it unmasked (no such dir).
    pub runtime_dir: Option<PathBuf>,
    /// The host's `/etc` entries (the host `claude-code` skipped), sorted by name.
    /// Re-created read-only under the tmpfs `/etc` that carries the managed bind.
    pub etc_entries: Vec<EtcEntry>,
}

/// The entries of `/etc` as [`EtcEntry`] values, sorted by file name, without
/// `claude-code` (the per-run managed dir takes its place). An unreadable `/etc`
/// yields an empty list.
pub fn etc_entries_in(etc: &Path) -> Vec<EtcEntry> {
    let Ok(entries) = std::fs::read_dir(etc) else {
        return Vec::new();
    };
    let mut named: Vec<(OsString, EtcEntry)> = entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name() != "claude-code")
        .map(|entry| {
            let path = entry.path();
            let kind = match std::fs::read_link(&path) {
                Ok(target) => EtcEntry::Symlink {
                    target,
                    path: path.clone(),
                },
                Err(_) => EtcEntry::Bind(path),
            };
            (entry.file_name(), kind)
        })
        .collect();
    named.sort_by(|a, b| a.0.cmp(&b.0));
    named.into_iter().map(|(_, entry)| entry).collect()
}

impl SandboxSpec {
    /// The spec for this host: the `$HOME` setup of this launch, the null-mask
    /// candidates that exist inside one of its read-only entries, and
    /// `$XDG_RUNTIME_DIR` when it is an absolute path. **The one function here that
    /// reads the file system and the environment.**
    pub fn from_host(paths: LaunchPaths, home: &Path, setup: &HomeSetup) -> Self {
        let ro_entries: Vec<&Path> = setup
            .binds
            .iter()
            .filter_map(|bind| match bind {
                HomeBind::Ro(path) => Some(path.as_path()),
                _ => None,
            })
            .collect();
        let in_a_bind = |path: &Path| ro_entries.iter().any(|root| path.starts_with(root));
        #[cfg_attr(not(any(target_os = "linux", target_os = "macos")), allow(unused_mut))]
        let mut null_masks: Vec<PathBuf> = NULL_MASK_FILES
            .iter()
            .map(|rel| home.join(rel))
            .filter(|p| p.exists() && in_a_bind(p))
            .collect();
        // The stored run token lives under `dirs::config_dir()`, which an
        // `XDG_CONFIG_HOME` moves away from `~/.config` (WR-02).
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        if let Some(token) = super::auth::token_path() {
            if token.exists() && in_a_bind(&token) && !null_masks.contains(&token) {
                null_masks.push(token);
            }
        }
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute());
        let guard_binds = guard_binds_for(&paths.project);
        SandboxSpec {
            home: home.to_owned(),
            paths,
            guard_binds,
            home_binds: setup.binds.clone(),
            gitconfig: setup.gitconfig.clone(),
            null_masks,
            runtime_dir,
            etc_entries: etc_entries_in(Path::new("/etc")),
        }
    }
}

fn os(s: &str) -> OsString {
    OsString::from(s)
}

/// The entries of the `$HOME` phase whose path `select` accepts: the allowlist binds,
/// the per-run gitconfig at `~/.gitconfig` and the `/dev/null` credential masks, in that
/// order. [`bwrap_argv`] calls it twice, so an entry under the project path follows the
/// project bind (WR-05).
fn push_home_entries(argv: &mut Vec<OsString>, spec: &SandboxSpec, select: impl Fn(&Path) -> bool) {
    for entry in spec.home_binds.iter().filter(|entry| select(entry.path())) {
        match entry {
            HomeBind::Ro(path) => {
                argv.push(os("--ro-bind"));
                argv.push(path.as_os_str().to_owned());
                argv.push(path.as_os_str().to_owned());
            }
            // The upper layer lives in RAM and vanishes with the sandbox: a crate
            // planted here is never seen by the host (D-15, T-28.1-34).
            HomeBind::Overlay(path) => {
                argv.push(os("--overlay-src"));
                argv.push(path.as_os_str().to_owned());
                argv.push(os("--tmp-overlay"));
                argv.push(path.as_os_str().to_owned());
            }
            HomeBind::Tmpfs { path, size } => {
                argv.push(os("--size"));
                argv.push(OsString::from(size.to_string()));
                argv.push(os("--tmpfs"));
                argv.push(path.as_os_str().to_owned());
            }
        }
    }
    // D-17: the per-run gitconfig, never the host's.
    if let Some(gitconfig) = &spec.gitconfig {
        let target = spec.home.join(".gitconfig");
        if select(&target) {
            argv.push(os("--ro-bind"));
            argv.push(gitconfig.as_os_str().to_owned());
            argv.push(target.into_os_string());
        }
    }
    for file in spec.null_masks.iter().filter(|file| select(file)) {
        argv.push(os("--ro-bind"));
        argv.push(os("/dev/null"));
        argv.push(file.as_os_str().to_owned());
    }
}

/// The `bwrap` command line that runs `command` in the sandbox, in a fixed
/// order (the order is the contract: a later bind covers an earlier one).
///
/// The drivers dir is covered by an empty tmpfs right after the project bind, so
/// sibling runs are hidden; the run's own ro-bind and lock masks follow it and are
/// re-established on top.
pub fn bwrap_argv(spec: &SandboxSpec, command: &[OsString]) -> Vec<OsString> {
    bwrap_argv_with(spec, RunDirMode::DriverReadOnly, command)
}

/// How [`bwrap_argv_with`] binds [`LaunchPaths::run_dir`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunDirMode {
    /// The driver profile: the run dir read-only, its two lock files masked with
    /// `/dev/null`, the driverctl socket re-bound into the runtime-dir tmpfs.
    DriverReadOnly,
    /// The one-off profile (quick 261103-f5y): the run dir (the run's `report/`
    /// directory) read-write, no lock masks, no driverctl socket. The egress socket
    /// is still re-bound.
    ReportWritable,
}

/// [`bwrap_argv`] with the run-dir profile chosen by `mode`. Every other entry is the
/// same, in the same order.
pub fn bwrap_argv_with(
    spec: &SandboxSpec,
    mode: RunDirMode,
    command: &[OsString],
) -> Vec<OsString> {
    bwrap_argv_masking(spec, mode, &[], command)
}

/// The directories among `candidates` that the sandbox must mask with an empty tmpfs
/// because `$HOME` and `/tmp` (already masked) do not cover them (quick 261103-hx7,
/// D-09, C-32). Pure and lexical: a candidate under `home` or under `/tmp` is dropped,
/// and so is a duplicate. The order of `candidates` is kept.
pub fn masks_outside(home: &Path, candidates: &[PathBuf]) -> Vec<PathBuf> {
    let mut kept: Vec<PathBuf> = Vec::new();
    for candidate in candidates {
        if candidate.starts_with(home) || candidate.starts_with("/tmp") || kept.contains(candidate)
        {
            continue;
        }
        kept.push(candidate.clone());
    }
    kept
}

/// [`bwrap_argv_with`] plus an empty `--tmpfs` for every directory in `masks` (quick
/// 261103-hx7, D-09).
///
/// The root is mounted read-only and only `/tmp` and `$HOME` are masked, so the host's
/// application data and config roots stay readable inside the sandbox when
/// `XDG_DATA_HOME` or `XDG_CONFIG_HOME` points outside `$HOME`: the one-off run dirs
/// with their host-only inbox, the driver's config dirs, and the stored run token. Each
/// mask follows the non-project `$HOME` entries and precedes the project bind, so the
/// binds that re-establish this run's own paths (clone, report dir, config dir,
/// sockets) land on top of it. With `masks` empty the argv is byte-identical to
/// [`bwrap_argv_with`]'s historical output.
pub fn bwrap_argv_masking(
    spec: &SandboxSpec,
    mode: RunDirMode,
    masks: &[PathBuf],
    command: &[OsString],
) -> Vec<OsString> {
    let paths = &spec.paths;
    let mut argv: Vec<OsString> = ["bwrap", "--ro-bind", "/", "/", "--dev", "/dev", "--proc"]
        .iter()
        .map(|s| os(s))
        .collect();
    argv.extend(["/proc", "--tmpfs", "/tmp"].iter().map(|s| os(s)));
    // D-12: managed settings reach the driver at `/etc/claude-code`. A plain bind
    // onto an absent path fails on a read-only root, so `/etc` becomes a tmpfs, the
    // host entries are re-bound read-only, the managed dir takes the place of the
    // host's `claude-code`, and the whole of `/etc` is remounted read-only.
    argv.push(os("--tmpfs"));
    argv.push(os("/etc"));
    for entry in &spec.etc_entries {
        match entry {
            // `-try`: an entry removed between the listing and the exec must not
            // abort the launch `[inferred — audit]`.
            EtcEntry::Bind(path) => {
                argv.push(os("--ro-bind-try"));
                argv.push(path.as_os_str().to_owned());
                argv.push(path.as_os_str().to_owned());
            }
            EtcEntry::Symlink { target, path } => {
                argv.push(os("--symlink"));
                argv.push(target.as_os_str().to_owned());
                argv.push(path.as_os_str().to_owned());
            }
        }
    }
    argv.push(os("--ro-bind"));
    argv.push(paths.config_dir.join("managed").into_os_string());
    argv.push(os("/etc/claude-code"));
    argv.push(os("--remount-ro"));
    argv.push(os("/etc"));
    // D-03: `/run` (and `/var/run`, its symlink) holds the host's root-equivalent
    // sockets (`docker.sock`, the system bus, snapd, ssh-unix-local). It is an empty
    // tmpfs here; the runtime dir below is then a tmpfs inside it, so `/run` lists
    // only the mount point `user`.
    argv.push(os("--tmpfs"));
    argv.push(os("/run"));
    // D-16: `$HOME` is a size-capped tmpfs and only the derived allowlist is re-bound
    // over it. All of this precedes the project, run-dir, config-dir and socket binds:
    // they live under `$HOME`.
    argv.push(os("--size"));
    argv.push(OsString::from(HOME_TMPFS_SIZE.to_string()));
    argv.push(os("--tmpfs"));
    argv.push(spec.home.as_os_str().to_owned());
    // WR-05: an entry that lies under the project is emitted after the project bind
    // (below), because a later bind covers an earlier one.
    push_home_entries(&mut argv, spec, |path| !path.starts_with(&paths.project));
    // D-09: host roots outside `$HOME` and `/tmp` that hold this app's own state.
    for mask in masks {
        argv.push(os("--tmpfs"));
        argv.push(mask.as_os_str().to_owned());
    }
    let bind = |argv: &mut Vec<OsString>, flag: &str, path: &Path| {
        argv.push(os(flag));
        argv.push(path.as_os_str().to_owned());
        argv.push(path.as_os_str().to_owned());
    };
    bind(&mut argv, "--bind", &paths.project);
    // WR-05: the `$HOME` entries inside the project are re-established on top of the
    // writable project bind, so they are not silently covered by it. A binary of the
    // launcher is never one of them (it is snapshotted out of the project,
    // `config_dir::snapshot_exe`): a read-only bind of the file `cargo build` replaces
    // would make that build fail with `EBUSY`.
    push_home_entries(&mut argv, spec, |path| path.starts_with(&paths.project));
    // D-21: the planted-file guard follows the project bind (a later bind covers an
    // earlier one) and precedes the drivers tmpfs; each self-bind precedes its children.
    for guard in &spec.guard_binds {
        match guard {
            GuardBind::SelfBind(path) => bind(&mut argv, "--bind", path),
            GuardBind::Ro(path) => bind(&mut argv, "--ro-bind", path),
        }
    }
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
    match mode {
        RunDirMode::DriverReadOnly => {
            bind(&mut argv, "--ro-bind", &paths.run_dir);
            for lock in ["launcher.lock", "DIRECTION.lock"] {
                argv.push(os("--ro-bind"));
                argv.push(os("/dev/null"));
                argv.push(paths.run_dir.join(lock).into_os_string());
            }
        }
        RunDirMode::ReportWritable => bind(&mut argv, "--bind", &paths.run_dir),
    }
    bind(&mut argv, "--bind", &paths.config_dir);
    // The gate's settings and the shim its hooks call stay unwritable for the driver.
    bind(
        &mut argv,
        "--ro-bind",
        &paths.config_dir.join("settings.json"),
    );
    bind(&mut argv, "--ro-bind", &paths.config_dir.join("bin"));
    // D-12: the managed file is also reachable through the writable config dir; it
    // is re-bound read-only here so the driver cannot rewrite what `/etc/claude-code`
    // shows.
    bind(&mut argv, "--ro-bind", &paths.config_dir.join("managed"));
    if let Some(runtime) = &spec.runtime_dir {
        argv.push(os("--tmpfs"));
        argv.push(runtime.as_os_str().to_owned());
        if mode == RunDirMode::DriverReadOnly && paths.socket.starts_with(runtime) {
            bind(&mut argv, "--bind", &paths.socket);
        }
        if paths.egress_socket.starts_with(runtime) {
            bind(&mut argv, "--bind", &paths.egress_socket);
        }
    }
    // D-02: a fresh network namespace (only `lo`), SysV IPC and UTS; the bridge is the
    // sandbox's one way out.
    for flag in [
        "--unshare-pid",
        "--unshare-net",
        "--unshare-ipc",
        "--unshare-uts",
    ] {
        argv.push(os(flag));
    }
    argv.push(os("--hostname"));
    argv.push(os(SANDBOX_HOSTNAME));
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

/// The command bwrap runs: the bridge, then the driver as its child (D-08).
///
/// `<exe> bridge --port <port> --sock <sock> -- <inner...>` (the 28.1-05
/// subcommand). `exe` is this binary, readable inside because the root is bound
/// read-only.
pub fn bridged_command(exe: &Path, port: u16, sock: &Path, inner: &[OsString]) -> Vec<OsString> {
    let mut argv = vec![
        exe.as_os_str().to_owned(),
        os("bridge"),
        os("--port"),
        OsString::from(port.to_string()),
        os("--sock"),
        sock.as_os_str().to_owned(),
        os("--"),
    ];
    argv.extend(inner.iter().cloned());
    argv
}

/// The environment that sends every Claude Code request to the bridge (D-04):
/// the base URL and both proxy spellings at `http://127.0.0.1:<port>`, loopback
/// exempt from the proxy, no gzip request bodies (the egress server reads the body
/// to judge it), and the dummy token. Applied after the scrub by [`child_env`].
pub fn egress_env_sets(port: u16) -> Vec<(&'static str, OsString)> {
    let url = OsString::from(format!("http://127.0.0.1:{port}"));
    vec![
        ("ANTHROPIC_BASE_URL", url.clone()),
        ("HTTPS_PROXY", url.clone()),
        ("https_proxy", url.clone()),
        ("HTTP_PROXY", url.clone()),
        ("http_proxy", url),
        ("NO_PROXY", os("127.0.0.1,localhost")),
        ("no_proxy", os("127.0.0.1,localhost")),
        ("CLAUDE_CODE_GZIP_REQUEST_BODIES", os("0")),
        ("CLAUDE_CODE_OAUTH_TOKEN", os(DUMMY_OAUTH_TOKEN)),
    ]
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
            egress_socket: PathBuf::from("/run/user/1000/gsdmm-driverctl/egress-ab12cd34.sock"),
        }
    }

    fn spec() -> SandboxSpec {
        SandboxSpec {
            home: PathBuf::from("/home/u"),
            paths: paths(),
            guard_binds: Vec::new(),
            home_binds: vec![
                HomeBind::Ro(PathBuf::from("/home/u/.claude/hooks")),
                HomeBind::Overlay(PathBuf::from("/home/u/.cargo/registry")),
                HomeBind::Tmpfs {
                    path: PathBuf::from("/home/u/.cache"),
                    size: 1_073_741_824,
                },
            ],
            gitconfig: Some(PathBuf::from("/data/driver/ab12cd34/r1/gitconfig")),
            null_masks: vec![PathBuf::from("/home/u/.claude/hooks/token")],
            runtime_dir: Some(PathBuf::from("/run/user/1000")),
            etc_entries: vec![
                EtcEntry::Bind(PathBuf::from("/etc/hosts")),
                EtcEntry::Symlink {
                    target: PathBuf::from("../run/resolv.conf"),
                    path: PathBuf::from("/etc/resolv.conf"),
                },
            ],
        }
    }

    fn strings(argv: &[OsString]) -> Vec<String> {
        argv.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn the_report_writable_profile_differs_from_the_driver_profile_in_three_places_only() {
        let spec = spec();
        let command = vec![os("claude"), os("-p")];
        let driver = strings(&bwrap_argv(&spec, &command));
        assert_eq!(
            driver,
            strings(&bwrap_argv_with(
                &spec,
                RunDirMode::DriverReadOnly,
                &command
            ))
        );
        let oneoff = strings(&bwrap_argv_with(
            &spec,
            RunDirMode::ReportWritable,
            &command,
        ));
        let run_dir = spec.paths.run_dir.display().to_string();
        let socket = spec.paths.socket.display().to_string();
        // Rebuild the expected one-off argv from the driver one by applying exactly the
        // three documented differences.
        let mut expected: Vec<String> = Vec::new();
        let mut i = 0;
        while i < driver.len() {
            let triple = &driver[i..(i + 3).min(driver.len())];
            if triple.len() == 3 && triple[0] == "--ro-bind" && triple[1] == run_dir {
                expected.extend(["--bind".to_owned(), run_dir.clone(), run_dir.clone()]);
                i += 3;
            } else if triple.len() == 3
                && ((triple[0] == "--ro-bind"
                    && triple[1] == "/dev/null"
                    && (triple[2].ends_with("launcher.lock")
                        || triple[2].ends_with("DIRECTION.lock")))
                    || (triple[0] == "--bind" && triple[1] == socket))
            {
                i += 3;
            } else {
                expected.push(driver[i].clone());
                i += 1;
            }
        }
        assert_eq!(oneoff, expected);
        assert_eq!(
            driver.len() - oneoff.len(),
            9,
            "two masks and one socket bind"
        );
        let egress = spec.paths.egress_socket.display().to_string();
        assert!(oneoff.contains(&egress), "the egress socket is still bound");
        assert!(!oneoff.iter().any(|a| a.ends_with("launcher.lock")));
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
            "/etc",
            "--ro-bind-try",
            "/etc/hosts",
            "/etc/hosts",
            "--symlink",
            "../run/resolv.conf",
            "/etc/resolv.conf",
            "--ro-bind",
            "/data/driver/ab12cd34/r1/claude-config/managed",
            "/etc/claude-code",
            "--remount-ro",
            "/etc",
            "--tmpfs",
            "/run",
            "--size",
            "268435456",
            "--tmpfs",
            "/home/u",
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
            "/data/driver/ab12cd34/r1/gitconfig",
            "/home/u/.gitconfig",
            "--ro-bind",
            "/dev/null",
            "/home/u/.claude/hooks/token",
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
            "--ro-bind",
            "/data/driver/ab12cd34/r1/claude-config/managed",
            "/data/driver/ab12cd34/r1/claude-config/managed",
            "--tmpfs",
            "/run/user/1000",
            "--bind",
            "/run/user/1000/gsdmm-driverctl/r1-ab12cd34.sock",
            "/run/user/1000/gsdmm-driverctl/r1-ab12cd34.sock",
            "--bind",
            "/run/user/1000/gsdmm-driverctl/egress-ab12cd34.sock",
            "/run/user/1000/gsdmm-driverctl/egress-ab12cd34.sock",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
        let tail = "--unshare-pid --unshare-net --unshare-ipc --unshare-uts --hostname \
                    gsdmm-sandbox --new-session --die-with-parent --chdir /work/proj -- \
                    claude -p";
        let (head, rest) = argv.split_at(expected.len());
        assert_eq!(head, expected);
        assert_eq!(rest.join(" "), tail);
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
    fn home_entries_under_the_project_follow_the_project_bind() {
        // WR-05: a later bind covers an earlier one, so a read-only, overlay or tmpfs
        // entry (and the gitconfig and the credential masks) that lies under the
        // project must be emitted after `--bind project project`.
        let mut s = spec();
        s.home = PathBuf::from("/work");
        s.home_binds = vec![
            HomeBind::Ro(PathBuf::from("/work/tools")),
            HomeBind::Ro(PathBuf::from("/work/proj/tools/bin")),
            HomeBind::Overlay(PathBuf::from("/work/proj/vendor/cache")),
            HomeBind::Tmpfs {
                path: PathBuf::from("/work/proj/scratch"),
                size: 4096,
            },
        ];
        s.gitconfig = Some(PathBuf::from("/data/gitconfig"));
        s.null_masks = vec![
            PathBuf::from("/work/.npmrc"),
            PathBuf::from("/work/proj/.npmrc"),
        ];
        let argv = strings(&bwrap_argv(&s, &[OsString::from("claude")]));
        let project = pos(&argv, "--bind", "/work/proj");
        let before = |i: usize| i < project;
        // Outside the project: before the project bind.
        assert!(before(pos(&argv, "--ro-bind", "/work/tools")));
        assert!(before(pos(&argv, "--ro-bind", "/data/gitconfig")));
        let masks: Vec<usize> = (0..argv.len())
            .filter(|&i| argv[i] == "/dev/null" && argv[i - 1] == "--ro-bind")
            .filter(|&i| argv[i + 1].ends_with(".npmrc"))
            .collect();
        assert_eq!(masks.len(), 2);
        assert!(before(masks[0]) && argv[masks[0] + 1] == "/work/.npmrc");
        // Under the project: after it.
        assert!(pos(&argv, "--ro-bind", "/work/proj/tools/bin") > project);
        assert!(pos(&argv, "--overlay-src", "/work/proj/vendor/cache") > project);
        assert!(pos(&argv, "--tmpfs", "/work/proj/scratch") > project);
        assert!(masks[1] > project && argv[masks[1] + 1] == "/work/proj/.npmrc");
        // Every entry appears once, none is dropped.
        assert_eq!(
            argv.iter().filter(|a| *a == "/work/proj/tools/bin").count(),
            2
        );
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
    fn from_host_keeps_only_null_masks_that_exist_inside_a_read_only_bind() {
        let home = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(home.path().join(".ssh")).unwrap();
        std::fs::write(home.path().join(".claude.json"), "{}").unwrap();
        std::fs::create_dir_all(home.path().join(".cargo/bin")).unwrap();
        std::fs::write(home.path().join(".cargo/bin/credentials"), "x").unwrap();
        let setup = HomeSetup {
            binds: vec![HomeBind::Ro(home.path().join(".cargo"))],
            gitconfig: Some(PathBuf::from("/state/gitconfig")),
            ..HomeSetup::default()
        };
        std::fs::write(home.path().join(".cargo/credentials.toml"), "x").unwrap();
        let spec = SandboxSpec::from_host(paths(), home.path(), &setup);
        // `.claude.json` exists but is in no bind (the tmpfs hides it); the cargo
        // credentials file is inside the read-only `.cargo` entry.
        assert_eq!(
            spec.null_masks,
            [home.path().join(".cargo/credentials.toml")]
        );
        assert_eq!(spec.home_binds, setup.binds);
        assert_eq!(spec.gitconfig, setup.gitconfig);
    }
}
