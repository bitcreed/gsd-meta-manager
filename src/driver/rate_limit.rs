//! The subscription quota signal: classifying one retained `rate_limit_event`.
//!
//! **This module performs no I/O of its own and reads no clock**, in the same
//! register and for the same reason as [`super::bounds`] and [`super::router`]:
//! the caller retains the payload off the stream and hands in the instant to
//! judge a reset time against (D-11). That is what makes the sanity bound below
//! testable one second either side without waiting thirty days, and what lets a
//! committed capture's fixed timestamp be judged against a `now` chosen to sit
//! beside it — a bound measured against the wall clock would turn every fixture
//! assertion into a test that passes today and fails next month.
//!
//! **The payload is untrusted input.** It arrives from a process that itself
//! consumed untrusted repository content, so everything here is tolerant
//! parsing: no strict unknown-field rejection (forbidden everywhere under this
//! source tree, and grepped for), no panicking accessor, and no `unwrap` on a
//! wire field. `status` and `rateLimitType` are matched as `&str` with an
//! explicit fallback arm that carries the observed value, following
//! `src/executor/stream_json.rs:14-18` and `src/executor/outcome.rs:9-12`: this
//! CLI shipped **three** new `rateLimitType` values on one version line
//! (`seven_day_opus`, `seven_day_sonnet`, `seven_day_overage_included`), and a
//! typed parse pinned at the version this repository's captures were taken from
//! would have lost all three while a string match with a fallback loses none.
//!
//! **The reason vocabulary is closed and greppable**, one `pub const REASON_*`
//! per [`QuotaReason`] arm through [`QuotaReason::as_str`], mirroring
//! `src/envelope/policy.rs`'s `ParkReason` shape. It is a **fourth sibling
//! taxonomy** beside the envelope's `ParkReason`, the router's `RouterReason`
//! and the bounds' `BoundsReason` — never an extension of any of them, and in
//! particular not a fifth `BoundsReason` arm: CTRL-06's four detectors are facts
//! about *this* run's budget, while a quota rejection is a fact about a resource
//! shared with every other Claude surface the user has. All four reach a reader
//! through the one `JournalEvent::Parked` record and the one `parked:` terminal
//! label.
//!
//! **A rate limit is read from the structured transport and never from the
//! agent's prose** (D-10), and **no dollar figure is ever presented as what a run
//! cost** (D-16): under subscription auth the CLI's `total_cost_usd` is a
//! notional API-equivalent price rather than a charge, and the quota — not a
//! budget — is the constraint the user is deciding about.

use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::Value;

/// The stream event's own field carrying the quota object.
///
/// Named here rather than at the call site so every piece of wire knowledge
/// about this event lives in one module; the driver retains the event verbatim
/// and asks no questions of it.
pub const RATE_LIMIT_INFO_FIELD: &str = "rate_limit_info";

/// The quota object's status field.
pub const STATUS_FIELD: &str = "status";

/// The quota object's window-type field.
pub const RATE_LIMIT_TYPE_FIELD: &str = "rateLimitType";

/// The quota object's reset-time field, in unix epoch **seconds**.
pub const RESETS_AT_FIELD: &str = "resetsAt";

/// The one `status` value that parks a run.
///
/// **`allowed` and `allowed_warning` deliberately have no constants of their
/// own**, because nothing here matches on them: every value that is not this one
/// is not a park, and a list of non-parking values would be a second thing to
/// keep in sync with a CLI that ships new enum members on a patch line.
pub const STATUS_REJECTED: &str = "rejected";

/// The five-hour window, verbatim from the 2.1.235 binary's string table.
pub const WINDOW_FIVE_HOUR: &str = "five_hour";
/// The plain seven-day window.
pub const WINDOW_SEVEN_DAY: &str = "seven_day";
/// The Opus-specific seven-day window.
pub const WINDOW_SEVEN_DAY_OPUS: &str = "seven_day_opus";
/// The Sonnet-specific seven-day window.
pub const WINDOW_SEVEN_DAY_SONNET: &str = "seven_day_sonnet";
/// The seven-day window with purchased overage folded in.
pub const WINDOW_SEVEN_DAY_OVERAGE_INCLUDED: &str = "seven_day_overage_included";

/// The stable identifier a quota park writes into `JournalEvent::Parked`.
pub const REASON_QUOTA_REJECTED: &str = "quota_rejected";

/// What a window or a reset time is called when it could not be established.
///
/// One spelling, used for both, so a reader greps once. It is deliberately a
/// word rather than an omission: a park record that simply left the window out
/// would be indistinguishable from one written by a version that never recorded
/// it.
pub const UNKNOWN: &str = "unknown";

/// How far from `now` a reset time may sit and still be reported.
///
/// **Roughly thirty days, and it is a validation rather than a preference.** The
/// research document's assumption log records that no unit is stated anywhere in
/// the binary's strings: two observed ten-digit values and a delta consistent
/// with a seven-day window are the whole of the evidence that `resetsAt` is
/// epoch *seconds*. Were it milliseconds, the same value would render a reset
/// roughly fifty-six thousand years out — as fact, to a human deciding when to
/// come back. The bound is that assumption's stated mitigation, and it doubles
/// as the bound on a corrupted or hostile value.
///
/// Both windows this event describes are at most seven days, so a legitimate
/// reset can never approach thirty; the margin exists so ordinary clock skew and
/// a queued overage window cannot suppress a true value.
///
/// **It bounds the FUTURE direction only.** See
/// [`RESET_MAX_SKEW_BEHIND_SECS`] for the other side and why the two differ.
pub const RESET_SANITY_WINDOW_SECS: i64 = 30 * 24 * 60 * 60;

/// How far *behind* `now` a reset time may sit and still be reported.
///
/// **Five minutes, and the asymmetry with [`RESET_SANITY_WINDOW_SECS`] is the
/// whole point.** A quota window that "resets" before now is nonsense by
/// construction: there is nothing to wait for, and rendering it into
/// `park_detail` as `resets_at=<a past instant>` tells a human to come back at a
/// time that has already been and gone. The bound existed to reject exactly this
/// class of corrupted, stale or hostile value, and a symmetric bound — which is
/// what `saturating_abs` produced — was the one shape it let through: a reset
/// thirty days in the past passed validation and was reported as fact (WR-03).
///
/// Five minutes is a clock-skew allowance rather than a window: the driver's
/// clock and the API's may disagree by seconds, and a reset that has *just*
/// passed is a true statement about a window that is reopening. Anything further
/// behind is not a reset time.
pub const RESET_MAX_SKEW_BEHIND_SECS: i64 = 5 * 60;

/// The longest observed window string that reaches the journal.
///
/// The window string is untrusted wire content and the journal lands in
/// `.planning/`, a directory users commit (SAFE-04, T-17-05, T-20-27). The
/// **type** carries the observed value verbatim, which is what
/// "never mapped, never dropped" requires; this bound applies only where the
/// value is rendered into a record, alongside control-character removal, so an
/// unbounded or line-breaking value cannot be written through a record whose
/// one-line-per-entry shape every reader depends on.
pub const MAX_OBSERVED_WINDOW_CHARS: usize = 64;

/// The substring that makes a CLI terminal reason a rate-limit condition.
///
/// See [`terminal_reason_names_a_rate_limit`] for why this and not the broader
/// `api_error` prefix.
pub const RATE_LIMIT_TERMINAL_MARKER: &str = "rate_limit";

/// Why a run parked on a subscription quota.
///
/// One arm today, and it is spelled as an enum rather than as a bare constant
/// for the reason its three sibling taxonomies are: a second quota condition —
/// an overage window, an organisation spend cap — is a new arm here and a
/// compile error at every site that has to classify it, rather than a second
/// literal minted at a call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuotaReason {
    /// The transport reported `status: "rejected"`, or the run's own failure
    /// named a rate-limit terminal reason.
    Rejected,
}

impl QuotaReason {
    /// The stable snake_case identifier a later reader greps for.
    ///
    /// One arm per variant, no wildcard.
    pub fn as_str(&self) -> &'static str {
        match self {
            QuotaReason::Rejected => REASON_QUOTA_REJECTED,
        }
    }
}

/// Which quota window a payload named.
///
/// The four seven-day members of the wire enum collapse into one arm on purpose:
/// what a human deciding when to resume needs is *which budget* is exhausted and
/// therefore how long the wait is, and `seven_day_opus` and `seven_day_sonnet`
/// answer that identically. The five-hour window is the one genuinely different
/// answer, and the requirement is exactly to distinguish it from the seven-day
/// family.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuotaWindow {
    /// `five_hour`.
    FiveHour,
    /// `seven_day` and its three model- and overage-specific siblings.
    SevenDay,
    /// A value outside the known set, carried **verbatim**, or no value at all.
    ///
    /// `Some` is a string the wire supplied that this build does not recognise —
    /// which on a CLI that ships new members on a patch line is an ordinary
    /// forward-compatibility event, not a fault. `None` is an absent, non-object
    /// or type-less payload: the difference matters, because a build that
    /// flattened both into an empty string could not tell "the CLI named a
    /// window we have not seen" from "there was no window to name".
    Unknown(Option<String>),
}

impl QuotaWindow {
    /// How this window is written into a journal record.
    ///
    /// Bounded and control-character-free; see [`MAX_OBSERVED_WINDOW_CHARS`] for
    /// why the rendering and the carried value differ.
    pub fn detail(&self) -> String {
        match self {
            QuotaWindow::FiveHour => WINDOW_FIVE_HOUR.to_string(),
            QuotaWindow::SevenDay => WINDOW_SEVEN_DAY.to_string(),
            QuotaWindow::Unknown(None) => UNKNOWN.to_string(),
            QuotaWindow::Unknown(Some(observed)) => {
                format!("{UNKNOWN}({})", sanitize_observed(observed))
            }
        }
    }
}

/// What [`classify`] answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuotaVerdict {
    /// Nothing here parks a run.
    ///
    /// **This is the answer for `allowed`, for `allowed_warning`, and for every
    /// malformed payload**, and the breadth is the point. A committed capture of
    /// a completely successful run carries `status: "allowed"`
    /// (`tests/fixtures/transcripts/01-success-textonly.ndjson`), and
    /// `allowed_warning` is informational — so a rule keyed on the mere
    /// *presence* of a `rate_limit_event` would park every healthy run. That is
    /// the mistake this classifier exists to not make.
    Allowed,
    /// The transport reported `rejected`. The run parks.
    Rejected {
        /// Which window blocked it.
        window: QuotaWindow,
        /// When it resets, when that could be established and trusted.
        resets_at: Option<DateTime<Utc>>,
    },
}

/// The `rate_limit_info` object of `event`, if there is one and it is an object.
fn info(event: Option<&Value>) -> Option<&Value> {
    event
        .and_then(|event| event.get(RATE_LIMIT_INFO_FIELD))
        .filter(|info| info.is_object())
}

/// Strip control characters and bound the length of an observed wire string.
fn sanitize_observed(observed: &str) -> String {
    let mut rendered: String = observed
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_OBSERVED_WINDOW_CHARS)
        .collect();
    if observed.chars().filter(|c| !c.is_control()).count() > MAX_OBSERVED_WINDOW_CHARS {
        rendered.push_str("...");
    }
    rendered
}

/// Which window `event` named.
///
/// Every path yields a value and none panics: an absent event, a payload that is
/// not an object, a missing `rateLimitType` and a `rateLimitType` that is not a
/// string all yield [`QuotaWindow::Unknown`] rather than a guess.
pub fn window(event: Option<&Value>) -> QuotaWindow {
    let Some(info) = info(event) else {
        return QuotaWindow::Unknown(None);
    };

    match info.get(RATE_LIMIT_TYPE_FIELD).and_then(Value::as_str) {
        Some(WINDOW_FIVE_HOUR) => QuotaWindow::FiveHour,
        Some(
            WINDOW_SEVEN_DAY
            | WINDOW_SEVEN_DAY_OPUS
            | WINDOW_SEVEN_DAY_SONNET
            | WINDOW_SEVEN_DAY_OVERAGE_INCLUDED,
        ) => QuotaWindow::SevenDay,
        Some(observed) => QuotaWindow::Unknown(Some(observed.to_string())),
        None => QuotaWindow::Unknown(None),
    }
}

/// When `event` says the window resets, if that can be established and trusted.
///
/// **Read through an integer accessor, with no floating-point path anywhere.**
/// `serde_json`'s `as_i64` yields `None` for a float, for a string and for an
/// absent field, so a value that is not an integer produces no reset time rather
/// than a truncated or rounded one.
///
/// Then bounded, and **the bound is asymmetric**: the full window forwards
/// ([`RESET_SANITY_WINDOW_SECS`]) and clock skew only backwards
/// ([`RESET_MAX_SKEW_BEHIND_SECS`]). A reset meaningfully in the past is not a
/// reset time — it is the corrupted or stale value the bound exists to catch,
/// and a symmetric bound was the one shape that let it through (WR-03).
///
/// Nothing in this crate schedules anything against the result — it is displayed
/// and never acted on — so a value inside the bound is still only a report.
pub fn reset_time(event: Option<&Value>, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    let secs = info(event)?.get(RESETS_AT_FIELD)?.as_i64()?;
    // `from_timestamp` returns `None` rather than panicking or wrapping for a
    // value outside the representable range, which is the whole reason the
    // construction happens before the bound instead of after it: a value the
    // constructor refuses must be reported as unknown, not compared.
    let resets_at = DateTime::from_timestamp(secs, 0)?;
    // `saturating_sub` because the constructor admits instants hundreds of
    // thousands of years out, and the difference of two i64 timestamps is only
    // *nearly* always in range.
    let ahead = resets_at.timestamp().saturating_sub(now.timestamp());
    (ahead >= -RESET_MAX_SKEW_BEHIND_SECS && ahead <= RESET_SANITY_WINDOW_SECS).then_some(resets_at)
}

/// Whether this payload carries the one `status` that parks a run.
///
/// **The predicate half of [`classify`], split out so the drain loop can LATCH a
/// rejection without reading a clock.** The loop retains one payload and
/// classifies it once, at the end of the iteration; retaining simply the *latest*
/// event meant a `rejected` on the five-hour window followed by an `allowed` on
/// the seven-day one — one event per window, or one per turn on a steered run —
/// left the driver holding the `allowed` payload and continuing (WR-02). The
/// retained value is still uninspected *by the driver*: every question about it
/// is asked here, which is the one place a wire field is interpreted and the one
/// place its malformed shapes are tested.
///
/// Every malformed shape answers `false`, exactly as [`classify`] answers
/// [`QuotaVerdict::Allowed`] for them: only the observed word parks.
pub fn is_rejection(event: Option<&Value>) -> bool {
    info(event)
        .and_then(|payload| payload.get(STATUS_FIELD))
        .and_then(Value::as_str)
        == Some(STATUS_REJECTED)
}

/// Whether this retained payload parks the run, and under which window.
///
/// **Only `status: "rejected"` parks.** See [`QuotaVerdict::Allowed`] for why
/// keying on the event's presence — or on `allowed_warning`, or on a utilization
/// threshold — would halt healthy runs, with the committed capture that proves
/// it named there.
///
/// `now` is supplied by the caller rather than read here, which is what keeps
/// this module free of a clock and the sanity bound testable at one-second
/// resolution.
pub fn classify(event: Option<&Value>, now: DateTime<Utc>) -> QuotaVerdict {
    if !is_rejection(event) {
        return QuotaVerdict::Allowed;
    }

    QuotaVerdict::Rejected {
        window: window(event),
        resets_at: reset_time(event, now),
    }
}

/// Whether a failed run's own terminal reason names a rate-limit condition.
///
/// **The second detector**, and it exists because no capture of a `rejected`
/// `rate_limit_event` can be produced without burning quota: the research
/// document's assumption A2 records that the emission path for one on a `-p`
/// stream is evidenced by the binary's strings rather than observed, and
/// recommends covering the case where the rejection arrives only on the failure
/// envelope. It reads a field the driver already holds, so nothing about the
/// one outcome-derivation entry point changes.
///
/// **`contains("rate_limit")` rather than `starts_with("api_error")`.** The
/// research document suggested the prefix; the prefix is wrong, because it
/// reports *every* API error — an overloaded upstream, a 500, a refused
/// request — as a quota exhaustion. That names the wrong cause and tells the
/// user to wait for a reset that would not have fixed anything, which is
/// precisely the misinformation CTRL-07's transparency prohibition is about. No
/// terminal reason in the observed vocabulary contains this substring for any
/// other cause.
pub fn terminal_reason_names_a_rate_limit(terminal_reason: Option<&str>) -> bool {
    terminal_reason.is_some_and(|reason| reason.contains(RATE_LIMIT_TERMINAL_MARKER))
}

/// The detail a quota park records beside its reason.
///
/// Two facts and no third: which window blocked the run, and when it resets or
/// that this is unknown. **No dollar figure appears here and none may** (D-16) —
/// under subscription auth the CLI's `total_cost_usd` is a notional
/// API-equivalent price rather than a charge, and presenting it as what the run
/// cost misinforms the one decision the user is making.
pub fn park_detail(window: &QuotaWindow, resets_at: Option<DateTime<Utc>>) -> String {
    let resets = match resets_at {
        Some(at) => at.to_rfc3339_opts(SecondsFormat::Secs, true),
        None => UNKNOWN.to_string(),
    };
    format!("window={} resets_at={resets}", window.detail())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::stream_json::{parse_line, Envelope, StreamMessage};
    use serde_json::json;

    /// The committed capture carrying `status: "allowed"` on a fully successful
    /// run — the fixture that makes a presence-keyed rule unshippable.
    const T01: &str = include_str!("../../tests/fixtures/transcripts/01-success-textonly.ndjson");
    /// The committed capture carrying `status: "allowed_warning"` at 25%
    /// utilization on a `seven_day` window.
    const T08: &str =
        include_str!("../../tests/fixtures/transcripts/08-tooluse-queued-two-turns.ndjson");

    /// Every `rate_limit_event` payload in `transcript`, through the shipped
    /// parser rather than through a second reader written for the test.
    fn events(transcript: &str) -> Vec<Value> {
        transcript
            .lines()
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| match parse_line(line) {
                Envelope::Parsed {
                    msg: StreamMessage::RateLimitEvent(value),
                    ..
                } => Some(value),
                _ => None,
            })
            .collect()
    }

    /// The one `rate_limit_event` payload in `transcript`.
    fn only_event(transcript: &str) -> Value {
        let mut found = events(transcript);
        assert_eq!(
            found.len(),
            1,
            "the fixtures this module drives from carry exactly one rate-limit \
             event; a second would make 'the retained payload' ambiguous"
        );
        found.remove(0)
    }

    /// An instant, from epoch seconds.
    fn at(secs: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(secs, 0).expect("the test's own instants are representable")
    }

    /// A synthetic event carrying `info` as its `rate_limit_info`.
    fn event(info: Value) -> Value {
        json!({ "rate_limit_info": info })
    }

    // -----------------------------------------------------------------------
    // Only `rejected` parks — proven from the committed captures
    // -----------------------------------------------------------------------

    #[test]
    fn the_allowed_status_on_a_fully_successful_capture_is_not_a_park() {
        let observed = only_event(T01);
        assert_eq!(
            classify(Some(&observed), at(0)),
            QuotaVerdict::Allowed,
            "fixture 01 is a completely successful run that nevertheless carries a \
             rate_limit_event. A rule keyed on the event's PRESENCE would park \
             this run — and every other healthy one — which is the single mistake \
             this classifier exists to not make"
        );
    }

    #[test]
    fn the_allowed_warning_status_is_not_a_park_at_any_utilization_including_one() {
        let observed = only_event(T08);
        assert_eq!(
            classify(Some(&observed), at(0)),
            QuotaVerdict::Allowed,
            "fixture 08 carries allowed_warning at 25% utilization on a completed \
             run. A warning is informational: the request was served"
        );

        let saturated = event(json!({
            "status": "allowed_warning",
            "rateLimitType": "seven_day",
            "resetsAt": 1785859200,
            "utilization": 1.0,
        }));
        assert_eq!(
            classify(Some(&saturated), at(0)),
            QuotaVerdict::Allowed,
            "a utilization of one on an ALLOWED_WARNING is still a request that \
             was served. Parking here would invent a refusal the transport never \
             reported, and utilization is not the field that says whether it did"
        );
    }

    #[test]
    fn a_rejected_status_parks_at_every_utilization_value() {
        for utilization in [json!(null), json!(0.0), json!(0.5), json!(1.0)] {
            let observed = event(json!({
                "status": "rejected",
                "rateLimitType": "five_hour",
                "utilization": utilization,
            }));
            assert!(
                matches!(
                    classify(Some(&observed), at(0)),
                    QuotaVerdict::Rejected { .. }
                ),
                "a rejected status is the transport saying the request was \
                 REFUSED. Gating that on a utilization threshold would let a \
                 refused run continue on the strength of a number that did not \
                 refuse it; utilization was {utilization}"
            );
        }
    }

    #[test]
    fn the_overage_status_field_is_not_mistaken_for_the_status_field() {
        // Fixture 01 carries `overageStatus: "rejected"` beside
        // `status: "allowed"`. A classifier that searched the object for the
        // word rather than reading the named field would park every run that
        // ever carried this capture's shape — which is all seven of the
        // committed ones.
        let observed = only_event(T01);
        assert_eq!(
            observed[RATE_LIMIT_INFO_FIELD]["overageStatus"],
            json!(STATUS_REJECTED),
            "the fixture must still carry the decoy, or this test proves nothing"
        );
        assert_eq!(classify(Some(&observed), at(0)), QuotaVerdict::Allowed);
    }

    // -----------------------------------------------------------------------
    // The window
    // -----------------------------------------------------------------------

    #[test]
    fn the_five_hour_value_maps_to_the_five_hour_window() {
        let observed = event(json!({ "rateLimitType": WINDOW_FIVE_HOUR }));
        assert_eq!(window(Some(&observed)), QuotaWindow::FiveHour);
        assert_eq!(
            window(Some(&only_event(T01))),
            QuotaWindow::FiveHour,
            "and the committed capture agrees, so the mapping is proven against \
             the wire rather than against the test's own literal"
        );
    }

    #[test]
    fn every_seven_day_family_value_maps_to_the_seven_day_window() {
        for value in [
            WINDOW_SEVEN_DAY,
            WINDOW_SEVEN_DAY_OPUS,
            WINDOW_SEVEN_DAY_SONNET,
            WINDOW_SEVEN_DAY_OVERAGE_INCLUDED,
        ] {
            let observed = event(json!({ "rateLimitType": value }));
            assert_eq!(
                window(Some(&observed)),
                QuotaWindow::SevenDay,
                "`{value}` is a seven-day budget. What a human deciding when to \
                 resume needs is how long the wait is, and all four members \
                 answer that identically"
            );
        }
        assert_eq!(window(Some(&only_event(T08))), QuotaWindow::SevenDay);
    }

    #[test]
    fn an_unrecognised_window_value_is_carried_verbatim_rather_than_mapped() {
        let observed = event(json!({ "rateLimitType": "thirty_day_opus_max" }));
        assert_eq!(
            window(Some(&observed)),
            QuotaWindow::Unknown(Some("thirty_day_opus_max".to_string())),
            "this CLI shipped three new rateLimitType values on one version line. \
             A value outside the known set must be carried as ITSELF — mapping it \
             onto five_hour or seven_day would report a wait length nobody \
             observed, and dropping it would leave a support report with nothing \
             to grep"
        );
    }

    #[test]
    fn an_absent_a_non_object_and_a_typeless_payload_are_all_unknown_and_none_panic() {
        assert_eq!(window(None), QuotaWindow::Unknown(None), "absent");
        assert_eq!(
            window(Some(&json!({ "rate_limit_info": "not an object" }))),
            QuotaWindow::Unknown(None),
            "a non-object payload"
        );
        assert_eq!(
            window(Some(&json!({ "rate_limit_info": null }))),
            QuotaWindow::Unknown(None),
            "a null payload"
        );
        assert_eq!(
            window(Some(&json!({ "uuid": "no payload at all" }))),
            QuotaWindow::Unknown(None),
            "an event with no rate_limit_info field"
        );
        assert_eq!(
            window(Some(&event(json!({ "status": "rejected" })))),
            QuotaWindow::Unknown(None),
            "a payload with a status but no window type"
        );
        assert_eq!(
            window(Some(&event(json!({ "rateLimitType": 5 })))),
            QuotaWindow::Unknown(None),
            "a window type that is not a string is not a window type"
        );
    }

    #[test]
    fn a_rejected_payload_with_no_usable_window_still_parks_and_says_unknown() {
        let observed = event(json!({ "status": "rejected" }));
        assert_eq!(
            classify(Some(&observed), at(0)),
            QuotaVerdict::Rejected {
                window: QuotaWindow::Unknown(None),
                resets_at: None,
            },
            "a refusal whose window could not be read is still a refusal. \
             Continuing because the window was unreadable would spend a shared \
             quota on the strength of a parse failure"
        );
    }

    // -----------------------------------------------------------------------
    // The reset time
    // -----------------------------------------------------------------------

    #[test]
    fn a_reset_time_is_read_as_an_integer_and_never_as_a_float_or_a_string() {
        let now = at(1_785_327_000);

        for shape in [
            json!(1_785_327_060.0_f64),
            json!("1785327060"),
            json!(null),
            json!({ "secs": 1_785_327_060 }),
        ] {
            let observed = event(json!({ "resetsAt": shape }));
            assert_eq!(
                reset_time(Some(&observed), now),
                None,
                "a reset time that is not an integer must yield NO reset time \
                 rather than a rounded, truncated or parsed one. A wrong instant \
                 rendered as fact is worse than an absent one, because a reader \
                 acts on it; shape was {shape}"
            );
        }

        let absent = event(json!({ "status": "rejected" }));
        assert_eq!(reset_time(Some(&absent), now), None, "an absent field");
        assert_eq!(reset_time(None, now), None, "an absent event");
    }

    #[test]
    fn a_reset_value_the_timestamp_constructor_rejects_yields_no_reset_time() {
        let now = at(1_785_327_000);
        for secs in [i64::MAX, i64::MIN, 900_000_000_000_000_i64] {
            assert!(
                DateTime::from_timestamp(secs, 0).is_none(),
                "the arm is only meaningful while the constructor really refuses \
                 {secs} — a control on the test's own premise"
            );
            let observed = event(json!({ "resetsAt": secs }));
            assert_eq!(
                reset_time(Some(&observed), now),
                None,
                "a value the timestamp constructor refuses must produce unknown, \
                 never a panic and never a wrapped date"
            );
        }
    }

    #[test]
    fn the_sanity_bound_is_asymmetric_a_full_window_ahead_and_clock_skew_only_behind() {
        let now = at(1_785_327_000);

        // ---- Ahead: the full window, one second either side of the edge ----
        for (offset, expected) in [
            (RESET_SANITY_WINDOW_SECS - 1, true),
            (RESET_SANITY_WINDOW_SECS, true),
            (RESET_SANITY_WINDOW_SECS + 1, false),
        ] {
            let secs = now.timestamp() + offset;
            assert_eq!(
                reset_time(Some(&event(json!({ "resetsAt": secs }))), now).is_some(),
                expected,
                "ahead by {offset}s: the bound itself is inside, and one second \
                 past it is unknown. If resetsAt were milliseconds rather than \
                 seconds the same field would render a reset fifty-six thousand \
                 years out — as FACT, to a human deciding when to come back"
            );
        }

        // ---- Behind: clock skew ONLY, and this is WR-03 --------------------
        //
        // The bound used to be symmetric — `saturating_abs` over the delta — so
        // a reset up to thirty days IN THE PAST passed validation and was
        // rendered into `park_detail` as when the quota resets. A window that
        // reset before now is nonsense by construction: there is nothing to wait
        // for, and the user is told to come back at a time that has been and
        // gone. That is precisely the corrupted-or-stale value the bound exists
        // to catch, and it was the one shape it let through.
        for (offset, expected) in [
            (RESET_MAX_SKEW_BEHIND_SECS - 1, true),
            (RESET_MAX_SKEW_BEHIND_SECS, true),
            (RESET_MAX_SKEW_BEHIND_SECS + 1, false),
            (RESET_SANITY_WINDOW_SECS - 1, false),
        ] {
            let secs = now.timestamp() - offset;
            assert_eq!(
                reset_time(Some(&event(json!({ "resetsAt": secs }))), now).is_some(),
                expected,
                "behind by {offset}s: five minutes is a clock-skew allowance, not \
                 a window. A reset that has JUST passed is a true statement about \
                 a window reopening; anything further behind is not a reset time \
                 at all, and reporting one is the misinformation CTRL-07's \
                 transparency requirement forbids"
            );
        }

        // And the asymmetry is a property rather than a coincidence of the two
        // numbers: a bound that treated the directions alike could not tell a
        // seven-day window from a seven-day-old one.
        assert!(
            RESET_MAX_SKEW_BEHIND_SECS < RESET_SANITY_WINDOW_SECS,
            "the backwards allowance must stay strictly smaller than the forward \
             window, or the bound is symmetric again under a different name"
        );
    }

    #[test]
    fn an_accepted_reset_time_round_trips_to_the_instant_its_epoch_seconds_denote() {
        // The committed capture's own value, judged against a `now` sitting one
        // hour before it — so this assertion is as true in ten years as today.
        // A bound measured against the wall clock would make every fixture
        // assertion a test that passes this month and fails the next.
        const CAPTURED: i64 = 1_785_327_000;
        let now = at(CAPTURED - 3_600);

        let resets_at = reset_time(Some(&only_event(T01)), now)
            .expect("the capture's reset time is an integer inside the bound");
        assert_eq!(
            resets_at.timestamp(),
            CAPTURED,
            "the parsed instant must denote exactly the epoch SECOND on the wire. \
             A millisecond reading would land the same value in the year 58 500 \
             and a nanosecond reading would refuse to construct at all"
        );
        assert_eq!(
            resets_at.to_rfc3339_opts(SecondsFormat::Secs, true),
            "2026-07-29T12:10:00Z",
            "and it renders as the instant a reader would check a clock against"
        );
    }

    // -----------------------------------------------------------------------
    // The record a park writes
    // -----------------------------------------------------------------------

    #[test]
    fn the_park_detail_names_the_window_and_the_reset_and_never_a_dollar_figure() {
        let detail = park_detail(&QuotaWindow::FiveHour, Some(at(1_785_327_000)));
        assert!(
            detail.contains(WINDOW_FIVE_HOUR),
            "the park must name which window blocked the run: {detail}"
        );
        assert!(
            detail.contains("2026-07-29T12:10:00Z"),
            "and when it resets: {detail}"
        );

        let unknown = park_detail(&QuotaWindow::Unknown(None), None);
        assert!(
            unknown.contains(UNKNOWN),
            "an unknown window and an unknown reset are SAID rather than omitted: \
             an omission is indistinguishable from a build that never recorded \
             the field, which is the ambiguity the word closes. Got: {unknown}"
        );

        for rendered in [detail, unknown] {
            for forbidden in ["$", "usd", "USD", "cost"] {
                assert!(
                    !rendered.contains(forbidden),
                    "no dollar figure may be presented as what this run cost \
                     (D-16). Under subscription auth the CLI's total_cost_usd is \
                     a notional API-equivalent price and not a charge, and \
                     Phase 15's OQ3 spike confirmed --max-budget-usd is a \
                     post-turn breaker only. Found `{forbidden}` in: {rendered}"
                );
            }
        }
    }

    #[test]
    fn an_observed_window_string_reaching_the_journal_is_bounded_and_line_safe() {
        let hostile = format!("{}\nparked\r{}", "a".repeat(4096), "b".repeat(4096));
        let rendered = QuotaWindow::Unknown(Some(hostile)).detail();

        assert!(
            !rendered.contains('\n') && !rendered.contains('\r'),
            "the journal is one NDJSON record per line and lands in a directory \
             users commit. A window string carrying a newline is a record that \
             reads as two: {rendered}"
        );
        assert!(
            rendered.chars().count() <= MAX_OBSERVED_WINDOW_CHARS + UNKNOWN.len() + 5,
            "and it is bounded, so an oversize wire value cannot be written \
             through a record every reader reads whole: {} chars",
            rendered.chars().count()
        );
        assert!(
            rendered.starts_with(UNKNOWN),
            "it still says the window was not recognised: {rendered}"
        );
    }

    // -----------------------------------------------------------------------
    // The second detector
    // -----------------------------------------------------------------------

    #[test]
    fn a_terminal_reason_naming_a_rate_limit_is_the_second_detector() {
        for reason in [
            "api_error_rate_limit",
            "rate_limit_exceeded",
            "rate_limited_by_rate_limit",
        ] {
            assert!(
                terminal_reason_names_a_rate_limit(Some(reason)),
                "no capture of a rejected rate_limit_event can be produced without \
                 burning quota, so the failure envelope's own terminal reason is \
                 the backstop: `{reason}`"
            );
        }
    }

    #[test]
    fn an_ordinary_api_error_is_not_reported_as_a_quota_exhaustion() {
        for reason in [
            "api_error",
            "api_error_overloaded",
            "budget_exhausted",
            "completed",
            "aborted_tools",
            "aborted_streaming",
            "error_during_execution",
        ] {
            assert!(
                !terminal_reason_names_a_rate_limit(Some(reason)),
                "the research document suggested `starts_with(\"api_error\")`; \
                 that would report `{reason}` as a quota exhaustion, naming the \
                 wrong cause and telling the user to wait for a reset that would \
                 not have fixed anything. `budget_exhausted` in particular is the \
                 post-turn --max-budget-usd breaker, which is a different \
                 constraint from the subscription quota entirely (D-16)"
            );
        }
        assert!(
            !terminal_reason_names_a_rate_limit(None),
            "a run with no terminal reason has not reported a rate limit"
        );
    }

    // -----------------------------------------------------------------------
    // The taxonomy
    // -----------------------------------------------------------------------

    #[test]
    fn the_quota_reason_is_greppable_and_collides_with_no_sibling_taxonomy() {
        assert_eq!(QuotaReason::Rejected.as_str(), REASON_QUOTA_REJECTED);
        assert!(
            REASON_QUOTA_REJECTED.starts_with("quota_"),
            "a quota reason must be greppable as one, the way `bounds_` and \
             `router_` are: {REASON_QUOTA_REJECTED}"
        );

        for sibling in [
            crate::driver::bounds::REASON_NO_PROGRESS,
            crate::driver::bounds::REASON_COMMAND_REPEAT,
            crate::driver::bounds::REASON_STEP_CAP,
            crate::driver::bounds::REASON_WALL_CLOCK,
            crate::driver::router::REASON_NO_RULE,
            crate::driver::router::REASON_STATE_UNVERIFIED,
            crate::driver::router::REASON_DEPENDENCY_UNSATISFIED,
        ] {
            assert_ne!(
                REASON_QUOTA_REJECTED, sibling,
                "four taxonomies ride the one `JournalEvent::Parked.reason` \
                 field. Two of them sharing an identifier makes 'why did this run \
                 end' unanswerable from the terminal record"
            );
        }
    }
}
