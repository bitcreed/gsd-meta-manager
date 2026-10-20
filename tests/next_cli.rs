//! `gsd-meta-manager next <project> --json`: binary-level proofs (Phase 29, PRMT-05;
//! D-20, D-21).
//!
//! Each test builds a real `.planning/` tree by writing the artifact files the
//! state reader looks for, then runs the shipped binary against it. The router's
//! own table is proved elsewhere (`driver_router_table`, `driver_router_conformance`);
//! what is proved here is the command wrapped around it.

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
/// phase is in the current milestone).
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
            "### Phase {number}: {slug}\n\n**Goal**: exercise next\n**Depends on**: Nothing\n\n"
        ));
    }
    write(&dir.path().join(".planning/ROADMAP.md"), &roadmap);
    write(
        &dir.path().join(".planning/STATE.md"),
        "---\nstatus: in_progress\n---\n\n# State\n",
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

fn next(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .arg("next")
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
    serde_json::from_str(stdout.trim_end()).unwrap_or_else(|e| panic!("stdout is JSON: {e}: {stdout}"))
}

#[test]
fn a_discussed_phase_is_routed_to_planning_as_an_advisory_run() {
    let dir = project(&[("1", "Alpha")]);
    discussed(dir.path(), "1", "alpha");
    let out = next(&[dir.path().to_str().unwrap(), "--target-phase", "1", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let hint = json(&out);
    assert_eq!(hint["advisory"], Value::Bool(true));
    assert_eq!(hint["source"], "phase-20-router");
    assert_eq!(hint["decision"], "run");
    assert!(
        hint["command"]
            .as_str()
            .is_some_and(|c| c.starts_with("/gsd-plan-phase 1")),
        "{hint}"
    );
    assert_eq!(hint["target_phase"], "1");
    assert!(hint["detail"].is_null(), "{hint}");
}

#[test]
fn a_phase_with_gaps_found_parks_with_the_routers_own_reason() {
    let dir = project(&[("1", "Alpha")]);
    let root = dir.path();
    // Executed, then the verifier's verdict overwritten with the gate state.
    passed(root, "1", "alpha");
    verification(root, "1", "alpha", "gaps_found");
    let out = next(&[root.to_str().unwrap(), "--target-phase", "1", "--json"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let hint = json(&out);
    assert_eq!(hint["decision"], "park");
    assert_eq!(
        hint["reason"],
        gsd_meta_manager::driver::router::RouterReason::GateVerificationGapsFound.as_str()
    );
    assert!(hint["command"].is_null(), "{hint}");
}
