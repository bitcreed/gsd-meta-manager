//! Which gsd-core a project would load, and how that version relates to the
//! gsd-core this build is synced against (quick task 260926-j0a).
//!
//! # Where an install lives
//!
//! gsd-core's installer writes `<targetDir>/gsd-core/VERSION` (the bare
//! `package.json` version, no trailing newline) for every runtime it installs
//! into (`bin/install.js` of `release-1.15.0`). Its own "is it installed?"
//! signal is that file's existence, so it is this module's signal too.
//!
//! [`install_candidates`] lists the `gsd-core` directories in the SAME order
//! gsd-core's runtime resolver probes them
//! (`gsd-core/references/gsd-run-resolver.md`):
//!
//! 1. `<root>/gsd-core` (a Cline-style local install, labelled `(root)`)
//! 2. `<root>/.claude/gsd-core`
//! 3. `<root>/.codex/gsd-core`
//! 4. `${CLAUDE_CONFIG_DIR:-~/.claude}/gsd-core`
//! 5. `${CODEX_HOME:-~/.codex}/gsd-core`
//!
//! The first candidate whose `VERSION` is a regular file is the EFFECTIVE
//! install, so a project-local install overrides a global one and Claude beats
//! Codex at the same scope. The gsd-tools resolver in
//! [`super::queue_md`] walks the same list, projected onto
//! `bin/gsd-tools.cjs`, so the two can never disagree about the order.
//!
//! # The comparison rule (inferred I-1, audit)
//!
//! The in-sync range runs from [`GSD_CORE_SYNCED_VERSION`] (the conformance
//! oracle pin, the newest npm release) to [`GSD_CORE_SYNCED_TREE_VERSION`]
//! (the synced tree's own `package.json` version), inclusive:
//!
//! * above the ceiling -> [`SyncRelation::Newer`] (a warning: formats this
//!   build does not model may be unread);
//! * inside the range -> [`SyncRelation::InRange`] (no note);
//! * below the floor -> [`SyncRelation::Older`] (an informational note).
//!
//! Semver precedence applies: a prerelease sorts below its release and build
//! metadata is ignored.
//!
//! # No process is spawned
//!
//! Detection is a handful of `stat`s and at most one bounded read per project.
//! Nothing here runs `node` or `gsd-tools`. [`InstallRoots::from_env`] is the
//! only process-environment read, so every test injects its roots.

use std::cmp::Ordering;
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use super::config_json::{GSD_CORE_SYNCED_TREE_VERSION, GSD_CORE_SYNCED_VERSION};

/// The most bytes of a `VERSION` file that are ever read. A real one is a
/// handful of bytes; anything longer is not a version and is reported as
/// [`InstalledVersion::Unrecognised`] after reading one byte past the cap.
pub const VERSION_READ_CAP: u64 = 256;

/// The home directory and the two runtime-home overrides detection resolves
/// against. Injected everywhere so no test depends on the real environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstallRoots {
    /// The user's home directory, when known.
    pub home: Option<PathBuf>,
    /// `CLAUDE_CONFIG_DIR`, as the process saw it.
    pub claude_config_dir: Option<OsString>,
    /// `CODEX_HOME`, as the process saw it.
    pub codex_home: Option<OsString>,
}

impl InstallRoots {
    /// Read the home directory, `CLAUDE_CONFIG_DIR` and `CODEX_HOME` from the
    /// process. The ONLY environment read of the feature.
    pub fn from_env() -> Self {
        Self {
            home: dirs::home_dir(),
            claude_config_dir: std::env::var_os("CLAUDE_CONFIG_DIR"),
            codex_home: std::env::var_os("CODEX_HOME"),
        }
    }
}

/// Which runtime's layout a candidate belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallRuntime {
    /// `<root>/gsd-core`: the project root itself is the install target.
    ProjectRoot,
    /// A `.claude` (or `CLAUDE_CONFIG_DIR`) install.
    Claude,
    /// A `.codex` (or `CODEX_HOME`) install.
    Codex,
}

/// Whether a candidate is inside the project or in the user's home.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallScope {
    /// Inside the project root.
    ProjectLocal,
    /// In a runtime home shared by every project.
    Global,
}

/// One place a gsd-core install may live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallCandidate {
    /// The `gsd-core` directory itself (holds `VERSION` and `bin/`).
    pub gsd_core_dir: PathBuf,
    /// Which runtime's layout it is.
    pub runtime: InstallRuntime,
    /// Project-local or global.
    pub scope: InstallScope,
}

/// Every `gsd-core` directory that may hold the effective install, in the
/// resolver's order (see the module doc). Pure.
///
/// Without a project root only the two global candidates are returned; without
/// a home and without overrides only the project-local ones.
pub fn install_candidates(
    _project_root: Option<&Path>,
    _roots: &InstallRoots,
) -> Vec<InstallCandidate> {
    Vec::new()
}

/// A validated semver version.
///
/// **`pre` is safe to display.** [`GsdVersion::parse`] only accepts
/// dot-separated identifiers of `[0-9A-Za-z-]`, so a prerelease can carry no
/// control character, escape or invisible code point. Build metadata is
/// validated the same way and then dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GsdVersion {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Option<String>,
}

impl GsdVersion {
    /// Parse `MAJOR.MINOR.PATCH[-pre][+build]`, trimmed, with an optional
    /// leading `v`. Anything else is `None`.
    pub fn parse(_raw: &str) -> Option<Self> {
        None
    }

    /// Semver precedence (build metadata already dropped).
    pub fn precedence_cmp(&self, _other: &Self) -> Ordering {
        Ordering::Equal
    }
}

impl fmt::Display for GsdVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if let Some(pre) = &self.pre {
            write!(f, "-{pre}")?;
        }
        Ok(())
    }
}

/// How an installed version relates to the synced range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncRelation {
    /// Above the synced tree's version.
    Newer,
    /// Between the oracle pin and the synced tree's version, inclusive.
    InRange,
    /// Below the oracle pin.
    Older,
}

/// `v` against an explicit `[floor, ceiling]` range.
pub fn relation_between(_v: &GsdVersion, _floor: &GsdVersion, _ceiling: &GsdVersion) -> SyncRelation {
    SyncRelation::InRange
}

/// `v` against [`GSD_CORE_SYNCED_VERSION`] (floor) and
/// [`GSD_CORE_SYNCED_TREE_VERSION`] (ceiling).
pub fn relation_to_synced(_v: &GsdVersion) -> SyncRelation {
    SyncRelation::InRange
}

/// What a `VERSION` file said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstalledVersion {
    /// A valid semver version.
    Parsed(GsdVersion),
    /// Present but not a version (too long, not UTF-8, or unparseable). Its
    /// content is never displayed.
    Unrecognised,
}

/// Read `path` as a gsd-core `VERSION` file. `None` unless it is a regular
/// file.
fn read_version_file(_path: &Path) -> Option<InstalledVersion> {
    None
}

/// The effective install [`detect_gsd_install`] found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedInstall {
    pub runtime: InstallRuntime,
    pub scope: InstallScope,
    /// Where it lives. Logged, never drawn.
    pub gsd_core_dir: PathBuf,
    pub version: InstalledVersion,
}

/// The effective gsd-core install for a project (or globally).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum GsdInstallStatus {
    /// No candidate holds a `VERSION` file.
    #[default]
    NotFound,
    /// The first candidate holding a `VERSION` file.
    Found(DetectedInstall),
}

/// Walk [`install_candidates`] and return the first one whose `VERSION` is a
/// regular file. File reads only.
pub fn detect_gsd_install(_project_root: Option<&Path>, _roots: &InstallRoots) -> GsdInstallStatus {
    GsdInstallStatus::NotFound
}

/// How loudly a label should be drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Ok,
    Info,
    Warning,
}

/// The one-line label for `status`, and its severity.
pub fn install_label(_status: &GsdInstallStatus) -> (String, Severity) {
    (String::new(), Severity::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn floor() -> GsdVersion {
        GsdVersion::parse(GSD_CORE_SYNCED_VERSION).expect("floor parses")
    }

    fn ceiling() -> GsdVersion {
        GsdVersion::parse(GSD_CORE_SYNCED_TREE_VERSION).expect("ceiling parses")
    }

    /// A version above the ceiling, derived from the constant so a future
    /// sync cannot silently flip the fixture.
    fn newer_str() -> String {
        format!("{}.0.0", ceiling().major + 1)
    }

    const OLDER: &str = "0.1.0";

    fn v(s: &str) -> GsdVersion {
        GsdVersion::parse(s).unwrap_or_else(|| panic!("{s:?} should parse"))
    }

    fn roots(home: &Path) -> InstallRoots {
        InstallRoots {
            home: Some(home.to_path_buf()),
            ..Default::default()
        }
    }

    fn write_version(base: &Path, rel: &str, content: &[u8]) {
        let dir = base.join(rel);
        fs::create_dir_all(&dir).expect("mkdir");
        fs::write(dir.join("VERSION"), content).expect("write VERSION");
    }

    fn found(status: &GsdInstallStatus) -> &DetectedInstall {
        match status {
            GsdInstallStatus::Found(d) => d,
            GsdInstallStatus::NotFound => panic!("expected an install, got NotFound"),
        }
    }

    fn layout(c: &[InstallCandidate]) -> Vec<(PathBuf, InstallRuntime, InstallScope)> {
        c.iter()
            .map(|c| (c.gsd_core_dir.clone(), c.runtime, c.scope))
            .collect()
    }

    // ── candidates ─────────────────────────────────────────────────────

    #[test]
    fn install_candidates_follow_the_resolver_order() {
        let root = Path::new("/p");
        let home = Path::new("/h");
        use InstallRuntime::*;
        use InstallScope::*;
        assert_eq!(
            layout(&install_candidates(Some(root), &roots(home))),
            vec![
                (PathBuf::from("/p/gsd-core"), ProjectRoot, ProjectLocal),
                (PathBuf::from("/p/.claude/gsd-core"), Claude, ProjectLocal),
                (PathBuf::from("/p/.codex/gsd-core"), Codex, ProjectLocal),
                (PathBuf::from("/h/.claude/gsd-core"), Claude, Global),
                (PathBuf::from("/h/.codex/gsd-core"), Codex, Global),
            ]
        );
    }

    #[test]
    fn install_candidates_without_a_project_are_only_global() {
        let got = install_candidates(None, &roots(Path::new("/h")));
        assert_eq!(
            layout(&got),
            vec![
                (PathBuf::from("/h/.claude/gsd-core"), InstallRuntime::Claude, InstallScope::Global),
                (PathBuf::from("/h/.codex/gsd-core"), InstallRuntime::Codex, InstallScope::Global),
            ]
        );
    }

    #[test]
    fn install_candidates_without_a_home_are_only_project_local() {
        let got = install_candidates(Some(Path::new("/p")), &InstallRoots::default());
        assert_eq!(got.len(), 3);
        assert!(got.iter().all(|c| c.scope == InstallScope::ProjectLocal));
    }

    #[test]
    fn a_non_blank_override_replaces_the_home_default() {
        let r = InstallRoots {
            home: Some(PathBuf::from("/h")),
            claude_config_dir: Some(OsString::from("/cc")),
            codex_home: Some(OsString::from("/cx")),
        };
        let got = install_candidates(None, &r);
        assert_eq!(
            got.iter().map(|c| c.gsd_core_dir.clone()).collect::<Vec<_>>(),
            vec![PathBuf::from("/cc/gsd-core"), PathBuf::from("/cx/gsd-core")],
            "an override REPLACES the home default, it does not add a candidate"
        );
        // Without a home, an override still yields its global candidate.
        let no_home = InstallRoots {
            home: None,
            ..r
        };
        assert_eq!(install_candidates(None, &no_home).len(), 2);
    }

    #[test]
    fn a_blank_override_falls_back_to_the_home_default() {
        for blank in ["", "   ", "\t"] {
            let r = InstallRoots {
                home: Some(PathBuf::from("/h")),
                claude_config_dir: Some(OsString::from(blank)),
                codex_home: Some(OsString::from(blank)),
            };
            assert_eq!(
                install_candidates(None, &r)
                    .iter()
                    .map(|c| c.gsd_core_dir.clone())
                    .collect::<Vec<_>>(),
                vec![
                    PathBuf::from("/h/.claude/gsd-core"),
                    PathBuf::from("/h/.codex/gsd-core"),
                ],
                "override {blank:?} is blank and must fall back"
            );
        }
    }

    #[test]
    fn a_tilde_override_expands_against_home() {
        let r = InstallRoots {
            home: Some(PathBuf::from("/h")),
            claude_config_dir: Some(OsString::from("~/x")),
            codex_home: Some(OsString::from("~")),
        };
        assert_eq!(
            install_candidates(None, &r)
                .iter()
                .map(|c| c.gsd_core_dir.clone())
                .collect::<Vec<_>>(),
            vec![PathBuf::from("/h/x/gsd-core"), PathBuf::from("/h/gsd-core")]
        );
    }

    // ── detection ──────────────────────────────────────────────────────

    #[test]
    fn a_project_local_install_overrides_a_global_one() {
        let root = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        write_version(root.path(), ".claude/gsd-core", OLDER.as_bytes());
        write_version(home.path(), ".claude/gsd-core", newer_str().as_bytes());

        let status = detect_gsd_install(Some(root.path()), &roots(home.path()));
        let d = found(&status);
        assert_eq!(d.scope, InstallScope::ProjectLocal);
        assert_eq!(d.runtime, InstallRuntime::Claude);
        assert_eq!(d.gsd_core_dir, root.path().join(".claude/gsd-core"));
        assert_eq!(d.version, InstalledVersion::Parsed(v(OLDER)));
    }

    #[test]
    fn a_codex_only_home_is_a_global_codex_install() {
        let root = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        write_version(home.path(), ".codex/gsd-core", GSD_CORE_SYNCED_VERSION.as_bytes());
        let status = detect_gsd_install(Some(root.path()), &roots(home.path()));
        let d = found(&status);
        assert_eq!((d.runtime, d.scope), (InstallRuntime::Codex, InstallScope::Global));
    }

    #[test]
    fn claude_beats_codex_at_global_scope() {
        let home = TempDir::new().unwrap();
        write_version(home.path(), ".claude/gsd-core", GSD_CORE_SYNCED_VERSION.as_bytes());
        write_version(home.path(), ".codex/gsd-core", GSD_CORE_SYNCED_TREE_VERSION.as_bytes());
        let status = detect_gsd_install(None, &roots(home.path()));
        let d = found(&status);
        assert_eq!((d.runtime, d.scope), (InstallRuntime::Claude, InstallScope::Global));
        assert_eq!(d.version, InstalledVersion::Parsed(floor()));
    }

    #[test]
    fn a_project_local_codex_beats_a_global_claude() {
        let root = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        write_version(root.path(), ".codex/gsd-core", GSD_CORE_SYNCED_TREE_VERSION.as_bytes());
        write_version(home.path(), ".claude/gsd-core", GSD_CORE_SYNCED_VERSION.as_bytes());
        let status = detect_gsd_install(Some(root.path()), &roots(home.path()));
        let d = found(&status);
        assert_eq!((d.runtime, d.scope), (InstallRuntime::Codex, InstallScope::ProjectLocal));
    }

    #[test]
    fn no_version_anywhere_is_not_found() {
        let root = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        // A gsd-core checkout carries `gsd-core/` but no VERSION.
        fs::create_dir_all(root.path().join("gsd-core/bin")).unwrap();
        assert_eq!(
            detect_gsd_install(Some(root.path()), &roots(home.path())),
            GsdInstallStatus::NotFound
        );
    }

    #[test]
    fn a_version_directory_is_skipped() {
        let root = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        fs::create_dir_all(root.path().join(".claude/gsd-core/VERSION")).unwrap();
        write_version(home.path(), ".codex/gsd-core", GSD_CORE_SYNCED_VERSION.as_bytes());
        let status = detect_gsd_install(Some(root.path()), &roots(home.path()));
        let d = found(&status);
        assert_eq!((d.runtime, d.scope), (InstallRuntime::Codex, InstallScope::Global));
    }

    #[test]
    fn a_garbage_local_version_is_unrecognised_and_does_not_fall_through() {
        let root = TempDir::new().unwrap();
        let home = TempDir::new().unwrap();
        write_version(root.path(), ".claude/gsd-core", b"banana");
        write_version(home.path(), ".claude/gsd-core", GSD_CORE_SYNCED_VERSION.as_bytes());
        let status = detect_gsd_install(Some(root.path()), &roots(home.path()));
        let d = found(&status);
        assert_eq!(d.scope, InstallScope::ProjectLocal);
        assert_eq!(d.version, InstalledVersion::Unrecognised);
    }

    #[test]
    fn an_oversized_or_non_utf8_version_is_unrecognised() {
        let home = TempDir::new().unwrap();
        let mut big = GSD_CORE_SYNCED_VERSION.as_bytes().to_vec();
        big.resize(VERSION_READ_CAP as usize + 1, b' ');
        write_version(home.path(), ".claude/gsd-core", &big);
        let status = detect_gsd_install(None, &roots(home.path()));
        assert_eq!(found(&status).version, InstalledVersion::Unrecognised);

        // Exactly at the cap still parses (trailing whitespace is trimmed).
        big.truncate(VERSION_READ_CAP as usize);
        write_version(home.path(), ".claude/gsd-core", &big);
        let status = detect_gsd_install(None, &roots(home.path()));
        assert_eq!(found(&status).version, InstalledVersion::Parsed(floor()));

        write_version(home.path(), ".claude/gsd-core", b"1.14.0\xff");
        let status = detect_gsd_install(None, &roots(home.path()));
        assert_eq!(found(&status).version, InstalledVersion::Unrecognised);
    }

    // ── parsing and precedence ─────────────────────────────────────────

    #[test]
    fn version_parse_accepts_semver_forms() {
        assert_eq!(v("1.14.0"), GsdVersion { major: 1, minor: 14, patch: 0, pre: None });
        assert_eq!(v("1.14.0\n"), v("1.14.0"));
        assert_eq!(v(" v1.15.0 "), v("1.15.0"));
        assert_eq!(v("1.16.0-rc.1").pre.as_deref(), Some("rc.1"));
        assert_eq!(v("1.15.0+build.5"), v("1.15.0"), "build metadata is dropped");
        assert_eq!(v("1.16.0-rc.1").to_string(), "1.16.0-rc.1");
    }

    #[test]
    fn version_parse_rejects_everything_else() {
        for bad in [
            "",
            "banana",
            "1.2",
            "1.2.3.4",
            "1.2.x",
            "1.2.3-",
            "1.2.3-rc..1",
            "1.2.3+",
            "1.2.3-r\u{1b}c",
            "\u{1b}[31m1.2.3",
            "1.2.3 4",
        ] {
            assert_eq!(GsdVersion::parse(bad), None, "{bad:?} must not parse");
        }
    }

    #[test]
    fn relation_between_the_floor_and_the_ceiling() {
        let (f, c) = (v("1.14.0"), v("1.15.0"));
        for s in ["1.16.0", "1.15.1", "2.0.0"] {
            assert_eq!(relation_between(&v(s), &f, &c), SyncRelation::Newer, "{s}");
        }
        for s in ["1.15.0", "1.14.0", "1.14.5", "1.15.0-rc.1"] {
            assert_eq!(relation_between(&v(s), &f, &c), SyncRelation::InRange, "{s}");
        }
        for s in ["1.13.9", "1.14.0-rc.1"] {
            assert_eq!(relation_between(&v(s), &f, &c), SyncRelation::Older, "{s}");
        }
    }

    #[test]
    fn prerelease_identifiers_follow_semver_precedence() {
        use Ordering::*;
        let cmp = |a: &str, b: &str| v(a).precedence_cmp(&v(b));
        assert_eq!(cmp("1.0.0-rc.2", "1.0.0-rc.10"), Less, "numeric ids compare by value");
        assert_eq!(cmp("1.0.0-1", "1.0.0-alpha"), Less, "numeric < alphanumeric");
        assert_eq!(cmp("1.0.0-alpha", "1.0.0-beta"), Less);
        assert_eq!(cmp("1.0.0-alpha", "1.0.0-alpha.1"), Less, "a shorter prefix sorts lower");
        assert_eq!(cmp("1.0.0-rc.1", "1.0.0"), Less, "a release beats its prereleases");
        assert_eq!(cmp("1.0.0", "1.0.0"), Equal);
        assert_eq!(cmp("1.0.0+a", "1.0.0+b"), Equal, "build metadata is ignored");
        assert_eq!(cmp("2.0.0", "1.99.99"), Greater);
    }

    #[test]
    fn relation_to_synced_uses_the_constants() {
        assert_eq!(relation_to_synced(&ceiling()), SyncRelation::InRange);
        assert_eq!(relation_to_synced(&floor()), SyncRelation::InRange);
        assert_eq!(relation_to_synced(&v(&newer_str())), SyncRelation::Newer);
        assert_eq!(relation_to_synced(&v(OLDER)), SyncRelation::Older);
    }

    // ── labels ─────────────────────────────────────────────────────────

    fn install(runtime: InstallRuntime, scope: InstallScope, version: InstalledVersion) -> GsdInstallStatus {
        GsdInstallStatus::Found(DetectedInstall {
            runtime,
            scope,
            gsd_core_dir: PathBuf::from("/secret/path/gsd-core"),
            version,
        })
    }

    #[test]
    fn install_label_names_version_source_and_relation() {
        use InstallRuntime::*;
        use InstallScope::*;
        let newer = newer_str();
        let cases = [
            (
                install(Claude, Global, InstalledVersion::Parsed(ceiling())),
                format!("GSD {GSD_CORE_SYNCED_TREE_VERSION} · global Claude"),
                Severity::Ok,
            ),
            (
                install(Codex, Global, InstalledVersion::Parsed(v(&newer))),
                format!("GSD {newer} · global Codex · newer than synced {GSD_CORE_SYNCED_TREE_VERSION}"),
                Severity::Warning,
            ),
            (
                install(Claude, ProjectLocal, InstalledVersion::Parsed(v(OLDER))),
                format!("GSD {OLDER} · project-local Claude · older than {GSD_CORE_SYNCED_VERSION}"),
                Severity::Info,
            ),
            (
                install(Codex, ProjectLocal, InstalledVersion::Unrecognised),
                "GSD (unrecognised VERSION) · project-local Codex".to_string(),
                Severity::Info,
            ),
            (
                install(ProjectRoot, ProjectLocal, InstalledVersion::Parsed(floor())),
                format!("GSD {GSD_CORE_SYNCED_VERSION} · project-local (root)"),
                Severity::Ok,
            ),
            (
                GsdInstallStatus::NotFound,
                format!("GSD not found · app synced to {GSD_CORE_SYNCED_TREE_VERSION}"),
                Severity::Info,
            ),
        ];
        for (status, text, severity) in cases {
            let (got, sev) = install_label(&status);
            assert_eq!(got, text);
            assert_eq!(sev, severity, "{text}");
            assert!(!got.contains("/secret"), "a label never carries the path: {got}");
        }
    }

    /// T-j0a-01: an ESC-prefixed VERSION is unrecognised, and nothing of it
    /// reaches a label.
    #[test]
    fn an_escape_in_a_version_file_never_reaches_the_label() {
        let root = TempDir::new().unwrap();
        write_version(root.path(), ".claude/gsd-core", b"\x1b[31m1.2.3");
        let status = detect_gsd_install(Some(root.path()), &InstallRoots::default());
        assert_eq!(found(&status).version, InstalledVersion::Unrecognised);
        let (label, _) = install_label(&status);
        assert!(!label.contains('\u{1b}'), "{label:?}");
        assert!(!label.contains("1.2.3"), "garbage content is never displayed: {label:?}");
        assert!(label.contains("unrecognised VERSION"), "{label:?}");
    }
}
