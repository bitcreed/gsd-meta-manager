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

    // **The SECOND layer, and it is a second layer rather than the fix.**
    //
    // No valid invocation puts a program the envelope governs in its own VERB
    // slot: `git git push --force` is not a command. An argv shaped this way is
    // one [`resolve_program`] mis-indexed, and answering `Allow` from the
    // denylist's default arm below is exactly how a mis-index became a permit
    // (`T-19-60`, audit 2). After the command-position rule this argv no longer
    // reaches here through `super::hooks::guard_in`, so this arm is pinned by a
    // UNIT test on this function rather than through the guard.
    //
    // **It is not dead code and it must not be deleted as unreachable.** It is
    // the layer that holds if resolution ever mis-indexes again, which it has
    // now done twice. It is also, on its own, insufficient: the verb it would
    // see for `env -u git timeout 5 git push --force origin main` is `timeout`,
    // and the forge path never enters this function at all — see
    // [`forge_subcommand_names_a_governed_program`].
    if GOVERNED_PROGRAMS.contains(&program_name(verb)) {
        return refuse(
            ParkReason::EnvelopeAssertionFailed,
            format!(
                "this argv puts `{}` — a program the envelope governs — in its own verb \
                 slot, which no valid invocation does; the command that reaches `execve` \
                 is therefore not the one being classified",
                program_name(verb)
            ),
        );
    }

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
///
/// # An UNESTABLISHED verb slot is a refusal, not the next non-`-` word
///
/// **This scan used to guess.** [`leading_git_option`] answered a word count, and
/// for a spelling it did not recognise the answer was one — so the loop advanced a
/// single word, landed on the option's VALUE, saw it did not begin with `-`, and
/// broke with that value as the verb. [`classify_git`] then found that value in no
/// denylist arm and answered `Allow`. `git --attr-source HEAD push --force origin
/// main` exited 0 that way, and against a bare upstream it rewrote `main`.
///
/// **The fix inverts the failure direction rather than lengthening the list**, and
/// it is the same fail-closed treatment [`resolve_program`]'s wrapper axis already
/// has: a governed candidate whose command position cannot be established
/// STRUCTURALLY is a refusal there, not a mis-index (`T-19-60`, audit 3). This
/// scan was the one arm of the same question that was never given that treatment.
/// A verb the guard cannot establish is now
/// [`ParkReason::EnvelopeAssertionFailed`] — the identifier this module already
/// uses for *the argv that runs is not the argv the classifier reads*.
///
/// **It is produced HERE, inside the one scan, and returned through the
/// `(usize, Option<GitVerdict>)` channel that already carries the unreadable-key
/// refusal below.** That is the one-scan-two-callers property being RELIED ON
/// rather than worked around: [`push_needs_resolved_dests`] inherits the new
/// refusal exactly as it inherits the existing one, because a command that will be
/// refused needs no destination resolved. No new park reason, no second reading
/// site, no change to [`classify_git`], and no arm added to
/// [`resolve_program_with_head`].
///
/// # An UNBOUNDED config assignment is a refusal too, and it is a CHANGE OF QUESTION
///
/// **This scan used to ask *does this assignment spell `core.hooksPath`*. It now
/// asks *can I establish what `core.hooksPath` will be*.** The old question is a
/// string comparison against one key; the new one is a property of the key, and
/// the difference is the whole of `T-19-103`. Git's config graph is spliced in
/// from elsewhere by exactly one mechanism, identified by the SECTION half of
/// the key, so `-c include.path=<file>` sets `core.hooksPath` at command-line
/// precedence **without the string `core.hooksPath` appearing anywhere on the
/// command line** — measured against `git version 2.43.0` with the envelope's
/// own injection as the control.
///
/// An assignment this scan can BOUND to the key it names leaves the injection
/// alone; one that adds an indirection leaves it unknowable, and an unresolvable
/// command is refused rather than guessed at. See
/// [`config_key_names_an_indirection_section`] for the rule, the three rejected
/// options and **the residue it fails OPEN on, which no control covers**.
///
/// **The three clauses of the assignment block are ordered, and the order is
/// what a reader must not have to infer**: (1) the key half carries a character
/// the shell may rewrite → unreadable; (2) the key's section names an
/// indirection → unbounded; (3) the key IS `core.hooksPath` →
/// [`ParkReason::HookBypassBlocked`]. The refusal is raised at the FIRST
/// assignment the loop cannot bound, so a line carrying both an indirection and
/// a hooks-path key earns whichever the scan reaches first — the two spellings
/// are pinned at deliberately different identifiers, and a clause raised in a
/// second pass over the leading tokens turns one of them red.
///
/// **What it deliberately does NOT do.** It does not widen into tokens this scan
/// does not treat as options: the break on a non-`-` token, the break on a bare
/// `-` and the `--` end-of-options marker are the scan's termination conditions
/// and they run BEFORE any option check, so `git - push --force origin main` stays
/// permitted — real git answers `unknown option: -` — and `git -- push --force
/// origin main` stays at `force_push_blocked`. And it is not a blanket refusal of
/// anything beginning with `-`: `git --no-pager status`, `git -c user.name="$NAME"
/// commit`, `git --git-dir=/tmp/g status` and `git -C /tmp status` all still run,
/// which is the half that keeps this a grammar MODEL rather than a control that
/// fails into unusability (AR-19-11).
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
        let (assignment, consumed) = match leading_git_option(argv, index) {
            LeadingOptionGrammar::SelfContained { assignment } => (assignment, 1),
            LeadingOptionGrammar::ConsumesASeparateWord { assignment } => (assignment, 2),
            // **The verb slot is unestablished, so there is no verb to classify.**
            // Returning the current index alongside the refusal keeps this
            // function's contract intact for the caller that ignores the refusal
            // in favour of the index; the refusal is what both callers act on.
            LeadingOptionGrammar::NotEstablished => {
                return (index, Some(unestablished_verb_refusal(token)));
            }
        };
        if let Some(assignment) = assignment {
            // **A key this scan cannot read is a decision it cannot make.**
            // This function's ONE job is to decide whether `core.hooksPath` is
            // being set at command-line precedence — the single form that
            // outranks the envelope's own env-injected setting (D-09) — and it
            // decides it by comparing the KEY half. `git -c $K commit -m x`
            // hands it a key whose value is bound after the guard has answered,
            // so the comparison is a guess.
            //
            // **The check is on the key half only, and that boundary is the
            // whole of its cost containment.** A VALUE carrying an expansion —
            // `git -c user.name="$NAME" commit -m x` — changes what the setting
            // IS, not WHICH setting it is, and is pinned PERMITTED. A rule that
            // refused the value half would refuse every `git -c` that
            // interpolates anything, which is `T-19-75` widened from `rg` to
            // ordinary configuration (AR-19-11).
            //
            // **Textual rather than bit-flagged, and this is the ONE exception
            // to that rule in the whole module**, because this function is a
            // pure argv function and must stay one: it is called by
            // [`push_needs_resolved_dests`] before classification and by
            // [`classify_git`] during it, from `&[&str]` in both cases. The
            // `Token.literal` bit is consulted once, at the decision boundary,
            // by [`first_unreadable_decision_word`].
            //
            // **The class is the same class the bit covers** — `$` and a
            // backtick, plus `*`, `?`, `[`, `~` and `{`, because a key half the
            // shell rewrites is a key half this scan cannot compare. **Its cost
            // is disclosed and it is a false POSITIVE**: a pure argv function
            // sees the word after the tokenizer removed its quoting and cannot
            // tell that the quoting made the character literal, so
            // `git -c 'user.na*e=x' commit -m y` is refused too. That cost is
            // essentially zero and the reason is stated rather than assumed — a
            // git config key is `section.key` over alphanumerics, `.`, `-` and
            // `_`, so no LEGAL key can carry one of these characters. It is
            // pinned in `tests/envelope_literal_decision.rs` beside its permitted
            // twin rather than argued.
            //
            // Both callers inherit the refusal, which is the point of there
            // being one scan.
            let key = config_key_of(assignment);
            if key.chars().any(|ch| REWRITING_CHARACTERS.contains(&ch) || ch == '{') {
                return (
                    index,
                    Some(refuse(
                        ParkReason::EnvelopeAssertionFailed,
                        format!(
                            "the key half of `git -c {assignment}` carries a character the \
                             shell may rewrite, so whether this command sets core.hooksPath \
                             at command-line precedence — the one form that outranks the \
                             envelope's own env-injected setting (D-09) — cannot be \
                             established before it runs; refused rather than guessed at"
                        ),
                    )),
                );
            }
            // **The CONFINEMENT clause, and it is a change of question rather
            // than a longer list.** The clause above asks whether the key can
            // be READ; this one asks whether its effect can be BOUNDED to the
            // key it names. A key whose SECTION names an indirection splices in
            // a file at the precedence of the directive that named it, so
            // `core.hooksPath` — the setting the envelope's whole hook layer IS
            // — becomes unknowable from argv without the string
            // `core.hooksPath` appearing anywhere on the line. Measured:
            // `-c include.path=<f>` makes `git config --get core.hooksPath`
            // print `/INCLUDE_WINS` where the envelope's own injection alone
            // prints `/ENV_WINS`.
            //
            // **It is ordered HERE, after the unreadable-key refusal and BEFORE
            // `is_hooks_path_key`, and the ordering is load-bearing rather than
            // stylistic.** The refusal is raised at the FIRST assignment this
            // loop cannot bound and no assignment after it is read, so
            // `git -c include.path=<f> -c core.hooksPath=/dev/null push --force
            // origin main` earns the unresolvable identifier while the reverse
            // spelling earns `HookBypassBlocked` — the two are pinned at
            // deliberately different identifiers. **A clause raised in a SECOND
            // PASS over the leading tokens turns one of them red**, and naming
            // `HookBypassBlocked` for a line whose hooks-path write the guard
            // never established would attribute the refusal to a mechanism that
            // did not produce it (D-24).
            //
            // No new park reason, no second reading site, and both callers
            // inherit it through the channel that already carries two refusals.
            if config_key_names_an_indirection_section(key) {
                let section = config_key_section(key).unwrap_or(key);
                return (
                    index,
                    Some(unbounded_config_assignment_refusal(key, section)),
                );
            }
            if is_hooks_path_key(key) {
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
/// **Membership here is a MEASUREMENT of the installed `git`, not a reading of
/// its documentation.** Every entry was classified by the two-sided probe
/// `git <opt> version` versus `git <opt> XVALUE version`, and
/// [`every_leading_git_option_the_guard_calls_value_taking_really_consumes_the_next_word`]
/// re-runs that probe over this constant on every test run. It exists because
/// this list had **no pin and no test reference of any kind** — audit 6 measured
/// exactly two mentions of it in the whole repository, its definition and its one
/// use — and drifted in both directions while nothing went red.
///
/// **`--super-prefix` was REMOVED here and removing it is part of the fix rather
/// than tidying.** The installed git answers `unknown option: --super-prefix` in
/// both probe forms, and a stale entry is fail-open in the OVER-consuming
/// direction, which is a **bypass** and not an over-refusal: measured against the
/// built binary, `git --super-prefix push --force origin main` exited **0**,
/// because the scan swallowed the real verb `push` as the option's value and read
/// `origin` as the verb instead. That row is inert only because git itself rejects
/// the option — **a stale entry for an option git ACCEPTS would be a live
/// bypass**, and that is the direction the drift pin exists for.
///
/// `--attr-source` and `--shallow-file` were ADDED: the probe reaches the verb
/// through both, and their absence was `T-19-100` — `git --attr-source HEAD push
/// --force origin main` exited 0 while its attached twin
/// `--attr-source=HEAD` was correctly refused.
///
/// `--exec-path` is deliberately absent and belongs to
/// [`GIT_GLOBAL_SELF_CONTAINED_OPTS`]: without `=` it prints a path and runs
/// nothing, so treating the next token as its value would swallow the verb and
/// hand the classifier an argv with no command in it.
const GIT_GLOBAL_VALUE_OPTS: &[&str] = &[
    "-c",
    "-C",
    "--config-env",
    "--git-dir",
    "--work-tree",
    "--namespace",
    "--attr-source",
    "--shallow-file",
];

/// Leading `git` options that occupy **exactly one** word: the token after them
/// is the verb.
///
/// **This constant is knowledge the guard never had.** Before the inversion,
/// silence meant "one word", so there was nothing to enumerate — and that silence
/// is exactly what made an option the guard had never heard of swallow the verb.
/// Now silence means *grammar not established*, so the self-contained spellings
/// have to be stated, and every one of them was classified by the same two-sided
/// probe: `git <opt> version` prints `git version …` while
/// `git <opt> XVALUE version` answers `'XVALUE' is not a git command`.
///
/// **The TERMINATING family is folded in here deliberately** rather than given a
/// third category. `--exec-path`, `--html-path`, `--man-path`, `--info-path` and
/// `--version` print a path or a version and run no verb at all; for them the
/// probe is that the one-word and two-word forms produce IDENTICAL first lines,
/// which is the proof that no following word can reach a verb slot. They differ
/// from the booleans in what git does NEXT, not in the one bit this scan asks
/// about.
///
/// **The rejected third category, recorded rather than silently not taken.** A
/// `Terminates` answer would let `scan_leading` report "this command runs no verb
/// at all", and `git --exec-path push --force origin main` would become a permit,
/// because real git prints `/usr/lib/git-core` and does not push. It is rejected
/// because a control that ADDS a permit needs stronger evidence than one that
/// preserves a refusal, and the refusals it would remove are refusals of commands
/// that do nothing. That row stays at `force_push_blocked`.
///
/// **The `--no-*` convention was considered and rejected too.** Git's `--no-X`
/// global options are all boolean, so a rule "a leading `--no-` option is
/// self-contained" would remove the future-git cost for that whole family for
/// free. It is not taken: it is precisely the shape of assumption about a callee's
/// grammar that this phase has now been punished for six times, and the two
/// measured stand-ins for a future git — `--no-advice` and `--no-lazy-fetch` —
/// would be silently absorbed by it rather than surfacing as the one refusal each
/// that tells a maintainer the constant needs a row. Enumerate instead.
const GIT_GLOBAL_SELF_CONTAINED_OPTS: &[&str] = &[
    "--no-pager",
    "-p",
    "--paginate",
    "-P",
    "--bare",
    "--no-replace-objects",
    "--literal-pathspecs",
    "--glob-pathspecs",
    "--noglob-pathspecs",
    "--icase-pathspecs",
    "--no-optional-locks",
    "--exec-path",
    "--html-path",
    "--man-path",
    "--info-path",
    "--version",
];

/// The three answers to the ONE question [`scan_leading`] asks about a leading
/// `git` option: **does it consume the next word.**
///
/// The third variant is the whole of `T-19-100`. There used to be two answers
/// spelled as a word count, and an option the guard did not recognise was
/// ASSUMED to occupy one word — so the scan advanced onto the option's VALUE, saw
/// it did not start with `-`, and broke with that value as the verb. Making the
/// ABSENCE of the bit an answer in its own right is what lets the scan refuse
/// instead of guess.
enum LeadingOptionGrammar<'a> {
    /// The option occupies exactly one word; the next token is the verb.
    SelfContained { assignment: Option<&'a str> },
    /// The option occupies two words; the next token is its VALUE.
    ConsumesASeparateWord { assignment: Option<&'a str> },
    /// **The guard has no bit for this spelling, so the verb slot is
    /// unestablished.** [`scan_leading`] refuses on this.
    NotEstablished,
}

/// One leading option, answered as a three-valued grammar question.
///
/// **The arms are ordered so the rules that need NO KNOWLEDGE OF GIT run first,
/// and that ordering is the reduction that makes the constants smaller.** The
/// scan needs one bit per option; three structural facts supply it for free, and
/// only the spellings those facts do not cover need a constant at all.
///
/// 1. `-c` / `--config-env` bare — consumes a separate word, carrying the next
///    token as the config assignment. Unchanged.
/// 2. `--config-env=…` and `-c<non-empty>` — self-contained with the attached
///    assignment. Unchanged. **Ordered before rule 3** so the assignment is not
///    lost to the structural rule that would otherwise also match.
/// 3. **Any `--`-prefixed token containing `=` — SELF-CONTAINED, whatever the
///    option is.** This needs no knowledge of git: git's own grammar attaches the
///    value, so no following word can ever be consumed. It is why
///    `git --attr-source=HEAD push --force origin main` was ALREADY refused
///    correctly while its separate-value twin exited 0, and it is what keeps every
///    attached spelling of an option the constants have never heard of working —
///    `git --git-dir=/tmp/g status`, `git --namespace=n log`.
/// 4. **The SHORT half of the same structural rule, and it is a DEVIATION this
///    executor added because the plan's arm list regressed a row `19-20` had
///    pinned green.** A single-dash token longer than two characters whose first
///    two characters name a member of [`GIT_GLOBAL_VALUE_OPTS`] carries that
///    option's value ATTACHED, so it too is self-contained: `-C/tmp` is `-C` with
///    `/tmp` attached. Without this arm `git -C/tmp push --force origin main`
///    moved from `force_push_blocked` to `envelope_assertion_failed`, breaking
///    `the_already_correct_planning_cells_keep_their_verdicts_as_controls` in a
///    file this round may not edit.
///    **The restriction to VALUE-TAKING heads is what makes it a grammar claim
///    rather than a convenience.** Only an option that takes a value can have one
///    attached; a self-contained option followed by more characters is a BUNDLE,
///    and git 2.43.0 accepts no short-option bundling at all (`-pc`, `-pP` and a
///    bare `-` each answer `unknown option:`). So `-pc user.name=x` — whose head
///    `-p` is self-contained — does NOT match here and stays unestablished, which
///    is the pinned verdict for it. This mirrors the existing `-c<key>=<value>`
///    arm above rather than inventing a second convention.
/// 5. Membership in [`GIT_GLOBAL_VALUE_OPTS`] — consumes a separate word.
/// 6. Membership in [`GIT_GLOBAL_SELF_CONTAINED_OPTS`] — self-contained.
/// 7. **Otherwise the grammar is not established.**
///
/// The two termination rules [`scan_leading`] applies before calling this — a
/// token that does not begin with `-` or is exactly `-`, and the `--`
/// end-of-options marker — are the other two knowledge-free facts, and they are
/// deliberately left where they are rather than folded in here.
fn leading_git_option<'a>(argv: &[&'a str], index: usize) -> LeadingOptionGrammar<'a> {
    let token = argv[index];

    if token == "-c" || token == "--config-env" {
        return LeadingOptionGrammar::ConsumesASeparateWord {
            assignment: argv.get(index + 1).copied(),
        };
    }
    if let Some(rest) = token.strip_prefix("--config-env=") {
        return LeadingOptionGrammar::SelfContained {
            assignment: Some(rest),
        };
    }
    // git's short-option parser accepts `-ckey=value` with no space.
    if let Some(rest) = token.strip_prefix("-c") {
        if !rest.is_empty() && !token.starts_with("--") {
            return LeadingOptionGrammar::SelfContained {
                assignment: Some(rest),
            };
        }
    }
    // The structural rule: an attached value cannot consume a following word.
    if token.starts_with("--") && token.contains('=') {
        return LeadingOptionGrammar::SelfContained { assignment: None };
    }
    // The SHORT half of the same structural rule. See this function's doc, rule
    // 4: only an option that TAKES a value can carry one attached, so the
    // remainder of `-C/tmp` is `-C`'s value while the remainder of `-pc` is a
    // BUNDLE and stays unestablished.
    if !token.starts_with("--") && token.len() > 2 {
        let (head, rest) = token.split_at(2);
        if !rest.is_empty() && GIT_GLOBAL_VALUE_OPTS.contains(&head) {
            return LeadingOptionGrammar::SelfContained { assignment: None };
        }
    }
    if GIT_GLOBAL_VALUE_OPTS.contains(&token) {
        return LeadingOptionGrammar::ConsumesASeparateWord { assignment: None };
    }
    if GIT_GLOBAL_SELF_CONTAINED_OPTS.contains(&token) {
        return LeadingOptionGrammar::SelfContained { assignment: None };
    }
    LeadingOptionGrammar::NotEstablished
}

/// The refusal an unestablished verb slot earns, naming the OPTION TOKEN and the
/// recovery path.
///
/// **The option token is named and the command is never quoted back** (SAFE-04),
/// on exactly the same footing as [`scan_leading`]'s existing
/// `git -c {assignment}` refusals: a refusal a user cannot act on is a control
/// that gets switched off (AR-19-11).
///
/// **The attached spelling is offered FIRST and never ALONE**, and that is a
/// measured constraint rather than a stylistic one. An attached value is always
/// *self-contained* by git's grammar, but it is not always *accepted*:
/// `git --shallow-file=/tmp/s version` answers `unknown option:
/// --shallow-file=/tmp/s` on git 2.43.0, while `--attr-source=`, `--git-dir=`,
/// `--namespace=` and `--work-tree=` all reach the verb. A message that promised
/// the attached form as universally available would send a user in a circle.
fn unestablished_verb_refusal(token: &str) -> GitVerdict {
    refuse(
        ParkReason::EnvelopeAssertionFailed,
        format!(
            "this command carries the leading git option `{token}`, whose grammar the \
             guard cannot establish: whether the word after it is that option's value or \
             git's own subcommand is not knowable before the command runs, so the verb \
             this command would actually run is unestablished and the command is refused \
             rather than guessed at. To proceed: spell the option with its value attached \
             (`--option=value`) where git accepts that form, since an attached value can \
             never consume a following word — git does not accept the attached spelling \
             for every option, so this step is the first to try and not the only one; or \
             drop the option; or add the spelling to the guard's leading-option grammar, \
             which the drift pin over those constants will name"
        ),
    )
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

/// The two config SECTIONS through which git splices configuration in from a
/// file named by the directive itself.
///
/// **This is not a list of dangerous keys and it must not be allowed to become
/// one.** It names the two sections of git's own configuration grammar that
/// make a config key an INDIRECTION — a directive whose effect is the contents
/// of some other file, read at the precedence of the directive that named it.
/// Everything else about the key is deliberately unread: see
/// [`config_key_names_an_indirection_section`].
///
/// Measured against `git version 2.43.0`, with the exact
/// `GIT_CONFIG_COUNT`/`KEY_0`/`VALUE_0` triplet [`super::cred::hooks_path_env`]
/// emits as the control (it alone resolves `core.hooksPath` to `/ENV_WINS`):
///
/// ```text
/// -c include.path=<file>                 -> /INCLUDE_WINS
/// -c INCLUDE.PATH=<file>                 -> /INCLUDE_WINS
/// -c includeIf.gitdir:<p>.path=<file>    -> /INCLUDE_WINS
/// -c INCLUDEIF.gitdir:<p>.PATH=<file>    -> /INCLUDE_WINS
/// --config-env=include.path=<VAR>        -> /INCLUDE_WINS
/// -c include.pathx=<file>                -> /ENV_WINS   (git IGNORES it)
/// -c notinclude.path=<file>              -> /ENV_WINS
/// ```
///
/// # THE RESIDUE, STATED PLAINLY AND NOT HANDED TO ANY CONTROL
///
/// **This constant is a recognition of a closed grammatical fact, NOT a
/// fail-closed default. Its SILENCE IS A PERMIT.** A future git that adds a
/// **THIRD** indirection section is **not covered, and this rule fails OPEN on
/// it**, and **there is NO automated control over that direction.**
///
/// The drift pin below holds the **REVERSE** direction — it turns red if the
/// installed git stops honouring a section this constant already names — and
/// **it cannot observe a section it does not name, because it iterates these
/// entries and an entry that does not exist is never probed.** A third
/// indirection section therefore reaches `core.hooksPath` **silently** until a
/// human reading a future git's release notes adds it here. Writing that the
/// pin covers this direction would be honest about the residual and then hand
/// it to a control that cannot cover it, which is `T-19-107`'s own failure mode
/// — false reassurance in a control's own doc.
const INDIRECTION_SECTIONS: &[&str] = &["include", "includeIf"];

/// The SECTION half of a git config key: the text before the **first** `.`.
///
/// That is git's own rule, and a key with no `.` names no section at all —
/// `git -c a=b config --get a` answers `error: key does not contain a section:
/// a`. `None` is therefore a positive fact about the key rather than a parse
/// failure, and [`config_key_names_an_indirection_section`] treats it as one.
fn config_key_section(key: &str) -> Option<&str> {
    key.split_once('.').map(|(section, _)| section)
}

/// Whether a config key's SECTION names an indirection — the one question
/// [`scan_leading`] can answer about an assignment it cannot otherwise bound.
///
/// # THE CHANGE OF QUESTION
///
/// The guard used to ask *does this assignment spell `core.hooksPath`*. It now
/// asks *can I establish what `core.hooksPath` will be*. An assignment whose
/// key names an ordinary section leaves the envelope's injected `core.hooksPath`
/// exactly as injected — measured, `-c user.name=x` and `-c core.pager=cat`
/// both leave the control printing `/ENV_WINS`. An assignment in an
/// **indirection** section replaces it with the contents of a file this
/// function has not read, so the setting the envelope's whole hook layer IS
/// becomes unknowable from argv. The second is **unresolvable**, and an
/// unresolvable command is refused rather than guessed at.
///
/// # THE SUBSECTION AND THE VARIABLE ARE NEVER READ, AND THAT IS THE RULE
///
/// Only [`config_key_section`]'s answer is consulted, compared with
/// `eq_ignore_ascii_case` because git folds section names (measured:
/// `-c INCLUDE.PATH=<file>` resolves the include exactly as
/// `-c include.path=<file>` does).
///
/// * **The SUBSECTION is not read** because it carries `includeIf`'s CONDITION
///   — `gitdir:`, `gitdir/i:`, `onbranch:`, `hasconfig:remote.*.url:` — which
///   is an OPEN family a future git extends. Not reading it covers the whole
///   family by construction. It is also the one half of a git config key that
///   git does NOT fold, so reading it would import a case rule as well.
/// * **The VARIABLE is not read** because it is an open family too: `path` is
///   the only spelling today, and a future variable in either section would
///   defeat any enumeration of it.
///
/// **The measured cost of not reading the variable is one row, disclosed rather
/// than discovered**: `git -c include.pathx=/tmp/evil.cfg status` is a key real
/// git IGNORES (the control still prints `/ENV_WINS`) and this rule refuses it.
/// That is over-refusal in the safe direction. The correct response to it is
/// NOT to start reading the variable — that re-opens both families.
///
/// # A KEY WITH NO SECTION IS CONFINED, AND THAT IS LOAD-BEARING
///
/// `None` answers `false`. A key with no `.` names no section, so it provably
/// is not an include directive; real git RUNS `git -c a=b version` at rc 0 and
/// errors only when something READS the key. It therefore falls through to
/// [`is_hooks_path_key`] exactly as it did before this rule existed.
///
/// **Refusing it would turn round 7's entire callee-grammar generative property
/// permanently red in a file this module's rules may not edit**:
/// `tests/envelope_wrapper_class.rs`'s `CALLEE_KNOWN_LEADING_PREFIX` is
/// `"-c a=b"` and every case of that property is spliced behind it.
///
/// # WHY A FAIL-CLOSED DEFAULT WAS NOT AVAILABLE, AND THE THREE REJECTED OPTIONS
///
/// Rounds 3, 5, 6 and 7 each made the guard's SILENCE a refusal. **This rule
/// cannot, because the complement is unbounded**: a fail-closed default over
/// config sections would have to refuse every section the guard has not
/// enumerated, and users legitimately set arbitrary ones —
/// `git -c user.name="$NAME" commit -m x` is this axis's ordinary invocation, and
/// a control that refuses ordinary configuration gets switched off (AR-19-11).
/// What is available instead is the closed grammatical fact above. **See
/// [`INDIRECTION_SECTIONS`] for the residue that leaves, which is stated there
/// and is not covered by any control.**
///
/// Three options were rejected on measured grounds:
///
/// 1. **Add `include.path` to [`is_hooks_path_key`]** — `includeIf.<cond>.path`
///    is an open family, so the list is wrong the moment a condition type is
///    used; and it would attribute the refusal to
///    [`ParkReason::HookBypassBlocked`] on a line where the guard established no
///    hooks-path write, which is D-24's own prohibition.
/// 2. **Read the included file and resolve the config at guard time** — the
///    guard runs synchronously on the `PreToolUse` critical path where a
///    reproduced 180-240 second hang is why [`push_needs_resolved_dests`]
///    exists; the file may not exist at guard time or may change between guard
///    and exec; and `includeIf`'s conditions depend on the repository the
///    command will run in, which a pure argv function does not know.
/// 3. **Refuse every `-c`** — pinned red by the permitted half above.
fn config_key_names_an_indirection_section(key: &str) -> bool {
    match config_key_section(key) {
        Some(section) => INDIRECTION_SECTIONS
            .iter()
            .any(|known| section.eq_ignore_ascii_case(known)),
        // No `.`, so no section, so provably not an indirection. CONFINED.
        None => false,
    }
}

/// The refusal an unbounded config assignment earns, naming the KEY and the
/// SECTION and never quoting the command back (SAFE-04).
///
/// **It names what the guard could not establish and what the user can do**, on
/// the same footing as [`scan_leading`]'s two existing refusals, because a
/// refusal a user cannot act on is a control that gets switched off (AR-19-11).
/// **Both recovery steps are ones git accepts** — setting the key directly and
/// dropping the directive — which is the correction `19-21`'s attached-spelling
/// wording needed: a message must not promise a step git may reject.
fn unbounded_config_assignment_refusal(key: &str, section: &str) -> GitVerdict {
    refuse(
        ParkReason::EnvelopeAssertionFailed,
        format!(
            "this command sets the git configuration key `{key}`, whose `{section}` \
             section splices in configuration from a file the guard has not read and \
             cannot read before the command runs, so what core.hooksPath will be while \
             this command executes — the setting the envelope's hook layer IS — cannot \
             be established; the command is refused rather than guessed at. To proceed: \
             set the key the included file would have set, directly, so the guard can \
             see it; or drop the directive"
        ),
    )
}

/// Whether a config key names `core.hooksPath`, at any casing git accepts.
///
/// **The whole-key `eq_ignore_ascii_case` fold is MORE permissive than git for a
/// key that carries a subsection, and that is the safe direction.** Git folds
/// the section and the variable and leaves the subsection case-sensitive, so a
/// guard that folds the whole key can only ever match keys git also matches
/// plus some it does not — an over-refusal, never a miss. `core.hooksPath` has
/// no subsection, so today the difference is not reachable; the sentence is
/// recorded because it was previously implicit.
fn is_hooks_path_key(key: &str) -> bool {
    key.eq_ignore_ascii_case("core.hookspath")
}

/// Long push options that take a value, so their value is never read as a
/// refspec.
///
/// **`recurse-submodules` was added and NOTHING ELSE, and the one-entry size of
/// the change is the point rather than an economy.** Without it
/// `git push --recurse-submodules on-demand origin refs/heads/gsd-auto/alpha/w`
/// was refused at `push_outside_namespace` — `on-demand` became the repository
/// and `origin` became a refspec resolving to `refs/heads/origin` — while real
/// git runs the line to completion (`Everything up-to-date`). That was
/// `T-19-102`: an over-refusal of an ordinary in-namespace push.
///
/// **`signed` is NOT added, and it is the control that proves the list was not
/// completed from `git push -h`.** The help text spells
/// `--recurse-submodules (check|on-demand|no)` and
/// `--signed[=(yes|no|if-asked)]` — a REQUIRED separate value and an
/// ATTACHED-ONLY optional one — and the two look alike while git treats them
/// differently. Measured: `git push --dry-run --recurse-submodules on-demand
/// <repo> <ref>` runs, while `git push --dry-run --signed no <repo> <ref>`
/// answers `error: src refspec ... does not match any`, because `no` becomes the
/// repository. Adding `signed` would make the guard skip a word git reads as an
/// operand — a real MIS-PARSE bought in exchange for removing a false refusal.
/// `19-20` pinned `git push --signed no origin refs/heads/gsd-auto/alpha/w`
/// REFUSED, so a list completed from the help text lands red.
///
/// **Why [`push_operands`] does NOT get [`scan_leading`]'s fail-closed default,
/// asked and answered here because a reader will ask.** The two enumerations have
/// the same shape and OPPOSITE failure directions. `scan_leading`'s unknown-option
/// direction is a BYPASS: the option's value becomes the verb, and the verb is
/// what the denylist reads. This one's is an OVER-REFUSAL: an unconsumed value
/// becomes an extra refspec, which refuses. A fail-closed `push_operands` would
/// refuse `git push --dry-run origin <ref>` and every other one-word push flag —
/// a control that fails into unusability and gets switched off (AR-19-11).
///
/// **But it IS fail-open in the OVER-consuming direction, exactly as
/// `GIT_GLOBAL_VALUE_OPTS` was**: an entry git does not treat as value-taking
/// makes the guard skip a word git reads as a refspec. That direction is covered
/// by the real-git drift pin
/// [`every_push_value_opt_really_consumes_its_value_and_signed_is_the_control_that_proves_it`],
/// which probes every entry against the installed binary and carries `--signed`
/// and `--dry-run` as its negative controls — not by a fail-closed default.
const PUSH_VALUE_OPTS: &[&str] = &[
    "repo",
    "push-option",
    "receive-pack",
    "exec",
    "recurse-submodules",
];

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

/// One walk of a `git config` argv: its operands paired with their indices into
/// `rest`, and the three facts the classifier decides on.
struct ConfigScan<'a> {
    /// Every operand, with the index into `rest` it was found at.
    operands: Vec<(usize, &'a str)>,
    /// Where the KEY operands begin — after the subcommand form's verb, if the
    /// argv uses that form.
    key_start: usize,
    is_read: bool,
    is_write: bool,
    whole_file_write: bool,
}

impl<'a> ConfigScan<'a> {
    /// The operand [`classify_config`] tests with [`is_hooks_path_key`], with
    /// its index into `rest`.
    fn key_operand(&self) -> Option<(usize, &'a str)> {
        self.operands.get(self.key_start).copied()
    }

    /// How many key operands there are — what tells `git config <key> <value>`
    /// (a write) from `git config <key>` (a read).
    fn key_operand_count(&self) -> usize {
        self.operands.len().saturating_sub(self.key_start)
    }
}

/// The single walk of a `git config` argv, over which both
/// [`classify_config`] and the `config` half of
/// [`first_unreadable_decision_word`] are defined.
///
/// **The index primitive, for the reason [`scan_leading`]'s doc already records
/// for its two callers**: a second copy of a loop is a second thing to keep in
/// step with git's own option grammar, and the day they drift is the day the
/// rule guards a different word than the one the classifier reads.
fn scan_config<'a>(rest: &[&'a str]) -> ConfigScan<'a> {
    let mut operands: Vec<(usize, &'a str)> = Vec::new();
    let mut is_read = false;
    let mut is_write = false;
    let mut whole_file_write = false;
    let mut index = 0;

    while index < rest.len() {
        let token = rest[index];
        if !token.starts_with('-') || token == "-" {
            operands.push((index, token));
            index += 1;
            continue;
        }
        if token == "--" {
            operands.extend(
                rest[index + 1..]
                    .iter()
                    .enumerate()
                    .map(|(offset, token)| (index + 1 + offset, *token)),
            );
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
    let mut key_start = 0;
    if let Some((_, first)) = operands.first() {
        if CONFIG_WRITE_SUBCOMMANDS.contains(first) {
            is_write = true;
            if *first == "edit" {
                whole_file_write = true;
            }
            key_start = 1;
        } else if CONFIG_READ_SUBCOMMANDS.contains(first) {
            is_read = true;
            key_start = 1;
        }
    }

    ConfigScan {
        operands,
        key_start,
        is_read,
        is_write,
        whole_file_write,
    }
}

/// The index into `rest` of the operand [`classify_config`] would test with
/// [`is_hooks_path_key`], if there is one.
///
/// `rest` is the argv AFTER the `config` verb, as [`classify_config`] receives
/// it.
fn config_key_operand_index(rest: &[&str]) -> Option<usize> {
    scan_config(rest).key_operand().map(|(index, _)| index)
}

fn classify_config(rest: &[&str]) -> GitVerdict {
    let scan = scan_config(rest);
    let ConfigScan {
        is_read,
        mut is_write,
        whole_file_write,
        ..
    } = scan;

    // The classic form: `git config <key> <value>` is a write, `git config
    // <key>` alone prints the value and is a read.
    if !is_read && !is_write && scan.key_operand_count() >= 2 {
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
        if let Some((_, key)) = scan.key_operand() {
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
    /// For an OPERATOR token only: whether it **severed a word** — a word was in
    /// progress at the instant the character arrived.
    ///
    /// **The discriminator, and the cases it tells apart, because that is the
    /// whole of Rule B's safety.** It is computed for `(`, `)`, `{` and `}` and
    /// for nothing else:
    ///
    /// * `${X}push`, `$(true)push`, `${C}NT` and `${C}` have a word in progress
    ///   at every one of their boundaries, so each boundary is a flush and the
    ///   fragment after it CONTINUES the enclosing word;
    /// * `{ cmd; }` and `( cmd )` have a word in progress at NEITHER, because
    ///   bash's own grammar requires it: `{` is a reserved word and must be
    ///   followed by whitespace, and `}` must follow a `;` or a newline. A
    ///   grouped command therefore keeps exactly today's segments.
    ///
    /// **The condition is a word BEFORE the character, never a word after it.**
    /// The latter would mark `(cd /tmp && ls)` and refuse an ordinary subshell.
    ///
    /// **`;`, `|`, `&` and newline are always `false`**, and that is a safety
    /// property rather than an omission: they are command operators in every
    /// context, and marking `hi&&git` or `$X|grep` as a severed word would merge
    /// two separate commands into one and refuse the second.
    ///
    /// What quoting already handles, so the flag never has to: a substitution
    /// inside double quotes is consumed by the quote loop and never reaches the
    /// separator arm at all, so `"$(pwd)"` produces no flush marker.
    ///
    /// Always `false` for an ordinary word.
    pub word_splitting_flush: bool,
    /// **POSITIVE evidence that the shell hands this word to the program
    /// byte-identically to how it is written.**
    ///
    /// This is the inversion round 5 makes, and the direction is the whole of
    /// it. `expansion` above answers "did the tokenizer SEE one of the two
    /// characters that mark an expansion" — an enumeration of the ways bash
    /// assembles a word, guarded one at a time, which has not converged in five
    /// rounds. This answers the opposite question: **is this word literal**, so
    /// that anything not provably literal is unresolvable and refuses. Brace
    /// expansion, pathname expansion, tilde expansion, `$IFS`-driven re-splitting
    /// and the mechanisms nobody has enumerated are one rule rather than five.
    ///
    /// **The evidence is collected HERE, while the word is consumed, and it can
    /// never be recovered from [`Token::text`] afterwards.** `text` has had its
    /// quoting removed, so in it `gh api "repos/{owner}/{repo}/pulls"` — literal,
    /// and COUNTED today — is indistinguishable from an unquoted word the shell
    /// rewrites.
    ///
    /// Cleared when the tokenizer consumes, **OUTSIDE quotes**:
    ///
    /// | class | characters | why it is not literal |
    /// |---|---|---|
    /// | expansion | `$`, `` ` `` | parameter, command and arithmetic expansion — and every `$IFS` re-split of the result. The value is unknowable here |
    /// | pathname | `*`, `?`, `[` | the result depends on the working directory, so it is unknowable **whether or not a file matches today** — a precondition an agent satisfies with `touch push` in the same tool call |
    /// | tilde | `~` | the result depends on the passwd database of the machine the command will run on |
    /// | brace | a `{`…`}` pair classified as an EXPANSION, or a `{` with no match | bash splices its alternatives back into the enclosing command, so the word never exists as written |
    ///
    /// Inside DOUBLE quotes only `$` and `` ` `` clear it: the other classes do
    /// not expand there, and treating them as if they did would refuse
    /// `gh api "repos/{owner}/{repo}/pulls"`, which is counted today.
    ///
    /// **Deliberately NOT cleared** for anything inside single quotes; for a
    /// backslash-escaped character, because escaping is exactly what makes a
    /// character literal; or for a `{`…`}` pair with no comma and no range, which
    /// bash passes through unchanged — `printf "[%s]" repos/{owner}/{repo}/pulls`
    /// prints `[repos/{owner}/{repo}/pulls]`, measured.
    ///
    /// **This is not a second `expansion`.** `expansion` stays exactly as it is,
    /// because [`resolve_program`] steps 3 and 5 decide on it and this rule does
    /// not move them; this bit is read at the DECISION boundary, in the one
    /// closure of [`first_unreadable_decision_word`].
    pub literal: bool,
    /// For an OPERATOR token only: this `{` or `}` belongs to a `{`…`}` pair the
    /// tokenizer classified as a brace EXPANSION, or to a `{` it could not match
    /// at all.
    ///
    /// A brace expansion is a property of the enclosing SIMPLE COMMAND rather
    /// than of a word — bash splices its alternatives back in around whatever
    /// else is there — so the fact is carried to [`Segment`] and applied to every
    /// segment of that command. See [`Segment::brace_spliced`].
    pub brace_splice: bool,
    /// For the OPERATOR token that opened the word's FIRST brace expansion only:
    /// a word the splice can PRODUCE has a basename in [`GOVERNED_PROGRAMS`], or
    /// its products could not be enumerated at all.
    ///
    /// **Products, never names**, and the two are one slot apart:
    /// `{g..g}it push --force origin main` runs `git push --force origin main`
    /// under bash and its only alternative is `g`, which names nothing.
    /// See [`brace_word_products`] for the whole-word scan that computes them and
    /// for every trigger that makes a product set unenumerable.
    pub splice_can_produce_governed: bool,
    /// For an OPERATOR token only: an unquoted `<` or `>` the redirection
    /// production could NOT resolve into a complete (operator, target) pair.
    ///
    /// **The fail-closed residue that keeps the production from being a sixth
    /// enumeration.** Bash's redirection rule is a finite grammar production —
    /// `[IO_NUMBER] OPERATOR WORD` — and [`tokenize`] consumes it, emitting no
    /// token for either the operator or its target. A spelling the production
    /// does not COMPLETE does not fall through as an ordinary word: it is marked
    /// here, carried to [`Segment::redirection_unresolvable`] and refused when a
    /// governed program is reached.
    ///
    /// Set for exactly two shapes, both measured:
    ///
    /// * an operator with no following target word — `git >`, which bash does
    ///   not run either (`syntax error near unexpected token 'newline'`);
    /// * a `{name}` fd-allocation prefix (bash 4.1) — see
    ///   [`is_fd_allocation_prefix`] for why it is not modelled.
    ///
    /// Always `false` for an ordinary word and for a resolved redirection.
    pub redirection_unresolvable: bool,
}

/// The shell control operators that end one simple command and begin the next.
///
/// `>` and `<` are deliberately absent: a redirection does not start a new
/// command, so treating it as a separator would hide the command it redirects.
///
/// **That reasoning is right, and only the sentence that used to follow it was
/// wrong.** It read "it stays an ordinary word and travels into the classifier
/// with the rest" — which is exactly `T-19-97`: bash DELETES a redirection's
/// operator and its target from argv before `execve`, so a word that travelled
/// into the classifier was a word the program never received, displacing every
/// decision word one slot right. What `<` and `>` are is what bash's lexer
/// already makes them: **word-terminating METACHARACTERS whose operator and
/// target are deleted**, which is neither a separator nor an ordinary word.
/// [`tokenize`] consumes that production; this list is unchanged, and
/// [`is_separator`] stays `false` for `>` because a redirection still does not
/// start a command.
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
    Some(
        split_segments_with_heads(cmd)?
            .into_iter()
            .map(|segment| segment.tokens)
            .collect(),
    )
}

/// One simple command, with the facts about its CONTEXT that the tokens
/// themselves cannot carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// The words of the simple command, exactly as [`split_segments`] reports
    /// them.
    pub tokens: Vec<Token>,
    /// Whether the first token of this segment is at a **command position**.
    ///
    /// False when the operator IMMEDIATELY preceding the segment was a
    /// word-splitting CLOSER — `}` or `)` with a word in progress before it (see
    /// [`Token::word_splitting_flush`]). Such a segment is a fragment continuing
    /// the enclosing word after an expansion, so what the shell will put in front
    /// of its first token is unknowable.
    pub head_is_command_position: bool,
    /// Whether this segment belongs to a **simple command a brace expansion
    /// splices into**.
    ///
    /// **Why this is a property of the command and not of a word.**
    /// `git {push,--force} origin main` splits into `git`, `push,--force` and
    /// `origin main`. The first segment resolves `Governed` with an EMPTY argv
    /// and [`classify_git`] answers `Allow` for a bare `git`, so there is no
    /// decision WORD to test at all — while bash runs
    /// `git push --force origin main`. The argv the classifier reads is not the
    /// argv that runs, so the whole simple command is unresolvable.
    ///
    /// Computed in [`split_segments_with_heads`] — the ONE walk — from the
    /// operator tokens the tokenizer marked, applied to every segment of the
    /// simple command **including the one already pushed when the `{` arrived**,
    /// and reset at each REAL command operator (`;`, `&&`, `||`, `|`, `&`,
    /// newline) because those end the simple command.
    pub brace_spliced: bool,
    /// Whether a word this command's splice can PRODUCE has a basename in
    /// [`GOVERNED_PROGRAMS`], or the products could not be enumerated at all.
    ///
    /// **The second half of clause 2, and both halves are load-bearing.**
    /// [`Segment::brace_spliced`] alone permits `{git,push,--force,origin,main}`,
    /// whose single segment's basename is not a governed program and where
    /// nothing resolves `Governed` at all. This half alone permits
    /// `git push {--force,origin} main`, where the governed word is outside the
    /// braces and what the splice hides is `--force`.
    ///
    /// A product set that could not be computed cannot be cleared, so
    /// unenumerable answers `true` here — fail closed.
    pub splice_can_produce_governed: bool,
    /// Whether this segment belongs to a **simple command carrying a redirection
    /// the parser could not resolve** (see [`Token::redirection_unresolvable`]).
    ///
    /// **A property of the COMMAND and not of a word**, for the same reason
    /// [`Segment::brace_spliced`] is: a redirection stands anywhere in a simple
    /// command and deletes words from anywhere in its argv.
    ///
    /// Computed in [`split_segments_with_heads`] — the ONE walk — from the
    /// operator tokens the tokenizer marked, applied to every segment of the
    /// simple command **including the one already pushed when the operator
    /// arrived**, and reset at each REAL command operator.
    ///
    /// **It refuses only when a GOVERNED program is reached**, the cost
    /// containment [`Segment::brace_spliced`] already establishes: `ls >` stays
    /// permitted, `git >` does not.
    pub redirection_unresolvable: bool,
}

/// [`split_segments`], plus a per-segment report of whether its head is at a
/// command position.
///
/// **The one scan, not a second one.** `split_segments` is defined over this
/// function rather than beside it, because the defect this whole round is about
/// is a decision region derived from a scan other than the one the classifier
/// runs — it has sat one slot over four times in this phase. There is one walk of
/// the tokens here and every caller reads its answer.
///
/// **The opener is EXCLUDED, and that exclusion is what keeps ordinary shell
/// working.** A segment following `{` is the parameter expansion's variable NAME
/// and can never be a governed program. A segment following `(` is the command
/// substitution's OWN CONTENTS, which IS at a genuine command position and must
/// keep being classified rather than refused — which is what leaves
/// `echo $(git rev-parse HEAD)`, `git log --format=%h $(git rev-parse HEAD)`,
/// `ROOT=$(git rev-parse --show-toplevel)`, `DIR=$(mktemp -d)`,
/// `RUN_ID=$(uuidgen)`, `CONFIG=$(cat cfg)` and `COMMAND=$(which git)`
/// permitted. The first of those is named in [`resolve_program`]'s own doc as
/// the bill of closing `T-19-74`.
///
/// **"IMMEDIATELY preceding" is load-bearing.** In `(git status)&&git fetch
/// origin` the second segment's last preceding operator is `&&`, not `)`, so it
/// is untouched. A rule that looked further back than one operator would refuse
/// an ordinary sequence.
pub fn split_segments_with_heads(cmd: &str) -> Option<Vec<Segment>> {
    let tokens = tokenize(cmd)?;
    let mut segments: Vec<Segment> = Vec::new();
    let mut current: Vec<Token> = Vec::new();
    // The state of the operator most recently passed. Before any operator there
    // is nothing in front of the first token but the start of the line, which is
    // a command position.
    let mut last_operator_severed = false;
    let mut head_is_command_position = true;
    // The brace facts of the simple command being accumulated. `command_start`
    // is where its segments begin in `segments`, so a `{` that arrives AFTER a
    // segment was already pushed — `git {push,--force} origin main`, where `git`
    // is pushed the instant the `{` is seen — still marks that segment. This is
    // the same walk, not a second one.
    let mut command_start = 0usize;
    let mut brace_spliced = false;
    let mut splice_can_produce_governed = false;
    // The redirection fact of the simple command being accumulated, carried the
    // same way and for the same reason: an unresolvable `>` that arrives after
    // `git` was already pushed — `git >` — must still mark that segment.
    let mut redirection_unresolvable = false;

    for token in tokens {
        if token.operator {
            if !current.is_empty() {
                segments.push(Segment {
                    tokens: std::mem::take(&mut current),
                    head_is_command_position,
                    brace_spliced,
                    splice_can_produce_governed,
                    redirection_unresolvable,
                });
            }
            if token.redirection_unresolvable {
                redirection_unresolvable = true;
                // Retroactive for the same reason the brace mark is: a
                // redirection belongs to the whole simple command, so everything
                // since the last REAL command operator carries the fact.
                for segment in &mut segments[command_start..] {
                    segment.redirection_unresolvable = true;
                }
            }
            if token.brace_splice {
                brace_spliced = true;
                splice_can_produce_governed |= token.splice_can_produce_governed;
                // Retroactive, because bash splices words back in around
                // whatever is already there: everything since the last REAL
                // command operator is part of the command this `{` splices into.
                for segment in &mut segments[command_start..] {
                    segment.brace_spliced = true;
                    segment.splice_can_produce_governed |= splice_can_produce_governed;
                }
            }
            // A REAL command operator ends the simple command; `(`, `)`, `{` and
            // `}` do not, which is exactly why the fact is per-command rather
            // than per-segment.
            if matches!(token.text.as_str(), ";" | "&&" | "||" | "|" | "&" | "\n") {
                command_start = segments.len();
                brace_spliced = false;
                splice_can_produce_governed = false;
                redirection_unresolvable = false;
            }
            last_operator_severed =
                token.word_splitting_flush && matches!(token.text.as_str(), "}" | ")");
            continue;
        }
        if current.is_empty() {
            head_is_command_position = !last_operator_severed;
        }
        current.push(token);
    }
    if !current.is_empty() {
        segments.push(Segment {
            tokens: current,
            head_is_command_position,
            brace_spliced,
            splice_can_produce_governed,
            redirection_unresolvable,
        });
    }

    Some(segments)
}

/// The characters that make a word non-literal because the SHELL rewrites it or
/// because its result is not knowable at guard time.
///
/// `{` is deliberately absent: whether a `{` rewrites the word is a question
/// about the PAIR, answered by the tokenizer's three-way classification, not
/// about the character. `printf "[%s]" repos/{owner}/{repo}/pulls` prints its
/// argument unchanged.
const REWRITING_CHARACTERS: &[char] = &['$', '`', '*', '?', '[', '~'];

/// The most words one brace-expanded shell word may produce before the guard
/// stops enumerating and refuses instead.
///
/// A cap rather than an unbounded product, because the whole point of the scan
/// is that it runs on the agent's critical path inside `PreToolUse`; a word over
/// the cap is UNENUMERABLE, which is a refusal. `{a..z}` is 26 and fits.
const MAX_BRACE_PRODUCTS: usize = 64;

/// What the whole-word brace scan concluded about one shell word.
#[derive(Debug, PartialEq, Eq)]
struct BraceProducts {
    /// Every word the splice can produce, or `None` when they could not be
    /// enumerated at all.
    products: Option<Vec<String>>,
}

impl BraceProducts {
    /// Clause 2(b)'s question. `None` answers `true`: **a product set that
    /// cannot be computed cannot be cleared**, so unenumerable fails closed.
    fn can_produce_governed(&self) -> bool {
        match &self.products {
            None => true,
            Some(products) => products
                .iter()
                .any(|product| GOVERNED_PROGRAMS.contains(&program_name(product))),
        }
    }

    /// The unenumerable answer, spelled once so every trigger returns the same
    /// thing.
    fn unenumerable() -> Self {
        Self { products: None }
    }
}

/// Every word a brace-expanded shell word can PRODUCE, composed over the WHOLE
/// WORD.
///
/// **Products, never names, and that distinction is the finding that reshaped
/// round 5.** A splice concatenated with literal characters produces a governed
/// program no alternative spells. All eight of these are measured at exit 0
/// against the pre-`19-17` tree, and bash assembles the range spellings into a
/// real `git push --force origin main`:
///
/// ```text
/// {g..g}it push --force origin main            {g,g}{i,i}{t,t} push --force origin main
/// g{i,i}t push --force origin main             {g..g..1}it push --force origin main
/// {g..g}{i..i}t push --force origin main       {g{i,i}t,x} push --force origin main
/// "g"{i,i}"t" push --force origin main         {g..g}"it" push --force origin main
/// ```
///
/// None is reached by asking what the alternatives are CALLED — they are `g`,
/// `i`, `t` and `x`, which name nothing — nor by the word-level literalness bit,
/// because the composed word never exists as a token.
///
/// **The extent is the WHOLE WORD and it never stops at a `{` or a `}`.** It
/// runs from `start` — the beginning of the word in progress when the first
/// expansion `{` arrived — forward to the next unquoted whitespace or REAL
/// command operator (`;`, `&&`, `||`, `|`, `&`, newline). **Bash composes across
/// `{`s**, so a per-`{` computation whose prefix and suffix runs ended at a brace
/// would answer `g` and `it` for `{g..g}{i..i}t` — neither governed, both sets
/// enumerating cleanly — and rows 3, 4 and 5 above would be permitted. That is
/// the whole of this clause's safety and it is stated at the definition rather
/// than as a caveat somewhere else.
///
/// **Literal runs are joined with their QUOTING REMOVED**, on the same footing
/// [`Token::text`] already stands on, because quote removal is bash's own last
/// word-expansion step. A quoted run IS literal by the Token bit's own test, so
/// a word of quoted runs enumerates cleanly — and joined as written the product
/// of `"g"{i,i}"t"` reads `"g"i"t"`, whose basename is not governed.
///
/// **A word whose products cannot be ENUMERATED is refused, and every trigger
/// lives here beside the scan** — an escape hatch a shorter scan can never reach
/// is not a control:
///
/// * an unmatched `{` or a stray `}` anywhere in the extent;
/// * a range that is not exactly two endpoints, **which explicitly includes the
///   increment form `{a..b..n}`**: it is neither a comma list nor a two-endpoint
///   range, and `{g..g..1}it push --force origin main` is measured at exit 0;
/// * a range whose endpoints are not both single characters or both integers;
/// * **an alternative or a range member that itself contains a `{`** — the
///   NESTED case, taken as a TRIGGER rather than by composing recursively,
///   because a fail-closed refusal needs no recursion depth, no bound and no
///   second reading where a recursive composition needs all three.
///   `{g{i,i}t,x} push --force origin main` is measured at exit 0 and its
///   top-level alternatives are `g{i,i}t` and `x`, neither governed;
/// * an expansion or a literal run carrying a `$`, a backtick, a glob character
///   or a tilde — the same class [`Token::literal`] is cleared for;
/// * a quote inside an expansion's contents, which this scan does not resolve;
/// * a cartesian count over [`MAX_BRACE_PRODUCTS`].
///
/// **This is narrower than a mention test, not wider**, and the control is
/// pinned: `ls {git,svn}-repo` produces `git-repo` and `svn-repo`, neither of
/// whose basename is governed, and stays PERMITTED.
///
/// Hand-rolled against `std`, like every other parser in this module, so no
/// crate is added (`T-19-SC`).
fn brace_word_products(chars: &[char], start: usize) -> BraceProducts {
    // Each element is the alternatives one part of the word contributes. A
    // literal run contributes exactly one.
    let mut parts: Vec<Vec<String>> = Vec::new();
    let mut run = String::new();
    let mut unenumerable = false;
    let mut index = start;

    while index < chars.len() {
        let ch = chars[index];
        match ch {
            // The extent ends here: unquoted whitespace, or a REAL command
            // operator. Never a `{` and never a `}`.
            ' ' | '\t' | '\r' | '\n' | ';' | '|' | '&' => break,
            '\'' => {
                index += 1;
                loop {
                    match chars.get(index) {
                        Some('\'') => {
                            index += 1;
                            break;
                        }
                        Some(inner) => {
                            run.push(*inner);
                            index += 1;
                        }
                        None => {
                            unenumerable = true;
                            break;
                        }
                    }
                }
                if unenumerable {
                    break;
                }
            }
            '"' => {
                index += 1;
                loop {
                    match chars.get(index) {
                        Some('"') => {
                            index += 1;
                            break;
                        }
                        Some('\\') => {
                            match chars.get(index + 1) {
                                Some(esc @ ('"' | '\\' | '$' | '`')) => run.push(*esc),
                                Some(other) => {
                                    run.push('\\');
                                    run.push(*other);
                                }
                                None => unenumerable = true,
                            }
                            index += 2;
                        }
                        // Expansion still happens inside double quotes, so the
                        // run is not literal and the product is not knowable.
                        Some('$' | '`') => {
                            unenumerable = true;
                            break;
                        }
                        Some(inner) => {
                            run.push(*inner);
                            index += 1;
                        }
                        None => {
                            unenumerable = true;
                            break;
                        }
                    }
                    if unenumerable {
                        break;
                    }
                }
                if unenumerable {
                    break;
                }
            }
            '\\' => match chars.get(index + 1) {
                // Escaping is exactly what makes a character literal.
                Some(escaped) => {
                    run.push(*escaped);
                    index += 2;
                }
                None => {
                    unenumerable = true;
                    break;
                }
            },
            '{' => {
                let Some(close) = matching_brace(chars, index) else {
                    unenumerable = true;
                    break;
                };
                let contents: String = chars[index + 1..close].iter().collect();
                match brace_alternatives(&contents) {
                    BracePair::Expansion(alternatives) => {
                        parts.push(vec![std::mem::take(&mut run)]);
                        parts.push(alternatives);
                    }
                    // No comma and no range: bash passes it through unchanged,
                    // so it is ordinary literal text in this word.
                    BracePair::Literal => {
                        run.push('{');
                        run.push_str(&contents);
                        run.push('}');
                    }
                    BracePair::Unenumerable => {
                        unenumerable = true;
                        break;
                    }
                }
                index = close + 1;
            }
            // A `}` reached at the top level of the extent has no opener: an
            // unmatched brace, which is unenumerable.
            '}' => {
                unenumerable = true;
                break;
            }
            _ => {
                if REWRITING_CHARACTERS.contains(&ch) {
                    unenumerable = true;
                    break;
                }
                run.push(ch);
                index += 1;
            }
        }
    }

    if unenumerable {
        return BraceProducts::unenumerable();
    }
    parts.push(vec![run]);

    let count = parts
        .iter()
        .try_fold(1usize, |total, part| total.checked_mul(part.len()));
    match count {
        Some(total) if total <= MAX_BRACE_PRODUCTS => {}
        _ => return BraceProducts::unenumerable(),
    }

    // The cartesian product itself, over every expansion in the word.
    let mut products = vec![String::new()];
    for part in &parts {
        let mut next = Vec::with_capacity(products.len() * part.len());
        for product in &products {
            for alternative in part {
                next.push(format!("{product}{alternative}"));
            }
        }
        products = next;
    }

    BraceProducts {
        products: Some(products),
    }
}

/// What one `{`…`}` pair is, in bash's own terms.
enum BracePair {
    /// A comma list or a two-endpoint range: bash splices these alternatives
    /// back into the enclosing command.
    Expansion(Vec<String>),
    /// No comma and no range: bash passes the pair through unchanged, which is
    /// what makes `gh`'s documented `repos/{owner}/{repo}/pulls` one word.
    Literal,
    /// The pair exists but its alternatives cannot be enumerated. Refuse.
    Unenumerable,
}

/// The index of the `}` matching the `{` at `open`, counting nesting and
/// respecting quotes, or `None` for an unmatched brace.
fn matching_brace(chars: &[char], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut index = open;
    while index < chars.len() {
        match chars[index] {
            '\\' => index += 1,
            '\'' => {
                index += 1;
                while index < chars.len() && chars[index] != '\'' {
                    index += 1;
                }
                if index >= chars.len() {
                    return None;
                }
            }
            '"' => {
                index += 1;
                while index < chars.len() && chars[index] != '"' {
                    if chars[index] == '\\' {
                        index += 1;
                    }
                    index += 1;
                }
                if index >= chars.len() {
                    return None;
                }
            }
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
        index += 1;
    }
    None
}

/// Classify one brace pair's contents and enumerate its alternatives.
///
/// Comma FIRST, because bash reads `{a,b..c}` as a three-way… no: as the
/// two-element comma list `a` and `b..c`. A comma anywhere at the top level
/// makes the pair a list.
fn brace_alternatives(contents: &str) -> BracePair {
    let chars: Vec<char> = contents.chars().collect();
    let commas = top_level_split(&chars, ',');
    if commas.len() > 1 {
        return match members_are_enumerable(&commas) {
            true => BracePair::Expansion(commas),
            false => BracePair::Unenumerable,
        };
    }

    if contents.contains("..") {
        // A range. Exactly two endpoints or nothing: `{a..b..n}` is the
        // INCREMENT form, which is neither a comma list nor a two-endpoint
        // range, and `{g..g..1}it push --force origin main` is measured at exit
        // 0 against the pre-`19-17` tree.
        let endpoints: Vec<&str> = contents.split("..").collect();
        if endpoints.len() != 2 {
            return BracePair::Unenumerable;
        }
        let (from, to) = (endpoints[0], endpoints[1]);
        if let (Ok(from), Ok(to)) = (from.parse::<i64>(), to.parse::<i64>()) {
            let span = (from - to).unsigned_abs() as usize;
            if span >= MAX_BRACE_PRODUCTS {
                return BracePair::Unenumerable;
            }
            let range: Vec<String> = if from <= to {
                (from..=to).map(|n| n.to_string()).collect()
            } else {
                (to..=from).rev().map(|n| n.to_string()).collect()
            };
            return BracePair::Expansion(range);
        }
        let (from, to) = (from.chars().collect::<Vec<_>>(), to.chars().collect::<Vec<_>>());
        if from.len() != 1 || to.len() != 1 {
            return BracePair::Unenumerable;
        }
        let (from, to) = (from[0] as u32, to[0] as u32);
        if from.abs_diff(to) as usize >= MAX_BRACE_PRODUCTS {
            return BracePair::Unenumerable;
        }
        let range: Vec<String> = if from <= to {
            (from..=to).filter_map(char::from_u32).map(String::from).collect()
        } else {
            (to..=from)
                .rev()
                .filter_map(char::from_u32)
                .map(String::from)
                .collect()
        };
        return match members_are_enumerable(&range) {
            true => BracePair::Expansion(range),
            false => BracePair::Unenumerable,
        };
    }

    BracePair::Literal
}

/// Whether every alternative or range member is itself a literal run this scan
/// can compose.
///
/// A member containing a `{` is the NESTED case and is a fail-closed TRIGGER
/// rather than a recursion: `{g{i,i}t,x} push --force origin main` decomposes
/// into the top-level alternatives `g{i,i}t` and `x`, neither of which is
/// governed, and it is measured at exit 0 against the pre-`19-17` tree.
fn members_are_enumerable(members: &[String]) -> bool {
    members.iter().all(|member| {
        !member.contains('{')
            && !member.contains('}')
            && !member.contains('\'')
            && !member.contains('"')
            && !member.chars().any(|ch| REWRITING_CHARACTERS.contains(&ch))
    })
}

/// Split on `separator` at brace depth zero and outside quotes.
fn top_level_split(chars: &[char], separator: char) -> Vec<String> {
    let mut parts = vec![String::new()];
    let mut depth = 0usize;
    let mut index = 0usize;
    while index < chars.len() {
        let ch = chars[index];
        match ch {
            '{' => {
                depth += 1;
                parts.last_mut().expect("one part always exists").push(ch);
            }
            '}' => {
                depth = depth.saturating_sub(1);
                parts.last_mut().expect("one part always exists").push(ch);
            }
            _ if ch == separator && depth == 0 => parts.push(String::new()),
            _ => parts.last_mut().expect("one part always exists").push(ch),
        }
        index += 1;
    }
    parts
}

/// The length of the redirection operator beginning at `index`, longest match
/// first, or `None` if no operator begins there.
///
/// **Bash's production is a FINITE grammar rule, which is why modelling it is
/// not a sixth enumeration.** Rounds 1–3 enumerated open-ended, value-dependent
/// ways a word can be REWRITTEN; this is one closed rule with twelve operators,
/// and anything it does not COMPLETE fails closed through
/// [`Token::redirection_unresolvable`] rather than falling through as a word.
///
/// `&>` and `&>>` are here, and the caller matches them BEFORE `&` reaches the
/// separator arm — see [`tokenize`]. Without that ordering
/// `git &>/tmp/o push --force origin main`, which is ONE simple command to bash
/// (`ARGV[git]: [push] [--force] [origin] [main]`, measured), is split into TWO
/// segments and a redirection parser running afterwards never sees it.
fn redirection_operator_len(chars: &[char], index: usize) -> Option<usize> {
    let at = |offset: usize| chars.get(index + offset).copied();
    match at(0)? {
        '<' => match (at(1), at(2)) {
            (Some('<'), Some('<')) => Some(3), // <<<  here-string
            (Some('<'), Some('-')) => Some(3), // <<-  heredoc, leading tabs stripped
            (Some('<'), _) => Some(2),         // <<   heredoc
            (Some('>'), _) => Some(2),         // <>   open read-write
            (Some('&'), _) => Some(2),         // <&   duplicate input fd
            _ => Some(1),                      // <
        },
        '>' => match at(1) {
            Some('>') => Some(2), // >>  append
            Some('|') => Some(2), // >|  clobber past `noclobber`
            Some('&') => Some(2), // >&  duplicate output fd
            _ => Some(1),         // >
        },
        '&' => match (at(1), at(2)) {
            (Some('>'), Some('>')) => Some(3), // &>>
            (Some('>'), _) => Some(2),         // &>
            _ => None,
        },
        _ => None,
    }
}

/// Advance past a redirection's TARGET word, returning the index after it, or
/// `None` when the production has no target to complete it.
///
/// The target gets **the same quoting rules any word gets**, because it is an
/// ordinary shell word — bash simply removes it from argv rather than passing
/// it. Its text is never needed, only its extent, because the whole point is
/// that it produces no [`Token`] at all.
///
/// For `<<` and `<<-` the target is the heredoc DELIMITER. The body is not argv
/// and this function does not model it: bash runs a single-line heredoc anyway
/// (warning `here-document at line 1 delimited by end-of-file`) and
/// `git <<EOF push --force origin main` gives `[push] [--force] [origin]
/// [main]`, measured.
fn skip_redirection_target(chars: &[char], mut index: usize) -> Option<usize> {
    // Bash allows whitespace between the operator and its target:
    // `git > /tmp/o push --force origin main` gives `[push] [--force] [origin]
    // [main]`, measured.
    while matches!(chars.get(index), Some(' ' | '\t')) {
        index += 1;
    }
    let start = index;
    while let Some(&ch) = chars.get(index) {
        match ch {
            // A metacharacter ends the target. A target that never began means
            // the production did not complete — `git >` — which fails closed.
            ' ' | '\t' | '\r' | '\n' | ';' | '|' | '&' | '(' | ')' | '<' | '>' => break,
            '\'' => {
                index += 1;
                loop {
                    match chars.get(index) {
                        Some('\'') => {
                            index += 1;
                            break;
                        }
                        Some(_) => index += 1,
                        // An unterminated quote has no knowable boundary.
                        None => return None,
                    }
                }
            }
            '"' => {
                index += 1;
                loop {
                    match chars.get(index) {
                        Some('"') => {
                            index += 1;
                            break;
                        }
                        Some('\\') if chars.get(index + 1).is_some() => index += 2,
                        Some(_) => index += 1,
                        None => return None,
                    }
                }
            }
            '\\' => {
                chars.get(index + 1)?;
                index += 2;
            }
            _ => index += 1,
        }
    }
    if index == start { None } else { Some(index) }
}

/// Whether the word in progress before a redirection operator is a bash 4.1
/// `{name}` FD-ALLOCATION prefix.
///
/// **Deliberately not modelled, and refused rather than guessed at.** Round 5's
/// literal-brace branch absorbs `{v}` into the word as ordinary characters
/// (correctly — `printf "[%s]" repos/{owner}/{repo}/pulls` prints its argument
/// unchanged), so unwinding that to model an fd allocation would cost the very
/// thing it buys. `git {v}>/tmp/o push --force origin main` is not a shape
/// anyone writes; it is marked unresolvable and refuses when it reaches a
/// governed program.
fn is_fd_allocation_prefix(word: &str) -> bool {
    let Some(inner) = word.strip_prefix('{').and_then(|w| w.strip_suffix('}')) else {
        return false;
    };
    !inner.is_empty()
        && !inner.starts_with(|c: char| c.is_ascii_digit())
        && inner.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// The quoting state machine behind [`split_command`] and [`split_segments`].
fn tokenize(cmd: &str) -> Option<Vec<Token>> {
    let chars: Vec<char> = cmd.chars().collect();
    let mut tokens: Vec<Token> = Vec::new();
    let mut text = String::new();
    let mut started = false;
    let mut expansion = false;
    // Positive evidence, per WORD: cleared the moment the tokenizer consumes
    // something outside quotes that the shell rewrites. See `Token::literal`.
    let mut literal = true;
    // Where the word in progress began in `chars`. The whole-word product scan's
    // extent starts here, not at the `{`.
    let mut word_start = 0usize;
    // We are inside a shell word a brace expansion splices. It survives the
    // `{`/`}` operator flushes — those cut the word into fragments but the SHELL
    // word continues — and is reset at a real word boundary.
    let mut splice_word = false;
    // Whether the character just consumed into the word was an UNQUOTED `$`.
    // `"$"{a,b}` is not a parameter expansion, and `text.ends_with('$')` cannot
    // tell the two apart because `text` has had its quoting removed already.
    let mut last_was_unquoted_dollar = false;
    // Whether every character of the word in progress arrived BARE — through the
    // ordinary-word arm, unquoted and unescaped — and was an ASCII digit. An
    // IO_NUMBER is a bare digits-only run and nothing else: measured,
    // `git "2">/tmp/o push --force origin main` gives bash
    // `ARGV[git]: [2] [push] [--force] [origin] [main]`, so a QUOTED digit run is
    // a real argv word. Reading `Token::text` here would not do — its quoting has
    // already been removed, so `"2"` and `2` are indistinguishable in it, which
    // is the same "the evidence must be collected while the word is consumed"
    // reasoning `Token::literal` is built on.
    let mut word_all_bare_digits = true;
    let mut index = 0usize;

    // A word ends; push it if one was started at all. `started` distinguishes
    // an empty word that was written (`""`) from no word at all.
    macro_rules! flush {
        () => {
            if started {
                tokens.push(Token {
                    text: std::mem::take(&mut text),
                    operator: false,
                    expansion,
                    word_splitting_flush: false,
                    literal: literal && !splice_word,
                    brace_splice: false,
                    splice_can_produce_governed: false,
                    redirection_unresolvable: false,
                });
                started = false;
                expansion = false;
                literal = true;
                word_all_bare_digits = true;
            }
        };
    }

    // `index` has already advanced past the character being handled, so the word
    // begins one back. The whole-word product scan's extent starts here.
    macro_rules! begin_word {
        () => {
            if !started {
                started = true;
                word_start = index - 1;
            }
        };
    }

    // **The redirection production, consumed inside the ONE walk.**
    //
    // The invariant, in one sentence: *the words the guard classifies must be
    // exactly the words the program receives, in the same order — no more and no
    // fewer.* This is the SECOND half of round 5's rule rather than a
    // replacement for it — `Token::literal` is the first half and proves a
    // word's BYTES; this proves its SURVIVAL and its SLOT. Both are needed:
    // `>/dev/null` is fully LITERAL by round 5's own test and the bit is RIGHT
    // about it, yet the program never receives it.
    //
    // **Consumed here, so a deleted word never becomes a `Token` at all**, which
    // is why every downstream index — `scan_leading`'s verb index,
    // `config_key_operand_index`, `subcommand_word_indices`, `scan_gh_api`'s own
    // walk — is over the SURVIVING argv automatically, and why
    // `first_unreadable_decision_word`'s one `at(index, role)` closure needs no
    // change. Round 3's principle is discharged, not weakened.
    //
    // `$op_start` is the index of the operator's FIRST character.
    macro_rules! consume_redirection {
        ($op_start:expr) => {{
            let op_start = $op_start;
            let mut unresolvable = false;

            // 1. **The fd prefix, which is a DIGITS-ONLY run since the start of
            //    the word and NOTHING ELSE.** Over-deletion is as dangerous as
            //    under-deletion — it displaces every decision word LEFT — and
            //    `git x2>/tmp/o push --force origin main` is the control:
            //    bash gives git `[x2] [push] [--force] [origin] [main]`, so `x2`
            //    is a real argv word the operator merely TERMINATED, and it is
            //    FLUSHED rather than discarded.
            if started {
                if word_all_bare_digits && !text.is_empty() {
                    // An IO_NUMBER, discarded with the redirection it belongs
                    // to: `git 2>/dev/null push …` gives `[push] …`.
                    text.clear();
                    started = false;
                    expansion = false;
                    literal = true;
                    word_all_bare_digits = true;
                } else {
                    // `{v}>` is not modelled; see `is_fd_allocation_prefix`.
                    unresolvable = is_fd_allocation_prefix(&text);
                    flush!();
                }
            }

            // 2. The operator, longest match first.
            let op_len = redirection_operator_len(&chars, op_start).unwrap_or(1);

            // 3. The target word — **and neither it nor the operator emits a
            //    token.** A production that does not complete fails closed
            //    rather than falling through as an ordinary word.
            match skip_redirection_target(&chars, op_start + op_len) {
                Some(after) => index = after,
                None => {
                    unresolvable = true;
                    index = op_start + op_len;
                }
            }

            if unresolvable {
                tokens.push(Token {
                    text: chars[op_start..op_start + op_len].iter().collect(),
                    operator: true,
                    expansion: false,
                    word_splitting_flush: false,
                    literal: true,
                    brace_splice: false,
                    splice_can_produce_governed: false,
                    redirection_unresolvable: true,
                });
            }

            // A redirection is a word boundary exactly as whitespace is.
            splice_word = false;
        }};
    }

    while index < chars.len() {
        let ch = chars[index];
        index += 1;
        // A `$` reaches the ordinary-word arm below and nowhere else, so this is
        // exactly "the previous character was an unquoted `$`".
        let previous_was_unquoted_dollar = last_was_unquoted_dollar;
        last_was_unquoted_dollar = ch == '$';
        match ch {
            ' ' | '\t' | '\r' => {
                flush!();
                splice_word = false;
            }
            '{' => {
                // **Bash's own three questions about a `{`, each answer naming
                // the case it tells apart.**
                //
                // 1. A `{` immediately preceded IN-WORD by an unquoted `$` is a
                //    PARAMETER expansion. Today's behaviour EXACTLY — separator,
                //    word-splitting flush — and **this case exists to keep Rule B
                //    load-bearing.** Fold it into case 3 and `${C}_COUNT` becomes
                //    one word: `C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin`
                //    would then be refused by `resolve_program` step 5's prefix
                //    rule instead of by Rule B, every verdict pin in the suite
                //    would stay green, and Rule B would quietly be dead code.
                //    `rule_b_still_reports_a_severed_head_as_not_a_command_position`
                //    is the mechanism pin that turns that red.
                let parameter_expansion = started && previous_was_unquoted_dollar;
                // 2. A `{` that is a COMPLETE WORD — nothing in progress before
                //    it and whitespace, a newline or end-of-input after it — is
                //    bash's reserved word opening a GROUP. Today's behaviour
                //    EXACTLY, so `{ cmd; }` keeps its segments byte-for-byte and
                //    the `echo hi && git push --force` class stays closed one
                //    level in.
                let reserved_word = !started
                    && chars
                        .get(index)
                        .is_none_or(|next| matches!(next, ' ' | '\t' | '\r' | '\n'));

                let mut brace_splice = false;
                let mut splice_can_produce_governed = false;
                if !parameter_expansion && !reserved_word {
                    // 3. Otherwise a brace-pair CANDIDATE, resolved by scanning
                    //    ahead for the matching `}`.
                    let opened = index - 1;
                    match matching_brace(&chars, opened) {
                        Some(close) => {
                            let contents: String = chars[opened + 1..close].iter().collect();
                            match brace_alternatives(&contents) {
                                // A LITERAL pair: no comma, no range. Bash passes
                                // it through unchanged, so the `{`, its contents
                                // and the `}` are absorbed as ordinary word
                                // characters and the word survives whole.
                                // Measured: `printf "[%s]" repos/{owner}/{repo}/pulls`
                                // prints `[repos/{owner}/{repo}/pulls]`, which is
                                // why `gh`'s documented `{owner}`/`{repo}`
                                // placeholders reach `endpoint_is_pulls` intact
                                // and the creation is COUNTED (`T-19-93`).
                                BracePair::Literal => {
                                    begin_word!();
                                    word_all_bare_digits = false;
                                    text.push('{');
                                    text.push_str(&contents);
                                    text.push('}');
                                    index = close + 1;
                                    continue;
                                }
                                // A brace EXPANSION, or a pair whose
                                // alternatives cannot be enumerated: today's
                                // segmentation, unchanged, PLUS the mark.
                                BracePair::Expansion(_) | BracePair::Unenumerable => {
                                    brace_splice = true;
                                }
                            }
                        }
                        // No matching `}` at all: unclassifiable, so today's
                        // segmentation PLUS the mark, fail-closed. Bash would
                        // treat this `{` literally; the guard refuses what it
                        // cannot establish instead. The cost is nil for an
                        // ungoverned command, because the mark only refuses when
                        // a governed program is reached or produced.
                        None => brace_splice = true,
                    }

                    if brace_splice {
                        // **The product scan is a WHOLE-WORD scan and it runs
                        // ONCE, at the word's FIRST expansion `{`.** Its extent
                        // begins at the start of the word in progress — not at
                        // this `{` — and reaches to the next unquoted whitespace
                        // or REAL command operator, never stopping at a `{` or a
                        // `}`. Later `{`s in the same word are already covered by
                        // it, so nothing is computed twice.
                        if !splice_word {
                            let start = if started { word_start } else { opened };
                            splice_can_produce_governed =
                                brace_word_products(&chars, start).can_produce_governed();
                        }
                        splice_word = true;
                        literal = false;
                    }
                }

                let severed_a_word = started;
                flush!();
                tokens.push(Token {
                    text: "{".to_string(),
                    operator: true,
                    expansion: false,
                    word_splitting_flush: severed_a_word,
                    literal: true,
                    brace_splice,
                    splice_can_produce_governed,
                    redirection_unresolvable: false,
                });
            }
            // **`&>` and `&>>` are recognised HERE, before `&` reaches the
            // separator arm below and before its `&&` two-character
            // consumption.** `&` is in `SEPARATORS`, so without this arm the
            // guard splits ONE simple command into TWO — `git`, then
            // `>/tmp/o push --force origin main` — and a redirection parser
            // running after the separator arm never sees it. Measured:
            // `git &>/tmp/o push --force origin main` is one simple command to
            // bash, `ARGV[git]: [push] [--force] [origin] [main]`. Match arms
            // are tried in order, and that order is the whole of this fix.
            '&' if chars.get(index) == Some(&'>') => {
                consume_redirection!(index - 1);
            }
            // An unquoted `<` or `>` is a word-terminating METACHARACTER, which
            // is what bash's lexer already makes it — neither a separator nor an
            // ordinary word. `SEPARATORS` is unchanged and `is_separator(">")`
            // is still false: a redirection does not start a new command.
            //
            // **Only OUTSIDE quotes.** A quoted or backslash-escaped `>` never
            // reaches this arm — the quote loops and the backslash arm consume
            // it — which is why `git commit -m ">"`, `git log --grep='>'`,
            // `rg ">" src/` and `--push-option="a>b"` stay permitted.
            '<' | '>' => {
                consume_redirection!(index - 1);
            }
            '\n' | ';' | '|' | '&' | '(' | ')' | '}' => {
                // **Read BEFORE the flush, because `flush!` clears `started`.**
                // A word in progress at the instant one of these four characters
                // arrives means the character split a word rather than ended a
                // command. See `Token::word_splitting_flush` for the cases this
                // tells apart and for why the other four separators are never
                // marked.
                let severed_a_word = started && matches!(ch, '(' | ')' | '}');
                flush!();
                // `&&` and `||` are one operator, not two. Which one it is does
                // not matter to a classifier that treats every separator alike,
                // but consuming both characters keeps the token list honest.
                let mut op = ch.to_string();
                if (ch == '&' || ch == '|') && chars.get(index) == Some(&ch) {
                    index += 1;
                    op.push(ch);
                }
                // A REAL command operator ends the shell word as well as the
                // simple command; `(`, `)` and `}` cut a word into fragments that
                // still belong to it.
                if matches!(ch, '\n' | ';' | '|' | '&') {
                    splice_word = false;
                }
                tokens.push(Token {
                    text: op,
                    operator: true,
                    expansion: false,
                    word_splitting_flush: severed_a_word,
                    literal: true,
                    brace_splice: false,
                    splice_can_produce_governed: false,
                    redirection_unresolvable: false,
                });
            }
            '\'' => {
                begin_word!();
                // A quoted digit run is NOT an IO_NUMBER; see the flag's doc.
                word_all_bare_digits = false;
                // Single quotes are literal all the way through, including `$`.
                loop {
                    match chars.get(index) {
                        Some('\'') => {
                            index += 1;
                            break;
                        }
                        Some(inner) => {
                            text.push(*inner);
                            index += 1;
                        }
                        // An unterminated quote has no knowable word boundary.
                        None => return None,
                    }
                }
            }
            '"' => {
                begin_word!();
                // A quoted digit run is NOT an IO_NUMBER; see the flag's doc.
                word_all_bare_digits = false;
                loop {
                    match chars.get(index) {
                        Some('"') => {
                            index += 1;
                            break;
                        }
                        Some('\\') => match chars.get(index + 1) {
                            // **Bash performs the LINE CONTINUATION inside
                            // double quotes too**, and this branch is reached
                            // instead of the top-level one, which is why it is a
                            // spelling of its own rather than the same row.
                            // Measured: `git "pu\<NL>sh" --force origin main`
                            // gives `ARGV[git]: [push] [--force] [origin]
                            // [main]`. Both characters are deleted, producing no
                            // character; `literal` stays true because a deletion
                            // is not a rewrite.
                            Some('\n') => {
                                index += 2;
                            }
                            // Only these four are escapes inside double quotes;
                            // every other backslash is a literal backslash, and
                            // a splitter that dropped it would change the word.
                            Some(esc @ ('"' | '\\' | '$' | '`')) => {
                                text.push(*esc);
                                index += 2;
                            }
                            Some(other) => {
                                text.push('\\');
                                text.push(*other);
                                index += 2;
                            }
                            None => return None,
                        },
                        Some(inner) => {
                            // Expansion still happens inside double quotes — and
                            // ONLY expansion does. A glob, a tilde or a brace
                            // inside double quotes is passed through byte for
                            // byte, which is why `gh api "repos/{owner}/{repo}/pulls"`
                            // is literal and counted today.
                            if *inner == '$' || *inner == '`' {
                                expansion = true;
                                literal = false;
                            }
                            text.push(*inner);
                            index += 1;
                        }
                        None => return None,
                    }
                }
            }
            '\\' => {
                // A trailing backslash is a line continuation whose second half
                // this function was never given, so this refuses the whole input.
                let escaped = *chars.get(index)?;
                if escaped == '\n' {
                    // **A LINE CONTINUATION, not an escape.** The reasoning
                    // below — "escaping is exactly what makes a character
                    // literal" — is true of every character it names and wrong
                    // only for the one it does not: a backslash DELETES a
                    // newline rather than protecting it, so this produces NO
                    // character. **It must not START a word** (hence no
                    // `begin_word!`): `git \<NL>push --force origin main` gives
                    // bash two words, not three, and a continuation that started
                    // one would flush an empty word into the REMOTE slot — the
                    // displacement `T-19-98` is about. **`literal` stays TRUE**:
                    // a deletion is not a rewrite. The SINGLE-quote loop is
                    // untouched, measured rather than assumed —
                    // `git 'pu\<NL>sh'` gives `[pu\<NL>sh]`, bytes verified with
                    // `od -c` — and `\`+CR and `\`+TAB stay ordinary escapes.
                    index += 1;
                } else {
                    begin_word!();
                    // An ESCAPED digit is not a bare one: `git \2>/tmp/o push`
                    // is the word `2` to bash, not an fd.
                    word_all_bare_digits = false;
                    index += 1;
                    // Escaping is exactly what makes a character literal, so the
                    // bit is deliberately not cleared here.
                    text.push(escaped);
                }
            }
            '#' if !started => {
                // A comment runs to end of line; nothing after it is a command.
                while index < chars.len() && chars[index] != '\n' {
                    index += 1;
                }
            }
            _ => {
                begin_word!();
                word_all_bare_digits &= ch.is_ascii_digit();
                if ch == '$' || ch == '`' {
                    expansion = true;
                }
                if REWRITING_CHARACTERS.contains(&ch) {
                    literal = false;
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
            word_splitting_flush: false,
            literal: literal && !splice_word,
            brace_splice: false,
            splice_can_produce_governed: false,
            redirection_unresolvable: false,
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

// ---------------------------------------------------------------------------
// Which token is the effective program (T-19-60)
// ---------------------------------------------------------------------------

/// The programs this envelope has a classifier for.
///
/// **This is the ONE enumeration the wrapper fix contains, and it is not the
/// same kind of list as a list of wrapper names.** The distinction is what the
/// whole design turns on, so it is recorded here rather than left to be
/// inferred:
///
/// - This set is **closed and already defined elsewhere in this module** —
///   [`classify_git`] handles `git`, [`pr_command_label`] handles `gh` and
///   `glab`, and a fourth entry here with no classifier is a *refusal*, not a
///   silent permit (see [`ProgramResolution::Governed`]). Adding a governed
///   program is a deliberate act with a compiler-adjacent consequence.
/// - The set of things that can *precede* a program is **open and unlistable**.
///   `env`, `timeout`, `nohup`, `command`, `nice`, `stdbuf`, `setsid`, `ionice`,
///   `chrt`, `taskset`, `doas`, `runuser`, `sudo`, `xargs`, `time`,
///   `busybox env`, `/usr/bin/env` — and the seventh one nobody listed. A fix
///   built on naming them is green on the day it lands and silent afterwards,
///   which is why [`resolve_program`] never asks what the wrapper is called
///   (D-08).
pub const GOVERNED_PROGRAMS: &[&str] = &["git", "gh", "glab"];

/// The environment keys the envelope itself injects into a driven child.
///
/// A word that assigns to one of these, or that bare-names one, is refused
/// under [`ParkReason::HookBypassBlocked`] — see [`tampers_with_envelope_env`]
/// for why the bare name counts too.
///
/// **An entry ending in `_` is matched as a prefix**, because its suffix is an
/// index git generates (`GIT_CONFIG_KEY_0`, `GIT_CONFIG_VALUE_0`, …); every
/// other entry is matched exactly.
///
/// This list is **drift-pinned** against the environment the DRIVER hands the
/// child — a unit test iterates the entries of
/// `build_env_in(...).with_run_id(...)` and asserts that **every** entry it
/// carries is covered here, SET or REMOVED, whatever the key is called. The pin
/// used to filter on `value.is_some()` and on a `GIT_`/`GH_` name prefix; both
/// filters are gone, because between them they hid the two `SSH_*` removals and
/// the run-journal locator (`T-19-82`). That is the discipline
/// [`forbidden_repo_prefixes`] already
/// uses by deriving its runs path from `journal::RUNS_SUBDIR`: a second
/// spelling of a fact is a second thing to keep in step, and here the drift
/// would be a refusal that silently stopped covering the key it was written
/// for.
///
/// **The pin's SOURCE is the whole child environment rather than one
/// constructor call, and that is `T-19-90`.** `super::cred::build_env_in`
/// returns an `EnvelopeEnv` to which the driver then appends
/// [`super::cred::RUN_ID_ENV`] through `with_run_id`, at the one seam that hands
/// the environment to the spawn closure. A pin sourced from `build_env_in`
/// alone is one seam short of the child and cannot see the entry that seam adds,
/// whatever names are in this constant — which is exactly how `GSD_MM_RUN_ID`
/// stayed uncovered while being carried to every driven child.
pub const ENVELOPE_ENV_KEYS: &[&str] = &[
    "GIT_CONFIG_COUNT",
    "GIT_CONFIG_KEY_",
    "GIT_CONFIG_VALUE_",
    "GIT_CONFIG_GLOBAL",
    "GIT_CONFIG_SYSTEM",
    "GIT_ASKPASS",
    "GIT_TERMINAL_PROMPT",
    // Found by the drift pin below rather than by inspection, and it belongs
    // here for the same reason the rest do: `GIT_SSH_COMMAND` is what carries
    // `IdentitiesOnly=yes`, `IdentityAgent=none` and `-F /dev/null`, so a
    // command that reassigns it puts the user's own agent and default identity
    // back within reach of the run (D-16).
    "GIT_SSH_COMMAND",
    "GH_CONFIG_DIR",
    // `T-19-82`. The two entries `cred::build_env_in` REMOVES rather than sets,
    // and the belt D-16 relies on: an ambient agent socket is the shortest path
    // from a driven run to the user's own keys, so a command that puts either
    // back — `SSH_AUTH_SOCK=/tmp/evil git fetch origin` — restores exactly what
    // the removal exists to take away. They were absent because the drift pin
    // filtered on `value.is_some()` and could not see a removal at all.
    "SSH_AUTH_SOCK",
    "SSH_AGENT_PID",
    // The run-journal locator, which the old pin's `GIT_`/`GH_` name filter
    // excluded. A child that unsets it loses its park evidence — D-24 requires
    // every envelope refusal to park and D-25 requires the park to land where a
    // later reader can find it — without gaining any ability to push. So this
    // entry protects the EVIDENCE rather than the containment, and it is the
    // reason the pin's name filter had to go rather than be widened.
    //
    // The false-positive cost of all three is the one
    // `tampers_with_envelope_env` already states for every key in this list: a
    // driven run cannot name them as a bare unquoted word, and must quote it.
    // That is the direction to be wrong in.
    "GSD_MM_ENVELOPE_PROJECT_ROOT",
    // `T-19-90`. The run identifier the DRIVER appends through
    // `cred::EnvelopeEnv::with_run_id`, which is why the drift pin could not
    // see it until the pin's source was moved to that seam.
    //
    // What it carries and which harm its removal causes:
    // `super::hooks::current_run_id` falls back to the `"unattributed-run"`
    // placeholder bucket when this key is absent, so every park the run
    // produces is attributed to a run nobody can find. D-24 requires every
    // envelope refusal to park and D-25 requires the park to land where a later
    // reader can find it — so this entry protects the EVIDENCE rather than the
    // containment, exactly as recorded above for its sibling locator
    // `GSD_MM_ENVELOPE_PROJECT_ROOT`. That reasoning had simply not yet reached
    // this key. Its false-positive cost is the same as every other entry's: a
    // driven run cannot name it as a bare unquoted word, and must quote it.
    "GSD_MM_RUN_ID",
];

/// The [`ENVELOPE_ENV_KEYS`] entry covering `name`, if any.
fn envelope_env_key(name: &str) -> Option<&'static str> {
    ENVELOPE_ENV_KEYS
        .iter()
        .copied()
        .find(|entry| name == *entry || (entry.ends_with('_') && name.starts_with(*entry)))
}

/// Whether a word is a shell **assignment word**, by the shell's own grammar.
///
/// A `=` exists, the half before it is non-empty, its first character is an
/// ASCII letter or `_`, and every remaining character of it is alphanumeric or
/// `_`.
///
/// **The grammar rather than a `contains('=')` test**, and the difference is
/// load-bearing: `--opt=value`, `a/b=c` and `x.y=z` all contain `=` and none of
/// them is an assignment. Treating one as an assignment would make
/// [`resolve_program`] skip past it looking for a program, which is how a
/// resolver ends up consuming the very token it was trying to find.
pub fn is_assignment_word(word: &str) -> bool {
    let Some((name, _)) = word.split_once('=') else {
        return false;
    };
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

/// The envelope key a word assigns to or names, if it is reaching for one.
///
/// Two shapes, because there are two ways to make an injected key stop
/// applying:
///
/// - `GIT_CONFIG_COUNT=0 git push --force` **overwrites** it. `19-SECURITY.md`
///   records that this single line neutralises layer 3 — no injected
///   `core.hooksPath`, so no `pre-push` hook — with the *same* token that made
///   layer 2 fail to recognise the command. That is why the threat is high
///   rather than medium, and why the assignment is refused **on its own
///   account** rather than merely parsed past (D-09).
/// - `env -u GIT_CONFIG_COUNT git push --force` and `unset GIT_CONFIG_COUNT`
///   **remove** it, and neither spells `=`. So a bare word that *is* one of the
///   keys counts too.
///
/// **The false-positive cost, stated rather than discovered:** a driven run
/// cannot mention one of these key names as a bare unquoted word — `grep
/// GIT_ASKPASS .` is refused. That is the direction to be wrong in, for the
/// same reason [`validate_namespace`] degrades to the tighter default: the
/// refusal is legible and the run can quote the word, while the other direction
/// is a disarmed enforcement layer nobody notices.
pub fn tampers_with_envelope_env(word: &str) -> Option<&'static str> {
    if is_assignment_word(word) {
        let key = word.split_once('=').map(|(key, _)| key).unwrap_or(word);
        return envelope_env_key(key);
    }
    envelope_env_key(word)
}

/// Whether any whitespace-delimited word of `text` names a governed program.
///
/// Used on a payload the guard could not split into words — an unbalanced quote
/// inside a `-c` string, say — so that such a payload is refused only when it
/// actually mentions something this envelope governs. Refusing every
/// unsplittable payload would refuse `grep -c "don't" file` for nothing.
pub fn mentions_governed_program(text: &str) -> bool {
    text.split_whitespace()
        .any(|word| GOVERNED_PROGRAMS.contains(&program_name(word)))
}

/// What [`resolve_program`] concluded about one simple command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProgramResolution {
    /// The segment runs no program at all — a bare `FOO=bar`.
    NoProgram,
    /// A program this envelope governs, and **the index its own argv begins
    /// at**. The index rather than a boolean, so a classifier is applied to the
    /// same argv it would have seen unwrapped.
    Governed {
        /// Index into the segment of the governed program's own token.
        index: usize,
    },
    /// The token at `index` is a command line handed to some other program —
    /// a `-c` payload, or a quoted string whose first word names a governed
    /// program. The caller re-splits and re-classifies it.
    NestedPayload {
        /// Index into the segment of the word carrying the nested command.
        index: usize,
    },
    /// The segment is refused before any classifier sees it.
    ///
    /// The [`ParkReason`] travels with the message rather than being inferred
    /// from it, for the reason `super::hooks::classify_segments` already
    /// records about its own refusal pair: a caller that read the reason out of
    /// the message would be deriving the same fact a second way, and that is
    /// how a journal comes to disagree with the refusal it records.
    Refuse {
        /// The member of D-24's taxonomy this refusal parks under.
        reason: ParkReason,
        /// One line naming what caused the refusal. Never the whole command:
        /// a detail that quotes the command back can carry a secret into the
        /// journal (SAFE-04), the same rule [`GitVerdict::Refuse`] follows.
        detail: String,
    },
    /// The segment provably reaches no governed program in any command
    /// position.
    ///
    /// **This is a PERMIT, and it is the answer rather than a fall-through.**
    /// The `PreToolUse` guard is registered against *every* Bash tool call, so
    /// a resolution that denied what it did not recognise would deny `ls`,
    /// `cargo test` and `rg`, and a control that fails into unusability is a
    /// control that gets switched off. What fails closed is the other
    /// direction: once resolution reaches a governed program, the caller must
    /// classify it or refuse it, never permit it silently (D-06, D-24).
    Ungoverned,
}

/// Resolve which token of a simple command is the **effective program**.
///
/// The gap this closes (`T-19-60`) is that the guard used to decide what a
/// command was by looking at `words[0]`. Five measured lines walked through it:
/// `env`, a `NAME=VALUE` prefix, `timeout`, `command`, and `env gh pr create`,
/// the last of which bypassed the SAFE-06 cap with no ledger line and therefore
/// no park.
///
/// **The class, not the instances.** The obvious fix is a list of wrapper names
/// plus a test row each; it is green on the day it lands and silent on
/// `stdbuf`, `setsid`, `ionice`, `doas`, `busybox env` and the seventh one
/// nobody listed. So this function never asks what the wrapper is *called*. It
/// consumes leading assignment words by the shell's own grammar and then finds
/// the first token whose **basename** names a program in
/// [`GOVERNED_PROGRAMS`]. There is nothing for a new wrapper to be missing
/// from.
///
/// **The order below is part of the contract**, because each step exists to be
/// reached only when the one above it did not answer:
///
/// 1. **Any token reaching for an envelope environment key** →
///    `Refuse(HookBypassBlocked)`. The assignment is refused for what it does
///    to layer 3, before anything is parsed past.
/// 2. **Skip leading assignment words.** Nothing left → `NoProgram`; a bare
///    `FOO=bar` executes no program and refusing it would be refusing an
///    assignment. But first: **a leading assignment whose VALUE names an
///    envelope key** → `Refuse(HookBypassBlocked)`, checked *before* the
///    `NoProgram` return because `K=GIT_SSH_COMMAND` runs no program and step 7
///    would never be reached for it (`T-19-81`).
/// 3. **The head carries an expansion** → `Refuse(EnvelopeAssertionFailed)`.
/// 4. **The head is `eval`** → `Refuse(EnvelopeAssertionFailed)`.
/// 5. **COMMAND POSITION.** Candidates are the non-flag, non-expansion tokens
///    that either name a governed program (`Governed`) or are a quoted string
///    whose first word does (`NestedPayload`). Finding one is not the same as
///    having placed it:
///    - a candidate **at the head** — the first word after the assignment
///      prefix — IS the command position and answers immediately, because
///      nothing precedes it but assignments and no wrapper grammar is in play.
///      This shortcut is what keeps `git commit -m "git push --force is now
///      blocked"` and `gh pr create --title "stop git push --force"` working;
///    - otherwise the candidate sits behind a wrapper prefix. **Two or more
///      candidates** → `Refuse(EnvelopeAssertionFailed)`: which one is the
///      command and which is an option's operand depends on that wrapper's flag
///      grammar, which this resolver deliberately does not know, so it is
///      refused rather than mis-indexed;
///    - **an expansion-carrying word between the head and the candidate** →
///      `Refuse(EnvelopeAssertionFailed)`, because what that prefix does to the
///      environment is decided after the guard has answered;
///    - exactly one candidate behind a knowable prefix resolves as before.
/// 6. **Otherwise the first bundled short option containing `c`** hands its
///    following word over as a nested command line. `-c` is the shell's own
///    spelling for "here is a command line", so `sh`, `bash`, `busybox sh`,
///    `script` and anything else that consumes one are covered without being
///    named — and `bash -lc "…"` is covered where an exact `-c` match was not.
/// 7. **Otherwise a leading assignment whose VALUE names a governed program**
///    → `Refuse(EnvelopeAssertionFailed)`. Checked *here*, after resolution has
///    otherwise failed, so it can never fire on `git commit -m x=git`, whose
///    segment already answered at step 5.
/// 8. `Ungoverned`.
///
/// ## The three shapes this does NOT cover, named rather than left to be found
///
/// * **`T-19-74` — an expansion-assembled program behind a wrapper.**
///   `env $X push --force`, where `$X` was bound outside this command line,
///   resolves to `Ungoverned` and is permitted. Closing it would require
///   refusing every `$VAR` in an ungoverned command, which also refuses
///   `echo $(git rev-parse HEAD)` and `cd "$HOME"` — a control that fails into
///   unusability gets switched off. It is narrowed on two sides: step 1 refuses
///   the envelope-key assignments that would pair with it, and step 7 refuses
///   binding a governed program name to a variable *in the same command line*.
///   Each Bash tool call being its own shell process is what keeps it narrow.
/// * **`T-19-75` — over-refusal from step 5(b).** An argument that literally
///   spells a refused git command is refused: `rg "git push --force" src/` does
///   not run. The rule **discriminates** rather than blanket-denying, because
///   the payload is classified — `rg "git status" src/` is permitted — and the
///   cost is confined to driven runs and legible when it fires.
///
///   The command-position rule extends this accepted class to governed heads: a
///   WRAPPED command that also quotes a string beginning with a governed
///   program name — `env gh pr create --title "git push --force"` — is now a
///   two-candidate segment and is refused, while the unwrapped spelling
///   answers at the head and runs. That is the same cost in a new spelling, not
///   a new kind of cost, and it is pinned beside the spelling that works.
/// * **`T-19-103`'s over-refusal — a config key git IGNORES.**
///   `git -c include.pathx=/tmp/evil.cfg status` is exit 0 against real git (the
///   envelope's injection still wins, so git ignores the key) and is REFUSED
///   here, because [`config_key_names_an_indirection_section`] reads the SECTION
///   and deliberately not the variable. That is the whole measured cost of this
///   round's rule and it falls in the safe direction; the correct response to
///   it is not to start reading the variable, which would re-open `includeIf`'s
///   condition family and any future variable in either section. The permitted
///   half is pinned beside it: `git -c user.name="$NAME" commit -m x`,
///   `git -c core.pager=cat log`, `git -c a=b status`,
///   `git -c includepath=…` and `git -c notinclude.path=…` all still run.
/// * **`T-19-86` — a GOVERNED program's own operand naming a governed
///   command.** `git submodule foreach git push --force origin main`,
///   `git rebase -x "git push --force origin main" HEAD~3`,
///   `git bisect run sh -c "git push --force origin main"` and
///   `git -c alias.p='!git push --force origin main' p` all resolve at the head
///   — **correctly**, because the head is the command position — and are then
///   permitted by [`classify_git`]'s denylist default arm, whose verbs here are
///   `submodule`, `rebase`, `bisect` and `p`. They were permitted before the
///   command-position rule and they are permitted after it.
///
///   So step 5 closes the **wrapper-operand** sub-class of `T-19-60` — a token
///   that is not the effective program capturing the index because it is
///   spelled `git`/`gh`/`glab` in a WRAPPER's operand slot — and it does not
///   close this one. The four spellings are pinned at their current permitted
///   verdict in `tests/envelope_command_position.rs`, registered in
///   `19-SECURITY.md` and `deferred-items.md`, and left for a later round: they
///   were found while planning the round that closed the sub-class, and a plan
///   cannot both discover a threat and be the plan that measured it fail first.
/// * **`T-19-91` — a git classifier's own DECISION OPERAND, assembled by
///   expansion, for every verb but `config`.** `git reflog $S`,
///   `git reflog show $S` and `git symbolic-ref $S` are measured PERMITTED:
///   `classify_reflog` matches its first non-flag token against `delete`,
///   `expire` and `drop`, and `classify_symbolic_ref` counts operands and looks
///   for `-d` — so an operand neither can read falls to an `Allow` arm.
///   `git config`'s key operand is the one cell of this shape that IS closed,
///   because `git ${X}config core.hooksPath /tmp/x` is a row in audit 3's own
///   measured bypass list and a region principle that stopped at the verb while
///   `config`'s key stayed unreadable would not be coherent. The rest was found
///   while checking plan 19-14, and a plan cannot both discover a threat and be
///   the plan that measured it fail first.
///
///   **This is NOT a second-carrier argument.** `reflog` and `symbolic-ref`
///   have no `pre-push` and no `pre-commit` behind them — git runs no hook for
///   either — and [`classify_reflog`]'s own refusal text records that the
///   reflog is the recovery path for every other destructive git operation.
///   Only `git push` has a hook behind it. **That asymmetry is correct and it is
///   the whole of the argument; what used to be written beside it was not.**
///
///   This bullet used to add that `push`'s "refspec operand already fails
///   CLOSED". It does not, and the correction is recorded here rather than
///   quietly dropped. The narrow claim below — that `git push origin $REF` is
///   refused, an unreadable refspec not carrying the namespace prefix — is
///   RIGHT and audit 4 re-measured it at exit 2. The BARE `git push $REF` is a
///   different shape: [`push_needs_resolved_dests`] answers true for it, so the
///   verdict is resolved from a repository and is **cwd-dependent — measured at
///   exit 0 in a repository whose current branch is inside the envelope's
///   namespace, which is the state a driven run is DESIGNED to be in**, and at
///   exit 2 (`push_outside_namespace`) elsewhere, for a reason that has nothing
///   to do with the operand. Audit 4 measured it and `19-16` reproduced it in a
///   purpose-built fixture with the repository passed to `guard_in` explicitly.
///
///   So [`classify_push`]'s refspec operand is a **THIRD arm of `T-19-91`'s
///   shape**, beside [`classify_reflog`]'s and [`classify_symbolic_ref`]'s,
///   rather than the one arm that was already closed. `T-19-91` stays **OPEN at
///   `high`**: this is a correction to the RECORD and not to the remedy, no
///   decision-operand rule was added for `reflog`, `symbolic-ref` or `push`, and
///   the residual is pinned in `tests/envelope_expansion_slots.rs` and
///   `tests/envelope_literal_decision.rs` and registered in `19-SECURITY.md` and
///   `deferred-items.md`.
/// * **`T-19-17r` — over-refusal from the literalness rule, disclosed here
///   beside the residuals rather than left to be found.** Since `19-17` a brace
///   expansion anywhere in a governed simple command is refused
///   (`git commit -m {a,b}`, `git add {src,tests}/x.rs`,
///   `rg "git status" {src,tests}`), a splice whose PRODUCTS name a governed
///   program is refused even in an ungoverned command (`echo {git,x}`), and a
///   glob or tilde in a DECISION word is refused
///   (`git config --get-regexp branch.*` unquoted). The `-c` key-half class is
///   widened textually, so `git -c 'user.na*e=x' commit` is refused even though
///   the quoting made the character literal — disclosed, and essentially zero,
///   because no legal git config key can carry one of these characters. Every
///   one is pinned in `tests/envelope_literal_decision.rs` beside its PERMITTED
///   twin and the clause that produces it, and `ls {git,svn}-repo` is pinned
///   permitted as the control that keeps clause 2(b) a PRODUCT test rather than
///   a mention test.
/// * **`T-19-96` — a glob in a PUSH FLAG, one slot outside the decision
///   region.** `git push --forc? origin refs/heads/gsd-auto/alpha/w` is measured
///   at exit 0 and its literal twin at exit 2 under `force_push_blocked`.
///   Registered by `19-16`, pinned at its measured verdict, and NOT fixed: the
///   decision region is not widened here, and widening it to
///   [`classify_push`]'s flags is the same move as closing `T-19-91`.
/// * **`T-19-100r` — over-refusal from the leading-option grammar rule, and its
///   cost is ZERO on the installed git and BOUNDED on a future one.** Since this
///   round a leading `git` option whose grammar [`leading_git_option`] cannot
///   establish makes the VERB SLOT unestablished, and [`scan_leading`] refuses on
///   it rather than reading the option's value as the verb.
///
///   **Zero on git 2.43.0**: every option this git accepts is classified by the
///   two-sided probe and enumerated in the two constants, so the only commands
///   moving permitted → refused are ones git ITSELF rejects —
///   `git --bogus-opt status`, `git --super-prefix x status`,
///   `git -pc user.name=x status` — refusals of commands that already do nothing.
///
///   **One refusal per newly added global option on a FUTURE git**, until the
///   constant learns it. `--no-advice` and `--no-lazy-fetch` are the measured
///   stand-ins — real global options in later releases, rejected by this one — and
///   both are in the corpus, so the future cost is checkable rather than argued.
///
///   **The recovery path, in the order the refusal message offers it**: spell the
///   option with its value attached (`--option=value`), which needs no constant
///   change because git's own grammar makes an attached value self-contained —
///   **offered first and never alone, because an attached spelling is always
///   self-contained but is NOT always accepted**
///   (`git --shallow-file=/tmp/s version` answers `unknown option:` on this git,
///   while `--attr-source=`, `--git-dir=`, `--namespace=` and `--work-tree=` all
///   reach the verb); or drop the option; or add the spelling to the constant,
///   which the drift pin will name. Twelve ordinary invocations —
///   `git --no-pager status`, `git -c user.name="$NAME" commit -m x`,
///   `git --git-dir=/tmp/g status`, `git -C /tmp status` among them — are pinned
///   at exit 0 in `tests/envelope_callee_grammar.rs` beside the refused rows,
///   because `git --no-pager status` is this axis's `ls {git,svn}-repo` and a rule
///   that refused it would be a blanket refusal of anything beginning with `-`.
/// * **`T-19-19r` — over-refusal from the redirection rule, and it is NET
///   NEGATIVE.** Two shapes are newly refused, each pinned in
///   `tests/envelope_argv_deletion.rs` beside its PERMITTED twin so a later
///   reader can tell a cost that was accepted from one that grew: an
///   UNRESOLVABLE redirection in a governed simple command (`git >` refused,
///   twin `ls >` permitted — and bash does not run `git >` either, answering
///   `syntax error near unexpected token 'newline'`), and a `{name}`
///   FD-ALLOCATION prefix, which is deliberately not modelled
///   (`git {v}>/tmp/o push --force origin main` and its permitted-half twin
///   `git {v}>/tmp/o status` both refused, twin `git >/dev/null push …`
///   modelled and classified). Against that, the rule REMOVES two measured
///   FALSE REFUSALS that existed before it:
///   `git push origin refs/heads/gsd-auto/alpha/w > log.txt` and its
///   `\`+newline spelling, both at exit 2 `push_outside_namespace` before and
///   exit 0 after, matching their one-line twins. Ordinary redirection keeps
///   working — `git log > out`, `git status > /tmp/s.txt`,
///   `git commit -m "x" >> build.log`, `gh pr list 2>/dev/null`,
///   `git fetch origin 2>&1 | tee log` — and `gh pr create --title x > /tmp/o`
///   stays COUNTED with a ledger line, which is the measured cost that rejected
///   the blanket-refusal design.
///
/// ## What IS covered in the program's own arguments, since 19-14
///
/// The exemption above is no longer total, and the boundary is exact.
/// [`first_unreadable_decision_word`] refuses a word each matched classifier arm
/// READS that is not provably LITERAL — the git verb, `config`'s key operand, a
/// forge's first two subcommand words, and the `gh api` endpoint, method and
/// flag-ness words — before either classifier runs and before the ledger write.
/// Everything to the right of those is an OPERAND and stays free, which is what
/// keeps `git commit -m "$MSG"`, `gh pr create --title "$TITLE"`,
/// `gh api repos/o/r/pulls -f title="$T"` and `git -c user.name="$NAME" commit`
/// working (`T-19-88`, `T-19-87` in part).
///
/// ## What IS covered about the whole simple command, since 19-17
///
/// [`resolve_program_with_head`] refuses a command a brace expansion splices
/// into, on both halves of clause 2 — a segment resolving `Governed` or
/// `NestedPayload`, and a word the splice can PRODUCE whose basename is
/// governed or whose products cannot be enumerated. That is the one control in
/// this module that is not about a word, because a brace expansion is not a
/// word-level fact.
///
/// ## What IS covered about the segment's own HEAD, since 19-15
///
/// This function answers about a segment whose first token is at a **command
/// position**. When it is not — a fragment continuing an enclosing word after an
/// expansion — [`resolve_program_with_head`] refuses a governed resolution
/// instead. `resolve_program` is that function with the head treated as real, so
/// every caller holding a bare `&[Token]` is unchanged.
///
/// **That rule's disclosed cost, beside `T-19-74` and `T-19-75` rather than
/// below them.** A command that places a governed program IMMEDIATELY after a
/// substitution which carried a literal prefix is refused:
/// `ROOT=$(git rev-parse --show-toplevel) git status` does not run, while
/// `ROOT=$(git rev-parse --show-toplevel)` alone and `git status` alone both do.
/// The cost is exactly that juxtaposition and nothing wider, and it is pinned as
/// a PAIR in `tests/envelope_expansion_slots.rs` so a cost that grows is a cost
/// something goes red about.
pub fn resolve_program(segment: &[Token]) -> ProgramResolution {
    // 1. An envelope key is refused on its own account, wherever it appears.
    for token in segment {
        if let Some(key) = tampers_with_envelope_env(&token.text) {
            return ProgramResolution::Refuse {
                reason: ParkReason::HookBypassBlocked,
                detail: format!(
                    "this command sets or removes `{key}`, which is one of the environment \
                     keys the envelope injects; the same token that hides a command from \
                     the guard is the one that stops the git hooks from running (D-09)"
                ),
            };
        }
    }

    // 2. Leading assignment words are a prefix, not the program.
    let mut head = 0;
    while head < segment.len() && is_assignment_word(&segment[head].text) {
        head += 1;
    }

    // 2b. A leading assignment whose VALUE names an envelope key.
    //
    // **The placement is the whole point of this check, not an implementation
    // detail.** `19-SECURITY.md`'s measured `T-19-81` line binds the key in a
    // segment that runs no program at all — `K=GIT_SSH_COMMAND; …` — so a check
    // placed after resolution, where step 7's `GOVERNED_PROGRAMS` value check
    // sits, is never reached for it: the `NoProgram` return below fires first.
    //
    // **The asymmetry with step 7 is deliberate and is recorded rather than
    // tidied away.** This half costs one disclosed row —
    // `FOO=GIT_ASKPASS echo hi` is refused, pinned in
    // `tests/envelope_command_position.rs` beside the `echo FOO=GIT_ASKPASS`
    // that still runs — while step 7's half sits at the boundary of the
    // ACCEPTED `T-19-74` residual, which
    // `the_residual_begins_exactly_at_the_command_line_boundary` pins and this
    // rule does not move. Moving step 7 up here would refuse
    // `X=git; env $X push --force origin main`, which is accepted (AR-19-10).
    for token in &segment[..head] {
        let Some((_, value)) = token.text.split_once('=') else {
            continue;
        };
        if let Some(key) = envelope_env_key(value) {
            return ProgramResolution::Refuse {
                reason: ParkReason::HookBypassBlocked,
                detail: format!(
                    "this command binds the name of `{key}` — one of the environment keys \
                     the envelope injects — to a shell variable, which is how the key is \
                     removed a word at a time without ever being spelled where the guard \
                     can see it (D-09, D-16)"
                ),
            };
        }
    }

    let Some(head_token) = segment.get(head) else {
        return ProgramResolution::NoProgram;
    };

    // 3. A program the shell assembles at run time is not knowable here.
    if head_token.expansion {
        return ProgramResolution::Refuse {
            reason: ParkReason::EnvelopeAssertionFailed,
            detail: "this command's program is assembled by shell expansion, so what it \
                     will run is not knowable before it runs; refused rather than guessed \
                     at"
                .to_string(),
        };
    }

    // 4. `eval` builds its command at run time, so no classifier can see it.
    if program_name(&head_token.text) == "eval" {
        return ProgramResolution::Refuse {
            reason: ParkReason::EnvelopeAssertionFailed,
            detail: "`eval` builds a command at run time, so no classifier can see what it \
                     will run"
                .to_string(),
        };
    }

    // 5. COMMAND POSITION, not merely "the first token that looks governed".
    //
    // The skip rules and the two candidate kinds are unchanged from `19-11`;
    // what changed is that finding a candidate is no longer the same thing as
    // having placed it. See this function's doc for the rule and its cost.
    let mut first: Option<(usize, ProgramResolution)> = None;
    let mut candidates = 0usize;

    for (index, token) in segment.iter().enumerate().skip(head) {
        if token.expansion || token.text.starts_with('-') {
            continue;
        }
        let found = if GOVERNED_PROGRAMS.contains(&program_name(&token.text)) {
            ProgramResolution::Governed { index }
        } else if token.text.split_whitespace().count() > 1
            && token
                .text
                .split_whitespace()
                .next()
                .is_some_and(|word| GOVERNED_PROGRAMS.contains(&program_name(word)))
        {
            ProgramResolution::NestedPayload { index }
        } else {
            continue;
        };

        candidates += 1;
        if first.is_none() {
            // **The head shortcut.** The head IS the command position: nothing
            // precedes it but assignment words, so no wrapper's flag grammar is
            // in play and there is nothing to disambiguate. Answering here,
            // without looking for competitors, is what keeps
            // `git commit -m "git push --force is now blocked"` and
            // `gh pr create --title "stop git push --force"` working — both
            // measured permitted in `19-SECURITY.md`'s blast-radius table.
            // Without it, every commit message and PR title that quotes a git
            // command would be refused: `T-19-75` widened from `rg` to every
            // commit, which is a control that fails into unusability and
            // therefore gets switched off (AR-19-11).
            if index == head {
                return found;
            }
            first = Some((index, found));
        }
    }

    if let Some((index, found)) = first {
        // **Two or more candidates behind a wrapper prefix cannot be placed.**
        // Two governed names in one simple command with a wrapper between them
        // means the command position depends on that wrapper's flag grammar —
        // whether `-u` takes the next word, whether `--` ends the options —
        // which is precisely the knowledge this resolver refuses to encode,
        // because encoding it is a wrapper-name list wearing a flag's clothes.
        // So the segment is REFUSED rather than mis-indexed. Mis-indexing is
        // what made `env -u git git push --force origin main` a permit: the
        // decoy captured the index, the real command became `argv[0]` of the
        // classified argv, and `classify_git` read its verb as `git`.
        if candidates >= 2 {
            return ProgramResolution::Refuse {
                reason: ParkReason::EnvelopeAssertionFailed,
                detail: "this command names a program the envelope governs more than once \
                         behind a prefix, so which of them is the command and which is an \
                         option's operand cannot be established without knowing that \
                         prefix's own flag grammar; refused rather than guessed at"
                    .to_string(),
            };
        }

        // **An expansion anywhere in the WRAPPER PREFIX region.** What that
        // prefix does to the environment — and therefore which program runs and
        // with what — is decided after the guard has answered. Restricted to
        // this region deliberately: an expansion in the ASSIGNMENT prefix
        // (`FOO=$BAR git status`) and one in the program's own OPERANDS
        // (`git commit -m "$MSG"`) are untouched, and a segment reaching no
        // governed program at all is untouched — which is what leaves the
        // accepted `T-19-74` residual exactly where it is.
        //
        // **The words between this region and the operands are no longer
        // exempt, and that is `T-19-88`.** Audit 3 quoted the sentence above as
        // the disclosure that left the VERB SLOT open; the classifier's own
        // decision words are now checked by
        // [`first_unreadable_decision_word`], at the one call site in
        // `super::hooks::classify_segments`. Nothing about this rule changed —
        // it still governs only the prefix — but a reader meeting it should not
        // conclude that everything after the head is free.
        //
        // The head itself is not examined here because step 3 already refused
        // it. This is the half of `T-19-81` that closes the CLASS rather than
        // the spelling: `env -u ${K}_COMMAND git fetch origin` is refused
        // however the key name is assembled, without the guard ever learning
        // what `-u` means.
        if segment[head + 1..index].iter().any(|token| token.expansion) {
            return ProgramResolution::Refuse {
                reason: ParkReason::EnvelopeAssertionFailed,
                detail: "a word between this command's head and the program the envelope \
                         governs is assembled by shell expansion, so what that prefix does \
                         to the environment the program runs in is not knowable before it \
                         runs; refused rather than guessed at"
                    .to_string(),
            };
        }

        // Exactly one candidate behind a prefix whose words are all knowable:
        // resolved as `19-11` and `19-12` already pin it.
        return found;
    }

    // 6. `-c` is the shell's own spelling for "the next word is a command line".
    for (index, token) in segment.iter().enumerate().skip(head) {
        let word = token.text.as_str();
        if word == "-" || word.starts_with("--") || !word.starts_with('-') {
            continue;
        }
        if word[1..].contains('c') && segment.len() > index + 1 {
            return ProgramResolution::NestedPayload { index: index + 1 };
        }
    }

    // 7. A governed program bound to a name in this same command line.
    for token in &segment[..head] {
        let value = token.text.split_once('=').map(|(_, value)| value);
        if value.is_some_and(|value| GOVERNED_PROGRAMS.contains(&program_name(value))) {
            return ProgramResolution::Refuse {
                reason: ParkReason::EnvelopeAssertionFailed,
                detail: "this command binds the name of a program the envelope governs to a \
                         shell variable, so which program it reaches is decided after the \
                         guard has answered; refused rather than guessed at"
                    .to_string(),
            };
        }
    }

    // 8. No governed program is reachable. See `ProgramResolution::Ungoverned`
    //    for why this is the answer and not a gap.
    ProgramResolution::Ungoverned
}

/// [`resolve_program`], for a segment that may not begin at a command position.
///
/// **A SEVERED PREFIX IS NOT A COMMAND POSITION (`T-19-87`, Rule B).** When
/// `head_is_command_position` is false the segment is a fragment continuing an
/// enclosing word after an expansion — [`Segment::head_is_command_position`]
/// reports it from the flush flag on the operator immediately preceding the
/// segment — so what the shell will put in front of its first token is
/// unknowable. A governed program found there is REFUSED rather than classified.
///
/// This is step 5's existing wrapper-prefix rule applied where the prefix was
/// SEVERED rather than merely expanded. The measured line it closes:
///
/// ```text
/// C=GIT_CONFIG; env -u ${C}_COUNT git fetch origin
///   ->  `C=GIT_CONFIG`  |  `env -u $`  |  `C`  |  `_COUNT git fetch origin`
/// ```
///
/// In bash that is `env -u GIT_CONFIG_COUNT git fetch origin`: layer 3's
/// `core.hooksPath` carrier removed, with `GIT_ASKPASS`, `GIT_CONFIG_GLOBAL` and
/// `GIT_SSH_COMMAND` intact. The last fragment resolves `git` behind a clean
/// one-word literal prefix whose verb is the literal `fetch`, so no verb-slot
/// rule can see it — by the time resolution runs the shape is gone.
///
/// **The rule is POSITIONAL. It reads no name, no substring and no length.**
///
/// # The formulation that was WITHDRAWN, recorded with both measurements
///
/// An earlier draft keyed this on the literal fragment (`_COUNT`) being a
/// substring of an [`ENVELOPE_ENV_KEYS`] entry. It was withdrawn on two
/// measurements, and a later reader tempted by the textual version needs to find
/// them here rather than repeat them:
///
/// * **Evadable — move the split point.** `C=GIT_CONFIG_COU; env -u ${C}NT git
///   fetch origin` leaves the two-character fragment `NT`;
///   `C=GIT_CONFIG_COUN; env -u ${C}T …` leaves one character; and the bare
///   `env -u ${C} git fetch origin` leaves NO literal fragment at all, so there
///   is nothing to match. Any minimum length is a floor an author ducks under by
///   moving the cut one character to the left.
/// * **Unshippable — it refuses ordinary shell.** The commonest `$(`-carrying
///   shape in real use is an uppercase assignment, and `ROOT`, `DIR`, `RUN`,
///   `CONFIG` and `COMMAND` all sit inside envelope key names —
///   `GSD_MM_RUN_ID` included, which 19-14 added. `ROOT=$(git rev-parse
///   --show-toplevel)`, `DIR=$(mktemp -d)`, `RUN_ID=$(uuidgen)`,
///   `CONFIG=$(cat cfg)` and `COMMAND=$(which git)` would each be refused on
///   every Bash tool call.
///
/// The positional rule refuses all three evasion spellings and permits all five
/// assignments, because it decides about COMMAND POSITION. It has nothing to
/// floor and nothing to duck under.
///
/// **The other resolutions pass through unchanged, and the order matters.** An
/// envelope-key refusal (step 1) is more specific than this one and keeps its own
/// `HookBypassBlocked` identifier; `NoProgram` and `Ungoverned` are permits about
/// a segment that reaches nothing this envelope governs, and refusing those would
/// deny `_COUNT ls` for nothing.
///
/// ## Clause 2, folded in here rather than added beside it
///
/// A brace expansion is a property of the SIMPLE COMMAND, not of a word:
/// `git {push,--force} origin main` splits into `git`, `push,--force` and
/// `origin main`, the first resolves `Governed` with an EMPTY argv, and
/// [`classify_git`] answers `Allow` for a bare `git`. There is no decision word
/// to test. So a command a brace expansion splices into is unresolvable and is
/// refused when **(a)** any of its segments resolves `Governed` or
/// `NestedPayload`, **or (b)** any word the splice can PRODUCE has a governed
/// basename — or its products could not be enumerated at all.
///
/// **Both halves are load-bearing**, and each has a measured row the other does
/// not reach. (a) alone permits `{git,push,--force,origin,main}`, whose single
/// segment's basename is not a governed program. (b) alone permits
/// `git push {--force,origin} main`, where the governed word is outside the
/// braces and what the splice hides is `--force`.
///
/// **It is folded into THIS function rather than added beside it** because two
/// post-filters over one resolution are two things to keep in step, and the day
/// they drift is the day one permits what the other refuses.
///
/// ## The match is EXHAUSTIVE, one arm per variant
///
/// It used to be `Governed | NestedPayload => Refuse, other => other`. That
/// wildcard means a future variant meaning "this segment reaches a program the
/// envelope governs" would compile, pass a severed head silently, and turn no
/// test red. One arm per [`ProgramResolution`] variant makes a sixth an E0004 —
/// the discipline `T-19-45` already establishes for `PermissionMode`.
pub fn resolve_program_with_head(entry: &Segment) -> ProgramResolution {
    let segment = entry.tokens.as_slice();
    let resolved = resolve_program(segment);

    match resolved {
        // Rule B (a severed head) and clause 2(a) (a brace-spliced simple
        // command) are the same answer about the same fact: the argv that runs
        // is not the argv the classifier would read.
        ProgramResolution::Governed { .. } | ProgramResolution::NestedPayload { .. } => {
            if !entry.head_is_command_position {
                return ProgramResolution::Refuse {
                    reason: ParkReason::EnvelopeAssertionFailed,
                    // Names the SHAPE and never quotes the command back (SAFE-04).
                    detail: "this command reaches a program the envelope governs from a \
                             fragment that continues an enclosing word after a shell \
                             expansion, so what the shell will put in front of that program \
                             is decided after the guard has answered; the fragment's first \
                             word is not a command position, and it is refused rather than \
                             guessed at"
                        .to_string(),
                };
            }
            if entry.brace_spliced {
                return ProgramResolution::Refuse {
                    reason: ParkReason::EnvelopeAssertionFailed,
                    detail: "a brace expansion splices words back into this command after \
                             the guard has answered, so the argv a classifier would read is \
                             not the argv that runs; the command cannot be established as \
                             literal and is refused rather than guessed at"
                        .to_string(),
                };
            }
            // **The same answer, one axis over.** Rule B and clause 2(a) above
            // answer about words the guard READS that the shell REWRITES or
            // SPLICES; this answers about words it reads that never ARRIVE. A
            // redirection the parser could not resolve into a complete
            // (operator, target) pair means the shell deletes an unknown span
            // from argv, so which words reach the program — and in which slots —
            // is not knowable before it runs.
            if entry.redirection_unresolvable {
                return ProgramResolution::Refuse {
                    reason: ParkReason::EnvelopeAssertionFailed,
                    // Names the SHAPE and never quotes the command back (SAFE-04).
                    detail: "this command reaches a program the envelope governs through a \
                             redirection the guard cannot resolve into an operator and its \
                             target, so the shell removes words from its argv that the guard \
                             cannot identify; which words reach the program is not knowable \
                             before it runs, and it is refused rather than guessed at"
                        .to_string(),
                };
            }
            resolved
        }

        // **Clause 2(b), and this half is what reaches the rows nothing else
        // does.** `{git,push,--force,origin,main}`, `{env,git} push --force …`,
        // `{g..g}it push --force …`, `g{i,i}t push --force …`,
        // `{g..g}{i..i}t push --force …`, `{g,g}{i,i}{t,t} push --force …`,
        // `"g"{i,i}"t" push --force …`, `{g..g}"it" push --force …` and the
        // unenumerable `{g..g..1}it push --force …` and `{g{i,i}t,x} push --force
        // …` are ALL measured at exit 0 against the pre-`19-17` tree, and in
        // every one of them nothing resolves `Governed` at all — the head word is
        // `it`, `g`, `t` or a comma list naming nothing.
        //
        // **It reads PRODUCTS rather than names, and there is a control
        // attached**: `ls {git,svn}-repo` produces `git-repo` and `svn-repo`,
        // neither governed, and stays PERMITTED. A mention test would refuse it,
        // and a mention test is one slot away from the class.
        //
        // **There is deliberately NO clause-2(b) analogue for the redirection
        // mark, and the asymmetry has a reason rather than being an omission.**
        // A brace splice can PRODUCE a governed program out of words that name
        // nothing, which is why the half above exists. A redirection cannot: its
        // target is REMOVED from argv rather than spliced into it, so it can
        // only ever delete words from a command, never conjure a program into
        // one. So `ls >` stays PERMITTED on the same cost containment
        // `brace_spliced` already establishes — the mark refuses only when a
        // governed program is actually reached. A rule that denied what it did
        // not recognise would deny `ls`, `cargo test` and `rg`, which is how a
        // safety control gets switched off (AR-19-11).
        ProgramResolution::NoProgram | ProgramResolution::Ungoverned => {
            if entry.brace_spliced && entry.splice_can_produce_governed {
                return ProgramResolution::Refuse {
                    reason: ParkReason::EnvelopeAssertionFailed,
                    detail: "a brace expansion in this command can produce a word naming a \
                             program the envelope governs, or its alternatives cannot be \
                             enumerated at all, so what this command runs is not knowable \
                             before it runs; refused rather than guessed at"
                        .to_string(),
                };
            }
            resolved
        }

        // A refusal passes through WHOLE, keeping its own more specific
        // identifier: step 1's `HookBypassBlocked` must not be overwritten by
        // this one.
        ProgramResolution::Refuse { .. } => resolved,
    }
}

/// A word a classifier's matched arm READS, which the shell assembles at run
/// time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionWord {
    /// Its index in the segment.
    pub index: usize,
    /// The word as the tokenizer recovered it — `$V`, `$`, `` `true`pr ``.
    pub word: String,
    /// Which decision it is, for a refusal a reader can act on.
    pub role: &'static str,
}

/// Whether a `gh api` token is one whose FLAG-NESS the arm decides on.
///
/// Two clauses, and the code says why neither alone is enough.
/// [`gh_api_posts_a_pull_request`] sets `implies_post` by WHOLE-TOKEN membership
/// in [`GH_API_IMPLIES_POST`], so a token whose spelling is not knowable yields
/// `implies_post == false`, `pr_command_label` returns `None`, and a pull
/// request opens with no refusal, no ledger line and no cap charge.
///
/// * **marker-initial** — `F=-f; gh api repos/o/r/pulls $F title=x`. The whole
///   token is assembled, so whether it is a flag at all is unknowable.
/// * **`-`-initial with an expansion before the first `=`** —
///   `F=f; gh api repos/o/r/pulls -$F title=x`, and its `-${F}` spelling, which
///   the tokenizer flushes to a bare `-$`. This is the SAME hole one character
///   to the left: the token begins with `-`, so it is neither one of the first
///   two subcommand words (it is a flag to [`subcommand_word_indices`]), nor the
///   method value, nor marker-initial — a clause written only for the first form
///   leaves it in no part of the region at all.
///
/// The second clause is the same key-half readability test [`scan_leading`]
/// applies to `git -c`, and it is what leaves
/// `gh api repos/o/r/pulls -f title="$T"` alone: `-f` has a readable key half,
/// and `title="$T"` neither begins with `-` nor with a marker.
///
/// **The marker class is the same class [`Token::literal`] is cleared for, and
/// it tracks that class rather than restating a subset of it.** `-?` and
/// `-{f,x}` are the SAME hole as `-$F`: a `?` is resolved from the working
/// directory and a `{` is spliced back into the command, so whether the word is
/// a flag at all is no more knowable than when a `$` assembles it, and
/// `gh api repos/o/r/pulls -? title=x` was measured at exit 0 with an EMPTY walk
/// — the SAFE-06 cap BYPASSED rather than exceeded, with no second carrier
/// (`T-19-35`). A clause written for only two of the seven characters leaves the
/// other five in no part of the region at all.
///
/// **This selects an INDEX; it does not decide.** It is a pure `&str` predicate
/// because this is a pure argv function, and the readability question about the
/// word it names is answered by `Token.literal` in
/// [`first_unreadable_decision_word`]'s one closure — so a QUOTED `-'*'` is
/// named here and then passes there, because quoting made it literal.
fn api_flag_ness_is_unreadable(word: &str) -> bool {
    let rewritten = |text: &str| {
        text.chars()
            .any(|ch| REWRITING_CHARACTERS.contains(&ch) || ch == '{')
    };
    if word.starts_with(|ch| REWRITING_CHARACTERS.contains(&ch) || ch == '{') {
        return true;
    }
    if word.starts_with('-') {
        let key = word.split_once('=').map(|(key, _)| key).unwrap_or(word);
        return rewritten(key);
    }
    false
}

/// The first word a governed program's classifier DECIDES ON that is **not
/// provably LITERAL**, or `None`.
///
/// ## The inversion, and why it is one rather than a sixth enumeration
///
/// Rounds 1–3 enumerated the ways a shell assembles a word and guarded each: an
/// expansion bit at the decision boundary (Rule A), a severed head behind a
/// word-splitting closer (Rule B). **That did not converge in five rounds** —
/// each rule decides on one signal, and neither can fail on the ways bash makes
/// a word that set no signal at all: brace expansion, pathname expansion, tilde
/// expansion, `$IFS`-driven re-splitting.
///
/// So the question changes direction. **A decision word must be LITERAL — the
/// shell must hand it to the program byte-identically to how it is written — and
/// anything that is not provably literal is unresolvable and refuses.** That is
/// positive evidence of literalness instead of an enumeration of
/// non-literalness, and it closes every mechanism above, plus the ones nobody
/// has enumerated, in ONE rule. The evidence is [`Token::literal`], collected by
/// `tokenize` while the word is consumed; it is read HERE, in the one closure
/// that used to read `Token.expansion`, and nowhere else.
///
/// **The boundary is the same boundary, and it is what keeps the cost small.**
/// Operands are deliberately free: a rule that refused every non-literal word in
/// a governed segment would refuse ordinary commit messages and pull-request
/// titles, and a control that fails into unusability gets switched off
/// (AR-19-11). `git commit -m "use ${HOME} here"`,
/// `gh pr create --title 'fix $PATH handling'`, `git add src/*.rs` and
/// `rg "x" src/*` all keep working, and
/// `gh api repos/{owner}/{repo}/pulls -f title=x` — `gh`'s own documented
/// placeholder syntax, literal in bash — stays COUNTED.
///
/// **The whole rule is not here.** A brace expansion is a property of the
/// SIMPLE COMMAND rather than of a word — `git {push,--force} origin main` has
/// no decision word to test at all — so clause 2 lives in
/// [`resolve_program_with_head`], reading [`Segment::brace_spliced`] and
/// [`Segment::splice_can_produce_governed`].
///
/// **The root cause this closes, in one paragraph.**
/// `super::hooks::classify_segments` collapses each [`Token`] to its `text`
/// before either classifier runs, so `Token.expansion` is structurally
/// unavailable to [`classify_git`] and [`pr_command_label`] — both of which are
/// pure argv functions and must stay so. [`resolve_program`] refuses an
/// expansion at the HEAD (step 3) and one in the WRAPPER PREFIX (step 5), and
/// the words between them — the ones every classifier decision turns on — were
/// exempt by an explicit comment. This function is where the bit is restored: at
/// the decision boundary, over the same slice the caller already holds, called
/// ONCE, before both classifiers and before the ledger write.
///
/// ## The region is exactly the words the matched arm reads
///
/// * **git — the VERB**, at the absolute index [`classify_git`] itself slices to
///   (`governed + 1 + scan_leading`), plus, when that verb is `config`, the one
///   operand [`classify_config`] tests with [`is_hooks_path_key`], at the index
///   [`config_key_operand_index`] reports.
/// * **forge — the first TWO non-flag subcommand words**, because
///   [`pr_command_label`] matches on `["pr", "create", ..]` and
///   `["mr", "create", ..]`.
/// * **forge, `api` arm only** — the ENDPOINT at the index [`scan_gh_api`]
///   reports, the `-X`/`--method` value in both its spellings, and any token
///   whose flag-ness that arm cannot read
///   ([`api_flag_ness_is_unreadable`]).
///
/// Every index is reported by the scan the classifier itself runs, so the region
/// cannot drift one slot from the words that decide. That drift is the defect
/// this round is about, and it had already happened: see [`scan_gh_api`]'s doc
/// for the endpoint an `api` option value displaces out of
/// [`subcommand_word_indices`]' first two words entirely.
///
/// ## What is deliberately NOT in the region, and why
///
/// * **Operands.** `git commit -m "$MSG"`, `gh pr create --title "$TITLE"` and
///   `gh api repos/o/r/pulls -f title="$T"` all still run. A rule that refused
///   every expansion in a governed segment would be `T-19-75` widened from `rg`
///   to every commit, and a control that fails into unusability gets switched
///   off (AR-19-11). This boundary is the whole of the design's cost
///   containment and it is pinned from both sides in
///   `tests/envelope_expansion_slots.rs`.
/// * **The tokens [`scan_leading`] WALKS.** [`leading_git_option`] consumes a
///   `-c` option AND its assignment token, so a region defined as "what that
///   scan walked" would put `user.name="$NAME"` inside it and refuse
///   `git -c user.name="$NAME" commit -m x`, which is pinned permitted. Those
///   options are governed by [`scan_leading`]'s own key-half check instead,
///   which matches the only decision that scan makes.
/// * **All of [`subcommand_word_indices`].** That pulls `--title`'s value into
///   the region and refuses a pull-request title carrying a `$`.
///
/// A governed program with no classifier arm has no decision region here; the
/// fail-closed arm in `classify_segments` already refuses it.
pub fn first_unreadable_decision_word(segment: &[Token], governed: usize) -> Option<DecisionWord> {
    let words: Vec<&str> = segment.iter().map(|token| token.text.as_str()).collect();
    let program = program_name(words.get(governed)?);

    // **The ONE place the bit is read, and the whole of the inversion.** Every
    // clause below reports an index; this turns an index into a finding. It used
    // to ask `token.expansion` — did the tokenizer SEE one of two characters —
    // and it now asks for positive evidence that the shell hands this word over
    // unchanged. If a second reading site is ever needed here, the decision
    // region was not factored the way `19-14` claims and audit 4 verified, and
    // that is a finding to report rather than a place to add one.
    let at = |index: usize, role: &'static str| -> Option<DecisionWord> {
        let token = segment.get(index)?;
        (!token.literal).then(|| DecisionWord {
            index,
            word: token.text.clone(),
            role,
        })
    };

    let rest_start = governed + 1;
    let rest: Vec<&str> = words.get(rest_start..).unwrap_or_default().to_vec();

    match program {
        "git" => {
            // The verb, at the absolute index `classify_git` slices to. The
            // refusal `scan_leading` may also have earned is not consulted here:
            // this function answers about readability, and `classify_git` still
            // runs afterwards.
            let (verb_index, _) = scan_leading(&rest);
            if let Some(found) = at(rest_start + verb_index, "the git verb") {
                return Some(found);
            }

            // `config`'s key operand — the one other word a git classifier arm
            // decides on that this plan closes. `classify_config` reaches
            // `is_hooks_path_key` on it and answers `Allow` when it cannot read
            // it, so `git config $K /tmp/x` disarms layer 3 exactly as
            // `git config core.hooksPath /tmp/x` would.
            if rest.get(verb_index).copied() == Some("config") {
                let config_rest = &rest[verb_index + 1..];
                if let Some(key_index) = config_key_operand_index(config_rest) {
                    let absolute = rest_start + verb_index + 1 + key_index;
                    if let Some(found) = at(absolute, "the `git config` key operand") {
                        return Some(found);
                    }
                }
            }
        }

        "gh" | "glab" => {
            // The first TWO subcommand words, because `pr_command_label` matches
            // on two. A ONE-word region leaves `P=create; gh pr $P --title x`
            // matching no arm — neither refused nor counted, which is the
            // SAFE-06 cap BYPASSED rather than exceeded, and the cap has no
            // second carrier (`T-19-35`).
            let subcommands = subcommand_word_indices(&rest);
            for index in subcommands.iter().take(2) {
                if let Some(found) = at(rest_start + index, "a forge subcommand word") {
                    return Some(found);
                }
            }

            // The `api` arm reads three more kinds of word, and it reads them
            // through its OWN scan. See `scan_gh_api`'s doc for why taking the
            // endpoint from `subcommand_word_indices` instead loses it entirely.
            let is_api = program == "gh"
                && subcommands
                    .first()
                    .is_some_and(|index| rest[*index] == "api");
            if is_api {
                let scan = scan_gh_api(&rest);
                if let Some(endpoint) = scan.endpoint {
                    if let Some(found) = at(rest_start + endpoint, "the `gh api` endpoint") {
                        return Some(found);
                    }
                }
                if let Some(method) = scan.method_word {
                    if let Some(found) = at(rest_start + method, "the `gh api` method") {
                        return Some(found);
                    }
                }
                for (offset, word) in rest.iter().enumerate() {
                    if api_flag_ness_is_unreadable(word) {
                        if let Some(found) =
                            at(rest_start + offset, "a `gh api` word whose flag-ness decides")
                        {
                            return Some(found);
                        }
                    }
                }
            }
        }

        _ => {}
    }

    None
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

/// The governed program named by a forge argv's own first subcommand word, if
/// any.
///
/// **The forge twin of [`classify_git`]'s governed-verb refusal, and a SECOND
/// layer for the same reason.** `env -u gh gh pr create --title x` was measured
/// at exit 0 with **no ledger line** (`19-SECURITY.md`, audit 2): the decoy
/// captured the resolver's index, [`pr_command_label`] was then handed an argv
/// whose subcommand chain begins `["gh", "pr", "create", …]`, no arm matched, and
/// the cap was not exceeded — it was never counted. `classify_git`'s arm cannot
/// see this, because a forge command never enters that function.
///
/// A `Some` here is a REFUSAL at the call site, taken **before** the ledger
/// write, so a decoy never consumes cap budget.
///
/// After the command-position rule this argv no longer reaches the forge arm
/// through `super::hooks::guard_in`, so this predicate is pinned by a unit test
/// on the function itself. It is not dead code: it is what holds if resolution
/// ever mis-indexes again.
pub fn forge_subcommand_names_a_governed_program(argv: &[&str]) -> Option<&'static str> {
    let rest = argv.get(1..)?;
    let first = subcommand_words(rest).first().copied()?;
    GOVERNED_PROGRAMS
        .iter()
        .copied()
        .find(|governed| *governed == program_name(first))
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

/// The INDICES into `rest` of the non-flag words of a subcommand chain, in
/// order.
///
/// **The index primitive, and [`subcommand_words`] is defined over it.** The
/// decision region [`first_unreadable_decision_word`] computes has to name the
/// words `pr_command_label` matches its arms on, at the positions they occupy in
/// the segment — and the only way to be sure it names the same words is to take
/// them from the same walk. A second copy of this loop would be a second thing
/// to keep in step, and the day they drift is the day the rule guards a
/// different word than the one the classifier reads. That is exactly what had
/// already happened between this scan and [`scan_gh_api`]; see
/// [`scan_gh_api`]'s doc.
///
/// Flags are skipped rather than terminating the scan, because `gh --repo o/r pr
/// create` is a legal invocation and a scan that stopped at the first `-` would
/// miss it.
fn subcommand_word_indices(rest: &[&str]) -> Vec<usize> {
    let mut indices = Vec::new();
    let mut index = 0;

    while index < rest.len() {
        let word = rest[index];
        if FORGE_VALUE_OPTS.contains(&word) {
            index += 2;
            continue;
        }
        if !word.starts_with('-') {
            indices.push(index);
        }
        index += 1;
    }

    indices
}

/// The non-flag words of a subcommand chain, in order.
fn subcommand_words<'a>(rest: &[&'a str]) -> Vec<&'a str> {
    subcommand_word_indices(rest)
        .into_iter()
        .map(|index| rest[index])
        .collect()
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

/// One walk of a `gh api` argv, reported by index.
struct GhApiScan {
    /// The index into `rest` of the ENDPOINT this arm takes, if it found one.
    endpoint: Option<usize>,
    /// The index into `rest` of the word carrying the HTTP method — the VALUE
    /// word of a separate `-X`/`--method`, or the `-X=…` token itself.
    method_word: Option<usize>,
    /// The method, upper-cased.
    method: Option<String>,
    /// Whether a member of [`GH_API_IMPLIES_POST`] is present.
    implies_post: bool,
}

/// The single walk of a `gh api` argv, over which both
/// [`gh_api_posts_a_pull_request`] and the `api` half of
/// [`first_unreadable_decision_word`] are defined.
///
/// **This primitive is not symmetry with [`subcommand_word_indices`], it is a
/// MEASURED hole, and the reason is recorded here rather than left to be
/// inferred.** The two forge scans DISAGREE about which words are flags:
/// [`subcommand_word_indices`] skips only [`FORGE_VALUE_OPTS`] (`-R`, `--repo`,
/// `--hostname`), while this one skips all of [`GH_API_VALUE_OPTS`] (`-f`,
/// `-F`, `--field`, `-H`, `--header`, `-q`, `-t`, `--input`, …). Any of those
/// option VALUES is therefore an ordinary non-flag word to the first scan and
/// displaces the endpoint past the first two subcommand words:
///
/// ```text
/// E=pulls; gh api -f title=x repos/o/r/$E
///   subcommand_words -> ["api", "title=x", "repos/o/r/$E"]
/// ```
///
/// An endpoint taken from `subcommand_words`' first two words would then be in
/// NO part of the decision region — the token is not one of the first two, it is
/// not the method value, it does not begin with `-`, and it does not begin with
/// an expansion marker. Meanwhile this scan skips the whole `-f` pair, sets
/// `implies_post`, takes `path = repos/o/r/$E`, and [`endpoint_is_pulls`]
/// compares `$E` against `"pulls"` — so the label is `None` and a pull request
/// opens with no refusal, no ledger line and no cap charge.
///
/// **A region computed by a second scan is the defect this whole round is
/// about.** It has sat one slot over four times now — the wrapper operand, the
/// governed program's own operand, the verb slot, and here. Every index in the
/// region is reported by the scan whose answer it guards.
fn scan_gh_api(rest: &[&str]) -> GhApiScan {
    let mut scan = GhApiScan {
        endpoint: None,
        method_word: None,
        method: None,
        implies_post: false,
    };
    let mut index = 0;
    // `api` itself is the first non-flag word; the endpoint is the second.
    let mut seen_api = false;

    while index < rest.len() {
        let token = rest[index];

        if let Some((name, value)) = token.split_once('=') {
            if name == "-X" || name == "--method" {
                scan.method = Some(value.to_ascii_uppercase());
                // The `=`-attached spelling carries the method in the option
                // token itself, so the decision word is that token.
                scan.method_word = Some(index);
                index += 1;
                continue;
            }
        }
        if GH_API_IMPLIES_POST.contains(&token) {
            scan.implies_post = true;
        }
        if GH_API_VALUE_OPTS.contains(&token) {
            if token == "-X" || token == "--method" {
                scan.method = rest.get(index + 1).map(|m| m.to_ascii_uppercase());
                scan.method_word = rest.get(index + 1).map(|_| index + 1);
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
        } else if scan.endpoint.is_none() {
            scan.endpoint = Some(index);
        }
        index += 1;
    }

    scan
}

fn gh_api_posts_a_pull_request(rest: &[&str]) -> bool {
    let scan = scan_gh_api(rest);

    let posts = match scan.method.as_deref() {
        Some(explicit) => explicit == "POST",
        None => scan.implies_post,
    };

    posts && scan.endpoint.is_some_and(|index| endpoint_is_pulls(rest[index]))
}

/// Whether an endpoint names the pull-request collection.
///
/// The query string and any trailing slash are stripped first, and the check is
/// on the final path segment: `repos/o/r/pulls`, a full
/// `https://api.github.com/repos/o/r/pulls`, and `/repos/o/r/pulls?state=open`
/// are the same endpoint. A single pull request (`…/pulls/7`) is **not** the
/// collection, and a `POST` to it is a review comment rather than a creation.
///
/// ## `T-19-93`, and why this function was NOT changed
///
/// `gh api repos/{owner}/{repo}/pulls -f title=x` is `gh`'s own documented
/// placeholder syntax, not an evasion, and it was measured at exit 0 with an
/// EMPTY envelope walk — the SAFE-06 cap BYPASSED rather than exceeded, with no
/// second carrier (`T-19-35`). **The correctness bar was COUNT, not refuse**: an
/// agent following `gh`'s own manual must not be denied, because a control that
/// fails into unusability gets switched off (AR-19-11).
///
/// It is closed in the TOKENIZER and nowhere else. Bash passes a comma-free
/// brace pair through unchanged — `printf "[%s]" repos/{owner}/{repo}/pulls`
/// prints it byte-identically, measured — so `tokenize` classifies it as a
/// LITERAL pair and absorbs it into the word. The endpoint therefore reaches
/// [`scan_gh_api`] intact and **this function already answers `true` for it**,
/// with no change to either forge scan.
///
/// **The rejected alternative was a placeholder tolerance HERE**, and it was
/// rejected for three reasons a later reader can check. It is a second consumer
/// compensating for a splitter that mangles the word, which is the defect this
/// phase produced four times. It fixes one cell and leaves the identical
/// spelling one slot over in a git verb, a `config` key operand and a `-c`
/// assignment. And it would still have been needed on top of the tokenizer work
/// `T-19-92` required anyway.
///
/// The boundary is unchanged and pinned: `repos/{owner}/{repo}/pulls/7` is a
/// single pull request and stays UNCOUNTED, so the tolerance cannot degrade into
/// "any endpoint with braces counts".
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
            // Deliberately empty: this fixture is the pre-Phase-19 shape, and
            // these tests are about envelope field resolution rather than the
            // prompt-input disclosure.
            prompt_inputs: Vec::new(),
            extra: Default::default(),
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

    /// Which of the tokenizer's three `{` cases a line took, read off the token
    /// stream rather than asserted about the implementation.
    #[derive(Debug, PartialEq, Eq)]
    enum BraceCase {
        /// Case 1 or case 2: today's behaviour byte-for-byte — a `{` operator
        /// with no splice mark. The two are told apart by the flush flag.
        PassedThrough { flush: bool },
        /// Case 3, literal branch: no `{` operator token at all, because the
        /// pair was absorbed into the word.
        Absorbed,
        /// Case 3, expansion or unmatched branch: today's segmentation PLUS the
        /// mark.
        Spliced { produces_governed: bool },
    }

    fn brace_case(cmd: &str) -> BraceCase {
        let tokens = tokenize(cmd).unwrap_or_else(|| panic!("`{cmd}` tokenizes"));
        match tokens
            .iter()
            .find(|token| token.operator && token.text == "{")
        {
            None => BraceCase::Absorbed,
            Some(open) if open.brace_splice => BraceCase::Spliced {
                produces_governed: tokens
                    .iter()
                    .any(|token| token.splice_can_produce_governed),
            },
            Some(open) => BraceCase::PassedThrough {
                flush: open.word_splitting_flush,
            },
        }
    }

    fn words(cmd: &str) -> Vec<String> {
        split_command(cmd).unwrap_or_else(|| panic!("`{cmd}` splits"))
    }

    /// The products of ONE word, scanned from its first character — the column
    /// that is per WORD rather than per `{`.
    fn products_of(word: &str) -> Option<Vec<String>> {
        let chars: Vec<char> = word.chars().collect();
        brace_word_products(&chars, 0).products
    }

    fn some(products: &[&str]) -> Option<Vec<String>> {
        Some(products.iter().map(|p| (*p).to_string()).collect())
    }

    #[test]
    fn the_tokenizers_three_way_brace_classification_and_its_per_word_products() {
        // **The load-bearing arithmetic of this table is its PRODUCTS column.**
        // Nothing else here distinguishes `{g..g}{i..i}t` from `{a,b}`: both are
        // brace expansions, both set the same mark, both keep today's
        // segmentation. A per-`{` implementation answers `g` and `it` for the
        // first — neither governed, both sets enumerating cleanly — and one that
        // joined its literal runs AS WRITTEN answers `"g"i"t"` for
        // `"g"{i,i}"t"`. Each of those turns a row below red, which is why the
        // products are asserted here rather than only a verdict somewhere else.

        // --- case 1: a `{` preceded in-word by an unquoted `$` -------------
        //
        // Today's behaviour EXACTLY, and this case exists so that Rule B stays
        // load-bearing: `${C}_COUNT` must keep fragmenting.
        assert_eq!(
            brace_case("${X}push"),
            BraceCase::PassedThrough { flush: true },
            "a parameter expansion is a separator with a word-splitting flush, as it was \
             before this round. Folding it into case 3 makes `${{C}}_COUNT` one word and \
             Rule B dead code."
        );
        assert_eq!(words("${X}push"), vec!["$", "{", "X", "}", "push"]);

        // --- case 2: a `{` that is a complete word -------------------------
        assert_eq!(
            brace_case("{ cmd; }"),
            BraceCase::PassedThrough { flush: false },
            "bash's reserved word opening a GROUP. `{{ cmd; }}` keeps its segments \
             byte-for-byte, or the `echo hi && git push --force` class re-opens one level in."
        );
        assert_eq!(
            split_segments("( cmd )")
                .unwrap()
                .iter()
                .map(|segment| segment.iter().map(|t| t.text.clone()).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            vec![vec!["cmd".to_string()]],
            "`(` and `)` are not touched by this round at all"
        );

        // --- case 3, literal branch: no comma and no range ------------------
        //
        // Measured: `printf "[%s]" repos/{owner}/{repo}/pulls` prints
        // `[repos/{owner}/{repo}/pulls]` — bash passes the pair through.
        assert_eq!(brace_case("repos/{owner}/{repo}/pulls"), BraceCase::Absorbed);
        assert_eq!(
            words("gh api repos/{owner}/{repo}/pulls -f title=x"),
            vec!["gh", "api", "repos/{owner}/{repo}/pulls", "-f", "title=x"],
            "the endpoint must reach `scan_gh_api` as ONE word, or `endpoint_is_pulls` \
             cannot read it and the creation is never COUNTED (`T-19-93`)"
        );
        assert_eq!(brace_case("git reflog delete HEAD@{0}"), BraceCase::Absorbed);
        assert_eq!(
            words("git reflog delete HEAD@{0}"),
            vec!["git", "reflog", "delete", "HEAD@{0}"]
        );
        assert_eq!(
            brace_case("gh api \"repos/{owner}/{repo}/pulls\""),
            BraceCase::Absorbed,
            "inside double quotes the braces never reach the classification at all, and \
             this spelling is COUNTED today"
        );
        assert_eq!(brace_case("echo '{a,b}'"), BraceCase::Absorbed);
        assert_eq!(
            words("echo '{a,b}'"),
            vec!["echo", "{a,b}"],
            "single quotes make it literal; a comma inside them is not a splice"
        );

        // --- case 3, expansion branch --------------------------------------
        for (cmd, produces_governed) in [
            ("git {push,--force} origin main", false),
            ("{git,push,--force,origin,main}", true),
            ("{g..g}it push", true),
            ("g{i,i}t push", true),
            ("{g..g}{i..i}t push", true),
            ("{g,g}{i,i}{t,t} push", true),
            ("{g..g..1}it push", true),
            ("{g{i,i}t,x} push", true),
            ("\"g\"{i,i}\"t\" push", true),
            ("{g..g}\"it\" push", true),
            ("echo {a,b}", false),
            ("ls {git,svn}-repo", false),
            ("cp x{,.bak}", false),
        ] {
            assert_eq!(
                brace_case(cmd),
                BraceCase::Spliced { produces_governed },
                "`{cmd}` is a brace EXPANSION; its splice marks the enclosing simple command \
                 and its products decide clause 2(b)"
            );
        }

        // --- case 3, unmatched: fail closed --------------------------------
        assert_eq!(
            brace_case("x{"),
            BraceCase::Spliced {
                produces_governed: true
            },
            "an unmatched brace is unclassifiable, so it is marked and its products are \
             unenumerable — refusing what cannot be established"
        );

        // --- the PRODUCTS column, per WORD ---------------------------------
        assert_eq!(products_of("{a,b}"), some(&["a", "b"]));
        assert_eq!(products_of("{a..d}"), some(&["a", "b", "c", "d"]));
        assert_eq!(products_of("{1..3}"), some(&["1", "2", "3"]));
        assert_eq!(products_of("x{,.bak}"), some(&["x", "x.bak"]));
        assert_eq!(products_of("{git,svn}-repo"), some(&["git-repo", "svn-repo"]));
        assert_eq!(products_of("{git,x}"), some(&["git", "x"]));

        // The six that PRODUCE `git`, none of which any alternative spells.
        assert_eq!(products_of("{g..g}it"), some(&["git"]));
        assert_eq!(products_of("g{i,i}t"), some(&["git", "git"]));
        assert_eq!(
            products_of("{g..g}{i..i}t"),
            some(&["git"]),
            "composed across BOTH `{{`s. A per-`{{` scan answers `g` and `it` here, neither \
             governed and both enumerating cleanly, and the command is permitted."
        );
        assert_eq!(
            products_of("{g,g}{i,i}{t,t}"),
            some(&["git", "git", "git", "git", "git", "git", "git", "git"])
        );
        assert_eq!(
            products_of("\"g\"{i,i}\"t\""),
            some(&["git", "git"]),
            "the literal runs are joined with their QUOTING REMOVED. Joined as written this \
             reads `\"g\"i\"t\"`, whose basename is not governed."
        );
        assert_eq!(products_of("{g..g}\"it\""), some(&["git"]));

        // The two that are UNENUMERABLE, and each is a trigger of its own.
        assert_eq!(
            products_of("{g..g..1}it"),
            None,
            "the INCREMENT form is neither a comma list nor a two-endpoint range, so it \
             falls outside every other trigger and needs one of its own"
        );
        assert_eq!(
            products_of("{g{i,i}t,x}"),
            None,
            "a NESTED `{{` inside an alternative is a fail-closed TRIGGER rather than a \
             recursion. Decomposed into top-level alternatives this is `g{{i,i}}t` and `x`, \
             neither governed."
        );
        assert_eq!(products_of("x{"), None, "an unmatched brace");
        assert_eq!(products_of("{a,b}$X"), None, "a `$` in a literal run");
        assert_eq!(products_of("{a,b}*"), None, "a glob in a literal run");
        assert_eq!(products_of("{a,b}~"), None, "a tilde in a literal run");
        assert_eq!(products_of("{$X,b}"), None, "a `$` inside an alternative");
        assert_eq!(products_of("{a..zzz}"), None, "endpoints that are not single characters");
        assert_eq!(products_of("{1..9999}"), None, "a range over the product cap");

        // The extent stops at unquoted whitespace and at a REAL command
        // operator, and NEVER at a `{` or a `}`.
        assert_eq!(
            products_of("{g..g}it push --force origin main"),
            some(&["git"]),
            "the extent ends at the first unquoted space, so the operands are not products"
        );
        assert_eq!(products_of("{a,b};git"), some(&["a", "b"]));
        assert_eq!(products_of("{a,b}|git"), some(&["a", "b"]));
        assert_eq!(products_of("{a,b}&&git"), some(&["a", "b"]));
    }

    #[test]
    fn the_brace_splice_fact_reaches_every_segment_of_the_simple_command() {
        // The mark is a property of the simple COMMAND, so it applies to the
        // segment that was already pushed when the `{` arrived — `git` here —
        // and it resets at the next REAL command operator.
        let segments = split_segments_with_heads("git {push,--force} origin main").unwrap();
        assert!(
            segments.iter().all(|segment| segment.brace_spliced),
            "every segment of `git {{push,--force}} origin main` belongs to one spliced \
             simple command, including the `git` pushed before the `{{` was seen: {segments:?}"
        );
        assert!(
            segments
                .iter()
                .all(|segment| !segment.splice_can_produce_governed),
            "its products are `push` and `--force`; neither basename is governed, so this \
             command is reached by clause 2(a) and NOT by 2(b)"
        );

        let mixed = split_segments_with_heads("echo {a,b} && git status").unwrap();
        let git = mixed
            .iter()
            .find(|segment| segment.tokens.first().is_some_and(|t| t.text == "git"))
            .expect("one segment begins with `git`");
        assert!(
            !git.brace_spliced,
            "`&&` ends the simple command, so the splice in front of it must not reach \
             `git status`: {mixed:?}"
        );

        let produced = split_segments_with_heads("{env,git} push --force origin main").unwrap();
        assert!(
            produced
                .iter()
                .all(|segment| segment.splice_can_produce_governed),
            "nothing here resolves `Governed` — the head word is `env,git` — so only the \
             PRODUCTS reach it: {produced:?}"
        );
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

    // ---- the redirection production and the line continuation (round 6) ----

    /// Whether any segment of `cmd` carries the unresolvable-redirection mark.
    fn unresolvable(cmd: &str) -> bool {
        split_segments_with_heads(cmd)
            .unwrap_or_else(|| panic!("`{cmd}` must tokenize"))
            .iter()
            .any(|segment| segment.redirection_unresolvable)
    }

    #[test]
    fn the_redirection_and_continuation_table_recovers_exactly_the_argv_bash_runs() {
        // **The invariant, and the whole of round 6: the words the guard
        // classifies must be exactly the words the program receives, in the same
        // order — no more and no fewer.** Every expected argv below was measured
        // under bash against argv-printing shims that write to a SIDE FILE
        // rather than stdout, precisely because eleven of these rows redirect
        // stdout and would otherwise print nothing.
        //
        // Each row is `(command, surviving words, is the command unresolvable)`.
        for (cmd, expected, marked) in [
            // --- the redirection production: operator AND target deleted ---
            // ARGV[git]: [push] [--force] [origin] [main] for every one of these.
            (
                "git >/dev/null push --force origin main",
                vec!["git", "push", "--force", "origin", "main"],
                false,
            ),
            // A bare digits-only run since the word start IS an IO_NUMBER.
            (
                "git 2>/dev/null push --force origin main",
                vec!["git", "push", "--force", "origin", "main"],
                false,
            ),
            // **The OVER-DELETION control, and the row that makes this table
            // non-decorative.** `x2` is NOT digits-only, so bash ends the word
            // `x2` and begins the redirection: `ARGV[git]: [x2] [push] [--force]
            // [origin] [main]`. `x2` IS argv, and real git answers
            // `git: 'x2' is not a git command`. An implementation that deleted
            // any word part before a `>` turns this row RED — and nothing else
            // in this table's columns distinguishes over-deletion from the
            // model.
            (
                "git x2>/tmp/o push --force origin main",
                vec!["git", "x2", "push", "--force", "origin", "main"],
                false,
            ),
            // A QUOTED digit run is not an IO_NUMBER either: measured,
            // `ARGV[git]: [2] [push] [--force] [origin] [main]`.
            (
                "git \"2\">/tmp/o push --force origin main",
                vec!["git", "2", "push", "--force", "origin", "main"],
                false,
            ),
            // An ATTACHED operator terminates the word before it; the operator
            // need not be its own word and `push` survives.
            (
                "git push>/dev/null --force origin main",
                vec!["git", "push", "--force", "origin", "main"],
                false,
            ),
            // Whitespace is allowed between the operator and its target.
            (
                "git > /tmp/o push --force origin main",
                vec!["git", "push", "--force", "origin", "main"],
                false,
            ),
            // `&>` recognised BEFORE `&` reaches the separator arm — one simple
            // command to bash, and two segments to the guard without this.
            (
                "git &>/tmp/o push --force origin main",
                vec!["git", "push", "--force", "origin", "main"],
                false,
            ),
            (
                "git &>>/tmp/o push --force origin main",
                vec!["git", "push", "--force", "origin", "main"],
                false,
            ),
            // The multi-character operators no single-`>` rule reaches.
            (
                "git <<<x push --force origin main",
                vec!["git", "push", "--force", "origin", "main"],
                false,
            ),
            (
                "git >|/tmp/o push --force origin main",
                vec!["git", "push", "--force", "origin", "main"],
                false,
            ),
            (
                "git <>/tmp/o push --force origin main",
                vec!["git", "push", "--force", "origin", "main"],
                false,
            ),
            (
                "git >>/tmp/x push --force origin main",
                vec!["git", "push", "--force", "origin", "main"],
                false,
            ),
            // A heredoc: the DELIMITER is the target and is deleted; the body is
            // not argv and is not modelled.
            (
                "git <<EOF push --force origin main",
                vec!["git", "push", "--force", "origin", "main"],
                false,
            ),
            // `2>&1` — the fd-duplicating operator, target `1`.
            (
                "git fetch origin 2>&1",
                vec!["git", "fetch", "origin"],
                false,
            ),
            // --- fail-closed: a production that does not COMPLETE ---
            // `git >` — bash does not run this either (`syntax error near
            // unexpected token 'newline'`). Its permitted twin `ls >` carries
            // the same mark and is NOT refused, because the mark only refuses
            // when a governed program is reached.
            ("git >", vec!["git", ">"], true),
            ("ls >", vec!["ls", ">"], true),
            // A `{name}` fd-allocation prefix is deliberately not modelled.
            (
                "git {v}>/tmp/o push --force origin main",
                vec!["git", "{v}", ">", "push", "--force", "origin", "main"],
                true,
            ),
            // --- quoting: an operator is recognised only OUTSIDE quotes ---
            // These rows turn RED for an implementation that ignores quoting,
            // and nothing else in the table's columns catches that.
            (
                "git commit -m \">\"",
                vec!["git", "commit", "-m", ">"],
                false,
            ),
            (
                "git log --grep='>'",
                vec!["git", "log", "--grep=>"],
                false,
            ),
            ("rg \">\" src/", vec!["rg", ">", "src/"], false),
            // A BACKSLASH-escaped operator is a literal character too:
            // `ARGV[git]: [>x] [push]`, measured.
            ("git \\>x push", vec!["git", ">x", "push"], false),
            // --- the line continuation: two characters bash DELETES ---
            // `git pu\<NL>sh --force origin main`
            //   -> ARGV[git]: [push] [--force] [origin] [main]
            (
                "git pu\\\nsh --force origin main",
                vec!["git", "push", "--force", "origin", "main"],
                false,
            ),
            // It must NOT start a word: two words here, not three.
            (
                "git \\\npush --force origin main",
                vec!["git", "push", "--force", "origin", "main"],
                false,
            ),
            // Bash performs the continuation inside DOUBLE quotes too, through
            // the quote loop's own backslash branch.
            (
                "git \"pu\\\nsh\" --force origin main",
                vec!["git", "push", "--force", "origin", "main"],
                false,
            ),
            // **The SINGLE-quote loop is untouched**, and this row is why:
            // bash performs no continuation there, and
            // `git 'pu\<NL>sh' --force origin main` gives `[pu\<NL>sh]` —
            // bytes verified with `od -c`.
            (
                "git 'pu\\\nsh' --force origin main",
                vec!["git", "pu\\\nsh", "--force", "origin", "main"],
                false,
            ),
            // The displacement row: the whitespace AFTER a continuation used to
            // flush it into its own word occupying the REMOTE slot.
            (
                "git push \\\n origin refs/heads/gsd-auto/alpha/w",
                vec!["git", "push", "origin", "refs/heads/gsd-auto/alpha/w"],
                false,
            ),
        ] {
            assert_eq!(
                words(cmd),
                expected,
                "the words recovered from `{cmd:?}` must be exactly the argv bash runs"
            );
            assert_eq!(
                unresolvable(cmd),
                marked,
                "the unresolvable mark for `{cmd:?}`"
            );
        }

        // **`literal` is NOT cleared by any of this, and a deletion is not a
        // rewrite.** Round 5's bit stays RIGHT about the words it is right
        // about; clearing it to obtain a refusal would turn every one of round
        // 5's verdict pins green while the inversion quietly stopped being the
        // thing that produced them.
        for cmd in [
            "git pu\\\nsh --force origin main",
            "git \"pu\\\nsh\" --force origin main",
        ] {
            assert!(
                tokenize(cmd)
                    .unwrap()
                    .iter()
                    .all(|token| token.literal),
                "every word of `{cmd:?}` is handed to the program byte for byte"
            );
        }
    }

    #[test]
    fn an_unenumerable_brace_word_is_distinguishable_from_an_enumerated_governed_one() {
        // **Audit 5's disclosed CORPUS LIMIT, addressed in the one place the
        // distinction is observable.** `sh {-c,"git push --force …"}` is refused
        // because a QUOTE inside an alternative makes the word's product set
        // unenumerable — correct, and fail-closed. But the corpus cannot tell
        // that refusal apart from an ENUMERATED one: both reach the same
        // `envelope_assertion_failed` identifier through the same clause at the
        // guard boundary. The difference exists only inside the whole-word
        // product scan, which is why this is a unit assertion and not a driven
        // row.
        let scan = |word: &str| {
            let chars: Vec<char> = word.chars().collect();
            brace_word_products(&chars, 0)
        };

        // UNENUMERABLE — `products` is `None`, and `can_produce_governed`
        // answers `true` because a set that cannot be computed cannot be
        // cleared, not because a governed name was found in it.
        let quoted_alternative = scan("{-c,\"git push --force origin main\"}");
        assert_eq!(
            quoted_alternative.products, None,
            "a quote inside an alternative is UNENUMERABLE: bash's quote removal \
             happens after the splice, so the products are not knowable here"
        );
        assert!(
            quoted_alternative.can_produce_governed(),
            "and it fails CLOSED"
        );

        // ENUMERATED — a real product set, and a governed BASENAME actually in
        // it. `{g..g}it` produces exactly `git`, which no alternative spells:
        // products, never names, and the two are one slot apart.
        let range = scan("{g..g}it");
        assert_eq!(
            range.products,
            Some(vec!["git".to_string()]),
            "the products are ENUMERATED, and the governed word is the one the \
             splice PRODUCES rather than one any alternative spells"
        );
        assert!(range.can_produce_governed());

        // The control that keeps the two answers apart from each other: an
        // enumerated set with NO governed product is cleared, so `true` above
        // is not vacuous.
        let benign = scan("{git,svn}-repo");
        assert_eq!(
            benign.products,
            Some(vec!["git-repo".to_string(), "svn-repo".to_string()])
        );
        assert!(
            !benign.can_produce_governed(),
            "`ls {{git,svn}}-repo` stays PERMITTED — a mention test would refuse \
             it, and a mention test is one slot away from the class"
        );
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

    // ---- which token is the effective program (T-19-60) ----

    /// The resolution of the FIRST simple command of `cmd`.
    fn resolve(cmd: &str) -> ProgramResolution {
        let segments = split_segments(cmd).expect("the fixture command's words are recoverable");
        resolve_program(&segments[0])
    }

    #[test]
    fn the_six_measured_bypass_lines_all_resolve_to_the_governed_program() {
        // The lines `19-SECURITY.md` reproduced at exit 0 against the built
        // binary. The index is the position `git`/`gh` occupies, because the
        // classifiers must be applied to the same argv they would have seen
        // unwrapped.
        assert_eq!(
            resolve("env git push --force origin main"),
            ProgramResolution::Governed { index: 1 }
        );
        assert_eq!(
            resolve("timeout 60 git push --force origin main"),
            ProgramResolution::Governed { index: 2 }
        );
        assert_eq!(
            resolve("command git push --force origin main"),
            ProgramResolution::Governed { index: 1 }
        );
        assert_eq!(
            resolve("env gh pr create --title x"),
            ProgramResolution::Governed { index: 1 }
        );
        assert_eq!(
            resolve("git push --force origin main"),
            ProgramResolution::Governed { index: 0 },
            "the unwrapped control: the wrapped and unwrapped forms must reach the same \
             classifier at the same argv"
        );

        // The assignment prefix is refused rather than merely skipped, because
        // the token that hides the command from layer 2 is the same one that
        // disarms layer 3.
        match resolve("GIT_CONFIG_COUNT=0 git push --force origin main") {
            ProgramResolution::Refuse { reason, detail } => {
                assert_eq!(reason, ParkReason::HookBypassBlocked);
                assert!(detail.contains("GIT_CONFIG_COUNT"), "{detail}");
            }
            other => panic!("the envelope-key assignment must refuse: {other:?}"),
        }
    }

    #[test]
    fn a_wrapper_name_that_appears_nowhere_in_this_crate_resolves_exactly_like_env() {
        // **This is the point of the whole design.** A fix built on a list of
        // wrapper names is green on the day it lands and silent on the seventh
        // wrapper nobody listed. Nothing here knows what `env` is called, so a
        // name invented in this assertion behaves identically to it.
        let known = resolve("env git push --force origin main");
        let invented = resolve("made-up-wrapper-9000 git push --force origin main");
        assert_eq!(
            known, invented,
            "an unlisted wrapper must resolve identically to a listed one, or the fix is \
             over a list rather than over the class"
        );

        assert_eq!(
            resolve("made-up-wrapper-9000 --flag git push --force"),
            ProgramResolution::Governed { index: 2 },
            "the unknown wrapper's own flags are skipped without being understood"
        );
        assert_eq!(
            resolve("nohup nice -n 10 stdbuf -oL setsid git push --force origin main"),
            ProgramResolution::Governed { index: 7 },
            "a chain of five names, none of which this module may know"
        );
    }

    #[test]
    fn a_wrapper_is_recognised_by_basename_at_any_path_spelling() {
        for spelling in [
            "/usr/bin/env git push --force",
            "busybox env git push --force",
            "/bin/busybox env /usr/bin/git push --force",
        ] {
            assert!(
                matches!(resolve(spelling), ProgramResolution::Governed { .. }),
                "`{spelling}` must resolve to the governed program: `/usr/bin/git` and \
                 `git` are the same program, and comparing whole strings is defeated by \
                 what `command -v` prints"
            );
        }
    }

    #[test]
    fn leading_assignment_words_are_a_prefix_and_a_bare_assignment_runs_nothing() {
        assert_eq!(
            resolve("FOO=bar BAZ=qux git push --force"),
            ProgramResolution::Governed { index: 2 }
        );
        assert_eq!(
            resolve("FOO=bar"),
            ProgramResolution::NoProgram,
            "a bare assignment executes no program, and refusing it would be refusing an \
             assignment"
        );
        assert_eq!(resolve("FOO=bar BAZ=qux"), ProgramResolution::NoProgram);
    }

    #[test]
    fn the_assignment_grammar_is_the_shells_own_and_not_a_test_for_an_equals_sign() {
        assert!(is_assignment_word("FOO=bar"));
        assert!(is_assignment_word("_x1="));
        assert!(
            !is_assignment_word("--opt=value"),
            "a long option is not an assignment, and skipping it as one would consume the \
             program token"
        );
        assert!(!is_assignment_word("a/b=c"));
        assert!(!is_assignment_word("core.hooksPath=x"));
        assert!(!is_assignment_word("=bar"));
        assert!(!is_assignment_word("plain"));

        // The consequence at the resolver: a `-m` message containing `=` must
        // not swallow anything.
        assert_eq!(
            resolve("git commit -m x=git"),
            ProgramResolution::Governed { index: 0 },
            "step 5 answers first, so step 7 can never fire on a commit message"
        );
    }

    #[test]
    fn an_expansion_assembled_head_and_an_eval_are_both_refused() {
        for hostile in ["$TOOL push --force", "eval \"git push --force\""] {
            assert!(
                matches!(
                    resolve(hostile),
                    ProgramResolution::Refuse {
                        reason: ParkReason::EnvelopeAssertionFailed,
                        ..
                    }
                ),
                "`{hostile}` must be refused: {:?}",
                resolve(hostile)
            );
        }
    }

    #[test]
    fn binding_a_governed_program_name_in_the_same_command_line_is_refused() {
        // The narrow half of `T-19-74` that IS closable: the binding and the
        // use are in one segment, so the guard can see both.
        assert!(
            matches!(
                resolve("X=git made-up-wrapper-9000 --flag run"),
                ProgramResolution::Refuse {
                    reason: ParkReason::EnvelopeAssertionFailed,
                    ..
                }
            ),
            "{:?}",
            resolve("X=git made-up-wrapper-9000 --flag run")
        );
    }

    #[test]
    fn a_dash_c_payload_is_followed_whatever_program_consumes_it() {
        // The structural replacement for the deleted `NESTED_SHELLS` list.
        // `-c` is the shell's own spelling for "here is a command line", so
        // nothing here needs to know what a shell is called.
        assert_eq!(
            resolve("bash -c \"git push --force\""),
            ProgramResolution::NestedPayload { index: 2 }
        );
        assert_eq!(
            resolve("bash -lc \"git push --force\""),
            ProgramResolution::NestedPayload { index: 2 },
            "`-lc` is covered where an exact `-c` match was not"
        );
        assert_eq!(
            resolve("script -c \"git push --force\" /dev/null"),
            ProgramResolution::NestedPayload { index: 2 },
            "`script` was never in the shell list and needs no entry in one"
        );
        assert_eq!(
            resolve("tar -czf a.tgz dir"),
            ProgramResolution::NestedPayload { index: 2 },
            "a `-c` that is not a shell's `-c` hands over a payload that resolves to \
             NOTHING rather than to a refusal, which is why following it costs nothing"
        );
        assert_eq!(
            resolve_program(&split_segments("a.tgz").unwrap()[0]),
            ProgramResolution::Ungoverned,
            "and that is the resolution of the payload `tar -czf` handed over"
        );
    }

    #[test]
    fn a_quoted_command_line_is_followed_when_its_first_word_is_governed() {
        // Step 5(b): the payload half of the class — a command handed to an
        // unknown program as one quoted string.
        assert_eq!(
            resolve("ssh host \"git push --force\""),
            ProgramResolution::NestedPayload { index: 2 }
        );
        assert_eq!(
            resolve("rg \"git status\" src/"),
            ProgramResolution::NestedPayload { index: 1 },
            "it DISCRIMINATES rather than blanket-denying: the payload is classified, so \
             a search for an allowed command stays allowed. The paired cost is T-19-75"
        );
    }

    #[test]
    fn an_ordinary_command_is_ungoverned_which_is_a_permit_and_not_a_gap() {
        // The guard sees EVERY Bash tool call. A resolution that denied what it
        // did not recognise would deny each of these and make a driven run
        // unusable, which is how a safety control gets switched off.
        for ordinary in [
            "ls -la",
            "echo hi",
            "rg -n TODO src/",
            "cargo test --lib",
            "timeout 5 ls",
            "make -j8 all",
        ] {
            assert_eq!(
                resolve(ordinary),
                ProgramResolution::Ungoverned,
                "`{ordinary}` must resolve to a permit"
            );
        }
    }

    #[test]
    fn removing_an_envelope_key_counts_as_reaching_for_it_just_as_setting_it_does() {
        // `env -u GIT_CONFIG_COUNT git push` and `unset GIT_CONFIG_COUNT` are
        // how the key is removed rather than set, and neither spells `=`.
        for shape in [
            "env -u GIT_CONFIG_COUNT git push --force",
            "unset GIT_CONFIG_COUNT",
            "GIT_CONFIG_KEY_0=core.hooksPath git push",
            "GIT_SSH_COMMAND=ssh git push origin refs/heads/gsd-auto/a/b",
        ] {
            assert!(
                matches!(
                    resolve(shape),
                    ProgramResolution::Refuse {
                        reason: ParkReason::HookBypassBlocked,
                        ..
                    }
                ),
                "`{shape}` must park under hook_bypass_blocked: {:?}",
                resolve(shape)
            );
        }

        assert!(
            tampers_with_envelope_env("GIT_CONFIG_VALUE_11").is_some(),
            "the indexed keys are matched as prefixes, because their suffix is an index"
        );
        assert!(
            tampers_with_envelope_env("GIT_COMMITTER_NAME=x").is_none(),
            "a git variable the envelope does NOT inject is not this refusal's business"
        );
    }

    #[test]
    fn classify_git_refuses_an_argv_whose_own_verb_is_a_governed_program() {
        // **The second layer, pinned HERE rather than through the guard, and the
        // placement is the honest one.** After the command-position rule
        // `resolve_program` refuses these segments before `classify_git` is
        // reached through `hooks::guard_in`, so a test driving the guard would
        // pass on the FIRST layer and prove nothing about this arm. Asserting it
        // on the pure function is the only way to observe it at all.
        //
        // It is not dead code and must not be deleted as unreachable: it is what
        // holds if resolution ever mis-indexes again, which it has now done
        // twice. Its fail-first was measured by reverting the arm, running this
        // test, observing red, and restoring — recorded in `19-13-SUMMARY.md`.
        for argv in [
            ["git", "push", "--force"].as_slice(),
            ["gh", "pr", "create"].as_slice(),
            ["glab", "mr", "create"].as_slice(),
            // Leading git options are scanned first, so the verb this arm reads
            // is the one AFTER them.
            ["-c", "user.name=x", "git", "push"].as_slice(),
            // Basename normalisation, so an absolute path cannot walk around it.
            ["/usr/bin/git", "push", "--force"].as_slice(),
        ] {
            assert!(
                matches!(
                    classify_git(argv, &ctx()),
                    GitVerdict::Refuse {
                        reason: ParkReason::EnvelopeAssertionFailed,
                        ..
                    }
                ),
                "`git {argv:?}` puts a governed program in its own VERB slot, which no \
                 valid invocation does. Reaching the denylist's default arm and answering \
                 `Allow` here is exactly how a mis-index became a permit (`T-19-60`, \
                 audit 2)."
            );
        }

        // The discrimination: an ordinary verb is untouched, and so is the
        // `T-19-86` shape, whose verb is `submodule` — that residual is
        // registered and deliberately still permitted.
        assert!(matches!(
            classify_git(&["status"], &ctx()),
            GitVerdict::Allow
        ));
        assert!(matches!(
            classify_git(
                &["submodule", "foreach", "git", "push", "--force", "origin", "main"],
                &ctx()
            ),
            GitVerdict::Allow
        ));
    }

    #[test]
    fn the_forge_predicate_names_a_governed_first_subcommand_word() {
        // The forge twin, pinned on the pure function for the same reason: after
        // the command-position rule it is unreachable through `guard_in`, and it
        // is the layer that holds if resolution mis-indexes again. Its fail-first
        // was measured the same way and is recorded in `19-13-SUMMARY.md`.
        //
        // The measured line: `env -u gh gh pr create --title x` exited 0 with NO
        // ledger line, because `pr_command_label` saw the chain `["gh", "pr",
        // "create", …]` and matched nothing. The cap was not exceeded — it was
        // never counted, which is why this refusal has to run BEFORE the ledger
        // write rather than after it.
        assert_eq!(
            forge_subcommand_names_a_governed_program(&["gh", "gh", "pr", "create", "--title"]),
            Some("gh")
        );
        assert_eq!(
            forge_subcommand_names_a_governed_program(&["gh", "git", "push", "--force"]),
            Some("git")
        );
        assert_eq!(
            forge_subcommand_names_a_governed_program(&["glab", "glab", "mr", "create"]),
            Some("glab")
        );
        // Flags are skipped rather than terminating the scan, so a global option
        // in front of the decoy does not hide it.
        assert_eq!(
            forge_subcommand_names_a_governed_program(&[
                "gh", "--repo", "o/r", "gh", "pr", "create"
            ]),
            Some("gh")
        );
        // Basename normalisation on the decoy.
        assert_eq!(
            forge_subcommand_names_a_governed_program(&["gh", "/usr/bin/gh", "pr", "create"]),
            Some("gh")
        );

        // Discrimination: an ordinary forge command names nothing governed, and
        // a governed name appearing as an option VALUE rather than as the first
        // subcommand word is not this predicate's business.
        assert_eq!(
            forge_subcommand_names_a_governed_program(&["gh", "pr", "create", "--title", "x"]),
            None
        );
        assert_eq!(
            forge_subcommand_names_a_governed_program(&["gh", "pr", "list", "--limit", "5"]),
            None
        );
        assert_eq!(forge_subcommand_names_a_governed_program(&["gh"]), None);
    }

    #[test]
    fn every_envelope_key_the_child_environment_actually_carries_is_covered_by_the_constant() {
        // The drift pin. `ENVELOPE_ENV_KEYS` is a second spelling of a fact
        // `cred::build_env_in` already owns, and the day they drift is the day
        // this refusal silently stops covering the key it was written for —
        // the same discipline `forbidden_repo_prefixes` uses by deriving its
        // runs path from `journal::RUNS_SUBDIR`.
        //
        // **Both of this pin's filters are DELETED, and that is `T-19-82`.**
        // It used to iterate only entries with `value.is_some()` and only names
        // beginning `GIT_`/`GH_`:
        //
        // - `value.is_some()` made every REMOVAL invisible. `SSH_AUTH_SOCK` and
        //   `SSH_AGENT_PID` are removed rather than set — they are the belt D-16
        //   relies on — so the pin could not see the two entries whose absence
        //   from the constant let a driven run put the user's own ssh-agent
        //   back. It would not have caught `GIT_SSH_COMMAND` either, had that
        //   key been a removal.
        // - The name filter would silently exclude a future `GITHUB_TOKEN`,
        //   `GLAB_CONFIG_DIR` or `SSH_*`, and it already excluded
        //   `GSD_MM_ENVELOPE_PROJECT_ROOT`.
        //
        // The pin now covers what its name claims: EVERY entry
        // `EnvelopeEnv::entries()` carries, set or removed, whatever it is
        // called. The four floors below each name the filter they replace, so
        // re-introducing either one turns this test red instead of green.
        //
        // **And the SOURCE is now the environment the DRIVER hands the child,
        // which is `T-19-90`.** `build_env_in` is one seam short of the child:
        // the driver appends `cred::RUN_ID_ENV` afterwards through
        // `with_run_id`, at the seam that hands the environment to the spawn
        // closure. A pin sourced from the bare constructor could not see that
        // entry however wide its floors were, so `GSD_MM_RUN_ID` was carried to
        // every driven child while being outside the pin's reach entirely.
        // Floor 5 below is what turns this test red if the source is ever
        // narrowed back.
        let envelope = tempfile::TempDir::new().unwrap();
        let project = tempfile::TempDir::new().unwrap();
        let env = crate::envelope::cred::build_env_in(
            envelope.path(),
            "alpha",
            project.path(),
            std::path::Path::new("/opt/gsd-meta-manager"),
        )
        .expect("the fixture environment builds")
        .with_run_id("fixture-run-id");

        let carried: Vec<(String, bool)> = env
            .entries()
            .iter()
            .map(|(name, value)| (name.to_string_lossy().into_owned(), value.is_none()))
            .collect();

        // --- floor 1: the entry set exists at all ------------------------
        assert!(
            !carried.is_empty(),
            "an empty environment satisfies the coverage claim below having proved \
             nothing at all"
        );

        // --- floor 2: it is the real environment, not a stub -------------
        assert!(
            carried.len() >= 5,
            "the built child environment must actually carry the keys this pin is about, \
             or the coverage assertion proves nothing: {carried:?}"
        );

        // --- floor 3: REMOVALS are present, the direction the old pin was
        //     structurally blind in. `value.is_some()` hid exactly these.
        let removals: Vec<&String> = carried
            .iter()
            .filter(|(_, removed)| *removed)
            .map(|(name, _)| name)
            .collect();
        assert!(
            removals.len() >= 2,
            "the environment must carry at least two REMOVAL entries, or this pin cannot \
             tell that it is covering them. This floor exists because the deleted \
             `value.is_some()` filter made every removal invisible, which is how \
             `SSH_AUTH_SOCK` and `SSH_AGENT_PID` — the belt D-16 relies on — stayed out \
             of `ENVELOPE_ENV_KEYS` unnoticed (`T-19-82`). Got: {removals:?}"
        );

        // --- floor 4: coverage reaches beyond the old name filter --------
        assert!(
            carried.iter().any(|(name, _)| {
                !name.starts_with("GIT_")
                    && !name.starts_with("GH_")
                    && envelope_env_key(name).is_some()
            }),
            "at least one COVERED key must begin with neither `GIT_` nor `GH_`. This floor \
             exists because the deleted name filter would silently exclude a future \
             `GITHUB_TOKEN`, `GLAB_CONFIG_DIR` or `SSH_*` — and already excluded \
             `GSD_MM_ENVELOPE_PROJECT_ROOT`. Re-introducing that filter turns this red. \
             Carried: {carried:?}, covered set: {ENVELOPE_ENV_KEYS:?}"
        );

        // --- floor 5: the SOURCE reaches the child, not just the constructor
        //
        // `T-19-90`. This floor stands for a defect that no amount of widening
        // the COVERAGE could have caught: a pin whose source is a constructor
        // call one seam short of the child cannot see the entry that seam
        // appends, whatever names are in the constant. Re-sourcing this pin back
        // to a bare `build_env_in(...)` — dropping the `.with_run_id(...)` above
        // — turns this red.
        assert!(
            carried
                .iter()
                .any(|(name, _)| name == crate::envelope::cred::RUN_ID_ENV),
            "the pinned environment must carry `{}`, the entry the DRIVER appends through \
             `cred::EnvelopeEnv::with_run_id` after `build_env_in` returns. If this is \
             absent, the pin's source is one seam short of the environment the child \
             actually receives, and every coverage assertion below is being made about a \
             smaller set than the one that reaches `execve`. Carried: {carried:?}",
            crate::envelope::cred::RUN_ID_ENV
        );

        for (name, removed) in &carried {
            let how = if *removed { "REMOVED from" } else { "SET in" };
            assert!(
                envelope_env_key(name).is_some(),
                "`{name}` is {how} the driven child's environment by `cred::build_env_in` \
                 but is not covered by `ENVELOPE_ENV_KEYS`, so a command that reassigns or \
                 removes it is not refused. **Add the key to the constant.** Do NOT narrow \
                 this test, and in particular do NOT re-introduce either of the filters \
                 this pin used to carry — a `value.is_some()` filter hides every removal, \
                 and a `GIT_`/`GH_` name filter hides every key the envelope grows that is \
                 not called after git. Covered set: {ENVELOPE_ENV_KEYS:?}"
            );
        }
    }

    // =======================================================================
    // The REAL-GIT DRIFT PIN over the leading-option grammar constants
    //
    // **`GIT_GLOBAL_VALUE_OPTS` had no pin and no test reference of any kind
    // until this round** — audit 6 measured exactly two mentions of it in the
    // whole repository, its definition and its one use — and that is how it came
    // to be wrong in BOTH directions against the installed git: `--attr-source`
    // and `--shallow-file` missing, `--super-prefix` present and rejected.
    //
    // `ENVELOPE_ENV_KEYS` was given a pin sourced from the place that changes
    // when the fact changes, after being wrong twice (`T-19-82`, `T-19-90`).
    // There is no in-codebase source for git's option grammar, so the source
    // here is git ITSELF — consulted in a TEST and never in the guard, because
    // the guard runs synchronously on the agent's `PreToolUse` critical path and
    // because a guard that asks the program it guards to describe its own
    // grammar can be lied to by a `git` earlier on `PATH`.
    //
    // # THE PIN'S OWN LIMIT, STATED CORRECTLY RATHER THAN COMFORTABLY
    //
    // **It pins the constants against the DEVELOPER's git, not the runtime git,
    // and the fail-closed default covers only the SILENT case.**
    //
    // A MISSING bit costs a refusal: a spelling in neither constant falls to
    // `LeadingOptionGrammar::NotEstablished` and the command is refused. That
    // direction is safe by construction.
    //
    // **A WRONG bit costs a SHIFTED VERB, and nothing in the guard covers it.**
    // A self-contained entry that a runtime git treats as value-taking, or a
    // value-taking entry that it rejects, makes `scan_leading` land short of or
    // step over the real verb — which is exactly the `--super-prefix` mechanism
    // measured at exit 0 this round. **So this pin is the ONLY control over the
    // wrong-bit direction**, and a constant that outruns the runtime git is a
    // BYPASS rather than an over-refusal.
    //
    // It must therefore NOT be written to skip when `git` is absent: a skipped
    // pin is a fail-open pin, and git is already a hard runtime dependency of
    // this guard.
    // =======================================================================

    /// The leading options no probe of the two-sided shape can classify, named so
    /// the unprobed set is BOUNDED rather than open.
    ///
    /// `git --help XVALUE version` answers `No manual entry for gitXVALUE`, which
    /// neither reaches a verb nor names the following word as a value, so neither
    /// arm of the probe fires. They are carried in NEITHER grammar constant and
    /// therefore fall to `LeadingOptionGrammar::NotEstablished` like anything else
    /// the guard cannot establish — the fail-closed answer rather than a guess.
    ///
    /// **It lives INSIDE this test module rather than beside the two constants,
    /// and that placement is load-bearing rather than tidy.** It is the pin's
    /// bound on the unprobed set and the guard never consults it; and
    /// `tests/envelope_wrapper_class.rs`'s anti-vacuity stripper treats the FIRST
    /// line reading `#[cfg(test)]` as the end of this file's production logic, so
    /// a `#[cfg(test)]` constant declared up beside `GIT_GLOBAL_VALUE_OPTS` would
    /// truncate that control's view of the file at line 600 and make every
    /// absence assertion in it vacuous. It caught exactly that when this constant
    /// was first written there.
    const GIT_GLOBAL_UNPROBED_OPTS: &[&str] = &["--help", "-h"];

    #[test]
    fn the_guards_own_path_shells_out_to_nothing() {
        // **The compensating control for this file's `SPAWN_ALLOWLIST` entry, and
        // it is STRICTER than the entry it replaces.**
        //
        // `tests/spawn_seam_guard.rs` asserts that no file under `src/` outside a
        // declared allowlist contains a process-spawn site. The drift pin below
        // needs one — it asks the installed `git` to classify its own option
        // grammar — so this file was added to that allowlist, which makes the
        // control stop looking at this file ENTIRELY. That is a file-level
        // permission for a test-only need, so the property it gives up is
        // re-asserted here at the granularity that actually matters.
        //
        // **The property: the GUARD's path shells out to nothing.** It runs
        // synchronously on the agent's `PreToolUse` critical path, where a
        // reproduced 180-240 second hang is the reason `push_needs_resolved_dests`
        // exists at all; and a guard that asks the program it is guarding to
        // describe its own grammar can be lied to by a `git` earlier on `PATH` —
        // the same surface this phase's own argv-printing shims demonstrate. The
        // probe therefore lives in a TEST and nowhere else.
        const SELF: &str = include_str!("policy.rs");
        const SPAWN_MARKERS: &[&str] = &["Command::new(", "process_group("];

        // Everything above the FIRST `#[cfg(test)]` line is the production half.
        // This is the same sentinel `tests/envelope_wrapper_class.rs`'s
        // anti-vacuity stripper uses, and the test module is the last item here.
        let production: String = SELF
            .lines()
            .take_while(|line| line.trim_start() != "#[cfg(test)]")
            .map(|line| format!("{line}\n"))
            .collect();

        // POSITIVE CONTROLS, so an absence assertion cannot pass because the
        // stripper ate the file or the marker was never findable.
        assert!(
            SELF.contains("Command::new("),
            "the raw source of this file must contain `Command::new(` — the drift pin's own \
             probe uses it. If it does not, this control is asserting the absence of a \
             string that was never there and certifies nothing."
        );
        assert!(
            production.contains("fn scan_leading"),
            "the production half must contain `fn scan_leading`. If it does not, the \
             `#[cfg(test)]` sentinel matched too early and every assertion below is being \
             made about a truncated string."
        );
        assert!(
            production.len() >= 40_000,
            "the production half must be substantially the whole guard. Got {} bytes.",
            production.len()
        );

        for marker in SPAWN_MARKERS {
            let offenders: Vec<&str> = production
                .lines()
                .filter(|line| !line.trim_start().starts_with("//"))
                .filter(|line| line.contains(marker))
                .collect();
            assert!(
                offenders.is_empty(),
                "`{marker}` appears in the PRODUCTION half of `policy.rs`. The guard must \
                 never spawn a process: it runs synchronously on `PreToolUse`, and a guard \
                 that asks the program it guards to describe its own grammar can be lied to \
                 by a binary earlier on `PATH`. Move it into the `#[cfg(test)]` module or \
                 delete it; do NOT relax this control, and in particular do not rely on \
                 this file's `SPAWN_ALLOWLIST` entry, which exists only for the drift pin. \
                 Offending lines: {offenders:?}"
            );
        }
    }

    /// Run one probe against the installed `git` and return stdout+stderr.
    ///
    /// Panics rather than skipping when `git` is missing — see the section
    /// comment above for why a skip here would be a fail-open pin.
    fn git_grammar_probe(dir: &std::path::Path, args: &[&str]) -> String {
        let output = std::process::Command::new("git")
            .args(args)
            .current_dir(dir)
            .env("GSD_MM_GRAMMAR_PROBE", "probe-value")
            .env("GIT_TERMINAL_PROMPT", "0")
            .output()
            .expect(
                "the leading-option grammar drift pin requires a real `git` on PATH. It is \
                 deliberately NOT written to skip when git is absent: a skipped pin is a \
                 fail-open pin, and git is already a hard runtime dependency of this guard.",
            );
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        text
    }

    fn probe_first_line(text: &str) -> &str {
        text.lines().next().unwrap_or("")
    }

    /// The value the TWO-WORD arm of the probe hands one option.
    ///
    /// Three options need a well-formed value rather than an arbitrary word, and
    /// each variant is named with the reason it is a variant rather than an
    /// exception: `-c` needs a config assignment, `-C` needs an existing
    /// directory, and `--config-env` needs a `KEY=ENVVAR` pair naming a variable
    /// this probe sets. Their ONE-WORD arms still discriminate — `git -C version`
    /// answering `cannot change to 'version'` IS the proof that `-C` swallowed
    /// the word.
    fn grammar_probe_value(option: &str, scratch: &std::path::Path) -> String {
        match option {
            "-c" => "probe.key=probe-value".to_string(),
            "-C" => scratch.display().to_string(),
            "--config-env" => "probe.key=GSD_MM_GRAMMAR_PROBE".to_string(),
            _ => "XVALUE".to_string(),
        }
    }

    #[test]
    fn every_leading_git_option_the_guard_calls_value_taking_really_consumes_the_next_word() {
        let scratch = tempfile::TempDir::new().unwrap();

        // --- floor 1: the constant carries the six spellings the corpus's
        //     class-1 alphabet splices BY NAME (`-C`, `--git-dir`,
        //     `--work-tree`, `--namespace`, `--attr-source`, `--shallow-file`),
        //     so six is the arithmetic and not a round number. `-c` and
        //     `--config-env` are two more, handled by their own arms in
        //     `leading_git_option` before the constant is consulted.
        assert!(
            GIT_GLOBAL_VALUE_OPTS.len() >= 6,
            "`GIT_GLOBAL_VALUE_OPTS` must carry at least six spellings — the six \
             `tests/envelope_wrapper_class.rs`'s class-1 alphabet splices by name. The \
             correct response to this failing is to RESTORE the entries, never to lower \
             the floor. Got: {GIT_GLOBAL_VALUE_OPTS:?}"
        );

        // --- floor 2: the two entries `T-19-100` is about are present BY NAME,
        //     and the stale entry that was a bypass is absent BY NAME.
        for required in ["--attr-source", "--shallow-file"] {
            assert!(
                GIT_GLOBAL_VALUE_OPTS.contains(&required),
                "`{required}` must be in `GIT_GLOBAL_VALUE_OPTS`. Its absence is `T-19-100`: \
                 `git {required} <value> push --force origin main` exited 0 because the scan \
                 read the option's VALUE as the verb."
            );
        }
        assert!(
            !GIT_GLOBAL_VALUE_OPTS.contains(&"--super-prefix"),
            "`--super-prefix` must NOT be in `GIT_GLOBAL_VALUE_OPTS`. The installed git \
             answers `unknown option: --super-prefix` in both probe forms, and a stale \
             entry here is fail-open in the OVER-consuming direction — measured, \
             `git --super-prefix push --force origin main` exited 0 because the scan \
             swallowed the real verb `push` as the option's value."
        );

        // --- floor 3: the NEGATIVE control, so a probe that answered the same
        //     for everything turns this red rather than passing vacuously.
        let bogus_one = git_grammar_probe(scratch.path(), &["--bogus-opt", "version"]);
        let bogus_two = git_grammar_probe(scratch.path(), &["--bogus-opt", "XVALUE", "version"]);
        assert!(
            bogus_one.contains("unknown option") && bogus_two.contains("unknown option"),
            "the probe must classify `--bogus-opt` as NOT ACCEPTED by this git. If it does \
             not, the probe is not reading git's answer and every classification below is \
             vacuous. 1W: {bogus_one} 2W: {bogus_two}"
        );
        assert!(
            !probe_first_line(&bogus_two).starts_with("git version"),
            "the probe must not report a rejected option as reaching the verb. 2W: {bogus_two}"
        );

        // --- the pin itself, two-sided, over EVERY entry.
        for option in GIT_GLOBAL_VALUE_OPTS {
            let value = grammar_probe_value(option, scratch.path());
            let one_word = git_grammar_probe(scratch.path(), &[option, "version"]);
            let two_word = git_grammar_probe(scratch.path(), &[option, &value, "version"]);

            assert!(
                !one_word.contains(&format!("unknown option: {option}")),
                "the installed git REJECTS `{option}`, which `GIT_GLOBAL_VALUE_OPTS` claims \
                 consumes a following word. A stale entry here is fail-open in the \
                 OVER-consuming direction and is a BYPASS rather than an over-refusal: the \
                 scan skips a word git itself reads as the verb. **Remove the entry.** \
                 Probe answered: {one_word}"
            );
            assert!(
                probe_first_line(&two_word).starts_with("git version"),
                "`git {option} <value> version` must reach the `version` verb, which is the \
                 proof that `{option}` consumed its value. It did not, so this entry claims \
                 a grammar the installed git does not have. Probe answered: {two_word}"
            );
            assert!(
                !probe_first_line(&one_word).starts_with("git version"),
                "`git {option} version` must NOT reach the `version` verb: if it does, \
                 `{option}` did not consume the following word and it belongs in \
                 `GIT_GLOBAL_SELF_CONTAINED_OPTS` instead. Probe answered: {one_word}"
            );
        }
    }

    #[test]
    fn every_leading_git_option_the_guard_calls_self_contained_really_consumes_nothing() {
        let scratch = tempfile::TempDir::new().unwrap();

        // --- floor 1: the seven spellings other files pin BY NAME must be
        //     present — five from `tests/envelope_wrapper_class.rs`'s class-2
        //     alphabet and two more from `tests/envelope_callee_grammar.rs`'s
        //     permitted half — and the floor is set one above that count, so a
        //     constant shrunk to exactly the pinned set is still visible.
        for required in [
            "--no-pager",
            "--bare",
            "--literal-pathspecs",
            "--no-optional-locks",
            "-p",
            "--exec-path",
            "--version",
        ] {
            assert!(
                GIT_GLOBAL_SELF_CONTAINED_OPTS.contains(&required),
                "`{required}` must be in `GIT_GLOBAL_SELF_CONTAINED_OPTS`. It is pinned by \
                 name in the corpus, and without it `git {required} status` falls to \
                 *grammar not established* and is refused — an over-refusal of an ordinary \
                 invocation, which is how a safety control gets switched off (AR-19-11)."
            );
        }
        assert!(
            GIT_GLOBAL_SELF_CONTAINED_OPTS.len() >= 8,
            "`GIT_GLOBAL_SELF_CONTAINED_OPTS` must carry at least eight spellings: the \
             SEVEN asserted by name above plus at least one more, because a constant shrunk \
             to exactly the pinned set has silently lost the rest of the probed family and \
             every loss is a new over-refusal. RESTORE the entries; do not lower the floor. \
             Got: {GIT_GLOBAL_SELF_CONTAINED_OPTS:?}"
        );

        // --- floor 2: the NEGATIVE control — a value-taking spelling must NOT
        //     satisfy the self-contained probe, or the two arms are the same
        //     question asked twice.
        let taking_one = git_grammar_probe(scratch.path(), &["--attr-source", "version"]);
        assert!(
            !probe_first_line(&taking_one).starts_with("git version"),
            "`--attr-source` must FAIL the self-contained probe. If a value-taking option \
             satisfied it, the two constants would be describing the same thing and the \
             grammar question would not be being asked. Probe answered: {taking_one}"
        );

        for option in GIT_GLOBAL_SELF_CONTAINED_OPTS {
            let one_word = git_grammar_probe(scratch.path(), &[option, "version"]);
            let two_word = git_grammar_probe(scratch.path(), &[option, "XVALUE", "version"]);

            assert!(
                !one_word.contains(&format!("unknown option: {option}")),
                "the installed git REJECTS `{option}`, which `GIT_GLOBAL_SELF_CONTAINED_OPTS` \
                 claims it accepts. A stale entry here is an over-refusal in one direction \
                 and a shifted verb in the other. **Remove the entry.** Probe: {one_word}"
            );

            // The BOOLEAN arm: git reaches `version`, and the extra word is then
            // read as a verb git does not have.
            let boolean = probe_first_line(&one_word).starts_with("git version")
                && two_word.contains("is not a git command");
            // The TERMINATING arm: `--exec-path` and friends run no verb at all,
            // so the proof is that a following word changes NOTHING.
            let terminating = !probe_first_line(&one_word).is_empty()
                && probe_first_line(&one_word) == probe_first_line(&two_word);

            assert!(
                boolean || terminating,
                "`{option}` satisfies NEITHER arm of the self-contained probe, so the guard's \
                 claim that it occupies exactly one word is not backed by the installed git. \
                 A wrong bit here SHIFTS THE VERB — the `--super-prefix` mechanism — so this \
                 is not a cosmetic disagreement. 1W: {one_word} 2W: {two_word}"
            );
        }
    }

    #[test]
    fn the_two_leading_option_constants_are_disjoint_and_the_unprobed_set_is_bounded_and_named() {
        for option in GIT_GLOBAL_VALUE_OPTS {
            assert!(
                !GIT_GLOBAL_SELF_CONTAINED_OPTS.contains(option),
                "`{option}` is in BOTH grammar constants. The two answer opposite halves of \
                 the one question this scan asks — does the option consume the next word — \
                 so a spelling in both means the arms of `leading_git_option` decide by \
                 their ORDER rather than by the measurement."
            );
        }

        // The UNPROBED set is BOUNDED and both members are NAMED, so "no probe of
        // this shape classifies it" cannot quietly grow into a third list.
        assert!(
            GIT_GLOBAL_UNPROBED_OPTS.len() <= 2,
            "at most TWO leading options may be carried as UNPROBED. `--help` and `-h` are \
             the two: `git --help XVALUE version` answers `No manual entry for gitXVALUE`, \
             which neither reaches a verb nor names the following word as a value, so \
             neither arm of the two-sided probe fires. A third entry means a spelling was \
             put beyond the probe's reach rather than measured. Got: \
             {GIT_GLOBAL_UNPROBED_OPTS:?}"
        );
        assert_eq!(
            GIT_GLOBAL_UNPROBED_OPTS,
            &["--help", "-h"],
            "the UNPROBED set must be exactly the two spellings named in its own doc"
        );
        for option in GIT_GLOBAL_UNPROBED_OPTS {
            assert!(
                !GIT_GLOBAL_VALUE_OPTS.contains(option)
                    && !GIT_GLOBAL_SELF_CONTAINED_OPTS.contains(option),
                "`{option}` is recorded UNPROBED but is carried by a grammar constant, which \
                 means a bit nothing measured was written down anyway. An unprobed spelling \
                 takes the fail-closed path instead."
            );
        }
    }

    /// A git repository in `scratch`, because `includeIf.gitdir:` matches against
    /// a real `.git` directory and answers nothing without one.
    fn config_resolution_scratch_repo(scratch: &std::path::Path) {
        let out = std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(scratch)
            .output()
            .expect(
                "the config-resolution pin requires a real `git` on PATH. It is deliberately \
                 NOT written to skip when git is absent: a skipped pin is a fail-open pin.",
            );
        assert!(
            out.status.success(),
            "`git init` failed in the pin's scratch directory, so no probe below can be \
             trusted: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    /// Resolve `core.hooksPath` under **the envelope's own injection**, plus
    /// whatever leading arguments and environment the caller is measuring.
    ///
    /// The control is [`crate::envelope::cred::hooks_path_env`] itself rather than a
    /// hand-written triplet, so the pin measures the mechanism the envelope
    /// actually emits — including its DERIVED `GIT_CONFIG_COUNT` — and not a
    /// stand-in that could drift away from it silently.
    fn resolves_hooks_path_under_injection(
        scratch: &std::path::Path,
        leading: &[&str],
        extra_env: &[(&str, &str)],
    ) -> String {
        let mut args: Vec<&str> = leading.to_vec();
        args.extend_from_slice(&["config", "--get", "core.hooksPath"]);

        let mut command = std::process::Command::new("git");
        command.args(&args).current_dir(scratch);
        for (key, value) in
            crate::envelope::cred::hooks_path_env(std::path::Path::new("/ENV_WINS"))
        {
            command.env(key, value);
        }
        for (key, value) in extra_env {
            command.env(key, value);
        }
        let output = command.output().expect(
            "the config-resolution pin requires a real `git` on PATH. It is deliberately NOT \
             written to skip when git is absent: a skipped pin is a fail-open pin, and git is \
             already a hard runtime dependency of this guard.",
        );
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        text.trim().to_string()
    }

    #[test]
    fn every_indirection_section_the_guard_names_really_outranks_the_envelopes_own_injection() {
        // =======================================================================
        // **THE DIRECTION THIS PIN HOLDS, STATED BEFORE ANYTHING ELSE.**
        //
        // It holds the REVERSE direction only: it turns red if the installed git
        // stops honouring a section `INDIRECTION_SECTIONS` already NAMES. **It
        // cannot observe a section it does not name, because it iterates that
        // constant's entries and an entry that does not exist is never probed.**
        //
        // **It is therefore NOT a control over the fail-open residue.** A future
        // git that adds a THIRD indirection section reaches `core.hooksPath`
        // silently until a human adds it to the constant, and there is NO
        // automated control over that direction. Saying otherwise here would be
        // false reassurance in a control's own doc — `T-19-107`'s failure mode,
        // committed inside the round that registers `T-19-107`.
        // =======================================================================
        let scratch = tempfile::TempDir::new().unwrap();
        config_resolution_scratch_repo(scratch.path());

        let include_file = scratch.path().join("inc.cfg");
        std::fs::write(&include_file, "[core]\n\thooksPath = /INCLUDE_WINS\n").unwrap();
        let include_path = include_file.display().to_string();
        let repo_prefix = format!("{}/", scratch.path().display());

        // --- the NON-VACUITY CONTROL. Without it, every "the include wins"
        //     assertion below could be passing on a probe that never observed
        //     the envelope's injection at all.
        let control = resolves_hooks_path_under_injection(scratch.path(), &[], &[]);
        assert_eq!(
            control, "/ENV_WINS",
            "the envelope's own `hooks_path_env` triplet must resolve `core.hooksPath` to its \
             injected value with no carrier present. If it does not, this probe is not \
             observing the injection and EVERY assertion in this pin is vacuous. Got: {control}"
        );

        // --- floor 1: at least TWO indirection sections are named. The
        //     arithmetic: git 2.43.0 splices configuration in from elsewhere
        //     through exactly `include` and `includeIf`, so two is the measured
        //     count and not a round number. The correct response to this failing
        //     is to RESTORE the measured section, never to lower the floor.
        assert!(
            INDIRECTION_SECTIONS.len() >= 2,
            "`INDIRECTION_SECTIONS` must name at least the TWO sections through which git \
             splices configuration in from a file — `include` and `includeIf`. A constant \
             short of that is a rule that fails OPEN on a carrier this git honours. RESTORE \
             the entry; do not lower the floor. Got: {INDIRECTION_SECTIONS:?}"
        );

        // --- the pin itself: every NAMED section really outranks the injection.
        //     This is what makes the rule a MEASUREMENT rather than a claim.
        for section in INDIRECTION_SECTIONS {
            // Both sections are probed in the spelling git actually accepts:
            // `include` takes no subsection, `includeIf` requires a condition.
            let assignment = if section.eq_ignore_ascii_case("includeIf") {
                format!("{section}.gitdir:{repo_prefix}.path={include_path}")
            } else {
                format!("{section}.path={include_path}")
            };
            let resolved =
                resolves_hooks_path_under_injection(scratch.path(), &["-c", &assignment], &[]);
            assert_eq!(
                resolved, "/INCLUDE_WINS",
                "`git -c {assignment}` must OUTRANK the envelope's own injected \
                 `core.hooksPath`. `INDIRECTION_SECTIONS` names `{section}` as a section that \
                 splices configuration in from a file, and `scan_leading` refuses a command \
                 carrying one on exactly that basis. If this git no longer honours it, the \
                 refusal has become an over-refusal with no hazard behind it and the entry \
                 must be RE-MEASURED — this is the one direction this pin holds. Got: {resolved}"
            );

            // And the guard's own helper agrees about the same spelling, so the
            // constant and the rule cannot drift apart.
            let key = config_key_of(&assignment);
            assert!(
                config_key_names_an_indirection_section(key),
                "`{key}` resolves the include against real git but the guard's own helper \
                 answers CONFINED for it. The measurement and the rule have drifted apart."
            );
        }

        // --- the CASE fact, measured in BOTH halves of the key. Git folds the
        //     SECTION and the VARIABLE and leaves the SUBSECTION case-sensitive,
        //     which is why the helper compares the section with
        //     `eq_ignore_ascii_case` and reads the subsection not at all.
        let upper_simple = resolves_hooks_path_under_injection(
            scratch.path(),
            &["-c", &format!("INCLUDE.PATH={include_path}")],
            &[],
        );
        assert_eq!(
            upper_simple, "/INCLUDE_WINS",
            "`-c INCLUDE.PATH=<file>` must resolve the include: git folds the SECTION and the \
             VARIABLE to lower case. A guard comparing the section case-SENSITIVELY would miss \
             this carrier entirely. Got: {upper_simple}"
        );
        let upper_subsectioned = resolves_hooks_path_under_injection(
            scratch.path(),
            &[
                "-c",
                &format!("INCLUDEIF.gitdir:{repo_prefix}.PATH={include_path}"),
            ],
            &[],
        );
        assert_eq!(
            upper_subsectioned, "/INCLUDE_WINS",
            "`-c INCLUDEIF.gitdir:<p>.PATH=<file>` must resolve the include too — the fold \
             covers the section and the variable while the SUBSECTION between them stays \
             case-sensitive, which is the half the helper deliberately never reads. \
             Got: {upper_subsectioned}"
        );

        // --- `--config-env` is the SAME key reached through a second carrier,
        //     and `leading_git_option`'s arms hand both to the same key check.
        let via_env = resolves_hooks_path_under_injection(
            scratch.path(),
            &["--config-env=include.path=GSD_MM_INCLUDE_PROBE"],
            &[("GSD_MM_INCLUDE_PROBE", include_path.as_str())],
        );
        assert_eq!(
            via_env, "/INCLUDE_WINS",
            "`--config-env=include.path=<VAR>` must resolve the include as `-c` does. Both \
             carriers reach the same key check in `scan_leading`, so a difference here would \
             mean one of them is unmeasured. Got: {via_env}"
        );

        // --- floor 2: at least FOUR CONFINED negative cases, and they must NOT
        //     outrank. Without these, an assertion that indirections outrank
        //     would pass on a probe that reported "outranks" for everything.
        //     `includepath` and `notinclude.path` are omitted from the RESOLUTION
        //     half deliberately: real git refuses to PARSE a dotless key
        //     (`error: key does not contain a section: includepath`), so they are
        //     measured by the helper below instead of by resolution.
        let confined: &[&str] = &[
            "user.name=x",
            "core.pager=cat",
            "include.pathx=/tmp/evil.cfg",
            "core.editor=true",
        ];
        assert!(
            confined.len() >= 4,
            "at least FOUR confined carriers must be probed, so this arm cannot be satisfied \
             by a single lucky row. Got: {confined:?}"
        );
        for assignment in confined {
            let resolved =
                resolves_hooks_path_under_injection(scratch.path(), &["-c", assignment], &[]);
            assert_eq!(
                resolved, "/ENV_WINS",
                "`git -c {assignment}` must LEAVE the envelope's injected `core.hooksPath` in \
                 place. If a confined carrier outranked it, this pin would be reporting \
                 `outranks` for everything and the discrimination the rule rests on would not \
                 exist. Got: {resolved}"
            );
        }

        // `include.pathx` is the DISCLOSED COST, and it is pinned from both
        // sides in one place: real git IGNORES it (above), and the guard REFUSES
        // it (below), because the rule reads the section and deliberately not the
        // variable. The correct response to a red here is NOT to start reading
        // the variable — that re-opens `includeIf`'s condition family.
        assert!(
            config_key_names_an_indirection_section("include.pathx"),
            "`include.pathx` must be refused by the guard even though real git ignores it. \
             That is this rule's whole measured over-refusal cost and it falls in the safe \
             direction."
        );
    }

    #[test]
    fn the_section_helper_reads_the_section_and_neither_the_subsection_nor_the_variable() {
        // The rule's own behaviour, over both halves of its answer. It needs no
        // git: it is a statement about the guard, and the git-side facts it rests
        // on are measured by the pin above.
        for key in [
            "include.path",
            "includeIf.gitdir:/x/.path",
            "INCLUDE.PATH",
            // The VARIABLE is not read, so a variable git ignores is still an
            // indirection SECTION. This is the disclosed cost.
            "include.pathx",
            // `config_key_of` returns a valueless assignment whole, and
            // `git -c include.path` with no value is still a write of that key.
            "include.path",
            // The SUBSECTION is not read, so an `includeIf` condition family git
            // has not invented yet is covered by construction.
            "includeIf.hasconfig:remote.*.url:https://x/.path",
        ] {
            assert!(
                config_key_names_an_indirection_section(key),
                "`{key}` names an INDIRECTION section and must answer true. The rule compares \
                 the text before the FIRST `.` against `INDIRECTION_SECTIONS` with \
                 `eq_ignore_ascii_case` and reads nothing else."
            );
        }

        for key in [
            // A SUBSTRING rule lands red on these two, which is why they are here:
            // `git -c includepath=…` and `git -c notinclude.path=…` are both
            // measured PERMITTED and pinned at exit 0 in the corpus.
            "includepath",
            "notinclude.path",
            // DOTLESS: no `.`, so no section, so provably not an indirection.
            // Refusing it would turn round 7's entire callee-grammar generative
            // property permanently red behind `CALLEE_KNOWN_LEADING_PREFIX`.
            "a",
            "user.name",
            // And the hooks-path key answers FALSE, because it must fall through
            // to `is_hooks_path_key` and earn `HookBypassBlocked` rather than
            // being absorbed by this clause.
            "core.hooksPath",
        ] {
            assert!(
                !config_key_names_an_indirection_section(key),
                "`{key}` is CONFINED and must answer false. A rule written as a substring test \
                 on `include` lands red on `includepath` and `notinclude.path`; a rule that \
                 refused a key it cannot decompose into a section lands red on `a`; and a rule \
                 that absorbed `core.hooksPath` would attribute its refusal to a mechanism \
                 that did not produce it (D-24)."
            );
        }

        // The section split itself is git's own rule — the FIRST `.` — and a key
        // with none names no section at all.
        assert_eq!(config_key_section("includeIf.gitdir:/x/.path"), Some("includeIf"));
        assert_eq!(config_key_section("include.path"), Some("include"));
        assert_eq!(
            config_key_section("a"),
            None,
            "a key with no `.` names NO section. That is a positive fact about the key — real \
             git answers `error: key does not contain a section: a` — and not a parse failure."
        );
    }

    #[test]
    fn every_push_value_opt_really_consumes_its_value_and_signed_is_the_control_that_proves_it() {
        // **The over-consuming direction of `PUSH_VALUE_OPTS`, which
        // `push_operands` deliberately does NOT fail closed on.** An entry git
        // does not treat as value-taking makes the guard skip a word git reads as
        // a refspec — the same fail-open shape `GIT_GLOBAL_VALUE_OPTS` had — and
        // this pin is what covers it instead of a fail-closed default.
        //
        // The probe is two-sided by CONSTRUCTION: each option is spliced ahead of
        // a real repository operand and a refspec. If the option consumed
        // `XVALUE`, the repository is the upstream path and the refspec is
        // `refs/heads/probe`. If it did NOT, `XVALUE` becomes the repository and
        // the upstream PATH is read as a refspec — which git reports as
        // `invalid refspec '<upstream>'`. That string is the discriminator.
        let root = tempfile::TempDir::new().unwrap();
        let upstream = root.path().join("upstream.git");
        let work = root.path().join("work");
        std::fs::create_dir_all(&upstream).unwrap();
        std::fs::create_dir_all(&work).unwrap();
        git_grammar_probe(&upstream, &["init", "--bare", "-q", "."]);
        git_grammar_probe(&work, &["init", "-q", "."]);
        let upstream_arg = upstream.display().to_string();
        let not_consumed = format!("invalid refspec '{upstream_arg}'");

        let probe_push = |flag: &str| -> String {
            git_grammar_probe(
                &work,
                &[
                    "push",
                    "--dry-run",
                    flag,
                    "XVALUE",
                    &upstream_arg,
                    "refs/heads/probe",
                ],
            )
        };

        // --- the NEGATIVE controls, which make the assertion below non-vacuous.
        //
        // `--signed` is the control `T-19-102` turns on. `git push -h` spells
        // `--recurse-submodules (check|on-demand|no)` and
        // `--signed[=(yes|no|if-asked)]` — a REQUIRED value and an ATTACHED-ONLY
        // optional one — and the help text makes them look alike while git
        // treats them differently. Adding `signed` to the constant would
        // introduce a real MIS-PARSE, so it is pinned absent here and its push
        // is pinned REFUSED in `tests/envelope_callee_grammar.rs`.
        for control in ["--signed", "--dry-run"] {
            let answered = probe_push(control);
            assert!(
                answered.contains(&not_consumed),
                "`{control}` must NOT consume a following word on the installed git — if \
                 this probe cannot tell a value-taking push flag from a boolean one, every \
                 assertion below passes vacuously. Probe answered: {answered}"
            );
        }
        assert!(
            !PUSH_VALUE_OPTS.contains(&"signed"),
            "`signed` must NOT be in `PUSH_VALUE_OPTS`. `--signed[=(yes|no|if-asked)]` takes \
             an ATTACHED-ONLY optional value and real git reads the following word as the \
             REPOSITORY, so adding it would make the guard skip a word git reads as an \
             operand. Completing this list from `git push -h` is exactly the move this \
             control exists to catch."
        );
        assert!(
            PUSH_VALUE_OPTS.contains(&"recurse-submodules"),
            "`recurse-submodules` must be in `PUSH_VALUE_OPTS`: `--recurse-submodules \
             on-demand` takes a REQUIRED separate value, and without the entry the guard \
             reads `on-demand` as the repository and FALSELY REFUSES an ordinary \
             in-namespace push (`T-19-102`)."
        );

        for name in PUSH_VALUE_OPTS {
            let flag = format!("--{name}");
            let answered = probe_push(&flag);
            assert!(
                !answered.contains(&not_consumed),
                "`{flag}` does NOT consume its value on the installed git — the upstream \
                 path was read as a REFSPEC, which means `XVALUE` became the repository. \
                 `PUSH_VALUE_OPTS` claiming otherwise makes `push_operands` skip a word git \
                 reads as an operand. Probe answered: {answered}"
            );
        }
    }
}
