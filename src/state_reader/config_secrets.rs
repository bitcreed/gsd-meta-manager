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
