//! The acknowledge round trip against gsd-core's own writer (37 D-16, OWN-22).
//!
//! The product never writes into a project; `A` runs gsd-core's acknowledge
//! command. This test runs the **product's own** `ack_target_for`, `ack_args` and
//! `ACK_ENV_REMOVE` through the real `gsd-tools` on a temp copy of a fixture and
//! reads the result back with the product's own digest reader: the group must be
//! gone from the lanes and `hidden.acknowledged_files` must grow by one.
//!
//! **Temp copies only.** The writer edits a file's frontmatter; every run here
//! operates on a copy under a `TempDir`, never on the repository tree or on a
//! registered project.
//!
//! **The oracle is required** (the `owner_open_conformance` policy): with no
//! gsd-core install the tests fail, unless `GSD_META_MANAGER_ALLOW_MISSING_ORACLE`
//! holds a non-blank value. `scripts/pre-tag-check.sh --container` provisions it.

use std::path::{Path, PathBuf};
use std::process::Command;

use chrono::{DateTime, Utc};
use gsd_meta_manager::state_reader::gsd_install::{install_candidates, InstallRoots};
use gsd_meta_manager::state_reader::owner_items::{
    read_owner_digest, Kind, Lane, OwnerDigest, OwnerGroup,
};
use gsd_meta_manager::state_reader::{parse_project_state, ProjectState};
use gsd_meta_manager::ui::screens::owner_actions::{ack_args, ack_target_for, ACK_ENV_REMOVE};
use tempfile::TempDir;

const ALLOW_MISSING_ORACLE: &str = "GSD_META_MANAGER_ALLOW_MISSING_ORACLE";

fn now() -> DateTime<Utc> {
    "2026-11-14T00:00:00Z".parse().unwrap()
}

/// A resolved way to run gsd-core's own `gsd-tools`.
struct Oracle {
    program: PathBuf,
    script: Option<PathBuf>,
}

impl Oracle {
    /// The installed `gsd-tools`, or `None` when this environment has none and says
    /// so through [`ALLOW_MISSING_ORACLE`]; a missing oracle otherwise fails the
    /// test (the default is fail-loud).
    fn resolve() -> Option<Self> {
        let found = Self::find();
        if found.is_none() {
            let opt_out = std::env::var(ALLOW_MISSING_ORACLE).unwrap_or_default();
            assert!(
                !opt_out.trim().is_empty(),
                "the gsd-core oracle is not installed, so the acknowledge round trip cannot run; \
                 install it, or set {ALLOW_MISSING_ORACLE} to accept a run that checked nothing"
            );
            eprintln!("SKIP: no gsd-core oracle, and {ALLOW_MISSING_ORACLE} accepts that");
        }
        found
    }

    fn find() -> Option<Self> {
        let node_runs = Command::new("node").arg("--version").output().is_ok();
        for candidate in install_candidates(None, &InstallRoots::from_env()) {
            let script = candidate.gsd_core_dir.join("bin/gsd-tools.cjs");
            if script.is_file() && node_runs {
                return Some(Oracle {
                    program: PathBuf::from("node"),
                    script: Some(script),
                });
            }
        }
        if Command::new("gsd-tools").arg("--version").output().is_ok() {
            return Some(Oracle {
                program: PathBuf::from("gsd-tools"),
                script: None,
            });
        }
        None
    }

    /// Run the product's acknowledge argv exactly as `run_ack` would: cwd = the
    /// project root, every selector variable removed.
    fn acknowledge(&self, root: &Path, args: &[String]) -> Result<String, String> {
        let mut command = Command::new(&self.program);
        if let Some(script) = &self.script {
            command.arg(script);
        }
        command.args(args).current_dir(root);
        for name in ACK_ENV_REMOVE {
            command.env_remove(name);
        }
        let output = command
            .output()
            .map_err(|err| format!("the oracle did not run: {}", err.kind()))?;
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        if output.status.success() {
            Ok(text)
        } else {
            Err(text)
        }
    }
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// A fixture copied into a TempDir, with `milestone: v1.0` in the root STATE.md
/// and in every workstream's, so `ack_target_for` has a version to name.
fn temp_project(name: &str) -> (TempDir, PathBuf) {
    let src = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/owner")
        .join(name)
        .join(".planning");
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().to_path_buf();
    copy_dir(&src, &root.join(".planning"));
    let state = "---\nstatus: in_progress\nmilestone: v1.0\n---\n\n# State\n";
    std::fs::write(root.join(".planning/STATE.md"), state).unwrap();
    if let Ok(streams) = std::fs::read_dir(root.join(".planning/workstreams")) {
        for stream in streams.flatten() {
            std::fs::write(stream.path().join("STATE.md"), state).unwrap();
        }
    }
    (dir, root)
}

fn read(root: &Path) -> (ProjectState, OwnerDigest) {
    let planning = root.join(".planning");
    let state = parse_project_state(&planning);
    let digest = read_owner_digest(&planning, &state, now(), 30);
    (state, digest)
}

/// The first unacknowledged UAT group of `workstream` in a lane the owner works.
fn uat_group<'a>(digest: &'a OwnerDigest, workstream: Option<&str>) -> &'a OwnerGroup {
    digest
        .groups
        .iter()
        .find(|group| {
            group.kind == Kind::Uat
                && group.lane != Lane::Blocked
                && !group.acknowledged
                && group.workstream.as_deref() == workstream
                && group.scope == gsd_meta_manager::state_reader::owner_items::Scope::Current
        })
        .expect("the fixture has such a UAT group")
}

fn lists(digest: &OwnerDigest, file: &str, workstream: Option<&str>) -> bool {
    digest.groups.iter().any(|group| {
        group.file == file && group.workstream.as_deref() == workstream && !group.acknowledged
    })
}

#[test]
fn acknowledging_a_uat_group_hides_it_on_the_next_read() {
    let Some(oracle) = Oracle::resolve() else {
        return;
    };
    let (_guard, root) = temp_project("uat-pending");
    let (state, before) = read(&root);
    let group = uat_group(&before, None);
    let file = group.file.clone();
    let target = ack_target_for("demo", &state, &before, group, false).expect("acknowledgeable");
    assert_eq!(target.category, "uat_gaps");
    let args = ack_args(&target);
    oracle
        .acknowledge(&root, &args)
        .unwrap_or_else(|out| panic!("the writer refused {args:?}: {out}"));

    let (_, after) = read(&root);
    assert!(
        !lists(&after, &file, None),
        "the acknowledged file must leave the lanes"
    );
    assert_eq!(
        after.hidden.acknowledged_files,
        before.hidden.acknowledged_files + 1,
        "the file is counted as acknowledged"
    );
    let text = std::fs::read_to_string(root.join(&file)).unwrap();
    assert!(
        text.contains("audit_acknowledged"),
        "gsd-core wrote the marker into the file's frontmatter"
    );
}

#[test]
fn a_workstream_group_acknowledges_with_ws() {
    let Some(oracle) = Oracle::resolve() else {
        return;
    };
    let (_guard, root) = temp_project("workstreams");
    let (state, before) = read(&root);
    let group = uat_group(&before, Some("alpha"));
    let file = group.file.clone();
    let target = ack_target_for("demo", &state, &before, group, false).expect("acknowledgeable");
    let args = ack_args(&target);
    assert!(
        args.windows(2).any(|pair| pair == ["--ws", "alpha"]),
        "a workstream group names its workstream: {args:?}"
    );
    oracle
        .acknowledge(&root, &args)
        .unwrap_or_else(|out| panic!("the writer refused {args:?}: {out}"));

    let (_, after) = read(&root);
    assert!(!lists(&after, &file, Some("alpha")));
    assert_eq!(
        after.hidden.acknowledged_files,
        before.hidden.acknowledged_files + 1
    );
    // The root group of the same project is untouched.
    let root_group = uat_group(&before, None);
    assert!(
        lists(&after, &root_group.file, None),
        "the root file stays listed"
    );
}

#[test]
fn a_root_group_is_refused_while_an_active_workstream_is_set() {
    // No oracle needed: the refusal happens before anything would run, which is the
    // point (there is no CLI spelling for "the root" while a pointer is set).
    let (_guard, root) = temp_project("workstreams");
    std::fs::write(root.join(".planning/active-workstream"), "alpha\n").unwrap();
    let (state, digest) = read(&root);
    let group = uat_group(&digest, None);
    let refused = ack_target_for("demo", &state, &digest, group, true).unwrap_err();
    assert!(
        refused.contains("has an active workstream set; run it in the project"),
        "{refused}"
    );
    // A workstream group names its workstream, so the pointer cannot redirect it.
    let stream = uat_group(&digest, Some("alpha"));
    assert!(ack_target_for("demo", &state, &digest, stream, true).is_ok());
}
