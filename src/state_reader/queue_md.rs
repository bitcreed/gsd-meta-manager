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

/// Resolve a runnable `gsd-tools` launcher for `project_root`.
///
/// Checks candidate locations in the same order GSD's own launcher resolution
/// uses, returning the first that exists:
///   1. `<root>/gsd-core/bin/gsd-tools.cjs`      (run via `node`)
///   2. `<root>/.claude/gsd-core/bin/gsd-tools.cjs` (run via `node`)
///   3. `~/.claude/gsd-core/bin/gsd-tools.cjs`   (run via `node`)
///   4. `gsd-tools` on `PATH`                    (invoked directly)
///
/// Returns `None` when none resolve.
fn resolve_gsd_tools(project_root: &Path) -> Option<GsdToolsCmd> {
    let mut candidates: Vec<PathBuf> = vec![
        project_root.join("gsd-core/bin/gsd-tools.cjs"),
        project_root.join(".claude/gsd-core/bin/gsd-tools.cjs"),
    ];
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".claude/gsd-core/bin/gsd-tools.cjs"));
    }
    for candidate in candidates {
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

/// Load queued actions from a project's .planning/QUEUE.md file.
/// Returns an empty vec if the file doesn't exist or can't be read.
pub fn load_queue(planning_dir: &Path) -> Vec<QueuedAction> {
    let queue_path = planning_dir.join("QUEUE.md");
    match std::fs::read_to_string(&queue_path) {
        Ok(content) => parse_queue_md(&content),
        Err(_) => Vec::new(),
    }
}

/// Save queued actions atomically to .planning/QUEUE.md.
/// Writes to a temporary file first, then renames to avoid partial writes.
pub fn save_queue(planning_dir: &Path, actions: &[QueuedAction]) -> anyhow::Result<()> {
    if !planning_dir.is_dir() {
        anyhow::bail!("Run GSD in this project first to enable queue");
    }
    let final_path = planning_dir.join("QUEUE.md");
    if actions.is_empty() {
        // Remove QUEUE.md when queue is empty
        let _ = std::fs::remove_file(&final_path);
        return Ok(());
    }
    let content = write_queue_md(actions);
    let tmp_path = planning_dir.join("QUEUE.md.tmp");
    std::fs::write(&tmp_path, &content)?;
    std::fs::rename(&tmp_path, &final_path)?;
    Ok(())
}

/// Suggest GSD commands based on current project state.
/// Returns context-aware suggestions that the user can tab-cycle through.
pub fn suggest_next_commands(state: &super::ProjectState) -> Vec<String> {
    let status_lower = state.status.to_lowercase();
    let next_phase = state.completed_phases + 1;

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
}
