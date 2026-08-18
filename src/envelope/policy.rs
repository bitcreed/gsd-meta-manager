//! Pure decision functions over refspecs and the reserved branch namespace.
//!
//! **No I/O, no processes, no network.** Everything here is a total function of
//! its arguments, which is what makes the envelope's decisions exhaustively
//! unit-testable without a repository, a child process or a credential — the
//! same property `journal::is_plain_path_component` was written for, and for the same
//! reason: a decision that needs the world to answer cannot be tested against
//! the world's hostile cases.
//!
//! Everything that *acts* on these verdicts lives in [`super::hooks`], with one
//! named exception: [`resolve_push_context`] reads a repository's own config to
//! answer the one question argv cannot ("where would a bare `git push` go?"),
//! and it is the only function here that touches the world.

use crate::config::{CredentialSource, DriverOptIn};
use std::path::Path;

/// The reserved push namespace root (D-05).
///
/// GitHub Copilot's precedent is a flat `copilot/` prefix. The extra `<alias>`
/// segment is added because this tool drives N projects from one machine and one
/// credential, so an errant ref names the project it came from without a lookup.
pub const DEFAULT_NAMESPACE_ROOT: &str = "refs/heads/gsd-auto/";

/// The ref prefix every branch namespace must sit under.
const REFS_HEADS: &str = "refs/heads/";

/// First segments a namespace may never claim, at any casing git treats as
/// distinct. `HEAD` is included because `refs/heads/HEAD/` is a working shape
/// that would make every push look like a detached-head update.
const RESERVED_FIRST_SEGMENTS: &[&str] = &["main", "master", "HEAD"];

/// D-24's park reason for a push whose destination is outside the namespace.
pub const REASON_PUSH_OUTSIDE_NAMESPACE: &str = "push_outside_namespace";

/// D-24's park reason for a destructive git operation the envelope refuses.
pub const REASON_FORCE_PUSH_BLOCKED: &str = "force_push_blocked";

/// D-24's park reason for a command that would make an enforcement layer
/// unreachable.
pub const REASON_HOOK_BYPASS_BLOCKED: &str = "hook_bypass_blocked";

/// D-24's park reason for a credential found by the pre-push scan (SAFE-03).
pub const REASON_SECRET_DETECTED: &str = "secret_detected";

/// D-24's park reason for a PR attempt beyond the configured cap (SAFE-06).
pub const REASON_PR_CAP_EXCEEDED: &str = "pr_cap_exceeded";

/// D-24's park reason for a run that has no credential to push with (SAFE-05).
pub const REASON_CREDENTIAL_UNAVAILABLE: &str = "credential_unavailable";

/// D-24's park reason for an envelope input the enforcement point cannot read.
pub const REASON_ENVELOPE_ASSERTION_FAILED: &str = "envelope_assertion_failed";

/// The namespace an alias gets when the project configures none.
///
/// Always a prefix, always ending in `/`: see [`validate_namespace`] for why the
/// trailing slash is load-bearing rather than cosmetic.
pub fn default_namespace(alias: &str) -> String {
    format!("{DEFAULT_NAMESPACE_ROOT}{alias}/")
}

/// A configured namespace, or `None` if its shape would weaken the control.
///
/// D-05's rules, each of which exists to refuse a specific way of accidentally
/// disabling the boundary:
///
/// - **non-empty** and **under `refs/heads/`** — a namespace that is not a
///   branch prefix does not constrain a branch push at all;
/// - **at least two `/`-separated segments below `refs/heads/`** — a single
///   segment like `refs/heads/x/` is legal but leaves no room for the per-alias
///   segment the whole scheme is built on;
/// - **the first segment below `refs/heads/` is not `main`, `master` or
///   `HEAD`** — the branches the envelope exists to protect;
/// - **it ends in `/`** — so it is a *prefix* and not one branch. Without this,
///   a user configuring `refs/heads/` would silently allow every branch, which
///   is disabling the control by typo.
///
/// Returns the candidate unchanged on success. It deliberately does **not**
/// normalise: a namespace that has to be repaired before it is usable is a
/// namespace whose written form differs from its effect, and the whole point of
/// the shape rules is that what the user typed is what applies.
pub fn validate_namespace(candidate: &str) -> Option<String> {
    let below = candidate.strip_prefix(REFS_HEADS)?;
    if !below.ends_with('/') {
        return None;
    }
    let segments: Vec<&str> = below.trim_end_matches('/').split('/').collect();
    if segments.len() < 2 {
        return None;
    }
    if segments.iter().any(|segment| segment.is_empty()) {
        return None;
    }
    if RESERVED_FIRST_SEGMENTS.contains(&segments[0]) {
        return None;
    }
    Some(candidate.to_string())
}

/// What the envelope decided about one push destination.
///
/// `reason` is `&'static str` on purpose: every reason is a member of D-24's
/// fixed taxonomy, so a caller cannot invent one at the call site and a reader
/// can find every producer of a given reason by grepping the constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushVerdict {
    /// The destination is inside the reserved namespace.
    Allow,
    /// The destination is outside it, or could not be read.
    Refuse {
        /// A member of D-24's reason taxonomy.
        reason: &'static str,
        /// The destination ref as git reported it, for a legible message.
        ref_name: String,
    },
}

/// Classify one push destination against a namespace prefix.
///
/// A plain prefix test, and that is the whole decision. It is written as a
/// separate function rather than inlined at the hook because this is the
/// sentence the tracer test flips to prove the refusal is load-bearing: making
/// this return [`PushVerdict::Allow`] unconditionally must turn the suite red.
pub fn classify_push_ref(dest_ref: &str, namespace: &str) -> PushVerdict {
    if dest_ref.starts_with(namespace) {
        PushVerdict::Allow
    } else {
        PushVerdict::Refuse {
            reason: REASON_PUSH_OUTSIDE_NAMESPACE,
            ref_name: dest_ref.to_string(),
        }
    }
}

/// The park-reason taxonomy this phase owns (D-24).
///
/// Modelled on [`crate::journal::JournalEvent::Diagnostic`]'s `code` field,
/// documented there as *"a short stable identifier for the condition"* — which
/// is exactly what a park reason is. Two conventions for one idea is how a
/// later reader ends up grepping for the wrong string.
///
/// **Phase 19 owns *producing* these reasons. Phase 20 owns what happens to a
/// run that carries one.** No resume, retry or backoff logic belongs next to
/// this type: a reason that also decides the recovery has to be edited every
/// time the recovery changes, and then the identifier is no longer stable.
///
/// The set is closed on purpose. When a refusal does not have its own reason —
/// `git stash`, `update-ref`, `filter-branch` — it is reported under
/// [`ParkReason::ForcePushBlocked`] as the destructive-git family, and the
/// specific command travels in the verdict's `detail`. That is stated here
/// rather than left for a reader to infer from a surprising string, because
/// inventing an eighth reason at a call site is how a taxonomy stops being one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParkReason {
    /// A push whose destination is outside the reserved namespace (D-05).
    PushOutsideNamespace,
    /// A destructive git operation: force push, delete, or history rewrite.
    ForcePushBlocked,
    /// A command that would make an enforcement layer unreachable.
    HookBypassBlocked,
    /// The pre-push scan found a credential (SAFE-03).
    SecretDetected,
    /// A PR attempt beyond the configured cap (SAFE-06).
    PrCapExceeded,
    /// The run has no credential configured and cannot push (SAFE-05).
    CredentialUnavailable,
    /// An envelope input the enforcement point could not read.
    EnvelopeAssertionFailed,
}

impl ParkReason {
    /// The stable snake_case identifier a later reader greps for.
    ///
    /// Every arm returns one of the `REASON_*` constants above rather than a
    /// fresh literal, so the constant and the enum can never disagree and
    /// `grep REASON_FORCE_PUSH_BLOCKED` finds every producer.
    pub fn as_str(&self) -> &'static str {
        match self {
            ParkReason::PushOutsideNamespace => REASON_PUSH_OUTSIDE_NAMESPACE,
            ParkReason::ForcePushBlocked => REASON_FORCE_PUSH_BLOCKED,
            ParkReason::HookBypassBlocked => REASON_HOOK_BYPASS_BLOCKED,
            ParkReason::SecretDetected => REASON_SECRET_DETECTED,
            ParkReason::PrCapExceeded => REASON_PR_CAP_EXCEEDED,
            ParkReason::CredentialUnavailable => REASON_CREDENTIAL_UNAVAILABLE,
            ParkReason::EnvelopeAssertionFailed => REASON_ENVELOPE_ASSERTION_FAILED,
        }
    }
}

/// What the envelope decided about one whole `git` invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitVerdict {
    /// Nothing on the denied list was found. See [`classify_git`] for what that
    /// does and does not mean.
    Allow,
    /// The command is refused, with a reason a run can park on and a detail a
    /// human can act on.
    Refuse {
        /// The member of D-24's taxonomy this refusal parks under.
        reason: ParkReason,
        /// One line naming the exact token that caused the refusal. Never the
        /// whole argv: a detail that quotes the command back is a detail that
        /// can carry a secret into the journal (SAFE-04).
        detail: String,
    },
}

/// The two facts [`classify_git`] cannot compute from argv alone.
///
/// Passed in rather than resolved inside, and that is the whole reason the
/// classifier is a pure function: the namespace comes from configuration and
/// the destinations come from a repository, so a classifier that fetched them
/// itself could only be tested against a real repository.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitContext {
    /// The reserved namespace prefix, already validated by
    /// [`validate_namespace`] or defaulted by [`default_namespace`].
    pub namespace: String,
    /// Fully-qualified destination refs a `push` with no refspec would produce,
    /// as resolved by [`resolve_push_context`]. **Empty means "could not be
    /// resolved", which is a refusal, never an allow.**
    pub resolved_push_dests: Vec<String>,
}

/// Classify one `git` invocation against D-08's denied set.
///
/// **This is a denylist over git verbs, not an allowlist.** An allowlist is the
/// stronger control and it was declined, for a concrete reason rather than a
/// stylistic one: a GSD skill runs whatever plumbing command it needs —
/// `cat-file`, `for-each-ref`, `merge-base`, `rev-list` — and an allowlist would
/// break the first one nobody anticipated, which is how a safety control gets
/// switched off. So an unlisted verb is allowed here, and the gaps that choice
/// leaves are covered elsewhere: the `pre-push` and `pre-commit` hooks observe
/// what git actually does, regardless of which verb reached this function. That
/// is what makes the denylist's inevitable gaps non-fatal rather than fatal.
///
/// **The deferred revisit condition, so this is not a permanent excuse:** once
/// the Phase 20 router narrows which commands a driven run actually issues, the
/// allowlist becomes computable from that set instead of guessed, and this
/// function should become one.
///
/// Four argv-shape rules, each of which is how a denylist becomes decoration if
/// it is skipped:
///
/// 1. **Tokens, never a joined string.** `git  push   --force` and
///    `git push --force` are the same argv and must be the same verdict.
/// 2. **`--opt=value` and `--opt value` are the same option.**
/// 3. **Bundled short flags are unbundled**, so the `-f` inside `-fu` is found.
/// 4. **Leading `git -c KEY=VALUE` / `--config-env` options are scanned before
///    the verb**, because they apply to whatever verb follows — and `-c
///    core.hooksPath=…` is the one form that outranks the env-injected setting
///    D-09 relies on.
///
/// Config keys are compared case-insensitively, because git accepts
/// `core.hookspath` and a case-sensitive check would be a one-keystroke bypass.
pub fn classify_git(argv: &[&str], ctx: &GitContext) -> GitVerdict {
    let (index, refusal) = scan_leading(argv);
    if let Some(refusal) = refusal {
        return refusal;
    }

    let Some(verb) = argv.get(index) else {
        // `git` with no subcommand prints usage. There is nothing to refuse.
        return GitVerdict::Allow;
    };
    let rest = &argv[index + 1..];

    match *verb {
        "push" => classify_push(rest, ctx),
        "config" => classify_config(rest),
        "stash" => refuse(
            ParkReason::ForcePushBlocked,
            "`git stash` removes the human's uncommitted work from the tree, where \
             `git fsck --lost-found` is the only recovery"
                .to_string(),
        ),
        "update-ref" | "filter-branch" | "filter-repo" => refuse(
            ParkReason::ForcePushBlocked,
            format!("`git {verb}` rewrites history or moves a ref out from under it"),
        ),
        "reflog" => classify_reflog(rest),
        "symbolic-ref" => classify_symbolic_ref(rest),
        // A denylist: an unlisted verb is allowed. See this function's doc for
        // why, and for what covers the gap.
        _ => GitVerdict::Allow,
    }
}

/// Build a refusal. A free function so every producer reads the same and the
/// `detail` is always a sentence rather than a repeated `format!` shape.
fn refuse(reason: ParkReason, detail: String) -> GitVerdict {
    GitVerdict::Refuse { reason, detail }
}

/// Walk the leading `git` options (rule 4) and report where the verb starts,
/// plus the refusal any of them earns.
///
/// **One scan, two callers.** [`classify_git`] needs the refusal and the index;
/// [`push_needs_resolved_dests`] needs only the index, and it is asked by the
/// `PreToolUse` guard *before* classification in order to decide whether this
/// command is the one case that has to read the repository. A second copy of
/// this loop would be a second thing to keep in step with git's own option
/// grammar, and the day they drift is the day the guard resolves a context for
/// the wrong argv — or fails to resolve one for the right argv, which reads as
/// an unresolvable destination and refuses a push that was inside the namespace.
fn scan_leading(argv: &[&str]) -> (usize, Option<GitVerdict>) {
    let mut index = 0;

    while index < argv.len() {
        let token = argv[index];
        if !token.starts_with('-') || token == "-" {
            break;
        }
        if token == "--" {
            index += 1;
            break;
        }
        let (assignment, consumed) = leading_git_option(argv, index);
        if let Some(assignment) = assignment {
            if is_hooks_path_key(config_key_of(assignment)) {
                return (
                    index,
                    Some(refuse(
                        ParkReason::HookBypassBlocked,
                        format!(
                            "`git -c {assignment}` sets core.hooksPath at command-line \
                             precedence, which is the one form that outranks the envelope's own \
                             env-injected setting (D-09)"
                        ),
                    )),
                );
            }
        }
        index += consumed;
    }

    (index, None)
}

/// Whether this argv is a `git push` that carries no refspec, and therefore
/// needs [`GitContext::resolved_push_dests`] filled in from the repository.
///
/// **Latency, and specifically the reproduced 180-240 second `PreToolUse` hang
/// at `src/executor/mod.rs:225-239`, is why this predicate exists.** Resolving
/// the push context shells out to git; the guard runs synchronously on the
/// agent's critical path, so it must ask that question for the one command shape
/// that cannot be judged without it and for no other. Every other command —
/// including a `push` that names its refspec — is decided from argv alone.
///
/// A command that will be refused by [`scan_leading`] anyway answers `false`: it
/// needs no destination to be refused, and reading a repository to reach a
/// verdict already reached is the definition of latency spent for nothing.
pub fn push_needs_resolved_dests(argv: &[&str]) -> bool {
    let (index, refusal) = scan_leading(argv);
    if refusal.is_some() {
        return false;
    }
    if argv.get(index).copied() != Some("push") {
        return false;
    }
    match push_operands(&argv[index + 1..]) {
        // git's own operand order: the first is the repository, the rest are
        // refspecs. No second operand means no refspec.
        Ok(operands) => operands.len() <= 1,
        // A denied flag refuses without consulting a destination.
        Err(_) => false,
    }
}

/// Leading `git` options that consume a **separate** following token.
///
/// `--exec-path` is deliberately absent: without `=` it prints a path and runs
/// nothing, so treating the next token as its value would swallow the verb and
/// hand the classifier an argv with no command in it.
const GIT_GLOBAL_VALUE_OPTS: &[&str] = &[
    "-C",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--super-prefix",
];

/// One leading option: its config assignment if it carries one, and how many
/// tokens it occupies.
fn leading_git_option<'a>(argv: &[&'a str], index: usize) -> (Option<&'a str>, usize) {
    let token = argv[index];

    if token == "-c" || token == "--config-env" {
        return (argv.get(index + 1).copied(), 2);
    }
    if let Some(rest) = token.strip_prefix("--config-env=") {
        return (Some(rest), 1);
    }
    // git's short-option parser accepts `-ckey=value` with no space.
    if let Some(rest) = token.strip_prefix("-c") {
        if !rest.is_empty() && !token.starts_with("--") {
            return (Some(rest), 1);
        }
    }
    if GIT_GLOBAL_VALUE_OPTS.contains(&token) {
        return (None, 2);
    }
    (None, 1)
}

/// The key half of a `KEY=VALUE` assignment.
///
/// A bare `KEY` is returned whole: `git -c core.hooksPath` with no value sets
/// the key to boolean true, which is still a write of that key.
fn config_key_of(assignment: &str) -> &str {
    assignment
        .split_once('=')
        .map(|(key, _)| key)
        .unwrap_or(assignment)
}

/// Whether a config key names `core.hooksPath`, at any casing git accepts.
fn is_hooks_path_key(key: &str) -> bool {
    key.eq_ignore_ascii_case("core.hookspath")
}

/// Long push options that take a value, so their value is never read as a
/// refspec.
const PUSH_VALUE_OPTS: &[&str] = &["repo", "push-option", "receive-pack", "exec"];

/// D-08's denied push flags, and the reason each parks under.
fn denied_push_flag(name: &str) -> Option<(ParkReason, &'static str)> {
    match name {
        "force" => Some((ParkReason::ForcePushBlocked, "--force")),
        "force-with-lease" => Some((ParkReason::ForcePushBlocked, "--force-with-lease")),
        "force-if-includes" => Some((ParkReason::ForcePushBlocked, "--force-if-includes")),
        "mirror" => Some((ParkReason::ForcePushBlocked, "--mirror")),
        "delete" => Some((ParkReason::ForcePushBlocked, "--delete")),
        "no-verify" => Some((ParkReason::HookBypassBlocked, "--no-verify")),
        _ => None,
    }
}

fn classify_push(rest: &[&str], ctx: &GitContext) -> GitVerdict {
    let operands = match push_operands(rest) {
        Ok(operands) => operands,
        Err(refusal) => return refusal,
    };

    // git's own operand order: the first is the repository, the rest are
    // refspecs. A push with no refspec resolves through local config instead.
    let refspecs = operands.get(1..).unwrap_or_default();

    if refspecs.is_empty() {
        if ctx.resolved_push_dests.is_empty() {
            return refuse(
                ParkReason::PushOutsideNamespace,
                "this push carries no refspec and no destination could be resolved from local \
                 git config (see `PushPreview.note` for git's own account of why); an \
                 unresolvable destination is refused, never allowed"
                    .to_string(),
            );
        }
        for dest in &ctx.resolved_push_dests {
            if let PushVerdict::Refuse { reason, ref_name } = classify_push_ref(dest, &ctx.namespace)
            {
                return refuse(
                    ParkReason::PushOutsideNamespace,
                    format!(
                        "this push carries no refspec, and its resolved destination \
                         `{ref_name}` is outside `{}` ({reason})",
                        ctx.namespace
                    ),
                );
            }
        }
        return GitVerdict::Allow;
    }

    for spec in refspecs {
        if spec.starts_with('+') {
            return refuse(
                ParkReason::ForcePushBlocked,
                format!(
                    "the refspec `{spec}` is `+`-prefixed, which is a force push spelled as a \
                     refspec rather than as a flag"
                ),
            );
        }
        let (src, dst) = match spec.split_once(':') {
            Some((src, dst)) => (src, dst),
            // A lone ref names the same ref at both ends.
            None => (*spec, *spec),
        };
        if src.is_empty() || dst.is_empty() {
            return refuse(
                ParkReason::ForcePushBlocked,
                format!(
                    "the refspec `{spec}` has an empty half, which is how a push deletes a \
                     remote ref without naming `--delete`"
                ),
            );
        }
        let qualified = qualify_destination(dst);
        if let PushVerdict::Refuse { reason, ref_name } =
            classify_push_ref(&qualified, &ctx.namespace)
        {
            return refuse(
                ParkReason::PushOutsideNamespace,
                format!(
                    "the refspec `{spec}` resolves to `{ref_name}`, which is outside `{}` \
                     ({reason})",
                    ctx.namespace
                ),
            );
        }
    }

    GitVerdict::Allow
}

/// The operands of a `git push`, or the refusal one of its flags earns.
///
/// Split out from [`classify_push`] so [`push_needs_resolved_dests`] can ask
/// "does this push name a refspec?" through the **same** parser that judges it.
/// Value-taking flags are consumed explicitly here, because a value read as an
/// operand is how a denylist both false-refuses (`-o ci.skip` read as a
/// repository) and false-allows.
fn push_operands<'a>(rest: &[&'a str]) -> Result<Vec<&'a str>, GitVerdict> {
    let mut operands: Vec<&str> = Vec::new();
    let mut index = 0;
    let mut end_of_options = false;

    while index < rest.len() {
        let token = rest[index];

        if end_of_options || !token.starts_with('-') || token == "-" {
            operands.push(token);
            index += 1;
            continue;
        }
        if token == "--" {
            end_of_options = true;
            index += 1;
            continue;
        }

        if let Some(long) = token.strip_prefix("--") {
            // Rule 2: `--opt=value` and `--opt value` are the same option.
            let (name, has_inline_value) = match long.split_once('=') {
                Some((name, _)) => (name, true),
                None => (long, false),
            };
            if let Some((reason, spelling)) = denied_push_flag(name) {
                return Err(refuse(reason, denied_push_detail(reason, spelling)));
            }
            if PUSH_VALUE_OPTS.contains(&name) && !has_inline_value {
                index += 1;
            }
            index += 1;
            continue;
        }

        // Rule 3: unbundle short flags, so the `-f` inside `-fu` is found.
        let mut chars = token[1..].chars();
        while let Some(flag) = chars.next() {
            match flag {
                'f' => {
                    return Err(refuse(
                        ParkReason::ForcePushBlocked,
                        denied_push_detail(ParkReason::ForcePushBlocked, "-f"),
                    ))
                }
                'd' => {
                    return Err(refuse(
                        ParkReason::ForcePushBlocked,
                        denied_push_detail(ParkReason::ForcePushBlocked, "-d"),
                    ))
                }
                // `-o` takes a value: the rest of the bundle, or the next token.
                'o' => {
                    if chars.as_str().is_empty() {
                        index += 1;
                    }
                    break;
                }
                _ => {}
            }
        }
        index += 1;
    }

    Ok(operands)
}

/// One sentence per denied push flag, naming the consequence rather than the
/// rule.
fn denied_push_detail(reason: ParkReason, spelling: &str) -> String {
    match reason {
        ParkReason::HookBypassBlocked => format!(
            "`git push {spelling}` makes the pre-push hook unreachable, which is the layer that \
             observes what git actually does"
        ),
        _ => format!(
            "`git push {spelling}` can overwrite or remove a remote ref, which is the blast \
             radius SAFE-02 exists to bound"
        ),
    }
}

/// A destination ref as git would resolve it.
///
/// An unqualified name is a branch. Leaving it unqualified would make
/// `git push origin main` an unrecognised destination and therefore silently
/// allowed, which is the failure this function exists to prevent.
fn qualify_destination(dst: &str) -> String {
    if dst.starts_with("refs/") {
        dst.to_string()
    } else {
        format!("{REFS_HEADS}{dst}")
    }
}

/// `git config` flags that take a **separate** value, whose value must not be
/// mistaken for the key.
const CONFIG_VALUE_OPTS: &[&str] = &[
    "--file", "-f", "--blob", "--type", "-t", "--default", "--comment",
];

/// Flags whose presence makes the invocation a read.
const CONFIG_READ_OPTS: &[&str] = &[
    "--get",
    "--get-all",
    "--get-regexp",
    "--get-urlmatch",
    "--get-color",
    "--get-colorbool",
    "--list",
    "-l",
];

/// Flags whose presence makes the invocation a write.
const CONFIG_WRITE_OPTS: &[&str] = &[
    "--add",
    "--replace-all",
    "--unset",
    "--unset-all",
    "--rename-section",
    "--remove-section",
    "--edit",
    "-e",
];

/// git 2.46's subcommand spellings, which reach the same config file.
const CONFIG_WRITE_SUBCOMMANDS: &[&str] = &[
    "set",
    "unset",
    "add",
    "replace-all",
    "unset-all",
    "rename-section",
    "remove-section",
    "edit",
];
const CONFIG_READ_SUBCOMMANDS: &[&str] = &["get", "list"];

fn classify_config(rest: &[&str]) -> GitVerdict {
    let mut operands: Vec<&str> = Vec::new();
    let mut is_read = false;
    let mut is_write = false;
    let mut whole_file_write = false;
    let mut index = 0;

    while index < rest.len() {
        let token = rest[index];
        if !token.starts_with('-') || token == "-" {
            operands.push(token);
            index += 1;
            continue;
        }
        if token == "--" {
            operands.extend_from_slice(&rest[index + 1..]);
            break;
        }
        let name = token.split_once('=').map(|(n, _)| n).unwrap_or(token);
        if CONFIG_READ_OPTS.contains(&name) {
            is_read = true;
        }
        if CONFIG_WRITE_OPTS.contains(&name) {
            is_write = true;
            if name == "--edit" || name == "-e" {
                whole_file_write = true;
            }
        }
        if CONFIG_VALUE_OPTS.contains(&name) && !token.contains('=') {
            index += 1;
        }
        index += 1;
    }

    // The subcommand form puts the verb where the key would otherwise be.
    let mut key_operands = operands.as_slice();
    if let Some(first) = operands.first() {
        if CONFIG_WRITE_SUBCOMMANDS.contains(first) {
            is_write = true;
            if *first == "edit" {
                whole_file_write = true;
            }
            key_operands = &operands[1..];
        } else if CONFIG_READ_SUBCOMMANDS.contains(first) {
            is_read = true;
            key_operands = &operands[1..];
        }
    }

    // The classic form: `git config <key> <value>` is a write, `git config
    // <key>` alone prints the value and is a read.
    if !is_read && !is_write && key_operands.len() >= 2 {
        is_write = true;
    }

    if whole_file_write {
        return refuse(
            ParkReason::HookBypassBlocked,
            "`git config --edit` opens the whole config file for writing, so it can set \
             core.hooksPath without ever naming it"
                .to_string(),
        );
    }

    if is_write && !is_read {
        if let Some(key) = key_operands.first() {
            if is_hooks_path_key(key) {
                return refuse(
                    ParkReason::HookBypassBlocked,
                    format!(
                        "writing `{key}` moves the hook directory the envelope installed into, \
                         which disarms the only layer that observes what git actually does"
                    ),
                );
            }
        }
    }

    GitVerdict::Allow
}

fn classify_reflog(rest: &[&str]) -> GitVerdict {
    let subcommand = rest.iter().find(|token| !token.starts_with('-'));
    match subcommand {
        Some(&sub @ ("delete" | "expire" | "drop")) => refuse(
            ParkReason::ForcePushBlocked,
            format!(
                "`git reflog {sub}` destroys the reflog, which is the recovery path for every \
                 other destructive git operation"
            ),
        ),
        // `git reflog` and `git reflog show` are reads.
        _ => GitVerdict::Allow,
    }
}

fn classify_symbolic_ref(rest: &[&str]) -> GitVerdict {
    let mut operands = 0;
    let mut deleting = false;
    for token in rest {
        if token.starts_with('-') && *token != "-" {
            if *token == "-d" || *token == "--delete" {
                deleting = true;
            }
            continue;
        }
        operands += 1;
    }

    // Two operands is `symbolic-ref <name> <ref>`, which writes. One is a read.
    if deleting || operands >= 2 {
        refuse(
            ParkReason::ForcePushBlocked,
            "`git symbolic-ref` with a value repoints HEAD, which changes what every \
             subsequent commit and push means"
                .to_string(),
        )
    } else {
        GitVerdict::Allow
    }
}

/// Resolve the implicit push destinations for a repository — the **one** impure
/// function in this module.
///
/// It calls [`crate::state_reader::git_ops::push_refspecs`], which already
/// resolves every config key involved — its own doc enumerates them in order —
/// with no network and no credential. **The key names are deliberately not
/// repeated here.** A second copy of that list is a second thing to keep in
/// step, and a second implementation of the resolution would be a second thing
/// that can drift from the preview the user was shown; the whole value of the
/// dry-run preview is that it describes the push the envelope will judge.
///
/// An unresolvable destination yields an empty `resolved_push_dests`, which
/// [`classify_git`] treats as a refusal. Unknown is never allowed.
pub fn resolve_push_context(project_root: &Path, namespace: &str) -> GitContext {
    let preview = crate::state_reader::git_ops::push_refspecs(project_root);

    let resolved_push_dests = preview
        .refspecs
        .iter()
        .filter_map(|refspec| refspec.split_once(':').map(|(_, dst)| dst.trim().to_string()))
        .filter(|dst| !dst.is_empty())
        .collect();

    GitContext {
        namespace: namespace.to_string(),
        resolved_push_dests,
    }
}

/// The `--disallowedTools` patterns for the driven child (D-06 layer 1).
///
/// Layer 1 of the enforcement stack: cheapest, fires before the tool runs, and
/// cannot be silently dropped because it is argv rather than a file that could
/// fail validation. It is **not** a substitute for [`classify_git`] — this list
/// is matched by the agent runtime against a command line, while the classifier
/// is applied to parsed argv by the `PreToolUse` guard.
///
/// The `Write`/`Edit` entries are not decoration either: the driven
/// repository's project-tier `.claude/settings.json` **is** loaded by the child
/// (`--setting-sources project`) and **is** agent-writable, so the envelope
/// denies writing it rather than trusting it. A control whose carrier the agent
/// can edit is not a control.
pub fn disallowed_tools() -> Vec<String> {
    let mut patterns: Vec<String> = [
        "git push",
        "git stash",
        "git config",
        "git update-ref",
        "git reflog",
        "git filter-branch",
        "git filter-repo",
        "git symbolic-ref",
    ]
    .iter()
    .map(|verb| format!("Bash({verb}:*)"))
    .collect();

    patterns.push("Write(.claude/**)".to_string());
    patterns.push("Edit(.claude/**)".to_string());
    patterns
}

/// The default rolling-24h PR cap: **3**.
///
/// **Arbitrary, and recorded as arbitrary rather than presented as derived.**
/// PITFALLS suggests it explicitly as a starting number, not as a measurement.
/// A named constant with the reason beside it is what stops a later reader from
/// reverse-engineering a justification that never existed.
pub const DEFAULT_PR_CAP_PER_24H: u32 = 3;

/// The default per-run PR cap: **1**. Arbitrary in the same way as
/// [`DEFAULT_PR_CAP_PER_24H`], and for the same recorded reason.
pub const DEFAULT_PR_CAP_PER_RUN: u32 = 1;

/// Every envelope setting for one alias, with every default already decided.
///
/// **`Default` is deliberately NOT derived**, following the lesson
/// `config.rs:109-117` records for `Preferences::driver_max_concurrent`: a
/// derived `Default` would make each cap `u32::default()`, which is `0`, which
/// here means *"no PR may ever be opened"* — a total denial of service wearing
/// the costume of a default. The constants above are the defaults, and
/// [`EnvelopePolicy::resolve`] is the only place they are applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopePolicy {
    /// The validated namespace every push from this alias must sit under.
    pub namespace: String,
    /// PRs allowed in a rolling 24 hours.
    pub pr_cap_per_24h: u32,
    /// PRs allowed in a single run.
    pub pr_cap_per_run: u32,
    /// Where the run's credential comes from, or `None` for **no credential**,
    /// which is the default and means every push fails closed (D-18).
    pub credential: Option<CredentialSource>,
}

impl EnvelopePolicy {
    /// Resolve one project's envelope settings — **the only place an envelope
    /// default is decided** (D-30).
    ///
    /// That is the whole point of the function existing: with four
    /// `Option` fields and three consumers, a default applied at the point of
    /// use is a default applied differently at each point of use, and the first
    /// symptom is a run confined to a namespace the preview did not show.
    ///
    /// **An invalid configured namespace degrades to the safe default and
    /// warns; it never widens the control.** A user who types `refs/heads/` has
    /// written something that would allow every branch, and honouring it would
    /// be disabling the boundary by typo. Falling back is the only direction to
    /// be wrong in, and the warning is what keeps the fallback from being
    /// silent.
    pub fn resolve(alias: &str, opt_in: &DriverOptIn) -> EnvelopePolicy {
        let namespace = match opt_in.branch_namespace.as_deref() {
            Some(configured) => match validate_namespace(configured) {
                Some(valid) => valid,
                None => {
                    tracing::warn!(
                        alias,
                        configured,
                        "the configured branch namespace fails the shape rules that keep it a \
                         boundary, so the default applies instead",
                    );
                    default_namespace(alias)
                }
            },
            None => default_namespace(alias),
        };

        EnvelopePolicy {
            namespace,
            pr_cap_per_24h: opt_in.pr_cap_per_24h.unwrap_or(DEFAULT_PR_CAP_PER_24H),
            pr_cap_per_run: opt_in.pr_cap_per_run.unwrap_or(DEFAULT_PR_CAP_PER_RUN),
            credential: opt_in.credential.clone(),
        }
    }
}

/// One word of a shell command line, with the three facts a classifier needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    /// The word with its quoting removed, as the shell would pass it to `execve`.
    pub text: String,
    /// Whether the word is an unquoted control operator (`;`, `&&`, `|`, …).
    pub operator: bool,
    /// Whether any part of the word is subject to expansion — an unquoted or
    /// double-quoted `$`, or a backtick. **The value is unknowable here**, which
    /// is the whole reason the flag exists rather than an attempt to evaluate it.
    pub expansion: bool,
}

/// The shell control operators that end one simple command and begin the next.
///
/// `>` and `<` are deliberately absent: a redirection does not start a new
/// command, so treating it as a separator would hide the command it redirects.
/// It stays an ordinary word and travels into the classifier with the rest.
const SEPARATORS: &[&str] = &[";", "&&", "||", "|", "&", "\n", "(", ")", "{", "}"];

/// Split a shell command line into words, POSIX quoting rules applied.
///
/// **This is layer 2's weak joint, and the doc says so rather than implying
/// otherwise.** A quoting-aware split handles the honest cases: single quotes,
/// double quotes with backslash escapes, backslash-escaped separators, and
/// arbitrary runs of whitespace — so `git  push   --force` and `git push -f`
/// arrive at the classifier as the same shape of argv and reach the same
/// verdict. It does **not** handle, and cannot:
///
/// - a verb assembled from a shell variable (`g=push; git $g --force`),
/// - `eval`, or any other construct that builds a command at run time,
/// - a base64-decoded payload piped into a shell,
/// - a script the agent writes to a file and then runs.
///
/// The correct framing, recorded here rather than left to be inferred: **layer 2
/// raises the cost of an accident to near-certain detection, and layer 3 — the
/// `pre-push` hook, which sees what git actually does regardless of how git was
/// invoked — is what covers deliberate evasion of layer 2.** Up to the point
/// where layer 3 is itself evaded, at which point the remote's own ruleset is
/// the only remaining answer, which is why D-27's server-side recommendation is
/// this phase's conclusion rather than its footnote.
///
/// What this function *will* do about the cases it cannot recover is return
/// `None`: an unterminated quote and a trailing line-continuation backslash are
/// both inputs whose word boundaries are not knowable, and **the guard denies
/// whatever it cannot parse**. Never allow-by-default; a splitter that guessed
/// would be a control that guessed.
///
/// **No crate was added for this.** A hand-rolled splitter with an exhaustive
/// test table is what keeps this phase's dependency set unchanged, which is the
/// mitigation for the package-legitimacy threat the phase register carries.
///
/// Operators appear in the returned vector as their own words, so a caller that
/// wants one simple command wants [`split_segments`] instead.
pub fn split_command(cmd: &str) -> Option<Vec<String>> {
    Some(
        tokenize(cmd)?
            .into_iter()
            .map(|token| token.text)
            .collect(),
    )
}

/// Split a shell command line into the simple commands it is composed of.
///
/// **A single-command splitter would be a hole rather than a control.**
/// `echo hi && git push --force` has `echo` as its first word, so a guard that
/// classified only the first command would look at `echo` and allow the force
/// push sitting behind the `&&`. Every segment is classified.
pub fn split_segments(cmd: &str) -> Option<Vec<Vec<Token>>> {
    let tokens = tokenize(cmd)?;
    let mut segments: Vec<Vec<Token>> = Vec::new();
    let mut current: Vec<Token> = Vec::new();

    for token in tokens {
        if token.operator {
            if !current.is_empty() {
                segments.push(std::mem::take(&mut current));
            }
            continue;
        }
        current.push(token);
    }
    if !current.is_empty() {
        segments.push(current);
    }

    Some(segments)
}

/// The quoting state machine behind [`split_command`] and [`split_segments`].
fn tokenize(cmd: &str) -> Option<Vec<Token>> {
    let mut tokens: Vec<Token> = Vec::new();
    let mut text = String::new();
    let mut started = false;
    let mut expansion = false;
    let mut chars = cmd.chars().peekable();

    // A word ends; push it if one was started at all. `started` distinguishes
    // an empty word that was written (`""`) from no word at all.
    macro_rules! flush {
        () => {
            if started {
                tokens.push(Token {
                    text: std::mem::take(&mut text),
                    operator: false,
                    expansion,
                });
                started = false;
                expansion = false;
            }
        };
    }

    while let Some(ch) = chars.next() {
        match ch {
            ' ' | '\t' | '\r' => flush!(),
            '\n' | ';' | '|' | '&' | '(' | ')' | '{' | '}' => {
                flush!();
                // `&&` and `||` are one operator, not two. Which one it is does
                // not matter to a classifier that treats every separator alike,
                // but consuming both characters keeps the token list honest.
                let mut op = ch.to_string();
                if (ch == '&' || ch == '|') && chars.peek() == Some(&ch) {
                    chars.next();
                    op.push(ch);
                }
                tokens.push(Token {
                    text: op,
                    operator: true,
                    expansion: false,
                });
            }
            '\'' => {
                started = true;
                // Single quotes are literal all the way through, including `$`.
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(inner) => text.push(inner),
                        // An unterminated quote has no knowable word boundary.
                        None => return None,
                    }
                }
            }
            '"' => {
                started = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            // Only these four are escapes inside double quotes;
                            // every other backslash is a literal backslash, and
                            // a splitter that dropped it would change the word.
                            Some(esc @ ('"' | '\\' | '$' | '`')) => text.push(esc),
                            Some(other) => {
                                text.push('\\');
                                text.push(other);
                            }
                            None => return None,
                        },
                        Some(inner) => {
                            // Expansion still happens inside double quotes.
                            if inner == '$' || inner == '`' {
                                expansion = true;
                            }
                            text.push(inner);
                        }
                        None => return None,
                    }
                }
            }
            '\\' => {
                // A trailing backslash is a line continuation whose second half
                // this function was never given, so `?` refuses the whole input.
                let escaped = chars.next()?;
                started = true;
                text.push(escaped);
            }
            '#' if !started => {
                // A comment runs to end of line; nothing after it is a command.
                for next in chars.by_ref() {
                    if next == '\n' {
                        break;
                    }
                }
            }
            _ => {
                started = true;
                if ch == '$' || ch == '`' {
                    expansion = true;
                }
                text.push(ch);
            }
        }
    }

    // The final word, pushed directly rather than through `flush!`: the macro's
    // bookkeeping assignments would be dead at this point, and a lint suppressed
    // is a lint that stops being read.
    if started {
        tokens.push(Token {
            text,
            operator: false,
            expansion,
        });
    }

    Some(tokens)
}

/// Whether a token is one of the control operators, by text.
///
/// Used by tests and by callers holding already-split words; the tokenizer
/// itself sets the flag directly.
pub fn is_separator(text: &str) -> bool {
    SEPARATORS.contains(&text)
}

/// The program a path names, without its directory.
///
/// `/usr/bin/gh` and `gh` are the same program, and a classifier that compared
/// whole strings would be defeated by an absolute path — which is not even an
/// evasion, it is what `command -v` prints.
pub fn program_name(program: &str) -> &str {
    program.rsplit('/').next().unwrap_or(program)
}

/// The forge and the recorded shape of a pull-request creation, or `None`.
///
/// The three forms D-19 names, matched on **tokens** rather than on a joined
/// string, because a substring match on `"pr create"` finds it inside a commit
/// message and misses it when an extra space is typed.
///
/// The returned label is what reaches [`super::ledger::LedgerEntry::command`],
/// and it is a **classification** rather than the command line: a ledger that
/// quoted the command back would be a file on disk that can hold a token
/// (SAFE-04).
pub fn pr_command_label(argv: &[&str]) -> Option<(&'static str, String)> {
    let program = program_name(argv.first()?);
    let rest = &argv[1..];

    match program {
        "gh" => {
            let words = subcommand_words(rest);
            match words.as_slice() {
                // `gh pr create …`
                ["pr", "create", ..] => Some(("github", "gh pr create".to_string())),
                // `gh api … /pulls` with a POST.
                ["api", ..] => {
                    if gh_api_posts_a_pull_request(rest) {
                        Some(("github", "gh api POST …/pulls".to_string()))
                    } else {
                        None
                    }
                }
                _ => None,
            }
        }
        "glab" => match subcommand_words(rest).as_slice() {
            ["mr", "create", ..] => Some(("gitlab", "glab mr create".to_string())),
            _ => None,
        },
        _ => None,
    }
}

/// Whether this argv creates a pull request (or a merge request) — D-19's three
/// shapes.
///
/// A thin `is_some` over [`pr_command_label`] so there is exactly one place the
/// three shapes are recognised. Two predicates would be two things to keep in
/// step, and the drift would be a cap that counts a form it does not refuse.
pub fn classify_pr_command(argv: &[&str]) -> bool {
    pr_command_label(argv).is_some()
}

/// Global flags of `gh`/`glab` that consume a **separate** following word.
///
/// The list matters for the same reason `PUSH_VALUE_OPTS` does: with `--repo`
/// unhandled, `gh --repo o/r pr create` yields the words `o/r pr create` and the
/// subcommand match on `["pr", "create"]` fails — a creation form that is not
/// counted, which is the under-counting direction the cap exists to prevent.
const FORGE_VALUE_OPTS: &[&str] = &["-R", "--repo", "--hostname"];

/// The non-flag words of a subcommand chain, in order.
///
/// Flags are skipped rather than terminating the scan, because `gh --repo o/r pr
/// create` is a legal invocation and a scan that stopped at the first `-` would
/// miss it.
fn subcommand_words<'a>(rest: &[&'a str]) -> Vec<&'a str> {
    let mut words = Vec::new();
    let mut index = 0;

    while index < rest.len() {
        let word = rest[index];
        if FORGE_VALUE_OPTS.contains(&word) {
            index += 2;
            continue;
        }
        if !word.starts_with('-') {
            words.push(word);
        }
        index += 1;
    }

    words
}

/// Long and short flags of `gh api` that take a **separate** following value.
const GH_API_VALUE_OPTS: &[&str] = &[
    "-X",
    "--method",
    "-f",
    "--raw-field",
    "-F",
    "--field",
    "-H",
    "--header",
    "-q",
    "--jq",
    "-t",
    "--template",
    "--input",
    "--hostname",
    "--cache",
    "-p",
    "--preview",
];

/// The flags whose mere presence makes `gh api` default to `POST`.
///
/// This is `gh`'s own documented behaviour, and it is the reason a method check
/// alone would be a hole: `gh api repos/o/r/pulls -f title=x` opens a pull
/// request and never spells `POST`.
const GH_API_IMPLIES_POST: &[&str] = &["-f", "--raw-field", "-F", "--field", "--input"];

fn gh_api_posts_a_pull_request(rest: &[&str]) -> bool {
    let mut method: Option<String> = None;
    let mut implies_post = false;
    let mut path: Option<&str> = None;
    let mut index = 0;
    // `api` itself is the first non-flag word; the endpoint is the second.
    let mut seen_api = false;

    while index < rest.len() {
        let token = rest[index];

        if let Some((name, value)) = token.split_once('=') {
            if name == "-X" || name == "--method" {
                method = Some(value.to_ascii_uppercase());
                index += 1;
                continue;
            }
        }
        if GH_API_IMPLIES_POST.contains(&token) {
            implies_post = true;
        }
        if GH_API_VALUE_OPTS.contains(&token) {
            if token == "-X" || token == "--method" {
                method = rest.get(index + 1).map(|m| m.to_ascii_uppercase());
            }
            index += 2;
            continue;
        }
        if token.starts_with('-') {
            index += 1;
            continue;
        }
        if !seen_api {
            seen_api = true;
        } else if path.is_none() {
            path = Some(token);
        }
        index += 1;
    }

    let posts = match method.as_deref() {
        Some(explicit) => explicit == "POST",
        None => implies_post,
    };

    posts && path.is_some_and(endpoint_is_pulls)
}

/// Whether an endpoint names the pull-request collection.
///
/// The query string and any trailing slash are stripped first, and the check is
/// on the final path segment: `repos/o/r/pulls`, a full
/// `https://api.github.com/repos/o/r/pulls`, and `/repos/o/r/pulls?state=open`
/// are the same endpoint. A single pull request (`…/pulls/7`) is **not** the
/// collection, and a `POST` to it is a review comment rather than a creation.
fn endpoint_is_pulls(endpoint: &str) -> bool {
    let path = endpoint.split(['?', '#']).next().unwrap_or(endpoint);
    path.trim_end_matches('/').rsplit('/').next() == Some("pulls")
}

/// The worktree-relocation directory a driven agent's `git add -A` sweeps up.
///
/// Named here rather than in a hook, because two enforcement points read it and
/// the whole reason [`forbidden_repo_path`] exists is that they must not be able
/// to disagree.
pub const WORKTREES_DIR: &str = ".claude/worktrees";

/// The planning directory the run journal lives under.
const PLANNING_DIR: &str = ".planning";

/// Every repository-relative prefix a driven commit may never carry (D-22).
///
/// The runs path is **derived** from [`crate::journal::RUNS_SUBDIR`], never
/// re-spelled as a literal. A second spelling of a path is a second thing to
/// keep in step, and the day they drift is the day this refusal stops covering
/// the directory it was written for.
pub fn forbidden_repo_prefixes() -> &'static [String] {
    static PREFIXES: std::sync::LazyLock<Vec<String>> = std::sync::LazyLock::new(|| {
        vec![
            WORKTREES_DIR.to_string(),
            format!("{PLANNING_DIR}/{}", crate::journal::RUNS_SUBDIR),
        ]
    });
    &PREFIXES
}

/// Whether a repository-relative path is one a driven commit may never carry.
///
/// **One predicate, two enforcement points** (D-22). The `pre-commit` hook asks
/// it about staged paths and the `pre-push` hook asks it about the paths the
/// commits being pushed touch. A second copy of these rules — one per hook —
/// would be two things to keep in step, and the drift would be invisible until
/// a swept worktree reached a remote.
///
/// `contains_nested_git` is supplied by the caller rather than probed here, for
/// the same reason [`GitContext`] is passed into [`classify_git`]: it is a fact
/// about a filesystem, and a predicate that went and looked could only be tested
/// against a real repository. The caller answers "does any ancestor directory of
/// this path itself contain a `.git` entry" — a nested repository or a linked
/// worktree, either of which is a whole second repository being swept into this
/// one's history.
///
/// The reason is [`ParkReason::ForcePushBlocked`], which is D-24's
/// destructive-git family rather than a fresh eighth member of a closed
/// taxonomy — the same mapping [`classify_git`] already uses for `stash` and
/// `update-ref`, and stated here so a reader meeting `force_push_blocked` on a
/// swept worktree is not misled.
pub fn forbidden_repo_path(rel: &Path, contains_nested_git: bool) -> Option<ParkReason> {
    if contains_nested_git {
        return Some(ParkReason::ForcePushBlocked);
    }

    // Normalised to forward slashes and stripped of a leading `./`, because git
    // reports paths that way and a caller on Windows would not.
    let text = rel.to_string_lossy().replace('\\', "/");
    let text = text.trim_start_matches("./").trim_start_matches('/');
    if text.is_empty() {
        return None;
    }

    for prefix in forbidden_repo_prefixes() {
        if text == prefix.as_str() || text.starts_with(&format!("{prefix}/")) {
            return Some(ParkReason::ForcePushBlocked);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `DriverOptIn` with every envelope field absent — the shape a
    /// pre-Phase-19 config deserialises to.
    fn bare_opt_in() -> DriverOptIn {
        DriverOptIn {
            opted_in_at: "2026-08-18T00:00:00Z".to_string(),
            claude_md_digest: None,
            branch_namespace: None,
            credential: None,
            pr_cap_per_24h: None,
            pr_cap_per_run: None,
        }
    }

    #[test]
    fn an_unconfigured_project_resolves_to_the_default_namespace_and_the_default_caps() {
        let policy = EnvelopePolicy::resolve("demo", &bare_opt_in());

        assert_eq!(policy.namespace, "refs/heads/gsd-auto/demo/");
        assert_eq!(
            policy.pr_cap_per_24h, DEFAULT_PR_CAP_PER_24H,
            "a derived `Default` would make this 0, which means no PR may ever be opened \
             (config.rs:109-117's lesson, applied to a cap)"
        );
        assert_eq!(policy.pr_cap_per_run, DEFAULT_PR_CAP_PER_RUN);
        assert!(
            policy.credential.is_none(),
            "no credential is the default, and it must never resolve to the user's ambient \
             one — SAFE-05 names that substitution as the failure"
        );
    }

    #[test]
    fn pr_caps_default_to_three_and_one_from_an_absent_key_and_from_an_explicit_none() {
        // The two arms `driver_max_concurrent_defaults_to_one_from_default_and_from_empty_json`
        // established: the struct path and the JSON path can disagree, and only
        // exercising both catches it.
        let from_none = EnvelopePolicy::resolve("demo", &bare_opt_in());
        assert_eq!(from_none.pr_cap_per_24h, 3);
        assert_eq!(from_none.pr_cap_per_run, 1);

        let from_json: DriverOptIn = serde_json::from_str(
            r#"{"opted_in_at":"2026-08-18T00:00:00Z","claude_md_digest":null}"#,
        )
        .expect("a pre-Phase-19 opt-in record must still deserialise");
        let from_absent_key = EnvelopePolicy::resolve("demo", &from_json);
        assert_eq!(
            from_absent_key.pr_cap_per_24h, 3,
            "an absent key and an explicit `None` must resolve identically, or an old config \
             silently gets a different cap from a new one"
        );
        assert_eq!(from_absent_key.pr_cap_per_run, 1);
    }

    #[test]
    fn an_invalid_configured_namespace_falls_back_to_the_default_rather_than_widening() {
        for hostile in [
            "refs/heads/",
            "refs/heads/main/",
            "refs/heads/x/",
            "gsd-auto/demo/",
            "refs/heads/gsd-auto/demo",
        ] {
            let opt_in = DriverOptIn {
                branch_namespace: Some(hostile.to_string()),
                ..bare_opt_in()
            };
            assert_eq!(
                EnvelopePolicy::resolve("demo", &opt_in).namespace,
                "refs/heads/gsd-auto/demo/",
                "a configured namespace that fails the shape rules must degrade to the \
                 default; honouring {hostile:?} would disable the boundary by typo"
            );
        }
    }

    #[test]
    fn a_valid_configured_namespace_is_honoured_verbatim() {
        let opt_in = DriverOptIn {
            branch_namespace: Some("refs/heads/bots/nightly/".to_string()),
            ..bare_opt_in()
        };
        assert_eq!(
            EnvelopePolicy::resolve("demo", &opt_in).namespace,
            "refs/heads/bots/nightly/",
            "a namespace that passes the shape rules applies exactly as written — the \
             validator deliberately does not normalise"
        );
    }

    /// The context every argv-shape test runs against: the default namespace
    /// for `demo`, and one resolved destination inside it, so a bare `push`
    /// has something legitimate to resolve to.
    fn ctx() -> GitContext {
        GitContext {
            namespace: default_namespace("demo"),
            resolved_push_dests: vec!["refs/heads/gsd-auto/demo/work".to_string()],
        }
    }

    /// A context whose implicit destination could not be resolved.
    fn unresolved_ctx() -> GitContext {
        GitContext {
            namespace: default_namespace("demo"),
            resolved_push_dests: Vec::new(),
        }
    }

    #[track_caller]
    fn assert_refused(argv: &[&str], reason: ParkReason, ctx: &GitContext) {
        match classify_git(argv, ctx) {
            GitVerdict::Refuse {
                reason: got,
                detail,
            } => assert_eq!(
                got,
                reason,
                "{argv:?} is refused for the wrong reason; a park reason that does not \
                 match the refusal sends a later reader to the wrong recovery. detail: {detail}"
            ),
            GitVerdict::Allow => panic!(
                "{argv:?} reached git unrefused — D-08 names this form as denied, so allowing \
                 it means the envelope is decoration"
            ),
        }
    }

    #[track_caller]
    fn assert_allowed(argv: &[&str], ctx: &GitContext) {
        assert_eq!(
            classify_git(argv, ctx),
            GitVerdict::Allow,
            "{argv:?} must be allowed — this is a denylist, and refusing an unanticipated \
             command breaks the GSD skills the driven run exists to execute"
        );
    }

    #[test]
    fn every_force_push_spelling_is_refused() {
        for argv in [
            vec!["push", "--force", "origin", "main"],
            vec!["push", "-f"],
            vec!["push", "-fu", "origin", "main"],
            vec!["push", "--force-with-lease"],
            vec!["push", "--force-with-lease=refs/heads/x:0123abc"],
            vec!["push", "--force-if-includes"],
            vec!["push", "--mirror"],
            vec!["push", "--delete", "origin", "x"],
            vec!["push", "-d", "origin", "x"],
        ] {
            assert_refused(&argv, ParkReason::ForcePushBlocked, &ctx());
        }
    }

    #[test]
    fn a_plus_prefixed_refspec_is_refused_because_it_is_a_force_push_by_another_spelling() {
        assert_refused(
            &["push", "origin", "+refs/heads/a:refs/heads/b"],
            ParkReason::ForcePushBlocked,
            &ctx(),
        );
    }

    #[test]
    fn push_no_verify_is_refused_as_a_hook_bypass() {
        assert_refused(
            &["push", "--no-verify"],
            ParkReason::HookBypassBlocked,
            &ctx(),
        );
    }

    #[test]
    fn every_writing_form_of_config_touching_core_hookspath_is_refused_at_every_scope() {
        for argv in [
            vec!["config", "--global", "core.hooksPath", "/tmp/x"],
            vec!["config", "--system", "core.hooksPath", "/tmp/x"],
            vec!["config", "--local", "core.hooksPath", "/tmp/x"],
            vec!["config", "--worktree", "core.hooksPath", "/tmp/x"],
            vec!["config", "--file", "other.cfg", "core.hooksPath", "/tmp/x"],
            vec!["config", "core.hooksPath", "/tmp/x"],
            // git accepts the key case-insensitively, so a case-sensitive
            // check would be a one-keystroke bypass.
            vec!["config", "--global", "core.hookspath", "/tmp/x"],
            vec!["config", "--global", "CORE.HOOKSPATH", "/tmp/x"],
            vec!["config", "--unset", "core.hooksPath"],
            vec!["config", "--replace-all", "core.hooksPath", "/tmp/x"],
            // git 2.46's subcommand spelling of the same write.
            vec!["config", "set", "--global", "core.hooksPath", "/tmp/x"],
            vec!["config", "unset", "core.hooksPath"],
        ] {
            assert_refused(&argv, ParkReason::HookBypassBlocked, &ctx());
        }
    }

    #[test]
    fn reading_core_hookspath_is_allowed_because_a_read_is_not_a_write() {
        for argv in [
            vec!["config", "--get", "core.hooksPath"],
            vec!["config", "--get-all", "core.hooksPath"],
            vec!["config", "get", "core.hooksPath"],
            vec!["config", "--list"],
            vec!["config", "--get", "user.email"],
            vec!["config", "user.email"],
        ] {
            assert_allowed(&argv, &ctx());
        }
    }

    #[test]
    fn the_command_line_config_form_that_outranks_the_envelope_is_refused() {
        for argv in [
            vec!["-c", "core.hooksPath=/tmp/x", "push"],
            vec!["-c", "core.hookspath=/tmp/x", "status"],
            vec!["-c", "core.hooksPath", "push"],
            vec!["--config-env=core.hooksPath=EVIL", "push"],
            vec!["--config-env", "core.hooksPath=EVIL", "push"],
            vec!["-C", "/tmp", "-c", "core.hooksPath=/tmp/x", "push"],
        ] {
            assert_refused(&argv, ParkReason::HookBypassBlocked, &ctx());
        }
    }

    #[test]
    fn a_leading_dash_c_that_sets_an_unrelated_key_does_not_refuse_by_itself() {
        assert_allowed(&["-c", "user.email=a@b.c", "status"], &ctx());
    }

    #[test]
    fn every_stash_form_is_refused_because_recovery_runs_through_git_fsck() {
        for argv in [
            vec!["stash"],
            vec!["stash", "push"],
            vec!["stash", "save"],
            vec!["stash", "-u"],
            vec!["stash", "list"],
            vec!["stash", "pop"],
        ] {
            assert_refused(&argv, ParkReason::ForcePushBlocked, &ctx());
        }
    }

    #[test]
    fn the_history_rewriting_family_is_refused() {
        for argv in [
            vec!["update-ref", "refs/heads/main", "0123abc"],
            vec!["update-ref", "-d", "refs/heads/main"],
            vec!["reflog", "delete", "HEAD@{0}"],
            vec!["reflog", "expire", "--expire=now", "--all"],
            vec!["filter-branch"],
            vec!["filter-repo", "--path", "x"],
            vec!["symbolic-ref", "HEAD", "refs/heads/x"],
        ] {
            assert_refused(&argv, ParkReason::ForcePushBlocked, &ctx());
        }
    }

    #[test]
    fn a_read_from_the_same_verb_family_is_allowed() {
        for argv in [
            vec!["symbolic-ref", "--short", "HEAD"],
            vec!["symbolic-ref", "--quiet", "--short", "HEAD"],
            vec!["reflog"],
            vec!["reflog", "show", "HEAD"],
        ] {
            assert_allowed(&argv, &ctx());
        }
    }

    #[test]
    fn an_unanticipated_plumbing_verb_is_allowed_because_this_is_a_denylist() {
        for argv in [
            vec!["status"],
            vec!["log"],
            vec!["rev-parse", "HEAD"],
            vec!["cat-file", "-p", "HEAD"],
            vec!["for-each-ref", "refs/heads"],
            vec!["merge-base", "a", "b"],
            vec!["commit", "-m", "a message"],
            vec![],
        ] {
            assert_allowed(&argv, &ctx());
        }
    }

    #[test]
    fn a_push_inside_the_namespace_is_allowed_and_one_outside_it_is_refused() {
        assert_allowed(
            &["push", "origin", "HEAD:refs/heads/gsd-auto/demo/x"],
            &ctx(),
        );
        assert_refused(
            &["push", "origin", "HEAD:refs/heads/main"],
            ParkReason::PushOutsideNamespace,
            &ctx(),
        );
        // An unqualified destination is qualified as a branch, which is what
        // git does; leaving it unqualified would make `git push origin main`
        // an unrecognised destination and therefore silently allowed.
        assert_refused(
            &["push", "origin", "main"],
            ParkReason::PushOutsideNamespace,
            &ctx(),
        );
    }

    #[test]
    fn a_push_with_no_refspec_is_judged_against_the_resolved_destination() {
        assert_allowed(&["push"], &ctx());
        assert_allowed(&["push", "origin"], &ctx());

        let outside = GitContext {
            namespace: default_namespace("demo"),
            resolved_push_dests: vec!["refs/heads/main".to_string()],
        };
        assert_refused(&["push"], ParkReason::PushOutsideNamespace, &outside);
    }

    #[test]
    fn a_push_whose_destination_cannot_be_resolved_is_refused_never_allowed() {
        assert_refused(
            &["push"],
            ParkReason::PushOutsideNamespace,
            &unresolved_ctx(),
        );
    }

    #[test]
    fn a_push_option_value_is_never_mistaken_for_a_refspec() {
        // `-o` and `--push-option` take a value; a classifier that read that
        // value as a refspec would refuse a legitimate push, and one that read
        // a refspec as a value would allow an illegitimate one.
        assert_allowed(&["push", "-o", "ci.skip", "origin"], &ctx());
        assert_refused(
            &["push", "--push-option", "ci.skip", "origin", "main"],
            ParkReason::PushOutsideNamespace,
            &ctx(),
        );
    }

    #[test]
    fn every_park_reason_has_a_distinct_stable_snake_case_identifier() {
        let all = [
            ParkReason::PushOutsideNamespace,
            ParkReason::ForcePushBlocked,
            ParkReason::HookBypassBlocked,
            ParkReason::SecretDetected,
            ParkReason::PrCapExceeded,
            ParkReason::CredentialUnavailable,
            ParkReason::EnvelopeAssertionFailed,
        ];
        let rendered: Vec<&str> = all.iter().map(|reason| reason.as_str()).collect();

        assert_eq!(
            rendered,
            vec![
                "push_outside_namespace",
                "force_push_blocked",
                "hook_bypass_blocked",
                "secret_detected",
                "pr_cap_exceeded",
                "credential_unavailable",
                "envelope_assertion_failed",
            ],
            "these seven strings are what Phase 20 reads out of a parked run; changing one \
             is a wire-format change, not a rename"
        );
    }

    #[test]
    fn the_disallowed_tool_patterns_cover_every_denied_verb_and_the_settings_directory() {
        let patterns = disallowed_tools();

        for verb in [
            "git push",
            "git stash",
            "git config",
            "git update-ref",
            "git reflog",
            "git filter-branch",
            "git filter-repo",
            "git symbolic-ref",
        ] {
            assert!(
                patterns.iter().any(|p| p.contains(verb)),
                "layer 1 must name `{verb}`; a verb the classifier refuses but the tool \
                 denylist omits is one that reaches the shell before anything looks at it"
            );
        }

        for tool in ["Write(.claude/", "Edit(.claude/"] {
            assert!(
                patterns.iter().any(|p| p.starts_with(tool)),
                "`{tool}…` must be denied: the driven repository's project-tier settings file \
                 IS loaded by the child and is agent-writable, so the envelope denies writing \
                 it rather than trusting it"
            );
        }
    }

    #[test]
    fn an_unresolvable_repository_yields_no_destination_rather_than_a_guess() {
        // The impure helper, against a directory that is not a repository at
        // all: the honest answer is "no destination", and `classify_git` turns
        // that into a refusal rather than into an allow.
        let root = tempfile::TempDir::new().expect("temp dir");
        let namespace = default_namespace("demo");
        let resolved = resolve_push_context(root.path(), &namespace);

        assert_eq!(resolved.namespace, namespace);
        assert!(
            resolved.resolved_push_dests.is_empty(),
            "a non-repository has no push destination; inventing one would mean judging a \
             push against a ref nobody configured"
        );
        assert_refused(&["push"], ParkReason::PushOutsideNamespace, &resolved);
    }

    #[test]
    fn the_default_namespace_of_any_plain_alias_satisfies_the_shape_rules() {
        for alias in ["demo", "gsd-meta-manager", "a.b_c-1"] {
            let namespace = default_namespace(alias);
            assert_eq!(
                validate_namespace(&namespace),
                Some(namespace.clone()),
                "the default namespace must pass the validator it ships with"
            );
        }
    }

    #[test]
    fn a_namespace_that_would_disable_the_control_is_refused() {
        for hostile in [
            "",
            "refs/heads/",
            "refs/heads",
            "gsd-auto/demo/",
            "refs/tags/gsd-auto/demo/",
            "refs/heads/demo/",
            "refs/heads/gsd-auto/demo",
            "refs/heads/main/demo/",
            "refs/heads/master/demo/",
            "refs/heads/HEAD/demo/",
            "refs/heads//demo/",
        ] {
            assert!(
                validate_namespace(hostile).is_none(),
                "{hostile:?} must not be accepted as a push namespace"
            );
        }
    }

    #[test]
    fn a_namespace_with_two_segments_below_refs_heads_is_accepted() {
        for ok in [
            "refs/heads/gsd-auto/demo/",
            "refs/heads/bots/nightly/",
            "refs/heads/bots/nightly/deep/",
        ] {
            assert_eq!(validate_namespace(ok), Some(ok.to_string()));
        }
    }

    #[test]
    fn only_refs_inside_the_namespace_are_allowed() {
        let namespace = default_namespace("demo");

        assert_eq!(
            classify_push_ref("refs/heads/gsd-auto/demo/tracer", &namespace),
            PushVerdict::Allow
        );

        for outside in [
            "refs/heads/main",
            "refs/heads/master",
            "refs/heads/gsd-auto/other/tracer",
            "refs/heads/gsd-auto-demo/tracer",
            "refs/tags/v1.0.0",
            "HEAD",
            "",
        ] {
            assert_eq!(
                classify_push_ref(outside, &namespace),
                PushVerdict::Refuse {
                    reason: REASON_PUSH_OUTSIDE_NAMESPACE,
                    ref_name: outside.to_string(),
                },
                "{outside:?} is outside {namespace} and must be refused"
            );
        }
    }

    #[test]
    fn a_one_character_difference_at_the_prefix_boundary_flips_the_verdict() {
        // The edge SAFE-01's flagged assumption names: a ref one character
        // outside the prefix is refused, a ref exactly at it is allowed.
        let namespace = default_namespace("demo");
        assert_eq!(
            classify_push_ref("refs/heads/gsd-auto/demo/x", &namespace),
            PushVerdict::Allow
        );
        assert!(matches!(
            classify_push_ref("refs/heads/gsd-auto/demox", &namespace),
            PushVerdict::Refuse { .. }
        ));
    }

    #[test]
    fn the_swept_worktree_and_runs_paths_are_refused() {
        for swept in [
            ".claude/worktrees",
            ".claude/worktrees/agent-abc/src/main.rs",
            ".planning/meta-manager/runs",
            ".planning/meta-manager/runs/20260818-x/journal.ndjson",
            "./.claude/worktrees/agent-abc/x",
        ] {
            assert_eq!(
                forbidden_repo_path(Path::new(swept), false),
                Some(ParkReason::ForcePushBlocked),
                "{swept:?} is exactly the sweep D-22 closes"
            );
        }
    }

    #[test]
    fn a_nested_repository_is_refused_on_the_callers_report_alone() {
        assert_eq!(
            forbidden_repo_path(Path::new("vendor/thing/src/lib.rs"), true),
            Some(ParkReason::ForcePushBlocked),
            "a whole second repository swept into this one's history"
        );
    }

    #[test]
    fn an_ordinary_path_and_a_near_miss_prefix_are_allowed() {
        // The paired allow, without which the predicate could refuse everything
        // and still pass every refusal row above.
        for ordinary in [
            "src/main.rs",
            ".planning/STATE.md",
            ".planning/meta-manager/inbox.ndjson",
            ".claude/settings.json",
            // One character past the boundary, the shape a prefix check gets
            // wrong when it forgets the separator.
            ".claude/worktreesX/file",
            ".planning/meta-manager/runsX/file",
        ] {
            assert_eq!(
                forbidden_repo_path(Path::new(ordinary), false),
                None,
                "{ordinary:?} must not be refused, or the hook is a wall"
            );
        }
    }

    #[test]
    fn the_runs_prefix_is_derived_from_the_journals_own_constant() {
        // The point of deriving rather than re-spelling: this assertion follows
        // RUNS_SUBDIR wherever it goes.
        assert!(
            forbidden_repo_prefixes()
                .iter()
                .any(|prefix| prefix.ends_with(crate::journal::RUNS_SUBDIR)),
            "{:?}",
            forbidden_repo_prefixes()
        );
    }

    // ---- the shell split (D-06 layer 2's weak joint) ----

    #[test]
    fn a_single_quoted_argument_is_recovered_whole() {
        assert_eq!(
            split_command("git commit -m 'one two three'").unwrap(),
            vec!["git", "commit", "-m", "one two three"]
        );
    }

    #[test]
    fn a_double_quoted_argument_containing_spaces_is_one_word() {
        assert_eq!(
            split_command(r#"git commit -m "one two three""#).unwrap(),
            vec!["git", "commit", "-m", "one two three"]
        );
    }

    #[test]
    fn a_backslash_escaped_space_does_not_split_a_word() {
        assert_eq!(
            split_command(r"git add my\ file.txt").unwrap(),
            vec!["git", "add", "my file.txt"]
        );
    }

    #[test]
    fn a_run_of_whitespace_is_one_boundary_not_several_empty_words() {
        assert_eq!(
            split_command("git  push   --force  origin\tmain").unwrap(),
            vec!["git", "push", "--force", "origin", "main"]
        );
    }

    #[test]
    fn an_escape_inside_double_quotes_keeps_only_the_four_that_are_escapes() {
        assert_eq!(
            split_command(r#"echo "a\"b\\c\nd""#).unwrap(),
            vec!["echo", r#"a"b\c\nd"#],
            "only \\\" \\\\ \\$ and \\` are escapes inside double quotes; every other \
             backslash is literal, and dropping it would change the word"
        );
    }

    #[test]
    fn an_unterminated_quote_is_unrecoverable_and_yields_none() {
        assert_eq!(split_command("git commit -m 'unterminated"), None);
        assert_eq!(split_command(r#"git commit -m "unterminated"#), None);
        assert_eq!(
            split_command(r"git push --force \"),
            None,
            "a trailing backslash is a line continuation whose second half we were \
             never given"
        );
    }

    #[test]
    fn an_empty_quoted_word_is_a_word_and_an_empty_line_is_no_words() {
        assert_eq!(split_command(r#"echo "" x"#).unwrap(), vec!["echo", "", "x"]);
        assert!(split_command("   ").unwrap().is_empty());
    }

    #[test]
    fn every_command_behind_a_separator_is_its_own_segment() {
        let segments = split_segments("echo hi && git push --force origin main").unwrap();
        let words: Vec<Vec<String>> = segments
            .iter()
            .map(|segment| segment.iter().map(|t| t.text.clone()).collect())
            .collect();
        assert_eq!(
            words,
            vec![
                vec!["echo".to_string(), "hi".to_string()],
                vec![
                    "git".to_string(),
                    "push".to_string(),
                    "--force".to_string(),
                    "origin".to_string(),
                    "main".to_string()
                ]
            ],
            "a guard that classified only the first command would look at `echo` and \
             allow the force push sitting behind the `&&`"
        );
    }

    #[test]
    fn a_separator_inside_quotes_is_a_character_not_a_separator() {
        let segments = split_segments("git commit -m 'fix; and push'").unwrap();
        assert_eq!(segments.len(), 1, "{segments:?}");
        assert_eq!(segments[0].last().unwrap().text, "fix; and push");
    }

    #[test]
    fn a_word_subject_to_expansion_is_flagged_because_its_value_is_unknowable() {
        let segments = split_segments("git $verb --force").unwrap();
        assert!(
            segments[0][1].expansion,
            "an unquoted `$` is expansion: {:?}",
            segments[0]
        );
        let quoted = split_segments("git 'literal$verb'").unwrap();
        assert!(
            !quoted[0][1].expansion,
            "single quotes suppress expansion, so flagging it would false-refuse"
        );
        let double = split_segments(r#"git "$verb""#).unwrap();
        assert!(
            double[0][1].expansion,
            "double quotes do NOT suppress expansion"
        );
    }

    #[test]
    fn separators_are_named_in_one_list_that_the_predicate_reads() {
        for op in [";", "&&", "||", "|", "&"] {
            assert!(is_separator(op), "{op}");
        }
        assert!(!is_separator(">"), "a redirection does not start a command");
    }

    // ---- pull-request creation (D-19's three shapes) ----

    #[test]
    fn the_github_pull_request_creation_form_is_recognised() {
        assert!(classify_pr_command(&["gh", "pr", "create", "--title", "x"]));
        assert!(
            classify_pr_command(&["/usr/bin/gh", "pr", "create"]),
            "an absolute path is not an evasion, it is what `command -v` prints"
        );
        assert!(
            classify_pr_command(&["gh", "--repo", "o/r", "pr", "create"]),
            "a global flag before the subcommand is a legal invocation"
        );
        assert_eq!(
            pr_command_label(&["gh", "pr", "create"]),
            Some(("github", "gh pr create".to_string()))
        );
    }

    #[test]
    fn a_raw_api_post_to_a_pulls_path_is_recognised_in_both_its_spellings() {
        assert!(classify_pr_command(&[
            "gh", "api", "-X", "POST", "repos/o/r/pulls"
        ]));
        assert!(classify_pr_command(&[
            "gh",
            "api",
            "--method=POST",
            "https://api.github.com/repos/o/r/pulls"
        ]));
        assert!(
            classify_pr_command(&["gh", "api", "repos/o/r/pulls", "-f", "title=x"]),
            "`gh api` defaults to POST once a field is supplied, so a method check \
             alone would be a hole this form walks through"
        );
    }

    #[test]
    fn the_gitlab_merge_request_creation_form_is_recognised() {
        assert!(classify_pr_command(&["glab", "mr", "create"]));
        assert_eq!(
            pr_command_label(&["glab", "mr", "create", "--fill"]),
            Some(("gitlab", "glab mr create".to_string()))
        );
    }

    #[test]
    fn a_read_only_listing_form_is_not_a_creation() {
        // The paired allow test. A classifier that returned true for everything
        // would pass every assertion above and cap a run out of existence.
        assert!(!classify_pr_command(&["gh", "pr", "list"]));
        assert!(!classify_pr_command(&["gh", "pr", "view", "7"]));
        assert!(!classify_pr_command(&["glab", "mr", "list"]));
        assert!(
            !classify_pr_command(&["gh", "api", "repos/o/r/pulls"]),
            "a bare `gh api` is a GET"
        );
        assert!(
            !classify_pr_command(&["gh", "api", "-X", "GET", "repos/o/r/pulls"]),
            "an explicit GET is a GET even with the right path"
        );
        assert!(
            !classify_pr_command(&["gh", "api", "-X", "POST", "repos/o/r/issues"]),
            "a POST to another collection is not a pull request"
        );
        assert!(
            !classify_pr_command(&["gh", "api", "-X", "POST", "repos/o/r/pulls/7/reviews"]),
            "a POST to one pull request is a review, not a creation"
        );
        assert!(!classify_pr_command(&["git", "push"]));
        assert!(!classify_pr_command(&[]));
    }

    // ---- the one command shape that has to read the repository ----

    #[test]
    fn only_a_push_with_no_refspec_needs_the_repository_consulted() {
        assert!(push_needs_resolved_dests(&["push"]));
        assert!(push_needs_resolved_dests(&["push", "origin"]));
        assert!(
            push_needs_resolved_dests(&["-C", "/tmp/x", "push", "origin"]),
            "the leading-option scan is shared with the classifier, so both agree \
             about where the verb starts"
        );
        assert!(
            !push_needs_resolved_dests(&["push", "origin", "refs/heads/gsd-auto/a/b"]),
            "a push that names its refspec is judged from argv alone"
        );
        assert!(!push_needs_resolved_dests(&["status"]));
        assert!(
            !push_needs_resolved_dests(&["push", "--force"]),
            "a command that is refused anyway needs no destination resolved — reading \
             a repository to reach a verdict already reached is latency spent for \
             nothing on the agent's critical path"
        );
        assert!(
            !push_needs_resolved_dests(&["push", "-o", "ci.skip", "origin", "refs/heads/x"]),
            "an option value is not an operand"
        );
    }
}
