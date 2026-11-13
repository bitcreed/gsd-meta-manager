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
//! * GSD and rtk: [`CLAUDE_RO_REL`] under every Claude root, [`HOME_RO_REL`] and
//!   `<config home>/rtk`;
//! * this binary (the `driverctl` shim and the bridge run it): its tool root when it has
//!   a `bin/` or `versions/` layout, else only the file;
//! * `$CARGO_HOME/{registry,git}` as a throw-away overlay when bwrap can build one,
//!   else read-only, never read-write: a crate planted there would be compiled later
//!   on the host (T-28.1-34);
//! * a per-run tmpfs at `~/.cache` and `~/.local/share/rtk` (and at the relocated
//!   `$XDG_CACHE_HOME` / `$XDG_DATA_HOME/rtk`).
//!
//! Every path comes from the running host (`PATH`, `CARGO_HOME`, `RUSTUP_HOME`, the
//! binary's own location), never from one user's layout. The derivation is a pure
//! function ([`home_allowlist`]) of a [`HostFacts`] value, so the rules are
//! golden-tested on any runner; [`HostFacts::from_host`] and [`HomeSetup::for_host`]
//! are the impure entries (their `_with` variants take the [`HostDirs`] they resolve
//! the host environment into). The rules are `[inferred — audit]` (CONTEXT D-16).
//!
//! # Relocated host directories (quick 261113-dck)
//!
//! `CLAUDE_CONFIG_DIR` and the `XDG_*` variables move the Claude root and the credential
//! stores. The binds and masks here are the **union** of the default and the relocated
//! location (I-4), so no default is ever dropped. A Claude root under `$HOME` is bound
//! like any `$HOME` entry; one under `/tmp` is re-bound without an extra tmpfs; one
//! outside both becomes an empty tmpfs with only its GSD entries re-bound read-only (its
//! credential files are `/dev/null`-masked in `sandbox`); a root that is `/` or an
//! ancestor of `$HOME`, `/tmp` or a tool gets no tmpfs at all (I-5, `host_dirs::placement`).
//!
//! The git configuration is not bound either: [`HomeSetup`] writes a three-key
//! per-run gitconfig and the sandbox binds that at `~/.gitconfig` (D-17), so no
//! alias, include, `credential.helper` or other config-executed program of the host
//! reaches the driver (T-28.1-33).

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::host_dirs::{placement, HostDirs, Placement};

/// The programs the driver runs, resolved on the host `PATH`. Those found under
/// `$HOME` are re-bound (D-16).
pub const HOME_TOOLS: &[&str] = &[
    "claude", "node", "npm", "npx", "rtk", "cargo", "rustc", "rustup", "git",
];

/// Entries of a Claude root re-bound read-only when they exist (D-16): the GSD files
/// the hooks and `gsd-tools` read and the GSD profile. Relative to each Claude root
/// (`$CLAUDE_CONFIG_DIR` and `~/.claude`, quick 261113-dck).
pub const CLAUDE_RO_REL: &[&str] = &[
    "agents",
    "commands",
    "skills",
    "gsd-core",
    "get-shit-done",
    "hooks",
    "scripts",
    ".gsd-profile",
];

/// Entries under `$HOME` re-bound read-only when they exist (D-16): the GSD profile and
/// capability files. The Claude root's entries are [`CLAUDE_RO_REL`] and the rtk
/// configuration is `<config home>/rtk`.
pub const HOME_RO_REL: &[&str] = &[".gsd", ".gsd-capabilities.json"];

/// The size of the empty tmpfs over a Claude root outside `$HOME` and `/tmp`: 1 MiB
/// `[inferred — audit]` (the driver never writes there; it exists to hide the root's
/// other contents and to give the read-only re-binds a mount point).
pub const CLAUDE_ROOT_TMPFS_SIZE: u64 = 1_048_576;

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

/// `rtk`, relative to a data home (`$XDG_DATA_HOME/rtk`).
const RTK_DATA_REL: &str = "rtk";

/// Directories no install root may be: they hold far more than one tool. `.local/bin`
/// and `bin` are shared bin directories, where users keep pipx entry points and
/// personal scripts, so a binary installed there is bound as a single file and its
/// siblings stay invisible inside the sandbox.
const REFUSED_ROOT_REL: &[&str] = &[
    ".local",
    ".local/share",
    ".local/bin",
    ".config",
    ".cache",
    "bin",
];

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
    /// The relocatable host directories (`dirs.home == home`).
    pub dirs: HostDirs,
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
/// [`HostDirs::credential_dirs`] (the `$HOME`-relative defaults and the relocated XDG
/// ones).
fn touches_mask_dir(path: &Path, facts: &HostFacts) -> bool {
    facts
        .dirs
        .credential_dirs()
        .iter()
        .any(|mask| path.starts_with(mask) || mask.starts_with(path))
}

/// The paths a mask must never cover: every tool hit and resolved path, this binary,
/// `$CARGO_HOME` and `$RUSTUP_HOME`.
fn protected_paths(facts: &HostFacts) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = facts
        .tools
        .iter()
        .flat_map(|tool| [tool.hit.clone(), tool.real.clone()])
        .collect();
    out.push(facts.exe.clone());
    out.push(facts.cargo_home.clone());
    out.push(facts.rustup_home.clone());
    out
}

/// The per-run tmpfs entries: `~/.cache` and `~/.local/share/rtk` always, then the
/// relocated `$XDG_CACHE_HOME` and `$XDG_DATA_HOME/rtk` when they differ and lie under
/// `$HOME` or where a tmpfs is safe (I-5).
fn per_run_tmpfs(facts: &HostFacts) -> Vec<(PathBuf, u64)> {
    let home = &facts.home;
    let protected = protected_paths(facts);
    let mut out = vec![
        (home.join(CACHE_REL), CACHE_TMPFS_SIZE),
        (home.join(RTK_REL), RTK_TMPFS_SIZE),
    ];
    let relocated = [
        (facts.dirs.cache_home.clone(), CACHE_TMPFS_SIZE),
        (facts.dirs.data_home.join(RTK_DATA_REL), RTK_TMPFS_SIZE),
    ];
    for (path, size) in relocated {
        let usable = matches!(
            placement(&path, home, &protected),
            Placement::UnderHome | Placement::Maskable
        );
        if usable && !out.iter().any(|(known, _)| known == &path) {
            out.push((path, size));
        }
    }
    out
}

/// True when `root` may not be bound as a whole: it is `$HOME` or outside it, one of
/// the shared directories [`REFUSED_ROOT_REL`], a relocated XDG home or a Claude root,
/// `$CARGO_HOME`, or it would expose a credential directory, a masked credential file
/// or one of the per-run tmpfs paths.
fn root_refused(root: &Path, facts: &HostFacts) -> bool {
    let home = &facts.home;
    let dirs = &facts.dirs;
    if !strictly_under(root, home)
        || root == facts.cargo_home
        || REFUSED_ROOT_REL.iter().any(|rel| root == home.join(rel))
        || [&dirs.config_home, &dirs.data_home, &dirs.cache_home]
            .iter()
            .any(|dir| root == dir.as_path())
        || dirs.claude_roots().iter().any(|claude| root == claude)
        || touches_mask_dir(root, facts)
    {
        return true;
    }
    dirs.credential_files()
        .iter()
        .any(|file| file.starts_with(root))
        || per_run_tmpfs(facts)
            .iter()
            .any(|(path, _)| path.starts_with(root))
}

/// The directory to bind for a tool whose resolved binary is `real`: the grandparent
/// when the parent is `bin` or `versions` (a tool directory with a layout of its own),
/// else `None`. `None` also when that root is refused ([`root_refused`]); the caller
/// then binds only the binary file.
///
/// A binary whose parent is any other directory (`~/Downloads/node`, `~/tools/x/node`)
/// gets no directory bind: the parent is not known to hold only this product, and its
/// siblings (scripts, wrappers with embedded tokens, other installers' state) must stay
/// invisible inside the sandbox (WR-04). A tool that needs siblings next to its binary
/// has to be installed under a `bin/` or `versions/` layout `[inferred — audit]`.
pub fn install_root(real: &Path, facts: &HostFacts) -> Option<PathBuf> {
    let parent = real.parent()?;
    let root = match parent.file_name().and_then(|n| n.to_str()) {
        Some("bin" | "versions") => parent.parent()?,
        _ => return None,
    };
    (!root_refused(root, facts)).then(|| root.to_owned())
}

/// A path the allowlist may name at all: strictly under `$HOME` and clear of every
/// credential directory.
fn admissible(path: &Path, facts: &HostFacts) -> bool {
    strictly_under(path, &facts.home) && !touches_mask_dir(path, facts)
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
    for config_home in facts.dirs.config_homes() {
        ro(config_home.join("rtk"), &mut out);
    }
    // Every Claude root, the configured one and `~/.claude` (I-4, I-5).
    let protected = protected_paths(facts);
    for root in facts.dirs.claude_roots() {
        match placement(&root, home, &protected) {
            Placement::UnderHome => {
                for rel in CLAUDE_RO_REL {
                    ro(root.join(rel), &mut out);
                }
            }
            Placement::UnderTmp => {
                for rel in CLAUDE_RO_REL {
                    out.push(HomeBind::Ro(root.join(rel)));
                }
            }
            Placement::Maskable => {
                out.push(HomeBind::Tmpfs {
                    path: root.clone(),
                    size: CLAUDE_ROOT_TMPFS_SIZE,
                });
                for rel in CLAUDE_RO_REL {
                    out.push(HomeBind::Ro(root.join(rel)));
                }
            }
            Placement::Unmaskable => {}
        }
    }
    // The shim and the bridge execute one file: bind its tool root when it has a
    // `bin/` or `versions/` layout, else only the file (WR-04).
    ro(
        install_root(&facts.exe, facts).unwrap_or_else(|| facts.exe.clone()),
        &mut out,
    );
    for (path, size) in per_run_tmpfs(facts) {
        out.push(HomeBind::Tmpfs { path, size });
    }
    out
}

/// The `$HOME` entries of the sandbox, derived from `facts`.
///
/// Only paths in `facts.existing` are named (a bind of a missing path aborts bwrap);
/// a tmpfs under `$HOME` is always present, one elsewhere only when its path exists.
/// Parents come before children (by depth, then name), an entry nested in another
/// read-only or overlay entry is dropped, and a tool or tree outside `$HOME` is skipped:
/// it is visible through the read-only `/`. The exception is a Claude root outside both
/// `$HOME` and `/tmp`, which is an empty tmpfs with its GSD entries re-bound (I-5).
pub fn home_allowlist(facts: &HostFacts, overlay: bool) -> Vec<HomeBind> {
    let mut picked: Vec<HomeBind> = candidates(facts, overlay)
        .into_iter()
        .filter(|bind| match bind {
            HomeBind::Tmpfs { path, .. } => {
                path.starts_with(&facts.home) || facts.existing.contains(path)
            }
            _ => facts.existing.contains(bind.path()),
        })
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
    dangling_within(binds, None, &[])
}

/// [`dangling_symlinks`], ignoring targets outside `home` when given: those are
/// visible through the read-only `/` and do not dangle. The one-level `skills` scan
/// applies to `<claude root>/skills` for every one of `claude_roots` as well as to a
/// `skills` directory whose parent is named `.claude`.
fn dangling_within(
    binds: &[HomeBind],
    home: Option<&Path>,
    claude_roots: &[PathBuf],
) -> Vec<PathBuf> {
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
            && (dir
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|n| n == ".claude")
                || dir
                    .parent()
                    .is_some_and(|parent| claude_roots.iter().any(|root| root == parent)));
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
    use crate::launcher::host_dirs::HostDirs;
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
        /// The facts of this host, its relocatable directories resolved from the
        /// process environment. **The impure entry**; see [`HostFacts::from_host_with`].
        pub fn from_host(home: &Path, exe: &Path) -> HostFacts {
            HostFacts::from_host_with(&HostDirs::from_host(home), exe)
        }

        /// The facts of this host for the given `dirs`. **Reads the file system and
        /// `PATH`, `CARGO_HOME` and `RUSTUP_HOME`.**
        ///
        /// A `$HOME` that is itself a symlink is handled: resolved paths are
        /// re-expressed under the name the launcher knows `$HOME` by.
        pub fn from_host_with(dirs: &HostDirs, exe: &Path) -> HostFacts {
            let home = dirs.home.as_path();
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
                dirs: dirs.clone(),
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
            HomeSetup::for_host_with(&HostDirs::from_host(home), exe, overlay, state_dir)
        }

        /// [`for_host`](Self::for_host) for the given `dirs`: production and every
        /// hermetic test resolve a [`HostDirs`] once and pass it here.
        pub fn for_host_with(
            dirs: &HostDirs,
            exe: &Path,
            overlay: bool,
            state_dir: &Path,
        ) -> io::Result<HomeSetup> {
            let home = dirs.home.as_path();
            let facts = HostFacts::from_host_with(dirs, exe);
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
            let dangling = dangling_within(&binds, Some(home), &dirs.claude_roots());
            Ok(HomeSetup {
                binds,
                gitconfig: Some(gitconfig),
                keep_roots,
                dangling,
            })
        }
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
        assert!(dangling_within(&binds, Some(Path::new("/elsewhere")), &[]).is_empty());
    }

    #[test]
    fn for_host_writes_a_private_gitconfig_and_names_only_host_entries_as_roots() {
        let root = tempfile::TempDir::new().unwrap();
        let home = root.path().join("home");
        let state = root.path().join("state");
        fs::create_dir_all(home.join(".gsd")).unwrap();
        fs::create_dir_all(home.join(".ssh")).unwrap();
        fs::create_dir_all(&state).unwrap();
        let setup = HomeSetup::for_host_with(
            &HostDirs::resolve(&home, |_| None),
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
                let dirs = HostDirs::resolve(&home, |_| None);
                std::thread::spawn(move || {
                    HomeSetup::for_host_with(&dirs, Path::new("/nonexistent/x"), false, &state)
                        .unwrap()
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
        let facts = HostFacts::from_host_with(
            &HostDirs::resolve(&link, |_| None),
            &link.join("bin/gsd-meta-manager"),
        );
        assert_eq!(facts.exe, link.join("bin/gsd-meta-manager"));
        assert!(facts.existing.contains(&link.join(".gsd")));
        let got = home_allowlist(&facts, false);
        assert!(got.contains(&HomeBind::Ro(link.join(".gsd"))), "{got:?}");
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
            dirs: HostDirs::resolve(Path::new("/home/u"), |_| None),
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
        // WR-04: any other parent is not known to hold only this product.
        assert_eq!(
            install_root(Path::new("/home/u/tools/x/launcher"), &f),
            None
        );
    }

    #[test]
    fn a_binary_in_an_arbitrary_directory_binds_only_the_file() {
        // WR-04: `~/Downloads/node` must not expose the rest of `~/Downloads`.
        let mut f = facts();
        f.tools = vec![ToolHit {
            name: "node".to_owned(),
            hit: PathBuf::from("/home/u/Downloads/node"),
            real: PathBuf::from("/home/u/Downloads/node"),
        }];
        f.exe = PathBuf::from("/home/u/tools/x/gsd-meta-manager");
        f.existing = candidates(&f, true)
            .iter()
            .map(|bind| bind.path().to_owned())
            .collect();
        let got = paths(&home_allowlist(&f, true));
        for file in ["/home/u/Downloads/node", "/home/u/tools/x/gsd-meta-manager"] {
            assert!(got.contains(&format!("ro {file}")), "{got:?}");
        }
        for dir in ["/home/u/Downloads", "/home/u/tools/x", "/home/u/tools"] {
            assert!(!got.contains(&format!("ro {dir}")), "{got:?}");
        }
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
        let hostile_tools = |mut hostile: HostFacts| {
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
            hostile.tools.push(ToolHit {
                name: "gh2".to_owned(),
                hit: PathBuf::from("/home/u/cfg/gh/bin/gh"),
                real: PathBuf::from("/home/u/cfg/gh/bin/gh"),
            });
            hostile.exe = PathBuf::from("/home/u/.aws/gsd-meta-manager");
            hostile.existing = candidates(&hostile, true)
                .iter()
                .map(|bind| bind.path().to_owned())
                .collect();
            hostile
        };
        let relocated = relocated_facts(&[
            ("XDG_CONFIG_HOME", "/home/u/cfg"),
            ("XDG_DATA_HOME", "/home/u/data"),
            ("CLAUDE_CONFIG_DIR", "/home/u/profiles/work"),
        ]);
        for facts in [
            facts(),
            hostile_tools(facts()),
            relocated.clone(),
            hostile_tools(relocated),
        ] {
            for overlay in [true, false] {
                for bind in home_allowlist(&facts, overlay) {
                    for mask in facts.dirs.credential_dirs() {
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

    /// [`facts`] with the host directories resolved from `env`, every candidate existing.
    fn relocated_facts(env: &[(&str, &str)]) -> HostFacts {
        let mut f = facts();
        f.dirs = HostDirs::resolve(Path::new("/home/u"), |name| {
            env.iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| std::ffi::OsString::from(v))
        });
        f.existing = candidates(&f, true)
            .iter()
            .map(|bind| bind.path().to_owned())
            .collect();
        f
    }

    #[test]
    fn a_claude_root_under_home_binds_the_gsd_entries_of_both_roots() {
        let f = relocated_facts(&[("CLAUDE_CONFIG_DIR", "/home/u/profiles/work")]);
        let got = paths(&home_allowlist(&f, true));
        for rel in CLAUDE_RO_REL {
            for root in ["/home/u/.claude", "/home/u/profiles/work"] {
                assert!(
                    got.contains(&format!("ro {root}/{rel}")),
                    "{root}/{rel}: {got:?}"
                );
            }
        }
        assert!(!got.iter().any(|e| e.starts_with("tmpfs /home/u/profiles")));
    }

    #[test]
    fn a_relocated_claude_root_is_never_an_install_root() {
        let mut f = relocated_facts(&[("CLAUDE_CONFIG_DIR", "/home/u/profiles/work")]);
        f.tools = vec![ToolHit {
            name: "x".to_owned(),
            hit: PathBuf::from("/home/u/profiles/work/bin/x"),
            real: PathBuf::from("/home/u/profiles/work/bin/x"),
        }];
        f.existing = candidates(&f, true)
            .iter()
            .map(|bind| bind.path().to_owned())
            .collect();
        assert_eq!(
            install_root(Path::new("/home/u/profiles/work/bin/x"), &f),
            None
        );
        let got = paths(&home_allowlist(&f, true));
        assert!(
            got.contains(&"ro /home/u/profiles/work/bin/x".to_owned()),
            "{got:?}"
        );
        assert!(
            !got.contains(&"ro /home/u/profiles/work".to_owned()),
            "{got:?}"
        );
    }

    #[test]
    fn a_claude_root_outside_home_is_an_empty_tmpfs_with_its_gsd_entries_rebound() {
        let f = relocated_facts(&[("CLAUDE_CONFIG_DIR", "/srv/profiles/work")]);
        let got = paths(&home_allowlist(&f, true));
        let tmpfs = got
            .iter()
            .position(|e| e == "tmpfs /srv/profiles/work")
            .unwrap_or_else(|| panic!("no tmpfs over the profile: {got:?}"));
        let agents = got
            .iter()
            .position(|e| e == "ro /srv/profiles/work/agents")
            .unwrap_or_else(|| panic!("agents not re-bound: {got:?}"));
        assert!(tmpfs < agents, "the tmpfs precedes its re-binds: {got:?}");
        assert!(got.contains(&"ro /srv/profiles/work/hooks".to_owned()));
        // The default root is still bound.
        assert!(got.contains(&"ro /home/u/.claude/hooks".to_owned()));
        assert!(f
            .dirs
            .claude_roots()
            .iter()
            .all(|root| !root.starts_with("/home/u/.ssh")));
        // The tmpfs is named only when the root exists.
        let mut absent = f.clone();
        absent.existing.remove(Path::new("/srv/profiles/work"));
        let got = paths(&home_allowlist(&absent, true));
        assert!(
            !got.contains(&"tmpfs /srv/profiles/work".to_owned()),
            "{got:?}"
        );
    }

    #[test]
    fn a_claude_root_under_tmp_is_rebound_without_a_tmpfs() {
        let f = relocated_facts(&[("CLAUDE_CONFIG_DIR", "/tmp/profile")]);
        let got = paths(&home_allowlist(&f, true));
        assert!(
            got.contains(&"ro /tmp/profile/agents".to_owned()),
            "{got:?}"
        );
        assert!(!got.iter().any(|e| e.starts_with("tmpfs /tmp")), "{got:?}");
    }

    #[test]
    fn a_degenerate_claude_root_never_gets_a_tmpfs() {
        // `/`, an ancestor of `$HOME` and an ancestor of a tool or the binary.
        for root in ["/", "/home", "/home/u/.cargo/bin", "/home/u/.nvm/versions"] {
            let f = relocated_facts(&[("CLAUDE_CONFIG_DIR", root)]);
            for bind in home_allowlist(&f, true) {
                if let HomeBind::Tmpfs { path, .. } = &bind {
                    assert!(
                        path.starts_with("/home/u") && !f.dirs.claude_roots().contains(path),
                        "{root}: {bind:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn a_relocated_xdg_config_home_under_home_is_refused_and_masked() {
        let f = relocated_facts(&[("XDG_CONFIG_HOME", "/home/u/cfg")]);
        let got = paths(&home_allowlist(&f, true));
        assert!(got.contains(&"ro /home/u/cfg/rtk".to_owned()), "{got:?}");
        assert!(
            got.contains(&"ro /home/u/.config/rtk".to_owned()),
            "{got:?}"
        );
        // The whole directory is never an install root, and its gh store is a mask dir.
        assert_eq!(
            install_root(Path::new("/home/u/cfg/bin/x"), &f),
            None,
            "XDG_CONFIG_HOME is a refused root"
        );
        assert!(touches_mask_dir(Path::new("/home/u/cfg/gh/hosts.yml"), &f));
        assert!(touches_mask_dir(Path::new("/home/u/cfg"), &f));
        assert!(!touches_mask_dir(Path::new("/home/u/cfg/rtk"), &f));
    }

    #[test]
    fn a_relocated_cache_and_rtk_data_dir_get_a_per_run_tmpfs_next_to_the_defaults() {
        let f = relocated_facts(&[
            ("XDG_CACHE_HOME", "/home/u/xcache"),
            ("XDG_DATA_HOME", "/home/u/xdata"),
        ]);
        let got = paths(&home_allowlist(&f, true));
        for want in [
            "tmpfs /home/u/.cache",
            "tmpfs /home/u/.local/share/rtk",
            "tmpfs /home/u/xcache",
            "tmpfs /home/u/xdata/rtk",
        ] {
            assert!(got.contains(&want.to_owned()), "{want}: {got:?}");
        }
        // Outside `$HOME` a relocated cache is a tmpfs only when it exists.
        let mut outside = relocated_facts(&[("XDG_CACHE_HOME", "/srv/cache")]);
        outside.existing.remove(Path::new("/srv/cache"));
        assert!(!paths(&home_allowlist(&outside, true)).contains(&"tmpfs /srv/cache".to_owned()));
        outside.existing.insert(PathBuf::from("/srv/cache"));
        assert!(paths(&home_allowlist(&outside, true)).contains(&"tmpfs /srv/cache".to_owned()));
        // And is never an install root.
        assert_eq!(install_root(Path::new("/home/u/xcache/bin/x"), &f), None);
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
    fn an_exe_outside_a_bin_layout_binds_only_the_file() {
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
