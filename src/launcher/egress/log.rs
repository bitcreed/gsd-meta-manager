//! The header-free `egress.log` (28.1-01, D-05).
//!
//! One line per request: when, which kind, method (or `CONNECT host:port`), path
//! without query, status and a verdict phrase. Nothing else is ever written: no
//! header value, no token, no body, no resolved address. A refusal names the
//! address CLASS (`loopback`), never the address.

use std::io;
use std::path::Path;

/// What a request turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A `/v1/messages`-family request.
    Inference,
    /// A `CONNECT`.
    Connect,
    /// A request refused before it was classified further (unknown target, a
    /// head that never completed).
    Refused,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Inference => "inference",
            Kind::Connect => "connect",
            Kind::Refused => "refused",
        }
    }
}

/// RED stub: sanitised in the GREEN commit.
pub fn clean(text: &str) -> String {
    text.to_owned()
}

/// RED stub: formatted in the GREEN commit.
pub fn line(
    _timestamp: &str,
    _kind: Kind,
    _subject: &str,
    _path: &str,
    _status: u16,
    _verdict: &str,
) -> String {
    String::new()
}

/// RED stub: the writer lands in the GREEN commit.
pub struct EgressLog;

impl EgressLog {
    pub fn open(_path: &Path) -> io::Result<EgressLog> {
        Ok(EgressLog)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_derived_text_cannot_break_a_line_or_a_field() {
        assert_eq!(clean("a\nb\rc|d\te\u{7f}f\u{0}g"), "a b c d e f g");
        assert_eq!(clean("  padded  "), "padded");
        assert_eq!(clean(&"x".repeat(500)).len(), 200);
        assert_eq!(clean("é"), "é");
    }

    #[test]
    fn a_line_has_six_pipe_separated_fields_and_one_newline() {
        let l = line(
            "2026-10-24T20:00:00.000Z",
            Kind::Inference,
            "POST",
            "/v1/messages",
            200,
            "forwarded",
        );
        assert_eq!(
            l,
            "2026-10-24T20:00:00.000Z|inference|POST|/v1/messages|200|forwarded\n"
        );
        let hostile = line(
            "t",
            Kind::Refused,
            "GE\nT",
            "/a|b\r\n/c",
            403,
            "denied: x\ny",
        );
        assert_eq!(hostile.matches('\n').count(), 1, "{hostile:?}");
        assert!(hostile.ends_with('\n'));
        assert_eq!(hostile.trim_end().split('|').count(), 6, "{hostile:?}");
    }
}
