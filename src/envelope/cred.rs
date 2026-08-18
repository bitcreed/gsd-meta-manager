//! The git configuration and credential posture the envelope injects through
//! the **environment**.
//!
//! Git reads `GIT_CONFIG_COUNT` / `GIT_CONFIG_KEY_n` / `GIT_CONFIG_VALUE_n`
//! (since 2.31; this machine has 2.43) at the same precedence as `-c`, which is
//! **above repo-local config**, and every `git` invocation in the child's
//! process tree inherits it — including ones an agent makes from a nested shell
//! or from a script it wrote.
//!
//! ## The scrub is not new; its scope is (D-16)
//!
//! `src/executor/claude.rs` already scrubs every inherited `CLAUDE*` variable
//! before spawning the agent, on the argument that a TUI launched from inside a
//! Claude Code session would otherwise leak the parent session's configuration
//! into the driven child. [`build_env`] is the **second half of that same
//! argument**, extended from the agent's own variable family to git's and ssh's:
//! a TUI launched from a shell with an ssh agent socket, a keychain-backed
//! credential helper and a `~/.gitconfig` would otherwise hand all three to a
//! child an untrusted agent drives.
//!
//! The difference from the `CLAUDE*` scrub is that this one **removes** rather
//! than overwrites where removal is the point. `SSH_AUTH_SOCK` set to an empty
//! string is still a variable an agent can notice and work around; an absent one
//! is absent. That distinction is carried in the type — see [`EnvelopeEnv`].
//!
//! ## The honest limit
//!
//! An agent that unsets these variables in a subshell **escapes this layer**.
//! Nothing here is a guarantee: it makes the user's ambient identity unreachable
//! to a cooperating process tree, which is a real and useful boundary, and it is
//! not a boundary against a process that is trying to leave. The only boundaries
//! that do not depend on the agent's cooperation are the remote's own ruleset
//! and the scope of the credential, which is why D-27's server-side branch
//! protection recommendation is this phase's conclusion rather than its
//! footnote.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use tempfile::NamedTempFile;

/// The run-journal locator carried into every hook and guard re-entry (D-24,
/// D-25).
pub const PROJECT_ROOT_ENV: &str = "GSD_MM_ENVELOPE_PROJECT_ROOT";

/// The filename of the envelope-generated git config, inside the alias's
/// envelope directory.
const GITCONFIG_FILE: &str = "gitconfig";

/// The ssh invocation that offers no identity, consults no agent, reads no user
/// configuration and cannot prompt (D-16).
///
/// Each option closes one reachability path, and none of them is decoration:
///
/// * `IdentitiesOnly=yes` — ssh offers only keys named on the command line, so
///   a default `~/.ssh/id_*` is never tried.
/// * `IdentityAgent=none` — no agent is consulted even if a socket path is
///   somehow reachable, which makes the `SSH_AUTH_SOCK` removal belt *and*
///   braces rather than a single point of failure.
/// * `BatchMode=yes` — no passphrase prompt, no host-key confirmation; ssh
///   fails instead of waiting.
/// * `-F /dev/null` — no `~/.ssh/config`, so a `Host *` block carrying an
///   `IdentityFile` cannot put a key back.
pub const ENVELOPE_SSH_COMMAND: &str =
    "ssh -o IdentitiesOnly=yes -o IdentityAgent=none -o BatchMode=yes -F /dev/null";

/// One child-environment instruction: **remove** the variable (`None`) or
/// **set** it (`Some`).
pub type EnvelopeVar = (OsString, Option<OsString>);

/// The child's environment as a value, so it can be asserted on without
/// spawning anything (D-32).
///
/// **The `Option` is the design, not an implementation detail.** D-16 turns on
/// removal rather than overwriting: `SSH_AUTH_SOCK=""` is still a variable an
/// agent can notice and work around, while an absent one is absent. Encoding
/// that distinction in the type means a caller applying this to a `Command`
/// cannot collapse the two by accident — it has to match on the `Option` and
/// call `env_remove` or `env` accordingly.
///
/// Returning a value rather than mutating a `Command` is the same factoring
/// `executor::claude::build_argv` uses, and for the same reason: it makes
/// "assert on the constructed child environment" a unit test with no process in
/// it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeEnv(Vec<EnvelopeVar>);

impl EnvelopeEnv {
    /// Every instruction, in the order a caller should apply them.
    pub fn entries(&self) -> &[EnvelopeVar] {
        &self.0
    }

    /// The instruction for `key`, or `None` when the envelope says nothing
    /// about it.
    ///
    /// Note the two nested `Option`s and that they mean different things:
    /// `None` is "not mentioned", `Some(None)` is "explicitly removed".
    pub fn get(&self, key: &str) -> Option<&Option<OsString>> {
        self.0
            .iter()
            .find(|(name, _)| name.as_os_str() == OsStr::new(key))
            .map(|(_, value)| value)
    }

    /// The value `key` is **set** to, or `None` when it is removed or absent.
    pub fn value(&self, key: &str) -> Option<&OsStr> {
        match self.get(key) {
            Some(Some(value)) => Some(value.as_os_str()),
            _ => None,
        }
    }

    /// Whether `key` carries the removal marker, as opposed to any value at all.
    pub fn is_removed(&self, key: &str) -> bool {
        matches!(self.get(key), Some(None))
    }
}

/// `GIT_CONFIG_COUNT` / `KEY_n` / `VALUE_n` for an arbitrary set of keys.
///
/// **The count is derived from the pairs, never written by hand.** A count that
/// disagrees with the keys makes git read a `GIT_CONFIG_KEY_n` that is not
/// there, and git's response is to ignore the injection *entirely* — silently.
/// For a module whose whole job is delivering configuration an agent cannot
/// remove, a hardcoded `1` that a later key addition forgets to bump is the
/// failure that costs the most and shows the least.
fn config_env(pairs: &[(&str, &OsStr)]) -> Vec<(OsString, OsString)> {
    let mut out = Vec::with_capacity(pairs.len() * 2 + 1);
    out.push((
        OsString::from("GIT_CONFIG_COUNT"),
        OsString::from(pairs.len().to_string()),
    ));
    for (index, (key, value)) in pairs.iter().enumerate() {
        out.push((
            OsString::from(format!("GIT_CONFIG_KEY_{index}")),
            OsString::from(key),
        ));
        out.push((
            OsString::from(format!("GIT_CONFIG_VALUE_{index}")),
            (*value).to_owned(),
        ));
    }
    out
}

/// The env-injected `core.hooksPath` triplet pointing git at `hooks_dir`.
///
/// ## Four things this buys over installing into the repository's hooks
/// directory, which is why the obvious approach is declined (D-09)
///
/// 1. **The user's own git commands in that repository are unaffected.** A
///    `pre-push` hook installed into `.git/hooks/` fires for the human too,
///    which would make this tool's safety envelope silently police the user's
///    own pushes. That is a bug, not a feature.
/// 2. **Nothing is left behind** when a run crashes, is SIGKILLed, or the TUI is
///    closed — the same posture as "crash reconciliation performs zero disk
///    writes".
/// 3. **The agent cannot uninstall it by editing a file in the repository**,
///    because there is no file in the repository.
/// 4. **Repo-local `core.hooksPath` cannot override it**, because env-injected
///    configuration outranks repository configuration.
///
/// ## The limit, stated rather than implied
///
/// An agent that unsets `GIT_CONFIG_COUNT` in a subshell **escapes this layer**.
/// That is not a hole this project can close client-side, and the one form that
/// outranks this injection is `git -c core.hooksPath=… push` — which plan 19-02
/// denies **by name** at the tool boundary for exactly that reason. Neither the
/// denial nor this injection is a guarantee; server-side branch protection is
/// the only boundary that does not depend on the agent's cooperation.
pub fn hooks_path_env(hooks_dir: &Path) -> Vec<(OsString, OsString)> {
    config_env(&[("core.hooksPath", hooks_dir.as_os_str())])
}

/// Write `<envelope_dir>/gitconfig` for `alias`, returning its path.
///
/// The file carries **only** `user.name` and `user.email`, copied from the
/// user's resolved configuration so a driven run's commits are attributable and
/// do not fail. It carries **no credential helper of any kind**, and that
/// absence is the point of the whole function: pointed at by both
/// `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM`, it puts all three of the user's
/// helper routes out of reach —
///
/// 1. the **OS keychain** helper (`credential.helper = osxkeychain`,
///    `libsecret`, `wincred`),
/// 2. the plaintext **credential store** (`credential.helper = store`, reading
///    `~/.git-credentials`),
/// 3. the **external GitHub client's** helper (`credential.helper = !gh auth
///    git-credential`), which `GH_CONFIG_DIR` closes from the other side.
///
/// — because git resolves `credential.helper` from system and global config,
/// and there is now no system or global config that mentions one.
pub fn write_gitconfig(alias: &str, name: &str, email: &str) -> anyhow::Result<PathBuf> {
    let root = super::envelope_root().ok_or_else(|| {
        anyhow!(
            "no application data directory is resolvable, and the envelope refuses \
             to fall back to a directory that could sit inside a repository"
        )
    })?;
    write_gitconfig_in(&root, alias, name, email)
}

/// [`write_gitconfig`] against an explicit envelope root.
///
/// The `X` / `X_in` pair this module tree established in plan 19-01, for the
/// same reason: a test may not write into the developer's real
/// `~/.local/share`.
pub fn write_gitconfig_in(
    root: &Path,
    alias: &str,
    name: &str,
    email: &str,
) -> anyhow::Result<PathBuf> {
    let dir = super::envelope_dir_in(root, alias).ok_or_else(|| {
        anyhow!("refusing to generate a git config for alias {alias:?}: not a plain path component")
    })?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create {}", dir.display()))?;

    let path = dir.join(GITCONFIG_FILE);
    // The `config.rs` atomic-write idiom: a temp file in the *target* directory,
    // then `persist`. A half-written config is a config git would read.
    let mut tmp = NamedTempFile::new_in(&dir)
        .with_context(|| format!("failed to create a temp file in {}", dir.display()))?;
    // RED: the identity is not copied through yet — that lands in the GREEN
    // step. An empty identity is the "config git resolves nothing from" case, so
    // the assertion below fails against it rather than passing vacuously.
    let _ = (name, email);
    tmp.write_all(gitconfig_body("", "").as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))?;
    tmp.persist(&path)
        .with_context(|| format!("failed to persist the generated config to {}", path.display()))?;
    Ok(path)
}

/// The generated config's bytes.
///
/// Newlines in a value would let a caller inject a second section, so both
/// values are flattened first. `user.name` reaching here is the user's own
/// configured name, not agent input — but a generated configuration file is not
/// a place to rely on that, for the same reason `hooks::sh_quote` exists.
fn gitconfig_body(name: &str, email: &str) -> String {
    format!(
        "# Generated by gsd-meta-manager for one driven run (D-16).\n\
         # Identity only. No credential helper is named here, and that absence is\n\
         # what puts the user's keychain, credential store and gh helper out of\n\
         # this run's reach.\n\
         [user]\n\
         \tname = {}\n\
         \temail = {}\n",
        flatten(name),
        flatten(email),
    )
}

/// One line, whatever came in.
fn flatten(value: &str) -> String {
    value.replace(['\n', '\r'], " ").trim().to_string()
}

/// The child's whole environment for a driven run of `alias` in `project_root`.
///
/// See the module doc for what this is and what it is not. Every entry below
/// carries its reason at the line.
pub fn build_env(alias: &str, project_root: &Path) -> anyhow::Result<EnvelopeEnv> {
    let root = super::envelope_root().ok_or_else(|| {
        anyhow!(
            "no application data directory is resolvable, and the envelope refuses \
             to fall back to a directory that could sit inside a repository"
        )
    })?;
    build_env_in(&root, alias, project_root)
}

/// [`build_env`] against an explicit envelope root.
pub fn build_env_in(root: &Path, alias: &str, project_root: &Path) -> anyhow::Result<EnvelopeEnv> {
    let _ = (root, alias, project_root);
    // RED: the real builder lands in the GREEN step. An empty environment is the
    // "envelope that does nothing" this plan exists to make impossible, so every
    // assertion below fails against it — which is what proves each one is
    // load-bearing (the pattern plan 19-02 established).
    Ok(EnvelopeEnv(Vec::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const ALIAS: &str = "demo";

    /// An envelope root and a project root, both inside one temp directory.
    fn roots() -> (TempDir, PathBuf, PathBuf) {
        let tmp = TempDir::new().expect("a temp directory");
        let envelope = tmp.path().join("envelope");
        let project = tmp.path().join("project");
        std::fs::create_dir_all(&envelope).expect("the envelope root");
        std::fs::create_dir_all(&project).expect("the project root");
        (tmp, envelope, project)
    }

    #[test]
    fn the_triplet_names_the_count_the_key_and_the_directory() {
        let env = hooks_path_env(Path::new("/data/envelope/demo/hooks"));
        assert_eq!(
            env,
            vec![
                (OsString::from("GIT_CONFIG_COUNT"), OsString::from("1")),
                (
                    OsString::from("GIT_CONFIG_KEY_0"),
                    OsString::from("core.hooksPath")
                ),
                (
                    OsString::from("GIT_CONFIG_VALUE_0"),
                    OsString::from("/data/envelope/demo/hooks")
                ),
            ]
        );
    }

    #[test]
    fn the_count_matches_the_number_of_key_value_pairs() {
        // A count that disagrees with the pairs makes git read a key that is not
        // there, and git's response to that is to ignore the whole injection.
        let env = hooks_path_env(Path::new("/x"));
        let count: usize = env[0].1.to_string_lossy().parse().unwrap();
        assert_eq!(count * 2 + 1, env.len());
    }

    #[test]
    fn the_ambient_ssh_agent_is_removed_rather_than_overwritten() {
        let (_tmp, root, project) = roots();
        let env = build_env_in(&root, ALIAS, &project).expect("a plain alias builds an env");

        for key in ["SSH_AUTH_SOCK", "SSH_AGENT_PID"] {
            assert!(
                env.get(key).is_some(),
                "{key} is not mentioned at all, so the child inherits the user's agent"
            );
            assert!(
                env.is_removed(key),
                "{key} carries a value rather than the removal marker; an empty value \
                 is still a variable an agent can notice and work around (D-16)"
            );
        }
    }

    #[test]
    fn the_ssh_command_offers_no_identity_no_agent_and_no_prompt() {
        let (_tmp, root, project) = roots();
        let env = build_env_in(&root, ALIAS, &project).expect("a plain alias builds an env");

        let command = env
            .value("GIT_SSH_COMMAND")
            .expect("GIT_SSH_COMMAND must be set")
            .to_string_lossy()
            .into_owned();
        for option in [
            "IdentitiesOnly=yes",
            "IdentityAgent=none",
            "BatchMode=yes",
            "-F /dev/null",
        ] {
            assert!(
                command.contains(option),
                "GIT_SSH_COMMAND is missing {option:?}: {command}"
            );
        }
    }

    #[test]
    fn the_config_and_gh_directories_point_inside_this_aliass_envelope() {
        let (_tmp, root, project) = roots();
        let env = build_env_in(&root, ALIAS, &project).expect("a plain alias builds an env");
        let dir = super::super::envelope_dir_in(&root, ALIAS).expect("a plain alias");

        for key in ["GIT_CONFIG_GLOBAL", "GIT_CONFIG_SYSTEM", "GH_CONFIG_DIR"] {
            let value = env
                .value(key)
                .unwrap_or_else(|| panic!("{key} must be set"));
            assert!(
                Path::new(value).starts_with(&dir),
                "{key} points at {value:?}, outside {}",
                dir.display()
            );
        }
    }

    #[test]
    fn terminal_prompts_are_disabled_because_a_detached_run_has_no_terminal() {
        let (_tmp, root, project) = roots();
        let env = build_env_in(&root, ALIAS, &project).expect("a plain alias builds an env");
        assert_eq!(
            env.value("GIT_TERMINAL_PROMPT"),
            Some(OsStr::new("0")),
            "a prompt into a null stdio is not a prompt, it is a hang the idle cap \
             kills hours later (D-16)"
        );
    }

    #[test]
    fn the_injected_config_count_equals_the_number_of_keys_present() {
        let (_tmp, root, project) = roots();
        let env = build_env_in(&root, ALIAS, &project).expect("a plain alias builds an env");

        let count: usize = env
            .value("GIT_CONFIG_COUNT")
            .expect("GIT_CONFIG_COUNT must be set")
            .to_string_lossy()
            .parse()
            .expect("GIT_CONFIG_COUNT is a number");
        let keys = env
            .entries()
            .iter()
            .filter(|(name, _)| name.to_string_lossy().starts_with("GIT_CONFIG_KEY_"))
            .count();
        assert_eq!(
            count, keys,
            "a count that disagrees with the keys makes git drop the whole injection"
        );
        assert!(count >= 1, "the injection carries no keys at all");
    }

    #[test]
    fn the_run_journal_locator_carries_the_canonicalized_project_root() {
        let (_tmp, root, project) = roots();
        let env = build_env_in(&root, ALIAS, &project).expect("a plain alias builds an env");

        let canonical = std::fs::canonicalize(&project).expect("the project root canonicalizes");
        assert_eq!(
            env.value(PROJECT_ROOT_ENV),
            Some(canonical.as_os_str()),
            "a hook or guard re-entry cannot find the run journal it has to park \
             without this (D-24, D-25)"
        );
    }

    #[test]
    fn the_generated_config_carries_an_identity_and_no_credential_helper() {
        let (_tmp, root, _project) = roots();
        let path = write_gitconfig_in(&root, ALIAS, "Test User", "test@example.com")
            .expect("a plain alias generates a config");
        let body = std::fs::read_to_string(&path).expect("the generated config is readable");

        assert!(
            body.contains("test@example.com") && body.contains("Test User"),
            "the generated config must carry the identity, or a driven commit \
             fails with \"Please tell me who you are\":\n{body}"
        );
        // Comment lines are excluded on purpose: the generated file explains in
        // prose why no helper is named, and a raw substring search would fire on
        // that explanation. The criterion is about the config *keys* git will
        // resolve, and git resolves nothing from a `#` line.
        for line in body.lines().filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.is_empty() && !trimmed.starts_with('#') && !trimmed.starts_with(';')
        }) {
            let lower = line.to_lowercase();
            assert!(
                !lower.contains("helper") && !lower.contains("credential"),
                "the generated config resolves a credential-helper key on {line:?}, \
                 which puts the user's keychain back within reach (D-16):\n{body}"
            );
        }
    }

    #[test]
    fn a_hostile_alias_yields_an_error_rather_than_an_environment() {
        let (_tmp, root, project) = roots();
        for hostile in ["", ".", "..", "../escaped", "a/b", "/etc/passwd"] {
            assert!(
                build_env_in(&root, hostile, &project).is_err(),
                "{hostile:?} must not reach the point where an environment is built"
            );
            assert!(
                write_gitconfig_in(&root, hostile, "n", "e").is_err(),
                "{hostile:?} must not reach the point where a config is written"
            );
        }
    }
}
