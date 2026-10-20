//! The driver's master prompt, embedded and versioned (Phase 29, PRMT-01, D-01..D-04).
//!
//! `assets/MASTER-PROMPT.md` is the single source of the contract text the headless
//! driver is given. It is compiled in with `include_str!`, so a release binary
//! carries exactly the prompt its tests checked. Its first line is an HTML comment
//! starting `<!-- driver-prompt/X.Y.Z`; [`DRIVER_PROMPT_VERSION`] must agree with
//! it (a test in `tests/driver_prompt.rs` asserts that), so a run never records a
//! version it did not use.
//!
//! Placeholders keep the spike spelling (`{RUN_DIR}`, `{PROJECT_ROOT}`, `{MODE}`)
//! and are filled by [`super::sandbox::render_prompt`], unchanged.
//!
//! A prompt that is NOT the embedded one (the hidden `--prompt-file` override,
//! D-04) is recorded as `custom:<sha256-8 of its bytes>` ([`custom_version`]), so
//! a run never claims the shipped version for text it did not use.
//!
//! This module is pure text and hashing: it is deliberately not
//! `cfg(target_os = "linux")`-gated, like [`super::sandbox`].

use std::borrow::Cow;

use sha2::{Digest, Sha256};

/// The shipped prompt version. Must equal the version in line 1 of the asset.
pub const DRIVER_PROMPT_VERSION: &str = "driver-prompt/1.0.0";

/// The embedded master prompt template (placeholders unfilled).
pub const MASTER_PROMPT: &str = include_str!("assets/MASTER-PROMPT.md");

/// The line that opens the stop-rule precedence block (shared with the DIRECTION
/// template, which copies the block byte for byte).
pub const STOP_RULES_BEGIN: &str = "<!-- stop-rules:begin -->";

/// The line that closes the stop-rule precedence block.
pub const STOP_RULES_END: &str = "<!-- stop-rules:end -->";

/// The prefix of the version token on line 1 of a prompt.
const HEADER_PREFIX: &str = "<!-- ";

/// The slice of `text` from [`STOP_RULES_BEGIN`] through [`STOP_RULES_END`],
/// inclusive.
///
/// `None` when either marker is missing, the end precedes the begin, or the begin
/// marker occurs twice (an ambiguous block is never guessed at).
pub fn stop_rules_block(text: &str) -> Option<&str> {
    let begin = text.find(STOP_RULES_BEGIN)?;
    if text[begin + STOP_RULES_BEGIN.len()..].contains(STOP_RULES_BEGIN) {
        return None;
    }
    let end_rel = text[begin..].find(STOP_RULES_END)?;
    Some(&text[begin..begin + end_rel + STOP_RULES_END.len()])
}

/// The `driver-prompt/X.Y.Z` token on line 1 (`<!-- driver-prompt/X.Y.Z ...`).
///
/// Only line 1 is read. `None` when line 1 is not that shape.
pub fn header_version(text: &str) -> Option<&str> {
    let first = text.lines().next()?;
    let rest = first.strip_prefix(HEADER_PREFIX)?;
    let token = rest.split_whitespace().next()?;
    token.starts_with("driver-prompt/").then_some(token)
}

/// A prompt template and the version a run records for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPrompt {
    /// The template, placeholders unfilled (render with `sandbox::render_prompt`).
    pub template: Cow<'static, str>,
    /// `driver-prompt/X.Y.Z` for the embedded prompt, `custom:<hash8>` otherwise.
    pub version: String,
}

/// The embedded prompt with its shipped version.
pub fn embedded() -> ResolvedPrompt {
    ResolvedPrompt {
        template: Cow::Borrowed(MASTER_PROMPT),
        version: DRIVER_PROMPT_VERSION.to_string(),
    }
}

/// A caller-supplied template with a `custom:` version.
pub fn custom(template: String) -> ResolvedPrompt {
    let version = custom_version(template.as_bytes());
    ResolvedPrompt {
        template: Cow::Owned(template),
        version,
    }
}

/// `custom:` plus the first four bytes of `sha256(bytes)` as eight lowercase hex
/// digits (the same shape as `driverctl::serve::hash8`, which hashes paths).
pub fn custom_version(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().take(4).map(|b| format!("{b:02x}")).collect();
    format!("custom:{hex}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_version_matches_the_known_sha256_vectors() {
        assert_eq!(custom_version(b""), "custom:e3b0c442");
        assert_eq!(custom_version(b"abc"), "custom:ba7816bf");
    }

    #[test]
    fn custom_wraps_the_template_with_its_hash() {
        let resolved = custom("abc".to_string());
        assert_eq!(resolved.template, "abc");
        assert_eq!(resolved.version, "custom:ba7816bf");
    }

    #[test]
    fn embedded_pairs_the_asset_with_the_shipped_version() {
        let resolved = embedded();
        assert_eq!(resolved.template, MASTER_PROMPT);
        assert_eq!(resolved.version, DRIVER_PROMPT_VERSION);
    }

    #[test]
    fn the_asset_header_names_the_shipped_version() {
        assert_eq!(header_version(MASTER_PROMPT), Some(DRIVER_PROMPT_VERSION));
    }

    #[test]
    fn header_version_reads_only_line_one() {
        assert_eq!(
            header_version("<!-- driver-prompt/2.3.4 more\nbody"),
            Some("driver-prompt/2.3.4")
        );
        assert_eq!(header_version("# title\n<!-- driver-prompt/1.0.0"), None);
        assert_eq!(header_version("<!-- something else"), None);
        assert_eq!(header_version(""), None);
    }

    #[test]
    fn stop_rules_block_is_inclusive_of_both_markers() {
        let text = format!("before\n{STOP_RULES_BEGIN}\n1. a\n{STOP_RULES_END}\nafter");
        assert_eq!(
            stop_rules_block(&text),
            Some("<!-- stop-rules:begin -->\n1. a\n<!-- stop-rules:end -->")
        );
    }

    #[test]
    fn stop_rules_block_is_none_for_a_missing_end() {
        let text = format!("{STOP_RULES_BEGIN}\n1. a\n");
        assert_eq!(stop_rules_block(&text), None);
    }

    #[test]
    fn stop_rules_block_is_none_for_an_end_before_the_begin() {
        let text = format!("{STOP_RULES_END}\n1. a\n{STOP_RULES_BEGIN}\n");
        assert_eq!(stop_rules_block(&text), None);
    }

    #[test]
    fn stop_rules_block_is_none_for_a_doubled_begin() {
        let text = format!("{STOP_RULES_BEGIN}\n{STOP_RULES_BEGIN}\n{STOP_RULES_END}\n");
        assert_eq!(stop_rules_block(&text), None);
    }
}
