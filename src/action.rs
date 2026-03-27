use crate::state_reader::ProjectState;
use crate::state_reader::backlog::BacklogItem;
use crate::state_reader::git_ops::{GitLogEntry, GitDiffStat};
use crossterm::event::KeyEvent;

#[derive(Debug, Clone)]
pub enum Action {
    Tick,
    RawKey(KeyEvent),
    Resize,
    FileChanged {
        project_path: std::path::PathBuf,
    },
    CreateProjectResult {
        alias: String,
        path: std::path::PathBuf,
        success: bool,
        error: Option<String>,
    },
    ProjectStateLoaded {
        alias: String,
        state: ProjectState,
    },
    BacklogLoaded {
        alias: String,
        items: Vec<BacklogItem>,
    },
    BacklogContentLoaded {
        alias: String,
        item_number: String,
        content: String,
    },
    GitLogLoaded {
        alias: String,
        entries: Vec<GitLogEntry>,
        planning_only: bool,
    },
    GitDiffStatLoaded {
        alias: String,
        hash: String,
        stat: GitDiffStat,
    },
    Noop,
}
