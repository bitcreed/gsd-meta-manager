//! The sandbox's `$HOME`: an allowlist derived from the running host
//! (28.1-08; D-16, D-17, D-18, D-20).
//!
//! Phase 28 masked a list of known credential locations and left the rest of `$HOME`
//! readable. That is a denylist: any credential file the list did not name
//! (`~/.pgpass`, `~/.password-store`, `~/.claudear/env`, a token under `~/.config/*`)
//! was readable inside, and with the network gone it is still usable through every
//! allowlisted host and every file the operator reads later (AUDIT B2). The profile
//! now replaces `$HOME` with a size-capped tmpfs and re-binds only what a run needs:
//!
//! * the toolchain: the `PATH` hit and the install root of each of [`HOME_TOOLS`]
//!   that lives under `$HOME`, `$CARGO_HOME/{bin,env,config.toml,config}` and
//!   `$RUSTUP_HOME`, so `~/.cargo/credentials.toml` is out by construction;
//! * GSD and rtk: [`HOME_RO_REL`];
//! * the directory of this binary (the `driverctl` shim and the bridge run it);
//! * `$CARGO_HOME/{registry,git}` as a throw-away overlay when bwrap can build one,
//!   else read-only, never read-write: a crate planted there would be compiled later
//!   on the host (T-28.1-34);
//! * a per-run tmpfs at `~/.cache` and `~/.local/share/rtk`.
//!
//! Every path comes from the running host (`PATH`, `CARGO_HOME`, `RUSTUP_HOME`, the
//! binary's own location), never from one user's layout. The derivation is a pure
//! function ([`home_allowlist`]) of a [`HostFacts`] value, so the rules are
//! golden-tested on any runner; [`HostFacts::from_host`] and [`HomeSetup::for_host`]
//! are the impure entries. The rules are `[inferred — audit]` (CONTEXT D-16).
//!
//! The git configuration is not bound either: [`HomeSetup`] writes a three-key
//! per-run gitconfig and the sandbox binds that at `~/.gitconfig` (D-17), so no
//! alias, include, `credential.helper` or other config-executed program of the host
//! reaches the driver (T-28.1-33).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::sandbox::{NULL_MASK_FILES, TMPFS_MASK_DIRS};

/// The programs the driver runs, resolved on the host `PATH`. Those found under
/// `$HOME` are re-bound (D-16).
pub const HOME_TOOLS: &[&str] = &[
    "claude", "node", "npm", "npx", "rtk", "cargo", "rustc", "rustup", "git",
];

/// Entries under `$HOME` re-bound read-only when they exist (D-16): the GSD files the
/// hooks and `gsd-tools` read, the GSD profile and capability files, and the rtk
/// configuration.
pub const HOME_RO_REL: &[&str] = &[
    ".claude/agents",
    ".claude/commands",
    ".claude/skills",
    ".claude/gsd-core",
    ".claude/get-shit-done",
    ".claude/hooks",
    ".claude/scripts",
    ".claude/.gsd-profile",
    ".gsd",
    ".gsd-capabilities.json",
    ".config/rtk",
];

/// The entries of `$CARGO_HOME` re-bound read-only (D-16). Not the whole directory:
/// `credentials.toml` stays out by construction.
pub const CARGO_RO_REL: &[&str] = &["bin", "env", "config.toml", "config"];

/// The entries of `$CARGO_HOME` that a build writes into: an overlay when bwrap can
/// build one, else read-only (D-15, D-16). Never read-write.
pub const CARGO_OVERLAY_REL: &[&str] = &["registry", "git"];

/// The size of the tmpfs over `$HOME`: 256 MiB `[inferred — audit]` (RESEARCH Pitfall 7;
/// tmpfs pages count against the scope's `MemoryMax=6G`).
pub const HOME_TMPFS_SIZE: u64 = 268_435_456;

/// The size of the per-run `~/.cache` tmpfs: 1 GiB `[inferred — audit]`.
pub const CACHE_TMPFS_SIZE: u64 = 1_073_741_824;

/// The size of the per-run `~/.local/share/rtk` tmpfs: 64 MiB `[inferred — audit]`.
pub const RTK_TMPFS_SIZE: u64 = 67_108_864;

/// The per-run gitconfig's file name inside the run's state directory.
pub const GITCONFIG_FILE: &str = "gitconfig";

/// `~/.cache`, relative to `$HOME`.
const CACHE_REL: &str = ".cache";

/// `~/.local/share/rtk`, relative to `$HOME`.
const RTK_REL: &str = ".local/share/rtk";

/// Directories no install root may be: they hold far more than one tool.
const REFUSED_ROOT_REL: &[&str] = &[".local", ".local/share", ".config", ".cache"];

/// The most dangling names [`HomeSetup::dangling_notice`] prints.
const NOTICE_NAMES: usize = 10;

/// One thing the sandbox does to a path under `$HOME`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HomeBind {
    /// `--ro-bind p p`.
    Ro(PathBuf),
    /// `--overlay-src p --tmp-overlay p`: writes land in a throw-away upper layer.
    Overlay(PathBuf),
    /// `--size <size> --tmpfs p`: an empty, size-capped tmpfs.
    Tmpfs { path: PathBuf, size: u64 },
}

impl HomeBind {
    /// The path this entry acts on.
    pub fn path(&self) -> &Path {
        match self {
            HomeBind::Ro(path) | HomeBind::Overlay(path) => path,
            HomeBind::Tmpfs { path, .. } => path,
        }
    }

    /// True for the entries that show host content (everything but a tmpfs).
    fn shows_host(&self) -> bool {
        !matches!(self, HomeBind::Tmpfs { .. })
    }
}

/// One program of [`HOME_TOOLS`] found on the host `PATH`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolHit {
    pub name: String,
    /// The first executable `PATH` hit.
    pub hit: PathBuf,
    /// Its fully resolved path.
    pub real: PathBuf,
}

/// The host facts [`home_allowlist`] derives from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostFacts {
    pub home: PathBuf,
    pub cargo_home: PathBuf,
    pub rustup_home: PathBuf,
    /// This binary, fully resolved.
    pub exe: PathBuf,
    pub tools: Vec<ToolHit>,
    /// Every candidate path of the allowlist that exists on the host.
    pub existing: BTreeSet<PathBuf>,
}

/// True when `path` lies strictly under `home`.
fn strictly_under(path: &Path, home: &Path) -> bool {
    path != home && path.starts_with(home)
}

/// True when `path` is, contains or lies inside one of the credential directories of
/// [`TMPFS_MASK_DIRS`].
fn touches_mask_dir(path: &Path, home: &Path) -> bool {
    TMPFS_MASK_DIRS.iter().any(|rel| {
        let mask = home.join(rel);
        path.starts_with(&mask) || mask.starts_with(path)
    })
}

/// True when `root` may not be bound as a whole: it is `$HOME` or outside it, one of
/// the shared directories [`REFUSED_ROOT_REL`], `$CARGO_HOME`, or it would expose a
/// credential directory, a masked credential file or one of the per-run tmpfs paths.
fn root_refused(root: &Path, facts: &HostFacts) -> bool {
    let home = &facts.home;
    if !strictly_under(root, home)
        || root == facts.cargo_home
        || REFUSED_ROOT_REL.iter().any(|rel| root == home.join(rel))
        || touches_mask_dir(root, home)
    {
        return true;
    }
    NULL_MASK_FILES
        .iter()
        .chain([&CACHE_REL, &RTK_REL])
        .any(|rel| home.join(rel).starts_with(root))
}

/// The directory to bind for a tool whose resolved binary is `real`: the grandparent
/// when the parent is `bin` or `versions`, else the parent. `None` when that root is
/// refused ([`root_refused`]); the caller then binds only the binary file.
pub fn install_root(real: &Path, facts: &HostFacts) -> Option<PathBuf> {
    let parent = real.parent()?;
    let root = match parent.file_name().and_then(|n| n.to_str()) {
        Some("bin" | "versions") => parent.parent()?,
        _ => parent,
    };
    (!root_refused(root, facts)).then(|| root.to_owned())
}

/// A path the allowlist may name at all: strictly under `$HOME` and clear of every
/// credential directory.
fn admissible(path: &Path, facts: &HostFacts) -> bool {
    strictly_under(path, &facts.home) && !touches_mask_dir(path, &facts.home)
}

/// Every path the allowlist could name, before the existence check, the ordering and
/// the nested-entry pruning. `overlay` picks the kind of the two build caches.
fn candidates(facts: &HostFacts, overlay: bool) -> Vec<HomeBind> {
    let home = &facts.home;
    let mut out: Vec<HomeBind> = Vec::new();
    let ro = |path: PathBuf, out: &mut Vec<HomeBind>| {
        if admissible(&path, facts) {
            out.push(HomeBind::Ro(path));
        }
    };
    for tool in &facts.tools {
        ro(tool.hit.clone(), &mut out);
        if let Some(root) = install_root(&tool.real, facts) {
            ro(root, &mut out);
        }
    }
    for rel in CARGO_RO_REL {
        ro(facts.cargo_home.join(rel), &mut out);
    }
    for rel in CARGO_OVERLAY_REL {
        let path = facts.cargo_home.join(rel);
        if admissible(&path, facts) {
            out.push(if overlay {
                HomeBind::Overlay(path)
            } else {
                HomeBind::Ro(path)
            });
        }
    }
    ro(facts.rustup_home.clone(), &mut out);
    for rel in HOME_RO_REL {
        ro(home.join(rel), &mut out);
    }
    if let Some(dir) = facts.exe.parent() {
        if root_refused(dir, facts) {
            ro(facts.exe.clone(), &mut out);
        } else {
            ro(dir.to_owned(), &mut out);
        }
    }
    out.push(HomeBind::Tmpfs {
        path: home.join(CACHE_REL),
        size: CACHE_TMPFS_SIZE,
    });
    out.push(HomeBind::Tmpfs {
        path: home.join(RTK_REL),
        size: RTK_TMPFS_SIZE,
    });
    out
}

/// The `$HOME` entries of the sandbox, derived from `facts`.
///
/// Only paths in `facts.existing` are named (a bind of a missing path aborts bwrap);
/// the two tmpfs entries are always present. Parents come before children (by depth,
/// then name), an entry nested in another read-only or overlay entry is dropped, and
/// anything outside `$HOME` is skipped: it is visible through the read-only `/`.
pub fn home_allowlist(facts: &HostFacts, overlay: bool) -> Vec<HomeBind> {
    let mut picked: Vec<HomeBind> = candidates(facts, overlay)
        .into_iter()
        .filter(|bind| !bind.shows_host() || facts.existing.contains(bind.path()))
        .collect();
    picked.sort_by(|a, b| {
        let key = |bind: &HomeBind| (bind.path().components().count(), bind.path().to_owned());
        key(a).cmp(&key(b))
    });
    let mut kept: Vec<HomeBind> = Vec::new();
    for bind in picked {
        let shadowed = kept.iter().any(|earlier| {
            earlier.shows_host()
                && (earlier.path() == bind.path() || bind.path().starts_with(earlier.path()))
        });
        if !shadowed {
            kept.push(bind);
        }
    }
    kept
}

/// The text of the per-run gitconfig (D-17): `user.name`, `user.email` and
/// `init.defaultBranch`, each only when given. A value is quoted with `\` and `"`
/// escaped; one holding a line break or a NUL is dropped, so a hostile host value
/// cannot inject a second key.
pub fn render_gitconfig(
    name: Option<&str>,
    email: Option<&str>,
    default_branch: Option<&str>,
) -> String {
    fn quote(value: &str) -> Option<String> {
        if value.contains(['\n', '\r', '\0']) {
            return None;
        }
        Some(format!(
            "\"{}\"",
            value.replace('\\', "\\\\").replace('"', "\\\"")
        ))
    }
    let mut out = String::new();
    let user: Vec<(&str, String)> = [("name", name), ("email", email)]
        .into_iter()
        .filter_map(|(key, value)| Some((key, quote(value?)?)))
        .collect();
    if !user.is_empty() {
        out.push_str("[user]\n");
        for (key, value) in user {
            out.push_str(&format!("\t{key} = {value}\n"));
        }
    }
    if let Some(branch) = default_branch.and_then(quote) {
        out.push_str(&format!("[init]\n\tdefaultBranch = {branch}\n"));
    }
    out
}

/// The symlinks inside the read-only binds whose target lies outside every bind
/// (D-18): the top level of each read-only or overlay directory, plus one level below
/// `~/.claude/skills`. A link is never followed by the sandbox, so it dangles there.
/// A link that is broken on the host too is not reported.
pub fn dangling_symlinks(binds: &[HomeBind]) -> Vec<PathBuf> {
    dangling_within(binds, None)
}

/// [`dangling_symlinks`], ignoring targets outside `home` when given: those are
/// visible through the read-only `/` and do not dangle.
fn dangling_within(binds: &[HomeBind], home: Option<&Path>) -> Vec<PathBuf> {
    let bound: Vec<&Path> = binds
        .iter()
        .filter(|bind| bind.shows_host())
        .map(HomeBind::path)
        .collect();
    let dangles = |link: &Path| -> bool {
        let Ok(target) = std::fs::canonicalize(link) else {
            return false;
        };
        if home.is_some_and(|home| !target.starts_with(home)) {
            return false;
        }
        !bound.iter().any(|root| target.starts_with(root))
    };
    let is_link = |path: &Path| {
        std::fs::symlink_metadata(path).is_ok_and(|meta| meta.file_type().is_symlink())
    };
    let mut out: Vec<PathBuf> = Vec::new();
    for dir in bound.iter().filter(|path| path.is_dir()) {
        let is_skills = dir.file_name().is_some_and(|n| n == "skills")
            && dir
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|n| n == ".claude");
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if is_link(&path) {
                if dangles(&path) {
                    out.push(path);
                }
            } else if is_skills && path.is_dir() {
                let Ok(inner) = std::fs::read_dir(&path) else {
                    continue;
                };
                for child in inner.filter_map(Result::ok).map(|e| e.path()) {
                    if is_link(&child) && dangles(&child) {
                        out.push(child);
                    }
                }
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// What one launch needs from `$HOME` besides the host's own environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HomeSetup {
    /// The `$HOME` entries of the sandbox ([`home_allowlist`]).
    pub binds: Vec<HomeBind>,
    /// The per-run gitconfig, bound at `~/.gitconfig`.
    pub gitconfig: Option<PathBuf>,
    /// The read-only and overlay entries: the roots a kept user hook may name
    /// (D-19).
    pub keep_roots: Vec<PathBuf>,
    /// The symlinks that dangle inside (D-18).
    pub dangling: Vec<PathBuf>,
}

impl HomeSetup {
    /// The line printed once at launch for the dangling links (D-18): the count and
    /// at most ten names, relative to `home`. `None` when nothing dangles.
    pub fn dangling_notice(&self, home: &Path) -> Option<String> {
        if self.dangling.is_empty() {
            return None;
        }
        let names: Vec<String> = self
            .dangling
            .iter()
            .take(NOTICE_NAMES)
            .map(|path| match path.strip_prefix(home) {
                Ok(rest) => format!("~/{}", rest.display()),
                Err(_) => path.display().to_string(),
            })
            .collect();
        let more = self.dangling.len().saturating_sub(NOTICE_NAMES);
        let tail = if more > 0 {
            format!(", and {more} more")
        } else {
            String::new()
        };
        Some(format!(
            "driver launch: {} entries dangle inside the sandbox (their targets are outside the $HOME allowlist): {}{tail}",
            self.dangling.len(),
            names.join(", ")
        ))
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod imp {
    use std::ffi::OsString;
    use std::io;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use super::{
        candidates, dangling_within, home_allowlist, render_gitconfig, HomeBind, HomeSetup,
        HostFacts, ToolHit, GITCONFIG_FILE, HOME_TOOLS,
    };
    use crate::launcher::config_dir::write_atomic;
    use crate::launcher::process::run_bounded;

    /// The budget of one `git config --get`.
    const GIT_BUDGET: Duration = Duration::from_secs(5);

    /// The first executable file called `name` on `PATH`.
    fn which(name: &str) -> Option<PathBuf> {
        let path = std::env::var_os("PATH")?;
        std::env::split_paths(&path)
            .filter(|dir| dir.is_absolute())
            .map(|dir| dir.join(name))
            .find(|candidate| {
                std::fs::metadata(candidate)
                    .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
            })
    }

    /// An absolute directory from the environment, else `home/<default>`.
    fn dir_from_env(name: &str, home: &Path, default: &str) -> PathBuf {
        std::env::var_os(name)
            .map(PathBuf::from)
            .filter(|dir| dir.is_absolute())
            .unwrap_or_else(|| home.join(default))
    }

    impl HostFacts {
        /// The facts of this host. **The one function here that reads the file system
        /// and the environment.**
        ///
        /// A `$HOME` that is itself a symlink is handled: resolved paths are
        /// re-expressed under the name the launcher knows `$HOME` by.
        pub fn from_host(home: &Path, exe: &Path) -> HostFacts {
            let home_real = std::fs::canonicalize(home).unwrap_or_else(|_| home.to_owned());
            let rebase = |path: PathBuf| match path.strip_prefix(&home_real) {
                Ok(rest) if home_real != home => home.join(rest),
                _ => path,
            };
            let tools = HOME_TOOLS
                .iter()
                .filter_map(|name| {
                    let hit = which(name)?;
                    let real = rebase(std::fs::canonicalize(&hit).ok()?);
                    Some(ToolHit {
                        name: (*name).to_owned(),
                        hit,
                        real,
                    })
                })
                .collect();
            let mut facts = HostFacts {
                home: home.to_owned(),
                cargo_home: dir_from_env("CARGO_HOME", home, ".cargo"),
                rustup_home: dir_from_env("RUSTUP_HOME", home, ".rustup"),
                exe: rebase(std::fs::canonicalize(exe).unwrap_or_else(|_| exe.to_owned())),
                tools,
                existing: Default::default(),
            };
            facts.existing = candidates(&facts, true)
                .into_iter()
                .map(|bind| bind.path().to_owned())
                .filter(|path| path.exists())
                .collect();
            facts
        }
    }

    /// `git config --get key` on the host, run from `/` so no repository's own
    /// configuration applies. `None` when unset, unreadable or empty.
    fn git_config_value(key: &str) -> Option<String> {
        let args: Vec<OsString> = vec!["config".into(), "--get".into(), key.into()];
        let out = run_bounded("git", &args, Some(Path::new("/")), &[], GIT_BUDGET)?;
        if !out.status.success() {
            return None;
        }
        let text = String::from_utf8(out.stdout).ok()?;
        let value = text.trim_end_matches(['\n', '\r']);
        (!value.is_empty()).then(|| value.to_owned())
    }

    impl HomeSetup {
        /// Derive the `$HOME` setup of one launch and write its gitconfig into
        /// `state_dir` (0600, through a temp file and a rename, so two launches never
        /// see a half-written file). **The one impure entry; tests call it too.**
        ///
        /// `overlay` is the host's measured overlay capability
        /// ([`crate::launcher::host::HostCaps::overlay`]).
        pub fn for_host(
            home: &Path,
            exe: &Path,
            overlay: bool,
            state_dir: &Path,
        ) -> io::Result<HomeSetup> {
            let facts = HostFacts::from_host(home, exe);
            let binds = home_allowlist(&facts, overlay);
            let name = git_config_value("user.name");
            let email = git_config_value("user.email");
            let branch = git_config_value("init.defaultBranch");
            let text = render_gitconfig(name.as_deref(), email.as_deref(), branch.as_deref());
            let gitconfig = state_dir.join(GITCONFIG_FILE);
            write_atomic(&gitconfig, text.as_bytes(), 0o600)?;
            let keep_roots = binds
                .iter()
                .filter(|bind| !matches!(bind, HomeBind::Tmpfs { .. }))
                .map(|bind| bind.path().to_owned())
                .collect();
            let dangling = dangling_within(&binds, Some(home));
            Ok(HomeSetup {
                binds,
                gitconfig: Some(gitconfig),
                keep_roots,
                dangling,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A host like the one the plan describes, every candidate existing.
    fn facts() -> HostFacts {
        let tool = |name: &str, hit: &str, real: &str| ToolHit {
            name: name.to_owned(),
            hit: PathBuf::from(hit),
            real: PathBuf::from(real),
        };
        let node = "/home/u/.nvm/versions/node/v24";
        let mut facts = HostFacts {
            home: PathBuf::from("/home/u"),
            cargo_home: PathBuf::from("/home/u/.cargo"),
            rustup_home: PathBuf::from("/home/u/.rustup"),
            exe: PathBuf::from("/home/u/.cargo/bin/gsd-meta-manager"),
            tools: vec![
                tool(
                    "claude",
                    "/home/u/.local/bin/claude",
                    "/home/u/.local/share/claude/versions/2.1.310",
                ),
                tool(
                    "node",
                    &format!("{node}/bin/node"),
                    &format!("{node}/bin/node"),
                ),
                tool(
                    "npm",
                    &format!("{node}/bin/npm"),
                    &format!("{node}/lib/node_modules/npm/bin/npm-cli.js"),
                ),
                tool("rtk", "/home/u/.local/bin/rtk", "/home/u/.local/bin/rtk"),
                tool(
                    "cargo",
                    "/home/u/.cargo/bin/cargo",
                    "/home/u/.cargo/bin/rustup",
                ),
                tool("git", "/usr/bin/git", "/usr/bin/git"),
            ],
            existing: BTreeSet::new(),
        };
        facts.existing = candidates(&facts, true)
            .iter()
            .map(|bind| bind.path().to_owned())
            .collect();
        facts
    }

    fn paths(binds: &[HomeBind]) -> Vec<String> {
        binds
            .iter()
            .map(|bind| {
                let tag = match bind {
                    HomeBind::Ro(_) => "ro",
                    HomeBind::Overlay(_) => "overlay",
                    HomeBind::Tmpfs { .. } => "tmpfs",
                };
                format!("{tag} {}", bind.path().display())
            })
            .collect()
    }

    #[test]
    fn the_golden_allowlist_with_overlay() {
        let got = paths(&home_allowlist(&facts(), true));
        // Parents before children: by depth, then name. Pinned literally so a change
        // to a derivation rule shows up as a diff of this list.
        let want = [
            "tmpfs /home/u/.cache",
            "ro /home/u/.gsd",
            "ro /home/u/.gsd-capabilities.json",
            "ro /home/u/.rustup",
            "ro /home/u/.cargo/bin",
            "ro /home/u/.cargo/config",
            "ro /home/u/.cargo/config.toml",
            "ro /home/u/.cargo/env",
            "overlay /home/u/.cargo/git",
            "overlay /home/u/.cargo/registry",
            "ro /home/u/.claude/.gsd-profile",
            "ro /home/u/.claude/agents",
            "ro /home/u/.claude/commands",
            "ro /home/u/.claude/get-shit-done",
            "ro /home/u/.claude/gsd-core",
            "ro /home/u/.claude/hooks",
            "ro /home/u/.claude/scripts",
            "ro /home/u/.claude/skills",
            "ro /home/u/.config/rtk",
            "ro /home/u/.local/bin/claude",
            "ro /home/u/.local/bin/rtk",
            "ro /home/u/.local/share/claude",
            "tmpfs /home/u/.local/share/rtk",
            "ro /home/u/.nvm/versions/node/v24",
        ];
        let want: Vec<String> = want.iter().map(|s| (*s).to_owned()).collect();
        assert_eq!(got, want);
        assert!(!got.iter().any(|e| e == "ro /home/u/.cargo"));
        assert!(!got.iter().any(|e| e == "ro /home/u/.local"));
        assert!(!got.iter().any(|e| e.contains("/usr/bin")));
    }

    #[test]
    fn without_overlay_the_build_caches_are_read_only() {
        let got = paths(&home_allowlist(&facts(), false));
        assert!(got.contains(&"ro /home/u/.cargo/registry".to_owned()));
        assert!(got.contains(&"ro /home/u/.cargo/git".to_owned()));
        assert!(!got.iter().any(|e| e.starts_with("overlay")));
    }

    #[test]
    fn an_install_root_is_the_grandparent_of_a_bin_or_versions_parent() {
        let f = facts();
        assert_eq!(
            install_root(
                Path::new("/home/u/.local/share/claude/versions/2.1.310"),
                &f
            ),
            Some(PathBuf::from("/home/u/.local/share/claude"))
        );
        assert_eq!(
            install_root(Path::new("/home/u/.nvm/versions/node/v24/bin/node"), &f),
            Some(PathBuf::from("/home/u/.nvm/versions/node/v24"))
        );
        assert_eq!(
            install_root(Path::new("/home/u/tools/x/launcher"), &f),
            Some(PathBuf::from("/home/u/tools/x"))
        );
    }

    #[test]
    fn a_shared_directory_is_never_an_install_root() {
        let f = facts();
        for real in [
            "/home/u/.local/bin/rtk",
            "/home/u/.local/share/foo",
            "/home/u/.config/foo",
            "/home/u/.cache/foo",
            "/home/u/.cargo/bin/rustup",
            "/home/u/tool",
            "/usr/bin/git",
            "/home/u/.claude/foo",
            "/home/u/.ssh/bin/tool",
            "/home/u/.config/gh/tool",
        ] {
            assert_eq!(install_root(Path::new(real), &f), None, "{real}");
        }
    }

    #[test]
    fn no_allowlist_entry_overlaps_a_mask_dir() {
        let mut hostile = facts();
        hostile.tools.push(ToolHit {
            name: "evil".to_owned(),
            hit: PathBuf::from("/home/u/.ssh/bin/evil"),
            real: PathBuf::from("/home/u/.ssh/bin/evil"),
        });
        hostile.tools.push(ToolHit {
            name: "gh".to_owned(),
            hit: PathBuf::from("/home/u/.config/gh/bin/gh"),
            real: PathBuf::from("/home/u/.config/gh/bin/gh"),
        });
        hostile.exe = PathBuf::from("/home/u/.aws/gsd-meta-manager");
        hostile.existing = candidates(&hostile, true)
            .iter()
            .map(|bind| bind.path().to_owned())
            .collect();
        for facts in [facts(), hostile] {
            for overlay in [true, false] {
                for bind in home_allowlist(&facts, overlay) {
                    for rel in TMPFS_MASK_DIRS {
                        let mask = facts.home.join(rel);
                        assert!(
                            !bind.path().starts_with(&mask) && !mask.starts_with(bind.path()),
                            "{bind:?} overlaps the credential directory {}",
                            mask.display()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_tool_outside_home_adds_no_entry() {
        let mut f = facts();
        f.tools = vec![ToolHit {
            name: "git".to_owned(),
            hit: PathBuf::from("/usr/bin/git"),
            real: PathBuf::from("/usr/bin/git"),
        }];
        f.exe = PathBuf::from("/usr/local/bin/gsd-meta-manager");
        f.existing = candidates(&f, true)
            .iter()
            .map(|bind| bind.path().to_owned())
            .collect();
        for bind in home_allowlist(&f, true) {
            assert!(
                matches!(bind, HomeBind::Tmpfs { .. })
                    || !bind.path().to_string_lossy().contains("/usr/"),
                "{bind:?}"
            );
        }
    }

    #[test]
    fn only_existing_paths_are_named_and_the_tmpfs_entries_always() {
        let mut f = facts();
        f.existing.clear();
        assert_eq!(
            paths(&home_allowlist(&f, true)),
            ["tmpfs /home/u/.cache", "tmpfs /home/u/.local/share/rtk"]
        );
    }

    #[test]
    fn the_exe_directory_is_bound_unless_it_is_a_shared_directory() {
        let mut f = facts();
        f.tools.clear();
        f.exe = PathBuf::from("/home/u/.local/gsd-meta-manager");
        f.existing = candidates(&f, true)
            .iter()
            .map(|bind| bind.path().to_owned())
            .collect();
        let got = paths(&home_allowlist(&f, true));
        assert!(got.contains(&"ro /home/u/.local/gsd-meta-manager".to_owned()));
        assert!(!got.contains(&"ro /home/u/.local".to_owned()));
    }

    #[test]
    fn an_exe_in_a_shared_bin_directory_binds_only_the_file() {
        for dir in ["/home/u/.local/bin", "/home/u/bin"] {
            let mut f = facts();
            f.tools.clear();
            f.exe = PathBuf::from(format!("{dir}/gsd-meta-manager"));
            f.existing = candidates(&f, true)
                .iter()
                .map(|bind| bind.path().to_owned())
                .collect();
            let got = paths(&home_allowlist(&f, true));
            assert!(
                got.contains(&format!("ro {dir}/gsd-meta-manager")),
                "{got:?}"
            );
            assert!(!got.contains(&format!("ro {dir}")), "{got:?}");
        }
    }

    #[test]
    fn the_gitconfig_has_the_three_keys_and_escapes_values() {
        assert_eq!(
            render_gitconfig(Some("Ann \"A\" B\\C"), Some("a@b.c"), Some("main")),
            "[user]\n\tname = \"Ann \\\"A\\\" B\\\\C\"\n\temail = \"a@b.c\"\n[init]\n\tdefaultBranch = \"main\"\n"
        );
        assert_eq!(render_gitconfig(None, None, None), "");
        assert_eq!(
            render_gitconfig(None, Some("a@b.c"), None),
            "[user]\n\temail = \"a@b.c\"\n"
        );
    }

    #[test]
    fn a_value_with_a_line_break_is_dropped() {
        let text = render_gitconfig(
            Some("x\n[core]\n\tfsmonitor = evil"),
            Some("a@b.c"),
            Some("m\rain"),
        );
        assert_eq!(text, "[user]\n\temail = \"a@b.c\"\n");
        assert!(!text.contains("fsmonitor"));
    }

    #[test]
    fn the_dangling_notice_lists_ten_names_and_a_count() {
        let home = Path::new("/home/u");
        let setup = HomeSetup {
            dangling: (0..12)
                .map(|n| PathBuf::from(format!("/home/u/.claude/skills/s{n:02}")))
                .collect(),
            ..HomeSetup::default()
        };
        let line = setup.dangling_notice(home).unwrap();
        assert!(line.starts_with("driver launch: 12 entries dangle inside the sandbox"));
        assert!(line.contains("~/.claude/skills/s09"));
        assert!(!line.contains("s10"));
        assert!(line.ends_with(", and 2 more"));
        assert_eq!(HomeSetup::default().dangling_notice(home), None);
    }
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod host_tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::{symlink, PermissionsExt};

    #[test]
    fn dangling_symlinks_reports_links_leaving_the_binds() {
        let root = tempfile::TempDir::new().unwrap();
        let root = root.path().canonicalize().unwrap();
        let skills = root.join(".claude/skills");
        let inside = root.join(".claude/agents");
        let outside = root.join("projects/skill");
        fs::create_dir_all(skills.join("alpha")).unwrap();
        fs::create_dir_all(&inside).unwrap();
        fs::create_dir_all(&outside).unwrap();
        fs::write(inside.join("a.md"), "x").unwrap();
        symlink(&outside, skills.join("away")).unwrap();
        symlink(&inside, skills.join("home-link")).unwrap();
        symlink(&outside, skills.join("alpha/nested-away")).unwrap();
        symlink("/nowhere/at/all", skills.join("broken")).unwrap();
        let binds = vec![HomeBind::Ro(skills.clone()), HomeBind::Ro(inside)];
        assert_eq!(
            dangling_symlinks(&binds),
            [skills.join("alpha/nested-away"), skills.join("away")]
        );
        // The same links, with the home set to a directory that holds no target.
        assert!(dangling_within(&binds, Some(Path::new("/elsewhere"))).is_empty());
    }

    #[test]
    fn for_host_writes_a_private_gitconfig_and_names_only_host_entries_as_roots() {
        let root = tempfile::TempDir::new().unwrap();
        let home = root.path().join("home");
        let state = root.path().join("state");
        fs::create_dir_all(home.join(".gsd")).unwrap();
        fs::create_dir_all(home.join(".ssh")).unwrap();
        fs::create_dir_all(&state).unwrap();
        let setup = HomeSetup::for_host(
            &home,
            Path::new("/nonexistent/gsd-meta-manager"),
            true,
            &state,
        )
        .unwrap();
        let gitconfig = setup.gitconfig.clone().unwrap();
        assert_eq!(gitconfig, state.join(GITCONFIG_FILE));
        let mode = fs::metadata(&gitconfig).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        // Only the three keys, whatever the host's own git configuration holds.
        let text = fs::read_to_string(&gitconfig).unwrap();
        for line in text.lines() {
            let line = line.trim();
            assert!(
                line.is_empty()
                    || ["[user]", "[init]"].contains(&line)
                    || ["name = ", "email = ", "defaultBranch = "]
                        .iter()
                        .any(|key| line.starts_with(key)),
                "unexpected gitconfig line {line:?}"
            );
        }
        assert!(setup.keep_roots.contains(&home.join(".gsd")));
        assert!(setup
            .keep_roots
            .iter()
            .all(|root| !root.starts_with(home.join(".ssh"))));
        assert!(setup.binds.iter().any(
            |bind| matches!(bind, HomeBind::Tmpfs { path, .. } if path == &home.join(".cache"))
        ));
        assert!(!setup
            .keep_roots
            .iter()
            .any(|root| root == &home.join(".cache")));
    }

    #[test]
    fn two_launches_write_separate_gitconfigs_and_leave_no_temp_file() {
        let root = tempfile::TempDir::new().unwrap();
        let home = root.path().join("home");
        fs::create_dir_all(&home).unwrap();
        let states: Vec<PathBuf> = ["a", "b"]
            .iter()
            .map(|name| {
                let dir = root.path().join(name);
                fs::create_dir_all(&dir).unwrap();
                dir
            })
            .collect();
        let handles: Vec<_> = states
            .iter()
            .flat_map(|state| std::iter::repeat_n(state.clone(), 3))
            .map(|state| {
                let home = home.clone();
                std::thread::spawn(move || {
                    HomeSetup::for_host(&home, Path::new("/nonexistent/x"), false, &state).unwrap()
                })
            })
            .collect();
        let setups: Vec<HomeSetup> = handles.into_iter().map(|h| h.join().unwrap()).collect();
        assert_eq!(setups[0].gitconfig, Some(states[0].join(GITCONFIG_FILE)));
        assert_eq!(setups[3].gitconfig, Some(states[1].join(GITCONFIG_FILE)));
        for state in &states {
            let names: Vec<String> = fs::read_dir(state)
                .unwrap()
                .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .collect();
            assert_eq!(names, [GITCONFIG_FILE], "{state:?}");
        }
        // Whatever the interleaving, a reader sees the whole file.
        assert_eq!(
            fs::read(states[0].join(GITCONFIG_FILE)).unwrap(),
            fs::read(states[1].join(GITCONFIG_FILE)).unwrap()
        );
    }

    #[test]
    fn host_facts_resolve_a_home_symlink_back_to_the_known_name() {
        let root = tempfile::TempDir::new().unwrap();
        let real = root.path().canonicalize().unwrap().join("real-home");
        fs::create_dir_all(real.join(".gsd")).unwrap();
        fs::create_dir_all(real.join("bin")).unwrap();
        fs::write(real.join("bin/gsd-meta-manager"), "x").unwrap();
        let link = root.path().canonicalize().unwrap().join("home-link");
        symlink(&real, &link).unwrap();
        let facts = HostFacts::from_host(&link, &link.join("bin/gsd-meta-manager"));
        assert_eq!(facts.exe, link.join("bin/gsd-meta-manager"));
        assert!(facts.existing.contains(&link.join(".gsd")));
        let got = home_allowlist(&facts, false);
        assert!(got.contains(&HomeBind::Ro(link.join(".gsd"))), "{got:?}");
    }
}
