//! The host directories the launcher reads, binds and masks (quick 261113-dck).
//!
//! `CLAUDE_CONFIG_DIR` and the XDG base-directory variables relocate the Claude
//! root and the credential stores of gh, gcloud, glab and the keyrings. The
//! launcher used to hard-code `~/.claude`, `~/.config` and `~/.local/share`, so on
//! a host with a relocated root it read the wrong account's token and left the
//! relocated credential stores readable inside the sandbox. [`HostDirs`] is the one
//! place those locations are decided; every launcher consumer takes it by
//! reference.
//!
//! **Blank or relative counts as unset `[inferred — audit]` (I-1).**
//! `CLAUDE_CONFIG_DIR` goes through the existing
//! [`resolve_config_root`](crate::agents::adapters::claude::resolve_config_root),
//! which Claude Code's own rule mirrors: a non-empty absolute value wins, anything
//! else is ignored (a literal `~/x` is relative, so ignored too). The `XDG_*`
//! variables use one private helper with the same semantics. The `dirs` crate is
//! not reused for them because it cannot be injected in a test and ignores XDG on
//! macOS while gh and gcloud honour it; `InstallRoots::config_home` is not reused
//! because it accepts relative values and expands `~`, which is unsafe for a mount
//! path. The stored run token keeps [`auth::token_path`](super::auth::token_path),
//! the resolver its writer uses.
//!
//! **Defaults are never dropped (I-4).** Every list here starts with the
//! `$HOME`-relative default and adds the relocated path, so a mask or bind that
//! existed before still exists.
//!
//! [`resolve`](HostDirs::resolve) is pure: it takes the home and an environment
//! lookup. [`from_host`](HostDirs::from_host) is the one impure entry.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use super::sandbox::{NULL_MASK_FILES, TMPFS_MASK_DIRS};
use crate::agents::adapters::claude::resolve_config_root;

/// Credential directories under `$XDG_CONFIG_HOME`; the `$HOME`-relative twins are
/// `.config/gh`, `.config/gcloud` and `.config/glab-cli` in
/// [`TMPFS_MASK_DIRS`].
pub const XDG_CONFIG_CREDENTIAL_DIRS: &[&str] = &["gh", "gcloud", "glab-cli"];

/// Credential directories under `$XDG_DATA_HOME`; the `$HOME`-relative twin is
/// `.local/share/keyrings` in [`TMPFS_MASK_DIRS`].
pub const XDG_DATA_CREDENTIAL_DIRS: &[&str] = &["keyrings"];

/// Credential files under `$XDG_CONFIG_HOME`; the `$HOME`-relative twin is
/// `.config/git/credentials` in [`NULL_MASK_FILES`].
pub const XDG_CONFIG_CREDENTIAL_FILES: &[&str] = &["git/credentials"];

/// The host's relocatable directories, resolved once per launch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostDirs {
    /// The host `$HOME`.
    pub home: PathBuf,
    /// The Claude root: `$CLAUDE_CONFIG_DIR` when non-empty and absolute, else
    /// `<home>/.claude`.
    pub claude_root: PathBuf,
    /// `$XDG_CONFIG_HOME`, else `<home>/.config`.
    pub config_home: PathBuf,
    /// `$XDG_DATA_HOME`, else `<home>/.local/share`.
    pub data_home: PathBuf,
    /// `$XDG_CACHE_HOME`, else `<home>/.cache`.
    pub cache_home: PathBuf,
    /// `$XDG_RUNTIME_DIR` when non-empty and absolute.
    pub runtime_dir: Option<PathBuf>,
    /// Where the stored run token lives (`auth::token_path()`).
    pub run_token: Option<PathBuf>,
}

/// A non-empty absolute value wins, else `default`.
fn xdg_dir(
    env: &impl Fn(&str) -> Option<OsString>,
    name: &str,
    home: &Path,
    default: &str,
) -> PathBuf {
    env(name)
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty() && p.is_absolute())
        .unwrap_or_else(|| home.join(default))
}

/// The stored token's path where the platform has one.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn host_run_token() -> Option<PathBuf> {
    super::auth::token_path()
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn host_run_token() -> Option<PathBuf> {
    None
}

/// Lexical order-preserving dedup, first occurrence kept.
fn dedup(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::with_capacity(paths.len());
    for p in paths {
        if !out.contains(&p) {
            out.push(p);
        }
    }
    out
}

impl HostDirs {
    /// Resolve the directories from `home` and an environment lookup. Pure;
    /// `run_token` stays `None`.
    pub fn resolve(home: &Path, env: impl Fn(&str) -> Option<OsString>) -> HostDirs {
        let claude_root = resolve_config_root(env("CLAUDE_CONFIG_DIR"), Some(home.to_path_buf()))
            .unwrap_or_else(|| home.join(".claude"));
        let runtime_dir = env("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .filter(|p| !p.as_os_str().is_empty() && p.is_absolute());
        HostDirs {
            home: home.to_path_buf(),
            claude_root,
            config_home: xdg_dir(&env, "XDG_CONFIG_HOME", home, ".config"),
            data_home: xdg_dir(&env, "XDG_DATA_HOME", home, ".local/share"),
            cache_home: xdg_dir(&env, "XDG_CACHE_HOME", home, ".cache"),
            runtime_dir,
            run_token: None,
        }
    }

    /// [`resolve`](Self::resolve) against the process environment, with the stored
    /// token's path. **The impure entry.**
    pub fn from_host(home: &Path) -> HostDirs {
        let mut dirs = HostDirs::resolve(home, |name| std::env::var_os(name));
        dirs.run_token = host_run_token();
        dirs
    }

    /// `[claude_root, <home>/.claude]`, deduplicated.
    pub fn claude_roots(&self) -> Vec<PathBuf> {
        dedup(vec![self.claude_root.clone(), self.home.join(".claude")])
    }

    /// `[<home>/.config, config_home]`, deduplicated.
    pub fn config_homes(&self) -> Vec<PathBuf> {
        dedup(vec![self.home.join(".config"), self.config_home.clone()])
    }

    /// Every credential directory: the `$HOME`-relative defaults in their constant
    /// order, then the relocated XDG ones.
    pub fn credential_dirs(&self) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = TMPFS_MASK_DIRS.iter().map(|r| self.home.join(r)).collect();
        out.extend(
            XDG_CONFIG_CREDENTIAL_DIRS
                .iter()
                .map(|r| self.config_home.join(r)),
        );
        out.extend(
            XDG_DATA_CREDENTIAL_DIRS
                .iter()
                .map(|r| self.data_home.join(r)),
        );
        dedup(out)
    }

    /// The credential files inside the Claude root. Claude Code keeps
    /// `.claude.json` inside `$CLAUDE_CONFIG_DIR` when that is set.
    pub fn claude_credential_files(&self) -> Vec<PathBuf> {
        vec![
            self.claude_root.join(".credentials.json"),
            self.claude_root.join(".claude.json"),
        ]
    }

    /// Every credential file: the `$HOME`-relative defaults in their constant order,
    /// then the Claude root's, the relocated git credentials and the run token.
    pub fn credential_files(&self) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = NULL_MASK_FILES.iter().map(|r| self.home.join(r)).collect();
        out.extend(self.claude_credential_files());
        out.extend(
            XDG_CONFIG_CREDENTIAL_FILES
                .iter()
                .map(|r| self.config_home.join(r)),
        );
        out.extend(self.run_token.clone());
        dedup(out)
    }
}

/// Where a path sits relative to `$HOME`, `/tmp` and the paths a mask must never
/// cover.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// At or under `$HOME`.
    UnderHome,
    /// At or under `/tmp`.
    UnderTmp,
    /// Outside both, and safe to cover with a mask.
    Maskable,
    /// Not absolute, `/`, or an ancestor-or-equal of `$HOME`, `/tmp` or a
    /// protected path: a mask there would hide the sandbox's own footing.
    Unmaskable,
}

/// Classify `path`. Lexical and component-wise, checked in order: under `home`,
/// under `/tmp`, then unmaskable, else maskable.
pub fn placement(path: &Path, home: &Path, protected: &[PathBuf]) -> Placement {
    if path.starts_with(home) {
        return Placement::UnderHome;
    }
    if path.starts_with("/tmp") {
        return Placement::UnderTmp;
    }
    let ancestor_of = |other: &Path| other.starts_with(path);
    if !path.is_absolute()
        || path == Path::new("/")
        || ancestor_of(home)
        || ancestor_of(Path::new("/tmp"))
        || protected.iter().any(|p| ancestor_of(p))
    {
        return Placement::Unmaskable;
    }
    Placement::Maskable
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<OsString> + 'a {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| OsString::from(v))
        }
    }

    fn home() -> PathBuf {
        PathBuf::from("/home/u")
    }

    #[test]
    fn no_env_yields_exactly_the_defaults() {
        let d = HostDirs::resolve(&home(), env_of(&[]));
        assert_eq!(d.home, home());
        assert_eq!(d.claude_root, PathBuf::from("/home/u/.claude"));
        assert_eq!(d.config_home, PathBuf::from("/home/u/.config"));
        assert_eq!(d.data_home, PathBuf::from("/home/u/.local/share"));
        assert_eq!(d.cache_home, PathBuf::from("/home/u/.cache"));
        assert_eq!(d.runtime_dir, None);
        assert_eq!(d.run_token, None);
        assert_eq!(d.claude_roots(), vec![PathBuf::from("/home/u/.claude")]);
        assert_eq!(d.config_homes(), vec![PathBuf::from("/home/u/.config")]);
    }

    #[test]
    fn absolute_values_win_for_each_variable() {
        let d = HostDirs::resolve(
            &home(),
            env_of(&[
                ("CLAUDE_CONFIG_DIR", "/p/claude"),
                ("XDG_CONFIG_HOME", "/x/cfg"),
                ("XDG_DATA_HOME", "/x/data"),
                ("XDG_CACHE_HOME", "/x/cache"),
                ("XDG_RUNTIME_DIR", "/run/user/1"),
            ]),
        );
        assert_eq!(d.claude_root, PathBuf::from("/p/claude"));
        assert_eq!(d.config_home, PathBuf::from("/x/cfg"));
        assert_eq!(d.data_home, PathBuf::from("/x/data"));
        assert_eq!(d.cache_home, PathBuf::from("/x/cache"));
        assert_eq!(d.runtime_dir, Some(PathBuf::from("/run/user/1")));
    }

    #[test]
    fn blank_relative_and_tilde_values_count_as_unset() {
        for bad in ["", "   ", "rel/dir", "~/x"] {
            let d = HostDirs::resolve(
                &home(),
                env_of(&[
                    ("CLAUDE_CONFIG_DIR", bad),
                    ("XDG_CONFIG_HOME", bad),
                    ("XDG_DATA_HOME", bad),
                    ("XDG_CACHE_HOME", bad),
                    ("XDG_RUNTIME_DIR", bad),
                ]),
            );
            let none = HostDirs::resolve(&home(), env_of(&[]));
            assert_eq!(d, none, "{bad:?} must count as unset");
        }
    }

    #[test]
    fn claude_root_matches_resolve_config_root_for_every_input() {
        for value in ["", "   ", "rel/dir", "~/x", "/abs/profile"] {
            let d = HostDirs::resolve(&home(), env_of(&[("CLAUDE_CONFIG_DIR", value)]));
            assert_eq!(
                Some(d.claude_root),
                resolve_config_root(Some(OsString::from(value)), Some(home())),
                "{value:?}"
            );
        }
    }

    #[test]
    fn claude_roots_and_config_homes_deduplicate() {
        let same = HostDirs::resolve(
            &home(),
            env_of(&[
                ("CLAUDE_CONFIG_DIR", "/home/u/.claude"),
                ("XDG_CONFIG_HOME", "/home/u/.config"),
            ]),
        );
        assert_eq!(same.claude_roots().len(), 1);
        assert_eq!(same.config_homes().len(), 1);
        let moved = HostDirs::resolve(
            &home(),
            env_of(&[
                ("CLAUDE_CONFIG_DIR", "/p/claude"),
                ("XDG_CONFIG_HOME", "/x/cfg"),
            ]),
        );
        assert_eq!(
            moved.claude_roots(),
            vec![PathBuf::from("/p/claude"), PathBuf::from("/home/u/.claude")]
        );
        assert_eq!(
            moved.config_homes(),
            vec![PathBuf::from("/home/u/.config"), PathBuf::from("/x/cfg")]
        );
    }

    #[test]
    fn credential_dirs_start_with_the_home_defaults_then_the_relocated_ones() {
        let defaults = HostDirs::resolve(&home(), env_of(&[]));
        let expected: Vec<PathBuf> = TMPFS_MASK_DIRS.iter().map(|r| home().join(r)).collect();
        assert_eq!(
            defaults.credential_dirs(),
            expected,
            "no relocation adds nothing"
        );

        let moved = HostDirs::resolve(
            &home(),
            env_of(&[("XDG_CONFIG_HOME", "/x/cfg"), ("XDG_DATA_HOME", "/x/data")]),
        );
        let got = moved.credential_dirs();
        assert_eq!(&got[..expected.len()], &expected[..]);
        assert_eq!(
            &got[expected.len()..],
            &[
                PathBuf::from("/x/cfg/gh"),
                PathBuf::from("/x/cfg/gcloud"),
                PathBuf::from("/x/cfg/glab-cli"),
                PathBuf::from("/x/data/keyrings"),
            ]
        );
    }

    #[test]
    fn credential_files_start_with_the_defaults_then_claude_git_and_the_token() {
        let mut d = HostDirs::resolve(
            &home(),
            env_of(&[
                ("CLAUDE_CONFIG_DIR", "/p/claude"),
                ("XDG_CONFIG_HOME", "/x/cfg"),
            ]),
        );
        d.run_token = Some(PathBuf::from("/x/cfg/gsd-meta-manager/driver-oauth-token"));
        let expected: Vec<PathBuf> = NULL_MASK_FILES.iter().map(|r| home().join(r)).collect();
        let got = d.credential_files();
        assert_eq!(&got[..expected.len()], &expected[..]);
        assert_eq!(
            &got[expected.len()..],
            &[
                PathBuf::from("/p/claude/.credentials.json"),
                PathBuf::from("/p/claude/.claude.json"),
                PathBuf::from("/x/cfg/git/credentials"),
                PathBuf::from("/x/cfg/gsd-meta-manager/driver-oauth-token"),
            ]
        );
        assert_eq!(
            d.claude_credential_files(),
            vec![
                PathBuf::from("/p/claude/.credentials.json"),
                PathBuf::from("/p/claude/.claude.json"),
            ]
        );
    }

    #[test]
    fn credential_files_deduplicate_the_default_claude_root() {
        let mut d = HostDirs::resolve(&home(), env_of(&[]));
        d.run_token = Some(home().join(".config/gsd-meta-manager/driver-oauth-token"));
        let got = d.credential_files();
        let defaults: Vec<PathBuf> = NULL_MASK_FILES.iter().map(|r| home().join(r)).collect();
        assert_eq!(&got[..defaults.len()], &defaults[..]);
        // The default root's `.credentials.json` and the run token repeat a default
        // entry and are dropped; the root's own `.claude.json` is the one addition.
        assert_eq!(got.len(), defaults.len() + 1, "{got:?}");
        assert_eq!(
            got.last(),
            Some(&PathBuf::from("/home/u/.claude/.claude.json"))
        );
        let unique: std::collections::HashSet<_> = got.iter().collect();
        assert_eq!(unique.len(), got.len(), "no repeats");
    }

    #[test]
    fn placement_classifies_every_case() {
        let h = home();
        let protected = vec![PathBuf::from("/opt/tools/bin/x")];
        let p = |s: &str| placement(Path::new(s), &h, &protected);
        assert_eq!(p("/home/u"), Placement::UnderHome, "equal to home");
        assert_eq!(p("/home/u/.claude"), Placement::UnderHome);
        assert_eq!(p("/tmp"), Placement::UnderTmp);
        assert_eq!(p("/tmp/x/y"), Placement::UnderTmp);
        assert_eq!(p("/"), Placement::Unmaskable);
        assert_eq!(p("/home"), Placement::Unmaskable, "ancestor of home");
        assert_eq!(p("/opt"), Placement::Unmaskable, "protected ancestor");
        assert_eq!(p("/opt/tools/bin/x"), Placement::Unmaskable, "protected");
        assert_eq!(p("rel/dir"), Placement::Unmaskable, "not absolute");
        assert_eq!(p("/srv/profiles/work"), Placement::Maskable);
        assert_eq!(
            p("/homework/x"),
            Placement::Maskable,
            "component-wise, not a string prefix"
        );
    }
}
