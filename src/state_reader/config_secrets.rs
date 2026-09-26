//! Which gsd-core config keys hold SECRETS, and the masking every Config-tab
//! surface applies to them (quick task 260926-jnf).
//!
//! **Pure functions only.** Nothing here reads a file or draws a cell; the
//! Config tab builds every `ConfigEntry.value` for a secret-classified path
//! from these helpers, so no render, prefill or status path can reach the raw
//! value by construction (T-jnf-01).
//!
//! **Provenance of the explicit list (INFERRED I-3).** Upstream gsd-core
//! disagrees with itself: its code (`src/secrets.cts`, `SECRET_CONFIG_KEYS`)
//! lists three keys — `brave_search`, `firecrawl`, `exa_search` — while its
//! `docs/CONFIGURATION.md` "Search API keys" table documents SEVEN as "Masked
//! in display", adding `tavily_search`, `ref_search`, `perplexity` and `jina`.
//! This build honours the documented promise: all seven are secret here.
//!
//! **The name heuristic (INFERRED I-4)** covers keys this build does not model
//! — pass-through rows and members inside read-only JSON values — by a
//! case-insensitive substring match of [`SECRET_NAME_MARKERS`] against the FULL
//! dotted path. Its measured false positives in upstream's key table are the
//! four numeric/boolean budget keys in [`NON_SECRET_CONFIG_KEYS`].
//!
//! **The mask diverges from upstream on purpose (INFERRED I-1).** gsd-core's
//! `maskSecret` renders `****<last-4>`; this build renders a fixed
//! [`MASKED_SECRET`] with no suffix and no length, because the README ships
//! screenshots of this TUI: the last four characters leak 16-24 bits of key
//! entropy, and the length leaks the provider's key format. Nothing in this TUI
//! needs to tell two keys apart — replacing a key means typing the new one.

use serde_json::Value;

/// The config keys whose value IS an API key (or a `true`/`false` override of
/// the provider's auto-detection) — upstream code's three plus upstream docs'
/// four (INFERRED I-3).
pub const SECRET_CONFIG_KEYS: &[&str] = &[
    "brave_search",
    "firecrawl",
    "exa_search",
    "tavily_search",
    "ref_search",
    "perplexity",
    "jina",
];

/// Keys whose names trip [`SECRET_NAME_MARKERS`] but which hold budgets, not
/// secrets — MEASURED as every key cell of upstream `docs/CONFIGURATION.md`
/// (release-1.15.0) matching the markers:
///
/// ```text
/// git -C ~/projects/node/gsd-core show upstream/release-1.15.0:docs/CONFIGURATION.md \
///   | grep -oP '^\| `\K[a-z0-9_.<>-]+(?=`)' | sort -u \
///   | grep -iE 'api_key|apikey|api-key|token|secret|password|passwd'
/// ```
///
/// Matched as the exact path or as a `<key>.` prefix, so the templated
/// `review.max_prompt_tokens_per_reviewer.<slug>` family is exempt too.
pub const NON_SECRET_CONFIG_KEYS: &[&str] = &[
    "review.max_prompt_tokens",
    "review.max_prompt_tokens_per_reviewer",
    "statusline.show_context_tokens",
    "workflow.smart_zone_tokens",
];

/// Lowercase substrings that mark an unmodelled key path as secret.
pub const SECRET_NAME_MARKERS: &[&str] = &[
    "api_key", "apikey", "api-key", "token", "secret", "password", "passwd",
];

/// What a SET secret renders as: exactly eight bullets, no suffix, no length.
pub const MASKED_SECRET: &str = "•••••••• (set)";

/// What a null, missing or empty-string value renders as.
pub const UNSET_LABEL: &str = "(unset)";

/// Is `path` (a full dotted config path) a secret-bearing key?
///
/// The explicit list first, then the measured exemptions, then the name
/// heuristic.
pub fn is_secret_key(path: &str) -> bool {
    if SECRET_CONFIG_KEYS.contains(&path) {
        return true;
    }
    let exempt = NON_SECRET_CONFIG_KEYS.iter().any(|key| {
        path == *key
            || path
                .strip_prefix(key)
                .is_some_and(|rest| rest.starts_with('.'))
    });
    if exempt {
        return false;
    }
    let lower = path.to_lowercase();
    SECRET_NAME_MARKERS.iter().any(|marker| lower.contains(marker))
}

/// The display text for a value stored under a SECRET key (INFERRED I-1).
///
/// - null or `""` -> [`UNSET_LABEL`];
/// - a boolean -> `true`/`false` verbatim: it is the documented auto-detection
///   override and carries no secret;
/// - anything else (string, number, array, object) -> [`MASKED_SECRET`].
pub fn mask_json_value(value: &Value) -> String {
    match value {
        Value::Null => UNSET_LABEL.to_string(),
        Value::String(s) if s.is_empty() => UNSET_LABEL.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => MASKED_SECRET.to_string(),
    }
}

/// A copy of `value` in which every object member whose path
/// [`is_secret_key`] is replaced by its [`mask_json_value`] text.
///
/// Member paths are `path.member` (`member` when `path` is empty); array
/// elements keep their parent's path. `path` itself is NOT tested — that is
/// [`display_config_value`]'s job.
pub fn redact_json_for_display(path: &str, value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(member, inner)| {
                    let member_path = if path.is_empty() {
                        member.clone()
                    } else {
                        format!("{path}.{member}")
                    };
                    let shown = if is_secret_key(&member_path) {
                        Value::String(mask_json_value(inner))
                    } else {
                        redact_json_for_display(&member_path, inner)
                    };
                    (member.clone(), shown)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| redact_json_for_display(path, item))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// The one display route for a config value at `path` that this build shows
/// as JSON (pass-through rows and read-only JSON rows).
///
/// A secret path renders through [`mask_json_value`]; any other path renders
/// its [`redact_json_for_display`] copy — a bare string without quotes, any
/// other shape as compact JSON.
pub fn display_config_value(path: &str, value: &Value) -> String {
    if is_secret_key(path) {
        return mask_json_value(value);
    }
    match redact_json_for_display(path, value) {
        Value::String(s) => s,
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_secret_key_lists_are_pinned() {
        assert_eq!(SECRET_CONFIG_KEYS.len(), 7, "upstream code (3) + upstream docs table (4)");
        assert_eq!(NON_SECRET_CONFIG_KEYS.len(), 4, "the four measured budget-key exemptions");
    }

    #[test]
    fn is_secret_key_matches_the_list_and_the_name_heuristic() {
        for key in SECRET_CONFIG_KEYS {
            assert!(is_secret_key(key), "`{key}` is an explicit secret key");
        }
        for key in [
            "foo_api_key",
            "FOO_API_KEY",
            "x.apiKey",
            "github_token",
            "db_password",
            "client_secret",
        ] {
            assert!(is_secret_key(key), "`{key}` trips the secret heuristic");
        }
    }

    #[test]
    fn is_secret_key_spares_the_measured_budget_keys() {
        for key in [
            "mode",
            "workflow.smart_zone_tokens",
            "statusline.show_context_tokens",
            "review.max_prompt_tokens",
            "review.max_prompt_tokens_per_reviewer.claude",
        ] {
            assert!(!is_secret_key(key), "`{key}` is not a secret");
        }
    }

    #[test]
    fn mask_json_value_hides_every_non_boolean_secret() {
        use serde_json::json;
        assert_eq!(mask_json_value(&json!(null)), UNSET_LABEL);
        assert_eq!(mask_json_value(&json!("")), UNSET_LABEL);
        assert_eq!(mask_json_value(&json!(true)), "true");
        assert_eq!(mask_json_value(&json!(false)), "false");
        assert_eq!(mask_json_value(&json!("sk-anything")), MASKED_SECRET);
        assert_eq!(mask_json_value(&json!(42)), MASKED_SECRET);
        assert_eq!(MASKED_SECRET, "•••••••• (set)");
    }

    #[test]
    fn redact_json_for_display_masks_nested_secret_members() {
        use serde_json::json;
        let value = json!({
            "github_token": "GH-SECRET-B9R1",
            "url": "https://example.invalid",
            "nested": [{ "api_key": "NESTED-SECRET-K2K2", "cli": "x" }],
            "max_prompt_tokens": 5000
        });
        let redacted = redact_json_for_display("integrations", &value);
        let text = redacted.to_string();
        assert!(!text.contains("B9R1"), "{text}");
        assert!(!text.contains("K2K2"), "{text}");
        assert!(text.contains("https://example.invalid"), "{text}");
        assert!(text.contains(MASKED_SECRET), "{text}");
        assert_eq!(redacted["nested"][0]["cli"], json!("x"));

        // The exemption holds for a nested member at an exempt PATH.
        let review = json!({ "max_prompt_tokens": 5000 });
        assert_eq!(redact_json_for_display("review", &review), review);
    }
}
