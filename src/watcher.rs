// STATE-05: GSD hooks could push state updates via FIFO, signal, or socket.
// However, file-watching already covers all state change scenarios since GSD
// writes to .planning/ files on every state transition. Hook-based push adds
// complexity without meaningful benefit. The EventBus architecture supports
// adding additional event sources if hooks are ever needed.

use crate::action::Action;
use crate::journal::{classify_change, ChangeKind};
use notify::RecursiveMode;
use notify_debouncer_full::{new_debouncer, DebounceEventResult, Debouncer, RecommendedCache};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
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

/// Fold one debounce batch of changed paths into the actions to send.
///
/// **The dedup key is `(project_root, classification)`, not `project_root`,
/// and that is a correctness fix rather than a tidy-up (D-10).** With a
/// per-root key the *first* path for a project in a batch wins and every later
/// path for that root is silently dropped. Once driver journal appends start
/// arriving, a single 200 ms batch routinely carries a journal append **and** a
/// `STATE.md` write — and if the journal append comes first, the state write is
/// swallowed and the project never re-parses. Dedup on the pair and each root
/// emits at most one driver-path event and one planning-path event per batch.
/// Do not simplify this back to a set of roots.
///
/// This lives outside the debouncer closure so the fold is reachable from a
/// test without a live debouncer; without that extraction the regression above
/// would ship silently.
///
/// [`classify_change`] is a **pure** function with no filesystem access (D-11),
/// which is precisely what lets it run here: this body executes on the
/// debouncer's callback thread, which has no runtime, no `App`, and no business
/// stat-ing anything.
fn batch_actions<'a>(paths: impl IntoIterator<Item = &'a Path>) -> Vec<Action> {
    let mut seen: HashSet<(PathBuf, ChangeKind)> = HashSet::new();
    let mut actions = Vec::new();

    for path in paths {
        let Some(root) = extract_project_root(path) else {
            continue;
        };
        let kind = classify_change(root, path);
        if seen.insert((root.to_path_buf(), kind)) {
            actions.push(Action::FileChanged {
                project_path: root.to_path_buf(),
                changed_path: path.to_path_buf(),
            });
        }
    }

    actions
}

impl FileWatcher {
    /// Create a new FileWatcher that sends `Action::FileChanged` events
    /// through the given `tx` channel. Uses a 200ms debounce window.
    ///
    /// No new watcher registration is needed for the driver's run directories:
    /// [`FileWatcher::watch`] is already `RecursiveMode::Recursive` and
    /// [`extract_project_root`] already resolves arbitrarily deep `.planning/`
    /// paths. That is the entire reason "free watching" was free in the first
    /// place — and why the only cost of the driver is on the consumer side.
    pub fn new(tx: UnboundedSender<Action>) -> anyhow::Result<Self> {
        let debouncer = new_debouncer(
            Duration::from_millis(200),
            None,
            move |result: DebounceEventResult| {
                match result {
                    Ok(events) => {
                        let paths = events.iter().flat_map(|event| event.paths.iter());
                        for action in batch_actions(paths.map(PathBuf::as_path)) {
                            let _ = tx.send(action);
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

    const ROOT: &str = "/home/user/projects/myapp";
    const RUN: &str = "2026-07-29T09-00-00Z-a3f9";

    fn fold(paths: &[PathBuf]) -> Vec<Action> {
        batch_actions(paths.iter().map(PathBuf::as_path))
    }

    fn changed_paths(actions: &[Action]) -> Vec<&Path> {
        actions
            .iter()
            .map(|action| match action {
                Action::FileChanged { changed_path, .. } => changed_path.as_path(),
                other => panic!("the fold must only emit FileChanged, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_batch_with_the_journal_first_still_emits_the_planning_event() {
        // The ordering is the whole point. Under the old per-root dedup the
        // FIRST path for a root won and every later path was dropped, so a
        // batch shaped like this one — journal append at index 0, state write
        // at index 1 — silently lost the state write and the project never
        // re-parsed (D-10, RESEARCH Pitfall 6). Reordering this assertion to
        // accept a single action would re-admit that bug.
        let root = PathBuf::from(ROOT);
        let journal = root.join(format!(".planning/meta-manager/runs/{RUN}/journal.jsonl"));
        let state_md = root.join(".planning/STATE.md");

        let actions = fold(&[journal.clone(), state_md.clone()]);

        assert_eq!(
            actions.len(),
            2,
            "a journal append must not swallow the STATE.md write that follows it \
             in the same batch; got {:?}",
            changed_paths(&actions)
        );
        assert_eq!(changed_paths(&actions), vec![journal.as_path(), state_md.as_path()]);
    }

    #[test]
    fn two_journal_paths_in_one_batch_emit_one_driver_event() {
        let root = PathBuf::from(ROOT);
        let actions = fold(&[
            root.join(format!(".planning/meta-manager/runs/{RUN}/journal.jsonl")),
            root.join(format!(".planning/meta-manager/runs/{RUN}/run.json")),
        ]);

        assert_eq!(
            actions.len(),
            1,
            "two writes under the same run collapse to one tail; got {:?}",
            changed_paths(&actions)
        );
    }

    #[test]
    fn a_driver_path_and_a_planning_path_emit_two_events_for_one_root() {
        let root = PathBuf::from(ROOT);
        let state_md = root.join(".planning/STATE.md");
        let journal = root.join(format!(".planning/meta-manager/runs/{RUN}/journal.jsonl"));

        // Planning first — the ordering the old shape happened to survive.
        let actions = fold(&[state_md.clone(), journal.clone()]);
        assert_eq!(changed_paths(&actions), vec![state_md.as_path(), journal.as_path()]);

        for action in &actions {
            let Action::FileChanged { project_path, .. } = action else {
                unreachable!()
            };
            assert_eq!(project_path, &root, "both events name the same project root");
        }

        // Two planning writes for one root still collapse to one, which is the
        // half of the old behaviour worth keeping.
        let collapsed = fold(&[state_md, root.join(".planning/ROADMAP.md")]);
        assert_eq!(collapsed.len(), 1);
    }

    #[test]
    fn paths_outside_a_planning_directory_emit_nothing() {
        let actions = fold(&[
            PathBuf::from("/home/user/projects/myapp/src/main.rs"),
            PathBuf::from("/etc/passwd"),
        ]);
        assert!(actions.is_empty(), "got {:?}", changed_paths(&actions));
    }
}
