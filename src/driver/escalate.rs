//! The model seam's per-run budget: how often a run may consult the model, and
//! what it does when the answer is unusable.
//!
//! **This module performs no I/O and reads no clock**, in the same register and
//! for the same reason as [`super::bounds`], [`super::router`],
//! [`super::rate_limit`] and [`super::goal`]: the caller makes the model call,
//! retains what it retains, and hands in the values judged here. That is what
//! makes the cap's boundary testable one either side without a spawn, a network
//! or a run directory anywhere in the test.
//!
//! # A fifth sibling taxonomy
//!
//! [`EscalationReason`] is a **fifth sibling** beside `crate::envelope::policy`'s
//! `ParkReason`, [`super::router::RouterReason`], [`super::bounds::BoundsReason`]
//! and [`super::rate_limit::QuotaReason`] — never an extension of any of them,
//! and in particular **not a fifth `BoundsReason` arm**. `rate_limit.rs:26-33`
//! records this identical question being asked and answered for the quota park,
//! and the axis here is the same kind of axis:
//!
//! CTRL-06's four bounds are facts about **whether the run is making progress**.
//! An escalation count is a fact about **how much the model was consulted**. A
//! run can burn its whole escalation budget while making excellent progress, and
//! a stalled run can burn none of it. They answer different questions, they are
//! read by different people for different reasons, and they want different
//! prefixes on disk. All five reach a reader through the one
//! `JournalEvent::Parked` record and the one `parked:` terminal label; the table
//! beside that field's declaration names all five, because its own text says
//! naming only some of them would be a quiet lie.
//!
//! # The payload this classifies is untrusted input
//!
//! It arrives from a process that itself consumed untrusted repository content —
//! a third party's `.planning/`, their `ROADMAP.md`, their `CLAUDE.md` — so
//! everything here is tolerant parsing, on the same terms as
//! `src/executor/stream_json.rs:14-18` and [`super::rate_limit`]: no strict
//! unknown-field rejection, no panicking accessor, and no `unwrap` on a wire
//! field. A value this build does not recognise is carried verbatim rather than
//! mapped or dropped, and the distinction between "the seam named something we
//! do not accept" and "there was nothing to name" is preserved by the type
//! ([`NamedAction`]), exactly as [`super::rate_limit::QuotaWindow::Unknown`]
//! preserves it for a quota window.
//!
//! **A named-but-refused action is evidence, never a command.** It is recorded so
//! a later reader can see that a repository asked for something outside the
//! alphabet; it is never interpolated into a command string, a shell line or a
//! pasteable preview. `super::router::RouterAction::command_for` is private for
//! precisely that reason, and its doc records WR-09 as what it cost the last time
//! an unvalidated token reached a rendering path.

use std::fmt;

/// The per-run escalation budget was spent, so the run parks.
pub const REASON_ESCALATION_CAP_REACHED: &str = "escalation_cap_reached";
/// The seam named an action outside `super::router::SAFE_COMMAND_ALPHABET`.
pub const REASON_ESCALATION_ACTION_REFUSED: &str = "escalation_action_refused";
/// The seam produced nothing usable: no structured payload, a malformed one, or
/// the CLI's own structured-output retry loop exhausted.
pub const REASON_ESCALATION_OUTPUT_UNUSABLE: &str = "escalation_output_unusable";

/// What a refused action is called when there was nothing to name.
///
/// A word rather than an omission, for [`super::rate_limit::UNKNOWN`]'s reason: a
/// record that simply left the field out would be indistinguishable from one
/// written by a build that never recorded it.
pub const UNNAMED: &str = "unnamed";

/// How many model consultations one run may make unless argv says otherwise.
///
/// **Three: the goal decomposition, plus two genuine ambiguities.** A driven run
/// consults the model once at the start to turn a goal into a plan over the
/// router's alphabet, and thereafter only where the deterministic router returns
/// `router::REASON_NO_RULE` — a state that is rare by construction, because the
/// rule table is the thing that decides and the seam is the thing that is asked
/// when the table cannot. A run that needs a fourth consultation is a run whose
/// state the rule table does not describe, and the honest response to that is to
/// park for a human rather than to keep asking.
///
/// It is a **default and not a ceiling**: [`resolve`] refuses a supplied cap that
/// could never bind, which is what makes "there is no disablement" a property of
/// the parser rather than of the caller's restraint.
pub const DEFAULT_MAX_ESCALATIONS: u32 = 3;

/// Why a run parked on the model seam.
///
/// A **sibling** of `crate::envelope::policy::ParkReason`,
/// [`super::router::RouterReason`], [`super::bounds::BoundsReason`] and
/// [`super::rate_limit::QuotaReason`], never an extension of any of them; see the
/// module doc for the axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalationReason {
    /// The per-run cap was reached. The run parks rather than quietly
    /// continuing rules-only: a run that stopped consulting the model has
    /// materially changed what it is, and a change that large has to be readable
    /// off disk.
    CapReached,
    /// The seam named an action outside the safe alphabet.
    ActionRefused,
    /// The seam produced no usable answer — an absent structured payload, a
    /// malformed one, or the CLI's structured-output retry loop exhausted.
    OutputUnusable,
}

impl EscalationReason {
    /// The stable snake_case identifier a later reader greps for.
    ///
    /// One arm per variant, **no wildcard**: a new condition is a compile error
    /// here rather than a park that borrows somebody else's reason string.
    pub fn as_str(&self) -> &'static str {
        match self {
            EscalationReason::CapReached => REASON_ESCALATION_CAP_REACHED,
            EscalationReason::ActionRefused => REASON_ESCALATION_ACTION_REFUSED,
            EscalationReason::OutputUnusable => REASON_ESCALATION_OUTPUT_UNUSABLE,
        }
    }

    /// Every arm, in declaration order — the enumeration the both-directions
    /// guard walks.
    pub const ALL: &'static [EscalationReason] = &[
        EscalationReason::CapReached,
        EscalationReason::ActionRefused,
        EscalationReason::OutputUnusable,
    ];
}

/// What the seam named, when what it named was refused.
///
/// **The two arms are not the same state and must never collapse into one**, on
/// exactly [`super::rate_limit::QuotaWindow::Unknown`]'s terms: `Named` is "the
/// seam named something this build does not accept", which on a model seam is an
/// ordinary event and sometimes an attack; `Absent` is "there was nothing to
/// name" — an empty payload, a missing field, a non-object. A build that
/// flattened both into an empty string could not tell a hostile repository from
/// a transport that returned nothing.
///
/// **The type carries the observed string whole.** Only [`detail`](Self::detail),
/// the rendering into a record, is bounded and control-character-stripped — the
/// same split [`super::rate_limit::MAX_OBSERVED_WINDOW_CHARS`] and
/// [`super::untrusted::MAX_UNTRUSTED_FIELD_CHARS`] both document, and for the
/// same reason: the journal lands in `.planning/`, a directory users commit, and
/// every reader of it depends on one line per entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamedAction {
    /// The seam named this, verbatim and whole.
    Named(String),
    /// There was nothing to name.
    Absent,
}

impl NamedAction {
    /// The observed string, **whole and unmodified**, or `None` when there was
    /// nothing to name.
    ///
    /// Never build a command line, a shell string or a pasteable preview from
    /// this. It is evidence about a repository, and the one thing it may not
    /// become is an instruction.
    pub fn observed(&self) -> Option<&str> {
        match self {
            NamedAction::Named(value) => Some(value),
            NamedAction::Absent => None,
        }
    }

    /// How this is written into a journal record: bounded and control-character
    /// free, so one value cannot become two record lines.
    pub fn detail(&self) -> String {
        match self {
            NamedAction::Named(value) => super::untrusted::bounded(value),
            NamedAction::Absent => UNNAMED.to_string(),
        }
    }
}

/// Why a proposed escalation cap was refused before the run existed.
///
/// Each maps to one typed `DriveError` variant at the seam in `driver::drive`,
/// mirroring [`super::bounds::BoundsRefusal`] exactly: the refusal happens where
/// a bad value first arrives, not inside a `value_parser` wired to one of the
/// three paths that can supply it — a hand-typed invocation, the TUI's argv
/// builder, and a re-read run record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscalationRefusal {
    /// A supplied cap of zero: a seam that can never fire while still looking
    /// configured, which is the same defect [`super::bounds::resolve`] refuses
    /// for a zero step cap.
    ZeroCap,
    /// A supplied cap at or above the run's **resolved** step cap.
    ///
    /// Carries both numbers, because a refusal that named only one of them could
    /// not be acted on: the caller can lower the cap or raise the step cap, and
    /// which of those is right depends on what they were trying to bound.
    CapCannotBind {
        /// What was asked for, verbatim.
        cap: u32,
        /// The step cap `bounds::resolve` actually returned for this run — never
        /// [`super::bounds::DEFAULT_MAX_STEPS`].
        max_steps: u32,
    },
}

impl fmt::Display for EscalationRefusal {
    /// Each message names the flag, both numbers where there are two, **and**
    /// what to pass instead, because a refusal a caller cannot act on is a bug
    /// report rather than an error message.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EscalationRefusal::ZeroCap => write!(
                f,
                "`--max-escalations 0` is a seam that can never fire while still \
                 looking configured. Pass at least 1, or omit the flag and the run \
                 takes the default of {DEFAULT_MAX_ESCALATIONS}, reduced to what its \
                 step cap leaves room for"
            ),
            EscalationRefusal::CapCannotBind { cap, max_steps } => write!(
                f,
                "`--max-escalations {cap}` is at or above this run's resolved step \
                 cap of {max_steps}, so it can never bind: the run halts on its step \
                 cap first and the escalation budget is a disablement wearing a \
                 cap's clothing. Pass at most {}, or raise `--max-steps` above \
                 {cap}",
                max_steps.saturating_sub(1)
            ),
        }
    }
}

/// The model-consultation budget in force for one run, and what it has spent.
///
/// A value rather than two loose numbers, for [`super::bounds::RunBounds`]'
/// reason: the run records what bounded it, and a test constructs the boundary
/// case in one expression.
///
/// **The counter is checked BEFORE a consultation, never after** — see
/// [`permit_consultation`](Self::permit_consultation) for the tie-breaking
/// contract and why the direction matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EscalationBudget {
    /// How many consultations this run may make.
    cap: u32,
    /// How many it has been permitted so far.
    used: u32,
}

impl EscalationBudget {
    /// How many consultations this run may make.
    pub fn cap(&self) -> u32 {
        self.cap
    }

    /// How many it has been permitted so far.
    ///
    /// This is the number a run record reports, and it **never under-reports**:
    /// see [`permit_consultation`](Self::permit_consultation) for why it is
    /// incremented on the ask rather than on the answer.
    pub fn used(&self) -> u32 {
        self.used
    }

    /// How many are left.
    ///
    /// Saturating, so a budget whose counter was somehow pushed past its cap
    /// reports zero rather than wrapping to something enormous.
    pub fn remaining(&self) -> u32 {
        self.cap.saturating_sub(self.used)
    }

    /// Whether this run may escalate at all.
    ///
    /// A cap of zero is reachable only by derivation, never by supply:
    /// [`resolve`] refuses a supplied zero, and reduces an unsupplied default to
    /// what the step cap leaves room for. A one-step run therefore truthfully
    /// reports that it cannot escalate, rather than carrying a cap that pretends
    /// to bind.
    pub fn can_escalate(&self) -> bool {
        self.cap > 0
    }

    /// May the run consult the model once more? **Ask before consulting.**
    ///
    /// The tie-breaking contract, stated as the sentence its test was written
    /// from: **a run with cap N performs at most N consultations, and the
    /// consultation that would be number N+1 does not happen.** Checking after
    /// the fact would make the cap describe a consultation that has already
    /// occurred — the model has already been asked, the tokens are already
    /// spent, and the "cap" is a report rather than a control.
    ///
    /// The count is incremented **on the ask**, not on the answer. A permitted
    /// consultation that then fails to happen therefore still counts, which is
    /// the safe direction: the alternative under-reports how often the model was
    /// consulted, and a number that reads as a safety property while
    /// under-reporting is worse than no number.
    #[must_use = "the answer is the permission; ignoring it consults the model past its cap"]
    pub fn permit_consultation(&mut self) -> Permission {
        if self.used >= self.cap {
            return Permission::CapReached;
        }
        self.record();
        Permission::Granted
    }

    /// Count one consultation, **saturating rather than wrapping**.
    ///
    /// Private because [`permit_consultation`](Self::permit_consultation) is the
    /// only sanctioned way to spend the budget — a counter a caller could
    /// increment on its own is a counter that can disagree with the permission
    /// that authorised it. The saturation is defence rather than an expected
    /// path: wrapping at `u32::MAX` would reset a spent budget to zero, turning
    /// an exhausted seam back into an open one.
    fn record(&mut self) {
        self.used = self.used.saturating_add(1);
    }
}

/// The answer to "may the run consult the model once more?".
///
/// A two-arm enum rather than a `bool` so the refusal hands the caller the
/// taxonomy member it parks under, instead of a bare `false` every call site has
/// to translate into a reason string of its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    /// Permitted, and counted.
    Granted,
    /// The budget is spent. The run parks under [`EscalationReason::CapReached`]
    /// and does **not** continue rules-only.
    CapReached,
}

impl Permission {
    /// The taxonomy member a refused permission parks under, or `None` when it
    /// was granted.
    pub fn reason(&self) -> Option<EscalationReason> {
        match self {
            Permission::Granted => None,
            Permission::CapReached => Some(EscalationReason::CapReached),
        }
    }
}

/// Resolve argv's optional cap into the budget a run will consult under, or
/// refuse.
///
/// **Pure, and refusing here is what stops a cap that cannot bind from looking
/// like one that can.** `resolved_max_steps` is the value
/// [`super::bounds::resolve`] returned for *this* run — never
/// [`super::bounds::DEFAULT_MAX_STEPS`]. Comparing against the constant is the
/// exact shape of the Critical Phase 20's review found: under a default of 20 a
/// cap of 2 and a cap of 3 are both accepted, so a run bounded at two steps
/// carries an escalation cap that can never fire, and the guard that was
/// supposed to catch it compared two constants with each other.
///
/// The rules, and what each is for:
///
/// * **A supplied cap of zero is refused.** It is a seam that can never fire
///   while still looking configured — the same failure
///   [`super::bounds::resolve`] refuses for a zero step cap.
/// * **A supplied cap at or above the resolved step cap is refused**, naming
///   both numbers. The run halts on its step cap first, so such a cap is a
///   disablement wearing a cap's clothing (research Q3). Refusing it in the
///   parser is what makes "there is no disablement" a property of the parser
///   rather than of the caller's restraint — the justification
///   [`super::bounds::MAX_WALL_CLOCK_CAP_SECS`] states at length.
/// * **An unsupplied cap takes [`DEFAULT_MAX_ESCALATIONS`], reduced to the
///   largest value that can bind under this run's step cap.** The default is not
///   exempt from the comparison — a default that survived it unexamined would be
///   exactly the "quietly legal" case this function exists to prevent — but it is
///   *reduced* rather than refused, because a refusal must name an action the
///   caller can take and for `--max-steps 1` no legal cap exists at all. A
///   one-step run therefore gets a budget of zero and truthfully reports that it
///   cannot escalate (see [`EscalationBudget::can_escalate`]), instead of being
///   refused with no remedy. The reduced value is what the budget carries, so a
///   reader sees the cap that was really in force rather than the constant.
pub fn resolve(
    max_escalations: Option<u32>,
    resolved_max_steps: u32,
) -> Result<EscalationBudget, EscalationRefusal> {
    let cap = match max_escalations {
        Some(0) => return Err(EscalationRefusal::ZeroCap),
        Some(cap) if cap >= resolved_max_steps => {
            return Err(EscalationRefusal::CapCannotBind {
                cap,
                max_steps: resolved_max_steps,
            })
        }
        Some(cap) => cap,
        // The default, put through the same comparison rather than around it.
        None => DEFAULT_MAX_ESCALATIONS.min(resolved_max_steps.saturating_sub(1)),
    };

    Ok(EscalationBudget { cap, used: 0 })
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::driver::{bounds, untrusted};

    /// The step cap a run bounded at `max_steps` is really running under, built
    /// the only sanctioned way: by asking [`bounds::resolve`] and reading
    /// `max_steps` off what it returned.
    ///
    /// **Never `bounds::DEFAULT_MAX_STEPS`.** A test that reached for the
    /// constant would be comparing two constants with each other, which is
    /// precisely the defect the boundary test below exists to catch.
    fn resolved_step_cap(max_steps: Option<u32>) -> u32 {
        bounds::resolve(max_steps, None)
            .expect("a legal step cap resolves")
            .max_steps
    }

    #[test]
    fn every_arm_is_prefixed_and_the_constants_and_arms_match_both_ways() {
        let mut from_arms: Vec<&str> = EscalationReason::ALL
            .iter()
            .map(|reason| reason.as_str())
            .collect();
        assert!(!from_arms.is_empty(), "the taxonomy is empty");

        for name in &from_arms {
            assert!(
                name.starts_with("escalation_"),
                "every arm's identifier must carry the taxonomy's prefix so the \
                 producer is readable from the string alone; {name} does not"
            );
        }

        // Spelled out rather than derived from the enum: a list built by walking
        // `as_str` would be the enum compared with itself and could not detect a
        // missing constant.
        let mut declared = vec![
            REASON_ESCALATION_CAP_REACHED,
            REASON_ESCALATION_ACTION_REFUSED,
            REASON_ESCALATION_OUTPUT_UNUSABLE,
        ];

        from_arms.sort_unstable();
        declared.sort_unstable();
        assert_eq!(
            from_arms, declared,
            "the arms and the constants disagree. An arm without a constant is a \
             reason nobody can grep for; a constant without an arm is a vocabulary \
             wider than the truth it describes."
        );

        let mut unique = from_arms.clone();
        unique.dedup();
        assert_eq!(unique.len(), from_arms.len(), "two arms share an identifier");
    }

    #[test]
    fn each_arm_renders_its_own_constant() {
        assert_eq!(
            EscalationReason::CapReached.as_str(),
            REASON_ESCALATION_CAP_REACHED
        );
        assert_eq!(
            EscalationReason::ActionRefused.as_str(),
            REASON_ESCALATION_ACTION_REFUSED
        );
        assert_eq!(
            EscalationReason::OutputUnusable.as_str(),
            REASON_ESCALATION_OUTPUT_UNUSABLE
        );
    }

    /// **The phase's most load-bearing guard, and the shape of Phase 20's
    /// Critical.**
    ///
    /// The step cap is built by asking [`bounds::resolve`] and reading the value
    /// back — never by naming `bounds::DEFAULT_MAX_STEPS`. Against a resolution
    /// that compared the supplied cap with the constant, a cap of 2 and a cap of
    /// 3 would both be accepted under a default of 20 and this test FAILS.
    #[test]
    fn a_cap_that_cannot_bind_is_refused_against_the_resolved_step_cap() {
        let two = resolved_step_cap(Some(2));
        assert_eq!(two, 2, "the fixture must be running under a step cap of two");

        // One below the resolved cap binds, so it is accepted.
        let budget = resolve(Some(1), two).expect("a cap below the step cap can bind");
        assert_eq!(budget.cap(), 1);

        // Equal to it, and above it, cannot.
        for cap in [2, 3] {
            let refusal = resolve(Some(cap), two)
                .expect_err("a cap at or above the resolved step cap can never bind");
            assert_eq!(
                refusal,
                EscalationRefusal::CapCannotBind { cap, max_steps: two }
            );

            let rendered = refusal.to_string();
            assert!(
                rendered.contains(&cap.to_string()) && rendered.contains(&two.to_string()),
                "the refusal must name BOTH the supplied cap and the resolved step \
                 cap — a caller who is told only one of them cannot tell which to \
                 change; got: {rendered}"
            );
            assert!(
                rendered.contains("--max-steps"),
                "the refusal must name at least one concrete action the caller can \
                 take; got: {rendered}"
            );
        }

        // The control arm: the same caps are legal under a roomier resolved step
        // cap, so the refusals above are about the resolved value and not about
        // the numbers 2 and 3.
        let roomy = resolved_step_cap(None);
        assert!(resolve(Some(2), roomy).is_ok());
        assert!(resolve(Some(3), roomy).is_ok());
    }

    /// The default is put THROUGH the comparison rather than around it: under a
    /// resolved step cap of two it does not survive as three.
    ///
    /// Against a resolution that special-cased the unsupplied cap — handing back
    /// `DEFAULT_MAX_ESCALATIONS` without consulting the step cap at all — this
    /// FAILS, because the budget would carry 3 under a run that can take 2 steps.
    #[test]
    fn the_default_does_not_survive_a_step_cap_it_could_not_bind_under() {
        let two = resolved_step_cap(Some(2));
        let budget = resolve(None, two).expect("an unsupplied cap always resolves");
        assert_eq!(
            budget.cap(),
            1,
            "the default of {DEFAULT_MAX_ESCALATIONS} cannot bind under a step cap \
             of {two}, so the budget must carry the largest value that can — not \
             the constant"
        );
        assert!(budget.can_escalate());

        // A one-step run can afford no consultation at all, and says so rather
        // than carrying a cap that pretends to bind.
        let one = resolved_step_cap(Some(1));
        let budget = resolve(None, one).expect("an unsupplied cap always resolves");
        assert_eq!(budget.cap(), 0);
        assert!(
            !budget.can_escalate(),
            "a one-step run truthfully reports that it cannot escalate"
        );

        // And a roomy run keeps the default whole, so the reduction above is
        // about the step cap rather than a cap that is always reduced.
        let roomy = resolved_step_cap(None);
        assert_eq!(
            resolve(None, roomy).expect("resolves").cap(),
            DEFAULT_MAX_ESCALATIONS
        );
    }

    #[test]
    fn a_supplied_cap_of_zero_is_refused_and_the_message_names_zero() {
        let refusal = resolve(Some(0), resolved_step_cap(None))
            .expect_err("a supplied zero is a seam that can never fire");
        assert_eq!(refusal, EscalationRefusal::ZeroCap);
        assert!(
            refusal.to_string().contains('0'),
            "the refusal must name the value that was refused; got: {refusal}"
        );
        assert!(
            refusal.to_string().contains("--max-escalations"),
            "the refusal must name the flag; got: {refusal}"
        );
    }

    /// **The tie-breaking contract, as a test written from the sentence in
    /// [`EscalationBudget::permit_consultation`]'s doc.**
    ///
    /// Against a check-after-the-fact counter — one that consults first and asks
    /// afterwards — this FAILS with a recorded count of 2 under a cap of 1.
    #[test]
    fn a_budget_of_one_permits_exactly_one_consultation() {
        let mut budget = resolve(Some(1), resolved_step_cap(None)).expect("resolves");
        assert_eq!(budget.used(), 0);
        assert_eq!(budget.remaining(), 1);

        assert_eq!(budget.permit_consultation(), Permission::Granted);
        assert_eq!(budget.used(), 1);
        assert_eq!(budget.remaining(), 0);

        assert_eq!(budget.permit_consultation(), Permission::CapReached);
        assert_eq!(
            budget.used(),
            1,
            "the consultation that would be number N+1 does not happen, so it is \
             not counted; a count of 2 means the check ran after the model had \
             already been asked"
        );
        assert_eq!(
            budget.permit_consultation().reason(),
            Some(EscalationReason::CapReached),
            "a refused permission hands the caller the taxonomy member it parks \
             under, rather than a bare false"
        );
        assert_eq!(budget.used(), 1, "a refused ask never spends the budget");
    }

    #[test]
    fn the_counter_saturates_rather_than_wrapping() {
        let mut budget = resolve(Some(1), resolved_step_cap(None)).expect("resolves");
        // Reached directly because `permit_consultation` cannot get here: it
        // refuses at the cap. The saturation is defence for the counter itself —
        // wrapping would turn an exhausted seam back into an open one.
        budget.used = u32::MAX;
        budget.record();
        assert_eq!(
            budget.used, u32::MAX,
            "the count saturates; wrapping to zero would reset a spent budget"
        );
        assert_eq!(budget.remaining(), 0);
        assert_eq!(budget.permit_consultation(), Permission::CapReached);
    }

    #[test]
    fn a_named_but_refused_action_is_carried_whole_and_rendered_bounded() {
        let long =
            "/gsd-".to_string() + &"x".repeat(untrusted::MAX_UNTRUSTED_FIELD_CHARS * 2);
        let action = NamedAction::Named(long.clone());

        assert_eq!(
            action.observed(),
            Some(long.as_str()),
            "the TYPE carries the observed value whole — never mapped, never \
             dropped"
        );

        let detail = action.detail();
        assert!(
            detail.chars().count()
                <= untrusted::MAX_UNTRUSTED_FIELD_CHARS
                    + untrusted::TRUNCATION_MARKER.chars().count(),
            "the RENDERING is bounded; got {} characters",
            detail.chars().count()
        );
        assert!(
            detail.ends_with(untrusted::TRUNCATION_MARKER),
            "a shortened value is marked as shortened, or a reader cannot tell it \
             from a short one; got: {detail}"
        );
    }

    #[test]
    fn a_refused_action_cannot_become_two_record_lines() {
        let hostile = "/gsd-progress\n{\"reason\":\"forged\"}\u{0007}\r\ntrailing";
        let action = NamedAction::Named(hostile.to_string());

        assert_eq!(
            action.observed(),
            Some(hostile),
            "the evidence is kept verbatim; the cleaning happens at the record"
        );

        let detail = action.detail();
        assert_eq!(
            detail.lines().count(),
            1,
            "the journal is one line per entry, and an untrusted value that could \
             carry a newline into it could forge a record; got: {detail:?}"
        );
        assert!(
            !detail.chars().any(|c| c.is_control()),
            "control characters are stripped; got: {detail:?}"
        );
    }

    #[test]
    fn nothing_named_and_something_unrecognised_are_different_states() {
        assert_eq!(NamedAction::Absent.observed(), None);
        assert_eq!(NamedAction::Absent.detail(), UNNAMED);
        assert_eq!(NamedAction::Named(String::new()).observed(), Some(""));
        assert_ne!(
            NamedAction::Named(String::new()),
            NamedAction::Absent,
            "an empty string the seam supplied and an absent field are not the \
             same observation, and a build that flattened them could not tell a \
             hostile repository from a transport that returned nothing"
        );
    }
}
