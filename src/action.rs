use crossterm::event::KeyEvent;

#[derive(Debug, Clone)]
pub enum Action {
    Tick,
    Quit,
    RawKey(KeyEvent),
    AddProjectConfirm {
        alias: String,
        path: std::path::PathBuf,
    },
    RemoveProjectConfirm {
        alias: String,
    },
    ProjectLoaded {
        alias: String,
        state: Option<crate::state_reader::ProjectState>,
    },
    Noop,
}
