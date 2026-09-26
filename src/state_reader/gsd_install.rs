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

    /// The global Claude home: `${CLAUDE_CONFIG_DIR:-~/.claude}`.
    pub fn claude_home(&self) -> Option<PathBuf> {
        self.config_home(self.claude_config_dir.as_ref(), ".claude")
    }

    /// The global Codex home: `${CODEX_HOME:-~/.codex}`.
    pub fn codex_home_dir(&self) -> Option<PathBuf> {
        self.config_home(self.codex_home.as_ref(), ".codex")
    }

    /// Upstream `runtime-homes.cts` `dot-home`: a NON-BLANK override wins
    /// (`hasNonBlankOverride`, whitespace-only counts as unset) and has a
    /// leading `~` expanded against home (`expandTilde`); otherwise
    /// `<home>/<name>`. Inferred I-5.
    fn config_home(&self, override_: Option<&OsString>, name: &str) -> Option<PathBuf> {
        match override_.filter(|v| !v.to_string_lossy().trim().is_empty()) {
            Some(value) => Some(expand_tilde(value, self.home.as_deref())),
            None => self.home.as_ref().map(|home| home.join(name)),
        }
    }
}

/// `~` and `~/rest` expand against `home`; anything else (including `~user`)
/// is taken literally, as upstream's `expandTilde` does. Without a home the
/// value stays literal and simply will not exist.
fn expand_tilde(value: &OsString, home: Option<&Path>) -> PathBuf {
    if let (Some(home), Some(s)) = (home, value.to_str()) {
        if s == "~" {
            return home.to_path_buf();
        }
        if let Some(rest) = s.strip_prefix("~/") {
            return home.join(rest);
        }
    }
    PathBuf::from(value)
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
    project_root: Option<&Path>,
    roots: &InstallRoots,
) -> Vec<InstallCandidate> {
    fn candidate(dir: PathBuf, runtime: InstallRuntime, scope: InstallScope) -> InstallCandidate {
        InstallCandidate {
            gsd_core_dir: dir.join("gsd-core"),
            runtime,
            scope,
        }
    }
    let mut out = Vec::with_capacity(5);
    if let Some(root) = project_root {
        use InstallRuntime::*;
        out.push(candidate(root.to_path_buf(), ProjectRoot, InstallScope::ProjectLocal));
        out.push(candidate(root.join(".claude"), Claude, InstallScope::ProjectLocal));
        out.push(candidate(root.join(".codex"), Codex, InstallScope::ProjectLocal));
    }
    if let Some(home) = roots.claude_home() {
        out.push(candidate(home, InstallRuntime::Claude, InstallScope::Global));
    }
    if let Some(home) = roots.codex_home_dir() {
        out.push(candidate(home, InstallRuntime::Codex, InstallScope::Global));
    }
    out
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
    pub fn parse(raw: &str) -> Option<Self> {
        let s = raw.trim();
        let s = s.strip_prefix('v').unwrap_or(s);
        let s = match s.split_once('+') {
            Some((rest, build)) => {
                if !valid_identifiers(build) {
                    return None;
                }
                rest
            }
            None => s,
        };
        let (core, pre) = match s.split_once('-') {
            Some((core, pre)) => {
                if !valid_identifiers(pre) {
                    return None;
                }
                (core, Some(pre.to_string()))
            }
            None => (s, None),
        };
        let mut parts = core.split('.');
        let mut number = || -> Option<u64> {
            let part = parts.next()?;
            if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            part.parse().ok()
        };
        let (major, minor, patch) = (number()?, number()?, number()?);
        if parts.next().is_some() {
            return None;
        }
        Some(Self {
            major,
            minor,
            patch,
            pre,
        })
    }

    /// Semver precedence (build metadata already dropped): the numeric triple
    /// first, then a release above any of its prereleases, then prerelease
    /// identifiers left to right — numeric by value, numeric below
    /// alphanumeric, alphanumeric by ASCII, and a shorter prefix lower.
    pub fn precedence_cmp(&self, other: &Self) -> Ordering {
        (self.major, self.minor, self.patch)
            .cmp(&(other.major, other.minor, other.patch))
            .then_with(|| match (&self.pre, &other.pre) {
                (None, None) => Ordering::Equal,
                (None, Some(_)) => Ordering::Greater,
                (Some(_), None) => Ordering::Less,
                (Some(a), Some(b)) => compare_prerelease(a, b),
            })
    }
}

/// Non-empty dot-separated identifiers of `[0-9A-Za-z-]` only.
fn valid_identifiers(s: &str) -> bool {
    s.split('.').all(|id| {
        !id.is_empty() && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    })
}

fn compare_prerelease(a: &str, b: &str) -> Ordering {
    fn numeric(id: &str) -> bool {
        id.bytes().all(|b| b.is_ascii_digit())
    }
    let (mut left, mut right) = (a.split('.'), b.split('.'));
    loop {
        match (left.next(), right.next()) {
            (None, None) => return Ordering::Equal,
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(x), Some(y)) => {
                let ord = match (numeric(x), numeric(y)) {
                    (true, true) => {
                        // By value without overflow: fewer significant digits
                        // is smaller, then digit-wise.
                        let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
                        x.len().cmp(&y.len()).then_with(|| x.cmp(y))
                    }
                    (true, false) => Ordering::Less,
                    (false, true) => Ordering::Greater,
                    (false, false) => x.cmp(y),
                };
                if ord != Ordering::Equal {
                    return ord;
                }
            }
        }
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
pub fn relation_between(v: &GsdVersion, floor: &GsdVersion, ceiling: &GsdVersion) -> SyncRelation {
    if v.precedence_cmp(ceiling) == Ordering::Greater {
        SyncRelation::Newer
    } else if v.precedence_cmp(floor) == Ordering::Less {
        SyncRelation::Older
    } else {
        SyncRelation::InRange
    }
}

/// The synced floor ([`GSD_CORE_SYNCED_VERSION`]).
fn synced_floor() -> GsdVersion {
    GsdVersion::parse(GSD_CORE_SYNCED_VERSION)
        .expect("GSD_CORE_SYNCED_VERSION parses — pinned by the_gsd_core_sync_baseline_is_recorded")
}

/// The synced ceiling ([`GSD_CORE_SYNCED_TREE_VERSION`]).
fn synced_ceiling() -> GsdVersion {
    GsdVersion::parse(GSD_CORE_SYNCED_TREE_VERSION).expect(
        "GSD_CORE_SYNCED_TREE_VERSION parses — pinned by the_gsd_core_sync_baseline_is_recorded",
    )
}

/// `v` against [`GSD_CORE_SYNCED_VERSION`] (floor) and
/// [`GSD_CORE_SYNCED_TREE_VERSION`] (ceiling) — the rule of inferred I-1.
pub fn relation_to_synced(v: &GsdVersion) -> SyncRelation {
    relation_between(v, &synced_floor(), &synced_ceiling())
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
///
/// The `is_file` gate runs first, so a FIFO or device is never opened
/// (T-j0a-02), and at most [`VERSION_READ_CAP`] + 1 bytes are read, so a huge
/// file costs a handful of bytes.
fn read_version_file(path: &Path) -> Option<InstalledVersion> {
    use std::io::Read;
    if !path.is_file() {
        return None;
    }
    let mut buf = Vec::new();
    let read = std::fs::File::open(path)
        .and_then(|file| file.take(VERSION_READ_CAP + 1).read_to_end(&mut buf));
    if read.is_err() || buf.len() as u64 > VERSION_READ_CAP {
        return Some(InstalledVersion::Unrecognised);
    }
    Some(
        std::str::from_utf8(&buf)
            .ok()
            .and_then(GsdVersion::parse)
            .map_or(InstalledVersion::Unrecognised, InstalledVersion::Parsed),
    )
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
///
/// A garbage `VERSION` still wins and reports
/// [`InstalledVersion::Unrecognised`]: falling through to the next candidate
/// would misreport which install GSD would actually load (inferred I-3).
pub fn detect_gsd_install(project_root: Option<&Path>, roots: &InstallRoots) -> GsdInstallStatus {
    install_candidates(project_root, roots)
        .into_iter()
        .find_map(|c| {
            read_version_file(&c.gsd_core_dir.join("VERSION")).map(|version| DetectedInstall {
                runtime: c.runtime,
                scope: c.scope,
                gsd_core_dir: c.gsd_core_dir,
                version,
            })
        })
        .map_or(GsdInstallStatus::NotFound, GsdInstallStatus::Found)
}

impl DetectedInstall {
    /// `project-local Claude`, `global Codex`, `project-local (root)`, ...
    /// Built from the enums only, never from the path (inferred I-10).
    pub fn source_label(&self) -> &'static str {
        match (self.scope, self.runtime) {
            (InstallScope::ProjectLocal, InstallRuntime::ProjectRoot) => "project-local (root)",
            (InstallScope::ProjectLocal, InstallRuntime::Claude) => "project-local Claude",
            (InstallScope::ProjectLocal, InstallRuntime::Codex) => "project-local Codex",
            (InstallScope::Global, InstallRuntime::ProjectRoot) => "global (root)",
            (InstallScope::Global, InstallRuntime::Claude) => "global Claude",
            (InstallScope::Global, InstallRuntime::Codex) => "global Codex",
        }
    }
}

/// How loudly a label should be drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Ok,
    Info,
    Warning,
}

/// The one-line label for `status`, and its severity.
///
/// Every piece is an enum word, a validated [`GsdVersion`] or a constant, so
/// no byte of a `VERSION` file that failed to parse can reach it.
pub fn install_label(status: &GsdInstallStatus) -> (String, Severity) {
    let GsdInstallStatus::Found(install) = status else {
        return (
            format!("GSD not found · app synced to {GSD_CORE_SYNCED_TREE_VERSION}"),
            Severity::Info,
        );
    };
    let source = install.source_label();
    let InstalledVersion::Parsed(v) = &install.version else {
        return (format!("GSD (unrecognised VERSION) · {source}"), Severity::Info);
    };
    match relation_to_synced(v) {
        SyncRelation::InRange => (format!("GSD {v} · {source}"), Severity::Ok),
        SyncRelation::Newer => (
            format!("GSD {v} · {source} · newer than synced {GSD_CORE_SYNCED_TREE_VERSION}"),
            Severity::Warning,
        ),
        SyncRelation::Older => (
            format!("GSD {v} · {source} · older than {GSD_CORE_SYNCED_VERSION}"),
            Severity::Info,
        ),
    }
}

/// The one startup status line: the global install against the synced
/// version, plus how many projects use a project-local install (and how many
/// of those are newer). Source words come from the enums only.
pub fn startup_summary<'a>(
    _global: &GsdInstallStatus,
    _projects: impl IntoIterator<Item = &'a GsdInstallStatus>,
) -> String {
    String::new()
}

/// The highest installed version among `statuses` that is newer than the
/// synced ceiling, if any.
pub fn newest_newer_than_synced<'a>(
    _statuses: impl IntoIterator<Item = &'a GsdInstallStatus>,
) -> Option<GsdVersion> {
    None
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

    // ── startup summary and dashboard scan ─────────────────────────────

    #[test]
    fn startup_summary_names_the_global_gsd_install_against_the_synced_version() {
        use InstallRuntime::*;
        use InstallScope::*;
        let none: [&GsdInstallStatus; 0] = [];
        let newer = newer_str();
        let tree = GSD_CORE_SYNCED_TREE_VERSION;
        let cases = [
            (
                install(Claude, Global, InstalledVersion::Parsed(ceiling())),
                format!("GSD {tree} (global Claude) · app synced to gsd-core {tree}"),
            ),
            (
                install(Claude, Global, InstalledVersion::Parsed(v(&newer))),
                format!(
                    "GSD {newer} (global Claude) is newer than synced gsd-core {tree} — \
                     some formats may be unrecognised"
                ),
            ),
            (
                install(Codex, Global, InstalledVersion::Parsed(v(OLDER))),
                format!(
                    "GSD {OLDER} (global Codex) is older than this app's \
                     {GSD_CORE_SYNCED_VERSION} baseline"
                ),
            ),
            (
                install(Claude, Global, InstalledVersion::Unrecognised),
                format!("GSD (global Claude) has an unrecognised VERSION · app synced to gsd-core {tree}"),
            ),
            (
                GsdInstallStatus::NotFound,
                format!("No global GSD install found · app synced to gsd-core {tree}"),
            ),
        ];
        for (global, text) in cases {
            assert_eq!(startup_summary(&global, none), text);
        }
    }

    #[test]
    fn startup_summary_counts_project_local_gsd_installs() {
        use InstallRuntime::*;
        use InstallScope::*;
        let tree = GSD_CORE_SYNCED_TREE_VERSION;
        let global = GsdInstallStatus::NotFound;
        let base = format!("No global GSD install found · app synced to gsd-core {tree}");
        let local_ok = install(Claude, ProjectLocal, InstalledVersion::Parsed(ceiling()));
        let local_newer = install(Codex, ProjectLocal, InstalledVersion::Parsed(v(&newer_str())));
        // A project resolving to the global install is not counted.
        let global_use = install(Claude, Global, InstalledVersion::Parsed(v(&newer_str())));

        assert_eq!(
            startup_summary(&global, [&local_ok, &global_use, &GsdInstallStatus::NotFound]),
            format!("{base} · 1 project uses a project-local GSD")
        );
        assert_eq!(
            startup_summary(&global, [&local_ok, &local_newer, &global_use]),
            format!("{base} · 2 projects use a project-local GSD (1 newer)")
        );
    }

    #[test]
    fn newest_newer_than_synced_picks_the_highest_newer_gsd_install() {
        use InstallRuntime::*;
        use InstallScope::*;
        let c = ceiling();
        let a = format!("{}.0.0", c.major + 1);
        let b = format!("{}.1.0", c.major + 1);
        let statuses = [
            install(Claude, Global, InstalledVersion::Parsed(v(&a))),
            install(Codex, ProjectLocal, InstalledVersion::Parsed(v(&b))),
            install(Claude, ProjectLocal, InstalledVersion::Parsed(c.clone())),
            install(Claude, ProjectLocal, InstalledVersion::Unrecognised),
            GsdInstallStatus::NotFound,
        ];
        assert_eq!(newest_newer_than_synced(&statuses), Some(v(&b)));
        assert_eq!(newest_newer_than_synced(&statuses[2..]), None);
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
