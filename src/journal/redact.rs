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
//! 4. **Class, not a second table** (Phase 19, D-12). Each rule carries a
//!    [`SecretClass`], and there are **two consumers of the one table**:
//!    [`redact`] reads every rule, and the pre-push credential scanner
//!    ([`crate::envelope::scan`], via [`credential_alternation`]) reads only the
//!    `Credential` ones. The reason is concrete rather than tidy.
//!    `/home/<user>` and Claude Code's dash-encoded `-home-<user>-<repo>` form
//!    are redacted from logs **for privacy**, and a source file containing a
//!    home-directory string is not a secret. A scanner that blocked every push
//!    over one would be switched off within a day, and a control that gets
//!    switched off is worse than one that was never claimed. Redaction
//!    behaviour is **unchanged** by the split — the corpus test above and its
//!    idempotence sibling pin that, and neither was edited to accommodate it.
//!    **A new rule is still added in exactly one place**: a row in [`PARTS`],
//!    tagged with the class that says which consumers read it.
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

/// What a rule in [`PARTS`] is for, and therefore which consumer reads it.
///
/// One table, two consumers (D-12). See point 4 of the module doc for why the
/// split is a class tag on the existing table rather than a second table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretClass {
    /// A credential shape. Read by **both** [`redact`] and the pre-push
    /// scanner ([`crate::envelope::scan`]); a match blocks a push.
    Credential,
    /// A privacy shape — a home directory, a scratch path. Read by [`redact`]
    /// **only**; a match never blocks anything.
    PathHygiene,
}

/// `(group name, pattern, replacement literal, class)`, **in significant
/// order**.
///
/// Replacement literals contain no quote, backslash or newline (D-24) and are
/// chosen so that no pattern in this table can match one — which is what the
/// idempotence test over the corpus verifies.
///
/// The class is the fourth member rather than a second table, and the order is
/// **unchanged** by the split: see the WR-15 note below, which records that a
/// naive reorder silently regressed seven of eight fixtures.
const PARTS: &[(&str, &str, &str, SecretClass)] = &[
    (
        "pem",
        r"-----BEGIN [A-Z ]*PRIVATE KEY-----(?s:.)*?-----END [A-Z ]*PRIVATE KEY-----",
        "[REDACTED:private-key]",
        SecretClass::Credential,
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
        SecretClass::Credential,
    ),
    (
        "env",
        r#"(?i:[A-Za-z_][A-Za-z0-9_]*(?:TOKEN|SECRET|PASSWORD|PASSWD|API_?KEY|_KEY|CREDENTIALS?))[ \t]*[:=][ \t]*"?[^\s"',}\]]+"#,
        "[REDACTED:env]",
        SecretClass::Credential,
    ),
    (
        "bearer",
        r"(?i:bearer)[ \t]+[A-Za-z0-9._\-+/=]{8,}",
        "[REDACTED:bearer]",
        SecretClass::Credential,
    ),
    (
        "skant",
        r"sk-ant-[A-Za-z0-9_\-]{8,}",
        "[REDACTED:anthropic-key]",
        SecretClass::Credential,
    ),
    (
        "sk",
        r"\bsk-[A-Za-z0-9_\-]{16,}",
        "[REDACTED:api-key]",
        SecretClass::Credential,
    ),
    (
        "ghpat",
        r"\bgithub_pat_[A-Za-z0-9_]{20,}",
        "[REDACTED:github-token]",
        SecretClass::Credential,
    ),
    (
        "gh",
        r"\bgh[pousr]_[A-Za-z0-9]{20,}",
        "[REDACTED:github-token]",
        SecretClass::Credential,
    ),
    (
        "aws",
        r"\b(?:AKIA|ASIA)[0-9A-Z]{16}\b",
        "[REDACTED:aws-key-id]",
        SecretClass::Credential,
    ),
    (
        "slack",
        r"\bxox[baprs]-[A-Za-z0-9\-]{10,}",
        "[REDACTED:slack-token]",
        SecretClass::Credential,
    ),
    (
        "jwt",
        r"\beyJ[A-Za-z0-9_\-]{6,}\.[A-Za-z0-9_\-]{6,}\.[A-Za-z0-9_\-]{6,}",
        "[REDACTED:jwt]",
        SecretClass::Credential,
    ),
    (
        "uinfo",
        r#"(?:[A-Za-z][A-Za-z0-9+.\-]*)://[^/\s:@"]+:[^/\s@"]+@"#,
        "[REDACTED:userinfo]@",
        SecretClass::Credential,
    ),
    // ---- WR-15: the DASH-ENCODED forms must come BEFORE the slash forms.
    //      Claude Code encodes a session directory as `-home-<user>-<repo>`, and
    //      the original Phase 15 sweep matched only the slash form and missed
    //      this shape in seven of eight fixtures.
    (
        "dtmp",
        r"-tmp-claude-[^/\s\\\x22]*",
        "-tmp-scratch-",
        SecretClass::PathHygiene,
    ),
    (
        "dhome",
        r"-(?:home|Users|root)-[^/\s\\\x22]*",
        "-home-redacted-project",
        SecretClass::PathHygiene,
    ),
    // ---- slash forms. `[` and `]` are EXCLUDED from the tail character class
    //      so the replacement literal can never be re-matched: without the
    //      exclusion, `/home/[REDACTED:user]` matched the home rule again,
    //      stopped at the `]` and re-appended one, degrading the log by a
    //      bracket per pass.
    (
        "stmp",
        r"/tmp/claude-[0-9]+",
        "/tmp/claude-[REDACTED:uid]",
        SecretClass::PathHygiene,
    ),
    (
        "shome",
        r#"/(?:home|Users|var/home)/[^/\s"':,)\[\]}\\]+"#,
        "/home/[REDACTED:user]",
        SecretClass::PathHygiene,
    ),
];

/// The generic shape rules, compiled once as a single alternation.
///
/// `LazyLock` is stable since 1.80 and this crate's MSRV is 1.87, so the
/// compiled-once regex costs **no** new dependency.
static RE: LazyLock<Regex> = LazyLock::new(|| build_alternation(|_| true));

/// One alternation over the rules `keep` selects, with the same
/// `(?P<name>pattern)` builder for every consumer.
///
/// One builder rather than two, so the two alternations cannot drift in how a
/// match is identified: both report the rule through a **named capture group**,
/// which is what lets a caller name the rule that fired without re-scanning the
/// input once per rule.
fn build_alternation(keep: impl Fn(SecretClass) -> bool) -> Regex {
    let alt = PARTS
        .iter()
        .filter(|(_, _, _, class)| keep(*class))
        .map(|(name, pattern, _, _)| format!("(?P<{name}>{pattern})"))
        .collect::<Vec<_>>()
        .join("|");
    Regex::new(&alt).expect("redaction alternation must compile")
}

/// The `Credential`-tagged subset of [`PARTS`], compiled once (D-12).
///
/// This is the **scanner's** view of the table: a match here blocks a push. It
/// is deliberately narrower than [`redact`]'s view — see point 4 of the module
/// doc for the reason, which is concrete rather than tidy.
///
/// Compiled once in a `LazyLock`, exactly as the full alternation is, so a
/// worktree scan pays the compile cost once no matter how many files it reads.
pub fn credential_alternation() -> &'static Regex {
    static CREDENTIAL_RE: LazyLock<Regex> =
        LazyLock::new(|| build_alternation(|class| class == SecretClass::Credential));
    &CREDENTIAL_RE
}

/// The rule names [`credential_alternation`] can report, in table order.
///
/// Exported so [`crate::envelope::scan`] can name the rule that fired **without
/// importing the pattern strings** — a scanner that held its own copy of the
/// patterns would be the second table D-12 exists to prevent.
pub fn credential_rule_names() -> &'static [&'static str] {
    static NAMES: LazyLock<Vec<&'static str>> = LazyLock::new(|| {
        PARTS
            .iter()
            .filter(|(_, _, _, class)| *class == SecretClass::Credential)
            .map(|(name, _, _, _)| *name)
            .collect()
    });
    &NAMES
}

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
        for (name, _, replacement, _) in PARTS {
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

    /// The credential and path shapes this redactor is pinned to, as
    /// `(name, input, expected)`.
    ///
    /// Every row asserts an **exact** output. A row asserting merely that the
    /// output changed would pass against a redactor that mangles prose.
    ///
    /// Six rows are the WR-15 cases and are deliberately separate rather than
    /// folded into one general "path" case: `tests/fixtures/transcripts/README.md`
    /// records that the original Phase 15 sweep matched only the slash form and
    /// missed the dash-encoded shapes in **seven of eight** fixtures, and
    /// concludes that any future sweep must scan both encodings (D-24). A grep
    /// for the slash form alone is not evidence of a clean capture.
    ///
    /// Three rows expect their input back verbatim. Those are as load-bearing as
    /// the matches: D-25's tuning direction is over-redaction, but total prose
    /// destruction is not the goal.
    const CORPUS: &[(&str, &str, &str)] = &[
        // ---- paths, slash encoding ----
        (
            "slash home",
            "cwd is /home/blk/projects/rust/gsd-meta-manager/src",
            "cwd is /home/[REDACTED:user]/projects/rust/gsd-meta-manager/src",
        ),
        (
            "macos home",
            "cwd is /Users/andy/Code/thing",
            "cwd is /home/[REDACTED:user]/Code/thing",
        ),
        (
            "silverblue home",
            "cwd is /var/home/blk/projects/x",
            "cwd is /home/[REDACTED:user]/projects/x",
        ),
        (
            "slash tmp scratch",
            "scratch /tmp/claude-1000/work",
            "scratch /tmp/claude-[REDACTED:uid]/work",
        ),
        // ---- paths, dash encoding (WR-15) ----
        (
            "dash home nested in a slash path",
            "/home/blk/.claude/projects/-home-blk-projects-rust-gsd-meta-manager/x.jsonl",
            "/home/[REDACTED:user]/.claude/projects/-home-redacted-project/x.jsonl",
        ),
        (
            "dash home under a slash tmp path",
            "/tmp/x/-home-blk-projects-y/z",
            "/tmp/x/-home-redacted-project/z",
        ),
        (
            "dash users home",
            "sess dir -Users-andy-Code-thing here",
            "sess dir -home-redacted-project here",
        ),
        (
            "dash tmp scratch directory",
            "/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/4661fdcd/scratchpad",
            "/tmp/claude-[REDACTED:uid]/-home-redacted-project/4661fdcd/scratchpad",
        ),
        (
            "dash tmp claude session prefix",
            "sess -tmp-claude-1000--home-blk-projects-x/memory/",
            "sess -tmp-scratch-/memory/",
        ),
        // ---- credentials ----
        (
            "anthropic key",
            "key sk-ant-api03-AbCdEf012345_-XyZ end",
            "key [REDACTED:anthropic-key] end",
        ),
        (
            "generic provider key",
            "OPENAI sk-proj-abcdefghijklmnopqrstuvwxyz012345 end",
            "OPENAI [REDACTED:api-key] end",
        ),
        (
            "github classic token",
            "token ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345 end",
            "token [REDACTED:github-token] end",
        ),
        (
            "github fine-grained token",
            "github_pat_11ABCDEFG0abcdefghijklmnop_qrstuvwxyz01234",
            "[REDACTED:github-token]",
        ),
        (
            "aws access key id",
            "AWS AKIAIOSFODNN7EXAMPLE here",
            "AWS [REDACTED:aws-key-id] here",
        ),
        (
            "slack token",
            "xoxb-1234567890-abcdefghijkl",
            "[REDACTED:slack-token]",
        ),
        (
            "jwt",
            "tok eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U end",
            "tok [REDACTED:jwt] end",
        ),
        (
            "url userinfo",
            "clone https://andy:hunter2@github.com/org/repo.git",
            "clone [REDACTED:userinfo]@github.com/org/repo.git",
        ),
        (
            "pem private key block",
            "-----BEGIN RSA PRIVATE KEY-----\nMIIBOgIBAAJBAKj34GkxFhD9\nabcdefgh\n-----END RSA PRIVATE KEY-----",
            "[REDACTED:private-key]",
        ),
        // ---- headers. `authz basic` is the leak-1 regression row: a value scan
        //      that stopped at whitespace redacted only the scheme word and
        //      wrote the base64 credential to disk.
        (
            "authorization bearer header",
            "Authorization: Bearer abc123def456ghi789",
            "[REDACTED:authorization]",
        ),
        (
            "authorization basic header with base64",
            "authorization: Basic dXNlcjpwYXNzd29yZA==",
            "[REDACTED:authorization]",
        ),
        (
            "bare bearer scheme",
            "hdr was Bearer abc123def456ghi789 ok",
            "hdr was [REDACTED:bearer] ok",
        ),
        // ---- environment assignments ----
        (
            "env token",
            "GITHUB_TOKEN=ghp_zzzzzzzzzzzzzzzzzzzzzzzzzz",
            "[REDACTED:env]",
        ),
        (
            "env secret",
            "MY_SECRET=supersecretvalue rest",
            "[REDACTED:env] rest",
        ),
        (
            "env anthropic key",
            "ANTHROPIC_API_KEY=sk-ant-xxxxxxxxxxxxxxxx",
            "[REDACTED:env]",
        ),
        (
            "env password",
            "DB_PASSWORD: hunter2 and more",
            "[REDACTED:env] and more",
        ),
        // ---- structured payload ----
        (
            "path in a json leaf",
            r#"{"memory_paths":{"/home/blk/.claude/CLAUDE.md":1}}"#,
            r#"{"memory_paths":{"/home/[REDACTED:user]/.claude/CLAUDE.md":1}}"#,
        ),
        // ---- deliberate NON-matches (D-25) ----
        (
            "prose naming a planning path",
            "The phase writes .planning/meta-manager/runs/ and is fine.",
            "The phase writes .planning/meta-manager/runs/ and is fine.",
        ),
        (
            "url with a port but no userinfo",
            "see https://example.com:8443/path/to/x",
            "see https://example.com:8443/path/to/x",
        ),
        (
            "env var name with no assignment",
            "the API_KEY variable is documented",
            "the API_KEY variable is documented",
        ),
    ];

    #[test]
    fn every_corpus_case_redacts_exactly_as_specified() {
        assert!(
            CORPUS.len() >= 27,
            "the corpus is a floor, not a sample: {} rows",
            CORPUS.len()
        );

        for (name, input, expected) in CORPUS {
            assert_eq!(&redact(input), expected, "corpus row `{name}`");
        }

        // The leak-1 regression, asserted by name rather than by hoping a row
        // covers it: the base64 must be gone, not merely the scheme word.
        let (_, basic_input, basic_expected) = CORPUS
            .iter()
            .find(|(name, _, _)| *name == "authorization basic header with base64")
            .expect("the header row must stay in the corpus");
        assert!(basic_input.contains("dXNlcjpwYXNzd29yZA=="));
        assert!(
            !basic_expected.contains("dXNlcjpwYXNzd29yZA=="),
            "the credential after the scheme word must not survive"
        );

        // The deliberate non-matches, counted so a later edit cannot quietly
        // turn the redactor into a prose shredder.
        let unchanged = CORPUS
            .iter()
            .filter(|(_, input, expected)| input == expected)
            .count();
        assert!(
            unchanged >= 3,
            "at least three rows must expect their input back verbatim, found {unchanged}"
        );
    }

    #[test]
    fn redaction_is_idempotent_over_the_whole_corpus() {
        // The cheapest possible detector for "a replacement literal is itself
        // redactable" and for "rule A ate rule B's output". It iterates the same
        // CORPUS the exactness test uses, so a row added there is automatically
        // covered here.
        for (name, input, _) in CORPUS {
            let once = redact(input);
            let twice = redact(&once);
            assert_eq!(once, twice, "corpus row `{name}` is not a fixed point");
        }
    }

    #[test]
    fn object_keys_are_redacted_and_collisions_are_disambiguated() {
        let mut value = serde_json::json!({
            "text": "run with GITHUB_TOKEN=ghp_zzzzzzzzzzzzzzzzzzzzzzzzzz in /home/blk/p",
            "memory_paths": {
                "/home/blk/.claude/CLAUDE.md": 1,
                "/home/andy/.claude/CLAUDE.md": 2
            },
            "nested": [ { "cwd": "/home/blk/x" }, [ "/home/blk/y" ] ],
            "cost": 1.83,
            "ok": true,
            "nil": null
        });
        redact_value(&mut value);

        let base = "/home/[REDACTED:user]/.claude/CLAUDE.md";
        let paths = value["memory_paths"]
            .as_object()
            .expect("memory_paths stays an object");
        assert!(paths.contains_key(base), "keys must be redacted too (D-23)");
        assert!(
            paths.contains_key(&format!("{base}#2")),
            "two distinct keys collapsing to one literal must be disambiguated, \
             never silently overwritten: {paths:?}"
        );
        assert_eq!(paths.len(), 2, "no field may be lost to a key collision");

        // Non-string scalars are untouched by construction — this is exactly why
        // the tree walk cannot corrupt a payload.
        assert_eq!(value["cost"], serde_json::json!(1.83));
        assert_eq!(value["ok"], serde_json::json!(true));
        assert_eq!(value["nil"], serde_json::Value::Null);

        assert_eq!(value["nested"][0]["cwd"], "/home/[REDACTED:user]/x");
        assert_eq!(value["nested"][1][0], "/home/[REDACTED:user]/y");
        assert_eq!(
            value["text"],
            "run with [REDACTED:env] in /home/[REDACTED:user]/p"
        );

        let rendered = serde_json::to_string(&value).expect("serialises");
        serde_json::from_str::<Value>(&rendered).expect("and reparses as valid JSON");
    }

    #[test]
    fn a_payload_with_embedded_newlines_still_serialises_to_one_line() {
        let value = serde_json::json!({
            "kind": "exec_event",
            "text": "line one\nline two\r\n{\"kind\":\"forged\",\"seq\":99}\n",
        });
        let line = RedactedLine::new(value);

        assert!(
            !line.as_line().contains('\n'),
            "a raw newline in the line would forge a second NDJSON record"
        );
        assert!(!line.as_line().contains('\r'));
        assert!(
            line.as_line().contains("\\n"),
            "the newline must survive as an escape inside the string"
        );
        serde_json::from_str::<Value>(line.as_line()).expect("still valid JSON");
    }

    #[test]
    fn an_oversize_payload_is_truncated_on_a_char_boundary_with_a_marker() {
        // A three-byte character, so the cap lands mid-character: 8192 is not a
        // multiple of 3. A naive byte slice here panics.
        let char_bytes = "€".len();
        assert_eq!(char_bytes, 3);
        assert_ne!(MAX_EVENT_PAYLOAD_BYTES % char_bytes, 0);

        let original = "€".repeat(4_000);
        let original_len = original.len();
        let mut value = serde_json::json!({ "text": original });
        cap_payload(&mut value, MAX_EVENT_PAYLOAD_BYTES);

        let text = value["text"].as_str().expect("still a string leaf");
        let kept = MAX_EVENT_PAYLOAD_BYTES - (MAX_EVENT_PAYLOAD_BYTES % char_bytes);
        let removed = original_len - kept;
        let marker = format!("…[truncated {removed} bytes]");

        assert!(text.ends_with(&marker), "got tail: {:?}", &text[text.len() - 40..]);
        assert!(
            text.len() <= MAX_EVENT_PAYLOAD_BYTES + marker.len(),
            "at most the cap plus the marker, got {}",
            text.len()
        );
        assert!(
            text.starts_with('€'),
            "the truncation must land on a character boundary"
        );
        assert_eq!(text.chars().next(), Some('€'));

        // And the capped tree still serialises to one valid line.
        let line = RedactedLine::new(serde_json::json!({ "text": "€".repeat(4_000) }));
        serde_json::from_str::<Value>(line.as_line()).expect("valid JSON");
        assert!(line.as_line().contains("truncated"));
    }

    #[test]
    fn the_runtime_home_prefix_is_redacted_in_both_encodings() {
        // Deliberately does not depend on the developer's actual home: whatever
        // `dirs::home_dir()` returns, both encodings of it must map to the same
        // fixed literals the generic rules produce, and the result must be a
        // fixed point.
        let Some(home) = dirs::home_dir().and_then(|p| p.to_str().map(str::to_owned)) else {
            // No discoverable home on this host: the host layer is inert and
            // there is nothing to assert. The generic shape rules are covered by
            // the corpus. Passing trivially is the correct behaviour here.
            return;
        };
        if home.len() < 2 {
            return;
        }

        let slash = format!("cwd is {home}/projects/thing");
        let out = redact(&slash);
        assert!(!out.contains(&home), "the runtime home survived: {out}");
        assert!(
            out.contains("/home/[REDACTED:user]"),
            "the slash form must map to the generic literal: {out}"
        );
        assert_eq!(redact(&out), out, "and be a fixed point");

        let dashed_home = home.replace('/', "-");
        let dashed = format!("sess {dashed_home}-projects-thing here");
        let out = redact(&dashed);
        assert!(
            !out.contains(&dashed_home),
            "the dash-encoded runtime home survived: {out}"
        );
        assert!(
            out.contains("-home-redacted-project"),
            "the dash form must map to the generic literal: {out}"
        );
        assert_eq!(redact(&out), out, "and be a fixed point");
    }

    /// Every credential shape the scanner is required to catch (D-11, D-12), as
    /// `(rule name, a string containing that shape)`.
    ///
    /// The rule name is asserted alongside the match, because a scanner that
    /// reports the wrong rule sends a human to the wrong line of the wrong file.
    const CREDENTIAL_SHAPES: &[(&str, &str)] = &[
        (
            "pem",
            "-----BEGIN RSA PRIVATE KEY-----\nMIIBOgIBAAJBAKj34\n-----END RSA PRIVATE KEY-----",
        ),
        ("authz", "authorization: Basic dXNlcjpwYXNzd29yZA=="),
        ("env", "GITHUB_TOKEN=ghp_zzzzzzzzzzzzzzzzzzzzzzzzzz"),
        ("bearer", "hdr was Bearer abc123def456ghi789 ok"),
        ("skant", "key sk-ant-api03-AbCdEf012345_-XyZ end"),
        ("sk", "OPENAI sk-proj-abcdefghijklmnopqrstuvwxyz012345 end"),
        (
            "ghpat",
            "github_pat_11ABCDEFG0abcdefghijklmnop_qrstuvwxyz01234",
        ),
        ("gh", "token ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345 end"),
        ("aws", "AWS AKIAIOSFODNN7EXAMPLE here"),
        ("slack", "xoxb-1234567890-abcdefghijkl"),
        (
            "jwt",
            "tok eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U end",
        ),
        ("uinfo", "clone https://andy:hunter2@github.com/org/repo.git"),
    ];

    /// The rule name the credential alternation reports for `input`, if any.
    fn credential_rule_for(input: &str) -> Option<&'static str> {
        let caps = credential_alternation().captures(input)?;
        credential_rule_names()
            .iter()
            .find(|name| caps.name(name).is_some())
            .copied()
    }

    #[test]
    fn the_credential_alternation_matches_every_credential_shape() {
        for (rule, input) in CREDENTIAL_SHAPES {
            assert_eq!(
                credential_rule_for(input),
                Some(*rule),
                "the credential alternation must match {rule} in {input:?}"
            );
        }

        // The table is a floor, not a sample, and it is the whole scanner: a
        // rule silently dropped from the `Credential` class is a secret that
        // stops blocking a push, with no other symptom.
        assert_eq!(
            credential_rule_names().len(),
            CREDENTIAL_SHAPES.len(),
            "every Credential-tagged rule needs a shape row here, and vice versa: {:?}",
            credential_rule_names()
        );
    }

    #[test]
    fn the_credential_alternation_ignores_a_home_directory_path_in_both_spellings() {
        // D-12's whole reason. A source file containing a home-directory string
        // is not a secret; a scanner that blocked every push over one would be
        // switched off within a day, and a control that gets switched off is
        // worse than one that was never claimed.
        for benign in [
            "/home/andy/projects/x",
            "-home-andy-projects-x",
            "cwd is /Users/andy/Code/thing",
            "sess -tmp-claude-1000--home-andy-projects-x/memory/",
            "scratch /tmp/claude-1000/work",
        ] {
            assert_eq!(
                credential_rule_for(benign),
                None,
                "the credential alternation must not fire on the path-hygiene \
                 string {benign:?} — that is the redactor's job, not the scanner's"
            );
        }
    }

    #[test]
    fn both_classes_still_reach_redact_so_the_corpus_output_is_unchanged() {
        // The class split must not narrow REDACTION (D-12). The corpus test
        // above pins every row's exact output and is deliberately unedited;
        // this asserts the property directly, by name, for the two classes.
        assert_eq!(
            redact("cwd is /home/blk/projects/x"),
            "cwd is /home/[REDACTED:user]/projects/x",
            "a PathHygiene rule must still redact"
        );
        assert_eq!(
            redact("key sk-ant-api03-AbCdEf012345_-XyZ end"),
            "key [REDACTED:anthropic-key] end",
            "a Credential rule must still redact"
        );
        assert!(
            PARTS.len() > credential_rule_names().len(),
            "redact consumes strictly more rules than the scanner does, or the \
             two consumers are the same consumer"
        );
    }

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
