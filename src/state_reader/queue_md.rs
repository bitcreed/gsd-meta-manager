use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct QueuedAction {
    pub command: String,
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
