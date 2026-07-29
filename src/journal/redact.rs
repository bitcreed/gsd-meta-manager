//! The capture-path redaction seam (SAFE-04, D-21..D-27).
//!
//! Redaction happens **here**, before bytes reach the file, and never in
//! `src/ui/`. A retrofit would leave every log written before it lands
//! unredacted forever, and the user will `cat` or attach that file.
//!
//! The seam is held by a type, not by discipline (D-22): the journal writer
//! accepts only [`RedactedLine`], whose sole constructor runs the redactor. The
//! newtype has a private field, one crate-private accessor, and **implements no
//! traits at all** — no conversion, no dereference, no display, and no
//! test-only escape-hatch constructor. Each of those would reopen exactly the
//! seam this type closes.
//!
//! Three things about the pattern table are load-bearing and were settled by
//! executing it rather than by reading it:
//!
//! 1. **Order.** Rust's `regex` alternation is leftmost-**first**, not
//!    leftmost-longest, so the dash-encoded path forms precede the slash forms
//!    and the most specific credential shapes precede the generic ones.
//! 2. **One alternation, one pass.** A sequential list of `replace_all` calls is
//!    four times faster and measurably wrong: rule 9 was observed consuming rule
//!    8's `[REDACTED:bearer]` output. In a single alternation no rule can ever
//!    see another rule's output.
//! 3. **Idempotence is a test, not an argument.** `redact(redact(x)) ==
//!    redact(x)` over the whole corpus is the cheapest possible detector for "a
//!    replacement literal is itself redactable" and for "rule A ate rule B's
//!    output". It caught three real defects in one session that review had not.
//!
//! ## The honest limit (D-25)
//!
//! A pattern redactor **cannot** catch an arbitrary high-entropy secret with no
//! recognisable shape. Tuning is deliberately biased toward over-redaction — a
//! false positive costs a slightly less readable log, a false negative is a
//! persistent secret sink under `.planning/` — but total prose destruction is
//! not the goal, so an environment-variable *name* with no assignment after it
//! is left alone, and **a bare username outside a path context is deliberately
//! not redacted**: word-bounded username redaction would shred readable logs for
//! a marginal gain, because every path-shaped occurrence is already covered by
//! the generic shape rules and by the runtime-home layer.
//!
//! SAFE-04 is a capture-path control, **not a guarantee**. Phase 19's
//! tool-boundary denial is the complementary control that fires *before* a
//! secret enters context. Neither substitutes for the other.
//!
//! ## What this module will not grow (D-26)
//!
//! There is no raw sidecar file, no unredact path, and no verbose mode that
//! skips the filter. Any of the three would reintroduce the exact file SAFE-04
//! exists to prevent.
//!
//! ## What this module may log (D-28)
//!
//! Nothing that has not already been through [`redact`]. A `tracing` call in
//! this file carries counts, an already-redacted path, or no event content at
//! all — never a raw leaf, never a pre-redaction line, never the input that
//! triggered a match.

use std::sync::LazyLock;

use regex::{Captures, Regex};
use serde_json::{Map, Value};

use super::MAX_EVENT_PAYLOAD_BYTES;

/// `(group name, pattern, replacement literal)`, **in significant order**.
///
/// Replacement literals contain no quote, backslash or newline (D-24) and are
/// chosen so that no pattern in this table can match one — which is what the
/// idempotence test over the corpus verifies.
const PARTS: &[(&str, &str, &str)] = &[
    (
        "pem",
        r"-----BEGIN [A-Z ]*PRIVATE KEY-----(?s:.)*?-----END [A-Z ]*PRIVATE KEY-----",
        "[REDACTED:private-key]",
    ),
    // The header value scan runs to END OF LINE, not to whitespace. A
    // whitespace-terminated scan stops at the space after the scheme word, so
    // `authorization: Basic <base64>` redacted only the word `Basic` and wrote
    // the credential to disk. That is a false-NEGATIVE redaction — the SAFE-04
    // failure mode, not a cosmetic one.
    (
        "authz",
        r#"(?i:authorization)[ \t]*[:=][ \t]*"?[^\r\n"]{4,}"#,
        "[REDACTED:authorization]",
    ),
    (
        "env",
        r#"(?i:[A-Za-z_][A-Za-z0-9_]*(?:TOKEN|SECRET|PASSWORD|PASSWD|API_?KEY|_KEY|CREDENTIALS?))[ \t]*[:=][ \t]*"?[^\s"',}\]]+"#,
        "[REDACTED:env]",
    ),
    (
        "bearer",
        r"(?i:bearer)[ \t]+[A-Za-z0-9._\-+/=]{8,}",
        "[REDACTED:bearer]",
    ),
    ("skant", r"sk-ant-[A-Za-z0-9_\-]{8,}", "[REDACTED:anthropic-key]"),
    ("sk", r"\bsk-[A-Za-z0-9_\-]{16,}", "[REDACTED:api-key]"),
    (
        "ghpat",
        r"\bgithub_pat_[A-Za-z0-9_]{20,}",
        "[REDACTED:github-token]",
    ),
    ("gh", r"\bgh[pousr]_[A-Za-z0-9]{20,}", "[REDACTED:github-token]"),
    (
        "aws",
        r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b",
        "[REDACTED:aws-key-id]",
    ),
    (
        "slack",
        r"\bxox[baprs]-[A-Za-z0-9\-]{10,}",
        "[REDACTED:slack-token]",
    ),
    (
        "jwt",
        r"\beyJ[A-Za-z0-9_\-]{6,}\.[A-Za-z0-9_\-]{6,}\.[A-Za-z0-9_\-]{6,}",
        "[REDACTED:jwt]",
    ),
    (
        "uinfo",
        r#"(?:[A-Za-z][A-Za-z0-9+.\-]*)://[^/\s:@"]+:[^/\s@"]+@"#,
        "[REDACTED:userinfo]@",
    ),
    // ---- WR-15: the DASH-ENCODED forms must come BEFORE the slash forms.
    //      Claude Code encodes a session directory as `-home-<user>-<repo>`, and
    //      the original Phase 15 sweep matched only the slash form and missed
    //      this shape in seven of eight fixtures.
    ("dtmp", r"-tmp-claude-[^/\s\\\x22]*", "-tmp-scratch-"),
    (
        "dhome",
        r"-(?:home|Users|root)-[^/\s\\\x22]*",
        "-home-redacted-project",
    ),
    // ---- slash forms. `[` and `]` are EXCLUDED from the tail character class
    //      so the replacement literal can never be re-matched: without the
    //      exclusion, `/home/[REDACTED:user]` matched the home rule again,
    //      stopped at the `]` and re-appended one, degrading the log by a
    //      bracket per pass.
    ("stmp", r"/tmp/claude-[0-9]+", "/tmp/claude-[REDACTED:uid]"),
    (
        "shome",
        r#"/(?:home|Users|var/home)/[^/\s"':,)\[\]}\\]+"#,
        "/home/[REDACTED:user]",
    ),
];

/// The generic shape rules, compiled once as a single alternation.
///
/// `LazyLock` is stable since 1.80 and this crate's MSRV is 1.87, so the
/// compiled-once regex costs **no** new dependency.
static RE: LazyLock<Regex> = LazyLock::new(|| {
    let alt = PARTS
        .iter()
        .map(|(name, pattern, _)| format!("(?P<{name}>{pattern})"))
        .collect::<Vec<_>>()
        .join("|");
    Regex::new(&alt).expect("redaction alternation must compile")
});

/// The host-specific layer: this process's actual home path, both encodings.
///
/// The generic rules catch *any* user's home in a recognisable shape. This layer
/// catches the one home the generic shapes can miss — a containerised or
/// otherwise unusual root such as `/opt/me` — in both the slash form and Claude
/// Code's slash-to-dash transform of it (D-24, WR-15).
///
/// **Its replacements are the same literals the generic rules produce**, which
/// is what makes the two layers commute into one fixed point instead of fighting
/// each other.
///
/// `None` when the process has no discoverable home, in which case this layer is
/// inert and the generic rules carry the whole load.
static HOST_RE: LazyLock<Option<Regex>> = LazyLock::new(|| {
    let home = dirs::home_dir()?.to_str()?.to_owned();
    // A home of `/` or an empty home would match everything; refuse it.
    if home.len() < 2 {
        return None;
    }
    let dashed = home.replace('/', "-");
    let alt = format!(
        "(?P<hdash>{})|(?P<hslash>{})",
        regex::escape(&dashed),
        regex::escape(&home)
    );
    Regex::new(&alt).ok()
});

/// Replace every credential and path shape in `s` with a fixed literal.
///
/// The generic alternation runs **first** and the runtime-home layer second.
/// That ordering is deliberate and is the one place this implementation departs
/// from the plan's sketch: the host layer holds a bare literal home path, and
/// a bare `/home/<user>` is a *substring* of the Silverblue form
/// `/var/home/<user>`. Applying it first would rewrite the inner half and strip
/// the `/var/home/` context the generic rule needs, yielding
/// `/var/home/[REDACTED:user]` instead of `/home/[REDACTED:user]` — a
/// host-dependent result on exactly the machine whose home was matched. Running
/// the generic pass first removes the interaction entirely, and because both
/// layers emit the *same* replacement literals, the second pass is a no-op on
/// whatever the first produced.
pub fn redact(s: &str) -> String {
    let generic = RE.replace_all(s, |caps: &Captures| {
        for (name, _, replacement) in PARTS {
            if caps.name(name).is_some() {
                return (*replacement).to_string();
            }
        }
        unreachable!("a match with no named group")
    });

    match HOST_RE.as_ref() {
        None => generic.into_owned(),
        Some(host) => host
            .replace_all(&generic, |caps: &Captures| {
                if caps.name("hdash").is_some() {
                    "-home-redacted-project"
                } else {
                    "/home/[REDACTED:user]"
                }
                .to_string()
            })
            .into_owned(),
    }
}

/// Rewrite every string leaf **and every object key** in a JSON tree (D-23).
///
/// Walking the tree rather than regex-replacing the finished line is what makes
/// this structurally incapable of producing invalid JSON: numbers, booleans and
/// null carry no text and are left untouched **by construction**, and no
/// replacement can ever land inside an escape sequence or a delimiter. Post-
/// serialisation replacement has neither property.
///
/// Keys are rewritten because `memory_paths`-style maps put paths in **key**
/// position, and the actual lesson of the WR-15 leak is that the leak was inside
/// a payload nobody thought to scan.
pub fn redact_value(v: &mut Value) {
    match v {
        Value::String(s) => {
            let redacted = redact(s);
            if redacted != *s {
                *s = redacted;
            }
        }
        Value::Array(a) => a.iter_mut().for_each(redact_value),
        Value::Object(m) => {
            // `serde_json::Map` has no rename-key API, so the map is drained and
            // rebuilt. `mem::take` avoids cloning every value.
            let mut out = Map::with_capacity(m.len());
            for (k, mut val) in std::mem::take(m) {
                redact_value(&mut val);
                let new_key = redact(&k);
                match out.entry(new_key.clone()) {
                    serde_json::map::Entry::Vacant(e) => {
                        e.insert(val);
                    }
                    // Two distinct keys can collapse to the same redacted key —
                    // two users' home paths reduce to one literal. Disambiguate;
                    // never let an insert silently overwrite, which would lose
                    // an event field outright.
                    serde_json::map::Entry::Occupied(_) => {
                        let mut i = 2usize;
                        loop {
                            let candidate = format!("{new_key}#{i}");
                            if !out.contains_key(&candidate) {
                                out.insert(candidate, val);
                                break;
                            }
                            i += 1;
                        }
                    }
                }
            }
            *m = out;
        }
        // Numbers, bools and null carry no text.
        _ => {}
    }
}

/// Truncate over-long string leaves at a character boundary (D-31).
///
/// Byte slicing (`&s[..max]`) **panics** on a multi-byte boundary, so the cut is
/// found with `char_indices`. The marker mirrors the field shape of the
/// executor's `ExecutionEvent::LineTruncated { bytes, prefix }` so truncation is
/// visible in the file rather than silent.
fn cap_payload(v: &mut Value, max: usize) {
    match v {
        Value::String(s) => {
            if s.len() > max {
                let keep = s
                    .char_indices()
                    .map(|(i, c)| i + c.len_utf8())
                    .take_while(|end| *end <= max)
                    .last()
                    .unwrap_or(0);
                let removed = s.len() - keep;
                s.truncate(keep);
                s.push_str(&format!("…[truncated {removed} bytes]"));
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|item| cap_payload(item, max)),
        Value::Object(m) => m.values_mut().for_each(|val| cap_payload(val, max)),
        _ => {}
    }
}

/// The only value the journal writer accepts (D-22).
///
/// Its sole constructor runs the redactor, the inner `String` is private, the
/// accessor is crate-private, and the type implements no traits whatsoever.
/// Together those mean there is **no compilable path from a bare `String` to a
/// written journal line** — the compiler holds the seam, not a code review.
///
/// This follows `DrivableProject` (`src/executor/mod.rs:109-140`) with one
/// deliberate deviation: that type carries a loudly-named test escape hatch, and
/// this one must not, because such a constructor would reopen exactly the seam
/// the type exists to close.
pub struct RedactedLine(String);

impl RedactedLine {
    /// Redact, cap, then serialise — **in that order**, which is load-bearing.
    ///
    /// Capping *after* redacting is what stops a truncation boundary from
    /// splitting a secret so that it no longer matches a pattern. Capping the
    /// tree rather than the serialised string is what keeps the output valid
    /// JSON. And `to_string` rather than its pretty variant is what keeps NDJSON
    /// framing intact: pretty output emits embedded newlines and would break the
    /// format outright.
    pub fn new(mut v: Value) -> Self {
        redact_value(&mut v);
        cap_payload(&mut v, MAX_EVENT_PAYLOAD_BYTES);
        RedactedLine(serde_json::to_string(&v).expect("a Value always serialises"))
    }

    /// The redacted line, without its newline.
    pub(crate) fn as_line(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_only_constructor_redacts_before_it_serialises() {
        let value = serde_json::json!({
            "text": "key sk-ant-api03-AbCdEf012345_-XyZ end",
        });
        let line = RedactedLine::new(value);
        assert!(!line.as_line().contains("sk-ant-api03"));
        assert!(line.as_line().contains("[REDACTED:anthropic-key]"));
        assert!(
            serde_json::from_str::<Value>(line.as_line()).is_ok(),
            "every produced line must reparse as JSON"
        );
    }
}
