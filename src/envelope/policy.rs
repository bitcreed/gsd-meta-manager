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
    let mut index = 0;

    // Rule 4: the leading options apply to whatever verb follows, so they are
    // judged before the verb is even known.
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
                return refuse(
                    ParkReason::HookBypassBlocked,
                    format!(
                        "`git -c {assignment}` sets core.hooksPath at command-line precedence, \
                         which is the one form that outranks the envelope's own env-injected \
                         setting (D-09)"
                    ),
                );
            }
        }
        index += consumed;
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
                return refuse(reason, denied_push_detail(reason, spelling));
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
                    return refuse(
                        ParkReason::ForcePushBlocked,
                        denied_push_detail(ParkReason::ForcePushBlocked, "-f"),
                    )
                }
                'd' => {
                    return refuse(
                        ParkReason::ForcePushBlocked,
                        denied_push_detail(ParkReason::ForcePushBlocked, "-d"),
                    )
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
