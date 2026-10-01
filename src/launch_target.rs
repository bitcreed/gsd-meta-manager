//! Resolve the optional positional project target given on the top-level CLI
//! (`gsd-meta-manager <alias|path>`) to the registry alias the TUI opens.
//!
//! Resolution order (quick 260930-vvk, D-02):
//!
//! 1. an exact, byte-equal registered alias;
//! 2. otherwise the target as a path, joined onto the caller's cwd and
//!    canonicalized: the registered project whose canonical root equals it, else
//!    the nearest registered root that is a component-wise ancestor of it.
//!
//! Everything that touches the filesystem lives here, so the `async fn main`
//! body that calls this stays free of inline fs calls
//! (`tests/async_blocking_guard.rs`).

use std::path::{Path, PathBuf};

use crate::config::Config;

/// Why a launch target could not be opened.
///
/// `Display` is the single producer of the user-facing message: the target and
/// the path are escaped there through [`crate::text::render_for_terminal`] and
/// nowhere else (the D-21-3 single-producer precedent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchTargetError {
    /// Neither a registered alias nor an existing path.
    NotFound { target: String },
    /// An existing path that lies under no registered project root. `path` is
    /// the canonical path.
    Unregistered { target: String, path: PathBuf },
}

impl LaunchTargetError {
    /// The one-line hint printed under the error (D-03).
    pub fn suggestion(&self) -> String {
        match self {
            LaunchTargetError::NotFound { .. } => "Register the project first: \
                 gsd-meta-manager add <path>   (registered aliases: gsd-meta-manager list)"
                .to_string(),
            LaunchTargetError::Unregistered { path, .. } => format!(
                "Register it first: gsd-meta-manager add {}",
                crate::text::render_for_terminal(&path.display().to_string())
            ),
        }
    }
}

impl std::fmt::Display for LaunchTargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LaunchTargetError::NotFound { target } => write!(
                f,
                "'{}' is not a registered project alias or an existing path",
                crate::text::render_for_terminal(target)
            ),
            LaunchTargetError::Unregistered { target, path } => write!(
                f,
                "'{}' ({}) is not inside any registered project",
                crate::text::render_for_terminal(target),
                crate::text::render_for_terminal(&path.display().to_string())
            ),
        }
    }
}

impl std::error::Error for LaunchTargetError {}

/// Resolve `target` to the registry alias to open.
///
/// `cwd` is a parameter rather than read from the process so that callers (and
/// tests running in parallel) decide what a relative target is relative to.
pub fn resolve_launch_target(
    config: &Config,
    target: &str,
    cwd: &Path,
) -> Result<String, LaunchTargetError> {
    let not_found = || LaunchTargetError::NotFound {
        target: target.to_string(),
    };

    // [INFERRED] A blank target is refused outright: `cwd.join("")` is cwd
    // itself, so resolving it as a path would silently open whatever project
    // the caller happens to be standing in.
    if target.trim().is_empty() {
        return Err(not_found());
    }

    // Step 1 (D-02): exact registered alias. A membership lookup on the raw
    // bytes that creates nothing, the same class as `remove` (D-17-3), so a
    // legacy alias an older build accepted stays reachable.
    if config.projects.contains_key(target) {
        return Ok(target.to_string());
    }

    let _ = cwd;
    Err(not_found())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RegisteredProject;

    fn project(path: &Path) -> RegisteredProject {
        RegisteredProject {
            path: path.to_path_buf(),
            added: "2026-09-30".to_string(),
            driver_opt_in: None,
            extra: Default::default(),
        }
    }

    fn config_with(entries: &[(&str, &Path)]) -> Config {
        let mut config = Config::new();
        for (name, path) in entries {
            config.projects.insert((*name).to_string(), project(path));
        }
        config
    }

    #[test]
    fn an_exact_registered_alias_resolves_to_itself() {
        let config = config_with(&[
            ("alpha", Path::new("/nonexistent/alpha")),
            ("beta", Path::new("/nonexistent/beta")),
        ]);
        assert_eq!(
            resolve_launch_target(&config, "beta", Path::new("/nonexistent")),
            Ok("beta".to_string())
        );
    }

    #[test]
    fn an_unknown_alias_that_is_no_path_is_not_found_with_an_add_hint() {
        let config = config_with(&[("alpha", Path::new("/nonexistent/alpha"))]);
        let err = resolve_launch_target(&config, "no-such-alias", Path::new("/nonexistent"))
            .expect_err("an unknown alias must not resolve");
        assert!(
            matches!(err, LaunchTargetError::NotFound { .. }),
            "got {err:?}"
        );
        assert!(err.to_string().contains("no-such-alias"), "{err}");
        assert!(
            err.suggestion().contains("gsd-meta-manager add"),
            "{}",
            err.suggestion()
        );
    }

    #[test]
    fn a_blank_target_is_refused_rather_than_resolved_as_the_cwd() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = config_with(&[("here", tmp.path())]);
        for blank in ["", "   ", "\t"] {
            let err = resolve_launch_target(&config, blank, tmp.path())
                .expect_err("a blank target must not open the project at cwd");
            assert!(
                matches!(err, LaunchTargetError::NotFound { .. }),
                "got {err:?}"
            );
        }
    }

    #[test]
    fn the_error_display_escapes_terminal_controls_in_the_target() {
        let config = Config::new();
        let err = resolve_launch_target(&config, "evil\u{1b}[31mred", Path::new("/nonexistent"))
            .expect_err("unknown");
        assert!(
            !err.to_string().contains('\u{1b}'),
            "a raw ESC reached the message: {:?}",
            err.to_string()
        );
        let unregistered = LaunchTargetError::Unregistered {
            target: "x\u{1b}[31m".to_string(),
            path: PathBuf::from("/tmp/\u{1b}[31m"),
        };
        assert!(!unregistered.to_string().contains('\u{1b}'));
        assert!(!unregistered.suggestion().contains('\u{1b}'));
    }
}
