//! How a launch ended, decided once (Phase 28; LNCH-03, LNCH-05, D-15, D-16, D-23).
//!
//! [`decide`] is the one pure table the launch loop applies at the end of every
//! launch. First match wins:
//!
//! 1. the user asked to stop -> `stopped`, `stop:user`;
//! 2. a cap tripped -> keep (the trip already recorded `stopped`);
//! 3. the driver ended the run itself with a terminal `RUN end` -> keep;
//! 4. the driver ended only its context (`fresh-exit`: still `running`) -> relaunch;
//! 5. the launch's result is an authentication failure (D-23) -> relaunch with the
//!    host token again (at most twice per run), or park when the stored token was
//!    the one rejected;
//! 6. the quota was exhausted (D-15): a rejected `rate_limit_event`, or a terminal
//!    reason naming a rate limit -> `parked`, never retried;
//! 7. otherwise [`classify_exit`]: `stalled` (clean exit, worker outstanding) or
//!    `crashed`.
//!
//! The inputs are facts the launcher observed (exit code, the stream file the
//! driver wrote, the store's status), never a count or label the driver sends.
//! [`summarize_stream`] reads the launch's stream-json lines tolerantly: the file
//! is written by the sandboxed model's process, so every field is untrusted.

use chrono::{DateTime, Utc};
use serde_json::Value;

use super::auth::TokenSource;
use crate::driver::rate_limit::{self, QuotaVerdict, QuotaWindow};
use crate::driverctl::counters::{classify_exit, ExitClass};
use crate::driverctl::store::TerminalStatus;
use crate::state_reader::driver_frontmatter::RunStatus;

/// What a launch's stream-json output says about how it went.
#[derive(Debug, Clone, PartialEq)]
pub struct StreamSummary {
    /// The last `"type":"result"` line, if any.
    pub result: Option<Value>,
    /// The quota verdict: the first rejected `rate_limit_event`, else the latest.
    pub rate_limit: QuotaVerdict,
    /// The last result's `terminal_reason`.
    pub terminal_reason: Option<String>,
}

/// Everything [`decide`] looks at.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchEnd {
    /// The driver's exit code; `None` when there was none to report.
    pub rc: Option<i32>,
    /// `driver stop` (SIGTERM/SIGINT to the launcher) was received.
    pub stop_requested: bool,
    /// The cap the monitor tripped, if any.
    pub tripped: Option<String>,
    /// The run's status in the store when the launch ended.
    pub status: RunStatus,
    /// Whether the driver itself logged `RUN end` in this launch.
    pub ended_by_driver: bool,
    pub stream: StreamSummary,
    /// Where this launch's token came from.
    pub token_source: TokenSource,
    /// How many auth relaunches this run has already made.
    pub auth_relaunches: u32,
    /// The item of the latest dispatch that has no report in LOG.md.
    pub outstanding: Option<String>,
}

/// What the launch loop does next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The status stands; record the launch end only.
    Keep,
    /// The driver ended its context: record the launch end and start the next one.
    FreshRelaunch,
    /// The host token was rejected: re-read it and start the next launch.
    AuthRelaunch,
    /// Stop with status `parked` and this reason; no relaunch.
    Park(String),
    /// Stop with this terminal status and reason.
    Terminal(TerminalStatus, String),
}

/// How many times a run relaunches after a rejected host token (D-23).
pub const MAX_AUTH_RELAUNCHES: u32 = 2;

/// The strings that make an error result an authentication failure (D-23, spike
/// `run.sh`): a token the host's own refresh revoked, or a login that failed.
const AUTH_FAILURE_MARKERS: [&str; 2] = ["OAuth token revoked", "Failed to authenticate"];

/// Only a line that could be one of the two kinds the summary reads is parsed, so a
/// long stream of assistant turns costs a substring search each, not a JSON parse.
fn worth_parsing(line: &str) -> bool {
    line.contains("\"result\"") || line.contains("rate_limit_event")
}

/// Reads a launch's stream-json one line at a time, keeping only what
/// [`StreamSummary`] needs: the stream can be megabytes and is written by the
/// sandboxed model's process, so nothing else is retained. Every field is untrusted.
#[derive(Default)]
pub struct StreamAccumulator {
    result: Option<Value>,
    /// The first rejected `rate_limit_event` (latched: a later `allowed` event does
    /// not erase it, WR-02 of the Phase 20 driver).
    rejected: Option<Value>,
    latest_quota: Option<Value>,
}

impl StreamAccumulator {
    /// Take one line. Anything that is not a JSON object is ignored.
    pub fn push(&mut self, line: &str) {
        let line = line.trim();
        if !line.starts_with('{') || !worth_parsing(line) {
            return;
        }
        let Ok(value @ Value::Object(_)) = serde_json::from_str::<Value>(line) else {
            return;
        };
        match value.get("type").and_then(Value::as_str) {
            Some("result") => self.result = Some(value),
            Some("rate_limit_event") => {
                if self.rejected.is_none() && rate_limit::is_rejection(Some(&value)) {
                    self.rejected = Some(value.clone());
                }
                self.latest_quota = Some(value);
            }
            _ => {}
        }
    }

    /// The summary, with reset times judged against `now`. One classification: the
    /// latched rejection if there was one, else the latest event (as the Phase 20
    /// driver does it).
    pub fn into_summary(self, now: DateTime<Utc>) -> StreamSummary {
        let rate_limit =
            rate_limit::classify(self.rejected.as_ref().or(self.latest_quota.as_ref()), now);
        let terminal_reason = self
            .result
            .as_ref()
            .and_then(|r| r.get("terminal_reason"))
            .and_then(Value::as_str)
            .map(str::to_owned);
        StreamSummary {
            result: self.result,
            rate_limit,
            terminal_reason,
        }
    }
}

/// Summarize a launch's stream-json lines (see [`StreamAccumulator`]).
pub fn summarize_stream<'a>(
    lines: impl Iterator<Item = &'a str>,
    now: DateTime<Utc>,
) -> StreamSummary {
    let mut acc = StreamAccumulator::default();
    for line in lines {
        acc.push(line);
    }
    acc.into_summary(now)
}

impl StreamSummary {
    /// Whether the last result line has subtype `success` (spike
    /// `classify_run_exit`); no result at all is `false`.
    pub fn result_ok(&self) -> bool {
        self.result
            .as_ref()
            .and_then(|r| r.get("subtype"))
            .and_then(Value::as_str)
            == Some("success")
    }
}

/// Whether `result` is an authentication failure (D-23 `[inferred — audit]`): an
/// ERROR result (`is_error` true or an `error_*` subtype) whose text, or one of its
/// `errors`, contains [`AUTH_FAILURE_MARKERS`]. The error requirement is what keeps
/// a successful run that merely wrote about authentication from being relaunched.
pub fn is_auth_failure(result: Option<&Value>) -> bool {
    let Some(Value::Object(map)) = result else {
        return false;
    };
    let is_error = map.get("is_error").and_then(Value::as_bool) == Some(true)
        || map
            .get("subtype")
            .and_then(Value::as_str)
            .is_some_and(|subtype| subtype.starts_with("error"));
    if !is_error {
        return false;
    }
    let mut texts: Vec<&str> = Vec::new();
    texts.extend(map.get("result").and_then(Value::as_str));
    if let Some(Value::Array(errors)) = map.get("errors") {
        texts.extend(errors.iter().filter_map(Value::as_str));
    }
    texts.iter().any(|text| {
        AUTH_FAILURE_MARKERS
            .iter()
            .any(|marker| text.contains(marker))
    })
}

/// The verdict for a launch end. Pure; the order is the module header's.
pub fn decide(end: &LaunchEnd) -> Verdict {
    if end.stop_requested {
        return Verdict::Terminal(TerminalStatus::Stopped, "stop:user".to_owned());
    }
    if end.tripped.is_some() {
        return Verdict::Keep;
    }
    if end.ended_by_driver && end.status.is_terminal() {
        return Verdict::Keep;
    }
    if end.ended_by_driver && end.status == RunStatus::Running {
        return Verdict::FreshRelaunch;
    }
    if is_auth_failure(end.stream.result.as_ref()) {
        return match end.token_source {
            TokenSource::Host if end.auth_relaunches < MAX_AUTH_RELAUNCHES => Verdict::AuthRelaunch,
            TokenSource::Host => Verdict::Park(format!(
                "auth: host token rejected after {MAX_AUTH_RELAUNCHES} relaunches"
            )),
            TokenSource::Stored => Verdict::Park(
                "auth: stored token rejected \u{2014} rerun driver auth setup".to_owned(),
            ),
        };
    }
    match &end.stream.rate_limit {
        QuotaVerdict::Rejected { window, resets_at } => {
            return Verdict::Park(format!(
                "rate-limit: {}",
                rate_limit::park_detail(window, *resets_at)
            ));
        }
        QuotaVerdict::Allowed => {
            if rate_limit::terminal_reason_names_a_rate_limit(end.stream.terminal_reason.as_deref())
            {
                return Verdict::Park(format!(
                    "rate-limit: {}",
                    rate_limit::park_detail(&QuotaWindow::Unknown(None), None)
                ));
            }
        }
    }
    match classify_exit(
        end.rc.unwrap_or(-1),
        false,
        false,
        end.stream.result_ok(),
        end.outstanding.as_deref(),
    ) {
        Some((ExitClass::Stalled, reason)) => Verdict::Terminal(TerminalStatus::Stalled, reason),
        Some((ExitClass::Crashed, reason)) => Verdict::Terminal(TerminalStatus::Crashed, reason),
        // Unreachable: classify_exit returns None only for a trip or a driver end,
        // both handled above. Keeping the status is the safe reading.
        None => Verdict::Keep,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::driver::rate_limit::QuotaWindow;
    use serde_json::json;

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).unwrap()
    }

    fn clean_stream() -> StreamSummary {
        StreamSummary {
            result: None,
            rate_limit: QuotaVerdict::Allowed,
            terminal_reason: None,
        }
    }

    fn revoked_result() -> Value {
        json!({
            "type": "result",
            "subtype": "success",
            "is_error": true,
            "result": "Failed to authenticate: OAuth token revoked",
        })
    }

    /// A launch that ended with rc 1 and nothing else to say about it.
    fn base() -> LaunchEnd {
        LaunchEnd {
            rc: Some(1),
            stop_requested: false,
            tripped: None,
            status: RunStatus::Running,
            ended_by_driver: false,
            stream: clean_stream(),
            token_source: TokenSource::Stored,
            auth_relaunches: 0,
            outstanding: None,
        }
    }

    fn rejected() -> QuotaVerdict {
        QuotaVerdict::Rejected {
            window: QuotaWindow::FiveHour,
            resets_at: DateTime::from_timestamp(1_800_003_600, 0),
        }
    }

    fn parked_reason(verdict: Verdict) -> String {
        match verdict {
            Verdict::Park(reason) => reason,
            other => panic!("expected a park, got {other:?}"),
        }
    }

    // -- the decision table, top to bottom

    #[test]
    fn a_requested_stop_beats_everything() {
        let end = LaunchEnd {
            stop_requested: true,
            tripped: Some("gap_rounds".to_owned()),
            status: RunStatus::Running,
            ended_by_driver: true,
            stream: StreamSummary {
                result: Some(revoked_result()),
                rate_limit: rejected(),
                terminal_reason: None,
            },
            ..base()
        };
        assert_eq!(
            decide(&end),
            Verdict::Terminal(TerminalStatus::Stopped, "stop:user".to_owned())
        );
    }

    #[test]
    fn a_tripped_cap_keeps_the_status_over_auth_rate_limit_and_crash() {
        let end = LaunchEnd {
            tripped: Some("wall_clock_min".to_owned()),
            status: RunStatus::Stopped,
            stream: StreamSummary {
                result: Some(revoked_result()),
                rate_limit: rejected(),
                terminal_reason: None,
            },
            ..base()
        };
        assert_eq!(decide(&end), Verdict::Keep);
    }

    #[test]
    fn a_terminal_driver_end_is_kept() {
        let end = LaunchEnd {
            status: RunStatus::Done,
            ended_by_driver: true,
            stream: StreamSummary {
                result: Some(revoked_result()),
                rate_limit: rejected(),
                terminal_reason: None,
            },
            ..base()
        };
        assert_eq!(decide(&end), Verdict::Keep);
    }

    #[test]
    fn fresh_exit_is_a_relaunch_even_if_the_stream_looks_like_an_auth_failure() {
        let end = LaunchEnd {
            rc: Some(0),
            status: RunStatus::Running,
            ended_by_driver: true,
            stream: StreamSummary {
                result: Some(revoked_result()),
                rate_limit: rejected(),
                terminal_reason: None,
            },
            ..base()
        };
        assert_eq!(decide(&end), Verdict::FreshRelaunch);
    }

    #[test]
    fn a_rejected_host_token_relaunches_at_most_twice() {
        let end = |relaunches| LaunchEnd {
            token_source: TokenSource::Host,
            auth_relaunches: relaunches,
            stream: StreamSummary {
                result: Some(revoked_result()),
                rate_limit: QuotaVerdict::Allowed,
                terminal_reason: None,
            },
            ..base()
        };
        assert_eq!(decide(&end(0)), Verdict::AuthRelaunch);
        assert_eq!(decide(&end(1)), Verdict::AuthRelaunch);
        assert_eq!(
            parked_reason(decide(&end(2))),
            "auth: host token rejected after 2 relaunches"
        );
        assert_eq!(
            parked_reason(decide(&end(7))),
            "auth: host token rejected after 2 relaunches"
        );
    }

    #[test]
    fn a_rejected_stored_token_parks_and_names_the_remedy() {
        let end = LaunchEnd {
            token_source: TokenSource::Stored,
            stream: StreamSummary {
                result: Some(revoked_result()),
                rate_limit: QuotaVerdict::Allowed,
                terminal_reason: None,
            },
            ..base()
        };
        assert_eq!(
            parked_reason(decide(&end)),
            "auth: stored token rejected \u{2014} rerun driver auth setup"
        );
    }

    #[test]
    fn an_auth_failure_is_decided_before_a_rate_limit() {
        let end = LaunchEnd {
            token_source: TokenSource::Host,
            stream: StreamSummary {
                result: Some(revoked_result()),
                rate_limit: rejected(),
                terminal_reason: Some("rate_limit".to_owned()),
            },
            ..base()
        };
        assert_eq!(decide(&end), Verdict::AuthRelaunch);
    }

    #[test]
    fn a_rejected_quota_parks_with_its_window_and_reset_time_and_is_not_retried() {
        let end = LaunchEnd {
            stream: StreamSummary {
                result: Some(json!({"type": "result", "subtype": "error_during_execution"})),
                rate_limit: rejected(),
                terminal_reason: None,
            },
            ..base()
        };
        let reason = parked_reason(decide(&end));
        assert!(
            reason.starts_with("rate-limit: window=five_hour resets_at="),
            "{reason}"
        );
        assert_eq!(
            reason,
            format!(
                "rate-limit: {}",
                crate::driver::rate_limit::park_detail(
                    &QuotaWindow::FiveHour,
                    DateTime::from_timestamp(1_800_003_600, 0)
                )
            )
        );
    }

    #[test]
    fn a_terminal_reason_naming_a_rate_limit_parks_with_an_unknown_window() {
        let end = LaunchEnd {
            stream: StreamSummary {
                result: Some(json!({"type": "result", "subtype": "error_during_execution"})),
                rate_limit: QuotaVerdict::Allowed,
                terminal_reason: Some("api_error: rate_limit_exceeded".to_owned()),
            },
            ..base()
        };
        assert_eq!(
            parked_reason(decide(&end)),
            "rate-limit: window=unknown resets_at=unknown"
        );
    }

    #[test]
    fn a_clean_exit_with_a_worker_outstanding_is_stalled() {
        let end = LaunchEnd {
            rc: Some(0),
            stream: StreamSummary {
                result: Some(json!({"type": "result", "subtype": "success"})),
                rate_limit: QuotaVerdict::Allowed,
                terminal_reason: None,
            },
            outstanding: Some("I-01".to_owned()),
            ..base()
        };
        match decide(&end) {
            Verdict::Terminal(TerminalStatus::Stalled, reason) => {
                assert!(reason.contains("I-01"), "{reason}");
            }
            other => panic!("expected stalled, got {other:?}"),
        }
    }

    #[test]
    fn anything_else_is_crashed() {
        for (rc, outstanding) in [(Some(1), None), (Some(137), Some("I-01")), (None, None)] {
            let end = LaunchEnd {
                rc,
                outstanding: outstanding.map(str::to_owned),
                ..base()
            };
            match decide(&end) {
                Verdict::Terminal(TerminalStatus::Crashed, reason) => {
                    assert!(reason.contains("without RUN end"), "{reason}");
                }
                other => panic!("{rc:?}: expected crashed, got {other:?}"),
            }
        }
        // rc 0 but no successful result and nothing outstanding is not a stall either.
        let end = LaunchEnd {
            rc: Some(0),
            ..base()
        };
        assert!(matches!(
            decide(&end),
            Verdict::Terminal(TerminalStatus::Crashed, _)
        ));
    }

    // -- summarize_stream

    fn summarize(lines: &[&str]) -> StreamSummary {
        summarize_stream(lines.iter().copied(), now())
    }

    const REJECTED: &str = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","rateLimitType":"five_hour","resetsAt":1800003600}}"#;
    const ALLOWED: &str = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","rateLimitType":"seven_day","resetsAt":1800090000}}"#;
    const WARNING: &str = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed_warning","rateLimitType":"seven_day","resetsAt":1800090000}}"#;

    #[test]
    fn a_rejection_followed_by_an_allowed_event_is_still_reported() {
        let summary = summarize(&[REJECTED, ALLOWED]);
        assert_eq!(summary.rate_limit, rejected());
    }

    #[test]
    fn the_first_rejection_is_the_one_that_is_latched() {
        let second = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","rateLimitType":"seven_day","resetsAt":1800090000}}"#;
        let summary = summarize(&[REJECTED, second]);
        assert_eq!(summary.rate_limit, rejected());
    }

    #[test]
    fn allowed_and_allowed_warning_do_not_park() {
        assert_eq!(summarize(&[ALLOWED]).rate_limit, QuotaVerdict::Allowed);
        assert_eq!(summarize(&[WARNING]).rate_limit, QuotaVerdict::Allowed);
        assert_eq!(summarize(&[]).rate_limit, QuotaVerdict::Allowed);
    }

    #[test]
    fn the_last_result_line_wins_and_supplies_the_terminal_reason() {
        let summary = summarize(&[
            r#"{"type":"system","subtype":"init"}"#,
            r#"{"type":"result","subtype":"success","terminal_reason":"completed"}"#,
            "not json at all",
            r#"{"type":"result","subtype":"error_during_execution","terminal_reason":"api_error"}"#,
        ]);
        let result = summary.result.expect("a result");
        assert_eq!(result["subtype"], "error_during_execution");
        assert_eq!(summary.terminal_reason.as_deref(), Some("api_error"));
    }

    #[test]
    fn a_stream_with_no_result_has_none() {
        let summary = summarize(&[r#"{"type":"assistant"}"#, "", "{"]);
        assert_eq!(summary.result, None);
        assert_eq!(summary.terminal_reason, None);
    }

    // -- is_auth_failure

    #[test]
    fn an_error_result_naming_a_revoked_token_or_a_failed_login_is_an_auth_failure() {
        assert!(is_auth_failure(Some(&revoked_result())));
        assert!(is_auth_failure(Some(&json!({
            "type": "result",
            "subtype": "error_during_execution",
            "is_error": true,
            "result": "API Error: OAuth token revoked",
        }))));
        assert!(is_auth_failure(Some(&json!({
            "type": "result",
            "subtype": "error_during_execution",
            "errors": ["Failed to authenticate: 401"],
        }))));
    }

    #[test]
    fn prose_in_a_successful_result_is_not_an_auth_failure() {
        assert!(!is_auth_failure(Some(&json!({
            "type": "result",
            "subtype": "success",
            "is_error": false,
            "result": "I explained that Failed to authenticate means OAuth token revoked.",
        }))));
        assert!(!is_auth_failure(Some(&json!({
            "type": "result",
            "subtype": "success",
        }))));
        assert!(!is_auth_failure(None));
    }
}
