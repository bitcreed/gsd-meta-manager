use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct GsdConfig {
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub granularity: String,
    #[serde(default)]
    pub model_profile: String,
    #[serde(default)]
    pub commit_docs: Option<bool>,
    #[serde(default)]
    pub parallelization: Option<bool>,
    #[serde(default)]
    pub search_gitignored: Option<bool>,
    #[serde(default)]
    pub brave_search: Option<bool>,
    #[serde(default)]
    pub firecrawl: Option<bool>,
    #[serde(default)]
    pub exa_search: Option<bool>,
    #[serde(default)]
    pub project_code: Option<String>,
    #[serde(default)]
    pub phase_naming: Option<String>,
    #[serde(default)]
    pub response_language: Option<String>,
    #[serde(default)]
    pub git: Option<GitConfig>,
    #[serde(default)]
    pub workflow: Option<WorkflowConfig>,
    #[serde(default)]
    pub hooks: Option<HooksConfig>,
    #[serde(default)]
    pub intel: Option<IntelConfig>,
    #[serde(default)]
    pub graphify: Option<GraphifyConfig>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct GitConfig {
    #[serde(default)]
    pub branching_strategy: Option<String>,
    #[serde(default)]
    pub base_branch: Option<String>,
    #[serde(default)]
    pub phase_branch_template: Option<String>,
    #[serde(default)]
    pub milestone_branch_template: Option<String>,
    #[serde(default)]
    pub quick_branch_template: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct WorkflowConfig {
    #[serde(default)]
    pub research: Option<bool>,
    #[serde(default)]
    pub plan_check: Option<bool>,
    #[serde(default)]
    pub verifier: Option<bool>,
    #[serde(default)]
    pub nyquist_validation: Option<bool>,
    #[serde(default)]
    pub auto_advance: Option<bool>,
    #[serde(default)]
    pub node_repair: Option<bool>,
    #[serde(default)]
    pub node_repair_budget: Option<u32>,
    #[serde(default)]
    pub ui_phase: Option<bool>,
    #[serde(default)]
    pub ui_safety_gate: Option<bool>,
    #[serde(default)]
    pub text_mode: Option<bool>,
    #[serde(default)]
    pub research_before_questions: Option<bool>,
    #[serde(default)]
    pub discuss_mode: Option<String>,
    #[serde(default)]
    pub skip_discuss: Option<bool>,
    #[serde(rename = "_auto_chain_active", default)]
    pub auto_chain_active: Option<bool>,
    #[serde(default)]
    pub use_worktrees: Option<bool>,
    #[serde(default)]
    pub subagent_timeout: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct HooksConfig {
    #[serde(default)]
    pub context_warnings: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct IntelConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct GraphifyConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub build_timeout: Option<u32>,
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

/// Serialize a GsdConfig back to pretty-printed JSON.
pub fn serialize_gsd_config(config: &GsdConfig) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(config)
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

    #[test]
    fn test_parse_full_config_roundtrip() {
        let content = r#"{
            "mode": "yolo",
            "granularity": "coarse",
            "model_profile": "quality",
            "commit_docs": true,
            "parallelization": false,
            "search_gitignored": true,
            "brave_search": false,
            "firecrawl": false,
            "exa_search": false,
            "project_code": "TST",
            "phase_naming": "auto",
            "response_language": "en",
            "git": {
                "branching_strategy": "phase",
                "base_branch": "main",
                "phase_branch_template": "phase-{number}",
                "milestone_branch_template": null,
                "quick_branch_template": "quick-{id}"
            },
            "workflow": {
                "research": true,
                "plan_check": true,
                "verifier": true,
                "nyquist_validation": false,
                "auto_advance": false,
                "node_repair": true,
                "node_repair_budget": 3,
                "ui_phase": false,
                "ui_safety_gate": false,
                "text_mode": false,
                "research_before_questions": true,
                "discuss_mode": "discuss",
                "skip_discuss": false,
                "_auto_chain_active": false,
                "use_worktrees": true,
                "subagent_timeout": 300
            },
            "hooks": {
                "context_warnings": true
            }
        }"#;
        let config = parse_gsd_config(content).unwrap();
        assert_eq!(config.mode, "yolo");
        assert_eq!(config.model_profile, "quality");
        assert_eq!(config.commit_docs, Some(true));
        assert_eq!(config.project_code, Some("TST".to_string()));

        let git = config.git.as_ref().unwrap();
        assert_eq!(git.branching_strategy, Some("phase".to_string()));
        assert_eq!(git.base_branch, Some("main".to_string()));

        let wf = config.workflow.as_ref().unwrap();
        assert_eq!(wf.research, Some(true));
        assert_eq!(wf.node_repair_budget, Some(3));
        assert_eq!(wf.discuss_mode, Some("discuss".to_string()));
        assert_eq!(wf.use_worktrees, Some(true));
        assert_eq!(wf.subagent_timeout, Some(300));

        let hooks = config.hooks.as_ref().unwrap();
        assert_eq!(hooks.context_warnings, Some(true));

        // Round-trip: serialize and re-parse
        let serialized = serialize_gsd_config(&config).unwrap();
        let reparsed = parse_gsd_config(&serialized).unwrap();
        assert_eq!(reparsed.mode, "yolo");
        assert_eq!(reparsed.model_profile, "quality");
        assert_eq!(
            reparsed.workflow.as_ref().unwrap().node_repair_budget,
            Some(3)
        );
    }
}
