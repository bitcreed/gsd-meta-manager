use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// The registry schema version this build writes.
///
/// **Version 2 means one thing and only one thing: a registry entry may carry a
/// [`DriverOptIn`] record.** A version 1 `config.json` — every config any
/// pre-Phase-17 binary ever wrote — loads unchanged, because
/// [`RegisteredProject::driver_opt_in`] is `#[serde(default)]` and the default is
/// `None`.
///
/// `Config::version` deliberately has **no** `#[serde(default)]`: a config
/// lacking the field already fails to parse today. That is the existing
/// schema-version discipline, and relaxing it here would be an unrelated
/// behaviour change smuggled in under a migration (D-15).
pub const CONFIG_SCHEMA_VERSION: u32 = 2;

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
    /// Every field of this entry that this build does not model.
    ///
    /// The same tolerance technique `JournalRecord.rest` uses
    /// (`src/journal/reader.rs:196-211`), applied to a file **two binary
    /// versions may share**. Without it, a v1.6 binary loading a config written
    /// by a v2.1 binary and then saving it would silently delete every field it
    /// had never heard of — a destructive rewrite by omission, and precisely
    /// what D-15 forbids. A downgrade must preserve, never delete.
    ///
    /// `#[serde(flatten)]` supplies the default-when-absent semantics for free:
    /// an entry with no unknown fields deserialises to an empty map and
    /// serialises back to nothing at all. That is asserted by
    /// `a_config_written_by_a_newer_binary_keeps_its_unknown_fields_and_its_version`
    /// rather than assumed.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
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

/// The default number of driver processes that may run at once: **1**.
///
/// A free function rather than a literal because `#[serde(default = "…")]` needs
/// one, and because the value has a reason that belongs next to it — see
/// [`Preferences::driver_max_concurrent`].
fn default_driver_max_concurrent() -> usize {
    1
}

/// **`Default` is deliberately NOT derived on this struct.**
///
/// A derived `Default` would yield `usize::default()` for
/// [`Preferences::driver_max_concurrent`], which is `0`, which means "no run may
/// ever start" — a silent, total denial of service dressed up as a default. This
/// is the one place D-18's "almost free" costs more than a field, and the
/// two-armed test
/// `driver_max_concurrent_defaults_to_one_from_default_and_from_empty_json`
/// exists to fail on the derived version.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Preferences {
    #[serde(default)]
    pub hooks: HooksConfig,
    #[serde(default)]
    pub gsd_integration: bool,
    /// How many driver processes may run concurrently across the whole fleet.
    ///
    /// **The fleet dimension enters v2.0 as a count, never as a second run
    /// identity (D-18).** One driver per project is what makes the kill switch
    /// simple — one pgid, one project, one `RunRecord` — and fleet-level driving
    /// is deferred past v2.0. What Phase 20 will actually want is a global
    /// concurrency cap, and this field is its seam.
    ///
    /// The default is **1** because the 5h/7d Claude quota is shared across every
    /// surface the user has: N concurrently driven projects burn it N times over
    /// (PITFALLS:329). Phase 20 inherits a working cap instead of building one.
    ///
    /// **Enforcement is not here.** The count is compared against live runs at
    /// the spawn seam in plan 17-05; this plan ships the preference and its
    /// default, so nobody should look for the policy in this file.
    #[serde(default = "default_driver_max_concurrent")]
    pub driver_max_concurrent: usize,
    /// Every preference this build does not model.
    ///
    /// See [`RegisteredProject::extra`] — same technique, same reason, same
    /// shared-file hazard.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Preferences {
    fn default() -> Self {
        Preferences {
            hooks: HooksConfig::default(),
            gsd_integration: false,
            driver_max_concurrent: default_driver_max_concurrent(),
            extra: Map::new(),
        }
    }
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
            version: CONFIG_SCHEMA_VERSION,
            projects: HashMap::new(),
            preferences: Preferences::default(),
        }
    }
}

/// Bring a freshly parsed config up to [`CONFIG_SCHEMA_VERSION`] **in memory**.
///
/// The shape is the v1.6 QUEUE relocation's: a **read-side fallback** plus a
/// **write-side one-shot**, and no destructive rewrite of a file an older binary
/// might still be reading (`src/state_reader/queue_md.rs:189-230`). The write
/// side needs no code at all — `migrate` has already set `version` in memory, so
/// the next ordinary [`save_config`] stamps the file as a side effect of a save
/// the user was making anyway.
///
/// Three rules, in this order:
///
/// 1. `version < CONFIG_SCHEMA_VERSION` → set `version = CONFIG_SCHEMA_VERSION`
///    and **change nothing else**. Every `driver_opt_in` is already `None`,
///    because `#[serde(default)]` produced it, and **that is the entire
///    migration**: an old config has no opted-in project and must not acquire one
///    by being read. Silent enrollment is the single failure FEATURES:236 names
///    as *"invisible until the agent has already committed"*, so a byte-for-byte
///    literal fixture asserts it rather than this comment claiming it.
/// 2. `version > CONFIG_SCHEMA_VERSION` → leave it alone and warn, naming both
///    numbers and nothing else. The file was written by a newer binary; stamping
///    the version back down would tell that binary its own migration had already
///    run, and [`RegisteredProject::extra`] is what keeps its data intact in the
///    meantime.
/// 3. Equal → nothing happens, which is what makes this **idempotent by
///    construction**. `the_migration_is_idempotent` asserts it.
fn migrate(config: &mut Config) {
    if config.version < CONFIG_SCHEMA_VERSION {
        config.version = CONFIG_SCHEMA_VERSION;
    } else if config.version > CONFIG_SCHEMA_VERSION {
        tracing::warn!(
            recorded = config.version,
            supported = CONFIG_SCHEMA_VERSION,
            "the config records a newer schema version than this build supports",
        );
    }
}

/// Load config from the given path. Returns a new default Config if the file doesn't exist.
pub fn load_config(path: &Path) -> anyhow::Result<Config> {
    if !path.exists() {
        return Ok(Config::new());
    }
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;
    let mut config: Config = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse config JSON: {}", path.display()))?;
    migrate(&mut config);
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

#[cfg(test)]
mod tests {
    use super::*;

    /// A real pre-Phase-17 `config.json`, byte for byte.
    ///
    /// **Deliberately a literal, never a serialised `Config`.** Serialising a
    /// `Config` and reading it back tests the round-trip; only a literal written
    /// the way the old binary wrote it tests the *migration*, which is the thing
    /// that decides whether a user who upgrades wakes up with an opted-in
    /// project (D-15).
    const PRE_PHASE_17_CONFIG: &str = r#"{
  "version": 1,
  "projects": {
    "alpha": {
      "path": "/home/testuser/projects/alpha",
      "added": "2026-01-04T09:15:00+00:00"
    },
    "beta": {
      "path": "/home/testuser/projects/beta",
      "added": "2026-02-17T22:41:03+00:00"
    }
  },
  "preferences": {
    "hooks": {
      "pre_create": null,
      "post_create": null
    },
    "gsd_integration": true
  }
}"#;

    /// A config written by a binary from this build's future.
    ///
    /// Two invented keys — one on a registry entry, one on preferences — plus a
    /// schema version well above [`CONFIG_SCHEMA_VERSION`].
    const FUTURE_CONFIG: &str = r#"{
  "version": 99,
  "projects": {
    "gamma": {
      "path": "/home/testuser/projects/gamma",
      "added": "2027-03-01T00:00:00+00:00",
      "driver_quarantine_until": "2027-04-01T00:00:00+00:00"
    }
  },
  "preferences": {
    "hooks": {
      "pre_create": null,
      "post_create": null
    },
    "gsd_integration": false,
    "driver_nice_level": 7
  }
}"#;

    fn write_config(dir: &Path, body: &str) -> PathBuf {
        let path = dir.join("config.json");
        std::fs::write(&path, body).expect("the fixture config is writable");
        path
    }

    #[test]
    fn a_pre_phase_17_config_loads_with_every_project_not_opted_in() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = write_config(dir.path(), PRE_PHASE_17_CONFIG);

        let config = load_config(&path).expect("a version 1 config must still parse");

        assert_eq!(
            config.version, CONFIG_SCHEMA_VERSION,
            "the read-side migration lifts the recorded version in memory"
        );
        assert_eq!(config.projects.len(), 2, "both projects survive the load");

        // The load-bearing assertion of this entire plan.
        for (alias, entry) in &config.projects {
            assert!(
                entry.driver_opt_in.is_none(),
                "reading an old config must never enrol a project into being \
                 driven, but '{alias}' came back opted in (D-15, FEATURES:236)"
            );
        }

        assert_eq!(
            config.preferences.driver_max_concurrent, 1,
            "a preferences object with no such key defaults to one, not zero"
        );
        assert!(
            config.preferences.gsd_integration,
            "the pre-existing preference is carried through unchanged"
        );

        let alpha = &config.projects["alpha"];
        assert_eq!(alpha.path, PathBuf::from("/home/testuser/projects/alpha"));
        assert_eq!(alpha.added, "2026-01-04T09:15:00+00:00");
        let beta = &config.projects["beta"];
        assert_eq!(beta.path, PathBuf::from("/home/testuser/projects/beta"));
        assert_eq!(beta.added, "2026-02-17T22:41:03+00:00");
    }

    #[test]
    fn the_migration_is_idempotent() {
        let mut config: Config =
            serde_json::from_str(PRE_PHASE_17_CONFIG).expect("the fixture parses");

        migrate(&mut config);
        let once = serde_json::to_string_pretty(&config).expect("serialises");
        migrate(&mut config);
        let twice = serde_json::to_string_pretty(&config).expect("serialises");

        assert_eq!(
            once, twice,
            "running the migration on an already-migrated config must change nothing"
        );
        assert_eq!(config.version, CONFIG_SCHEMA_VERSION);
    }

    #[test]
    fn a_config_written_by_a_newer_binary_keeps_its_unknown_fields_and_its_version() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = write_config(dir.path(), FUTURE_CONFIG);

        let config = load_config(&path).expect("a future config must still parse");

        assert_eq!(
            config.version, 99,
            "a higher recorded version is never downgraded — doing so would tell \
             the newer binary its own migration had already run"
        );

        save_config(&config, &path).expect("the round-tripped config saves");
        let text = std::fs::read_to_string(&path).expect("the saved config is readable");

        assert!(
            text.contains("driver_quarantine_until"),
            "an unknown registry-entry field must survive a save by this build; \
             deleting it would be a destructive rewrite by omission. Got:\n{text}"
        );
        assert!(
            text.contains("driver_nice_level"),
            "an unknown preferences field must survive a save by this build. Got:\n{text}"
        );
        assert!(
            text.contains("\"version\": 99"),
            "the recorded version survives the save verbatim. Got:\n{text}"
        );
    }

    #[test]
    fn driver_max_concurrent_defaults_to_one_from_default_and_from_empty_json() {
        // Two arms because there are two independent ways to reach a zero, and
        // each arm catches exactly one of them. This one fails if `Default` is
        // derived: `usize::default()` is 0.
        assert_eq!(
            Preferences::default().driver_max_concurrent,
            1,
            "a derived `Default` would make this 0, which means 'no run may ever \
             start' (D-18)"
        );
        // And this one fails if the serde attribute is a bare
        // `#[serde(default)]` rather than the named function — the half that only
        // shows up when a real config predating the field is loaded.
        assert_eq!(
            serde_json::from_str::<Preferences>("{}")
                .expect("an empty preferences object parses")
                .driver_max_concurrent,
            1,
            "an absent key must default to one via `default_driver_max_concurrent`"
        );
    }
}
