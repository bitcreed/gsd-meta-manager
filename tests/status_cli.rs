//! `gsd-meta-manager status --json`: binary-level proofs (quick 261110-qap).
//!
//! Each test builds a real `.planning/` tree by writing the artifact files the
//! state reader looks for, then runs the shipped binary against it.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;
use tempfile::TempDir;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("fixture directories are creatable");
    }
    std::fs::write(path, contents).expect("fixture files are writable");
}

/// A project whose roadmap lists the given phase numbers (no milestones, so every
/// phase is in the current milestone). STATE.md names milestone `v1.1 Second`.
fn project(phases: &[(&str, &str)]) -> TempDir {
    let dir = tempfile::tempdir().expect("a temp dir");
    let mut roadmap = String::from("# Roadmap\n\n## Phases\n\n");
    for (number, slug) in phases {
        roadmap.push_str(&format!(
            "- [ ] **Phase {number}: {slug}** - fixture phase\n"
        ));
    }
    roadmap.push('\n');
    for (number, slug) in phases {
        roadmap.push_str(&format!(
            "### Phase {number}: {slug}\n\n**Goal**: exercise status\n**Depends on**: Nothing\n\n"
        ));
    }
    write(&dir.path().join(".planning/ROADMAP.md"), &roadmap);
    write(
        &dir.path().join(".planning/STATE.md"),
        "---\nmilestone: v1.1\nmilestone_name: Second\nstatus: in_progress\n---\n\n# State\n",
    );
    dir
}

fn phase_dir(root: &Path, number: &str, slug: &str) -> std::path::PathBuf {
    root.join(format!(".planning/phases/{number}-{slug}"))
}

fn discussed(root: &Path, number: &str, slug: &str) {
    write(
        &phase_dir(root, number, slug).join(format!("{number}-CONTEXT.md")),
        "# Context\n",
    );
}

fn verification(root: &Path, number: &str, slug: &str, status: &str) {
    write(
        &phase_dir(root, number, slug).join(format!("{number}-VERIFICATION.md")),
        &format!("---\nphase: {number}\nstatus: {status}\n---\n\n# Verification\n"),
    );
}

fn passed(root: &Path, number: &str, slug: &str) {
    let dir = phase_dir(root, number, slug);
    write(
        &dir.join(format!("{number}-01-PLAN.md")),
        &format!("---\nphase: {number}\n---\n\n# Plan\n"),
    );
    write(
        &dir.join(format!("{number}-01-SUMMARY.md")),
        &format!("---\nphase: {number}\nstatus: complete\n---\n\n# Summary\n"),
    );
    verification(root, number, slug, "passed");
}

fn status(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .arg("status")
        .args(args)
        .output()
        .expect("the binary runs")
}

fn json(output: &Output) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim_end().lines().count(),
        1,
        "exactly one line on stdout: {stdout:?}"
    );
    serde_json::from_str(stdout.trim_end())
        .unwrap_or_else(|e| panic!("stdout is JSON: {e}: {stdout}"))
}

/// A two-phase fixture: phase 1 passed, phase 2 discussed.
fn two_phase_project() -> TempDir {
    let dir = project(&[("1", "Alpha"), ("2", "Beta")]);
    passed(dir.path(), "1", "alpha");
    discussed(dir.path(), "2", "beta");
    dir
}

#[test]
fn a_path_target_gives_one_schema_v1_line_with_milestone_phase_and_progress() {
    let dir = two_phase_project();
    let target = dir.path().to_str().unwrap();
    let out = status(&[target, "--json"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let doc = json(&out);
    assert_eq!(doc["schema"], "gsd-meta-manager/status");
    assert_eq!(doc["schema_version"], 1);
    assert!(doc["sessions_supported"].is_boolean(), "{doc}");
    let projects = doc["projects"].as_array().expect("projects is an array");
    assert_eq!(projects.len(), 1, "{doc}");
    let entry = &projects[0];
    assert_eq!(entry["target"], target);
    assert!(entry["alias"].is_null(), "{entry}");
    let root = std::fs::canonicalize(dir.path()).unwrap();
    assert_eq!(entry["path"], root.display().to_string().as_str());
    assert!(entry["error"].is_null(), "{entry}");
    assert_eq!(entry["milestone"]["version"], "v1.1");
    assert_eq!(entry["milestone"]["name"], "Second");
    assert_eq!(entry["milestone"]["status"], "in_progress");
    assert_eq!(entry["phase"]["id"], "2");
    assert_eq!(entry["phase"]["status"], "discussed");
    let state = gsd_meta_manager::state_reader::parse_project_state(&dir.path().join(".planning"));
    let expected_name = state.roadmap_phase("2").expect("phase 2 is in the roadmap");
    assert_eq!(entry["phase"]["name"], expected_name.name.as_str());
    assert_eq!(entry["progress"]["phases_done"], 1);
    assert_eq!(entry["progress"]["phases_total"], 2);
}

#[test]
fn the_next_object_equals_what_next_json_prints() {
    let dir = two_phase_project();
    let target = dir.path().to_str().unwrap();
    let doc = json(&status(&[target, "--json"]));
    let next = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .args(["next", target, "--json"])
        .output()
        .unwrap();
    assert_eq!(next.status.code(), Some(0), "{next:?}");
    assert_eq!(doc["projects"][0]["next"], json(&next));
}

#[test]
fn without_json_clap_exits_two() {
    let dir = two_phase_project();
    let out = status(&[dir.path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    assert!(out.stdout.is_empty());
}
