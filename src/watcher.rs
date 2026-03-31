// STATE-05: GSD hooks could push state updates via FIFO, signal, or socket.
// However, file-watching already covers all state change scenarios since GSD
// writes to .planning/ files on every state transition. Hook-based push adds
// complexity without meaningful benefit. The EventBus architecture supports
// adding additional event sources if hooks are ever needed.

use crate::action::Action;
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

/// Watches `.planning/` directories for filesystem changes and sends
/// `Action::FileChanged` events through the EventBus channel.
pub struct FileWatcher {
    debouncer: Debouncer<notify::RecommendedWatcher, RecommendedCache>,
}

/// Walk up from a changed file path to find the parent of `.planning/`.
/// Returns the project root (the directory containing `.planning/`).
fn extract_project_root(path: &Path) -> Option<&Path> {
    let mut current = path;
    loop {
        if let Some(name) = current.file_name() {
            if name == ".planning" {
                return current.parent();
            }
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => return None,
        }
    }
}

impl FileWatcher {
    /// Create a new FileWatcher that sends `Action::FileChanged` events
    /// through the given `tx` channel. Uses a 200ms debounce window.
    pub fn new(tx: UnboundedSender<Action>) -> anyhow::Result<Self> {
        let debouncer = new_debouncer(
            Duration::from_millis(200),
            None,
            move |result: DebounceEventResult| {
                match result {
                    Ok(events) => {
                        // Collect unique project roots from all changed paths
                        let mut seen = std::collections::HashSet::new();
                        for event in &events {
                            for path in &event.paths {
                                if let Some(root) = extract_project_root(path) {
                                    if seen.insert(root.to_path_buf()) {
                                        let _ = tx.send(Action::FileChanged {
                                            project_path: root.to_path_buf(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                    Err(errors) => {
                        for error in errors {
                            tracing::warn!("File watcher error: {}", error);
                        }
                    }
                }
            },
        )?;

        Ok(Self { debouncer })
    }

    /// Start watching a `.planning/` directory recursively.
    pub fn watch(&mut self, planning_dir: &Path) -> anyhow::Result<()> {
        self.debouncer
            .watch(planning_dir, RecursiveMode::Recursive)
            .map_err(|e| {
                tracing::warn!("Failed to watch {}: {}", planning_dir.display(), e);
                anyhow::anyhow!("Failed to watch {}: {}", planning_dir.display(), e)
            })
    }

    /// Stop watching a `.planning/` directory.
    pub fn unwatch(&mut self, planning_dir: &Path) -> anyhow::Result<()> {
        self.debouncer.unwatch(planning_dir).map_err(|e| {
            tracing::warn!("Failed to unwatch {}: {}", planning_dir.display(), e);
            anyhow::anyhow!("Failed to unwatch {}: {}", planning_dir.display(), e)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_project_root_from_state_md() {
        let path = PathBuf::from("/home/user/projects/myapp/.planning/STATE.md");
        let root = extract_project_root(&path);
        assert_eq!(root, Some(Path::new("/home/user/projects/myapp")));
    }

    #[test]
    fn test_extract_project_root_from_nested() {
        let path = PathBuf::from("/home/user/projects/myapp/.planning/phases/01/plan.md");
        let root = extract_project_root(&path);
        assert_eq!(root, Some(Path::new("/home/user/projects/myapp")));
    }

    #[test]
    fn test_extract_project_root_no_planning() {
        let path = PathBuf::from("/home/user/projects/myapp/src/main.rs");
        let root = extract_project_root(&path);
        assert_eq!(root, None);
    }
}
