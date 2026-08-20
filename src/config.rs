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
/// 3. Phase 21 re-confirms the opt-in when a disclosed file drifts, and adding
///    the digest later would be a second migration of a user-owned file.
///
/// **Phase 21 is that phase, and it has landed.** The record now carries
/// [`DriverOptIn::prompt_inputs`] — the files whose bytes can reach a model
/// prompt, each with a `sha256:` digest — and the drift check runs at the spawn
/// gate. Reason 3 above paid off exactly as intended: no user's `config.json`
/// needed a migration, because the record was already a struct.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct DriverOptIn {
    /// RFC3339 UTC at **second** precision.
    ///
    /// Produced by `chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs,
    /// true)` — the same call `JournalRun::finish` uses, deliberately not
    /// `registry.rs`'s bare `to_rfc3339()`, so an opt-in stamp and a run's
    /// `ended_at` compare directly without normalising.
    pub opted_in_at: String,
    /// **Legacy.** A digest of the project's `CLAUDE.md` as it stood at opt-in
    /// time, written by Phase 17 and Phase 19 binaries.
    ///
    /// **Nothing writes this any more** — Phase 21 superseded it with
    /// [`DriverOptIn::prompt_inputs`], which covers `CLAUDE.md` *and* the other
    /// files whose bytes can reach a prompt, and does so with a digest that is
    /// a security control rather than an FNV-1a fingerprint that says in its own
    /// documentation that it is not one (C-4).
    ///
    /// It is **kept, not removed**, and that is the whole reason no user needs a
    /// migration: an older binary that loads and re-saves this file still finds
    /// the field it expects. Nothing reads it as an integrity check — a record
    /// carrying only this key has an empty `prompt_inputs`, and an empty
    /// `prompt_inputs` re-confirms.
    pub claude_md_digest: Option<String>,
    /// The files whose bytes can reach a model prompt, each with the digest it
    /// had when the user opted in.
    ///
    /// **The safe reading is one reading for five different states.** An absent
    /// list, an entry that does not parse, an entry whose digest prefix names a
    /// hash family this binary does not compute, an entry whose digest no longer
    /// matches the file on disk, and an entry for a file that has appeared or
    /// vanished since opt-in **all mean the same thing: re-confirm.** That is
    /// deliberately the same register as
    /// [`DriverOptIn::branch_namespace`]'s "an absent value and a rejected value
    /// therefore mean the same thing, which is the safe thing" — a gate with one
    /// answer for every unclear state has no unclear state left to get wrong.
    ///
    /// `#[serde(default)]` so every pre-Phase-21 `config.json` loads unchanged,
    /// coming back with an empty list — which re-confirms, which is correct: a
    /// record written before this field existed never disclosed anything to the
    /// user, so it cannot stand as an informed approval of what reaches a prompt.
    #[serde(default)]
    pub prompt_inputs: Vec<PromptInput>,
    /// Every field of this record that this build does not model.
    ///
    /// **The same technique as [`RegisteredProject::extra`], and it belongs here
    /// for a reason that was measured rather than assumed.** That flatten sits
    /// on the *enclosing* struct, and a second sits on [`Preferences`] — but
    /// until Phase 21 there was none here, so a key nested inside
    /// `driver_opt_in` was silently deleted by a load-and-save while a key one
    /// level up survived. A probe against the real `load_config`/`save_config`
    /// confirmed it: entry-level unknown survived, nested unknown did not.
    ///
    /// That made T-21-20's stated mitigation — "the existing flatten-preservation
    /// posture" — a paper mitigation at this nesting level, and it would have
    /// left `prompt_inputs` deletable by any older binary that merely opened the
    /// config.
    ///
    /// **The failure mode was fail-safe, not a vulnerability**, and the
    /// distinction is worth keeping: a deleted `prompt_inputs` reads as absent,
    /// absent re-confirms, so the cost was a spurious re-confirmation after a
    /// version downgrade rather than a silent approval. It is fixed anyway,
    /// because "annoying" is not a reason to leave a user-owned file lossy.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
    /// The push namespace this project's driven runs are confined to, or
    /// `None` for the default `refs/heads/gsd-auto/<alias>/` (D-30).
    ///
    /// **Never trusted raw.** `envelope::policy::validate_namespace` refuses
    /// every shape that would disable the control — a bare `refs/heads/`, a
    /// single segment, `main`/`master`/`HEAD` — and
    /// `envelope::policy::EnvelopePolicy::resolve` degrades an invalid value to
    /// the default rather than widening the boundary. An absent value and a
    /// rejected value therefore mean the same thing, which is the safe thing.
    #[serde(default)]
    pub branch_namespace: Option<String>,
    /// Where this project's driven runs get their git credential (D-18).
    ///
    /// `None` — the default — means **no credential**, which means every push
    /// fails closed with a legible reason. See [`CredentialSource`] for why
    /// there is deliberately no ambient fallback.
    #[serde(default)]
    pub credential: Option<CredentialSource>,
    /// How many pull requests a driven run of this project may open in a
    /// rolling 24 hours, or `None` for
    /// `envelope::policy::DEFAULT_PR_CAP_PER_24H`.
    ///
    /// `Option<u32>` rather than a `#[serde(default = "…")] u32` on purpose:
    /// every envelope default is decided in exactly one function (D-30), and a
    /// serde default here would be a second place a cap is chosen.
    #[serde(default)]
    pub pr_cap_per_24h: Option<u32>,
    /// How many pull requests a single driven run may open, or `None` for
    /// `envelope::policy::DEFAULT_PR_CAP_PER_RUN`. Same `Option` argument as
    /// [`DriverOptIn::pr_cap_per_24h`].
    #[serde(default)]
    pub pr_cap_per_run: Option<u32>,
}

/// One file whose bytes can reach a model prompt, and its digest at opt-in time.
///
/// **A path plus a digest, and nothing about *which* prompt profile reads it.**
/// The profile attribution is owned by
/// [`crate::registry::DISCLOSED_PROMPT_INPUTS`] in code, not recorded here, and
/// that split is deliberate: which files a build actually feeds to a model is a
/// fact about the build, so storing it in a user-owned file would let a stale
/// record disagree with the binary about what it does. The *list* is recorded —
/// so the disclosure the user approved is what gets rendered back — while the
/// label describing each entry comes from the code that does the reading.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct PromptInput {
    /// The file's path **relative to the project root** — `CLAUDE.md`,
    /// `.planning/STATE.md`. Relative rather than absolute so a record survives
    /// the project being moved or the config being synced between machines.
    pub path: String,
    /// The `sha256:`-prefixed digest of the file's bytes at opt-in time, or
    /// `None` for a file that **did not exist** then.
    ///
    /// `None` is a recorded fact, not a missing one, and conflating the two
    /// would open the hole this field exists to close: if an absent file were
    /// simply left out of the list, a `CLAUDE.md` that appears *after* opt-in
    /// would reach the executor's prompt with no digest to contradict and no
    /// re-confirmation. Recorded as `None`, its later appearance is a mismatch
    /// like any other, and mismatches re-confirm.
    pub digest: Option<String>,
    /// Every field of this entry that this build does not model.
    ///
    /// See [`DriverOptIn::extra`] — same technique, same reason, and applied
    /// here rather than only one level up because this struct lives in the same
    /// user-owned, version-shared file and would otherwise reproduce that exact
    /// bug one level deeper.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// Where a driven run's git credential comes from (D-18).
///
/// Exactly two variants, and **the absent third one is the decision**: there is
/// deliberately no literal-token variant. `config.json` lives in the registry
/// next to project paths, is written with ordinary file permissions, and is the
/// kind of file that gets synced, backed up and pasted into a bug report. It has
/// no protection posture at all, so it is not a secret store and this type will
/// not pretend otherwise by offering a field that invites one.
///
/// **The default is no credential, and no credential means every push fails
/// closed with a legible reason.** There is no ambient fallback, and that
/// absence is the requirement rather than an omission: SAFE-05 says a driven run
/// must not inherit the user's credentials, and "unconfigured" silently meaning
/// "the user's credentials" is the exact failure it names. A project with no
/// configured credential can still be driven, can read and can commit; it cannot
/// push, and it says so up front rather than at minute 90.
///
/// Tagged (`{"source": "env", …}`) so the JSON is self-describing — a registry
/// file two binary versions may share should not need a schema to be read.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(tag = "source", rename_all = "snake_case")]
pub enum CredentialSource {
    /// Read the token from a named environment variable of the TUI's own
    /// process.
    Env {
        /// The variable **name**. The value is never stored here, and never
        /// written anywhere by this build.
        var: String,
    },
    /// Run a command and read the token from its stdout — `gh auth token`,
    /// `pass show …`, `op read …`.
    Command {
        /// argv, **never a shell string**. The same rule
        /// `executor::claude::build_argv` records for the agent spawn: with no
        /// shell in the path, a value containing a flag-shaped token or a `;`
        /// cannot become a second command.
        argv: Vec<String>,
    },
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

    /// A real pre-Phase-21 `config.json`, byte for byte: an opt-in carrying the
    /// legacy `claude_md_digest` with the `fnv1a64:` prefix `journal::argv_digest`
    /// actually emits, and **no** `prompt_inputs` key at all.
    ///
    /// **A byte literal, never a serialised `Config`.** The same distinction
    /// [`PRE_PHASE_17_CONFIG`] and [`PRE_PHASE_19_CONFIG`] record: round-tripping
    /// a struct would pass even if `prompt_inputs` had been made mandatory, and
    /// mandatory is exactly the failure that would force a migration of a
    /// user-owned file.
    const PRE_PHASE_21_CONFIG: &str = r#"{
  "version": 2,
  "projects": {
    "alpha": {
      "path": "/home/testuser/projects/alpha",
      "added": "2026-01-04T09:15:00+00:00",
      "driver_opt_in": {
        "opted_in_at": "2026-07-29T11:59:00Z",
        "claude_md_digest": "fnv1a64:0123456789abcdef"
      }
    }
  },
  "preferences": {
    "hooks": {
      "pre_create": null,
      "post_create": null
    },
    "gsd_integration": false
  }
}"#;

    /// A config written by a binary from this build's future, carrying an
    /// unknown key **nested inside `driver_opt_in`**.
    ///
    /// Distinct from [`FUTURE_CONFIG`], whose invented keys sit one level up. The
    /// nesting is the whole point: until Phase 21 `DriverOptIn` carried no
    /// `extra` flatten, so a key at *this* depth was silently deleted by a
    /// load-and-save while a key at the entry level survived.
    const FUTURE_CONFIG_NESTED_UNKNOWN: &str = r#"{
  "version": 2,
  "projects": {
    "alpha": {
      "path": "/home/testuser/projects/alpha",
      "added": "2026-01-04T09:15:00+00:00",
      "driver_opt_in": {
        "opted_in_at": "2026-07-29T11:59:00Z",
        "claude_md_digest": null,
        "prompt_inputs": [],
        "driver_attestation_v3": "a key this build has never heard of"
      }
    }
  },
  "preferences": {}
}"#;

    /// A real pre-Phase-19 `config.json`, byte for byte: schema version 2, one
    /// opted-in project, and a `driver_opt_in` record carrying exactly the two
    /// fields Phase 17 wrote.
    ///
    /// **Deliberately a literal, never a serialised `Config`** — the same
    /// distinction [`PRE_PHASE_17_CONFIG`] records. Serialising a `Config` and
    /// reading it back tests the round-trip, which would pass even if every new
    /// field had been made mandatory; only a literal written the way the old
    /// binary wrote it tests the *migration*, which is what decides whether a
    /// user who upgrades wakes up inside a namespace or a cap they never chose.
    const PRE_PHASE_19_CONFIG: &str = r#"{
  "version": 2,
  "projects": {
    "alpha": {
      "path": "/home/testuser/projects/alpha",
      "added": "2026-01-04T09:15:00+00:00",
      "driver_opt_in": {
        "opted_in_at": "2026-07-29T11:59:00Z",
        "claude_md_digest": "fnv1a:0123456789abcdef"
      }
    }
  },
  "preferences": {
    "hooks": {
      "pre_create": null,
      "post_create": null
    },
    "gsd_integration": true,
    "driver_max_concurrent": 1
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
    fn a_pre_phase_19_config_loads_with_every_envelope_field_absent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = write_config(dir.path(), PRE_PHASE_19_CONFIG);

        let config = load_config(&path).expect("a version 2 config must still parse");

        assert_eq!(
            config.version, CONFIG_SCHEMA_VERSION,
            "adding `#[serde(default)]` fields is not a schema change; version 2 still means \
             exactly the one thing its doc says it means"
        );

        let opt_in = config.projects["alpha"]
            .driver_opt_in
            .as_ref()
            .expect("the recorded opt-in survives the load unchanged");
        assert_eq!(opt_in.opted_in_at, "2026-07-29T11:59:00Z");
        assert_eq!(
            opt_in.claude_md_digest.as_deref(),
            Some("fnv1a:0123456789abcdef")
        );

        // The load-bearing assertion: reading an old config must never
        // silently enrol a project into a namespace or a cap it did not choose.
        assert!(
            opt_in.branch_namespace.is_none(),
            "an old config must not acquire a push namespace by being read — the envelope's \
             own default applies, resolved in one place (D-30)"
        );
        assert!(
            opt_in.credential.is_none(),
            "an old config must not acquire a credential source by being read; no credential \
             is the default, and it fails closed rather than reaching for the user's own \
             (SAFE-05, D-18)"
        );
        assert!(
            opt_in.pr_cap_per_24h.is_none(),
            "an old config must not acquire a PR cap by being read"
        );
        assert!(opt_in.pr_cap_per_run.is_none());
    }

    #[test]
    fn a_credential_source_round_trips_through_self_describing_json() {
        let env: CredentialSource =
            serde_json::from_str(r#"{"source":"env","var":"GSD_MM_GIT_TOKEN"}"#)
                .expect("the env form parses");
        assert_eq!(
            env,
            CredentialSource::Env {
                var: "GSD_MM_GIT_TOKEN".to_string()
            }
        );

        let command: CredentialSource =
            serde_json::from_str(r#"{"source":"command","argv":["gh","auth","token"]}"#)
                .expect("the command form parses");
        assert_eq!(
            command,
            CredentialSource::Command {
                argv: vec!["gh".to_string(), "auth".to_string(), "token".to_string()],
            },
            "the command is argv, never a shell string — a shell in this path would make a \
             credential lookup into an arbitrary-command seam"
        );

        let rendered = serde_json::to_string(&command).expect("serialises");
        assert!(
            rendered.contains(r#""source":"command""#),
            "the tag is what makes the registry file readable without a schema, got {rendered}"
        );
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
    fn a_pre_phase_21_opt_in_loads_with_no_migration_and_reads_as_re_confirm() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = write_config(dir.path(), PRE_PHASE_21_CONFIG);

        let config = load_config(&path).expect(
            "a config written before `prompt_inputs` existed must load unchanged; \
             needing a migration here is the one-way door the record shape exists to avoid",
        );
        let opt_in = config.projects["alpha"]
            .driver_opt_in
            .as_ref()
            .expect("the opt-in record survives");

        assert_eq!(
            opt_in.claude_md_digest.as_deref(),
            Some("fnv1a64:0123456789abcdef"),
            "the legacy key is kept, not dropped — that is what makes an older \
             binary's load-and-save non-destructive"
        );
        assert!(
            opt_in.prompt_inputs.is_empty(),
            "an absent `prompt_inputs` key defaults to empty rather than failing to parse"
        );

        // The safe reading: a record that never disclosed anything cannot stand
        // as an informed approval of what reaches a prompt.
        let drift = crate::registry::check_prompt_input_drift(dir.path(), opt_in);
        assert_eq!(
            drift,
            Some(crate::registry::OptInDrift::NothingDisclosed),
            "a pre-disclosure opt-in must re-confirm, never silently pass the gate"
        );
    }

    #[test]
    fn an_unknown_key_nested_inside_the_opt_in_survives_a_load_and_save() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = write_config(dir.path(), FUTURE_CONFIG_NESTED_UNKNOWN);

        let config = load_config(&path).expect("a future config must still parse");
        save_config(&config, &path).expect("the round-tripped config saves");
        let text = std::fs::read_to_string(&path).expect("the saved config is readable");

        assert!(
            text.contains("driver_attestation_v3"),
            "an unknown key INSIDE `driver_opt_in` must survive a save by this \
             build. Before Phase 21 added the `extra` flatten to `DriverOptIn` \
             this failed, while the same assertion one level up passed — which is \
             why T-21-20's 'the existing flatten-preservation posture' was a paper \
             mitigation at this nesting level. Got:\n{text}"
        );
    }

    #[test]
    fn a_digest_from_an_unknown_hash_family_reads_as_re_confirm() {
        let dir = tempfile::tempdir().expect("temp dir");

        // Matched on "is not sha256:", NOT on "is fnv1a64:". The pre-Phase-19
        // fixture in this very file carries `fnv1a:` — a real legacy value an
        // exact-prefix match would sail straight past.
        for legacy in [
            "fnv1a64:0123456789abcdef",
            "fnv1a:0123456789abcdef",
            "blake3:00112233",
            "0123456789abcdef",
        ] {
            let opt_in = DriverOptIn {
                opted_in_at: "2026-07-29T11:59:00Z".to_string(),
                claude_md_digest: None,
                prompt_inputs: vec![PromptInput {
                    path: "CLAUDE.md".to_string(),
                    digest: Some(legacy.to_string()),
                    extra: Default::default(),
                }],
                extra: Default::default(),
                branch_namespace: None,
                credential: None,
                pr_cap_per_24h: None,
                pr_cap_per_run: None,
            };

            let drift = crate::registry::check_prompt_input_drift(dir.path(), &opt_in);
            assert!(
                matches!(
                    drift,
                    Some(crate::registry::OptInDrift::LegacyDigest { .. })
                ),
                "`{legacy}` names a hash family this binary does not compute, so it \
                 must re-confirm rather than read as a match or as an error. Got: {drift:?}"
            );
        }
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
