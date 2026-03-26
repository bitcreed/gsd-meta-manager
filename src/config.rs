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
}

impl Config {
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from(".config"))
            .join("gsd-manager")
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
    let content =
        std::fs::read_to_string(path).with_context(|| format!("Failed to read config: {}", path.display()))?;
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
