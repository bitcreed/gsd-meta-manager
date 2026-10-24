//! What the host can do for a launch (Phase 28 plan 28-04; D-28, D-29, D-31).
//!
//! The launcher refuses to run a driver unsandboxed unless that launch carries
//! `--unsandboxed`. This module is the half of that rule that looks at the host:
//! [`detect`] runs the probes, [`decide`] turns the answer and the flag into a
//! [`Posture`] or a typed refusal. A `which` is not a probe (RESEARCH §1): a
//! copied or AppArmor-blocked `bwrap` is on `PATH` and cannot create a user
//! namespace, so `bwrap` is only counted when it actually runs a trivial command.
//!
//! * `bwrap --unshare-pid --unshare-net --unshare-ipc --unshare-uts --ro-bind / /
//!   --dev /dev -- true` must exit 0. The probe carries the namespaces the 28.1
//!   profile unshares, so a host whose policy forbids them (an AppArmor rule that
//!   allows a plain user namespace only) is refused as `bwrap` missing instead of
//!   failing at spawn (RESEARCH Pitfall 12);
//! * overlay support, probed only when `bwrap` ran: `bwrap --version` at least
//!   [`BWRAP_OVERLAY_MIN`] (0.11.0, when `--overlay-src` / `--tmp-overlay` landed,
//!   RESEARCH Q2) AND a dry `--overlay-src /usr --tmp-overlay /usr -- true` that
//!   exits 0. It never refuses a launch; it only decides whether
//!   `registry-overlay` is in [`protections_absent`];
//! * `systemd-run --version` must exit 0;
//! * `systemctl --user is-system-running` must print `running` or `degraded`: the
//!   EXIT CODE is ignored, because a perfectly usable user manager that reports
//!   `degraded` (one failed unit) exits 1 (RESEARCH §2);
//! * `claude --version` must answer a version of at least [`MIN_CLAUDE_VERSION`]
//!   (the first Claude Code with the hook `onFailure` field the gate relies on,
//!   RESEARCH §3). That is checked in BOTH modes: the degraded driver runs the same
//!   gate hooks.
//!
//! Every probe goes through [`process::run_bounded`](super::process::run_bounded)
//! (the one launcher spawn file) with a [`PROBE_BUDGET`], so a hung tool cannot
//! hang the launch. The parsers are pure.

use std::ffi::OsString;
use std::time::Duration;

use super::process::run_bounded;
use super::LauncherError;

/// The oldest Claude Code whose hooks understand `onFailure` (RESEARCH §3).
pub const MIN_CLAUDE_VERSION: (u32, u32, u32) = (2, 1, 295);

/// The first bubblewrap with `--overlay-src` / `--tmp-overlay` (RESEARCH Q2; CONTEXT
/// D-15's "0.10" was outdated).
pub const BWRAP_OVERLAY_MIN: (u32, u32, u32) = (0, 11, 0);

/// How long each probe may take.
const PROBE_BUDGET: Duration = Duration::from_secs(5);

/// The names [`HostCaps::missing`] uses.
pub const MISSING_BWRAP: &str = "bwrap";
pub const MISSING_SYSTEMD_RUN: &str = "systemd-run";
pub const MISSING_USER_MANAGER: &str = "user-systemd";

/// The protection words [`protections_absent`] reports (D-27, corrected by RESEARCH
/// Q2). Each names an EFFECT the sandbox profile provides, not a host tool piece:
/// `sandbox_missing` keeps naming the pieces (`bwrap`, `systemd-run`, ...).
pub const ABSENT_NET_ISOLATION: &str = "net-isolation";
pub const ABSENT_EGRESS_PROXY: &str = "egress-proxy";
pub const ABSENT_TOKEN_INJECTION: &str = "token-injection";
pub const ABSENT_HOME_ALLOWLIST: &str = "home-allowlist";
pub const ABSENT_RUN_TMPFS: &str = "run-tmpfs";
pub const ABSENT_PLANTED_FILE_RO: &str = "planted-file-ro";
pub const ABSENT_MANAGED_SETTINGS: &str = "managed-settings";
pub const ABSENT_REGISTRY_OVERLAY: &str = "registry-overlay";

/// Every protection, in the fixed vocabulary order [`protections_absent`] returns
/// its words in.
pub const PROTECTIONS: [&str; 8] = [
    ABSENT_NET_ISOLATION,
    ABSENT_EGRESS_PROXY,
    ABSENT_TOKEN_INJECTION,
    ABSENT_HOME_ALLOWLIST,
    ABSENT_RUN_TMPFS,
    ABSENT_PLANTED_FILE_RO,
    ABSENT_MANAGED_SETTINGS,
    ABSENT_REGISTRY_OVERLAY,
];

/// Whether this build can ever contain a launch in a sandbox.
///
/// `bwrap` and `systemd-run --user --scope` are Linux tools. Off Linux (macOS,
/// where a `bwrap` or `systemd-run` shim could sit on `PATH`), [`detect`] does not
/// probe at all and reports all three sandbox pieces missing, so
/// [`decide`]`(_, false)` always refuses and [`Posture::BwrapScope`] is unreachable
/// by construction: `decide` is the only producer of a [`Posture`]. A const rather
/// than a `cfg` block, so both arms are compiled (and the off-platform one tested)
/// on Linux, like `LIVENESS_SUPPORTED` in `driver::liveness`.
pub const SANDBOX_SUPPORTED: bool = cfg!(target_os = "linux");

/// The sandbox pieces [`detect`] reports missing on a host that cannot sandbox at
/// all, in probe order.
pub fn off_platform_missing() -> Vec<String> {
    vec![
        MISSING_BWRAP.to_owned(),
        MISSING_SYSTEMD_RUN.to_owned(),
        MISSING_USER_MANAGER.to_owned(),
    ]
}

/// What the probes found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCaps {
    /// The sandbox pieces that do not work, in probe order.
    pub missing: Vec<String>,
    /// The version `claude --version` answered, `None` when it could not be read.
    pub claude_version: Option<(u32, u32, u32)>,
    /// Whether `bwrap` can build the cargo-registry overlay (version at least
    /// [`BWRAP_OVERLAY_MIN`] and a working dry overlay probe). Probed only when
    /// `bwrap` works; `false` off-platform and on a failed `bwrap`.
    pub overlay: bool,
}

/// How a launch is contained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Posture {
    /// `systemd-run --user --scope` around `bwrap` (the default).
    BwrapScope,
    /// `--unsandboxed`: the driver runs as a supervised process group, with no
    /// `bwrap` and no scope. `missing` is what the probes found absent.
    Degraded { missing: Vec<String> },
}

impl Posture {
    /// The word recorded as `sandbox:` in state.json and the DIRECTION frontmatter
    /// (D-30) and as the launcher connection's mode.
    pub fn as_str(&self) -> &'static str {
        match self {
            Posture::BwrapScope => "bwrap-scope",
            Posture::Degraded { .. } => "degraded",
        }
    }

    /// What was missing, for a degraded launch; empty otherwise.
    pub fn missing(&self) -> &[String] {
        match self {
            Posture::BwrapScope => &[],
            Posture::Degraded { missing } => missing,
        }
    }
}

/// Whether `systemctl --user is-system-running` stdout says the user manager can
/// run a scope: the first trimmed line is `running` or `degraded`.
///
/// The exit code is deliberately not an input (RESEARCH §2).
pub fn parse_user_manager(stdout: &str) -> bool {
    matches!(
        stdout.lines().next().map(str::trim),
        Some("running" | "degraded")
    )
}

/// The first `N.N.N` in `claude --version` output (`2.1.303 (Claude Code)`).
pub fn parse_claude_version(stdout: &str) -> Option<(u32, u32, u32)> {
    first_dotted_triple(stdout)
}

/// The first `N.N.N` in `bwrap --version` output (`bubblewrap 0.11.1`).
pub fn parse_bwrap_version(stdout: &str) -> Option<(u32, u32, u32)> {
    first_dotted_triple(stdout)
}

fn first_dotted_triple(stdout: &str) -> Option<(u32, u32, u32)> {
    stdout.split_whitespace().find_map(|word| {
        let mut parts = word.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        parts.next().is_none().then_some((major, minor, patch))
    })
}

fn run_probe(program: &str, args: &[&str]) -> Option<(bool, String)> {
    let args: Vec<OsString> = args.iter().map(OsString::from).collect();
    run_bounded(program, &args, None, &[], PROBE_BUDGET).map(|out| {
        (
            out.status.success(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        )
    })
}

/// The bwrap argv of the "does it run here" probe: the namespaces the 28.1 profile
/// unshares, so a host that forbids them refuses the launch up front (Pitfall 12).
const BWRAP_PROBE_ARGS: [&str; 11] = [
    "--unshare-pid",
    "--unshare-net",
    "--unshare-ipc",
    "--unshare-uts",
    "--ro-bind",
    "/",
    "/",
    "--dev",
    "/dev",
    "--",
    "true",
];

/// The dry overlay probe: the same two options the profile uses for the cargo
/// registry, against a directory every host has.
const BWRAP_OVERLAY_PROBE_ARGS: [&str; 10] = [
    "--ro-bind",
    "/",
    "/",
    "--dev",
    "/dev",
    "--overlay-src",
    "/usr",
    "--tmp-overlay",
    "/usr",
    "--",
];

/// Whether this `bwrap` can build overlay mounts: version floor AND a dry probe.
fn probe_overlay() -> bool {
    let version_ok = run_probe("bwrap", &["--version"])
        .filter(|(ok, _)| *ok)
        .and_then(|(_, stdout)| parse_bwrap_version(&stdout))
        .is_some_and(|found| found >= BWRAP_OVERLAY_MIN);
    if !version_ok {
        return false;
    }
    let mut args: Vec<&str> = BWRAP_OVERLAY_PROBE_ARGS.to_vec();
    args.push("true");
    run_probe("bwrap", &args).is_some_and(|(ok, _)| ok)
}

fn probe_sandbox_pieces() -> (Vec<String>, bool) {
    let mut missing = Vec::new();
    let bwrap_runs = run_probe("bwrap", &BWRAP_PROBE_ARGS).is_some_and(|(ok, _)| ok);
    if !bwrap_runs {
        missing.push(MISSING_BWRAP.to_owned());
    }
    let overlay = bwrap_runs && probe_overlay();
    if !run_probe("systemd-run", &["--version"]).is_some_and(|(ok, _)| ok) {
        missing.push(MISSING_SYSTEMD_RUN.to_owned());
    }
    let manager_usable = run_probe("systemctl", &["--user", "is-system-running"])
        .is_some_and(|(_, stdout)| parse_user_manager(&stdout));
    if !manager_usable {
        missing.push(MISSING_USER_MANAGER.to_owned());
    }
    (missing, overlay)
}

/// Run the probes on this host. Writes nothing.
///
/// Off Linux ([`SANDBOX_SUPPORTED`] is false) the sandbox probes are skipped and
/// every sandbox piece is reported missing; only `claude --version` still runs.
pub fn detect() -> HostCaps {
    let (missing, overlay) = if SANDBOX_SUPPORTED {
        probe_sandbox_pieces()
    } else {
        (off_platform_missing(), false)
    };
    let claude_version = run_probe("claude", &["--version"])
        .filter(|(ok, _)| *ok)
        .and_then(|(_, stdout)| parse_claude_version(&stdout));
    HostCaps {
        missing,
        claude_version,
        overlay,
    }
}

/// The protections a launch with this `posture` lacks, in [`PROTECTIONS`] order, each
/// once (D-27, corrected by RESEARCH Q2). Derived from the posture and the MEASURED
/// `overlay` fact, never from an assumed default (LNCH-15):
///
/// * a degraded run lists all eight: no `bwrap` profile, no namespaces, no proxy
///   wiring, no tmpfs `$HOME`, no managed settings;
/// * a bwrap-scope run lists `registry-overlay` only, and only when `overlay` is
///   false. `managed-settings` is never listed for a bwrap run: the profile
///   re-binds `/etc/claude-code` over a tmpfs `/etc` on every bwrap version.
///
/// Kept apart from `sandbox_missing` (host tool pieces, the causes): the two lists
/// are never merged.
pub fn protections_absent(posture: &Posture, overlay: bool) -> Vec<&'static str> {
    match posture {
        Posture::Degraded { .. } => PROTECTIONS.to_vec(),
        Posture::BwrapScope => PROTECTIONS
            .iter()
            .copied()
            .filter(|word| *word == ABSENT_REGISTRY_OVERLAY && !overlay)
            .collect(),
    }
}

fn version_text((major, minor, patch): (u32, u32, u32)) -> String {
    format!("{major}.{minor}.{patch}")
}

/// Turn the probes and the `--unsandboxed` flag into how this launch is contained.
///
/// * a `claude` that is absent, unparseable or older than [`MIN_CLAUDE_VERSION`]
///   is refused in both modes `[inferred — audit]`;
/// * with the flag, the launch is degraded whatever the probes found (the flag is
///   the operator's explicit choice, D-29), recording what was missing;
/// * without it, anything missing refuses the launch, naming each piece and the
///   flag (D-28). There is no other path to a degraded run: no config default and
///   no automatic fallback.
pub fn decide(caps: &HostCaps, unsandboxed: bool) -> Result<Posture, LauncherError> {
    let minimum = version_text(MIN_CLAUDE_VERSION);
    match caps.claude_version {
        None => {
            return Err(LauncherError::ClaudeVersion {
                detail: format!(
                    "`claude --version` gave no readable version; Claude Code {minimum} or newer \
                     is required (the hook `onFailure` field)"
                ),
            })
        }
        Some(found) if found < MIN_CLAUDE_VERSION => {
            return Err(LauncherError::ClaudeVersion {
                detail: format!(
                    "Claude Code {} is older than {minimum}, the first version with the hook \
                     `onFailure` field the gate relies on",
                    version_text(found)
                ),
            })
        }
        Some(_) => {}
    }
    if unsandboxed {
        return Ok(Posture::Degraded {
            missing: caps.missing.clone(),
        });
    }
    if caps.missing.is_empty() {
        Ok(Posture::BwrapScope)
    } else {
        Err(LauncherError::SandboxUnavailable {
            missing: caps.missing.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_and_degraded_are_usable_user_managers() {
        assert!(parse_user_manager("running\n"));
        assert!(parse_user_manager("degraded\n"));
        assert!(parse_user_manager("  degraded  \nextra"));
    }

    #[test]
    fn anything_else_is_not_a_usable_user_manager() {
        for stdout in [
            "offline\n",
            "starting\n",
            "stopping\n",
            "maintenance\n",
            "",
            "\n",
            "Failed to connect to bus: No medium found\n",
            "not running\n",
        ] {
            assert!(!parse_user_manager(stdout), "{stdout:?}");
        }
    }

    #[test]
    fn claude_versions_parse_from_the_first_dotted_triple() {
        assert_eq!(
            parse_claude_version("2.1.303 (Claude Code)\n"),
            Some((2, 1, 303))
        );
        assert_eq!(parse_claude_version("2.1.294"), Some((2, 1, 294)));
        assert_eq!(parse_claude_version("claude 10.20.30"), Some((10, 20, 30)));
        for garbage in ["garbage", "", "2.1", "2.1.x", "1.2.3.4", "v2.1.303"] {
            assert_eq!(parse_claude_version(garbage), None, "{garbage:?}");
        }
    }

    #[test]
    fn the_minimum_is_the_on_failure_version() {
        assert_eq!(MIN_CLAUDE_VERSION, (2, 1, 295));
        assert!((2, 1, 294) < MIN_CLAUDE_VERSION);
        assert!((2, 1, 295) >= MIN_CLAUDE_VERSION);
        assert!((2, 2, 0) >= MIN_CLAUDE_VERSION);
        assert!((3, 0, 0) >= MIN_CLAUDE_VERSION);
    }

    fn caps(missing: &[&str], claude: Option<(u32, u32, u32)>) -> HostCaps {
        HostCaps {
            missing: missing.iter().map(|m| (*m).to_owned()).collect(),
            claude_version: claude,
            overlay: missing.is_empty(),
        }
    }

    #[test]
    fn a_complete_host_is_sandboxed_and_the_flag_still_degrades_it() {
        let host = caps(&[], Some((2, 1, 303)));
        assert_eq!(decide(&host, false), Ok(Posture::BwrapScope));
        assert_eq!(
            decide(&host, true),
            Ok(Posture::Degraded { missing: vec![] })
        );
    }

    #[test]
    fn a_missing_piece_refuses_without_the_flag_and_degrades_with_it() {
        let host = caps(&["bwrap", "user-systemd"], Some((2, 1, 303)));
        let refusal = decide(&host, false).unwrap_err();
        assert_eq!(
            refusal,
            LauncherError::SandboxUnavailable {
                missing: vec!["bwrap".to_owned(), "user-systemd".to_owned()]
            }
        );
        let text = refusal.to_string();
        assert!(text.contains("bwrap, user-systemd"), "{text}");
        assert!(text.contains("--unsandboxed"), "{text}");
        assert_eq!(refusal.exit_code(), 1);
        let degraded = decide(&host, true).unwrap();
        assert_eq!(degraded.as_str(), "degraded");
        assert_eq!(degraded.missing(), ["bwrap", "user-systemd"]);
    }

    #[test]
    fn an_old_or_unreadable_claude_is_refused_in_both_modes() {
        for claude in [Some((2, 1, 294)), Some((1, 9, 999)), None] {
            for unsandboxed in [false, true] {
                let error = decide(&caps(&[], claude), unsandboxed).unwrap_err();
                assert!(
                    matches!(error, LauncherError::ClaudeVersion { .. }),
                    "{claude:?} {unsandboxed}: {error}"
                );
                assert!(error.to_string().contains("2.1.295"), "{error}");
                assert_eq!(error.exit_code(), 1);
            }
        }
    }

    #[test]
    fn the_version_refusal_wins_over_the_sandbox_refusal() {
        let error = decide(&caps(&["bwrap"], Some((2, 1, 200))), false).unwrap_err();
        assert!(
            matches!(error, LauncherError::ClaudeVersion { .. }),
            "{error}"
        );
    }

    #[test]
    fn the_sandbox_is_supported_on_linux_only() {
        assert_eq!(SANDBOX_SUPPORTED, cfg!(target_os = "linux"));
    }

    #[test]
    fn off_platform_the_sandbox_is_refused_by_default_naming_bwrap_and_systemd_run() {
        let host = HostCaps {
            missing: off_platform_missing(),
            claude_version: Some(MIN_CLAUDE_VERSION),
            overlay: false,
        };
        let refusal = decide(&host, false).unwrap_err();
        let LauncherError::SandboxUnavailable { missing } = &refusal else {
            panic!("expected SandboxUnavailable, got {refusal}");
        };
        assert!(missing.iter().any(|m| m == "bwrap"), "{missing:?}");
        assert!(missing.iter().any(|m| m == "systemd-run"), "{missing:?}");
    }

    #[test]
    fn off_platform_the_flag_runs_the_degraded_posture() {
        let host = HostCaps {
            missing: off_platform_missing(),
            claude_version: Some(MIN_CLAUDE_VERSION),
            overlay: false,
        };
        let posture = decide(&host, true).unwrap();
        assert!(matches!(posture, Posture::Degraded { .. }), "{posture:?}");
        assert_eq!(posture.as_str(), "degraded");
    }

    #[test]
    fn the_off_platform_list_is_the_three_pieces_in_probe_order() {
        assert_eq!(
            off_platform_missing(),
            [MISSING_BWRAP, MISSING_SYSTEMD_RUN, MISSING_USER_MANAGER]
        );
    }

    #[test]
    fn bwrap_versions_parse_from_the_first_dotted_triple() {
        assert_eq!(parse_bwrap_version("bubblewrap 0.11.1\n"), Some((0, 11, 1)));
        assert_eq!(parse_bwrap_version("bubblewrap 0.9.0"), Some((0, 9, 0)));
        for garbage in ["", "bubblewrap", "bubblewrap 0.9", "bubblewrap x.y.z"] {
            assert_eq!(parse_bwrap_version(garbage), None, "{garbage:?}");
        }
        assert!((0, 9, 0) < BWRAP_OVERLAY_MIN);
        assert!((0, 10, 0) < BWRAP_OVERLAY_MIN);
        assert!((0, 11, 0) >= BWRAP_OVERLAY_MIN);
    }

    #[test]
    fn the_bwrap_probe_unshares_the_namespaces_the_profile_does() {
        for flag in [
            "--unshare-pid",
            "--unshare-net",
            "--unshare-ipc",
            "--unshare-uts",
        ] {
            assert!(BWRAP_PROBE_ARGS.contains(&flag), "{flag}");
        }
        assert_eq!(BWRAP_PROBE_ARGS.last(), Some(&"true"));
    }

    #[test]
    fn a_bwrap_scope_run_lacks_only_the_registry_overlay_and_only_without_overlay() {
        assert!(protections_absent(&Posture::BwrapScope, true).is_empty());
        assert_eq!(
            protections_absent(&Posture::BwrapScope, false),
            ["registry-overlay"]
        );
        assert!(!protections_absent(&Posture::BwrapScope, false).contains(&"managed-settings"));
    }

    #[test]
    fn a_degraded_run_lacks_all_eight_in_vocabulary_order() {
        let degraded = Posture::Degraded {
            missing: vec!["bwrap".to_owned()],
        };
        for overlay in [true, false] {
            assert_eq!(
                protections_absent(&degraded, overlay),
                [
                    "net-isolation",
                    "egress-proxy",
                    "token-injection",
                    "home-allowlist",
                    "run-tmpfs",
                    "planted-file-ro",
                    "managed-settings",
                    "registry-overlay"
                ]
            );
        }
    }

    #[test]
    fn every_posture_accounts_for_every_protection() {
        let postures = [
            Posture::BwrapScope,
            Posture::Degraded { missing: vec![] },
            Posture::Degraded {
                missing: vec!["bwrap".to_owned(), "systemd-run".to_owned()],
            },
        ];
        for posture in &postures {
            for overlay in [true, false] {
                let absent = protections_absent(posture, overlay);
                // Each absent word is a known protection, listed once, in order.
                let positions: Vec<usize> = absent
                    .iter()
                    .map(|w| PROTECTIONS.iter().position(|p| p == w).expect("known word"))
                    .collect();
                assert!(
                    positions.windows(2).all(|p| p[0] < p[1]),
                    "{posture:?}/{overlay}: {absent:?}"
                );
                // Every protection is either absent or provided by the posture.
                let provided: Vec<&str> = PROTECTIONS
                    .iter()
                    .copied()
                    .filter(|p| !absent.contains(p))
                    .collect();
                assert_eq!(provided.len() + absent.len(), PROTECTIONS.len());
                if matches!(posture, Posture::BwrapScope) {
                    // A contained run provides all but (maybe) the overlay.
                    assert!(provided.contains(&"net-isolation"));
                    assert!(provided.contains(&"managed-settings"));
                } else {
                    assert!(provided.is_empty(), "{provided:?}");
                }
            }
        }
    }

    #[test]
    fn postures_record_the_words_the_frontmatter_carries() {
        assert_eq!(Posture::BwrapScope.as_str(), "bwrap-scope");
        assert!(Posture::BwrapScope.missing().is_empty());
    }
}
