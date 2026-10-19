//! The `LOG.md` event grammar: the single owner of the header format.
//!
//! One event is a header line `- <utc> | <item> | <event>[ | k=v]...` followed
//! by zero or more four-space-indented detail lines. Later plans route every
//! header value through [`one_line`] so a value can never forge a second event.
//!
//! Ported from the spike tools: `format_line`/`append`/`summary` from
//! `driverctl.py` (`append_log`, `summary`, `one_line`) and `parse_log` from
//! `dup_check.py` (`EV_RE`), extended to capture detail lines.

use std::io::{self, Write as _};
use std::path::Path;
use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use regex::Regex;

/// The `last_event.summary` cap (characters).
const SUMMARY_MAX_CHARS: usize = 160;

static EV_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^- (\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z) \| (\S+) \| (\S+)((?: \| .*)?)$")
        .expect("static regex")
});

/// One parsed LOG event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEvent {
    /// The UTC timestamp text, as written.
    pub at: String,
    /// `RUN` or an item id such as `I-01`.
    pub item: String,
    pub event: String,
    /// `k=v` header pairs in order, both sides trimmed.
    pub kv: Vec<(String, String)>,
    /// Four-space-indented lines under the header, indent stripped.
    pub detail: Vec<String>,
}

impl LogEvent {
    /// The first value recorded under `key`.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.kv
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

/// `%Y-%m-%dT%H:%M:%SZ`.
pub fn utc_string(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// RFC 3339 UTC text (`Z`, optional fractional seconds) to float epoch seconds;
/// `None` on any parse error (the IN-02 posture: a bad stamp is bad input, not a panic).
pub fn epoch_seconds(utc: &str) -> Option<f64> {
    let dt = DateTime::parse_from_rfc3339(utc).ok()?;
    Some(dt.timestamp() as f64 + f64::from(dt.timestamp_subsec_nanos()) / 1e9)
}

/// One event as the exact bytes `append` writes: the header, `\n`, then each
/// detail line indented four spaces. `None` values are omitted.
pub fn format_line(
    at: &str,
    item: &str,
    event: &str,
    kv: &[(&str, Option<String>)],
    detail: &[String],
) -> String {
    let mut out = format!("- {at} | {item} | {event}");
    for (k, v) in kv {
        if let Some(v) = v {
            out.push_str(" | ");
            out.push_str(k);
            out.push('=');
            out.push_str(v);
        }
    }
    out.push('\n');
    for line in detail {
        out.push_str("    ");
        out.push_str(line);
        out.push('\n');
    }
    out
}

/// Append `data` with a single `write_all` on an append-mode handle, so a
/// reader never observes a torn event. Creates the file (mode 0644 on Unix).
pub fn append(path: &Path, data: &str) -> io::Result<()> {
    let mut opts = std::fs::OpenOptions::new();
    opts.append(true).create(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o644);
    }
    let mut f = opts.open(path)?;
    f.write_all(data.as_bytes())
}

/// The `last_event.summary`: `k=v` for each `Some` value joined by a space,
/// truncated to 160 characters (char-safe).
pub fn summary(kv: &[(&str, Option<String>)]) -> String {
    let joined = kv
        .iter()
        .filter_map(|(k, v)| v.as_ref().map(|v| format!("{k}={v}")))
        .collect::<Vec<_>>()
        .join(" ");
    joined.chars().take(SUMMARY_MAX_CHARS).collect()
}

/// Parse every event in a LOG; lines that match no event header (the title,
/// blank lines, prose) are skipped. Four-space-indented lines directly under an
/// event become its `detail`.
pub fn parse_log(text: &str) -> Vec<LogEvent> {
    let mut out: Vec<LogEvent> = Vec::new();
    let mut open = false;
    for line in text.lines() {
        if let Some(m) = EV_RE.captures(line) {
            let kv = m[4]
                .split(" | ")
                .skip(1)
                .filter_map(|seg| {
                    let (k, v) = seg.split_once('=')?;
                    Some((k.trim().to_owned(), v.trim().to_owned()))
                })
                .collect();
            out.push(LogEvent {
                at: m[1].to_owned(),
                item: m[2].to_owned(),
                event: m[3].to_owned(),
                kv,
                detail: Vec::new(),
            });
            open = true;
        } else if let (true, Some(rest), Some(last)) =
            (open, line.strip_prefix("    "), out.last_mut())
        {
            last.detail.push(rest.to_owned());
        } else {
            open = false;
        }
    }
    out
}

/// Validate one header value: required, a single line, no `|`, no control
/// character other than tab, at most `limit` characters; returns it trimmed. The
/// error text names the flag as `--<name>`.
///
/// CR is rejected as well as LF `[inferred - audit]`: the spike rejected only
/// LF, but a bare CR is a line break to some readers and would forge an event.
/// WR-06 `[inferred - audit]`: every other control character is rejected too (ESC
/// would inject terminal escapes when a human `cat`s the log; NUL, VT, FF, FS-RS and
/// U+0085 are line breaks to some readers, as are U+2028 and U+2029), matching the
/// launcher-side `header_safe` and the report-body rule. Tab stays allowed.
pub fn one_line(name: &str, value: Option<&str>, limit: usize) -> Result<String, String> {
    let v = match value {
        Some(v) if !v.trim().is_empty() => v,
        _ => return Err(format!("--{name} is required")),
    };
    let unsafe_char =
        |c: char| c == '|' || c == '\u{2028}' || c == '\u{2029}' || (c.is_control() && c != '\t');
    if v.chars().any(unsafe_char) || v.chars().count() > limit {
        return Err(format!(
            "--{name} must be one line, no '|', <= {limit} chars"
        ));
    }
    Ok(v.trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn s(v: &str) -> Option<String> {
        Some(v.to_owned())
    }

    #[test]
    fn format_line_omits_none_and_keeps_order() {
        let got = format_line(
            "2026-10-18T02:07:06Z",
            "RUN",
            "gap-observed",
            &[("phase", s("1")), ("count", s("1")), ("x", None)],
            &[],
        );
        assert_eq!(
            got,
            "- 2026-10-18T02:07:06Z | RUN | gap-observed | phase=1 | count=1\n"
        );
    }

    #[test]
    fn detail_lines_are_indented_four_spaces() {
        let got = format_line(
            "2026-10-18T02:07:06Z",
            "I-01",
            "report",
            &[("status", s("done"))],
            &["first".to_owned(), "second".to_owned()],
        );
        assert_eq!(
            got,
            "- 2026-10-18T02:07:06Z | I-01 | report | status=done\n    first\n    second\n"
        );
    }

    #[test]
    fn parse_log_round_trips_format_line_and_captures_detail() {
        let mut text = String::from("# Run LOG\n\nprose that is not an event\n");
        text.push_str(&format_line(
            "2026-10-18T02:07:06Z",
            "I-01",
            "dispatched",
            &[("step", s("plan")), ("n", s("2"))],
            &["a detail".to_owned(), "another".to_owned()],
        ));
        text.push_str("- not an event line\n");
        text.push_str(&format_line(
            "2026-10-18T02:07:09.250Z",
            "RUN",
            "end",
            &[("status", s("done"))],
            &[],
        ));
        let ev = parse_log(&text);
        assert_eq!(ev.len(), 2);
        assert_eq!(ev[0].at, "2026-10-18T02:07:06Z");
        assert_eq!(ev[0].item, "I-01");
        assert_eq!(ev[0].event, "dispatched");
        assert_eq!(
            ev[0].kv,
            vec![
                ("step".to_owned(), "plan".to_owned()),
                ("n".to_owned(), "2".to_owned())
            ]
        );
        assert_eq!(ev[0].detail, vec!["a detail", "another"]);
        assert_eq!(ev[1].at, "2026-10-18T02:07:09.250Z");
        assert_eq!(ev[1].get("status"), Some("done"));
        assert!(ev[1].detail.is_empty());
    }

    #[test]
    fn detail_does_not_leak_past_a_blank_line() {
        let text = "- 2026-10-18T02:07:06Z | RUN | start\n\n    orphan\n";
        let ev = parse_log(text);
        assert_eq!(ev.len(), 1);
        assert!(ev[0].detail.is_empty());
    }

    #[test]
    fn one_line_enforces_required_single_line_no_pipe_and_limit() {
        assert!(one_line("step", Some("a|b"), 200).is_err());
        assert!(one_line("step", Some("a\nb"), 200).is_err());
        assert!(one_line("step", Some("a\rb"), 200).is_err());
        // WR-06: any control character or Unicode line separator, not just CR/LF.
        for bad in [
            "a\u{1b}[31mb",
            "a\0b",
            "a\u{0b}b",
            "a\u{0c}b",
            "a\u{1c}b",
            "a\u{85}b",
            "a\u{2028}b",
            "a\u{2029}b",
            "a\u{7f}b",
        ] {
            assert!(one_line("step", Some(bad), 200).is_err(), "{bad:?}");
        }
        assert_eq!(one_line("step", Some("a\tb"), 200).unwrap(), "a\tb");
        assert_eq!(one_line("step", Some("déjà vu"), 200).unwrap(), "déjà vu");
        assert!(one_line("step", None, 200).is_err());
        assert!(one_line("step", Some("   "), 200).is_err());
        assert!(one_line("step", Some(&"x".repeat(201)), 200).is_err());
        assert_eq!(
            one_line("step", Some(&"x".repeat(200)), 200).unwrap().len(),
            200
        );
        assert_eq!(one_line("step", Some("  x  "), 200).unwrap(), "x");
        assert_eq!(
            one_line("step", Some("a|b"), 200).unwrap_err(),
            "--step must be one line, no '|', <= 200 chars"
        );
        assert_eq!(
            one_line("step", None, 200).unwrap_err(),
            "--step is required"
        );
    }

    #[test]
    fn epoch_seconds_parses_z_and_fractions_and_rejects_garbage() {
        assert_eq!(epoch_seconds("1970-01-01T00:00:01Z"), Some(1.0));
        assert_eq!(epoch_seconds("1970-01-01T00:00:01.5Z"), Some(1.5));
        assert_eq!(epoch_seconds("2026-13-40T25:61:61Z"), None);
        assert_eq!(epoch_seconds("not a time"), None);
        assert_eq!(epoch_seconds(""), None);
    }

    #[test]
    fn utc_string_formats_whole_seconds_with_z() {
        let dt = Utc.with_ymd_and_hms(2026, 10, 18, 2, 7, 6).unwrap();
        assert_eq!(utc_string(dt), "2026-10-18T02:07:06Z");
    }

    #[test]
    fn summary_joins_some_values_and_truncates_by_chars() {
        assert_eq!(
            summary(&[("a", s("1")), ("b", None), ("c", s("3"))]),
            "a=1 c=3"
        );
        let long = "\u{e9}".repeat(300);
        let got = summary(&[("k", Some(long))]);
        assert_eq!(got.chars().count(), 160);
        assert!(got.starts_with("k=\u{e9}"));
    }

    #[test]
    fn append_creates_then_appends_whole_events() {
        let td = tempfile::TempDir::new().unwrap();
        let p = td.path().join("LOG.md");
        let a = format_line("2026-10-18T02:07:06Z", "RUN", "start", &[], &[]);
        let b = format_line(
            "2026-10-18T02:07:07Z",
            "I-01",
            "report",
            &[],
            &["d".to_owned()],
        );
        append(&p, &a).unwrap();
        append(&p, &b).unwrap();
        assert_eq!(std::fs::read_to_string(&p).unwrap(), format!("{a}{b}"));
        assert_eq!(parse_log(&format!("{a}{b}")).len(), 2);
    }
}
