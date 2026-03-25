use serde::Deserialize;

#[derive(Debug, Deserialize, Clone, Default)]
pub struct GsdConfig {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub granularity: String,
}

/// Parse GSD's .planning/config.json content.
/// Returns None on invalid JSON or missing content.
pub fn parse_gsd_config(content: &str) -> Option<GsdConfig> {
    match serde_json::from_str(content) {
        Ok(config) => Some(config),
        Err(e) => {
            tracing::warn!("Failed to parse config.json: {}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gsd_config() {
        let content = r#"{"mode": "yolo", "granularity": "coarse"}"#;
        let config = parse_gsd_config(content).unwrap();
        assert_eq!(config.mode, "yolo");
        assert_eq!(config.granularity, "coarse");
    }

    #[test]
    fn test_parse_gsd_config_invalid_json() {
        assert!(parse_gsd_config("not json").is_none());
    }

    #[test]
    fn test_parse_gsd_config_missing_fields() {
        let config = parse_gsd_config("{}").unwrap();
        assert_eq!(config.mode, "");
    }
}
