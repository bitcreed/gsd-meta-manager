//! Binary-level checks for the positional launch target (quick 260930-vvk).
//!
//! A refused target must exit 1 BEFORE any terminal setup (D-03). The exit
//! code is itself evidence of that: a run that reached `tui::init()` with no
//! TTY dies differently, so `Some(1)` plus an empty alternate-screen trace on
//! stdout pins the refusal to the pre-TUI branch.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// The alternate-screen enter sequence `tui::init()` writes.
const ALT_SCREEN_ENTER: &str = "\x1b[?1049h";

fn canon(p: &Path) -> PathBuf {
    p.canonicalize().expect("canonicalize")
}

/// Run the freshly built binary against a scratch config, with XDG dirs inside
/// the tempdir so the developer's real config and log dir are never touched,
/// and stdin from /dev/null so a run that wrongly reached the TUI cannot hang.
fn run_bin(tmp: &Path, config: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .env("XDG_DATA_HOME", tmp.join("xdg-data"))
        .env("XDG_CONFIG_HOME", tmp.join("xdg-config"))
        .arg("--config")
        .arg(config)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("the binary runs")
}

fn assert_refused_before_the_tui(out: &std::process::Output, needle: &str) {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(
        out.status.code(),
        Some(1),
        "a refused target must exit exactly 1 before terminal setup; stderr: {stderr}"
    );
    assert!(stderr.contains(needle), "stderr does not name {needle:?}: {stderr}");
    assert!(
        stderr.contains("gsd-meta-manager add"),
        "stderr lacks the `add` suggestion: {stderr}"
    );
    assert!(
        !stdout.contains(ALT_SCREEN_ENTER),
        "the alternate screen was entered before the refusal: {stdout:?}"
    );
}

#[test]
fn an_existing_unregistered_path_is_refused_before_the_tui() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = canon(tmp.path());
    let unregistered = root.join("unregistered");
    std::fs::create_dir_all(&unregistered).expect("dir");
    let config = root.join("config.json");

    let out = run_bin(&root, &config, &[unregistered.to_str().unwrap()]);
    assert_refused_before_the_tui(&out, &unregistered.display().to_string());
}

#[test]
fn a_registered_alias_named_list_still_runs_the_list_subcommand() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = canon(tmp.path());
    let project = root.join("list");
    std::fs::create_dir_all(project.join(".planning")).expect("project dir");
    let config = root.join("config.json");

    let added = run_bin(&root, &config, &["add", project.to_str().unwrap()]);
    assert!(
        added.status.success(),
        "registering <tmp>/list failed: {}",
        String::from_utf8_lossy(&added.stderr)
    );

    let out = run_bin(&root, &config, &["list"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert_eq!(
        out.status.code(),
        Some(0),
        "`list` must run the subcommand even with a project aliased `list` (D-01); stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(stdout.contains("ALIAS"), "no list table header: {stdout}");
    assert!(stdout.contains("list"), "the `list` project is not listed: {stdout}");
    assert!(!stdout.contains(ALT_SCREEN_ENTER), "the TUI started: {stdout:?}");
}

#[test]
fn an_unknown_alias_is_refused_before_the_tui() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let root = canon(tmp.path());
    let config = root.join("config.json");

    let out = run_bin(&root, &config, &["no-such-alias-vvk"]);
    assert_refused_before_the_tui(&out, "no-such-alias-vvk");
}
