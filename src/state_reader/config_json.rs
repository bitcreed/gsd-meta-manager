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
    // --- GSD 1.4–1.8 additions ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_orchestration: Option<ClaudeOrchestrationConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub statusline: Option<StatuslineConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamic_routing: Option<DynamicRoutingConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<ReviewConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_job: Option<ExternalJobConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<CapabilitiesConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase_id_convention: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_md_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_repos: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ClaudeOrchestrationConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub execution_backend: Option<String>,
    #[serde(default)]
    pub min_agent_sdk_version: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct StatuslineConfig {
    #[serde(default)]
    pub show_context_tokens: Option<bool>,
    #[serde(default)]
    pub state_format: Option<String>,
    #[serde(default)]
    pub show_git: Option<bool>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct DynamicRoutingConfig {
    #[serde(default)]
    pub provider_escalation: Option<bool>,
    #[serde(default)]
    pub max_escalations: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ReviewConfig {
    /// Shape varies (count or list) — mirror the `quick_branch_template` precedent.
    #[serde(default)]
    pub reviewer_instances: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ExternalJobConfig {
    #[serde(default)]
    pub submit_timeout_ms: Option<u32>,
    #[serde(default)]
    pub poll_timeout_ms: Option<u32>,
    #[serde(default)]
    pub artifact_dir: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct CapabilitiesConfig {
    #[serde(default)]
    pub strict_known_registries: Option<bool>,
    #[serde(default)]
    pub auto_update: Option<bool>,
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
    #[serde(default)]
    pub pattern_mapper: Option<bool>,
    #[serde(default)]
    pub ai_integration_phase: Option<bool>,
    #[serde(default)]
    pub tdd_mode: Option<bool>,
    #[serde(default)]
    pub code_review: Option<bool>,
    #[serde(default)]
    pub code_review_depth: Option<String>,
    #[serde(default)]
    pub ui_review: Option<bool>,
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

/// Resolve the absolute path to the user-level GSD defaults file
/// (`~/.gsd/defaults.json`). Returns None when $HOME is unset.
pub fn user_defaults_path() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(|home| {
        let mut p = std::path::PathBuf::from(home);
        p.push(".gsd");
        p.push("defaults.json");
        p
    })
}

/// Load `~/.gsd/defaults.json` if present and parsable.
/// GSD layers this under project config at `/gsd-new-project` time;
/// we surface it in the TUI so users can see which fields fall back
/// to global defaults.
pub fn load_user_defaults() -> Option<GsdConfig> {
    let path = user_defaults_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    parse_gsd_config(&content)
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

    #[test]
    fn test_parse_new_toplevel_blocks() {
        let content = r#"{
            "claude_orchestration": {
                "enabled": true,
                "execution_backend": "claude-agent-sdk",
                "min_agent_sdk_version": "0.1.0"
            },
            "statusline": {
                "show_context_tokens": true,
                "state_format": "compact",
                "show_git": false
            },
            "dynamic_routing": {
                "provider_escalation": true,
                "max_escalations": 2
            },
            "review": {
                "reviewer_instances": 3
            },
            "external_job": {
                "submit_timeout_ms": 60000,
                "poll_timeout_ms": 5000,
                "artifact_dir": ".planning/jobs"
            },
            "capabilities": {
                "strict_known_registries": true,
                "auto_update": false
            },
            "phase_id_convention": "zero-padded",
            "claude_md_path": "./CLAUDE.md",
            "sub_repos": ["backend", "frontend"]
        }"#;
        let config = parse_gsd_config(content).unwrap();

        let co = config.claude_orchestration.as_ref().unwrap();
        assert_eq!(co.enabled, Some(true));
        assert_eq!(co.execution_backend, Some("claude-agent-sdk".to_string()));
        assert_eq!(co.min_agent_sdk_version, Some("0.1.0".to_string()));

        let sl = config.statusline.as_ref().unwrap();
        assert_eq!(sl.show_context_tokens, Some(true));
        assert_eq!(sl.state_format, Some("compact".to_string()));
        assert_eq!(sl.show_git, Some(false));

        let dr = config.dynamic_routing.as_ref().unwrap();
        assert_eq!(dr.provider_escalation, Some(true));
        assert_eq!(dr.max_escalations, Some(2));

        let rv = config.review.as_ref().unwrap();
        assert_eq!(
            rv.reviewer_instances,
            Some(serde_json::Value::from(3))
        );

        let ej = config.external_job.as_ref().unwrap();
        assert_eq!(ej.submit_timeout_ms, Some(60000));
        assert_eq!(ej.poll_timeout_ms, Some(5000));
        assert_eq!(ej.artifact_dir, Some(".planning/jobs".to_string()));

        let cap = config.capabilities.as_ref().unwrap();
        assert_eq!(cap.strict_known_registries, Some(true));
        assert_eq!(cap.auto_update, Some(false));

        assert_eq!(config.phase_id_convention, Some("zero-padded".to_string()));
        assert_eq!(config.claude_md_path, Some("./CLAUDE.md".to_string()));
        assert!(config.sub_repos.as_ref().unwrap().is_array());
    }

    #[test]
    fn test_new_toplevel_blocks_default_none() {
        let config = parse_gsd_config("{}").unwrap();
        assert!(config.claude_orchestration.is_none());
        assert!(config.statusline.is_none());
        assert!(config.dynamic_routing.is_none());
        assert!(config.review.is_none());
        assert!(config.external_job.is_none());
        assert!(config.capabilities.is_none());
        assert!(config.phase_id_convention.is_none());
        assert!(config.claude_md_path.is_none());
        assert!(config.sub_repos.is_none());
    }

    #[test]
    fn test_default_config_omits_new_toplevel_keys() {
        let config = GsdConfig::default();
        let serialized = serialize_gsd_config(&config).unwrap();
        assert!(!serialized.contains("claude_orchestration"));
        assert!(!serialized.contains("statusline"));
        assert!(!serialized.contains("dynamic_routing"));
        assert!(!serialized.contains("external_job"));
        assert!(!serialized.contains("capabilities"));
        assert!(!serialized.contains("phase_id_convention"));
        assert!(!serialized.contains("claude_md_path"));
        assert!(!serialized.contains("sub_repos"));
    }
}
