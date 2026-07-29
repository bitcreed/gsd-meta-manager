use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    pub projects: HashMap<String, RegisteredProject>,
    #[serde(default)]
    pub preferences: Preferences,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegisteredProject {
    pub path: PathBuf,
    pub added: String,
    /// The user's driver opt-in, or `None` for a project that may only be read.
    ///
    /// `#[serde(default)]` — the first serde attribute on this struct — makes
    /// every pre-Phase-17 `config.json` load unchanged, with every project
    /// coming back not-opted-in.
    ///
    /// Adding this field breaks the struct literals in `src/registry.rs`, and
    /// **that breakage is the feature**: the discovery path
    /// (`auto_register_from_sessions`) must now write `None` explicitly, so a
    /// compile error — not a code review — is what proves discovery can never
    /// silently enrol a project into being driven (D-14, D-15).
    #[serde(default)]
    pub driver_opt_in: Option<DriverOptIn>,
}

/// The user's deliberate designation of a project as drivable (D-14).
///
/// **A record, not a `bool`**, for three reasons and none of them is style:
///
/// 1. A bare `bool` on a registry entry is the wrong answer outright — it makes
///    "may be driven" indistinguishable from "was registered".
/// 2. A record cannot be accidentally true. `Some(record)` requires deliberate
///    construction; a defaulted `false` flipping to `true` has no analogue.
/// 3. Phase 21 re-confirms the opt-in when `CLAUDE.md` drifts, and adding the
///    digest later would be a second migration of a user-owned file.
///
/// Phase 17 **records** the digest and acts on nothing.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DriverOptIn {
    /// RFC3339 UTC at **second** precision.
    ///
    /// Produced by `chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs,
    /// true)` — the same call `JournalRun::finish` uses, deliberately not
    /// `registry.rs`'s bare `to_rfc3339()`, so an opt-in stamp and a run's
    /// `ended_at` compare directly without normalising.
    pub opted_in_at: String,
    /// A digest of the project's `CLAUDE.md` as it stood at opt-in time.
    ///
    /// Recorded now, acted on by **nothing** in this phase. Phase 21 re-confirms
    /// the opt-in on drift.
    pub claude_md_digest: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct HooksConfig {
    pub pre_create: Option<String>,
    pub post_create: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Preferences {
    #[serde(default)]
    pub hooks: HooksConfig,
    #[serde(default)]
    pub gsd_integration: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

impl Config {
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("gsd-meta-manager")
            .join("config.json")
    }

    pub fn new() -> Self {
        Config {
            version: 1,
            projects: HashMap::new(),
            preferences: Preferences::default(),
        }
    }
}

/// Load config from the given path. Returns a new default Config if the file doesn't exist.
pub fn load_config(path: &Path) -> anyhow::Result<Config> {
    if !path.exists() {
        return Ok(Config::new());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;
    let config: Config = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config JSON: {}", path.display()))?;
    Ok(config)
}

/// Save config atomically using tempfile + rename.
pub fn save_config(config: &Config, path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let dir = path.parent().unwrap_or(Path::new("."));
    let mut tmp = NamedTempFile::new_in(dir).context("Failed to create temp file for config")?;
    let json = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    tmp.write_all(json.as_bytes())
        .context("Failed to write config to temp file")?;
    tmp.persist(path)
        .with_context(|| format!("Failed to persist config to {}", path.display()))?;

    Ok(())
}
