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

    // Step 2 (D-02): the target as a path. `join` keeps an absolute target as
    // it is; a path that does not exist is simply not found.
    let Ok(canonical_target) = cwd.join(target).canonicalize() else {
        return Err(not_found());
    };

    // The nearest registered root that is the target or a component-wise
    // ancestor of it. `Path::starts_with` compares whole components, never a
    // string prefix, so /x/foo can never claim /x/foobar. An exact match is
    // simply the deepest case.
    let best = config
        .projects
        .iter()
        .filter_map(|(alias, project)| {
            // A stale root (no longer on disk) falls back to its stored path
            // rather than aborting the lookup, so one dead entry can never
            // block launching any other project.
            let root = project
                .path
                .canonicalize()
                .unwrap_or_else(|_| project.path.clone());
            canonical_target
                .starts_with(&root)
                .then(|| (root.components().count(), alias))
        })
        // Deepest root first; [INFERRED] ties (two aliases registered at one
        // root) go to the lexicographically smallest alias, because `projects`
        // is a HashMap and its iteration order would otherwise pick a
        // different project from run to run.
        .min_by(|(depth_a, alias_a), (depth_b, alias_b)| {
            depth_b.cmp(depth_a).then_with(|| alias_a.cmp(alias_b))
        });

    match best {
        Some((_, alias)) => Ok(alias.clone()),
        None => Err(LaunchTargetError::Unregistered {
            target: target.to_string(),
            path: canonical_target,
        }),
    }
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

    // ── Path targets (D-02 step 2) ──────────────────────────────────────

    /// T/alpha ("alpha"), T/alpha/nested ("inner"), T/foo ("foo") registered;
    /// T/foobar, T/alpha/src/deep and T/alpha/nested/x unregistered.
    fn path_fixture() -> (tempfile::TempDir, PathBuf, Config) {
        let tmp = tempfile::TempDir::new().unwrap();
        let t = tmp.path().canonicalize().unwrap();
        for dir in ["alpha/nested/x", "alpha/src/deep", "foo", "foobar"] {
            std::fs::create_dir_all(t.join(dir)).unwrap();
        }
        let config = config_with(&[
            ("alpha", &t.join("alpha")),
            ("inner", &t.join("alpha/nested")),
            ("foo", &t.join("foo")),
        ]);
        (tmp, t, config)
    }

    #[test]
    fn dot_inside_a_registered_root_resolves_to_it() {
        let (_tmp, t, config) = path_fixture();
        assert_eq!(
            resolve_launch_target(&config, ".", &t.join("alpha")),
            Ok("alpha".to_string())
        );
    }

    #[test]
    fn an_absolute_root_path_resolves_like_pwd() {
        let (_tmp, t, config) = path_fixture();
        let abs = t.join("alpha").display().to_string();
        assert_eq!(
            resolve_launch_target(&config, &abs, Path::new("/")),
            Ok("alpha".to_string())
        );
    }

    #[test]
    fn a_relative_path_is_joined_onto_the_cwd() {
        let (_tmp, t, config) = path_fixture();
        assert_eq!(
            resolve_launch_target(&config, "alpha/src", &t),
            Ok("alpha".to_string())
        );
    }

    #[test]
    fn a_subdirectory_resolves_to_its_registered_ancestor() {
        let (_tmp, t, config) = path_fixture();
        assert_eq!(
            resolve_launch_target(&config, "src/deep", &t.join("alpha")),
            Ok("alpha".to_string())
        );
    }

    #[test]
    fn the_nearest_registered_ancestor_wins_over_an_outer_one() {
        let (_tmp, t, config) = path_fixture();
        assert_eq!(
            resolve_launch_target(&config, ".", &t.join("alpha/nested/x")),
            Ok("inner".to_string())
        );
    }

    #[test]
    fn ancestry_is_component_wise_so_foo_does_not_claim_foobar() {
        let (_tmp, t, config) = path_fixture();
        let err = resolve_launch_target(&config, "../foobar", &t.join("foo"))
            .expect_err("T/foo is not an ancestor of T/foobar");
        assert_eq!(
            err,
            LaunchTargetError::Unregistered {
                target: "../foobar".to_string(),
                path: t.join("foobar"),
            }
        );
        let hint = err.suggestion();
        assert!(hint.contains("gsd-meta-manager add"), "{hint}");
        assert!(
            hint.contains(&t.join("foobar").display().to_string()),
            "{hint}"
        );
    }

    #[test]
    fn a_nonexistent_path_is_not_found() {
        let (_tmp, t, config) = path_fixture();
        assert!(matches!(
            resolve_launch_target(&config, "does/not/exist", &t),
            Err(LaunchTargetError::NotFound { .. })
        ));
    }

    #[test]
    fn an_exact_alias_takes_precedence_over_a_same_named_path() {
        let (_tmp, t, mut config) = path_fixture();
        std::fs::create_dir_all(t.join("beta-dir")).unwrap();
        config
            .projects
            .insert("x".to_string(), project(&t.join("beta-dir")));
        config
            .projects
            .insert("beta-dir".to_string(), project(&t.join("foo")));
        assert_eq!(
            resolve_launch_target(&config, "beta-dir", &t),
            Ok("beta-dir".to_string()),
            "D-02 step 1: the alias wins before the path reading is tried"
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlinked_registrations_and_targets_resolve_through_canonicalization() {
        let (_tmp, t, _config) = path_fixture();
        std::os::unix::fs::symlink(t.join("alpha"), t.join("link")).unwrap();

        // Registered via the symlink, targeted via the real path.
        let via_link = config_with(&[("linked", &t.join("link"))]);
        assert_eq!(
            resolve_launch_target(&via_link, &t.join("alpha/src").display().to_string(), &t),
            Ok("linked".to_string())
        );

        // Registered via the real path, targeted via the symlink.
        let via_real = config_with(&[("real", &t.join("alpha"))]);
        assert_eq!(
            resolve_launch_target(&via_real, "link/src", &t),
            Ok("real".to_string())
        );
    }

    #[test]
    fn two_aliases_at_one_root_tie_break_to_the_smallest_alias() {
        let (_tmp, t, _config) = path_fixture();
        let config = config_with(&[
            ("zeta", &t.join("foo")),
            ("mid", &t.join("foo")),
            ("able", &t.join("foo")),
        ]);
        for _ in 0..8 {
            assert_eq!(
                resolve_launch_target(&config, ".", &t.join("foo")),
                Ok("able".to_string())
            );
        }
    }

    #[test]
    fn a_stale_registered_root_does_not_block_resolving_another() {
        let (_tmp, t, mut config) = path_fixture();
        config
            .projects
            .insert("stale".to_string(), project(&t.join("gone")));
        assert_eq!(
            resolve_launch_target(&config, ".", &t.join("foo")),
            Ok("foo".to_string())
        );
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
