use serde::{Deserialize, Serialize};

/// The latest gsd-core release PUBLISHED ON NPM at or below
/// [`GSD_CORE_SYNCED_COMMIT`] — MEASURED rather than remembered (quick tasks
/// 260916-vqw ID-1/ID-4, 260926-gtk).
///
/// **It is the conformance-oracle pin, not the synced tree's version.**
/// `scripts/install-conformance-oracle.sh` greps this declaration and
/// npm-installs `@opengsd/gsd-core@<this>` — in the publish job
/// (`.github/workflows/release.yml`) and in `./scripts/pre-tag-check.sh
/// --container`. The config surface is synced to gsd-core's `release-1.15.0`
/// branch, whose `package.json` says `1.15.0`, but 1.15.0 is not on npm yet,
/// so pinning it here would break the publish job. Bump this to `1.15.0`
/// TOGETHER with [`GSD_CORE_SYNCED_COMMIT`] once `npm view @opengsd/gsd-core
/// version` reports 1.15.0 and a `v1.15.0` tag exists.
///
/// **What a future syncer does with the pair.** gsd-core adds config keys every
/// release, and a key this build does not model is a key the Defaults tab
/// cannot show. Before 260916-vqw it was also a key the SAVE path deleted, and
/// the drift was silent in both directions because nothing in the tree recorded
/// where the last sync stopped. This pair is the machine-readable half of that
/// record; `docs/GSD-CORE-SYNC.md` is the human-readable half and carries the
/// per-key inventory to diff against.
///
/// Re-measure both, never hand-edit, when syncing — against a REF, because the
/// local checkout's working-tree HEAD is not necessarily the sync target:
///
/// ```text
/// CORE=~/projects/node/gsd-core REF=upstream/release-1.15.0
/// git -C "$CORE" describe --tags --always "$REF"   # -> GSD_CORE_SYNCED_COMMIT
/// git -C "$CORE" show "$REF:package.json"          # -> GSD_CORE_SYNCED_TREE_VERSION
/// npm view @opengsd/gsd-core version               # -> GSD_CORE_SYNCED_VERSION
/// ```
pub const GSD_CORE_SYNCED_VERSION: &str = "1.14.0";

/// The exact gsd-core revision this build's config surface was synced
/// against, as `git describe --tags --always` spells it.
///
/// The tip of gsd-core's `release-1.15.0` branch — 111 commits past
/// `v1.14.0`, full sha `ec81d0d10f545dd5aea2cc893863a542bc49029d`, whose
/// `package.json` says `1.15.0` but which carries no tag. The describe names
/// `v1.14.0` because that is the newest tag reachable from it, which is why it
/// still contains [`GSD_CORE_SYNCED_VERSION`]; that stops holding once
/// `v1.15.0` is tagged, and is the signal to bump both together.
pub const GSD_CORE_SYNCED_COMMIT: &str = "v1.14.0-111-gec81d0d10";

/// The `package.json` version of the gsd-core tree at
/// [`GSD_CORE_SYNCED_COMMIT`], measured with `git -C "$CORE" show
/// "$REF:package.json"` (quick task 260926-j0a, inferred I-2).
///
/// It is the CEILING of the in-sync range the installed-GSD comparison uses
/// ([`crate::state_reader::gsd_install::relation_to_synced`]);
/// [`GSD_CORE_SYNCED_VERSION`] is the floor. README.md's `**GSD
/// compatibility:**` paragraph must name it —
/// `the_readme_compatibility_note_names_the_synced_baseline` fails otherwise.
///
/// Named so that the declaration regex `scripts/install-conformance-oracle.sh`
/// greps for [`GSD_CORE_SYNCED_VERSION`] cannot match it: that grep must keep
/// matching exactly one line, the oracle pin.
pub const GSD_CORE_SYNCED_TREE_VERSION: &str = "1.15.0";

/// Every key of a parsed block that this build has no typed field for.
///
/// **This is a DATA-LOSS repair, not a display feature** (T-VQW-02). The
/// Defaults tab's save path is `serialize_gsd_config(&GsdConfig)` written
/// straight over the operator's `.planning/config.json`, and serde drops what
/// it did not parse — so before this field existed, toggling one checkbox
/// silently deleted every gsd-core key this build does not model. Capturing the
/// remainder makes the writer lossless.
///
/// `#[serde(flatten)]` removes the CONSUMED keys before filling the map, so a
/// typed field can never also appear here and the serialised output cannot
/// carry a duplicate. It is also why no struct in this file may use
/// `deny_unknown_fields`: the two attributes are incompatible.
pub type ExtraKeys = serde_json::Map<String, serde_json::Value>;

/// The value of one gsd-core search-provider slot (`brave_search`,
/// `firecrawl`, `exa_search`, `tavily_search`, `ref_search`, `perplexity`,
/// `jina`) — upstream types each `string | boolean | null` (quick task
/// 260926-jnf).
///
/// - `Key(s)` — the provider's API KEY itself. SECRET.
/// - `Flag(b)` — the legacy sentinel overriding auto-detection from the
///   `<X>_API_KEY` env var or `~/.gsd/<x>_api_key` file.
/// - JSON `null` / absent — `None` on the enclosing `Option`.
///
/// **The bug this fixes (T-jnf-05).** These slots were `Option<bool>`, so a
/// project that stored its key the documented way made the WHOLE
/// `config.json` a parse failure: no Defaults rows, no save — and the serde
/// error, which quotes the offending value, went to the log.
///
/// `Flag` is listed first so an untagged JSON boolean can never become a key.
/// A number or object in a slot is still a whole-file parse failure (INFERRED
/// I-5, the accepted gtk I-5 divergence); the untagged-enum error names no
/// value.
///
/// **`Debug` is written by hand** (T-jnf-04) so `Key` prints as
/// `Key(<redacted>)`: `GsdConfig` derives `Debug`, and a derived one would
/// print the key wherever a config is debug-formatted.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ApiKeySetting {
    Flag(bool),
    Key(String),
}

impl std::fmt::Debug for ApiKeySetting {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiKeySetting::Flag(b) => f.debug_tuple("Flag").field(b).finish(),
            ApiKeySetting::Key(_) => f.write_str("Key(<redacted>)"),
        }
    }
}

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
    /// A search-provider slot — see [`ApiKeySetting`]. May hold an API key.
    #[serde(default)]
    pub brave_search: Option<ApiKeySetting>,
    /// A search-provider slot — see [`ApiKeySetting`]. May hold an API key.
    #[serde(default)]
    pub firecrawl: Option<ApiKeySetting>,
    /// A search-provider slot — see [`ApiKeySetting`]. May hold an API key.
    #[serde(default)]
    pub exa_search: Option<ApiKeySetting>,
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
    // --- gsd-core re-sync at 1.14.0 (quick task 260916-vqw) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_window: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub features: Option<FeaturesConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gates: Option<GatesConfig>,
    /// gsd-core's NAMESPACED spellings of three keys this build also models at
    /// the top level (`commit_docs`, `search_gitignored`, `sub_repos`).
    ///
    /// **Both are modelled on purpose.** They are distinct JSON paths, a real
    /// project may carry either, and collapsing them would make the Defaults
    /// tab show a value the file does not contain at the path it claims.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planning: Option<PlanningConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_review: Option<PlanReviewConfig>,
    // --- gsd-core re-sync at 1.15.0 (quick task 260926-gtk) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub planner: Option<PlannerConfig>,
    /// Top-level gsd-core keys this build does not model — see [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct FeaturesConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global_learnings: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thinking_partner: Option<bool>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct GatesConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm_breakdown: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm_phases: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm_plan: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm_project: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm_roadmap: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confirm_transition: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execute_next_plan: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub issues_review: Option<bool>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PlanningConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chunked_parallel: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub commit_docs: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pr_strict: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub search_gitignored: Option<bool>,
    /// A list of sub-repo paths — surfaced read-only, like the top-level
    /// `sub_repos` it namespaces.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_repos: Option<serde_json::Value>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PlanReviewConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_grounding: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_grounding_authority: Option<String>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

/// gsd-core's `planner` block, added at 1.15.0 (#4570 / PR #4585) with one
/// documented key, `stall_detection_enabled` (default `true`).
///
/// **Only that key is typed.** `stall_detect_interval_minutes` and
/// `stall_threshold_minutes`, which projects can also carry here, stay in
/// [`Self::extra`] as pass-through, per docs/GSD-CORE-SYNC.md's
/// runtime-tuning-knobs decision (quick task 260926-gtk, INFERRED I-3) — they
/// render as read-only `planner.<key>` rows and round-trip untouched.
///
/// **A deliberate divergence from gsd-core (INFERRED I-5, accepted).** gsd-core
/// treats a non-boolean `stall_detection_enabled` (string, number, null) as
/// `true` — only a real JSON boolean overrides the default. The typed
/// `Option<bool>` here instead makes such a hand edit a whole-file parse
/// failure, exactly like every other typed key in this file. gsd-core's own
/// `config-set` always writes a real boolean, so only a hand edit can produce
/// the case, and a failed parse draws no Defaults rows and therefore cannot
/// save over (and lose) the file.
#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct PlannerConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stall_detection_enabled: Option<bool>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ClaudeOrchestrationConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub execution_backend: Option<String>,
    #[serde(default)]
    pub min_agent_sdk_version: Option<String>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct StatuslineConfig {
    #[serde(default)]
    pub show_context_tokens: Option<bool>,
    #[serde(default)]
    pub state_format: Option<String>,
    #[serde(default)]
    pub show_git: Option<bool>,
    // --- gsd-core re-sync at 1.14.0 (quick task 260916-vqw) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_position: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_last_command: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub show_state_freshness: Option<bool>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct DynamicRoutingConfig {
    #[serde(default)]
    pub provider_escalation: Option<bool>,
    #[serde(default)]
    pub max_escalations: Option<u32>,
    // --- gsd-core re-sync at 1.14.0 (quick task 260916-vqw) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escalate_on_failure: Option<bool>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ReviewConfig {
    /// Shape varies (count or list) — mirror the `quick_branch_template` precedent.
    #[serde(default)]
    pub reviewer_instances: Option<serde_json::Value>,
    /// See [`ExtraKeys`]. This block carries gsd-core's templated
    /// `review.models.<cli>` families, which get no typed row by design (ID-3).
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ExternalJobConfig {
    #[serde(default)]
    pub submit_timeout_ms: Option<u32>,
    #[serde(default)]
    pub poll_timeout_ms: Option<u32>,
    #[serde(default)]
    pub artifact_dir: Option<String>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct CapabilitiesConfig {
    #[serde(default)]
    pub strict_known_registries: Option<bool>,
    #[serde(default)]
    pub auto_update: Option<bool>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
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
    // --- gsd-core re-sync at 1.14.0 (quick task 260916-vqw) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub create_tag: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allow_default_branch_commits: Option<bool>,
    /// A list of branch names — surfaced read-only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protected_branches: Option<serde_json::Value>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
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
    // --- GSD 1.8 gates ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_coverage_gate: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows_enforce: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specless_probe_fallback: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assumption_delta: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_gate_timeout: Option<u32>,
    // --- GSD 1.4–1.6 gates ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_guard_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_drift_precheck: Option<bool>,
    /// Shape varies (e.g. int `2` or string `"L2"`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_asvs_level: Option<serde_json::Value>,
    /// Shape varies (string or array).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_block_on: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mvp_mode: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_review_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_chunked: Option<bool>,
    // --- gsd-core re-sync at 1.14.0 (quick task 260916-vqw) ---
    // Every key below was documented by gsd-core and modelled NOWHERE by this
    // build; `docs/GSD-CORE-SYNC.md` records the per-key introduction version.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compact_content: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_hint_routing: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_prune_state: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub build_command: Option<String>,
    /// Shape varies (a list of `{ paths, depth }` rules) — surfaced read-only,
    /// the `security_block_on` precedent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_review_depth_overrides: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_review_point: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_coverage_gate: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_drift_action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_drift_precheck: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_ai_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_ai_execution: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cross_ai_timeout: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drift_action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drift_threshold: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub human_verify_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inline_plan_threshold: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub live_dom_uat: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_discuss_passes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_bounce: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_bounce_passes: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_bounce_script: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_review_convergence: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub post_planning_gaps: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security_enforcement: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub smart_zone_tokens: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub test_command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worktree_skip_hooks: Option<bool>,
    // --- gsd-core re-sync at 1.15.0 (quick task 260926-gtk) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_interaction_capture: Option<bool>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct HooksConfig {
    #[serde(default)]
    pub context_warnings: Option<bool>,
    // --- gsd-core re-sync at 1.14.0 (quick task 260916-vqw) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_guard: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_warning_threshold: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_critical_threshold: Option<u32>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct IntelConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct GraphifyConfig {
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub build_timeout: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub graph_path: Option<String>,
    // --- gsd-core re-sync at 1.14.0 (quick task 260916-vqw) ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_update: Option<bool>,
    /// See [`ExtraKeys`].
    #[serde(flatten)]
    pub extra: ExtraKeys,
}

/// Parse GSD's .planning/config.json content.
/// Returns None on invalid JSON or missing content.
pub fn parse_gsd_config(content: &str) -> Option<GsdConfig> {
    match serde_json::from_str(content) {
        Ok(config) => Some(config),
        Err(e) => {
            // T-jnf-03 (quick task 260926-jnf): NEVER the serde Display — it
            // quotes the offending value (`invalid type: string "sk-…"`), and
            // that value can be an API key. Category and position only.
            tracing::warn!("Failed to parse config.json: {}", parse_error_summary(&e));
            None
        }
    }
}

/// A value-free description of a config parse error: the serde error
/// CATEGORY plus line and column, built only from `classify()`, `line()` and
/// `column()` — never from the error's `Display`, which quotes the offending
/// value (T-jnf-03).
pub fn parse_error_summary(e: &serde_json::Error) -> String {
    use serde_json::error::Category;
    let category = match e.classify() {
        Category::Io => "I/O error",
        Category::Syntax => "syntax error",
        Category::Data => "data error (a value of the wrong type or shape)",
        Category::Eof => "unexpected end of input",
    };
    format!("{category} at line {}, column {}", e.line(), e.column())
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
        assert_eq!(config.brave_search, Some(ApiKeySetting::Flag(false)));

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
    fn test_parse_new_workflow_gates_and_graph_path_roundtrip() {
        let content = r#"{
            "workflow": {
                "api_coverage_gate": true,
                "windows_enforce": false,
                "specless_probe_fallback": true,
                "assumption_delta": false,
                "test_gate_timeout": 120,
                "context_guard_mode": "strict",
                "plan_drift_precheck": true,
                "security_asvs_level": "L2",
                "security_block_on": ["high", "critical"],
                "mvp_mode": true,
                "code_review_command": "/gsd:code-review",
                "plan_chunked": false
            },
            "graphify": {
                "enabled": true,
                "graph_path": ".planning/graphs"
            }
        }"#;
        let config = parse_gsd_config(content).unwrap();
        let wf = config.workflow.as_ref().unwrap();
        assert_eq!(wf.api_coverage_gate, Some(true));
        assert_eq!(wf.windows_enforce, Some(false));
        assert_eq!(wf.specless_probe_fallback, Some(true));
        assert_eq!(wf.assumption_delta, Some(false));
        assert_eq!(wf.test_gate_timeout, Some(120));
        assert_eq!(wf.context_guard_mode, Some("strict".to_string()));
        assert_eq!(wf.plan_drift_precheck, Some(true));
        assert_eq!(
            wf.security_asvs_level,
            Some(serde_json::Value::from("L2"))
        );
        assert!(wf.security_block_on.as_ref().unwrap().is_array());
        assert_eq!(wf.mvp_mode, Some(true));
        assert_eq!(
            wf.code_review_command,
            Some("/gsd:code-review".to_string())
        );
        assert_eq!(wf.plan_chunked, Some(false));

        let gf = config.graphify.as_ref().unwrap();
        assert_eq!(gf.graph_path, Some(".planning/graphs".to_string()));

        // Round-trip: serialize, re-parse, values survive
        let serialized = serialize_gsd_config(&config).unwrap();
        let reparsed = parse_gsd_config(&serialized).unwrap();
        let rwf = reparsed.workflow.as_ref().unwrap();
        assert_eq!(rwf.api_coverage_gate, Some(true));
        assert_eq!(rwf.test_gate_timeout, Some(120));
        assert_eq!(rwf.security_asvs_level, Some(serde_json::Value::from("L2")));
        assert_eq!(rwf.mvp_mode, Some(true));
        assert_eq!(
            reparsed.graphify.as_ref().unwrap().graph_path,
            Some(".planning/graphs".to_string())
        );
    }

    #[test]
    fn test_new_workflow_gates_default_none() {
        let config = parse_gsd_config("{}").unwrap();
        assert!(config.workflow.is_none());
        // A workflow block present but without the new keys leaves them None.
        let config2 = parse_gsd_config(r#"{"workflow": {"research": true}}"#).unwrap();
        let wf = config2.workflow.as_ref().unwrap();
        assert!(wf.api_coverage_gate.is_none());
        assert!(wf.windows_enforce.is_none());
        assert!(wf.specless_probe_fallback.is_none());
        assert!(wf.assumption_delta.is_none());
        assert!(wf.test_gate_timeout.is_none());
        assert!(wf.context_guard_mode.is_none());
        assert!(wf.plan_drift_precheck.is_none());
        assert!(wf.security_asvs_level.is_none());
        assert!(wf.security_block_on.is_none());
        assert!(wf.mvp_mode.is_none());
        assert!(wf.code_review_command.is_none());
        assert!(wf.plan_chunked.is_none());
        // graphify graph_path also None when absent
        let config3 = parse_gsd_config(r#"{"graphify": {"enabled": true}}"#).unwrap();
        assert!(config3.graphify.as_ref().unwrap().graph_path.is_none());
    }

    // ── Unknown-key preservation (quick task 260916-vqw, T-VQW-02) ────────

    /// The repair for the live data-loss bug: the Defaults tab's save path is
    /// `serialize_gsd_config` written straight over the operator's file, so a
    /// key this build does not model used to be DELETED by toggling an
    /// unrelated checkbox.
    ///
    /// The assertion is byte equality of two successive serialisations rather
    /// than a key spot-check, because a spot-check passes on a writer that
    /// keeps the key and mangles its value.
    #[test]
    fn unknown_keys_survive_serialize_and_reparse() {
        let content = r#"{
            "workflow": { "research": true, "unknown_gsd_key": 7 },
            "brand_new_block": { "a": 1 },
            "review": { "models": { "codex": "gpt-5" } },
            "effort": { "planner": "high" }
        }"#;
        let config = parse_gsd_config(content).expect("a config with unknown keys still parses");

        assert_eq!(
            config.extra.get("brand_new_block"),
            Some(&serde_json::json!({ "a": 1 })),
            "an unmodelled TOP-LEVEL block was dropped on the way in"
        );
        assert_eq!(
            config.extra.get("effort"),
            Some(&serde_json::json!({ "planner": "high" }))
        );
        let wf = config.workflow.as_ref().expect("the workflow block parsed");
        assert_eq!(wf.research, Some(true), "a MODELLED sibling key still parses");
        assert_eq!(
            wf.extra.get("unknown_gsd_key"),
            Some(&serde_json::json!(7)),
            "an unmodelled key NESTED in a modelled block was dropped"
        );
        assert!(
            !wf.extra.contains_key("research"),
            "flatten must remove the consumed keys, or the writer emits a duplicate"
        );
        let review = config.review.as_ref().expect("the review block parsed");
        assert_eq!(
            review.extra.get("models"),
            Some(&serde_json::json!({ "codex": "gpt-5" })),
            "gsd-core's templated review.models.<cli> family was dropped"
        );

        let serialized = serialize_gsd_config(&config).expect("serialises");
        let reparsed = parse_gsd_config(&serialized).expect("re-parses");
        let reserialized = serialize_gsd_config(&reparsed).expect("re-serialises");
        assert_eq!(
            serialized, reserialized,
            "the write path is not a fixed point: an unmodelled key changed \
             shape or vanished across one save/reload cycle"
        );
        assert!(serialized.contains("unknown_gsd_key"));
        assert!(serialized.contains("brand_new_block"));
        assert!(serialized.contains("\"codex\""));
    }

    /// The tracer key of quick task 260916-vqw: the one gsd-core 1.14 key the
    /// todo named by hand, modelled end to end.
    #[test]
    fn compact_content_is_a_modelled_workflow_key() {
        let config = parse_gsd_config(r#"{"workflow": {"compact_content": true}}"#).unwrap();
        let wf = config.workflow.as_ref().unwrap();
        assert_eq!(wf.compact_content, Some(true));
        assert!(
            !wf.extra.contains_key("compact_content"),
            "compact_content reached the pass-through map, so it is NOT modelled"
        );
        let reparsed = parse_gsd_config(&serialize_gsd_config(&config).unwrap()).unwrap();
        assert_eq!(reparsed.workflow.as_ref().unwrap().compact_content, Some(true));
    }

    /// The tracer key of the gsd-core release-1.15.0 re-sync (quick task
    /// 260926-gtk): `planner` became a typed block, but only its one boolean
    /// is modelled — the two stall-tuning knobs beside it must stay in the
    /// pass-through map and survive a save untouched (T-gtk-02).
    #[test]
    fn planner_stall_detection_enabled_is_a_modelled_planner_key() {
        let content = r#"{"planner":{"stall_detection_enabled":false,"stall_detect_interval_minutes":5,"stall_threshold_minutes":10}}"#;
        let config = parse_gsd_config(content).expect("the planner fixture parses");
        let planner = config.planner.as_ref().expect("planner is a typed block");
        assert_eq!(planner.stall_detection_enabled, Some(false));

        let mut extra: Vec<&str> = planner.extra.keys().map(String::as_str).collect();
        extra.sort_unstable();
        assert_eq!(
            extra,
            ["stall_detect_interval_minutes", "stall_threshold_minutes"],
            "the planner pass-through map must hold exactly the two tuning knobs"
        );
        assert!(
            !config.extra.contains_key("planner"),
            "planner fell through to the top-level pass-through map, so it is NOT modelled"
        );

        let serialized = serialize_gsd_config(&config).expect("serialises");
        let reparsed = parse_gsd_config(&serialized).expect("re-parses");
        assert_eq!(
            serialized,
            serialize_gsd_config(&reparsed).unwrap(),
            "the save path is not a fixed point over the planner block"
        );
        assert_eq!(
            reparsed.planner.as_ref().unwrap().stall_detection_enabled,
            Some(false)
        );
    }

    /// The second gsd-core release-1.15.0 key (#4223, capability-registered in
    /// capabilities/ui/capability.json rather than the schema manifest).
    #[test]
    fn ui_interaction_capture_is_a_modelled_workflow_key() {
        let config =
            parse_gsd_config(r#"{"workflow": {"ui_interaction_capture": true}}"#).unwrap();
        let wf = config.workflow.as_ref().unwrap();
        assert_eq!(wf.ui_interaction_capture, Some(true));
        assert!(
            !wf.extra.contains_key("ui_interaction_capture"),
            "ui_interaction_capture reached the pass-through map, so it is NOT modelled"
        );
        let reparsed = parse_gsd_config(&serialize_gsd_config(&config).unwrap()).unwrap();
        assert_eq!(
            reparsed.workflow.as_ref().unwrap().ui_interaction_capture,
            Some(true)
        );

        let without = parse_gsd_config(r#"{"workflow": {"research": true}}"#).unwrap();
        assert!(
            !serialize_gsd_config(&without)
                .unwrap()
                .contains("ui_interaction_capture"),
            "an unset ui_interaction_capture was invented on save"
        );
    }

    /// Every `workflow.*` key the gsd-core 1.14.0 re-sync added, parsed from
    /// GSD's own spelling and round-tripped.
    ///
    /// The round trip is the load-bearing half: a field that parses and then
    /// serialises under a different name, or not at all, is a key the Defaults
    /// tab can read and cannot save — which is the failure mode the whole quick
    /// task exists to remove.
    #[test]
    fn the_resynced_workflow_keys_parse_and_round_trip() {
        let content = r#"{
            "workflow": {
                "agent_hint_routing": false,
                "auto_prune_state": true,
                "build_command": "cargo build",
                "code_review_depth_overrides": [{ "paths": ["src/auth"], "depth": "deep" }],
                "code_review_point": "execute:wave:post",
                "compact_content": true,
                "context_coverage_gate": false,
                "context_drift_action": "block",
                "context_drift_precheck": false,
                "cross_ai_command": "codex exec -",
                "cross_ai_execution": true,
                "cross_ai_timeout": 600,
                "drift_action": "auto-remap",
                "drift_threshold": 5,
                "human_verify_mode": "mid-flight",
                "inline_plan_threshold": 4,
                "live_dom_uat": true,
                "max_discuss_passes": 2,
                "plan_bounce": true,
                "plan_bounce_passes": 3,
                "plan_bounce_script": "./scripts/bounce.sh",
                "plan_review_convergence": true,
                "post_planning_gaps": false,
                "security_enforcement": false,
                "smart_zone_tokens": 125000,
                "test_command": "cargo test --no-fail-fast",
                "worktree_skip_hooks": true
            }
        }"#;
        let config = parse_gsd_config(content).expect("the re-synced workflow fixture parses");
        let wf = config.workflow.as_ref().expect("the workflow block parsed");

        assert!(
            wf.extra.is_empty(),
            "these keys are supposed to be MODELLED, but {:?} fell through to the \
             pass-through map",
            wf.extra.keys().collect::<Vec<_>>()
        );

        assert_eq!(wf.agent_hint_routing, Some(false));
        assert_eq!(wf.auto_prune_state, Some(true));
        assert_eq!(wf.build_command.as_deref(), Some("cargo build"));
        assert!(wf.code_review_depth_overrides.as_ref().unwrap().is_array());
        assert_eq!(wf.code_review_point.as_deref(), Some("execute:wave:post"));
        assert_eq!(wf.compact_content, Some(true));
        assert_eq!(wf.context_coverage_gate, Some(false));
        assert_eq!(wf.context_drift_action.as_deref(), Some("block"));
        assert_eq!(wf.context_drift_precheck, Some(false));
        assert_eq!(wf.cross_ai_command.as_deref(), Some("codex exec -"));
        assert_eq!(wf.cross_ai_execution, Some(true));
        assert_eq!(wf.cross_ai_timeout, Some(600));
        assert_eq!(wf.drift_action.as_deref(), Some("auto-remap"));
        assert_eq!(wf.drift_threshold, Some(5));
        assert_eq!(wf.human_verify_mode.as_deref(), Some("mid-flight"));
        assert_eq!(wf.inline_plan_threshold, Some(4));
        assert_eq!(wf.live_dom_uat, Some(true));
        assert_eq!(wf.max_discuss_passes, Some(2));
        assert_eq!(wf.plan_bounce, Some(true));
        assert_eq!(wf.plan_bounce_passes, Some(3));
        assert_eq!(wf.plan_bounce_script.as_deref(), Some("./scripts/bounce.sh"));
        assert_eq!(wf.plan_review_convergence, Some(true));
        assert_eq!(wf.post_planning_gaps, Some(false));
        assert_eq!(wf.security_enforcement, Some(false));
        assert_eq!(wf.smart_zone_tokens, Some(125000));
        assert_eq!(wf.test_command.as_deref(), Some("cargo test --no-fail-fast"));
        assert_eq!(wf.worktree_skip_hooks, Some(true));

        let serialized = serialize_gsd_config(&config).expect("serialises");
        let reparsed = parse_gsd_config(&serialized).expect("re-parses");
        let rwf = reparsed.workflow.as_ref().unwrap();
        assert_eq!(rwf.agent_hint_routing, Some(false));
        assert_eq!(rwf.smart_zone_tokens, Some(125000));
        assert_eq!(rwf.human_verify_mode.as_deref(), Some("mid-flight"));
        assert_eq!(rwf.test_command.as_deref(), Some("cargo test --no-fail-fast"));
        assert!(rwf.code_review_depth_overrides.as_ref().unwrap().is_array());
        assert_eq!(
            serialized,
            serialize_gsd_config(&reparsed).unwrap(),
            "the save path is not a fixed point over the re-synced keys"
        );
    }

    /// A `workflow` block that names none of the new keys leaves every one of
    /// them unset, so an untouched project's config is not rewritten with
    /// defaults this build invented.
    #[test]
    fn the_resynced_workflow_keys_default_to_unset() {
        let config = parse_gsd_config(r#"{"workflow": {"research": true}}"#).unwrap();
        let wf = config.workflow.as_ref().unwrap();
        assert!(wf.agent_hint_routing.is_none());
        assert!(wf.build_command.is_none());
        assert!(wf.code_review_point.is_none());
        assert!(wf.compact_content.is_none());
        assert!(wf.smart_zone_tokens.is_none());
        assert!(wf.worktree_skip_hooks.is_none());
        let serialized = serialize_gsd_config(&config).unwrap();
        assert!(!serialized.contains("agent_hint_routing"));
        assert!(!serialized.contains("smart_zone_tokens"));
    }

    /// The 30 non-`workflow` keys the gsd-core 1.14.0 re-sync added, including
    /// four blocks this build had never seen at all.
    #[test]
    fn the_resynced_non_workflow_keys_parse_and_round_trip() {
        let content = r#"{
            "context_window": 1000000,
            "features": { "global_learnings": true, "thinking_partner": true },
            "gates": {
                "confirm_breakdown": false,
                "confirm_phases": false,
                "confirm_plan": false,
                "confirm_project": false,
                "confirm_roadmap": false,
                "confirm_transition": false,
                "execute_next_plan": false,
                "issues_review": false
            },
            "planning": {
                "chunked_parallel": true,
                "commit_docs": false,
                "pr_strict": true,
                "search_gitignored": true,
                "sub_repos": ["backend", "frontend"]
            },
            "plan_review": {
                "source_grounding": false,
                "source_grounding_authority": "intel"
            },
            "git": {
                "create_tag": false,
                "allow_default_branch_commits": true,
                "protected_branches": ["release", "staging"]
            },
            "hooks": {
                "workflow_guard": true,
                "context_warning_threshold": 40,
                "context_critical_threshold": 20
            },
            "graphify": { "auto_update": true },
            "statusline": {
                "context_position": "front",
                "show_last_command": true,
                "show_state_freshness": true
            },
            "dynamic_routing": { "enabled": true, "escalate_on_failure": false }
        }"#;
        let config = parse_gsd_config(content).expect("the re-synced fixture parses");

        assert_eq!(config.context_window, Some(1_000_000));
        let features = config.features.as_ref().unwrap();
        assert_eq!(features.global_learnings, Some(true));
        assert_eq!(features.thinking_partner, Some(true));
        let gates = config.gates.as_ref().unwrap();
        assert_eq!(gates.confirm_breakdown, Some(false));
        assert_eq!(gates.issues_review, Some(false));
        assert!(gates.extra.is_empty(), "a gates.* key fell through: {:?}", gates.extra);
        let planning = config.planning.as_ref().unwrap();
        assert_eq!(planning.chunked_parallel, Some(true));
        assert_eq!(planning.commit_docs, Some(false));
        assert_eq!(planning.pr_strict, Some(true));
        assert_eq!(planning.search_gitignored, Some(true));
        assert!(planning.sub_repos.as_ref().unwrap().is_array());
        // The namespaced spellings are a DIFFERENT path from the top-level
        // ones, and modelling both is deliberate — see `GsdConfig::planning`.
        assert!(
            config.commit_docs.is_none() && config.search_gitignored.is_none(),
            "planning.* leaked into the top-level keys, so the tab would show a \
             value at a path the file does not use"
        );
        let review = config.plan_review.as_ref().unwrap();
        assert_eq!(review.source_grounding, Some(false));
        assert_eq!(review.source_grounding_authority.as_deref(), Some("intel"));
        let git = config.git.as_ref().unwrap();
        assert_eq!(git.create_tag, Some(false));
        assert_eq!(git.allow_default_branch_commits, Some(true));
        assert!(git.protected_branches.as_ref().unwrap().is_array());
        let hooks = config.hooks.as_ref().unwrap();
        assert_eq!(hooks.workflow_guard, Some(true));
        assert_eq!(hooks.context_warning_threshold, Some(40));
        assert_eq!(hooks.context_critical_threshold, Some(20));
        assert_eq!(config.graphify.as_ref().unwrap().auto_update, Some(true));
        let sl = config.statusline.as_ref().unwrap();
        assert_eq!(sl.context_position.as_deref(), Some("front"));
        assert_eq!(sl.show_last_command, Some(true));
        assert_eq!(sl.show_state_freshness, Some(true));
        let dr = config.dynamic_routing.as_ref().unwrap();
        assert_eq!(dr.enabled, Some(true));
        assert_eq!(dr.escalate_on_failure, Some(false));

        let serialized = serialize_gsd_config(&config).expect("serialises");
        let reparsed = parse_gsd_config(&serialized).expect("re-parses");
        assert_eq!(reparsed.context_window, Some(1_000_000));
        assert_eq!(
            reparsed.gates.as_ref().unwrap().confirm_plan,
            Some(false)
        );
        assert_eq!(
            reparsed.plan_review.as_ref().unwrap().source_grounding_authority.as_deref(),
            Some("intel")
        );
        assert_eq!(
            serialized,
            serialize_gsd_config(&reparsed).unwrap(),
            "the save path is not a fixed point over the re-synced blocks"
        );
    }

    /// The re-synced blocks are omitted entirely from a default config, so
    /// saving an untouched project does not invent `gates` / `features` /
    /// `planning` / `plan_review` / `planner` sections gsd-core never saw.
    #[test]
    fn the_resynced_blocks_are_absent_from_a_default_config() {
        let serialized = serialize_gsd_config(&GsdConfig::default()).unwrap();
        for block in ["features", "gates", "planning", "plan_review", "planner", "context_window"] {
            assert!(
                !serialized.contains(block),
                "a default config emitted {block:?}"
            );
        }
        let empty = parse_gsd_config("{}").unwrap();
        assert!(empty.features.is_none());
        assert!(empty.gates.is_none());
        assert!(empty.planning.is_none());
        assert!(empty.plan_review.is_none());
        assert!(empty.planner.is_none());
        assert!(empty.context_window.is_none());
    }

    /// ID-1: the baseline a future sync diffs from has to be measured, and a
    /// placeholder left in it is worse than no constant at all.
    #[test]
    fn the_gsd_core_sync_baseline_is_recorded() {
        assert!(
            GSD_CORE_SYNCED_VERSION
                .split('.')
                .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit())),
            "GSD_CORE_SYNCED_VERSION is {GSD_CORE_SYNCED_VERSION:?}, which is not a \
             package.json version — re-measure it, do not guess"
        );
        assert!(
            GSD_CORE_SYNCED_COMMIT.starts_with('v') && GSD_CORE_SYNCED_COMMIT.len() >= 7,
            "GSD_CORE_SYNCED_COMMIT is {GSD_CORE_SYNCED_COMMIT:?}, which is not a \
             `git describe --tags --always` output"
        );
        assert!(
            GSD_CORE_SYNCED_COMMIT.contains(GSD_CORE_SYNCED_VERSION),
            "the commit describe {GSD_CORE_SYNCED_COMMIT:?} does not name version \
             {GSD_CORE_SYNCED_VERSION:?} — the two halves of the baseline were \
             measured at different times"
        );

        // The installed-GSD comparison (quick 260926-j0a) uses these two as the
        // floor and ceiling of the in-sync range, and `relation_to_synced`
        // `.expect()`s that both parse.
        use crate::state_reader::gsd_install::GsdVersion;
        let floor = GsdVersion::parse(GSD_CORE_SYNCED_VERSION)
            .expect("GSD_CORE_SYNCED_VERSION must be a semver version");
        let ceiling = GsdVersion::parse(GSD_CORE_SYNCED_TREE_VERSION)
            .expect("GSD_CORE_SYNCED_TREE_VERSION must be a semver version");
        assert_ne!(
            floor.precedence_cmp(&ceiling),
            std::cmp::Ordering::Greater,
            "the oracle pin {GSD_CORE_SYNCED_VERSION:?} is above the synced tree \
             {GSD_CORE_SYNCED_TREE_VERSION:?} — the in-sync range is empty"
        );
    }

    /// Quick task 260926-jnf (T-jnf-05, T-jnf-04). gsd-core types the three
    /// search-provider slots `string | boolean | null` — a string is the API
    /// key itself. Typed `Option<bool>` here, a key made the WHOLE file a parse
    /// failure (no Defaults rows, and the serde error quoted it into the log).
    #[test]
    fn a_search_provider_slot_holding_an_api_key_parses_and_round_trips_secret() {
        let content = r#"{"brave_search":"BSA-SECRET-Q7Z9","firecrawl":true,"exa_search":null}"#;
        let config = parse_gsd_config(content).expect("an API-key string in a search slot parses");
        assert_eq!(
            config.brave_search,
            Some(ApiKeySetting::Key("BSA-SECRET-Q7Z9".to_string()))
        );
        assert_eq!(config.firecrawl, Some(ApiKeySetting::Flag(true)));
        assert_eq!(config.exa_search, None);

        let serialized = serialize_gsd_config(&config).expect("serialises");
        let reparsed = parse_gsd_config(&serialized).expect("re-parses");
        assert_eq!(
            serialized,
            serialize_gsd_config(&reparsed).unwrap(),
            "the save path is not a fixed point over the search-provider slots"
        );
        assert_eq!(reparsed.brave_search, config.brave_search);

        let debug = format!("{config:?}");
        assert!(
            !debug.contains("BSA-SECRET-Q7Z9") && !debug.contains("Q7Z9"),
            "Debug of a config printed the API key: {debug}"
        );
    }

    /// Quick task 260926-jnf: the seven real top-level gsd-core keys that were
    /// still pass-through are TYPED, so none of them lands in `extra`, and an
    /// untouched config does not invent them on save.
    #[test]
    fn the_promoted_top_level_keys_are_typed_not_extra() {
        let config = parse_gsd_config(
            r#"{"runtime":"codex","context_profile":"review","agent_skills":{},"tavily_search":"T-SECRET-M1M1","jina":true}"#,
        )
        .expect("the promoted-key fixture parses");
        assert_eq!(config.runtime.as_deref(), Some("codex"));
        assert_eq!(config.context_profile.as_deref(), Some("review"));
        assert_eq!(config.agent_skills, Some(serde_json::json!({})));
        assert_eq!(
            config.tavily_search,
            Some(ApiKeySetting::Key("T-SECRET-M1M1".to_string()))
        );
        assert_eq!(config.jina, Some(ApiKeySetting::Flag(true)));
        assert!(config.ref_search.is_none() && config.perplexity.is_none());
        assert!(
            config.extra.is_empty(),
            "a promoted key fell through to the pass-through map: {:?}",
            config.extra.keys().collect::<Vec<_>>()
        );

        let empty = serialize_gsd_config(&parse_gsd_config("{}").unwrap()).unwrap();
        for key in [
            "tavily_search", "ref_search", "perplexity", "jina", "runtime", "context_profile",
            "agent_skills",
        ] {
            assert!(!empty.contains(key), "an unset `{key}` was invented on save");
        }
    }

    /// T-jnf-03: serde's Display quotes the offending value, so logging it
    /// would put a mistyped secret into the log file. The summary carries
    /// only the error category and position.
    #[test]
    fn a_config_parse_error_summary_never_quotes_the_value_secret() {
        let e = serde_json::from_str::<GsdConfig>(r#"{"workflow":{"research":"sk-SECRET-F6Y4"}}"#)
            .expect_err("a string where a boolean belongs is a parse error");
        assert!(
            e.to_string().contains("F6Y4"),
            "precondition: the raw serde Display DOES quote the value: {e}"
        );
        let summary = parse_error_summary(&e);
        assert!(!summary.contains("F6Y4") && !summary.contains("SECRET"), "{summary}");
        assert!(summary.contains(&format!("line {}", e.line())), "{summary}");
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
