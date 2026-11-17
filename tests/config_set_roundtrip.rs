//! The Config tab's project save round trip against gsd-core's own writer
//! (quick 261116-kft, D-08).
//!
//! The product never writes `.planning/config.json`; a project-scope save runs
//! `gsd-tools config-set` / `config-set-model-profile`. This test plans the
//! calls with the **product's own** `plan_config_set`, runs each call's argv
//! through the real `gsd-tools` exactly as the product does (cwd = the project
//! root, every `ACK_ENV_REMOVE` variable removed), re-reads the file and asks
//! the product's own diff whether the intended config is now on disk. A second
//! test pins the Rust replica of gsd-core's argv value parser (`cli_parse`)
//! against `config-set --dry-run`.
//!
//! **Temp projects only.** Every run operates on a project under a `TempDir`,
//! never on the repository tree or on a registered project, and `GSD_HOME` is
//! pointed at a `TempDir` too.
//!
//! **The oracle is required** (the `owner_open_conformance` policy): with no
//! gsd-core install the tests fail, unless `GSD_META_MANAGER_ALLOW_MISSING_ORACLE`
//! holds a non-blank value. `scripts/pre-tag-check.sh --container` provisions it.

use std::path::{Path, PathBuf};
use std::process::Command;

use gsd_meta_manager::state_reader::config_json::{parse_gsd_config, ApiKeySetting, GsdConfig};
use gsd_meta_manager::state_reader::config_set::{
    cli_parse, json_equal, plan_config_set, RefusalReason,
};
use gsd_meta_manager::state_reader::gsd_install::{install_candidates, InstallRoots};
use gsd_meta_manager::ui::screens::owner_actions::ACK_ENV_REMOVE;
use serde_json::Value;
use tempfile::TempDir;

const ALLOW_MISSING_ORACLE: &str = "GSD_META_MANAGER_ALLOW_MISSING_ORACLE";
const FIXTURE: &str = r#"{"mode":"interactive","commit_docs":true,"phase_naming":"sequential"}"#;

/// A resolved way to run gsd-core's own `gsd-tools`.
struct Oracle {
    program: PathBuf,
    script: Option<PathBuf>,
    home: TempDir,
}

impl Oracle {
    /// The installed `gsd-tools`, or `None` when this environment has none and
    /// says so through [`ALLOW_MISSING_ORACLE`]; a missing oracle otherwise fails
    /// the test (the default is fail-loud).
    fn resolve() -> Option<Self> {
        let found = Self::find();
        if found.is_none() {
            let opt_out = std::env::var(ALLOW_MISSING_ORACLE).unwrap_or_default();
            assert!(
                !opt_out.trim().is_empty(),
                "the gsd-core oracle is not installed, so the config-set round trip cannot run; \
                 install it, or set {ALLOW_MISSING_ORACLE} to accept a run that checked nothing"
            );
            eprintln!("SKIP: no gsd-core oracle, and {ALLOW_MISSING_ORACLE} accepts that");
        }
        found
    }

    fn find() -> Option<Self> {
        let home = tempfile::tempdir().unwrap();
        let node_runs = Command::new("node").arg("--version").output().is_ok();
        for candidate in install_candidates(None, &InstallRoots::from_env()) {
            let script = candidate.gsd_core_dir.join("bin/gsd-tools.cjs");
            if script.is_file() && node_runs {
                return Some(Oracle {
                    program: PathBuf::from("node"),
                    script: Some(script),
                    home,
                });
            }
        }
        if Command::new("gsd-tools").arg("--version").output().is_ok() {
            return Some(Oracle {
                program: PathBuf::from("gsd-tools"),
                script: None,
                home,
            });
        }
        None
    }

    /// Run `args` as the product's save does: cwd = the project root, every
    /// selector variable removed. Returns success and stdout.
    fn run(&self, root: &Path, args: &[String]) -> (bool, String, String) {
        let mut command = Command::new(&self.program);
        if let Some(script) = &self.script {
            command.arg(script);
        }
        command
            .args(args)
            .current_dir(root)
            .env("GSD_HOME", self.home.path());
        for name in ACK_ENV_REMOVE {
            command.env_remove(name);
        }
        let output = command.output().expect("the oracle runs");
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }
}

fn temp_project() -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    std::fs::create_dir_all(root.join(".planning")).unwrap();
    std::fs::write(root.join(".planning/config.json"), FIXTURE).unwrap();
    (dir, root)
}

fn on_disk(root: &Path) -> GsdConfig {
    parse_gsd_config(&std::fs::read_to_string(root.join(".planning/config.json")).unwrap())
        .expect("the saved file still parses")
}

/// Plan `before -> after`, run every call through the oracle, and assert that
/// the product's own diff sees `after` on disk.
fn apply(oracle: &Oracle, edit: impl FnOnce(&mut GsdConfig)) {
    let (_dir, root) = temp_project();
    let before = on_disk(&root);
    let mut after = before.clone();
    edit(&mut after);
    let plan = plan_config_set(&before, &after).expect("the edit can be sent");
    assert!(!plan.is_empty(), "the edit changes something");
    for call in &plan {
        let (ok, _stdout, stderr) = oracle.run(&root, &call.args());
        assert!(ok, "{call:?} failed: {stderr}");
    }
    let saved = on_disk(&root);
    let left = plan_config_set(&after, &saved).expect("the diff plans");
    assert!(
        left.is_empty(),
        "disk differs from the intended edit: {left:?}"
    );
}

#[test]
fn config_set_saves_every_edit_kind_exactly() {
    let Some(oracle) = Oracle::resolve() else {
        return;
    };
    apply(&oracle, |c| c.commit_docs = Some(false));
    apply(&oracle, |c| c.response_language = Some("Deutsch".into()));
    apply(&oracle, |c| c.commit_docs = None);
    apply(&oracle, |c| {
        c.workflow.get_or_insert_with(Default::default).plan_check = Some(true)
    });
    apply(&oracle, |c| c.context_window = Some(400_000));
    apply(&oracle, |c| c.model_profile = "quality".into());
    apply(&oracle, |c| {
        c.brave_search = Some(ApiKeySetting::Key("sk-test-brave".into()))
    });
    apply(&oracle, |c| {
        c.tavily_search = Some(ApiKeySetting::Key("sk-test-tavily".into()))
    });
}

#[test]
fn a_coercing_value_is_refused_and_nothing_runs() {
    let (_dir, root) = temp_project();
    let before = on_disk(&root);
    let mut after = before.clone();
    after.phase_naming = Some("12".into());
    let refusal = plan_config_set(&before, &after).unwrap_err();
    assert_eq!(refusal.key, "phase_naming");
    assert_eq!(
        refusal.reason,
        RefusalReason::Coerced {
            stored_as: "number"
        }
    );
    assert_eq!(
        std::fs::read_to_string(root.join(".planning/config.json")).unwrap(),
        FIXTURE
    );
}

/// `response_language` has no per-key validator at gsd-core 1.21.0 (measured),
/// so its `--dry-run` shows the parser alone. If a later install adds one,
/// switch to another free-form key and say why here.
#[test]
fn cli_parse_agrees_with_config_set_dry_run() {
    let Some(oracle) = Oracle::resolve() else {
        return;
    };
    let (_dir, root) = temp_project();
    let rows: &[(&str, &str)] = &[
        ("response_language", "true"),
        ("response_language", "false"),
        ("response_language", "null"),
        ("response_language", "TRUE"),
        ("response_language", "0"),
        ("response_language", "12"),
        ("response_language", "007"),
        ("response_language", "1.5"),
        ("response_language", ".5"),
        ("response_language", "1."),
        ("response_language", "+1"),
        ("response_language", "1e3"),
        ("response_language", "1E+3"),
        ("response_language", "1e400"),
        ("response_language", "Infinity"),
        ("response_language", "NaN"),
        ("response_language", "0x10"),
        ("response_language", "1_000"),
        ("response_language", " 12"),
        ("response_language", "12 "),
        ("response_language", "\u{661}\u{662}"),
        ("response_language", "[1,2]"),
        ("response_language", r#"{"a":1}"#),
        ("response_language", "[bad"),
        ("response_language", "{"),
        ("response_language", ""),
        ("response_language", "plain text"),
        ("project_code", "007"),
        ("project_code", "true"),
        ("project_code", "[1]"),
    ];
    let mut non_string = 0;
    for (key, arg) in rows {
        let args: Vec<String> = ["config-set", key, arg, "--dry-run"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let (ok, stdout, stderr) = oracle.run(&root, &args);
        assert!(ok, "{key} {arg:?}: {stderr}");
        let v: Value = serde_json::from_str(&stdout)
            .unwrap_or_else(|_| panic!("{key} {arg:?}: not JSON: {stdout}"));
        let oracle_value = if v.get("would_unset") == Some(&Value::Bool(true)) {
            Value::Null
        } else {
            v.get("value").cloned().unwrap_or(Value::Null)
        };
        let ours = cli_parse(key, arg);
        assert!(
            json_equal(&ours, &oracle_value),
            "{key} {arg:?}: replica {ours} vs gsd-tools {oracle_value}"
        );
        if !ours.is_string() {
            non_string += 1;
        }
    }
    assert!(non_string > 0, "the table must exercise coercion");
    assert_eq!(
        std::fs::read_to_string(root.join(".planning/config.json")).unwrap(),
        FIXTURE,
        "--dry-run writes nothing"
    );
}
