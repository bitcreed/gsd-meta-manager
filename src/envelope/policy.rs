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
    let _ = (argv, ctx);
    GitVerdict::Allow
}

/// Resolve the implicit push destinations for a repository — the **one** impure
/// function in this module.
///
/// It calls [`crate::state_reader::git_ops::push_refspecs`], which already
/// resolves `branch.<b>.remote`, `remote.pushDefault`, `remote.<r>.push` and
/// `push.default` with no network and no credential. **Do not re-derive any of
/// those here.** A second implementation of that resolution is a second thing
/// that can drift from the preview the user was shown, and the whole value of
/// the dry-run preview is that it describes the push the envelope will judge.
///
/// An unresolvable destination yields an empty `resolved_push_dests`, which
/// [`classify_git`] treats as a refusal. Unknown is never allowed.
pub fn resolve_push_context(project_root: &Path, namespace: &str) -> GitContext {
    let _ = (project_root, namespace);
    GitContext::default()
}

/// The `--disallowedTools` patterns for the driven child (D-06 layer 1).
///
/// Layer 1 of the enforcement stack: cheapest, fires before the tool runs, and
/// cannot be silently dropped because it is argv rather than a file that could
/// fail validation. It is **not** a substitute for [`classify_git`] — this list
/// is matched by the agent runtime against a command line, while the classifier
/// is applied to parsed argv by the `PreToolUse` guard.
pub fn disallowed_tools() -> Vec<String> {
    Vec::new()
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
