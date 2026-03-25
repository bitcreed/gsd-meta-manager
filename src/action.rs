#[derive(Debug, Clone)]
pub enum Action {
    Tick,
    Quit,
    MoveUp,
    MoveDown,
    AddProjectStart,
    AddProjectConfirm {
        alias: String,
        path: std::path::PathBuf,
    },
    RemoveProjectStart,
    RemoveProjectConfirm {
        alias: String,
    },
    ProjectLoaded {
        alias: String,
        state: Option<crate::state_reader::ProjectState>,
    },
    KeyInput(char),
    Backspace,
    CancelInput,
    Noop,
}
