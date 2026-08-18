// ============================================================================
// The `file://` bare-remote harness, shared by every envelope integration test
// (D-31).
//
// Extracted from `tests/envelope_tracer.rs` rather than copied into
// `tests/envelope_credential.rs`: two copies of a fixture is two things that
// can drift, and the property this one carries — a REAL remote, reached without
// a network, a credential or an agent — is exactly the property a drifted copy
// would quietly lose.
//
// `#![allow(dead_code)]` because cargo compiles this module into every
// integration target that declares it, and a helper one target does not happen
// to call is not dead code, it is a helper the other target uses.
// ============================================================================
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use gsd_meta_manager::envelope::hooks;
use tempfile::TempDir;

/// The binary under test, resolved by cargo for the integration target.
///
/// **Not `std::env::current_exe()`.** Under `cargo test` that is the test
/// binary, which has no `envelope` subcommand — a stub generated from it would
/// exit non-zero for a reason that has nothing to do with the policy, and every
/// refusal assertion would pass vacuously.
pub const BIN: &str = env!("CARGO_BIN_EXE_gsd-meta-manager");

pub struct Fixture {
    /// Held for its Drop; every path below lives inside it.
    _tmp: TempDir,
    /// The work repository a push is issued from.
    pub work: PathBuf,
    /// The bare repository standing in for the remote.
    pub bare: PathBuf,
    /// The envelope root, outside both repositories (D-02).
    pub envelope_root: PathBuf,
    /// The directory `core.hooksPath` is pointed at.
    pub hooks_dir: PathBuf,
    pub alias: String,
}

impl Fixture {
    pub fn inside_ref(&self) -> String {
        format!("refs/heads/gsd-auto/{}/tracer", self.alias)
    }

    /// A ref inside the reserved namespace, named by the caller.
    pub fn inside_ref_named(&self, leaf: &str) -> String {
        format!("refs/heads/gsd-auto/{}/{leaf}", self.alias)
    }

    /// The temp directory every fixture path hangs from.
    pub fn tmp(&self) -> &Path {
        self._tmp.path()
    }
}

pub fn git(dir: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .ok()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

/// A work repository with one commit, a `file://` bare remote, and an installed
/// envelope — or `None` when the sandbox forbids `git init`, so the tests skip
/// gracefully rather than failing for a reason that is not about the code.
pub fn fixture(alias: &str) -> Option<Fixture> {
    let tmp = TempDir::new().ok()?;
    let bare = tmp.path().join("remote.git");
    let work = tmp.path().join("work");
    let envelope_root = tmp.path().join("envelope");
    std::fs::create_dir_all(&bare).ok()?;
    std::fs::create_dir_all(&work).ok()?;

    if !git(&bare, &["init", "--bare", "--quiet"]) {
        return None;
    }
    if !git(&work, &["init", "--quiet"]) {
        return None;
    }
    // Repo-scoped identity, so the test neither depends on nor disturbs a
    // developer's global git configuration.
    git(&work, &["config", "user.email", "test@example.com"]);
    git(&work, &["config", "user.name", "Test User"]);
    git(&work, &["config", "commit.gpgsign", "false"]);

    std::fs::write(work.join("tracked.txt"), "one\n").ok()?;
    if !git(&work, &["add", "tracked.txt"]) {
        return None;
    }
    if !git(&work, &["commit", "-m", "initial commit", "--quiet"]) {
        return None;
    }
    let url = format!("file://{}", bare.display());
    if !git(&work, &["remote", "add", "origin", &url]) {
        return None;
    }

    let hooks_dir = hooks::install_in(&envelope_root, alias, Path::new(BIN))
        .expect("installing a hook stub for a plain alias succeeds");

    Some(Fixture {
        _tmp: tmp,
        work,
        bare,
        envelope_root,
        hooks_dir,
        alias: alias.to_string(),
    })
}

/// The remote's refs, read from the bare repository itself.
pub fn remote_refs(fx: &Fixture) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(&fx.bare)
        .args(["for-each-ref", "--format=%(refname)"])
        .output()
        .expect("git for-each-ref is runnable");
    String::from_utf8_lossy(&out.stdout).into_owned()
}
