pub mod action;
pub mod agents;
pub mod archive;
pub mod app;
pub mod browser;
pub mod change_tracker;
pub mod cli;
pub mod config;
pub mod driver;
pub mod envelope;
pub mod error;
pub mod executor;
/// The `GSDMM_EXPERIMENTAL_FEATURES` startup flag (quick task 260917-fko, D1).
pub mod experimental;
pub mod journal;
pub mod main_loop;
pub mod project_creator;
pub mod registry;
pub mod session_detector;
pub mod state_reader;
pub mod terminal_switch;
/// Shared fixture shape constants; two consts and no behaviour (D-17-5).
pub mod test_support;
pub mod text;
pub mod ui;
pub mod watcher;
