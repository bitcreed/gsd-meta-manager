pub mod action;
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
pub mod journal;
pub mod main_loop;
pub mod project_creator;
pub mod registry;
pub mod session_detector;
pub mod state_reader;
pub mod terminal_switch;
/// Fixtures shared by the crate's in-module tests; never compiled into a release.
#[cfg(test)]
pub mod test_support;
pub mod text;
pub mod ui;
pub mod watcher;
