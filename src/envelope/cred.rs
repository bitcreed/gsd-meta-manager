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

use crate::config::CredentialSource;
use crate::journal::redact::redact;

/// The filename of the generated askpass responder, inside the alias's envelope
/// directory. D-17 names this path.
const ASKPASS_FILE: &str = "askpass";

/// What the responder answers a `Username for …` prompt with.
///
/// A token-authenticated HTTPS push ignores the username, so there is no reason
/// for the secret to appear twice. A non-secret placeholder that says what it is
/// keeps the token to exactly one prompt.
const ASKPASS_USERNAME: &str = "x-access-token";

/// The run-journal locator carried into every hook and guard re-entry (D-24,
/// D-25).
pub const PROJECT_ROOT_ENV: &str = "GSD_MM_ENVELOPE_PROJECT_ROOT";

/// The run id carried into the `PreToolUse` guard, for the per-run PR cap
/// (SAFE-06, D-19).
///
/// **Named here rather than spelled at its two ends**, next to
/// [`PROJECT_ROOT_ENV`] and for the same reason: the guard is a fresh process
/// per tool call and has no other way to know which run it belongs to, so the
/// writer (the driver's spawn closure) and the reader
/// ([`super::hooks::guard`]) must agree on a string neither of them can see the
/// other type. If they disagree, every tool call reports a different run and the
/// per-run cap is silently unenforced — a control that is off with nothing
/// indicating it, which is the failure mode this whole phase is written against.
///
/// It is **not** set by [`build_env`]: that function is given an alias and a
/// project root, not a run, and inventing a run id inside it would be a second
/// place run ids come from. Wiring it is the driver's job.
pub const RUN_ID_ENV: &str = "GSD_MM_RUN_ID";

/// The filename of the envelope-generated git config, inside the alias's
/// envelope directory.
const GITCONFIG_FILE: &str = "gitconfig";

/// The subdirectory `GH_CONFIG_DIR` is pointed at.
///
/// It is created empty and left empty. The external GitHub client reads
/// `hosts.yml` from here, so an empty directory is a client that knows about no
/// host and holds no token — which is the whole intent, and it is why the
/// directory is created rather than merely named: a `GH_CONFIG_DIR` pointing at
/// a path that does not exist is a configuration some versions decline to
/// honour, and declining to honour it means falling back to the user's own.
const GH_SUBDIR: &str = "gh";

/// The identity a generated config falls back to when the user's own git
/// configuration names none.
///
/// A driven run with no `user.name` cannot commit at all — git refuses with
/// "Please tell me who you are" — so an absent identity has to become
/// *something*. It becomes a name that is obviously this tool and an address in
/// the RFC 2606 `.invalid` TLD, which can never resolve to a real mailbox. The
/// alternative, failing the run, would turn a cosmetic gap in the user's
/// configuration into a refusal, and refusals in this module are reserved for
/// the thing SAFE-05 is actually about.
const FALLBACK_NAME: &str = "gsd-meta-manager driven run";
const FALLBACK_EMAIL: &str = "driven-run@gsd-meta-manager.invalid";

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
    tmp.write_all(gitconfig_body(name, email).as_bytes())
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
    let binary = std::env::current_exe()
        .context("cannot resolve this binary's own path, so no askpass responder can name it")?;
    build_env_in(&root, alias, project_root, &binary)
}

/// [`build_env`] against an explicit envelope root.
///
/// The two `git config --get` lookups for the user's identity are the only I/O
/// beyond writing the generated config and creating the `gh` directory: no
/// process of this module's own is spawned, because both reads go through
/// [`crate::state_reader::git_ops::git_read_raw`], which already carries the
/// `--no-optional-locks` and failure-as-data properties a read that must not
/// mutate the repository it is reading needs.
pub fn build_env_in(
    root: &Path,
    alias: &str,
    project_root: &Path,
    binary: &Path,
) -> anyhow::Result<EnvelopeEnv> {
    let dir = super::envelope_dir_in(root, alias).ok_or_else(|| {
        anyhow!("refusing to build an environment for alias {alias:?}: not a plain path component")
    })?;

    // Canonicalised before anything is written, so a project root that does not
    // exist is an error here rather than a locator pointing at nothing.
    let project_root = std::fs::canonicalize(project_root).with_context(|| {
        format!(
            "cannot resolve the driven project root {}, so no run journal could be \
             located from inside a hook",
            project_root.display()
        )
    })?;

    let (name, email) = resolve_identity(&project_root);
    let gitconfig = write_gitconfig_in(root, alias, &name, &email)?;

    let gh_dir = dir.join(GH_SUBDIR);
    std::fs::create_dir_all(&gh_dir)
        .with_context(|| format!("failed to create {}", gh_dir.display()))?;

    let hooks_dir = super::hooks::hooks_dir_in(root, alias).ok_or_else(|| {
        anyhow!("alias {alias:?} sanctions no hooks directory, so no hook can be delivered")
    })?;

    // The host is resolved ONCE, here, from the remote the project has at run
    // start — and then baked into the generated responder. Re-deriving it inside
    // the responder would read a value the agent can change, which is precisely
    // the attack D-17 names: an agent that adds a second remote would move the
    // host the responder answers for, and be handed the token for it.
    let remote_host = configured_remote_host(&project_root).unwrap_or_default();
    let askpass = write_askpass_stub_in(root, alias, binary, &remote_host)?;

    let mut entries: Vec<EnvelopeVar> = vec![
        // Removed, not emptied. An ambient agent socket is the shortest path
        // from a driven run to the user's own keys, and an empty value would
        // leave a variable an agent can notice and work around (D-16).
        (OsString::from("SSH_AUTH_SOCK"), None),
        (OsString::from("SSH_AGENT_PID"), None),
        // No user ssh config, no agent, no default identity file, no prompt.
        (
            OsString::from("GIT_SSH_COMMAND"),
            Some(OsString::from(ENVELOPE_SSH_COMMAND)),
        ),
        // Both scopes, one file. git resolves `credential.helper` from system
        // and global configuration, and there is now no system or global
        // configuration that mentions one.
        (
            OsString::from("GIT_CONFIG_GLOBAL"),
            Some(gitconfig.clone().into_os_string()),
        ),
        (
            OsString::from("GIT_CONFIG_SYSTEM"),
            Some(gitconfig.into_os_string()),
        ),
        // The ONLY channel a token reaches git through (D-17). The responder
        // writes the secret to its stdout and nowhere else — never an argv
        // element, which is world-readable through the process table; never URL
        // userinfo, which lands in `.git/config` and in the `git remote -v`
        // output this project journals; and never a file under the envelope
        // directory, which would be a secret at rest with no protection posture.
        (
            OsString::from("GIT_ASKPASS"),
            Some(askpass.into_os_string()),
        ),
        // A detached driver's stdio is null (Phase 17 D-01), so an
        // authentication prompt is not a prompt — it is a hang the idle cap
        // eventually kills hours later. Set alongside GIT_ASKPASS on purpose: a
        // responder that refuses makes git fall back to the terminal, and this
        // is what turns that fallback into an immediate, legible failure instead
        // of a block on a null stdio.
        (
            OsString::from("GIT_TERMINAL_PROMPT"),
            Some(OsString::from("0")),
        ),
        // The external GitHub client cannot read the user's own
        // `~/.config/gh/hosts.yml`, so its credential helper has no host and no
        // token to offer.
        (
            OsString::from("GH_CONFIG_DIR"),
            Some(gh_dir.into_os_string()),
        ),
        // The run-journal locator, and the ONE variable here that exists for
        // **evidence** rather than for containment. D-24 requires every envelope
        // refusal to park the run and D-25 requires the park to land where a
        // later reader can find it — which means the hook and guard re-entries,
        // separate processes spawned by git and by the agent's own tooling, must
        // be able to resolve the active run's journal. `alias` alone cannot get
        // them there: mapping an alias to a project root runs through
        // `DrivableProject::from_registry`, which has exactly one production
        // call site that `tests/spawn_seam_guard.rs` holds (D-28). Carrying the
        // root on the environment the envelope already owns is what avoids
        // minting a second one.
        //
        // The honest limit: a child that unsets this loses its park evidence but
        // does **not** gain the ability to push. Refusal never depends on the
        // locator being present, and plan 19-07 wires the two in that order.
        (
            OsString::from(PROJECT_ROOT_ENV),
            Some(project_root.into_os_string()),
        ),
    ];

    // Folded in last so the count reflects every injected key. `config_env`
    // derives it from the pairs, so adding a key here cannot silently drop the
    // whole injection the way a hardcoded count would.
    for (key, value) in hooks_path_env(&hooks_dir) {
        entries.push((key, Some(value)));
    }

    Ok(EnvelopeEnv(entries))
}

/// Write `<envelope_dir>/askpass` for `alias`, returning its path (D-17).
///
/// A generated stub rather than a multi-word `GIT_ASKPASS` string, for the same
/// reason the hooks are stubs: `GIT_ASKPASS` is a **program**, and a value with
/// spaces in it depends on git deciding to route it through a shell. The stub is
/// the same three-line `exec` shape [`super::hooks`] generates, carries no
/// policy, and — the part that matters here — **carries no secret**. It names
/// the alias and the host; the token is resolved inside the binary at the moment
/// git asks, and goes to stdout.
///
/// **The limit, stated rather than discovered.** An agent inside the driven run
/// can execute this responder itself and read the token off its stdout. That is
/// not a hole this design can close: any credential a driven run can push with
/// is a credential the run can read. What D-17 buys is that the token is not
/// *at rest* anywhere, is not visible in the process table, and does not land in
/// `.git/config` — so it does not leak to everything that reads those. The
/// answer to the agent that wants it is the credential's own scope and D-27's
/// server-side branch protection, not a cleverer envelope.
pub fn write_askpass_stub_in(
    root: &Path,
    alias: &str,
    binary: &Path,
    host: &str,
) -> anyhow::Result<PathBuf> {
    let dir = super::envelope_dir_in(root, alias).ok_or_else(|| {
        anyhow!("refusing to generate an askpass responder for alias {alias:?}: not a plain path component")
    })?;
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("failed to create {}", dir.display()))?;

    let path = dir.join(ASKPASS_FILE);
    let body = format!(
        "#!/bin/sh\n\
         # gsd-meta-manager envelope askpass. The secret lives in the binary's\n\
         # stdout and nowhere in this file (D-17).\n\
         exec {} envelope askpass {} --host {} -- \"$1\"\n",
        super::hooks::sh_quote(&binary.to_string_lossy()),
        super::hooks::sh_quote(alias),
        super::hooks::sh_quote(host),
    );

    let mut tmp = NamedTempFile::new_in(&dir)
        .with_context(|| format!("failed to create a temp file in {}", dir.display()))?;
    tmp.write_all(body.as_bytes())
        .with_context(|| format!("failed to write {}", path.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tmp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o700))
            .with_context(|| format!("failed to make {} executable", path.display()))?;
    }
    tmp.persist(&path)
        .with_context(|| format!("failed to persist the askpass responder to {}", path.display()))?;
    Ok(path)
}

/// The host of the project's configured `origin` remote, or `None`.
fn configured_remote_host(project_root: &Path) -> Option<String> {
    let url = crate::state_reader::git_ops::git_read_raw(
        project_root,
        &["config", "--get", "remote.origin.url"],
    )?;
    url_host(url.trim())
}

/// The host component of a git remote URL, lowercased.
///
/// Covers the three shapes git accepts and one it does not need a credential
/// for: `scheme://[user@]host[:port]/path`, the scp-like `[user@]host:path`, and
/// `file://` / a bare local path — which yield `None`, because a local
/// repository is reached without authenticating to anybody.
///
/// `pub(super)` rather than private because [`super::advisory`] asks the same
/// question of the same URLs. A second copy of this parser is a second place for
/// the look-alike cases — userinfo containing an `@`, an IPv6 literal's own
/// colons, a bare local path — to be handled differently, and the drift would
/// show up as a probe reasoning about a host the responder never scoped.
pub(super) fn url_host(url: &str) -> Option<String> {
    if url.is_empty() {
        return None;
    }
    let after_scheme = match url.find("://") {
        Some(index) => {
            let scheme = url[..index].to_ascii_lowercase();
            if scheme == "file" {
                return None;
            }
            &url[index + 3..]
        }
        None => {
            // scp-like, or a bare path. A bare path has no `:` before the first
            // `/`, which is exactly how git tells the two apart.
            let colon = url.find(':')?;
            if url[..colon].contains('/') {
                return None;
            }
            &url[..colon]
        }
    };

    // Authority ends at the first `/`; userinfo ends at the last `@` inside it.
    let authority = after_scheme.split('/').next().unwrap_or_default();
    let host_port = match authority.rfind('@') {
        Some(index) => &authority[index + 1..],
        None => authority,
    };
    // A port, but not an IPv6 literal's own colons.
    let host = if host_port.starts_with('[') {
        host_port
            .split(']')
            .next()
            .map(|h| h.trim_start_matches('['))
            .unwrap_or(host_port)
    } else {
        host_port.split(':').next().unwrap_or(host_port)
    };

    if host.is_empty() {
        return None;
    }
    Some(host.to_ascii_lowercase())
}

/// Resolve the configured credential to a token, or report it unavailable
/// (D-18).
///
/// `Ok(None)` for an unset variable, an empty value, a command that could not be
/// run and a command that exited non-zero. All four are *unavailability*, which
/// is a legible operational state, not a bug — reporting them as errors would
/// make an unconfigured project look broken.
///
/// **There is deliberately no third branch that consults the user's ambient
/// credential**, and its absence is the requirement rather than an omission.
/// SAFE-05 says a driven run must not inherit the user's credentials, so
/// "unconfigured" silently meaning "the user's credentials" is the exact failure
/// the requirement names. A run with no configured credential can still read,
/// commit and be driven; it cannot push, and it says so up front rather than at
/// minute 90.
pub fn resolve_credential(source: &CredentialSource) -> anyhow::Result<Option<String>> {
    let raw = match source {
        CredentialSource::Env { var } => match std::env::var(var) {
            Ok(value) => value,
            // A variable that is unset or holds non-UTF-8 is a credential that
            // is not there. Neither is an error.
            Err(_) => return Ok(None),
        },
        CredentialSource::Command { argv } => {
            let Some((program, args)) = argv.split_first() else {
                return Ok(None);
            };
            // argv, never a shell string — the property `CredentialSource`
            // records at the type. With no shell in the path, a value carrying a
            // `;` cannot become a second command.
            let output = match std::process::Command::new(program).args(args).output() {
                Ok(output) => output,
                Err(_) => return Ok(None),
            };
            if !output.status.success() {
                return Ok(None);
            }
            String::from_utf8_lossy(&output.stdout).into_owned()
        }
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    Ok(Some(trimmed.to_string()))
}

/// What the responder should say, as a value.
///
/// `Ok` is the single line that goes to **stdout**; `Err` is the reason that
/// goes to stderr with a non-zero exit. Splitting the decision out of the I/O is
/// what makes "the credential is emitted for this host and not that one" a unit
/// test rather than a process.
fn askpass_reply(
    prompt: &str,
    configured_remote_host: &str,
    credential: Option<&str>,
) -> Result<String, String> {
    let Some(asked) = prompt_host(prompt) else {
        return Err(format!(
            "envelope askpass: cannot determine which host {:?} is authenticating to, \
             so no credential is emitted",
            redact(prompt)
        ));
    };

    // The host check runs BEFORE the credential is resolved, so a prompt for a
    // remote the envelope does not know never even triggers a lookup. An agent
    // that adds a second remote gets an authentication failure here, not a
    // token (D-17).
    if configured_remote_host.is_empty() || !asked.eq_ignore_ascii_case(configured_remote_host) {
        return Err(format!(
            "envelope askpass: refusing to answer for host {asked:?}; this run's \
             configured remote is {configured_remote_host:?}"
        ));
    }

    let Some(credential) = credential else {
        return Err(format!(
            "envelope askpass: {} — no credential is configured for this run, so it \
             cannot push. Configure `credential` on the project's driver opt-in; there \
             is deliberately no fallback to your own git credentials (D-18).",
            crate::envelope::policy::REASON_CREDENTIAL_UNAVAILABLE
        ));
    };

    if prompt.trim_start().to_ascii_lowercase().starts_with("username") {
        return Ok(ASKPASS_USERNAME.to_string());
    }
    Ok(credential.to_string())
}

/// The host git names in an askpass prompt.
///
/// git's prompts are `Username for 'https://host/path': ` and `Password for
/// 'https://user@host': `, so the URL is the first single-quoted run.
fn prompt_host(prompt: &str) -> Option<String> {
    let start = prompt.find('\'')? + 1;
    let rest = &prompt[start..];
    let end = rest.find('\'')?;
    url_host(&rest[..end])
}

/// The responder git re-enters this binary as (D-17).
///
/// Emits the credential on `out` for the configured host and nothing at all for
/// any other, returning the process exit code. The secret reaches **only** this
/// writer: no log, no `tracing`, no file, no stderr.
pub fn askpass_into<W: Write>(
    out: &mut W,
    prompt: &str,
    configured_remote_host: &str,
    credential: Option<&str>,
) -> anyhow::Result<i32> {
    match askpass_reply(prompt, configured_remote_host, credential) {
        Ok(line) => {
            // `writeln!` and then nothing: git reads one line from the
            // responder's stdout and that is the entire transport.
            writeln!(out, "{line}").context("failed to write the credential to stdout")?;
            Ok(0)
        }
        Err(reason) => {
            // Through the already-shipped redaction path, never a forked one
            // (SAFE-04, D-12). The reason carries a host and a prompt, and a
            // prompt is attacker-influenceable text.
            eprintln!("{}", redact(&reason));
            Ok(1)
        }
    }
}

/// [`askpass_into`] against the registry at the default path.
pub fn askpass(alias: &str, prompt: &str, configured_remote_host: &str) -> anyhow::Result<i32> {
    askpass_with_config(
        &crate::config::Config::default_path(),
        alias,
        prompt,
        configured_remote_host,
    )
}

/// [`askpass`] against an explicit registry path.
///
/// The generated stub does not pass `--config`, so in the shipped hook path this
/// resolves to the default registry either way. It takes the path anyway because
/// `main.rs` already has one in scope, and a responder that read a *different*
/// registry than the run was configured from would find no credential and fail
/// closed — the safe direction, but silently, which is the direction this
/// project does not ship things in.
pub fn askpass_with_config(
    config_path: &Path,
    alias: &str,
    prompt: &str,
    configured_remote_host: &str,
) -> anyhow::Result<i32> {
    if !crate::journal::is_plain_path_component(alias) {
        return Err(anyhow!(
            "alias {alias:?} is not a plain path component, so no envelope sanctions \
             a credential for it"
        ));
    }
    let config = crate::config::load_config(config_path)?;
    let source = config
        .projects
        .get(alias)
        .and_then(|project| project.driver_opt_in.as_ref())
        .and_then(|opt_in| opt_in.credential.clone());

    let credential = match source.as_ref() {
        Some(source) => resolve_credential(source)?,
        None => None,
    };

    askpass_into(
        &mut std::io::stdout(),
        prompt,
        configured_remote_host,
        credential.as_deref(),
    )
}

/// The user's resolved `user.name` and `user.email`, or the `.invalid`
/// fallbacks.
///
/// Read from the driven project, so a repository-local identity wins exactly as
/// it would for the human — the generated config is meant to preserve the
/// user's attribution, not to impose a new one.
fn resolve_identity(project_root: &Path) -> (String, String) {
    let read = |key: &str| -> Option<String> {
        let raw = crate::state_reader::git_ops::git_read_raw(
            project_root,
            &["config", "--get", key],
        )?;
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return None;
        }
        Some(trimmed.to_string())
    };
    (
        read("user.name").unwrap_or_else(|| FALLBACK_NAME.to_string()),
        read("user.email").unwrap_or_else(|| FALLBACK_EMAIL.to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const ALIAS: &str = "demo";

    /// A stand-in for the envelope binary. Nothing in this module execs it —
    /// [`build_env_in`] only writes its path into the generated responder — so a
    /// path is all these tests need, and naming a real binary would invite one of
    /// them to start running it.
    const FAKE_BIN: &str = "/nonexistent/gsd-meta-manager";

    const HOST: &str = "git.example.com";

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
        let env = build_env_in(&root, ALIAS, &project, Path::new(FAKE_BIN))
            .expect("a plain alias builds an env");

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
        let env = build_env_in(&root, ALIAS, &project, Path::new(FAKE_BIN))
            .expect("a plain alias builds an env");

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
        let env = build_env_in(&root, ALIAS, &project, Path::new(FAKE_BIN))
            .expect("a plain alias builds an env");
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
        let env = build_env_in(&root, ALIAS, &project, Path::new(FAKE_BIN))
            .expect("a plain alias builds an env");
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
        let env = build_env_in(&root, ALIAS, &project, Path::new(FAKE_BIN))
            .expect("a plain alias builds an env");

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
        let env = build_env_in(&root, ALIAS, &project, Path::new(FAKE_BIN))
            .expect("a plain alias builds an env");

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
                build_env_in(&root, hostile, &project, Path::new(FAKE_BIN)).is_err(),
                "{hostile:?} must not reach the point where an environment is built"
            );
            assert!(
                write_gitconfig_in(&root, hostile, "n", "e").is_err(),
                "{hostile:?} must not reach the point where a config is written"
            );
            assert!(
                write_askpass_stub_in(&root, hostile, Path::new(FAKE_BIN), HOST).is_err(),
                "{hostile:?} must not reach the point where a responder is written"
            );
        }
    }

    // -----------------------------------------------------------------------
    // The credential (D-17, D-18)
    // -----------------------------------------------------------------------

    #[test]
    fn the_askpass_pointer_is_set_and_no_url_userinfo_is_ever_constructed() {
        let (_tmp, root, project) = roots();
        let env = build_env_in(&root, ALIAS, &project, Path::new(FAKE_BIN))
            .expect("a plain alias builds an env");
        let dir = super::super::envelope_dir_in(&root, ALIAS).expect("a plain alias");

        let askpass = env.value("GIT_ASKPASS").expect("GIT_ASKPASS must be set");
        assert!(
            Path::new(askpass).starts_with(&dir),
            "GIT_ASKPASS points at {askpass:?}, outside {}",
            dir.display()
        );
        assert!(
            Path::new(askpass).is_file(),
            "GIT_ASKPASS names a file git will exec, so it has to exist"
        );
    }

    #[test]
    fn an_unset_variable_and_a_failing_command_are_unavailability_not_an_error() {
        // A variable this process could not plausibly have. Reading rather than
        // setting keeps this test free of the process-global env mutation that
        // makes a parallel suite flaky.
        let absent = CredentialSource::Env {
            var: "GSD_MM_NO_SUCH_CREDENTIAL_VARIABLE_19_04".to_string(),
        };
        assert_eq!(
            resolve_credential(&absent).expect("an unset variable is not an error"),
            None
        );

        let failing = CredentialSource::Command {
            argv: vec!["false".to_string()],
        };
        assert_eq!(
            resolve_credential(&failing).expect("a non-zero exit is not an error"),
            None
        );

        let missing = CredentialSource::Command {
            argv: vec!["/nonexistent/credential-helper-19-04".to_string()],
        };
        assert_eq!(
            resolve_credential(&missing).expect("an unrunnable command is not an error"),
            None
        );

        let empty = CredentialSource::Command { argv: Vec::new() };
        assert_eq!(resolve_credential(&empty).expect("an empty argv is not an error"), None);
    }

    #[test]
    fn a_command_credential_is_read_from_stdout_and_trimmed() {
        let source = CredentialSource::Command {
            argv: vec!["printf".to_string(), "  ghp_secret\n".to_string()],
        };
        assert_eq!(
            resolve_credential(&source).expect("printf runs"),
            Some("ghp_secret".to_string()),
        );

        // Whitespace-only stdout is a credential that is not there.
        let blank = CredentialSource::Command {
            argv: vec!["printf".to_string(), "   \n".to_string()],
        };
        assert_eq!(resolve_credential(&blank).expect("printf runs"), None);
    }

    #[test]
    fn the_credential_reaches_stdout_for_the_configured_host_and_nothing_else_does() {
        let mut out = Vec::new();
        let code = askpass_into(
            &mut out,
            &format!("Password for 'https://x@{HOST}': "),
            HOST,
            Some("ghp_secret"),
        )
        .expect("the responder writes");

        assert_eq!(code, 0, "the configured host must be answered");
        assert_eq!(String::from_utf8_lossy(&out), "ghp_secret\n");
    }

    #[test]
    fn a_second_remote_gets_an_authentication_failure_rather_than_the_token() {
        // The whole point of scoping: an agent that adds a remote of its own and
        // pushes to it must not be handed the user's token by the responder.
        for hostile in [
            "https://evil.example.net/repo.git",
            "https://x@evil.example.net/repo.git",
            "ssh://git@evil.example.net:22/repo.git",
        ] {
            let mut out = Vec::new();
            let code = askpass_into(
                &mut out,
                &format!("Password for '{hostile}': "),
                HOST,
                Some("ghp_secret"),
            )
            .expect("the responder writes");

            assert_eq!(code, 1, "{hostile} must not be answered");
            assert!(
                out.is_empty(),
                "{hostile} received {:?} on stdout",
                String::from_utf8_lossy(&out)
            );
        }
    }

    #[test]
    fn an_unparseable_prompt_and_an_unconfigured_host_are_both_refusals() {
        for (prompt, host) in [
            ("Password: ", HOST),
            ("", HOST),
            // No configured remote at all: the empty host answers nothing,
            // rather than matching an empty host in a prompt.
            (&format!("Password for 'https://{HOST}': ") as &str, ""),
        ] {
            let mut out = Vec::new();
            let code =
                askpass_into(&mut out, prompt, host, Some("ghp_secret")).expect("the responder writes");
            assert_eq!(code, 1, "{prompt:?} against host {host:?} must be refused");
            assert!(out.is_empty(), "{prompt:?} produced output");
        }
    }

    #[test]
    fn an_unconfigured_credential_fails_closed_with_a_named_reason() {
        let mut out = Vec::new();
        let code = askpass_into(&mut out, &format!("Password for 'https://{HOST}': "), HOST, None)
            .expect("the responder writes");

        assert_eq!(code, 1, "no credential must not mean the user's credential");
        assert!(
            out.is_empty(),
            "an unconfigured run emitted {:?}",
            String::from_utf8_lossy(&out)
        );

        // The reason is the D-24 identifier, so a later reader parses a constant
        // rather than prose.
        let reason = askpass_reply(&format!("Password for 'https://{HOST}': "), HOST, None)
            .expect_err("no credential is a refusal");
        assert!(
            reason.contains(crate::envelope::policy::REASON_CREDENTIAL_UNAVAILABLE),
            "the refusal must name credential_unavailable: {reason}"
        );
    }

    #[test]
    fn a_username_prompt_is_answered_without_spending_the_secret_twice() {
        let mut out = Vec::new();
        let code = askpass_into(
            &mut out,
            &format!("Username for 'https://{HOST}': "),
            HOST,
            Some("ghp_secret"),
        )
        .expect("the responder writes");

        assert_eq!(code, 0);
        assert_eq!(String::from_utf8_lossy(&out), format!("{ASKPASS_USERNAME}\n"));
    }

    #[test]
    fn no_file_under_the_envelope_directory_holds_the_credential() {
        // A behavior assertion over the directory contents, not a source grep:
        // the claim is that nothing this run wrote is a secret at rest, and only
        // reading what was written can settle that (D-17).
        const SECRET: &str = "ghp_A1b2C3d4E5f6G7h8I9j0K1l2M3n4O5p6Q7r8";
        let (_tmp, root, project) = roots();
        build_env_in(&root, ALIAS, &project, Path::new(FAKE_BIN)).expect("an env is built");

        let mut out = Vec::new();
        askpass_into(
            &mut out,
            &format!("Password for 'https://{HOST}': "),
            HOST,
            Some(SECRET),
        )
        .expect("the responder writes");
        assert!(
            String::from_utf8_lossy(&out).contains(SECRET),
            "the fixture must actually have emitted the secret, or the walk below \
             proves nothing"
        );

        let dir = super::super::envelope_dir_in(&root, ALIAS).expect("a plain alias");
        let mut seen = 0usize;
        let mut stack = vec![dir.clone()];
        while let Some(next) = stack.pop() {
            for entry in std::fs::read_dir(&next).expect("the envelope directory is readable") {
                let entry = entry.expect("a readable entry");
                let path = entry.path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                seen += 1;
                let bytes = std::fs::read(&path).expect("a readable file");
                assert!(
                    !String::from_utf8_lossy(&bytes).contains(SECRET),
                    "{} holds the credential; a secret at rest under the envelope \
                     has no protection posture (D-17)",
                    path.display()
                );
            }
        }
        assert!(seen > 0, "the walk saw no files, so it asserted nothing");
    }

    #[test]
    fn a_remote_url_yields_the_host_it_authenticates_to_and_a_local_one_yields_none() {
        for (url, expected) in [
            ("https://github.com/owner/repo.git", Some("github.com")),
            ("https://user@github.com/owner/repo.git", Some("github.com")),
            ("https://GitHub.com:443/owner/repo.git", Some("github.com")),
            ("ssh://git@git.example.com:2222/o/r.git", Some("git.example.com")),
            ("git@github.com:owner/repo.git", Some("github.com")),
            ("http://[::1]:8080/repo.git", Some("::1")),
            // Local: nobody to authenticate to, so nothing to answer for.
            ("file:///tmp/remote.git", None),
            ("/tmp/remote.git", None),
            ("", None),
        ] {
            assert_eq!(
                url_host(url).as_deref(),
                expected,
                "url_host({url:?}) disagreed"
            );
        }
    }
}
