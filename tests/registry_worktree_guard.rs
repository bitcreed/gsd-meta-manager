// ============================================================================
// Quick 260925-x0v: a git LINKED worktree is never registered as a project.
// Quick 260926-0u3: auto-discovery registers the MAIN of a worktree session
// instead; the worktree itself is still never registered.
//
// Integration test rather than in-source because every proof here needs a real
// `git worktree add` — the shape a GSD executor worktree actually has — and
// building that means spawning git. `src/registry.rs` is deliberately NOT on
// the spawn allowlist in `tests/spawn_seam_guard.rs`, so even its
// `#[cfg(test)]` code may not spawn; `tests/` is not walked by that guard.
//
// It reuses `common::git` but never `common::fixture()`, which installs
// envelope hooks this suite has no business with.
// ============================================================================

mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

use common::git;
use gsd_meta_manager::config::{load_config, Config};
use gsd_meta_manager::registry::{
    add_project, add_project_unchecked, auto_register_from_sessions, linked_worktree_main, Alias,
    LinkedWorktreeRefusal,
};
use gsd_meta_manager::session_detector::{ClaudeSession, SessionKind};
use tempfile::TempDir;

/// A repository `main` with a committed `.planning/STATE.md` and a linked
/// worktree at `.claude/worktrees/agent-x` on branch `worktree-agent-x`.
///
/// `None` when the sandbox forbids `git init`. Every step after a successful
/// init asserts, so the fixture cannot silently degrade into vacuous passes.
fn worktree_fixture() -> Option<(TempDir, PathBuf, PathBuf)> {
    let tmp = TempDir::new().ok()?;
    let root = std::fs::canonicalize(tmp.path()).ok()?;
    let main = root.join("main");
    std::fs::create_dir_all(&main).ok()?;
    if !git(&main, &["init", "--quiet"]) {
        return None;
    }
    assert!(git(&main, &["config", "user.email", "test@example.com"]));
    assert!(git(&main, &["config", "user.name", "Test User"]));
    assert!(git(&main, &["config", "commit.gpgsign", "false"]));
    std::fs::create_dir_all(main.join(".planning")).expect("fixture mkdir");
    std::fs::write(main.join(".planning/STATE.md"), "# State\n").expect("fixture write");
    assert!(git(&main, &["add", ".planning/STATE.md"]), "git add");
    assert!(
        git(&main, &["commit", "-m", "initial", "--quiet"]),
        "git commit"
    );
    assert!(
        git(
            &main,
            &[
                "worktree",
                "add",
                "--quiet",
                "-b",
                "worktree-agent-x",
                ".claude/worktrees/agent-x",
            ],
        ),
        "git worktree add"
    );
    let worktree = main.join(".claude/worktrees/agent-x");
    assert!(
        worktree.join(".planning").is_dir(),
        "the linked worktree carries a checked-out .planning/"
    );
    Some((tmp, main, worktree))
}

fn canon(p: &Path) -> PathBuf {
    std::fs::canonicalize(p).expect("canonicalize")
}

fn session_at(working_dir: PathBuf) -> ClaudeSession {
    ClaudeSession {
        pid: 1,
        kind: SessionKind::Claude,
        session_id: None,
        working_dir,
        start_time: None,
        tty: None,
    }
}

#[test]
fn a_linked_worktree_names_its_main_and_the_main_names_nothing() {
    let Some((_tmp, main, worktree)) = worktree_fixture() else {
        return;
    };
    assert_eq!(
        linked_worktree_main(&worktree).map(|p| canon(&p)),
        Some(canon(&main))
    );
    assert_eq!(linked_worktree_main(&main), None);
    // A subdirectory of the worktree is still inside it.
    assert_eq!(
        linked_worktree_main(&worktree.join(".planning")).map(|p| canon(&p)),
        Some(canon(&main))
    );
}

/// No registry entry's canonical path equals any of `worktrees` (UD-6).
fn assert_no_worktree_registered(config: &Config, worktrees: &[&Path]) {
    for (alias, entry) in &config.projects {
        let registered = canon(&entry.path);
        for wt in worktrees {
            assert_ne!(
                registered,
                canon(wt),
                "worktree registered under alias {alias}"
            );
        }
    }
}

#[test]
fn auto_discovery_registers_the_main_when_a_session_sits_in_its_linked_worktree() {
    let Some((_tmp, main, worktree)) = worktree_fixture() else {
        return;
    };
    let mut config = Config::new();
    let added = auto_register_from_sessions(&mut config, &[session_at(worktree.clone())]);
    assert_eq!(added.len(), 1, "exactly the main registers: {added:?}");
    assert_eq!(added[0].0, "main");
    assert_eq!(canon(&added[0].1), canon(&main));
    assert_eq!(config.projects.len(), 1);
    assert_eq!(canon(&config.projects["main"].path), canon(&main));
    assert_no_worktree_registered(&config, &[&worktree]);

    // A later poll with sessions at the worktree AND at the main is a no-op.
    let added = auto_register_from_sessions(
        &mut config,
        &[session_at(worktree.clone()), session_at(main.clone())],
    );
    assert!(added.is_empty(), "second poll registered: {added:?}");
    assert_eq!(config.projects.len(), 1);
    assert_no_worktree_registered(&config, &[&worktree]);
}

#[test]
fn both_registration_primitives_refuse_a_linked_worktree_naming_the_main() {
    let Some((_tmp, main, worktree)) = worktree_fixture() else {
        return;
    };
    let alias = Alias::new("agent-x").expect("plain alias");
    let main_display = canon(&main).display().to_string();

    for (name, result) in [
        ("add_project", {
            let mut cfg = Config::new();
            let r = add_project(&mut cfg, &alias, &worktree);
            assert!(cfg.projects.is_empty(), "add_project mutated the config");
            r
        }),
        ("add_project_unchecked", {
            let mut cfg = Config::new();
            let r = add_project_unchecked(&mut cfg, &alias, &worktree);
            assert!(
                cfg.projects.is_empty(),
                "add_project_unchecked mutated the config"
            );
            r
        }),
    ] {
        let err = result.expect_err(name);
        let refusal = err
            .downcast_ref::<LinkedWorktreeRefusal>()
            .unwrap_or_else(|| panic!("{name}: not a LinkedWorktreeRefusal: {err}"));
        assert_eq!(canon(&refusal.main_worktree), canon(&main), "{name}");
        assert!(
            err.to_string().contains(&main_display),
            "{name}: message lacks the main path: {err}"
        );
    }

    // And the main worktree registers.
    let mut cfg = Config::new();
    add_project(&mut cfg, &Alias::new("main").unwrap(), &main).expect("main registers");
    assert_eq!(cfg.projects.len(), 1);
}

/// Run the freshly built binary against a scratch config, with XDG dirs inside
/// the tempdir so the developer's real config and log dir are never touched.
fn run_bin(tmp: &Path, config: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .env("XDG_DATA_HOME", tmp.join("xdg-data"))
        .env("XDG_CONFIG_HOME", tmp.join("xdg-config"))
        .arg("--config")
        .arg(config)
        .args(args)
        .output()
        .expect("the binary runs")
}

#[test]
fn cli_add_refuses_a_linked_worktree_and_names_the_main() {
    let Some((tmp, main, worktree)) = worktree_fixture() else {
        return;
    };
    let root = canon(tmp.path());
    let cfg_dir = root.join("cfg");
    std::fs::create_dir_all(&cfg_dir).expect("cfg dir");
    let config = cfg_dir.join("config.json");

    let out = run_bin(&root, &config, &["add", worktree.to_str().unwrap()]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(!out.status.success(), "add <worktree> succeeded: {stderr}");
    assert!(
        stderr.contains(&canon(&main).display().to_string()),
        "stderr lacks the main path: {stderr}"
    );
    if config.exists() {
        let cfg = load_config(&config).expect("config readable");
        assert!(cfg.projects.is_empty(), "worktree was persisted");
    }

    let out = run_bin(&root, &config, &["add", main.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "add <main> failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let cfg = load_config(&config).expect("config readable");
    assert_eq!(cfg.projects.len(), 1);
    let only = cfg.projects.values().next().unwrap();
    assert_eq!(canon(&only.path), canon(&main));
}
