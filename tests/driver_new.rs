#![cfg(target_os = "linux")]
// ============================================================================
// `driver new` and `driver check` through the real binary (Phase 29, plan 29-04).
//
// No sandbox tool is involved: `new` and `check` only read the registry, create
// (or read) files under the project's run directory and print. Every test runs
// the binary under a private HOME / XDG tree with `--config` pointing at a
// registry the test wrote, in a project that is a real git repository so that
// "the run is never staged" is answered by git itself (staging, not
// `check-ignore`: see tests/journal_gitignore.rs for why).
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use chrono::Utc;
use gsd_meta_manager::driver::direction::split_frontmatter;
use gsd_meta_manager::driver_templates::{DIRECTION_TEMPLATE, RUN_ID_TOKEN};
use gsd_meta_manager::state_reader::driver_frontmatter::{
    read_driver_run, DerivedStatus, RunStatus,
};
use tempfile::TempDir;

const ALIAS: &str = "p29";

/// A git project with one committed `.planning/STATE.md` and `.gitignore`, a
/// registry naming it, and a private home.
struct World {
    tmp: TempDir,
}

fn git(repo: &Path, args: &[&str]) -> Output {
    let nowhere = repo.join("no-such-gitconfig");
    Command::new("git")
        .args(args)
        .current_dir(repo)
        .env("GIT_CONFIG_GLOBAL", &nowhere)
        .env("GIT_CONFIG_SYSTEM", &nowhere)
        .output()
        .expect("git runs")
}

fn git_ok(repo: &Path, args: &[&str]) -> String {
    let out = git(repo, args);
    assert!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

impl World {
    fn new() -> Self {
        let world = World {
            tmp: TempDir::new().expect("temp dir"),
        };
        let project = world.project();
        fs::create_dir_all(project.join(".planning")).expect("project dirs");
        for dir in ["home", "config", "data", "state", "run"] {
            fs::create_dir_all(world.scratch().join(dir)).expect("scratch dirs");
        }
        fs::write(project.join(".planning/STATE.md"), "# State\n").expect("STATE.md");
        fs::write(project.join(".gitignore"), "target/\n").expect(".gitignore");
        git_ok(&project, &["init", "--quiet"]);
        git_ok(&project, &["config", "user.email", "t@example.invalid"]);
        git_ok(&project, &["config", "user.name", "T"]);
        git_ok(&project, &["add", "-A"]);
        git_ok(&project, &["commit", "--quiet", "-m", "init"]);
        fs::write(
            world.registry(),
            serde_json::json!({
                "version": 2,
                "projects": {ALIAS: {"path": project, "added": "2026-01-01T00:00:00Z"}}
            })
            .to_string(),
        )
        .expect("registry");
        world
    }

    fn project(&self) -> PathBuf {
        let path = self.tmp.path().join("proj");
        fs::create_dir_all(&path).expect("project dir");
        path
    }

    fn scratch(&self) -> PathBuf {
        self.tmp.path().join("scratch")
    }

    fn registry(&self) -> PathBuf {
        self.scratch().join("registry.json")
    }

    fn drivers(&self) -> PathBuf {
        self.project().join(".planning/meta-manager/drivers")
    }

    /// `gsd-meta-manager --config <registry> driver <args>` under a hermetic env.
    fn driver(&self, args: &[&str]) -> Output {
        let scratch = self.scratch();
        Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
            .arg("--config")
            .arg(self.registry())
            .arg("driver")
            .args(args)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("HOME", scratch.join("home"))
            .env("XDG_RUNTIME_DIR", scratch.join("run"))
            .env("XDG_CONFIG_HOME", scratch.join("config"))
            .env("XDG_DATA_HOME", scratch.join("data"))
            .env("XDG_STATE_HOME", scratch.join("state"))
            .stdin(Stdio::null())
            .output()
            .expect("the binary runs")
    }
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// The UTC dates around a call, so a run across midnight cannot flake.
fn utc_date() -> String {
    Utc::now().format("%Y%m%d").to_string()
}

/// The run id from `driver new`'s first stdout line.
fn created_id(out: &Output) -> String {
    let text = stdout(out);
    let first = text.lines().next().unwrap_or_default();
    first
        .strip_prefix("driver new: created run ")
        .unwrap_or_else(|| panic!("unexpected first line {first:?}\nstderr: {}", stderr(out)))
        .to_owned()
}

fn dir_listing(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("read the run dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn driver_new_creates_a_self_ignoring_run_dir_and_prints_the_next_commands() {
    let world = World::new();
    let project_gitignore = fs::read(world.project().join(".gitignore")).expect("read");

    let before = utc_date();
    let out = world.driver(&["new", ALIAS, "smoke"]);
    let after = utc_date();
    assert!(out.status.success(), "stderr: {}", stderr(&out));

    let text = stdout(&out);
    let run_id = created_id(&out);
    assert!(
        [&before, &after]
            .iter()
            .any(|d| run_id == format!("{d}-smoke")),
        "run id {run_id:?} is <today UTC>-smoke"
    );

    let run_dir = world.drivers().join(&run_id);
    assert_eq!(
        dir_listing(&run_dir),
        [".gitignore", "DIRECTION.md", "LOG.md"],
        "exactly three files, and no state.json"
    );
    assert_eq!(
        fs::read_to_string(run_dir.join(".gitignore")).expect("read"),
        "*\n"
    );

    // The tool block is the Phase 27 emitter's never-launched view.
    let view = read_driver_run(&run_dir).expect("the fresh run parses");
    assert_eq!(view.frontmatter.status, RunStatus::Starting);
    assert_eq!(view.frontmatter.launches, 0);
    assert_eq!(
        view.derived,
        DerivedStatus::NotLaunched,
        "a fresh run is not a crash (27 D-22)"
    );

    // Below the tool block, the human's template with the id filled.
    let direction = fs::read_to_string(run_dir.join("DIRECTION.md")).expect("read");
    let (block, body) = split_frontmatter(&direction);
    assert!(block.is_some(), "the tool block is marked");
    assert_eq!(body, DIRECTION_TEMPLATE.replace(RUN_ID_TOKEN, &run_id));

    // Non-intrusive: the project's own .gitignore is untouched and git never
    // stages anything under the drivers dir.
    assert_eq!(
        fs::read(world.project().join(".gitignore")).expect("read"),
        project_gitignore
    );
    git_ok(&world.project(), &["add", "-A"]);
    let tracked = git_ok(&world.project(), &["ls-files"]);
    assert!(
        !tracked.contains("meta-manager"),
        "the run must not be staged:\n{tracked}"
    );

    // The next commands, exactly.
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4, "{text}");
    assert_eq!(
        lines[1],
        format!("  direction: {}", run_dir.join("DIRECTION.md").display())
    );
    assert_eq!(
        lines[2],
        format!("  check:  gsd-meta-manager driver check {ALIAS} {run_id}")
    );
    assert_eq!(
        lines[3],
        format!("  launch: gsd-meta-manager driver launch {ALIAS} {run_id}")
    );
    assert!(!text.contains("--prompt-file"), "{text}");
}
