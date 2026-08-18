//! What the remote itself enforces — probed, never assumed (D-26).
//!
//! Every other layer in this module is client-side, and the module doc for
//! [`super`] already states the ceiling that follows from that: an agent which
//! can spawn an unsupervised shell and chooses to can strip the environment the
//! other layers rely on. The two boundaries that survive that agent are the
//! **remote's own ruleset** and the **scope of the credential**. This file is
//! about the first of them.
//!
//! ## Why a probe rather than an assumption
//!
//! A tool that assumed the remote was protected would be making the exact claim
//! this phase exists to refuse to make. A tool that assumed it was unprotected
//! would cry wolf at every well-configured repository until the warning was
//! ignored. So the state is read: `protected`, `unprotected`, or
//! `unknown` **carrying the reason it is not known**.
//!
//! **`Unknown` is never rendered as `Protected`.** That is the whole decision in
//! one sentence. A probe that could not run is a statement about what we do not
//! know; it is never a statement about risk, and it is never quietly rounded
//! towards the comfortable answer. Not knowing is reported as not knowing, with
//! the reason attached, because a missing warning reads as an absent hazard.
//!
//! ## This module has no write path, by construction
//!
//! There is deliberately no function here that issues a request which could
//! change anything on the remote — no protection is ever applied, relaxed or
//! removed from this tool. Applying a ruleset needs an `Administration` scope
//! that the run credential withholds on purpose (D-18), and a module that could
//! apply protection is a module that could also remove it. Every query below is
//! a read.
//!
//! `tests/envelope_advisory.rs::the_module_has_no_write_path_at_all` scans this
//! file for the write verbs and requires none, so the property is a build
//! failure rather than a promise.
//!
//! **The deferred alternative, stated rather than left implicit.** A later
//! milestone that wants to *apply* protection needs a separate credential the
//! user explicitly consented to for that purpose. That is a product decision
//! about what a user is agreeing to when they opt a project in — not a code
//! change — and it is out of scope here for that reason, not because it is hard.
//!
//! ## Where the probe runs, and where it must never run
//!
//! Once, at run start. **Never on the guard's per-tool-call path.** A network
//! call on the agent's critical path is not a theoretical hazard in this
//! project: `src/executor/mod.rs:225-239` records a reproduced 180-240 second
//! `PreToolUse` hang and the mitigation it needed. [`probe_protection`] is
//! bounded by its own budget for the same reason, so the worst case at run start
//! is a delay measured in seconds followed by `Unknown`, never a run that never
//! begins.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::envelope::cred::EnvelopeEnv;
use crate::journal::redact::redact;

/// The external GitHub client the probe asks.
///
/// Its absence is `Unknown`, never a verdict: a machine without the client
/// installed knows nothing about the remote's rulesets, and "I could not ask"
/// must not render as "there is nothing to worry about".
const CLIENT: &str = "gh";

/// How long the whole probe may take before it gives up and says so.
///
/// A bound rather than a hope. `std::process::Command::output` waits forever,
/// and a run that never starts because a proxy swallowed a connection is a worse
/// failure than a run that starts without knowing the protection state.
const PROBE_BUDGET_SECS: u64 = 10;

/// How often the wait loop checks whether the child finished.
const PROBE_POLL: Duration = Duration::from_millis(25);

/// What the remote enforces on its default branch.
///
/// **The invariant that carries the whole decision:** [`ProtectionState::Unknown`]
/// is never rendered as [`ProtectionState::Protected`], and the reason a probe
/// failed travels with it. A failure to ask is a fact about the probe, not a
/// fact about the remote — and reporting it as an absence of risk is precisely
/// the overstated safety claim D-27 says is worse than a stated limitation,
/// because an overstated claim gets trusted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtectionState {
    /// An active branch ruleset, or branch protection, covers the default branch.
    Protected,
    /// The remote was asked and answered: neither control is in force.
    Unprotected,
    /// The remote could not be asked, or its answer could not be read.
    Unknown {
        /// Why it is not known — already redacted, and always shown.
        reason: String,
    },
}

impl ProtectionState {
    /// A short stable identifier, in the style of
    /// [`crate::journal::JournalEvent::Diagnostic`]'s `code`.
    ///
    /// Stable because a later reader greps for it: the identifier is the thing
    /// that survives a rewording of the prose around it.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProtectionState::Protected => "protected",
            ProtectionState::Unprotected => "unprotected",
            ProtectionState::Unknown { .. } => "unknown",
        }
    }

    /// The reason this state is `Unknown`, or `None` for the two known states.
    pub fn reason(&self) -> Option<&str> {
        match self {
            ProtectionState::Unknown { reason } => Some(reason.as_str()),
            _ => None,
        }
    }

    /// Whether this state warrants a warning marker wherever it is rendered.
    ///
    /// Both `Unprotected` and `Unknown` do (D-26). Only a remote that was asked
    /// and answered "protected" is rendered without one.
    pub fn is_warning(&self) -> bool {
        !matches!(self, ProtectionState::Protected)
    }

    /// An `Unknown` whose reason has been through the redaction table.
    ///
    /// **One constructor, so the redaction cannot be forgotten at one site.**
    /// Reasons quote remote URLs and client stderr, and those carry hosts, home
    /// directory paths and — in the failure modes that matter most — sometimes a
    /// token-shaped string (T-19-43). `crate::journal::redact` is the table this
    /// project already ships and tests; a second sanitiser here would be a
    /// second thing to keep in step.
    fn unknown(reason: impl AsRef<str>) -> Self {
        ProtectionState::Unknown {
            reason: redact(reason.as_ref()),
        }
    }
}

/// The state a preview reports when nothing probed the remote.
///
/// A dry run contacts no network — `dry_run::SECTION_REFSPECS` says so in the
/// same output — so the honest answer there is `Unknown` **with that as the
/// reason**, not a silently omitted section and certainly not an optimistic
/// default. The run-start probe is what fills this in for a real run.
pub fn not_probed() -> ProtectionState {
    ProtectionState::unknown(
        "not probed here: a dry run contacts no network, and the read-only \
         protection query runs once at run start under the run's own environment",
    )
}

/// The envelope's honesty statement (**pinned contract** — see the rule below).
///
/// **This constant is a contract, not decoration.** It gets the same treatment
/// `src/driver/dry_run.rs`'s three section constants carry, and for the same
/// reason their doc records: text that only *happens* to say something true is
/// text a later refactor tidies into something softer without anybody
/// noticing.
///
/// `tests/envelope_advisory.rs::the_honesty_statement_carries_each_of_its_three_required_parts`
/// pins it. That test matches a distinctive phrase from **each** of the three
/// parts below rather than the whole paragraph, so a rewording is allowed and a
/// *dropped part* is a build failure. Changing this text is a user-visible
/// output change: it is rendered into the dry-run preview and, through
/// [`envelope_notice`], into the run journal.
///
/// The three parts, in this order and for this reason:
///
/// 1. **What IS mechanically guaranteed.** Claimed narrowly, because a claim
///    wider than the mechanism is the thing that gets trusted and should not
///    be.
/// 2. **What is NOT.** Stated without hedging. Every clause here is one this
///    codebase's own module docs already record — [`super`]'s three-layer
///    contract names what each layer cannot see, [`super::cred`] states that an
///    agent inside the run can execute the askpass responder and read the token,
///    and `hooks::settings_value` records the one control with no git-hook
///    second carrier. This paragraph agrees with them rather than restating them
///    more comfortably.
/// 3. **Therefore: enable server-side branch protection.** The phase's
///    conclusion, stated as a recommendation rather than as a footnote, because
///    it is the only boundary here that does not depend on the agent's
///    cooperation.
///
/// **Do not soften part 2.** An overstated safety claim is worse than a stated
/// limitation, because it gets trusted — and the incident this phase is built on
/// had two failures, of which the second was reporting success over a deletion
/// the agent had performed.
pub const SECTION_ENVELOPE: &str = "== What this envelope guarantees, and what it does not ==\n\
    Mechanically guaranteed: this run cannot reach your ambient git credentials\n\
    or your SSH agent — the agent socket is removed rather than emptied, and\n\
    git's global and system configuration are redirected into a generated file\n\
    that names no credential helper. A push that reaches git through the driven\n\
    process tree passes the pre-push hook, which judges the refs git itself\n\
    hands it rather than the command line it was asked about. The pull-request\n\
    cap is enforced from an append-only ledger this repository does not contain,\n\
    so the run cannot reset its own limit by deleting a file it can see.\n\
    \n\
    Not guaranteed: client-side hooks, tool denies and env-injected git\n\
    configuration are all defeatable by an agent that can spawn an unsupervised\n\
    shell and chooses to. Each layer is documented with what it cannot see, an\n\
    agent that unsets GIT_CONFIG_COUNT in a subshell is past the last of them,\n\
    an agent that runs the askpass responder itself reads the token, and a\n\
    settings file the agent's own CLI silently ignores leaves the pull-request\n\
    cap unenforced, because no git hook observes a pull request. The only\n\
    boundaries that do not depend on the agent's cooperation are the remote's\n\
    own ruleset and the scope of the credential this run was given.\n\
    \n\
    Therefore: enable server-side branch protection on this repository. It is\n\
    the one control here an agent cannot talk its way past, and it is this\n\
    envelope's conclusion rather than its footnote.";

/// The whole envelope section — the statement plus the state of the one
/// boundary it says you should rely on.
///
/// **One producer, two consumers.** The dry-run preview renders this, and the
/// run journal records it at run start. Two renderings assembled separately
/// would be two things that can drift, and the drift would be the worst kind
/// available here: a preview and a journal disagreeing about what was claimed
/// (T-19-42). A test asserts the preview carries exactly this text.
pub fn envelope_notice(state: &ProtectionState) -> String {
    format!("{SECTION_ENVELOPE}\n{}", protection_line(state))
}

/// The marker every state except `protected` is rendered with.
///
/// A word rather than a symbol: this text goes to a plain stdout preview and
/// into a run journal, and a glyph that renders as a box in somebody's terminal
/// is a warning that did not warn.
pub const PROTECTION_WARNING: &str = "WARNING";

/// The exact wording reserved for a remote that was asked and said yes.
///
/// Named as a constant so a test can assert it is **absent** from the other two
/// renderings. That assertion is the mechanical form of D-26's invariant: an
/// `unknown` that borrowed the protected phrasing would be the optimistic
/// default this module exists to refuse.
const PROTECTED_CLAIM: &str =
    "an active branch ruleset or branch protection covers the default branch";

/// One line describing `state`, for every surface that renders it.
///
/// Both `unprotected` and `unknown` carry [`PROTECTION_WARNING`], and `unknown`
/// carries its reason verbatim — a probe failure is shown, never swallowed.
pub fn protection_line(state: &ProtectionState) -> String {
    match state {
        ProtectionState::Protected => {
            format!("  Remote protection: protected — {PROTECTED_CLAIM}.")
        }
        ProtectionState::Unprotected => format!(
            "  {PROTECTION_WARNING} — Remote protection: unprotected. Neither an \
             active branch ruleset nor branch protection was found on the default \
             branch, so nothing on the remote refuses a push that got past the \
             client-side layers."
        ),
        ProtectionState::Unknown { reason } => format!(
            "  {PROTECTION_WARNING} — Remote protection: unknown — {reason}. Not \
             knowing is reported as not knowing; it is not evidence that the \
             remote is protected."
        ),
    }
}

/// Read the remote's protection state for `project_root`, under `env`.
///
/// **Runs once, at run start. Never on the guard's per-tool-call path** — see
/// the module doc for the hang that constraint exists to avoid.
///
/// Every step is a read, and every failure mode is data:
///
/// 1. The remote URL comes from [`crate::state_reader::git_ops::git_read_raw`],
///    which carries `--no-optional-locks`. That flag is load-bearing rather than
///    hygiene: without it a git *read* opportunistically rewrites `.git/index`,
///    which would make a probe into a repository write.
/// 2. A remote that names no GitHub host yields `Unknown` with that as the
///    reason. A non-GitHub remote is a thing this probe cannot ask, not a thing
///    that is unprotected.
/// 3. The repository's rulesets are queried, falling back to the branch
///    protection endpoint when the first says nothing decisive. Both are reads.
/// 4. A missing client, a network failure, a permission failure, an exceeded
///    budget or an unreadable answer each yields `Unknown` carrying **that
///    specific reason**, so a reader can tell "you are not authenticated" from
///    "this host is unreachable".
///
/// The client runs under the envelope environment, so it reads the run's own
/// (empty) client configuration rather than the user's host configuration. The
/// practical consequence is stated rather than hidden: a run with no credential
/// reaching this code gets `Unknown` naming the authentication failure, which is
/// the correct answer — an unauthenticated client genuinely does not know.
pub fn probe_protection(project_root: &Path, env: &EnvelopeEnv) -> ProtectionState {
    let Some(url) = remote_url(project_root) else {
        return ProtectionState::unknown(
            "this project has no `origin` remote configured, so there is no remote \
             whose protection could be read",
        );
    };

    let Some(host) = crate::envelope::cred::url_host(&url) else {
        return ProtectionState::unknown(format!(
            "the remote {url} names no network host — a local or `file://` remote \
             is one this probe cannot ask, not one that is unprotected"
        ));
    };

    if !is_github_host(&host) {
        return ProtectionState::unknown(format!(
            "the remote host {host} is not a GitHub host, and no other forge's \
             protection API is asked here"
        ));
    }

    let Some(slug) = repo_slug(&url) else {
        return ProtectionState::unknown(format!(
            "the remote {url} does not resolve to an owner/repository pair, so no \
             repository could be asked about"
        ));
    };

    // 1. Rulesets. A ruleset that targets branches and is actively enforced is
    //    the modern answer, and one hit is enough.
    let rulesets = run_client(
        env,
        project_root,
        &["api", &format!("repos/{slug}/rulesets")],
    );
    let ruleset_note = match &rulesets {
        ClientAnswer::Body(body) => {
            if active_branch_ruleset(body) {
                return ProtectionState::Protected;
            }
            "no active branch ruleset".to_string()
        }
        // An older host has no rulesets endpoint at all. That is not a verdict —
        // it is a reason to ask the other endpoint.
        ClientAnswer::Absent(_) => "no rulesets endpoint on this host".to_string(),
        ClientAnswer::Failed(reason) => {
            return ProtectionState::unknown(format!(
                "the ruleset query could not be completed: {reason}"
            ));
        }
    };

    // 2. The default branch, so the fallback endpoint has a branch to ask about.
    let default_branch = match run_client(env, project_root, &["api", &format!("repos/{slug}")]) {
        ClientAnswer::Body(body) => match default_branch_of(&body) {
            Some(branch) => branch,
            None => {
                return ProtectionState::unknown(format!(
                    "{slug} answered without a readable default branch, so the \
                     branch protection endpoint could not be addressed"
                ));
            }
        },
        ClientAnswer::Absent(detail) => {
            return ProtectionState::unknown(format!(
                "{slug} could not be read ({detail}), so its default branch is unknown"
            ));
        }
        ClientAnswer::Failed(reason) => {
            return ProtectionState::unknown(format!(
                "the repository query could not be completed: {reason}"
            ));
        }
    };

    // 3. Branch protection on that branch. Two independent misses — no active
    //    ruleset and no protection — is what `unprotected` means here.
    match run_client(
        env,
        project_root,
        &[
            "api",
            &format!("repos/{slug}/branches/{default_branch}/protection"),
        ],
    ) {
        ClientAnswer::Body(_) => ProtectionState::Protected,
        ClientAnswer::Absent(_) => ProtectionState::Unprotected,
        ClientAnswer::Failed(reason) => ProtectionState::unknown(format!(
            "{ruleset_note} was found, and the branch protection query for \
             {default_branch} could not be completed: {reason}"
        )),
    }
}

/// What one read of the external client came back with.
///
/// `Absent` is separated from `Failed` deliberately: "the remote answered, and
/// the thing you asked about does not exist" is evidence, while "the query never
/// completed" is not. Folding them together would let a proxy error render as
/// `unprotected`, which is a false alarm, or worse — inverted somewhere later —
/// as a false assurance.
enum ClientAnswer {
    /// stdout, for a query that succeeded.
    Body(String),
    /// The remote said the resource is not there.
    Absent(String),
    /// The query did not complete, for the carried reason.
    Failed(String),
}

/// Run the external client read-only, under `env`, within the probe budget.
///
/// **The environment is applied entry by entry**, matching on the `Option` the
/// [`EnvelopeEnv`] type forces, so a removal stays a removal rather than
/// collapsing into an empty assignment.
///
/// **Why a hand-rolled wait loop rather than `output()`.** `output()` waits
/// without a bound, and this runs at run start where a stall delays every run
/// against that project. The loop kills the child at the deadline and reports
/// the budget as the reason. The stdout of these three reads is a few kilobytes,
/// well inside a pipe buffer, so nothing is lost by reading it after the wait;
/// were a response ever large enough to fill the buffer, the child would block
/// writing and the same deadline would end it with `Failed` rather than hanging.
fn run_client(env: &EnvelopeEnv, cwd: &Path, args: &[&str]) -> ClientAnswer {
    let mut command = Command::new(CLIENT);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in env.entries() {
        match value {
            None => {
                command.env_remove(key);
            }
            Some(set) => {
                command.env(key, set);
            }
        }
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return ClientAnswer::Failed(format!(
                "the GitHub client `{CLIENT}` could not be run ({error}), so nothing was asked"
            ));
        }
    };

    let deadline = Instant::now() + Duration::from_secs(PROBE_BUDGET_SECS);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return ClientAnswer::Failed(format!(
                        "the query exceeded its {PROBE_BUDGET_SECS}-second budget and was stopped"
                    ));
                }
                std::thread::sleep(PROBE_POLL);
            }
            Err(error) => {
                return ClientAnswer::Failed(format!(
                    "the client could not be waited on ({error})"
                ));
            }
        }
    }

    let finished = match child.wait_with_output() {
        Ok(finished) => finished,
        Err(error) => {
            return ClientAnswer::Failed(format!(
                "the client's answer could not be read ({error})"
            ));
        }
    };

    if finished.status.success() {
        return ClientAnswer::Body(String::from_utf8_lossy(&finished.stdout).into_owned());
    }

    let complaint = first_line(&String::from_utf8_lossy(&finished.stderr));
    if says_absent(&complaint) {
        return ClientAnswer::Absent(complaint);
    }
    ClientAnswer::Failed(complaint)
}

/// The first non-empty line of `text`, trimmed — the client's own complaint,
/// without the usage banner it sometimes appends beneath it.
fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("no detail was reported")
        .to_string()
}

/// Whether the client's complaint says the resource is not there, as opposed to
/// saying the query failed.
///
/// The branch protection endpoint answers 404 for an unprotected branch, and its
/// human-readable form is the string checked first.
fn says_absent(complaint: &str) -> bool {
    let lowered = complaint.to_ascii_lowercase();
    lowered.contains("branch not protected")
        || lowered.contains("http 404")
        || lowered.contains("not found")
}

/// The configured `origin` URL, read through the shared `--no-optional-locks`
/// git read.
fn remote_url(project_root: &Path) -> Option<String> {
    let raw = crate::state_reader::git_ops::git_read_raw(
        project_root,
        &["config", "--get", "remote.origin.url"],
    )?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    Some(trimmed.to_string())
}

/// Whether `host` is github.com or one of its subdomains.
///
/// Deliberately not "contains github": `github.com.evil.example` contains it and
/// is not GitHub. A host this returns `false` for yields `Unknown`, which is the
/// safe direction — it warns rather than reassures.
fn is_github_host(host: &str) -> bool {
    host == "github.com" || host.ends_with(".github.com")
}

/// `owner/repo` from a git remote URL, or `None`.
///
/// Both components are checked against a conservative character set before they
/// are interpolated into an API path. A remote URL is repository-controlled
/// data, and a component carrying `..` or a slash would be aiming the read at
/// some other path entirely.
fn repo_slug(url: &str) -> Option<String> {
    let rest = match url.find("://") {
        Some(index) => {
            let after = &url[index + 3..];
            let slash = after.find('/')?;
            &after[slash + 1..]
        }
        None => {
            let colon = url.find(':')?;
            &url[colon + 1..]
        }
    };

    let path = rest.trim_start_matches('/').trim_end_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path);
    let mut parts = path.split('/').filter(|part| !part.is_empty());
    let owner = parts.next()?;
    let repo = parts.next()?;
    if !is_plain_component(owner) || !is_plain_component(repo) {
        return None;
    }
    Some(format!("{owner}/{repo}"))
}

/// Whether `component` is a plain path segment safe to interpolate into a read.
fn is_plain_component(component: &str) -> bool {
    !component.is_empty()
        && component != "."
        && component != ".."
        && component
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Whether the rulesets answer carries at least one actively enforced branch
/// ruleset.
///
/// An unreadable answer is `false`, and `false` here does not mean unprotected —
/// it means this endpoint said nothing decisive, and the caller goes on to ask
/// the other one.
fn active_branch_ruleset(body: &str) -> bool {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body) else {
        return false;
    };
    let Some(entries) = parsed.as_array() else {
        return false;
    };
    entries.iter().any(|entry| {
        entry.get("target").and_then(serde_json::Value::as_str) == Some("branch")
            && entry.get("enforcement").and_then(serde_json::Value::as_str) == Some("active")
    })
}

/// The `default_branch` field of a repository answer.
fn default_branch_of(body: &str) -> Option<String> {
    let parsed = serde_json::from_str::<serde_json::Value>(body).ok()?;
    let branch = parsed.get("default_branch")?.as_str()?;
    if !branch.is_empty()
        && branch
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '/'))
        && !branch.contains("..")
    {
        return Some(branch.to_string());
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_states_carry_stable_identifiers() {
        assert_eq!(ProtectionState::Protected.as_str(), "protected");
        assert_eq!(ProtectionState::Unprotected.as_str(), "unprotected");
        assert_eq!(
            ProtectionState::unknown("because the sky fell").as_str(),
            "unknown"
        );
    }

    #[test]
    fn both_unprotected_and_unknown_warrant_a_warning_and_protected_does_not() {
        assert!(ProtectionState::Unprotected.is_warning());
        assert!(ProtectionState::unknown("no client").is_warning());
        assert!(!ProtectionState::Protected.is_warning());
    }

    #[test]
    fn an_unknown_carries_its_reason_and_the_known_states_carry_none() {
        assert_eq!(
            ProtectionState::unknown("no client was installed").reason(),
            Some("no client was installed")
        );
        assert_eq!(ProtectionState::Protected.reason(), None);
        assert_eq!(ProtectionState::Unprotected.reason(), None);
    }

    #[test]
    fn a_reason_passes_through_the_redaction_table_at_its_one_constructor() {
        let state = ProtectionState::unknown(
            "the client answered ghp_0123456789abcdefghijklmnopqrstuvwxyz",
        );

        let reason = state.reason().expect("an unknown carries a reason");
        assert!(
            !reason.contains("ghp_0123456789abcdefghijklmnopqrstuvwxyz"),
            "a credential-shaped string in a probe reason must not survive into a \
             log sink, got: {reason}"
        );
    }

    #[test]
    fn the_not_probed_state_is_unknown_and_says_why_rather_than_guessing() {
        let state = not_probed();

        assert_eq!(state.as_str(), "unknown");
        let reason = state.reason().expect("an unknown carries a reason");
        assert!(
            reason.contains("dry run contacts no network"),
            "the reason has to name the constraint that produced it, got: {reason}"
        );
    }

    #[test]
    fn only_github_com_and_its_subdomains_are_treated_as_a_probeable_host() {
        assert!(is_github_host("github.com"));
        assert!(is_github_host("api.github.com"));
        // The look-alike is the case worth pinning: `contains` would say yes.
        assert!(!is_github_host("github.com.evil.example"));
        assert!(!is_github_host("gitlab.com"));
        assert!(!is_github_host("git.example.internal"));
    }

    #[test]
    fn a_slug_is_recovered_from_every_url_shape_git_accepts() {
        for (url, expected) in [
            ("https://github.com/owner/repo.git", "owner/repo"),
            ("https://github.com/owner/repo", "owner/repo"),
            ("ssh://git@github.com/owner/repo.git", "owner/repo"),
            ("git@github.com:owner/repo.git", "owner/repo"),
            ("git@github.com:owner/repo", "owner/repo"),
        ] {
            assert_eq!(repo_slug(url).as_deref(), Some(expected), "slug for {url}");
        }
    }

    #[test]
    fn a_slug_component_that_could_aim_the_read_elsewhere_is_refused() {
        for hostile in [
            "https://github.com/../repo.git",
            "https://github.com/owner/../repo",
            "https://github.com/own er/repo",
            "https://github.com/owner",
            "https://github.com/",
        ] {
            assert!(
                repo_slug(hostile).is_none(),
                "{hostile} must not yield a slug that gets interpolated into a read"
            );
        }
    }

    #[test]
    fn an_actively_enforced_branch_ruleset_is_the_only_shape_that_counts() {
        let active = r#"[{"id":1,"target":"branch","enforcement":"active"}]"#;
        assert!(active_branch_ruleset(active));

        // Evaluate-only is a ruleset that enforces nothing; a tag ruleset does
        // not protect a branch; an empty list is the ordinary unprotected shape.
        let evaluate = r#"[{"id":1,"target":"branch","enforcement":"evaluate"}]"#;
        let tags = r#"[{"id":2,"target":"tag","enforcement":"active"}]"#;
        assert!(!active_branch_ruleset(evaluate));
        assert!(!active_branch_ruleset(tags));
        assert!(!active_branch_ruleset("[]"));
        // An unreadable answer is never a verdict.
        assert!(!active_branch_ruleset("<html>gateway timeout</html>"));
    }

    #[test]
    fn the_default_branch_is_read_and_a_traversal_shaped_one_is_refused() {
        assert_eq!(
            default_branch_of(r#"{"default_branch":"main"}"#).as_deref(),
            Some("main")
        );
        assert_eq!(
            default_branch_of(r#"{"default_branch":"release/v2"}"#).as_deref(),
            Some("release/v2")
        );
        assert_eq!(default_branch_of(r#"{"default_branch":"../../x"}"#), None);
        assert_eq!(default_branch_of(r#"{"name":"repo"}"#), None);
        assert_eq!(default_branch_of("not json at all"), None);
    }

    #[test]
    fn an_answer_that_says_the_resource_is_absent_is_told_apart_from_a_failure() {
        assert!(says_absent("gh: Branch not protected (HTTP 404)"));
        assert!(says_absent("gh: Not Found (HTTP 404)"));
        assert!(!says_absent("dial tcp: lookup github.com: no such host"));
        assert!(!says_absent("gh: authentication required (HTTP 401)"));
    }

    #[test]
    fn the_clients_complaint_is_reduced_to_its_first_meaningful_line() {
        assert_eq!(
            first_line("\n\n  gh: Not Found (HTTP 404)\nUsage: gh api ...\n"),
            "gh: Not Found (HTTP 404)"
        );
        assert_eq!(first_line("   \n"), "no detail was reported");
    }
}
