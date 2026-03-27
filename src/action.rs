use crate::state_reader::ProjectState;
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
    GitLogLoaded {
        alias: String,
        entries: Vec<GitLogEntry>,
        planning_only: bool,
    },
    GitDiffStatLoaded {
        alias: String,
        stat: GitDiffStat,
    },
    SessionsDetected {
        sessions: Vec<crate::session_detector::ClaudeSession>,
    },
}
