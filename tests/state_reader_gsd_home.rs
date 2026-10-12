//! `GSD_HOME` roots the global defaults file end to end (quick 261012-jqc).
//!
//! gsd-core resolves its global defaults store as
//! `(process.env.GSD_HOME || os.homedir()) + /.gsd/defaults.json`
//! (gsd-core fd1bfd09e, v1.16.0). This binary proves the production chain the
//! Config tab uses: process env -> `user_defaults_path()` ->
//! `load_user_defaults_from()`.
//!
//! **This file must hold exactly ONE `#[test]`.** The test mutates
//! process-global environment (`GSD_HOME`, `HOME`), and the test harness runs
//! tests in parallel threads of one process, so a second test here could race
//! it. Further `GSD_HOME` cases go inside the one function below, never into a
//! second test. This follows the `tests/driver_*.rs` precedent of mutating the
//! environment only inside its own test binary (its own process). The
//! resolution semantics themselves are covered race-free by pure unit tests in
//! `src/state_reader/config_json.rs`.

use std::path::Path;

use gsd_meta_manager::state_reader::config_json::{
    load_user_defaults_from, user_defaults_path, GSD_HOME_ENV,
};
use tempfile::TempDir;

/// Create a store root holding `.gsd/defaults.json` with the given `mode`.
fn store_with_mode(mode: &str) -> TempDir {
    let td = TempDir::new().expect("a store temp dir");
    let gsd = td.path().join(".gsd");
    std::fs::create_dir_all(&gsd).expect("create .gsd");
    std::fs::write(gsd.join("defaults.json"), format!(r#"{{"mode":"{mode}"}}"#))
        .expect("write defaults.json");
    td
}

fn defaults_in(root: &Path) -> std::path::PathBuf {
    root.join(".gsd").join("defaults.json")
}

/// Resolve through the production path and load what it points at, returning
/// the resolved path and the loaded `mode` (never Debug-printing a GsdConfig).
fn resolve_and_load() -> (std::path::PathBuf, String) {
    let path = user_defaults_path().expect("a usable root resolves a path");
    let mode = load_user_defaults_from(&path)
        .expect("the resolved defaults file loads")
        .mode;
    (path, mode)
}

#[test]
fn gsd_home_roots_the_global_defaults_file_with_home_as_fallback() {
    // The real store, and a decoy under HOME with a contradicting value, so a
    // resolver that ignores GSD_HOME is caught by the loaded mode as well as
    // the path.
    let gsd_store = store_with_mode("yolo");
    let home_decoy = store_with_mode("interactive");
    std::env::set_var("HOME", home_decoy.path());

    // 1. GSD_HOME set and non-empty: it wins over HOME.
    std::env::set_var(GSD_HOME_ENV, gsd_store.path());
    let (path, mode) = resolve_and_load();
    assert_eq!(path, defaults_in(gsd_store.path()));
    assert_eq!(mode, "yolo");

    // 2. GSD_HOME set to the empty string: falls back to HOME (upstream `||`).
    std::env::set_var(GSD_HOME_ENV, "");
    let (path, mode) = resolve_and_load();
    assert_eq!(path, defaults_in(home_decoy.path()));
    assert_eq!(mode, "interactive");

    // 3. GSD_HOME unset: falls back to HOME.
    std::env::remove_var(GSD_HOME_ENV);
    let (path, mode) = resolve_and_load();
    assert_eq!(path, defaults_in(home_decoy.path()));
    assert_eq!(mode, "interactive");
}
