//! Hook stub generation, and the hook body the stub re-enters this binary for.
//!
//! **The generated file carries no policy logic** (D-04). It is a shebang, a
//! comment and one `exec` of the absolute binary path captured from
//! [`std::env::current_exe`] at generation time. Three consequences, and each is
//! why the obvious "generate a shell script that checks the refs" is declined:
//!
//! 1. Policy lives in Rust, where [`super::policy`] is unit-testable against
//!    hostile inputs with no repository and no child process.
//! 2. One binary and one version means the TUI, the driver and the hook can
//!    never disagree about what the policy is.
//! 3. A shell script full of `case` statements is precisely the artifact an
//!    agent rewrites — and a rewritten three-line stub that no longer execs this
//!    binary is a stub that does nothing, which the pushing side notices,
//!    whereas a rewritten `case` arm is invisible.
//!
//! Nothing here writes into the driven repository. The stub is delivered to git
//! through the environment instead; see [`super::cred::hooks_path_env`].

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use tempfile::NamedTempFile;

use super::policy::{self, PushVerdict};

/// The hook filename git looks for, and the subdirectory it looks in.
pub const PRE_PUSH_HOOK: &str = "pre-push";
const HOOKS_SUBDIR: &str = "hooks";

/// Install the `pre-push` stub for `alias`, returning the hooks **directory**.
///
/// The directory rather than the file, because the directory is what
/// [`super::cred::hooks_path_env`] hands to git as `core.hooksPath`.
pub fn install(alias: &str) -> anyhow::Result<PathBuf> {
    let binary = std::env::current_exe()
        .context("cannot resolve this binary's own path, so no hook stub can name it")?;
    let root = super::envelope_root().ok_or_else(|| {
        anyhow!(
            "no application data directory is resolvable, and the envelope refuses \
             to fall back to a directory that could sit inside a repository"
        )
    })?;
    install_in(&root, alias, &binary)
}

/// [`install`] against an explicit envelope root and binary path.
///
/// Split out so the end-to-end fixture can install into a temporary directory
/// and name the built binary explicitly — under `cargo test`,
/// [`std::env::current_exe`] is the *test* binary, which would produce a stub
/// that execs something with no `envelope` subcommand.
pub fn install_in(root: &Path, alias: &str, binary: &Path) -> anyhow::Result<PathBuf> {
    let dir = super::envelope_dir_in(root, alias).ok_or_else(|| {
        anyhow!("refusing to install a hook for alias {alias:?}: not a plain path component")
    })?;
    let hooks_dir = dir.join(HOOKS_SUBDIR);
    std::fs::create_dir_all(&hooks_dir)
        .with_context(|| format!("failed to create {}", hooks_dir.display()))?;

    // The `config.rs:234-249` atomic-write idiom: a temp file in the *target*
    // directory, then `persist`. A half-written hook is a hook git would exec.
    let mut tmp = NamedTempFile::new_in(&hooks_dir)
        .with_context(|| format!("failed to create a temp file in {}", hooks_dir.display()))?;
    tmp.write_all(stub_body(binary, alias).as_bytes())
        .with_context(|| format!("failed to write the hook stub in {}", hooks_dir.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o755))
            .with_context(|| {
                format!("failed to make the hook stub executable in {}", hooks_dir.display())
            })?;
    }

    let hook = hooks_dir.join(PRE_PUSH_HOOK);
    tmp.persist(&hook)
        .with_context(|| format!("failed to persist the hook stub to {}", hook.display()))?;

    Ok(hooks_dir)
}

/// The three lines git will exec.
///
/// Both interpolated values are POSIX-quoted by [`sh_quote`] rather than wrapped
/// in double quotes. An alias is a *plain path component*, which is a weaker
/// constraint than "shell-safe" — `is_plain_run_id` accepts a quote character —
/// and a generated script is not a place to discover that difference.
///
/// `--hook-path "$0"` is the one value that is deliberately **not** baked in.
/// A path recorded at generation time travels with a copy of the file, so a
/// relocated stub would hand [`assert_provenance`] the original's path and
/// certify itself. `$0` is the path the shell was actually invoked as, which is
/// the only value a copy cannot forge by being copied.
fn stub_body(binary: &Path, alias: &str) -> String {
    format!(
        "#!/bin/sh\n\
         # gsd-meta-manager envelope hook. Policy lives in the binary below (D-04).\n\
         exec {} envelope pre-push {} --hook-path \"$0\"\n",
        sh_quote(&binary.to_string_lossy()),
        sh_quote(alias),
    )
}

/// POSIX single-quote `value` so no character in it can reach the shell.
fn sh_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// Refuse unless this hook is the sanctioned one, running the binary it names.
///
/// Two facts, and the hook refuses if either fails (D-10):
///
/// 1. **The binary the stub was generated with still exists on disk.** On Linux
///    a running-but-deleted executable reports its path as `… (deleted)`, so a
///    binary replaced or removed under a live envelope is detectable here rather
///    than at the next exec.
/// 2. **The file this process was invoked from is the one inside *this alias's*
///    envelope hooks directory.** A copy of the hook relocated to some other
///    `hooksPath` must not silently become the sanctioned one.
///
/// Both sides are canonicalised before comparison, so a symlinked data directory
/// — `~/.local/share` pointed elsewhere is ordinary — does not read as a
/// relocation. Canonicalisation is safe *here* in a way it is not in
/// [`crate::journal::is_plain_run_id`]: this compares two paths that must both
/// already exist, rather than deciding whether a path that does not exist yet is
/// allowed to be created.
pub fn assert_provenance(alias: &str, invoked_from: &Path) -> anyhow::Result<()> {
    let root = super::envelope_root().ok_or_else(|| {
        anyhow!("no application data directory is resolvable, so no hook can be sanctioned")
    })?;
    assert_provenance_in(&root, alias, invoked_from)
}

/// [`assert_provenance`] against an explicit envelope root.
pub fn assert_provenance_in(root: &Path, alias: &str, invoked_from: &Path) -> anyhow::Result<()> {
    let binary = std::env::current_exe()
        .context("cannot resolve this binary's own path, so its provenance is unknown")?;
    if !binary.exists() {
        return Err(anyhow!(
            "the envelope binary at {} no longer exists on disk; refusing to act \
             as a hook for a build that has been removed or replaced",
            binary.display()
        ));
    }

    let sanctioned = super::envelope_dir_in(root, alias)
        .map(|dir| dir.join(HOOKS_SUBDIR).join(PRE_PUSH_HOOK))
        .ok_or_else(|| {
            anyhow!("alias {alias:?} is not a plain path component, so it sanctions no hook")
        })?;

    let sanctioned_real = sanctioned.canonicalize().with_context(|| {
        format!(
            "no sanctioned hook exists at {}, so nothing can be certified against it",
            sanctioned.display()
        )
    })?;
    let invoked_real = invoked_from.canonicalize().with_context(|| {
        format!("cannot resolve the hook this process ran from ({})", invoked_from.display())
    })?;

    if invoked_real != sanctioned_real {
        return Err(anyhow!(
            "hook provenance failed: this process ran from {} but the sanctioned \
             hook for alias {alias:?} is {}; a relocated copy does not inherit the \
             envelope's authority",
            invoked_real.display(),
            sanctioned_real.display()
        ));
    }

    Ok(())
}

/// The `pre-push` hook body: assert provenance, then classify.
///
/// The order is the mechanism. Classification runs only for a hook that has
/// proved it is the sanctioned one, so a relocated copy cannot decide anything —
/// not even to allow.
pub fn pre_push(
    alias: &str,
    stdin: impl BufRead,
    invoked_from: &Path,
) -> anyhow::Result<i32> {
    assert_provenance(alias, invoked_from)?;
    classify_refs(alias, stdin)
}

/// Read git's ref lines and return the exit code, with no provenance check.
///
/// Separate from [`pre_push`] so the classification can be unit-tested against
/// hostile stdin without installing a hook on disk first. Nothing outside this
/// module may call it: a caller that skipped [`assert_provenance`] would be
/// re-opening exactly the seam D-10 closes.
///
/// git supplies `<local-ref> <local-sha> <remote-ref> <remote-sha>` on stdin,
/// **regardless of how git was invoked** — from a nested shell, from a Makefile,
/// from a script the agent wrote. That is why this is the layer that observes
/// ground truth rather than an argv the tool boundary happened to see.
///
/// Two behaviours that look like details and are not:
///
/// - A **deletion** line (an all-zero local sha) is still classified by its
///   destination ref. Deleting `main` is a write to `main`.
/// - An **unparseable** line is refused, never skipped. A hook that cannot read
///   its input must not allow; skipping would make a malformed line the cheapest
///   possible bypass.
///
/// The return value is the process exit code, and the exit code **is** the
/// control (D-25) — not the message, which is only there so a human reading a
/// failed push knows what happened.
fn classify_refs(alias: &str, stdin: impl BufRead) -> anyhow::Result<i32> {
    let namespace = policy::default_namespace(alias);
    let mut refused = 0usize;

    for (index, line) in stdin.lines().enumerate() {
        let line = line
            .with_context(|| format!("failed to read pre-push ref line {}", index + 1))?;
        // A line with no tokens carries no ref; it is nothing, not something
        // unreadable. Anything else with the wrong field count is refused below.
        if line.trim().is_empty() {
            continue;
        }

        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() != 4 {
            eprintln!(
                "gsd-meta-manager envelope: REFUSED — pre-push line {} is unreadable \
                 ({} fields, expected 4) (reason: {})",
                index + 1,
                fields.len(),
                policy::REASON_ENVELOPE_ASSERTION_FAILED,
            );
            refused += 1;
            continue;
        }

        match policy::classify_push_ref(fields[2], &namespace) {
            PushVerdict::Allow => {}
            PushVerdict::Refuse { reason, ref_name } => {
                eprintln!(
                    "gsd-meta-manager envelope: REFUSED push to {ref_name} \
                     (reason: {reason}); a driven run may push only inside {namespace}",
                );
                refused += 1;
            }
        }
    }

    Ok(if refused > 0 { 1 } else { 0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BIN: &str = "/opt/gsd-meta-manager";

    #[test]
    fn the_generated_stub_is_three_lines_and_execs_the_absolute_binary_path() {
        let body = stub_body(Path::new(BIN), "demo");
        assert_eq!(body.lines().count(), 3, "the stub must stay a stub:\n{body}");
        assert!(body.starts_with("#!/bin/sh\n"));
        assert!(body.ends_with(
            "exec '/opt/gsd-meta-manager' envelope pre-push 'demo' --hook-path \"$0\"\n"
        ));
    }

    #[test]
    fn a_quote_in_an_alias_cannot_escape_the_generated_stub() {
        let body = stub_body(Path::new(BIN), "de'mo");
        assert!(
            body.contains(r"'de'\''mo'"),
            "a quote must be POSIX-escaped, not interpolated raw:\n{body}"
        );
    }

    #[test]
    fn a_push_inside_the_namespace_exits_zero() {
        let stdin = "refs/heads/x abc refs/heads/gsd-auto/demo/x def\n";
        assert_eq!(classify_refs("demo", stdin.as_bytes()).unwrap(), 0);
    }

    #[test]
    fn a_push_outside_the_namespace_exits_non_zero() {
        let stdin = "refs/heads/x abc refs/heads/main def\n";
        assert_ne!(classify_refs("demo", stdin.as_bytes()).unwrap(), 0);
    }

    #[test]
    fn one_refused_ref_in_a_batch_refuses_the_whole_push() {
        let stdin = "refs/heads/a 1 refs/heads/gsd-auto/demo/a 2\n\
                     refs/heads/b 3 refs/heads/main 4\n";
        assert_ne!(classify_refs("demo", stdin.as_bytes()).unwrap(), 0);
    }

    #[test]
    fn a_deletion_is_classified_by_its_destination_ref() {
        let deleting_main =
            "(delete) 0000000000000000000000000000000000000000 refs/heads/main 5\n";
        assert_ne!(classify_refs("demo", deleting_main.as_bytes()).unwrap(), 0);
    }

    #[test]
    fn an_unreadable_ref_line_is_refused_rather_than_skipped() {
        assert_ne!(classify_refs("demo", "garbage\n".as_bytes()).unwrap(), 0);
        assert_ne!(
            classify_refs("demo", "one two three\n".as_bytes()).unwrap(),
            0,
            "a three-field line has no destination ref and must not be allowed"
        );
    }

    #[test]
    fn a_blank_line_is_nothing_rather_than_something_unreadable() {
        assert_eq!(classify_refs("demo", "\n\n".as_bytes()).unwrap(), 0);
    }

    #[test]
    fn a_hostile_alias_is_refused_before_any_directory_is_created() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = install_in(tmp.path(), "../escaped", Path::new(BIN)).unwrap_err();
        assert!(
            err.to_string().contains("plain path component"),
            "the refusal must name why: {err}"
        );
        assert_eq!(
            std::fs::read_dir(tmp.path()).unwrap().count(),
            0,
            "a refused alias must leave the envelope root untouched"
        );
    }

    #[test]
    fn the_installed_hook_certifies_itself() {
        let tmp = tempfile::TempDir::new().unwrap();
        let hooks_dir = install_in(tmp.path(), "demo", Path::new(BIN)).unwrap();
        assert_provenance_in(tmp.path(), "demo", &hooks_dir.join(PRE_PUSH_HOOK))
            .expect("the hook inside its own envelope directory is the sanctioned one");
    }

    #[test]
    fn a_copy_of_the_hook_outside_the_envelope_certifies_nothing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let hooks_dir = install_in(tmp.path(), "demo", Path::new(BIN)).unwrap();

        let elsewhere = tmp.path().join("elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        let copy = elsewhere.join(PRE_PUSH_HOOK);
        std::fs::copy(hooks_dir.join(PRE_PUSH_HOOK), &copy).unwrap();

        let err = assert_provenance_in(tmp.path(), "demo", &copy).unwrap_err();
        assert!(
            err.to_string().contains("provenance"),
            "the refusal must name provenance, not look like a policy verdict: {err}"
        );
    }

    #[test]
    fn a_hook_for_an_alias_with_no_installed_envelope_certifies_nothing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let hooks_dir = install_in(tmp.path(), "demo", Path::new(BIN)).unwrap();
        // Same file, asked to certify itself as some OTHER alias's hook: the
        // shape an agent reaches for when it wants one envelope's authority to
        // cover a project the user never opted in.
        let err =
            assert_provenance_in(tmp.path(), "other", &hooks_dir.join(PRE_PUSH_HOOK)).unwrap_err();
        assert!(
            err.to_string().contains("sanctioned hook"),
            "the refusal must say no sanctioned hook exists for that alias: {err}"
        );
    }

    #[test]
    fn a_hostile_alias_sanctions_no_hook_at_all() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = assert_provenance_in(tmp.path(), "../escaped", Path::new("/bin/sh")).unwrap_err();
        assert!(err.to_string().contains("plain path component"), "{err}");
    }
}
