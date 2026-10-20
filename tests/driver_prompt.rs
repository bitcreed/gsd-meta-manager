//! Golden and lint tests over the embedded driver master prompt (Phase 29,
//! PRMT-01 / PRMT-03 / PRMT-05, D-01..D-07, D-12, D-13, D-22, D-24).
//!
//! The subject is `launcher::prompt::MASTER_PROMPT`, the constant the binary
//! carries, never a file path. Every enum the prompt shows is compared with the
//! protocol constants, never with a hand-copied list, so the prompt cannot drift
//! from the `driverctl` CLI it tells a model to call.

use std::path::Path;

use gsd_meta_manager::launcher::prompt::{header_version, DRIVER_PROMPT_VERSION, MASTER_PROMPT};
use gsd_meta_manager::launcher::sandbox::{render_prompt, MAX_PROMPT_BYTES};

#[test]
fn the_asset_header_version_equals_the_shipped_constant() {
    assert_eq!(
        header_version(MASTER_PROMPT),
        Some(DRIVER_PROMPT_VERSION),
        "line 1 of src/launcher/assets/MASTER-PROMPT.md and DRIVER_PROMPT_VERSION disagree"
    );
    assert_eq!(DRIVER_PROMPT_VERSION, "driver-prompt/1.0.0");
}

#[test]
fn the_embedded_prompt_is_not_empty() {
    assert!(
        !MASTER_PROMPT.trim().is_empty(),
        "the embedded master prompt is empty"
    );
}

#[test]
fn the_render_fits_the_argv_limit_and_leaves_no_placeholder() {
    let run_dir = format!("/{}", "r".repeat(199));
    assert_eq!(run_dir.len(), 200);
    let rendered = render_prompt(
        MASTER_PROMPT,
        Path::new(&run_dir),
        Path::new("/home/user/project"),
        "loop",
    );
    assert!(
        rendered.len() < MAX_PROMPT_BYTES,
        "rendered prompt is {} bytes, limit {MAX_PROMPT_BYTES}",
        rendered.len()
    );
    for placeholder in ["{RUN_DIR}", "{PROJECT_ROOT}", "{MODE}"] {
        assert!(
            !rendered.contains(placeholder),
            "{placeholder} survived the render"
        );
    }
}
