//! What the host can do for a launch (Phase 28 plan 28-04; D-28, D-29, D-31).
//!
//! The launcher refuses to run a driver unsandboxed unless that launch carries
//! `--unsandboxed`. This module is the half of that rule that looks at the host:
//! [`detect`] runs the probes, [`decide`] turns the answer and the flag into a
//! [`Posture`] or a typed refusal. A `which` is not a probe (RESEARCH §1): a
//! copied or AppArmor-blocked `bwrap` is on `PATH` and cannot create a user
//! namespace, so `bwrap` is only counted when it actually runs a trivial command.
//!
//! * `bwrap --ro-bind / / --dev /dev -- true` must exit 0;
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

/// How long each probe may take.
const PROBE_BUDGET: Duration = Duration::from_secs(5);

/// The names [`HostCaps::missing`] uses.
pub const MISSING_BWRAP: &str = "bwrap";
pub const MISSING_SYSTEMD_RUN: &str = "systemd-run";
pub const MISSING_USER_MANAGER: &str = "user-systemd";

/// What the probes found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostCaps {
    /// The sandbox pieces that do not work, in probe order.
    pub missing: Vec<String>,
    /// The version `claude --version` answered, `None` when it could not be read.
    pub claude_version: Option<(u32, u32, u32)>,
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

/// Run the probes on this host. Writes nothing.
pub fn detect() -> HostCaps {
    let mut missing = Vec::new();
    let bwrap_runs = run_probe(
        "bwrap",
        &["--ro-bind", "/", "/", "--dev", "/dev", "--", "true"],
    )
    .is_some_and(|(ok, _)| ok);
    if !bwrap_runs {
        missing.push(MISSING_BWRAP.to_owned());
    }
    if !run_probe("systemd-run", &["--version"]).is_some_and(|(ok, _)| ok) {
        missing.push(MISSING_SYSTEMD_RUN.to_owned());
    }
    let manager_usable = run_probe("systemctl", &["--user", "is-system-running"])
        .is_some_and(|(_, stdout)| parse_user_manager(&stdout));
    if !manager_usable {
        missing.push(MISSING_USER_MANAGER.to_owned());
    }
    let claude_version = run_probe("claude", &["--version"])
        .filter(|(ok, _)| *ok)
        .and_then(|(_, stdout)| parse_claude_version(&stdout));
    HostCaps {
        missing,
        claude_version,
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
    fn postures_record_the_words_the_frontmatter_carries() {
        assert_eq!(Posture::BwrapScope.as_str(), "bwrap-scope");
        assert!(Posture::BwrapScope.missing().is_empty());
    }
}
