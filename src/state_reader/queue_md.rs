use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

#[derive(Debug, Clone, PartialEq)]
pub struct QueuedAction {
    pub command: String,
}

/// A resolved way to invoke GSD's `gsd-tools` launcher.
///
/// Mirrors GSD's own launcher resolution: the `.cjs` shim is run through
/// `node`, while a `gsd-tools` binary found on `PATH` is invoked directly.
#[derive(Debug, Clone, PartialEq)]
struct GsdToolsCmd {
    /// Program to spawn (`node` for `.cjs` shims, else `gsd-tools`).
    program: String,
    /// Leading args (the `.cjs` path when running through `node`, else empty).
    prefix_args: Vec<String>,
}

impl GsdToolsCmd {
    /// Build a `std::process::Command` for `<launcher> <args...>`.
    fn command(&self, extra: &[&str]) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.args(&self.prefix_args);
        cmd.args(extra);
        cmd
    }
}

/// Shape of GSD `smart-entry --json` output.
///
/// See gsd-core `tests/smart-entry.unit.test.cjs`:
/// `{ situation, actions: [ { command: "/gsd:...", recommended: bool }, ... ] }`
/// with exactly one `recommended: true`.
#[derive(Debug, Deserialize)]
struct SmartEntryOutput {
    #[serde(default)]
    actions: Vec<SmartEntryAction>,
}

#[derive(Debug, Deserialize)]
struct SmartEntryAction {
    command: String,
    #[serde(default)]
    recommended: bool,
}

/// Parse `gsd-tools smart-entry --json` output into an ordered command list.
///
/// Maps `actions[].command` to a `Vec<String>` with the `recommended: true`
/// command placed FIRST and the remaining commands following in their original
/// order. Returns `None` for malformed/empty JSON or when `actions` is
/// missing/empty. Pure — never spawns a process, never panics.
fn parse_smart_entry_json(raw: &str) -> Option<Vec<String>> {
    let parsed: SmartEntryOutput = serde_json::from_str(raw).ok()?;
    if parsed.actions.is_empty() {
        return None;
    }
    let mut recommended: Vec<String> = Vec::new();
    let mut rest: Vec<String> = Vec::new();
    for action in parsed.actions {
        if action.recommended {
            recommended.push(action.command);
        } else {
            rest.push(action.command);
        }
    }
    recommended.extend(rest);
    Some(recommended)
}

/// Relative path of GSD's launcher shim inside an install root.
const GSD_TOOLS_SHIM: &str = "gsd-core/bin/gsd-tools.cjs";

/// Ordered `gsd-tools.cjs` candidates for `project_root`. Pure: the home
/// directory and `CODEX_HOME` are passed in, never read from the process.
///
/// Order (mirrors gsd-core 1.15.0 `gsd-core/references/gsd-run-resolver.md`,
/// as reworked by #4834):
///   1. `<root>/gsd-core/bin/gsd-tools.cjs`
///   2. `<root>/.claude/gsd-core/bin/gsd-tools.cjs`
///   3. `<root>/.codex/gsd-core/bin/gsd-tools.cjs`
///   4. `~/.claude/gsd-core/bin/gsd-tools.cjs`
///   5. `${CODEX_HOME:-~/.codex}/gsd-core/bin/gsd-tools.cjs`
///
/// Like the shell `:-` expansion, an EMPTY `codex_home` falls back to
/// `~/.codex`. With neither a home nor a non-empty `codex_home`, only the
/// three project-local candidates are returned.
fn gsd_tools_candidates(
    project_root: &Path,
    home: Option<&Path>,
    codex_home: Option<&OsStr>,
) -> Vec<PathBuf> {
    let mut candidates: Vec<PathBuf> = vec![
        project_root.join(GSD_TOOLS_SHIM),
        project_root.join(".claude").join(GSD_TOOLS_SHIM),
        project_root.join(".codex").join(GSD_TOOLS_SHIM),
    ];
    if let Some(home) = home {
        candidates.push(home.join(".claude").join(GSD_TOOLS_SHIM));
    }
    let codex_root = match codex_home.filter(|v| !v.is_empty()) {
        Some(explicit) => Some(PathBuf::from(explicit)),
        None => home.map(|h| h.join(".codex")),
    };
    if let Some(codex_root) = codex_root {
        candidates.push(codex_root.join(GSD_TOOLS_SHIM));
    }
    candidates
}

/// Resolve a runnable `gsd-tools` launcher for `project_root`.
///
/// Thin wrapper over [`resolve_gsd_tools_from`] that supplies the user's home
/// directory and `CODEX_HOME`. This is the ONLY place the resolver reads
/// process state for those, so tests never have to mutate HOME/CODEX_HOME.
fn resolve_gsd_tools(project_root: &Path) -> Option<GsdToolsCmd> {
    let home = dirs::home_dir();
    let codex_home = std::env::var_os("CODEX_HOME");
    resolve_gsd_tools_from(project_root, home.as_deref(), codex_home.as_deref())
}

/// Resolve a runnable `gsd-tools` launcher with an injected home/CODEX_HOME.
///
/// Walks [`gsd_tools_candidates`] in order and runs the first existing
/// `.cjs` shim through `node`; failing that, falls back to a `gsd-tools`
/// executable on `PATH` (invoked directly). Returns `None` when none resolve.
///
/// Follows upstream gsd-core 1.15.0 `gsd-core/references/gsd-run-resolver.md`
/// (#4834). Since #4667 a Codex install is self-contained under
/// `${CODEX_HOME:-~/.codex}/gsd-core`, so a Codex-only machine has no
/// `~/.claude/gsd-core`; the Codex candidates keep such a user on GSD's own
/// `smart-entry` rather than the keyword heuristic. On a dual install the
/// `.claude` home candidate is earlier and still wins.
///
/// Known, deliberate differences from upstream:
///   - `CLAUDE_CONFIG_DIR` is not honoured for the Claude home candidate;
///   - upstream's other runtime homes (Gemini, OpenCode, ...) are not probed;
///   - the PATH arm is not gated on `gsd-tools runtime-identity`.
///
/// No `GSD_RUNTIME` pin is applied to the `smart-entry` call: 1.15.0
/// `src/smart-entry.cts` hardcodes the `/gsd:` spelling regardless of the
/// resolved runtime, so a Codex install answers exactly like a Claude one.
fn resolve_gsd_tools_from(
    project_root: &Path,
    home: Option<&Path>,
    codex_home: Option<&OsStr>,
) -> Option<GsdToolsCmd> {
    for candidate in gsd_tools_candidates(project_root, home, codex_home) {
        if candidate.is_file() {
            return Some(GsdToolsCmd {
                program: "node".to_string(),
                prefix_args: vec![candidate.to_string_lossy().into_owned()],
            });
        }
    }
    if gsd_tools_on_path() {
        return Some(GsdToolsCmd {
            program: "gsd-tools".to_string(),
            prefix_args: Vec::new(),
        });
    }
    None
}

/// Return true if a `gsd-tools` executable is resolvable on `PATH`.
fn gsd_tools_on_path() -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let candidate = dir.join("gsd-tools");
        candidate.is_file()
    })
}

/// Run `gsd-tools smart-entry --json` in `project_root` and parse its output.
///
/// Returns `None` on launcher-resolution failure, spawn failure, non-zero
/// exit, or parse failure. Never panics — every failure degrades to `None`
/// so the caller can fall back to the keyword heuristic.
fn smart_entry_commands(project_root: &Path) -> Option<Vec<String>> {
    let launcher = resolve_gsd_tools(project_root)?;
    let output = launcher
        .command(&["smart-entry", "--json"])
        .current_dir(project_root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_smart_entry_json(&stdout)
}

/// Parse QUEUE.md content into a list of queued actions.
/// Skips empty lines and comment/header lines starting with `#`.
/// Strips leading `- ` prefix from list items.
pub fn parse_queue_md(content: &str) -> Vec<QueuedAction> {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| {
            let command = line.strip_prefix("- ").unwrap_or(line).to_string();
            QueuedAction { command }
        })
        .collect()
}

/// Serialize queued actions into QUEUE.md format.
pub fn write_queue_md(actions: &[QueuedAction]) -> String {
    let mut output = String::from("# Queue\n\n");
    for action in actions {
        output.push_str(&format!("- {}\n", action.command));
    }
    output
}

/// Return the canonical and legacy queue file paths for a `.planning/` dir.
///
/// Returns `(canonical, legacy)` where:
///   - canonical = `.planning/meta-manager/QUEUE.md`
///   - legacy    = `.planning/QUEUE.md`
///
/// The queue lives in an app-namespaced subdirectory (`meta-manager/`) rather
/// than at the `.planning/` root to satisfy GSD 1.8.0's `/gsd-health` W019
/// check. That check flags any non-canonical `*.md` FILE at the `.planning/`
/// root (hardcoded allowlist in gsd-core `src/artifacts.cts`, with no user
/// allowlist) but skips subdirectories entirely (`if (!entry.isFile())
/// continue` in gsd-core `src/verify.cts`). Placing the queue under a subdir
/// therefore produces zero health findings.
fn queue_paths(planning_dir: &Path) -> (PathBuf, PathBuf) {
    let canonical = planning_dir.join("meta-manager").join("QUEUE.md");
    let legacy = planning_dir.join("QUEUE.md");
    (canonical, legacy)
}

/// Load queued actions from a project's queue file.
///
/// Reads the canonical `.planning/meta-manager/QUEUE.md` first; if it is
/// absent, falls back to the legacy `.planning/QUEUE.md` so existing installs
/// keep working with no user action. Returns an empty vec if neither exists or
/// can't be read.
pub fn load_queue(planning_dir: &Path) -> Vec<QueuedAction> {
    let (canonical, legacy) = queue_paths(planning_dir);
    if let Ok(content) = std::fs::read_to_string(&canonical) {
        return parse_queue_md(&content);
    }
    match std::fs::read_to_string(&legacy) {
        Ok(content) => parse_queue_md(&content),
        Err(_) => Vec::new(),
    }
}

/// Save queued actions atomically to `.planning/meta-manager/QUEUE.md`.
///
/// Writes to a temporary file first, then renames to avoid partial writes,
/// creating the `meta-manager/` subdir if needed. After a successful write,
/// a lingering legacy root `.planning/QUEUE.md` is removed (one-shot migration
/// on first write). When the queue is empty, both the canonical file and any
/// legacy root file are removed.
pub fn save_queue(planning_dir: &Path, actions: &[QueuedAction]) -> anyhow::Result<()> {
    if !planning_dir.is_dir() {
        anyhow::bail!("Run GSD in this project first to enable queue");
    }
    let (canonical, legacy) = queue_paths(planning_dir);
    if actions.is_empty() {
        // Remove the queue (both canonical and legacy) when empty.
        let _ = std::fs::remove_file(&canonical);
        let _ = std::fs::remove_file(&legacy);
        return Ok(());
    }
    let meta_dir = canonical
        .parent()
        .expect("canonical queue path always has a parent");
    std::fs::create_dir_all(meta_dir)?;
    let content = write_queue_md(actions);
    let tmp_path = meta_dir.join("QUEUE.md.tmp");
    std::fs::write(&tmp_path, &content)?;
    std::fs::rename(&tmp_path, &canonical)?;
    // One-shot migration: remove the legacy root queue now that the canonical
    // location holds the current queue.
    let _ = std::fs::remove_file(&legacy);
    Ok(())
}

/// Suggest GSD commands based on current project state.
///
/// Prefers GSD 1.8.0's `gsd-tools smart-entry`, which understands situations
/// the keyword heuristic cannot (idle-stranded, external-job-waiting, milestone
/// boundaries) and always returns exactly one recommended `/gsd:*` action. When
/// the project has a resolvable launcher and smart-entry yields a non-empty
/// list, that list is used; otherwise it falls back to the keyword heuristic.
///
/// This runs on-demand (when building the suggestion list in response to a user
/// action, not per-frame), so a brief synchronous subprocess is acceptable; any
/// failure falls back to the keyword heuristic instantly.
pub fn suggest_next_commands(state: &super::ProjectState) -> Vec<String> {
    if !state.project_root.as_os_str().is_empty() {
        if let Some(commands) = smart_entry_commands(&state.project_root) {
            if !commands.is_empty() {
                return commands;
            }
        }
    }
    keyword_suggestions(state)
}

/// Keyword-heuristic suggestions derived from the project's status string.
///
/// The original `suggest_next_commands` body, preserved verbatim as the
/// fallback path when smart-entry is unavailable.
fn keyword_suggestions(state: &super::ProjectState) -> Vec<String> {
    let status_lower = state.status.to_lowercase();
    // The phase these commands should target is the one that is actually next
    // on disk, not the one the roadmap's completion count implies — see
    // `ProjectState::active_phase_number`. Suggesting `/gsd:plan-phase 3` for a
    // project already executing phase 4 is worse than suggesting nothing.
    let next_phase = state.active_phase_number();

    let mut suggestions = Vec::new();

    if status_lower.contains("ready to plan") || status_lower.contains("idle") {
        suggestions.push(format!("/gsd:discuss-phase {}", next_phase));
        suggestions.push(format!("/gsd:plan-phase {}", next_phase));
    } else if status_lower.contains("executing") || status_lower.contains("active") {
        suggestions.push(format!("/gsd:execute-phase {}", next_phase));
        suggestions.push(format!("/gsd:verify-work {}", next_phase));
    } else if status_lower.contains("complete") || status_lower.contains("done") {
        suggestions.push("/gsd:progress".to_string());
    } else {
        // Default fallback
        suggestions.push(format!("/gsd:discuss-phase {}", next_phase));
        suggestions.push(format!("/gsd:plan-phase {}", next_phase));
        suggestions.push(format!("/gsd:execute-phase {}", next_phase));
    }

    // Always include /gsd:quick as a generic suggestion
    suggestions.push("/gsd:quick".to_string());

    suggestions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_queue_md_basic() {
        let content = "# Queue\n\n- /gsd:plan-phase 4\n- /gsd:execute-phase 4\n";
        let actions = parse_queue_md(content);
        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].command, "/gsd:plan-phase 4");
        assert_eq!(actions[1].command, "/gsd:execute-phase 4");
    }

    #[test]
    fn test_parse_queue_md_empty() {
        let actions = parse_queue_md("");
        assert!(actions.is_empty());
    }

    #[test]
    fn test_parse_queue_md_comments_only() {
        let content = "# Queue\n# Some comment\n";
        let actions = parse_queue_md(content);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_parse_queue_md_no_prefix() {
        let content = "/gsd:quick\n";
        let actions = parse_queue_md(content);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].command, "/gsd:quick");
    }

    #[test]
    fn test_write_queue_md_roundtrip() {
        let actions = vec![
            QueuedAction {
                command: "/gsd:plan-phase 4".to_string(),
            },
            QueuedAction {
                command: "/gsd:execute-phase 4".to_string(),
            },
        ];
        let content = write_queue_md(&actions);
        let parsed = parse_queue_md(&content);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].command, actions[0].command);
        assert_eq!(parsed[1].command, actions[1].command);
    }

    #[test]
    fn test_suggest_idle_project() {
        let state = super::super::ProjectState {
            status: "idle".to_string(),
            completed_phases: 2,
            ..Default::default()
        };
        let suggestions = suggest_next_commands(&state);
        assert!(suggestions[0].contains("discuss-phase 3"));
        assert!(suggestions.last().unwrap() == "/gsd:quick");
    }

    #[test]
    fn test_suggest_executing_project() {
        let state = super::super::ProjectState {
            status: "executing phase 4".to_string(),
            completed_phases: 3,
            ..Default::default()
        };
        let suggestions = suggest_next_commands(&state);
        assert!(suggestions[0].contains("execute-phase 4"));
    }

    #[test]
    fn test_parse_smart_entry_recommended_first() {
        let json = r#"{
            "situation": "idle",
            "actions": [
                { "command": "/gsd:progress", "recommended": false },
                { "command": "/gsd:discuss-phase 3", "recommended": true },
                { "command": "/gsd:quick", "recommended": false }
            ]
        }"#;
        let commands = parse_smart_entry_json(json).expect("valid JSON should parse");
        assert_eq!(commands[0], "/gsd:discuss-phase 3");
        // The two non-recommended commands follow in their original order.
        assert_eq!(commands[1], "/gsd:progress");
        assert_eq!(commands[2], "/gsd:quick");
    }

    #[test]
    fn test_parse_smart_entry_preserves_non_recommended_order() {
        let json = r#"{
            "actions": [
                { "command": "/gsd:execute-phase 4", "recommended": true },
                { "command": "/gsd:verify-work 4", "recommended": false }
            ]
        }"#;
        let commands = parse_smart_entry_json(json).expect("valid JSON should parse");
        assert_eq!(
            commands,
            vec!["/gsd:execute-phase 4".to_string(), "/gsd:verify-work 4".to_string()]
        );
    }

    #[test]
    fn test_parse_smart_entry_empty_actions_is_none() {
        assert!(parse_smart_entry_json(r#"{ "actions": [] }"#).is_none());
        assert!(parse_smart_entry_json(r#"{ "situation": "idle" }"#).is_none());
    }

    #[test]
    fn test_parse_smart_entry_garbage_is_none() {
        assert!(parse_smart_entry_json("").is_none());
        assert!(parse_smart_entry_json("not json at all").is_none());
        assert!(parse_smart_entry_json("{ broken").is_none());
    }

    #[test]
    fn test_suggest_complete_project() {
        let state = super::super::ProjectState {
            status: "complete".to_string(),
            completed_phases: 5,
            ..Default::default()
        };
        let suggestions = suggest_next_commands(&state);
        assert_eq!(suggestions[0], "/gsd:progress");
    }

    #[test]
    fn test_load_queue_reads_legacy_only() {
        let td = tempfile::tempdir().unwrap();
        let planning = td.path();
        std::fs::write(planning.join("QUEUE.md"), "# Queue\n\n- /gsd:quick\n").unwrap();
        let actions = load_queue(planning);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].command, "/gsd:quick");
    }

    #[test]
    fn test_load_queue_new_path_wins_over_legacy() {
        let td = tempfile::tempdir().unwrap();
        let planning = td.path();
        std::fs::write(planning.join("QUEUE.md"), "# Queue\n\n- /gsd:legacy\n").unwrap();
        std::fs::create_dir_all(planning.join("meta-manager")).unwrap();
        std::fs::write(
            planning.join("meta-manager").join("QUEUE.md"),
            "# Queue\n\n- /gsd:canonical\n",
        )
        .unwrap();
        let actions = load_queue(planning);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].command, "/gsd:canonical");
    }

    #[test]
    fn test_save_queue_writes_new_path_and_removes_legacy() {
        let td = tempfile::tempdir().unwrap();
        let planning = td.path();
        std::fs::write(planning.join("QUEUE.md"), "# Queue\n\n- /gsd:old\n").unwrap();
        let actions = vec![QueuedAction {
            command: "/gsd:plan-phase 4".to_string(),
        }];
        save_queue(planning, &actions).unwrap();
        let canonical = planning.join("meta-manager").join("QUEUE.md");
        assert!(canonical.is_file(), "canonical queue file should exist");
        assert!(
            !planning.join("QUEUE.md").exists(),
            "legacy root queue should be migrated away"
        );
        let reloaded = load_queue(planning);
        assert_eq!(reloaded.len(), 1);
        assert_eq!(reloaded[0].command, "/gsd:plan-phase 4");
    }

    #[test]
    fn test_save_queue_empty_removes_both_files() {
        let td = tempfile::tempdir().unwrap();
        let planning = td.path();
        std::fs::write(planning.join("QUEUE.md"), "# Queue\n\n- /gsd:old\n").unwrap();
        std::fs::create_dir_all(planning.join("meta-manager")).unwrap();
        std::fs::write(
            planning.join("meta-manager").join("QUEUE.md"),
            "# Queue\n\n- /gsd:current\n",
        )
        .unwrap();
        save_queue(planning, &[]).unwrap();
        assert!(
            !planning.join("QUEUE.md").exists(),
            "legacy root queue should be removed on empty save"
        );
        assert!(
            !planning.join("meta-manager").join("QUEUE.md").exists(),
            "canonical queue should be removed on empty save"
        );
        assert!(load_queue(planning).is_empty());
    }

    #[test]
    fn test_suggest_empty_project_root_uses_keyword_path() {
        // With an empty project_root (default/test states), smart-entry is
        // never attempted and the keyword heuristic is used directly.
        let state = super::super::ProjectState {
            status: "idle".to_string(),
            completed_phases: 1,
            ..Default::default()
        };
        assert!(state.project_root.as_os_str().is_empty());
        let suggestions = suggest_next_commands(&state);
        assert_eq!(suggestions, keyword_suggestions(&state));
        assert!(suggestions[0].contains("discuss-phase 2"));
    }

    // --- gsd-tools resolver order (gsd-core 1.15.0 gsd-run-resolver.md) ---

    const SHIM: &str = "gsd-core/bin/gsd-tools.cjs";

    /// Plant a stub `gsd-tools.cjs` under `base/<rel>/gsd-core/bin/`.
    fn plant(base: &Path, rel: &str) -> PathBuf {
        let path = base.join(rel).join(SHIM);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "// stub\n").unwrap();
        path
    }

    fn node_cmd(path: &Path) -> GsdToolsCmd {
        GsdToolsCmd {
            program: "node".to_string(),
            prefix_args: vec![path.to_string_lossy().into_owned()],
        }
    }

    #[test]
    fn gsd_tools_candidates_follow_the_upstream_1_15_0_order() {
        let root = Path::new("/p");
        let home = Path::new("/h");
        assert_eq!(
            gsd_tools_candidates(root, Some(home), None),
            vec![
                root.join("gsd-core/bin/gsd-tools.cjs"),
                root.join(".claude/gsd-core/bin/gsd-tools.cjs"),
                root.join(".codex/gsd-core/bin/gsd-tools.cjs"),
                home.join(".claude/gsd-core/bin/gsd-tools.cjs"),
                home.join(".codex/gsd-core/bin/gsd-tools.cjs"),
            ]
        );
    }

    #[test]
    fn gsd_tools_candidates_honour_a_non_empty_codex_home() {
        let root = Path::new("/p");
        let home = Path::new("/h");
        let got = gsd_tools_candidates(root, Some(home), Some(OsStr::new("/x/codex")));
        assert_eq!(got.len(), 5, "CODEX_HOME replaces ~/.codex, it does not add");
        assert_eq!(
            got.last().unwrap(),
            &PathBuf::from("/x/codex/gsd-core/bin/gsd-tools.cjs")
        );
        assert!(!got.contains(&home.join(".codex/gsd-core/bin/gsd-tools.cjs")));
    }

    #[test]
    fn gsd_tools_candidates_treat_an_empty_codex_home_as_unset() {
        // Mirrors the shell `${CODEX_HOME:-$HOME/.codex}` expansion.
        let root = Path::new("/p");
        let home = Path::new("/h");
        let got = gsd_tools_candidates(root, Some(home), Some(OsStr::new("")));
        assert_eq!(
            got.last().unwrap(),
            &home.join(".codex/gsd-core/bin/gsd-tools.cjs")
        );
    }

    #[test]
    fn gsd_tools_candidates_without_a_home() {
        let root = Path::new("/p");
        assert_eq!(
            gsd_tools_candidates(root, None, None),
            vec![
                root.join("gsd-core/bin/gsd-tools.cjs"),
                root.join(".claude/gsd-core/bin/gsd-tools.cjs"),
                root.join(".codex/gsd-core/bin/gsd-tools.cjs"),
            ]
        );
        let got = gsd_tools_candidates(root, None, Some(OsStr::new("/x/codex")));
        assert_eq!(got.len(), 4);
        assert_eq!(
            got.last().unwrap(),
            &PathBuf::from("/x/codex/gsd-core/bin/gsd-tools.cjs")
        );
    }

    #[test]
    fn a_codex_only_home_install_resolves_through_node() {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        let codex = plant(home.path(), ".codex");
        assert_eq!(
            resolve_gsd_tools_from(root.path(), Some(home.path()), None),
            Some(node_cmd(&codex))
        );
    }

    #[test]
    fn a_claude_home_install_beats_a_codex_home_install() {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        plant(home.path(), ".codex");
        let claude = plant(home.path(), ".claude");
        assert_eq!(
            resolve_gsd_tools_from(root.path(), Some(home.path()), None),
            Some(node_cmd(&claude))
        );
    }

    #[test]
    fn a_project_local_codex_install_beats_a_home_install() {
        let home = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        plant(home.path(), ".claude");
        plant(home.path(), ".codex");
        let local = plant(root.path(), ".codex");
        assert_eq!(
            resolve_gsd_tools_from(root.path(), Some(home.path()), None),
            Some(node_cmd(&local))
        );
    }
}
